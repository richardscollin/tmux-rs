// Copyright (c) 2007 Nicholas Marriott <nicholas.marriott@gmail.com>
//
// Permission to use, copy, modify, and distribute this software for any
// purpose with or without fee is hereby granted, provided that the above
// copyright notice and this permission notice appear in all copies.
//
// THE SOFTWARE IS PROVIDED "AS IS" AND THE AUTHOR DISCLAIMS ALL WARRANTIES
// WITH REGARD TO THIS SOFTWARE INCLUDING ALL IMPLIED WARRANTIES OF
// MERCHANTABILITY AND FITNESS. IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR
// ANY SPECIAL, DIRECT, INDIRECT, OR CONSEQUENTIAL DAMAGES OR ANY DAMAGES
// WHATSOEVER RESULTING FROM LOSS OF MIND, USE, DATA OR PROFITS, WHETHER
// IN AN ACTION OF CONTRACT, NEGLIGENCE OR OTHER TORTIOUS ACTION, ARISING
// OUT OF OR IN CONNECTION WITH THE USE OR PERFORMANCE OF THIS SOFTWARE.
use crate::*;

use crate::xmalloc::xstrndup;

use crate::compat::{S_ISDIR, fdforkpty::getptmfd, getprogname::getprogname};
use crate::libc::{
    CLOCK_MONOTONIC, CLOCK_REALTIME, CODESET, EEXIST, F_GETFL, F_SETFL, LC_CTYPE, LC_TIME,
    O_NONBLOCK, PATH_MAX, S_IRWXO, S_IRWXU, X_OK, access, clock_gettime, fcntl, getcwd, getenv,
    getpwuid, getuid, lstat, mkdir, nl_langinfo, realpath, setlocale, stat, strcasecmp, strcasestr,
    strchr, strcspn, strerror, strncmp, strrchr, strstr, timespec,
};

use crate::compat::getopt::{OPTARG, OPTIND, getopt};

pub static mut GLOBAL_OPTIONS: *mut options = null_mut();

pub static mut GLOBAL_S_OPTIONS: *mut options = null_mut();

pub static mut GLOBAL_W_OPTIONS: *mut options = null_mut();

pub static mut GLOBAL_ENVIRON: *mut environ = null_mut();

pub static mut START_TIME: timeval = timeval {
    tv_sec: 0,
    tv_usec: 0,
};

pub static mut SOCKET_PATH: *const u8 = null_mut();

pub static mut PTM_FD: c_int = -1;

pub static mut SHELL_COMMAND: *mut u8 = null_mut();

pub fn usage() -> ! {
    eprintln!(
        "usage: tmux-rs [-2CDlNuVv] [-c shell-command] [-f file] [-L socket-name]\n               [-S socket-path] [-T features] [command [flags]]\n"
    );
    std::process::exit(1)
}

pub unsafe fn getshell() -> *const u8 {
    unsafe {
        let shell = getenv(c!("SHELL"));
        if checkshell(shell) {
            return shell;
        }

        let pw = getpwuid(getuid());
        if !pw.is_null() && checkshell((*pw).pw_shell.cast()) {
            return (*pw).pw_shell.cast();
        }

        _PATH_BSHELL
    }
}

pub unsafe fn checkshell(shell: *const u8) -> bool {
    unsafe {
        if shell.is_null() || *shell != b'/' {
            return false;
        }
        if areshell(shell) != 0 {
            return false;
        }
        if access(shell.cast(), X_OK) != 0 {
            return false;
        }
    }
    true
}

pub unsafe fn areshell(shell: *const u8) -> c_int {
    unsafe {
        let ptr = strrchr(shell, b'/' as c_int);
        let ptr = if !ptr.is_null() {
            ptr.wrapping_add(1)
        } else {
            shell
        };
        let mut progname = getprogname();
        if *progname == b'-' {
            progname = progname.wrapping_add(1);
        }
        if libc::strcmp(ptr, progname) == 0 {
            1
        } else {
            0
        }
    }
}

pub unsafe fn expand_path(path: *const u8, home: *const u8) -> *mut u8 {
    unsafe {
        let mut expanded: *mut u8 = null_mut();
        let mut end: *const u8 = null_mut();

        if strncmp(path, c!("~/"), 2) == 0 {
            if home.is_null() {
                return null_mut();
            }
            return format_nul!("{}{}", _s(home), _s(path.add(1)));
        }

        if *path == b'$' {
            end = strchr(path, b'/' as i32);
            let name = if end.is_null() {
                xstrdup(path.add(1)).cast().as_ptr()
            } else {
                xstrndup(path.add(1), end.addr() - path.addr() - 1)
                    .cast()
                    .as_ptr()
            };
            let value = environ_find(GLOBAL_ENVIRON, name);
            free_(name);
            if value.is_null() {
                return null_mut();
            }
            if end.is_null() {
                end = c!("");
            }
            return format_nul!("{}{}", _s(transmute_ptr((*value).value)), _s(end));
        }

        xstrdup(path).cast().as_ptr()
    }
}

unsafe fn expand_paths(s: &str, paths: *mut *mut *mut u8, n: *mut u32, ignore_errors: i32) {
    unsafe {
        let home = find_home();
        let mut next: *const u8 = null_mut();
        let mut resolved: [u8; PATH_MAX as usize] = zeroed(); // TODO use unint version
        let mut path = null_mut();

        let func = "expand_paths";

        *paths = null_mut();
        *n = 0;

        let mut tmp: *mut u8 = xstrdup__(s);
        let copy = tmp;
        while {
            next = strsep(&raw mut tmp as _, c!(":").cast());
            !next.is_null()
        } {
            let expanded = expand_path(next, home);
            if expanded.is_null() {
                log_debug!("{}: invalid path: {}", func, _s(next));
                continue;
            }
            if realpath(expanded.cast(), resolved.as_mut_ptr()).is_null() {
                log_debug!(
                    "{}: realpath(\"{}\") failed: {}",
                    func,
                    _s(expanded),
                    _s(strerror(errno!())),
                );
                if ignore_errors != 0 {
                    free_(expanded);
                    continue;
                }
                path = expanded;
            } else {
                path = xstrdup(resolved.as_ptr()).cast().as_ptr();
                free_(expanded);
            }
            let mut i = 0;
            for j in 0..*n {
                i = j;
                if libc::strcmp(path as _, *(*paths).add(i as usize)) == 0 {
                    break;
                }
            }
            if i != *n {
                log_debug!("{}: duplicate path: {}", func, _s(path));
                free_(path);
                continue;
            }
            *paths = xreallocarray_::<*mut u8>(*paths, (*n + 1) as usize).as_ptr();
            *(*paths).add((*n) as usize) = path;
            *n += 1;
        }
        free_(copy);
    }
}

unsafe fn make_label(mut label: *const u8, cause: *mut *mut u8) -> *const u8 {
    let mut paths: *mut *mut u8 = null_mut();
    let mut path: *mut u8 = null_mut();
    let mut base: *mut u8 = null_mut();
    let mut sb: stat = unsafe { zeroed() }; // TODO use uninit
    let mut n: u32 = 0;

    unsafe {
        'fail: {
            *cause = null_mut();
            if label.is_null() {
                label = c!("default");
            }
            let uid = getuid();

            expand_paths(TMUX_SOCK, &raw mut paths, &raw mut n, 1);
            if n == 0 {
                *cause = format_nul!("no suitable socket path");
                return null_mut();
            }
            path = *paths; /* can only have one socket! */
            for i in 1..n {
                free_(*paths.add(i as usize));
            }
            free_(paths);

            base = format_nul!("{}/tmux-{}", _s(path), uid);
            free_(path);
            if mkdir(base.cast(), S_IRWXU) != 0 && errno!() != EEXIST {
                *cause = format_nul!(
                    "couldn't create directory {} ({})",
                    _s(base),
                    _s(strerror(errno!()))
                );
                break 'fail;
            }
            if lstat(base.cast(), &raw mut sb) != 0 {
                *cause = format_nul!(
                    "couldn't read directory {} ({})",
                    _s(base),
                    _s(strerror(errno!())),
                );
                break 'fail;
            }
            if !S_ISDIR(sb.st_mode) {
                *cause = format_nul!("{} is not a directory", _s(base));
                break 'fail;
            }
            if sb.st_uid != uid || (sb.st_mode & S_IRWXO) != 0 {
                *cause = format_nul!("directory {} has unsafe permissions", _s(base));
                break 'fail;
            }
            path = format_nul!("{}/{}", _s(base), _s(label));
            free_(base);
            return path;
        }

        // fail:
        free_(base);
        null_mut()
    }
}

pub unsafe fn shell_argv0(shell: *const u8, is_login: c_int) -> *mut u8 {
    unsafe {
        let slash = strrchr(shell, b'/' as _);
        let name = if !slash.is_null() && *slash.add(1) != b'\0' {
            slash.add(1)
        } else {
            shell
        };

        if is_login != 0 {
            format_nul!("-{}", _s(name))
        } else {
            format_nul!("{}", _s(name))
        }
    }
}

pub unsafe fn setblocking(fd: c_int, state: c_int) {
    unsafe {
        let mut mode = fcntl(fd, F_GETFL);

        if mode != -1 {
            if state == 0 {
                mode |= O_NONBLOCK;
            } else {
                mode &= !O_NONBLOCK;
            }
            fcntl(fd, F_SETFL, mode);
        }
    }
}

pub unsafe fn get_timer() -> u64 {
    unsafe {
        let mut ts: timespec = zeroed();
        //We want a timestamp in milliseconds suitable for time measurement,
        //so prefer the monotonic clock.
        if clock_gettime(CLOCK_MONOTONIC, &raw mut ts) != 0 {
            clock_gettime(CLOCK_REALTIME, &raw mut ts);
        }
        (ts.tv_sec as u64 * 1000) + (ts.tv_nsec as u64 / 1000000)
    }
}

pub unsafe fn find_cwd() -> *mut u8 {
    static mut CWD: [u8; PATH_MAX as usize] = [0; PATH_MAX as usize];
    unsafe {
        let mut resolved1: [u8; PATH_MAX as usize] = [0; PATH_MAX as usize];
        let mut resolved2: [u8; PATH_MAX as usize] = [0; PATH_MAX as usize];

        if getcwd(&raw mut CWD as _, size_of::<[u8; PATH_MAX as usize]>()).is_null() {
            return null_mut();
        }
        let pwd = getenv(c!("PWD"));
        if pwd.is_null() || *pwd == b'\0' {
            return &raw mut CWD as _;
        }

        //We want to use PWD so that symbolic links are maintained,
        //but only if it matches the actual working directory.

        if realpath(pwd, &raw mut resolved1 as _).is_null() {
            return &raw mut CWD as _;
        }
        if realpath(&raw mut CWD as _, &raw mut resolved2 as _).is_null() {
            return &raw mut CWD as _;
        }
        if libc::strcmp(&raw mut resolved1 as _, &raw mut resolved2 as _) != 0 {
            return &raw mut CWD as _;
        }
        pwd
    }
}

pub unsafe fn find_home() -> *mut u8 {
    static mut HOME: *mut u8 = null_mut();

    unsafe {
        if !HOME.is_null() {
            HOME
        } else {
            HOME = getenv(c!("HOME"));
            if HOME.is_null() || *HOME == b'\0' {
                let pw = getpwuid(getuid());
                if !pw.is_null() {
                    HOME = (*pw).pw_dir.cast();
                } else {
                    HOME = null_mut();
                }
            }

            HOME
        }
    }
}

pub fn getversion() -> &'static str {
    crate::TMUX_VERSION
}

/// entrypoint for tmux binary
pub unsafe fn tmux_main(mut argc: i32, mut argv: *mut *mut u8, env: *mut *mut u8) {
    std::panic::set_hook(Box::new(|panic_info| {
        let backtrace = std::backtrace::Backtrace::capture();
        let err_str = format!("{backtrace:#?}");
        std::fs::write("client-panic.txt", err_str).unwrap();
    }));

    unsafe {
        // setproctitle_init(argc, argv.cast(), env.cast());
        let mut cause: *mut u8 = null_mut();
        let mut path: *const u8 = null_mut();
        let mut label: *mut u8 = null_mut();
        let mut feat: i32 = 0;
        let mut fflag: i32 = 0;
        let mut flags: client_flag = client_flag::empty();

        if setlocale(LC_CTYPE, c!("en_US.UTF-8")).is_null()
            && setlocale(LC_CTYPE, c!("C.UTF-8")).is_null()
        {
            if setlocale(LC_CTYPE, c!("")).is_null() {
                eprintln!("invalid LC_ALL, LC_CTYPE or LANG");
                std::process::exit(1);
            }
            let s: *mut u8 = nl_langinfo(CODESET).cast();
            if strcasecmp(s, c!("UTF-8")) != 0 && strcasecmp(s, c!("UTF8")) != 0 {
                eprintln!("need UTF-8 locale (LC_CTYPE) but have {}", _s(s));
                std::process::exit(1);
            }
        }

        setlocale(LC_TIME, c!(""));
        tzset();

        if **argv == b'-' {
            flags = client_flag::LOGIN;
        }

        GLOBAL_ENVIRON = environ_create().as_ptr();

        let mut var = environ;
        while !(*var).is_null() {
            environ_put(GLOBAL_ENVIRON, *var, 0);
            var = var.add(1);
        }

        let cwd = find_cwd();
        if !cwd.is_null() {
            environ_set!(GLOBAL_ENVIRON, c!("PWD"), 0, "{}", _s(cwd));
        }
        expand_paths(TMUX_CONF, &raw mut CFG_FILES, &raw mut CFG_NFILES, 1);

        let mut opt;
        while {
            opt = getopt(argc, argv.cast(), c!("2c:CDdf:lL:NqS:T:uUvV"));
            opt != -1
        } {
            match opt as u8 {
                b'2' => tty_add_features(&raw mut feat, "256", c!(":,")),
                b'c' => SHELL_COMMAND = OPTARG.cast(),
                b'D' => flags |= client_flag::NOFORK,
                b'C' => {
                    if flags.intersects(client_flag::CONTROL) {
                        flags |= client_flag::CONTROLCONTROL;
                    } else {
                        flags |= client_flag::CONTROL;
                    }
                }
                b'f' => {
                    if fflag == 0 {
                        fflag = 1;
                        for i in 0..CFG_NFILES {
                            free((*CFG_FILES.add(i as usize)) as _);
                        }
                        CFG_NFILES = 0;
                    }
                    CFG_FILES =
                        xreallocarray_::<*mut u8>(CFG_FILES, CFG_NFILES as usize + 1).as_ptr();
                    *CFG_FILES.add(CFG_NFILES as usize) = xstrdup(OPTARG.cast()).cast().as_ptr();
                    CFG_NFILES += 1;
                    CFG_QUIET.store(false, atomic::Ordering::Relaxed);
                }
                b'V' => {
                    println!("tmux {}", getversion());
                    std::process::exit(0);
                }
                b'l' => flags |= client_flag::LOGIN,
                b'L' => {
                    free(label as _);
                    label = xstrdup(OPTARG.cast()).cast().as_ptr();
                }
                b'N' => flags |= client_flag::NOSTARTSERVER,
                b'q' => (),
                b'S' => {
                    free(path as _);
                    path = xstrdup(OPTARG.cast()).cast().as_ptr();
                }
                b'T' => tty_add_features(&raw mut feat, cstr_to_str(OPTARG.cast()), c!(":,")),
                b'u' => flags |= client_flag::UTF8,
                b'v' => log_add_level(),
                _ => usage(),
            }
        }
        argc -= OPTIND;
        argv = argv.add(OPTIND as usize);

        if !SHELL_COMMAND.is_null() && argc != 0 {
            usage();
        }
        if flags.intersects(client_flag::NOFORK) && argc != 0 {
            usage();
        }

        PTM_FD = getptmfd();
        if PTM_FD == -1 {
            eprintln!("getptmfd failed!");
            std::process::exit(1);
        }

        /*
        // TODO no pledge on linux
            if pledge("stdio rpath wpath cpath flock fattr unix getpw sendfd recvfd proc exec tty ps", null_mut()) != 0 {
                err(1, "pledge");
        }
        */

        // tmux is a UTF-8 terminal, so if TMUX is set, assume UTF-8.
        // Otherwise, if the user has set LC_ALL, LC_CTYPE or LANG to contain
        // UTF-8, it is a safe assumption that either they are using a UTF-8
        // terminal, or if not they know that output from UTF-8-capable
        // programs may be wrong.
        if !getenv(c!("TMUX")).is_null() {
            flags |= client_flag::UTF8;
        } else {
            let mut s = getenv(c!("LC_ALL")) as *const u8;
            if s.is_null() || *s == b'\0' {
                s = getenv(c!("LC_CTYPE")) as *const u8;
            }
            if s.is_null() || *s == b'\0' {
                s = getenv(c!("LANG")) as *const u8;
            }
            if s.is_null() || *s == b'\0' {
                s = c!("");
            }
            if !strcasestr(s, c!("UTF-8")).is_null() || !strcasestr(s, c!("UTF8")).is_null() {
                flags |= client_flag::UTF8;
            }
        }

        GLOBAL_OPTIONS = options_create(null_mut());
        GLOBAL_S_OPTIONS = options_create(null_mut());
        GLOBAL_W_OPTIONS = options_create(null_mut());

        let mut oe: *const options_table_entry = &raw const OPTIONS_TABLE as _;
        while !(*oe).name.is_null() {
            if (*oe).scope & OPTIONS_TABLE_SERVER != 0 {
                options_default(GLOBAL_OPTIONS, oe);
            }
            if (*oe).scope & OPTIONS_TABLE_SESSION != 0 {
                options_default(GLOBAL_S_OPTIONS, oe);
            }
            if (*oe).scope & OPTIONS_TABLE_WINDOW != 0 {
                options_default(GLOBAL_W_OPTIONS, oe);
            }
            oe = oe.add(1);
        }

        // The default shell comes from SHELL or from the user's passwd entry if available.
        options_set_string!(
            GLOBAL_S_OPTIONS,
            c!("default-shell"),
            0,
            "{}",
            _s(getshell()),
        );

        // Override keys to vi if VISUAL or EDITOR are set.
        let mut s = getenv(c!("VISUAL"));
        if !s.is_null()
            || ({
                s = getenv(c!("EDITOR"));
                !s.is_null()
            })
        {
            options_set_string!(GLOBAL_OPTIONS, c!("editor"), 0, "{}", _s(s));
            if !strrchr(s, b'/' as _).is_null() {
                s = strrchr(s, b'/' as _).add(1);
            }
            let keys = if !strstr(s, c!("vi")).is_null() {
                modekey::MODEKEY_VI
            } else {
                modekey::MODEKEY_EMACS
            };
            options_set_number(GLOBAL_S_OPTIONS, c!("status-keys"), keys as _);
            options_set_number(GLOBAL_W_OPTIONS, c!("mode-keys"), keys as _);
        }

        // If socket is specified on the command-line with -S or -L, it is
        // used. Otherwise, $TMUX is checked and if that fails "default" is
        // used.
        if path.is_null() && label.is_null() {
            s = getenv(c!("TMUX"));
            if !s.is_null() && *s != b'\0' && *s != b',' {
                let tmp: *mut u8 = xstrdup(s).cast().as_ptr();
                *tmp.add(strcspn(tmp, c!(","))) = b'\0';
                path = tmp;
            }
        }
        if path.is_null() {
            path = make_label(label.cast(), &raw mut cause);
            if path.is_null() {
                if !cause.is_null() {
                    eprintln!("{}", _s(cause));
                    free(cause as _);
                }
                std::process::exit(1);
            }
            flags |= client_flag::DEFAULTSOCKET;
        }
        SOCKET_PATH = path;
        free_(label);

        // Pass control to the client.
        std::process::exit(client_main(osdep_event_init(), argc, argv, flags, feat))
    }
}

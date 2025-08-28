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
use std::borrow::Cow;
use std::path::PathBuf;
use std::sync::OnceLock;

use crate::compat::getopt::{OPTARG, OPTIND, getopt};
use crate::compat::{S_ISDIR, fdforkpty::getptmfd, getprogname::getprogname};
use crate::libc::{
    CLOCK_MONOTONIC, CLOCK_REALTIME, CODESET, EEXIST, F_GETFL, F_SETFL, LC_CTYPE, LC_TIME,
    O_NONBLOCK, S_IRWXO, S_IRWXU, X_OK, access, clock_gettime, fcntl, getpwuid, getuid, lstat,
    mkdir, nl_langinfo, setlocale, stat, strcasecmp, strchr, strcspn, strerror, strncmp, strrchr,
    timespec,
};
use crate::*;

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

unsafe fn getshell() -> Cow<'static, CStr> {
    unsafe {
        if let Ok(shell) = std::env::var("SHELL")
            && let shell = CString::new(shell).unwrap()
            && checkshell(Some(&shell))
        {
            return Cow::Owned(shell);
        }

        if let Some(pw) = NonNull::new(getpwuid(getuid()))
            && !(*pw.as_ptr()).pw_shell.is_null()
            && checkshell(Some(CStr::from_ptr((*pw.as_ptr()).pw_shell)))
        {
            return Cow::Owned(CString::new(cstr_to_str((*pw.as_ptr()).pw_shell.cast())).unwrap());
        }

        Cow::Borrowed(CStr::from_ptr(_PATH_BSHELL.cast()))
    }
}

pub unsafe fn checkshell(shell: Option<&CStr>) -> bool {
    unsafe {
        let Some(shell) = shell else {
            return false;
        };
        if shell.to_bytes()[0] != b'/' {
            return false;
        }
        if areshell(shell) {
            return false;
        }
        if access(shell.as_ptr().cast(), X_OK) != 0 {
            return false;
        }
    }
    true
}

pub unsafe fn checkshell_(shell: *const u8) -> bool {
    unsafe {
        if shell.is_null() {
            return false;
        };
        if *shell != b'/' {
            return false;
        }
        if areshell(CStr::from_ptr(shell.cast())) {
            return false;
        }
        if access(shell.cast(), X_OK) != 0 {
            return false;
        }
    }
    true
}

unsafe fn areshell(shell: &CStr) -> bool {
    unsafe {
        let ptr = strrchr(shell.as_ptr().cast(), b'/' as c_int);
        let ptr = if !ptr.is_null() {
            ptr.wrapping_add(1)
        } else {
            shell.as_ptr().cast()
        };
        let mut progname = getprogname();
        if *progname == b'-' {
            progname = progname.wrapping_add(1);
        }
        libc::strcmp(ptr, progname) == 0
    }
}

unsafe fn expand_path(path: *const u8, home: Option<&CStr>) -> Option<CString> {
    unsafe {
        if strncmp(path, c!("~/"), 2) == 0 {
            return Some(
                CString::new(format!("{}{}", home?.to_str().unwrap(), _s(path.add(1)))).unwrap(),
            );
        }

        if *path == b'$' {
            let mut end: *const u8 = strchr(path, b'/' as i32).cast();
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
                return None;
            }
            if end.is_null() {
                end = c!("");
            }
            return Some(
                CString::new(format!("{}{}", _s(transmute_ptr((*value).value)), _s(end))).unwrap(),
            );
        }

        Some(CString::new(cstr_to_str(path)).unwrap())
    }
}

unsafe fn expand_paths(s: &str, paths: &mut Vec<CString>, ignore_errors: i32) {
    unsafe {
        let home = find_home();
        let mut path: CString;

        let func = "expand_paths";

        paths.clear();

        let mut next: *const u8;
        let mut tmp: *mut u8 = xstrdup__(s);
        let copy = tmp;
        while {
            next = strsep(&raw mut tmp as _, c!(":").cast());
            !next.is_null()
        } {
            let Some(expanded) = expand_path(next, home) else {
                log_debug!("{func}: invalid path: {}", _s(next));
                continue;
            };

            match PathBuf::from(expanded.to_str().unwrap()).canonicalize() {
                Ok(resolved) => {
                    path = CString::new(resolved.into_os_string().into_string().unwrap()).unwrap();
                    // free_(expanded);
                }
                Err(_) => {
                    log_debug!(
                        "{func}: realpath(\"{}\") failed: {}",
                        expanded.to_string_lossy(),
                        _s(strerror(errno!())),
                    );
                    if ignore_errors != 0 {
                        // free_(expanded);
                        continue;
                    }
                    path = expanded;
                }
            }

            if paths.contains(&path) {
                log_debug!("{func}: duplicate path: {}", path.to_string_lossy());
                // free_(path);
                continue;
            }

            paths.push(path);
        }
        free_(copy);
    }
}

unsafe fn make_label(mut label: *const u8, cause: *mut *mut u8) -> *const u8 {
    let mut paths: Vec<CString> = Vec::new();
    let base: *mut u8;
    let mut sb: stat = unsafe { zeroed() }; // TODO use uninit

    unsafe {
        'fail: {
            *cause = null_mut();
            if label.is_null() {
                label = c!("default");
            }
            let uid = getuid();

            expand_paths(TMUX_SOCK, &mut paths, 1);
            if paths.is_empty() {
                *cause = format_nul!("no suitable socket path");
                return null_mut();
            }

            paths.truncate(1);
            let mut path = paths.pop().unwrap(); /* can only have one socket! */

            base = format_nul!("{}/tmux-rs-{}", path.to_string_lossy(), uid);
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
            path = CString::new(format!("{}/{}", _s(base), _s(label))).unwrap();
            free_(base);
            return path.into_raw().cast();
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
        // We want a timestamp in milliseconds suitable for time measurement,
        // so prefer the monotonic clock.
        if clock_gettime(CLOCK_MONOTONIC, &raw mut ts) != 0 {
            clock_gettime(CLOCK_REALTIME, &raw mut ts);
        }
        (ts.tv_sec as u64 * 1000) + (ts.tv_nsec as u64 / 1000000)
    }
}

pub fn find_cwd() -> Option<PathBuf> {
    let cwd = std::env::current_dir().ok()?;

    let pwd = match std::env::var("PWD") {
        Ok(val) if !val.is_empty() => PathBuf::from(val),
        _ => return Some(cwd),
    };

    // We want to use PWD so that symbolic links are maintained,
    // but only if it matches the actual working directory.

    let Ok(resolved1) = pwd.canonicalize() else {
        return Some(cwd);
    };

    let Ok(resolved2) = cwd.canonicalize() else {
        return Some(cwd);
    };

    if resolved1 == resolved2 {
        return Some(cwd);
    }

    Some(pwd)
}

pub fn find_home() -> Option<&'static CStr> {
    unsafe {
        static HOME: OnceLock<Option<CString>> = OnceLock::new();
        HOME.get_or_init(|| match std::env::var("HOME") {
            Ok(home) if !home.is_empty() => Some(CString::new(home).unwrap()),
            _ => NonNull::new(getpwuid(getuid()))
                .map(|pw| CString::new(cstr_to_str((*pw.as_ptr()).pw_dir.cast())).unwrap()),
        })
        .as_deref()
    }
}

pub fn getversion() -> &'static str {
    crate::TMUX_VERSION
}

/// Entrypoint for tmux binary
///
/// # Safety
///
/// This code is work in progress. There is no guarantee that the code is safe.
/// This function should only be called by the tmux binary crate to start tmux.
pub unsafe fn tmux_main(mut argc: i32, mut argv: *mut *mut u8, _env: *mut *mut u8) {
    std::panic::set_hook(Box::new(|_panic_info| {
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
            environ_put(GLOBAL_ENVIRON, *var, environ_flags::empty());
            var = var.add(1);
        }

        if let Some(cwd) = find_cwd() {
            environ_set!(
                GLOBAL_ENVIRON,
                c!("PWD"),
                environ_flags::empty(),
                "{}",
                cwd.to_str().unwrap()
            );
        }
        expand_paths(TMUX_CONF, &mut CFG_FILES.lock().unwrap(), 1);

        while let Some(opt) = getopt(argc, argv.cast(), c!("2c:CDdf:lL:NqS:T:uUvV")) {
            match opt {
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
                        CFG_FILES.lock().unwrap().clear();
                    }
                    CFG_FILES
                        .lock()
                        .unwrap()
                        .push(CString::new(cstr_to_str(OPTARG)).unwrap());
                    CFG_QUIET.store(false, atomic::Ordering::Relaxed);
                }
                b'V' => {
                    println!("tmux-rs {}", getversion());
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
        if std::env::var("TMUX").is_ok() {
            flags |= client_flag::UTF8;
        } else {
            let s = std::env::var("LC_ALL")
                .or_else(|_| std::env::var("LC_CTYPE"))
                .or_else(|_| std::env::var("LANG"))
                .unwrap_or_default()
                .to_ascii_lowercase();

            if s.contains("utf-8") || s.contains("utf8") {
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
            false,
            "{}",
            getshell().to_string_lossy(),
        );

        // Override keys to vi if VISUAL or EDITOR are set.
        if let Ok(s) = std::env::var("VISUAL").or_else(|_| std::env::var("EDITOR")) {
            options_set_string!(GLOBAL_OPTIONS, c!("editor"), false, "{s}");

            let s = if let Some(slash_end) = s.rfind('/') {
                &s[slash_end + 1..]
            } else {
                &s
            };

            let keys = if s.contains("vi") {
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
        if path.is_null()
            && label.is_null()
            && let Ok(s) = std::env::var("TMUX")
            && !s.is_empty()
            && s.as_bytes()[0] != b','
        {
            let tmp: *mut u8 = xstrdup__(&s);
            *tmp.add(strcspn(tmp, c!(","))) = b'\0';
            path = tmp;
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

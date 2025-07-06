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

use crate::libc::{
    _IOLBF, AF_UNIX, CREAD, CS8, EAGAIN, ECHILD, ECONNREFUSED, EINTR, ENAMETOOLONG, ENOENT, HUPCL,
    ICRNL, IXANY, LOCK_EX, LOCK_NB, O_CREAT, O_WRONLY, ONLCR, OPOST, SA_RESTART, SIG_DFL, SIG_IGN,
    SIGCHLD, SIGCONT, SIGHUP, SIGTERM, SIGTSTP, SIGWINCH, SOCK_STREAM, STDERR_FILENO, STDIN_FILENO,
    STDOUT_FILENO, TCSAFLUSH, TCSANOW, VMIN, VTIME, WNOHANG, cfgetispeed, cfgetospeed, cfmakeraw,
    cfsetispeed, cfsetospeed, close, connect, dup, execl, fflush, flock, fprintf, getenv, getline,
    getppid, isatty, kill, memcpy, memset, open, printf, setenv, setvbuf, sigaction, sigemptyset,
    sockaddr, sockaddr_un, socket, strerror, strlen, strsignal, system, tcgetattr, tcsetattr,
    unlink, waitpid,
};

use crate::compat::{
    WAIT_ANY, closefrom,
    imsg::{IMSG_HEADER_SIZE, MAX_IMSGSIZE, imsg},
    strlcpy,
    tree::rb_initializer,
};

unsafe extern "C" {
    fn ttyname(fd: i32) -> *mut u8;
}

pub static mut client_proc: *mut tmuxproc = null_mut();

pub static mut client_peer: *mut tmuxpeer = null_mut();

pub static mut client_flags: client_flag = client_flag::empty();

pub static mut client_suspended: i32 = 0;

#[repr(i32)]
#[derive(Copy, Clone, Eq, PartialEq)]
pub enum client_exitreason {
    CLIENT_EXIT_NONE,
    CLIENT_EXIT_DETACHED,
    CLIENT_EXIT_DETACHED_HUP,
    CLIENT_EXIT_LOST_TTY,
    CLIENT_EXIT_TERMINATED,
    CLIENT_EXIT_LOST_SERVER,
    CLIENT_EXIT_EXITED,
    CLIENT_EXIT_SERVER_EXITED,
    CLIENT_EXIT_MESSAGE_PROVIDED,
}

pub static mut client_exitreason: client_exitreason = client_exitreason::CLIENT_EXIT_NONE;

pub static mut client_exitflag: i32 = 0;

pub static mut client_exitval: i32 = 0;

static mut client_exittype: msgtype = msgtype::MSG_ZERO; // TODO

static mut client_exitsession: *mut u8 = null_mut();

static mut client_exitmessage: *mut u8 = null_mut();

static mut client_execshell: *mut u8 = null_mut();

static mut client_execcmd: *mut u8 = null_mut();

static mut client_attached: i32 = 0;

static mut client_files: client_files = rb_initializer();

pub unsafe fn client_get_lock(lockfile: *mut u8) -> i32 {
    unsafe {
        log_debug!("lock file is {}", _s(lockfile));

        let lockfd = open(lockfile, O_WRONLY | O_CREAT, 0o600);
        if lockfd == -1 {
            log_debug!("open failed: {}", _s(strerror(errno!())));
            return -1;
        }

        if flock(lockfd, LOCK_EX | LOCK_NB) == -1 {
            log_debug!("flock failed: {}", _s(strerror(errno!())));
            if errno!() != EAGAIN {
                return lockfd;
            }
            while flock(lockfd, LOCK_EX) == -1 && errno!() == EINTR {}
            close(lockfd);
            return -2;
        }
        log_debug!("flock succeeded");

        lockfd
    }
}

pub unsafe fn client_connect(base: *mut event_base, path: *const u8, flags: client_flag) -> i32 {
    unsafe {
        let mut sa: sockaddr_un = zeroed();
        let mut fd = 0;
        let mut lockfd = -1;
        let mut locked: i32 = 0;
        let mut lockfile: *mut u8 = null_mut();

        sa.sun_family = AF_UNIX as libc::sa_family_t;
        let size = strlcpy(&raw mut sa.sun_path as _, path, size_of_val(&sa.sun_path));
        if size >= size_of_val(&sa.sun_path) {
            errno!() = ENAMETOOLONG;
            return -1;
        }
        log_debug!("socket is {}", _s(path));

        'failed: {
            'retry: loop {
                fd = socket(AF_UNIX, SOCK_STREAM, 0);
                if fd == -1 {
                    return -1;
                }

                log_debug!("trying connect");
                if connect(
                    fd,
                    &raw const sa as *const sockaddr,
                    size_of::<sockaddr_un>() as u32,
                ) == -1
                {
                    log_debug!("connect failed: {}", _s(strerror(errno!())));
                    if errno!() != ECONNREFUSED && errno!() != ENOENT {
                        break 'failed;
                    }
                    if flags.intersects(client_flag::NOSTARTSERVER) {
                        break 'failed;
                    }
                    if !flags.intersects(client_flag::STARTSERVER) {
                        break 'failed;
                    }
                    close(fd);

                    if locked == 0 {
                        lockfile = format_nul!("{}.lock", _s(path));
                        lockfd = client_get_lock(lockfile);
                        if lockfd < 0 {
                            log_debug!("didn't get lock ({})", lockfd);

                            free_(lockfile);
                            lockfile = null_mut();

                            if lockfd == -2 {
                                continue 'retry;
                            }
                        }
                        log_debug!("got lock ({})", lockfd);

                        locked = 1;
                        continue 'retry;
                    }

                    if lockfd >= 0 && unlink(path) != 0 && errno!() != ENOENT {
                        free_(lockfile);
                        close(lockfd);
                        return -1;
                    }
                    fd = server_start(client_proc, flags, base, lockfd, lockfile);
                }

                break 'retry;
            }

            if locked != 0 && lockfd >= 0 {
                free_(lockfile);
                close(lockfd);
            }
            setblocking(fd, 0);
            return fd;
        }

        // failed:
        if locked != 0 {
            free_(lockfile as _);
            close(lockfd);
        }
        close(fd);
        -1
    }
}

pub unsafe fn client_exit_message() -> *const u8 {
    type msgbuf = [u8; 256];
    static mut msg: msgbuf = [0; 256];

    match unsafe { client_exitreason } {
        client_exitreason::CLIENT_EXIT_DETACHED => {
            unsafe {
                if !client_exitsession.is_null() {
                    xsnprintf_!(
                        &raw mut msg as _,
                        size_of::<msgbuf>(),
                        "detached (from session {})",
                        _s(client_exitsession),
                    );
                    return &raw mut msg as _;
                }
            }
            c!("detached")
        }
        client_exitreason::CLIENT_EXIT_DETACHED_HUP => {
            unsafe {
                if !client_exitsession.is_null() {
                    let tmp = client_exitsession;
                    xsnprintf_!(
                        &raw mut msg as _,
                        size_of::<msgbuf>(),
                        "detached and SIGHUP (from session {})",
                        _s(tmp),
                    );
                    return &raw mut msg as _;
                }
            }
            c!("detached and SIGHUP")
        }
        client_exitreason::CLIENT_EXIT_LOST_TTY => c!("lost tty"),
        client_exitreason::CLIENT_EXIT_TERMINATED => c!("terminated"),
        client_exitreason::CLIENT_EXIT_LOST_SERVER => c!("server exited unexpectedly"),
        client_exitreason::CLIENT_EXIT_EXITED => c!("exited"),
        client_exitreason::CLIENT_EXIT_SERVER_EXITED => c!("server exited"),
        client_exitreason::CLIENT_EXIT_MESSAGE_PROVIDED => unsafe { client_exitmessage },
        client_exitreason::CLIENT_EXIT_NONE => c!("unknown reason"),
    }
}

unsafe fn client_exit() {
    unsafe {
        if file_write_left(&raw mut client_files) == 0 {
            proc_exit(client_proc);
        }
    }
}

#[expect(clippy::deref_addrof)]
pub unsafe extern "C-unwind" fn client_main(
    base: *mut event_base,
    argc: i32,
    argv: *mut *mut u8,
    mut flags: client_flag,
    feat: i32,
) -> i32 {
    unsafe {
        let mut data: *mut msg_command = null_mut();
        let mut fd = 0;
        let mut cwd: *const u8 = null_mut();
        let mut ttynam: *const u8 = null_mut();
        let mut termname: *const u8 = null_mut();
        let msg: msgtype;
        let mut tio: termios = zeroed();
        let mut saved_tio: termios = zeroed();
        let mut linesize = 0;
        let mut line: *mut u8 = null_mut();
        let mut caps: *mut *mut u8 = null_mut();
        let mut cause: *mut u8 = null_mut();
        let mut ncaps: u32 = 0;
        let mut values: *mut args_value = null_mut();

        if !shell_command.is_null() {
            msg = msgtype::MSG_SHELL;
            flags |= client_flag::STARTSERVER;
        } else if argc == 0 {
            msg = msgtype::MSG_COMMAND;
            flags |= client_flag::STARTSERVER;
        } else {
            msg = msgtype::MSG_COMMAND;

            values = args_from_vector(argc, argv);
            match cmd_parse_from_arguments(values, argc as u32, None) {
                Ok(cmdlist) => {
                    if cmd_list_any_have(cmdlist, cmd_flag::CMD_STARTSERVER) {
                        flags |= client_flag::STARTSERVER;
                    }
                    cmd_list_free(cmdlist);
                }
                Err(error) => {
                    free_(error);
                }
            }
            args_free_values(values, argc as u32);
            free_(values);
        }

        client_proc = proc_start(c"client");
        proc_set_signals(client_proc, Some(client_signal));

        client_flags = flags;
        log_debug!(
            "flags are {:#x}",
            (*&raw mut client_flags).bits() as c_ulonglong
        );

        // #ifdef HAVE_SYSTEMD
        #[cfg(feature = "systemd")]
        {
            unsafe extern "C" {
                fn systemd_activated() -> i32;
            }
            if systemd_activated() != 0 {
                fd = server_start(client_proc, flags, base, 0, null_mut());
            } else {
                fd = client_connect(base, socket_path, client_flags);
            }
        }
        #[cfg(not(feature = "systemd"))]
        {
            fd = client_connect(base, socket_path, client_flags);
        }
        if fd == -1 {
            if errno!() == ECONNREFUSED {
                fprintf(stderr, c"no server running on %s\n".as_ptr(), socket_path);
            } else {
                fprintf(
                    stderr,
                    c"error connecting to %s (%s)\n".as_ptr(),
                    socket_path,
                    strerror(errno!()),
                );
            }
            return 1;
        }
        client_peer = proc_add_peer(client_proc, fd, Some(client_dispatch), null_mut());

        cwd = find_cwd();
        if cwd.is_null()
            && ({
                cwd = find_home();
                cwd.is_null()
            })
        {
            cwd = c!("/");
        }
        ttynam = ttyname(STDIN_FILENO);
        if ttynam.is_null() {
            ttynam = c!("");
        }
        termname = getenv(c!("TERM"));
        if termname.is_null() {
            termname = c!("");
        }

        /*
            // TODO no pledge
            if pledge( c!("stdio rpath wpath cpath unix sendfd proc exec tty"), null_mut()) != 0 {
                fatal( c!("pledge failed"));
            }
        */

        if isatty(STDIN_FILENO) != 0
            && *termname != b'\0'
            && tty_term_read_list(
                termname,
                STDIN_FILENO,
                &raw mut caps,
                &raw mut ncaps,
                &raw mut cause,
            ) != 0
        {
            fprintf(stderr, c"%s\n".as_ptr(), cause);
            free(cause as _);
            return 1;
        }

        if ptm_fd != -1 {
            close(ptm_fd);
        }
        options_free(global_options);
        options_free(global_s_options);
        options_free(global_w_options);
        environ_free(global_environ);

        if (*&raw const client_flags).intersects(client_flag::CONTROLCONTROL) {
            if tcgetattr(STDIN_FILENO, &raw mut saved_tio) != 0 {
                fprintf(
                    stderr,
                    c"tcgetattr failed: %s\n".as_ptr(),
                    strerror(errno!()),
                );
                return 1;
            }
            cfmakeraw(&raw mut tio);
            tio.c_iflag = ICRNL | IXANY;
            tio.c_oflag = OPOST | ONLCR;
            #[cfg(feature = "nokerninfo")]
            {
                // tio.c_lflag = NOKERNINFO;
            }
            tio.c_cflag = CREAD | CS8 | HUPCL;
            tio.c_cc[VMIN] = 1;
            tio.c_cc[VTIME] = 0;
            cfsetispeed(&raw mut tio, cfgetispeed(&raw mut saved_tio));
            cfsetospeed(&raw mut tio, cfgetospeed(&raw mut saved_tio));
            tcsetattr(STDIN_FILENO, TCSANOW, &tio);
        }

        client_send_identify(ttynam, termname, caps, ncaps, cwd, feat);
        tty_term_free_list(caps, ncaps);
        proc_flush_peer(client_peer);

        if msg == msgtype::MSG_COMMAND {
            let mut size = 0;
            for i in 0..argc {
                size += strlen(*argv.add(i as _)) + 1;
            }
            if size > MAX_IMSGSIZE - size_of::<msg_command>() {
                fprintf(stderr, c"command too long\n".as_ptr());
                return 1;
            }
            data = xmalloc(size_of::<msg_command>() + size).cast().as_ptr();

            (*data).argc = argc;
            // TODO this cast seems fishy
            if cmd_pack_argv(argc, argv, data.add(1).cast(), size) != 0 {
                fprintf(stderr, c"command too long\n".as_ptr());
                free_(data);
                return 1;
            }
            size += size_of::<msg_command>();

            if proc_send(client_peer, msg, -1, data as _, size) != 0 {
                fprintf(stderr, c"failed to send command\n".as_ptr());
                free_(data);
                return 1;
            }
            free_(data);
        } else if msg == msgtype::MSG_SHELL {
            proc_send(client_peer, msg, -1, null_mut(), 0);
        }

        proc_loop(client_proc, None);

        if client_exittype == msgtype::MSG_EXEC {
            if (*&raw const client_flags).intersects(client_flag::CONTROLCONTROL) {
                tcsetattr(STDOUT_FILENO, TCSAFLUSH, &saved_tio);
            }
            client_exec(client_execshell, client_execcmd);
        }

        setblocking(STDIN_FILENO, 1);
        setblocking(STDOUT_FILENO, 1);
        setblocking(STDERR_FILENO, 1);

        if client_attached != 0 {
            if client_exitreason != client_exitreason::CLIENT_EXIT_NONE {
                printf(c"[%s]\n".as_ptr(), client_exit_message());
            }

            let ppid = getppid();
            if client_exittype == msgtype::MSG_DETACHKILL && ppid > 1 {
                kill(ppid, SIGHUP);
            }
        } else if (*&raw const client_flags).intersects(client_flag::CONTROL) {
            if client_exitreason != client_exitreason::CLIENT_EXIT_NONE {
                printf(c"%%exit %s\n".as_ptr(), client_exit_message());
            } else {
                printf(c"%%exit\n".as_ptr());
            }
            fflush(stdout);
            if (*&raw const client_flags).intersects(client_flag::CONTROL_WAITEXIT) {
                setvbuf(stdin, null_mut(), _IOLBF, 0);
                loop {
                    let linelen = getline(&raw mut line as _, &raw mut linesize, stdin);
                    if linelen <= 1 {
                        break;
                    }
                }
                free(line as _);
            }
            if (*&raw const client_flags).intersects(client_flag::CONTROLCONTROL) {
                // TODO originally octal 033
                printf(c"\x1b\\".as_ptr());
                fflush(stdout);
                tcsetattr(STDOUT_FILENO, TCSAFLUSH, &raw mut saved_tio);
            }
        } else if client_exitreason != client_exitreason::CLIENT_EXIT_NONE {
            fprintf(stderr, c"%s\n".as_ptr(), client_exit_message());
        }

        client_exitval
    }
}

unsafe fn client_send_identify(
    ttynam: *const u8,
    termname: *const u8,
    caps: *mut *mut u8,
    ncaps: u32,
    cwd: *const u8,
    mut feat: i32,
) {
    unsafe {
        // char	**ss;
        let sslen: usize = 0;
        // int	  fd;
        let mut flags: client_flag = client_flags;
        // pid_t	  pid;
        // u_int	  i;

        proc_send(
            client_peer,
            msgtype::MSG_IDENTIFY_LONGFLAGS,
            -1,
            &raw mut flags as _,
            size_of::<u64>(),
        );
        proc_send(
            client_peer,
            msgtype::MSG_IDENTIFY_LONGFLAGS,
            -1,
            &raw mut client_flags as _,
            size_of::<u64>(),
        );

        proc_send(
            client_peer,
            msgtype::MSG_IDENTIFY_TERM,
            -1,
            termname as _,
            strlen(termname) + 1,
        );
        proc_send(
            client_peer,
            msgtype::MSG_IDENTIFY_FEATURES,
            -1,
            &raw mut feat as _,
            size_of::<i32>(),
        );

        proc_send(
            client_peer,
            msgtype::MSG_IDENTIFY_TTYNAME,
            -1,
            ttynam as _,
            strlen(ttynam) + 1,
        );
        proc_send(
            client_peer,
            msgtype::MSG_IDENTIFY_CWD,
            -1,
            cwd as _,
            strlen(cwd) + 1,
        );

        for i in 0..ncaps {
            proc_send(
                client_peer,
                msgtype::MSG_IDENTIFY_TERMINFO,
                -1,
                *caps.add(i as usize) as _,
                strlen(*caps.add(i as usize)) + 1,
            );
        }

        let fd = dup(STDIN_FILENO);
        if fd == -1 {
            fatal("dup failed");
        }
        proc_send(client_peer, msgtype::MSG_IDENTIFY_STDIN, fd, null_mut(), 0);

        let fd = dup(STDOUT_FILENO);
        if fd == -1 {
            fatal("dup failed");
        }
        proc_send(client_peer, msgtype::MSG_IDENTIFY_STDOUT, fd, null_mut(), 0);

        let mut pid = std::process::id() as i32;
        proc_send(
            client_peer,
            msgtype::MSG_IDENTIFY_CLIENTPID,
            -1,
            &raw mut pid as _,
            size_of::<i32>(),
        );

        let mut ss = environ;
        while !(*ss).is_null() {
            let sslen = strlen(*ss) + 1;
            if sslen > MAX_IMSGSIZE - IMSG_HEADER_SIZE {
                ss = ss.add(1);
                continue;
            }
            proc_send(
                client_peer,
                msgtype::MSG_IDENTIFY_ENVIRON,
                -1,
                *ss as _,
                sslen,
            );
            ss = ss.add(1);
        }

        proc_send(client_peer, msgtype::MSG_IDENTIFY_DONE, -1, null_mut(), 0);
    }
}

#[expect(clippy::deref_addrof)]
unsafe fn client_exec(shell: *mut u8, shellcmd: *mut u8) {
    unsafe {
        log_debug!("shell {}, command {}", _s(shell), _s(shellcmd));
        let argv0 = shell_argv0(
            shell,
            (*&raw const client_flags).intersects(client_flag::LOGIN) as c_int,
        );
        setenv(c!("SHELL"), shell, 1);

        proc_clear_signals(client_proc, 1);

        setblocking(STDIN_FILENO, 1);
        setblocking(STDOUT_FILENO, 1);
        setblocking(STDERR_FILENO, 1);
        closefrom(STDERR_FILENO + 1);

        execl(
            shell.cast(),
            argv0.cast(),
            c"-c".as_ptr(),
            shellcmd,
            null_mut::<c_void>(),
        );
        fatal("execl failed");
    }
}

unsafe fn client_signal(sig: i32) {
    unsafe {
        let mut sigact: sigaction = zeroed();
        let mut status: i32 = 0;
        let mut pid: pid_t = 0;

        log_debug!("{}: {}", "client_signal", _s(strsignal(sig).cast::<u8>()));
        if sig == SIGCHLD {
            loop {
                pid = waitpid(WAIT_ANY, &raw mut status, WNOHANG);
                if pid == 0 {
                    break;
                }
                if pid == -1 {
                    if errno!() == ECHILD {
                        break;
                    }
                    log_debug!("waitpid failed: {}", _s(strerror(errno!())));
                }
            }
        } else if client_attached == 0 {
            if sig == SIGTERM || sig == SIGHUP {
                proc_exit(client_proc);
            }
        } else {
            match sig {
                SIGHUP => {
                    client_exitreason = client_exitreason::CLIENT_EXIT_LOST_TTY;
                    client_exitval = 1;
                    proc_send(client_peer, msgtype::MSG_EXITING, -1, null_mut(), 0);
                }
                SIGTERM => {
                    if client_suspended == 0 {
                        client_exitreason = client_exitreason::CLIENT_EXIT_TERMINATED;
                    }
                    client_exitval = 1;
                    proc_send(client_peer, msgtype::MSG_EXITING, -1, null_mut(), 0);
                }
                SIGWINCH => {
                    proc_send(client_peer, msgtype::MSG_RESIZE, -1, null_mut(), 0);
                }
                SIGCONT => {
                    memset(&raw mut sigact as _, 0, size_of::<sigaction>());
                    sigemptyset(&raw mut sigact.sa_mask);
                    sigact.sa_flags = SA_RESTART;
                    sigact.sa_sigaction = SIG_IGN;
                    if sigaction(SIGTSTP, &raw mut sigact, null_mut()) != 0 {
                        fatal("sigaction failed");
                    }
                    proc_send(client_peer, msgtype::MSG_WAKEUP, -1, null_mut(), 0);
                    client_suspended = 0;
                }
                _ => (),
            }
        }
    }
}

unsafe fn client_file_check_cb(
    _c: *mut client,
    _path: *mut u8,
    _error: i32,
    _closed: i32,
    _buffer: *mut evbuffer,
    data: *mut c_void,
) {
    unsafe {
        if client_exitflag != 0 {
            client_exit();
        }
    }
}

unsafe fn client_dispatch(imsg: *mut imsg, _arg: *mut c_void) {
    unsafe {
        if imsg.is_null() {
            if client_exitflag == 0 {
                client_exitreason = client_exitreason::CLIENT_EXIT_LOST_SERVER;
                client_exitval = 1;
            }
            proc_exit(client_proc);
            return;
        }

        if client_attached != 0 {
            client_dispatch_attached(imsg);
        } else {
            client_dispatch_wait(imsg);
        }
    }
}

unsafe fn client_dispatch_exit_message(mut data: *const u8, mut datalen: usize) {
    unsafe {
        let mut retval = 0;
        const size_of_retval: usize = size_of::<i32>();

        if datalen < size_of_retval && datalen != 0 {
            fatalx("bad MSG_EXIT size");
        }

        if datalen >= size_of_retval {
            memcpy(&raw mut retval as _, data as _, size_of_retval);
            client_exitval = retval;
        }

        if datalen > size_of_retval {
            datalen -= size_of_retval;
            data = data.add(size_of_retval);

            client_exitmessage = xmalloc(datalen).cast().as_ptr();
            memcpy(client_exitmessage as _, data as _, datalen);
            *client_exitmessage.add(datalen - 1) = b'\0';

            client_exitreason = client_exitreason::CLIENT_EXIT_MESSAGE_PROVIDED;
        }
    }
}

#[expect(clippy::deref_addrof)]
unsafe fn client_dispatch_wait(imsg: *mut imsg) {
    static mut pledge_applied: i32 = 0;

    unsafe {
        /*
        // TODO no pledge
        if pledge_applied == 0 {
            if pledge("stdio rpath wpath cpath unix proc exec tty", null_mut()) != 0 {
                fatal( c!("pledge failed"));
            }
            pledge_applied = 1;
        }
        */

        let data: *mut u8 = (*imsg).data as _;
        let datalen = (*imsg).hdr.len as usize - IMSG_HEADER_SIZE;

        let msg_hdr_type: msgtype = (*imsg).hdr.type_.try_into().expect("invalid enum variant"); // TODO
        match msg_hdr_type {
            msgtype::MSG_EXIT | msgtype::MSG_SHUTDOWN => {
                client_dispatch_exit_message(data, datalen);
                client_exitflag = 1;
                client_exit();
            }
            msgtype::MSG_READY => {
                if datalen != 0 {
                    fatalx("bad MSG_READY size");
                }

                client_attached = 1;
                proc_send(client_peer, msgtype::MSG_RESIZE, -1, null_mut(), 0);
            }
            msgtype::MSG_VERSION => {
                if datalen != 0 {
                    fatalx("bad MSG_VERSION size");
                }

                fprintf(
                    stderr,
                    c"protocol version mismatch (client %d, server %u)\n".as_ptr(),
                    PROTOCOL_VERSION,
                    (*imsg).hdr.peerid & 0xff,
                );
                client_exitval = 1;
                proc_exit(client_proc);
            }
            msgtype::MSG_FLAGS => {
                if datalen != size_of::<u64>() {
                    fatalx("bad MSG_FLAGS string");
                }

                memcpy(
                    &raw mut client_flags as *mut c_void,
                    data as *const c_void,
                    size_of::<u64>(),
                );
                log_debug!(
                    "new flags are {:#x}",
                    (*&raw const client_flags).bits() as c_ulonglong
                );
            }
            msgtype::MSG_SHELL => {
                if datalen == 0 || *data.add(datalen - 1) != b'\0' {
                    fatalx("bad MSG_SHELL string");
                }

                client_exec(data, shell_command);
            }
            msgtype::MSG_DETACH | msgtype::MSG_DETACHKILL => {
                proc_send(client_peer, msgtype::MSG_EXITING, -1, null_mut(), 0);
            }
            msgtype::MSG_EXITED => proc_exit(client_proc),
            msgtype::MSG_READ_OPEN => {
                file_read_open(
                    &raw mut client_files,
                    client_peer,
                    imsg,
                    1,
                    !(*&raw const client_flags).intersects(client_flag::CONTROL) as i32,
                    Some(client_file_check_cb),
                    null_mut(),
                );
            }
            msgtype::MSG_READ_CANCEL => file_read_cancel(&raw mut client_files, imsg),
            msgtype::MSG_WRITE_OPEN => {
                file_write_open(
                    &raw mut client_files,
                    client_peer,
                    imsg,
                    1,
                    !(*&raw const client_flags).intersects(client_flag::CONTROL) as i32,
                    Some(client_file_check_cb),
                    null_mut(),
                );
            }
            msgtype::MSG_WRITE => file_write_data(&raw mut client_files, imsg),
            msgtype::MSG_WRITE_CLOSE => file_write_close(&raw mut client_files, imsg),
            msgtype::MSG_OLDSTDERR | msgtype::MSG_OLDSTDIN | msgtype::MSG_OLDSTDOUT => {
                fprintf(stderr, c"server version is too old for client\n".as_ptr());
                proc_exit(client_proc);
            }
            _ => (), // TODO
        }
    }
}

#[expect(clippy::deref_addrof)]
unsafe fn client_dispatch_attached(imsg: *mut imsg) {
    unsafe {
        let mut sigact: sigaction = zeroed();
        let data: *mut u8 = (*imsg).data as _;
        let datalen = (*imsg).hdr.len as usize - IMSG_HEADER_SIZE;

        let mht: msgtype = (*imsg).hdr.type_.try_into().expect("invalid enum variant"); // TODO
        match mht {
            msgtype::MSG_FLAGS => {
                if datalen != size_of::<u64>() {
                    // TODO use size_of_val_raw
                    fatalx("bad MSG_FLAGS string");
                }

                memcpy(
                    &raw mut client_flags as *mut c_void,
                    data as *const c_void,
                    size_of::<u64>(),
                );
                log_debug!(
                    "new flags are {:#x}",
                    (*&raw const client_flags).bits() as c_ulonglong
                );
            }
            msgtype::MSG_DETACH | msgtype::MSG_DETACHKILL => {
                if datalen == 0 || *data.add(datalen - 1) != b'\0' {
                    fatalx("bad MSG_DETACH string");
                }

                client_exitsession = xstrdup(data).as_ptr();
                client_exittype = mht;
                if (*imsg).hdr.type_ == msgtype::MSG_DETACHKILL as u32 {
                    client_exitreason = client_exitreason::CLIENT_EXIT_DETACHED_HUP;
                } else {
                    client_exitreason = client_exitreason::CLIENT_EXIT_DETACHED;
                }
                proc_send(client_peer, msgtype::MSG_EXITING, -1, null_mut(), 0);
            }
            msgtype::MSG_EXEC => {
                if datalen == 0
                    || *data.add(datalen - 1) != b'\0'
                    || strlen(data) + 1 == datalen
                {
                    fatalx("bad MSG_EXEC string");
                }
                client_execcmd = xstrdup(data).as_ptr();
                client_execshell = xstrdup(data.add(strlen(data) + 1)).as_ptr();

                client_exittype = mht;
                proc_send(client_peer, msgtype::MSG_EXITING, -1, null_mut(), 0);
            }
            msgtype::MSG_EXIT => {
                client_dispatch_exit_message(data, datalen);
                if client_exitreason == client_exitreason::CLIENT_EXIT_NONE {
                    client_exitreason = client_exitreason::CLIENT_EXIT_EXITED;
                }
                proc_send(client_peer, msgtype::MSG_EXITING, -1, null_mut(), 0);
            }
            msgtype::MSG_EXITED => {
                if datalen != 0 {
                    fatalx("bad MSG_EXITED size");
                }

                proc_exit(client_proc);
            }
            msgtype::MSG_SHUTDOWN => {
                if datalen != 0 {
                    fatalx("bad MSG_SHUTDOWN size");
                }

                proc_send(client_peer, msgtype::MSG_EXITING, -1, null_mut(), 0);
                client_exitreason = client_exitreason::CLIENT_EXIT_SERVER_EXITED;
                client_exitval = 1;
            }
            msgtype::MSG_SUSPEND => {
                if datalen != 0 {
                    fatalx("bad MSG_SUSPEND size");
                }

                memset(&raw mut sigact as _, 0, size_of::<sigaction>());
                sigemptyset(&raw mut sigact.sa_mask);
                sigact.sa_flags = SA_RESTART;
                sigact.sa_sigaction = SIG_DFL;
                if sigaction(SIGTSTP, &raw mut sigact, null_mut()) != 0 {
                    fatal("sigaction failed");
                }
                client_suspended = 1;
                kill(std::process::id() as i32, SIGTSTP);
            }
            msgtype::MSG_LOCK => {
                if datalen == 0 || *data.add(datalen - 1) != b'\0' {
                    fatalx("bad MSG_LOCK string");
                }

                system(data.cast());
                proc_send(client_peer, msgtype::MSG_UNLOCK, -1, null_mut(), 0);
            }
            _ => (),
        }
    }
}

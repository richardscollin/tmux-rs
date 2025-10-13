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
use crate::compat::ACCESSPERMS;
use crate::libc::{
    AF_UNIX, ECHILD, ENAMETOOLONG, S_IRGRP, S_IROTH, S_IRUSR, S_IRWXG, S_IRWXO, S_IXGRP, S_IXOTH,
    S_IXUSR, SIG_BLOCK, SIG_SETMASK, SIGCONT, SIGTTIN, SIGTTOU, SOCK_STREAM, WIFEXITED,
    WIFSIGNALED, WIFSTOPPED, WNOHANG, WSTOPSIG, WUNTRACED, accept, bind, chmod, close,
    gettimeofday, kill, killpg, listen, sigfillset, sigprocmask, sigset_t, sockaddr_storage,
    sockaddr_un, socket, socklen_t, stat, strerror, strsignal, umask, unlink, waitpid,
};
use crate::*;

pub static mut CLIENTS: clients = unsafe { zeroed() };
pub static mut SERVER_PROC: *mut tmuxproc = null_mut();
pub static mut SERVER_FD: c_int = -1;
pub static mut SERVER_CLIENT_FLAGS: client_flag = client_flag::empty();
pub static mut SERVER_EXIT: c_int = 0;
pub static mut SERVER_EV_ACCEPT: event = unsafe { zeroed() };
pub static mut SERVER_EV_TIDY: event = unsafe { zeroed() };
pub static mut MARKED_PANE: cmd_find_state = unsafe { zeroed() };
pub static mut MESSAGE_NEXT: c_uint = 0;
pub static mut MESSAGE_LOG: message_list = unsafe { zeroed() };
pub static mut CURRENT_TIME: time_t = unsafe { zeroed() };

pub unsafe fn server_set_marked(s: *mut session, wl: *mut winlink, wp: *mut window_pane) {
    unsafe {
        cmd_find_clear_state(&raw mut MARKED_PANE, cmd_find_flags::empty());
        MARKED_PANE.s = s;
        MARKED_PANE.wl = wl;
        MARKED_PANE.w = (*wl).window;
        MARKED_PANE.wp = wp;
    }
}

pub unsafe fn server_clear_marked() {
    unsafe {
        cmd_find_clear_state(&raw mut MARKED_PANE, cmd_find_flags::empty());
    }
}

pub unsafe fn server_is_marked(s: *mut session, wl: *mut winlink, wp: *mut window_pane) -> bool {
    if s.is_null() || wl.is_null() || wp.is_null() {
        return false;
    }

    unsafe {
        if MARKED_PANE.s != s || MARKED_PANE.wl != wl {
            return false;
        }
        if MARKED_PANE.wp != wp {
            return false;
        }
        server_check_marked()
    }
}

pub unsafe fn server_check_marked() -> bool {
    unsafe { cmd_find_valid_state(&raw mut MARKED_PANE) }
}

pub unsafe fn server_create_socket(flags: client_flag, cause: *mut *mut u8) -> c_int {
    unsafe {
        'fail: {
            let mut sa: sockaddr_un = zeroed();
            sa.sun_family = AF_UNIX as _;
            let size = strlcpy(
                sa.sun_path.as_mut_ptr().cast(),
                SOCKET_PATH,
                size_of_val(&sa.sun_path),
            );
            if size >= size_of_val(&sa.sun_path) {
                errno!() = ENAMETOOLONG;
                break 'fail;
            }
            unlink(sa.sun_path.as_ptr().cast());

            let fd = socket(AF_UNIX, SOCK_STREAM, 0);
            if fd == -1 {
                break 'fail;
            }

            let mask = if flags.intersects(client_flag::DEFAULTSOCKET) {
                umask(S_IXUSR | S_IXGRP | S_IRWXO)
            } else {
                umask(S_IXUSR | S_IRWXG | S_IRWXO)
            };

            let saved_errno: c_int;
            if bind(fd, &raw const sa as _, size_of::<sockaddr_un>() as _) == -1 {
                saved_errno = errno!();
                close(fd);
                errno!() = saved_errno;
                break 'fail;
            }
            umask(mask);

            if listen(fd, 128) == -1 {
                saved_errno = errno!();
                close(fd);
                errno!() = saved_errno;
                break 'fail;
            }
            setblocking(fd, 0);

            return fd;
        }

        // fail:
        if !cause.is_null() {
            *cause = format_nul!(
                "error creating {} ({})",
                _s(SOCKET_PATH),
                strerror(errno!())
            );
        }
        -1
    }
}

/// Tidy up every hour.
unsafe extern "C-unwind" fn server_tidy_event(_fd: i32, _events: i16, _data: *mut c_void) {
    let tv = timeval {
        tv_sec: 3600,
        tv_usec: 0,
    };
    unsafe {
        let t = get_timer();

        format_tidy_jobs();

        #[cfg(not(target_os = "macos"))]
        {
            libc::malloc_trim(0);
        }

        log_debug!(
            "{}: took {} milliseconds",
            "server_tidy_event",
            get_timer() - t
        );
        event_add(&raw mut SERVER_EV_TIDY, &raw const tv);
    }
}

pub unsafe fn server_start(
    client: *mut tmuxproc,
    flags: client_flag,
    base: *mut event_base,
    lockfd: c_int,
    lockfile: *mut u8,
) -> c_int {
    unsafe {
        let mut fd = 0;
        let mut set: sigset_t = zeroed();
        let mut oldset: sigset_t = zeroed();

        let mut c: *mut client = null_mut();
        let mut cause: *mut u8 = null_mut();
        let tv: timeval = timeval {
            tv_sec: 3600,
            tv_usec: 0,
        };

        sigfillset(&raw mut set);
        sigprocmask(SIG_BLOCK, &raw const set, &raw mut oldset);

        if !flags.intersects(client_flag::NOFORK) && proc_fork_and_daemon(&raw mut fd) != 0 {
            // in parent process i.e. client
            sigprocmask(SIG_SETMASK, &raw mut oldset, null_mut());
            return fd;
        }

        std::panic::set_hook(Box::new(|panic_info| {
            use std::fmt::Write;
            let backtrace = std::backtrace::Backtrace::force_capture();
            let location = panic_info.location();

            let mut err_str = String::new();

            if let Some(s) = panic_info.payload().downcast_ref::<&str>() {
                _ = write!(&mut err_str, "panic! {s:?}\n{backtrace:#?}");
                log_debug!(
                    "panic{}: {s}",
                    location
                        .map(|loc| format!(" at {}:{}", loc.file(), loc.line()))
                        .unwrap_or_default()
                );
            } else if let Some(s) = panic_info.payload().downcast_ref::<String>() {
                _ = write!(&mut err_str, "panic! {s:?}\n{backtrace:#?}");
                log_debug!(
                    "panic{}: {s}",
                    location
                        .map(|loc| format!(" at {}:{}", loc.file(), loc.line()))
                        .unwrap_or_default()
                );
            }

            log_close();

            if let Err(err) =
                std::fs::write(format!("server-panic-{}.txt", std::process::id()), err_str)
            {
                eprintln!("error in panic handler! {err}");
            }
        }));

        // now in child process i.e. server
        proc_clear_signals(client, 0);
        SERVER_CLIENT_FLAGS = flags;

        if event_reinit(base) != 0 {
            fatalx("event_reinit failed");
        }
        SERVER_PROC = proc_start(c"server");

        proc_set_signals(SERVER_PROC, Some(server_signal));
        sigprocmask(SIG_SETMASK, &raw mut oldset, null_mut());

        if log_get_level() > 1 {
            tty_create_log();
        }

        // TODO pledge

        input_key_build();
        rb_init(&raw mut WINDOWS);
        rb_init(&raw mut ALL_WINDOW_PANES);
        tailq_init(&raw mut CLIENTS);
        rb_init(&raw mut SESSIONS);
        key_bindings_init();
        tailq_init(&raw mut MESSAGE_LOG);
        gettimeofday(&raw mut START_TIME, null_mut());

        if cfg!(feature = "systemd") {
            // TODO we could be truncating important bits
            SERVER_FD =
                crate::compat::systemd::systemd_create_socket(flags.bits() as i32, &raw mut cause);
        } else {
            SERVER_FD = server_create_socket(flags, &raw mut cause);
        }
        if SERVER_FD != -1 {
            server_update_socket();
        }
        if !flags.intersects(client_flag::NOFORK) {
            c = server_client_create(fd);
        } else {
            options_set_number(GLOBAL_OPTIONS, c!("exit-empty"), 0);
        }

        if lockfd >= 0 {
            unlink(lockfile);
            free_(lockfile);
            close(lockfd);
        }

        if !cause.is_null() {
            if !c.is_null() {
                (*c).exit_message = cause;
                (*c).flags |= client_flag::EXIT;
            } else {
                eprintln!("{}", _s(cause));
                libc::exit(1);
            }
        }

        evtimer_set_no_args(&raw mut SERVER_EV_TIDY, server_tidy_event);
        evtimer_add(&raw mut SERVER_EV_TIDY, &raw const tv);

        server_acl_init();

        server_add_accept(0);
        proc_loop(SERVER_PROC, Some(server_loop));

        job_kill_all();
        status_prompt_save_history();

        libc::exit(0)
    }
}

pub unsafe fn server_loop() -> i32 {
    unsafe {
        CURRENT_TIME = libc::time(null_mut());

        loop {
            let mut items = cmdq_next(null_mut());
            for c in tailq_foreach(&raw mut CLIENTS).map(NonNull::as_ptr) {
                if (*c).flags.intersects(client_flag::IDENTIFIED) {
                    items += cmdq_next(c);
                }
            }

            if items == 0 {
                break;
            }
        }

        server_client_loop();

        if options_get_number_(GLOBAL_OPTIONS, "exit-empty") == 0 && SERVER_EXIT == 0 {
            return 0;
        }

        if options_get_number_(GLOBAL_OPTIONS, "exit-unattached") == 0
            && !rb_empty(&raw mut SESSIONS)
        {
            return 0;
        }

        for c in tailq_foreach(&raw mut CLIENTS) {
            if !(*c.as_ptr()).session.is_null() {
                return 0;
            }
        }

        // No attached clients therefore want to exit - flush any waiting
        // clients but don't actually exit until they've gone.
        cmd_wait_for_flush();
        if !tailq_empty(&raw const CLIENTS) {
            return 0;
        }

        if job_still_running() {
            return 0;
        }

        1
    }
}

unsafe fn server_send_exit() {
    unsafe {
        cmd_wait_for_flush();

        for c in tailq_foreach(&raw mut CLIENTS).map(NonNull::as_ptr) {
            if (*c).flags.intersects(client_flag::SUSPENDED) {
                server_client_lost(c);
            } else {
                (*c).flags |= client_flag::EXIT;
                (*c).exit_type = exit_type::CLIENT_EXIT_SHUTDOWN;
            }
            (*c).session = null_mut();
        }

        for s in rb_foreach(&raw mut SESSIONS).map(NonNull::as_ptr) {
            session_destroy(s, 1, c!("server_send_exit"));
        }
    }
}

pub unsafe fn server_update_socket() {
    static mut LAST: c_int = -1;
    unsafe {
        let mut sb: stat = zeroed(); // TODO remove unecessary init

        let mut n = 0;
        for s in rb_foreach(&raw mut SESSIONS).map(|s| s.as_ptr()) {
            if (*s).attached != 0 {
                n += 1;
                break;
            }
        }

        if n != LAST {
            LAST = n;

            if stat(SOCKET_PATH.cast(), &raw mut sb) != 0 {
                return;
            }
            let mut mode = sb.st_mode & ACCESSPERMS;
            if n != 0 {
                if mode & S_IRUSR != 0 {
                    mode |= S_IXUSR;
                }
                if mode & S_IRGRP != 0 {
                    mode |= S_IXGRP;
                }
                if mode & S_IROTH != 0 {
                    mode |= S_IXOTH;
                }
            } else {
                mode &= !(S_IXUSR | S_IXGRP | S_IXOTH);
            }
            chmod(SOCKET_PATH.cast(), mode);
        }
    }
}

unsafe extern "C-unwind" fn server_accept(fd: i32, events: i16, _data: *mut c_void) {
    unsafe {
        let mut sa: sockaddr_storage = zeroed(); // TODO remove this init
        let mut slen: socklen_t = size_of::<sockaddr_storage>() as socklen_t;

        server_add_accept(0);
        if events & EV_READ == 0 {
            return;
        }

        let newfd = accept(fd, &raw mut sa as _, &raw mut slen);
        if newfd == -1 {
            match errno!() {
                libc::EAGAIN | libc::EINTR | libc::ECONNABORTED => return,
                libc::ENFILE | libc::EMFILE => {
                    // Delete and don't try again for 1 second.
                    server_add_accept(1);
                    return;
                }
                _ => fatal("accept failed"),
            }
        }

        if SERVER_EXIT != 0 {
            close(newfd);
            return;
        }
        let c = server_client_create(newfd);
        if server_acl_join(c) == 0 {
            (*c).exit_message = xmalloc::xstrdup(c!("access not allowed")).cast().as_ptr();
            (*c).flags |= client_flag::EXIT;
        }
    }
}

pub unsafe fn server_add_accept(timeout: c_int) {
    unsafe {
        let mut tv = timeval {
            tv_sec: timeout as i64,
            tv_usec: 0,
        };

        if SERVER_FD == -1 {
            return;
        }

        if event_initialized(&raw mut SERVER_EV_ACCEPT) != 0 {
            event_del(&raw mut SERVER_EV_ACCEPT);
        }

        if timeout == 0 {
            event_set(
                &raw mut SERVER_EV_ACCEPT,
                SERVER_FD,
                EV_READ,
                Some(server_accept),
                null_mut(),
            );
            event_add(&raw mut SERVER_EV_ACCEPT, null_mut());
        } else {
            event_set(
                &raw mut SERVER_EV_ACCEPT,
                SERVER_FD,
                EV_TIMEOUT,
                Some(server_accept),
                null_mut(),
            );
            event_add(&raw mut SERVER_EV_ACCEPT, &raw mut tv);
        }
    }
}

/// Signal handler.
unsafe fn server_signal(sig: i32) {
    unsafe {
        log_debug!("{}: {}", "server_signal", _s(strsignal(sig).cast::<u8>()));
        match sig {
            libc::SIGINT | libc::SIGTERM => {
                SERVER_EXIT = 1;
                server_send_exit();
            }
            libc::SIGCHLD => server_child_signal(),
            libc::SIGUSR1 => {
                event_del(&raw mut SERVER_EV_ACCEPT);
                let fd = server_create_socket(SERVER_CLIENT_FLAGS, null_mut());
                if fd != -1 {
                    close(SERVER_FD);
                    SERVER_FD = fd;
                    server_update_socket();
                }
                server_add_accept(0);
            }
            libc::SIGUSR2 => proc_toggle_log(SERVER_PROC),
            _ => {
                // nop
            }
        }
    }
}

// handle SIGCHLD

unsafe fn server_child_signal() {
    let mut status = 0i32;
    unsafe {
        loop {
            let pid: pid_t = waitpid(
                crate::compat::WAIT_ANY,
                &raw mut status,
                WNOHANG | WUNTRACED,
            );
            match pid {
                -1 => {
                    if errno!() == ECHILD {
                        return;
                    }
                    fatal("waitpid failed");
                }
                0 => return,
                _ => {
                    if WIFSTOPPED(status) {
                        server_child_stopped(pid, status);
                    } else if WIFEXITED(status) || WIFSIGNALED(status) {
                        server_child_exited(pid, status);
                    }
                }
            }
        }
    }
}

unsafe fn server_child_exited(pid: pid_t, status: i32) {
    unsafe {
        for w in rb_foreach(&raw mut WINDOWS).map(NonNull::as_ptr) {
            for wp in tailq_foreach::<_, discr_entry>(&raw mut (*w).panes).map(NonNull::as_ptr) {
                if (*wp).pid == pid {
                    (*wp).status = status;
                    (*wp).flags |= window_pane_flags::PANE_STATUSREADY;

                    log_debug!("%%{} exited", (*wp).id);
                    (*wp).flags |= window_pane_flags::PANE_EXITED;

                    if window_pane_destroy_ready(wp) {
                        server_destroy_pane(wp, 1);
                    }
                    break;
                }
            }
        }
        job_check_died(pid, status);
    }
}

unsafe fn server_child_stopped(pid: pid_t, status: i32) {
    unsafe {
        if WSTOPSIG(status) == SIGTTIN || WSTOPSIG(status) == SIGTTOU {
            return;
        }

        for w in rb_foreach(&raw mut WINDOWS).map(NonNull::as_ptr) {
            for wp in tailq_foreach::<_, discr_entry>(&raw mut (*w).panes).map(NonNull::as_ptr) {
                if (*wp).pid == pid && killpg(pid, SIGCONT) != 0 {
                    kill(pid, SIGCONT);
                }
            }
        }
        job_check_died(pid, status);
    }
}

macro_rules! server_add_message {
   ($fmt:literal $(, $args:expr)* $(,)?) => {
        crate::server::server_add_message_(format_args!($fmt $(, $args)*))
    };
}
pub(crate) use server_add_message;
pub unsafe fn server_add_message_(args: std::fmt::Arguments) {
    unsafe {
        let mut s = args.to_string();
        s.push('\0');
        let s = s.leak().as_mut_ptr().cast();

        log_debug!("message: {}", _s(s));

        let msg: *mut message_entry = xcalloc1::<message_entry>() as *mut message_entry;
        gettimeofday(&raw mut (*msg).msg_time, null_mut());
        (*msg).msg_num = MESSAGE_NEXT + 1;
        MESSAGE_NEXT += 1;
        (*msg).msg = s;

        tailq_insert_tail(&raw mut MESSAGE_LOG, msg);

        let limit = options_get_number_(GLOBAL_OPTIONS, "message-limit") as u32;
        for msg in tailq_foreach(&raw mut MESSAGE_LOG).map(NonNull::as_ptr) {
            if (*msg).msg_num + limit >= MESSAGE_NEXT {
                continue;
            }
            free_((*msg).msg);
            tailq_remove(&raw mut MESSAGE_LOG, msg);
            free_(msg);
        }
    }
}

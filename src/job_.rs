// Copyright (c) 2009 Nicholas Marriott <nicholas.marriott@gmail.com>
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
use std::path::Path;

use crate::compat::fdforkpty::fdforkpty;
use crate::libc::{
    AF_UNIX, PF_UNSPEC, SHUT_WR, SIG_BLOCK, SIG_SETMASK, SIGCONT, SIGTERM, SIGTTIN,
    SIGTTOU, SOCK_STREAM, STDERR_FILENO, STDIN_FILENO, STDOUT_FILENO, TIOCSWINSZ, WIFSTOPPED,
    WSTOPSIG, close, dup2, execl, execvp, fork, ioctl, kill, killpg, memset, shutdown,
    sigfillset, sigprocmask, sigset_t, socketpair, winsize,
};
use crate::*;
use crate::options_::{options, options_get_string_};

pub type job_update_cb = Option<unsafe fn(*mut job)>;
pub type job_complete_cb = Option<unsafe fn(*mut job)>;
pub type job_free_cb = Option<unsafe fn(*mut c_void)>;

#[derive(Default, Eq, PartialEq)]
#[repr(i32)]
pub enum job_state {
    #[default]
    JOB_RUNNING = 0,
    JOB_DEAD = 1,
    JOB_CLOSED = 2,
}

#[repr(C)]
#[derive(Default)]
pub struct job {
    pub state: job_state,

    pub flags: job_flag,

    pub cmd: *mut u8,
    pub pid: pid_t,
    pub tty: [u8; TTY_NAME_MAX],
    pub status: i32,

    pub fd: c_int,
    pub event: *mut bufferevent,

    pub updatecb: job_update_cb,
    pub completecb: job_complete_cb,
    pub freecb: job_free_cb,
    pub data: *mut c_void,

    pub entry: list_entry<job>,
}
impl ListEntry<job, ()> for job {
    unsafe fn field(this: *mut Self) -> *mut list_entry<job> {
        unsafe { &raw mut (*this).entry }
    }
}

type joblist = list_head<job>;
static mut ALL_JOBS: joblist = list_head_initializer();

#[cfg(target_os = "windows")]
pub unsafe fn job_run(
    _cmd: *const u8,
    _argc: c_int,
    _argv: *mut *mut u8,
    _e: *mut environ,
    _s: *mut session,
    _cwd: *const u8,
    _updatecb: job_update_cb,
    _completecb: job_complete_cb,
    _freecb: job_free_cb,
    _data: *mut c_void,
    _flags: job_flag,
    _sx: c_int,
    _sy: c_int,
) -> *mut job {
    todo!()
}

#[cfg(not(target_os = "windows"))]
pub unsafe fn job_run(
    cmd: *const u8,
    argc: c_int,
    argv: *mut *mut u8,
    e: *mut environ,
    s: *mut session,
    cwd: *const u8,
    updatecb: job_update_cb,
    completecb: job_complete_cb,
    freecb: job_free_cb,
    data: *mut c_void,
    flags: job_flag,
    sx: c_int,
    sy: c_int,
) -> *mut job {
    use crate::compat::closefrom::closefrom;

    let __func__ = "job_run";
    unsafe {
        let job: *mut job;
        let env: *mut environ;
        let pid: pid_t;
        let mut out: [i32; 2] = [0; 2];
        let mut master: i32 = 0;
        let mut shell: *const u8;
        let mut set = MaybeUninit::<sigset_t>::uninit();
        let mut oldset = MaybeUninit::<sigset_t>::uninit();
        let mut ws = MaybeUninit::<winsize>::uninit();
        let argvp: *mut *mut u8;
        // let mut tty = MaybeUninit::<[c_char; TTY_NAME_MAX]>::uninit();
        let mut tty = [0i8; 64];
        let argv0: *mut u8;
        let oo: *mut options;

        'fail: {
            env = environ_for_session(s, !CFG_FINISHED.load(atomic::Ordering::Acquire) as i32);
            if !e.is_null() {
                environ_copy(e, env);
            }

            if !flags.intersects(job_flag::JOB_DEFAULTSHELL) {
                shell = _PATH_BSHELL;
            } else {
                if !s.is_null() {
                    oo = (*s).options;
                } else {
                    oo = GLOBAL_S_OPTIONS;
                }
                shell = options_get_string_(oo, "default-shell");
                if !checkshell_(shell) {
                    shell = _PATH_BSHELL;
                }
            }
            argv0 = shell_argv0(shell, 0);

            sigfillset(set.as_mut_ptr());
            sigprocmask(SIG_BLOCK, set.as_mut_ptr(), oldset.as_mut_ptr());

            if flags.intersects(job_flag::JOB_PTY) {
                memset(ws.as_mut_ptr().cast(), 0, size_of::<winsize>());
                (*ws.as_mut_ptr()).ws_col = sx as u16;
                (*ws.as_mut_ptr()).ws_row = sy as u16;
                pid = fdforkpty(
                    PTM_FD,
                    &raw mut master,
                    (&raw mut tty) as *mut u8,
                    null_mut(),
                    ws.as_mut_ptr(),
                );
            } else {
                if socketpair(AF_UNIX, SOCK_STREAM, PF_UNSPEC, &raw mut out as *mut c_int) != 0 {
                    break 'fail;
                }
                pid = fork();
            }

            if cmd.is_null() {
                cmd_log_argv!(argc, argv, "{__func__}");
                log_debug!(
                    "{} cwd={} shell={}",
                    __func__,
                    _s(if cwd.is_null() { c!("") } else { cwd }),
                    _s(shell),
                );
            } else {
                log_debug!(
                    "{} cmd={} cwd={} shell={}",
                    __func__,
                    _s(cmd),
                    _s(if cwd.is_null() { c!("") } else { cwd }),
                    _s(shell),
                );
            }

            match pid {
                -1 => {
                    if !flags.intersects(job_flag::JOB_PTY) {
                        close(out[0]);
                        close(out[1]);
                    }
                    break 'fail;
                }
                0 => {
                    proc_clear_signals(SERVER_PROC, 1);
                    sigprocmask(SIG_SETMASK, oldset.as_mut_ptr(), null_mut());

                    if (cwd.is_null() || std::env::set_current_dir(cstr_to_str(cwd)).is_err())
                        && find_home().is_none_or(|home| {
                            std::env::set_current_dir(home.to_str().expect("TODO")).is_err()
                        })
                        && std::env::set_current_dir(Path::new("/")).is_err()
                    {
                        fatal("chdir failed");
                    }

                    environ_push(env);
                    environ_free(env);

                    if !flags.intersects(job_flag::JOB_PTY) {
                        if dup2(out[1], STDIN_FILENO) == -1 {
                            fatal("dup2 failed");
                        }
                        if dup2(out[1], STDOUT_FILENO) == -1 {
                            fatal("dup2 failed");
                        }
                        if out[1] != STDIN_FILENO && out[1] != STDOUT_FILENO {
                            close(out[1]);
                        }
                        close(out[0]);

                        use std::os::unix::io::IntoRawFd;
                        let nullfd= std::fs::OpenOptions::new().read(true).write(true).open(PATH_DEVNULL).expect("open failed").into_raw_fd();
                        if dup2(nullfd, STDERR_FILENO) == -1 {
                            fatal("dup2 failed");
                        }
                        if nullfd != STDERR_FILENO {
                            close(nullfd);
                        }
                    }
                    closefrom(STDERR_FILENO + 1);

                    if !cmd.is_null() {
                        std::env::set_var("SHELL", cstr_to_str(shell));
                        execl(
                            shell.cast(),
                            argv0.cast(),
                            c!("-c"),
                            cmd,
                            null_mut::<c_void>(),
                        );
                        fatal("execl failed");
                    } else {
                        argvp = cmd_copy_argv(argc, argv);
                        execvp((*argvp).cast(), argvp.cast());
                        fatal("execvp failed");
                    }
                }
                _ => (),
            }

            sigprocmask(SIG_SETMASK, oldset.as_ptr(), null_mut());
            environ_free(env);
            free_(argv0);

            job = Box::leak(Box::new(job {
                state: job_state::JOB_RUNNING,
                flags,
                cmd: if !cmd.is_null() {
                    xstrdup(cmd).as_ptr()
                } else {
                    CString::new(cmd_stringify_argv(argc, argv))
                        .unwrap()
                        .into_raw()
                        .cast()
                },
                pid,
                status: 0,
                ..Default::default()
            }));

            strlcpy((*job).tty.as_mut_ptr(), tty.as_ptr().cast(), TTY_NAME_MAX);

            list_insert_head(&raw mut ALL_JOBS, job);

            (*job).updatecb = updatecb;
            (*job).completecb = completecb;
            (*job).freecb = freecb;
            (*job).data = data;

            if !flags.intersects(job_flag::JOB_PTY) {
                close(out[1]);
                (*job).fd = out[0];
            } else {
                (*job).fd = master;
            }
            setblocking((*job).fd, 0);

            (*job).event = bufferevent_new(
                (*job).fd,
                Some(job_read_callback),
                Some(job_write_callback),
                Some(job_error_callback),
                job as *mut c_void,
            );
            if (*job).event.is_null() {
                fatalx("out of memory");
            }
            bufferevent_enable((*job).event, EV_READ | EV_WRITE);

            log_debug!("run job {:p}: {} pid {}", job, _s((*job).cmd), (*job).pid);
            return job;
        }

        sigprocmask(SIG_SETMASK, oldset.as_ptr(), null_mut());
        environ_free(env);
        free_(argv0);
        null_mut()
    }
}

pub unsafe fn job_transfer(job: *mut job, pid: *mut pid_t, tty: *mut u8, ttylen: usize) -> c_int {
    unsafe {
        let fd = (*job).fd;

        log_debug!("transfer job {:p}: {}", job, _s((*job).cmd));

        if !pid.is_null() {
            *pid = (*job).pid;
        }
        if !tty.is_null() {
            strlcpy(tty, ((*job).tty).as_mut_ptr(), ttylen);
        }

        list_remove(job);
        free_((*job).cmd);

        if let Some(freecb) = (*job).freecb
            && !(*job).data.is_null()
        {
            freecb((*job).data);
        }

        if !(*job).event.is_null() {
            bufferevent_free((*job).event);
        }

        free_(job);
        fd
    }
}

pub unsafe fn job_free(job: *mut job) {
    unsafe {
        log_debug!("free job {:p}: {}", job, _s((*job).cmd));

        list_remove(job);
        free_((*job).cmd);

        if let Some(freecb) = (*job).freecb
            && !((*job).data).is_null()
        {
            freecb((*job).data);
        }
        if (*job).pid != -1 {
            kill((*job).pid, SIGTERM);
        }
        if !((*job).event).is_null() {
            bufferevent_free((*job).event);
        }
        if (*job).fd != -1 {
            close((*job).fd);
        }
        free_(job);
    }
}

pub unsafe fn job_resize(job: *mut job, sx: c_uint, sy: c_uint) {
    let mut ws = MaybeUninit::<winsize>::uninit();

    unsafe {
        let ws = ws.as_mut_ptr();
        if (*job).fd == -1 || !(*job).flags.intersects(job_flag::JOB_PTY) {
            return;
        }
        log_debug!("resize job {:p}: {}x{}", job, sx, sy);
        (*ws).ws_col = sx as u16;
        (*ws).ws_row = sy as u16;

        #[cfg(not(target_os = "windows"))]
        if ioctl((*job).fd, TIOCSWINSZ, ws) == -1 {
            fatal("ioctl failed");
        }
        #[cfg(target_os = "windows")]
        {
            let _ = ws;
            todo!("job_resize not implemented for Windows");
        }
    }
}

unsafe extern "C-unwind" fn job_read_callback(_bufev: *mut bufferevent, data: *mut c_void) {
    let job = data as *mut job;

    unsafe {
        if let Some(updatecb) = (*job).updatecb {
            updatecb(job);
        }
    }
}
unsafe extern "C-unwind" fn job_write_callback(_bufev: *mut bufferevent, data: *mut c_void) {
    unsafe {
        let job = data as *mut job;
        let len = EVBUFFER_LENGTH(EVBUFFER_OUTPUT((*job).event));

        log_debug!(
            "job write {:p}: {}, pid {}, output left {}",
            job,
            _s((*job).cmd),
            (*job).pid,
            len,
        );

        if len == 0 && !(*job).flags.intersects(job_flag::JOB_KEEPWRITE) {
            shutdown((*job).fd, SHUT_WR);
            bufferevent_disable((*job).event, EV_WRITE);
        }
    }
}

unsafe extern "C-unwind" fn job_error_callback(
    _bufev: *mut bufferevent,
    _events: libc::c_short,
    data: *mut c_void,
) {
    let job: *mut job = data.cast();

    unsafe {
        log_debug!(
            "job error {:p}: {}, pid {}",
            job,
            _s((*job).cmd),
            (*job).pid
        );
        if (*job).state == job_state::JOB_DEAD {
            if let Some(completecb) = (*job).completecb {
                completecb(job);
            }
            job_free(job);
        } else {
            bufferevent_disable((*job).event, EV_READ);
            (*job).state = job_state::JOB_CLOSED;
        }
    }
}

pub unsafe fn job_check_died(pid: pid_t, status: i32) {
    unsafe {
        let Some(job) = list_foreach(&raw mut ALL_JOBS).find(|job| pid == (*job.as_ptr()).pid)
        else {
            return;
        };
        let job = job.as_ptr();

        if WIFSTOPPED(status) {
            if WSTOPSIG(status) == SIGTTIN || WSTOPSIG(status) == SIGTTOU {
                return;
            }
            killpg((*job).pid, SIGCONT);
            return;
        }
        log_debug!(
            "job died {:p}: {} pid {}",
            job,
            _s((*job).cmd),
            (*job).pid as c_long
        );

        (*job).status = status;

        if (*job).state == job_state::JOB_CLOSED {
            if let Some(completecb) = (*job).completecb {
                completecb(job);
            }
            job_free(job);
        } else {
            (*job).pid = -1;
            (*job).state = job_state::JOB_DEAD;
        }
    }
}

pub unsafe fn job_get_status(job: *mut job) -> i32 {
    unsafe { (*job).status }
}

pub unsafe fn job_get_data(job: *mut job) -> *mut c_void {
    unsafe { (*job).data }
}

pub unsafe fn job_get_event(job: *mut job) -> *mut bufferevent {
    unsafe { (*job).event }
}

pub unsafe fn job_kill_all() {
    unsafe {
        for job in list_foreach(&raw mut ALL_JOBS).map(NonNull::as_ptr) {
            if (*job).pid != -1 {
                kill((*job).pid, SIGTERM);
            }
        }
    }
}

pub unsafe fn job_still_running() -> bool {
    unsafe {
        list_foreach(&raw mut ALL_JOBS)
            .map(NonNull::as_ptr)
            .any(|job| {
                !(*job).flags.intersects(job_flag::JOB_NOWAIT)
                    && (*job).state == job_state::JOB_RUNNING
            })
    }
}

pub unsafe fn job_print_summary(item: *mut cmdq_item, mut blank: i32) {
    unsafe {
        for (n, job) in list_foreach(&raw mut ALL_JOBS)
            .map(NonNull::as_ptr)
            .enumerate()
        {
            if blank != 0 {
                cmdq_print!(item, "");
                blank = 0;
            }
            cmdq_print!(
                item,
                "Job {}: {} [fd={}, pid={}, status={}]",
                n,
                _s((*job).cmd),
                (*job).fd,
                (*job).pid,
                (*job).status,
            );
        }
    }
}

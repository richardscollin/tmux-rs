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
#[cfg_attr(target_os = "windows", expect(unused_imports))]
use crate::libc::{
    _exit, STDERR_FILENO, STDIN_FILENO,
    STDOUT_FILENO, close, dup2, execl
};
use crate::*;

pub static CMD_PIPE_PANE_ENTRY: cmd_entry = cmd_entry {
    name: "pipe-pane",
    alias: Some("pipep"),

    args: args_parse::new("IOot:", 0, 1, None),
    usage: "[-IOo] [-t target-pane] [shell-command]",

    target: cmd_entry_flag::new(b't', cmd_find_type::CMD_FIND_PANE, cmd_find_flags::empty()),
    source: cmd_entry_flag::zeroed(),

    flags: cmd_flag::CMD_AFTERHOOK,
    exec: cmd_pipe_pane_exec,
};

#[cfg(target_os = "windows")]
pub unsafe fn cmd_pipe_pane_exec(_self_: *mut cmd, _item: *mut cmdq_item) -> cmd_retval {
    todo!()
}

#[cfg(not(target_os = "windows"))]
pub unsafe fn cmd_pipe_pane_exec(self_: *mut cmd, item: *mut cmdq_item) -> cmd_retval {
    use crate::compat::closefrom::closefrom;
    use crate::libc::{
        AF_UNIX, PF_UNSPEC, SIG_BLOCK, SIG_SETMASK, sigfillset, sigprocmask, sigset_t, socketpair,
    };

    unsafe {
        let args = cmd_get_args(self_);
        let target = cmdq_get_target(item);
        let tc = cmdq_get_target_client(item);
        let wp = (*target).wp;
        let s = (*target).s;
        let wl = (*target).wl;
        let wpo = &raw mut (*wp).pipe_offset;
        let mut pipe_fd: [i32; 2] = [0; 2];
        let in_: bool;
        let out: bool;
        let mut set: sigset_t = zeroed(); // TODO uninit
        let mut oldset: sigset_t = zeroed(); // TODO uninit

        // Do nothing if pane is dead.
        if window_pane_exited(wp) {
            cmdq_error!(item, "target pane has exited");
            return cmd_retval::CMD_RETURN_ERROR;
        }

        // Destroy the old pipe.
        let old_fd = (*wp).pipe_fd;
        if (*wp).pipe_fd != -1 {
            bufferevent_free((*wp).pipe_event);
            close((*wp).pipe_fd);
            (*wp).pipe_fd = -1;

            if window_pane_destroy_ready(wp) {
                server_destroy_pane(wp, 1);
                return cmd_retval::CMD_RETURN_NORMAL;
            }
        }

        // If no pipe command, that is enough.
        if args_count(args) == 0 || *args_string(args, 0) == b'\0' {
            return cmd_retval::CMD_RETURN_NORMAL;
        }

        // With -o, only open the new pipe if there was no previous one. This
        // allows a pipe to be toggled with a single key, for example:
        //
        // 	bind ^p pipep -o 'cat >>~/output'
        if args_has(args, 'o') && old_fd != -1 {
            return cmd_retval::CMD_RETURN_NORMAL;
        }

        // What do we want to do? Neither -I or -O is -O.
        if args_has(args, 'I') {
            in_ = true;
            out = args_has(args, 'O');
        } else {
            in_ = false;
            out = true;
        }

        // Open the new pipe.
        if socketpair(AF_UNIX, libc::SOCK_STREAM, PF_UNSPEC, pipe_fd.as_mut_ptr()) != 0 {
            cmdq_error!(item, "socketpair error: {}", strerror(errno!()));
            return cmd_retval::CMD_RETURN_ERROR;
        }

        // Expand the command.
        let ft = format_create(
            cmdq_get_client(item),
            item,
            FORMAT_NONE,
            format_flags::empty(),
        );
        format_defaults(ft, tc, NonNull::new(s), NonNull::new(wl), NonNull::new(wp));
        let cmd = format_expand_time(ft, args_string(args, 0));
        format_free(ft);

        // Fork the child.
        sigfillset(&raw mut set);
        sigprocmask(SIG_BLOCK, &raw const set, &raw mut oldset);
        match libc::fork() {
            -1 => {
                sigprocmask(SIG_SETMASK, &raw const oldset, null_mut());
                cmdq_error!(item, "fork error: {}", strerror(errno!()));

                free_(cmd);
                cmd_retval::CMD_RETURN_ERROR
            }
            0 => {
                proc_clear_signals(SERVER_PROC, 1);
                sigprocmask(SIG_SETMASK, &oldset, null_mut());
                close(pipe_fd[0]);

                let null_fd = std::fs::OpenOptions::new().write(true).open(PATH_DEVNULL).map(std::os::fd::IntoRawFd::into_raw_fd).unwrap_or(-1);
                #[expect(clippy::collapsible_else_if)]
                if out {
                    if dup2(pipe_fd[1], STDIN_FILENO) == -1 {
                        _exit(1);
                    }
                } else {
                    if dup2(null_fd, STDIN_FILENO) == -1 {
                        _exit(1);
                    }
                }
                #[expect(clippy::collapsible_else_if)]
                if in_ {
                    if dup2(pipe_fd[1], STDOUT_FILENO) == -1 {
                        _exit(1);
                    }
                    if pipe_fd[1] != STDOUT_FILENO {
                        close(pipe_fd[1]);
                    }
                } else {
                    if dup2(null_fd, STDOUT_FILENO) == -1 {
                        _exit(1);
                    }
                }
                if dup2(null_fd, STDERR_FILENO) == -1 {
                    _exit(1);
                }
                closefrom(STDERR_FILENO + 1);

                execl(
                    _PATH_BSHELL.cast(),
                    c"sh".as_ptr(),
                    c"-c".as_ptr(),
                    cmd,
                    null_mut::<c_void>(),
                );
                _exit(1)
            }
            _ => {
                // Parent process.
                sigprocmask(SIG_SETMASK, &raw mut oldset, null_mut());
                close(pipe_fd[1]);

                (*wp).pipe_fd = pipe_fd[0];
                memcpy__(wpo, &raw mut (*wp).offset);

                setblocking((*wp).pipe_fd, 0);
                (*wp).pipe_event = bufferevent_new(
                    (*wp).pipe_fd,
                    Some(cmd_pipe_pane_read_callback),
                    Some(cmd_pipe_pane_write_callback),
                    Some(cmd_pipe_pane_error_callback),
                    wp.cast(),
                );
                if (*wp).pipe_event.is_null() {
                    fatalx("out of memory");
                }
                if out {
                    bufferevent_enable((*wp).pipe_event, EV_WRITE);
                }
                if in_ {
                    bufferevent_enable((*wp).pipe_event, EV_READ);
                }

                free_(cmd);
                cmd_retval::CMD_RETURN_NORMAL
            }
        }
    }
}

pub unsafe extern "C-unwind" fn cmd_pipe_pane_read_callback(
    _bufev: *mut bufferevent,
    data: *mut c_void,
) {
    unsafe {
        let wp: *mut window_pane = data as *mut window_pane;
        let evb = (*(*wp).pipe_event).input;

        let available = EVBUFFER_LENGTH(evb);
        log_debug!("%%{} pipe read {}", (*wp).id, available);

        bufferevent_write((*wp).event, EVBUFFER_DATA(evb).cast(), available);
        evbuffer_drain(evb, available);

        if window_pane_destroy_ready(wp) {
            server_destroy_pane(wp, 1);
        }
    }
}

pub unsafe extern "C-unwind" fn cmd_pipe_pane_write_callback(
    _bufev: *mut bufferevent,
    data: *mut c_void,
) {
    unsafe {
        let wp: *mut window_pane = data as *mut window_pane;

        log_debug!("%%{} pipe empty", (*wp).id);

        if window_pane_destroy_ready(wp) {
            server_destroy_pane(wp, 1);
        }
    }
}

pub unsafe extern "C-unwind" fn cmd_pipe_pane_error_callback(
    _bufev: *mut bufferevent,
    _what: i16,
    data: *mut c_void,
) {
    unsafe {
        let wp: *mut window_pane = data as *mut window_pane;

        log_debug!("%%{} pipe error", (*wp).id);

        bufferevent_free((*wp).pipe_event);
        close((*wp).pipe_fd);
        (*wp).pipe_fd = -1;

        if window_pane_destroy_ready(wp) {
            server_destroy_pane(wp, 1);
        }
    }
}

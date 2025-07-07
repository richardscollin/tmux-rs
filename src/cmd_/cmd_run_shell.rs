// Copyright (c) 2009 Tiago Cunha <me@tiagocunha.org>
// Copyright (c) 2009 Nicholas Marriott <nicm@openbsd.org>
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

use crate::libc::{WEXITSTATUS, WIFEXITED, WIFSIGNALED, WTERMSIG, memcpy, strtod, toupper};

use crate::compat::queue::tailq_first;

pub static CMD_RUN_SHELL_ENTRY: cmd_entry = cmd_entry {
    name: SyncCharPtr::new(c"run-shell"),
    alias: SyncCharPtr::new(c"run"),

    args: args_parse::new(c"bd:Ct:c:", 0, 2, Some(cmd_run_shell_args_parse)),
    usage: SyncCharPtr::new(
        c"[-bC] [-c start-directory] [-d delay] [-t target-pane] [shell-command]",
    ),

    target: cmd_entry_flag::new(b't', cmd_find_type::CMD_FIND_PANE, CMD_FIND_CANFAIL),

    flags: cmd_flag::empty(),
    exec: cmd_run_shell_exec,
    source: cmd_entry_flag::zeroed(),
};

#[repr(C)]
pub struct cmd_run_shell_data<'a> {
    pub client: *mut client,
    pub cmd: *mut u8,
    pub state: *mut args_command_state<'a>,
    pub cwd: *mut u8,
    pub item: *mut cmdq_item,
    pub s: *mut session,
    pub wp_id: i32,
    pub timer: event,
    pub flags: job_flag,
}

pub unsafe fn cmd_run_shell_args_parse(
    args: *mut args,
    _idx: u32,
    cause: *mut *mut u8,
) -> args_parse_type {
    unsafe {
        if args_has_(args, 'C') {
            return args_parse_type::ARGS_PARSE_COMMANDS_OR_STRING;
        }
    }

    args_parse_type::ARGS_PARSE_STRING
}

pub unsafe fn cmd_run_shell_print(job: *mut job, msg: *const u8) {
    unsafe {
        let cdata: *mut cmd_run_shell_data = job_get_data(job) as *mut cmd_run_shell_data;
        let mut wp = null_mut();
        let mut fs: cmd_find_state = zeroed();

        if (*cdata).wp_id != -1 {
            wp = window_pane_find_by_id((*cdata).wp_id as u32);
        }
        if wp.is_null() {
            if !(*cdata).item.is_null() {
                cmdq_print!((*cdata).item, "{}", _s(msg));
                return;
            }
            if !(*cdata).item.is_null() && !(*cdata).client.is_null() {
                wp = server_client_get_pane((*cdata).client);
            }
            if wp.is_null() && cmd_find_from_nothing(&raw mut fs, 0) == 0 {
                wp = fs.wp;
            }
            if wp.is_null() {
                return;
            }
        }

        let wme = tailq_first(&raw mut (*wp).modes);
        if wme.is_null() || (*wme).mode != &raw const WINDOW_VIEW_MODE {
            window_pane_set_mode(
                wp,
                null_mut(),
                &raw const WINDOW_VIEW_MODE,
                null_mut(),
                null_mut(),
            );
        }
        window_copy_add!(wp, 1, "{}", _s(msg));
    }
}

pub unsafe fn cmd_run_shell_exec(self_: *mut cmd, item: *mut cmdq_item) -> cmd_retval {
    let __func__ = c!("cmd_run_shell_exec");
    unsafe {
        let args = cmd_get_args(self_);
        let target = cmdq_get_target(item);
        let c = cmdq_get_client(item);
        let tc = cmdq_get_target_client(item);
        let s = (*target).s;
        let wp = (*target).wp;
        let mut d: f64 = 0.0;
        let mut end: *mut u8 = null_mut();
        let wait = !args_has(args, b'b') as i32;

        let delay = args_get(args, b'd');
        if !delay.is_null() {
            d = strtod(delay, &raw mut end);
            if *end != b'\0' as _ {
                cmdq_error!(item, "invalid delay time: {}", _s(delay));
                return cmd_retval::CMD_RETURN_ERROR;
            }
        } else if args_count(args) == 0 {
            return cmd_retval::CMD_RETURN_NORMAL;
        }

        let cdata = xcalloc1::<cmd_run_shell_data>() as *mut cmd_run_shell_data;
        if !args_has_(args, 'C') {
            let cmd = args_string(args, 0);
            if !cmd.is_null() {
                (*cdata).cmd = format_single_from_target(item, cmd);
            }
        } else {
            (*cdata).state = args_make_commands_prepare(self_, item, 0, null_mut(), wait, 1);
        }

        if args_has_(args, 't') && !wp.is_null() {
            (*cdata).wp_id = (*wp).id as i32;
        } else {
            (*cdata).wp_id = -1;
        }

        if wait != 0 {
            (*cdata).client = c;
            (*cdata).item = item;
        } else {
            (*cdata).client = tc;
            (*cdata).flags |= job_flag::JOB_NOWAIT;
        }
        if !(*cdata).client.is_null() {
            (*(*cdata).client).references += 1;
        }
        if args_has_(args, 'c') {
            (*cdata).cwd = xstrdup(args_get_(args, 'c')).as_ptr();
        } else {
            (*cdata).cwd = xstrdup(server_client_get_cwd(c, s)).as_ptr();
        }

        (*cdata).s = s;
        if !s.is_null() {
            session_add_ref(s, __func__);
        }

        evtimer_set(
            &raw mut (*cdata).timer,
            Some(cmd_run_shell_timer),
            cdata.cast(),
        );
        if !delay.is_null() {
            let mut tv: timeval = timeval {
                tv_sec: d as time_t,
                tv_usec: (d - (d as time_t as f64)) as libc::suseconds_t * 1000000,
            };
            evtimer_add(&raw mut (*cdata).timer, &raw mut tv);
        } else {
            event_active(&raw mut (*cdata).timer, EV_TIMEOUT as i32, 1);
        }

        if wait == 0 {
            return cmd_retval::CMD_RETURN_NORMAL;
        }
    }
    cmd_retval::CMD_RETURN_WAIT
}

pub unsafe extern "C" fn cmd_run_shell_timer(_fd: i32, _events: i16, arg: *mut c_void) {
    unsafe {
        let cdata = arg as *mut cmd_run_shell_data;
        let c = (*cdata).client;
        let cmd = (*cdata).cmd;
        let item = (*cdata).item;
        let mut error = null_mut::<u8>();

        if (*cdata).state.is_null() {
            if cmd.is_null() {
                if !(*cdata).item.is_null() {
                    cmdq_continue((*cdata).item);
                }
                cmd_run_shell_free(cdata.cast());
                return;
            }
            if job_run(
                cmd,
                0,
                null_mut(),
                null_mut(),
                (*cdata).s,
                (*cdata).cwd,
                None,
                Some(cmd_run_shell_callback),
                Some(cmd_run_shell_free),
                cdata.cast(),
                (*cdata).flags,
                -1,
                -1,
            )
            .is_null()
            {
                cmd_run_shell_free(cdata.cast());
            }
            return;
        }

        let cmdlist = args_make_commands((*cdata).state, 0, null_mut(), &raw mut error);
        if cmdlist.is_null() {
            if (*cdata).item.is_null() {
                *error = (*error).to_ascii_uppercase();
                status_message_set!(c, -1, 1, 0, "{}", _s(error));
            } else {
                cmdq_error!((*cdata).item, "{}", _s(error));
            }
            free_(error);
        } else if item.is_null() {
            let new_item = cmdq_get_command(cmdlist, null_mut());
            cmdq_append(c, new_item);
        } else {
            let new_item = cmdq_get_command(cmdlist, cmdq_get_state(item));
            cmdq_insert_after(item, new_item);
        }

        if !(*cdata).item.is_null() {
            cmdq_continue((*cdata).item);
        }
        cmd_run_shell_free(cdata.cast());
    }
}

pub unsafe fn cmd_run_shell_callback(job: *mut job) {
    unsafe {
        let cdata = job_get_data(job) as *mut cmd_run_shell_data;
        let event = job_get_event(job);
        let item = (*cdata).item;
        let cmd = (*cdata).cmd;
        let mut msg = null_mut();
        // *line;
        // size_t size;
        let mut retcode: i32 = 0;
        let status: i32 = 0;
        // int retcode, status;

        let mut line = null_mut::<u8>();
        loop {
            line = evbuffer_readln(
                (*event).input,
                null_mut(),
                evbuffer_eol_style_EVBUFFER_EOL_LF,
            );
            if !line.is_null() {
                cmd_run_shell_print(job, line);
                free_(line);
            }
            if line.is_null() {
                break;
            }
        }

        let size = EVBUFFER_LENGTH((*event).input);
        if size != 0 {
            line = xmalloc(size + 1).cast().as_ptr();
            memcpy(line.cast(), EVBUFFER_DATA((*event).input).cast(), size);
            *line.add(size) = b'\0';

            cmd_run_shell_print(job, line);

            free_(line);
        }

        let status = job_get_status(job);
        if WIFEXITED(status) {
            retcode = WEXITSTATUS(status);
            if retcode != 0 {
                msg = format_nul!("'{}' returned {}", _s(cmd), retcode);
            }
        } else if WIFSIGNALED(status) {
            retcode = WTERMSIG(status);
            msg = format_nul!("'{}' terminated by signal {}", _s(cmd), retcode);
            retcode += 128;
        } else {
            retcode = 0;
        }
        if !msg.is_null() {
            cmd_run_shell_print(job, msg);
        }
        free_(msg);

        if !item.is_null() {
            if !cmdq_get_client(item).is_null() && (*cmdq_get_client(item)).session.is_null() {
                (*cmdq_get_client(item)).retval = retcode;
            }
            cmdq_continue(item);
        }
    }
}

pub unsafe fn cmd_run_shell_free(data: *mut c_void) {
    unsafe {
        let __func__ = c!("cmd_run_shell_free");
        let cdata = data as *mut cmd_run_shell_data;

        evtimer_del(&raw mut (*cdata).timer);
        if !(*cdata).s.is_null() {
            session_remove_ref((*cdata).s, __func__);
        }
        if !(*cdata).client.is_null() {
            server_client_unref((*cdata).client);
        }
        if !(*cdata).state.is_null() {
            args_make_commands_free((*cdata).state);
        }
        free_((*cdata).cwd);
        free_((*cdata).cmd);
        free_(cdata);
    }
}

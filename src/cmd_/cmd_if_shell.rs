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
use crate::libc::{WEXITSTATUS, WIFEXITED};
use crate::*;

pub static CMD_IF_SHELL_ENTRY: cmd_entry = cmd_entry {
    name: "if-shell",
    alias: Some("if"),

    args: args_parse::new(c"bFt:", 2, 3, Some(cmd_if_shell_args_parse)),
    usage: "[-bF] [-t target-pane] shell-command command [command]",

    target: cmd_entry_flag::new(
        b't',
        cmd_find_type::CMD_FIND_PANE,
        cmd_find_flags::CMD_FIND_CANFAIL,
    ),

    flags: cmd_flag::empty(),
    exec: cmd_if_shell_exec,
    source: cmd_entry_flag::zeroed(),
};

#[repr(C)]
pub struct cmd_if_shell_data<'a> {
    pub cmd_if: *mut args_command_state<'a>,
    pub cmd_else: *mut args_command_state<'a>,

    pub client: *mut client,
    pub item: *mut cmdq_item,
}

fn cmd_if_shell_args_parse(_: *mut args, idx: u32, _: *mut *mut u8) -> args_parse_type {
    if idx == 1 || idx == 2 {
        args_parse_type::ARGS_PARSE_COMMANDS_OR_STRING
    } else {
        args_parse_type::ARGS_PARSE_STRING
    }
}

unsafe fn cmd_if_shell_exec(self_: *mut cmd, item: *mut cmdq_item) -> cmd_retval {
    unsafe {
        let args = cmd_get_args(self_);
        let target = cmdq_get_target(item);
        let tc = cmdq_get_target_client(item);
        let s = (*target).s;
        let count = args_count(args);
        let wait = !args_has(args, 'b');

        let shellcmd = format_single_from_target(item, args_string(args, 0));
        if args_has(args, 'F') {
            let cmdlist = if *shellcmd != b'0' as _ && *shellcmd != b'\0' as _ {
                args_make_commands_now(self_, item, 1, false)
            } else if count == 3 {
                args_make_commands_now(self_, item, 2, false)
            } else {
                free_(shellcmd);
                return cmd_retval::CMD_RETURN_NORMAL;
            };
            free_(shellcmd);
            if cmdlist.is_null() {
                return cmd_retval::CMD_RETURN_ERROR;
            }
            let new_item = cmdq_get_command(cmdlist, cmdq_get_state(item));
            cmdq_insert_after(item, new_item);
            return cmd_retval::CMD_RETURN_NORMAL;
        }

        let cdata = xcalloc_::<cmd_if_shell_data>(1).as_ptr();

        (*cdata).cmd_if = args_make_commands_prepare(self_, item, 1, null_mut(), wait, false);
        if count == 3 {
            (*cdata).cmd_else = args_make_commands_prepare(self_, item, 2, null_mut(), wait, false);
        }

        if wait {
            (*cdata).client = cmdq_get_client(item);
            (*cdata).item = item;
        } else {
            (*cdata).client = tc;
        }
        if !(*cdata).client.is_null() {
            (*(*cdata).client).references += 1;
        }

        if job_run(
            shellcmd,
            0,
            null_mut(),
            null_mut(),
            s,
            server_client_get_cwd(cmdq_get_client(item), s),
            None,
            Some(cmd_if_shell_callback),
            Some(cmd_if_shell_free),
            cdata as _,
            job_flag::empty(),
            -1,
            -1,
        )
        .is_null()
        {
            cmdq_error!(item, "failed to run command: {}", _s(shellcmd));
            free_(shellcmd);
            free_(cdata);
            return cmd_retval::CMD_RETURN_ERROR;
        }
        free_(shellcmd);

        if !wait {
            return cmd_retval::CMD_RETURN_NORMAL;
        }
        cmd_retval::CMD_RETURN_WAIT
    }
}

unsafe fn cmd_if_shell_callback(job: *mut job) {
    unsafe {
        let cdata = job_get_data(job) as *mut cmd_if_shell_data;
        let c = (*cdata).client;
        let item = (*cdata).item;
        let mut error: *mut u8 = null_mut();

        let state: *mut args_command_state;
        let status = job_get_status(job);

        'out: {
            if !WIFEXITED(status) || WEXITSTATUS(status) != 0 {
                state = (*cdata).cmd_else;
            } else {
                state = (*cdata).cmd_if;
            }
            if state.is_null() {
                break 'out;
            }

            let cmdlist = args_make_commands(state, 0, null_mut(), &raw mut error);
            if cmdlist.is_null() {
                if (*cdata).item.is_null() {
                    *error = (*error).to_ascii_uppercase();
                    status_message_set!(c, -1, 1, false, "{}", _s(error));
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
        }

        // out:
        if !(*cdata).item.is_null() {
            cmdq_continue((*cdata).item);
        }
    }
}

unsafe fn cmd_if_shell_free(data: *mut c_void) {
    unsafe {
        let cdata = data as *mut cmd_if_shell_data;

        if !(*cdata).client.is_null() {
            server_client_unref((*cdata).client);
        }

        if !(*cdata).cmd_else.is_null() {
            args_make_commands_free((*cdata).cmd_else);
        }
        args_make_commands_free((*cdata).cmd_if);

        free_(cdata);
    }
}

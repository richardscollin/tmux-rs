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

use crate::compat::queue::tailq_foreach;

pub static mut cmd_detach_client_entry: cmd_entry = cmd_entry {
    name: c"detach-client".as_ptr(),
    alias: c"detach".as_ptr(),

    args: args_parse::new(c"aE:s:t:P", 0, 0, None),
    usage: c"[-aP] [-E shell-command] [-s target-session] [-t target-client]".as_ptr(),

    source: cmd_entry_flag::new(b's', cmd_find_type::CMD_FIND_SESSION, CMD_FIND_CANFAIL),

    flags: cmd_flag::CMD_READONLY.union(cmd_flag::CMD_CLIENT_TFLAG),
    exec: Some(cmd_detach_client_exec),
    ..unsafe { zeroed() }
};

pub static mut cmd_suspend_client_entry: cmd_entry = cmd_entry {
    name: c"suspend-client".as_ptr(),
    alias: c"suspendc".as_ptr(),

    args: args_parse::new(c"t:", 0, 0, None),
    usage: c"[-t target-client]".as_ptr(),

    flags: cmd_flag::CMD_CLIENT_TFLAG,
    exec: Some(cmd_detach_client_exec),
    ..unsafe { zeroed() }
};

pub unsafe fn cmd_detach_client_exec(
    self_: *mut cmd,
    item: *mut cmdq_item,
) -> cmd_retval {
    unsafe {
        let args = cmd_get_args(self_);
        let source = cmdq_get_source(item);
        let tc = cmdq_get_target_client(item);
        let cmd = args_get(args, b'E');

        if cmd_get_entry(self_) == &raw mut cmd_suspend_client_entry {
            server_client_suspend(tc);
            return cmd_retval::CMD_RETURN_NORMAL;
        }

        let msgtype = if args_has(args, b'P') != 0 {
            msgtype::MSG_DETACHKILL
        } else {
            msgtype::MSG_DETACH
        };

        let mut s: *mut session = null_mut();
        if args_has(args, b's') != 0 {
            s = (*source).s;
            if s.is_null() {
                return cmd_retval::CMD_RETURN_NORMAL;
            }
            for loop_ in tailq_foreach(&raw mut clients).map(NonNull::as_ptr) {
                if (*loop_).session == s {
                    if !cmd.is_null() {
                        server_client_exec(loop_, cmd);
                    } else {
                        server_client_detach(loop_, msgtype);
                    }
                }
            }
            return cmd_retval::CMD_RETURN_STOP;
        }

        if args_has(args, b'a') != 0 {
            for loop_ in tailq_foreach(&raw mut clients).map(NonNull::as_ptr) {
                if !(*loop_).session.is_null() && loop_ != tc {
                    if !cmd.is_null() {
                        server_client_exec(loop_, cmd);
                    } else {
                        server_client_detach(loop_, msgtype);
                    }
                }
            }
            return cmd_retval::CMD_RETURN_NORMAL;
        }

        if !cmd.is_null() {
            server_client_exec(tc, cmd);
        } else {
            server_client_detach(tc, msgtype);
        }
        cmd_retval::CMD_RETURN_STOP
    }
}

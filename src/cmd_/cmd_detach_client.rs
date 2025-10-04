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
use crate::compat::queue::tailq_foreach;
use crate::*;

pub static CMD_DETACH_CLIENT_ENTRY: cmd_entry = cmd_entry {
    name: "detach-client",
    alias: Some("detach"),

    args: args_parse::new(c"aE:s:t:P", 0, 0, None),
    usage: SyncCharPtr::new(c"[-aP] [-E shell-command] [-s target-session] [-t target-client]"),

    source: cmd_entry_flag::new(
        b's',
        cmd_find_type::CMD_FIND_SESSION,
        cmd_find_flags::CMD_FIND_CANFAIL,
    ),

    flags: cmd_flag::CMD_READONLY.union(cmd_flag::CMD_CLIENT_TFLAG),
    exec: cmd_detach_client_exec,
    target: cmd_entry_flag::zeroed(),
};

pub static CMD_SUSPEND_CLIENT_ENTRY: cmd_entry = cmd_entry {
    name: "suspend-client",
    alias: Some("suspendc"),

    args: args_parse::new(c"t:", 0, 0, None),
    usage: SyncCharPtr::new(c"[-t target-client]"),

    flags: cmd_flag::CMD_CLIENT_TFLAG,
    exec: cmd_detach_client_exec,
    source: cmd_entry_flag::zeroed(),
    target: cmd_entry_flag::zeroed(),
};

pub unsafe fn cmd_detach_client_exec(self_: *mut cmd, item: *mut cmdq_item) -> cmd_retval {
    unsafe {
        let args = cmd_get_args(self_);
        let source = cmdq_get_source(item);
        let tc = cmdq_get_target_client(item);
        let cmd = args_get(args, b'E');

        if std::ptr::eq(cmd_get_entry(self_), &CMD_SUSPEND_CLIENT_ENTRY) {
            server_client_suspend(tc);
            return cmd_retval::CMD_RETURN_NORMAL;
        }

        let msgtype = if args_has(args, 'P') {
            msgtype::MSG_DETACHKILL
        } else {
            msgtype::MSG_DETACH
        };

        if args_has(args, 's') {
            let s = (*source).s;
            if s.is_null() {
                return cmd_retval::CMD_RETURN_NORMAL;
            }
            for loop_ in tailq_foreach(&raw mut CLIENTS).map(NonNull::as_ptr) {
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

        if args_has(args, 'a') {
            for loop_ in tailq_foreach(&raw mut CLIENTS).map(NonNull::as_ptr) {
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

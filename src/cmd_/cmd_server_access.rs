// Copyright (c) 2021 Dallas Lyons <dallasdlyons@gmail.com>
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
use crate::libc::{getpwnam, getuid};
use crate::*;

pub static CMD_SERVER_ACCESS_ENTRY: cmd_entry = cmd_entry {
    name: "server-access",
    alias: None,

    args: args_parse::new(c"adlrw", 0, 1, None),
    usage: "[-adlrw] [-t target-pane] [user]",

    flags: cmd_flag::CMD_CLIENT_CANFAIL,
    exec: cmd_server_access_exec,
    source: cmd_entry_flag::zeroed(),
    target: cmd_entry_flag::zeroed(),
};

unsafe fn cmd_server_access_deny(item: *mut cmdq_item, pw: *mut libc::passwd) -> cmd_retval {
    unsafe {
        let user = server_acl_user_find((*pw).pw_uid);
        if user.is_null() {
            cmdq_error!(item, "user {} not found", _s((*pw).pw_name));
            return cmd_retval::CMD_RETURN_ERROR;
        }
        for loop_ in tailq_foreach(&raw mut CLIENTS).map(NonNull::as_ptr) {
            let uid = proc_get_peer_uid((*loop_).peer);
            if uid == server_acl_get_uid(user) {
                (*loop_).exit_message = xstrdup_(c"access not allowed").as_ptr();
                (*loop_).flags |= client_flag::EXIT;
            }
        }
        server_acl_user_deny((*pw).pw_uid);

        cmd_retval::CMD_RETURN_NORMAL
    }
}

unsafe fn cmd_server_access_exec(self_: *mut cmd, item: *mut cmdq_item) -> cmd_retval {
    unsafe {
        let args = cmd_get_args(self_);
        let c = cmdq_get_target_client(item);

        if args_has(args, 'l') {
            server_acl_display(item);
            return cmd_retval::CMD_RETURN_NORMAL;
        }
        if args_count(args) == 0 {
            cmdq_error!(item, "missing user argument");
            return cmd_retval::CMD_RETURN_ERROR;
        }

        let name = format_single(
            item,
            args_string(args, 0),
            c,
            null_mut(),
            null_mut(),
            null_mut(),
        );
        let mut pw = null_mut();
        if *name != b'\0' as _ {
            pw = getpwnam(name.cast());
        }
        if pw.is_null() {
            cmdq_error!(item, "unknown user: {}", _s(name));
            return cmd_retval::CMD_RETURN_ERROR;
        }
        free_(name);

        if (*pw).pw_uid == 0 || (*pw).pw_uid == getuid() {
            cmdq_error!(
                item,
                "{} owns the server, can't change access",
                _s((*pw).pw_name),
            );
            return cmd_retval::CMD_RETURN_ERROR;
        }

        if args_has(args, 'a') && args_has(args, 'd') {
            cmdq_error!(item, "-a and -d cannot be used together");
            return cmd_retval::CMD_RETURN_ERROR;
        }
        if args_has(args, 'w') && args_has(args, 'r') {
            cmdq_error!(item, "-r and -w cannot be used together");
            return cmd_retval::CMD_RETURN_ERROR;
        }

        if args_has(args, 'd') {
            return cmd_server_access_deny(item, pw);
        }
        if args_has(args, 'a') {
            if !server_acl_user_find((*pw).pw_uid).is_null() {
                cmdq_error!(item, "user {} is already added", _s((*pw).pw_name));
                return cmd_retval::CMD_RETURN_ERROR;
            }
            server_acl_user_allow((*pw).pw_uid);
            // Do not return - allow -r or -w with -a.
        } else if (args_has(args, 'r') || args_has(args, 'w'))
            && server_acl_user_find((*pw).pw_uid).is_null()
        {
            server_acl_user_allow((*pw).pw_uid);
        } /* -r or -w implies -a if user does not exist. */

        if args_has(args, 'w') {
            if server_acl_user_find((*pw).pw_uid).is_null() {
                cmdq_error!(item, "user {} not found", _s((*pw).pw_name));
                return cmd_retval::CMD_RETURN_ERROR;
            }
            server_acl_user_allow_write((*pw).pw_uid);
            return cmd_retval::CMD_RETURN_NORMAL;
        }

        if args_has(args, 'r') {
            if server_acl_user_find((*pw).pw_uid).is_null() {
                cmdq_error!(item, "user {} not found", _s((*pw).pw_name));
                return cmd_retval::CMD_RETURN_ERROR;
            }
            server_acl_user_deny_write((*pw).pw_uid);
            return cmd_retval::CMD_RETURN_NORMAL;
        }

        cmd_retval::CMD_RETURN_NORMAL
    }
}

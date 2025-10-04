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

use crate::compat::tree::{rb_insert, rb_remove};
use crate::*;

pub static CMD_RENAME_SESSION_ENTRY: cmd_entry = cmd_entry {
    name: "rename-session",
    alias: Some("rename"),

    args: args_parse::new(c"t:", 1, 1, None),
    usage: SyncCharPtr::new(c"[-t target-session] new-name"),

    target: cmd_entry_flag::new(
        b't',
        cmd_find_type::CMD_FIND_SESSION,
        cmd_find_flags::empty(),
    ),
    source: cmd_entry_flag::zeroed(),

    flags: cmd_flag::CMD_AFTERHOOK,
    exec: cmd_rename_session_exec,
};

unsafe fn cmd_rename_session_exec(self_: *mut cmd, item: *mut cmdq_item) -> cmd_retval {
    unsafe {
        let args = cmd_get_args(self_);
        let target = cmdq_get_target(item);
        let s = (*target).s;

        let tmp = format_single_from_target(item, args_string(args, 0));
        let newname = session_check_name(tmp);
        if newname.is_null() {
            cmdq_error!(item, "invalid session: {}", _s(tmp));
            free_(tmp);
            return cmd_retval::CMD_RETURN_ERROR;
        }
        free_(tmp);
        if libc::strcmp(newname, (*s).name) == 0 {
            free_(newname);
            return cmd_retval::CMD_RETURN_NORMAL;
        }
        if !session_find(newname).is_null() {
            cmdq_error!(item, "duplicate session: {}", _s(newname));
            free_(newname);
            return cmd_retval::CMD_RETURN_ERROR;
        }

        rb_remove(&raw mut SESSIONS, s);
        free_((*s).name);
        (*s).name = newname;
        rb_insert(&raw mut SESSIONS, s);

        server_status_session(s);
        notify_session(c"session-renamed", s);
    }

    cmd_retval::CMD_RETURN_NORMAL
}

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

use crate::compat::tree::rb_foreach;

pub static CMD_KILL_SESSION_ENTRY: cmd_entry = cmd_entry {
    name: SyncCharPtr::new(c"kill-session"),
    alias: SyncCharPtr::null(),

    args: args_parse::new(c"aCt:", 0, 0, None),
    usage: SyncCharPtr::new(c"[-aC] [-t target-session]"),

    target: cmd_entry_flag::new(b't', cmd_find_type::CMD_FIND_SESSION, 0),
    source: cmd_entry_flag::zeroed(),

    flags: cmd_flag::empty(),
    exec: cmd_kill_session_exec,
};

unsafe fn cmd_kill_session_exec(self_: *mut cmd, item: *mut cmdq_item) -> cmd_retval {
    unsafe {
        let args = cmd_get_args(self_);
        let target = cmdq_get_target(item);
        let s = (*target).s;

        if args_has(args, b'C') != 0 {
            for wl in rb_foreach(&raw mut (*s).windows).map(NonNull::as_ptr) {
                (*(*wl).window).flags &= !WINDOW_ALERTFLAGS;
                (*wl).flags &= !WINLINK_ALERTFLAGS;
            }
            server_redraw_session(s);
        } else if args_has(args, b'a') != 0 {
            for sloop in rb_foreach(&raw mut SESSIONS).map(NonNull::as_ptr) {
                if sloop != s {
                    server_destroy_session(sloop);
                    session_destroy(sloop, 1, c!("cmd_kill_session_exec"));
                }
            }
        } else {
            server_destroy_session(s);
            session_destroy(s, 1, c!("cmd_kill_session_exec"));
        }
        cmd_retval::CMD_RETURN_NORMAL
    }
}

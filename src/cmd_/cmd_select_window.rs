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

pub static CMD_SELECT_WINDOW_ENTRY: cmd_entry = cmd_entry {
    name: "select-window",
    alias: Some("selectw"),

    args: args_parse::new(c"lnpTt:", 0, 0, None),
    usage: SyncCharPtr::new(c"[-lnpT] [-t target-window]"),

    target: cmd_entry_flag::new(
        b't',
        cmd_find_type::CMD_FIND_WINDOW,
        cmd_find_flags::empty(),
    ),

    flags: cmd_flag::empty(),
    exec: cmd_select_window_exec,
    source: cmd_entry_flag::zeroed(),
};

pub static CMD_NEXT_WINDOW_ENTRY: cmd_entry = cmd_entry {
    name: "next-window",
    alias: Some("next"),

    args: args_parse::new(c"at:", 0, 0, None),
    usage: SyncCharPtr::new(c"[-a] [-t target-session]"),

    target: cmd_entry_flag::new(
        b't',
        cmd_find_type::CMD_FIND_SESSION,
        cmd_find_flags::empty(),
    ),

    flags: cmd_flag::empty(),
    exec: cmd_select_window_exec,
    source: cmd_entry_flag::zeroed(),
};

pub static CMD_PREVIOUS_WINDOW_ENTRY: cmd_entry = cmd_entry {
    name: "previous-window",
    alias: Some("prev"),

    args: args_parse::new(c"at:", 0, 0, None),
    usage: SyncCharPtr::new(c"[-a] [-t target-session]"),

    target: cmd_entry_flag::new(
        b't',
        cmd_find_type::CMD_FIND_SESSION,
        cmd_find_flags::empty(),
    ),

    flags: cmd_flag::empty(),
    exec: cmd_select_window_exec,
    source: cmd_entry_flag::zeroed(),
};

pub static CMD_LAST_WINDOW_ENTRY: cmd_entry = cmd_entry {
    name: "last-window",
    alias: Some("last"),

    args: args_parse::new(c"t:", 0, 0, None),
    usage: SyncCharPtr::new(c"[-t target-session]"),

    target: cmd_entry_flag::new(
        b't',
        cmd_find_type::CMD_FIND_SESSION,
        cmd_find_flags::empty(),
    ),

    flags: cmd_flag::empty(),
    exec: cmd_select_window_exec,
    source: cmd_entry_flag::zeroed(),
};

unsafe fn cmd_select_window_exec(self_: *mut cmd, item: *mut cmdq_item) -> cmd_retval {
    unsafe {
        let args = cmd_get_args(self_);
        let c = cmdq_get_client(item);
        let current = cmdq_get_current(item);
        let target = cmdq_get_target(item);
        let wl = (*target).wl;
        let s = (*target).s;

        let mut next = std::ptr::eq(cmd_get_entry(self_), &CMD_NEXT_WINDOW_ENTRY);
        if args_has(args, 'n') {
            next = true;
        }
        let mut previous = std::ptr::eq(cmd_get_entry(self_), &CMD_PREVIOUS_WINDOW_ENTRY);
        if args_has(args, 'p') {
            previous = true;
        }
        let mut last = std::ptr::eq(cmd_get_entry(self_), &CMD_LAST_WINDOW_ENTRY);
        if args_has(args, 'l') {
            last = true;
        }

        if next || previous || last {
            let activity = args_has(args, 'a');
            #[expect(clippy::collapsible_else_if)]
            if next {
                if session_next(s, activity) != 0 {
                    cmdq_error!(item, "no next window");
                    return cmd_retval::CMD_RETURN_ERROR;
                }
            } else if previous {
                if session_previous(s, activity) != 0 {
                    cmdq_error!(item, "no previous window");
                    return cmd_retval::CMD_RETURN_ERROR;
                }
            } else {
                if session_last(s) != 0 {
                    cmdq_error!(item, "no last window");
                    return cmd_retval::CMD_RETURN_ERROR;
                }
            }
            cmd_find_from_session(current, s, cmd_find_flags::empty());
            server_redraw_session(s);
            cmdq_insert_hook!(s, item, current, "after-select-window");
        } else {
            // If -T and select-window is invoked on same window as
            // current, switch to previous window.
            if args_has(args, 'T') && wl == (*s).curw {
                if session_last(s) != 0 {
                    cmdq_error!(item, "no last window");
                    return cmd_retval::CMD_RETURN_ERROR;
                }
                if (*current).s == s {
                    cmd_find_from_session(current, s, cmd_find_flags::empty());
                }
                server_redraw_session(s);
            } else if session_select(s, (*wl).idx) == 0 {
                cmd_find_from_session(current, s, cmd_find_flags::empty());
                server_redraw_session(s);
            }
            cmdq_insert_hook!(s, item, current, "after-select-window");
        }
        if !c.is_null() && !(*c).session.is_null() {
            (*(*(*s).curw).window).latest = c as _;
        }
        recalculate_sizes();

        cmd_retval::CMD_RETURN_NORMAL
    }
}

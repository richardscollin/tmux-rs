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

pub static mut cmd_select_window_entry: cmd_entry = cmd_entry {
    name: c"select-window".as_ptr(),
    alias: c"selectw".as_ptr(),

    args: args_parse::new(c"lnpTt:", 0, 0, None),
    usage: c"[-lnpT] [-t target-window]".as_ptr(),

    target: cmd_entry_flag::new(b't', cmd_find_type::CMD_FIND_WINDOW, 0),

    flags: cmd_flag::empty(),
    exec: Some(cmd_select_window_exec),
    ..unsafe { zeroed() }
};

pub static mut cmd_next_window_entry: cmd_entry = cmd_entry {
    name: c"next-window".as_ptr(),
    alias: c"next".as_ptr(),

    args: args_parse::new(c"at:", 0, 0, None),
    usage: c"[-a] [-t target-session]".as_ptr(),

    target: cmd_entry_flag::new(b't', cmd_find_type::CMD_FIND_SESSION, 0),

    flags: cmd_flag::empty(),
    exec: Some(cmd_select_window_exec),
    ..unsafe { zeroed() }
};

pub static mut cmd_previous_window_entry: cmd_entry = cmd_entry {
    name: c"previous-window".as_ptr(),
    alias: c"prev".as_ptr(),

    args: args_parse::new(c"at:", 0, 0, None),
    usage: c"[-a] [-t target-session]".as_ptr(),

    target: cmd_entry_flag::new(b't', cmd_find_type::CMD_FIND_SESSION, 0),

    flags: cmd_flag::empty(),
    exec: Some(cmd_select_window_exec),
    ..unsafe { zeroed() }
};

pub static mut cmd_last_window_entry: cmd_entry = cmd_entry {
    name: c"last-window".as_ptr(),
    alias: c"last".as_ptr(),

    args: args_parse::new(c"t:", 0, 0, None),
    usage: c"[-t target-session]".as_ptr(),

    target: cmd_entry_flag::new(b't', cmd_find_type::CMD_FIND_SESSION, 0),

    flags: cmd_flag::empty(),
    exec: Some(cmd_select_window_exec),
    ..unsafe { zeroed() }
};

unsafe fn cmd_select_window_exec(self_: *mut cmd, item: *mut cmdq_item) -> cmd_retval {
    unsafe {
        let args = cmd_get_args(self_);
        let c = cmdq_get_client(item);
        let current = cmdq_get_current(item);
        let target = cmdq_get_target(item);
        let wl = (*target).wl;
        let s = (*target).s;
        //int			 next, previous, last, activity;

        let mut next = cmd_get_entry(self_) == &raw mut cmd_next_window_entry;
        if args_has_(args, 'n') {
            next = true;
        }
        let mut previous = cmd_get_entry(self_) == &raw mut cmd_previous_window_entry;
        if args_has_(args, 'p') {
            previous = true;
        }
        let mut last = cmd_get_entry(self_) == &raw mut cmd_last_window_entry;
        if args_has_(args, 'l') {
            last = true;
        }

        if next || previous || last {
            let activity = args_has(args, b'a');
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
                #[allow(clippy::collapsible_else_if)]
                if session_last(s) != 0 {
                    cmdq_error!(item, "no last window");
                    return cmd_retval::CMD_RETURN_ERROR;
                }
            }
            cmd_find_from_session(current, s, 0);
            server_redraw_session(s);
            cmdq_insert_hook!(s, item, current, "after-select-window");
        } else {
            /*
             * If -T and select-window is invoked on same window as
             * current, switch to previous window.
             */
            if args_has_(args, 'T') && wl == (*s).curw {
                if session_last(s) != 0 {
                    cmdq_error!(item, "no last window");
                    return cmd_retval::CMD_RETURN_ERROR;
                }
                if (*current).s == s {
                    cmd_find_from_session(current, s, 0);
                }
                server_redraw_session(s);
            } else if session_select(s, (*wl).idx) == 0 {
                cmd_find_from_session(current, s, 0);
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

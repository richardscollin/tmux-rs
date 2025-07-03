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
use crate::*;

use crate::compat::queue::{tailq_first, tailq_foreach, tailq_next, tailq_prev};

pub static mut cmd_select_pane_entry: cmd_entry = cmd_entry {
    name: c"select-pane".as_ptr(),
    alias: c"selectp".as_ptr(),

    args: args_parse::new(c"DdegLlMmP:RT:t:UZ", 0, 0, None), /* -P and -g deprecated */
    usage: c"[-DdeLlMmRUZ] [-T title] [-t target-pane]".as_ptr(),

    target: cmd_entry_flag::new(b't', cmd_find_type::CMD_FIND_PANE, 0),

    flags: cmd_flag::empty(),
    exec: Some(cmd_select_pane_exec),

    ..unsafe { zeroed() }
};

pub static mut cmd_last_pane_entry: cmd_entry = cmd_entry {
    name: c"last-pane".as_ptr(),
    alias: c"lastp".as_ptr(),

    args: args_parse::new(c"det:Z", 0, 0, None),
    usage: c"[-deZ] [-t target-window]".as_ptr(),

    target: cmd_entry_flag::new(b't', cmd_find_type::CMD_FIND_WINDOW, 0),

    flags: cmd_flag::empty(),
    exec: Some(cmd_select_pane_exec),
    ..unsafe { zeroed() }
};

pub unsafe fn cmd_select_pane_redraw(w: *mut window) {
    unsafe {
        /*
         * Redraw entire window if it is bigger than the client (the
         * offset may change), otherwise just draw borders.
         */

        for c in tailq_foreach(&raw mut clients).map(NonNull::as_ptr) {
            if (*c).session.is_null() || ((*c).flags.intersects(client_flag::CONTROL)) {
                continue;
            }
            if (*(*(*c).session).curw).window == w && tty_window_bigger(&raw mut (*c).tty) {
                server_redraw_client(c);
            } else {
                if (*(*(*c).session).curw).window == w {
                    (*c).flags |= client_flag::REDRAWBORDERS;
                }
                if session_has((*c).session, w) != 0 {
                    (*c).flags |= client_flag::REDRAWSTATUS;
                }
            }
        }
    }
}

pub unsafe fn cmd_select_pane_exec(self_: *mut cmd, item: *mut cmdq_item) -> cmd_retval {
    unsafe {
        let args = cmd_get_args(self_);
        let entry = cmd_get_entry(self_);
        let current = cmdq_get_current(item);
        let target = cmdq_get_target(item);
        let c = cmdq_get_client(item);
        let wl = (*target).wl;
        let w = (*wl).window;
        let s = (*target).s;
        let mut wp = (*target).wp;
        let oo = (*wp).options;

        let mut activewp = null_mut();
        let mut lastwp: *mut window_pane = null_mut();
        let mut markedwp = null_mut();

        if entry == &raw mut cmd_last_pane_entry || args_has_(args, 'l') {
            /*
             * Check for no last pane found in case the other pane was
             * spawned without being visited (for example split-window -d).
             */
            lastwp = tailq_first(&raw mut (*w).last_panes);
            if lastwp.is_null() && window_count_panes(w) == 2 {
                lastwp = tailq_prev::<_, _, discr_entry>((*w).active);
                if lastwp.is_null() {
                    lastwp = tailq_next::<_, _, discr_entry>((*w).active);
                }
            }
            if lastwp.is_null() {
                cmdq_error!(item, "no last pane");
                return cmd_retval::CMD_RETURN_ERROR;
            }
            if args_has_(args, 'e') {
                (*lastwp).flags &= !window_pane_flags::PANE_INPUTOFF;
                server_redraw_window_borders((*lastwp).window);
                server_status_window((*lastwp).window);
            } else if args_has_(args, 'd') {
                (*lastwp).flags |= window_pane_flags::PANE_INPUTOFF;
                server_redraw_window_borders((*lastwp).window);
                server_status_window((*lastwp).window);
            } else {
                if window_push_zoom(w, 0, args_has(args, b'Z')) != 0 {
                    server_redraw_window(w);
                }
                window_redraw_active_switch(w, lastwp);
                if window_set_active_pane(w, lastwp, 1) != 0 {
                    cmd_find_from_winlink(current, wl, 0);
                    cmd_select_pane_redraw(w);
                }
                if window_pop_zoom(w) != 0 {
                    server_redraw_window(w);
                }
            }
            return cmd_retval::CMD_RETURN_NORMAL;
        }

        if args_has_(args, 'm') || args_has_(args, 'M') {
            if args_has_(args, 'm') && window_pane_visible(wp) == 0 {
                return cmd_retval::CMD_RETURN_NORMAL;
            }
            if server_check_marked() {
                lastwp = marked_pane.wp;
            } else {
                lastwp = null_mut();
            }

            if args_has_(args, 'M') || server_is_marked(s, wl, wp) {
                server_clear_marked();
            } else {
                server_set_marked(s, wl, wp);
            }
            markedwp = marked_pane.wp;

            if !lastwp.is_null() {
                (*lastwp).flags |=
                    window_pane_flags::PANE_REDRAW | window_pane_flags::PANE_STYLECHANGED;
                server_redraw_window_borders((*lastwp).window);
                server_status_window((*lastwp).window);
            }
            if !markedwp.is_null() {
                (*markedwp).flags |=
                    window_pane_flags::PANE_REDRAW | window_pane_flags::PANE_STYLECHANGED;
                server_redraw_window_borders((*markedwp).window);
                server_status_window((*markedwp).window);
            }
            return cmd_retval::CMD_RETURN_NORMAL;
        }

        let style = args_get(args, b'P');
        if !style.is_null() {
            let o = options_set_string!(oo, c"window-style".as_ptr(), 0, "{}", _s(style));
            if o.is_null() {
                cmdq_error!(item, "bad style: {}", _s(style));
                return cmd_retval::CMD_RETURN_ERROR;
            }
            options_set_string!(oo, c"window-active-style".as_ptr(), 0, "{}", _s(style),);
            (*wp).flags |= window_pane_flags::PANE_REDRAW | window_pane_flags::PANE_STYLECHANGED;
        }
        if args_has_(args, 'g') {
            cmdq_print!(item, "{}", _s(options_get_string_(oo, c"window-style")),);
            return cmd_retval::CMD_RETURN_NORMAL;
        }

        if args_has_(args, 'L') {
            window_push_zoom(w, 0, 1);
            wp = window_pane_find_left(wp);
            window_pop_zoom(w);
        } else if args_has_(args, 'R') {
            window_push_zoom(w, 0, 1);
            wp = window_pane_find_right(wp);
            window_pop_zoom(w);
        } else if args_has_(args, 'U') {
            window_push_zoom(w, 0, 1);
            wp = window_pane_find_up(wp);
            window_pop_zoom(w);
        } else if args_has_(args, 'D') {
            window_push_zoom(w, 0, 1);
            wp = window_pane_find_down(wp);
            window_pop_zoom(w);
        }
        if wp.is_null() {
            return cmd_retval::CMD_RETURN_NORMAL;
        }

        if args_has_(args, 'e') {
            (*wp).flags &= !window_pane_flags::PANE_INPUTOFF;
            server_redraw_window_borders((*wp).window);
            server_status_window((*wp).window);
            return cmd_retval::CMD_RETURN_NORMAL;
        }
        if args_has_(args, 'd') {
            (*wp).flags |= window_pane_flags::PANE_INPUTOFF;
            server_redraw_window_borders((*wp).window);
            server_status_window((*wp).window);
            return cmd_retval::CMD_RETURN_NORMAL;
        }

        if args_has_(args, 'T') {
            let title = format_single_from_target(item, args_get_(args, 'T'));
            if screen_set_title(&raw mut (*wp).base, title) != 0 {
                notify_pane(c"pane-title-changed", wp);
                server_redraw_window_borders((*wp).window);
                server_status_window((*wp).window);
            }
            free_(title);
            return cmd_retval::CMD_RETURN_NORMAL;
        }

        if !c.is_null()
            && !(*c).session.is_null()
            && ((*c).flags.intersects(client_flag::ACTIVEPANE))
        {
            activewp = server_client_get_pane(c);
        } else {
            activewp = (*w).active;
        }
        if wp == activewp {
            return cmd_retval::CMD_RETURN_NORMAL;
        }
        if window_push_zoom(w, 0, args_has(args, b'Z')) != 0 {
            server_redraw_window(w);
        }
        window_redraw_active_switch(w, wp);
        if !c.is_null()
            && !(*c).session.is_null()
            && ((*c).flags.intersects(client_flag::ACTIVEPANE))
        {
            server_client_set_pane(c, wp);
        } else if window_set_active_pane(w, wp, 1) != 0 {
            cmd_find_from_winlink_pane(current, wl, wp, 0);
        }
        cmdq_insert_hook!(s, item, current, "after-select-pane");
        cmd_select_pane_redraw(w);
        if window_pop_zoom(w) != 0 {
            server_redraw_window(w);
        }

        cmd_retval::CMD_RETURN_NORMAL
    }
}

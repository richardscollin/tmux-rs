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

use crate::compat::{queue::tailq_remove, tailq_insert_head};

pub static mut cmd_break_pane_entry: cmd_entry = cmd_entry {
    name: c"break-pane".as_ptr(),
    alias: c"breakp".as_ptr(),

    args: args_parse::new(c"abdPF:n:s:t:", 0, 0, None),
    usage: c"[-abdP] [-F format] [-n window-name] [-s src-pane] [-t dst-window]".as_ptr(),

    source: cmd_entry_flag::new(b's', cmd_find_type::CMD_FIND_PANE, 0),
    target: cmd_entry_flag::new(b't', cmd_find_type::CMD_FIND_WINDOW, CMD_FIND_WINDOW_INDEX),

    flags: cmd_flag::empty(),
    exec: Some(cmd_break_pane_exec),
};

pub unsafe fn cmd_break_pane_exec(self_: *mut cmd, item: *mut cmdq_item) -> cmd_retval {
    unsafe {
        let args = cmd_get_args(self_);
        let current = cmdq_get_current(item);
        let target = cmdq_get_target(item);
        let source = cmdq_get_source(item);
        let tc = cmdq_get_target_client(item);
        let mut wl = (*source).wl;
        let src_s = (*source).s;
        let dst_s = (*target).s;
        let wp = (*source).wp;
        let mut w = (*wl).window;

        let mut name: *mut c_char = null_mut();
        let mut cause: *mut c_char = null_mut();
        let mut cp: *mut c_char = null_mut();
        let mut idx = (*target).idx;
        let mut template: *const c_char = null_mut();

        let before = args_has(args, b'b');
        if args_has(args, b'a') != 0 || before != 0 {
            idx = if !(*target).wl.is_null() {
                winlink_shuffle_up(dst_s, (*target).wl, before)
            } else {
                winlink_shuffle_up(dst_s, (*dst_s).curw, before)
            };
            if idx == -1 {
                return cmd_retval::CMD_RETURN_ERROR;
            }
        }
        server_unzoom_window(w);

        if window_count_panes(w) == 1 {
            if server_link_window(
                src_s,
                wl,
                dst_s,
                idx,
                0,
                !args_has(args, b'd'),
                &raw mut cause,
            ) != 0
            {
                cmdq_error!(item, "{}", _s(cause));
                free_(cause);
                return cmd_retval::CMD_RETURN_ERROR;
            }
            if args_has(args, b'n') != 0 {
                window_set_name(w, args_get(args, b'n'));
                options_set_number((*w).options, c"automatic-rename".as_ptr(), 0);
            }
            server_unlink_window(src_s, wl);
            return cmd_retval::CMD_RETURN_NORMAL;
        }
        if idx != -1 && !winlink_find_by_index(&raw mut (*dst_s).windows, idx).is_null() {
            cmdq_error!(item, "index in use: {}", idx);
            return cmd_retval::CMD_RETURN_ERROR;
        }

        tailq_remove::<_, discr_entry>(&raw mut (*w).panes, wp);
        server_client_remove_pane(wp);
        window_lost_pane(w, wp);
        layout_close_pane(wp);

        (*wp).window = window_create((*w).sx, (*w).sy, (*w).xpixel, (*w).ypixel);
        w = (*wp).window;

        options_set_parent((*wp).options, (*w).options);
        (*wp).flags |= window_pane_flags::PANE_STYLECHANGED;
        tailq_insert_head::<_, discr_entry>(&raw mut (*w).panes, wp);
        (*w).active = wp;
        (*w).latest = tc as *mut c_void;

        if args_has(args, b'n') == 0 {
            name = default_window_name(w);
            window_set_name(w, name);
            free_(name);
        } else {
            window_set_name(w, args_get(args, b'n'));
            options_set_number((*w).options, c"automatic-rename".as_ptr(), 0);
        }

        layout_init(w, wp);
        (*wp).flags |= window_pane_flags::PANE_CHANGED;
        colour_palette_from_option(&raw mut (*wp).palette, (*wp).options);

        if idx == -1 {
            idx = -1 - options_get_number_((*dst_s).options, c"base-index") as i32;
        }
        wl = session_attach(dst_s, w, idx, &raw mut cause);
        if args_has(args, b'd') == 0 {
            session_select(dst_s, (*wl).idx);
            cmd_find_from_session(current, dst_s, 0);
        }

        server_redraw_session(src_s);
        if src_s != dst_s {
            server_redraw_session(dst_s);
        }
        server_status_session_group(src_s);
        if src_s != dst_s {
            server_status_session_group(dst_s);
        }

        if args_has(args, b'P') != 0 {
            template = args_get(args, b'F');
            if template.is_null() {
                template = c"#{session_name}:#{window_index}.#{pane_index}".as_ptr();
            }
            cp = format_single(item, template, tc, dst_s, wl, wp);
            cmdq_print!(item, "{}", _s(cp));
            free_(cp);
        }

        cmd_retval::CMD_RETURN_NORMAL
    }
}

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

use crate::compat::queue::{
    tailq_first, tailq_insert_after, tailq_insert_head, tailq_last, tailq_next, tailq_prev,
    tailq_remove, tailq_replace,
};

pub static CMD_SWAP_PANE_ENTRY: cmd_entry = cmd_entry {
    name: SyncCharPtr::new(c"swap-pane"),
    alias: SyncCharPtr::new(c"swapp"),

    args: args_parse::new(c"dDs:t:UZ", 0, 0, None),
    usage: SyncCharPtr::new(c"[-dDUZ] [-s src-window] [-t dst-window]"),

    source: cmd_entry_flag::new(
        b's',
        cmd_find_type::CMD_FIND_PANE,
        cmd_find_flags::CMD_FIND_DEFAULT_MARKED,
    ),
    target: cmd_entry_flag::new(b't', cmd_find_type::CMD_FIND_PANE, cmd_find_flags::empty()),

    flags: cmd_flag::empty(),
    exec: cmd_swap_pane_exec,
};

unsafe fn cmd_swap_pane_exec(self_: *mut cmd, item: *mut cmdq_item) -> cmd_retval {
    unsafe {
        let args = cmd_get_args(self_);
        let source = cmdq_get_source(item);
        let target = cmdq_get_target(item);

        let dst_w = (*(*target).wl).window;
        let dst_wp = (*target).wp;
        let mut src_w = (*(*source).wl).window;
        let mut src_wp = (*source).wp;

        if window_push_zoom(dst_w, 0, args_has(args, b'Z')) != 0 {
            server_redraw_window(dst_w);
        }

        'out: {
            if args_has_(args, 'D') {
                src_w = dst_w;
                src_wp = tailq_next::<_, _, discr_entry>(dst_wp);
                if src_wp.is_null() {
                    src_wp = tailq_first(&raw mut (*dst_w).panes);
                }
            } else if args_has_(args, 'U') {
                src_w = dst_w;
                src_wp = tailq_prev::<_, _, discr_entry>(dst_wp);
                if src_wp.is_null() {
                    src_wp = tailq_last(&raw mut (*dst_w).panes);
                }
            }

            if src_w != dst_w && window_push_zoom(src_w, 0, args_has(args, b'Z')) != 0 {
                server_redraw_window(src_w);
            }

            if src_wp == dst_wp {
                break 'out;
            }

            server_client_remove_pane(src_wp);
            server_client_remove_pane(dst_wp);

            let mut tmp_wp = tailq_prev::<_, _, discr_entry>(dst_wp);
            tailq_remove::<_, discr_entry>(&raw mut (*dst_w).panes, dst_wp);
            tailq_replace::<_, discr_entry>(&raw mut (*src_w).panes, src_wp, dst_wp);
            if tmp_wp == src_wp {
                tmp_wp = dst_wp;
            }
            if tmp_wp.is_null() {
                tailq_insert_head::<_, discr_entry>(&raw mut (*dst_w).panes, src_wp);
            } else {
                tailq_insert_after::<_, discr_entry>(&raw mut (*dst_w).panes, tmp_wp, src_wp);
            }

            let src_lc = (*src_wp).layout_cell;
            let dst_lc = (*dst_wp).layout_cell;
            (*src_lc).wp = dst_wp;
            (*dst_wp).layout_cell = src_lc;
            (*dst_lc).wp = src_wp;
            (*src_wp).layout_cell = dst_lc;

            (*src_wp).window = dst_w;
            options_set_parent((*src_wp).options, (*dst_w).options);
            (*src_wp).flags |= window_pane_flags::PANE_STYLECHANGED;
            (*dst_wp).window = src_w;
            options_set_parent((*dst_wp).options, (*src_w).options);
            (*dst_wp).flags |= window_pane_flags::PANE_STYLECHANGED;

            let sx = (*src_wp).sx;
            let sy = (*src_wp).sy;
            let xoff = (*src_wp).xoff;
            let yoff = (*src_wp).yoff;
            (*src_wp).xoff = (*dst_wp).xoff;
            (*src_wp).yoff = (*dst_wp).yoff;
            window_pane_resize(src_wp, (*dst_wp).sx, (*dst_wp).sy);
            (*dst_wp).xoff = xoff;
            (*dst_wp).yoff = yoff;
            window_pane_resize(dst_wp, sx, sy);

            if !args_has_(args, 'd') {
                if src_w != dst_w {
                    window_set_active_pane(src_w, dst_wp, 1);
                    window_set_active_pane(dst_w, src_wp, 1);
                } else {
                    tmp_wp = dst_wp;
                    window_set_active_pane(src_w, tmp_wp, 1);
                }
            } else {
                if (*src_w).active == src_wp {
                    window_set_active_pane(src_w, dst_wp, 1);
                }
                if (*dst_w).active == dst_wp {
                    window_set_active_pane(dst_w, src_wp, 1);
                }
            }
            if src_w != dst_w {
                window_pane_stack_remove(&raw mut (*src_w).last_panes, src_wp);
                window_pane_stack_remove(&raw mut (*dst_w).last_panes, dst_wp);
                colour_palette_from_option(&raw mut (*src_wp).palette, (*src_wp).options);
                colour_palette_from_option(&raw mut (*dst_wp).palette, (*dst_wp).options);
            }
            server_redraw_window(src_w);
            server_redraw_window(dst_w);
            notify_window(c"window-layout-changed", src_w);
            if src_w != dst_w {
                notify_window(c"window-layout-changed", dst_w);
            }
        }

        if window_pop_zoom(src_w) != 0 {
            server_redraw_window(src_w);
        }
        if src_w != dst_w && window_pop_zoom(dst_w) != 0 {
            server_redraw_window(dst_w);
        }

        cmd_retval::CMD_RETURN_NORMAL
    }
}

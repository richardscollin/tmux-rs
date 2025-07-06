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
    tailq_first, tailq_foreach, tailq_foreach_reverse, tailq_insert_head, tailq_insert_tail,
    tailq_last, tailq_next, tailq_prev, tailq_remove,
};

pub static CMD_ROTATE_WINDOW_ENTRY: cmd_entry = cmd_entry {
    name: SyncCharPtr::new(c"rotate-window"),
    alias: SyncCharPtr::new(c"rotatew"),

    args: args_parse::new(c"Dt:UZ", 0, 0, None),
    usage: SyncCharPtr::new(c"[-DUZ] [-t target-window]"),

    target: cmd_entry_flag::new(b't', cmd_find_type::CMD_FIND_WINDOW, 0),

    flags: cmd_flag::empty(),
    exec: cmd_rotate_window_exec,
    source: cmd_entry_flag::zeroed(),
};

unsafe fn cmd_rotate_window_exec(self_: *mut cmd, item: *mut cmdq_item) -> cmd_retval {
    unsafe {
        let args = cmd_get_args(self_);
        let current = cmdq_get_current(item);
        let target = cmdq_get_target(item);
        let wl = (*target).wl;
        let w = (*wl).window;
        let mut wp: *mut window_pane;
        let mut wp2: *mut window_pane;
        let lc: *mut layout_cell;
        let sx: u32;
        let sy: u32;
        let xoff: u32;
        let yoff: u32;

        window_push_zoom(w, 0, args_has(args, b'Z'));

        if args_has_(args, 'D') {
            wp = tailq_last(&raw mut (*w).panes);
            tailq_remove::<_, discr_entry>(&raw mut (*w).panes, wp);
            tailq_insert_head::<_, discr_entry>(&raw mut (*w).panes, wp);

            lc = (*wp).layout_cell;
            xoff = (*wp).xoff;
            yoff = (*wp).yoff;

            sx = (*wp).sx;
            sy = (*wp).sy;

            for wp_ in tailq_foreach::<_, discr_entry>(&raw mut (*w).panes).map(NonNull::as_ptr) {
                wp = wp_;
                let wp2 = tailq_next::<_, _, discr_entry>(wp);
                if wp2.is_null() {
                    break;
                }
                (*wp).layout_cell = (*wp2).layout_cell;
                if !(*wp).layout_cell.is_null() {
                    (*(*wp).layout_cell).wp = wp;
                }
                (*wp).xoff = (*wp2).xoff;
                (*wp).yoff = (*wp2).yoff;
                window_pane_resize(wp, (*wp2).sx, (*wp2).sy);
            }
            (*wp).layout_cell = lc;
            if !(*wp).layout_cell.is_null() {
                (*(*wp).layout_cell).wp = wp;
            }
            (*wp).xoff = xoff;
            (*wp).yoff = yoff;
            window_pane_resize(wp, sx, sy);

            wp = tailq_prev::<_, _, discr_entry>((*w).active);
            if wp.is_null() {
                wp = tailq_last(&raw mut (*w).panes);
            }
        } else {
            wp = tailq_first(&raw mut (*w).panes);
            tailq_remove::<_, discr_entry>(&raw mut (*w).panes, wp);
            tailq_insert_tail::<_, discr_entry>(&raw mut (*w).panes, wp);

            lc = (*wp).layout_cell;
            xoff = (*wp).xoff;
            yoff = (*wp).yoff;
            sx = (*wp).sx;
            sy = (*wp).sy;
            for wp_ in
                tailq_foreach_reverse::<_, discr_entry>(&raw mut (*w).panes).map(NonNull::as_ptr)
            {
                wp = wp_;
                let wp2 = tailq_prev::<_, _, discr_entry>(wp);
                if wp2.is_null() {
                    break;
                }
                (*wp).layout_cell = (*wp2).layout_cell;
                if !(*wp).layout_cell.is_null() {
                    (*(*wp).layout_cell).wp = wp;
                }
                (*wp).xoff = (*wp2).xoff;
                (*wp).yoff = (*wp2).yoff;
                window_pane_resize(wp, (*wp2).sx, (*wp2).sy);
            }
            (*wp).layout_cell = lc;
            if !(*wp).layout_cell.is_null() {
                (*(*wp).layout_cell).wp = wp;
            }
            (*wp).xoff = xoff;
            (*wp).yoff = yoff;
            window_pane_resize(wp, sx, sy);

            wp = tailq_next::<_, _, discr_entry>((*w).active);
            if wp.is_null() {
                wp = tailq_first(&raw mut (*w).panes);
            }
        }

        window_set_active_pane(w, wp, 1);
        cmd_find_from_winlink_pane(current, wl, wp, 0);
        window_pop_zoom(w);
        server_redraw_window(w);

        cmd_retval::CMD_RETURN_NORMAL
    }
}

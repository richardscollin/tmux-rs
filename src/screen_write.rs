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

use crate::compat::{
    TAILQ_HEAD_INITIALIZER, impl_tailq_entry,
    queue::{
        tailq_concat, tailq_empty, tailq_init, tailq_insert_after, tailq_insert_before,
        tailq_insert_tail, tailq_remove,
    },
};
use crate::options_::options_get_number_;

#[repr(i32)]
#[derive(Copy, Clone, Eq, PartialEq)]
pub enum screen_write_citem_type {
    Text,
    Clear,
}

impl_tailq_entry!(screen_write_citem, entry, tailq_entry<screen_write_citem>);
#[repr(C)]
pub struct screen_write_citem {
    x: u32,
    wrapped: i32,

    type_: screen_write_citem_type,
    used: u32,
    bg: u32,

    gc: grid_cell,

    entry: tailq_entry<screen_write_citem>,
}

#[repr(C)]
pub struct screen_write_cline {
    data: *mut u8,
    items: tailq_head<screen_write_citem>,
}

pub static mut SCREEN_WRITE_CITEM_FREELIST: tailq_head<screen_write_citem> =
    TAILQ_HEAD_INITIALIZER!(SCREEN_WRITE_CITEM_FREELIST);

unsafe fn screen_write_get_citem() -> NonNull<screen_write_citem> {
    unsafe {
        if let Some(ci) = NonNull::new(tailq_first(&raw mut SCREEN_WRITE_CITEM_FREELIST)) {
            tailq_remove(&raw mut SCREEN_WRITE_CITEM_FREELIST, ci.as_ptr());
            memset0(ci.as_ptr());
            return ci;
        }
        NonNull::new(xcalloc1::<screen_write_citem>()).unwrap()
    }
}

unsafe fn screen_write_free_citem(ci: *mut screen_write_citem) {
    unsafe {
        tailq_insert_tail(&raw mut SCREEN_WRITE_CITEM_FREELIST, ci);
    }
}

unsafe extern "C-unwind" fn screen_write_offset_timer(_fd: i32, _events: i16, data: *mut c_void) {
    unsafe {
        tty_update_window_offset(data.cast());
    }
}

/// Set cursor position.
unsafe fn screen_write_set_cursor(ctx: *mut screen_write_ctx, mut cx: i32, mut cy: i32) {
    unsafe {
        let wp = (*ctx).wp;
        let s = (*ctx).s;
        let tv: timeval = timeval {
            tv_usec: 10000,
            tv_sec: 0,
        };

        if cx != -1 && cx as u32 == (*s).cx && cy != -1 && cy as u32 == (*s).cy {
            return;
        }

        if cx != -1 {
            if cx as u32 > screen_size_x(s) {
                cx = screen_size_x(s) as i32 - 1;
            } // allow last column
            (*s).cx = cx as u32;
        }
        if cy != -1 {
            if cy as u32 > screen_size_y(s) - 1 {
                cy = screen_size_y(s) as i32 - 1;
            }
            (*s).cy = cy as u32;
        }

        if wp.is_null() {
            return;
        }
        let w = (*wp).window;

        if event_initialized(&raw mut (*w).offset_timer) == 0 {
            evtimer_set(
                &raw mut (*w).offset_timer,
                Some(screen_write_offset_timer),
                w.cast(),
            );
        }
        if evtimer_pending(&raw mut (*w).offset_timer, null_mut()) == 0 {
            evtimer_add(&raw mut (*w).offset_timer, &raw const tv);
        }
    }
}

/// Do a full redraw.
unsafe fn screen_write_redraw_cb(ttyctx: *const tty_ctx) {
    unsafe {
        let wp: *mut window_pane = (*ttyctx).arg.cast();

        if !wp.is_null() {
            (*wp).flags |= window_pane_flags::PANE_REDRAW;
        }
    }
}

/// Update context for client.
unsafe fn screen_write_set_client_cb(ttyctx: *mut tty_ctx, c: *mut client) -> i32 {
    unsafe {
        let wp: *mut window_pane = (*ttyctx).arg.cast();

        if (*ttyctx).allow_invisible_panes != 0 {
            if session_has((*c).session, (*wp).window) != 0 {
                return 1;
            }
            return 0;
        }

        if (*(*(*c).session).curw).window != (*wp).window {
            return 0;
        }
        if (*wp).layout_cell.is_null() {
            return 0;
        }

        if (*wp)
            .flags
            .intersects(window_pane_flags::PANE_REDRAW | window_pane_flags::PANE_DROP)
        {
            return -1;
        }
        if (*c).flags.intersects(client_flag::REDRAWPANES) {
            /*
             * Redraw is already deferred to redraw another pane - redraw
             * this one also when that happens.
             */
            // log_debug("%s: adding %%%u to deferred redraw", __func__, (*wp).id);
            (*wp).flags |= window_pane_flags::PANE_REDRAW;
            return -1;
        }

        (*ttyctx).bigger = tty_window_offset(
            &raw mut (*c).tty,
            &raw mut (*ttyctx).wox,
            &raw mut (*ttyctx).woy,
            &raw mut (*ttyctx).wsx,
            &raw mut (*ttyctx).wsy,
        );

        (*ttyctx).rxoff = (*wp).xoff;
        (*ttyctx).xoff = (*wp).xoff;

        (*ttyctx).ryoff = (*wp).yoff;
        (*ttyctx).yoff = (*wp).yoff;

        if status_at_line(c) == 0 {
            (*ttyctx).yoff += status_line_size(c);
        }

        1
    }
}

/// Set up context for TTY command.
unsafe fn screen_write_initctx(ctx: *mut screen_write_ctx, ttyctx: *mut tty_ctx, sync: i32) {
    unsafe {
        let s = (*ctx).s;

        memset0(ttyctx);

        (*ttyctx).s = s;
        (*ttyctx).sx = screen_size_x(s);
        (*ttyctx).sy = screen_size_y(s);

        (*ttyctx).ocx = (*s).cx;
        (*ttyctx).ocy = (*s).cy;
        (*ttyctx).orlower = (*s).rlower;
        (*ttyctx).orupper = (*s).rupper;

        memcpy__(&raw mut (*ttyctx).defaults, &raw const GRID_DEFAULT_CELL);
        if let Some(init_ctx_cb) = (*ctx).init_ctx_cb {
            init_ctx_cb(ctx, ttyctx);
            if !(*ttyctx).palette.is_null() {
                if (*ttyctx).defaults.fg == 8 {
                    (*ttyctx).defaults.fg = (*(*ttyctx).palette).fg;
                }
                if (*ttyctx).defaults.bg == 8 {
                    (*ttyctx).defaults.bg = (*(*ttyctx).palette).bg;
                }
            }
        } else {
            (*ttyctx).redraw_cb = Some(screen_write_redraw_cb);
            if !(*ctx).wp.is_null() {
                tty_default_colours(&raw mut (*ttyctx).defaults, (*ctx).wp);
                (*ttyctx).palette = &raw mut (*(*ctx).wp).palette;
                (*ttyctx).set_client_cb = Some(screen_write_set_client_cb);
                (*ttyctx).arg = (*ctx).wp.cast();
            }
        }

        if (*ctx).flags & SCREEN_WRITE_SYNC == 0 {
            /*
             * For the active pane or for an overlay (no pane), we want to
             * only use synchronized updates if requested (commands that
             * move the cursor); for other panes, always use it, since the
             * cursor will have to move.
             */
            if !(*ctx).wp.is_null() {
                if (*ctx).wp != (*(*(*ctx).wp).window).active {
                    (*ttyctx).num = 1;
                } else {
                    (*ttyctx).num = sync as u32;
                }
            } else {
                (*ttyctx).num = 0x10 | (sync as u32);
            }
            tty_write(Some(tty_cmd_syncstart), ttyctx);
            (*ctx).flags |= SCREEN_WRITE_SYNC;
        }
    }
}

/// Make write list.
pub unsafe fn screen_write_make_list(s: *mut screen) {
    unsafe {
        (*s).write_list = xcalloc_(screen_size_y(s) as usize).as_ptr();
        for y in 0..screen_size_y(s) {
            tailq_init(&raw mut (*(*s).write_list.add(y as usize)).items);
        }
    }
}

/// Free write list.
pub unsafe fn screen_write_free_list(s: *mut screen) {
    unsafe {
        for y in 0..screen_size_y(s) {
            free_((*(*s).write_list.add(y as usize)).data);
        }
        free_((*s).write_list);
    }
}

/// Set up for writing.
unsafe fn screen_write_init(ctx: *mut screen_write_ctx, s: *mut screen) {
    unsafe {
        memset0(ctx);

        (*ctx).s = s;

        if (*(*ctx).s).write_list.is_null() {
            screen_write_make_list((*ctx).s);
        }
        (*ctx).item = screen_write_get_citem().as_ptr();

        (*ctx).scrolled = 0;
        (*ctx).bg = 8;
    }
}

/// Initialize writing with a pane.
pub unsafe fn screen_write_start_pane(
    ctx: *mut screen_write_ctx,
    wp: *mut window_pane,
    mut s: *mut screen,
) {
    unsafe {
        if s.is_null() {
            s = (*wp).screen;
        }
        screen_write_init(ctx, s);
        (*ctx).wp = wp;

        if log_get_level() != 0 {
            // log_debug("%s: size %ux%u, pane %%%u (at %u,%u)", __func__, screen_size_x((*ctx).s), screen_size_y((*ctx).s), (*wp).id, (*wp).xoff, (*wp).yoff);
        }
    }
}

/// Initialize writing with a callback.
pub unsafe fn screen_write_start_callback(
    ctx: *mut screen_write_ctx,
    s: *mut screen,
    cb: screen_write_init_ctx_cb,
    arg: *mut c_void,
) {
    unsafe {
        screen_write_init(ctx, s);

        (*ctx).init_ctx_cb = cb;
        (*ctx).arg = arg;

        if log_get_level() != 0 {
            // log_debug("%s: size %ux%u, with callback", __func__, screen_size_x((*ctx).s), screen_size_y((*ctx).s));
        }
    }
}

/// Initialize writing.
pub unsafe fn screen_write_start(ctx: *mut screen_write_ctx, s: *mut screen) {
    unsafe {
        screen_write_init(ctx, s);

        if log_get_level() != 0 {
            // log_debug("%s: size %ux%u, no pane", __func__, screen_size_x((*ctx).s), screen_size_y((*ctx).s));
        }
    }
}

/// Finish writing.
pub unsafe fn screen_write_stop(ctx: *mut screen_write_ctx) {
    unsafe {
        screen_write_collect_end(ctx);
        screen_write_collect_flush(ctx, 0, c!("screen_write_stop"));

        screen_write_free_citem((*ctx).item);
    }
}

/// Reset screen state.
pub unsafe fn screen_write_reset(ctx: *mut screen_write_ctx) {
    unsafe {
        let s = (*ctx).s;

        screen_reset_tabs(s);
        screen_write_scrollregion(ctx, 0, screen_size_y(s) - 1);

        (*s).mode = mode_flag::MODE_CURSOR | mode_flag::MODE_WRAP;

        if options_get_number_(GLOBAL_OPTIONS, c"extended-keys") == 2 {
            (*s).mode = ((*s).mode & !EXTENDED_KEY_MODES) | mode_flag::MODE_KEYS_EXTENDED;
        }

        screen_write_clearscreen(ctx, 8);
        screen_write_set_cursor(ctx, 0, 0);
    }
}

/// Write character.
pub unsafe fn screen_write_putc(ctx: *mut screen_write_ctx, gcp: *const grid_cell, ch: u8) {
    unsafe {
        let mut gc: grid_cell = zeroed();
        memcpy__(&raw mut gc, gcp);

        utf8_set(&raw mut gc.data, ch);
        screen_write_cell(ctx, &raw mut gc);
    }
}

macro_rules! screen_write_strlen {
   ($fmt:literal $(, $args:expr)* $(,)?) => {
        crate::screen_write::screen_write_strlen_(format_args!($fmt $(, $args)*))
    };
}
pub(crate) use screen_write_strlen;
/// Calculate string length.
pub unsafe fn screen_write_strlen_(args: std::fmt::Arguments) -> usize {
    unsafe {
        let mut msg: *mut u8 = null_mut();
        let mut ud: utf8_data = zeroed();

        let mut left = 0;
        let mut size = 0;
        let mut more: utf8_state = utf8_state::UTF8_DONE;

        let mut msg = args.to_string();
        msg.push('\0');
        let mut ptr: *mut u8 = msg.as_mut_ptr();

        while *ptr != b'\0' {
            if *ptr > 0x7f && utf8_open(&raw mut ud, *ptr) == utf8_state::UTF8_MORE {
                ptr = ptr.add(1);

                left = strlen(ptr.cast());
                if left < ud.size as usize - 1 {
                    break;
                }
                while {
                    more = utf8_append(&raw mut ud, *ptr);
                    more == utf8_state::UTF8_MORE
                } {
                    ptr = ptr.add(1);
                }
                ptr = ptr.add(1);

                if more == utf8_state::UTF8_DONE {
                    size += ud.width;
                }
            } else {
                if *ptr > 0x1f && *ptr < 0x7f {
                    size += 1;
                }
                ptr = ptr.add(1);
            }
        }

        size as usize
    }
}

macro_rules! screen_write_text {
   ($ctx:expr, $cx:expr, $width: expr, $lines: expr, $more: expr, $gcp: expr, $fmt:literal $(, $args:expr)* $(,)?) => {
        crate::screen_write::screen_write_text_($ctx, $cx, $width, $lines, $more, $gcp, format_args!($fmt $(, $args)*))
    };
}
pub(crate) use screen_write_text;

/// Write string wrapped over lines.
pub unsafe fn screen_write_text_(
    ctx: *mut screen_write_ctx,
    cx: u32,
    width: u32,
    lines: u32,
    more: i32,
    gcp: *const grid_cell,
    args: std::fmt::Arguments,
) -> bool {
    unsafe {
        let more = more != 0;
        let s = (*ctx).s;
        let cy = (*s).cy;
        let mut idx = 0;

        let mut gc: grid_cell = zeroed();
        memcpy__(&raw mut gc, gcp);

        let mut tmp = args.to_string();
        tmp.push('\0');
        let tmp = tmp.as_mut_ptr().cast();
        let text = utf8_fromcstr(tmp);

        let mut left = (cx + width) - (*s).cx;
        loop {
            // Find the end of what can fit on the line.

            let mut end = idx;
            let mut at = 0;
            for _ in idx..text.len() {
                if text[end].size == 1 && text[end].data[0] == b'\n' {
                    break;
                }
                if at + text[end].width as u32 > left {
                    break;
                }

                at += text[end].width as u32;
                end += 1;
            }

            /*
             * If we're on a space, that's the end. If not, walk back to
             * try and find one.
             */

            // note with the new box slice rust version text_end.size == 0 shouldn't occur
            let next = if end == text.len() {
                end
            } else if text[end].size == 1 && matches!(text[end].data[0], b'\n' | b' ') {
                end + 1
            } else {
                let mut i = end;
                while i > idx {
                    if text[i].size == 1 && text[i].data[0] == b' ' {
                        break;
                    }
                    i -= 1;
                }
                if i != idx {
                    end = i;
                    i + 1
                } else {
                    end
                }
            };

            // Print the line.
            for i in idx..end {
                utf8_copy(&raw mut gc.data, &raw const text[i]);
                screen_write_cell(ctx, &gc);
            }

            // If at the bottom, stop.
            idx = next;
            if (*s).cy == cy + lines - 1 || idx == text.len() {
                break;
            }

            screen_write_cursormove(ctx, cx as i32, (*s).cy as i32 + 1, 0);
            left = width;
        }

        /*
         * Fail if on the last line and there is more to come or at the end, or
         * if the text was not entirely consumed.
         */
        if ((*s).cy == cy + lines - 1 && (!more || (*s).cx == cx + width)) || idx != text.len() {
            return false;
        }

        /*
         * If no more to come, move to the next line. Otherwise, leave on
         * the same line (except if at the end).
         */
        if !more || (*s).cx == cx + width {
            screen_write_cursormove(ctx, cx as i32, (*s).cy as i32 + 1, 0);
        }
        true
    }
}

/// Write simple string (no maximum length).
macro_rules! screen_write_puts {
   ($ctx:expr, $gcp:expr, $fmt:literal $(, $args:expr)* $(,)?) => {
        crate::screen_write::screen_write_vnputs!($ctx, -1, $gcp, $fmt $(, $args)*);
   }
}
pub(crate) use screen_write_puts;

/// Write string with length limit (-1 for unlimited).
macro_rules! screen_write_nputs {
   ($ctx:expr, $maxlen:expr, $gcp:expr, $fmt:literal $(, $args:expr)* $(,)?) => {
        crate::screen_write::screen_write_vnputs!($ctx, $maxlen, $gcp, $fmt $(, $args)*);
   }
}
pub(crate) use screen_write_nputs;

macro_rules! screen_write_vnputs {
   ($ctx:expr, $maxlen:expr, $gcp:expr, $fmt:literal $(, $args:expr)* $(,)?) => {
        crate::screen_write::screen_write_vnputs_($ctx, $maxlen, $gcp, format_args!($fmt $(, $args)*))
    };
}
pub(crate) use screen_write_vnputs;

pub(crate) unsafe fn screen_write_vnputs_(
    ctx: *mut screen_write_ctx,
    maxlen: isize,
    gcp: *const grid_cell,
    args: std::fmt::Arguments,
) {
    unsafe {
        let mut gc: grid_cell = zeroed();
        let ud: *mut utf8_data = &raw mut gc.data;
        let mut size: usize = 0;
        let mut more: utf8_state = utf8_state::UTF8_DONE;

        memcpy__(&raw mut gc, gcp);
        let mut msg = args.to_string();
        msg.push('\0');

        let mut ptr: *mut u8 = msg.as_mut_ptr();
        while *ptr != b'\0' {
            if *ptr > 0x7f && utf8_open(ud, *ptr) == utf8_state::UTF8_MORE {
                ptr = ptr.add(1);

                let left = strlen(ptr.cast());
                if left < (*ud).size as usize - 1 {
                    break;
                }
                while {
                    more = utf8_append(ud, *ptr);
                    more == utf8_state::UTF8_MORE
                } {
                    ptr = ptr.add(1);
                }
                ptr = ptr.add(1);

                if more != utf8_state::UTF8_DONE {
                    continue;
                }
                if maxlen > 0 && size + (*ud).width as usize > maxlen as usize {
                    while size < maxlen as usize {
                        screen_write_putc(ctx, &raw const gc, b' ');
                        size += 1;
                    }
                    break;
                }
                size += (*ud).width as usize;
                screen_write_cell(ctx, &raw const gc);
            } else {
                if maxlen > 0 && size + 1 > maxlen as usize {
                    break;
                }

                if *ptr == b'\x01' {
                    gc.attr ^= grid_attr::GRID_ATTR_CHARSET;
                } else if *ptr == b'\n' {
                    screen_write_linefeed(ctx, 0, 8);
                    screen_write_carriagereturn(ctx);
                } else if *ptr > 0x1f && *ptr < 0x7f {
                    size += 1;
                    screen_write_putc(ctx, &gc, *ptr);
                }
                ptr = ptr.add(1);
            }
        }
    }
}

/// Copy from another screen but without the selection stuff. Assumes the target
/// region is already big enough.
pub unsafe fn screen_write_fast_copy(
    ctx: *mut screen_write_ctx,
    src: *mut screen,
    px: u32,
    py: u32,
    nx: u32,
    ny: u32,
) {
    unsafe {
        let s = (*ctx).s;
        let gd = (*src).grid;
        let mut gc: grid_cell = zeroed();

        if nx == 0 || ny == 0 {
            return;
        }

        let mut cy = (*s).cy;
        for yy in py..(py + ny) {
            if yy >= (*gd).hsize + (*gd).sy {
                break;
            }
            let mut cx = (*s).cx;
            for xx in px..(px + nx) {
                if xx >= (*grid_get_line(gd, yy)).cellsize {
                    break;
                }
                grid_get_cell(gd, xx, yy, &raw mut gc);
                if xx + gc.data.width as u32 > px + nx {
                    break;
                }
                grid_view_set_cell((*(*ctx).s).grid, cx, cy, &gc);
                cx += 1;
            }
            cy += 1;
        }
    }
}

/// Select character set for drawing border lines.
unsafe fn screen_write_box_border_set(lines: box_lines, cell_type: cell_type, gc: *mut grid_cell) {
    unsafe {
        match lines {
            box_lines::BOX_LINES_NONE => (),
            box_lines::BOX_LINES_DOUBLE => {
                (*gc).attr &= !grid_attr::GRID_ATTR_CHARSET;
                utf8_copy(&raw mut (*gc).data, tty_acs_double_borders(cell_type));
            }
            box_lines::BOX_LINES_HEAVY => {
                (*gc).attr &= !grid_attr::GRID_ATTR_CHARSET;
                utf8_copy(&raw mut (*gc).data, tty_acs_heavy_borders(cell_type));
            }
            box_lines::BOX_LINES_ROUNDED => {
                (*gc).attr &= !grid_attr::GRID_ATTR_CHARSET;
                utf8_copy(&raw mut (*gc).data, tty_acs_rounded_borders(cell_type));
            }
            box_lines::BOX_LINES_SIMPLE => {
                (*gc).attr &= !grid_attr::GRID_ATTR_CHARSET;
                utf8_set(&raw mut (*gc).data, SIMPLE_BORDERS[cell_type as usize]);
            }
            box_lines::BOX_LINES_PADDED => {
                (*gc).attr &= !grid_attr::GRID_ATTR_CHARSET;
                utf8_set(&raw mut (*gc).data, PADDED_BORDERS[cell_type as usize]);
            }
            box_lines::BOX_LINES_SINGLE | box_lines::BOX_LINES_DEFAULT => {
                (*gc).attr |= grid_attr::GRID_ATTR_CHARSET;
                utf8_set(&raw mut (*gc).data, CELL_BORDERS[cell_type as usize]);
            }
        }
    }
}

/// Draw a horizontal line on screen.
pub unsafe fn screen_write_hline(
    ctx: *mut screen_write_ctx,
    nx: u32,
    left: i32,
    right: i32,
    lines: box_lines,
    border_gc: *const grid_cell,
) {
    unsafe {
        let s: *mut screen = (*ctx).s;
        let mut gc: grid_cell = zeroed();
        // u_int cx, cy, i;

        let cx = (*s).cx;
        let cy = (*s).cy;

        if !border_gc.is_null() {
            memcpy__(&raw mut gc, border_gc);
        } else {
            memcpy__(&raw mut gc, &raw const GRID_DEFAULT_CELL);
        }
        gc.attr |= grid_attr::GRID_ATTR_CHARSET;

        if left != 0 {
            screen_write_box_border_set(lines, CELL_LEFTJOIN, &raw mut gc);
        } else {
            screen_write_box_border_set(lines, CELL_LEFTRIGHT, &raw mut gc);
        }
        screen_write_cell(ctx, &gc);

        screen_write_box_border_set(lines, CELL_LEFTRIGHT, &raw mut gc);
        for i in 1..(nx - 1) {
            screen_write_cell(ctx, &raw mut gc);
        }

        if right != 0 {
            screen_write_box_border_set(lines, CELL_RIGHTJOIN, &raw mut gc);
        } else {
            screen_write_box_border_set(lines, CELL_LEFTRIGHT, &raw mut gc);
        }
        screen_write_cell(ctx, &raw const gc);

        screen_write_set_cursor(ctx, cx as i32, cy as i32);
    }
}

/// Draw a vertical line on screen.
pub unsafe fn screen_write_vline(ctx: *mut screen_write_ctx, ny: u32, top: i32, bottom: i32) {
    unsafe {
        let s = (*ctx).s;
        let mut gc: grid_cell = zeroed();

        let cx = (*s).cx;
        let cy = (*s).cy;

        memcpy__(&raw mut gc, &raw const GRID_DEFAULT_CELL);
        gc.attr |= grid_attr::GRID_ATTR_CHARSET;

        screen_write_putc(ctx, &raw const gc, if top != 0 { b'w' } else { b'x' });

        for i in 1..(ny - 1) {
            screen_write_set_cursor(ctx, cx as i32, (cy + i) as i32);
            screen_write_putc(ctx, &raw const gc, b'x');
        }
        screen_write_set_cursor(ctx, cx as i32, (cy + ny - 1) as i32);
        screen_write_putc(ctx, &raw const gc, if bottom != 0 { b'v' } else { b'x' });

        screen_write_set_cursor(ctx, cx as i32, cy as i32);
    }
}

/// Draw a menu on screen.
pub unsafe fn screen_write_menu(
    ctx: *mut screen_write_ctx,
    menu: *mut menu,
    choice: i32,
    lines: box_lines,
    menu_gc: *const grid_cell,
    border_gc: *const grid_cell,
    choice_gc: *const grid_cell,
) {
    unsafe {
        let s = (*ctx).s;
        let mut default_gc: grid_cell = zeroed();
        let mut gc = &raw const default_gc;

        // u_int cx, cy, i, j;
        let width = (*menu).width;

        let cx = (*s).cx;
        let cy = (*s).cy;

        memcpy__(&raw mut default_gc, menu_gc);

        screen_write_box(
            ctx,
            (*menu).width + 4,
            (*menu).count + 2,
            lines,
            border_gc,
            (*menu).title,
        );

        for i in 0..(*menu).count {
            let name = (*(*menu).items.add(i as usize)).name.as_ptr();
            if name.is_null() {
                screen_write_cursormove(ctx, cx as i32, (cy + 1 + i) as i32, 0);
                screen_write_hline(ctx, width + 4, 1, 1, lines, border_gc);
                continue;
            }

            if choice >= 0 && i == choice as u32 && *name != b'-' {
                gc = choice_gc;
            }

            screen_write_cursormove(ctx, cx as i32 + 1, (cy + 1 + i) as i32, 0);
            for j in 0..(width + 2) {
                screen_write_putc(ctx, gc, b' ');
            }

            screen_write_cursormove(ctx, cx as i32 + 2, (cy + 1 + i) as i32, 0);
            if *name == b'-' {
                default_gc.attr |= grid_attr::GRID_ATTR_DIM;
                format_draw(ctx, gc, width, name.add(1), null_mut(), 0);
                default_gc.attr &= !grid_attr::GRID_ATTR_DIM;
                continue;
            }

            format_draw(ctx, gc, width, name, null_mut(), 0);
            gc = &raw mut default_gc;
        }

        screen_write_set_cursor(ctx, cx as i32, cy as i32);
    }
}

/// Draw a box on screen.
pub unsafe fn screen_write_box(
    ctx: *mut screen_write_ctx,
    nx: u32,
    ny: u32,
    lines: box_lines,
    gcp: *const grid_cell,
    title: *const u8,
) {
    unsafe {
        let s = (*ctx).s;
        let mut gc: grid_cell = zeroed();

        let cx = (*s).cx;
        let cy = (*s).cy;

        if !gcp.is_null() {
            memcpy__(&raw mut gc, gcp);
        } else {
            memcpy__(&raw mut gc, &raw const GRID_DEFAULT_CELL);
        }

        gc.attr |= grid_attr::GRID_ATTR_CHARSET;
        gc.flags |= grid_flag::NOPALETTE;

        /* Draw top border */
        screen_write_box_border_set(lines, CELL_TOPLEFT, &raw mut gc);
        screen_write_cell(ctx, &raw const gc);
        screen_write_box_border_set(lines, CELL_LEFTRIGHT, &raw mut gc);
        for i in 1..(nx - 1) {
            screen_write_cell(ctx, &raw const gc);
        }
        screen_write_box_border_set(lines, CELL_TOPRIGHT, &raw mut gc);
        screen_write_cell(ctx, &raw const gc);

        /* Draw bottom border */
        screen_write_set_cursor(ctx, cx as i32, (cy + ny - 1) as i32);
        screen_write_box_border_set(lines, CELL_BOTTOMLEFT, &raw mut gc);
        screen_write_cell(ctx, &gc);
        screen_write_box_border_set(lines, CELL_LEFTRIGHT, &raw mut gc);
        for i in 1..(nx - 1) {
            screen_write_cell(ctx, &raw const gc);
        }
        screen_write_box_border_set(lines, CELL_BOTTOMRIGHT, &raw mut gc);
        screen_write_cell(ctx, &raw const gc);

        /* Draw sides */
        screen_write_box_border_set(lines, CELL_TOPBOTTOM, &raw mut gc);
        for i in 1..(ny - 1) {
            /* left side */
            screen_write_set_cursor(ctx, cx as i32, (cy + i) as i32);
            screen_write_cell(ctx, &raw const gc);
            /* right side */
            screen_write_set_cursor(ctx, (cx + nx - 1) as i32, (cy + i) as i32);
            screen_write_cell(ctx, &raw const gc);
        }

        if !title.is_null() {
            gc.attr &= !grid_attr::GRID_ATTR_CHARSET;
            screen_write_cursormove(ctx, (cx + 2) as i32, cy as i32, 0);
            format_draw(ctx, &raw const gc, nx - 4, title, null_mut(), 0);
        }

        screen_write_set_cursor(ctx, cx as i32, cy as i32);
    }
}

/// Write a preview version of a window. Assumes target area is big enough and already cleared.
pub unsafe fn screen_write_preview(ctx: *mut screen_write_ctx, src: *mut screen, nx: u32, ny: u32) {
    unsafe {
        let s = (*ctx).s;
        let mut gc: grid_cell = zeroed();

        let cx = (*s).cx;
        let cy = (*s).cy;

        /*
         * If the cursor is on, pick the area around the cursor, otherwise use
         * the top left.
         */
        let mut px: u32;
        let mut py: u32;
        if (*src).mode.intersects(mode_flag::MODE_CURSOR) {
            px = (*src).cx;
            if px < nx / 3 {
                px = 0;
            } else {
                px -= nx / 3;
            }
            if px + nx > screen_size_x(src) {
                if nx > screen_size_x(src) {
                    px = 0;
                } else {
                    px = screen_size_x(src) - nx;
                }
            }
            py = (*src).cy;
            if py < ny / 3 {
                py = 0;
            } else {
                py -= ny / 3;
            }
            if py + ny > screen_size_y(src) {
                if ny > screen_size_y(src) {
                    py = 0;
                } else {
                    py = screen_size_y(src) - ny;
                }
            }
        } else {
            px = 0;
            py = 0;
        }

        screen_write_fast_copy(ctx, src, px, (*(*src).grid).hsize + py, nx, ny);

        if (*src).mode.intersects(mode_flag::MODE_CURSOR) {
            grid_view_get_cell((*src).grid, (*src).cx, (*src).cy, &raw mut gc);
            gc.attr |= grid_attr::GRID_ATTR_REVERSE;
            screen_write_set_cursor(
                ctx,
                cx as i32 + ((*src).cx - px) as i32,
                cy as i32 + ((*src).cy - py) as i32,
            );
            screen_write_cell(ctx, &raw const gc);
        }
    }
}

/// Set a mode.
pub unsafe fn screen_write_mode_set(ctx: *mut screen_write_ctx, mode: mode_flag) {
    unsafe {
        let s = (*ctx).s;

        (*s).mode |= mode;

        if log_get_level() != 0 {
            // log_debug("%s: %s", __func__, screen_mode_to_string(mode));
        }
    }
}

/// Clear a mode.
pub unsafe fn screen_write_mode_clear(ctx: *mut screen_write_ctx, mode: mode_flag) {
    unsafe {
        let s = (*ctx).s;

        (*s).mode &= !mode;

        if log_get_level() != 0 {
            // log_debug("%s: %s", __func__, screen_mode_to_string(mode));
        }
    }
}

/// Cursor up by ny.
pub unsafe fn screen_write_cursorup(ctx: *mut screen_write_ctx, mut ny: u32) {
    unsafe {
        let s = (*ctx).s;
        let mut cx: u32 = (*s).cx;
        let mut cy: u32 = (*s).cy;

        if ny == 0 {
            ny = 1;
        }

        if cy < (*s).rupper {
            /* Above region. */
            if ny > cy {
                ny = cy;
            }
        } else {
            /* Below region. */
            if ny > cy - (*s).rupper {
                ny = cy - (*s).rupper;
            }
        }
        if cx == screen_size_x(s) {
            cx -= 1;
        }

        cy -= ny;

        screen_write_set_cursor(ctx, cx as i32, cy as i32);
    }
}

/// Cursor down by ny.
pub unsafe fn screen_write_cursordown(ctx: *mut screen_write_ctx, mut ny: u32) {
    unsafe {
        let s: *mut screen = (*ctx).s;
        let mut cx: u32 = (*s).cx;
        let mut cy: u32 = (*s).cy;

        if ny == 0 {
            ny = 1;
        }

        if cy > (*s).rlower {
            /* Below region. */
            if ny > screen_size_y(s) - 1 - cy {
                ny = screen_size_y(s) - 1 - cy;
            }
        } else {
            /* Above region. */
            if ny > (*s).rlower - cy {
                ny = (*s).rlower - cy;
            }
        }
        if cx == screen_size_x(s) {
            cx -= 1;
        } else if ny == 0 {
            return;
        }

        cy += ny;

        screen_write_set_cursor(ctx, cx as i32, cy as i32);
    }
}

/// Cursor right by nx.
pub unsafe fn screen_write_cursorright(ctx: *mut screen_write_ctx, mut nx: u32) {
    unsafe {
        let s = (*ctx).s;
        let mut cx: u32 = (*s).cx;
        let cy: u32 = (*s).cy;

        if nx == 0 {
            nx = 1;
        }

        if nx > screen_size_x(s) - 1 - cx {
            nx = screen_size_x(s) - 1 - cx;
        }
        if nx == 0 {
            return;
        }

        cx += nx;

        screen_write_set_cursor(ctx, cx as i32, cy as i32);
    }
}

/// Cursor left by nx.
pub unsafe fn screen_write_cursorleft(ctx: *mut screen_write_ctx, mut nx: u32) {
    unsafe {
        let s = (*ctx).s;
        let mut cx: u32 = (*s).cx;
        let cy: u32 = (*s).cy;

        if nx == 0 {
            nx = 1;
        }

        if nx > cx {
            nx = cx;
        }
        if nx == 0 {
            return;
        }

        cx -= nx;

        screen_write_set_cursor(ctx, cx as i32, cy as i32);
    }
}

/// Backspace; cursor left unless at start of wrapped line when can move up.
pub unsafe fn screen_write_backspace(ctx: *mut screen_write_ctx) {
    unsafe {
        let s = (*ctx).s;
        let mut cx = (*s).cx;
        let mut cy = (*s).cy;

        if cx == 0 {
            if cy == 0 {
                return;
            }
            let gl = grid_get_line((*s).grid, (*(*s).grid).hsize + cy - 1);
            if (*gl).flags.intersects(grid_line_flag::WRAPPED) {
                cy -= 1;
                cx = screen_size_x(s) - 1;
            }
        } else {
            cx -= 1;
        }

        screen_write_set_cursor(ctx, cx as i32, cy as i32);
    }
}

/// VT100 alignment test.
pub unsafe fn screen_write_alignmenttest(ctx: *mut screen_write_ctx) {
    unsafe {
        let s = (*ctx).s;
        let mut ttyctx: tty_ctx = zeroed();
        let mut gc: grid_cell = zeroed();

        memcpy__(&raw mut gc, &raw const GRID_DEFAULT_CELL);
        utf8_set(&raw mut gc.data, b'E');

        #[cfg(feature = "sixel")]
        {
            if image_free_all(s) && !(*ctx).wp.is_null() {
                (*(*ctx).wp).flags |= window_pane_flags::PANE_REDRAW;
            }
        }

        for yy in 0..screen_size_y(s) {
            for xx in 0..screen_size_x(s) {
                grid_view_set_cell((*s).grid, xx, yy, &raw const gc);
            }
        }

        screen_write_set_cursor(ctx, 0, 0);

        (*s).rupper = 0;
        (*s).rlower = screen_size_y(s) - 1;

        screen_write_initctx(ctx, &raw mut ttyctx, 1);

        screen_write_collect_clear(ctx, 0, screen_size_y(s) - 1);
        tty_write(Some(tty_cmd_alignmenttest), &raw mut ttyctx);
    }
}

/// Insert nx characters.
pub unsafe fn screen_write_insertcharacter(ctx: *mut screen_write_ctx, mut nx: u32, bg: u32) {
    unsafe {
        let s = (*ctx).s;
        let mut ttyctx: tty_ctx = zeroed();

        if nx == 0 {
            nx = 1;
        }

        if nx > screen_size_x(s) - (*s).cx {
            nx = screen_size_x(s) - (*s).cx;
        }
        if nx == 0 {
            return;
        }

        if (*s).cx > screen_size_x(s) - 1 {
            return;
        }

        #[cfg(feature = "sixel")]
        {
            if image_check_line(s, (*s).cy, 1) && !(*ctx).wp.is_null() {
                (*(*ctx).wp).flags |= window_pane_flags::PANE_REDRAW;
            }
        }

        screen_write_initctx(ctx, &raw mut ttyctx, 0);
        ttyctx.bg = bg;

        grid_view_insert_cells((*s).grid, (*s).cx, (*s).cy, nx, bg);

        screen_write_collect_flush(ctx, 0, c!("screen_write_insertcharacter"));
        ttyctx.num = nx;
        tty_write(Some(tty_cmd_insertcharacter), &raw mut ttyctx);
    }
}

/// Delete nx characters.
pub unsafe fn screen_write_deletecharacter(ctx: *mut screen_write_ctx, mut nx: u32, bg: u32) {
    unsafe {
        let s = (*ctx).s;
        let mut ttyctx: tty_ctx = zeroed();

        if nx == 0 {
            nx = 1;
        }

        if nx > screen_size_x(s) - (*s).cx {
            nx = screen_size_x(s) - (*s).cx;
        }
        if nx == 0 {
            return;
        }

        if (*s).cx > screen_size_x(s) - 1 {
            return;
        }

        #[cfg(feature = "sixel")]
        {
            if image_check_line(s, (*s).cy, 1) && !(*ctx).wp.is_null() {
                (*(*ctx).wp).flags |= window_pane_flags::PANE_REDRAW;
            }
        }

        screen_write_initctx(ctx, &raw mut ttyctx, 0);
        ttyctx.bg = bg;

        grid_view_delete_cells((*s).grid, (*s).cx, (*s).cy, nx, bg);

        screen_write_collect_flush(ctx, 0, c!("screen_write_deletecharacter"));
        ttyctx.num = nx;
        tty_write(Some(tty_cmd_deletecharacter), &raw mut ttyctx);
    }
}

/// Clear nx characters.
pub unsafe fn screen_write_clearcharacter(ctx: *mut screen_write_ctx, mut nx: u32, bg: u32) {
    unsafe {
        let s = (*ctx).s;
        let mut ttyctx: tty_ctx = zeroed();

        if nx == 0 {
            nx = 1;
        }

        if nx > screen_size_x(s) - (*s).cx {
            nx = screen_size_x(s) - (*s).cx;
        }
        if nx == 0 {
            return;
        }

        if (*s).cx > screen_size_x(s) - 1 {
            return;
        }

        #[cfg(feature = "sixel")]
        {
            if image_check_line(s, (*s).cy, 1) && !(*ctx).wp.is_null() {
                (*(*ctx).wp).flags |= window_pane_flags::PANE_REDRAW;
            }
        }

        screen_write_initctx(ctx, &raw mut ttyctx, 0);
        ttyctx.bg = bg;

        grid_view_clear((*s).grid, (*s).cx, (*s).cy, nx, 1, bg);

        screen_write_collect_flush(ctx, 0, c!("screen_write_clearcharacter"));
        ttyctx.num = nx;
        tty_write(Some(tty_cmd_clearcharacter), &raw mut ttyctx);
    }
}

/// Insert ny lines.
pub unsafe fn screen_write_insertline(ctx: *mut screen_write_ctx, mut ny: u32, bg: u32) {
    unsafe {
        let s = (*ctx).s;
        let gd = (*s).grid;
        let mut ttyctx: tty_ctx = zeroed();

        let mut sy: u32;

        #[cfg(feature = "sixel")]
        {
            sy = screen_size_y(s);
        }

        if ny == 0 {
            ny = 1;
        }

        #[cfg(feature = "sixel")]
        {
            if image_check_line(s, (*s).cy, sy - (*s).cy) && !(*ctx).wp.is_null() {
                (*(*ctx).wp).flags |= window_pane_flags::PANE_REDRAW;
            }
        }

        if (*s).cy < (*s).rupper || (*s).cy > (*s).rlower {
            if ny > screen_size_y(s) - (*s).cy {
                ny = screen_size_y(s) - (*s).cy;
            }
            if ny == 0 {
                return;
            }

            screen_write_initctx(ctx, &raw mut ttyctx, 1);
            ttyctx.bg = bg;

            grid_view_insert_lines(gd, (*s).cy, ny, bg);

            screen_write_collect_flush(ctx, 0, c!("screen_write_insertline"));
            ttyctx.num = ny;
            tty_write(Some(tty_cmd_insertline), &raw mut ttyctx);
            return;
        }

        if ny > (*s).rlower + 1 - (*s).cy {
            ny = (*s).rlower + 1 - (*s).cy;
        }
        if ny == 0 {
            return;
        }

        screen_write_initctx(ctx, &raw mut ttyctx, 1);
        ttyctx.bg = bg;

        if (*s).cy < (*s).rupper || (*s).cy > (*s).rlower {
            grid_view_insert_lines(gd, (*s).cy, ny, bg);
        } else {
            grid_view_insert_lines_region(gd, (*s).rlower, (*s).cy, ny, bg);
        }

        screen_write_collect_flush(ctx, 0, c!("screen_write_insertline"));

        ttyctx.num = ny;
        tty_write(Some(tty_cmd_insertline), &raw mut ttyctx);
    }
}

/// Delete ny lines.
pub unsafe fn screen_write_deleteline(ctx: *mut screen_write_ctx, mut ny: u32, bg: u32) {
    unsafe {
        let s = (*ctx).s;
        let gd = (*s).grid;
        let mut ttyctx: tty_ctx = zeroed();
        let sy = screen_size_y(s);

        if ny == 0 {
            ny = 1;
        }

        #[cfg(feature = "sixel")]
        {
            if image_check_line(s, (*s).cy, sy - (*s).cy) && !(*ctx).wp.is_null() {
                (*(*ctx).wp).flags |= window_pane_flags::PANE_REDRAW;
            }
        }

        if (*s).cy < (*s).rupper || (*s).cy > (*s).rlower {
            if ny > sy - (*s).cy {
                ny = sy - (*s).cy;
            }
            if ny == 0 {
                return;
            }

            screen_write_initctx(ctx, &raw mut ttyctx, 1);
            ttyctx.bg = bg;

            grid_view_delete_lines(gd, (*s).cy, ny, bg);

            screen_write_collect_flush(ctx, 0, c!("screen_write_deleteline"));
            ttyctx.num = ny;
            tty_write(Some(tty_cmd_deleteline), &raw mut ttyctx);
            return;
        }

        if ny > (*s).rlower + 1 - (*s).cy {
            ny = (*s).rlower + 1 - (*s).cy;
        }
        if ny == 0 {
            return;
        }

        screen_write_initctx(ctx, &raw mut ttyctx, 1);
        ttyctx.bg = bg;

        if (*s).cy < (*s).rupper || (*s).cy > (*s).rlower {
            grid_view_delete_lines(gd, (*s).cy, ny, bg);
        } else {
            grid_view_delete_lines_region(gd, (*s).rlower, (*s).cy, ny, bg);
        }

        screen_write_collect_flush(ctx, 0, c!("screen_write_deleteline"));
        ttyctx.num = ny;
        tty_write(Some(tty_cmd_deleteline), &raw mut ttyctx);
    }
}

/// Clear line at cursor.
pub unsafe fn screen_write_clearline(ctx: *mut screen_write_ctx, bg: u32) {
    unsafe {
        let s = (*ctx).s;
        let sx = screen_size_x(s);
        let ci = (*ctx).item;

        let gl = grid_get_line((*s).grid, (*(*s).grid).hsize + (*s).cy);
        if (*gl).cellsize == 0 && COLOUR_DEFAULT(bg as i32) {
            return;
        }

        #[cfg(feature = "sixel")]
        {
            if image_check_line(s, (*s).cy, 1) && !(*ctx).wp.is_null() {
                (*(*ctx).wp).flags |= window_pane_flags::PANE_REDRAW;
            }
        }

        grid_view_clear((*s).grid, 0, (*s).cy, sx, 1, bg);

        screen_write_collect_clear(ctx, (*s).cy, 1);
        (*ci).x = 0;
        (*ci).used = sx;
        (*ci).type_ = screen_write_citem_type::Clear;
        (*ci).bg = bg;
        tailq_insert_tail(
            &raw mut (*(*(*ctx).s).write_list.add((*s).cy as usize)).items,
            ci,
        );
        (*ctx).item = screen_write_get_citem().as_ptr();
    }
}

/// Clear to end of line from cursor.
pub unsafe fn screen_write_clearendofline(ctx: *mut screen_write_ctx, bg: u32) {
    unsafe {
        let s = (*ctx).s;
        let sx = screen_size_x(s);
        let ci = (*ctx).item;

        if (*s).cx == 0 {
            screen_write_clearline(ctx, bg);
            return;
        }

        let gl = grid_get_line((*s).grid, (*(*s).grid).hsize + (*s).cy);
        if (*s).cx > sx - 1 || ((*s).cx >= (*gl).cellsize && COLOUR_DEFAULT(bg as i32)) {
            return;
        }

        #[cfg(feature = "sixel")]
        {
            if image_check_line(s, (*s).cy, 1) && !(*ctx).wp.is_null() {
                (*(*ctx).wp).flags |= window_pane_flags::PANE_REDRAW;
            }
        }

        grid_view_clear((*s).grid, (*s).cx, (*s).cy, sx - (*s).cx, 1, bg);

        let before = screen_write_collect_trim(ctx, (*s).cy, (*s).cx, sx - (*s).cx, null_mut());
        (*ci).x = (*s).cx;
        (*ci).used = sx - (*s).cx;
        (*ci).type_ = screen_write_citem_type::Clear;
        (*ci).bg = bg;
        if before.is_null() {
            tailq_insert_tail(
                &raw mut (*(*(*ctx).s).write_list.add((*s).cy as usize)).items,
                ci,
            );
        } else {
            tailq_insert_before(before, ci);
        }
        (*ctx).item = screen_write_get_citem().as_ptr();
    }
}

/// Clear to start of line from cursor.
pub unsafe fn screen_write_clearstartofline(ctx: *mut screen_write_ctx, bg: u32) {
    unsafe {
        let s = (*ctx).s;
        let sx = screen_size_x(s);
        let ci = (*ctx).item;

        if (*s).cx >= sx - 1 {
            screen_write_clearline(ctx, bg);
            return;
        }

        #[cfg(feature = "sixel")]
        {
            if image_check_line(s, (*s).cy, 1) && !(*ctx).wp.is_null() {
                (*(*ctx).wp).flags |= window_pane_flags::PANE_REDRAW;
            }
        }

        if (*s).cx > sx - 1 {
            grid_view_clear((*s).grid, 0, (*s).cy, sx, 1, bg);
        } else {
            grid_view_clear((*s).grid, 0, (*s).cy, (*s).cx + 1, 1, bg);
        }

        let before = screen_write_collect_trim(ctx, (*s).cy, 0, (*s).cx + 1, null_mut());
        (*ci).x = 0;
        (*ci).used = (*s).cx + 1;
        (*ci).type_ = screen_write_citem_type::Clear;
        (*ci).bg = bg;
        if before.is_null() {
            tailq_insert_tail(
                &raw mut (*(*(*ctx).s).write_list.add((*s).cy as usize)).items,
                ci,
            );
        } else {
            tailq_insert_before(before, ci);
        }
        (*ctx).item = screen_write_get_citem().as_ptr();
    }
}

/// Move cursor to px,py.
pub unsafe fn screen_write_cursormove(
    ctx: *mut screen_write_ctx,
    mut px: i32,
    mut py: i32,
    origin: i32,
) {
    unsafe {
        let s = (*ctx).s;

        if origin != 0 && py != -1 && (*s).mode.intersects(mode_flag::MODE_ORIGIN) {
            if py as u32 > (*s).rlower - (*s).rupper {
                py = (*s).rlower as i32;
            } else {
                py += (*s).rupper as i32;
            }
        }

        if px != -1 && px as u32 > screen_size_x(s) - 1 {
            px = screen_size_x(s) as i32 - 1;
        }
        if py != -1 && py as u32 > screen_size_y(s) - 1 {
            py = screen_size_y(s) as i32 - 1;
        }

        // log_debug("%s: from %u,%u to %u,%u", __func__, (*s).cx, (*s).cy, px, py);
        screen_write_set_cursor(ctx, px, py);
    }
}

/// Reverse index (up with scroll).
pub unsafe fn screen_write_reverseindex(ctx: *mut screen_write_ctx, bg: u32) {
    unsafe {
        let s = (*ctx).s;
        let mut ttyctx: tty_ctx = zeroed();

        if (*s).cy == (*s).rupper {
            #[cfg(feature = "sixel")]
            {
                if image_free_all(s) && !(*ctx).wp.is_null() {
                    (*(*ctx).wp).flags |= window_pane_flags::PANE_REDRAW;
                }
            }

            grid_view_scroll_region_down((*s).grid, (*s).rupper, (*s).rlower, bg);
            screen_write_collect_flush(ctx, 0, c!("screen_write_reverseindex"));

            screen_write_initctx(ctx, &raw mut ttyctx, 1);
            ttyctx.bg = bg;

            tty_write(Some(tty_cmd_reverseindex), &raw mut ttyctx);
        } else if (*s).cy > 0 {
            screen_write_set_cursor(ctx, -1, (*s).cy as i32 - 1);
        }
    }
}

/// Set scroll region.
pub unsafe fn screen_write_scrollregion(
    ctx: *mut screen_write_ctx,
    mut rupper: u32,
    mut rlower: u32,
) {
    unsafe {
        let s = (*ctx).s;

        if rupper > screen_size_y(s) - 1 {
            rupper = screen_size_y(s) - 1;
        }
        if rlower > screen_size_y(s) - 1 {
            rlower = screen_size_y(s) - 1;
        }
        if rupper >= rlower {
            return;
        } /* cannot be one line */

        screen_write_collect_flush(ctx, 0, c!("screen_write_scrollregion"));

        /* Cursor moves to top-left. */
        screen_write_set_cursor(ctx, 0, 0);

        (*s).rupper = rupper;
        (*s).rlower = rlower;
    }
}

/// Line feed.
pub unsafe fn screen_write_linefeed(ctx: *mut screen_write_ctx, wrapped: i32, bg: u32) {
    unsafe {
        let s = (*ctx).s;
        let gd = (*s).grid;
        let mut redraw: i32;
        #[cfg(feature = "sixel")]
        {
            redraw = 0;
        }

        let rupper = (*s).rupper;
        let rlower = (*s).rlower;

        let gl = grid_get_line(gd, (*gd).hsize + (*s).cy);
        if wrapped != 0 {
            (*gl).flags |= grid_line_flag::WRAPPED;
        }

        // log_debug("%s: at %u,%u (region %u-%u)", __func__, (*s).cx, (*s).cy, rupper, rlower);

        if bg != (*ctx).bg {
            screen_write_collect_flush(ctx, 1, c!("screen_write_linefeed"));
            (*ctx).bg = bg;
        }

        if (*s).cy == (*s).rlower {
            #[cfg(feature = "sixel")]
            {
                if (rlower == screen_size_y(s) - 1) {
                    redraw = image_scroll_up(s, 1);
                } else {
                    redraw = image_check_line(s, rupper, rlower - rupper);
                }
                if redraw != 0 && !(*ctx).wp.is_null() {
                    (*(*ctx).wp).flags |= window_pane_flags::PANE_REDRAW;
                }
            }
            grid_view_scroll_region_up(gd, (*s).rupper, (*s).rlower, bg);
            screen_write_collect_scroll(ctx, bg);
            (*ctx).scrolled += 1;
        } else if (*s).cy < screen_size_y(s) - 1 {
            screen_write_set_cursor(ctx, -1, (*s).cy as i32 + 1);
        }
    }
}

/// Scroll up.
pub unsafe fn screen_write_scrollup(ctx: *mut screen_write_ctx, mut lines: u32, bg: u32) {
    unsafe {
        let s = (*ctx).s;
        let gd = (*s).grid;

        if lines == 0 {
            lines = 1;
        } else if lines > (*s).rlower - (*s).rupper + 1 {
            lines = (*s).rlower - (*s).rupper + 1;
        }

        if bg != (*ctx).bg {
            screen_write_collect_flush(ctx, 1, c!("screen_write_scrollup"));
            (*ctx).bg = bg;
        }

        #[cfg(feature = "sixel")]
        {
            if image_scroll_up(s, lines) && !(*ctx).wp.is_null() {
                (*(*ctx).wp).flags |= window_pane_flag::PANE_REDRAW;
            }
        }

        for i in 0..lines {
            grid_view_scroll_region_up(gd, (*s).rupper, (*s).rlower, bg);
            screen_write_collect_scroll(ctx, bg);
        }
        (*ctx).scrolled += lines;
    }
}

/// Scroll down.
pub unsafe fn screen_write_scrolldown(ctx: *mut screen_write_ctx, mut lines: u32, bg: u32) {
    unsafe {
        let s = (*ctx).s;
        let gd = (*s).grid;
        let mut ttyctx: tty_ctx = zeroed();

        screen_write_initctx(ctx, &raw mut ttyctx, 1);
        ttyctx.bg = bg;

        if lines == 0 {
            lines = 1;
        } else if lines > (*s).rlower - (*s).rupper + 1 {
            lines = (*s).rlower - (*s).rupper + 1;
        }

        #[cfg(feature = "sixel")]
        {
            if image_free_all(s) && !(*ctx).wp.is_null() {
                (*(*ctx).wp).flags |= window_pane_flags::PANE_REDRAW;
            }
        }

        for i in 0..lines {
            grid_view_scroll_region_down(gd, (*s).rupper, (*s).rlower, bg);
        }

        screen_write_collect_flush(ctx, 0, c!("screen_write_scrolldown"));
        ttyctx.num = lines;
        tty_write(Some(tty_cmd_scrolldown), &raw mut ttyctx);
    }
}

/// Carriage return (cursor to start of line).
pub unsafe fn screen_write_carriagereturn(ctx: *mut screen_write_ctx) {
    unsafe {
        screen_write_set_cursor(ctx, 0, -1);
    }
}

/// Clear to end of screen from cursor.
pub unsafe fn screen_write_clearendofscreen(ctx: *mut screen_write_ctx, bg: u32) {
    unsafe {
        let s = (*ctx).s;
        let gd = (*s).grid;
        let mut ttyctx: tty_ctx = zeroed();
        let sx = screen_size_x(s);
        let sy = screen_size_y(s);

        #[cfg(feature = "sixel")]
        {
            if image_check_line(s, (*s).cy, sy - (*s).cy) && !(*ctx).wp.is_null() {
                (*(*ctx).wp).flags |= window_pane_flags::PANE_REDRAW;
            }
        }

        screen_write_initctx(ctx, &raw mut ttyctx, 1);
        ttyctx.bg = bg;

        /* Scroll into history if it is enabled and clearing entire screen. */
        if (*s).cx == 0
            && (*s).cy == 0
            && ((*gd).flags & GRID_HISTORY != 0)
            && !(*ctx).wp.is_null()
            && options_get_number_((*(*ctx).wp).options, c"scroll-on-clear") != 0
        {
            grid_view_clear_history(gd, bg);
        } else {
            if (*s).cx < sx {
                grid_view_clear(gd, (*s).cx, (*s).cy, sx - (*s).cx, 1, bg);
            }
            grid_view_clear(gd, 0, (*s).cy + 1, sx, sy - ((*s).cy + 1), bg);
        }

        screen_write_collect_clear(ctx, (*s).cy + 1, sy - ((*s).cy + 1));
        screen_write_collect_flush(ctx, 0, c!("screen_write_clearendofscreen"));
        tty_write(Some(tty_cmd_clearendofscreen), &raw mut ttyctx);
    }
}

/// Clear to start of screen.
pub unsafe fn screen_write_clearstartofscreen(ctx: *mut screen_write_ctx, bg: u32) {
    unsafe {
        let s = (*ctx).s;
        let mut ttyctx: tty_ctx = zeroed();
        let sx = screen_size_x(s);

        #[cfg(feature = "sixel")]
        {
            if image_check_line(s, 0, (*s).cy - 1) && (*ctx).wp.is_null() {
                (*(*ctx).wp).flags |= window_pane_flag::PANE_REDRAW;
            }
        }

        screen_write_initctx(ctx, &raw mut ttyctx, 1);
        ttyctx.bg = bg;

        if (*s).cy > 0 {
            grid_view_clear((*s).grid, 0, 0, sx, (*s).cy, bg);
        }
        if (*s).cx > sx - 1 {
            grid_view_clear((*s).grid, 0, (*s).cy, sx, 1, bg);
        } else {
            grid_view_clear((*s).grid, 0, (*s).cy, (*s).cx + 1, 1, bg);
        }

        screen_write_collect_clear(ctx, 0, (*s).cy);
        screen_write_collect_flush(ctx, 0, c!("screen_write_clearstartofscreen"));
        tty_write(Some(tty_cmd_clearstartofscreen), &raw mut ttyctx);
    }
}

/// Clear entire screen.
pub unsafe fn screen_write_clearscreen(ctx: *mut screen_write_ctx, bg: u32) {
    unsafe {
        let s = (*ctx).s;
        let mut ttyctx: tty_ctx = zeroed();
        let sx = screen_size_x(s);
        let sy = screen_size_y(s);

        #[cfg(feature = "sixel")]
        {
            if image_free_all(s) && !(*ctx).wp.is_null() {
                (*(*ctx).wp).flags |= window_pane_flag::PANE_REDRAW;
            }
        }

        screen_write_initctx(ctx, &raw mut ttyctx, 1);
        ttyctx.bg = bg;

        /* Scroll into history if it is enabled. */
        if ((*(*s).grid).flags & GRID_HISTORY != 0)
            && !(*ctx).wp.is_null()
            && options_get_number_((*(*ctx).wp).options, c"scroll-on-clear") != 0
        {
            grid_view_clear_history((*s).grid, bg);
        } else {
            grid_view_clear((*s).grid, 0, 0, sx, sy, bg);
        }

        screen_write_collect_clear(ctx, 0, sy);
        tty_write(Some(tty_cmd_clearscreen), &raw mut ttyctx);
    }
}

/// Clear entire history.
pub unsafe fn screen_write_clearhistory(ctx: *mut screen_write_ctx) {
    unsafe {
        grid_clear_history((*(*ctx).s).grid);
    }
}

/// Force a full redraw.
pub unsafe fn screen_write_fullredraw(ctx: *mut screen_write_ctx) {
    unsafe {
        let mut ttyctx: tty_ctx = zeroed();

        screen_write_collect_flush(ctx, 0, c!("screen_write_fullredraw"));

        screen_write_initctx(ctx, &raw mut ttyctx, 1);
        if let Some(redraw_cb) = ttyctx.redraw_cb {
            redraw_cb(&raw const ttyctx);
        }
    }
}

/// Trim collected items.
pub unsafe fn screen_write_collect_trim(
    ctx: *mut screen_write_ctx,
    y: u32,
    x: u32,
    used: u32,
    wrapped: *mut i32,
) -> *mut screen_write_citem {
    unsafe {
        let cl = (*(*ctx).s).write_list.add(y as usize);
        let mut before = null_mut();
        let sx = x;
        let ex = x + used - 1;

        if tailq_empty(&raw const (*cl).items) {
            return null_mut();
        }
        for ci in tailq_foreach(&raw mut (*cl).items).map(NonNull::as_ptr) {
            let csx = (*ci).x;
            let cex = (*ci).x + (*ci).used - 1;

            /* Item is entirely before. */
            if cex < sx {
                continue;
            } // log_debug("%s: %p %u-%u before %u-%u", __func__, ci, csx, cex, sx, ex);

            /* Item is entirely after. */
            if csx > ex {
                // log_debug("%s: %p %u-%u after %u-%u", __func__, ci, csx, cex, sx, ex);
                before = ci;
                break;
            }

            /* Item is entirely inside. */
            if csx >= sx && cex <= ex {
                // log_debug("%s: %p %u-%u inside %u-%u", __func__, ci, csx, cex, sx, ex);
                tailq_remove(&raw mut (*cl).items, ci);
                screen_write_free_citem(ci);
                if csx == 0 && (*ci).wrapped != 0 && !wrapped.is_null() {
                    *wrapped = 1;
                }
                continue;
            }

            /* Item under the start. */
            if csx < sx && cex >= sx && cex <= ex {
                // log_debug("%s: %p %u-%u start %u-%u", __func__, ci, csx, cex, sx, ex);
                (*ci).used = sx - csx;
                // log_debug("%s: %p now %u-%u", __func__, ci, (*ci).x, (*ci).x + (*ci).used + 1);
                continue;
            }

            /* Item covers the end. */
            if cex > ex && csx >= sx && csx <= ex {
                // log_debug("%s: %p %u-%u end %u-%u", __func__, ci, csx, cex, sx, ex);
                (*ci).x = ex + 1;
                (*ci).used = cex - ex;
                // log_debug("%s: %p now %u-%u", __func__, ci, (*ci).x, (*ci).x + (*ci).used + 1);
                before = ci;
                break;
            }

            /* Item must cover both sides. */
            // log_debug("%s: %p %u-%u under %u-%u", __func__, ci, csx, cex, sx, ex);
            let ci2 = screen_write_get_citem().as_ptr();
            (*ci2).type_ = (*ci).type_;
            (*ci2).bg = (*ci).bg;
            memcpy__(&raw mut (*ci2).gc, &raw mut (*ci).gc);
            tailq_insert_after(&raw mut (*cl).items, ci, ci2);

            (*ci).used = sx - csx;
            (*ci2).x = ex + 1;
            (*ci2).used = cex - ex;

            // log_debug("%s: %p now %u-%u (%p) and %u-%u (%p)", __func__, ci, (*ci).x, (*ci).x + (*ci).used - 1, ci, (*ci2).x, (*ci2).x + (*ci2).used - 1, ci2);
            before = ci2;
            break;
        }
        before
    }
}

/// Clear collected lines.
pub unsafe fn screen_write_collect_clear(ctx: *mut screen_write_ctx, y: u32, n: u32) {
    unsafe {
        for i in y..(y + n) {
            let cl = (*(*ctx).s).write_list.add(i as usize);
            tailq_concat(&raw mut SCREEN_WRITE_CITEM_FREELIST, &raw mut (*cl).items);
        }
    }
}

/// Scroll collected lines up.
pub unsafe fn screen_write_collect_scroll(ctx: *mut screen_write_ctx, bg: u32) {
    unsafe {
        let s = (*ctx).s;
        // log_debug("%s: at %u,%u (region %u-%u)", __func__, (*s).cx, (*s).cy, (*s).rupper, (*s).rlower);

        screen_write_collect_clear(ctx, (*s).rupper, 1);
        let saved = (*(*(*ctx).s).write_list.add((*s).rupper as usize)).data;
        for y in (*s).rupper..(*s).rlower {
            let cl = (*(*ctx).s).write_list.add(y as usize + 1);
            tailq_concat(
                &raw mut (*(*(*ctx).s).write_list.add(y as usize)).items,
                &raw mut (*cl).items,
            );
            (*(*(*ctx).s).write_list.add(y as usize)).data = (*cl).data;
        }
        (*(*(*ctx).s).write_list.add((*s).rlower as usize)).data = saved;

        let ci = screen_write_get_citem().as_ptr();
        (*ci).x = 0;
        (*ci).used = screen_size_x(s);
        (*ci).type_ = screen_write_citem_type::Clear;
        (*ci).bg = bg;
        tailq_insert_tail(
            &raw mut (*(*(*ctx).s).write_list.add((*s).rlower as usize)).items,
            ci,
        );
    }
}

/// Flush collected lines.
pub unsafe fn screen_write_collect_flush(
    ctx: *mut screen_write_ctx,
    scroll_only: u32,
    from: *const u8,
) {
    unsafe {
        let s = (*ctx).s;
        // struct screen_write_citem *ci, *tmp;
        // struct screen_write_cline *cl;
        // u_int y, cx, cy, last, items = 0;
        let mut items = 0;
        let mut ttyctx: tty_ctx = zeroed();

        if (*ctx).scrolled != 0 {
            // log_debug("%s: scrolled %u (region %u-%u)", __func__, (*ctx).scrolled, (*s).rupper, (*s).rlower);
            if (*ctx).scrolled > (*s).rlower - (*s).rupper + 1 {
                (*ctx).scrolled = (*s).rlower - (*s).rupper + 1;
            }

            screen_write_initctx(ctx, &raw mut ttyctx, 1);
            ttyctx.num = (*ctx).scrolled;
            ttyctx.bg = (*ctx).bg;
            tty_write(Some(tty_cmd_scrollup), &raw mut ttyctx);
        }
        (*ctx).scrolled = 0;
        (*ctx).bg = 8;

        if scroll_only != 0 {
            return;
        }

        let cx = (*s).cx;
        let cy = (*s).cy;
        for y in 0..screen_size_y(s) {
            let cl = (*(*ctx).s).write_list.add(y as usize);
            let mut last = u32::MAX;
            for ci in tailq_foreach(&raw mut (*cl).items).map(NonNull::as_ptr) {
                if last != u32::MAX && (*ci).x <= last {
                    panic!("collect list not in order: {} <= {}", (*ci).x, last);
                } // fatalx("collect list not in order: %u <= %u", (*ci).x, last);
                screen_write_set_cursor(ctx, (*ci).x as i32, y as i32);
                if (*ci).type_ == screen_write_citem_type::Clear {
                    screen_write_initctx(ctx, &raw mut ttyctx, 1);
                    ttyctx.bg = (*ci).bg;
                    ttyctx.num = (*ci).used;
                    tty_write(Some(tty_cmd_clearcharacter), &raw mut ttyctx);
                } else {
                    screen_write_initctx(ctx, &raw mut ttyctx, 0);
                    ttyctx.cell = &(*ci).gc;
                    ttyctx.wrapped = (*ci).wrapped;
                    ttyctx.ptr = (*cl).data.add((*ci).x as usize).cast();
                    ttyctx.num = (*ci).used;
                    tty_write(Some(tty_cmd_cells), &raw mut ttyctx);
                }
                items += 1;

                tailq_remove(&raw mut (*cl).items, ci);
                screen_write_free_citem(ci);
                last = (*ci).x;
            }
        }
        (*s).cx = cx;
        (*s).cy = cy;

        // log_debug("%s: flushed %u items (%s)", __func__, items, from);
    }
}

/// Finish and store collected cells.
pub unsafe fn screen_write_collect_end(ctx: *mut screen_write_ctx) {
    unsafe {
        let s = (*ctx).s;
        let ci = (*ctx).item;
        let cl = (*s).write_list.add((*s).cy as usize);
        let mut gc: grid_cell = zeroed();
        let mut wrapped = (*ci).wrapped;

        if (*ci).used == 0 {
            return;
        }

        let before = screen_write_collect_trim(ctx, (*s).cy, (*s).cx, (*ci).used, &raw mut wrapped);
        (*ci).x = (*s).cx;
        (*ci).wrapped = wrapped;
        if before.is_null() {
            tailq_insert_tail(&raw mut (*cl).items, ci);
        } else {
            tailq_insert_before(before, ci);
        }
        (*ctx).item = screen_write_get_citem().as_ptr();

        // log_debug("%s: %u %.*s (at %u,%u)", __func__, (*ci).used, (int)(*ci).used, (*cl).data + (*ci).x, (*s).cx, (*s).cy);

        if (*s).cx != 0 {
            let mut xx = (*s).cx;
            while xx > 0 {
                grid_view_get_cell((*s).grid, xx, (*s).cy, &raw mut gc);
                if !gc.flags.intersects(grid_flag::PADDING) {
                    break;
                }
                grid_view_set_cell((*s).grid, xx, (*s).cy, &GRID_DEFAULT_CELL);
                xx -= 1;
            }
            if gc.data.width > 1 {
                grid_view_set_cell((*s).grid, xx, (*s).cy, &GRID_DEFAULT_CELL);
            }
        }

        #[cfg(feature = "sixel")]
        {
            if image_check_area(s, (*s).cx, (*s).cy, (*ci).used, 1) && !(*ctx).wp.is_null() {
                (*(*ctx).wp).flags |= window_pane_flag::PANE_REDRAW;
            }
        }

        grid_view_set_cells(
            (*s).grid,
            (*s).cx,
            (*s).cy,
            &(*ci).gc,
            (*cl).data.add((*ci).x as usize),
            (*ci).used as usize,
        );
        screen_write_set_cursor(ctx, ((*s).cx + (*ci).used) as i32, -1);

        for xx in (*s).cx..screen_size_x(s) {
            grid_view_get_cell((*s).grid, xx, (*s).cy, &raw mut gc);
            if !gc.flags.intersects(grid_flag::PADDING) {
                break;
            }
            grid_view_set_cell((*s).grid, xx, (*s).cy, &GRID_DEFAULT_CELL);
        }
    }
}

/// Write cell data, collecting if necessary.
pub unsafe fn screen_write_collect_add(ctx: *mut screen_write_ctx, gc: *const grid_cell) {
    unsafe {
        let s = (*ctx).s;
        let mut ci: *mut screen_write_citem = null_mut();
        let sx = screen_size_x(s);

        /*
         * Don't need to check that the attributes and whatnot are still the
         * same - input_parse will end the collection when anything that isn't
         * a plain character is encountered.
         */

        if ((*gc).data.width != 1 || (*gc).data.size != 1 || (*gc).data.data[0] >= 0x7f)
            || (*gc).attr.intersects(grid_attr::GRID_ATTR_CHARSET)
            || !(*s).mode.intersects(mode_flag::MODE_WRAP)
            || (*s).mode.intersects(mode_flag::MODE_INSERT)
            || !(*s).sel.is_null()
        {
            screen_write_collect_end(ctx);
            screen_write_collect_flush(ctx, 0, c!("screen_write_collect_add"));
            screen_write_cell(ctx, gc);
            return;
        }

        if (*s).cx > sx - 1 || (*(*ctx).item).used > sx - 1 - (*s).cx {
            screen_write_collect_end(ctx);
        }
        ci = (*ctx).item; /* may have changed */

        if (*s).cx > sx - 1 {
            // log_debug!("%s: wrapped at %u,%u", __func__, (*s).cx, (*s).cy);
            (*ci).wrapped = 1;
            screen_write_linefeed(ctx, 1, 8);
            screen_write_set_cursor(ctx, 0, -1);
        }

        if (*ci).used == 0 {
            memcpy__(&raw mut (*ci).gc, gc);
        }
        if (*(*(*ctx).s).write_list.add((*s).cy as usize))
            .data
            .is_null()
        {
            (*(*(*ctx).s).write_list.add((*s).cy as usize)).data =
                xmalloc(screen_size_x((*ctx).s) as usize).as_ptr().cast();
        }
        *(*(*(*ctx).s).write_list.add((*s).cy as usize))
            .data
            .add(((*s).cx + (*ci).used) as usize) = (*gc).data.data[0];
        (*ci).used += 1;
    }
}

/// Write cell data.
pub unsafe fn screen_write_cell(ctx: *mut screen_write_ctx, gc: *const grid_cell) {
    unsafe {
        let s = (*ctx).s;
        let gd = (*s).grid;
        let ud = &raw const (*gc).data;

        let mut gl: *mut grid_line = null_mut();
        let mut gce: *mut grid_cell_entry = null_mut();

        const SIZE_OF_TMP_GC: usize = size_of::<grid_cell>();
        let mut tmp_gc: grid_cell = zeroed();
        let mut now_gc: grid_cell = zeroed();
        let mut ttyctx: tty_ctx = zeroed();

        let sx = screen_size_x(s);
        let sy = screen_size_y(s);

        let width = (*ud).width as u32;
        // xx, not_wrap;
        let mut skip = 1;

        /* Ignore padding cells. */
        if (*gc).flags.intersects(grid_flag::PADDING) {
            return;
        }

        /* Get the previous cell to check for combining. */
        if screen_write_combine(ctx, gc) != 0 {
            return;
        }

        /* Flush any existing scrolling. */
        screen_write_collect_flush(ctx, 1, c!("screen_write_cell"));

        /* If this character doesn't fit, ignore it. */
        if !(*s).mode.intersects(mode_flag::MODE_WRAP)
            && width > 1
            && (width > sx || ((*s).cx != sx && (*s).cx > sx - width))
        {
            return;
        }

        /* If in insert mode, make space for the cells. */
        if (*s).mode.intersects(mode_flag::MODE_INSERT) {
            grid_view_insert_cells((*s).grid, (*s).cx, (*s).cy, width, 8);
            skip = 0;
        }

        /* Check this will fit on the current line and wrap if not. */
        if (*s).mode.intersects(mode_flag::MODE_WRAP) && (*s).cx > sx - width {
            // log_debug("%s: wrapped at %u,%u", __func__, (*s).cx, (*s).cy);
            screen_write_linefeed(ctx, 1, 8);
            screen_write_set_cursor(ctx, 0, -1);
            screen_write_collect_flush(ctx, 1, c!("screen_write_cell"));
        }

        /* Sanity check cursor position. */
        if (*s).cx > sx - width || (*s).cy > sy - 1 {
            return;
        }
        screen_write_initctx(ctx, &raw mut ttyctx, 0);

        /* Handle overwriting of UTF-8 characters. */
        gl = grid_get_line((*s).grid, (*(*s).grid).hsize + (*s).cy);
        if (*gl).flags.intersects(grid_line_flag::EXTENDED) {
            grid_view_get_cell(gd, (*s).cx, (*s).cy, &raw mut now_gc);
            if screen_write_overwrite(ctx, &raw mut now_gc, width) != 0 {
                skip = 0;
            }
        }

        /*
         * If the new character is UTF-8 wide, fill in padding cells. Have
         * already ensured there is enough room.
         */
        for xx in ((*s).cx + 1)..((*s).cx + width) {
            // log_debug("%s: new padding at %u,%u", __func__, xx, (*s).cy);
            grid_view_set_padding(gd, xx, (*s).cy);
            skip = 0;
        }

        // If no change, do not draw.
        if skip != 0 {
            if (*s).cx >= (*gl).cellsize {
                skip = grid_cells_equal(gc, &GRID_DEFAULT_CELL);
            } else {
                gce = (*gl).celldata.add((*s).cx as usize);
                if (*gce).flags.intersects(grid_flag::EXTENDED)
                    || (*gc).flags != (*gce).flags
                    || (*gc).attr.bits() != (*gce).union_.data.attr as u16
                    || (*gc).fg != (*gce).union_.data.fg as i32
                    || (*gc).bg != (*gce).union_.data.bg as i32
                    || (*gc).data.width != 1
                    || (*gc).data.size != 1
                    || (*gce).union_.data.data != (*gc).data.data[0]
                {
                    skip = 0;
                }
            }
        }

        /* Update the selected flag and set the cell. */
        let selected = screen_check_selection(s, (*s).cx, (*s).cy) != 0;
        if selected && !(*gc).flags.intersects(grid_flag::SELECTED) {
            memcpy__(&raw mut tmp_gc, gc);
            tmp_gc.flags |= grid_flag::SELECTED;
            grid_view_set_cell(gd, (*s).cx, (*s).cy, &raw const tmp_gc);
        } else if !selected && ((*gc).flags.intersects(grid_flag::SELECTED)) {
            memcpy__(&raw mut tmp_gc, gc);
            tmp_gc.flags &= !grid_flag::SELECTED;
            grid_view_set_cell(gd, (*s).cx, (*s).cy, &tmp_gc);
        } else if skip == 0 {
            grid_view_set_cell(gd, (*s).cx, (*s).cy, gc);
        }
        if selected {
            skip = 0;
        }

        /*
         * Move the cursor. If not wrapping, stick at the last character and
         * replace it.
         */
        let not_wrap = !((*s).mode.intersects(mode_flag::MODE_WRAP)) as i32;
        if (*s).cx <= (sx as i32 - not_wrap - width as i32) as u32 {
            screen_write_set_cursor(ctx, ((*s).cx + width) as i32, -1);
        } else {
            screen_write_set_cursor(ctx, sx as i32 - not_wrap, -1);
        }

        /* Create space for character in insert mode. */
        if (*s).mode.intersects(mode_flag::MODE_INSERT) {
            screen_write_collect_flush(ctx, 0, c!("screen_write_cell"));
            ttyctx.num = width;
            tty_write(Some(tty_cmd_insertcharacter), &raw mut ttyctx);
        }

        /* Write to the screen. */
        if skip == 0 {
            if selected {
                screen_select_cell(s, &raw mut tmp_gc, gc);
                ttyctx.cell = &tmp_gc;
            } else {
                ttyctx.cell = gc;
            }
            tty_write(Some(tty_cmd_cell), &raw mut ttyctx);
        }
    }
}

/// Combine a UTF-8 zero-width character onto the previous if necessary.
pub unsafe fn screen_write_combine(ctx: *mut screen_write_ctx, gc: *const grid_cell) -> i32 {
    unsafe {
        let s = (*ctx).s;
        let gd = (*s).grid;
        let ud: *const utf8_data = &raw const (*gc).data;
        let mut cx = (*s).cx;
        let cy = (*s).cy;

        let mut last: grid_cell = zeroed();
        let mut ttyctx: tty_ctx = zeroed();

        let mut force_wide = 0;
        let mut zero_width = 0;

        /*
         * Is this character which makes no sense without being combined? If
         * this is true then flag it here and discard the character (return 1)
         * if we cannot combine it.
         */
        if utf8_is_zwj(ud) != 0 {
            zero_width = 1;
        } else if utf8_is_vs(ud) != 0 {
            zero_width = 1;
            force_wide = 1;
        } else if (*ud).width == 0 {
            zero_width = 1;
        }

        /* Cannot combine empty character or at left. */
        if (*ud).size < 2 || cx == 0 {
            return zero_width;
        }
        // log_debug("%s: character %.*s at %u,%u (width %u)", __func__, (int)(*ud).size, (*ud).data, cx, cy, (*ud).width);

        /* Find the cell to combine with. */
        let mut n = 1;
        grid_view_get_cell(gd, cx - n, cy, &raw mut last);
        if cx != 1 && last.flags.intersects(grid_flag::PADDING) {
            n = 2;
            grid_view_get_cell(gd, cx - n, cy, &raw mut last);
        }
        if n != last.data.width as u32 || last.flags.intersects(grid_flag::PADDING) {
            return zero_width;
        }

        /*
         * Check if we need to combine characters. This could be zero width
         * (set above), a modifier character (with an existing Unicode
         * character) or a previous ZWJ.
         */
        if zero_width == 0 {
            if utf8_is_modifier(ud) != 0 {
                if last.data.size < 2 {
                    return 0;
                }
                force_wide = 1;
            } else if utf8_has_zwj(&raw mut last.data) == 0 {
                return 0;
            }
        }

        /* Check if this combined character would be too long. */
        if last.data.size + (*ud).size > UTF8_SIZE as u8 {
            return 0;
        }

        /* Combining; flush any pending output. */
        screen_write_collect_flush(ctx, 0, c!("screen_write_combine"));

        // log_debug("%s: %.*s -> %.*s at %u,%u (offset %u, width %u)", __func__, (int)(*ud).size, (*ud).data, (int)last.data.size, last.data.data, cx - n, cy, n, last.data.width);

        /* Append the data. */
        libc::memcpy(
            (&raw mut last.data.data[last.data.size as usize]).cast(),
            (&raw const (*ud).data).cast(),
            (*ud).size as usize,
        );
        last.data.size += (*ud).size;

        /* Force the width to 2 for modifiers and variation selector. */
        if last.data.width == 1 && force_wide != 0 {
            last.data.width = 2;
            n = 2;
            cx += 1;
        } else {
            force_wide = 0;
        }

        /* Set the new cell. */
        grid_view_set_cell(gd, cx - n, cy, &last);
        if force_wide != 0 {
            grid_view_set_padding(gd, cx - 1, cy);
        }

        /*
         * Redraw the combined cell. If forcing the cell to width 2, reset the
         * cached cursor position in the tty, since we don't really know
         * whether the terminal thought the character was width 1 or width 2
         * and what it is going to do now.
         */
        screen_write_set_cursor(ctx, cx as i32 - n as i32, cy as i32);
        screen_write_initctx(ctx, &raw mut ttyctx, 0);
        ttyctx.cell = &raw const last;
        ttyctx.num = force_wide; /* reset cached cursor position */
        tty_write(Some(tty_cmd_cell), &raw mut ttyctx);
        screen_write_set_cursor(ctx, cx as i32, cy as i32);

        1
    }
}

/*
 * UTF-8 wide characters are a bit of an annoyance. They take up more than one
 * cell on the screen, so following cells must not be drawn by marking them as
 * padding.
 *
 * So far, so good. The problem is, when overwriting a padding cell, or a UTF-8
 * character, it is necessary to also overwrite any other cells which covered
 * by the same character.
 */

pub unsafe fn screen_write_overwrite(
    ctx: *mut screen_write_ctx,
    gc: *mut grid_cell,
    width: u32,
) -> i32 {
    unsafe {
        let s = (*ctx).s;
        let gd = (*s).grid;

        let mut tmp_gc: grid_cell = zeroed();
        let mut xx: u32 = 0;
        let mut done = 0;

        if (*gc).flags.intersects(grid_flag::PADDING) {
            /*
             * A padding cell, so clear any following and leading padding
             * cells back to the character. Don't overwrite the current
             * cell as that happens later anyway.
             */
            xx = (*s).cx + 1;
            while {
                xx -= 1;
                xx > 0
            } {
                grid_view_get_cell(gd, xx, (*s).cy, &raw mut tmp_gc);
                if !tmp_gc.flags.intersects(grid_flag::PADDING) {
                    break;
                }
                // log_debug("%s: padding at %u,%u", __func__, xx, (*s).cy);
                grid_view_set_cell(gd, xx, (*s).cy, &raw const GRID_DEFAULT_CELL);
            }

            /* Overwrite the character at the start of this padding. */
            // log_debug("%s: character at %u,%u", __func__, xx, (*s).cy);
            grid_view_set_cell(gd, xx, (*s).cy, &raw const GRID_DEFAULT_CELL);
            done = 1;
        }

        /*
         * Overwrite any padding cells that belong to any UTF-8 characters
         * we'll be overwriting with the current character.
         */
        if width != 1 || (*gc).data.width != 1 || (*gc).flags.intersects(grid_flag::PADDING) {
            xx = (*s).cx + width - 1;
            while {
                xx += 1;
                xx < screen_size_x(s)
            } {
                grid_view_get_cell(gd, xx, (*s).cy, &raw mut tmp_gc);
                if !tmp_gc.flags.intersects(grid_flag::PADDING) {
                    break;
                }
                // log_debug("%s: overwrite at %u,%u", __func__, xx, (*s).cy);
                grid_view_set_cell(gd, xx, (*s).cy, &raw const GRID_DEFAULT_CELL);
                done = 1;
            }
        }

        done
    }
}

/// Set external clipboard.
pub unsafe fn screen_write_setselection(
    ctx: *mut screen_write_ctx,
    flags: *const u8,
    str: *mut u8,
    len: u32,
) {
    unsafe {
        let mut ttyctx: tty_ctx = zeroed();

        screen_write_initctx(ctx, &raw mut ttyctx, 0);
        ttyctx.ptr = str.cast();
        ttyctx.ptr2 = flags as *mut c_void; // TODO casting away const
        ttyctx.num = len;

        tty_write(Some(tty_cmd_setselection), &raw mut ttyctx);
    }
}

/// Write unmodified string.
pub unsafe fn screen_write_rawstring(
    ctx: *mut screen_write_ctx,
    str: *mut u8,
    len: u32,
    allow_invisible_panes: i32,
) {
    unsafe {
        let mut ttyctx: tty_ctx = zeroed();

        screen_write_initctx(ctx, &raw mut ttyctx, 0);
        ttyctx.ptr = str.cast();
        ttyctx.num = len;
        ttyctx.allow_invisible_panes = allow_invisible_panes;

        tty_write(Some(tty_cmd_rawstring), &raw mut ttyctx);
    }
}

// TODO
/// Write a SIXEL image.
#[cfg(feature = "sixel")]
unsafe fn screen_write_sixelimage(ctx: *mut screen_write_ctx, si: *mut sixel_image, bg: u32) {
    unsafe {
        let mut s = (*ctx).s;
        let mut gd = (*s).grid;
        let mut ttyctx: tty_ctx = zeroed();

        // u_int x, y, sx, sy, cx = (*s).cx, cy = (*s).cy, i, lines;
        let mut x: u32 = 0;
        let mut y: u32 = 0;
        let mut sx: u32 = 0;
        let mut sy: u32 = 0;
        let mut cx: u32 = (*s).cx;
        let mut cy: u32 = (*s).cy;
        let mut new: *mut sixel_image = null_mut();

        sixel_size_in_cells(si, &raw mut x, &raw mut y);
        if (x > screen_size_x(s) || y > screen_size_y(s)) {
            if (x > screen_size_x(s) - cx) {
                sx = screen_size_x(s) - cx;
            } else {
                sx = x;
            }
            if (y > screen_size_y(s) - 1) {
                sy = screen_size_y(s) - 1;
            } else {
                sy = y;
            }
            new = sixel_scale(si, 0, 0, 0, y - sy, sx, sy, 1);
            sixel_free(si);
            si = new;

            /* Bail out if the image cannot be scaled. */
            if si.is_null() {
                return;
            }
            sixel_size_in_cells(si, &x, &y);
        }

        sy = screen_size_y(s) - cy;
        if (sy < y) {
            lines = y - sy + 1;
            if image_scroll_up(s, lines) && (*ctx).wp != NULL {
                (*(*ctx).wp).flags |= PANE_REDRAW;
            }
            for i in 0..lines {
                grid_view_scroll_region_up(gd, 0, screen_size_y(s) - 1, bg);
                screen_write_collect_scroll(ctx, bg);
            }
            (*ctx).scrolled += lines;
            if (lines > cy) {
                screen_write_cursormove(ctx, -1, 0, 0);
            } else {
                screen_write_cursormove(ctx, -1, cy - lines, 0);
            }
        }
        screen_write_collect_flush(ctx, 0, __func__);

        screen_write_initctx(ctx, &ttyctx, 0);
        ttyctx.ptr = image_store(s, si);

        tty_write(tty_cmd_sixelimage, &ttyctx);

        screen_write_cursormove(ctx, 0, cy + y, 0);
    }
}

/// Turn alternate screen on.
pub unsafe fn screen_write_alternateon(
    ctx: *mut screen_write_ctx,
    gc: *mut grid_cell,
    cursor: i32,
) {
    unsafe {
        let mut ttyctx: tty_ctx = zeroed();
        let wp = (*ctx).wp;

        if !wp.is_null() && options_get_number_((*wp).options, c"alternate-screen") == 0 {
            return;
        }

        screen_write_collect_flush(ctx, 0, c!("screen_write_alternateon"));
        screen_alternate_on((*ctx).s, gc, cursor);

        screen_write_initctx(ctx, &raw mut ttyctx, 1);
        if let Some(redraw_cb) = ttyctx.redraw_cb {
            redraw_cb(&raw const ttyctx);
        }
    }
}

/// Turn alternate screen off.
pub unsafe fn screen_write_alternateoff(
    ctx: *mut screen_write_ctx,
    gc: *mut grid_cell,
    cursor: i32,
) {
    unsafe {
        let mut ttyctx: tty_ctx = zeroed();
        let wp = (*ctx).wp;

        if !wp.is_null() && !options_get_number_((*wp).options, c"alternate-screen") != 0 {
            return;
        }

        screen_write_collect_flush(ctx, 0, c!("screen_write_alternateoff"));
        screen_alternate_off((*ctx).s, gc, cursor);

        screen_write_initctx(ctx, &raw mut ttyctx, 1);
        if let Some(redraw_cb) = ttyctx.redraw_cb {
            redraw_cb(&raw mut ttyctx);
        }
    }
}

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

use std::ptr::{addr_of, addr_of_mut};

use crate::compat::{
    impl_tailq_entry,
    queue::{tailq_first, tailq_init, tailq_insert_head, tailq_remove},
    strlcat,
};

/// Selected area in screen.
#[repr(C)]
pub struct screen_sel {
    pub hidden: i32,
    pub rectangle: i32,
    pub modekeys: modekey,

    pub sx: u32,
    pub sy: u32,

    pub ex: u32,
    pub ey: u32,

    pub cell: grid_cell,
}

impl_tailq_entry!(screen_title_entry, entry, tailq_entry<screen_title_entry>);
/// Entry on title stack.
#[repr(C)]
pub struct screen_title_entry {
    pub text: *mut u8,

    pub entry: tailq_entry<screen_title_entry>,
}
pub type screen_titles = tailq_head<screen_title_entry>;

/// Free titles stack.
pub unsafe fn screen_free_titles(s: *mut screen) {
    unsafe {
        if (*s).titles.is_null() {
            return;
        }

        while let Some(title_entry) = NonNull::new(tailq_first((*s).titles)) {
            let title_entry = title_entry.as_ptr();
            tailq_remove((*s).titles, title_entry);
            free_((*title_entry).text);
            free_(title_entry);
        }

        free_((*s).titles);
        (*s).titles = null_mut();
    }
}

/// Create a new screen.
pub unsafe fn screen_init(s: *mut screen, sx: u32, sy: u32, hlimit: u32) {
    unsafe {
        (*s).grid = grid_create(sx, sy, hlimit);
        (*s).saved_grid = null_mut();

        (*s).title = xstrdup_(c"").as_ptr();
        (*s).titles = null_mut();
        (*s).path = null_mut();

        (*s).cstyle = screen_cursor_style::SCREEN_CURSOR_DEFAULT;
        (*s).default_cstyle = screen_cursor_style::SCREEN_CURSOR_DEFAULT;
        (*s).mode = mode_flag::MODE_CURSOR;
        (*s).default_mode = mode_flag::empty();
        (*s).ccolour = -1;
        (*s).default_ccolour = -1;
        (*s).tabs = null_mut();
        (*s).sel = null_mut();

        #[cfg(feature = "sixel")]
        tailq_init(&raw mut (*s).images);

        (*s).write_list = null_mut();
        (*s).hyperlinks = null_mut();

        screen_reinit(s);
    }
}

/// Reinitialise screen.
pub unsafe fn screen_reinit(s: *mut screen) {
    unsafe {
        (*s).cx = 0;
        (*s).cy = 0;

        (*s).rupper = 0;
        (*s).rlower = screen_size_y(s) - 1;

        (*s).mode =
            mode_flag::MODE_CURSOR | mode_flag::MODE_WRAP | ((*s).mode & mode_flag::MODE_CRLF);

        if options_get_number_(global_options, c"extended-keys") == 2 {
            (*s).mode = ((*s).mode & !EXTENDED_KEY_MODES) | mode_flag::MODE_KEYS_EXTENDED;
        }

        if !(*s).saved_grid.is_null() {
            screen_alternate_off(s, null_mut(), 0);
        }
        (*s).saved_cx = u32::MAX;
        (*s).saved_cy = u32::MAX;

        screen_reset_tabs(s);

        grid_clear_lines((*s).grid, (*(*s).grid).hsize, (*(*s).grid).sy, 8);

        screen_clear_selection(s);
        screen_free_titles(s);

        #[cfg(feature = "sixel")]
        image_free_all(s);

        screen_reset_hyperlinks(s);
    }
}

/// Reset hyperlinks of a screen.
pub unsafe fn screen_reset_hyperlinks(s: *mut screen) {
    unsafe {
        if (*s).hyperlinks.is_null() {
            (*s).hyperlinks = hyperlinks_init();
        } else {
            hyperlinks_reset((*s).hyperlinks);
        }
    }
}

/// Destroy a screen.
pub unsafe fn screen_free(s: *mut screen) {
    unsafe {
        free_((*s).sel);
        free_((*s).tabs);
        free_((*s).path);
        free_((*s).title);

        if !(*s).write_list.is_null() {
            screen_write_free_list(s);
        }

        if !(*s).saved_grid.is_null() {
            grid_destroy((*s).saved_grid);
        }
        grid_destroy((*s).grid);

        if !(*s).hyperlinks.is_null() {
            hyperlinks_free((*s).hyperlinks);
        }
        screen_free_titles(s);

        #[cfg(feature = "sixel")]
        image_free_all(s);
    }
}

/// Reset tabs to default, eight spaces apart.
pub unsafe fn screen_reset_tabs(s: *mut screen) {
    unsafe {
        free_((*s).tabs);

        (*s).tabs = bit_alloc(screen_size_x(s));
        if (*s).tabs.is_null() {
            fatal("bit_alloc failed");
        }

        let mut i = 8;
        while i < screen_size_x(s) {
            bit_set((*s).tabs, i);
            i += 8;
        }
    }
}

unsafe fn bit_alloc(nbits: u32) -> *mut u8 {
    unsafe { libc::calloc(nbits.div_ceil(8) as usize, 1).cast() }
}
unsafe fn bit_set(bits: *mut u8, i: u32) {
    unsafe {
        let byte_index = i / 8;
        let bit_index = i % 8;
        *bits.add(byte_index as usize) |= 1 << bit_index;
    }
}

/// Set screen cursor style and mode.
pub unsafe fn screen_set_cursor_style(
    style: u32,
    cstyle: *mut screen_cursor_style,
    mode: *mut mode_flag,
) {
    unsafe {
        match style {
            0 => *cstyle = screen_cursor_style::SCREEN_CURSOR_DEFAULT,
            1 => {
                *cstyle = screen_cursor_style::SCREEN_CURSOR_BLOCK;
                *mode |= mode_flag::MODE_CURSOR_BLINKING;
            }
            2 => {
                *cstyle = screen_cursor_style::SCREEN_CURSOR_BLOCK;
                *mode &= !mode_flag::MODE_CURSOR_BLINKING;
            }
            3 => {
                *cstyle = screen_cursor_style::SCREEN_CURSOR_UNDERLINE;
                *mode |= mode_flag::MODE_CURSOR_BLINKING;
            }
            4 => {
                *cstyle = screen_cursor_style::SCREEN_CURSOR_UNDERLINE;
                *mode &= !mode_flag::MODE_CURSOR_BLINKING;
            }
            5 => {
                *cstyle = screen_cursor_style::SCREEN_CURSOR_BAR;
                *mode |= mode_flag::MODE_CURSOR_BLINKING;
            }
            6 => {
                *cstyle = screen_cursor_style::SCREEN_CURSOR_BAR;
                *mode &= !mode_flag::MODE_CURSOR_BLINKING;
            }
            _ => (),
        }
    }
}

/// Set screen cursor colour.
pub unsafe fn screen_set_cursor_colour(s: *mut screen, colour: c_int) {
    unsafe {
        (*s).ccolour = colour;
    }
}

/// Set screen title.
pub unsafe fn screen_set_title(s: *mut screen, title: *const u8) -> c_int {
    unsafe {
        if !utf8_isvalid(title) {
            return 0;
        }
        free_((*s).title);
        (*s).title = xstrdup(title).as_ptr();
        1
    }
}

/// Set screen path.
pub unsafe fn screen_set_path(s: *mut screen, path: *const u8) {
    unsafe {
        free_((*s).path);
        utf8_stravis(
            &mut (*s).path,
            path,
            vis_flags::VIS_OCTAL | vis_flags::VIS_CSTYLE | vis_flags::VIS_TAB | vis_flags::VIS_NL,
        );
    }
}

/// Push the current title onto the stack.
pub unsafe fn screen_push_title(s: *mut screen) {
    unsafe {
        if (*s).titles.is_null() {
            (*s).titles = xmalloc_::<screen_titles>().as_ptr();
            tailq_init((*s).titles);
        }

        let title_entry = xmalloc_::<screen_title_entry>().as_ptr();
        (*title_entry).text = xstrdup((*s).title).as_ptr();
        tailq_insert_head((*s).titles, title_entry);
    }
}

/// Pop a title from the stack and set it as the screen title. If the stack is empty, do nothing.
pub unsafe fn screen_pop_title(s: *mut screen) {
    unsafe {
        if (*s).titles.is_null() {
            return;
        }

        if let Some(title_entry) = NonNull::new(tailq_first((*s).titles)) {
            screen_set_title(s, (*title_entry.as_ptr()).text);

            tailq_remove((*s).titles, title_entry.as_ptr());
            free_((*title_entry.as_ptr()).text);
            free_(title_entry.as_ptr());
        }
    }
}

/// Resize screen with options.
pub unsafe fn screen_resize_cursor(
    s: *mut screen,
    sx: u32,
    sy: u32,
    mut reflow: i32,
    eat_empty: i32,
    cursor: i32,
) {
    let __func__ = "screen_resize_cursor";
    unsafe {
        let mut cx = (*s).cx;
        let mut cy = (*(*s).grid).hsize + (*s).cy;

        if !(*s).write_list.is_null() {
            screen_write_free_list(s);
        }

        log_debug!(
            "{}: new size {}{}, now {}x{} (cursor {},{} = {},{})",
            __func__,
            sx,
            sy,
            screen_size_x(s),
            screen_size_y(s),
            (*s).cx,
            (*s).cy,
            cx,
            cy,
        );

        let sx = if sx < 1 { 1 } else { sx };
        let sy = if sy < 1 { 1 } else { sy };

        if sx != screen_size_x(s) {
            (*(*s).grid).sx = sx;
            screen_reset_tabs(s);
        } else {
            reflow = 0;
        }

        if sy != screen_size_y(s) {
            screen_resize_y(s, sy, eat_empty, &mut cy);
        }

        #[cfg(feature = "sixel")]
        image_free_all(s);

        if reflow != 0 {
            screen_reflow(s, sx, &mut cx, &mut cy, cursor);
        }

        if cy >= (*(*s).grid).hsize {
            (*s).cx = cx;
            (*s).cy = cy - (*(*s).grid).hsize;
        } else {
            (*s).cx = 0;
            (*s).cy = 0;
        }

        log_debug!(
            "{}: cursor finished at {},{} = {},{}",
            __func__,
            (*s).cx,
            (*s).cy,
            cx,
            cy,
        );

        if !(*s).write_list.is_null() {
            screen_write_make_list(s);
        }
    }
}

/// Resize screen.
pub unsafe fn screen_resize(s: *mut screen, sx: u32, sy: u32, reflow: i32) {
    unsafe {
        screen_resize_cursor(s, sx, sy, reflow, 1, 1);
    }
}

/// Resize screen vertically.
unsafe fn screen_resize_y(s: *mut screen, sy: u32, eat_empty: i32, cy: *mut u32) {
    unsafe {
        let gd = (*s).grid;

        if sy == 0 {
            fatalx("zero size");
        }
        let oldy = screen_size_y(s);

        // When resizing:
        //
        // If the height is decreasing, delete lines from the bottom until
        // hitting the cursor, then push lines from the top into the history.
        //
        // When increasing, pull as many lines as possible from scrolled
        // history (not explicitly cleared from view) to the top, then fill the
        // remaining with blanks at the bottom.

        // Size decreasing
        if sy < oldy {
            let mut needed = oldy - sy;

            // Delete as many lines as possible from the bottom
            if eat_empty != 0 {
                let mut available = oldy - 1 - (*s).cy;
                if available > 0 {
                    if available > needed {
                        available = needed;
                    }
                    grid_view_delete_lines(gd, oldy - available, available, 8);
                }
                needed -= available;
            }

            // Now just increase the history size, if possible, to take
            // over the lines which are left. If history is off, delete
            // lines from the top.
            let mut available = (*s).cy;
            if (*gd).flags & GRID_HISTORY != 0 {
                (*gd).hscrolled += needed;
                (*gd).hsize += needed;
            } else if needed > 0 && available > 0 {
                if available > needed {
                    available = needed;
                }
                grid_view_delete_lines(gd, 0, available, 8);
                *cy -= available;
            }
        }

        // Resize line array
        grid_adjust_lines(gd, (*gd).hsize + sy);

        // Size increasing
        if sy > oldy {
            let mut needed = sy - oldy;

            // Try to pull as much as possible out of scrolled history, if
            // it is enabled.
            let mut available = (*gd).hscrolled;
            if (*gd).flags & GRID_HISTORY != 0 && available > 0 {
                if available > needed {
                    available = needed;
                }
                (*gd).hscrolled -= available;
                (*gd).hsize -= available;
            } else {
                available = 0;
            }
            needed -= available;

            // Then fill the rest in with blanks
            for i in ((*gd).hsize + sy - needed)..((*gd).hsize + sy) {
                grid_empty_line(gd, i, 8);
            }
        }

        // Set the new size, and reset the scroll region
        (*gd).sy = sy;
        (*s).rupper = 0;
        (*s).rlower = screen_size_y(s) - 1;
    }
}

/// Set selection.
pub unsafe fn screen_set_selection(
    s: *mut screen,
    sx: u32,
    sy: u32,
    ex: u32,
    ey: u32,
    rectangle: u32,
    modekeys: modekey,
    gc: *mut grid_cell,
) {
    unsafe {
        if (*s).sel.is_null() {
            (*s).sel = xcalloc1::<screen_sel>() as *mut screen_sel;
        }

        memcpy__(&raw mut (*(*s).sel).cell, gc);
        (*(*s).sel).hidden = 0;
        (*(*s).sel).rectangle = rectangle as i32;
        (*(*s).sel).modekeys = modekeys;

        (*(*s).sel).sx = sx;
        (*(*s).sel).sy = sy;
        (*(*s).sel).ex = ex;
        (*(*s).sel).ey = ey;
    }
}

/// Clear selection.
pub unsafe fn screen_clear_selection(s: *mut screen) {
    unsafe {
        free_((*s).sel);
        (*s).sel = null_mut();
    }
}

/// Hide selection.
pub unsafe fn screen_hide_selection(s: *mut screen) {
    unsafe {
        if !(*s).sel.is_null() {
            (*(*s).sel).hidden = 1;
        }
    }
}

/// Check if cell in selection.
pub unsafe fn screen_check_selection(s: *mut screen, px: u32, py: u32) -> c_int {
    unsafe {
        let sel = (*s).sel;
        let xx: u32;

        if sel.is_null() || (*sel).hidden != 0 {
            return 0;
        }

        if (*sel).rectangle != 0 {
            if (*sel).sy < (*sel).ey {
                // start line < end line -- downward selection.
                if py < (*sel).sy || py > (*sel).ey {
                    return 0;
                }
            } else if (*sel).sy > (*sel).ey {
                // start line > end line -- upward selection.
                if py > (*sel).sy || py < (*sel).ey {
                    return 0;
                }
            } else {
                // starting line == ending line.
                if py != (*sel).sy {
                    return 0;
                }
            }

            /*
             * Need to include the selection start row, but not the cursor
             * row, which means the selection changes depending on which
             * one is on the left.
             */
            if (*sel).ex < (*sel).sx {
                // Cursor (ex) is on the left.
                if px < (*sel).ex {
                    return 0;
                }

                if px > (*sel).sx {
                    return 0;
                }
            } else {
                // Selection start (sx) is on the left.
                if px < (*sel).sx {
                    return 0;
                }

                if px > (*sel).ex {
                    return 0;
                }
            }
        } else {
            /*
             * Like emacs, keep the top-left-most character, and drop the
             * bottom-right-most, regardless of copy direction.
             */
            if (*sel).sy < (*sel).ey {
                // starting line < ending line -- downward selection.
                if py < (*sel).sy || py > (*sel).ey {
                    return 0;
                }

                if py == (*sel).sy && px < (*sel).sx {
                    return 0;
                }

                if (*sel).modekeys == modekey::MODEKEY_EMACS {
                    xx = if (*sel).ex == 0 { 0 } else { (*sel).ex - 1 };
                } else {
                    xx = (*sel).ex;
                }
                if py == (*sel).ey && px > xx {
                    return 0;
                }
            } else if (*sel).sy > (*sel).ey {
                // starting line > ending line -- upward selection.
                if py > (*sel).sy || py < (*sel).ey {
                    return 0;
                }

                if py == (*sel).ey && px < (*sel).ex {
                    return 0;
                }

                if (*sel).modekeys == modekey::MODEKEY_EMACS {
                    xx = (*sel).sx - 1;
                } else {
                    xx = (*sel).sx;
                }
                if py == (*sel).sy && ((*sel).sx == 0 || px > xx) {
                    return 0;
                }
            } else {
                // starting line == ending line.
                if py != (*sel).sy {
                    return 0;
                }

                if (*sel).ex < (*sel).sx {
                    // cursor (ex) is on the left
                    if (*sel).modekeys == modekey::MODEKEY_EMACS {
                        xx = (*sel).sx - 1;
                    } else {
                        xx = (*sel).sx;
                    }
                    if px > xx || px < (*sel).ex {
                        return 0;
                    }
                } else {
                    // selection start (sx) is on the left
                    if (*sel).modekeys == modekey::MODEKEY_EMACS {
                        xx = if (*sel).ex == 0 { 0 } else { (*sel).ex - 1 };
                    } else {
                        xx = (*sel).ex;
                    }
                    if px < (*sel).sx || px > xx {
                        return 0;
                    }
                }
            }
        }

        1
    }
}

/// Get selected grid cell.
pub unsafe fn screen_select_cell(s: *mut screen, dst: *mut grid_cell, src: *const grid_cell) {
    unsafe {
        if (*s).sel.is_null() || (*(*s).sel).hidden != 0 {
            return;
        }

        memcpy__(dst, &raw const (*(*s).sel).cell);

        utf8_copy(&mut (*dst).data, &(*src).data);
        (*dst).attr &= !grid_attr::GRID_ATTR_CHARSET;
        (*dst).attr |= (*src).attr & grid_attr::GRID_ATTR_CHARSET;
        (*dst).flags = (*src).flags;
    }
}

/// Reflow wrapped lines.
unsafe fn screen_reflow(s: *mut screen, new_x: u32, cx: *mut u32, cy: *mut u32, cursor: i32) {
    unsafe {
        let mut wx: u32 = 0;
        let mut wy: u32 = 0;

        if cursor != 0 {
            grid_wrap_position((*s).grid, *cx, *cy, &mut wx, &mut wy);
            log_debug!(
                "{}: cursor {},{} is {},{}",
                "screen_reflow",
                *cx,
                *cy,
                wx,
                wy,
            );
        }

        grid_reflow((*s).grid, new_x);

        if cursor != 0 {
            grid_unwrap_position((*s).grid, cx, cy, wx, wy);
            log_debug!("{}: new cursor is {},{}", "screen_reflow", *cx, *cy);
        } else {
            *cx = 0;
            *cy = (*(*s).grid).hsize;
        }
    }
}

/// Enter alternative screen mode. A copy of the visible screen is saved and the
/// history is not updated.
pub unsafe fn screen_alternate_on(s: *mut screen, gc: *mut grid_cell, cursor: i32) {
    unsafe {
        if !(*s).saved_grid.is_null() {
            return;
        }
        let sx = screen_size_x(s);
        let sy = screen_size_y(s);

        (*s).saved_grid = grid_create(sx, sy, 0);
        grid_duplicate_lines((*s).saved_grid, 0, (*s).grid, screen_hsize(s), sy);
        if cursor != 0 {
            (*s).saved_cx = (*s).cx;
            (*s).saved_cy = (*s).cy;
        }
        memcpy__(&raw mut (*s).saved_cell, gc);

        grid_view_clear((*s).grid, 0, 0, sx, sy, 8);

        (*s).saved_flags = (*(*s).grid).flags;
        (*(*s).grid).flags &= !GRID_HISTORY;
    }
}

/// Exit alternate screen mode and restore the copied grid.
pub unsafe fn screen_alternate_off(s: *mut screen, gc: *mut grid_cell, cursor: i32) {
    unsafe {
        let sx = screen_size_x(s);
        let sy = screen_size_y(s);

        // If the current size is different, temporarily resize to the old size
        // before copying back.
        if !(*s).saved_grid.is_null() {
            screen_resize(s, (*(*s).saved_grid).sx, (*(*s).saved_grid).sy, 0);
        }

        // Restore the cursor position and cell. This happens even if not
        // currently in the alternate screen.
        if cursor != 0 && (*s).saved_cx != u32::MAX && (*s).saved_cy != u32::MAX {
            (*s).cx = (*s).saved_cx;
            (*s).cy = (*s).saved_cy;
            if !gc.is_null() {
                memcpy__(gc, &raw const (*s).saved_cell);
            }
        }

        // If not in the alternate screen, do nothing more.
        if (*s).saved_grid.is_null() {
            if (*s).cx > screen_size_x(s) - 1 {
                (*s).cx = screen_size_x(s) - 1;
            }
            if (*s).cy > screen_size_y(s) - 1 {
                (*s).cy = screen_size_y(s) - 1;
            }
            return;
        }

        // Restore the saved grid.
        grid_duplicate_lines(
            (*s).grid,
            screen_hsize(s),
            (*s).saved_grid,
            0,
            (*(*s).saved_grid).sy,
        );

        // Turn history back on (so resize can use it) and then resize back to
        // the current size.
        if (*s).saved_flags & GRID_HISTORY != 0 {
            (*(*s).grid).flags |= GRID_HISTORY;
        }
        screen_resize(s, sx, sy, 1);

        grid_destroy((*s).saved_grid);
        (*s).saved_grid = null_mut();

        if (*s).cx > screen_size_x(s) - 1 {
            (*s).cx = screen_size_x(s) - 1;
        }
        if (*s).cy > screen_size_y(s) - 1 {
            (*s).cy = screen_size_y(s) - 1;
        }
    }
}

/// Get mode as a string.
pub unsafe fn screen_mode_to_string(mode: mode_flag) -> *const u8 {
    const TMP_LEN: usize = 1024;
    static mut TMP: [MaybeUninit<u8>; 1024] = [MaybeUninit::uninit(); 1024];

    unsafe {
        if mode == mode_flag::empty() {
            return c!("NONE");
        }
        if mode.is_all() {
            return c!("ALL");
        }

        *TMP[0].as_mut_ptr().cast() = 0i8;

        if mode.intersects(mode_flag::MODE_CURSOR) {
            strlcat(addr_of_mut!(TMP).cast(), c!("CURSOR,"), TMP_LEN);
        }
        if mode.intersects(mode_flag::MODE_INSERT) {
            strlcat(addr_of_mut!(TMP).cast(), c!("INSERT,"), TMP_LEN);
        }
        if mode.intersects(mode_flag::MODE_KCURSOR) {
            strlcat(addr_of_mut!(TMP).cast(), c!("KCURSOR,"), TMP_LEN);
        }
        if mode.intersects(mode_flag::MODE_KKEYPAD) {
            strlcat(addr_of_mut!(TMP).cast(), c!("KKEYPAD,"), TMP_LEN);
        }
        if mode.intersects(mode_flag::MODE_WRAP) {
            strlcat(addr_of_mut!(TMP).cast(), c!("WRAP,"), TMP_LEN);
        }
        if mode.intersects(mode_flag::MODE_MOUSE_STANDARD) {
            strlcat(addr_of_mut!(TMP).cast(), c!("MOUSE_STANDARD,"), TMP_LEN);
        }
        if mode.intersects(mode_flag::MODE_MOUSE_BUTTON) {
            strlcat(addr_of_mut!(TMP).cast(), c!("MOUSE_BUTTON,"), TMP_LEN);
        }
        if mode.intersects(mode_flag::MODE_CURSOR_BLINKING) {
            strlcat(addr_of_mut!(TMP).cast(), c!("CURSOR_BLINKING,"), TMP_LEN);
        }
        if mode.intersects(mode_flag::MODE_CURSOR_VERY_VISIBLE) {
            strlcat(
                addr_of_mut!(TMP).cast(),
                c!("CURSOR_VERY_VISIBLE,"),
                TMP_LEN,
            );
        }
        if mode.intersects(mode_flag::MODE_MOUSE_UTF8) {
            strlcat(addr_of_mut!(TMP).cast(), c!("MOUSE_UTF8,"), TMP_LEN);
        }
        if mode.intersects(mode_flag::MODE_MOUSE_SGR) {
            strlcat(addr_of_mut!(TMP).cast(), c!("MOUSE_SGR,"), TMP_LEN);
        }
        if mode.intersects(mode_flag::MODE_BRACKETPASTE) {
            strlcat(addr_of_mut!(TMP).cast(), c!("BRACKETPASTE,"), TMP_LEN);
        }
        if mode.intersects(mode_flag::MODE_FOCUSON) {
            strlcat(addr_of_mut!(TMP).cast(), c!("FOCUSON,"), TMP_LEN);
        }
        if mode.intersects(mode_flag::MODE_MOUSE_ALL) {
            strlcat(addr_of_mut!(TMP).cast(), c!("MOUSE_ALL,"), TMP_LEN);
        }
        if mode.intersects(mode_flag::MODE_ORIGIN) {
            strlcat(addr_of_mut!(TMP).cast(), c!("ORIGIN,"), TMP_LEN);
        }
        if mode.intersects(mode_flag::MODE_CRLF) {
            strlcat(addr_of_mut!(TMP).cast(), c!("CRLF,"), TMP_LEN);
        }
        if mode.intersects(mode_flag::MODE_KEYS_EXTENDED) {
            strlcat(addr_of_mut!(TMP).cast(), c!("KEYS_EXTENDED,"), TMP_LEN);
        }
        if mode.intersects(mode_flag::MODE_KEYS_EXTENDED_2) {
            strlcat(addr_of_mut!(TMP).cast(), c!("KEYS_EXTENDED_2,"), TMP_LEN);
        }

        let len = strlen(addr_of!(TMP).cast());
        if len > 0 {
            *TMP[len - 1].as_mut_ptr().cast() = 0i8;
        }
        &raw mut TMP as *mut u8
    }
}

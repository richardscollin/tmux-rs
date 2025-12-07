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
use crate::options_::*;

struct layout_sets_entry {
    name: SyncCharPtr,
    arrange: Option<unsafe fn(*mut window)>,
}
impl layout_sets_entry {
    const fn new(name: &'static CStr, arrange: unsafe fn(*mut window)) -> Self {
        Self {
            name: SyncCharPtr::new(name),
            arrange: Some(arrange),
        }
    }
}

const LAYOUT_SETS_LEN: usize = 7;
static LAYOUT_SETS: [layout_sets_entry; LAYOUT_SETS_LEN] = [
    layout_sets_entry::new(c"even-horizontal", layout_set_even_h),
    layout_sets_entry::new(c"even-vertical", layout_set_even_v),
    layout_sets_entry::new(c"main-horizontal", layout_set_main_h),
    layout_sets_entry::new(c"main-horizontal-mirrored", layout_set_main_h_mirrored),
    layout_sets_entry::new(c"main-vertical", layout_set_main_v),
    layout_sets_entry::new(c"main-vertical-mirrored", layout_set_main_v_mirrored),
    layout_sets_entry::new(c"tiled", layout_set_tiled),
];

pub unsafe fn layout_set_lookup(name: *const u8) -> i32 {
    unsafe {
        let mut matched: i32 = -1;

        for (i, ls) in LAYOUT_SETS.iter().enumerate() {
            if libc::strcmp(ls.name.as_ptr(), name) == 0 {
                return i as i32;
            }
        }

        for (i, ls) in LAYOUT_SETS.iter().enumerate() {
            if libc::strncmp(ls.name.as_ptr(), name, strlen(name)) == 0 {
                if matched != -1 {
                    // ambiguous
                    return -1;
                }
                matched = i as i32;
            }
        }

        matched
    }
}

pub unsafe fn layout_set_select(w: *mut window, mut layout: u32) -> u32 {
    unsafe {
        if layout > LAYOUT_SETS_LEN as u32 - 1 {
            layout = LAYOUT_SETS_LEN as u32 - 1;
        }

        if let Some(arrange) = LAYOUT_SETS[layout as usize].arrange {
            arrange(w);
        }

        (*w).lastlayout = layout as i32;
        layout
    }
}

pub unsafe fn layout_set_next(w: *mut window) -> u32 {
    unsafe {
        let mut layout: u32;

        if (*w).lastlayout == -1 {
            layout = 0;
        } else {
            layout = ((*w).lastlayout + 1) as u32;
            if layout > LAYOUT_SETS_LEN as u32 - 1 {
                layout = 0;
            }
        }

        if let Some(arrange) = LAYOUT_SETS[layout as usize].arrange {
            arrange(w);
        }
        (*w).lastlayout = layout as i32;
        layout
    }
}

pub unsafe fn layout_set_previous(w: *mut window) -> u32 {
    unsafe {
        let mut layout: u32;

        if (*w).lastlayout == -1 {
            layout = (LAYOUT_SETS_LEN - 1) as u32;
        } else {
            layout = (*w).lastlayout as u32;
            if layout == 0 {
                layout = (LAYOUT_SETS_LEN - 1) as u32;
            } else {
                layout -= 1;
            }
        }

        if let Some(arrange) = LAYOUT_SETS[layout as usize].arrange {
            arrange(w);
        }
        (*w).lastlayout = layout as i32;
        layout
    }
}

pub unsafe fn layout_set_even(w: *mut window, type_: layout_type) {
    let __func__ = c!("layout_set_even");
    unsafe {
        let mut sx: u32;
        let mut sy: u32;

        layout_print_cell((*w).layout_root, __func__, 1);

        // Get number of panes.
        let n = window_count_panes(w);
        if n <= 1 {
            return;
        }

        // Free the old root and construct a new.
        layout_free(w);
        let lc = layout_create_cell(null_mut());
        (*w).layout_root = lc;
        if type_ == layout_type::LAYOUT_LEFTRIGHT {
            sx = (n * (PANE_MINIMUM + 1)) - 1;
            if sx < (*w).sx {
                sx = (*w).sx;
            }
            sy = (*w).sy;
        } else {
            sy = (n * (PANE_MINIMUM + 1)) - 1;
            if sy < (*w).sy {
                sy = (*w).sy;
            }
            sx = (*w).sx;
        }
        layout_set_size(lc, sx, sy, 0, 0);
        layout_make_node(lc, type_);

        // Build new leaf cells.
        for wp in tailq_foreach::<_, discr_entry>(&raw mut (*w).panes).map(NonNull::as_ptr) {
            let lcnew = layout_create_cell(lc);
            layout_make_leaf(lcnew, wp);
            (*lcnew).sx = (*w).sx;
            (*lcnew).sy = (*w).sy;
            tailq_insert_tail(&raw mut (*lc).cells, lcnew);
        }

        // Spread out cells.
        layout_spread_cell(w, lc);

        // Fix cell offsets.
        layout_fix_offsets(w);
        layout_fix_panes(w, null_mut());

        layout_print_cell((*w).layout_root, __func__, 1);

        window_resize(w, (*lc).sx, (*lc).sy, -1, -1);
        notify_window(c"window-layout-changed", w);
        server_redraw_window(w);
    }
}

unsafe fn layout_set_even_h(w: *mut window) {
    unsafe {
        layout_set_even(w, layout_type::LAYOUT_LEFTRIGHT);
    }
}

unsafe fn layout_set_even_v(w: *mut window) {
    unsafe {
        layout_set_even(w, layout_type::LAYOUT_TOPBOTTOM);
    }
}

pub unsafe fn layout_set_main_h(w: *mut window) {
    let __func__ = c!("layout_set_main_h");
    unsafe {
        // struct window_pane *wp;
        // struct layout_cell *lc, *lcmain, *lcother, *lcchild;
        // u_int n, mainh, otherh, sx, sy;
        // char *cause;
        // const char *s;
        let mut cause = null_mut();

        layout_print_cell((*w).layout_root, __func__, 1);

        // Get number of panes.
        let mut n = window_count_panes(w);
        if n <= 1 {
            return;
        }
        n -= 1; /* take off main pane */

        // Find available height - take off one line for the border.
        let sy = (*w).sy - 1;

        // Get the main pane height.
        let mut s = options_get_string_((*w).options, "main-pane-height");
        let mut mainh = args_string_percentage(s, 0, sy as i64, sy as i64, &raw mut cause) as u32;
        if !cause.is_null() {
            mainh = 24;
            free_(cause);
        }

        let mut otherh: u32;
        // Work out the other pane height.
        if mainh + PANE_MINIMUM >= sy {
            if sy <= PANE_MINIMUM + PANE_MINIMUM {
                mainh = PANE_MINIMUM;
            } else {
                mainh = sy - PANE_MINIMUM;
            }
            otherh = PANE_MINIMUM;
        } else {
            s = options_get_string_((*w).options, "other-pane-height");
            otherh = args_string_percentage(s, 0, sy as i64, sy as i64, &raw mut cause) as u32;
            if !cause.is_null() || otherh == 0 {
                otherh = sy - mainh;
                free_(cause);
            } else if otherh > sy || sy - otherh < mainh {
                otherh = sy - mainh;
            } else {
                mainh = sy - otherh;
            }
        }

        // Work out what width is needed.
        let mut sx = (n * (PANE_MINIMUM + 1)) - 1;
        if sx < (*w).sx {
            sx = (*w).sx;
        }

        // Free old tree and create a new root.
        layout_free(w);
        let lc = layout_create_cell(null_mut());
        (*w).layout_root = lc;
        layout_set_size(lc, sx, mainh + otherh + 1, 0, 0);
        layout_make_node(lc, layout_type::LAYOUT_TOPBOTTOM);

        // Create the main pane.
        let lcmain = layout_create_cell(lc);
        layout_set_size(lcmain, sx, mainh, 0, 0);
        layout_make_leaf(lcmain, tailq_first(&raw mut (*w).panes));
        tailq_insert_tail(&raw mut (*lc).cells, lcmain);

        // Create the other pane.
        let lcother = layout_create_cell(lc);
        layout_set_size(lcother, sx, otherh, 0, 0);
        if n == 1 {
            let wp = tailq_next::<_, _, discr_entry>(tailq_first(&raw mut (*w).panes));
            layout_make_leaf(lcother, wp);
            tailq_insert_tail(&raw mut (*lc).cells, lcother);
        } else {
            layout_make_node(lcother, layout_type::LAYOUT_LEFTRIGHT);
            tailq_insert_tail(&raw mut (*lc).cells, lcother);

            // Add the remaining panes as children.
            for wp in tailq_foreach::<_, discr_entry>(&raw mut (*w).panes).map(NonNull::as_ptr) {
                if wp == tailq_first(&raw mut (*w).panes) {
                    continue;
                }
                let lcchild = layout_create_cell(lcother);
                layout_set_size(lcchild, PANE_MINIMUM, otherh, 0, 0);
                layout_make_leaf(lcchild, wp);
                tailq_insert_tail(&raw mut (*lcother).cells, lcchild);
            }
            layout_spread_cell(w, lcother);
        }

        // Fix cell offsets.
        layout_fix_offsets(w);
        layout_fix_panes(w, null_mut());

        layout_print_cell((*w).layout_root, __func__, 1);

        window_resize(w, (*lc).sx, (*lc).sy, -1, -1);
        notify_window(c"window-layout-changed", w);
        server_redraw_window(w);
    }
}

pub unsafe fn layout_set_main_h_mirrored(w: *mut window) {
    let __func__ = c!("layout_set_main_h_mirrored");
    unsafe {
        let mut otherh: u32;
        let mut cause: *mut u8 = null_mut();

        layout_print_cell((*w).layout_root, __func__, 1);

        // Get number of panes.
        let mut n = window_count_panes(w);
        if n <= 1 {
            return;
        }
        n -= 1; // take off main pane

        // Find available height - take off one line for the border.
        let sy = (*w).sy - 1;

        // Get the main pane height.
        let s = options_get_string_((*w).options, "main-pane-height");
        let mut mainh = args_string_percentage(s, 0, sy as i64, sy as i64, &raw mut cause) as u32;
        if !cause.is_null() {
            mainh = 24;
            free_(cause);
        }

        // Work out the other pane height.
        if mainh + PANE_MINIMUM >= sy {
            if sy <= PANE_MINIMUM + PANE_MINIMUM {
                mainh = PANE_MINIMUM;
            } else {
                mainh = sy - PANE_MINIMUM;
            }
            otherh = PANE_MINIMUM;
        } else {
            let s = options_get_string_((*w).options, "other-pane-height");
            otherh = args_string_percentage(s, 0, sy as i64, sy as i64, &raw mut cause) as u32;
            if !cause.is_null() || otherh == 0 {
                otherh = sy - mainh;
                free_(cause);
            } else if otherh > sy || sy - otherh < mainh {
                otherh = sy - mainh;
            } else {
                mainh = sy - otherh;
            }
        }

        // Work out what width is needed.
        let mut sx = (n * (PANE_MINIMUM + 1)) - 1;
        if sx < (*w).sx {
            sx = (*w).sx;
        }

        // Free old tree and create a new root.
        layout_free(w);
        let lc = layout_create_cell(null_mut());
        (*w).layout_root = lc;
        layout_set_size(lc, sx, mainh + otherh + 1, 0, 0);
        layout_make_node(lc, layout_type::LAYOUT_TOPBOTTOM);

        // Create the other pane.
        let lcother = layout_create_cell(lc);
        layout_set_size(lcother, sx, otherh, 0, 0);
        if n == 1 {
            let wp = tailq_next::<_, _, discr_entry>(tailq_first(&raw mut (*w).panes));
            layout_make_leaf(lcother, wp);
            tailq_insert_tail(&raw mut (*lc).cells, lcother);
        } else {
            layout_make_node(lcother, layout_type::LAYOUT_LEFTRIGHT);
            tailq_insert_tail(&raw mut (*lc).cells, lcother);

            // Add the remaining panes as children.
            for wp in tailq_foreach::<_, discr_entry>(&raw mut (*w).panes).map(NonNull::as_ptr) {
                if wp == tailq_first(&raw mut (*w).panes) {
                    continue;
                }
                let lcchild = layout_create_cell(lcother);
                layout_set_size(lcchild, PANE_MINIMUM, otherh, 0, 0);
                layout_make_leaf(lcchild, wp);
                tailq_insert_tail(&raw mut (*lcother).cells, lcchild);
            }
            layout_spread_cell(w, lcother);
        }

        // Create the main pane.
        let lcmain = layout_create_cell(lc);
        layout_set_size(lcmain, sx, mainh, 0, 0);
        layout_make_leaf(lcmain, tailq_first(&raw mut (*w).panes));
        tailq_insert_tail(&raw mut (*lc).cells, lcmain);

        // Fix cell offsets.
        layout_fix_offsets(w);
        layout_fix_panes(w, null_mut());

        layout_print_cell((*w).layout_root, __func__, 1);

        window_resize(w, (*lc).sx, (*lc).sy, -1, -1);
        notify_window(c"window-layout-changed", w);
        server_redraw_window(w);
    }
}

pub unsafe fn layout_set_main_v(w: *mut window) {
    let __func__ = c!("layout_set_main_v");
    let mut cause = null_mut();

    unsafe {
        layout_print_cell((*w).layout_root, __func__, 1);

        // Get number of panes.
        let mut n = window_count_panes(w);
        if n <= 1 {
            return;
        }
        n -= 1; // take off main pane

        // Find available width - take off one line for the border.
        let sx = (*w).sx - 1;

        // Get the main pane width.
        let s = options_get_string_((*w).options, "main-pane-width");
        let mut mainw: u32 =
            args_string_percentage(s, 0, sx as i64, sx as i64, &raw mut cause) as u32;
        if cause.is_null() {
            mainw = 80;
            free_(cause);
        }

        // Work out the other pane width.
        let mut otherw;
        if mainw + PANE_MINIMUM >= sx {
            if sx <= PANE_MINIMUM + PANE_MINIMUM {
                mainw = PANE_MINIMUM;
            } else {
                mainw = sx - PANE_MINIMUM;
            }
            otherw = PANE_MINIMUM;
        } else {
            let s = options_get_string_((*w).options, "other-pane-width");
            otherw = args_string_percentage(s, 0, sx as i64, sx as i64, &raw mut cause) as u32;
            if !cause.is_null() || otherw == 0 {
                otherw = sx - mainw;
                free_(cause);
            } else if otherw > sx || sx - otherw < mainw {
                otherw = sx - mainw;
            } else {
                mainw = sx - otherw;
            }
        }

        // Work out what height is needed.
        let mut sy = (n * (PANE_MINIMUM + 1)) - 1;
        if sy < (*w).sy {
            sy = (*w).sy;
        }

        // Free old tree and create a new root.
        layout_free(w);
        let lc = layout_create_cell(null_mut());
        (*w).layout_root = lc;
        layout_set_size(lc, mainw + otherw + 1, sy, 0, 0);
        layout_make_node(lc, layout_type::LAYOUT_LEFTRIGHT);

        // Create the main pane.
        let lcmain = layout_create_cell(lc);
        layout_set_size(lcmain, mainw, sy, 0, 0);
        layout_make_leaf(lcmain, tailq_first(&raw mut (*w).panes));
        tailq_insert_tail(&raw mut (*lc).cells, lcmain);

        // Create the other pane.
        let lcother = layout_create_cell(lc);
        layout_set_size(lcother, otherw, sy, 0, 0);
        if n == 1 {
            let wp = tailq_next::<_, _, discr_entry>(tailq_first(&raw mut (*w).panes));
            layout_make_leaf(lcother, wp);
            tailq_insert_tail(&raw mut (*lc).cells, lcother);
        } else {
            layout_make_node(lcother, layout_type::LAYOUT_TOPBOTTOM);
            tailq_insert_tail(&raw mut (*lc).cells, lcother);

            // Add the remaining panes as children.
            for wp in tailq_foreach::<_, discr_entry>(&raw mut (*w).panes).map(NonNull::as_ptr) {
                if wp == tailq_first(&raw mut (*w).panes) {
                    continue;
                }
                let lcchild = layout_create_cell(lcother);
                layout_set_size(lcchild, otherw, PANE_MINIMUM, 0, 0);
                layout_make_leaf(lcchild, wp);
                tailq_insert_tail(&raw mut (*lcother).cells, lcchild);
            }
            layout_spread_cell(w, lcother);
        }

        // Fix cell offsets.
        layout_fix_offsets(w);
        layout_fix_panes(w, null_mut());

        layout_print_cell((*w).layout_root, __func__, 1);

        window_resize(w, (*lc).sx, (*lc).sy, -1, -1);
        notify_window(c"window-layout-changed", w);
        server_redraw_window(w);
    }
}

pub unsafe fn layout_set_main_v_mirrored(w: *mut window) {
    let __func__ = c!("layout_set_main_v_mirrored");
    unsafe {
        let mut cause: *mut u8 = null_mut();

        layout_print_cell((*w).layout_root, __func__, 1);

        // Get number of panes.
        let mut n = window_count_panes(w);
        if n <= 1 {
            return;
        }
        n -= 1; // take off main pane

        // Find available width - take off one line for the border.
        let sx = (*w).sx - 1;

        // Get the main pane width.
        let s = options_get_string_((*w).options, "main-pane-width");
        let mut mainw = args_string_percentage(s, 0, sx as i64, sx as i64, &raw mut cause) as u32;
        if !cause.is_null() {
            mainw = 80;
            free_(cause);
        }

        // Work out the other pane width.
        let mut otherw: u32;
        if mainw + PANE_MINIMUM >= sx {
            if sx <= PANE_MINIMUM + PANE_MINIMUM {
                mainw = PANE_MINIMUM;
            } else {
                mainw = sx - PANE_MINIMUM;
            }
            otherw = PANE_MINIMUM;
        } else {
            let s = options_get_string_((*w).options, "other-pane-width");
            otherw = args_string_percentage(s, 0, sx as i64, sx as i64, &raw mut cause) as u32;
            if !cause.is_null() || otherw == 0 {
                otherw = sx - mainw;
                free_(cause);
            } else if otherw > sx || sx - otherw < mainw {
                otherw = sx - mainw;
            } else {
                mainw = sx - otherw;
            }
        }

        // Work out what height is needed.
        let mut sy = (n * (PANE_MINIMUM + 1)) - 1;
        if sy < (*w).sy {
            sy = (*w).sy;
        }

        // Free old tree and create a new root.
        layout_free(w);
        let lc = layout_create_cell(null_mut());
        (*w).layout_root = lc;
        layout_set_size(lc, mainw + otherw + 1, sy, 0, 0);
        layout_make_node(lc, layout_type::LAYOUT_LEFTRIGHT);

        // Create the other pane.
        let lcother = layout_create_cell(lc);
        layout_set_size(lcother, otherw, sy, 0, 0);
        if n == 1 {
            let wp = tailq_next::<_, _, discr_entry>(tailq_first(&raw mut (*w).panes));
            layout_make_leaf(lcother, wp);
            tailq_insert_tail(&raw mut (*lc).cells, lcother);
        } else {
            layout_make_node(lcother, layout_type::LAYOUT_TOPBOTTOM);
            tailq_insert_tail(&raw mut (*lc).cells, lcother);

            // Add the remaining panes as children.
            for wp in tailq_foreach::<_, discr_entry>(&raw mut (*w).panes).map(NonNull::as_ptr) {
                if wp == tailq_first(&raw mut (*w).panes) {
                    continue;
                }
                let lcchild = layout_create_cell(lcother);
                layout_set_size(lcchild, otherw, PANE_MINIMUM, 0, 0);
                layout_make_leaf(lcchild, wp);
                tailq_insert_tail(&raw mut (*lcother).cells, lcchild);
            }
            layout_spread_cell(w, lcother);
        }

        // Create the main pane.
        let lcmain = layout_create_cell(lc);
        layout_set_size(lcmain, mainw, sy, 0, 0);
        layout_make_leaf(lcmain, tailq_first(&raw mut (*w).panes));
        tailq_insert_tail(&raw mut (*lc).cells, lcmain);

        // Fix cell offsets.
        layout_fix_offsets(w);
        layout_fix_panes(w, null_mut());

        layout_print_cell((*w).layout_root, __func__, 1);

        window_resize(w, (*lc).sx, (*lc).sy, -1, -1);
        notify_window(c"window-layout-changed", w);
        server_redraw_window(w);
    }
}

pub unsafe fn layout_set_tiled(w: *mut window) {
    let __func__ = c!("layout_set_tiled");

    unsafe {
        layout_print_cell((*w).layout_root, __func__, 1);

        // Get number of panes.
        let n = window_count_panes(w);
        if n <= 1 {
            return;
        }

        // How many rows and columns are wanted?
        let mut rows = 1;
        let mut columns = 1;
        while rows * columns < n {
            rows += 1;
            if rows * columns < n {
                columns += 1;
            }
        }

        // What width and height should they be?
        let mut width = ((*w).sx - (columns - 1)) / columns;
        if width < PANE_MINIMUM {
            width = PANE_MINIMUM;
        }
        let mut height = ((*w).sy - (rows - 1)) / rows;
        if height < PANE_MINIMUM {
            height = PANE_MINIMUM;
        }

        // Free old tree and create a new root.
        layout_free(w);
        let lc = layout_create_cell(null_mut());
        (*w).layout_root = lc;
        let mut sx = ((width + 1) * columns) - 1;
        if sx < (*w).sx {
            sx = (*w).sx;
        }
        let mut sy = ((height + 1) * rows) - 1;
        if sy < (*w).sy {
            sy = (*w).sy;
        }
        layout_set_size(lc, sx, sy, 0, 0);
        layout_make_node(lc, layout_type::LAYOUT_TOPBOTTOM);

        // Create a grid of the cells.
        let mut wp = tailq_first(&raw mut (*w).panes);
        for j in 0..rows {
            // If this is the last cell, all done.
            if wp.is_null() {
                break;
            }

            // Create the new row.
            let lcrow = layout_create_cell(lc);
            layout_set_size(lcrow, (*w).sx, height, 0, 0);
            tailq_insert_tail(&raw mut (*lc).cells, lcrow);

            // If only one column, just use the row directly.
            if n - (j * columns) == 1 || columns == 1 {
                layout_make_leaf(lcrow, wp);
                wp = tailq_next::<_, _, discr_entry>(wp);
                continue;
            }

            // Add in the columns.
            layout_make_node(lcrow, layout_type::LAYOUT_LEFTRIGHT);
            let mut i = 0;
            for i_ in 0..columns {
                i = i_;
                // Create and add a pane cell.
                let lcchild = layout_create_cell(lcrow);
                layout_set_size(lcchild, width, height, 0, 0);
                layout_make_leaf(lcchild, wp);
                tailq_insert_tail(&raw mut (*lcrow).cells, lcchild);

                // Move to the next cell.
                wp = tailq_next::<_, _, discr_entry>(wp);
                if wp.is_null() {
                    break;
                }
                i += 1;
            }

            // Adjust the row and columns to fit the full width if necessary.
            if i == columns {
                i -= 1;
            }
            let used = ((i + 1) * (width + 1)) - 1;
            if (*w).sx <= used {
                continue;
            }
            let lcchild = tailq_last(&raw mut (*lcrow).cells);
            layout_resize_adjust(
                w,
                lcchild,
                layout_type::LAYOUT_LEFTRIGHT,
                ((*w).sx - used) as i32,
            );
        }

        // Adjust the last row height to fit if necessary.
        let used = (rows * height) + rows - 1;
        if (*w).sy > used {
            let lcrow = tailq_last(&raw mut (*lc).cells);
            layout_resize_adjust(
                w,
                lcrow,
                layout_type::LAYOUT_TOPBOTTOM,
                ((*w).sy - used) as i32,
            );
        }

        // Fix cell offsets.
        layout_fix_offsets(w);
        layout_fix_panes(w, null_mut());

        layout_print_cell((*w).layout_root, __func__, 1);

        window_resize(w, (*lc).sx, (*lc).sy, -1, -1);
        notify_window(c"window-layout-changed", w);
        server_redraw_window(w);
    }
}

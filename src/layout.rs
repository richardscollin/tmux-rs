// Copyright (c) 2009 Nicholas Marriott <nicholas.marriott@gmail.com>
// Copyright (c) 2016 Stephen Kent <smkent@smkent.net>
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

pub unsafe fn layout_create_cell(lcparent: *mut layout_cell) -> *mut layout_cell {
    unsafe {
        let lc = Box::leak(Box::new(layout_cell {
            type_: layout_type::LAYOUT_WINDOWPANE,
            parent: lcparent,
            sx: u32::MAX,
            sy: u32::MAX,
            xoff: u32::MAX,
            yoff: u32::MAX,
            wp: null_mut(),
            cells: tailq_head {
                tqh_first: null_mut(),
                tqh_last: null_mut(),
            },
            entry: Default::default(),
        }));
        tailq_init(&raw mut lc.cells);

        lc
    }
}

pub unsafe fn layout_free_cell(lc: *mut layout_cell) {
    unsafe {
        match (*lc).type_ {
            layout_type::LAYOUT_LEFTRIGHT | layout_type::LAYOUT_TOPBOTTOM => {
                while !tailq_empty(&raw mut (*lc).cells) {
                    let lcchild = tailq_first(&raw mut (*lc).cells);
                    tailq_remove(&raw mut (*lc).cells, lcchild);
                    layout_free_cell(lcchild);
                }
            }
            layout_type::LAYOUT_WINDOWPANE => {
                if !(*lc).wp.is_null() {
                    (*(*lc).wp).layout_cell = null_mut();
                }
            }
        }

        free_(lc);
    }
}

pub unsafe fn layout_print_cell(lc: *mut layout_cell, hdr: *const u8, n: u32) {
    unsafe {
        let type_str = match (*lc).type_ {
            layout_type::LAYOUT_LEFTRIGHT => c"LEFTRIGHT",
            layout_type::LAYOUT_TOPBOTTOM => c"TOPBOTTOM",
            layout_type::LAYOUT_WINDOWPANE => c"WINDOWPANE",
        };

        log_debug!(
            "{}:{}{:p} type {} [parent {:p}] wp={:p} [{},{} {}x{}]",
            _s(hdr),
            if n == 0 { "" } else { " " },
            lc as *mut c_void,
            type_str.to_string_lossy(),
            (*lc).parent as *mut c_void,
            (*lc).wp as *mut c_void,
            (*lc).xoff,
            (*lc).yoff,
            (*lc).sx,
            (*lc).sy,
        );

        match (*lc).type_ {
            layout_type::LAYOUT_LEFTRIGHT | layout_type::LAYOUT_TOPBOTTOM => {
                for lcchild in tailq_foreach(&raw mut (*lc).cells) {
                    layout_print_cell(lcchild.as_ptr(), hdr, n + 1);
                }
            }
            layout_type::LAYOUT_WINDOWPANE => (),
        }
    }
}

pub unsafe fn layout_search_by_border(lc: *mut layout_cell, x: u32, y: u32) -> *mut layout_cell {
    unsafe {
        let mut last: *mut layout_cell = null_mut();

        for lcchild in tailq_foreach(&raw mut (*lc).cells) {
            let lcchild = lcchild.as_ptr();

            if x >= (*lcchild).xoff
                && x < (*lcchild).xoff + (*lcchild).sx
                && y >= (*lcchild).yoff
                && y < (*lcchild).yoff + (*lcchild).sy
            {
                // Inside the cell - recurse
                return layout_search_by_border(lcchild, x, y);
            }

            if last.is_null() {
                last = lcchild;
                continue;
            }

            match (*lc).type_ {
                layout_type::LAYOUT_LEFTRIGHT => {
                    if x < (*lcchild).xoff && x >= (*last).xoff + (*last).sx {
                        return last;
                    }
                }
                layout_type::LAYOUT_TOPBOTTOM => {
                    if y < (*lcchild).yoff && y >= (*last).yoff + (*last).sy {
                        return last;
                    }
                }
                layout_type::LAYOUT_WINDOWPANE => (),
            }

            last = lcchild;
        }

        null_mut()
    }
}

pub unsafe fn layout_set_size(lc: *mut layout_cell, sx: u32, sy: u32, xoff: u32, yoff: u32) {
    unsafe {
        (*lc).sx = sx;
        (*lc).sy = sy;
        (*lc).xoff = xoff;
        (*lc).yoff = yoff;
    }
}

pub unsafe fn layout_make_leaf(lc: *mut layout_cell, wp: *mut window_pane) {
    unsafe {
        (*lc).type_ = layout_type::LAYOUT_WINDOWPANE;
        tailq_init(&raw mut (*lc).cells);
        (*wp).layout_cell = lc;
        (*lc).wp = wp;
    }
}

pub unsafe fn layout_make_node(lc: *mut layout_cell, type_: layout_type) {
    unsafe {
        if type_ == layout_type::LAYOUT_WINDOWPANE {
            fatalx("bad layout type");
        }
        (*lc).type_ = type_;
        tailq_init(&raw mut (*lc).cells);

        if !(*lc).wp.is_null() {
            (*(*lc).wp).layout_cell = null_mut();
        }
        (*lc).wp = null_mut();
    }
}

/// Fix cell offsets for a child cell.
unsafe fn layout_fix_offsets1(lc: *mut layout_cell) {
    unsafe {
        if (*lc).type_ == layout_type::LAYOUT_LEFTRIGHT {
            let mut xoff = (*lc).xoff;
            for lcchild in tailq_foreach(&raw mut (*lc).cells) {
                let lcchild = lcchild.as_ptr();
                (*lcchild).xoff = xoff;
                (*lcchild).yoff = (*lc).yoff;
                if (*lcchild).type_ != layout_type::LAYOUT_WINDOWPANE {
                    layout_fix_offsets1(lcchild);
                }
                xoff += (*lcchild).sx + 1;
            }
        } else {
            let mut yoff = (*lc).yoff;
            for lcchild in tailq_foreach(&raw mut (*lc).cells) {
                let lcchild = lcchild.as_ptr();
                (*lcchild).xoff = (*lc).xoff;
                (*lcchild).yoff = yoff;
                if (*lcchild).type_ != layout_type::LAYOUT_WINDOWPANE {
                    layout_fix_offsets1(lcchild);
                }
                yoff += (*lcchild).sy + 1;
            }
        }
    }
}

/// Update cell offsets based on their sizes.
pub unsafe fn layout_fix_offsets(w: *mut window) {
    unsafe {
        let lc = (*w).layout_root;
        (*lc).xoff = 0;
        (*lc).yoff = 0;
        layout_fix_offsets1(lc);
    }
}

/// Is this a top cell?
unsafe fn layout_cell_is_top(w: *mut window, mut lc: *mut layout_cell) -> c_int {
    unsafe {
        while lc != (*w).layout_root {
            let next = (*lc).parent;
            if (*next).type_ == layout_type::LAYOUT_TOPBOTTOM
                && lc != tailq_first(&raw mut (*next).cells)
            {
                return 0;
            }
            lc = next;
        }
        1
    }
}

/// Is this a bottom cell?
unsafe fn layout_cell_is_bottom(w: *mut window, mut lc: *mut layout_cell) -> c_int {
    unsafe {
        while lc != (*w).layout_root {
            let next = (*lc).parent;
            if (*next).type_ == layout_type::LAYOUT_TOPBOTTOM
                && lc != tailq_last(&raw mut (*next).cells)
            {
                return 0;
            }
            lc = next;
        }
        1
    }
}

/// Returns 1 if we need to add an extra line for the pane status line. This is
/// the case for the most upper or lower panes only.
unsafe fn layout_add_border(w: *mut window, lc: *mut layout_cell, status: pane_status) -> bool {
    unsafe {
        if status == pane_status::PANE_STATUS_TOP {
            return layout_cell_is_top(w, lc) != 0;
        }
        if status == pane_status::PANE_STATUS_BOTTOM {
            return layout_cell_is_bottom(w, lc) != 0;
        }
        false
    }
}

/// Update pane offsets and sizes based on their cells.
pub unsafe fn layout_fix_panes(w: *mut window, skip: *mut window_pane) {
    unsafe {
        let status: pane_status =
            pane_status::try_from(options_get_number_((*w).options, "pane-border-status") as i32)
                .unwrap();

        for wp in tailq_foreach::<window_pane, discr_entry>(&raw mut (*w).panes) {
            let wp = wp.as_ptr();
            let lc = (*wp).layout_cell;
            if lc.is_null() || wp == skip {
                continue;
            }

            (*wp).xoff = (*lc).xoff;
            (*wp).yoff = (*lc).yoff;

            if layout_add_border(w, lc, status) {
                if status == pane_status::PANE_STATUS_TOP {
                    (*wp).yoff += 1;
                }
                window_pane_resize(wp, (*lc).sx, (*lc).sy - 1);
            } else {
                window_pane_resize(wp, (*lc).sx, (*lc).sy);
            }
        }
    }
}

/// Count the number of available cells in a layout.
pub unsafe fn layout_count_cells(lc: *mut layout_cell) -> u32 {
    unsafe {
        match (*lc).type_ {
            layout_type::LAYOUT_WINDOWPANE => 1,
            layout_type::LAYOUT_LEFTRIGHT | layout_type::LAYOUT_TOPBOTTOM => {
                let mut count = 0;
                for lcchild in tailq_foreach(&raw mut (*lc).cells) {
                    count += layout_count_cells(lcchild.as_ptr());
                }
                count
            }
        }
    }
}

/// Calculate how much size is available to be removed from a cell.
pub unsafe fn layout_resize_check(w: *mut window, lc: *mut layout_cell, type_: layout_type) -> u32 {
    unsafe {
        let mut available: u32;
        let mut minimum: u32;

        let status: pane_status =
            pane_status::try_from(options_get_number_((*w).options, "pane-border-status") as i32)
                .unwrap();

        if (*lc).type_ == layout_type::LAYOUT_WINDOWPANE {
            // Space available in this cell only.
            if type_ == layout_type::LAYOUT_LEFTRIGHT {
                available = (*lc).sx;
                minimum = PANE_MINIMUM;
            } else {
                available = (*lc).sy;
                if layout_add_border(w, lc, status) {
                    minimum = PANE_MINIMUM + 1;
                } else {
                    minimum = PANE_MINIMUM;
                }
            }
            if available > minimum {
                available -= minimum;
            } else {
                available = 0;
            }
        } else if (*lc).type_ == type_ {
            // Same type: total of available space in all child cells.
            available = 0;
            for lcchild in tailq_foreach(&raw mut (*lc).cells) {
                available += layout_resize_check(w, lcchild.as_ptr(), type_);
            }
        } else {
            // Different type: minimum of available space in child cells.
            minimum = u32::MAX;
            for lcchild in tailq_foreach(&raw mut (*lc).cells) {
                available = layout_resize_check(w, lcchild.as_ptr(), type_);
                if available < minimum {
                    minimum = available;
                }
            }
            available = minimum;
        }

        available
    }
}

/// Adjust cell size evenly, including altering its children. This function
/// expects the change to have already been bounded to the space available.
pub unsafe fn layout_resize_adjust(
    w: *mut window,
    lc: *mut layout_cell,
    type_: layout_type,
    mut change: i32,
) {
    unsafe {
        // Adjust the cell size
        if type_ == layout_type::LAYOUT_LEFTRIGHT {
            (*lc).sx = ((*lc).sx as i32 + change) as u32;
        } else {
            (*lc).sy = ((*lc).sy as i32 + change) as u32;
        }

        // If this is a leaf cell, that is all that is necessary
        if (*lc).type_ == layout_type::LAYOUT_WINDOWPANE {
            return;
        }

        // Child cell runs in a different direction
        if (*lc).type_ != type_ {
            for lcchild in tailq_foreach(&raw mut (*lc).cells) {
                layout_resize_adjust(w, lcchild.as_ptr(), type_, change);
            }
            return;
        }

        // Child cell runs in the same direction. Adjust each child equally
        // until no further change is possible
        while change != 0 {
            for lcchild in tailq_foreach(&raw mut (*lc).cells) {
                if change == 0 {
                    break;
                }
                if change > 0 {
                    layout_resize_adjust(w, lcchild.as_ptr(), type_, 1);
                    change -= 1;
                    continue;
                }
                if layout_resize_check(w, lcchild.as_ptr(), type_) > 0 {
                    layout_resize_adjust(w, lcchild.as_ptr(), type_, -1);
                    change += 1;
                }
            }
        }
    }
}

/// Destroy a cell and redistribute the space.
pub unsafe fn layout_destroy_cell(
    w: *mut window,
    lc: *mut layout_cell,
    lcroot: *mut *mut layout_cell,
) {
    unsafe {
        let lcparent = (*lc).parent;

        // If no parent, this is the last pane so window close is imminent and
        // there is no need to resize anything.
        if lcparent.is_null() {
            layout_free_cell(lc);
            *lcroot = std::ptr::null_mut();
            return;
        }

        // Merge the space into the previous or next cell
        let lcother: *mut layout_cell = if lc == tailq_first(&raw mut (*lcparent).cells) {
            tailq_next(lc)
        } else {
            tailq_prev(lc)
        };

        if !lcother.is_null() {
            if (*lcparent).type_ == layout_type::LAYOUT_LEFTRIGHT {
                layout_resize_adjust(w, lcother, (*lcparent).type_, (*lc).sx as i32 + 1);
            } else {
                layout_resize_adjust(w, lcother, (*lcparent).type_, (*lc).sy as i32 + 1);
            }
        }

        // Remove this from the parent's list
        tailq_remove(&mut (*lcparent).cells, lc);
        layout_free_cell(lc);

        // If the parent now has one cell, remove the parent from the tree and
        // replace it by that cell
        let lc = tailq_first(&raw mut (*lcparent).cells);
        if tailq_next(lc).is_null() {
            tailq_remove(&raw mut (*lcparent).cells, lc);

            (*lc).parent = (*lcparent).parent;
            if (*lc).parent.is_null() {
                (*lc).xoff = 0;
                (*lc).yoff = 0;
                *lcroot = lc;
            } else {
                tailq_replace(&mut (*(*lc).parent).cells, lcparent, lc);
            }

            layout_free_cell(lcparent);
        }
    }
}

pub unsafe fn layout_init(w: *mut window, wp: *mut window_pane) {
    unsafe {
        let lc = layout_create_cell(std::ptr::null_mut());
        (*w).layout_root = lc;
        layout_set_size(lc, (*w).sx, (*w).sy, 0, 0);
        layout_make_leaf(lc, wp);
        layout_fix_panes(w, std::ptr::null_mut());
    }
}

pub unsafe fn layout_free(w: *mut window) {
    unsafe {
        layout_free_cell((*w).layout_root);
    }
}

/// Resize the entire layout after window resize.
pub unsafe fn layout_resize(w: *mut window, sx: c_uint, sy: c_uint) {
    unsafe {
        let lc = (*w).layout_root;

        // Adjust horizontally. Do not attempt to reduce the layout lower than
        // the minimum (more than the amount returned by layout_resize_check).
        //
        // This can mean that the window size is smaller than the total layout
        // size: redrawing this is handled at a higher level, but it does leave
        // a problem with growing the window size here: if the current size is
        // < the minimum, growing proportionately by adding to each pane is
        // wrong as it would keep the layout size larger than the window size.
        // Instead, spread the difference between the minimum and the new size
        // out proportionately - this should leave the layout fitting the new
        // window size.
        let mut xchange = sx as c_int - (*lc).sx as c_int;
        let xlimit = layout_resize_check(w, lc, layout_type::LAYOUT_LEFTRIGHT) as i32;
        if xchange < 0 && xchange < -xlimit {
            xchange = -xlimit;
        }
        if xlimit == 0 {
            if sx <= (*lc).sx {
                // lc->sx is minimum possible
                xchange = 0;
            } else {
                xchange = sx as c_int - (*lc).sx as c_int;
            }
        }
        if xchange != 0 {
            layout_resize_adjust(w, lc, layout_type::LAYOUT_LEFTRIGHT, xchange);
        }

        // Adjust vertically in a similar fashion.
        let mut ychange = sy as c_int - (*lc).sy as c_int;
        let ylimit = layout_resize_check(w, lc, layout_type::LAYOUT_TOPBOTTOM) as i32;
        if ychange < 0 && ychange < -ylimit {
            ychange = -ylimit;
        }
        if ylimit == 0 {
            if sy <= (*lc).sy {
                // lc->sy is minimum possible
                ychange = 0;
            } else {
                ychange = sy as c_int - (*lc).sy as c_int;
            }
        }
        if ychange != 0 {
            layout_resize_adjust(w, lc, layout_type::LAYOUT_TOPBOTTOM, ychange);
        }

        // Fix cell offsets.
        layout_fix_offsets(w);
        layout_fix_panes(w, std::ptr::null_mut());
    }
}

/// Resize a pane to an absolute size.
pub unsafe fn layout_resize_pane_to(wp: *mut window_pane, type_: layout_type, new_size: u32) {
    unsafe {
        let mut lc = (*wp).layout_cell;
        let mut lcparent;

        // Find next parent of the same type
        lcparent = (*lc).parent;
        while !lcparent.is_null() && (*lcparent).type_ != type_ {
            lc = lcparent;
            lcparent = (*lc).parent;
        }
        if lcparent.is_null() {
            return;
        }

        // Work out the size adjustment
        let size = if type_ == layout_type::LAYOUT_LEFTRIGHT {
            (*lc).sx
        } else {
            (*lc).sy
        };

        let change = if lc == tailq_last(&raw mut (*lcparent).cells) {
            size as i32 - new_size as i32
        } else {
            new_size as i32 - size as i32
        };

        // Resize the pane
        layout_resize_pane(wp, type_, change, 1);
    }
}

pub unsafe fn layout_resize_layout(
    w: *mut window,
    lc: *mut layout_cell,
    type_: layout_type,
    change: c_int,
    opposite: c_int,
) {
    unsafe {
        let mut needed = change;
        let mut size;

        // Grow or shrink the cell
        while needed != 0 {
            if change > 0 {
                size = layout_resize_pane_grow(w, lc, type_, needed, opposite);
                needed -= size;
            } else {
                size = layout_resize_pane_shrink(w, lc, type_, needed);
                needed += size;
            }

            if size == 0 {
                // no more change possible
                break;
            }
        }

        // Fix cell offsets
        layout_fix_offsets(w);
        layout_fix_panes(w, null_mut());
        notify_window(c"window-layout-changed", w);
    }
}

pub unsafe fn layout_resize_pane(
    wp: *mut window_pane,
    type_: layout_type,
    change: c_int,
    opposite: c_int,
) {
    unsafe {
        let mut lc = (*wp).layout_cell;
        let mut lcparent;

        // Find next parent of the same type
        lcparent = (*lc).parent;
        while !lcparent.is_null() && (*lcparent).type_ != type_ {
            lc = lcparent;
            lcparent = (*lc).parent;
        }
        if lcparent.is_null() {
            return;
        }

        // If this is the last cell, move back one
        if lc == tailq_last(&raw mut (*lcparent).cells) {
            lc = tailq_prev(lc);
        }

        layout_resize_layout((*wp).window, lc, type_, change, opposite);
    }
}

/// Helper function to grow pane.
pub unsafe fn layout_resize_pane_grow(
    w: *mut window,
    lc: *mut layout_cell,
    type_: layout_type,
    needed: c_int,
    opposite: c_int,
) -> c_int {
    unsafe {
        let mut size: u32 = 0;

        // Growing. Always add to the current cell
        let lcadd = lc;

        // Look towards the tail for a suitable cell for reduction
        let mut lcremove = tailq_next(lc);
        while !lcremove.is_null() {
            size = layout_resize_check(w, lcremove, type_);
            if size > 0 {
                break;
            }
            lcremove = tailq_next(lcremove);
        }

        // If none found, look towards the head
        if opposite != 0 && lcremove.is_null() {
            lcremove = tailq_prev(lc);
            while !lcremove.is_null() {
                size = layout_resize_check(w, lcremove, type_);
                if size > 0 {
                    break;
                }
                lcremove = tailq_prev(lcremove);
            }
        }
        if lcremove.is_null() {
            return 0;
        }

        // Change the cells
        if size > needed as u32 {
            size = needed as u32;
        }
        layout_resize_adjust(w, lcadd, type_, size as c_int);
        layout_resize_adjust(w, lcremove, type_, -(size as c_int));
        size as c_int
    }
}

/// Helper function to shrink pane.
pub unsafe fn layout_resize_pane_shrink(
    w: *mut window,
    lc: *mut layout_cell,
    type_: layout_type,
    needed: c_int,
) -> c_int {
    unsafe {
        let mut size: u32;

        // Shrinking. Find cell to remove from by walking towards head
        let mut lcremove = lc;
        loop {
            size = layout_resize_check(w, lcremove, type_);
            if size != 0 {
                break;
            }
            lcremove = tailq_prev(lcremove);
            if lcremove.is_null() {
                break;
            }
        }
        if lcremove.is_null() {
            return 0;
        }

        // And add onto the next cell (from the original cell)
        let lcadd = tailq_next(lc);
        if lcadd.is_null() {
            return 0;
        }

        // Change the cells
        if size > (-needed) as u32 {
            size = (-needed) as u32;
        }
        layout_resize_adjust(w, lcadd, type_, size as c_int);
        layout_resize_adjust(w, lcremove, type_, -(size as c_int));
        size as c_int
    }
}

/// Assign window pane to newly split cell.
pub unsafe fn layout_assign_pane(lc: *mut layout_cell, wp: *mut window_pane, do_not_resize: c_int) {
    unsafe {
        layout_make_leaf(lc, wp);
        if do_not_resize != 0 {
            layout_fix_panes((*wp).window, wp);
        } else {
            layout_fix_panes((*wp).window, null_mut());
        }
    }
}

/// Calculate the new pane size for resized parent.
pub unsafe fn layout_new_pane_size(
    w: *mut window,
    previous: u32,
    lc: *mut layout_cell,
    type_: layout_type,
    size: u32,
    count_left: u32,
    size_left: u32,
) -> u32 {
    unsafe {
        // If this is the last cell, it can take all of the remaining size.
        if count_left == 1 {
            return size_left;
        }

        // How much is available in this parent?
        let available: u32 = layout_resize_check(w, lc, type_);

        // Work out the minimum size of this cell and the new size
        // proportionate to the previous size.
        let mut min: u32 = (PANE_MINIMUM + 1) * (count_left - 1);
        let mut new_size: u32 = if type_ == layout_type::LAYOUT_LEFTRIGHT {
            if (*lc).sx - available > min {
                min = (*lc).sx - available;
            }
            ((*lc).sx * size) / previous
        } else {
            if (*lc).sy - available > min {
                min = (*lc).sy - available;
            }
            ((*lc).sy * size) / previous
        };

        // Check against the maximum and minimum size.
        let max: u32 = size_left - min;
        if new_size > max {
            new_size = max;
        }
        if new_size < PANE_MINIMUM {
            new_size = PANE_MINIMUM;
        }
        new_size
    }
}

/// Check if the cell and all its children can be resized to a specific size.
pub unsafe fn layout_set_size_check(
    w: *mut window,
    lc: *mut layout_cell,
    type_: layout_type,
    size: c_int,
) -> bool {
    unsafe {
        let mut new_size: u32;
        let mut available: u32;
        let previous: u32;
        let mut idx: u32;

        // Cells with no children must just be bigger than minimum
        if (*lc).type_ == layout_type::LAYOUT_WINDOWPANE {
            return size >= PANE_MINIMUM as i32;
        }
        available = size as u32;

        // Count number of children
        let count: u32 = tailq_foreach(&raw mut (*lc).cells).count() as u32;

        // Check new size will work for each child
        if (*lc).type_ == type_ {
            if available < (count * 2) - 1 {
                return false;
            }

            if type_ == layout_type::LAYOUT_LEFTRIGHT {
                previous = (*lc).sx;
            } else {
                previous = (*lc).sy;
            }

            idx = 0;
            for lcchild in tailq_foreach(&raw mut (*lc).cells).map(NonNull::as_ptr) {
                new_size = layout_new_pane_size(
                    w,
                    previous,
                    lcchild,
                    type_,
                    size as u32,
                    count - idx,
                    available,
                );
                if idx == count - 1 {
                    if new_size > available {
                        return false;
                    }
                    available -= new_size;
                } else {
                    if new_size + 1 > available {
                        return false;
                    }
                    available -= new_size + 1;
                }
                if !layout_set_size_check(w, lcchild, type_, new_size as i32) {
                    return false;
                }
                idx += 1;
            }
        } else {
            for lcchild in tailq_foreach(&raw mut (*lc).cells).map(NonNull::as_ptr) {
                if (*lcchild).type_ == layout_type::LAYOUT_WINDOWPANE {
                    continue;
                }
                if !layout_set_size_check(w, lcchild, type_, size) {
                    return false;
                }
            }
        }

        true
    }
}

// unsafe extern "C" { pub fn layout_resize_child_cells(w: *mut window, lc: *mut layout_cell); }
/// Resize all child cells to fit within the current cell.
pub unsafe fn layout_resize_child_cells(w: *mut window, lc: *mut layout_cell) {
    unsafe {
        if (*lc).type_ == layout_type::LAYOUT_WINDOWPANE {
            return;
        }

        // What is the current size used?
        let mut count: u32 = 0;
        let mut previous: u32 = 0;
        for lcchild in tailq_foreach(&raw mut (*lc).cells).map(NonNull::as_ptr) {
            count += 1;
            if (*lc).type_ == layout_type::LAYOUT_LEFTRIGHT {
                previous += (*lcchild).sx;
            } else if (*lc).type_ == layout_type::LAYOUT_TOPBOTTOM {
                previous += (*lcchild).sy;
            }
        }
        previous += count - 1;

        // And how much is available?
        let mut available: u32 = 0;
        if (*lc).type_ == layout_type::LAYOUT_LEFTRIGHT {
            available = (*lc).sx;
        } else if (*lc).type_ == layout_type::LAYOUT_TOPBOTTOM {
            available = (*lc).sy;
        }

        // Resize children into the new size.
        for (idx, lcchild) in tailq_foreach(&raw mut (*lc).cells)
            .map(NonNull::as_ptr)
            .enumerate()
        {
            if (*lc).type_ == layout_type::LAYOUT_TOPBOTTOM {
                (*lcchild).sx = (*lc).sx;
                (*lcchild).xoff = (*lc).xoff;
            } else {
                (*lcchild).sx = layout_new_pane_size(
                    w,
                    previous,
                    lcchild,
                    (*lc).type_,
                    (*lc).sx,
                    count - idx as u32,
                    available,
                );
                available -= (*lcchild).sx + 1;
            }
            if (*lc).type_ == layout_type::LAYOUT_LEFTRIGHT {
                (*lcchild).sy = (*lc).sy;
            } else {
                (*lcchild).sy = layout_new_pane_size(
                    w,
                    previous,
                    lcchild,
                    (*lc).type_,
                    (*lc).sy,
                    count - idx as u32,
                    available,
                );
                available -= (*lcchild).sy + 1;
            }
            layout_resize_child_cells(w, lcchild);
        }
    }
}

/// Split a pane into two. size is a hint, or -1 for default half/half
/// split. This must be followed by `layout_assign_pane` before much else happens!
pub unsafe fn layout_split_pane(
    wp: *mut window_pane,
    type_: layout_type,
    size: i32,
    flags: spawn_flags,
) -> *mut layout_cell {
    unsafe {
        let minimum: u32;
        let mut resize_first: u32 = 0;
        let full_size = flags.intersects(SPAWN_FULLSIZE);

        // If full_size is specified, add a new cell at the top of the window
        // layout. Otherwise, split the cell for the current pane.
        let lc: *mut layout_cell = if full_size {
            (*(*wp).window).layout_root
        } else {
            (*wp).layout_cell
        };
        let status = pane_status::try_from(options_get_number_(
            (*(*wp).window).options,
            "pane-border-status",
        ) as i32)
        .unwrap();

        // Copy the old cell size
        let sx = (*lc).sx;
        let sy = (*lc).sy;
        let xoff = (*lc).xoff;
        let yoff = (*lc).yoff;

        // Check there is enough space for the two new panes
        match type_ {
            layout_type::LAYOUT_LEFTRIGHT => {
                if sx < PANE_MINIMUM * 2 + 1 {
                    return null_mut();
                }
            }
            layout_type::LAYOUT_TOPBOTTOM => {
                if layout_add_border((*wp).window, lc, status) {
                    minimum = PANE_MINIMUM * 2 + 2;
                } else {
                    minimum = PANE_MINIMUM * 2 + 1;
                }
                if sy < minimum {
                    return null_mut();
                }
            }
            _ => fatalx("bad layout type"),
        }

        // Calculate new cell sizes. size is the target size or -1 for middle
        // split, size1 is the size of the top/left and size2 the bottom/right.
        let saved_size = if type_ == layout_type::LAYOUT_LEFTRIGHT {
            sx
        } else {
            sy
        };

        let mut size2 = if size < 0 {
            saved_size.div_ceil(2) - 1
        } else if flags.intersects(SPAWN_BEFORE) {
            saved_size - size as u32 - 1
        } else {
            size as u32
        };

        if size2 < PANE_MINIMUM {
            size2 = PANE_MINIMUM;
        } else if size2 > saved_size - 2 {
            size2 = saved_size - 2;
        }
        let size1 = saved_size - 1 - size2;

        // Which size are we using?
        let new_size = if flags.intersects(SPAWN_BEFORE) {
            size2
        } else {
            size1
        };

        // Confirm there is enough space for full size pane.
        if full_size && !layout_set_size_check((*wp).window, lc, type_, new_size as i32) {
            return null_mut();
        }

        let lcparent: *mut layout_cell;
        let lcnew: *mut layout_cell;

        if !(*lc).parent.is_null() && (*(*lc).parent).type_ == type_ {
            // If the parent exists and is of the same type as the split,
            // create a new cell and insert it after this one.
            lcparent = (*lc).parent;
            lcnew = layout_create_cell(lcparent);
            if flags.intersects(SPAWN_BEFORE) {
                tailq_insert_before(lc, lcnew);
            } else {
                tailq_insert_after(&raw mut (*lcparent).cells, lc, lcnew);
            }
        } else if full_size && (*lc).parent.is_null() && (*lc).type_ == type_ {
            // If the new full size pane is the same type as the root
            // split, insert the new pane under the existing root cell
            // instead of creating a new root cell. The existing layout
            // must be resized before inserting the new cell.
            if (*lc).type_ == layout_type::LAYOUT_LEFTRIGHT {
                (*lc).sx = new_size;
                layout_resize_child_cells((*wp).window, lc);
                (*lc).sx = saved_size;
            } else if (*lc).type_ == layout_type::LAYOUT_TOPBOTTOM {
                (*lc).sy = new_size;
                layout_resize_child_cells((*wp).window, lc);
                (*lc).sy = saved_size;
            }
            resize_first = 1;

            // Create the new cell.
            lcnew = layout_create_cell(lc);
            let size = saved_size - 1 - new_size;
            if (*lc).type_ == layout_type::LAYOUT_LEFTRIGHT {
                layout_set_size(lcnew, size, sy, 0, 0);
            } else if (*lc).type_ == layout_type::LAYOUT_TOPBOTTOM {
                layout_set_size(lcnew, sx, size, 0, 0);
            }
            if flags.intersects(SPAWN_BEFORE) {
                tailq_insert_head(&raw mut (*lc).cells, lcnew);
            } else {
                tailq_insert_tail(&raw mut (*lc).cells, lcnew);
            }
        } else {
            // Otherwise create a new parent and insert it.

            // Create and insert the replacement parent.
            lcparent = layout_create_cell((*lc).parent);
            layout_make_node(lcparent, type_);
            layout_set_size(lcparent, sx, sy, xoff, yoff);
            if (*lc).parent.is_null() {
                (*(*wp).window).layout_root = lcparent;
            } else {
                tailq_replace(&raw mut (*(*lc).parent).cells, lc, lcparent);
            }

            // Insert the old cell.
            (*lc).parent = lcparent;
            tailq_insert_head(&raw mut (*lcparent).cells, lc);

            // Create the new child cell.
            lcnew = layout_create_cell(lcparent);
            if flags.intersects(SPAWN_BEFORE) {
                tailq_insert_head(&raw mut (*lcparent).cells, lcnew);
            } else {
                tailq_insert_tail(&raw mut (*lcparent).cells, lcnew);
            }
        }

        let (lc1, lc2) = if flags.intersects(SPAWN_BEFORE) {
            (lcnew, lc)
        } else {
            (lc, lcnew)
        };

        // Set new cell sizes. size1 is the size of the top/left and size2 the
        // bottom/right.
        if resize_first == 0 && type_ == layout_type::LAYOUT_LEFTRIGHT {
            layout_set_size(lc1, size1, sy, xoff, yoff);
            layout_set_size(lc2, size2, sy, xoff + (*lc1).sx + 1, yoff);
        } else if resize_first == 0 && type_ == layout_type::LAYOUT_TOPBOTTOM {
            layout_set_size(lc1, sx, size1, xoff, yoff);
            layout_set_size(lc2, sx, size2, xoff, yoff + (*lc1).sy + 1);
        }

        if full_size {
            if resize_first == 0 {
                layout_resize_child_cells((*wp).window, lc);
            }
            layout_fix_offsets((*wp).window);
        } else {
            layout_make_leaf(lc, wp);
        }

        lcnew
    }
}

/// Destroy the cell associated with a pane.
pub unsafe fn layout_close_pane(wp: *mut window_pane) {
    unsafe {
        let w = (*wp).window;

        // Remove the cell
        layout_destroy_cell(w, (*wp).layout_cell, &raw mut (*w).layout_root);

        // Fix pane offsets and sizes
        if !(*w).layout_root.is_null() {
            layout_fix_offsets(w);
            layout_fix_panes(w, null_mut());
        }
        notify_window(c"window-layout-changed", w);
    }
}

/// Spread cells evenly within a parent cell
pub unsafe fn layout_spread_cell(w: *mut window, parent: *mut layout_cell) -> c_int {
    unsafe {
        // Count number of cells
        let number = tailq_foreach(&raw mut (*parent).cells).count() as u32;
        if number <= 1 {
            return 0;
        }

        let status: pane_status = (options_get_number_((*w).options, "pane-border-status") as i32)
            .try_into()
            .unwrap();

        // Calculate available size
        let size = match (*parent).type_ {
            layout_type::LAYOUT_LEFTRIGHT => (*parent).sx,
            layout_type::LAYOUT_TOPBOTTOM => {
                if layout_add_border(w, parent, status) {
                    (*parent).sy - 1
                } else {
                    (*parent).sy
                }
            }
            _ => return 0,
        };

        if size < number - 1 {
            return 0;
        }

        let mut each = (size - (number - 1)) / number;
        if each == 0 {
            return 0;
        }

        let mut changed = 0;
        let mut idx = 0;
        for lc in tailq_foreach(&raw mut (*parent).cells).map(NonNull::as_ptr) {
            idx += 1;
            if idx == number {
                each = size - ((each + 1) * (number - 1));
            }

            let change = match (*parent).type_ {
                layout_type::LAYOUT_LEFTRIGHT => {
                    let change = each as i32 - (*lc).sx as i32;
                    layout_resize_adjust(w, lc, layout_type::LAYOUT_LEFTRIGHT, change);
                    change
                }
                layout_type::LAYOUT_TOPBOTTOM => {
                    let this = if layout_add_border(w, lc, status) {
                        each + 1
                    } else {
                        each
                    };
                    let change = this as i32 - (*lc).sy as i32;
                    layout_resize_adjust(w, lc, layout_type::LAYOUT_TOPBOTTOM, change);
                    change
                }
                _ => 0,
            };

            if change != 0 {
                changed = 1;
            }
        }

        changed
    }
}

/// Spread out a pane and its parent cells
pub unsafe fn layout_spread_out(wp: *mut window_pane) {
    unsafe {
        let mut parent = (*wp).layout_cell;
        if parent.is_null() {
            return;
        }
        parent = (*parent).parent;
        if parent.is_null() {
            return;
        }

        let w = (*wp).window;
        while !parent.is_null() {
            if layout_spread_cell(w, parent) != 0 {
                layout_fix_offsets(w);
                layout_fix_panes(w, null_mut());
                break;
            }
            parent = (*parent).parent;
        }
    }
}

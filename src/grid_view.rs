// Copyright (c) 2008 Nicholas Marriott <nicholas.marriott@gmail.com>
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

fn grid_view_x(_gd: *mut grid, x: u32) -> u32 {
    x
}
unsafe fn grid_view_y(gd: *mut grid, y: u32) -> u32 {
    unsafe { (*gd).hsize + (y) }
}

pub unsafe fn grid_view_get_cell(gd: *mut grid, px: u32, py: u32, gc: *mut grid_cell) {
    unsafe {
        grid_get_cell(gd, grid_view_x(gd, px), grid_view_y(gd, py), gc);
    }
}

pub unsafe fn grid_view_set_cell(gd: *mut grid, px: u32, py: u32, gc: *const grid_cell) {
    unsafe {
        grid_set_cell(gd, grid_view_x(gd, px), grid_view_y(gd, py), gc);
    }
}

pub unsafe fn grid_view_set_padding(gd: *mut grid, px: u32, py: u32) {
    unsafe {
        grid_set_padding(gd, grid_view_x(gd, px), grid_view_y(gd, py));
    }
}

pub unsafe fn grid_view_set_cells(
    gd: *mut grid,
    px: u32,
    py: u32,
    gc: *const grid_cell,
    s: *const u8,
    slen: usize,
) {
    unsafe {
        grid_set_cells(gd, grid_view_x(gd, px), grid_view_y(gd, py), gc, s, slen);
    }
}

pub unsafe fn grid_view_clear_history(gd: *mut grid, bg: u32) {
    unsafe {
        let mut last = 0u32;

        for yy in 0..(*gd).sy {
            let gl = grid_get_line(gd, grid_view_y(gd, yy));
            if (*gl).cellused != 0 {
                last = yy + 1;
            }
        }
        if last == 0 {
            grid_view_clear(gd, 0, 0, (*gd).sx, (*gd).sy, bg);
            return;
        }

        for _ in 0..(*gd).sy {
            grid_collect_history(gd);
            grid_scroll_history(gd, bg);
        }
        if last < (*gd).sy {
            grid_view_clear(gd, 0, 0, (*gd).sx, (*gd).sy - last, bg);
        }
        (*gd).hscrolled = 0;
    }
}

pub unsafe fn grid_view_clear(gd: *mut grid, mut px: u32, mut py: u32, nx: u32, ny: u32, bg: u32) {
    unsafe {
        px = grid_view_x(gd, px);
        py = grid_view_y(gd, py);

        grid_clear(gd, px, py, nx, ny, bg);
    }
}

pub unsafe fn grid_view_scroll_region_up(gd: *mut grid, mut rupper: u32, mut rlower: u32, bg: u32) {
    unsafe {
        if (*gd).flags & GRID_HISTORY != 0 {
            grid_collect_history(gd);
            if rupper == 0 && rlower == (*gd).sy - 1 {
                grid_scroll_history(gd, bg);
            } else {
                rupper = grid_view_y(gd, rupper);
                rlower = grid_view_y(gd, rlower);
                grid_scroll_history_region(gd, rupper, rlower, bg);
            }
        } else {
            rupper = grid_view_y(gd, rupper);
            rlower = grid_view_y(gd, rlower);
            grid_move_lines(gd, rupper, rupper + 1, rlower - rupper, bg);
        }
    }
}

pub unsafe fn grid_view_scroll_region_down(
    gd: *mut grid,
    mut rupper: u32,
    mut rlower: u32,
    bg: u32,
) {
    unsafe {
        rupper = grid_view_y(gd, rupper);
        rlower = grid_view_y(gd, rlower);

        grid_move_lines(gd, rupper + 1, rupper, rlower - rupper, bg);
    }
}

pub unsafe fn grid_view_insert_lines(gd: *mut grid, mut py: u32, ny: u32, bg: u32) {
    unsafe {
        py = grid_view_y(gd, py);

        let sy = grid_view_y(gd, (*gd).sy);

        grid_move_lines(gd, py + ny, py, sy - py - ny, bg);
    }
}

/// Insert lines in region.
pub unsafe fn grid_view_insert_lines_region(
    gd: *mut grid,
    mut rlower: u32,
    mut py: u32,
    ny: u32,
    bg: u32,
) {
    unsafe {
        rlower = grid_view_y(gd, rlower);

        py = grid_view_y(gd, py);

        let ny2 = rlower + 1 - py - ny;
        grid_move_lines(gd, rlower + 1 - ny2, py, ny2, bg);
        // TODO does this bug exist upstream?
        grid_clear(gd, 0, py + ny2, (*gd).sx, ny.saturating_sub(ny2), bg);
    }
}

/// Delete lines.
pub unsafe fn grid_view_delete_lines(gd: *mut grid, mut py: u32, ny: u32, bg: u32) {
    unsafe {
        py = grid_view_y(gd, py);

        let sy = grid_view_y(gd, (*gd).sy);

        grid_move_lines(gd, py, py + ny, sy - py - ny, bg);
        grid_clear(gd, 0, sy.saturating_sub(ny), (*gd).sx, ny, bg);
    }
}

/// Delete lines inside scroll region.
pub unsafe fn grid_view_delete_lines_region(
    gd: *mut grid,
    mut rlower: u32,
    mut py: u32,
    ny: u32,
    bg: u32,
) {
    unsafe {
        rlower = grid_view_y(gd, rlower);

        py = grid_view_y(gd, py);

        let ny2 = rlower + 1 - py - ny;
        grid_move_lines(gd, py, py + ny, ny2, bg);
        // TODO does this bug exist in the tmux source code too
        grid_clear(gd, 0, py + ny2, (*gd).sx, ny.saturating_sub(ny2), bg);
    }
}

/// Insert characters.
pub unsafe fn grid_view_insert_cells(gd: *mut grid, mut px: u32, mut py: u32, nx: u32, bg: u32) {
    unsafe {
        px = grid_view_x(gd, px);
        py = grid_view_y(gd, py);

        let sx = grid_view_x(gd, (*gd).sx);

        if px >= sx - 1 {
            grid_clear(gd, px, py, 1, 1, bg);
        } else {
            grid_move_cells(gd, px + nx, px, py, sx - px - nx, bg);
        }
    }
}

/// Delete characters.
pub unsafe fn grid_view_delete_cells(gd: *mut grid, mut px: u32, mut py: u32, nx: u32, bg: u32) {
    unsafe {
        px = grid_view_x(gd, px);
        py = grid_view_y(gd, py);

        let sx = grid_view_x(gd, (*gd).sx);

        grid_move_cells(gd, px, px + nx, py, sx - px - nx, bg);
        grid_clear(gd, sx - nx, py, nx, 1, bg);
    }
}

/// Convert cells into a string.
pub unsafe fn grid_view_string_cells(gd: *mut grid, mut px: u32, mut py: u32, nx: u32) -> *mut u8 {
    unsafe {
        px = grid_view_x(gd, px);
        py = grid_view_y(gd, py);

        grid_string_cells(
            gd,
            px,
            py,
            nx,
            null_mut(),
            grid_string_flags::empty(),
            null_mut(),
        )
    }
}

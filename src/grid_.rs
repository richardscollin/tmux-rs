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

use crate::compat::strlcat;
use crate::libc::strlen;
use crate::xmalloc::xreallocarray;

/// Default grid cell data.
pub static GRID_DEFAULT_CELL: grid_cell = grid_cell::new(
    utf8_data::new([b' '], 0, 1, 1),
    grid_attr::empty(),
    grid_flag::empty(),
    8,
    8,
    8,
    0,
);

/// Padding grid cell data. Padding cells are the only zero width cell that
/// appears in the grid - because of this, they are always extended cells.
pub static GRID_PADDING_CELL: grid_cell = grid_cell::new(
    utf8_data::new([b'!'], 0, 0, 0),
    grid_attr::empty(),
    grid_flag::PADDING,
    8,
    8,
    8,
    0,
);

/// Cleared grid cell data.
pub static GRID_CLEARED_CELL: grid_cell = grid_cell::new(
    utf8_data::new([b' '], 0, 1, 1),
    grid_attr::empty(),
    grid_flag::CLEARED,
    8,
    8,
    8,
    0,
);

pub static GRID_CLEARED_ENTRY: grid_cell_entry = grid_cell_entry {
    union_: grid_cell_entry_union {
        data: grid_cell_entry_data {
            attr: 0,
            fg: 8,
            bg: 8,
            data: b' ',
        },
    },
    flags: grid_flag::CLEARED,
};

/// Store cell in entry.
pub unsafe fn grid_store_cell(gce: *mut grid_cell_entry, gc: *const grid_cell, c: u8) {
    unsafe {
        (*gce).flags = (*gc).flags & !grid_flag::CLEARED;

        (*gce).union_.data.fg = ((*gc).fg & 0xff) as u8;
        if (*gc).fg & COLOUR_FLAG_256 != 0 {
            (*gce).flags |= grid_flag::FG256;
        }

        (*gce).union_.data.bg = ((*gc).bg & 0xff) as u8;
        if (*gc).bg & COLOUR_FLAG_256 != 0 {
            (*gce).flags |= grid_flag::BG256;
        }

        (*gce).union_.data.attr = (*gc).attr.bits() as u8;
        (*gce).union_.data.data = c;
    }
}

/// Check if a cell should be an extended cell.
pub unsafe fn grid_need_extended_cell(gce: *const grid_cell_entry, gc: *const grid_cell) -> bool {
    unsafe {
        if (*gce).flags.contains(grid_flag::EXTENDED) {
            return true;
        }
        if (*gc).attr.bits() > 0xff {
            return true;
        }
        if (*gc).data.size != 1 || (*gc).data.width != 1 {
            return true;
        }
        if ((*gc).fg & COLOUR_FLAG_RGB != 0) || ((*gc).bg & COLOUR_FLAG_RGB != 0) {
            return true;
        }
        if (*gc).us != 8 {
            // only supports 256 or RGB
            return true;
        }
        if (*gc).link != 0 {
            return true;
        }
        false
    }
}

/// Get an extended cell.
pub unsafe fn grid_get_extended_cell(
    gl: *mut grid_line,
    gce: *mut grid_cell_entry,
    flags: grid_flag,
) {
    unsafe {
        let at = (*gl).extdsize + 1;

        (*gl).extddata = xreallocarray_((*gl).extddata, at as usize).as_ptr();
        (*gl).extdsize = at;

        (*gce).union_.offset = at - 1;
        (*gce).flags = flags | grid_flag::EXTENDED;
    }
}

/// Set cell as extended.
pub unsafe fn grid_extended_cell(
    gl: *mut grid_line,
    gce: *mut grid_cell_entry,
    gc: *const grid_cell,
) -> *mut grid_extd_entry {
    unsafe {
        let flags = (*gc).flags & !grid_flag::CLEARED;

        if !(*gce).flags.contains(grid_flag::EXTENDED) {
            grid_get_extended_cell(gl, gce, flags);
        } else if (*gce).union_.offset >= (*gl).extdsize {
            fatalx("offset too big");
        }
        (*gl).flags |= grid_line_flag::EXTENDED;

        let uc = match utf8_from_data(&(*gc).data) {
            Ok(value) | Err(value) => value,
        };

        let gee = &mut *(*gl).extddata.offset((*gce).union_.offset as isize);
        gee.data = uc;
        gee.attr = (*gc).attr.bits();
        gee.flags = flags.bits();
        gee.fg = (*gc).fg;
        gee.bg = (*gc).bg;
        gee.us = (*gc).us;
        gee.link = (*gc).link;
        gee
    }
}

/// Free up unused extended cells.
pub unsafe fn grid_compact_line(gl: *mut grid_line) {
    unsafe {
        let mut new_extdsize = 0u32;

        if (*gl).extdsize == 0 {
            return;
        }

        // Count extended cells
        for px in 0..(*gl).cellsize {
            let gce = &raw mut *(*gl).celldata.add(px as usize);
            if (*gce).flags.contains(grid_flag::EXTENDED) {
                new_extdsize += 1;
            }
        }

        if new_extdsize == 0 {
            free_((*gl).extddata);
            (*gl).extddata = null_mut();
            (*gl).extdsize = 0;
            return;
        }

        // Allocate new array
        let new_extddata: *mut grid_extd_entry =
            xreallocarray_(null_mut(), new_extdsize as usize).as_ptr();

        let mut idx = 0;
        for px in 0..(*gl).cellsize {
            let gce = (*gl).celldata.add(px as usize);
            if (*gce).flags.contains(grid_flag::EXTENDED) {
                let gee = (*gl).extddata.add((*gce).union_.offset as usize);
                std::ptr::copy_nonoverlapping(
                    gee as *const grid_extd_entry,
                    new_extddata.add(idx as usize),
                    1,
                );
                (*gce).union_.offset = idx;
                idx += 1;
            }
        }

        free_((*gl).extddata);
        (*gl).extddata = new_extddata;
        (*gl).extdsize = new_extdsize;
    }
}

/// Get line data.
pub unsafe fn grid_get_line(gd: *mut grid, line: c_uint) -> *mut grid_line {
    unsafe { (*gd).linedata.add(line as usize) }
}

/// Adjust number of lines.
pub unsafe fn grid_adjust_lines(gd: *mut grid, lines: c_uint) {
    unsafe {
        (*gd).linedata = xreallocarray_((*gd).linedata, lines as usize).as_ptr();
    }
}

/// Copy default into a cell.
pub unsafe fn grid_clear_cell(gd: *mut grid, px: c_uint, py: c_uint, bg: c_uint) {
    unsafe {
        let gl = (*gd).linedata.add(py as usize);
        let gce = (*gl).celldata.add(px as usize);
        std::ptr::copy_nonoverlapping(&raw const GRID_CLEARED_ENTRY, gce, 1);
        if bg != 8 {
            if (bg & COLOUR_FLAG_RGB as u32) != 0 {
                grid_get_extended_cell(gl, gce, (*gce).flags);
                let gee = grid_extended_cell(gl, gce, &raw const GRID_CLEARED_CELL);
                (*gee).bg = bg as i32;
            } else {
                if (bg & COLOUR_FLAG_256 as u32) != 0 {
                    (*gce).flags |= grid_flag::BG256;
                }
                (*gce).union_.data.bg = bg as c_uchar;
            }
        }
    }
}

/// Check grid y position.
pub unsafe fn grid_check_y(gd: *mut grid, from: *const u8, py: c_uint) -> c_int {
    unsafe {
        if py >= (*gd).hsize as c_uint + (*gd).sy as c_uint {
            log_debug!("{}: y out of range: {}", _s(from), py);
            return -1;
        }
    }
    0
}

/// Check if two styles are (visibly) the same.
pub unsafe fn grid_cells_look_equal(gc1: *const grid_cell, gc2: *const grid_cell) -> c_int {
    unsafe {
        if (*gc1).fg != (*gc2).fg || (*gc1).bg != (*gc2).bg {
            return 0;
        }
        if (*gc1).attr != (*gc2).attr || (*gc1).flags != (*gc2).flags {
            return 0;
        }
        if (*gc1).link != (*gc2).link {
            return 0;
        }
        1
    }
}

/// Compare grid cells. Return 1 if equal, 0 if not.
pub unsafe fn grid_cells_equal(gc1: *const grid_cell, gc2: *const grid_cell) -> c_int {
    unsafe {
        if grid_cells_look_equal(gc1, gc2) == 0 {
            return 0;
        }
        if (*gc1).data.width != (*gc2).data.width {
            return 0;
        }
        if (*gc1).data.size != (*gc2).data.size {
            return 0;
        }
        if libc::memcmp(
            (*gc1).data.data.as_ptr().cast(),
            (*gc2).data.data.as_ptr().cast(),
            (*gc1).data.size as usize,
        ) == 0
        {
            1
        } else {
            0
        }
    }
}

/// Free one line.
pub unsafe fn grid_free_line(gd: *mut grid, py: c_uint) {
    unsafe {
        free_((*(*gd).linedata.add(py as usize)).celldata);
        (*(*gd).linedata.add(py as usize)).celldata = null_mut();
        free_((*(*gd).linedata.add(py as usize)).extddata);
        (*(*gd).linedata.add(py as usize)).extddata = null_mut();
    }
}

/// Free several lines.
pub unsafe fn grid_free_lines(gd: *mut grid, py: c_uint, ny: c_uint) {
    unsafe {
        for yy in py..(py + ny) {
            grid_free_line(gd, yy);
        }
    }
}

/// Create a new grid.
pub unsafe fn grid_create(sx: u32, sy: u32, hlimit: u32) -> *mut grid {
    unsafe {
        let gd = xmalloc_::<grid>().as_ptr();
        (*gd).sx = sx;
        (*gd).sy = sy;
        (*gd).flags = if hlimit != 0 { GRID_HISTORY } else { 0 };

        (*gd).hscrolled = 0;
        (*gd).hsize = 0;
        (*gd).hlimit = hlimit;

        if (*gd).sy != 0 {
            (*gd).linedata = xcalloc_::<grid_line>((*gd).sy as usize).as_ptr();
        } else {
            (*gd).linedata = null_mut();
        }

        gd
    }
}

/// Destroy grid.
pub unsafe fn grid_destroy(gd: *mut grid) {
    unsafe {
        grid_free_lines(gd, 0, (*gd).hsize + (*gd).sy);
        free_((*gd).linedata);
        free_(gd);
    }
}

/// Compare grids.
pub unsafe fn grid_compare(ga: *mut grid, gb: *mut grid) -> c_int {
    unsafe {
        if (*ga).sx != (*gb).sx || (*ga).sy != (*gb).sy {
            return 1;
        }

        for yy in 0..(*ga).sy {
            let gla = &mut (*(*ga).linedata.add(yy as usize));
            let glb = &mut (*(*gb).linedata.add(yy as usize));

            if gla.cellsize != glb.cellsize {
                return 1;
            }

            for xx in 0..gla.cellsize {
                let mut gca = grid_cell::new(
                    utf8_data::new([0; 4], 0, 0, 0),
                    grid_attr::empty(),
                    grid_flag::empty(),
                    0,
                    0,
                    0,
                    0,
                );
                let mut gcb = grid_cell::new(
                    utf8_data::new([0; 4], 0, 0, 0),
                    grid_attr::empty(),
                    grid_flag::empty(),
                    0,
                    0,
                    0,
                    0,
                );

                grid_get_cell(ga, xx, yy, &mut gca);
                grid_get_cell(gb, xx, yy, &mut gcb);

                if grid_cells_equal(&gca, &gcb) == 0 {
                    return 1;
                }
            }
        }

        0
    }
}

/// Trim lines from the history.
unsafe fn grid_trim_history(gd: *mut grid, ny: c_uint) {
    unsafe {
        grid_free_lines(gd, 0, ny);
        libc::memmove(
            (*gd).linedata as *mut c_void,
            (*gd).linedata.add(ny as usize) as *const c_void,
            ((*gd).hsize + (*gd).sy - ny) as usize * size_of::<grid_line>(),
        );
    }
}

/// Collect lines from the history if at the limit. Free the top (oldest) 10% and shift up.
pub unsafe fn grid_collect_history(gd: *mut grid) {
    unsafe {
        if (*gd).hsize == 0 || (*gd).hsize < (*gd).hlimit {
            return;
        }

        let mut ny = (*gd).hlimit / 10;
        if ny < 1 {
            ny = 1;
        }
        if ny > (*gd).hsize {
            ny = (*gd).hsize;
        }

        // Free the lines from 0 to ny then move the remaining lines over them.
        grid_trim_history(gd, ny);

        (*gd).hsize -= ny;
        if (*gd).hscrolled > (*gd).hsize {
            (*gd).hscrolled = (*gd).hsize;
        }
    }
}

/// Remove lines from the bottom of the history.
pub unsafe fn grid_remove_history(gd: *mut grid, ny: c_uint) {
    unsafe {
        if ny > (*gd).hsize {
            return;
        }
        for yy in 0..ny {
            grid_free_line(gd, (*gd).hsize + (*gd).sy - 1 - yy);
        }
        (*gd).hsize -= ny;
    }
}

/// Scroll the entire visible screen, moving one line into the history. Just
/// allocate a new line at the bottom and move the history size indicator.
pub unsafe fn grid_scroll_history(gd: *mut grid, bg: c_uint) {
    unsafe {
        let yy = (*gd).hsize + (*gd).sy;
        (*gd).linedata = xreallocarray_((*gd).linedata, (yy + 1) as usize).as_ptr();

        grid_empty_line(gd, yy, bg);

        (*gd).hscrolled += 1;
        grid_compact_line(&mut (*(*gd).linedata.add((*gd).hsize as usize)));
        (*(*gd).linedata.add((*gd).hsize as usize)).time = CURRENT_TIME;
        (*gd).hsize += 1;
    }
}

/// Clear the history.
pub unsafe fn grid_clear_history(gd: *mut grid) {
    unsafe {
        grid_trim_history(gd, (*gd).hsize);

        (*gd).hscrolled = 0;
        (*gd).hsize = 0;

        (*gd).linedata = xreallocarray_((*gd).linedata, (*gd).sy as usize).as_ptr();
    }
}

/// Scroll a region up, moving the top line into the history.
pub unsafe fn grid_scroll_history_region(
    gd: *mut grid,
    mut upper: c_uint,
    mut lower: c_uint,
    bg: c_uint,
) {
    unsafe {
        let yy = (*gd).hsize + (*gd).sy;

        // Create space for new line
        (*gd).linedata = xreallocarray_((*gd).linedata, (yy + 1) as usize).as_ptr();

        // Move screen down to free space
        let gl_history = (*gd).linedata.add((*gd).hsize as usize);
        std::ptr::copy(gl_history, gl_history.add(1), (*gd).sy as usize);

        // Adjust region and find start/end
        upper += 1;
        let gl_upper = (*gd).linedata.add(upper as usize);
        lower += 1;

        // Move line into history
        std::ptr::copy_nonoverlapping(gl_upper, gl_history, 1);
        (*gl_history).time = CURRENT_TIME;

        // Move region up and clear bottom line
        std::ptr::copy(gl_upper.add(1), gl_upper, (lower - upper) as usize);
        grid_empty_line(gd, lower, bg);

        // Move history offset down
        (*gd).hscrolled += 1;
        (*gd).hsize += 1;
    }
}

/// Expand line to fit to cell.
unsafe fn grid_expand_line(gd: *mut grid, py: c_uint, mut sx: c_uint, bg: c_uint) {
    unsafe {
        let gl = (*gd).linedata.add(py as usize);
        if sx <= (*gl).cellsize {
            return;
        }

        if sx < (*gd).sx / 4 {
            sx = (*gd).sx / 4;
        } else if sx < (*gd).sx / 2 {
            sx = (*gd).sx / 2;
        } else if (*gd).sx > sx {
            sx = (*gd).sx;
        }

        (*gl).celldata = xreallocarray_((*gl).celldata, sx as usize).as_ptr();

        for xx in (*gl).cellsize..sx {
            grid_clear_cell(gd, xx, py, bg);
        }
        (*gl).cellsize = sx;
    }
}

/// Empty a line and set background colour if needed.
pub unsafe fn grid_empty_line(gd: *mut grid, py: c_uint, bg: c_uint) {
    unsafe {
        (*gd).linedata.add(py as usize).write(zeroed());
        if !COLOUR_DEFAULT(bg as i32) {
            grid_expand_line(gd, py, (*gd).sx, bg);
        }
    }
}

/// Peek at grid line.
pub unsafe fn grid_peek_line(gd: *mut grid, py: c_uint) -> *mut grid_line {
    unsafe {
        if grid_check_y(gd, c!("grid_peek_line"), py) != 0 {
            return null_mut();
        }
        (*gd).linedata.add(py as usize)
    }
}

/// Get cell from line.
unsafe fn grid_get_cell1(gl: *mut grid_line, px: c_uint, gc: *mut grid_cell) {
    unsafe {
        let gce = (*gl).celldata.add(px as usize);

        if (*gce).flags.contains(grid_flag::EXTENDED) {
            if (*gce).union_.offset >= (*gl).extdsize {
                std::ptr::copy(&GRID_DEFAULT_CELL, gc, 1);
            } else {
                let gee = (*gl).extddata.add((*gce).union_.offset as usize);
                (*gc).flags = grid_flag::from_bits((*gee).flags).unwrap();
                (*gc).attr = grid_attr::from_bits((*gee).attr).expect("invalid grid_attr");
                (*gc).fg = (*gee).fg;
                (*gc).bg = (*gee).bg;
                (*gc).us = (*gee).us;
                (*gc).link = (*gee).link;
                (*gc).data = utf8_to_data((*gee).data);
            }
            return;
        }

        (*gc).flags = (*gce).flags & !(grid_flag::FG256 | grid_flag::BG256);
        (*gc).attr = grid_attr::from_bits((*gce).union_.data.attr as u16).unwrap();
        (*gc).fg = (*gce).union_.data.fg as i32;
        if (*gce).flags.contains(grid_flag::FG256) {
            (*gc).fg |= COLOUR_FLAG_256;
        }
        (*gc).bg = (*gce).union_.data.bg as i32;
        if (*gce).flags.contains(grid_flag::BG256) {
            (*gc).bg |= COLOUR_FLAG_256;
        }
        (*gc).us = 8;
        utf8_set(&mut (*gc).data, (*gce).union_.data.data);
        (*gc).link = 0;
    }
}

/// Get cell for reading.
pub unsafe fn grid_get_cell(gd: *mut grid, px: c_uint, py: c_uint, gc: *mut grid_cell) {
    unsafe {
        if grid_check_y(gd, c!("grid_get_cell"), py) != 0
            || px >= (*(*gd).linedata.add(py as usize)).cellsize
        {
            std::ptr::copy(&raw const GRID_DEFAULT_CELL, gc, 1);
        } else {
            grid_get_cell1((*gd).linedata.add(py as usize), px, gc);
        }
    }
}

/// Set cell at position.
pub unsafe fn grid_set_cell(gd: *mut grid, px: c_uint, py: c_uint, gc: *const grid_cell) {
    unsafe {
        if grid_check_y(gd, c!("grid_set_cell"), py) != 0 {
            return;
        }

        grid_expand_line(gd, py, px + 1, 8);

        let gl = &mut (*(*gd).linedata.add(py as usize));
        if px + 1 > gl.cellused {
            gl.cellused = px + 1;
        }

        let gce = gl.celldata.add(px as usize);
        if grid_need_extended_cell(gce, gc) {
            grid_extended_cell(gl, gce, gc);
        } else {
            grid_store_cell(gce, gc, (*gc).data.data[0]);
        }
    }
}

/// Set padding at position.
pub unsafe fn grid_set_padding(gd: *mut grid, px: c_uint, py: c_uint) {
    unsafe {
        grid_set_cell(gd, px, py, &GRID_PADDING_CELL);
    }
}

/// Set cells at position.
pub unsafe fn grid_set_cells(
    gd: *mut grid,
    px: u32,
    py: u32,
    gc: *const grid_cell,
    s: *const u8,
    slen: usize,
) {
    unsafe {
        if grid_check_y(gd, c!("grid_set_cells"), py) != 0 {
            return;
        }

        grid_expand_line(gd, py, px + slen as c_uint, 8);

        let gl = (*gd).linedata.add(py as usize);
        if px + slen as c_uint > (*gl).cellused {
            (*gl).cellused = px + slen as c_uint;
        }

        for i in 0..slen {
            let gce = (*gl).celldata.add((px + i as c_uint) as usize);
            if grid_need_extended_cell(gce, gc) {
                let gee = grid_extended_cell(gl, gce, gc);
                (*gee).data = utf8_build_one(*s.add(i));
            } else {
                grid_store_cell(gce, gc, *s.add(i));
            }
        }
    }
}

/// Clear area.
pub unsafe fn grid_clear(
    gd: *mut grid,
    px: c_uint,
    py: c_uint,
    nx: c_uint,
    ny: c_uint,
    bg: c_uint,
) {
    unsafe {
        if nx == 0 || ny == 0 {
            return;
        }

        if px == 0 && nx == (*gd).sx {
            grid_clear_lines(gd, py, ny, bg);
            return;
        }

        if grid_check_y(gd, c!("grid_clear"), py) != 0 {
            return;
        }
        if grid_check_y(gd, c!("grid_clear"), py + ny - 1) != 0 {
            return;
        }

        for yy in py..py + ny {
            let gl = (*gd).linedata.add(yy as usize);

            let mut sx = (*gd).sx;
            if sx > (*gl).cellsize {
                sx = (*gl).cellsize;
            }
            let mut ox = nx;
            if COLOUR_DEFAULT(bg as i32) {
                if px > sx {
                    continue;
                }
                if px + nx > sx {
                    ox = sx - px;
                }
            }

            grid_expand_line(gd, yy, px + ox, 8); // default bg first
            for xx in px..px + ox {
                grid_clear_cell(gd, xx, yy, bg);
            }
        }
    }
}

/// Clear lines. This just frees and truncates the lines.
pub unsafe fn grid_clear_lines(gd: *mut grid, py: c_uint, ny: c_uint, bg: c_uint) {
    unsafe {
        if ny == 0 {
            return;
        }

        if grid_check_y(gd, c!("grid_clear_lines"), py) != 0 {
            return;
        }
        if grid_check_y(gd, c!("grid_clear_lines"), py + ny - 1) != 0 {
            return;
        }

        for yy in py..py + ny {
            grid_free_line(gd, yy);
            grid_empty_line(gd, yy, bg);
        }
        if py != 0 {
            (*(*gd).linedata.add(py as usize - 1)).flags &= !grid_line_flag::WRAPPED;
        }
    }
}

/// Move a group of lines.
pub unsafe fn grid_move_lines(gd: *mut grid, dy: c_uint, py: c_uint, ny: c_uint, bg: c_uint) {
    unsafe {
        if ny == 0 || py == dy {
            return;
        }

        if grid_check_y(gd, c!("grid_move_lines"), py) != 0 {
            return;
        }
        if grid_check_y(gd, c!("grid_move_lines"), py + ny - 1) != 0 {
            return;
        }
        if grid_check_y(gd, c!("grid_move_lines"), dy) != 0 {
            return;
        }
        if grid_check_y(gd, c!("grid_move_lines"), dy + ny - 1) != 0 {
            return;
        }

        // Free any lines which are being replaced
        for yy in dy..dy + ny {
            if yy >= py && yy < py + ny {
                continue;
            }
            grid_free_line(gd, yy);
        }
        if dy != 0 {
            (*(*gd).linedata.add(dy as usize - 1)).flags &= !grid_line_flag::WRAPPED;
        }

        // Move the lines
        let src = (*gd).linedata.add(py as usize);
        let dst = (*gd).linedata.add(dy as usize);
        std::ptr::copy(src, dst, ny as usize);

        // Wipe any lines that have been moved (without freeing them - they are still present)
        for yy in py..py + ny {
            if yy < dy || yy >= dy + ny {
                grid_empty_line(gd, yy, bg);
            }
        }
        if py != 0 && (py < dy || py >= dy + ny) {
            (*(*gd).linedata.add(py as usize - 1)).flags &= !grid_line_flag::WRAPPED;
        }
    }
}

/// Move a group of cells.
pub unsafe fn grid_move_cells(
    gd: *mut grid,
    dx: c_uint,
    px: c_uint,
    py: c_uint,
    nx: c_uint,
    bg: c_uint,
) {
    unsafe {
        if nx == 0 || px == dx {
            return;
        }

        if grid_check_y(gd, c!("grid_move_cells"), py) != 0 {
            return;
        }
        let gl = (*gd).linedata.add(py as usize);

        grid_expand_line(gd, py, px + nx, 8);
        grid_expand_line(gd, py, dx + nx, 8);

        let src = (*gl).celldata.add(px as usize);
        let dst = (*gl).celldata.add(dx as usize);
        std::ptr::copy(src, dst, nx as usize);

        if dx + nx > (*gl).cellused {
            (*gl).cellused = dx + nx;
        }

        // Wipe any cells that have been moved
        for xx in px..px + nx {
            if xx >= dx && xx < dx + nx {
                continue;
            }
            grid_clear_cell(gd, xx, py, bg);
        }
    }
}

/// Get ANSI foreground sequence.
pub unsafe fn grid_string_cells_fg(gc: *const grid_cell, values: *mut c_int) -> usize {
    unsafe {
        let mut n: usize = 0;

        if (*gc).fg & COLOUR_FLAG_256 != 0 {
            *values.add(n) = 38;
            n += 1;
            *values.add(n) = 5;
            n += 1;
            *values.add(n) = ((*gc).fg & 0xff) as c_int;
            n += 1;
        } else if (*gc).fg & COLOUR_FLAG_RGB != 0 {
            *values.add(n) = 38;
            n += 1;
            *values.add(n) = 2;
            n += 1;
            let (r, g, b) = colour_split_rgb((*gc).fg);
            *values.add(n) = r as c_int;
            n += 1;
            *values.add(n) = g as c_int;
            n += 1;
            *values.add(n) = b as c_int;
            n += 1;
        } else {
            match (*gc).fg {
                0..=7 => {
                    *values.add(n) = (*gc).fg + 30;
                    n += 1;
                }
                8 => {
                    *values.add(n) = 39;
                    n += 1;
                }
                90..=97 => {
                    *values.add(n) = (*gc).fg;
                    n += 1;
                }
                _ => {}
            }
        }
        n
    }
}

/// Get ANSI background sequence.
pub unsafe fn grid_string_cells_bg(gc: *const grid_cell, values: *mut c_int) -> usize {
    unsafe {
        let mut n: usize = 0;

        if (*gc).bg & COLOUR_FLAG_256 != 0 {
            *values.add(n) = 48;
            n += 1;
            *values.add(n) = 5;
            n += 1;
            *values.add(n) = ((*gc).bg & 0xff) as c_int;
            n += 1;
        } else if (*gc).bg & COLOUR_FLAG_RGB != 0 {
            *values.add(n) = 48;
            n += 1;
            *values.add(n) = 2;
            n += 1;
            let (r, g, b) = colour_split_rgb((*gc).bg);
            *values.add(n) = r as c_int;
            n += 1;
            *values.add(n) = g as c_int;
            n += 1;
            *values.add(n) = b as c_int;
            n += 1;
        } else {
            match (*gc).bg {
                0..=7 => {
                    *values.add(n) = (*gc).bg + 40;
                    n += 1;
                }
                8 => {
                    *values.add(n) = 49;
                    n += 1;
                }
                90..=97 => {
                    *values.add(n) = (*gc).bg + 10;
                    n += 1;
                }
                _ => {}
            }
        }
        n
    }
}

/// Get underscore colour sequence.
pub unsafe fn grid_string_cells_us(gc: *const grid_cell, values: *mut c_int) -> usize {
    unsafe {
        let mut n: usize = 0;
        if (*gc).us & COLOUR_FLAG_256 != 0 {
            *values.add(n) = 58;
            n += 1;
            *values.add(n) = 5;
            n += 1;
            *values.add(n) = ((*gc).us & 0xff) as c_int;
            n += 1;
        } else if (*gc).us & COLOUR_FLAG_RGB != 0 {
            *values.add(n) = 58;
            n += 1;
            *values.add(n) = 2;
            n += 1;
            let (r, g, b) = colour_split_rgb((*gc).us);
            *values.add(n) = r as c_int;
            n += 1;
            *values.add(n) = g as c_int;
            n += 1;
            *values.add(n) = b as c_int;
            n += 1;
        }
        n
    }
}

/// Add on SGR code.
pub unsafe fn grid_string_cells_add_code(
    buf: *mut u8,
    len: usize,
    n: c_uint,
    s: *mut c_int,
    newc: *mut c_int,
    oldc: *mut c_int,
    nnewc: usize,
    noldc: usize,
    flags: grid_string_flags,
) {
    unsafe {
        let mut tmp: [u8; 64] = [0; 64];
        let reset = n != 0 && *s == 0;

        if nnewc == 0 {
            return; // no code to add
        }
        if !reset
            && nnewc == noldc
            && libc::memcmp(
                newc as *const c_void,
                oldc as *const c_void,
                nnewc * std::mem::size_of::<c_int>(),
            ) == 0
        {
            return; // no reset and colour unchanged
        }
        if reset && (*newc == 49 || *newc == 39) {
            return; // reset and colour default
        }

        if flags.intersects(grid_string_flags::GRID_STRING_ESCAPE_SEQUENCES) {
            strlcat(buf, c!("\\033["), len);
        } else {
            strlcat(buf, c!("\x1b["), len);
        }

        for i in 0..nnewc {
            if i + 1 < nnewc {
                xsnprintf_!(tmp.as_mut_ptr(), tmp.len(), "{};", *newc.add(i));
            } else {
                xsnprintf_!(tmp.as_mut_ptr(), tmp.len(), "{}", *newc.add(i));
            }
            strlcat(buf, tmp.as_ptr(), len);
        }
        strlcat(buf, c!("m"), len);
    }
}

pub unsafe fn grid_string_cells_add_hyperlink(
    buf: *mut u8,
    len: usize,
    id: *const u8,
    uri: *const u8,
    flags: grid_string_flags,
) -> c_int {
    unsafe {
        if strlen(uri) + strlen(id) + 17 >= len {
            return 0;
        }

        if flags.intersects(grid_string_flags::GRID_STRING_ESCAPE_SEQUENCES) {
            strlcat(buf, c!("\\033]8;"), len);
        } else {
            strlcat(buf, c!("\x1b]8;"), len);
        }

        if *id != 0 {
            let tmp = format_nul!("id={};", _s(id));
            strlcat(buf, tmp, len);
            free_(tmp);
        } else {
            strlcat(buf, c!(";"), len);
        }

        strlcat(buf, uri, len);

        if flags.intersects(grid_string_flags::GRID_STRING_ESCAPE_SEQUENCES) {
            strlcat(buf, c!("\\033\\\\"), len);
        } else {
            strlcat(buf, c!("\x1b\\"), len);
        }

        1
    }
}

/// Returns ANSI code to set particular attributes (colour, bold and so on) given a current state.
pub unsafe fn grid_string_cells_code(
    lastgc: *const grid_cell,
    gc: *const grid_cell,
    buf: *mut u8,
    len: usize,
    flags: grid_string_flags,
    sc: *mut screen,
    has_link: *mut c_int,
) {
    unsafe {
        let mut oldc: [c_int; 64] = [0; 64];
        let mut newc: [c_int; 64] = [0; 64];
        let mut s: [c_int; 128] = [0; 128];
        let mut noldc: usize;
        let mut nnewc: usize;
        let mut n: u32 = 0;
        let attr = (*gc).attr;
        let mut lastattr = (*lastgc).attr;
        let mut tmp: [u8; 64] = [0; 64];
        let mut uri: *const u8 = null();
        let mut id: *const u8 = null();

        static ATTRS: [(grid_attr, c_uint); 13] = [
            (grid_attr::GRID_ATTR_BRIGHT, 1),
            (grid_attr::GRID_ATTR_DIM, 2),
            (grid_attr::GRID_ATTR_ITALICS, 3),
            (grid_attr::GRID_ATTR_UNDERSCORE, 4),
            (grid_attr::GRID_ATTR_BLINK, 5),
            (grid_attr::GRID_ATTR_REVERSE, 7),
            (grid_attr::GRID_ATTR_HIDDEN, 8),
            (grid_attr::GRID_ATTR_STRIKETHROUGH, 9),
            (grid_attr::GRID_ATTR_UNDERSCORE_2, 42),
            (grid_attr::GRID_ATTR_UNDERSCORE_3, 43),
            (grid_attr::GRID_ATTR_UNDERSCORE_4, 44),
            (grid_attr::GRID_ATTR_UNDERSCORE_5, 45),
            (grid_attr::GRID_ATTR_OVERLINE, 53),
        ];

        // If any attribute is removed, begin with 0
        for &(mask, _) in ATTRS.iter() {
            if !attr.intersects(mask) && lastattr.intersects(mask)
                || ((*lastgc).us != 8 && (*gc).us == 8)
            {
                s[n as usize] = 0;
                n += 1;
                lastattr &= grid_attr::GRID_ATTR_CHARSET;
                break;
            }
        }

        // For each attribute that is newly set, add its code
        for &(mask, code) in ATTRS.iter() {
            if attr.intersects(mask) && !lastattr.intersects(mask) {
                s[n as usize] = code as c_int;
                n += 1;
            }
        }

        // Write the attributes
        *buf = 0;
        if n > 0 {
            if flags.intersects(grid_string_flags::GRID_STRING_ESCAPE_SEQUENCES) {
                strlcat(buf, c!("\\033["), len);
            } else {
                strlcat(buf, c!("\x1b["), len);
            }

            for i in 0..n {
                if s[i as usize] < 10 {
                    xsnprintf_!(tmp.as_mut_ptr(), tmp.len(), "{}", s[i as usize],);
                } else {
                    xsnprintf_!(
                        tmp.as_mut_ptr(),
                        tmp.len(),
                        "{}:{}",
                        s[i as usize] / 10,
                        s[i as usize] % 10,
                    );
                }
                strlcat(buf, tmp.as_ptr(), len);
                if i + 1 < n {
                    strlcat(buf, c!(";"), len);
                }
            }
            strlcat(buf, c!("m"), len);
        }

        // If the foreground colour changed, write its parameters
        nnewc = grid_string_cells_fg(gc, newc.as_mut_ptr());
        noldc = grid_string_cells_fg(lastgc, oldc.as_mut_ptr());
        grid_string_cells_add_code(
            buf,
            len,
            n,
            s.as_mut_ptr(),
            newc.as_mut_ptr(),
            oldc.as_mut_ptr(),
            nnewc,
            noldc,
            flags,
        );

        // If the background colour changed, append its parameters
        nnewc = grid_string_cells_bg(gc, newc.as_mut_ptr());
        noldc = grid_string_cells_bg(lastgc, oldc.as_mut_ptr());
        grid_string_cells_add_code(
            buf,
            len,
            n,
            s.as_mut_ptr(),
            newc.as_mut_ptr(),
            oldc.as_mut_ptr(),
            nnewc,
            noldc,
            flags,
        );

        // If the underscore colour changed, append its parameters
        nnewc = grid_string_cells_us(gc, newc.as_mut_ptr());
        noldc = grid_string_cells_us(lastgc, oldc.as_mut_ptr());
        grid_string_cells_add_code(
            buf,
            len,
            n,
            s.as_mut_ptr(),
            newc.as_mut_ptr(),
            oldc.as_mut_ptr(),
            nnewc,
            noldc,
            flags,
        );

        // Append shift in/shift out if needed
        if attr.intersects(grid_attr::GRID_ATTR_CHARSET)
            && !lastattr.intersects(grid_attr::GRID_ATTR_CHARSET)
        {
            if flags.intersects(grid_string_flags::GRID_STRING_ESCAPE_SEQUENCES) {
                strlcat(buf, c!("\\016"), len); // SO
            } else {
                strlcat(buf, c!("\x0e"), len); // SO
            }
        }
        if !attr.intersects(grid_attr::GRID_ATTR_CHARSET)
            && lastattr.intersects(grid_attr::GRID_ATTR_CHARSET)
        {
            if flags.intersects(grid_string_flags::GRID_STRING_ESCAPE_SEQUENCES) {
                strlcat(buf, c!("\\017"), len); // SI
            } else {
                strlcat(buf, c!("\x0f"), len); // SI
            }
        }

        // Add hyperlink if changed
        if !sc.is_null() && !(*sc).hyperlinks.is_null() && (*lastgc).link != (*gc).link {
            if hyperlinks_get(
                (*sc).hyperlinks,
                (*gc).link,
                &raw mut uri,
                &raw mut id,
                null_mut(),
            ) {
                *has_link = grid_string_cells_add_hyperlink(buf, len, id, uri, flags);
            } else if *has_link != 0 {
                grid_string_cells_add_hyperlink(buf, len, c!(""), c!(""), flags);
                *has_link = 0;
            }
        }
    }
}

/// Convert cells into a string.
pub unsafe fn grid_string_cells(
    gd: *mut grid,
    px: c_uint,
    py: c_uint,
    nx: c_uint,
    lastgc: *mut *mut grid_cell,
    flags: grid_string_flags,
    s: *mut screen,
) -> *mut u8 {
    static mut LASTGC1: grid_cell = unsafe { zeroed() };
    unsafe {
        let mut gc: grid_cell = zeroed();
        let mut data: *const u8;
        let mut code: [u8; 8192] = [0; 8192];
        let mut len: usize = 128;
        let mut off: usize = 0;
        let mut size: usize = 0;
        let mut codelen: usize;
        let mut has_link: c_int = 0;

        if !lastgc.is_null() && (*lastgc).is_null() {
            std::ptr::copy(&GRID_DEFAULT_CELL, &raw mut LASTGC1, 1);
            *lastgc = &raw mut LASTGC1;
        }

        let mut buf: *mut u8 = xmalloc(len).as_ptr() as *mut u8;

        let gl = grid_peek_line(gd, py);
        let end = if flags.intersects(grid_string_flags::GRID_STRING_EMPTY_CELLS) {
            (*gl).cellsize
        } else {
            (*gl).cellused
        };

        for xx in px..px + nx {
            if gl.is_null() || xx >= end {
                break;
            }
            grid_get_cell(gd, xx, py, &mut gc);
            if gc.flags.intersects(grid_flag::PADDING) {
                continue;
            }

            if flags.intersects(grid_string_flags::GRID_STRING_WITH_SEQUENCES) {
                grid_string_cells_code(
                    *lastgc,
                    &gc,
                    code.as_mut_ptr(),
                    code.len(),
                    flags,
                    s,
                    &raw mut has_link,
                );
                codelen = strlen(code.as_ptr());
                std::ptr::copy(&gc, *lastgc, 1);
            } else {
                codelen = 0;
            }

            data = &raw const gc.data.data as *const u8;
            size = gc.data.size as usize;
            if flags.intersects(grid_string_flags::GRID_STRING_ESCAPE_SEQUENCES)
                && size == 1
                && *data == b'\\'
            {
                data = c!("\\\\");
                size = 2;
            }

            while len < off + size + codelen + 1 {
                buf = xreallocarray(buf.cast(), 2, len).as_ptr() as *mut u8;
                len *= 2;
            }

            if codelen != 0 {
                std::ptr::copy(code.as_ptr(), buf.add(off), codelen);
                off += codelen;
            }
            std::ptr::copy(data, buf.add(off), size);
            off += size;
        }

        if has_link != 0 {
            grid_string_cells_add_hyperlink(code.as_mut_ptr(), code.len(), c!(""), c!(""), flags);
            codelen = strlen(code.as_ptr());
            while len < off + size + codelen + 1 {
                buf = xreallocarray(buf.cast(), 2, len).as_ptr() as *mut u8;
                len *= 2;
            }
            std::ptr::copy(code.as_ptr(), buf.add(off), codelen);
            off += codelen;
        }

        if flags.intersects(grid_string_flags::GRID_STRING_TRIM_SPACES) {
            while off > 0 && *buf.add(off - 1) as u8 == b' ' {
                off -= 1;
            }
        }
        *buf.add(off) = 0;

        buf
    }
}

/// Duplicate a set of lines between two grids. Both source and destination should be big enough.
pub unsafe fn grid_duplicate_lines(
    dst: *mut grid,
    mut dy: c_uint,
    src: *mut grid,
    mut sy: c_uint,
    mut ny: c_uint,
) {
    unsafe {
        if dy + ny > (*dst).hsize + (*dst).sy {
            ny = (*dst).hsize + (*dst).sy - dy;
        }
        if sy + ny > (*src).hsize + (*src).sy {
            ny = (*src).hsize + (*src).sy - sy;
        }
        grid_free_lines(dst, dy, ny);

        for _ in 0..ny {
            let srcl = (*src).linedata.add(sy as usize);
            let dstl = (*dst).linedata.add(dy as usize);

            std::ptr::copy_nonoverlapping(srcl, dstl, 1);
            if (*srcl).cellsize != 0 {
                (*dstl).celldata =
                    xreallocarray_::<grid_cell_entry>(null_mut(), (*srcl).cellsize as usize)
                        .as_ptr();
                std::ptr::copy_nonoverlapping(
                    (*srcl).celldata,
                    (*dstl).celldata,
                    (*srcl).cellsize as usize,
                );
            } else {
                (*dstl).celldata = null_mut();
            }
            if (*srcl).extdsize != 0 {
                (*dstl).extdsize = (*srcl).extdsize;
                (*dstl).extddata =
                    xreallocarray_::<grid_extd_entry>(null_mut(), (*dstl).extdsize as usize)
                        .as_ptr();
                std::ptr::copy_nonoverlapping(
                    (*srcl).extddata,
                    (*dstl).extddata,
                    (*dstl).extdsize as usize,
                );
            } else {
                (*dstl).extddata = null_mut();
            }

            sy += 1;
            dy += 1;
        }
    }
}

/// Mark line as dead.
unsafe fn grid_reflow_dead(gl: *mut grid_line) {
    unsafe {
        std::ptr::write_bytes(gl as *mut u8, 0, std::mem::size_of::<grid_line>());
        (*gl).flags = grid_line_flag::DEAD;
    }
}

/// Add lines, return the first new one.
unsafe fn grid_reflow_add(gd: *mut grid, n: c_uint) -> *mut grid_line {
    unsafe {
        let sy = (*gd).sy + n;

        (*gd).linedata = xreallocarray_((*gd).linedata, sy as usize).as_ptr();
        let gl = (*gd).linedata.add((*gd).sy as usize);
        std::ptr::write_bytes(
            gl as *mut u8,
            0,
            (n as usize) * std::mem::size_of::<grid_line>(),
        );
        (*gd).sy = sy;
        gl
    }
}

/// Move a line across.
unsafe fn grid_reflow_move(gd: *mut grid, from: *mut grid_line) -> *mut grid_line {
    unsafe {
        let to = grid_reflow_add(gd, 1);
        std::ptr::copy_nonoverlapping(from, to, 1);
        grid_reflow_dead(from);
        to
    }
}

/// Join line below onto this one.
unsafe fn grid_reflow_join(
    target: *mut grid,
    gd: *mut grid,
    sx: c_uint,
    yy: c_uint,
    mut width: c_uint,
    already: c_int,
) {
    unsafe {
        let mut from: *mut grid_line = null_mut();
        let mut gc = zeroed();
        let mut lines = 0;
        let mut wrapped = 1;
        let mut want = 0;

        // Add a new target line
        let (to, gl) = if already == 0 {
            let to = (*target).sy;
            let gl = grid_reflow_move(target, (*gd).linedata.add(yy as usize));
            (to, gl)
        } else {
            let to = (*target).sy - 1;
            let gl = (*target).linedata.add(to as usize);
            (to, gl)
        };
        let mut at = (*gl).cellused;

        // Loop until no more to consume or target line is full
        loop {
            // If this is now the last line, nothing more to do
            if yy + 1 + lines == (*gd).hsize + (*gd).sy {
                break;
            }
            let line = yy + 1 + lines;

            // If next line is empty, skip it
            if !(*(*gd).linedata.add(line as usize))
                .flags
                .intersects(grid_line_flag::WRAPPED)
            {
                wrapped = 0;
            }
            if (*(*gd).linedata.add(line as usize)).cellused == 0 {
                if wrapped == 0 {
                    break;
                }
                lines += 1;
                continue;
            }

            // Is destination line now full? Copy first char separately
            grid_get_cell1((*gd).linedata.add(line as usize), 0, &mut gc);
            if width + gc.data.width as u32 > sx {
                break;
            }
            width += gc.data.width as u32;
            grid_set_cell(target, at, to, &gc);
            at += 1;

            // Join as much more as possible onto current line
            from = (*gd).linedata.add(line as usize);
            want = 1;
            while want < (*from).cellused {
                grid_get_cell1(from, want, &mut gc);
                if width + gc.data.width as u32 > sx {
                    break;
                }
                width += gc.data.width as u32;

                grid_set_cell(target, at, to, &gc);
                at += 1;
                want += 1;
            }
            lines += 1;

            // If line wasn't wrapped or we didn't consume entire line,
            // don't try to join further lines
            if wrapped == 0 || want != (*from).cellused || width == sx {
                break;
            }
        }
        if lines == 0 {
            return;
        }

        // If we didn't consume entire final line, remove what we did consume.
        // If we consumed entire line and it wasn't wrapped, remove wrap flag.
        let left = (*from).cellused - want;
        if left != 0 {
            grid_move_cells(gd, 0, want, yy + lines, left, 8);
            (*from).cellsize = left;
            (*from).cellused = left;
            lines -= 1;
        } else if wrapped == 0 {
            (*gl).flags &= !grid_line_flag::WRAPPED;
        }

        // Remove lines that were completely consumed
        for i in (yy + 1)..(yy + 1 + lines) {
            free((*(*gd).linedata.add(i as usize)).celldata.cast());
            free((*(*gd).linedata.add(i as usize)).extddata.cast());
            grid_reflow_dead((*gd).linedata.add(i as usize));
        }

        // Adjust scroll position
        if (*gd).hscrolled > to + lines {
            (*gd).hscrolled -= lines;
        } else if (*gd).hscrolled > to {
            (*gd).hscrolled = to;
        }
    }
}

/// Split this line into several new ones
pub unsafe fn grid_reflow_split(target: *mut grid, gd: *mut grid, sx: u32, yy: u32, at: u32) {
    unsafe {
        let gl = (*gd).linedata.add(yy as usize);
        let mut gc = zeroed();
        let used = (*gl).cellused;
        let flags = (*gl).flags;

        // How many lines do we need to insert? We know we need at least two.
        let lines = if !(*gl).flags.intersects(grid_line_flag::EXTENDED) {
            1 + ((*gl).cellused - 1) / sx
        } else {
            let mut lines = 2;
            let mut width = 0;
            for i in at..used {
                grid_get_cell1(gl, i, &mut gc);
                if width + gc.data.width as u32 > sx {
                    lines += 1;
                    width = 0;
                }
                width += gc.data.width as u32;
            }
            lines
        };

        // Insert new lines
        let mut line = (*target).sy + 1;
        let first = grid_reflow_add(target, lines);

        // Copy sections from original line
        let mut width = 0;
        let mut xx = 0;
        for i in at..used {
            grid_get_cell1(gl, i, &raw mut gc);
            if width + gc.data.width as u32 > sx {
                (*(*target).linedata.add(line as usize)).flags |= grid_line_flag::WRAPPED;

                line += 1;
                width = 0;
                xx = 0;
            }
            width += gc.data.width as u32;
            grid_set_cell(target, xx, line, &gc);
            xx += 1;
        }
        if flags.intersects(grid_line_flag::WRAPPED) {
            (*(*target).linedata.add(line as usize)).flags |= grid_line_flag::WRAPPED;
        }

        // Move remainder of original line
        (*gl).cellsize = at;
        (*gl).cellused = at;
        (*gl).flags |= grid_line_flag::WRAPPED;
        std::ptr::copy_nonoverlapping(gl as *const grid_line, first, 1);
        grid_reflow_dead(gl);

        // Adjust scroll position
        if yy <= (*gd).hscrolled {
            (*gd).hscrolled += lines - 1;
        }

        // If original line had wrapped flag and there is still space in last new line,
        // try to join with next lines
        if width < sx && flags.intersects(grid_line_flag::WRAPPED) {
            grid_reflow_join(target, gd, sx, yy, width, 1);
        }
    }
}

/// Reflow lines on grid to new width
pub unsafe fn grid_reflow(gd: *mut grid, sx: u32) {
    unsafe {
        // Create destination grid - just used as container for line data
        let target = grid_create((*gd).sx, 0, 0);

        // Loop over each source line
        for yy in 0..((*gd).hsize + (*gd).sy) {
            let gl = (*gd).linedata.add(yy as usize);
            if (*gl).flags.intersects(grid_line_flag::DEAD) {
                continue;
            }

            // Work out width of this line. at is point where available width is hit,
            // width is full line width
            let mut at = 0;
            let mut width = 0;
            let mut gc = zeroed();

            if !(*gl).flags.intersects(grid_line_flag::EXTENDED) {
                width = (*gl).cellused;
                if width > sx {
                    at = sx;
                } else {
                    at = width;
                }
            } else {
                for i in 0..(*gl).cellused {
                    grid_get_cell1(gl, i, &mut gc);
                    if at == 0 && width + gc.data.width as u32 > sx {
                        at = i;
                    }
                    width += gc.data.width as u32;
                }
            }

            // If line exactly right, move across unchanged
            if width == sx {
                grid_reflow_move(target, gl);
                continue;
            }

            // If line too big, needs to be split
            if width > sx {
                grid_reflow_split(target, gd, sx, yy, at);
                continue;
            }

            // If line was previously wrapped, join as much as possible of next line
            if (*gl).flags.intersects(grid_line_flag::WRAPPED) {
                grid_reflow_join(target, gd, sx, yy, width, 0);
            } else {
                grid_reflow_move(target, gl);
            }
        }

        // Replace old grid with new
        if (*target).sy < (*gd).sy {
            grid_reflow_add(target, (*gd).sy - (*target).sy);
        }
        (*gd).hsize = (*target).sy - (*gd).sy;
        if (*gd).hscrolled > (*gd).hsize {
            (*gd).hscrolled = (*gd).hsize;
        }
        free((*gd).linedata.cast());
        (*gd).linedata = (*target).linedata;
        free(target.cast());
    }
}

/// Convert to position based on wrapped lines
pub unsafe fn grid_wrap_position(gd: *mut grid, px: u32, py: u32, wx: *mut u32, wy: *mut u32) {
    unsafe {
        let mut ax = 0;
        let mut ay = 0;

        for yy in 0..py {
            if (*(*gd).linedata.add(yy as usize))
                .flags
                .intersects(grid_line_flag::WRAPPED)
            {
                ax += (*(*gd).linedata.add(yy as usize)).cellused;
            } else {
                ax = 0;
                ay += 1;
            }
        }

        if px >= (*(*gd).linedata.add(py as usize)).cellused {
            ax = u32::MAX;
        } else {
            ax += px;
        }
        *wx = ax;
        *wy = ay;
    }
}

/// Convert position based on wrapped lines back
pub unsafe fn grid_unwrap_position(
    gd: *mut grid,
    px: *mut u32,
    py: *mut u32,
    mut wx: u32,
    wy: u32,
) {
    unsafe {
        let mut ay = 0;
        let mut yy = 0;

        while yy < (*gd).hsize + (*gd).sy - 1 {
            if ay == wy {
                break;
            }
            if !(*(*gd).linedata.add(yy as usize))
                .flags
                .intersects(grid_line_flag::WRAPPED)
            {
                ay += 1;
            }
            yy += 1;
        }

        // yy is now 0 on unwrapped line containing wx
        // Walk forwards until we find end or line now containing wx
        if wx == u32::MAX {
            while (*(*gd).linedata.add(yy as usize))
                .flags
                .intersects(grid_line_flag::WRAPPED)
            {
                yy += 1;
            }
            wx = (*(*gd).linedata.add(yy as usize)).cellused;
        } else {
            while (*(*gd).linedata.add(yy as usize))
                .flags
                .intersects(grid_line_flag::WRAPPED)
            {
                if wx < (*(*gd).linedata.add(yy as usize)).cellused {
                    break;
                }
                wx -= (*(*gd).linedata.add(yy as usize)).cellused;
                yy += 1;
            }
        }
        *px = wx;
        *py = yy;
    }
}

/// Get length of line
pub unsafe fn grid_line_length(gd: *mut grid, py: u32) -> u32 {
    unsafe {
        let mut gc = zeroed();
        let mut px = (*grid_get_line(gd, py)).cellsize;
        if px > (*gd).sx {
            px = (*gd).sx;
        }
        while px > 0 {
            grid_get_cell(gd, px - 1, py, &mut gc);
            if (gc.flags.intersects(grid_flag::PADDING))
                || gc.data.size != 1
                || gc.data.data[0] != b' '
            {
                break;
            }
            px -= 1;
        }
        px
    }
}

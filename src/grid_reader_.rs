// Copyright (c) 2020 Anindya Mukherjee <anindya49@hotmail.com>
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

pub unsafe fn grid_reader_start(gr: *mut grid_reader, gd: *mut grid, cx: u32, cy: u32) {
    unsafe {
        (*gr).gd = gd;
        (*gr).cx = cx;
        (*gr).cy = cy;
    }
}

pub unsafe fn grid_reader_get_cursor(gr: *mut grid_reader, cx: *mut u32, cy: *mut u32) {
    unsafe {
        *cx = (*gr).cx;
        *cy = (*gr).cy;
    }
}

pub unsafe fn grid_reader_line_length(gr: *mut grid_reader) -> u32 {
    unsafe { grid_line_length((*gr).gd, (*gr).cy) }
}

pub unsafe fn grid_reader_cursor_right(gr: *mut grid_reader, wrap: u32, all: i32) {
    unsafe {
        let mut gc = MaybeUninit::<grid_cell>::uninit();

        let px = if all != 0 {
            (*(*gr).gd).sx
        } else {
            grid_reader_line_length(gr)
        };

        if wrap != 0 && (*gr).cx >= px && (*gr).cy < (*(*gr).gd).hsize + (*(*gr).gd).sy - 1 {
            grid_reader_cursor_start_of_line(gr, 0);
            grid_reader_cursor_down(gr);
        } else if (*gr).cx < px {
            (*gr).cx += 1;
            while (*gr).cx < px {
                grid_get_cell((*gr).gd, (*gr).cx, (*gr).cy, gc.as_mut_ptr());
                if !(*gc.as_ptr()).flags.intersects(grid_flag::PADDING) {
                    break;
                }
                (*gr).cx += 1;
            }
        }
    }
}

pub unsafe fn grid_reader_cursor_left(gr: *mut grid_reader, wrap: i32) {
    unsafe {
        let mut gc = MaybeUninit::<grid_cell>::uninit();

        while (*gr).cx > 0 {
            grid_get_cell((*gr).gd, (*gr).cx, (*gr).cy, gc.as_mut_ptr());
            if !(*gc.as_ptr()).flags.intersects(grid_flag::PADDING) {
                break;
            }
            (*gr).cx -= 1;
        }
        if (*gr).cx == 0
            && (*gr).cy > 0
            && (wrap != 0
                || (*grid_get_line((*gr).gd, (*gr).cy - 1))
                    .flags
                    .intersects(grid_line_flag::WRAPPED))
        {
            grid_reader_cursor_up(gr);
            grid_reader_cursor_end_of_line(gr, 0, 0);
        } else if (*gr).cx > 0 {
            (*gr).cx -= 1;
        }
    }
}

pub unsafe fn grid_reader_cursor_down(gr: *mut grid_reader) {
    unsafe {
        let mut gc = MaybeUninit::<grid_cell>::uninit();
        let gc = gc.as_mut_ptr();

        if (*gr).cy < (*(*gr).gd).hsize + (*(*gr).gd).sy - 1 {
            (*gr).cy += 1;
        }
        while (*gr).cx > 0 {
            grid_get_cell((*gr).gd, (*gr).cx, (*gr).cy, gc);
            if !(*gc).flags.intersects(grid_flag::PADDING) {
                break;
            }
            (*gr).cx -= 1;
        }
    }
}

pub unsafe fn grid_reader_cursor_up(gr: *mut grid_reader) {
    unsafe {
        let mut gc = MaybeUninit::<grid_cell>::uninit();
        let gc = gc.as_mut_ptr();

        if (*gr).cy > 0 {
            (*gr).cy -= 1;
        }
        while (*gr).cx > 0 {
            grid_get_cell((*gr).gd, (*gr).cx, (*gr).cy, gc);
            if !(*gc).flags.intersects(grid_flag::PADDING) {
                break;
            }
            (*gr).cx -= 1;
        }
    }
}

pub unsafe fn grid_reader_cursor_start_of_line(gr: *mut grid_reader, wrap: i32) {
    unsafe {
        if wrap != 0 {
            while (*gr).cy > 0
                && (*grid_get_line((*gr).gd, (*gr).cy - 1))
                    .flags
                    .intersects(grid_line_flag::WRAPPED)
            {
                (*gr).cy -= 1;
            }
        }
        (*gr).cx = 0;
    }
}

pub unsafe fn grid_reader_cursor_end_of_line(gr: *mut grid_reader, wrap: i32, all: i32) {
    unsafe {
        if wrap != 0 {
            let yy = (*(*gr).gd).hsize + (*(*gr).gd).sy - 1;
            while (*gr).cy < yy
                && (*grid_get_line((*gr).gd, (*gr).cy))
                    .flags
                    .intersects(grid_line_flag::WRAPPED)
            {
                (*gr).cy += 1;
            }
        }
        if all != 0 {
            (*gr).cx = (*(*gr).gd).sx;
        } else {
            (*gr).cx = grid_reader_line_length(gr);
        }
    }
}

pub unsafe fn grid_reader_handle_wrap(gr: *mut grid_reader, xx: *mut u32, yy: *mut u32) -> i32 {
    unsafe {
        while (*gr).cx > *xx {
            if (*gr).cy == *yy {
                return 0;
            }
            grid_reader_cursor_start_of_line(gr, 0);
            grid_reader_cursor_down(gr);

            if (*grid_get_line((*gr).gd, (*gr).cy))
                .flags
                .intersects(grid_line_flag::WRAPPED)
            {
                *xx = (*(*gr).gd).sx - 1;
            } else {
                *xx = grid_reader_line_length(gr);
            }
        }
    }
    1
}

pub unsafe fn grid_reader_in_set(gr: *mut grid_reader, set: *const u8) -> i32 {
    unsafe {
        let mut gc = MaybeUninit::<grid_cell>::uninit();
        let gc = gc.as_mut_ptr();

        grid_get_cell((*gr).gd, (*gr).cx, (*gr).cy, gc);
        if (*gc).flags.intersects(grid_flag::PADDING) {
            return 0;
        }
        utf8_cstrhas(set, &raw mut (*gc).data)
    }
}

pub unsafe fn grid_reader_cursor_next_word(gr: *mut grid_reader, separators: *const u8) {
    unsafe {
        /* Do not break up wrapped words. */
        let mut xx = if (*grid_get_line((*gr).gd, (*gr).cy))
            .flags
            .intersects(grid_line_flag::WRAPPED)
        {
            (*(*gr).gd).sx - 1
        } else {
            grid_reader_line_length(gr)
        };
        let mut yy = (*(*gr).gd).hsize + (*(*gr).gd).sy - 1;

        if grid_reader_handle_wrap(gr, &raw mut xx, &raw mut yy) == 0 {
            return;
        }
        if grid_reader_in_set(gr, WHITESPACE) == 0 {
            if grid_reader_in_set(gr, separators) != 0 {
                loop {
                    (*gr).cx += 1;

                    if !(grid_reader_handle_wrap(gr, &raw mut xx, &raw mut yy) != 0
                        && grid_reader_in_set(gr, separators) != 0
                        && grid_reader_in_set(gr, WHITESPACE) == 0)
                    {
                        break;
                    }
                }
            } else {
                loop {
                    (*gr).cx += 1;
                    if !(grid_reader_handle_wrap(gr, &raw mut xx, &raw mut yy) != 0
                        && (grid_reader_in_set(gr, separators) == 0
                            || grid_reader_in_set(gr, WHITESPACE) != 0))
                    {
                        break;
                    }
                }
            }
        }
        while grid_reader_handle_wrap(gr, &raw mut xx, &raw mut yy) != 0
            && grid_reader_in_set(gr, WHITESPACE) != 0
        {
            (*gr).cx += 1;
        }
    }
}

pub unsafe fn grid_reader_cursor_next_word_end(gr: *mut grid_reader, separators: *const u8) {
    unsafe {
        /* Do not break up wrapped words. */
        let mut xx = if (*grid_get_line((*gr).gd, (*gr).cy))
            .flags
            .intersects(grid_line_flag::WRAPPED)
        {
            (*(*gr).gd).sx - 1
        } else {
            grid_reader_line_length(gr)
        };
        let mut yy = (*(*gr).gd).hsize + (*(*gr).gd).sy - 1;

        while grid_reader_handle_wrap(gr, &raw mut xx, &raw mut yy) != 0 {
            if grid_reader_in_set(gr, WHITESPACE) != 0 {
                (*gr).cx += 1;
            } else if grid_reader_in_set(gr, separators) != 0 {
                loop {
                    (*gr).cx += 1;

                    if !(grid_reader_handle_wrap(gr, &raw mut xx, &raw mut yy) != 0
                        && grid_reader_in_set(gr, separators) != 0
                        && grid_reader_in_set(gr, WHITESPACE) == 0)
                    {
                        break;
                    }
                }
                return;
            } else {
                loop {
                    (*gr).cx += 1;

                    if !(grid_reader_handle_wrap(gr, &raw mut xx, &raw mut yy) != 0
                        && !(grid_reader_in_set(gr, WHITESPACE) != 0
                            || grid_reader_in_set(gr, separators) != 0))
                    {
                        break;
                    }
                }
                return;
            }
        }
    }
}

pub unsafe fn grid_reader_cursor_previous_word(
    gr: *mut grid_reader,
    separators: *const u8,
    already: i32,
    stop_at_eol: i32,
) {
    unsafe {
        // int oldx, oldy, at_eol, word_is_letters;
        let mut oldx: i32;
        let mut oldy: i32;
        let mut at_eol: i32 = 0;
        let word_is_letters;

        if already != 0 || grid_reader_in_set(gr, WHITESPACE) != 0 {
            loop {
                if (*gr).cx > 0 {
                    (*gr).cx -= 1;
                    if grid_reader_in_set(gr, WHITESPACE) == 0 {
                        word_is_letters = !grid_reader_in_set(gr, separators);
                        break;
                    }
                } else {
                    if (*gr).cy == 0 {
                        return;
                    }
                    grid_reader_cursor_up(gr);
                    grid_reader_cursor_end_of_line(gr, 0, 0);

                    if stop_at_eol != 0 && (*gr).cx > 0 {
                        oldx = (*gr).cx as i32;
                        (*gr).cx -= 1;
                        at_eol = grid_reader_in_set(gr, WHITESPACE);
                        (*gr).cx = oldx as u32;
                        if at_eol != 0 {
                            word_is_letters = 0;
                            break;
                        }
                    }
                }
            }
        } else {
            word_is_letters = !grid_reader_in_set(gr, separators);
        }

        let mut oldx;
        let mut oldy;
        loop {
            oldx = (*gr).cx;
            oldy = (*gr).cy;
            if (*gr).cx == 0 {
                if (*gr).cy == 0
                    || (!(*grid_get_line((*gr).gd, (*gr).cy - 1))
                        .flags
                        .intersects(grid_line_flag::WRAPPED))
                {
                    break;
                }
                grid_reader_cursor_up(gr);
                grid_reader_cursor_end_of_line(gr, 0, 1);
            }
            if (*gr).cx > 0 {
                (*gr).cx -= 1;
            }

            if !(grid_reader_in_set(gr, WHITESPACE) == 0
                && word_is_letters != grid_reader_in_set(gr, separators))
            {
                break;
            }
        }
        (*gr).cx = oldx;
        (*gr).cy = oldy;
    }
}

pub unsafe fn grid_reader_cursor_jump(gr: *mut grid_reader, jc: *const utf8_data) -> i32 {
    unsafe {
        let mut gc = MaybeUninit::<grid_cell>::uninit();
        let gc = gc.as_mut_ptr();

        let mut px = (*gr).cx;
        let yy = (*(*gr).gd).hsize + (*(*gr).gd).sy - 1;

        let mut py = (*gr).cy;
        while py <= yy {
            let xx = grid_line_length((*gr).gd, py);
            while px < xx {
                grid_get_cell((*gr).gd, px, py, gc);
                if !(*gc).flags.intersects(grid_flag::PADDING)
                    && (*gc).data.size == (*jc).size
                    && memcmp(
                        (*gc).data.data.as_ptr().cast(),
                        (*jc).data.as_ptr().cast(),
                        (*gc).data.size as usize,
                    ) == 0
                {
                    (*gr).cx = px;
                    (*gr).cy = py;
                    return 1;
                }
                px += 1;
            }

            if py == yy
                || !(*grid_get_line((*gr).gd, py))
                    .flags
                    .intersects(grid_line_flag::WRAPPED)
            {
                return 0;
            }
            px = 0;
            py += 1;
        }
    }
    0
}

pub unsafe fn grid_reader_cursor_jump_back(gr: *mut grid_reader, jc: *mut utf8_data) -> i32 {
    unsafe {
        let mut gc = MaybeUninit::<grid_cell>::uninit();
        let gc = gc.as_mut_ptr();

        let mut xx = (*gr).cx + 1;

        let mut py = (*gr).cy + 1;
        let mut px;
        while py > 0 {
            px = xx;
            while px > 0 {
                grid_get_cell((*gr).gd, px - 1, py - 1, gc);
                if !((*gc).flags.intersects(grid_flag::PADDING)
                    && (*gc).data.size == (*jc).size
                    && memcmp(
                        (*gc).data.data.as_ptr().cast(),
                        (*jc).data.as_ptr().cast(),
                        (*gc).data.size as usize,
                    ) == 0)
                {
                    (*gr).cx = px - 1;
                    (*gr).cy = py - 1;
                    return 1;
                }
                px -= 1;
            }

            if py == 1
                || !(*grid_get_line((*gr).gd, py - 2))
                    .flags
                    .intersects(grid_line_flag::WRAPPED)
            {
                return 0;
            }
            xx = grid_line_length((*gr).gd, py - 2);
            py -= 1;
        }
    }
    0
}

pub unsafe fn grid_reader_cursor_back_to_indentation(gr: *mut grid_reader) {
    unsafe {
        let mut gc = MaybeUninit::<grid_cell>::uninit();
        let gc = gc.as_mut_ptr();
        // u_int px, py, xx, yy, oldx, oldy;

        let yy = (*(*gr).gd).hsize + (*(*gr).gd).sy - 1;
        let oldx = (*gr).cx;
        let oldy = (*gr).cy;
        grid_reader_cursor_start_of_line(gr, 1);

        for py in (*gr).cy..=yy {
            let xx = grid_line_length((*gr).gd, py);
            for px in 0..xx {
                grid_get_cell((*gr).gd, px, py, gc);
                if (*gc).data.size != 1 || (*gc).data.data[0] != b' ' {
                    (*gr).cx = px;
                    (*gr).cy = py;
                    return;
                }
            }
            if !(*grid_get_line((*gr).gd, py))
                .flags
                .intersects(grid_line_flag::WRAPPED)
            {
                break;
            }
        }
        (*gr).cx = oldx;
        (*gr).cy = oldy;
    }
}

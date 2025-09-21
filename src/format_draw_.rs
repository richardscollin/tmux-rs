// Copyright (c) 2019 Nicholas Marriott <nicholas.marriott@gmail.com>
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

/// Format range.
struct format_range {
    index: u32,
    s: *mut screen,
    start: u32,
    end: u32,
    type_: style_range_type,
    argument: u32,
    string: [u8; 16],
    entry: tailq_entry<format_range>,
}
type format_ranges = tailq_head<format_range>;
impl_tailq_entry!(format_range, entry, tailq_entry<format_range>);

/// Does this range match this style?
fn format_is_type(fr: &format_range, sy: &style) -> bool {
    if fr.type_ != sy.range_type {
        return false;
    }

    match fr.type_ {
        style_range_type::STYLE_RANGE_NONE
        | style_range_type::STYLE_RANGE_LEFT
        | style_range_type::STYLE_RANGE_RIGHT => true,
        style_range_type::STYLE_RANGE_PANE
        | style_range_type::STYLE_RANGE_WINDOW
        | style_range_type::STYLE_RANGE_SESSION => fr.argument == sy.range_argument,
        style_range_type::STYLE_RANGE_USER => unsafe {
            libc::strcmp(
                (&raw const fr.string).cast(),
                (&raw const sy.range_string).cast(),
            ) == 0
        },
    }
}

/// Free a range.
unsafe fn format_free_range(frs: *mut format_ranges, fr: *mut format_range) {
    unsafe {
        tailq_remove(frs, fr);
        free(fr.cast());
    }
}

/// Fix range positions.
unsafe fn format_update_ranges(
    frs: *mut format_ranges,
    s: *mut screen,
    offset: u32,
    start: u32,
    width: u32,
) {
    unsafe {
        if frs.is_null() {
            return;
        }

        for fr in tailq_foreach(frs).map(NonNull::as_ptr) {
            if (*fr).s != s {
                continue;
            }

            if (*fr).end <= start || (*fr).start >= start + width {
                format_free_range(frs, fr);
                continue;
            }

            if (*fr).start < start {
                (*fr).start = start;
            }
            if (*fr).end > start + width {
                (*fr).end = start + width;
            }
            if (*fr).start == (*fr).end {
                format_free_range(frs, fr);
                continue;
            }

            (*fr).start -= start;
            (*fr).end -= start;

            (*fr).start += offset;
            (*fr).end += offset;
        }
    }
}

/// Draw a part of the format.
unsafe fn format_draw_put(
    octx: *mut screen_write_ctx,
    ocx: u32,
    ocy: u32,
    s: *mut screen,
    frs: *mut format_ranges,
    offset: u32,
    start: u32,
    width: u32,
) {
    unsafe {
        // The offset is how far from the cursor on the target screen; start
        // and width how much to copy from the source screen.
        screen_write_cursormove(octx, (ocx + offset) as c_int, ocy as c_int, 0);
        screen_write_fast_copy(octx, s, start, 0, width, 1);
        format_update_ranges(frs, s, offset, start, width);
    }
}

/// Draw list part of format.
unsafe fn format_draw_put_list(
    octx: *mut screen_write_ctx,
    ocx: u32,
    ocy: u32,
    mut offset: u32,
    mut width: u32,
    list: *mut screen,
    list_left: *mut screen,
    list_right: *mut screen,
    focus_start: i32,
    focus_end: i32,
    frs: *mut format_ranges,
) {
    unsafe {
        // If there is enough space for the list, draw it entirely.
        if width >= (*list).cx {
            format_draw_put(octx, ocx, ocy, list, frs, offset, 0, width);
            return;
        }

        // The list needs to be trimmed. Try to keep the focus visible.
        let focus_centre: u32 = (focus_start + (focus_end - focus_start) / 2) as u32;
        let mut start: u32 = focus_centre.saturating_sub(width / 2);
        if start + width > (*list).cx {
            start = (*list).cx - width;
        }

        // Draw <> markers at either side if needed.
        if start != 0 && width > (*list_left).cx {
            screen_write_cursormove(octx, (ocx + offset) as c_int, ocy as c_int, 0);
            screen_write_fast_copy(octx, list_left, 0, 0, (*list_left).cx, 1);
            offset += (*list_left).cx;
            start += (*list_left).cx;
            width -= (*list_left).cx;
        }
        if start + width < (*list).cx && width > (*list_right).cx {
            screen_write_cursormove(
                octx,
                (ocx + offset + width - (*list_right).cx) as c_int,
                ocy as c_int,
                0,
            );
            screen_write_fast_copy(octx, list_right, 0, 0, (*list_right).cx, 1);
            width -= (*list_right).cx;
        }

        // Draw the list screen itself.
        format_draw_put(octx, ocx, ocy, list, frs, offset, start, width);
    }
}

/// Draw format with no list.
unsafe fn format_draw_none(
    octx: *mut screen_write_ctx,
    available: u32,
    ocx: u32,
    ocy: u32,
    left: *mut screen,
    centre: *mut screen,
    right: *mut screen,
    abs_centre: *mut screen,
    frs: *mut format_ranges,
) {
    unsafe {
        let mut width_left: u32 = (*left).cx;
        let mut width_centre: u32 = (*centre).cx;
        let mut width_right: u32 = (*right).cx;
        let mut width_abs_centre: u32 = (*abs_centre).cx;

        // Try to keep as much of the left and right as possible at the expense * of the centre.
        while width_left + width_centre + width_right > available {
            if width_centre > 0 {
                width_centre -= 1;
            } else if width_right > 0 {
                width_right -= 1;
            } else {
                width_left -= 1;
            }
        }

        // Write left.
        format_draw_put(octx, ocx, ocy, left, frs, 0, 0, width_left);

        // Write right at available - width_right.
        format_draw_put(
            octx,
            ocx,
            ocy,
            right,
            frs,
            available - width_right,
            (*right).cx - width_right,
            width_right,
        );

        // Write centre halfway between
        //     width_left
        // and
        //     available - width_right.
        format_draw_put(
            octx,
            ocx,
            ocy,
            centre,
            frs,
            width_left + ((available - width_right) - width_left) / 2 - width_centre / 2,
            (*centre).cx / 2 - width_centre / 2,
            width_centre,
        );

        // Write abs_centre in the perfect centre of all horizontal space.
        if width_abs_centre > available {
            width_abs_centre = available;
        }
        format_draw_put(
            octx,
            ocx,
            ocy,
            abs_centre,
            frs,
            (available - width_abs_centre) / 2,
            0,
            width_abs_centre,
        );
    }
}

/// Draw format with list on the left.
unsafe fn format_draw_left(
    octx: *mut screen_write_ctx,
    available: u32,
    ocx: u32,
    ocy: u32,
    left: *mut screen,
    centre: *mut screen,
    right: *mut screen,
    abs_centre: *mut screen,
    list: *mut screen,
    list_left: *mut screen,
    list_right: *mut screen,
    after: *mut screen,
    mut focus_start: i32,
    mut focus_end: i32,
    frs: *mut format_ranges,
) {
    unsafe {
        let mut width_left: u32 = (*left).cx;
        let mut width_centre: u32 = (*centre).cx;
        let mut width_right: u32 = (*right).cx;
        let mut width_abs_centre: u32 = (*abs_centre).cx;
        let mut width_list: u32 = (*list).cx;
        let mut width_after: u32 = (*after).cx;
        let mut ctx: screen_write_ctx = std::mem::zeroed(); // TODO use uninit

        // Trim first the centre, then the list, then the right, then after the
        // list, then the left.
        while width_left + width_centre + width_right + width_list + width_after > available {
            if width_centre > 0 {
                width_centre -= 1;
            } else if width_list > 0 {
                width_list -= 1;
            } else if width_right > 0 {
                width_right -= 1;
            } else if width_after > 0 {
                width_after -= 1;
            } else {
                width_left -= 1;
            }
        }

        // If there is no list left, pass off to the no list function.
        if width_list == 0 {
            screen_write_start(&raw mut ctx, left);
            screen_write_fast_copy(&raw mut ctx, after, 0, 0, width_after, 1);
            screen_write_stop(&raw mut ctx);

            format_draw_none(
                octx, available, ocx, ocy, left, centre, right, abs_centre, frs,
            );
            return;
        }

        // Write left at 0.
        format_draw_put(octx, ocx, ocy, left, frs, 0, 0, width_left);

        // Write right at available - width_right.
        format_draw_put(
            octx,
            ocx,
            ocy,
            right,
            frs,
            available - width_right,
            (*right).cx - width_right,
            width_right,
        );

        // Write after at width_left + width_list.
        format_draw_put(
            octx,
            ocx,
            ocy,
            after,
            frs,
            width_left + width_list,
            0,
            width_after,
        );

        // Write centre halfway between
        //     width_left + width_list + width_after
        // and
        //     available - width_right.
        format_draw_put(
            octx,
            ocx,
            ocy,
            centre,
            frs,
            (width_left + width_list + width_after)
                + ((available - width_right) - (width_left + width_list + width_after)) / 2
                - width_centre / 2,
            (*centre).cx / 2 - width_centre / 2,
            width_centre,
        );

        // The list now goes from
        //     width_left
        // to
        //     width_left + width_list.
        // If there is no focus given, keep the left in focus.
        if focus_start == -1 || focus_end == -1 {
            focus_start = 0;
            focus_end = 0;
        }
        format_draw_put_list(
            octx,
            ocx,
            ocy,
            width_left,
            width_list,
            list,
            list_left,
            list_right,
            focus_start,
            focus_end,
            frs,
        );

        // Write abs_centre in the perfect centre of all horizontal space.
        if width_abs_centre > available {
            width_abs_centre = available;
        }
        format_draw_put(
            octx,
            ocx,
            ocy,
            abs_centre,
            frs,
            (available - width_abs_centre) / 2,
            0,
            width_abs_centre,
        );
    }
}

/// Draw format with list in the centre.
unsafe fn format_draw_centre(
    octx: *mut screen_write_ctx,
    available: u32,
    ocx: u32,
    ocy: u32,
    left: *mut screen,
    centre: *mut screen,
    right: *mut screen,
    abs_centre: *mut screen,
    list: *mut screen,
    list_left: *mut screen,
    list_right: *mut screen,
    after: *mut screen,
    mut focus_start: i32,
    mut focus_end: i32,
    frs: *mut format_ranges,
) {
    unsafe {
        let mut width_left: u32 = (*left).cx;
        let mut width_centre: u32 = (*centre).cx;
        let mut width_right: u32 = (*right).cx;
        let mut width_list: u32 = (*list).cx;
        let mut width_after: u32 = (*after).cx;
        let mut width_abs_centre: u32 = (*abs_centre).cx;
        let mut ctx: screen_write_ctx = std::mem::zeroed(); // TODO use uninit

        // Trim first the list, then after the list, then the centre, then the
        // right, then the left.
        while width_left + width_centre + width_right + width_list + width_after > available {
            if width_list > 0 {
                width_list -= 1;
            } else if width_after > 0 {
                width_after -= 1;
            } else if width_centre > 0 {
                width_centre -= 1;
            } else if width_right > 0 {
                width_right -= 1;
            } else {
                width_left -= 1;
            }
        }

        // If there is no list left, pass off to the no list function.
        if width_list == 0 {
            screen_write_start(&raw mut ctx, centre);
            screen_write_fast_copy(&raw mut ctx, after, 0, 0, width_after, 1);
            screen_write_stop(&raw mut ctx);

            format_draw_none(
                octx, available, ocx, ocy, left, centre, right, abs_centre, frs,
            );
            return;
        }

        // Write left at 0.
        format_draw_put(octx, ocx, ocy, left, frs, 0, 0, width_left);

        // Write right at available - width_right.
        format_draw_put(
            octx,
            ocx,
            ocy,
            right,
            frs,
            available - width_right,
            (*right).cx - width_right,
            width_right,
        );

        // All three centre sections are offset from the middle of the
        // available space.
        let middle = width_left + ((available - width_right) - width_left) / 2;

        // Write centre at
        //     middle - width_list / 2 - width_centre.
        format_draw_put(
            octx,
            ocx,
            ocy,
            centre,
            frs,
            middle - width_list / 2 - width_centre,
            0,
            width_centre,
        );

        // Write after at
        //     middle - width_list / 2 + width_list
        format_draw_put(
            octx,
            ocx,
            ocy,
            after,
            frs,
            middle - width_list / 2 + width_list,
            0,
            width_after,
        );

        // The list now goes from
        //     middle - width_list / 2
        // to
        //     middle + width_list / 2
        // If there is no focus given, keep the centre in focus.
        if focus_start == -1 || focus_end == -1 {
            focus_start = (*list).cx as i32 / 2;
            focus_end = (*list).cx as i32 / 2;
        }
        format_draw_put_list(
            octx,
            ocx,
            ocy,
            middle - width_list / 2,
            width_list,
            list,
            list_left,
            list_right,
            focus_start,
            focus_end,
            frs,
        );

        // Write abs_centre in the perfect centre of all horizontal space.
        if width_abs_centre > available {
            width_abs_centre = available;
        }
        format_draw_put(
            octx,
            ocx,
            ocy,
            abs_centre,
            frs,
            (available - width_abs_centre) / 2,
            0,
            width_abs_centre,
        );
    }
}

// Draw format with list on the right.

unsafe fn format_draw_right(
    octx: *mut screen_write_ctx,
    available: u32,
    ocx: u32,
    ocy: u32,
    left: *mut screen,
    centre: *mut screen,
    right: *mut screen,
    abs_centre: *mut screen,
    list: *mut screen,
    list_left: *mut screen,
    list_right: *mut screen,
    after: *mut screen,
    mut focus_start: i32,
    mut focus_end: i32,
    frs: *mut format_ranges,
) {
    unsafe {
        let mut width_left: u32 = (*left).cx;
        let mut width_centre: u32 = (*centre).cx;
        let mut width_right: u32 = (*right).cx;
        let mut width_list: u32 = (*list).cx;
        let mut width_after: u32 = (*after).cx;
        let mut width_abs_centre: u32 = (*abs_centre).cx;
        let mut ctx: screen_write_ctx = std::mem::zeroed(); // TODO use uninit

        // Trim first the centre, then the list, then the right, then
        // after the list, then the left.
        while width_left + width_centre + width_right + width_list + width_after > available {
            if width_centre > 0 {
                width_centre -= 1;
            } else if width_list > 0 {
                width_list -= 1;
            } else if width_right > 0 {
                width_right -= 1;
            } else if width_after > 0 {
                width_after -= 1;
            } else {
                width_left -= 1;
            }
        }

        // If there is no list left, pass off to the no list function.
        if width_list == 0 {
            screen_write_start(&raw mut ctx, right);
            screen_write_fast_copy(&raw mut ctx, after, 0, 0, width_after, 1);
            screen_write_stop(&raw mut ctx);

            format_draw_none(
                octx, available, ocx, ocy, left, centre, right, abs_centre, frs,
            );
            return;
        }

        // Write left at 0.
        format_draw_put(octx, ocx, ocy, left, frs, 0, 0, width_left);

        // Write after at available - width_after.
        format_draw_put(
            octx,
            ocx,
            ocy,
            after,
            frs,
            available - width_after,
            (*after).cx - width_after,
            width_after,
        );

        // Write right at
        //     available - width_right - width_list - width_after.
        format_draw_put(
            octx,
            ocx,
            ocy,
            right,
            frs,
            available - width_right - width_list - width_after,
            0,
            width_right,
        );

        // Write centre halfway between
        //     width_left
        // and
        //     available - width_right - width_list - width_after.
        format_draw_put(
            octx,
            ocx,
            ocy,
            centre,
            frs,
            width_left + ((available - width_right - width_list - width_after) - width_left) / 2
                - width_centre / 2,
            (*centre).cx / 2 - width_centre / 2,
            width_centre,
        );

        // The list now goes from
        //     available - width_list - width_after
        // to
        //     available - width_after
        // If there is no focus given, keep the right in focus.
        if focus_start == -1 || focus_end == -1 {
            focus_start = 0;
            focus_end = 0;
        }
        format_draw_put_list(
            octx,
            ocx,
            ocy,
            available - width_list - width_after,
            width_list,
            list,
            list_left,
            list_right,
            focus_start,
            focus_end,
            frs,
        );

        // Write abs_centre in the perfect centre of all horizontal space.
        if width_abs_centre > available {
            width_abs_centre = available;
        }
        format_draw_put(
            octx,
            ocx,
            ocy,
            abs_centre,
            frs,
            (available - width_abs_centre) / 2,
            0,
            width_abs_centre,
        );
    }
}

unsafe fn format_draw_absolute_centre(
    octx: *mut screen_write_ctx,
    available: u32,
    ocx: u32,
    ocy: u32,
    left: *mut screen,
    centre: *mut screen,
    right: *mut screen,
    abs_centre: *mut screen,
    list: *mut screen,
    list_left: *mut screen,
    list_right: *mut screen,
    after: *mut screen,
    mut focus_start: i32,
    mut focus_end: i32,
    frs: *mut format_ranges,
) {
    unsafe {
        let mut width_left: u32 = (*left).cx;
        let mut width_centre: u32 = (*centre).cx;
        let mut width_right: u32 = (*right).cx;
        let mut width_abs_centre: u32 = (*abs_centre).cx;
        let mut width_list: u32 = (*list).cx;
        let mut width_after: u32 = (*after).cx;

        // Trim first centre, then the right, then the left.
        while width_left + width_centre + width_right > available {
            if width_centre > 0 {
                width_centre -= 1;
            } else if width_right > 0 {
                width_right -= 1;
            } else {
                width_left -= 1;
            }
        }

        // We trim list after and abs_centre independently, as we are drawing
        // them over the rest. Trim first the list, then after the list, then
        // abs_centre.
        while width_list + width_after + width_abs_centre > available {
            if width_list > 0 {
                width_list -= 1;
            } else if width_after > 0 {
                width_after -= 1;
            } else {
                width_abs_centre -= 1;
            }
        }

        // Write left at 0.
        format_draw_put(octx, ocx, ocy, left, frs, 0, 0, width_left);

        // Write right at available - width_right.
        format_draw_put(
            octx,
            ocx,
            ocy,
            right,
            frs,
            available - width_right,
            (*right).cx - width_right,
            width_right,
        );

        // Keep writing centre at the relative centre. Only the list is written
        // in the absolute centre of the horizontal space.
        let middle = width_left + ((available - width_right) - width_left) / 2;

        // Write centre at
        //     middle - width_centre.
        format_draw_put(
            octx,
            ocx,
            ocy,
            centre,
            frs,
            middle - width_centre,
            0,
            width_centre,
        );

        // If there is no focus given, keep the centre in focus.
        if focus_start == -1 || focus_end == -1 {
            focus_start = (*list).cx as i32 / 2;
            focus_end = (*list).cx as i32 / 2;
        }

        // We centre abs_centre and the list together, so their shared centre is
        // in the perfect centre of horizontal space.
        let mut abs_centre_offset = (available - width_list - width_abs_centre) / 2;

        // Write abs_centre before the list.
        format_draw_put(
            octx,
            ocx,
            ocy,
            abs_centre,
            frs,
            abs_centre_offset,
            0,
            width_abs_centre,
        );
        abs_centre_offset += width_abs_centre;

        // Draw the list in the absolute centre
        format_draw_put_list(
            octx,
            ocx,
            ocy,
            abs_centre_offset,
            width_list,
            list,
            list_left,
            list_right,
            focus_start,
            focus_end,
            frs,
        );
        abs_centre_offset += width_list;

        // Write after at the end of the centre
        format_draw_put(
            octx,
            ocx,
            ocy,
            after,
            frs,
            abs_centre_offset,
            0,
            width_after,
        );
    }
}

/// Get width and count of any leading #s.
unsafe fn format_leading_hashes(cp: *const u8, n: *mut u32, width: *mut u32) -> *const u8 {
    unsafe {
        *n = 0;
        while *cp.add(*n as usize) == b'#' {
            *n += 1;
        }
        if *n == 0 {
            *width = 0;
            return cp;
        }
        if *cp.add(*n as usize) != b'[' {
            if (*n).is_multiple_of(2) {
                *width = *n / 2;
            } else {
                *width = *n / 2 + 1;
            }
            return cp.add(*n as usize);
        }
        *width = *n / 2;
        if (*n).is_multiple_of(2) {
            // An even number of #s means that all #s are escaped, so not a
            // style. The caller should not skip this. Return pointing to
            // the [.
            return cp.add(*n as usize);
        }
        // This is a style, so return pointing to the #.
        cp.add(*n as usize - 1)
    }
}

/// Draw multiple characters.
unsafe fn format_draw_many(ctx: *mut screen_write_ctx, sy: *mut style, ch: u8, n: u32) {
    unsafe {
        utf8_set(&raw mut (*sy).gc.data, ch);
        for _ in 0..n {
            screen_write_cell(ctx, &raw mut (*sy).gc);
        }
    }
}

/// Draw a format to a screen.
pub unsafe fn format_draw(
    octx: *mut screen_write_ctx,
    base: *const grid_cell,
    available: c_uint,
    expanded: *const u8,
    srs: *mut style_ranges,
    default_colours: c_int,
) {
    let func = "format_draw";
    let mut __func__ = c!("format_draw");
    unsafe {
        #[derive(Copy, Clone, Eq, PartialEq)]
        #[repr(u32)]
        enum Current {
            Left,
            Centre,
            Right,
            AbsoluteCentre,
            List,
            ListLeft,
            ListRight,
            After,
        }
        const TOTAL: usize = Current::After as usize + 1;

        let mut current = Current::Left;
        let mut last = Current::Left;

        static NAMES: [&str; TOTAL] = [
            "LEFT",
            "CENTRE",
            "RIGHT",
            "ABSOLUTE_CENTRE",
            "LIST",
            "LIST_LEFT",
            "LIST_RIGHT",
            "AFTER",
        ];

        let size = libc::strlen(expanded) as u32;
        let os: *mut screen = (*octx).s;
        let mut s: [screen; TOTAL] = zeroed();

        let mut ctx: [screen_write_ctx; TOTAL] = zeroed();
        let ocx: u32 = (*os).cx;
        let ocy: u32 = (*os).cy;
        let mut width: [u32; TOTAL] = [0; TOTAL];

        let mut map: [Current; 5] = [
            Current::Left,
            Current::Left,
            Current::Centre,
            Current::Right,
            Current::AbsoluteCentre,
        ];

        let mut focus_start: i32 = -1;
        let mut focus_end: i32 = -1;
        let mut list_state: i32 = -1;
        let mut fill = -1;
        let mut list_align = style_align::STYLE_ALIGN_DEFAULT;

        let mut gc: grid_cell = zeroed();
        let mut current_default: grid_cell = zeroed();
        let mut sy: style = zeroed();
        let mut saved_sy: style = zeroed();

        let ud: *mut utf8_data = &raw mut sy.gc.data;

        let mut fr = null_mut();
        let mut frs: format_ranges = zeroed();

        memcpy__(&raw mut current_default, base);
        style_set(&raw mut sy, &raw mut current_default);
        tailq_init(&raw mut frs);
        // log_debug("%s: %s", __func__, expanded);

        // We build three screens for left, right, centre alignment, one for
        // the list, one for anything after the list and two for the list left
        // and right markers.
        for i in 0..TOTAL {
            screen_init(&raw mut s[i], size, 1, 0);
            screen_write_start(&raw mut ctx[i], &raw mut s[i]);
            screen_write_clearendofline(&raw mut ctx[i], current_default.bg as u32);
            width[i] = 0;
        }

        'out: {
            // Walk the string and add to the corresponding screens,
            // parsing styles as we go.
            let mut cp = expanded;
            while *cp != b'\0' {
                // Handle sequences of #.
                if *cp == b'#' && *cp.add(1) != b'[' && *cp.add(1) != b'\0' {
                    let mut n: u32 = 1;
                    while *cp.add(n as usize) == b'#' {
                        n += 1;
                    }
                    let even = n.is_multiple_of(2);
                    if *cp.add(n as usize) != b'[' {
                        cp = cp.add(n as usize);
                        n = n.div_ceil(2);
                        width[current as usize] += n;
                        format_draw_many(&raw mut ctx[current as usize], &raw mut sy, b'#', n);
                        continue;
                    }
                    cp = cp.add(if even { n as usize + 1 } else { n as usize - 1 });
                    if sy.ignore != 0 {
                        continue;
                    }
                    format_draw_many(&raw mut ctx[current as usize], &raw mut sy, b'#', n / 2);
                    width[current as usize] += n / 2;
                    if even {
                        utf8_set(ud, b'[');
                        screen_write_cell(&raw mut ctx[current as usize], &raw mut sy.gc);
                        width[current as usize] += 1;
                    }
                    continue;
                }

                // Is this not a style?
                if *cp != b'#' || *cp.add(1) != b'[' || sy.ignore != 0 {
                    // See if this is a UTF-8 character.
                    let mut more = utf8_open(ud, *cp);
                    if more == utf8_state::UTF8_MORE {
                        while ({
                            cp = cp.add(1);
                            *cp != b'\0'
                        }) && more == utf8_state::UTF8_MORE
                        {
                            more = utf8_append(ud, *cp);
                        }
                        if more != utf8_state::UTF8_DONE {
                            cp = cp.wrapping_sub((*ud).have as usize);
                        }
                    }

                    // Not a UTF-8 character - ASCII or not valid.
                    if more != utf8_state::UTF8_DONE {
                        if *cp < 0x20 || *cp > 0x7e {
                            // Ignore nonprintable characters.
                            cp = cp.add(1);
                            continue;
                        }
                        utf8_set(ud, *cp);
                        cp = cp.add(1);
                    }

                    // Draw the cell to the current screen.
                    screen_write_cell(&raw mut ctx[current as u32 as usize], &raw mut sy.gc);
                    width[current as usize] += (*ud).width as u32;
                    continue;
                }

                // This is a style. Work out where the end is and parse it.
                let end = format_skip(cp.add(2), c!("]"));
                if end.is_null() {
                    // log_debug("%s: no terminating ] at '%s'", __func__, cp + 2);
                    for fr_ in tailq_foreach(&raw mut frs).map(NonNull::as_ptr) {
                        fr = fr_;
                        // TODO warning this seems to break the aliasing rules
                        format_free_range(&raw mut frs, fr);
                    }
                    break 'out;
                }
                let tmp: *mut u8 = xstrndup(cp.add(2), end.offset_from(cp.add(2)) as usize)
                    .as_ptr()
                    .cast();
                style_copy(&raw mut saved_sy, &raw const sy);
                if style_parse(&raw mut sy, &raw mut current_default, tmp) != 0 {
                    log_debug!("{}: invalid style '{}'", func, _s(tmp));
                    free_(tmp);
                    cp = end.add(1);
                    continue;
                }
                log_debug!(
                    "{}: style '{}' -> '{}'",
                    func,
                    _s(tmp),
                    _s(style_tostring(&raw const sy))
                );
                free_(tmp);
                if default_colours != 0 {
                    sy.gc.bg = (*base).bg;
                    sy.gc.fg = (*base).fg;
                }

                // If this style has a fill colour, store it for later.
                if sy.fill != 8 {
                    fill = sy.fill;
                }

                // If this style pushed or popped the default, update it.
                if sy.default_type == style_default_type::STYLE_DEFAULT_PUSH {
                    memcpy__(&raw mut current_default, &raw const saved_sy.gc);
                    sy.default_type = style_default_type::STYLE_DEFAULT_BASE;
                } else if sy.default_type == style_default_type::STYLE_DEFAULT_POP {
                    memcpy__(&raw mut current_default, base);
                    sy.default_type = style_default_type::STYLE_DEFAULT_BASE;
                }

                // Check the list state.
                match sy.list {
                    style_list::STYLE_LIST_ON => {
                        // Entering the list, exiting a marker, or exiting the
                        // focus.
                        if list_state != 0 {
                            if !fr.is_null() {
                                // abort any region
                                free_(fr);
                                fr = null_mut();
                            }
                            list_state = 0;
                            list_align = sy.align;
                        }

                        // End the focus if started.
                        if focus_start != -1 && focus_end == -1 {
                            focus_end = s[Current::List as usize].cx as i32;
                        }

                        current = Current::List;
                    }
                    style_list::STYLE_LIST_FOCUS => {
                        // Entering the focus.
                        if list_state != 0 {
                            break;
                        } /* not inside the list */
                        if focus_start == -1 {
                            focus_start = s[Current::List as usize].cx as i32;
                        } /* focus already started */
                    }
                    style_list::STYLE_LIST_OFF => {
                        // Exiting or outside the list.
                        if list_state == 0 {
                            if !fr.is_null() {
                                // abort any region
                                free_(fr);
                                fr = null_mut();
                            }
                            if focus_start != -1 && focus_end == -1 {
                                focus_end = s[Current::List as usize].cx as i32;
                            }

                            map[list_align as usize] = Current::After;
                            if list_align == style_align::STYLE_ALIGN_LEFT {
                                map[style_align::STYLE_ALIGN_DEFAULT as usize] = Current::After;
                            }
                            list_state = 1;
                        }
                        current = map[sy.align as usize];
                    }
                    style_list::STYLE_LIST_LEFT_MARKER => {
                        // Entering left marker.
                        if list_state != 0 {
                            break;
                        } /* not inside the list */
                        if s[Current::ListLeft as usize].cx != 0 {
                            break;
                        } /* already have marker */
                        if !fr.is_null() {
                            // abort any region
                            free_(fr);
                            fr = null_mut();
                        }
                        if focus_start != -1 && focus_end == -1 {
                            focus_start = -1;
                            focus_end = -1;
                        }
                        current = Current::ListLeft;
                    }
                    style_list::STYLE_LIST_RIGHT_MARKER => {
                        // note conditions are flipped from original c source because of break

                        if list_state == 0 && s[Current::ListRight as usize].cx == 0 {
                            if !fr.is_null() {
                                // abort any region
                                free_(fr);
                                fr = null_mut();
                            }
                            if focus_start != -1 && focus_end == -1 {
                                focus_start = -1;
                                focus_end = -1;
                            }
                            current = Current::ListRight;
                        }
                    }
                }

                if current != last {
                    log_debug!(
                        "{}: change {} -> {}",
                        func,
                        NAMES[last as usize],
                        NAMES[current as usize]
                    );
                    last = current;
                }

                // Check if the range style has changed and if so end the
                // current range and start a new one if needed.
                if !srs.is_null() {
                    if !fr.is_null() && !format_is_type(&*fr, &sy) {
                        if s[current as usize].cx != (*fr).start {
                            (*fr).end = s[current as usize].cx + 1;
                            tailq_insert_tail(&raw mut frs, fr);
                        } else {
                            free_(fr);
                        }
                        fr = null_mut();
                    }
                    if fr.is_null() && sy.range_type != style_range_type::STYLE_RANGE_NONE {
                        fr = xcalloc_(1).as_ptr();
                        (*fr).index = current as u32;

                        (*fr).s = &raw mut s[current as usize];
                        (*fr).start = s[current as usize].cx;

                        (*fr).type_ = sy.range_type;
                        (*fr).argument = sy.range_argument;
                        strlcpy(
                            (*fr).string.as_mut_ptr(),
                            sy.range_string.as_ptr(),
                            size_of::<[u8; 16]>(),
                        );
                    }
                }

                cp = end.add(1);
            }
            free_(fr);

            for i in 0..TOTAL {
                screen_write_stop(&raw mut ctx[i]);
                log_debug!("{}: width {} is {}", func, NAMES[i], width[i]);
            }
            if focus_start != -1 && focus_end != -1 {
                log_debug!("{}: focus {}-{}", func, focus_start, focus_end);
            }
            for fr in tailq_foreach(&raw mut frs).map(NonNull::as_ptr) {
                log_debug!(
                    "{}: range {}|{} is {} {}-{}",
                    func,
                    (*fr).type_ as u32,
                    (*fr).argument,
                    NAMES[(*fr).index as usize],
                    (*fr).start,
                    (*fr).end
                );
            }

            // Clear the available area.
            if fill != -1 {
                memcpy__(&raw mut gc, &raw const GRID_DEFAULT_CELL);
                gc.bg = fill;
                for _ in 0..available {
                    screen_write_putc(octx, &raw mut gc, b' ');
                }
            }

            // Draw the screens. How they are arranged depends on where the list appears.
            match list_align {
                // No list.
                style_align::STYLE_ALIGN_DEFAULT => format_draw_none(
                    octx,
                    available,
                    ocx,
                    ocy,
                    &raw mut s[Current::Left as usize],
                    &raw mut s[Current::Centre as usize],
                    &raw mut s[Current::Right as usize],
                    &raw mut s[Current::AbsoluteCentre as usize],
                    &raw mut frs,
                ),
                // List is part of the left.
                style_align::STYLE_ALIGN_LEFT => format_draw_left(
                    octx,
                    available,
                    ocx,
                    ocy,
                    &raw mut s[Current::Left as usize],
                    &raw mut s[Current::Centre as usize],
                    &raw mut s[Current::Right as usize],
                    &raw mut s[Current::AbsoluteCentre as usize],
                    &raw mut s[Current::List as usize],
                    &raw mut s[Current::ListLeft as usize],
                    &raw mut s[Current::ListRight as usize],
                    &raw mut s[Current::After as usize],
                    focus_start,
                    focus_end,
                    &raw mut frs,
                ),
                // List is part of the centre.
                style_align::STYLE_ALIGN_CENTRE => format_draw_centre(
                    octx,
                    available,
                    ocx,
                    ocy,
                    &raw mut s[Current::Left as usize],
                    &raw mut s[Current::Centre as usize],
                    &raw mut s[Current::Right as usize],
                    &raw mut s[Current::AbsoluteCentre as usize],
                    &raw mut s[Current::List as usize],
                    &raw mut s[Current::ListLeft as usize],
                    &raw mut s[Current::ListRight as usize],
                    &raw mut s[Current::After as usize],
                    focus_start,
                    focus_end,
                    &raw mut frs,
                ),
                // List is part of the right.
                style_align::STYLE_ALIGN_RIGHT => format_draw_right(
                    octx,
                    available,
                    ocx,
                    ocy,
                    &raw mut s[Current::Left as usize],
                    &raw mut s[Current::Centre as usize],
                    &raw mut s[Current::Right as usize],
                    &raw mut s[Current::AbsoluteCentre as usize],
                    &raw mut s[Current::List as usize],
                    &raw mut s[Current::ListLeft as usize],
                    &raw mut s[Current::ListRight as usize],
                    &raw mut s[Current::After as usize],
                    focus_start,
                    focus_end,
                    &raw mut frs,
                ),
                // List is in the centre of the entire horizontal space.
                style_align::STYLE_ALIGN_ABSOLUTE_CENTRE => format_draw_absolute_centre(
                    octx,
                    available,
                    ocx,
                    ocy,
                    &raw mut s[Current::Left as usize],
                    &raw mut s[Current::Centre as usize],
                    &raw mut s[Current::Right as usize],
                    &raw mut s[Current::AbsoluteCentre as usize],
                    &raw mut s[Current::List as usize],
                    &raw mut s[Current::ListLeft as usize],
                    &raw mut s[Current::ListRight as usize],
                    &raw mut s[Current::After as usize],
                    focus_start,
                    focus_end,
                    &raw mut frs,
                ),
            }

            // Create ranges to return.
            for fr in tailq_foreach(&mut frs).map(NonNull::as_ptr) {
                let sr = xcalloc1::<style_range>();
                sr.type_ = (*fr).type_;
                sr.argument = (*fr).argument;
                strlcpy(
                    sr.string.as_mut_ptr(),
                    (*fr).string.as_ptr(),
                    size_of::<[u8; 16]>(),
                );
                sr.start = (*fr).start;
                sr.end = (*fr).end;
                tailq_insert_tail(srs, sr);

                match sr.type_ {
                    style_range_type::STYLE_RANGE_NONE => (),
                    style_range_type::STYLE_RANGE_LEFT => {
                        log_debug!("{}: range left at {}-{}", func, sr.start, sr.end);
                    }
                    style_range_type::STYLE_RANGE_RIGHT => {
                        log_debug!("{}: range right at {}-{}", func, sr.start, sr.end);
                    }
                    style_range_type::STYLE_RANGE_PANE => {
                        log_debug!(
                            "{}: range pane|%%{} at {}-{}",
                            func,
                            sr.argument,
                            sr.start,
                            sr.end
                        );
                    }
                    style_range_type::STYLE_RANGE_WINDOW => {
                        log_debug!(
                            "{}: range window|{} at {}-{}",
                            func,
                            sr.argument,
                            sr.start,
                            sr.end
                        );
                    }
                    style_range_type::STYLE_RANGE_SESSION => {
                        log_debug!(
                            "{}: range session|${} at {}-{}",
                            func,
                            sr.argument,
                            sr.start,
                            sr.end
                        );
                    }
                    style_range_type::STYLE_RANGE_USER => {
                        log_debug!(
                            "{}: range user|{} at {}-{}",
                            func,
                            sr.argument,
                            sr.start,
                            sr.end
                        );
                    }
                }
                format_free_range(&raw mut frs, fr);
            }
        } // out:

        // Free the screens.
        for s_i in s.iter_mut() {
            screen_free(s_i);
        }

        // Restore the original cursor position.
        screen_write_cursormove(octx, ocx as i32, ocy as i32, 0);
    }
}

/// Get width, taking #[] into account.
pub unsafe fn format_width(expanded: *const u8) -> u32 {
    unsafe {
        let mut cp: *const u8 = expanded;

        let mut n: u32 = 0;
        let mut leading_width: u32 = 0;
        let mut width: u32 = 0;

        let mut ud: utf8_data = zeroed();

        while *cp != b'\0' {
            if *cp == b'#' {
                let mut end = format_leading_hashes(cp, &raw mut n, &raw mut leading_width);
                width += leading_width;
                cp = end;
                if *cp == b'#' {
                    end = format_skip(cp.add(2), c!("]"));
                    if end.is_null() {
                        return 0;
                    }
                    cp = end.add(1);
                }
            } else if let mut more = utf8_open(&raw mut ud, *cp)
                && more == utf8_state::UTF8_MORE
            {
                while ({
                    cp = cp.add(1);
                    *cp != b'\0'
                } && more == utf8_state::UTF8_MORE)
                {
                    more = utf8_append(&raw mut ud, *cp);
                }
                if more == utf8_state::UTF8_DONE {
                    width += ud.width as u32;
                } else {
                    cp = cp.wrapping_sub(ud.have as usize);
                }
            } else if *cp > 0x1f && *cp < 0x7f {
                width += 1;
                cp = cp.add(1);
            } else {
                cp = cp.add(1);
            }
        }
        width
    }
}

/// Get width, taking #[] into account.
pub unsafe fn format_width_(expanded: &str) -> u32 {
    // I ensured the one call to this function str was nul terminated
    unsafe { format_width(expanded.as_ptr()) }
}

/// Trim on the left, taking #[] into account.
///
/// Note, we copy the whole set of unescaped #s, but only add their escaped size to width.
/// This is because the format_draw function will actually do the escaping when it runs
pub unsafe fn format_trim_left(expanded: *const u8, limit: u32) -> *mut u8 {
    unsafe {
        let mut cp = expanded;

        let mut n: u32 = 0;
        let mut width: u32 = 0;
        let mut leading_width: u32 = 0;

        let mut ud: utf8_data = zeroed();

        let mut out: *mut u8 = xcalloc(2, strlen(expanded) + 1).as_ptr().cast();
        let copy = out;

        while *cp != b'\0' {
            if width >= limit {
                break;
            }
            if *cp == b'#' {
                let mut end = format_leading_hashes(cp, &raw mut n, &raw mut leading_width);
                if leading_width > limit - width {
                    leading_width = limit - width;
                }
                if leading_width != 0 {
                    if n == 1 {
                        *out = b'#';
                        out = out.add(1);
                    } else {
                        libc::memset(out.cast(), b'#' as i32, 2 * leading_width as usize);
                        out = out.add(2 * leading_width as usize);
                    }
                    width += leading_width;
                }
                cp = end;
                if *cp == b'#' {
                    end = format_skip(cp.add(2), c!("]"));
                    if end.is_null() {
                        break;
                    }
                    libc::memcpy(out.cast(), cp.cast(), end.add(1).offset_from(cp) as usize);
                    out = out.offset(end.add(1).offset_from(cp));
                    cp = end.add(1);
                }
            } else if let mut more = utf8_open(&raw mut ud, *cp)
                && more == utf8_state::UTF8_MORE
            {
                while ({
                    cp = cp.add(1);
                    *cp != b'\0'
                }) && more == utf8_state::UTF8_MORE
                {
                    more = utf8_append(&raw mut ud, *cp);
                }
                if more == utf8_state::UTF8_DONE {
                    if width + ud.width as u32 <= limit {
                        libc::memcpy(out.cast(), ud.data.as_ptr().cast(), ud.size as usize);
                        out = out.add(ud.size as usize);
                    }
                    width += ud.width as u32;
                } else {
                    cp = cp.wrapping_sub(ud.have as usize).add(1);
                }
            } else if *cp > 0x1f && *cp < 0x7f {
                if width < limit {
                    *out = *cp;
                    out = out.add(1);
                }
                width += 1;
                cp = cp.add(1);
            } else {
                cp = cp.add(1);
            }
        }
        *out = b'\0';
        copy
    }
}

/// Trim on the right, taking #[] into account.
pub unsafe fn format_trim_right(expanded: *const u8, limit: u32) -> *mut u8 {
    unsafe {
        let mut ud: utf8_data = std::mem::zeroed();

        let mut width: u32 = 0;
        let mut n: u32 = 0;

        let mut leading_width: u32 = 0;
        let mut copy_width: u32;

        let mut cp = expanded;

        let total_width: u32 = format_width(expanded);
        if total_width <= limit {
            return xstrdup(expanded).as_ptr();
        }
        let skip: u32 = total_width - limit;

        let mut out: *mut u8 = xcalloc(2, strlen(expanded) + 1).as_ptr().cast();
        let copy: *mut u8 = out;
        while *cp != b'\0' {
            if *cp == b'#' {
                let mut end: *const u8 =
                    format_leading_hashes(cp, &raw mut n, &raw mut leading_width);
                copy_width = leading_width;
                if width <= skip {
                    if skip - width >= copy_width {
                        copy_width = 0;
                    } else {
                        copy_width -= skip - width;
                    }
                }
                if copy_width != 0 {
                    if n == 1 {
                        *out = b'#';
                        out = out.add(1);
                    } else {
                        libc::memset(out.cast(), b'#' as i32, 2 * copy_width as usize);
                        out = out.add(2 * copy_width as usize);
                    }
                }
                width += leading_width;
                cp = end;
                if *cp == b'#' {
                    end = format_skip(cp.add(2), c!("]"));
                    if end.is_null() {
                        break;
                    }
                    libc::memcpy(out.cast(), cp.cast(), end.add(1).offset_from(cp) as usize);
                    out = out.offset(end.add(1).offset_from(cp));
                    cp = end.add(1);
                }
            } else if let mut more = utf8_open(&raw mut ud, *cp)
                && more == utf8_state::UTF8_MORE
            {
                while ({
                    cp = cp.add(1);
                    *(cp) != b'\0'
                }) && more == utf8_state::UTF8_MORE
                {
                    more = utf8_append(&raw mut ud, *cp);
                }
                if more == utf8_state::UTF8_DONE {
                    if width >= skip {
                        libc::memcpy(out.cast(), ud.data.as_ptr().cast(), ud.size as usize);
                        out = out.add(ud.size as usize);
                    }
                    width += ud.width as u32;
                } else {
                    cp = cp.wrapping_sub(ud.have as usize).add(1);
                }
            } else if *cp > 0x1f && *cp < 0x7f {
                if width >= skip {
                    *out = *cp;
                    out = out.add(1);
                }
                width += 1;
                cp = cp.add(1);
            } else {
                cp = cp.add(1);
            }
        }
        *out = b'\0';
        copy
    }
}

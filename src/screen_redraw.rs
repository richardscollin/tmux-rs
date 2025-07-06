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

const START_ISOLATE: *const u8 = b"\xe2\x81\xa6\0".as_ptr().cast();
const END_ISOLATE: *const u8 = b"\xe2\x81\xa9\0".as_ptr().cast();

/* Border in relation to a pane. */
#[repr(i32)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum screen_redraw_border_type {
    SCREEN_REDRAW_OUTSIDE,
    SCREEN_REDRAW_INSIDE,
    SCREEN_REDRAW_BORDER_LEFT,
    SCREEN_REDRAW_BORDER_RIGHT,
    SCREEN_REDRAW_BORDER_TOP,
    SCREEN_REDRAW_BORDER_BOTTOM,
}
const BORDER_MARKERS: [u8; 6] = [b' ', b' ', b'+', b',', b'.', b'-'];

/// Get cell border character.
pub unsafe fn screen_redraw_border_set(
    w: *mut window,
    wp: *mut window_pane,
    pane_lines: pane_lines,
    cell_type: cell_type,
    gc: *mut grid_cell,
) {
    unsafe {
        let mut idx: u32 = 0;

        if cell_type == CELL_OUTSIDE && !(*w).fill_character.is_null() {
            utf8_copy(&mut (*gc).data, (*w).fill_character);
            return;
        }

        match pane_lines {
            pane_lines::PANE_LINES_NUMBER => {
                if cell_type == CELL_OUTSIDE {
                    (*gc).attr |= grid_attr::GRID_ATTR_CHARSET;
                    utf8_set(&mut (*gc).data, CELL_BORDERS[CELL_OUTSIDE as usize]);
                    return;
                }
                (*gc).attr &= !grid_attr::GRID_ATTR_CHARSET;
                if !wp.is_null() && window_pane_index(wp, &raw mut idx) == 0 {
                    utf8_set(&mut (*gc).data, b'0' + ((idx % 10) as u8));
                } else {
                    utf8_set(&mut (*gc).data, b'*');
                }
            }
            pane_lines::PANE_LINES_DOUBLE => {
                (*gc).attr &= !grid_attr::GRID_ATTR_CHARSET;
                utf8_copy(&mut (*gc).data, tty_acs_double_borders(cell_type));
            }
            pane_lines::PANE_LINES_HEAVY => {
                (*gc).attr &= !grid_attr::GRID_ATTR_CHARSET;
                utf8_copy(&mut (*gc).data, tty_acs_heavy_borders(cell_type));
            }
            pane_lines::PANE_LINES_SIMPLE => {
                (*gc).attr &= !grid_attr::GRID_ATTR_CHARSET;
                utf8_set(&mut (*gc).data, SIMPLE_BORDERS[cell_type as usize]);
            }
            _ => {
                (*gc).attr |= grid_attr::GRID_ATTR_CHARSET;
                utf8_set(&mut (*gc).data, CELL_BORDERS[cell_type as usize]);
            }
        }
    }
}

/// Return if window has only two panes.
pub unsafe fn screen_redraw_two_panes(w: *mut window, direction: i32) -> i32 {
    unsafe {
        let wp: *mut window_pane =
            tailq_next::<_, _, discr_entry>(tailq_first(&raw mut (*w).panes));
        if wp.is_null() {
            return 0; /* one pane */
        }
        if !tailq_next::<_, _, discr_entry>(wp).is_null() {
            return 0; /* more than two panes */
        }
        if direction == 0 && (*wp).xoff == 0 {
            return 0;
        }
        if direction == 1 && (*wp).yoff == 0 {
            return 0;
        }
    }
    1
}

/// Check if cell is on the border of a pane.
pub unsafe fn screen_redraw_pane_border(
    ctx: *mut screen_redraw_ctx,
    wp: *mut window_pane,
    px: u32,
    py: u32,
) -> screen_redraw_border_type {
    unsafe {
        let oo = (*(*wp).window).options;
        let mut split = 0;
        let ex = (*wp).xoff + (*wp).sx;
        let ey = (*wp).yoff + (*wp).sy;
        let pane_status = (*ctx).pane_status;

        // Inside pane
        if px >= (*wp).xoff && px < ex && py >= (*wp).yoff && py < ey {
            return screen_redraw_border_type::SCREEN_REDRAW_INSIDE;
        }

        // Get pane indicator
        match pane_border_indicator::try_from(
            options_get_number_(oo, c"pane-border-indicators") as i32
        ) {
            Ok(pane_border_indicator::PANE_BORDER_COLOUR)
            | Ok(pane_border_indicator::PANE_BORDER_BOTH) => {
                split = 1;
            }
            _ => (),
        }

        // Left/right borders
        if pane_status == pane_status::PANE_STATUS_OFF {
            if screen_redraw_two_panes((*wp).window, 0) != 0 && split != 0 {
                if (*wp).xoff == 0 && px == (*wp).sx && py <= (*wp).sy / 2 {
                    return screen_redraw_border_type::SCREEN_REDRAW_BORDER_RIGHT;
                }
                if (*wp).xoff != 0 && px == (*wp).xoff - 1 && py > (*wp).sy / 2 {
                    return screen_redraw_border_type::SCREEN_REDRAW_BORDER_LEFT;
                }
            } else if ((*wp).yoff == 0 || py >= (*wp).yoff - 1) && py <= ey {
                if (*wp).xoff != 0 && px == (*wp).xoff - 1 {
                    return screen_redraw_border_type::SCREEN_REDRAW_BORDER_LEFT;
                }
                if px == ex {
                    return screen_redraw_border_type::SCREEN_REDRAW_BORDER_RIGHT;
                }
            }
        } else if ((*wp).yoff == 0 || py >= (*wp).yoff - 1) && py <= ey {
            if (*wp).xoff != 0 && px == (*wp).xoff - 1 {
                return screen_redraw_border_type::SCREEN_REDRAW_BORDER_LEFT;
            }
            if px == ex {
                return screen_redraw_border_type::SCREEN_REDRAW_BORDER_RIGHT;
            }
        }

        // Top/bottom borders
        if pane_status == pane_status::PANE_STATUS_OFF {
            if screen_redraw_two_panes((*wp).window, 1) != 0 && split != 0 {
                if (*wp).yoff == 0 && py == (*wp).sy && px <= (*wp).sx / 2 {
                    return screen_redraw_border_type::SCREEN_REDRAW_BORDER_BOTTOM;
                }
                if (*wp).yoff != 0 && py == (*wp).yoff - 1 && px > (*wp).sx / 2 {
                    return screen_redraw_border_type::SCREEN_REDRAW_BORDER_TOP;
                }
            } else if ((*wp).xoff == 0 || px >= (*wp).xoff - 1) && px <= ex {
                if (*wp).yoff != 0 && py == (*wp).yoff - 1 {
                    return screen_redraw_border_type::SCREEN_REDRAW_BORDER_TOP;
                }
                if py == ey {
                    return screen_redraw_border_type::SCREEN_REDRAW_BORDER_BOTTOM;
                }
            }
        } else if pane_status == pane_status::PANE_STATUS_TOP {
            if ((*wp).xoff == 0 || px >= (*wp).xoff - 1)
                && px <= ex
                && (*wp).yoff != 0
                && py == (*wp).yoff - 1
            {
                return screen_redraw_border_type::SCREEN_REDRAW_BORDER_TOP;
            }
        } else if ((*wp).xoff == 0 || px >= (*wp).xoff - 1) && px <= ex && py == ey {
            return screen_redraw_border_type::SCREEN_REDRAW_BORDER_BOTTOM;
        }

        // Outside pane
        screen_redraw_border_type::SCREEN_REDRAW_OUTSIDE
    }
}

/// Check if a cell is on a border.
pub unsafe fn screen_redraw_cell_border(ctx: *mut screen_redraw_ctx, px: u32, py: u32) -> i32 {
    unsafe {
        let c = (*ctx).c;
        let w = (*(*(*c).session).curw).window;

        // Outside the window?
        if px > (*w).sx || py > (*w).sy {
            return 0;
        }

        // On the window border?
        if px == (*w).sx || py == (*w).sy {
            return 1;
        }

        // Check all the panes
        let mut result = 0;
        for wp in tailq_foreach::<_, discr_entry>(&raw mut (*w).panes).map(NonNull::as_ptr) {
            if window_pane_visible(wp) == 0 {
                continue;
            }

            match screen_redraw_pane_border(ctx, wp, px, py) {
                screen_redraw_border_type::SCREEN_REDRAW_INSIDE => {
                    result = 0;
                    break;
                }
                screen_redraw_border_type::SCREEN_REDRAW_OUTSIDE => {}
                _ => {
                    result = 1;
                    break;
                }
            }
        }

        result
    }
}

/// Work out type of border cell from surrounding cells.
pub unsafe fn screen_redraw_type_of_cell(
    ctx: *mut screen_redraw_ctx,
    px: u32,
    py: u32,
) -> cell_type {
    unsafe {
        let c = (*ctx).c;
        let pane_status = (*ctx).pane_status;
        let w = (*(*(*c).session).curw).window;
        let sx = (*w).sx;
        let sy = (*w).sy;
        let mut borders = 0;

        // Is this outside the window?
        if px > sx || py > sy {
            return CELL_OUTSIDE;
        }

        // Construct a bitmask of whether the cells to the left (bit 4), right,
        // top, and bottom (bit 1) of this cell are borders.
        if px == 0 || screen_redraw_cell_border(ctx, px - 1, py) != 0 {
            borders |= 8;
        }
        if px <= sx && screen_redraw_cell_border(ctx, px + 1, py) != 0 {
            borders |= 4;
        }
        match pane_status {
            pane_status::PANE_STATUS_TOP => {
                if py != 0 && screen_redraw_cell_border(ctx, px, py - 1) != 0 {
                    borders |= 2;
                }
                if screen_redraw_cell_border(ctx, px, py + 1) != 0 {
                    borders |= 1;
                }
            }
            pane_status::PANE_STATUS_BOTTOM => {
                if py == 0 || screen_redraw_cell_border(ctx, px, py - 1) != 0 {
                    borders |= 2;
                }
                if py != sy - 1 && screen_redraw_cell_border(ctx, px, py + 1) != 0 {
                    borders |= 1;
                }
            }
            _ => {
                if py == 0 || screen_redraw_cell_border(ctx, px, py - 1) != 0 {
                    borders |= 2;
                }
                if screen_redraw_cell_border(ctx, px, py + 1) != 0 {
                    borders |= 1;
                }
            }
        }

        // Figure out what kind of border this cell is. Only one bit set
        // doesn't make sense (can't have a border cell with no others
        // connected).
        match borders {
            15 => CELL_JOIN,        // 1111, left right top bottom
            14 => CELL_BOTTOMJOIN,  // 1110, left right top
            13 => CELL_TOPJOIN,     // 1101, left right bottom
            12 => CELL_LEFTRIGHT,   // 1100, left right
            11 => CELL_RIGHTJOIN,   // 1011, left top bottom
            10 => CELL_BOTTOMRIGHT, // 1010, left top
            9 => CELL_TOPRIGHT,     // 1001, left bottom
            7 => CELL_LEFTJOIN,     // 0111, right top bottom
            6 => CELL_BOTTOMLEFT,   // 0110, right top
            5 => CELL_TOPLEFT,      // 0101, right bottom
            3 => CELL_TOPBOTTOM,    // 0011, top bottom
            _ => CELL_OUTSIDE,
        }
    }
}

/// Check if cell inside a pane.
pub unsafe fn screen_redraw_check_cell(
    ctx: *mut screen_redraw_ctx,
    px: u32,
    py: u32,
    wpp: *mut *mut window_pane,
) -> cell_type {
    unsafe {
        let c = (*ctx).c;
        let w = (*(*(*c).session).curw).window;
        let mut wp: *mut window_pane;
        let mut active: *mut window_pane;
        let pane_status = (*ctx).pane_status;
        let mut border: i32;
        let mut right: u32;
        let mut line: u32;

        *wpp = null_mut();

        if px > (*w).sx || py > (*w).sy {
            return CELL_OUTSIDE;
        }
        if px == (*w).sx || py == (*w).sy {
            /* window border */
            return screen_redraw_type_of_cell(ctx, px, py);
        }

        if pane_status != pane_status::PANE_STATUS_OFF {
            wp = server_client_get_pane(c);
            active = wp;
            loop {
                'next1: {
                    if window_pane_visible(wp) == 0 {
                        break 'next1;
                    }

                    if pane_status == pane_status::PANE_STATUS_TOP {
                        line = (*wp).yoff - 1;
                    } else {
                        line = (*wp).yoff + (*wp).sy;
                    }
                    right = (*wp).xoff + 2 + (*wp).status_size as u32 - 1;

                    if py == line && px >= (*wp).xoff + 2 && px <= right {
                        return CELL_INSIDE;
                    }
                }
                // next1
                wp = tailq_next::<_, _, discr_entry>(wp);
                if wp.is_null() {
                    wp = tailq_first(&raw mut (*w).panes);
                }
                if wp == active {
                    break;
                }
            }
        }

        wp = server_client_get_pane(c);
        active = wp;
        loop {
            'next2: {
                if window_pane_visible(wp) == 0 {
                    break 'next2;
                }
                *wpp = wp;

                // If definitely inside, return. If not on border, skip.
                // Otherwise work out the cell.
                border = screen_redraw_pane_border(ctx, wp, px, py) as i32;
                if border == screen_redraw_border_type::SCREEN_REDRAW_INSIDE as i32 {
                    return CELL_INSIDE;
                }
                if border == screen_redraw_border_type::SCREEN_REDRAW_OUTSIDE as i32 {
                    break 'next2;
                }
                return screen_redraw_type_of_cell(ctx, px, py);
            }
            // next2
            wp = tailq_next::<_, _, discr_entry>(wp);
            if wp.is_null() {
                wp = tailq_first(&raw mut (*w).panes);
            }
            if wp == active {
                break;
            }
        }

        CELL_OUTSIDE
    }
}

/// Check if the border of a particular pane.
pub unsafe fn screen_redraw_check_is(
    ctx: *mut screen_redraw_ctx,
    px: u32,
    py: u32,
    wp: *mut window_pane,
) -> i32 {
    unsafe {
        let border = screen_redraw_pane_border(ctx, wp, px, py);
        if border != screen_redraw_border_type::SCREEN_REDRAW_INSIDE
            && border != screen_redraw_border_type::SCREEN_REDRAW_OUTSIDE
        {
            return 1;
        }
        0
    }
}

/// Update pane status.
pub unsafe fn screen_redraw_make_pane_status(
    c: *mut client,
    wp: NonNull<window_pane>,
    rctx: *mut screen_redraw_ctx,
    pane_lines: pane_lines,
) -> i32 {
    unsafe {
        let w = (*wp.as_ptr()).window;
        let mut gc: grid_cell = std::mem::zeroed();
        let width: u32;
        let mut px: u32;
        let mut py: u32;
        let mut ctx: MaybeUninit<screen_write_ctx> = MaybeUninit::uninit();
        let mut old: MaybeUninit<screen> = MaybeUninit::uninit();
        let pane_status = (*rctx).pane_status;

        let ft = format_create(
            c,
            null_mut(),
            (FORMAT_PANE | (*wp.as_ptr()).id) as i32,
            format_flags::FORMAT_STATUS,
        );
        format_defaults(
            ft,
            c,
            NonNull::new((*c).session),
            NonNull::new((*(*c).session).curw),
            Some(wp),
        );

        if wp.as_ptr() == server_client_get_pane(c) {
            style_apply(&mut gc, (*w).options, c!("pane-active-border-style"), ft);
        } else {
            style_apply(&mut gc, (*w).options, c!("pane-border-style"), ft);
        }
        let wp = wp.as_ptr();
        let fmt = options_get_string_((*wp).options, c"pane-border-format");

        let expanded = format_expand_time(ft, fmt);
        if (*wp).sx < 4 {
            (*wp).status_size = 0;
            width = 0;
        } else {
            (*wp).status_size = (*wp).sx as usize - 4;
            width = (*wp).sx - 4;
        }

        memcpy__(old.as_mut_ptr(), &raw const (*wp).status_screen);
        screen_init(&raw mut (*wp).status_screen, width, 1, 0);
        (*wp).status_screen.mode = mode_flag::empty();

        screen_write_start(ctx.as_mut_ptr(), &raw mut (*wp).status_screen);

        for i in 0..width {
            px = (*wp).xoff + 2 + i;
            if pane_status == pane_status::PANE_STATUS_TOP {
                py = (*wp).yoff - 1;
            } else {
                py = (*wp).yoff + (*wp).sy;
            }
            let cell_type = screen_redraw_type_of_cell(rctx, px, py);
            screen_redraw_border_set(w, wp, pane_lines, cell_type, &raw mut gc);
            screen_write_cell(ctx.as_mut_ptr(), &raw const gc);
        }
        gc.attr &= !grid_attr::GRID_ATTR_CHARSET;

        screen_write_cursormove(ctx.as_mut_ptr(), 0, 0, 0);
        format_draw(
            ctx.as_mut_ptr(),
            &raw mut gc,
            width,
            expanded,
            null_mut(),
            0,
        );
        screen_write_stop(ctx.as_mut_ptr());

        free_(expanded);
        format_free(ft);

        if grid_compare((*wp).status_screen.grid, (*old.as_mut_ptr()).grid) == 0 {
            screen_free(old.as_mut_ptr());
            return 0;
        }
        screen_free(old.as_mut_ptr());
        1
    }
}

/// Draw pane status.
pub unsafe fn screen_redraw_draw_pane_status(ctx: *mut screen_redraw_ctx) {
    unsafe {
        let c = (*ctx).c;
        let w = (*(*(*c).session).curw).window;
        let tty = &raw mut (*c).tty;
        log_debug!(
            "{}: {} @{}",
            "screen_redraw_draw_pane_status",
            _s((*c).name),
            (*w).id,
        );

        for wp in tailq_foreach::<_, discr_entry>(&raw mut (*w).panes).map(NonNull::as_ptr) {
            if window_pane_visible(wp) == 0 {
                continue;
            }
            let s = &raw mut (*wp).status_screen;

            let size: u32 = (*wp).status_size as u32;
            let mut yoff = if (*ctx).pane_status == pane_status::PANE_STATUS_TOP {
                (*wp).yoff - 1
            } else {
                (*wp).yoff + (*wp).sy
            };
            let xoff = (*wp).xoff + 2;

            if xoff + size <= (*ctx).ox
                || xoff >= (*ctx).ox + (*ctx).sx
                || yoff < (*ctx).oy
                || yoff >= (*ctx).oy + (*ctx).sy
            {
                continue;
            }

            let (i, x, width) = if xoff >= (*ctx).ox && xoff + size <= (*ctx).ox + (*ctx).sx {
                // All visible
                (0, xoff - (*ctx).ox, size)
            } else if xoff < (*ctx).ox && xoff + size > (*ctx).ox + (*ctx).sx {
                // Both left and right not visible
                ((*ctx).ox, 0, (*ctx).sx)
            } else if xoff < (*ctx).ox {
                // Left not visible
                ((*ctx).ox - xoff, 0, size - ((*ctx).ox - xoff))
            } else {
                // Right not visible
                (0, xoff - (*ctx).ox, size - (xoff - (*ctx).ox))
            };

            if (*ctx).statustop != 0 {
                yoff += (*ctx).statuslines;
            }
            tty_draw_line(
                tty,
                s,
                i,
                0,
                width,
                x,
                yoff - (*ctx).oy,
                &raw const grid_default_cell,
                null_mut(),
            );
        }
        tty_cursor(tty, 0, 0);
    }
}

/// Update status line and change flags if unchanged.
unsafe fn screen_redraw_update(c: *mut client, mut flags: client_flag) -> client_flag {
    unsafe {
        let w = (*(*(*c).session).curw).window;
        let wo = (*w).options;
        let mut ctx = MaybeUninit::<screen_redraw_ctx>::uninit();

        let mut redraw = if !(*c).message_string.is_null() {
            status_message_redraw(c)
        } else if !(*c).prompt_string.is_null() {
            status_prompt_redraw(c)
        } else {
            status_redraw(c)
        };

        if redraw == 0 && !flags.intersects(client_flag::REDRAWSTATUSALWAYS) {
            flags &= !client_flag::REDRAWSTATUS;
        }

        if (*c).overlay_draw.is_some() {
            flags |= client_flag::REDRAWOVERLAY;
        }

        if options_get_number(wo, c!("pane-border-status")) as i32
            != pane_status::PANE_STATUS_OFF as i32
        {
            screen_redraw_set_context(c, ctx.as_mut_ptr());
            let lines = pane_lines::try_from(options_get_number_(wo, c"pane-border-lines") as i32)
                .unwrap_or_default();
            redraw = 0;

            // Safe replacement for TAILQ_FOREACH macro
            let mut wp = (*w).panes.tqh_first;
            while !wp.is_null() {
                if screen_redraw_make_pane_status(
                    c,
                    NonNull::new_unchecked(wp),
                    ctx.as_mut_ptr(),
                    lines,
                ) != 0
                {
                    redraw = 1;
                }
                wp = (*wp).entry.tqe_next;
            }

            if redraw != 0 {
                flags |= client_flag::REDRAWBORDERS;
            }
        }
        flags
    }
}

/// Set up redraw context.
pub unsafe fn screen_redraw_set_context(c: *mut client, ctx: *mut screen_redraw_ctx) {
    unsafe {
        let s = (*c).session;
        let oo = (*s).options;
        let w = (*(*s).curw).window;
        let wo = (*w).options;

        // Zero out context
        memset0(ctx);
        (*ctx).c = c;

        let mut lines = status_line_size(c);
        if !(*c).message_string.is_null() || !(*c).prompt_string.is_null() {
            lines = if lines == 0 { 1 } else { lines };
        }
        if lines != 0 && options_get_number_(oo, c"status-position") == 0 {
            (*ctx).statustop = 1;
        }
        (*ctx).statuslines = lines;

        (*ctx).pane_status = (options_get_number_(wo, c"pane-border-status") as i32)
            .try_into()
            .unwrap();
        (*ctx).pane_lines = (options_get_number_(wo, c"pane-border-lines") as i32)
            .try_into()
            .unwrap();

        tty_window_offset(
            &raw mut (*c).tty,
            &raw mut (*ctx).ox,
            &raw mut (*ctx).oy,
            &raw mut (*ctx).sx,
            &raw mut (*ctx).sy,
        );

        log_debug!(
            "{}: {} @{} ox={} oy={} sx={} sy={} {}/{}",
            "screen_redraw_set_context",
            _s((*c).name),
            (*w).id,
            (*ctx).ox,
            (*ctx).oy,
            (*ctx).sx,
            (*ctx).sy,
            (*ctx).statuslines,
            (*ctx).statustop,
        );
    }
}

/// Redraw entire screen.
pub unsafe fn screen_redraw_screen(c: *mut client) {
    unsafe {
        let mut ctx = MaybeUninit::<screen_redraw_ctx>::uninit();
        let ctx = ctx.as_mut_ptr();

        if (*c).flags.intersects(client_flag::SUSPENDED) {
            return;
        }

        let flags = screen_redraw_update(c, (*c).flags);
        if !flags.intersects(CLIENT_ALLREDRAWFLAGS) {
            return;
        }

        screen_redraw_set_context(c, ctx);
        tty_sync_start(&raw mut (*c).tty);
        tty_update_mode(&raw mut (*c).tty, (*c).tty.mode, null_mut());

        if flags.intersects(client_flag::REDRAWWINDOW | client_flag::REDRAWBORDERS) {
            log_debug!("{}: redrawing borders", _s((*c).name));
            if (*ctx).pane_status != pane_status::PANE_STATUS_OFF {
                screen_redraw_draw_pane_status(ctx);
            }
            screen_redraw_draw_borders(ctx);
        }
        if flags.intersects(client_flag::REDRAWWINDOW) {
            log_debug!("{}: redrawing panes", _s((*c).name));
            screen_redraw_draw_panes(ctx);
        }
        if (*ctx).statuslines != 0
            && flags.intersects(client_flag::REDRAWSTATUS | client_flag::REDRAWSTATUSALWAYS)
        {
            log_debug!("{}: redrawing status", _s((*c).name));
            screen_redraw_draw_status(ctx);
        }
        if let Some(overlay_draw) = (*c).overlay_draw
            && flags.intersects(client_flag::REDRAWOVERLAY)
        {
            log_debug!("{}: redrawing overlay", _s((*c).name));
            overlay_draw(c, (*c).overlay_data, ctx);
        }

        tty_reset(&raw mut (*c).tty);
    }
}

/// Redraw a single pane.
pub unsafe fn screen_redraw_pane(c: *mut client, wp: *mut window_pane) {
    unsafe {
        let mut ctx = MaybeUninit::<screen_redraw_ctx>::uninit();

        if window_pane_visible(wp) == 0 {
            return;
        }

        screen_redraw_set_context(c, ctx.as_mut_ptr());
        tty_sync_start(&raw mut (*c).tty);
        tty_update_mode(&raw mut (*c).tty, (*c).tty.mode, null_mut());

        screen_redraw_draw_pane(ctx.as_mut_ptr(), wp);

        tty_reset(&raw mut (*c).tty);
    }
}

/// Get border cell style.
pub unsafe fn screen_redraw_draw_borders_style(
    ctx: *mut screen_redraw_ctx,
    x: u32,
    y: u32,
    wp: *mut window_pane,
) -> *const grid_cell {
    unsafe {
        let c = (*ctx).c;
        let s = (*c).session;
        let w = (*(*s).curw).window;
        let active = server_client_get_pane(c);
        let oo = (*w).options;

        if (*wp).border_gc_set != 0 {
            return &raw const (*wp).border_gc;
        }
        (*wp).border_gc_set = 1;

        let ft = format_create_defaults(null_mut(), c, s, (*s).curw, wp);
        if screen_redraw_check_is(ctx, x, y, active) != 0 {
            style_apply(
                &raw mut (*wp).border_gc,
                oo,
                c!("pane-active-border-style"),
                ft,
            );
        } else {
            style_apply(&raw mut (*wp).border_gc, oo, c!("pane-border-style"), ft);
        }
        format_free(ft);

        &raw const (*wp).border_gc
    }
}

/// Draw a border cell.
pub unsafe fn screen_redraw_draw_borders_cell(ctx: *mut screen_redraw_ctx, i: u32, j: u32) {
    unsafe {
        let c = (*ctx).c;
        let s = (*c).session;
        let w = (*(*s).curw).window;
        let oo = (*w).options;
        let tty = &raw mut (*c).tty;
        let active = server_client_get_pane(c);
        let mut gc: grid_cell = zeroed();
        let mut arrows = 0;
        let border;
        let x = (*ctx).ox + i;
        let y = (*ctx).oy + j;

        if let Some(overlay_check) = (*c).overlay_check {
            let mut r: overlay_ranges = zeroed();
            overlay_check(c, (*c).overlay_data, x, y, 1, &raw mut r);
            if r.nx[0] + r.nx[1] == 0 {
                return;
            }
        }

        let mut wp = null_mut();
        let cell_type = screen_redraw_check_cell(ctx, x, y, &raw mut wp);
        if cell_type == CELL_INSIDE {
            return;
        }

        if wp.is_null() {
            if (*ctx).no_pane_gc_set == 0 {
                let ft = format_create_defaults(null_mut(), c, s, (*s).curw, null_mut());
                memcpy__(&raw mut (*ctx).no_pane_gc, &raw const grid_default_cell);
                style_add(&raw mut (*ctx).no_pane_gc, oo, c!("pane-border-style"), ft);
                format_free(ft);
                (*ctx).no_pane_gc_set = 1;
            }
            memcpy__(&raw mut gc, &raw const (*ctx).no_pane_gc);
        } else {
            let tmp = screen_redraw_draw_borders_style(ctx, x, y, wp);
            if tmp.is_null() {
                return;
            }
            memcpy__(&raw mut gc, tmp);

            if server_is_marked(s, (*s).curw, marked_pane.wp)
                && screen_redraw_check_is(ctx, x, y, marked_pane.wp) != 0
            {
                gc.attr ^= grid_attr::GRID_ATTR_REVERSE;
            }
        }
        screen_redraw_border_set(w, wp, (*ctx).pane_lines, cell_type, &raw mut gc);

        let isolates = cell_type == CELL_TOPBOTTOM
            && (*c).flags.intersects(client_flag::UTF8)
            && tty_term_has((*tty).term, tty_code_code::TTYC_BIDI);

        if (*ctx).statustop != 0 {
            tty_cursor(tty, i, (*ctx).statuslines + j);
        } else {
            tty_cursor(tty, i, j);
        }
        if isolates {
            tty_puts(tty, END_ISOLATE);
        }

        match pane_border_indicator::try_from(
            options_get_number_(oo, c"pane-border-indicators") as i32
        ) {
            Ok(pane_border_indicator::PANE_BORDER_ARROWS)
            | Ok(pane_border_indicator::PANE_BORDER_BOTH) => arrows = 1,
            _ => {}
        }

        if !wp.is_null() && arrows != 0 {
            border = screen_redraw_pane_border(ctx, active, x, y);
            if ((i == (*wp).xoff + 1
                && (cell_type == CELL_LEFTRIGHT
                    || (cell_type == CELL_TOPJOIN
                        && border == screen_redraw_border_type::SCREEN_REDRAW_BORDER_BOTTOM)
                    || (cell_type == CELL_BOTTOMJOIN
                        && border == screen_redraw_border_type::SCREEN_REDRAW_BORDER_TOP)))
                || (j == (*wp).yoff + 1
                    && (cell_type == CELL_TOPBOTTOM
                        || (cell_type == CELL_LEFTJOIN
                            && border == screen_redraw_border_type::SCREEN_REDRAW_BORDER_RIGHT)
                        || (cell_type == CELL_RIGHTJOIN
                            && border == screen_redraw_border_type::SCREEN_REDRAW_BORDER_LEFT))))
                && screen_redraw_check_is(ctx, x, y, active) != 0
            {
                gc.attr |= grid_attr::GRID_ATTR_CHARSET;
                utf8_set(&raw mut gc.data, BORDER_MARKERS[border as usize]);
            }
        }

        tty_cell(tty, &raw mut gc, &grid_default_cell, null_mut(), null_mut());
        if isolates {
            tty_puts(tty, START_ISOLATE);
        }
    }
}

/// Draw the borders.
pub unsafe fn screen_redraw_draw_borders(ctx: *mut screen_redraw_ctx) {
    unsafe {
        let c = (*ctx).c;
        let s = (*c).session;
        let w = (*(*s).curw).window;

        log_debug!(
            "{}: {} @{}",
            "screen_redraw_draw_borders",
            _s((*c).name),
            (*w).id,
        );

        for wp in tailq_foreach::<_, discr_entry>(&raw mut (*w).panes).map(NonNull::as_ptr) {
            (*wp).border_gc_set = 0;
        }

        for j in 0..(*c).tty.sy - (*ctx).statuslines {
            for i in 0..(*c).tty.sx {
                screen_redraw_draw_borders_cell(ctx, i, j);
            }
        }
    }
}

/// Draw the panes.
pub unsafe fn screen_redraw_draw_panes(ctx: *mut screen_redraw_ctx) {
    unsafe {
        let c = (*ctx).c;
        let w = (*(*(*c).session).curw).window;

        log_debug!(
            "{}: {} @{}",
            "screen_redraw_draw_panes",
            _s((*c).name),
            (*w).id
        );

        for wp in tailq_foreach::<_, discr_entry>(&raw mut (*w).panes).map(NonNull::as_ptr) {
            if window_pane_visible(wp) != 0 {
                screen_redraw_draw_pane(ctx, wp);
            }
        }
    }
}

/// Draw the status line.
pub unsafe fn screen_redraw_draw_status(ctx: *mut screen_redraw_ctx) {
    unsafe {
        let c = (*ctx).c;
        let w = (*(*(*c).session).curw).window;
        let tty = &raw mut (*c).tty;
        let s = (*c).status.active;

        log_debug!(
            "{}: {} @{}",
            "screen_redraw_draw_status",
            _s((*c).name),
            (*w).id
        );

        let y = if (*ctx).statustop != 0 {
            0
        } else {
            (*c).tty.sy - (*ctx).statuslines
        };

        for i in 0..(*ctx).statuslines {
            tty_draw_line(
                tty,
                s,
                0,
                i,
                u32::MAX,
                0,
                y + i,
                &grid_default_cell,
                null_mut(),
            );
        }
    }
}

/// Draw one pane.
pub unsafe fn screen_redraw_draw_pane(ctx: *mut screen_redraw_ctx, wp: *mut window_pane) {
    unsafe {
        let c = (*ctx).c;
        let w = (*(*(*c).session).curw).window;
        let tty = &raw mut (*c).tty;
        let s = (*wp).screen;
        let palette = &raw mut (*wp).palette;
        let mut defaults: grid_cell = zeroed();

        log_debug!(
            "{}: {} @{} %%{}",
            "screen_redraw_draw_pane",
            _s((*c).name),
            (*w).id,
            (*wp).id,
        );

        if (*wp).xoff + (*wp).sx <= (*ctx).ox || (*wp).xoff >= (*ctx).ox + (*ctx).sx {
            return;
        }

        let top = if (*ctx).statustop != 0 {
            (*ctx).statuslines
        } else {
            0
        };

        for j in 0..(*wp).sy {
            if (*wp).yoff + j < (*ctx).oy || (*wp).yoff + j >= (*ctx).oy + (*ctx).sy {
                continue;
            }
            let y = top + (*wp).yoff + j - (*ctx).oy;

            let (i, x, width) =
                if (*wp).xoff >= (*ctx).ox && (*wp).xoff + (*wp).sx <= (*ctx).ox + (*ctx).sx {
                    // All visible
                    (0, (*wp).xoff - (*ctx).ox, (*wp).sx)
                } else if (*wp).xoff < (*ctx).ox && (*wp).xoff + (*wp).sx > (*ctx).ox + (*ctx).sx {
                    // Both left and right not visible
                    ((*ctx).ox, 0, (*ctx).sx)
                } else if (*wp).xoff < (*ctx).ox {
                    // Left not visible
                    let i = (*ctx).ox - (*wp).xoff;
                    (i, 0, (*wp).sx - i)
                } else {
                    // Right not visible
                    let x = (*wp).xoff - (*ctx).ox;
                    (0, x, (*ctx).sx - x)
                };

            log_debug!(
                "{}: {} %%{} line {},{} at {},{}, width {}",
                "screen_redraw_draw_pane",
                _s((*c).name),
                (*wp).id,
                i,
                j,
                x,
                y,
                width,
            );

            tty_default_colours(&raw mut defaults, wp);
            tty_draw_line(tty, s, i, j, width, x, y, &raw mut defaults, palette);
        }

        #[cfg(feature = "sixel")]
        tty_draw_images(c, wp, s);
    }
}

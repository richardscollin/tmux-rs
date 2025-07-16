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
use super::*;

pub static WINDOW_COPY_MODE: window_mode = window_mode {
    name: SyncCharPtr::new(c"copy-mode"),
    init: window_copy_init,
    free: window_copy_free,
    resize: window_copy_resize,
    key_table: Some(window_copy_key_table),
    command: Some(window_copy_command),
    formats: Some(window_copy_formats),
    default_format: SyncCharPtr::null(),
    update: None,
    key: None,
};

pub static WINDOW_VIEW_MODE: window_mode = window_mode {
    name: SyncCharPtr::new(c"view-mode"),
    init: window_copy_view_init,
    free: window_copy_free,
    resize: window_copy_resize,
    key_table: Some(window_copy_key_table),
    command: Some(window_copy_command),
    formats: Some(window_copy_formats),
    default_format: SyncCharPtr::null(),
    update: None,
    key: None,
};

#[repr(i32)]
#[derive(Copy, Clone, Eq, PartialEq)]
pub enum window_copy {
    WINDOW_COPY_OFF,
    WINDOW_COPY_SEARCHUP,
    WINDOW_COPY_SEARCHDOWN,
    WINDOW_COPY_JUMPFORWARD,
    WINDOW_COPY_JUMPBACKWARD,
    WINDOW_COPY_JUMPTOFORWARD,
    WINDOW_COPY_JUMPTOBACKWARD,
}

#[repr(i32)]
#[derive(Copy, Clone, Eq, PartialEq)]
pub enum window_copy_rel_pos {
    WINDOW_COPY_REL_POS_ABOVE,
    WINDOW_COPY_REL_POS_ON_SCREEN,
    WINDOW_COPY_REL_POS_BELOW,
}

#[repr(i32)]
#[derive(Copy, Clone, Eq, PartialEq)]
pub enum window_copy_cmd_action {
    WINDOW_COPY_CMD_NOTHING,
    WINDOW_COPY_CMD_REDRAW,
    WINDOW_COPY_CMD_CANCEL,
}

#[repr(i32)]
#[derive(Copy, Clone, Eq, PartialEq)]
pub enum window_copy_cmd_clear {
    WINDOW_COPY_CMD_CLEAR_ALWAYS,
    WINDOW_COPY_CMD_CLEAR_NEVER,
    WINDOW_COPY_CMD_CLEAR_EMACS_ONLY,
}

#[repr(C)]
pub struct window_copy_cmd_state {
    wme: *mut window_mode_entry,
    args: *mut args,
    m: *mut mouse_event,

    c: *mut client,
    s: *mut session,
    wl: *mut winlink,
}

#[repr(i32)]
#[derive(Copy, Clone, Eq, PartialEq)]
enum selflag {
    /// select one char at a time
    SEL_CHAR,
    /// select one word at a time
    SEL_WORD,
    /// select one line at a time
    SEL_LINE,
}

#[repr(i32)]
#[derive(Copy, Clone, Eq, PartialEq)]
enum cursordrag {
    /// selection is independent of cursor
    CURSORDRAG_NONE,
    /// end is synchronized with cursor
    CURSORDRAG_ENDSEL,
    /// start is synchronized with cursor
    CURSORDRAG_SEL,
}

#[repr(i32)]
#[derive(Copy, Clone, Eq, PartialEq)]
enum line_sel {
    LINE_SEL_NONE,
    LINE_SEL_LEFT_RIGHT,
    LINE_SEL_RIGHT_LEFT,
}

const WINDOW_COPY_SEARCH_TIMEOUT: u64 = 10000;
const WINDOW_COPY_SEARCH_ALL_TIMEOUT: u64 = 200;
const WINDOW_COPY_SEARCH_MAX_LINE: u32 = 2000;

const WINDOW_COPY_DRAG_REPEAT_TIME: libc::suseconds_t = 50000;

/*
 * Copy mode's visible screen (the "screen" field) is filled from one of two
 * sources: the original contents of the pane (used when we actually enter via
 * the "copy-mode" command, to copy the contents of the current pane), or else
 * a series of lines containing the output from an output-writing tmux command
 * (such as any of the "show-*" or "list-*" commands).
 *
 * In either case, the full content of the copy-mode grid is pointed at by the
 * "backing" field, and is copied into "screen" as needed (that is, when
 * scrolling occurs). When copy-mode is backed by a pane, backing points
 * directly at that pane's screen structure (&wp->base); when backed by a list
 * of output-lines from a command, it points at a newly-allocated screen
 * structure (which is deallocated when the mode ends).
 */
#[repr(C)]
pub struct window_copy_mode_data {
    screen: screen,

    backing: *mut screen,
    backing_written: i32, /* backing display started */
    writing: *mut screen,
    ictx: *mut input_ctx,

    viewmode: i32, /* view mode entered */

    oy: u32, /* number of lines scrolled up */

    selx: u32, /* beginning of selection */
    sely: u32,

    endselx: u32, /* end of selection */
    endsely: u32,

    cursordrag: cursordrag,

    modekeys: modekey,
    lineflag: line_sel, /* line selection mode */
    rectflag: i32,      /* in rectangle copy mode? */
    scroll_exit: i32,   /* exit on scroll to end? */
    hide_position: i32, /* hide position marker */

    selflag: selflag,

    /// word separators
    separators: *const u8,

    /// drag start position x
    dx: u32,
    /// drag start position y
    dy: u32,

    // selection reset positions
    selrx: u32,
    selry: u32,
    endselrx: u32,
    endselry: u32,

    cx: u32,
    cy: u32,

    /* position in last line w/ content */
    lastcx: u32,
    /* size of last line w/ content */
    lastsx: u32,

    /* mark position */
    mx: u32,
    my: u32,
    showmark: i32,

    searchtype: window_copy,
    searchdirection: i32,
    searchregex: i32,
    searchstr: *mut u8,
    searchmark: *mut u8,
    searchcount: i32,
    searchmore: i32,
    searchall: i32,
    searchx: i32,
    searchy: i32,
    searcho: i32,
    searchgen: u8,

    /// search has timed out
    timeout: i32,

    jumptype: window_copy,
    jumpchar: Option<Box<utf8_data>>,

    dragtimer: event,
}

pub unsafe extern "C-unwind" fn window_copy_scroll_timer(
    _fd: i32,
    _events: i16,
    wme: NonNull<window_mode_entry>,
) {
    unsafe {
        let wp: *mut window_pane = (*wme.as_ptr()).wp;
        let data: *mut window_copy_mode_data = (*wme.as_ptr()).data.cast();
        let mut tv = libc::timeval {
            tv_sec: 0,
            tv_usec: WINDOW_COPY_DRAG_REPEAT_TIME,
        };

        evtimer_del(&raw mut (*data).dragtimer);

        if tailq_first(&raw mut (*wp).modes) != wme.as_ptr() {
            return;
        }

        if (*data).cy == 0 {
            evtimer_add(&raw mut (*data).dragtimer, &raw mut tv);
            window_copy_cursor_up(wme.as_ptr(), 1);
        } else if (*data).cy == screen_size_y(&raw mut (*data).screen) - 1 {
            evtimer_add(&raw mut (*data).dragtimer, &raw mut tv);
            window_copy_cursor_down(wme.as_ptr(), 1);
        }
    }
}

pub unsafe fn window_copy_clone_screen(
    src: *mut screen,
    hint: *mut screen,
    cx: *mut u32,
    cy: *mut u32,
    trim: i32,
) -> *mut screen {
    unsafe {
        let mut gl: *const grid_line = null();
        let mut wx: u32 = 0;
        let mut wy: u32 = 0;

        let mut reflow = false;

        let dst: *mut screen = xcalloc1();

        let mut sy = screen_hsize(src) + screen_size_y(src);
        if trim != 0 {
            while sy > screen_hsize(src) {
                gl = grid_peek_line((*src).grid, sy - 1);
                if (*gl).cellused != 0 {
                    break;
                }
                sy -= 1;
            }
        }
        // log_debug( "%s: target screen is %ux%u, source %ux%u", __func__, screen_size_x(src), sy, screen_size_x(hint), screen_hsize(src) + screen_size_y(src),);
        screen_init(dst, screen_size_x(src), sy, screen_hlimit(src));

        /*
         * Ensure history is on for the backing grid so lines are not deleted
         * during resizing.
         */
        (*(*dst).grid).flags |= GRID_HISTORY;
        grid_duplicate_lines((*dst).grid, 0, (*src).grid, 0, sy);

        (*(*dst).grid).sy = sy - screen_hsize(src);
        (*(*dst).grid).hsize = screen_hsize(src);
        (*(*dst).grid).hscrolled = (*(*src).grid).hscrolled;
        if (*src).cy > (*(*dst).grid).sy - 1 {
            (*dst).cx = 0;
            (*dst).cy = (*(*dst).grid).sy - 1;
        } else {
            (*dst).cx = (*src).cx;
            (*dst).cy = (*src).cy;
        }

        if !cx.is_null() && !cy.is_null() {
            *cx = (*dst).cx;
            *cy = screen_hsize(dst) + (*dst).cy;
            reflow = screen_size_x(hint) != screen_size_x(dst);
        } else {
            reflow = false;
        }
        if reflow {
            grid_wrap_position((*dst).grid, *cx, *cy, &raw mut wx, &raw mut wy);
        }
        screen_resize_cursor(dst, screen_size_x(hint), screen_size_y(hint), 1, 0, 0);
        if reflow {
            grid_unwrap_position((*dst).grid, cx, cy, wx, wy);
        }

        dst
    }
}

pub unsafe fn window_copy_common_init(wme: *mut window_mode_entry) -> *mut window_copy_mode_data {
    unsafe {
        let wp = (*wme).wp;
        let base = &raw mut (*wp).base;

        let data: *mut window_copy_mode_data = xcalloc1::<window_copy_mode_data>();
        (*wme).data = data.cast();

        (*data).cursordrag = cursordrag::CURSORDRAG_NONE;
        (*data).lineflag = line_sel::LINE_SEL_NONE;
        (*data).selflag = selflag::SEL_CHAR;

        if !(*wp).searchstr.is_null() {
            (*data).searchtype = window_copy::WINDOW_COPY_SEARCHUP;
            (*data).searchregex = (*wp).searchregex;
            (*data).searchstr = xstrdup((*wp).searchstr).as_ptr();
        } else {
            (*data).searchtype = window_copy::WINDOW_COPY_OFF;
            (*data).searchregex = 0;
            (*data).searchstr = null_mut();
        }
        (*data).searcho = -1;
        (*data).searchx = -1;
        (*data).searchy = -1;
        (*data).searchall = 1;

        (*data).jumptype = window_copy::WINDOW_COPY_OFF;
        (*data).jumpchar = None;

        screen_init(
            &raw mut (*data).screen,
            screen_size_x(base),
            screen_size_y(base),
            0,
        );
        (*data).modekeys =
            modekey::try_from(options_get_number_((*(*wp).window).options, c"mode-keys") as i32)
                .expect("invalid modekey");

        evtimer_set(
            &raw mut (*data).dragtimer,
            window_copy_scroll_timer,
            NonNull::new(wme).unwrap(),
        );

        data
    }
}

pub unsafe fn window_copy_init(
    wme: NonNull<window_mode_entry>,
    _fs: *mut cmd_find_state,
    args: *mut args,
) -> *mut screen {
    let wme = wme.as_ptr();
    unsafe {
        let wp = (*wme).swp;
        let mut data: *mut window_copy_mode_data = null_mut();
        let base = &raw mut (*wp).base;
        let mut ctx: screen_write_ctx = zeroed();
        let mut cx = 0;
        let mut cy = 0;

        data = window_copy_common_init(wme);
        (*data).backing = window_copy_clone_screen(
            base,
            &raw mut (*data).screen,
            &raw mut cx,
            &raw mut cy,
            ((*wme).swp != (*wme).wp) as i32,
        );

        (*data).cx = cx;
        if cy < screen_hsize((*data).backing) {
            (*data).cy = 0;
            (*data).oy = screen_hsize((*data).backing) - cy;
        } else {
            (*data).cy = cy - screen_hsize((*data).backing);
            (*data).oy = 0;
        }

        (*data).scroll_exit = args_has(args, b'e');
        (*data).hide_position = args_has(args, b'H');

        if !(*base).hyperlinks.is_null() {
            (*data).screen.hyperlinks = hyperlinks_copy((*base).hyperlinks);
        }
        (*data).screen.cx = (*data).cx;
        (*data).screen.cy = (*data).cy;
        (*data).mx = (*data).cx;
        (*data).my = screen_hsize((*data).backing) + (*data).cy - (*data).oy;
        (*data).showmark = 0;

        screen_write_start(&raw mut ctx, &raw mut (*data).screen);
        for i in 0..screen_size_y(&raw mut (*data).screen) {
            window_copy_write_line(wme, &raw mut ctx, i);
        }
        screen_write_cursormove(&raw mut ctx, (*data).cx as i32, (*data).cy as i32, 0);
        screen_write_stop(&raw mut ctx);

        &raw mut (*data).screen
    }
}

pub unsafe fn window_copy_view_init(
    wme: NonNull<window_mode_entry>,
    fs: *mut cmd_find_state,
    args: *mut args,
) -> *mut screen {
    let wme = wme.as_ptr();
    unsafe {
        let wp = (*wme).wp;
        // struct window_copy_mode_data *data;
        let base: *mut screen = &raw mut (*wp).base;
        let sx = screen_size_x(base);

        let data = window_copy_common_init(wme);
        (*data).viewmode = 1;

        (*data).backing = xmalloc_::<screen>().as_ptr();
        screen_init((*data).backing, sx, screen_size_y(base), u32::MAX);
        (*data).writing = xmalloc_::<screen>().as_ptr();
        screen_init((*data).writing, sx, screen_size_y(base), 0);
        (*data).ictx = input_init(null_mut(), null_mut(), null_mut());
        (*data).mx = (*data).cx;
        (*data).my = screen_hsize((*data).backing) + (*data).cy - (*data).oy;
        (*data).showmark = 0;

        &raw mut (*data).screen
    }
}

pub unsafe fn window_copy_free(wme: NonNull<window_mode_entry>) {
    unsafe {
        let wme = wme.as_ptr();
        let data: *mut window_copy_mode_data = (*wme).data.cast();

        evtimer_del(&raw mut (*data).dragtimer);

        free_((*data).searchmark);
        free_((*data).searchstr);
        (*data).jumpchar = None;

        if !(*data).writing.is_null() {
            screen_free((*data).writing);
            free_((*data).writing);
        }
        if !(*data).ictx.is_null() {
            input_free((*data).ictx);
        }
        screen_free((*data).backing);
        free_((*data).backing);

        screen_free(&raw mut (*data).screen);
        free_(data);
    }
}

macro_rules! window_copy_add {
   ($wp:expr, $parse:expr, $fmt:literal $(, $args:expr)* $(,)?) => {
        crate::window_copy::window_copy_vadd($wp, $parse, format_args!($fmt $(, $args)*))
    };
}
pub(crate) use window_copy_add;

pub unsafe fn window_copy_init_ctx_cb(ctx: *mut screen_write_ctx, ttyctx: *mut tty_ctx) {
    unsafe {
        memcpy__(&raw mut (*ttyctx).defaults, &raw const GRID_DEFAULT_CELL);
        (*ttyctx).palette = null_mut();
        (*ttyctx).redraw_cb = None;
        (*ttyctx).set_client_cb = None;
        (*ttyctx).arg = null_mut();
    }
}

pub unsafe fn window_copy_vadd(wp: *mut window_pane, parse: i32, args: std::fmt::Arguments) {
    unsafe {
        let wme: *mut window_mode_entry = tailq_first(&raw mut (*wp).modes);
        let data: *mut window_copy_mode_data = (*wme).data.cast();
        let backing: *mut screen = (*data).backing;
        let writing: *mut screen = (*data).writing;

        let mut writing_ctx: screen_write_ctx = zeroed();
        let mut backing_ctx: screen_write_ctx = zeroed();
        let mut ctx: screen_write_ctx = zeroed();

        let mut gc: grid_cell = zeroed();
        let mut old_hsize: u32 = 0;
        let mut old_cy: u32 = 0;
        let sx = screen_size_x(backing);

        if parse != 0 {
            let mut text = args.to_string();
            text.push('\0');
            screen_write_start(&raw mut writing_ctx, writing);
            screen_write_reset(&raw mut writing_ctx);
            input_parse_screen(
                (*data).ictx,
                writing,
                Some(window_copy_init_ctx_cb),
                data.cast(),
                text.as_mut_ptr(),
                text.len(),
            );
        }

        old_hsize = screen_hsize((*data).backing);
        screen_write_start(&raw mut backing_ctx, backing);
        if (*data).backing_written != 0 {
            /*
             * On the second or later line, do a CRLF before writing
             * (so it's on a new line).
             */
            screen_write_carriagereturn(&raw mut backing_ctx);
            screen_write_linefeed(&raw mut backing_ctx, 0, 8);
        } else {
            (*data).backing_written = 1;
        }
        old_cy = (*backing).cy;
        if parse != 0 {
            screen_write_fast_copy(&raw mut backing_ctx, writing, 0, 0, sx, 1);
        } else {
            memcpy__(&raw mut gc, &raw const GRID_DEFAULT_CELL);
            screen_write_vnputs_(&raw mut backing_ctx, 0, &raw const gc, args);
        }
        screen_write_stop(&raw mut backing_ctx);

        (*data).oy += screen_hsize((*data).backing) - old_hsize;

        screen_write_start_pane(&raw mut ctx, wp, &raw mut (*data).screen);

        /*
         * If the history has changed, draw the top line.
         * (If there's any history at all, it has changed.)
         */
        if screen_hsize((*data).backing) != 0 {
            window_copy_redraw_lines(wme, 0, 1);
        }

        /* Write the new lines. */
        window_copy_redraw_lines(wme, old_cy, (*backing).cy - old_cy + 1);

        screen_write_stop(&raw mut ctx);
    }
}

pub unsafe fn window_copy_pageup(wp: *mut window_pane, half_page: i32) {
    unsafe {
        window_copy_pageup1(tailq_first(&raw mut (*wp).modes), half_page);
    }
}

pub unsafe fn window_copy_pageup1(wme: *mut window_mode_entry, half_page: i32) {
    unsafe {
        let data: *mut window_copy_mode_data = (*wme).data.cast();
        let s: *mut screen = &raw mut (*data).screen;
        // u_int n, ox, oy, px, py;

        let oy = screen_hsize((*data).backing) + (*data).cy - (*data).oy;
        let ox = window_copy_find_length(wme, oy);

        if (*data).cx != ox {
            (*data).lastcx = (*data).cx;
            (*data).lastsx = ox;
        }
        (*data).cx = (*data).lastcx;

        let mut n = 1;
        if screen_size_y(s) > 2 {
            if half_page != 0 {
                n = screen_size_y(s) / 2;
            } else {
                n = screen_size_y(s) - 2;
            }
        }

        if (*data).oy + n > screen_hsize((*data).backing) {
            (*data).oy = screen_hsize((*data).backing);
            if (*data).cy < n {
                (*data).cy = 0;
            } else {
                (*data).cy -= n;
            }
        } else {
            (*data).oy += n;
        }

        if (*data).screen.sel.is_null() || (*data).rectflag == 0 {
            let py = screen_hsize((*data).backing) + (*data).cy - (*data).oy;
            let px = window_copy_find_length(wme, py);
            if ((*data).cx >= (*data).lastsx && (*data).cx != px) || (*data).cx > px {
                window_copy_cursor_end_of_line(wme);
            }
        }

        if !(*data).searchmark.is_null() && (*data).timeout == 0 {
            window_copy_search_marks(wme, null_mut(), (*data).searchregex, 1);
        }
        window_copy_update_selection(wme, 1, 0);
        window_copy_redraw_screen(wme);
    }
}

pub unsafe fn window_copy_pagedown(wp: *mut window_pane, half_page: i32, scroll_exit: i32) {
    unsafe {
        if window_copy_pagedown1(tailq_first(&raw mut (*wp).modes), half_page, scroll_exit) != 0 {
            window_pane_reset_mode(wp);
        }
    }
}

pub unsafe fn window_copy_pagedown1(
    wme: *mut window_mode_entry,
    half_page: i32,
    scroll_exit: i32,
) -> i32 {
    unsafe {
        let data: *mut window_copy_mode_data = (*wme).data.cast();
        let s: *mut screen = &raw mut (*data).screen;

        let oy = screen_hsize((*data).backing) + (*data).cy - (*data).oy;
        let ox = window_copy_find_length(wme, oy);

        if (*data).cx != ox {
            (*data).lastcx = (*data).cx;
            (*data).lastsx = ox;
        }
        (*data).cx = (*data).lastcx;

        let mut n = 1;
        if screen_size_y(s) > 2 {
            if half_page != 0 {
                n = screen_size_y(s) / 2;
            } else {
                n = screen_size_y(s) - 2;
            }
        }

        if (*data).oy < n {
            (*data).oy = 0;
            if (*data).cy + (n - (*data).oy) >= screen_size_y((*data).backing) {
                (*data).cy = screen_size_y((*data).backing) - 1;
            } else {
                (*data).cy += n - (*data).oy;
            }
        } else {
            (*data).oy -= n;
        }

        if (*data).screen.sel.is_null() || (*data).rectflag == 0 {
            let py = screen_hsize((*data).backing) + (*data).cy - (*data).oy;
            let px = window_copy_find_length(wme, py);
            if ((*data).cx >= (*data).lastsx && (*data).cx != px) || (*data).cx > px {
                window_copy_cursor_end_of_line(wme);
            }
        }

        if scroll_exit != 0 && (*data).oy == 0 {
            return 1;
        }
        if !(*data).searchmark.is_null() && (*data).timeout == 0 {
            window_copy_search_marks(wme, null_mut(), (*data).searchregex, 1);
        }
        window_copy_update_selection(wme, 1, 0);
        window_copy_redraw_screen(wme);

        0
    }
}

pub unsafe fn window_copy_previous_paragraph(wme: *mut window_mode_entry) {
    unsafe {
        let data: *mut window_copy_mode_data = (*wme).data.cast();

        let mut oy = screen_hsize((*data).backing) + (*data).cy - (*data).oy;

        while oy > 0 && window_copy_find_length(wme, oy) == 0 {
            oy -= 1;
        }

        while oy > 0 && window_copy_find_length(wme, oy) > 0 {
            oy -= 1;
        }

        window_copy_scroll_to(wme, 0, oy, false);
    }
}

pub unsafe fn window_copy_next_paragraph(wme: *mut window_mode_entry) {
    unsafe {
        let data: *mut window_copy_mode_data = (*wme).data.cast();
        let s: *mut screen = &raw mut (*data).screen;

        let mut oy = screen_hsize((*data).backing) + (*data).cy - (*data).oy;
        let maxy = screen_hsize((*data).backing) + screen_size_y(s) - 1;

        while oy < maxy && window_copy_find_length(wme, oy) == 0 {
            oy += 1;
        }

        while oy < maxy && window_copy_find_length(wme, oy) > 0 {
            oy += 1;
        }

        let ox = window_copy_find_length(wme, oy);
        window_copy_scroll_to(wme, ox, oy, false);
    }
}

pub unsafe fn window_copy_get_word(wp: *mut window_pane, x: u32, y: u32) -> *mut u8 {
    unsafe {
        let wme: *mut window_mode_entry = tailq_first(&raw mut (*wp).modes);
        let data: *mut window_copy_mode_data = (*wme).data.cast();
        let gd = (*data).screen.grid;

        format_grid_word(gd, x, (*gd).hsize + y)
    }
}

pub unsafe fn window_copy_get_line(wp: *mut window_pane, y: u32) -> *mut u8 {
    unsafe {
        let wme: *mut window_mode_entry = tailq_first(&raw mut (*wp).modes);
        let data: *mut window_copy_mode_data = (*wme).data.cast();
        let gd = (*data).screen.grid;

        format_grid_line(gd, (*gd).hsize + y)
    }
}

pub unsafe fn window_copy_cursor_hyperlink_cb(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        let wp = format_get_pane(ft);
        let wme = tailq_first(&raw mut (*wp).modes);
        let data: *mut window_copy_mode_data = (*wme).data.cast();
        let gd = (*data).screen.grid;

        format_grid_hyperlink(
            gd,
            (*data).cx,
            (*gd).hsize + (*data).cy,
            &raw mut (*data).screen,
        )
        .cast()
    }
}

pub unsafe fn window_copy_cursor_word_cb(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        let wp: *mut window_pane = format_get_pane(ft);
        let wme: *mut window_mode_entry = tailq_first(&raw mut (*wp).modes);
        let data: *mut window_copy_mode_data = (*wme).data.cast();

        window_copy_get_word(wp, (*data).cx, (*data).cy).cast()
    }
}

pub unsafe fn window_copy_cursor_line_cb(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        let wp: *mut window_pane = format_get_pane(ft);
        let wme: *mut window_mode_entry = tailq_first(&raw mut (*wp).modes);
        let data: *mut window_copy_mode_data = (*wme).data.cast();

        window_copy_get_line(wp, (*data).cy).cast()
    }
}

pub unsafe fn window_copy_search_match_cb(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        let wp: *mut window_pane = format_get_pane(ft);
        let wme: *mut window_mode_entry = tailq_first(&raw mut (*wp).modes);
        let data: *mut window_copy_mode_data = (*wme).data.cast();

        window_copy_match_at_cursor(data).cast()
    }
}

pub unsafe fn window_copy_formats(wme: *mut window_mode_entry, ft: *mut format_tree) {
    unsafe {
        let data: *mut window_copy_mode_data = (*wme).data.cast();

        format_add!(ft, c!("scroll_position"), "{}", (*data).oy);
        format_add!(ft, c!("rectangle_toggle"), "{}", (*data).rectflag,);

        format_add!(ft, c!("copy_cursor_x"), "{}", (*data).cx);
        format_add!(ft, c!("copy_cursor_y"), "{}", (*data).cy);

        if !(*data).screen.sel.is_null() {
            format_add!(ft, c!("selection_start_x"), "{}", (*data).selx,);
            format_add!(ft, c!("selection_start_y"), "{}", (*data).sely,);
            format_add!(ft, c!("selection_end_x"), "{}", (*data).endselx,);
            format_add!(ft, c!("selection_end_y"), "{}", (*data).endsely,);

            if (*data).cursordrag != cursordrag::CURSORDRAG_NONE {
                format_add!(ft, c!("selection_active"), "1");
            } else {
                format_add!(ft, c!("selection_active"), "0");
            }
            if (*data).endselx != (*data).selx || (*data).endsely != (*data).sely {
                format_add!(ft, c!("selection_present"), "1");
            } else {
                format_add!(ft, c!("selection_present"), "0");
            }
        } else {
            format_add!(ft, c!("selection_active"), "0");
            format_add!(ft, c!("selection_present"), "0");
        }

        format_add!(
            ft,
            c!("search_present"),
            "{}",
            !(*data).searchmark.is_null() as i32,
        );
        if (*data).searchcount != -1 {
            format_add!(ft, c!("search_count"), "{}", (*data).searchcount,);
            format_add!(ft, c!("search_count_partial"), "{}", (*data).searchmore,);
        }
        format_add_cb(ft, c!("search_match"), Some(window_copy_search_match_cb));

        format_add_cb(ft, c!("copy_cursor_word"), Some(window_copy_cursor_word_cb));
        format_add_cb(ft, c!("copy_cursor_line"), Some(window_copy_cursor_line_cb));
        format_add_cb(
            ft,
            c!("copy_cursor_hyperlink"),
            Some(window_copy_cursor_hyperlink_cb),
        );
    }
}

pub unsafe fn window_copy_size_changed(wme: *mut window_mode_entry) {
    unsafe {
        let data: *mut window_copy_mode_data = (*wme).data.cast();
        let s: *mut screen = &raw mut (*data).screen;
        let mut ctx: screen_write_ctx = zeroed();
        let search = !(*data).searchmark.is_null();

        window_copy_clear_selection(wme);
        window_copy_clear_marks(wme);

        screen_write_start(&raw mut ctx, s);
        window_copy_write_lines(wme, &raw mut ctx, 0, screen_size_y(s));
        screen_write_stop(&raw mut ctx);

        if search && (*data).timeout == 0 {
            window_copy_search_marks(wme, null_mut(), (*data).searchregex, 0);
        }
        (*data).searchx = (*data).cx as i32;
        (*data).searchy = (*data).cy as i32;
        (*data).searcho = (*data).oy as i32;
    }
}

pub unsafe fn window_copy_resize(wme: NonNull<window_mode_entry>, sx: u32, sy: u32) {
    unsafe {
        let wme = wme.as_ptr();
        let data: *mut window_copy_mode_data = (*wme).data.cast();
        let s: *mut screen = &raw mut (*data).screen;
        let gd: *mut grid = (*(*data).backing).grid;
        let mut wx = 0;
        let mut wy = 0;
        // int reflow;

        screen_resize(s, sx, sy, 0);
        let mut cx = (*data).cx;
        let mut cy = (*gd).hsize + (*data).cy - (*data).oy;
        let reflow = (*gd).sx != sx;
        if reflow {
            grid_wrap_position(gd, cx, cy, &raw mut wx, &raw mut wy);
        }
        screen_resize_cursor((*data).backing, sx, sy, 1, 0, 0);
        if reflow {
            grid_unwrap_position(gd, &raw mut cx, &raw mut cy, wx, wy);
        }

        (*data).cx = cx;
        if cy < (*gd).hsize {
            (*data).cy = 0;
            (*data).oy = (*gd).hsize - cy;
        } else {
            (*data).cy = cy - (*gd).hsize;
            (*data).oy = 0;
        }

        window_copy_size_changed(wme);
        window_copy_redraw_screen(wme);
    }
}

pub unsafe fn window_copy_key_table(wme: *mut window_mode_entry) -> *const u8 {
    unsafe {
        if matches!(
            modekey::try_from(
                options_get_number_((*(*(*wme).wp).window).options, c"mode-keys") as i32
            ),
            Ok(modekey::MODEKEY_VI)
        ) {
            c!("copy-mode-vi")
        } else {
            c!("copy-mode")
        }
    }
}

pub unsafe fn window_copy_expand_search_string(cs: *mut window_copy_cmd_state) -> i32 {
    unsafe {
        let wme: *mut window_mode_entry = (*cs).wme;
        let data: *mut window_copy_mode_data = (*wme).data.cast();
        let ss = args_string((*cs).args, 1);

        if ss.is_null() || *ss == b'\0' {
            return 0;
        }

        if args_has_((*cs).args, 'F') {
            let expanded = format_single(
                null_mut(),
                ss,
                null_mut(),
                null_mut(),
                null_mut(),
                (*wme).wp,
            );
            if *expanded == b'\0' {
                free_(expanded);
                return 0;
            }
            free_((*data).searchstr);
            (*data).searchstr = expanded;
        } else {
            free_((*data).searchstr);
            (*data).searchstr = xstrdup(ss).as_ptr();
        }
        1
    }
}

pub unsafe fn window_copy_cmd_append_selection(
    cs: *mut window_copy_cmd_state,
) -> window_copy_cmd_action {
    unsafe {
        let wme: *mut window_mode_entry = (*cs).wme;
        let s = (*cs).s;

        if !s.is_null() {
            window_copy_append_selection(wme);
        }
        window_copy_clear_selection(wme);
        window_copy_cmd_action::WINDOW_COPY_CMD_REDRAW
    }
}

pub unsafe fn window_copy_cmd_append_selection_and_cancel(
    cs: *mut window_copy_cmd_state,
) -> window_copy_cmd_action {
    unsafe {
        let wme = (*cs).wme;
        let s = (*cs).s;

        if !s.is_null() {
            window_copy_append_selection(wme);
        }
        window_copy_clear_selection(wme);
        window_copy_cmd_action::WINDOW_COPY_CMD_CANCEL
    }
}

pub unsafe fn window_copy_cmd_back_to_indentation(
    cs: *mut window_copy_cmd_state,
) -> window_copy_cmd_action {
    unsafe {
        window_copy_cursor_back_to_indentation((*cs).wme);
        window_copy_cmd_action::WINDOW_COPY_CMD_NOTHING
    }
}

pub unsafe fn window_copy_cmd_begin_selection(
    cs: *mut window_copy_cmd_state,
) -> window_copy_cmd_action {
    unsafe {
        let wme: *mut window_mode_entry = (*cs).wme.cast();
        let c: *mut client = (*cs).c;
        let m = (*cs).m;
        let data: *mut window_copy_mode_data = (*wme).data.cast();

        if !m.is_null() {
            window_copy_start_drag(c, m);
            return window_copy_cmd_action::WINDOW_COPY_CMD_NOTHING;
        }

        (*data).lineflag = line_sel::LINE_SEL_NONE;
        (*data).selflag = selflag::SEL_CHAR;
        window_copy_start_selection(wme);
        window_copy_cmd_action::WINDOW_COPY_CMD_REDRAW
    }
}

pub unsafe fn window_copy_cmd_stop_selection(
    cs: *mut window_copy_cmd_state,
) -> window_copy_cmd_action {
    unsafe {
        let wme = (*cs).wme;
        let data: *mut window_copy_mode_data = (*wme).data.cast();

        (*data).cursordrag = cursordrag::CURSORDRAG_NONE;
        (*data).lineflag = line_sel::LINE_SEL_NONE;
        (*data).selflag = selflag::SEL_CHAR;
        window_copy_cmd_action::WINDOW_COPY_CMD_NOTHING
    }
}

pub unsafe fn window_copy_cmd_bottom_line(
    cs: *mut window_copy_cmd_state,
) -> window_copy_cmd_action {
    unsafe {
        let wme = (*cs).wme;
        let data: *mut window_copy_mode_data = (*wme).data.cast();

        (*data).cx = 0;
        (*data).cy = screen_size_y(&raw mut (*data).screen) - 1;

        window_copy_update_selection(wme, 1, 0);
        window_copy_cmd_action::WINDOW_COPY_CMD_REDRAW
    }
}

pub unsafe fn window_copy_cmd_cancel(_cs: *mut window_copy_cmd_state) -> window_copy_cmd_action {
    unsafe { window_copy_cmd_action::WINDOW_COPY_CMD_CANCEL }
}

pub unsafe fn window_copy_cmd_clear_selection(
    cs: *mut window_copy_cmd_state,
) -> window_copy_cmd_action {
    unsafe {
        let wme: *mut window_mode_entry = (*cs).wme;

        window_copy_clear_selection(wme);
        window_copy_cmd_action::WINDOW_COPY_CMD_REDRAW
    }
}

pub unsafe fn window_copy_do_copy_end_of_line(
    cs: *mut window_copy_cmd_state,
    pipe: i32,
    cancel: i32,
) -> window_copy_cmd_action {
    unsafe {
        let wme = (*cs).wme;
        let c = (*cs).c;
        let s = (*cs).s;
        let wl = (*cs).wl;
        let wp = (*wme).wp;
        let count = args_count((*cs).args);
        let mut np = (*wme).prefix;
        let data: *mut window_copy_mode_data = (*wme).data.cast();
        let mut prefix = null_mut();
        let mut command = null_mut();
        let arg1 = args_string((*cs).args, 1);
        let arg2 = args_string((*cs).args, 2);

        if pipe != 0 {
            if count == 3 {
                prefix = format_single(null_mut(), arg2, c, s, wl, wp);
            }
            if !s.is_null() && count > 1 && *arg1 != b'\0' {
                command = format_single(null_mut(), arg1, c, s, wl, wp);
            }
        } else if count == 2 {
            prefix = format_single(null_mut(), arg1, c, s, wl, wp);
        }

        let ocx = (*data).cx;
        let ocy = (*data).cy;
        let ooy = (*data).oy;

        window_copy_start_selection(wme);
        while np > 1 {
            window_copy_cursor_down(wme, 0);
            np -= 1;
        }
        window_copy_cursor_end_of_line(wme);

        if !s.is_null() {
            if pipe != 0 {
                window_copy_copy_pipe(wme, s, prefix, command);
            } else {
                window_copy_copy_selection(wme, prefix);
            }

            if cancel != 0 {
                free_(prefix);
                free_(command);
                return window_copy_cmd_action::WINDOW_COPY_CMD_CANCEL;
            }
        }
        window_copy_clear_selection(wme);

        (*data).cx = ocx;
        (*data).cy = ocy;
        (*data).oy = ooy;

        free_(prefix);
        free_(command);
        window_copy_cmd_action::WINDOW_COPY_CMD_REDRAW
    }
}

pub unsafe fn window_copy_cmd_copy_end_of_line(
    cs: *mut window_copy_cmd_state,
) -> window_copy_cmd_action {
    unsafe { window_copy_do_copy_end_of_line(cs, 0, 0) }
}

pub unsafe fn window_copy_cmd_copy_end_of_line_and_cancel(
    cs: *mut window_copy_cmd_state,
) -> window_copy_cmd_action {
    unsafe { window_copy_do_copy_end_of_line(cs, 0, 1) }
}

pub unsafe fn window_copy_cmd_copy_pipe_end_of_line(
    cs: *mut window_copy_cmd_state,
) -> window_copy_cmd_action {
    unsafe { window_copy_do_copy_end_of_line(cs, 1, 0) }
}

pub unsafe fn window_copy_cmd_copy_pipe_end_of_line_and_cancel(
    cs: *mut window_copy_cmd_state,
) -> window_copy_cmd_action {
    unsafe { window_copy_do_copy_end_of_line(cs, 1, 1) }
}

pub unsafe fn window_copy_do_copy_line(
    cs: *mut window_copy_cmd_state,
    pipe: i32,
    cancel: i32,
) -> window_copy_cmd_action {
    unsafe {
        let wme: *mut window_mode_entry = (*cs).wme;
        let c = (*cs).c;
        let s = (*cs).s;
        let wl = (*cs).wl;
        let wp = (*wme).wp;
        let data: *mut window_copy_mode_data = (*wme).data.cast();
        let count = args_count((*cs).args);
        let mut np = (*wme).prefix;
        // ocx, ocy, ooy;
        let mut prefix = null_mut();
        let mut command = null_mut();

        let arg1 = args_string((*cs).args, 1);
        let arg2 = args_string((*cs).args, 2);

        if pipe != 0 {
            if count == 3 {
                prefix = format_single(null_mut(), arg2, c, s, wl, wp);
            }
            if !s.is_null() && count > 1 && *arg1 != b'\0' {
                command = format_single(null_mut(), arg1, c, s, wl, wp);
            }
        } else if count == 2 {
            prefix = format_single(null_mut(), arg1, c, s, wl, wp);
        }

        let ocx = (*data).cx;
        let ocy = (*data).cy;
        let ooy = (*data).oy;

        (*data).selflag = selflag::SEL_CHAR;
        window_copy_cursor_start_of_line(wme);
        window_copy_start_selection(wme);
        while np > 1 {
            window_copy_cursor_down(wme, 0);
            np -= 1;
        }
        window_copy_cursor_end_of_line(wme);

        if !s.is_null() {
            if pipe != 0 {
                window_copy_copy_pipe(wme, s, prefix, command);
            } else {
                window_copy_copy_selection(wme, prefix);
            }

            if cancel != 0 {
                free_(prefix);
                free_(command);
                return window_copy_cmd_action::WINDOW_COPY_CMD_CANCEL;
            }
        }
        window_copy_clear_selection(wme);

        (*data).cx = ocx;
        (*data).cy = ocy;
        (*data).oy = ooy;

        free_(prefix);
        free_(command);
        window_copy_cmd_action::WINDOW_COPY_CMD_REDRAW
    }
}

pub unsafe fn window_copy_cmd_copy_line(cs: *mut window_copy_cmd_state) -> window_copy_cmd_action {
    unsafe { window_copy_do_copy_line(cs, 0, 0) }
}

pub unsafe fn window_copy_cmd_copy_line_and_cancel(
    cs: *mut window_copy_cmd_state,
) -> window_copy_cmd_action {
    unsafe { window_copy_do_copy_line(cs, 0, 1) }
}

pub unsafe fn window_copy_cmd_copy_pipe_line(
    cs: *mut window_copy_cmd_state,
) -> window_copy_cmd_action {
    unsafe { window_copy_do_copy_line(cs, 1, 0) }
}

pub unsafe fn window_copy_cmd_copy_pipe_line_and_cancel(
    cs: *mut window_copy_cmd_state,
) -> window_copy_cmd_action {
    unsafe { window_copy_do_copy_line(cs, 1, 1) }
}

pub unsafe fn window_copy_cmd_copy_selection_no_clear(
    cs: *mut window_copy_cmd_state,
) -> window_copy_cmd_action {
    unsafe {
        let wme: *mut window_mode_entry = (*cs).wme;
        let c: *mut client = (*cs).c;
        let s: *mut session = (*cs).s;
        let wl: *mut winlink = (*cs).wl;
        let wp: *mut window_pane = (*wme).wp;
        let mut prefix = null_mut();
        let arg1 = args_string((*cs).args, 1);

        if !arg1.is_null() {
            prefix = format_single(null_mut(), arg1, c, s, wl, wp);
        }

        if !s.is_null() {
            window_copy_copy_selection(wme, prefix);
        }

        free_(prefix);
        window_copy_cmd_action::WINDOW_COPY_CMD_NOTHING
    }
}

pub unsafe fn window_copy_cmd_copy_selection(
    cs: *mut window_copy_cmd_state,
) -> window_copy_cmd_action {
    unsafe {
        let wme = (*cs).wme;

        window_copy_cmd_copy_selection_no_clear(cs);
        window_copy_clear_selection(wme);
        window_copy_cmd_action::WINDOW_COPY_CMD_REDRAW
    }
}

pub unsafe fn window_copy_cmd_copy_selection_and_cancel(
    cs: *mut window_copy_cmd_state,
) -> window_copy_cmd_action {
    unsafe {
        let wme = (*cs).wme;

        window_copy_cmd_copy_selection_no_clear(cs);
        window_copy_clear_selection(wme);
        window_copy_cmd_action::WINDOW_COPY_CMD_CANCEL
    }
}

pub unsafe fn window_copy_cmd_cursor_down(
    cs: *mut window_copy_cmd_state,
) -> window_copy_cmd_action {
    unsafe {
        let wme = (*cs).wme;
        let mut np = (*wme).prefix;

        while np != 0 {
            window_copy_cursor_down(wme, 0);
            np -= 1;
        }
        window_copy_cmd_action::WINDOW_COPY_CMD_NOTHING
    }
}

pub unsafe fn window_copy_cmd_cursor_down_and_cancel(
    cs: *mut window_copy_cmd_state,
) -> window_copy_cmd_action {
    unsafe {
        let wme = (*cs).wme;
        let data: *mut window_copy_mode_data = (*wme).data.cast();
        let mut np = (*wme).prefix;

        let cy = (*data).cy;
        while np != 0 {
            window_copy_cursor_down(wme, 0);
            np -= 1;
        }

        if cy == (*data).cy && (*data).oy == 0 {
            window_copy_cmd_action::WINDOW_COPY_CMD_CANCEL
        } else {
            window_copy_cmd_action::WINDOW_COPY_CMD_NOTHING
        }
    }
}

pub unsafe fn window_copy_cmd_cursor_left(
    cs: *mut window_copy_cmd_state,
) -> window_copy_cmd_action {
    unsafe {
        let wme: *mut window_mode_entry = (*cs).wme;
        let mut np = (*wme).prefix;

        while np != 0 {
            window_copy_cursor_left(wme);
            np -= 1;
        }
        window_copy_cmd_action::WINDOW_COPY_CMD_NOTHING
    }
}

pub unsafe fn window_copy_cmd_cursor_right(
    cs: *mut window_copy_cmd_state,
) -> window_copy_cmd_action {
    unsafe {
        let wme: *mut window_mode_entry = (*cs).wme;
        let data: *mut window_copy_mode_data = (*wme).data.cast();
        let mut np = (*wme).prefix;

        while np != 0 {
            window_copy_cursor_right(
                wme,
                (!(*data).screen.sel.is_null() && (*data).rectflag != 0) as i32,
            );
            np -= 1;
        }
        window_copy_cmd_action::WINDOW_COPY_CMD_NOTHING
    }
}

/* Scroll line containing the cursor to the given position. */

pub unsafe fn window_copy_cmd_scroll_to(
    cs: *mut window_copy_cmd_state,
    to: u32,
) -> window_copy_cmd_action {
    unsafe {
        let wme = (*cs).wme;
        let data: *mut window_copy_mode_data = (*wme).data.cast();
        let scroll_up: i32 = (*data).cy as i32 - to as i32;
        let delta: u32 = scroll_up.unsigned_abs();
        let oy = screen_hsize((*data).backing) - (*data).oy;

        /*
         * oy is the maximum scroll down amount, while (*data).oy is the maximum
         * scroll up amount.
         */
        if scroll_up > 0 && (*data).oy >= delta {
            window_copy_scroll_up(wme, delta);
            (*data).cy -= delta;
        } else if scroll_up < 0 && oy >= delta {
            window_copy_scroll_down(wme, delta);
            (*data).cy += delta;
        }

        window_copy_update_selection(wme, 0, 0);
        window_copy_cmd_action::WINDOW_COPY_CMD_REDRAW
    }
}

/* Scroll line containing the cursor to the bottom. */

pub unsafe fn window_copy_cmd_scroll_bottom(
    cs: *mut window_copy_cmd_state,
) -> window_copy_cmd_action {
    unsafe {
        let data: *mut window_copy_mode_data = (*(*cs).wme).data.cast();

        let bottom = screen_size_y(&raw mut (*data).screen) - 1;
        window_copy_cmd_scroll_to(cs, bottom)
    }
}

/* Scroll line containing the cursor to the middle. */

pub unsafe fn window_copy_cmd_scroll_middle(
    cs: *mut window_copy_cmd_state,
) -> window_copy_cmd_action {
    unsafe {
        let data: *mut window_copy_mode_data = (*(*cs).wme).data.cast();
        let mid_value = (screen_size_y(&raw mut (*data).screen) - 1) / 2;
        window_copy_cmd_scroll_to(cs, mid_value)
    }
}

/* Scroll line containing the cursor to the top. */

pub unsafe fn window_copy_cmd_scroll_top(cs: *mut window_copy_cmd_state) -> window_copy_cmd_action {
    unsafe { window_copy_cmd_scroll_to(cs, 0) }
}

pub unsafe fn window_copy_cmd_cursor_up(cs: *mut window_copy_cmd_state) -> window_copy_cmd_action {
    unsafe {
        let wme: *mut window_mode_entry = (*cs).wme;
        let mut np = (*wme).prefix;

        while np != 0 {
            window_copy_cursor_up(wme, 0);
            np -= 1;
        }
        window_copy_cmd_action::WINDOW_COPY_CMD_NOTHING
    }
}

pub unsafe fn window_copy_cmd_end_of_line(
    cs: *mut window_copy_cmd_state,
) -> window_copy_cmd_action {
    unsafe {
        let wme = (*cs).wme;

        window_copy_cursor_end_of_line(wme);
        window_copy_cmd_action::WINDOW_COPY_CMD_NOTHING
    }
}

pub unsafe fn window_copy_cmd_halfpage_down(
    cs: *mut window_copy_cmd_state,
) -> window_copy_cmd_action {
    unsafe {
        let wme = (*cs).wme;
        let data: *mut window_copy_mode_data = (*wme).data.cast();
        let mut np = (*wme).prefix;

        while np != 0 {
            if window_copy_pagedown1(wme, 1, (*data).scroll_exit) != 0 {
                return window_copy_cmd_action::WINDOW_COPY_CMD_CANCEL;
            }
            np -= 1;
        }
        window_copy_cmd_action::WINDOW_COPY_CMD_NOTHING
    }
}

pub unsafe fn window_copy_cmd_halfpage_down_and_cancel(
    cs: *mut window_copy_cmd_state,
) -> window_copy_cmd_action {
    unsafe {
        let wme = (*cs).wme;
        let mut np = (*wme).prefix;

        while np != 0 {
            if window_copy_pagedown1(wme, 1, 1) != 0 {
                return window_copy_cmd_action::WINDOW_COPY_CMD_CANCEL;
            }
            np -= 1;
        }
        window_copy_cmd_action::WINDOW_COPY_CMD_NOTHING
    }
}

pub unsafe fn window_copy_cmd_halfpage_up(
    cs: *mut window_copy_cmd_state,
) -> window_copy_cmd_action {
    unsafe {
        let wme: *mut window_mode_entry = (*cs).wme;
        let mut np = (*wme).prefix;

        while np != 0 {
            window_copy_pageup1(wme, 1);
            np -= 1;
        }
        window_copy_cmd_action::WINDOW_COPY_CMD_NOTHING
    }
}

pub unsafe fn window_copy_cmd_toggle_position(
    cs: *mut window_copy_cmd_state,
) -> window_copy_cmd_action {
    unsafe {
        let wme: *mut window_mode_entry = (*cs).wme;
        let data: *mut window_copy_mode_data = (*wme).data.cast();

        (*data).hide_position = !(*data).hide_position;
        window_copy_cmd_action::WINDOW_COPY_CMD_REDRAW
    }
}

pub unsafe fn window_copy_cmd_history_bottom(
    cs: *mut window_copy_cmd_state,
) -> window_copy_cmd_action {
    unsafe {
        let wme: *mut window_mode_entry = (*cs).wme;
        let data: *mut window_copy_mode_data = (*wme).data.cast();
        let s: *mut screen = (*data).backing;

        let oy = screen_hsize(s) + (*data).cy - (*data).oy;
        if (*data).lineflag == line_sel::LINE_SEL_RIGHT_LEFT && oy == (*data).endsely {
            window_copy_other_end(wme);
        }

        (*data).cy = screen_size_y(&(*data).screen) - 1;
        (*data).cx = window_copy_find_length(wme, screen_hsize(s) + (*data).cy);
        (*data).oy = 0;

        if !(*data).searchmark.is_null() && (*data).timeout == 0 {
            window_copy_search_marks(wme, null_mut(), (*data).searchregex, 1);
        }
        window_copy_update_selection(wme, 1, 0);
        window_copy_cmd_action::WINDOW_COPY_CMD_REDRAW
    }
}

pub unsafe fn window_copy_cmd_history_top(
    cs: *mut window_copy_cmd_state,
) -> window_copy_cmd_action {
    unsafe {
        let wme: *mut window_mode_entry = (*cs).wme;
        let data: *mut window_copy_mode_data = (*wme).data.cast();

        let oy = screen_hsize((*data).backing) + (*data).cy - (*data).oy;
        if (*data).lineflag == line_sel::LINE_SEL_LEFT_RIGHT && oy == (*data).sely {
            window_copy_other_end(wme);
        }

        (*data).cy = 0;
        (*data).cx = 0;
        (*data).oy = screen_hsize((*data).backing);

        if !(*data).searchmark.is_null() && (*data).timeout == 0 {
            window_copy_search_marks(wme, null_mut(), (*data).searchregex, 1);
        }
        window_copy_update_selection(wme, 1, 0);
        window_copy_cmd_action::WINDOW_COPY_CMD_REDRAW
    }
}

pub unsafe fn window_copy_cmd_jump_again(cs: *mut window_copy_cmd_state) -> window_copy_cmd_action {
    unsafe {
        let wme: *mut window_mode_entry = (*cs).wme;
        let data: *mut window_copy_mode_data = (*wme).data.cast();
        let mut np = (*wme).prefix;

        match (*data).jumptype {
            window_copy::WINDOW_COPY_JUMPFORWARD => {
                while np != 0 {
                    window_copy_cursor_jump(wme);
                    np -= 1;
                }
            }
            window_copy::WINDOW_COPY_JUMPBACKWARD => {
                while np != 0 {
                    window_copy_cursor_jump_back(wme);
                    np -= 1;
                }
            }
            window_copy::WINDOW_COPY_JUMPTOFORWARD => {
                while np != 0 {
                    window_copy_cursor_jump_to(wme);
                    np -= 1;
                }
            }
            window_copy::WINDOW_COPY_JUMPTOBACKWARD => {
                while np != 0 {
                    window_copy_cursor_jump_to_back(wme);
                    np -= 1;
                }
            }
            _ => (),
        }
        window_copy_cmd_action::WINDOW_COPY_CMD_NOTHING
    }
}

pub unsafe fn window_copy_cmd_jump_reverse(
    cs: *mut window_copy_cmd_state,
) -> window_copy_cmd_action {
    unsafe {
        let wme: *mut window_mode_entry = (*cs).wme;
        let data: *mut window_copy_mode_data = (*wme).data.cast();
        let mut np = (*wme).prefix;

        match (*data).jumptype {
            window_copy::WINDOW_COPY_JUMPFORWARD => {
                while np != 0 {
                    window_copy_cursor_jump_back(wme);
                    np -= 1;
                }
            }
            window_copy::WINDOW_COPY_JUMPBACKWARD => {
                while np != 0 {
                    window_copy_cursor_jump(wme);
                    np -= 1;
                }
            }
            window_copy::WINDOW_COPY_JUMPTOFORWARD => {
                while np != 0 {
                    window_copy_cursor_jump_to_back(wme);
                    np -= 1;
                }
            }
            window_copy::WINDOW_COPY_JUMPTOBACKWARD => {
                while np != 0 {
                    window_copy_cursor_jump_to(wme);
                    np -= 1;
                }
            }
            _ => (),
        }
        window_copy_cmd_action::WINDOW_COPY_CMD_NOTHING
    }
}

pub unsafe fn window_copy_cmd_middle_line(
    cs: *mut window_copy_cmd_state,
) -> window_copy_cmd_action {
    unsafe {
        let wme: *mut window_mode_entry = (*cs).wme;
        let data: *mut window_copy_mode_data = (*wme).data.cast();

        (*data).cx = 0;
        (*data).cy = (screen_size_y(&raw mut (*data).screen) - 1) / 2;

        window_copy_update_selection(wme, 1, 0);
        window_copy_cmd_action::WINDOW_COPY_CMD_REDRAW
    }
}

pub unsafe fn window_copy_cmd_previous_matching_bracket(
    cs: *mut window_copy_cmd_state,
) -> window_copy_cmd_action {
    unsafe {
        let wme: *mut window_mode_entry = (*cs).wme;
        let mut np = (*wme).prefix;
        let data: *mut window_copy_mode_data = (*wme).data.cast();
        let s: *mut screen = (*data).backing;
        let open: [u8; 4] = [b'{', b'[', b'(', b'\0'];
        let close: [u8; 4] = [b'}', b']', b')', b'\0'];

        let mut found: u8 = b'\0';
        let mut cp: *mut u8 = null_mut();
        let mut gc: grid_cell = zeroed();
        let failed = false;

        'outer: while np != 0 {
            /* Get cursor position and line length. */
            let mut px = (*data).cx;
            let mut py = screen_hsize(s) + (*data).cy - (*data).oy;
            let mut xx = window_copy_find_length(wme, py);
            if xx == 0 {
                break;
            }

            /*
             * Get the current character. If not on a bracket, try the
             * previous. If still not, then behave like previous-word.
             */
            let mut tried = false;
            'retry: loop {
                grid_get_cell((*s).grid, px, py, &raw mut gc);
                if gc.data.size != 1 || gc.flags.intersects(grid_flag::PADDING) {
                    cp = null_mut();
                } else {
                    found = gc.data.data[0];
                    cp = libc::strchr((&raw const close).cast(), found as i32);
                }
                if cp.is_null() {
                    if (*data).modekeys == modekey::MODEKEY_EMACS {
                        if !tried && px > 0 {
                            px -= 1;
                            tried = true;
                            continue 'retry;
                        }
                        window_copy_cursor_previous_word(wme, (&raw const close).cast(), 1);
                    }

                    np -= 1;
                    continue 'outer;
                }
                let start = open[cp.offset_from_unsigned((&raw const close).cast())];

                /* Walk backward until the matching bracket is reached. */
                let mut n: u32 = 1;
                let mut failed = 0;
                loop {
                    if px == 0 {
                        if py == 0 {
                            failed = 1;
                            break;
                        }
                        loop {
                            py -= 1;
                            xx = window_copy_find_length(wme, py);
                            if !(xx == 0 && py > 0) {
                                break;
                            }
                        }
                        if xx == 0 && py == 0 {
                            failed = 1;
                            break;
                        }
                        px = xx - 1;
                    } else {
                        px -= 1;
                    }

                    grid_get_cell((*s).grid, px, py, &raw mut gc);
                    if gc.data.size == 1 && !gc.flags.intersects(grid_flag::PADDING) {
                        if gc.data.data[0] == found {
                            n += 1;
                        } else if gc.data.data[0] == start {
                            n -= 1;
                        }
                    }
                    if n == 0 {
                        break;
                    }
                }

                // Move the cursor to the found location if any.
                if failed == 0 {
                    window_copy_scroll_to(wme, px, py, false);
                }
                break;
            } // retry
            np -= 1;
        }

        window_copy_cmd_action::WINDOW_COPY_CMD_NOTHING
    }
}

pub unsafe fn window_copy_cmd_next_matching_bracket(
    cs: *mut window_copy_cmd_state,
) -> window_copy_cmd_action {
    unsafe {
        let wme: *mut window_mode_entry = (*cs).wme;
        let mut np = (*wme).prefix;
        let data: *mut window_copy_mode_data = (*wme).data.cast();
        let s: *mut screen = (*data).backing;
        let open: [u8; 4] = [b'{', b'[', b'(', b'\0'];
        let close: [u8; 4] = [b'}', b']', b')', b'\0'];

        let mut found = b'\0';
        let mut gc: grid_cell = zeroed();
        let mut cp = null_mut();

        'outer: while np != 0 {
            /* Get cursor position and line length. */
            let mut px = (*data).cx;
            let mut py = screen_hsize(s) + (*data).cy - (*data).oy;
            let mut xx = window_copy_find_length(wme, py);
            let yy = screen_hsize(s) + screen_size_y(s) - 1;
            if xx == 0 {
                break;
            }

            /*
             * Get the current character. If not on a bracket, try the
             * next. If still not, then behave like next-word.
             */
            let mut tried = false;
            'retry: loop {
                grid_get_cell((*s).grid, px, py, &raw mut gc);
                if gc.data.size != 1 || gc.flags.intersects(grid_flag::PADDING) {
                    cp = null_mut();
                } else {
                    found = gc.data.data[0];

                    /*
                     * In vi mode, attempt to move to previous bracket if a
                     * closing bracket is found first. If this fails,
                     * return to the original cursor position.
                     */
                    cp = libc::strchr((&raw const close).cast(), found as i32);
                    if !cp.is_null() && (*data).modekeys == modekey::MODEKEY_VI {
                        let sx = (*data).cx;
                        let sy = screen_hsize(s) + (*data).cy - (*data).oy;

                        window_copy_scroll_to(wme, px, py, false);
                        window_copy_cmd_previous_matching_bracket(cs);

                        px = (*data).cx;
                        py = screen_hsize(s) + (*data).cy - (*data).oy;
                        grid_get_cell((*s).grid, px, py, &raw mut gc);
                        if gc.data.size == 1
                            && !gc.flags.intersects(grid_flag::PADDING)
                            && !libc::strchr((&raw const close).cast(), gc.data.data[0] as i32)
                                .is_null()
                        {
                            window_copy_scroll_to(wme, sx, sy, false);
                        }
                        break;
                    }

                    cp = libc::strchr((&raw const open).cast(), found as i32);
                }
                if cp.is_null() {
                    if (*data).modekeys == modekey::MODEKEY_EMACS {
                        if !tried && px <= xx {
                            px += 1;
                            tried = true;
                            continue 'retry;
                        }
                        window_copy_cursor_next_word_end(wme, (&raw const open).cast(), 0);
                        np -= 1;
                        continue 'outer;
                    }
                    /* For vi, continue searching for bracket until EOL. */
                    if px > xx {
                        if py == yy {
                            np -= 1;
                            continue 'outer;
                        }
                        let gl = grid_get_line((*s).grid, py);
                        if !(*gl).flags.intersects(grid_line_flag::WRAPPED) {
                            np -= 1;
                            continue 'outer;
                        }
                        if (*gl).cellsize > (*(*s).grid).sx {
                            np -= 1;
                            continue 'outer;
                        }
                        px = 0;
                        py += 1;
                        xx = window_copy_find_length(wme, py);
                    } else {
                        px += 1;
                    }
                    continue 'retry;
                }
                let end = close[cp.offset_from_unsigned((&raw const open).cast())];

                /* Walk forward until the matching bracket is reached. */
                let mut n = 1;
                let mut failed = false;
                loop {
                    if px > xx {
                        if py == yy {
                            failed = true;
                            break;
                        }
                        px = 0;
                        py += 1;
                        xx = window_copy_find_length(wme, py);
                    } else {
                        px += 1;
                    }

                    grid_get_cell((*s).grid, px, py, &raw mut gc);
                    if gc.data.size == 1 && !gc.flags.intersects(grid_flag::PADDING) {
                        if gc.data.data[0] == found {
                            n += 1;
                        } else if gc.data.data[0] == end {
                            n -= 1;
                        }
                    }
                    if n == 0 {
                        break;
                    }
                }

                /* Move the cursor to the found location if any. */
                if !failed {
                    window_copy_scroll_to(wme, px, py, false);
                }
                break;
            }
            np -= 1;
        }

        window_copy_cmd_action::WINDOW_COPY_CMD_NOTHING
    }
}

pub unsafe fn window_copy_cmd_next_paragraph(
    cs: *mut window_copy_cmd_state,
) -> window_copy_cmd_action {
    unsafe {
        let wme: *mut window_mode_entry = (*cs).wme;
        let mut np = (*wme).prefix;

        while np != 0 {
            window_copy_next_paragraph(wme);
            np -= 1;
        }
        window_copy_cmd_action::WINDOW_COPY_CMD_NOTHING
    }
}

pub unsafe fn window_copy_cmd_next_space(cs: *mut window_copy_cmd_state) -> window_copy_cmd_action {
    unsafe {
        let wme: *mut window_mode_entry = (*cs).wme;
        let mut np = (*wme).prefix;

        while np != 0 {
            window_copy_cursor_next_word(wme, c!(""));
            np -= 1;
        }
        window_copy_cmd_action::WINDOW_COPY_CMD_NOTHING
    }
}

pub unsafe fn window_copy_cmd_next_space_end(
    cs: *mut window_copy_cmd_state,
) -> window_copy_cmd_action {
    unsafe {
        let wme: *mut window_mode_entry = (*cs).wme;
        let mut np: u32 = (*wme).prefix;

        while np != 0 {
            window_copy_cursor_next_word_end(wme, c!(""), 0);
            np -= 1;
        }
        window_copy_cmd_action::WINDOW_COPY_CMD_NOTHING
    }
}

pub unsafe fn window_copy_cmd_next_word(cs: *mut window_copy_cmd_state) -> window_copy_cmd_action {
    unsafe {
        let wme: *mut window_mode_entry = (*cs).wme;
        let mut np = (*wme).prefix;

        let separators = options_get_string_((*(*cs).s).options, c"word-separators");

        while np != 0 {
            window_copy_cursor_next_word(wme, separators);
            np -= 1;
        }
        window_copy_cmd_action::WINDOW_COPY_CMD_NOTHING
    }
}

pub unsafe fn window_copy_cmd_next_word_end(
    cs: *mut window_copy_cmd_state,
) -> window_copy_cmd_action {
    unsafe {
        let wme: *mut window_mode_entry = (*cs).wme;
        let mut np = (*wme).prefix;

        let separators = options_get_string_((*(*cs).s).options, c"word-separators");

        while np != 0 {
            window_copy_cursor_next_word_end(wme, separators, 0);
            np -= 1;
        }
        window_copy_cmd_action::WINDOW_COPY_CMD_NOTHING
    }
}

pub unsafe fn window_copy_cmd_other_end(cs: *mut window_copy_cmd_state) -> window_copy_cmd_action {
    unsafe {
        let wme: *mut window_mode_entry = (*cs).wme;
        let np = (*wme).prefix;
        let data: *mut window_copy_mode_data = (*wme).data.cast();

        (*data).selflag = selflag::SEL_CHAR;
        if !np.is_multiple_of(2) {
            window_copy_other_end(wme);
        }
        window_copy_cmd_action::WINDOW_COPY_CMD_NOTHING
    }
}

pub unsafe fn window_copy_cmd_page_down(cs: *mut window_copy_cmd_state) -> window_copy_cmd_action {
    unsafe {
        let wme: *mut window_mode_entry = (*cs).wme;
        let data: *mut window_copy_mode_data = (*wme).data.cast();
        let mut np = (*wme).prefix;

        while np != 0 {
            if window_copy_pagedown1(wme, 0, (*data).scroll_exit) != 0 {
                return window_copy_cmd_action::WINDOW_COPY_CMD_CANCEL;
            }
            np -= 1;
        }
        window_copy_cmd_action::WINDOW_COPY_CMD_NOTHING
    }
}

pub unsafe fn window_copy_cmd_page_down_and_cancel(
    cs: *mut window_copy_cmd_state,
) -> window_copy_cmd_action {
    unsafe {
        let wme: *mut window_mode_entry = (*cs).wme;
        let mut np = (*wme).prefix;

        while np != 0 {
            if window_copy_pagedown1(wme, 0, 1) != 0 {
                return window_copy_cmd_action::WINDOW_COPY_CMD_CANCEL;
            }
            np -= 1;
        }
        window_copy_cmd_action::WINDOW_COPY_CMD_NOTHING
    }
}

pub unsafe fn window_copy_cmd_page_up(cs: *mut window_copy_cmd_state) -> window_copy_cmd_action {
    unsafe {
        let wme: *mut window_mode_entry = (*cs).wme;
        let mut np = (*wme).prefix;

        while np != 0 {
            window_copy_pageup1(wme, 0);
            np -= 1;
        }
        window_copy_cmd_action::WINDOW_COPY_CMD_NOTHING
    }
}

pub unsafe fn window_copy_cmd_previous_paragraph(
    cs: *mut window_copy_cmd_state,
) -> window_copy_cmd_action {
    unsafe {
        let wme: *mut window_mode_entry = (*cs).wme;
        let mut np = (*wme).prefix;

        while np != 0 {
            window_copy_previous_paragraph(wme);
            np -= 1;
        }
        window_copy_cmd_action::WINDOW_COPY_CMD_NOTHING
    }
}

pub unsafe fn window_copy_cmd_previous_space(
    cs: *mut window_copy_cmd_state,
) -> window_copy_cmd_action {
    unsafe {
        let wme: *mut window_mode_entry = (*cs).wme;
        let mut np = (*wme).prefix;

        while np != 0 {
            window_copy_cursor_previous_word(wme, c!(""), 1);
            np -= 1;
        }
        window_copy_cmd_action::WINDOW_COPY_CMD_NOTHING
    }
}

pub unsafe fn window_copy_cmd_previous_word(
    cs: *mut window_copy_cmd_state,
) -> window_copy_cmd_action {
    unsafe {
        let wme: *mut window_mode_entry = (*cs).wme;
        let mut np = (*wme).prefix;

        let separators = options_get_string_((*(*cs).s).options, c"word-separators");

        while np != 0 {
            window_copy_cursor_previous_word(wme, separators, 1);
            np -= 1;
        }
        window_copy_cmd_action::WINDOW_COPY_CMD_NOTHING
    }
}

pub unsafe fn window_copy_cmd_rectangle_on(
    cs: *mut window_copy_cmd_state,
) -> window_copy_cmd_action {
    unsafe {
        let wme: *mut window_mode_entry = (*cs).wme;
        let data: *mut window_copy_mode_data = (*wme).data.cast();

        (*data).lineflag = line_sel::LINE_SEL_NONE;
        window_copy_rectangle_set(wme, 1);

        window_copy_cmd_action::WINDOW_COPY_CMD_NOTHING
    }
}

pub unsafe fn window_copy_cmd_rectangle_off(
    cs: *mut window_copy_cmd_state,
) -> window_copy_cmd_action {
    unsafe {
        let wme: *mut window_mode_entry = (*cs).wme;
        let data: *mut window_copy_mode_data = (*wme).data.cast();

        (*data).lineflag = line_sel::LINE_SEL_NONE;
        window_copy_rectangle_set(wme, 0);

        window_copy_cmd_action::WINDOW_COPY_CMD_NOTHING
    }
}

pub unsafe fn window_copy_cmd_rectangle_toggle(
    cs: *mut window_copy_cmd_state,
) -> window_copy_cmd_action {
    unsafe {
        let wme: *mut window_mode_entry = (*cs).wme;
        let data: *mut window_copy_mode_data = (*wme).data.cast();

        (*data).lineflag = line_sel::LINE_SEL_NONE;
        window_copy_rectangle_set(wme, (!(*data).rectflag));

        window_copy_cmd_action::WINDOW_COPY_CMD_NOTHING
    }
}

pub unsafe fn window_copy_cmd_scroll_down(
    cs: *mut window_copy_cmd_state,
) -> window_copy_cmd_action {
    unsafe {
        let wme: *mut window_mode_entry = (*cs).wme;
        let data: *mut window_copy_mode_data = (*wme).data.cast();
        let mut np = (*wme).prefix;

        while np != 0 {
            window_copy_cursor_down(wme, 1);
            np -= 1;
        }
        if (*data).scroll_exit != 0 && (*data).oy == 0 {
            window_copy_cmd_action::WINDOW_COPY_CMD_CANCEL
        } else {
            window_copy_cmd_action::WINDOW_COPY_CMD_NOTHING
        }
    }
}

pub unsafe fn window_copy_cmd_scroll_down_and_cancel(
    cs: *mut window_copy_cmd_state,
) -> window_copy_cmd_action {
    unsafe {
        let wme: *mut window_mode_entry = (*cs).wme;
        let data: *mut window_copy_mode_data = (*wme).data.cast();
        let mut np = (*wme).prefix;

        while np != 0 {
            window_copy_cursor_down(wme, 1);
            np -= 1;
        }
        if (*data).oy == 0 {
            window_copy_cmd_action::WINDOW_COPY_CMD_CANCEL
        } else {
            window_copy_cmd_action::WINDOW_COPY_CMD_NOTHING
        }
    }
}

pub unsafe fn window_copy_cmd_scroll_up(cs: *mut window_copy_cmd_state) -> window_copy_cmd_action {
    unsafe {
        let wme: *mut window_mode_entry = (*cs).wme;
        let mut np = (*wme).prefix;

        while np != 0 {
            window_copy_cursor_up(wme, 1);
            np -= 1;
        }
        window_copy_cmd_action::WINDOW_COPY_CMD_NOTHING
    }
}

pub unsafe fn window_copy_cmd_search_again(
    cs: *mut window_copy_cmd_state,
) -> window_copy_cmd_action {
    unsafe {
        let wme: *mut window_mode_entry = (*cs).wme;
        let data: *mut window_copy_mode_data = (*wme).data.cast();
        let mut np = (*wme).prefix;

        if (*data).searchtype == window_copy::WINDOW_COPY_SEARCHUP {
            while np != 0 {
                window_copy_search_up(wme, (*data).searchregex);
                np -= 1;
            }
        } else if (*data).searchtype == window_copy::WINDOW_COPY_SEARCHDOWN {
            while np != 0 {
                window_copy_search_down(wme, (*data).searchregex);
                np -= 1;
            }
        }
        window_copy_cmd_action::WINDOW_COPY_CMD_NOTHING
    }
}

pub unsafe fn window_copy_cmd_search_reverse(
    cs: *mut window_copy_cmd_state,
) -> window_copy_cmd_action {
    unsafe {
        let wme: *mut window_mode_entry = (*cs).wme;
        let data: *mut window_copy_mode_data = (*wme).data.cast();
        let mut np = (*wme).prefix;

        if (*data).searchtype == window_copy::WINDOW_COPY_SEARCHUP {
            while np != 0 {
                window_copy_search_down(wme, (*data).searchregex);
                np -= 1;
            }
        } else if (*data).searchtype == window_copy::WINDOW_COPY_SEARCHDOWN {
            while np != 0 {
                window_copy_search_up(wme, (*data).searchregex);
                np -= 1;
            }
        }
        window_copy_cmd_action::WINDOW_COPY_CMD_NOTHING
    }
}

pub unsafe fn window_copy_cmd_select_line(
    cs: *mut window_copy_cmd_state,
) -> window_copy_cmd_action {
    unsafe {
        let wme: *mut window_mode_entry = (*cs).wme;
        let data: *mut window_copy_mode_data = (*wme).data.cast();
        let mut np = (*wme).prefix;

        (*data).lineflag = line_sel::LINE_SEL_LEFT_RIGHT;
        (*data).rectflag = 0;
        (*data).selflag = selflag::SEL_LINE;
        (*data).dx = (*data).cx;
        (*data).dy = screen_hsize((*data).backing) + (*data).cy - (*data).oy;

        window_copy_cursor_start_of_line(wme);
        (*data).selrx = (*data).cx;
        (*data).selry = screen_hsize((*data).backing) + (*data).cy - (*data).oy;
        (*data).endselry = (*data).selry;
        window_copy_start_selection(wme);
        window_copy_cursor_end_of_line(wme);
        (*data).endselry = screen_hsize((*data).backing) + (*data).cy - (*data).oy;
        (*data).endselrx = window_copy_find_length(wme, (*data).endselry);
        while np != 0 {
            window_copy_cursor_down(wme, 0);
            window_copy_cursor_end_of_line(wme);
            np -= 1;
        }

        window_copy_cmd_action::WINDOW_COPY_CMD_REDRAW
    }
}

pub unsafe fn window_copy_cmd_select_word(
    cs: *mut window_copy_cmd_state,
) -> window_copy_cmd_action {
    unsafe {
        let wme: *mut window_mode_entry = (*cs).wme;
        let session_options: *mut options = (*(*cs).s).options;
        let data: *mut window_copy_mode_data = (*wme).data.cast();
        // u_int px, py, nextx, nexty;

        (*data).lineflag = line_sel::LINE_SEL_LEFT_RIGHT;
        (*data).rectflag = 0;
        (*data).selflag = selflag::SEL_WORD;
        (*data).dx = (*data).cx;
        (*data).dy = screen_hsize((*data).backing) + (*data).cy - (*data).oy;

        (*data).separators = options_get_string_(session_options, c"word-separators");
        window_copy_cursor_previous_word(wme, (*data).separators, 0);
        let px = (*data).cx;
        let py = screen_hsize((*data).backing) + (*data).cy - (*data).oy;
        (*data).selrx = px;
        (*data).selry = py;
        window_copy_start_selection(wme);

        /* Handle single character words. */
        let mut nextx = px + 1;
        let mut nexty = py;
        if (*grid_get_line((*(*data).backing).grid, nexty))
            .flags
            .intersects(grid_line_flag::WRAPPED)
            && nextx > screen_size_x((*data).backing) - 1
        {
            nextx = 0;
            nexty += 1;
        }
        if px >= window_copy_find_length(wme, py)
            || window_copy_in_set(wme, nextx, nexty, WHITESPACE) == 0
        {
            window_copy_cursor_next_word_end(wme, (*data).separators, 1);
        } else {
            window_copy_update_cursor(wme, px, (*data).cy);
            if window_copy_update_selection(wme, 1, 1) != 0 {
                window_copy_redraw_lines(wme, (*data).cy, 1);
            }
        }
        (*data).endselrx = (*data).cx;
        (*data).endselry = screen_hsize((*data).backing) + (*data).cy - (*data).oy;
        if (*data).dy > (*data).endselry {
            (*data).dy = (*data).endselry;
            (*data).dx = (*data).endselrx;
        } else if (*data).dx > (*data).endselrx {
            (*data).dx = (*data).endselrx;
        }

        window_copy_cmd_action::WINDOW_COPY_CMD_REDRAW
    }
}

pub unsafe fn window_copy_cmd_set_mark(cs: *mut window_copy_cmd_state) -> window_copy_cmd_action {
    unsafe {
        let data: *mut window_copy_mode_data = (*(*cs).wme).data.cast();

        (*data).mx = (*data).cx;
        (*data).my = screen_hsize((*data).backing) + (*data).cy - (*data).oy;
        (*data).showmark = 1;
        window_copy_cmd_action::WINDOW_COPY_CMD_REDRAW
    }
}

pub unsafe fn window_copy_cmd_start_of_line(
    cs: *mut window_copy_cmd_state,
) -> window_copy_cmd_action {
    unsafe {
        window_copy_cursor_start_of_line((*cs).wme);
        window_copy_cmd_action::WINDOW_COPY_CMD_NOTHING
    }
}

pub unsafe fn window_copy_cmd_top_line(cs: *mut window_copy_cmd_state) -> window_copy_cmd_action {
    unsafe {
        let wme: *mut window_mode_entry = (*cs).wme;
        let data: *mut window_copy_mode_data = (*wme).data.cast();

        (*data).cx = 0;
        (*data).cy = 0;

        window_copy_update_selection(wme, 1, 0);
        window_copy_cmd_action::WINDOW_COPY_CMD_REDRAW
    }
}

pub unsafe fn window_copy_cmd_copy_pipe_no_clear(
    cs: *mut window_copy_cmd_state,
) -> window_copy_cmd_action {
    unsafe {
        let wme: *mut window_mode_entry = (*cs).wme;
        let c: *mut client = (*cs).c;
        let s: *mut session = (*cs).s;
        let wl: *mut winlink = (*cs).wl;
        let wp: *mut window_pane = (*wme).wp;
        let mut command = null_mut();
        let mut prefix = null_mut();
        let arg1 = args_string((*cs).args, 1);
        let arg2 = args_string((*cs).args, 2);

        if !arg2.is_null() {
            prefix = format_single(null_mut(), arg2, c, s, wl, wp);
        }

        if !s.is_null() && !arg1.is_null() && *arg1 != b'\0' {
            command = format_single(null_mut(), arg1, c, s, wl, wp);
        }
        window_copy_copy_pipe(wme, s, prefix, command);
        free_(command);

        free_(prefix);
        window_copy_cmd_action::WINDOW_COPY_CMD_NOTHING
    }
}

pub unsafe fn window_copy_cmd_copy_pipe(cs: *mut window_copy_cmd_state) -> window_copy_cmd_action {
    unsafe {
        let wme: *mut window_mode_entry = (*cs).wme;

        window_copy_cmd_copy_pipe_no_clear(cs);
        window_copy_clear_selection(wme);
        window_copy_cmd_action::WINDOW_COPY_CMD_REDRAW
    }
}

pub unsafe fn window_copy_cmd_copy_pipe_and_cancel(
    cs: *mut window_copy_cmd_state,
) -> window_copy_cmd_action {
    unsafe {
        let wme: *mut window_mode_entry = (*cs).wme;

        window_copy_cmd_copy_pipe_no_clear(cs);
        window_copy_clear_selection(wme);
        window_copy_cmd_action::WINDOW_COPY_CMD_CANCEL
    }
}

pub unsafe fn window_copy_cmd_pipe_no_clear(
    cs: *mut window_copy_cmd_state,
) -> window_copy_cmd_action {
    unsafe {
        let wme: *mut window_mode_entry = (*cs).wme;
        let c: *mut client = (*cs).c;
        let s: *mut session = (*cs).s;
        let wl: *mut winlink = (*cs).wl;
        let wp: *mut window_pane = (*wme).wp;
        let mut command = null_mut();
        let arg1 = args_string((*cs).args, 1);

        if !s.is_null() && !arg1.is_null() && *arg1 != b'\0' {
            command = format_single(null_mut(), arg1, c, s, wl, wp);
        }
        window_copy_pipe(wme, s, command);
        free_(command);

        window_copy_cmd_action::WINDOW_COPY_CMD_NOTHING
    }
}

pub unsafe fn window_copy_cmd_pipe(cs: *mut window_copy_cmd_state) -> window_copy_cmd_action {
    unsafe {
        let wme: *mut window_mode_entry = (*cs).wme;

        window_copy_cmd_pipe_no_clear(cs);
        window_copy_clear_selection(wme);
        window_copy_cmd_action::WINDOW_COPY_CMD_REDRAW
    }
}

pub unsafe fn window_copy_cmd_pipe_and_cancel(
    cs: *mut window_copy_cmd_state,
) -> window_copy_cmd_action {
    unsafe {
        let wme: *mut window_mode_entry = (*cs).wme;

        window_copy_cmd_pipe_no_clear(cs);
        window_copy_clear_selection(wme);
        window_copy_cmd_action::WINDOW_COPY_CMD_CANCEL
    }
}

pub unsafe fn window_copy_cmd_goto_line(cs: *mut window_copy_cmd_state) -> window_copy_cmd_action {
    unsafe {
        let wme: *mut window_mode_entry = (*cs).wme;
        let arg1 = args_string((*cs).args, 1);

        if *arg1 != b'\0' {
            window_copy_goto_line(wme, arg1);
        }
        window_copy_cmd_action::WINDOW_COPY_CMD_NOTHING
    }
}

pub unsafe fn window_copy_cmd_jump_backward(
    cs: *mut window_copy_cmd_state,
) -> window_copy_cmd_action {
    unsafe {
        let wme: *mut window_mode_entry = (*cs).wme;
        let data: *mut window_copy_mode_data = (*wme).data.cast();
        let mut np = (*wme).prefix;
        let arg1 = args_string((*cs).args, 1);

        if *arg1 != b'\0' {
            (*data).jumptype = window_copy::WINDOW_COPY_JUMPBACKWARD;
            (*data).jumpchar = Some(Box::new(utf8_fromcstr(arg1)[0]));
            while np != 0 {
                window_copy_cursor_jump_back(wme);
                np -= 1;
            }
        }
        window_copy_cmd_action::WINDOW_COPY_CMD_NOTHING
    }
}

pub unsafe fn window_copy_cmd_jump_forward(
    cs: *mut window_copy_cmd_state,
) -> window_copy_cmd_action {
    unsafe {
        let wme: *mut window_mode_entry = (*cs).wme;
        let data: *mut window_copy_mode_data = (*wme).data.cast();
        let mut np = (*wme).prefix;
        let arg1 = args_string((*cs).args, 1);

        if *arg1 != b'\0' {
            (*data).jumptype = window_copy::WINDOW_COPY_JUMPFORWARD;
            (*data).jumpchar = Some(Box::new(utf8_fromcstr(arg1)[0]));
            while np != 0 {
                window_copy_cursor_jump(wme);
                np -= 1;
            }
        }
        window_copy_cmd_action::WINDOW_COPY_CMD_NOTHING
    }
}

pub unsafe fn window_copy_cmd_jump_to_backward(
    cs: *mut window_copy_cmd_state,
) -> window_copy_cmd_action {
    unsafe {
        let wme: *mut window_mode_entry = (*cs).wme;
        let data: *mut window_copy_mode_data = (*wme).data.cast();
        let mut np = (*wme).prefix;
        let arg1 = args_string((*cs).args, 1);

        if *arg1 != b'\0' {
            (*data).jumptype = window_copy::WINDOW_COPY_JUMPTOBACKWARD;
            (*data).jumpchar = Some(Box::new(utf8_fromcstr(arg1)[0]));
            while np != 0 {
                window_copy_cursor_jump_to_back(wme);
                np -= 1;
            }
        }
        window_copy_cmd_action::WINDOW_COPY_CMD_NOTHING
    }
}

pub unsafe fn window_copy_cmd_jump_to_forward(
    cs: *mut window_copy_cmd_state,
) -> window_copy_cmd_action {
    unsafe {
        let wme: *mut window_mode_entry = (*cs).wme;
        let data: *mut window_copy_mode_data = (*wme).data.cast();
        let mut np = (*wme).prefix;
        let arg1 = args_string((*cs).args, 1);

        if *arg1 != b'\0' {
            (*data).jumptype = window_copy::WINDOW_COPY_JUMPTOFORWARD;
            (*data).jumpchar = Some(Box::new(utf8_fromcstr(arg1)[0]));
            while np != 0 {
                window_copy_cursor_jump_to(wme);
                np -= 1;
            }
        }
        window_copy_cmd_action::WINDOW_COPY_CMD_NOTHING
    }
}

pub unsafe fn window_copy_cmd_jump_to_mark(
    cs: *mut window_copy_cmd_state,
) -> window_copy_cmd_action {
    unsafe {
        let wme: *mut window_mode_entry = (*cs).wme;

        window_copy_jump_to_mark(wme);
        window_copy_cmd_action::WINDOW_COPY_CMD_NOTHING
    }
}

pub unsafe fn window_copy_cmd_next_prompt(
    cs: *mut window_copy_cmd_state,
) -> window_copy_cmd_action {
    unsafe {
        let wme: *mut window_mode_entry = (*cs).wme;
        let arg1 = args_string((*cs).args, 1);

        window_copy_cursor_prompt(wme, 1, arg1);
        window_copy_cmd_action::WINDOW_COPY_CMD_NOTHING
    }
}

pub unsafe fn window_copy_cmd_previous_prompt(
    cs: *mut window_copy_cmd_state,
) -> window_copy_cmd_action {
    unsafe {
        let wme: *mut window_mode_entry = (*cs).wme;
        let arg1 = args_string((*cs).args, 1);

        window_copy_cursor_prompt(wme, 0, arg1);
        window_copy_cmd_action::WINDOW_COPY_CMD_NOTHING
    }
}

pub unsafe fn window_copy_cmd_search_backward(
    cs: *mut window_copy_cmd_state,
) -> window_copy_cmd_action {
    unsafe {
        let wme: *mut window_mode_entry = (*cs).wme;
        let data: *mut window_copy_mode_data = (*wme).data.cast();
        let mut np = (*wme).prefix;

        if window_copy_expand_search_string(cs) == 0 {
            return window_copy_cmd_action::WINDOW_COPY_CMD_NOTHING;
        }

        if !(*data).searchstr.is_null() {
            (*data).searchtype = window_copy::WINDOW_COPY_SEARCHUP;
            (*data).searchregex = 1;
            (*data).timeout = 0;
            while np != 0 {
                window_copy_search_up(wme, 1);
                np -= 1;
            }
        }
        window_copy_cmd_action::WINDOW_COPY_CMD_NOTHING
    }
}

pub unsafe fn window_copy_cmd_search_backward_text(
    cs: *mut window_copy_cmd_state,
) -> window_copy_cmd_action {
    unsafe {
        let wme: *mut window_mode_entry = (*cs).wme;
        let data: *mut window_copy_mode_data = (*wme).data.cast();
        let mut np = (*wme).prefix;

        if window_copy_expand_search_string(cs) == 0 {
            return window_copy_cmd_action::WINDOW_COPY_CMD_NOTHING;
        }

        if !(*data).searchstr.is_null() {
            (*data).searchtype = window_copy::WINDOW_COPY_SEARCHUP;
            (*data).searchregex = 0;
            (*data).timeout = 0;
            while np != 0 {
                window_copy_search_up(wme, 0);
                np -= 1;
            }
        }
        window_copy_cmd_action::WINDOW_COPY_CMD_NOTHING
    }
}

pub unsafe fn window_copy_cmd_search_forward(
    cs: *mut window_copy_cmd_state,
) -> window_copy_cmd_action {
    unsafe {
        let wme: *mut window_mode_entry = (*cs).wme;
        let data: *mut window_copy_mode_data = (*wme).data.cast();
        let mut np = (*wme).prefix;

        if window_copy_expand_search_string(cs) == 0 {
            return window_copy_cmd_action::WINDOW_COPY_CMD_NOTHING;
        }

        if !(*data).searchstr.is_null() {
            (*data).searchtype = window_copy::WINDOW_COPY_SEARCHDOWN;
            (*data).searchregex = 1;
            (*data).timeout = 0;
            while np != 0 {
                window_copy_search_down(wme, 1);
                np -= 1;
            }
        }
        window_copy_cmd_action::WINDOW_COPY_CMD_NOTHING
    }
}

pub unsafe fn window_copy_cmd_search_forward_text(
    cs: *mut window_copy_cmd_state,
) -> window_copy_cmd_action {
    unsafe {
        let wme: *mut window_mode_entry = (*cs).wme;
        let data: *mut window_copy_mode_data = (*wme).data.cast();
        let mut np = (*wme).prefix;

        if window_copy_expand_search_string(cs) == 0 {
            return window_copy_cmd_action::WINDOW_COPY_CMD_NOTHING;
        }

        if !(*data).searchstr.is_null() {
            (*data).searchtype = window_copy::WINDOW_COPY_SEARCHDOWN;
            (*data).searchregex = 0;
            (*data).timeout = 0;
            while np != 0 {
                window_copy_search_down(wme, 0);
                np -= 1;
            }
        }
        window_copy_cmd_action::WINDOW_COPY_CMD_NOTHING
    }
}

pub unsafe fn window_copy_cmd_search_backward_incremental(
    cs: *mut window_copy_cmd_state,
) -> window_copy_cmd_action {
    unsafe {
        let wme: *mut window_mode_entry = (*cs).wme;
        let data: *mut window_copy_mode_data = (*wme).data.cast();
        let mut arg1 = args_string((*cs).args, 1);
        let ss = (*data).searchstr;
        let mut action = window_copy_cmd_action::WINDOW_COPY_CMD_NOTHING;

        (*data).timeout = 0;

        // log_debug("%s: %s", __func__, arg1);

        let prefix = *arg1;
        arg1 = arg1.add(1);
        if (*data).searchx == -1 || (*data).searchy == -1 {
            (*data).searchx = (*data).cx as i32;
            (*data).searchy = (*data).cy as i32;
            (*data).searcho = (*data).oy as i32;
        } else if !ss.is_null() && libc::strcmp(arg1, ss) != 0 {
            (*data).cx = (*data).searchx as u32;
            (*data).cy = (*data).searchy as u32;
            (*data).oy = (*data).searcho as u32;
            action = window_copy_cmd_action::WINDOW_COPY_CMD_REDRAW;
        }
        if *arg1 == b'\0' {
            window_copy_clear_marks(wme);
            return window_copy_cmd_action::WINDOW_COPY_CMD_REDRAW;
        }
        match prefix as u8 {
            b'=' | b'-' => {
                (*data).searchtype = window_copy::WINDOW_COPY_SEARCHUP;
                (*data).searchregex = 0;
                free_((*data).searchstr);
                (*data).searchstr = xstrdup(arg1).as_ptr();
                if window_copy_search_up(wme, 0) == 0 {
                    window_copy_clear_marks(wme);
                    return window_copy_cmd_action::WINDOW_COPY_CMD_REDRAW;
                }
            }
            b'+' => {
                (*data).searchtype = window_copy::WINDOW_COPY_SEARCHDOWN;
                (*data).searchregex = 0;
                free_((*data).searchstr);
                (*data).searchstr = xstrdup(arg1).as_ptr();
                if window_copy_search_down(wme, 0) == 0 {
                    window_copy_clear_marks(wme);
                    return window_copy_cmd_action::WINDOW_COPY_CMD_REDRAW;
                }
            }
            _ => (),
        }
        action
    }
}

pub unsafe fn window_copy_cmd_search_forward_incremental(
    cs: *mut window_copy_cmd_state,
) -> window_copy_cmd_action {
    unsafe {
        let wme: *mut window_mode_entry = (*cs).wme;
        let data: *mut window_copy_mode_data = (*wme).data.cast();
        let mut arg1 = args_string((*cs).args, 1);
        let ss = (*data).searchstr;
        let mut action = window_copy_cmd_action::WINDOW_COPY_CMD_NOTHING;

        (*data).timeout = 0;

        // log_debug("%s: %s", __func__, arg1);

        let prefix = *arg1;
        arg1 = arg1.add(1);
        if (*data).searchx == -1 || (*data).searchy == -1 {
            (*data).searchx = (*data).cx as i32;
            (*data).searchy = (*data).cy as i32;
            (*data).searcho = (*data).oy as i32;
        } else if !ss.is_null() && libc::strcmp(arg1, ss) != 0 {
            (*data).cx = (*data).searchx as u32;
            (*data).cy = (*data).searchy as u32;
            (*data).oy = (*data).searcho as u32;
            action = window_copy_cmd_action::WINDOW_COPY_CMD_REDRAW;
        }
        if *arg1 == b'\0' {
            window_copy_clear_marks(wme);
            return window_copy_cmd_action::WINDOW_COPY_CMD_REDRAW;
        }
        match prefix as u8 {
            b'=' | b'+' => {
                (*data).searchtype = window_copy::WINDOW_COPY_SEARCHDOWN;
                (*data).searchregex = 0;
                free_((*data).searchstr);
                (*data).searchstr = xstrdup(arg1).as_ptr();
                if window_copy_search_down(wme, 0) == 0 {
                    window_copy_clear_marks(wme);
                    return window_copy_cmd_action::WINDOW_COPY_CMD_REDRAW;
                }
            }
            b'-' => {
                (*data).searchtype = window_copy::WINDOW_COPY_SEARCHUP;
                (*data).searchregex = 0;
                free_((*data).searchstr);
                (*data).searchstr = xstrdup(arg1).as_ptr();
                if window_copy_search_up(wme, 0) == 0 {
                    window_copy_clear_marks(wme);
                    return window_copy_cmd_action::WINDOW_COPY_CMD_REDRAW;
                }
            }
            _ => (),
        }
        action
    }
}

pub unsafe fn window_copy_cmd_refresh_from_pane(
    cs: *mut window_copy_cmd_state,
) -> window_copy_cmd_action {
    unsafe {
        let wme: *mut window_mode_entry = (*cs).wme;
        let wp: *mut window_pane = (*wme).swp;
        let data: *mut window_copy_mode_data = (*wme).data.cast();

        if (*data).viewmode != 0 {
            return window_copy_cmd_action::WINDOW_COPY_CMD_NOTHING;
        }

        screen_free((*data).backing);
        free_((*data).backing);
        (*data).backing = window_copy_clone_screen(
            &raw mut (*wp).base,
            &raw mut (*data).screen,
            null_mut(),
            null_mut(),
            ((*wme).swp != (*wme).wp) as i32,
        );

        window_copy_size_changed(wme);
        window_copy_cmd_action::WINDOW_COPY_CMD_REDRAW
    }
}

#[repr(C)]
struct window_copy_cmd_table_entry {
    command: SyncCharPtr,
    minargs: u32,
    maxargs: u32,
    clear: window_copy_cmd_clear,
    f: unsafe fn(*mut window_copy_cmd_state) -> window_copy_cmd_action,
}

static WINDOW_COPY_CMD_TABLE: [window_copy_cmd_table_entry; 85] = [
    window_copy_cmd_table_entry {
        command: SyncCharPtr::new(c"append-selection"),
        minargs: 0,
        maxargs: 0,
        clear: window_copy_cmd_clear::WINDOW_COPY_CMD_CLEAR_ALWAYS,
        f: window_copy_cmd_append_selection,
    },
    window_copy_cmd_table_entry {
        command: SyncCharPtr::new(c"append-selection-and-cancel"),
        minargs: 0,
        maxargs: 0,
        clear: window_copy_cmd_clear::WINDOW_COPY_CMD_CLEAR_ALWAYS,
        f: window_copy_cmd_append_selection_and_cancel,
    },
    window_copy_cmd_table_entry {
        command: SyncCharPtr::new(c"back-to-indentation"),
        minargs: 0,
        maxargs: 0,
        clear: window_copy_cmd_clear::WINDOW_COPY_CMD_CLEAR_ALWAYS,
        f: window_copy_cmd_back_to_indentation,
    },
    window_copy_cmd_table_entry {
        command: SyncCharPtr::new(c"begin-selection"),
        minargs: 0,
        maxargs: 0,
        clear: window_copy_cmd_clear::WINDOW_COPY_CMD_CLEAR_ALWAYS,
        f: window_copy_cmd_begin_selection,
    },
    window_copy_cmd_table_entry {
        command: SyncCharPtr::new(c"bottom-line"),
        minargs: 0,
        maxargs: 0,
        clear: window_copy_cmd_clear::WINDOW_COPY_CMD_CLEAR_EMACS_ONLY,
        f: window_copy_cmd_bottom_line,
    },
    window_copy_cmd_table_entry {
        command: SyncCharPtr::new(c"cancel"),
        minargs: 0,
        maxargs: 0,
        clear: window_copy_cmd_clear::WINDOW_COPY_CMD_CLEAR_ALWAYS,
        f: window_copy_cmd_cancel,
    },
    window_copy_cmd_table_entry {
        command: SyncCharPtr::new(c"clear-selection"),
        minargs: 0,
        maxargs: 0,
        clear: window_copy_cmd_clear::WINDOW_COPY_CMD_CLEAR_ALWAYS,
        f: window_copy_cmd_clear_selection,
    },
    window_copy_cmd_table_entry {
        command: SyncCharPtr::new(c"copy-end-of-line"),
        minargs: 0,
        maxargs: 1,
        clear: window_copy_cmd_clear::WINDOW_COPY_CMD_CLEAR_ALWAYS,
        f: window_copy_cmd_copy_end_of_line,
    },
    window_copy_cmd_table_entry {
        command: SyncCharPtr::new(c"copy-end-of-line-and-cancel"),
        minargs: 0,
        maxargs: 1,
        clear: window_copy_cmd_clear::WINDOW_COPY_CMD_CLEAR_ALWAYS,
        f: window_copy_cmd_copy_end_of_line_and_cancel,
    },
    window_copy_cmd_table_entry {
        command: SyncCharPtr::new(c"copy-pipe-end-of-line"),
        minargs: 0,
        maxargs: 2,
        clear: window_copy_cmd_clear::WINDOW_COPY_CMD_CLEAR_ALWAYS,
        f: window_copy_cmd_copy_pipe_end_of_line,
    },
    window_copy_cmd_table_entry {
        command: SyncCharPtr::new(c"copy-pipe-end-of-line-and-cancel"),
        minargs: 0,
        maxargs: 2,
        clear: window_copy_cmd_clear::WINDOW_COPY_CMD_CLEAR_ALWAYS,
        f: window_copy_cmd_copy_pipe_end_of_line_and_cancel,
    },
    window_copy_cmd_table_entry {
        command: SyncCharPtr::new(c"copy-line"),
        minargs: 0,
        maxargs: 1,
        clear: window_copy_cmd_clear::WINDOW_COPY_CMD_CLEAR_ALWAYS,
        f: window_copy_cmd_copy_line,
    },
    window_copy_cmd_table_entry {
        command: SyncCharPtr::new(c"copy-line-and-cancel"),
        minargs: 0,
        maxargs: 1,
        clear: window_copy_cmd_clear::WINDOW_COPY_CMD_CLEAR_ALWAYS,
        f: window_copy_cmd_copy_line_and_cancel,
    },
    window_copy_cmd_table_entry {
        command: SyncCharPtr::new(c"copy-pipe-line"),
        minargs: 0,
        maxargs: 2,
        clear: window_copy_cmd_clear::WINDOW_COPY_CMD_CLEAR_ALWAYS,
        f: window_copy_cmd_copy_pipe_line,
    },
    window_copy_cmd_table_entry {
        command: SyncCharPtr::new(c"copy-pipe-line-and-cancel"),
        minargs: 0,
        maxargs: 2,
        clear: window_copy_cmd_clear::WINDOW_COPY_CMD_CLEAR_ALWAYS,
        f: window_copy_cmd_copy_pipe_line_and_cancel,
    },
    window_copy_cmd_table_entry {
        command: SyncCharPtr::new(c"copy-pipe-no-clear"),
        minargs: 0,
        maxargs: 2,
        clear: window_copy_cmd_clear::WINDOW_COPY_CMD_CLEAR_NEVER,
        f: window_copy_cmd_copy_pipe_no_clear,
    },
    window_copy_cmd_table_entry {
        command: SyncCharPtr::new(c"copy-pipe"),
        minargs: 0,
        maxargs: 2,
        clear: window_copy_cmd_clear::WINDOW_COPY_CMD_CLEAR_ALWAYS,
        f: window_copy_cmd_copy_pipe,
    },
    window_copy_cmd_table_entry {
        command: SyncCharPtr::new(c"copy-pipe-and-cancel"),
        minargs: 0,
        maxargs: 2,
        clear: window_copy_cmd_clear::WINDOW_COPY_CMD_CLEAR_ALWAYS,
        f: window_copy_cmd_copy_pipe_and_cancel,
    },
    window_copy_cmd_table_entry {
        command: SyncCharPtr::new(c"copy-selection-no-clear"),
        minargs: 0,
        maxargs: 1,
        clear: window_copy_cmd_clear::WINDOW_COPY_CMD_CLEAR_NEVER,
        f: window_copy_cmd_copy_selection_no_clear,
    },
    window_copy_cmd_table_entry {
        command: SyncCharPtr::new(c"copy-selection"),
        minargs: 0,
        maxargs: 1,
        clear: window_copy_cmd_clear::WINDOW_COPY_CMD_CLEAR_ALWAYS,
        f: window_copy_cmd_copy_selection,
    },
    window_copy_cmd_table_entry {
        command: SyncCharPtr::new(c"copy-selection-and-cancel"),
        minargs: 0,
        maxargs: 1,
        clear: window_copy_cmd_clear::WINDOW_COPY_CMD_CLEAR_ALWAYS,
        f: window_copy_cmd_copy_selection_and_cancel,
    },
    window_copy_cmd_table_entry {
        command: SyncCharPtr::new(c"cursor-down"),
        minargs: 0,
        maxargs: 0,
        clear: window_copy_cmd_clear::WINDOW_COPY_CMD_CLEAR_EMACS_ONLY,
        f: window_copy_cmd_cursor_down,
    },
    window_copy_cmd_table_entry {
        command: SyncCharPtr::new(c"cursor-down-and-cancel"),
        minargs: 0,
        maxargs: 0,
        clear: window_copy_cmd_clear::WINDOW_COPY_CMD_CLEAR_ALWAYS,
        f: window_copy_cmd_cursor_down_and_cancel,
    },
    window_copy_cmd_table_entry {
        command: SyncCharPtr::new(c"cursor-left"),
        minargs: 0,
        maxargs: 0,
        clear: window_copy_cmd_clear::WINDOW_COPY_CMD_CLEAR_EMACS_ONLY,
        f: window_copy_cmd_cursor_left,
    },
    window_copy_cmd_table_entry {
        command: SyncCharPtr::new(c"cursor-right"),
        minargs: 0,
        maxargs: 0,
        clear: window_copy_cmd_clear::WINDOW_COPY_CMD_CLEAR_EMACS_ONLY,
        f: window_copy_cmd_cursor_right,
    },
    window_copy_cmd_table_entry {
        command: SyncCharPtr::new(c"cursor-up"),
        minargs: 0,
        maxargs: 0,
        clear: window_copy_cmd_clear::WINDOW_COPY_CMD_CLEAR_EMACS_ONLY,
        f: window_copy_cmd_cursor_up,
    },
    window_copy_cmd_table_entry {
        command: SyncCharPtr::new(c"end-of-line"),
        minargs: 0,
        maxargs: 0,
        clear: window_copy_cmd_clear::WINDOW_COPY_CMD_CLEAR_EMACS_ONLY,
        f: window_copy_cmd_end_of_line,
    },
    window_copy_cmd_table_entry {
        command: SyncCharPtr::new(c"goto-line"),
        minargs: 1,
        maxargs: 1,
        clear: window_copy_cmd_clear::WINDOW_COPY_CMD_CLEAR_EMACS_ONLY,
        f: window_copy_cmd_goto_line,
    },
    window_copy_cmd_table_entry {
        command: SyncCharPtr::new(c"halfpage-down"),
        minargs: 0,
        maxargs: 0,
        clear: window_copy_cmd_clear::WINDOW_COPY_CMD_CLEAR_EMACS_ONLY,
        f: window_copy_cmd_halfpage_down,
    },
    window_copy_cmd_table_entry {
        command: SyncCharPtr::new(c"halfpage-down-and-cancel"),
        minargs: 0,
        maxargs: 0,
        clear: window_copy_cmd_clear::WINDOW_COPY_CMD_CLEAR_ALWAYS,
        f: window_copy_cmd_halfpage_down_and_cancel,
    },
    window_copy_cmd_table_entry {
        command: SyncCharPtr::new(c"halfpage-up"),
        minargs: 0,
        maxargs: 0,
        clear: window_copy_cmd_clear::WINDOW_COPY_CMD_CLEAR_EMACS_ONLY,
        f: window_copy_cmd_halfpage_up,
    },
    window_copy_cmd_table_entry {
        command: SyncCharPtr::new(c"history-bottom"),
        minargs: 0,
        maxargs: 0,
        clear: window_copy_cmd_clear::WINDOW_COPY_CMD_CLEAR_EMACS_ONLY,
        f: window_copy_cmd_history_bottom,
    },
    window_copy_cmd_table_entry {
        command: SyncCharPtr::new(c"history-top"),
        minargs: 0,
        maxargs: 0,
        clear: window_copy_cmd_clear::WINDOW_COPY_CMD_CLEAR_EMACS_ONLY,
        f: window_copy_cmd_history_top,
    },
    window_copy_cmd_table_entry {
        command: SyncCharPtr::new(c"jump-again"),
        minargs: 0,
        maxargs: 0,
        clear: window_copy_cmd_clear::WINDOW_COPY_CMD_CLEAR_EMACS_ONLY,
        f: window_copy_cmd_jump_again,
    },
    window_copy_cmd_table_entry {
        command: SyncCharPtr::new(c"jump-backward"),
        minargs: 1,
        maxargs: 1,
        clear: window_copy_cmd_clear::WINDOW_COPY_CMD_CLEAR_EMACS_ONLY,
        f: window_copy_cmd_jump_backward,
    },
    window_copy_cmd_table_entry {
        command: SyncCharPtr::new(c"jump-forward"),
        minargs: 1,
        maxargs: 1,
        clear: window_copy_cmd_clear::WINDOW_COPY_CMD_CLEAR_EMACS_ONLY,
        f: window_copy_cmd_jump_forward,
    },
    window_copy_cmd_table_entry {
        command: SyncCharPtr::new(c"jump-reverse"),
        minargs: 0,
        maxargs: 0,
        clear: window_copy_cmd_clear::WINDOW_COPY_CMD_CLEAR_EMACS_ONLY,
        f: window_copy_cmd_jump_reverse,
    },
    window_copy_cmd_table_entry {
        command: SyncCharPtr::new(c"jump-to-backward"),
        minargs: 1,
        maxargs: 1,
        clear: window_copy_cmd_clear::WINDOW_COPY_CMD_CLEAR_EMACS_ONLY,
        f: window_copy_cmd_jump_to_backward,
    },
    window_copy_cmd_table_entry {
        command: SyncCharPtr::new(c"jump-to-forward"),
        minargs: 1,
        maxargs: 1,
        clear: window_copy_cmd_clear::WINDOW_COPY_CMD_CLEAR_EMACS_ONLY,
        f: window_copy_cmd_jump_to_forward,
    },
    window_copy_cmd_table_entry {
        command: SyncCharPtr::new(c"jump-to-mark"),
        minargs: 0,
        maxargs: 0,
        clear: window_copy_cmd_clear::WINDOW_COPY_CMD_CLEAR_ALWAYS,
        f: window_copy_cmd_jump_to_mark,
    },
    window_copy_cmd_table_entry {
        command: SyncCharPtr::new(c"next-prompt"),
        minargs: 0,
        maxargs: 1,
        clear: window_copy_cmd_clear::WINDOW_COPY_CMD_CLEAR_ALWAYS,
        f: window_copy_cmd_next_prompt,
    },
    window_copy_cmd_table_entry {
        command: SyncCharPtr::new(c"previous-prompt"),
        minargs: 0,
        maxargs: 1,
        clear: window_copy_cmd_clear::WINDOW_COPY_CMD_CLEAR_ALWAYS,
        f: window_copy_cmd_previous_prompt,
    },
    window_copy_cmd_table_entry {
        command: SyncCharPtr::new(c"middle-line"),
        minargs: 0,
        maxargs: 0,
        clear: window_copy_cmd_clear::WINDOW_COPY_CMD_CLEAR_EMACS_ONLY,
        f: window_copy_cmd_middle_line,
    },
    window_copy_cmd_table_entry {
        command: SyncCharPtr::new(c"next-matching-bracket"),
        minargs: 0,
        maxargs: 0,
        clear: window_copy_cmd_clear::WINDOW_COPY_CMD_CLEAR_ALWAYS,
        f: window_copy_cmd_next_matching_bracket,
    },
    window_copy_cmd_table_entry {
        command: SyncCharPtr::new(c"next-paragraph"),
        minargs: 0,
        maxargs: 0,
        clear: window_copy_cmd_clear::WINDOW_COPY_CMD_CLEAR_EMACS_ONLY,
        f: window_copy_cmd_next_paragraph,
    },
    window_copy_cmd_table_entry {
        command: SyncCharPtr::new(c"next-space"),
        minargs: 0,
        maxargs: 0,
        clear: window_copy_cmd_clear::WINDOW_COPY_CMD_CLEAR_EMACS_ONLY,
        f: window_copy_cmd_next_space,
    },
    window_copy_cmd_table_entry {
        command: SyncCharPtr::new(c"next-space-end"),
        minargs: 0,
        maxargs: 0,
        clear: window_copy_cmd_clear::WINDOW_COPY_CMD_CLEAR_EMACS_ONLY,
        f: window_copy_cmd_next_space_end,
    },
    window_copy_cmd_table_entry {
        command: SyncCharPtr::new(c"next-word"),
        minargs: 0,
        maxargs: 0,
        clear: window_copy_cmd_clear::WINDOW_COPY_CMD_CLEAR_EMACS_ONLY,
        f: window_copy_cmd_next_word,
    },
    window_copy_cmd_table_entry {
        command: SyncCharPtr::new(c"next-word-end"),
        minargs: 0,
        maxargs: 0,
        clear: window_copy_cmd_clear::WINDOW_COPY_CMD_CLEAR_EMACS_ONLY,
        f: window_copy_cmd_next_word_end,
    },
    window_copy_cmd_table_entry {
        command: SyncCharPtr::new(c"other-end"),
        minargs: 0,
        maxargs: 0,
        clear: window_copy_cmd_clear::WINDOW_COPY_CMD_CLEAR_EMACS_ONLY,
        f: window_copy_cmd_other_end,
    },
    window_copy_cmd_table_entry {
        command: SyncCharPtr::new(c"page-down"),
        minargs: 0,
        maxargs: 0,
        clear: window_copy_cmd_clear::WINDOW_COPY_CMD_CLEAR_EMACS_ONLY,
        f: window_copy_cmd_page_down,
    },
    window_copy_cmd_table_entry {
        command: SyncCharPtr::new(c"page-down-and-cancel"),
        minargs: 0,
        maxargs: 0,
        clear: window_copy_cmd_clear::WINDOW_COPY_CMD_CLEAR_ALWAYS,
        f: window_copy_cmd_page_down_and_cancel,
    },
    window_copy_cmd_table_entry {
        command: SyncCharPtr::new(c"page-up"),
        minargs: 0,
        maxargs: 0,
        clear: window_copy_cmd_clear::WINDOW_COPY_CMD_CLEAR_EMACS_ONLY,
        f: window_copy_cmd_page_up,
    },
    window_copy_cmd_table_entry {
        command: SyncCharPtr::new(c"pipe-no-clear"),
        minargs: 0,
        maxargs: 1,
        clear: window_copy_cmd_clear::WINDOW_COPY_CMD_CLEAR_NEVER,
        f: window_copy_cmd_pipe_no_clear,
    },
    window_copy_cmd_table_entry {
        command: SyncCharPtr::new(c"pipe"),
        minargs: 0,
        maxargs: 1,
        clear: window_copy_cmd_clear::WINDOW_COPY_CMD_CLEAR_ALWAYS,
        f: window_copy_cmd_pipe,
    },
    window_copy_cmd_table_entry {
        command: SyncCharPtr::new(c"pipe-and-cancel"),
        minargs: 0,
        maxargs: 1,
        clear: window_copy_cmd_clear::WINDOW_COPY_CMD_CLEAR_ALWAYS,
        f: window_copy_cmd_pipe_and_cancel,
    },
    window_copy_cmd_table_entry {
        command: SyncCharPtr::new(c"previous-matching-bracket"),
        minargs: 0,
        maxargs: 0,
        clear: window_copy_cmd_clear::WINDOW_COPY_CMD_CLEAR_ALWAYS,
        f: window_copy_cmd_previous_matching_bracket,
    },
    window_copy_cmd_table_entry {
        command: SyncCharPtr::new(c"previous-paragraph"),
        minargs: 0,
        maxargs: 0,
        clear: window_copy_cmd_clear::WINDOW_COPY_CMD_CLEAR_EMACS_ONLY,
        f: window_copy_cmd_previous_paragraph,
    },
    window_copy_cmd_table_entry {
        command: SyncCharPtr::new(c"previous-space"),
        minargs: 0,
        maxargs: 0,
        clear: window_copy_cmd_clear::WINDOW_COPY_CMD_CLEAR_EMACS_ONLY,
        f: window_copy_cmd_previous_space,
    },
    window_copy_cmd_table_entry {
        command: SyncCharPtr::new(c"previous-word"),
        minargs: 0,
        maxargs: 0,
        clear: window_copy_cmd_clear::WINDOW_COPY_CMD_CLEAR_EMACS_ONLY,
        f: window_copy_cmd_previous_word,
    },
    window_copy_cmd_table_entry {
        command: SyncCharPtr::new(c"rectangle-on"),
        minargs: 0,
        maxargs: 0,
        clear: window_copy_cmd_clear::WINDOW_COPY_CMD_CLEAR_ALWAYS,
        f: window_copy_cmd_rectangle_on,
    },
    window_copy_cmd_table_entry {
        command: SyncCharPtr::new(c"rectangle-off"),
        minargs: 0,
        maxargs: 0,
        clear: window_copy_cmd_clear::WINDOW_COPY_CMD_CLEAR_ALWAYS,
        f: window_copy_cmd_rectangle_off,
    },
    window_copy_cmd_table_entry {
        command: SyncCharPtr::new(c"rectangle-toggle"),
        minargs: 0,
        maxargs: 0,
        clear: window_copy_cmd_clear::WINDOW_COPY_CMD_CLEAR_ALWAYS,
        f: window_copy_cmd_rectangle_toggle,
    },
    window_copy_cmd_table_entry {
        command: SyncCharPtr::new(c"refresh-from-pane"),
        minargs: 0,
        maxargs: 0,
        clear: window_copy_cmd_clear::WINDOW_COPY_CMD_CLEAR_ALWAYS,
        f: window_copy_cmd_refresh_from_pane,
    },
    window_copy_cmd_table_entry {
        command: SyncCharPtr::new(c"scroll-bottom"),
        minargs: 0,
        maxargs: 0,
        clear: window_copy_cmd_clear::WINDOW_COPY_CMD_CLEAR_ALWAYS,
        f: window_copy_cmd_scroll_bottom,
    },
    window_copy_cmd_table_entry {
        command: SyncCharPtr::new(c"scroll-down"),
        minargs: 0,
        maxargs: 0,
        clear: window_copy_cmd_clear::WINDOW_COPY_CMD_CLEAR_EMACS_ONLY,
        f: window_copy_cmd_scroll_down,
    },
    window_copy_cmd_table_entry {
        command: SyncCharPtr::new(c"scroll-down-and-cancel"),
        minargs: 0,
        maxargs: 0,
        clear: window_copy_cmd_clear::WINDOW_COPY_CMD_CLEAR_ALWAYS,
        f: window_copy_cmd_scroll_down_and_cancel,
    },
    window_copy_cmd_table_entry {
        command: SyncCharPtr::new(c"scroll-middle"),
        minargs: 0,
        maxargs: 0,
        clear: window_copy_cmd_clear::WINDOW_COPY_CMD_CLEAR_ALWAYS,
        f: window_copy_cmd_scroll_middle,
    },
    window_copy_cmd_table_entry {
        command: SyncCharPtr::new(c"scroll-top"),
        minargs: 0,
        maxargs: 0,
        clear: window_copy_cmd_clear::WINDOW_COPY_CMD_CLEAR_ALWAYS,
        f: window_copy_cmd_scroll_top,
    },
    window_copy_cmd_table_entry {
        command: SyncCharPtr::new(c"scroll-up"),
        minargs: 0,
        maxargs: 0,
        clear: window_copy_cmd_clear::WINDOW_COPY_CMD_CLEAR_EMACS_ONLY,
        f: window_copy_cmd_scroll_up,
    },
    window_copy_cmd_table_entry {
        command: SyncCharPtr::new(c"search-again"),
        minargs: 0,
        maxargs: 0,
        clear: window_copy_cmd_clear::WINDOW_COPY_CMD_CLEAR_ALWAYS,
        f: window_copy_cmd_search_again,
    },
    window_copy_cmd_table_entry {
        command: SyncCharPtr::new(c"search-backward"),
        minargs: 0,
        maxargs: 1,
        clear: window_copy_cmd_clear::WINDOW_COPY_CMD_CLEAR_ALWAYS,
        f: window_copy_cmd_search_backward,
    },
    window_copy_cmd_table_entry {
        command: SyncCharPtr::new(c"search-backward-text"),
        minargs: 0,
        maxargs: 1,
        clear: window_copy_cmd_clear::WINDOW_COPY_CMD_CLEAR_ALWAYS,
        f: window_copy_cmd_search_backward_text,
    },
    window_copy_cmd_table_entry {
        command: SyncCharPtr::new(c"search-backward-incremental"),
        minargs: 1,
        maxargs: 1,
        clear: window_copy_cmd_clear::WINDOW_COPY_CMD_CLEAR_ALWAYS,
        f: window_copy_cmd_search_backward_incremental,
    },
    window_copy_cmd_table_entry {
        command: SyncCharPtr::new(c"search-forward"),
        minargs: 0,
        maxargs: 1,
        clear: window_copy_cmd_clear::WINDOW_COPY_CMD_CLEAR_ALWAYS,
        f: window_copy_cmd_search_forward,
    },
    window_copy_cmd_table_entry {
        command: SyncCharPtr::new(c"search-forward-text"),
        minargs: 0,
        maxargs: 1,
        clear: window_copy_cmd_clear::WINDOW_COPY_CMD_CLEAR_ALWAYS,
        f: window_copy_cmd_search_forward_text,
    },
    window_copy_cmd_table_entry {
        command: SyncCharPtr::new(c"search-forward-incremental"),
        minargs: 1,
        maxargs: 1,
        clear: window_copy_cmd_clear::WINDOW_COPY_CMD_CLEAR_ALWAYS,
        f: window_copy_cmd_search_forward_incremental,
    },
    window_copy_cmd_table_entry {
        command: SyncCharPtr::new(c"search-reverse"),
        minargs: 0,
        maxargs: 0,
        clear: window_copy_cmd_clear::WINDOW_COPY_CMD_CLEAR_ALWAYS,
        f: window_copy_cmd_search_reverse,
    },
    window_copy_cmd_table_entry {
        command: SyncCharPtr::new(c"select-line"),
        minargs: 0,
        maxargs: 0,
        clear: window_copy_cmd_clear::WINDOW_COPY_CMD_CLEAR_ALWAYS,
        f: window_copy_cmd_select_line,
    },
    window_copy_cmd_table_entry {
        command: SyncCharPtr::new(c"select-word"),
        minargs: 0,
        maxargs: 0,
        clear: window_copy_cmd_clear::WINDOW_COPY_CMD_CLEAR_ALWAYS,
        f: window_copy_cmd_select_word,
    },
    window_copy_cmd_table_entry {
        command: SyncCharPtr::new(c"set-mark"),
        minargs: 0,
        maxargs: 0,
        clear: window_copy_cmd_clear::WINDOW_COPY_CMD_CLEAR_ALWAYS,
        f: window_copy_cmd_set_mark,
    },
    window_copy_cmd_table_entry {
        command: SyncCharPtr::new(c"start-of-line"),
        minargs: 0,
        maxargs: 0,
        clear: window_copy_cmd_clear::WINDOW_COPY_CMD_CLEAR_EMACS_ONLY,
        f: window_copy_cmd_start_of_line,
    },
    window_copy_cmd_table_entry {
        command: SyncCharPtr::new(c"stop-selection"),
        minargs: 0,
        maxargs: 0,
        clear: window_copy_cmd_clear::WINDOW_COPY_CMD_CLEAR_ALWAYS,
        f: window_copy_cmd_stop_selection,
    },
    window_copy_cmd_table_entry {
        command: SyncCharPtr::new(c"toggle-position"),
        minargs: 0,
        maxargs: 0,
        clear: window_copy_cmd_clear::WINDOW_COPY_CMD_CLEAR_NEVER,
        f: window_copy_cmd_toggle_position,
    },
    window_copy_cmd_table_entry {
        command: SyncCharPtr::new(c"top-line"),
        minargs: 0,
        maxargs: 0,
        clear: window_copy_cmd_clear::WINDOW_COPY_CMD_CLEAR_EMACS_ONLY,
        f: window_copy_cmd_top_line,
    },
];

pub unsafe fn window_copy_command(
    wme: NonNull<window_mode_entry>,
    c: *mut client,
    s: *mut session,
    wl: *mut winlink,
    args: *mut args,
    m: *mut mouse_event,
) {
    unsafe {
        let wme = wme.as_ptr();
        let data: *mut window_copy_mode_data = (*wme).data.cast();
        let mut cs: window_copy_cmd_state = zeroed();
        let mut clear = window_copy_cmd_clear::WINDOW_COPY_CMD_CLEAR_NEVER;
        let count = args_count(args);

        if count == 0 {
            return;
        }
        let command = args_string(args, 0);

        if !m.is_null() && (*m).valid != 0 && !MOUSE_WHEEL((*m).b) {
            window_copy_move_mouse(m);
        }

        cs.wme = wme;
        cs.args = args;
        cs.m = m;

        cs.c = c;
        cs.s = s;
        cs.wl = wl;

        let mut action = window_copy_cmd_action::WINDOW_COPY_CMD_NOTHING;
        for window_copy_cmd_table_i in &WINDOW_COPY_CMD_TABLE {
            if libc::strcmp(window_copy_cmd_table_i.command.as_ptr(), command) == 0 {
                if count - 1 < window_copy_cmd_table_i.minargs
                    || count - 1 > window_copy_cmd_table_i.maxargs
                {
                    break;
                }
                clear = window_copy_cmd_table_i.clear;
                action = (window_copy_cmd_table_i.f)(&raw mut cs);
                break;
            }
        }

        if libc::strncmp(command, c!("search-"), 7) != 0 && !(*data).searchmark.is_null() {
            let keys = modekey::try_from(options_get_number_(
                (*(*(*wme).wp).window).options,
                c"mode-keys",
            ) as i32);
            if clear == window_copy_cmd_clear::WINDOW_COPY_CMD_CLEAR_EMACS_ONLY
                && keys == Ok(modekey::MODEKEY_VI)
            {
                clear = window_copy_cmd_clear::WINDOW_COPY_CMD_CLEAR_NEVER;
            }
            if clear != window_copy_cmd_clear::WINDOW_COPY_CMD_CLEAR_NEVER {
                window_copy_clear_marks(wme);
                (*data).searchx = -1;
                (*data).searchy = -1;
            }
            if action == window_copy_cmd_action::WINDOW_COPY_CMD_NOTHING {
                action = window_copy_cmd_action::WINDOW_COPY_CMD_REDRAW;
            }
        }
        (*wme).prefix = 1;

        if action == window_copy_cmd_action::WINDOW_COPY_CMD_CANCEL {
            window_pane_reset_mode((*wme).wp);
        } else if action == window_copy_cmd_action::WINDOW_COPY_CMD_REDRAW {
            window_copy_redraw_screen(wme);
        }
    }
}

pub unsafe fn window_copy_scroll_to(
    wme: *mut window_mode_entry,
    px: u32,
    py: u32,
    no_redraw: bool,
) {
    unsafe {
        let data: *mut window_copy_mode_data = (*wme).data.cast();
        let gd: *mut grid = (*(*data).backing).grid;

        (*data).cx = px;

        if py >= (*gd).hsize - (*data).oy && py < (*gd).hsize - (*data).oy + (*gd).sy {
            (*data).cy = py - ((*gd).hsize - (*data).oy);
        } else {
            let gap = (*gd).sy / 4;
            let offset;
            if py < (*gd).sy {
                offset = 0;
                (*data).cy = py;
            } else if py > (*gd).hsize + (*gd).sy - gap {
                offset = (*gd).hsize;
                (*data).cy = py - (*gd).hsize;
            } else {
                offset = py + gap - (*gd).sy;
                (*data).cy = py - offset;
            }
            (*data).oy = (*gd).hsize - offset;
        }

        if !no_redraw && !(*data).searchmark.is_null() && (*data).timeout == 0 {
            window_copy_search_marks(wme, null_mut(), (*data).searchregex, 1);
        }
        window_copy_update_selection(wme, 1, 0);
        if !no_redraw {
            window_copy_redraw_screen(wme);
        }
    }
}

pub unsafe fn window_copy_search_compare(
    gd: *mut grid,
    px: u32,
    py: u32,
    sgd: *mut grid,
    spx: u32,
    cis: i32,
) -> bool {
    unsafe {
        let mut gc: grid_cell = zeroed();
        let mut sgc: grid_cell = zeroed();
        grid_get_cell(gd, px, py, &raw mut gc);
        let ud = &raw const gc.data;
        grid_get_cell(sgd, spx, 0, &raw mut sgc);
        let sud = &raw const sgc.data;

        if (*ud).size != (*sud).size || (*ud).width != (*sud).width {
            return false;
        }

        if cis != 0 && (*ud).size == 1 {
            return (*ud).data[0].to_ascii_lowercase() == (*sud).data[0];
        }

        libc::memcmp(
            (&raw const (*ud).data).cast(),
            (&raw const (*sud).data).cast(),
            (*ud).size as usize,
        ) == 0
    }
}

pub unsafe fn window_copy_search_lr(
    gd: *mut grid,
    sgd: *mut grid,
    ppx: *mut u32,
    py: u32,
    first: u32,
    last: u32,
    cis: i32,
) -> i32 {
    unsafe {
        // u_int ax, bx, px, pywrap, endline;
        // int matched;
        let mut gl: *mut grid_line = null_mut();

        let endline = (*gd).hsize + (*gd).sy - 1;
        for ax in first..last {
            let mut bx = 0;
            for bx_ in 0..(*sgd).sx {
                bx = bx_;
                let mut px = ax + bx;
                let mut pywrap = py;
                /* Wrap line. */
                while px >= (*gd).sx && pywrap < endline {
                    gl = grid_get_line(gd, pywrap);
                    if !(*gl).flags.intersects(grid_line_flag::WRAPPED) {
                        break;
                    }
                    px -= (*gd).sx;
                    pywrap += 1;
                }
                /* We have run off the end of the grid. */
                if px >= (*gd).sx {
                    break;
                }
                let matched = window_copy_search_compare(gd, px, pywrap, sgd, bx, cis);
                if !matched {
                    break;
                }
            }
            if bx == (*sgd).sx {
                *ppx = ax;
                return 1;
            }
        }
        0
    }
}

pub unsafe fn window_copy_search_rl(
    gd: *mut grid,
    sgd: *mut grid,
    ppx: *mut u32,
    py: u32,
    first: u32,
    last: u32,
    cis: i32,
) -> i32 {
    unsafe {
        // u_int ax, bx, px, pywrap, endline;
        let matched = 0;
        let mut gl: *mut grid_line = null_mut();
        let endline = (*gd).hsize + (*gd).sy - 1;

        // for (ax = last; ax > first; ax--) {
        let mut ax = last;
        while ax > first {
            let mut bx = 0;
            for bx_ in 0..(*sgd).sx {
                bx = bx_;
                let mut px = ax - 1 + bx;
                let mut pywrap = py;
                /* Wrap line. */
                while px >= (*gd).sx && pywrap < endline {
                    gl = grid_get_line(gd, pywrap);
                    if !(*gl).flags.intersects(grid_line_flag::WRAPPED) {
                        break;
                    }
                    px -= (*gd).sx;
                    pywrap += 1;
                }
                /* We have run off the end of the grid. */
                if px >= (*gd).sx {
                    break;
                }
                let matched = window_copy_search_compare(gd, px, pywrap, sgd, bx, cis);
                if !matched {
                    break;
                }
            }
            if bx == (*sgd).sx {
                *ppx = ax - 1;
                return 1;
            }
            ax -= 1;
        }
        0
    }
}

pub unsafe fn window_copy_search_lr_regex(
    gd: *mut grid,
    ppx: *mut u32,
    psx: *mut u32,
    py: u32,
    first: u32,
    last: u32,
    reg: *mut libc::regex_t,
) -> i32 {
    unsafe {
        let mut eflags = 0;
        let mut size: u32 = 1;
        // u_int endline, foundx, foundy, len, pywrap, size = 1;
        // char *buf;
        // regmatch_t regmatch;
        let mut regmatch: libc::regmatch_t = zeroed();
        // struct grid_line *gl;

        /*
         * This can happen during search if the last match was the last
         * character on a line.
         */
        if first >= last {
            return 0;
        }

        /* Set flags for regex search. */
        if first != 0 {
            eflags |= libc::REG_NOTBOL;
        }

        /* Need to look at the entire string. */
        let mut buf = xmalloc(size as usize).cast::<u8>().as_ptr();
        *buf = b'\0';
        buf = window_copy_stringify(gd, py, first, (*gd).sx, buf, &raw mut size);
        let mut len = (*gd).sx - first;
        let endline = (*gd).hsize + (*gd).sy - 1;
        let mut pywrap = py;
        while !buf.is_null() && pywrap <= endline && len < WINDOW_COPY_SEARCH_MAX_LINE {
            let gl = grid_get_line(gd, pywrap);
            if !(*gl).flags.intersects(grid_line_flag::WRAPPED) {
                break;
            }
            pywrap += 1;
            buf = window_copy_stringify(gd, pywrap, 0, (*gd).sx, buf, &raw mut size);
            len += (*gd).sx;
        }

        if libc::regexec(reg, buf, 1, &raw mut regmatch, eflags) == 0
            && regmatch.rm_so != regmatch.rm_eo
        {
            let mut foundx = first;
            let mut foundy = py;
            window_copy_cstrtocellpos(
                gd,
                len,
                &raw mut foundx,
                &raw mut foundy,
                buf.add(regmatch.rm_so as usize),
            );
            if foundy == py && foundx < last {
                *ppx = foundx;
                len -= foundx - first;
                window_copy_cstrtocellpos(
                    gd,
                    len,
                    &raw mut foundx,
                    &raw mut foundy,
                    buf.add(regmatch.rm_eo as usize),
                );
                *psx = foundx;
                while foundy > py {
                    *psx += (*gd).sx;
                    foundy -= 1;
                }
                *psx -= *ppx;
                free_(buf);
                return 1;
            }
        }

        free_(buf);
        *ppx = 0;
        *psx = 0;
        0
    }
}

pub unsafe fn window_copy_search_rl_regex(
    gd: *mut grid,
    ppx: *mut u32,
    psx: *mut u32,
    py: u32,
    first: u32,
    last: u32,
    reg: *mut libc::regex_t,
) -> i32 {
    unsafe {
        let mut eflags = 0;
        let mut size: u32 = 1;
        // u_int endline, len, pywrap, size = 1;
        // char *buf;
        // struct grid_line *gl;

        /* Set flags for regex search. */
        if first != 0 {
            eflags |= libc::REG_NOTBOL;
        }

        /* Need to look at the entire string. */
        let mut buf = xmalloc(size as usize).cast::<u8>().as_ptr();
        *buf = b'\0';
        buf = window_copy_stringify(gd, py, first, (*gd).sx, buf, &raw mut size);
        let mut len = (*gd).sx - first;
        let endline = (*gd).hsize + (*gd).sy - 1;
        let mut pywrap = py;
        while !buf.is_null() && pywrap <= endline && len < WINDOW_COPY_SEARCH_MAX_LINE {
            let gl = grid_get_line(gd, pywrap);
            if !(*gl).flags.intersects(grid_line_flag::WRAPPED) {
                break;
            }
            pywrap += 1;
            buf = window_copy_stringify(gd, pywrap, 0, (*gd).sx, buf, &raw mut size);
            len += (*gd).sx;
        }

        if window_copy_last_regex(gd, py, first, last, len, ppx, psx, buf, reg, eflags) != 0 {
            free_(buf);
            return 1;
        }

        free_(buf);
        *ppx = 0;
        *psx = 0;
        0
    }
}

pub unsafe fn window_copy_cellstring(
    gl: *mut grid_line,
    px: u32,
    size: *mut usize,
    allocated: *mut i32,
) -> *mut u8 {
    unsafe {
        // struct grid_cell_entry *gce;

        if px >= (*gl).cellsize {
            *size = 1;
            *allocated = 0;
            return c!(" ") as *mut u8; // TODO think of a better type-safe way to represent returning a MaybeAllocated type
        }

        let gce = (*gl).celldata.add(px as usize);
        if (*gce).flags.intersects(grid_flag::PADDING) {
            *size = 0;
            *allocated = 0;
            return null_mut();
        }
        if !(*gce).flags.intersects(grid_flag::EXTENDED) {
            *size = 1;
            *allocated = 0;
            return (&raw mut (*gce).union_.data.data).cast();
        }

        let ud = utf8_to_data((*(*gl).extddata.add((*gce).union_.offset as usize)).data);
        if ud.size == 0 {
            *size = 0;
            *allocated = 0;
            return null_mut();
        }
        *size = ud.size as usize;
        *allocated = 1;

        let copy: *mut u8 = xmalloc(ud.size as usize).as_ptr().cast();
        libc::memcpy(copy.cast(), (&raw const ud.data).cast(), ud.size as usize);
        copy
    }
}

/* Find last match in given range. */

pub unsafe fn window_copy_last_regex(
    gd: *mut grid,
    py: u32,
    first: u32,
    last: u32,
    mut len: u32,
    ppx: *mut u32,
    psx: *mut u32,
    buf: *const u8,
    preg: *const libc::regex_t,
    eflags: i32,
) -> i32 {
    unsafe {
        let oldx = 0;
        let mut px = 0;
        let mut savepx = 0;
        let mut savesx = 0;
        let mut regmatch: libc::regmatch_t = zeroed();

        let mut foundx = first;
        let mut foundy = py;
        let mut oldx = first;
        while libc::regexec(preg, buf.add(px), 1, &raw mut regmatch, eflags) == 0 {
            if regmatch.rm_so == regmatch.rm_eo {
                break;
            }
            window_copy_cstrtocellpos(
                gd,
                len,
                &raw mut foundx,
                &raw mut foundy,
                buf.add(px + regmatch.rm_so as usize),
            );
            if foundy > py || foundx >= last {
                break;
            }
            len -= foundx - oldx;
            savepx = foundx;
            window_copy_cstrtocellpos(
                gd,
                len,
                &raw mut foundx,
                &raw mut foundy,
                buf.add(px + regmatch.rm_eo as usize),
            );
            if foundy > py || foundx >= last {
                *ppx = savepx;
                *psx = foundx;
                while foundy > py {
                    *psx += (*gd).sx;
                    foundy -= 1;
                }
                *psx -= *ppx;
                return 1;
            } else {
                savesx = foundx - savepx;
                len -= savesx;
                oldx = foundx;
            }
            px += regmatch.rm_eo as usize;
        }

        if savesx > 0 {
            *ppx = savepx;
            *psx = savesx;
            1
        } else {
            *ppx = 0;
            *psx = 0;
            0
        }
    }
}

/* Stringify line and append to input buffer. Caller frees. */

pub unsafe fn window_copy_stringify(
    gd: *mut grid,
    py: u32,
    first: u32,
    last: u32,
    mut buf: *mut u8,
    size: *mut u32,
) -> *mut u8 {
    unsafe {
        let ax = 0;
        let mut bx = 0;

        let mut newsize = *size;

        let mut bufsize: usize = 1024;
        let mut dlen: usize = 0;
        let mut allocated = 0;

        while bufsize < newsize as usize {
            bufsize *= 2;
        }
        buf = xrealloc(buf.cast(), bufsize).as_ptr().cast();

        let gl = grid_peek_line(gd, py);
        bx = *size - 1;
        for ax in first..last {
            let d = window_copy_cellstring(gl, ax, &raw mut dlen, &raw mut allocated);
            newsize += dlen as u32;
            while bufsize < newsize as usize {
                bufsize *= 2;
                buf = xrealloc(buf.cast(), bufsize).as_ptr().cast();
            }
            if dlen == 1 {
                *buf.add(bx as usize) = *d;
                bx += 1;
            } else {
                libc::memcpy(buf.add(bx as usize).cast(), d.cast(), dlen);
                bx += dlen as u32;
            }
            if allocated != 0 {
                free_(d);
            }
        }
        *buf.add(newsize as usize - 1) = b'\0';

        *size = newsize;
        buf
    }
}

/// Map start of C string containing UTF-8 data to grid cell position.
pub unsafe fn window_copy_cstrtocellpos(
    gd: *mut grid,
    ncells: u32,
    ppx: *mut u32,
    ppy: *mut u32,
    str: *const u8,
) {
    unsafe {
        let mut ccell: u32 = 0;
        let mut px: u32 = 0;
        let mut pywrap: u32 = 0;
        let pos: u32 = 0;
        let mut len: u32 = 0;

        let mut match_: i32 = 0;

        // const char *d;
        // size_t dlen;

        #[repr(C)]
        struct cell {
            d: *const u8,
            dlen: usize,
            allocated: i32,
        };

        /* Populate the array of cell data. */
        let cells: *mut cell = xreallocarray_::<cell>(null_mut(), ncells as usize).as_ptr();
        let mut cell = 0;
        px = *ppx;
        pywrap = *ppy;
        let mut gl = grid_peek_line(gd, pywrap);
        while cell < ncells {
            (*cells.add(cell as usize)).d = window_copy_cellstring(
                gl,
                px,
                &raw mut (*cells.add(cell as usize)).dlen,
                &raw mut (*cells.add(cell as usize)).allocated,
            );
            cell += 1;
            px += 1;
            if px == (*gd).sx {
                px = 0;
                pywrap += 1;
                gl = grid_peek_line(gd, pywrap);
            }
        }

        /* Locate starting cell. */
        cell = 0;
        len = strlen(str) as u32;
        while cell < ncells {
            ccell = cell;
            let mut pos = 0;
            match_ = 1;
            while ccell < ncells {
                if *str.add(pos) == b'\0' {
                    match_ = 0;
                    break;
                }
                let d = (*cells.add(ccell as usize)).d;
                let mut dlen = (*cells.add(ccell as usize)).dlen;
                if dlen == 1 {
                    if *str.add(pos) != *d {
                        match_ = 0;
                        break;
                    }
                    pos += 1;
                } else {
                    if dlen > len as usize - pos {
                        dlen = len as usize - pos;
                    }
                    if memcmp(str.add(pos).cast(), d.cast(), dlen) != 0 {
                        match_ = 0;
                        break;
                    }
                    pos += dlen;
                }
                ccell += 1;
            }
            if match_ != 0 {
                break;
            }
            cell += 1;
        }

        /* If not found this will be one past the end. */
        px = *ppx + cell;
        pywrap = *ppy;
        while px >= (*gd).sx {
            px -= (*gd).sx;
            pywrap += 1;
        }

        *ppx = px;
        *ppy = pywrap;

        /* Free cell data. */
        for cell in 0..ncells {
            if (*cells.add(cell as usize)).allocated != 0 {
                free_((*cells.add(cell as usize)).d as *mut c_void);
            } // TODO cast away const
        }
        free_(cells);
    }
}

pub unsafe fn window_copy_move_left(s: *mut screen, fx: *mut u32, fy: *mut u32, wrapflag: i32) {
    unsafe {
        if *fx == 0 {
            /* left */
            if *fy == 0 {
                /* top */
                if wrapflag != 0 {
                    *fx = screen_size_x(s) - 1;
                    *fy = screen_hsize(s) + screen_size_y(s) - 1;
                }
                return;
            }
            *fx = screen_size_x(s) - 1;
            *fy -= 1;
        } else {
            *fx -= 1;
        }
    }
}

pub unsafe fn window_copy_move_right(s: *mut screen, fx: *mut u32, fy: *mut u32, wrapflag: i32) {
    unsafe {
        if *fx == screen_size_x(s) - 1 {
            /* right */
            if *fy == screen_hsize(s) + screen_size_y(s) - 1 {
                /* bottom */
                if wrapflag != 0 {
                    *fx = 0;
                    *fy = 0;
                }
                return;
            }
            *fx = 0;
            *fy += 1;
        } else {
            *fx += 1;
        }
    }
}

pub unsafe fn window_copy_is_lowercase(mut ptr: *const u8) -> bool {
    unsafe {
        while *ptr != b'\0' {
            if *ptr != (*ptr).to_ascii_lowercase() {
                return false;
            }
            ptr = ptr.add(1);
        }
        true
    }
}

/*
 * Handle backward wrapped regex searches with overlapping matches. In this case
 * find the longest overlapping match from previous wrapped lines.
 */

pub unsafe fn window_copy_search_back_overlap(
    gd: *mut grid,
    preg: *mut libc::regex_t,
    ppx: *mut u32,
    psx: *mut u32,
    ppy: *mut u32,
    endline: u32,
) {
    unsafe {
        let mut endx = 0;
        let mut oldendx = 0;
        let mut endy = 0;
        let mut oldendy = 0;
        let mut px = 0;
        let mut py = 0;
        let mut sx = 0;

        let mut found = true;

        oldendx = *ppx + *psx;
        oldendy = *ppy - 1;
        while oldendx > (*gd).sx - 1 {
            oldendx -= (*gd).sx;
            oldendy += 1;
        }
        endx = oldendx;
        endy = oldendy;
        px = *ppx;
        py = *ppy;
        while found
            && px == 0
            && py - 1 > endline
            && (*grid_get_line(gd, py - 2))
                .flags
                .intersects(grid_line_flag::WRAPPED)
            && endx == oldendx
            && endy == oldendy
        {
            py -= 1;
            found = window_copy_search_rl_regex(
                gd,
                &raw mut px,
                &raw mut sx,
                py - 1,
                0,
                (*gd).sx,
                preg,
            ) != 0;
            if found {
                endx = px + sx;
                endy = py - 1;
                while endx > (*gd).sx - 1 {
                    endx -= (*gd).sx;
                    endy += 1;
                }
                if endx == oldendx && endy == oldendy {
                    *ppx = px;
                    *ppy = py;
                }
            }
        }
    }
}

/*
 * Search for text stored in sgd starting from position fx,fy up to endline. If
 * found, jump to it. If cis then ignore case. The direction is 0 for searching
 * up, down otherwise. If wrap then go to begin/end of grid and try again if
 * not found.
 */

pub unsafe fn window_copy_search_jump(
    wme: *mut window_mode_entry,
    gd: *mut grid,
    sgd: *mut grid,
    mut fx: u32,
    fy: u32,
    endline: u32,
    cis: i32,
    wrap: i32,
    direction: i32,
    regex: i32,
) -> i32 {
    unsafe {
        let mut px = 0;
        let mut sx = 0;

        // u_int i, px, sx;
        let mut ssize: u32 = 1;
        let mut found = 0;
        let mut cflags = libc::REG_EXTENDED;

        let mut reg: libc::regex_t = zeroed();
        // char *sbuf;
        // regex_t reg;

        let mut sbuf = null_mut();
        if regex != 0 {
            sbuf = xmalloc(ssize as usize).as_ptr().cast();
            *sbuf = b'\0';
            sbuf = window_copy_stringify(sgd, 0, 0, (*sgd).sx, sbuf, &raw mut ssize);
            if cis != 0 {
                cflags |= REG_ICASE;
            }
            if libc::regcomp(&raw mut reg, sbuf, cflags) != 0 {
                free_(sbuf);
                return 0;
            }
            free_(sbuf);
        }

        let mut i = 0;
        if direction != 0 {
            for i_ in fy..=endline {
                i = i_;

                if regex != 0 {
                    found = window_copy_search_lr_regex(
                        gd,
                        &raw mut px,
                        &raw mut sx,
                        i,
                        fx,
                        (*gd).sx,
                        &raw mut reg,
                    );
                } else {
                    found = window_copy_search_lr(gd, sgd, &raw mut px, i, fx, (*gd).sx, cis);
                }
                if found != 0 {
                    break;
                }
                fx = 0;
            }
        } else {
            i = fy + 1;
            while endline < i {
                if regex != 0 {
                    found = window_copy_search_rl_regex(
                        gd,
                        &raw mut px,
                        &raw mut sx,
                        i - 1,
                        0,
                        fx + 1,
                        &raw mut reg,
                    );
                    if found != 0 {
                        window_copy_search_back_overlap(
                            gd,
                            &raw mut reg,
                            &raw mut px,
                            &raw mut sx,
                            &raw mut i,
                            endline,
                        );
                    }
                } else {
                    found = window_copy_search_rl(gd, sgd, &raw mut px, i - 1, 0, fx + 1, cis);
                }
                if found != 0 {
                    i -= 1;
                    break;
                }
                fx = (*gd).sx - 1;
                i -= 1;
            }
        }
        if regex != 0 {
            libc::regfree(&raw mut reg);
        }

        if found != 0 {
            window_copy_scroll_to(wme, px, i, true);
            return 1;
        }
        if wrap != 0 {
            return window_copy_search_jump(
                wme,
                gd,
                sgd,
                if direction != 0 { 0 } else { (*gd).sx - 1 },
                if direction != 0 {
                    0
                } else {
                    (*gd).hsize + (*gd).sy - 1
                },
                fy,
                cis,
                0,
                direction,
                regex,
            );
        }
        0
    }
}

pub unsafe fn window_copy_move_after_search_mark(
    data: *mut window_copy_mode_data,
    fx: *mut u32,
    fy: *mut u32,
    wrapflag: i32,
) {
    unsafe {
        let s = (*data).backing;
        let mut start = 0;
        let mut at = 0;

        if window_copy_search_mark_at(data, *fx, *fy, &raw mut start) == 0
            && *(*data).searchmark.add(start as usize) != 0
        {
            while window_copy_search_mark_at(data, *fx, *fy, &raw mut at) == 0 {
                if (*data).searchmark.add(at as usize) != (*data).searchmark.add(start as usize) {
                    break;
                }
                /* Stop if not wrapping and at the end of the grid. */
                if wrapflag == 0
                    && *fx == screen_size_x(s) - 1
                    && *fy == screen_hsize(s) + screen_size_y(s) - 1
                {
                    break;
                }

                window_copy_move_right(s, fx, fy, wrapflag);
            }
        }
    }
}

/*
 * Search in for text searchstr. If direction is 0 then search up, otherwise
 * down.
 */

pub unsafe fn window_copy_search(
    wme: *mut window_mode_entry,
    direction: i32,
    mut regex: i32,
) -> i32 {
    unsafe {
        let wp: *mut window_pane = (*wme).wp;
        let data: *mut window_copy_mode_data = (*wme).data.cast();
        let s: *mut screen = (*data).backing;
        let mut ss: screen = zeroed();
        let mut ctx: screen_write_ctx = zeroed();
        let gd: *mut grid = (*s).grid;
        let str: *mut u8 = (*data).searchstr;
        let mut at: u32 = 0;
        let mut endline: u32 = 0;
        let mut fx: u32 = 0;
        let mut fy: u32 = 0;
        let mut start: u32 = 0;
        let mut cis: i32 = 0;
        let mut found: i32 = 0;
        let mut visible_only: i32 = 0;
        let mut wrapflag: i32 = 0;

        if regex != 0 && *str.add(libc::strcspn(str, c!("^$*+()?[].\\"))) == b'\0' {
            regex = 0;
        }

        (*data).searchdirection = direction;

        if (*data).timeout != 0 {
            return 0;
        }

        if (*data).searchall != 0 || (*wp).searchstr.is_null() || (*wp).searchregex != regex {
            visible_only = 0;
            (*data).searchall = 0;
        } else {
            visible_only = (libc::strcmp((*wp).searchstr, str) == 0) as i32;
        }
        if visible_only == 0 && !(*data).searchmark.is_null() {
            window_copy_clear_marks(wme);
        }
        free_((*wp).searchstr);
        (*wp).searchstr = xstrdup(str).as_ptr();
        (*wp).searchregex = regex;

        fx = (*data).cx;
        fy = screen_hsize((*data).backing) - (*data).oy + (*data).cy;

        screen_init(
            &raw mut ss,
            screen_write_strlen!("{}", _s(str)) as u32,
            1,
            0,
        );
        screen_write_start(&raw mut ctx, &raw mut ss);
        screen_write_nputs!(
            &raw mut ctx,
            -1,
            &raw const GRID_DEFAULT_CELL,
            "{}",
            _s(str),
        );
        screen_write_stop(&raw mut ctx);

        wrapflag = options_get_number_((*(*wp).window).options, c"wrap-search") as i32;
        cis = window_copy_is_lowercase(str) as i32;

        let keys =
            modekey::try_from(options_get_number_((*(*wp).window).options, c"mode-keys") as i32);

        if direction != 0 {
            /*
             * Behave according to mode-keys. If it is emacs, search forward
             * leaves the cursor after the match. If it is vi, the cursor
             * remains at the beginning of the match, regardless of
             * direction, which means that we need to start the next search
             * after the term the cursor is currently on when searching
             * forward.
             */
            if keys == Ok(modekey::MODEKEY_VI) {
                if !(*data).searchmark.is_null() {
                    window_copy_move_after_search_mark(data, &raw mut fx, &raw mut fy, wrapflag);
                } else {
                    /*
                     * When there are no search marks, start the
                     * search after the current cursor position.
                     */
                    window_copy_move_right(s, &raw mut fx, &raw mut fy, wrapflag);
                }
            }
            endline = (*gd).hsize + (*gd).sy - 1;
        } else {
            window_copy_move_left(s, &raw mut fx, &raw mut fy, wrapflag);
            endline = 0;
        }

        found = window_copy_search_jump(
            wme, gd, ss.grid, fx, fy, endline, cis, wrapflag, direction, regex,
        );
        if found != 0 {
            window_copy_search_marks(wme, &raw mut ss, regex, visible_only);
            fx = (*data).cx;
            fy = screen_hsize((*data).backing) - (*data).oy + (*data).cy;

            /*
             * When searching forward, if the cursor is not at the beginning
             * of the mark, search again.
             */
            if direction != 0
                && window_copy_search_mark_at(data, fx, fy, &raw mut at) == 0
                && at > 0
                && !(*data).searchmark.is_null()
                && *(*data).searchmark.add(at as usize) == *(*data).searchmark.add(at as usize - 1)
            {
                window_copy_move_after_search_mark(data, &raw mut fx, &raw mut fy, wrapflag);
                window_copy_search_jump(
                    wme, gd, ss.grid, fx, fy, endline, cis, wrapflag, direction, regex,
                );
                fx = (*data).cx;
                fy = screen_hsize((*data).backing) - (*data).oy + (*data).cy;
            }

            if direction != 0 {
                /*
                 * When in Emacs mode, position the cursor just after
                 * the mark.
                 */
                if keys == Ok(modekey::MODEKEY_EMACS) {
                    window_copy_move_after_search_mark(data, &raw mut fx, &raw mut fy, wrapflag);
                    (*data).cx = fx;
                    (*data).cy = fy - screen_hsize((*data).backing) + (*data).oy;
                }
            } else {
                /*
                 * When searching backward, position the cursor at the
                 * beginning of the mark.
                 */
                if window_copy_search_mark_at(data, fx, fy, &raw mut start) == 0 {
                    while window_copy_search_mark_at(data, fx, fy, &raw mut at) == 0
                        && !(*data).searchmark.is_null()
                        && *(*data).searchmark.add(at as usize)
                            == *(*data).searchmark.add(start as usize)
                    {
                        (*data).cx = fx;
                        (*data).cy = fy - screen_hsize((*data).backing) + (*data).oy;
                        if at == 0 {
                            break;
                        }

                        window_copy_move_left(s, &raw mut fx, &raw mut fy, 0);
                    }
                }
            }
        }
        window_copy_redraw_screen(wme);

        screen_free(&raw mut ss);
        found
    }
}

pub unsafe fn window_copy_visible_lines(
    data: *mut window_copy_mode_data,
    start: *mut u32,
    end: *mut u32,
) {
    unsafe {
        let gd = (*(*data).backing).grid;

        *start = (*gd).hsize - (*data).oy;

        while *start > 0 {
            let gl = grid_peek_line(gd, (*start) - 1);
            if !(*gl).flags.intersects(grid_line_flag::WRAPPED) {
                break;
            }
            (*start) -= 1;
        }
        *end = (*gd).hsize - (*data).oy + (*gd).sy;
    }
}

pub unsafe fn window_copy_search_mark_at(
    data: *mut window_copy_mode_data,
    px: u32,
    py: u32,
    at: *mut u32,
) -> i32 {
    unsafe {
        let s: *mut screen = (*data).backing;
        let gd: *mut grid = (*s).grid;

        if py < (*gd).hsize - (*data).oy {
            return -1;
        }
        if py > (*gd).hsize - (*data).oy + (*gd).sy - 1 {
            return -1;
        }
        *at = ((py - ((*gd).hsize - (*data).oy)) * (*gd).sx) + px;
        0
    }
}

pub unsafe fn window_copy_search_marks(
    wme: *mut window_mode_entry,
    mut ssp: *mut screen,
    regex: i32,
    visible_only: i32,
) -> i32 {
    unsafe {
        let data: *mut window_copy_mode_data = (*wme).data.cast();
        let s: *mut screen = (*data).backing;
        let mut ss: screen = zeroed();
        let mut ctx: screen_write_ctx = zeroed();
        let gd: *mut grid = (*s).grid;
        let mut found: i32;
        let mut cis: i32 = 0;
        let mut stopped: i32 = 0;

        let mut cflags = libc::REG_EXTENDED;
        let mut px: u32;
        let mut py: u32;
        let mut b: u32 = 0;
        let mut nfound: u32 = 0;
        let mut width: u32 = 0;

        let mut ssize: u32 = 1;
        let mut start: u32 = 0;
        let mut end: u32 = 0;

        let mut sbuf = null_mut();
        let mut reg: libc::regex_t = zeroed();
        let mut stop: u64 = 0;
        let mut tstart: u64 = 0;
        let mut t: u64 = 0;
        'out: {
            if ssp.is_null() {
                width = screen_write_strlen!("{}", _s((*data).searchstr)) as u32;
                screen_init(&raw mut ss, width, 1, 0);
                screen_write_start(&raw mut ctx, &raw mut ss);
                screen_write_nputs!(
                    &raw mut ctx,
                    -1,
                    &raw const GRID_DEFAULT_CELL,
                    "{}",
                    _s((*data).searchstr),
                );
                screen_write_stop(&raw mut ctx);
                ssp = &raw mut ss;
            } else {
                width = screen_size_x(ssp);
            }

            cis = window_copy_is_lowercase((*data).searchstr) as i32;

            if regex != 0 {
                sbuf = xmalloc(ssize as usize).as_ptr().cast();
                *sbuf = b'\0';
                sbuf = window_copy_stringify(
                    (*ssp).grid,
                    0,
                    0,
                    (*(*ssp).grid).sx,
                    sbuf,
                    &raw mut ssize,
                );
                if cis != 0 {
                    cflags |= REG_ICASE;
                }
                if libc::regcomp(&raw mut reg, sbuf, cflags) != 0 {
                    free_(sbuf);
                    return 0;
                }
                free_(sbuf);
            }
            tstart = get_timer();

            if visible_only != 0 {
                window_copy_visible_lines(data, &raw mut start, &raw mut end);
            } else {
                start = 0;
                end = (*gd).hsize + (*gd).sy;
                stop = get_timer() + WINDOW_COPY_SEARCH_ALL_TIMEOUT;
            }

            'again: loop {
                free_((*data).searchmark);
                (*data).searchmark = xcalloc((*gd).sx as usize, (*gd).sy as usize)
                    .cast()
                    .as_ptr();
                (*data).searchgen = 1;

                for py in start..end {
                    px = 0;
                    loop {
                        if regex != 0 {
                            found = window_copy_search_lr_regex(
                                gd,
                                &raw mut px,
                                &raw mut width,
                                py,
                                px,
                                (*gd).sx,
                                &raw mut reg,
                            );
                            if found == 0 {
                                break;
                            }
                        } else {
                            found = window_copy_search_lr(
                                gd,
                                (*ssp).grid,
                                &raw mut px,
                                py,
                                px,
                                (*gd).sx,
                                cis,
                            );
                            if found == 0 {
                                break;
                            }
                        }
                        nfound += 1;

                        if window_copy_search_mark_at(data, px, py, &raw mut b) == 0 {
                            if b + width > (*gd).sx * (*gd).sy {
                                width = ((*gd).sx * (*gd).sy) - b;
                            }
                            for i in b..(b + width) {
                                if *(*data).searchmark.add(i as usize) != 0 {
                                    continue;
                                }
                                *(*data).searchmark.add(i as usize) = (*data).searchgen;
                            }
                            if (*data).searchgen == u8::MAX {
                                (*data).searchgen = 1;
                            } else {
                                (*data).searchgen += 1;
                            }
                        }
                        px += width;
                    }

                    t = get_timer();
                    if t - tstart > WINDOW_COPY_SEARCH_TIMEOUT {
                        (*data).timeout = 1;
                        break;
                    }
                    if stop != 0 && t > stop {
                        stopped = 1;
                        break;
                    }
                }
                if (*data).timeout != 0 {
                    window_copy_clear_marks(wme);
                    break 'out;
                }

                if stopped != 0 && stop != 0 {
                    /* Try again but just the visible context. */
                    window_copy_visible_lines(data, &raw mut start, &raw mut end);
                    stop = 0;
                    continue 'again;
                }

                if visible_only == 0 {
                    if stopped != 0 {
                        if nfound > 1000 {
                            (*data).searchcount = 1000;
                        } else if nfound > 100 {
                            (*data).searchcount = 100;
                        } else if nfound > 10 {
                            (*data).searchcount = 10;
                        } else {
                            (*data).searchcount = -1;
                        }
                        (*data).searchmore = 1;
                    } else {
                        (*data).searchcount = nfound as i32;
                        (*data).searchmore = 0;
                    }
                }

                break;
            }
        } // out:
        if ssp == &raw mut ss {
            screen_free(&raw mut ss);
        }
        if regex != 0 {
            libc::regfree(&raw mut reg);
        }
        1
    }
}

pub unsafe fn window_copy_clear_marks(wme: *mut window_mode_entry) {
    unsafe {
        let data: *mut window_copy_mode_data = (*wme).data.cast();

        free_((*data).searchmark);
        (*data).searchmark = null_mut();
    }
}

pub unsafe fn window_copy_search_up(wme: *mut window_mode_entry, regex: i32) -> i32 {
    unsafe { window_copy_search(wme, 0, regex) }
}

pub unsafe fn window_copy_search_down(wme: *mut window_mode_entry, regex: i32) -> i32 {
    unsafe { window_copy_search(wme, 1, regex) }
}

pub unsafe fn window_copy_goto_line(wme: *mut window_mode_entry, linestr: *const u8) {
    unsafe {
        let data: *mut window_copy_mode_data = (*wme).data.cast();
        let mut errstr: *const u8 = null();

        let Ok(mut lineno) = strtonum(linestr, -1, i32::MAX) else {
            return;
        };
        if lineno < 0 || lineno as u32 > screen_hsize((*data).backing) {
            lineno = screen_hsize((*data).backing) as i32;
        }

        (*data).oy = lineno as u32;
        window_copy_update_selection(wme, 1, 0);
        window_copy_redraw_screen(wme);
    }
}

pub unsafe fn window_copy_match_start_end(
    data: *mut window_copy_mode_data,
    at: u32,
    start: *mut u32,
    end: *mut u32,
) {
    unsafe {
        let gd: *mut grid = (*(*data).backing).grid;
        let last = ((*gd).sy * (*gd).sx) - 1;
        let mark = *(*data).searchmark.add(at as usize);

        *start = at;
        *end = at;
        while *start != 0 && *(*data).searchmark.add(*start as usize) == mark {
            (*start) -= 1;
        }
        if *(*data).searchmark.add(*start as usize) != mark {
            (*start) += 1;
        }
        while *end != last && *(*data).searchmark.add(*end as usize) == mark {
            (*end) += 1;
        }
        if *(*data).searchmark.add(*end as usize) != mark {
            (*end) -= 1;
        }
    }
}

pub unsafe fn window_copy_match_at_cursor(data: *mut window_copy_mode_data) -> *mut u8 {
    unsafe {
        let gd: *mut grid = (*(*data).backing).grid;
        let mut gc: grid_cell = zeroed();
        let mut at: u32 = 0;
        let mut start: u32 = 0;
        let mut end: u32 = 0;
        let cy: u32 = 0;
        let mut px: u32 = 0;
        let mut py: u32 = 0;
        let sx = screen_size_x((*data).backing);
        let mut buf: *mut u8 = null_mut();
        let mut len: usize = 0;

        if (*data).searchmark.is_null() {
            return null_mut();
        }

        let cy = screen_hsize((*data).backing) - (*data).oy + (*data).cy;
        if window_copy_search_mark_at(data, (*data).cx, cy, &raw mut at) != 0 {
            return null_mut();
        }
        if *(*data).searchmark.add(at as usize) == 0
            && (at == 0
                || ({
                    at -= 1;
                    *(*data).searchmark.add(at as usize) == 0
                }))
        {
            return null_mut();
        } /* Allow one position after the match. */
        window_copy_match_start_end(data, at, &raw mut start, &raw mut end);

        /*
         * Cells will not be set in the marked array unless they are valid text
         * and wrapping will be taken care of, so we can just copy.
         */
        for at in start..=end {
            py = at / sx;
            px = at - (py * sx);

            grid_get_cell(gd, px, (*gd).hsize + py - (*data).oy, &raw mut gc);
            buf = xrealloc(buf.cast(), len + gc.data.size as usize + 1)
                .cast()
                .as_ptr();
            libc::memcpy(
                buf.add(len).cast(),
                (&raw const gc.data.data).cast(),
                gc.data.size as usize,
            );
            len += gc.data.size as usize;
        }
        if len != 0 {
            *buf.add(len) = b'\0';
        }
        buf
    }
}

pub unsafe fn window_copy_update_style(
    wme: *mut window_mode_entry,
    fx: u32,
    fy: u32,
    gc: *mut grid_cell,
    mgc: *const grid_cell,
    cgc: *const grid_cell,
    mkgc: *const grid_cell,
) {
    unsafe {
        let wp: *mut window_pane = (*wme).wp;
        let data: *mut window_copy_mode_data = (*wme).data.cast();
        let mut mark: u32 = 0;
        let mut start: u32 = 0;
        let mut end: u32 = 0;
        let mut cy: u32 = 0;
        let mut cursor: u32 = 0;
        let mut current: u32 = 0;
        let mut inv = 0;
        let mut found = 0;

        if (*data).showmark != 0 && fy == (*data).my {
            (*gc).attr = (*mkgc).attr;
            if fx == (*data).mx {
                inv = 1;
            }
            if inv != 0 {
                (*gc).fg = (*mkgc).bg;
                (*gc).bg = (*mkgc).fg;
            } else {
                (*gc).fg = (*mkgc).fg;
                (*gc).bg = (*mkgc).bg;
            }
        }

        if (*data).searchmark.is_null() {
            return;
        }

        if window_copy_search_mark_at(data, fx, fy, &raw mut current) != 0 {
            return;
        }
        mark = *(*data).searchmark.add(current as usize) as u32;
        if mark == 0 {
            return;
        }

        cy = screen_hsize((*data).backing) - (*data).oy + (*data).cy;
        if window_copy_search_mark_at(data, (*data).cx, cy, &raw mut cursor) == 0 {
            let keys = modekey::try_from(
                options_get_number_((*(*wp).window).options, c"mode-keys") as i32,
            );
            if cursor != 0 && keys == Ok(modekey::MODEKEY_EMACS) && (*data).searchdirection != 0 {
                if *(*data).searchmark.add(cursor as usize - 1) as u32 == mark {
                    cursor -= 1;
                    found = 1;
                }
            } else if *(*data).searchmark.add(cursor as usize) as u32 == mark {
                found = 1;
            }
            if found != 0 {
                window_copy_match_start_end(data, cursor, &raw mut start, &raw mut end);
                if current >= start && current <= end {
                    (*gc).attr = (*cgc).attr;
                    if inv != 0 {
                        (*gc).fg = (*cgc).bg;
                        (*gc).bg = (*cgc).fg;
                    } else {
                        (*gc).fg = (*cgc).fg;
                        (*gc).bg = (*cgc).bg;
                    }
                    return;
                }
            }
        }

        (*gc).attr = (*mgc).attr;
        if inv != 0 {
            (*gc).fg = (*mgc).bg;
            (*gc).bg = (*mgc).fg;
        } else {
            (*gc).fg = (*mgc).fg;
            (*gc).bg = (*mgc).bg;
        }
    }
}

pub unsafe fn window_copy_write_one(
    wme: *mut window_mode_entry,
    ctx: *mut screen_write_ctx,
    py: u32,
    fy: u32,
    nx: u32,
    mgc: *const grid_cell,
    cgc: *const grid_cell,
    mkgc: *const grid_cell,
) {
    unsafe {
        let data: *mut window_copy_mode_data = (*wme).data.cast();
        let gd: *mut grid = (*(*data).backing).grid;
        let mut gc: grid_cell = zeroed();

        screen_write_cursormove(ctx, 0, py as i32, 0);
        for fx in 0..nx {
            grid_get_cell(gd, fx, fy, &raw mut gc);
            if fx + gc.data.width as u32 <= nx {
                window_copy_update_style(wme, fx, fy, &raw mut gc, mgc, cgc, mkgc);
                screen_write_cell(ctx, &raw mut gc);
            }
        }
    }
}

pub unsafe fn window_copy_write_line(
    wme: *mut window_mode_entry,
    ctx: *mut screen_write_ctx,
    py: u32,
) {
    unsafe {
        let wp: *mut window_pane = (*wme).wp;
        let data: *mut window_copy_mode_data = (*wme).data.cast();
        let s: *mut screen = &raw mut (*data).screen;
        let oo: *mut options = (*(*wp).window).options;
        let gl: *mut grid_line;
        let mut gc: grid_cell = zeroed();
        let mut mgc: grid_cell = zeroed();
        let mut cgc: grid_cell = zeroed();
        let mut mkgc: grid_cell = zeroed();
        let mut hdr: [u8; 512] = zeroed();
        let mut tmp: [u8; 512] = zeroed();
        let mut t: *mut u8 = null_mut();
        let mut size: usize = 0;
        let hsize = screen_hsize((*data).backing);

        style_apply(&raw mut gc, oo, c!("mode-style"), null_mut());
        gc.flags |= grid_flag::NOPALETTE;
        style_apply(&raw mut mgc, oo, c!("copy-mode-match-style"), null_mut());
        mgc.flags |= grid_flag::NOPALETTE;
        style_apply(
            &raw mut cgc,
            oo,
            c!("copy-mode-current-match-style"),
            null_mut(),
        );
        cgc.flags |= grid_flag::NOPALETTE;
        style_apply(&raw mut mkgc, oo, c!("copy-mode-mark-style"), null_mut());
        mkgc.flags |= grid_flag::NOPALETTE;

        if py == 0 && (*s).rupper < (*s).rlower && (*data).hide_position == 0 {
            gl = grid_get_line((*(*data).backing).grid, hsize - (*data).oy);
            if (*gl).time == 0 {
                xsnprintf_!((&raw mut tmp).cast(), 512, "[{}/{}]", (*data).oy, hsize,);
            } else {
                t = format_pretty_time((*gl).time, 1);
                xsnprintf_!(
                    (&raw mut tmp).cast(),
                    512,
                    "{} [{}/{}]",
                    _s(t),
                    (*data).oy,
                    hsize,
                );
                free_(t);
            }

            if (*data).searchmark.is_null() {
                if (*data).timeout != 0 {
                    size = xsnprintf_!(
                        (&raw mut hdr).cast(),
                        512,
                        "(timed out) {}",
                        _s(&raw const tmp as *mut u8)
                    )
                    .unwrap() as usize;
                } else {
                    size = xsnprintf_!(
                        (&raw mut hdr).cast(),
                        512,
                        "{}",
                        _s(&raw const tmp as *const u8)
                    )
                    .unwrap() as usize;
                }
            } else if (*data).searchcount == -1 {
                size = xsnprintf_!(
                    (&raw mut hdr).cast(),
                    512,
                    "{}",
                    _s(&raw const tmp as *const u8)
                )
                .unwrap() as usize;
            } else {
                size = xsnprintf_!(
                    (&raw mut hdr).cast(),
                    512,
                    "({}{} results) {}",
                    (*data).searchcount,
                    if (*data).searchmore != 0 { "+" } else { "" },
                    _s(&raw const tmp as *const u8)
                )
                .unwrap() as usize;
            }
            if size > screen_size_x(s) as usize {
                size = screen_size_x(s) as usize;
            }
            screen_write_cursormove(ctx, screen_size_x(s) as i32 - size as i32, 0, 0);
            screen_write_puts!(ctx, &raw mut gc, "{}", _s((&raw const hdr).cast::<u8>()));
        } else {
            size = 0;
        }

        if size < screen_size_x(s) as usize {
            window_copy_write_one(
                wme,
                ctx,
                py,
                hsize - (*data).oy + py,
                screen_size_x(s) - size as u32,
                &raw mut mgc,
                &raw mut cgc,
                &raw mut mkgc,
            );
        }

        if py == (*data).cy && (*data).cx == screen_size_x(s) {
            screen_write_cursormove(ctx, screen_size_x(s) as i32 - 1, py as i32, 0);
            screen_write_putc(ctx, &raw const GRID_DEFAULT_CELL, b'$');
        }
    }
}

pub unsafe fn window_copy_write_lines(
    wme: *mut window_mode_entry,
    ctx: *mut screen_write_ctx,
    py: u32,
    ny: u32,
) {
    unsafe {
        for yy in py..(py + ny) {
            window_copy_write_line(wme, ctx, py);
        }
    }
}

pub unsafe fn window_copy_redraw_selection(wme: *mut window_mode_entry, old_y: u32) {
    unsafe {
        let data: *mut window_copy_mode_data = (*wme).data.cast();
        let gd: *mut grid = (*(*data).backing).grid;

        let new_y = (*data).cy;
        let (start, mut end) = if old_y <= new_y {
            (old_y, new_y)
        } else {
            (new_y, old_y)
        };

        /*
         * In word selection mode the first word on the line below the cursor
         * might be selected, so add this line to the redraw area.
         */
        if (*data).selflag == selflag::SEL_WORD && end < (*gd).sy + (*data).oy - 1 {
            end += 1;
        } /* Last grid line in data coordinates. */
        window_copy_redraw_lines(wme, start, end - start + 1);
    }
}

pub unsafe fn window_copy_redraw_lines(wme: *mut window_mode_entry, py: u32, ny: u32) {
    unsafe {
        let wp: *mut window_pane = (*wme).wp;
        let data: *mut window_copy_mode_data = (*wme).data.cast();
        let mut ctx: screen_write_ctx = zeroed();

        screen_write_start_pane(&raw mut ctx, wp, null_mut());
        for i in py..(py + ny) {
            window_copy_write_line(wme, &raw mut ctx, i);
        }
        screen_write_cursormove(&raw mut ctx, (*data).cx as i32, (*data).cy as i32, 0);
        screen_write_stop(&raw mut ctx);
    }
}

pub unsafe fn window_copy_redraw_screen(wme: *mut window_mode_entry) {
    unsafe {
        let data: *mut window_copy_mode_data = (*wme).data.cast();

        window_copy_redraw_lines(wme, 0, screen_size_y(&raw mut (*data).screen));
    }
}

pub unsafe fn window_copy_synchronize_cursor_end(
    wme: *mut window_mode_entry,
    mut begin: i32,
    no_reset: i32,
) {
    unsafe {
        let data: *mut window_copy_mode_data = (*wme).data.cast();

        let mut xx = (*data).cx;
        let mut yy = screen_hsize((*data).backing) + (*data).cy - (*data).oy;
        match (*data).selflag {
            selflag::SEL_WORD => {
                if no_reset == 0 {
                    begin = 0;
                    if (*data).dy > yy || ((*data).dy == yy && (*data).dx > xx) {
                        /* Right to left selection. */
                        window_copy_cursor_previous_word_pos(
                            wme,
                            (*data).separators,
                            &raw mut xx,
                            &raw mut yy,
                        );
                        begin = 1;

                        /* Reset the end. */
                        (*data).endselx = (*data).endselrx;
                        (*data).endsely = (*data).endselry;
                    } else {
                        /* Left to right selection. */
                        if xx >= window_copy_find_length(wme, yy)
                            || window_copy_in_set(wme, xx + 1, yy, WHITESPACE) == 0
                        {
                            window_copy_cursor_next_word_end_pos(
                                wme,
                                (*data).separators,
                                &raw mut xx,
                                &raw mut yy,
                            );
                        }

                        /* Reset the start. */
                        (*data).selx = (*data).selrx;
                        (*data).sely = (*data).selry;
                    }
                }
            }
            selflag::SEL_LINE => {
                if no_reset == 0 {
                    begin = 0;
                    if (*data).dy > yy {
                        /* Right to left selection. */
                        xx = 0;
                        begin = 1;

                        /* Reset the end. */
                        (*data).endselx = (*data).endselrx;
                        (*data).endsely = (*data).endselry;
                    } else {
                        /* Left to right selection. */
                        if yy < (*data).endselry {
                            yy = (*data).endselry;
                        }
                        xx = window_copy_find_length(wme, yy);

                        /* Reset the start. */
                        (*data).selx = (*data).selrx;
                        (*data).sely = (*data).selry;
                    }
                }
            }
            selflag::SEL_CHAR => (),
        }
        if begin != 0 {
            (*data).selx = xx;
            (*data).sely = yy;
        } else {
            (*data).endselx = xx;
            (*data).endsely = yy;
        }
    }
}

pub unsafe fn window_copy_synchronize_cursor(wme: *mut window_mode_entry, no_reset: i32) {
    unsafe {
        let data: *mut window_copy_mode_data = (*wme).data.cast();

        match (*data).cursordrag {
            cursordrag::CURSORDRAG_ENDSEL => window_copy_synchronize_cursor_end(wme, 0, no_reset),
            cursordrag::CURSORDRAG_SEL => window_copy_synchronize_cursor_end(wme, 1, no_reset),
            cursordrag::CURSORDRAG_NONE => (),
        }
    }
}

pub unsafe fn window_copy_update_cursor(wme: *mut window_mode_entry, cx: u32, cy: u32) {
    unsafe {
        let wp: *mut window_pane = (*wme).wp;
        let data: *mut window_copy_mode_data = (*wme).data.cast();
        let s: *mut screen = &raw mut (*data).screen;
        let mut ctx: screen_write_ctx = zeroed();

        let old_cx = (*data).cx;
        let old_cy = (*data).cy;
        (*data).cx = cx;
        (*data).cy = cy;
        if old_cx == screen_size_x(s) {
            window_copy_redraw_lines(wme, old_cy, 1);
        }
        if (*data).cx == screen_size_x(s) {
            window_copy_redraw_lines(wme, (*data).cy, 1);
        } else {
            screen_write_start_pane(&raw mut ctx, wp, null_mut());
            screen_write_cursormove(&raw mut ctx, (*data).cx as i32, (*data).cy as i32, 0);
            screen_write_stop(&raw mut ctx);
        }
    }
}

pub unsafe fn window_copy_start_selection(wme: *mut window_mode_entry) {
    unsafe {
        let data: *mut window_copy_mode_data = (*wme).data.cast();

        (*data).selx = (*data).cx;
        (*data).sely = screen_hsize((*data).backing) + (*data).cy - (*data).oy;

        (*data).endselx = (*data).selx;
        (*data).endsely = (*data).sely;

        (*data).cursordrag = cursordrag::CURSORDRAG_ENDSEL;

        window_copy_set_selection(wme, 1, 0);
    }
}

pub unsafe fn window_copy_adjust_selection(
    wme: *mut window_mode_entry,
    selx: *mut u32,
    sely: *mut u32,
) -> window_copy_rel_pos {
    unsafe {
        let data: *mut window_copy_mode_data = (*wme).data.cast();
        let s: *mut screen = &raw mut (*data).screen;

        let mut sx = *selx;
        let mut sy = *sely;
        let mut relpos = window_copy_rel_pos::WINDOW_COPY_REL_POS_ABOVE;

        let ty = screen_hsize((*data).backing) - (*data).oy;
        if sy < ty {
            relpos = window_copy_rel_pos::WINDOW_COPY_REL_POS_ABOVE;
            if (*data).rectflag == 0 {
                sx = 0;
            }
            sy = 0;
        } else if sy > ty + screen_size_y(s) - 1 {
            relpos = window_copy_rel_pos::WINDOW_COPY_REL_POS_BELOW;
            if (*data).rectflag == 0 {
                sx = screen_size_x(s) - 1;
            }
            sy = screen_size_y(s) - 1;
        } else {
            relpos = window_copy_rel_pos::WINDOW_COPY_REL_POS_ON_SCREEN;
            sy -= ty;
        }

        *selx = sx;
        *sely = sy;
        relpos
    }
}

pub unsafe fn window_copy_update_selection(
    wme: *mut window_mode_entry,
    may_redraw: i32,
    no_reset: i32,
) -> i32 {
    unsafe {
        let data: *mut window_copy_mode_data = (*wme).data.cast();
        let s: *mut screen = &raw mut (*data).screen;

        if (*s).sel.is_null() && (*data).lineflag == line_sel::LINE_SEL_NONE {
            return 0;
        }
        window_copy_set_selection(wme, may_redraw, no_reset)
    }
}

pub unsafe fn window_copy_set_selection(
    wme: *mut window_mode_entry,
    may_redraw: i32,
    no_reset: i32,
) -> i32 {
    unsafe {
        let wp: *mut window_pane = (*wme).wp;
        let data: *mut window_copy_mode_data = (*wme).data.cast();
        let s: *mut screen = &raw mut (*data).screen;
        let oo: *mut options = (*(*wp).window).options;
        let mut gc: grid_cell = zeroed();
        // u_int sx, sy, cy, endsx, endsy;
        // int startrelpos, endrelpos;

        window_copy_synchronize_cursor(wme, no_reset);

        /* Adjust the selection. */
        let mut sx = (*data).selx;
        let mut sy = (*data).sely;
        let startrelpos = window_copy_adjust_selection(wme, &raw mut sx, &raw mut sy);

        /* Adjust the end of selection. */
        let mut endsx = (*data).endselx;
        let mut endsy = (*data).endsely;
        let endrelpos = window_copy_adjust_selection(wme, &raw mut endsx, &raw mut endsy);

        /* Selection is outside of the current screen */
        if startrelpos == endrelpos
            && startrelpos != window_copy_rel_pos::WINDOW_COPY_REL_POS_ON_SCREEN
        {
            screen_hide_selection(s);
            return 0;
        }

        /* Set colours and selection. */
        style_apply(&raw mut gc, oo, c!("mode-style"), null_mut());
        gc.flags |= grid_flag::NOPALETTE;
        screen_set_selection(
            s,
            sx,
            sy,
            endsx,
            endsy,
            (*data).rectflag as u32,
            (*data).modekeys,
            &raw mut gc,
        );

        if (*data).rectflag != 0 && may_redraw != 0 {
            /*
             * Can't rely on the caller to redraw the right lines for
             * rectangle selection - find the highest line and the number
             * of lines, and redraw just past that in both directions
             */
            let cy = (*data).cy;
            if (*data).cursordrag == cursordrag::CURSORDRAG_ENDSEL {
                if sy < cy {
                    window_copy_redraw_lines(wme, sy, cy - sy + 1);
                } else {
                    window_copy_redraw_lines(wme, cy, sy - cy + 1);
                }
            } else if endsy < cy {
                window_copy_redraw_lines(wme, endsy, cy - endsy + 1);
            } else {
                window_copy_redraw_lines(wme, cy, endsy - cy + 1);
            }
        }

        1
    }
}

pub unsafe fn window_copy_get_selection(wme: *mut window_mode_entry, len: *mut usize) -> *mut u8 {
    unsafe {
        let wp: *mut window_pane = (*wme).wp;
        let data: *mut window_copy_mode_data = (*wme).data.cast();
        let s: *mut screen = &raw mut (*data).screen;

        let mut buf: *mut u8 = null_mut();
        let mut off: usize = 0;

        // u_int i, xx, yy, sx, sy, ex, ey, ey_last;
        // u_int firstsx, lastex, restex, restsx, selx;

        let mut lastex = 0;
        let mut restex = 0;
        let mut firstsx = 0;
        let mut restsx = 0;

        if (*data).screen.sel.is_null() && (*data).lineflag == line_sel::LINE_SEL_NONE {
            buf = window_copy_match_at_cursor(data);
            if !buf.is_null() {
                *len = strlen(buf);
            } else {
                *len = 0;
            }
            return buf;
        }

        buf = xmalloc(1).as_ptr().cast();
        off = 0;

        *buf = b'\0';

        /*
         * The selection extends from selx,sely to (adjusted) cx,cy on
         * the base screen.
         */

        /* Find start and end. */
        let mut xx = (*data).endselx;
        let yy = (*data).endsely;
        let (sx, sy, mut ex, ey) = if yy < (*data).sely || (yy == (*data).sely && xx < (*data).selx)
        {
            (xx, yy, (*data).selx, (*data).sely)
        } else {
            ((*data).selx, (*data).sely, xx, yy)
        };

        /* Trim ex to end of line. */
        let ey_last = window_copy_find_length(wme, ey);
        if ex > ey_last {
            ex = ey_last;
        }

        /*
         * Deal with rectangle-copy if necessary; four situations: start of
         * first line (firstsx), end of last line (lastex), start (restsx) and
         * end (restex) of all other lines.
         */
        xx = screen_size_x(s);

        /*
         * Behave according to mode-keys. If it is emacs, copy like emacs,
         * keeping the top-left-most character, and dropping the
         * bottom-right-most, regardless of copy direction. If it is vi, also
         * keep bottom-right-most character.
         */
        let keys =
            modekey::try_from(options_get_number_((*(*wp).window).options, c"mode-keys") as i32);
        if (*data).rectflag != 0 {
            /*
             * Need to ignore the column with the cursor in it, which for
             * rectangular copy means knowing which side the cursor is on.
             */
            let selx = if (*data).cursordrag == cursordrag::CURSORDRAG_ENDSEL {
                (*data).selx
            } else {
                (*data).endselx
            };

            if selx < (*data).cx {
                /* Selection start is on the left. */
                if keys == Ok(modekey::MODEKEY_EMACS) {
                    lastex = (*data).cx;
                    restex = (*data).cx;
                } else {
                    lastex = (*data).cx + 1;
                    restex = (*data).cx + 1;
                }
                firstsx = selx;
                restsx = selx;
            } else {
                /* Cursor is on the left. */
                lastex = selx + 1;
                restex = selx + 1;
                firstsx = (*data).cx;
                restsx = (*data).cx;
            }
        } else {
            if keys == Ok(modekey::MODEKEY_EMACS) {
                lastex = ex;
            } else {
                lastex = ex + 1;
            }
            restex = xx;
            firstsx = sx;
            restsx = 0;
        }

        /* Copy the lines. */
        for i in sy..=ey {
            window_copy_copy_line(
                wme,
                &raw mut buf,
                &raw mut off,
                i,
                if i == sy { firstsx } else { restsx },
                if i == ey { lastex } else { restex },
            );
        }

        /* Don't bother if no data. */
        if off == 0 {
            free_(buf);
            *len = 0;
            return null_mut();
        }
        /* Remove final \n (unless at end in vi mode). */
        if (keys == Ok(modekey::MODEKEY_EMACS) || lastex <= ey_last)
            && (!(*grid_get_line((*(*data).backing).grid, ey))
                .flags
                .intersects(grid_line_flag::WRAPPED)
                || lastex != ey_last)
        {
            off -= 1;
        }
        *len = off;
        buf
    }
}

pub unsafe fn window_copy_copy_buffer(
    wme: *mut window_mode_entry,
    prefix: *const u8,
    buf: *mut c_void,
    len: usize,
) {
    unsafe {
        let wp: *mut window_pane = (*wme).wp;
        let mut ctx: screen_write_ctx = zeroed();

        if options_get_number_(GLOBAL_OPTIONS, c"set-clipboard") != 0 {
            screen_write_start_pane(&raw mut ctx, wp, null_mut());
            screen_write_setselection(&raw mut ctx, c!(""), buf.cast(), len as u32);
            screen_write_stop(&raw mut ctx);
            notify_pane(c"pane-set-clipboard", wp);
        }

        paste_add(prefix, buf.cast(), len);
    }
}

pub unsafe fn window_copy_pipe_run(
    wme: *mut window_mode_entry,
    s: *mut session,
    mut cmd: *const u8,
    len: *mut usize,
) -> *mut c_void {
    unsafe {
        let buf = window_copy_get_selection(wme, len);
        if cmd.is_null() || *cmd == b'\0' {
            cmd = options_get_string_(GLOBAL_OPTIONS, c"copy-command");
        }
        if !cmd.is_null() && *cmd != b'\0' {
            let job = job_run(
                cmd,
                0,
                null_mut(),
                null_mut(),
                s,
                null_mut(),
                None,
                None,
                None,
                null_mut(),
                job_flag::JOB_NOWAIT,
                -1,
                -1,
            );
            bufferevent_write(job_get_event(job), buf.cast(), *len);
        }
        buf.cast()
    }
}

pub unsafe fn window_copy_pipe(wme: *mut window_mode_entry, s: *mut session, cmd: *const u8) {
    unsafe {
        let mut len: usize = 0;

        window_copy_pipe_run(wme, s, cmd, &raw mut len);
    }
}

pub unsafe fn window_copy_copy_pipe(
    wme: *mut window_mode_entry,
    s: *mut session,
    prefix: *const u8,
    cmd: *const u8,
) {
    unsafe {
        let mut len: usize = 0;
        let buf = window_copy_pipe_run(wme, s, cmd, &raw mut len);
        if !buf.is_null() {
            window_copy_copy_buffer(wme, prefix, buf, len);
        }
    }
}

pub unsafe fn window_copy_copy_selection(wme: *mut window_mode_entry, prefix: *const u8) {
    unsafe {
        let mut len: usize = 0;
        let buf = window_copy_get_selection(wme, &raw mut len);
        if !buf.is_null() {
            window_copy_copy_buffer(wme, prefix, buf.cast(), len);
        }
    }
}

pub unsafe fn window_copy_append_selection(wme: *mut window_mode_entry) {
    unsafe {
        let wp: *mut window_pane = (*wme).wp;
        let mut pb: *mut paste_buffer = null_mut();
        let mut bufdata: *const u8 = null_mut();
        let mut bufname: *const u8 = null();
        let mut ctx: screen_write_ctx = zeroed();
        let mut bufsize = 0;
        let mut len: usize = 0;
        let mut buf = window_copy_get_selection(wme, &raw mut len);
        if buf.is_null() {
            return;
        }

        if options_get_number_(GLOBAL_OPTIONS, c"set-clipboard") != 0 {
            screen_write_start_pane(&raw mut ctx, wp, null_mut());
            screen_write_setselection(&raw mut ctx, c!(""), buf.cast(), len as u32);
            screen_write_stop(&raw mut ctx);
            notify_pane(c"pane-set-clipboard", wp);
        }

        pb = paste_get_top(&raw mut bufname);
        if !pb.is_null() {
            bufdata = paste_buffer_data(pb, &raw mut bufsize);
            buf = xrealloc(buf.cast(), len + bufsize).as_ptr().cast();
            libc::memmove(buf.add(bufsize).cast(), buf.cast(), len);
            libc::memcpy(buf.cast(), bufdata.cast(), bufsize);
            len += bufsize;
        }
        if paste_set(buf, len, bufname, null_mut()) != 0 {
            free_(buf);
        }
    }
}

pub unsafe fn window_copy_copy_line(
    wme: *mut window_mode_entry,
    buf: *mut *mut u8,
    off: *mut usize,
    sy: u32,
    mut sx: u32,
    mut ex: u32,
) {
    unsafe {
        let data: *mut window_copy_mode_data = (*wme).data.cast();
        let gd: *mut grid = (*(*data).backing).grid;
        let mut gc: grid_cell = zeroed();
        let mut ud: utf8_data = zeroed();
        // const char *s;
        let mut wrapped: u32 = 0;

        if sx > ex {
            return;
        }

        /*
         * Work out if the line was wrapped at the screen edge and all of it is
         * on screen.
         */
        let gl = grid_get_line(gd, sy);
        if (*gl).flags.intersects(grid_line_flag::WRAPPED) && (*gl).cellsize <= (*gd).sx {
            wrapped = 1;
        }

        /* If the line was wrapped, don't strip spaces (use the full length). */
        let xx = if wrapped != 0 {
            (*gl).cellsize
        } else {
            window_copy_find_length(wme, sy)
        };

        if ex > xx {
            ex = xx;
        }
        if sx > xx {
            sx = xx;
        }

        if sx < ex {
            for i in sx..ex {
                grid_get_cell(gd, i, sy, &raw mut gc);
                if gc.flags.intersects(grid_flag::PADDING) {
                    continue;
                }
                utf8_copy(&raw mut ud, &raw mut gc.data);
                if ud.size == 1 && gc.attr.intersects(grid_attr::GRID_ATTR_CHARSET) {
                    let s = tty_acs_get(null_mut(), ud.data[0]);
                    if !s.is_null() && strlen(s) <= UTF8_SIZE {
                        ud.size = strlen(s) as u8;
                        libc::memcpy((&raw mut ud.data).cast(), s.cast(), ud.size as usize);
                    }
                }

                *buf = xrealloc((*buf).cast(), (*off) + ud.size as usize)
                    .as_ptr()
                    .cast();
                libc::memcpy(
                    (*buf).add(*off).cast(),
                    (&raw const ud.data).cast(),
                    ud.size as usize,
                );
                *off += ud.size as usize;
            }
        }

        /* Only add a newline if the line wasn't wrapped. */
        if wrapped == 0 || ex != xx {
            *buf = xrealloc((*buf).cast(), (*off) + 1).as_ptr().cast();
            *(*buf).add(*off) = b'\n';
            (*off) += 1;
        }
    }
}

pub unsafe fn window_copy_clear_selection(wme: *mut window_mode_entry) {
    unsafe {
        let data: *mut window_copy_mode_data = (*wme).data.cast();

        screen_clear_selection(&raw mut (*data).screen);

        (*data).cursordrag = cursordrag::CURSORDRAG_NONE;
        (*data).lineflag = line_sel::LINE_SEL_NONE;
        (*data).selflag = selflag::SEL_CHAR;

        let py = screen_hsize((*data).backing) + (*data).cy - (*data).oy;
        let px = window_copy_find_length(wme, py);
        if (*data).cx > px {
            window_copy_update_cursor(wme, px, (*data).cy);
        }
    }
}

pub unsafe fn window_copy_in_set(
    wme: *mut window_mode_entry,
    px: u32,
    py: u32,
    set: *const u8,
) -> i32 {
    unsafe {
        let data: *mut window_copy_mode_data = (*wme).data.cast();
        let mut gc: grid_cell = zeroed();

        grid_get_cell((*(*data).backing).grid, px, py, &raw mut gc);
        if gc.flags.intersects(grid_flag::PADDING) {
            return 0;
        }
        utf8_cstrhas(set, &gc.data)
    }
}

pub unsafe fn window_copy_find_length(wme: *mut window_mode_entry, py: u32) -> u32 {
    unsafe {
        let data: *mut window_copy_mode_data = (*wme).data.cast();

        grid_line_length((*(*data).backing).grid, py)
    }
}

pub unsafe fn window_copy_cursor_start_of_line(wme: *mut window_mode_entry) {
    unsafe {
        let data: *mut window_copy_mode_data = (*wme).data.cast();
        let back_s: *mut screen = (*data).backing;
        let mut gr: grid_reader = zeroed();

        let mut px = (*data).cx;
        let hsize = screen_hsize(back_s);
        let mut py = hsize + (*data).cy - (*data).oy;
        let oldy = (*data).cy;

        grid_reader_start(&raw mut gr, (*back_s).grid, px, py);
        grid_reader_cursor_start_of_line(&raw mut gr, 1);
        grid_reader_get_cursor(&raw mut gr, &raw mut px, &raw mut py);
        window_copy_acquire_cursor_up(wme, hsize, (*data).oy, oldy, px, py);
    }
}

pub unsafe fn window_copy_cursor_back_to_indentation(wme: *mut window_mode_entry) {
    unsafe {
        let data: *mut window_copy_mode_data = (*wme).data.cast();
        let back_s: *mut screen = (*data).backing;
        let mut gr: grid_reader = zeroed();

        let mut px = (*data).cx;
        let hsize = screen_hsize(back_s);
        let mut py = hsize + (*data).cy - (*data).oy;
        let oldy = (*data).cy;

        grid_reader_start(&raw mut gr, (*back_s).grid, px, py);
        grid_reader_cursor_back_to_indentation(&raw mut gr);
        grid_reader_get_cursor(&raw mut gr, &raw mut px, &raw mut py);
        window_copy_acquire_cursor_up(wme, hsize, (*data).oy, oldy, px, py);
    }
}

pub unsafe fn window_copy_cursor_end_of_line(wme: *mut window_mode_entry) {
    unsafe {
        let data: *mut window_copy_mode_data = (*wme).data.cast();
        let back_s: *mut screen = (*data).backing;
        let mut gr: grid_reader = zeroed();

        let mut px = (*data).cx;
        let hsize = screen_hsize(back_s);
        let mut py = hsize + (*data).cy - (*data).oy;
        let oldy = (*data).cy;

        grid_reader_start(&raw mut gr, (*back_s).grid, px, py);
        if !(*data).screen.sel.is_null() && (*data).rectflag != 0 {
            grid_reader_cursor_end_of_line(&raw mut gr, 1, 1);
        } else {
            grid_reader_cursor_end_of_line(&raw mut gr, 1, 0);
        }
        grid_reader_get_cursor(&raw mut gr, &raw mut px, &raw mut py);
        window_copy_acquire_cursor_down(
            wme,
            hsize,
            screen_size_y(back_s),
            (*data).oy,
            oldy,
            px,
            py,
            0,
        );
    }
}

pub unsafe fn window_copy_other_end(wme: *mut window_mode_entry) {
    unsafe {
        let data: *mut window_copy_mode_data = (*wme).data.cast();
        let s: *mut screen = &raw mut (*data).screen;
        // u_int selx, sely, cy, yy, hsize;

        if (*s).sel.is_null() && (*data).lineflag == line_sel::LINE_SEL_NONE {
            return;
        }

        if (*data).lineflag == line_sel::LINE_SEL_LEFT_RIGHT {
            (*data).lineflag = line_sel::LINE_SEL_RIGHT_LEFT;
        } else if (*data).lineflag == line_sel::LINE_SEL_RIGHT_LEFT {
            (*data).lineflag = line_sel::LINE_SEL_LEFT_RIGHT;
        }

        match (*data).cursordrag {
            cursordrag::CURSORDRAG_NONE | cursordrag::CURSORDRAG_SEL => {
                (*data).cursordrag = cursordrag::CURSORDRAG_ENDSEL
            }
            cursordrag::CURSORDRAG_ENDSEL => (*data).cursordrag = cursordrag::CURSORDRAG_SEL,
        }

        let mut selx = (*data).endselx;
        let mut sely = (*data).endsely;
        if (*data).cursordrag == cursordrag::CURSORDRAG_SEL {
            selx = (*data).selx;
            sely = (*data).sely;
        }

        let cy = (*data).cy;
        let yy = screen_hsize((*data).backing) + (*data).cy - (*data).oy;

        (*data).cx = selx;

        let hsize = screen_hsize((*data).backing);
        if sely < hsize - (*data).oy {
            /* above */
            (*data).oy = hsize - sely;
            (*data).cy = 0;
        } else if sely > hsize - (*data).oy + screen_size_y(s) {
            /* below */
            (*data).oy = hsize - sely + screen_size_y(s) - 1;
            (*data).cy = screen_size_y(s) - 1;
        } else {
            (*data).cy = cy + sely - yy;
        }

        window_copy_update_selection(wme, 1, 1);
        window_copy_redraw_screen(wme);
    }
}

pub unsafe fn window_copy_cursor_left(wme: *mut window_mode_entry) {
    unsafe {
        let data: *mut window_copy_mode_data = (*wme).data.cast();
        let back_s: *mut screen = (*data).backing;
        let mut gr: grid_reader = zeroed();

        let mut px = (*data).cx;
        let hsize = screen_hsize(back_s);
        let mut py = hsize + (*data).cy - (*data).oy;
        let oldy = (*data).cy;

        grid_reader_start(&raw mut gr, (*back_s).grid, px, py);
        grid_reader_cursor_left(&raw mut gr, 1);
        grid_reader_get_cursor(&raw mut gr, &raw mut px, &raw mut py);
        window_copy_acquire_cursor_up(wme, hsize, (*data).oy, oldy, px, py);
    }
}

pub unsafe fn window_copy_cursor_right(wme: *mut window_mode_entry, all: i32) {
    unsafe {
        let data: *mut window_copy_mode_data = (*wme).data.cast();
        let back_s: *mut screen = (*data).backing;
        let mut gr: grid_reader = zeroed();

        let mut px = (*data).cx;
        let hsize = screen_hsize(back_s);
        let mut py = hsize + (*data).cy - (*data).oy;
        let oldy = (*data).cy;

        grid_reader_start(&raw mut gr, (*back_s).grid, px, py);
        grid_reader_cursor_right(&raw mut gr, 1, all);
        grid_reader_get_cursor(&raw mut gr, &raw mut px, &raw mut py);
        window_copy_acquire_cursor_down(
            wme,
            hsize,
            screen_size_y(back_s),
            (*data).oy,
            oldy,
            px,
            py,
            0,
        );
    }
}

pub unsafe fn window_copy_cursor_up(wme: *mut window_mode_entry, scroll_only: i32) {
    unsafe {
        let data: *mut window_copy_mode_data = (*wme).data.cast();
        let s: *mut screen = &raw mut (*data).screen;
        let mut px = 0;
        let mut py = 0;

        let norectsel = (*data).screen.sel.is_null() || (*data).rectflag == 0;
        let oy = screen_hsize((*data).backing) + (*data).cy - (*data).oy;
        let ox = window_copy_find_length(wme, oy);
        if norectsel && (*data).cx != ox {
            (*data).lastcx = (*data).cx;
            (*data).lastsx = ox;
        }

        if (*data).lineflag == line_sel::LINE_SEL_LEFT_RIGHT && oy == (*data).sely {
            window_copy_other_end(wme);
        }

        if scroll_only != 0 || (*data).cy == 0 {
            if norectsel {
                (*data).cx = (*data).lastcx;
            }
            window_copy_scroll_down(wme, 1);
            if scroll_only != 0 {
                if (*data).cy == screen_size_y(s) - 1 {
                    window_copy_redraw_lines(wme, (*data).cy, 1);
                } else {
                    window_copy_redraw_lines(wme, (*data).cy, 2);
                }
            }
        } else {
            if norectsel {
                window_copy_update_cursor(wme, (*data).lastcx, (*data).cy - 1);
            } else {
                window_copy_update_cursor(wme, (*data).cx, (*data).cy - 1);
            }
            if window_copy_update_selection(wme, 1, 0) != 0 {
                if (*data).cy == screen_size_y(s) - 1 {
                    window_copy_redraw_lines(wme, (*data).cy, 1);
                } else {
                    window_copy_redraw_lines(wme, (*data).cy, 2);
                }
            }
        }

        if norectsel {
            py = screen_hsize((*data).backing) + (*data).cy - (*data).oy;
            px = window_copy_find_length(wme, py);
            if ((*data).cx >= (*data).lastsx && (*data).cx != px) || (*data).cx > px {
                window_copy_update_cursor(wme, px, (*data).cy);
                if window_copy_update_selection(wme, 1, 0) != 0 {
                    window_copy_redraw_lines(wme, (*data).cy, 1);
                }
            }
        }

        if (*data).lineflag == line_sel::LINE_SEL_LEFT_RIGHT {
            py = screen_hsize((*data).backing) + (*data).cy - (*data).oy;
            if (*data).rectflag != 0 {
                px = screen_size_x((*data).backing);
            } else {
                px = window_copy_find_length(wme, py);
            }
            window_copy_update_cursor(wme, px, (*data).cy);
            if window_copy_update_selection(wme, 1, 0) != 0 {
                window_copy_redraw_lines(wme, (*data).cy, 1);
            }
        } else if (*data).lineflag == line_sel::LINE_SEL_RIGHT_LEFT {
            window_copy_update_cursor(wme, 0, (*data).cy);
            if window_copy_update_selection(wme, 1, 0) != 0 {
                window_copy_redraw_lines(wme, (*data).cy, 1);
            }
        }
    }
}

pub unsafe fn window_copy_cursor_down(wme: *mut window_mode_entry, scroll_only: i32) {
    unsafe {
        let data: *mut window_copy_mode_data = (*wme).data.cast();
        let s: *mut screen = &raw mut (*data).screen;
        let mut px = 0;
        let mut py = 0;

        let norectsel = (*data).screen.sel.is_null() || (*data).rectflag == 0;
        let oy = screen_hsize((*data).backing) + (*data).cy - (*data).oy;
        let ox = window_copy_find_length(wme, oy);
        if norectsel && (*data).cx != ox {
            (*data).lastcx = (*data).cx;
            (*data).lastsx = ox;
        }

        if (*data).lineflag == line_sel::LINE_SEL_RIGHT_LEFT && oy == (*data).endsely {
            window_copy_other_end(wme);
        }

        if scroll_only != 0 || (*data).cy == screen_size_y(s) - 1 {
            if norectsel {
                (*data).cx = (*data).lastcx;
            }
            window_copy_scroll_up(wme, 1);
            if scroll_only != 0 && (*data).cy > 0 {
                window_copy_redraw_lines(wme, (*data).cy - 1, 2);
            }
        } else {
            if norectsel {
                window_copy_update_cursor(wme, (*data).lastcx, (*data).cy + 1);
            } else {
                window_copy_update_cursor(wme, (*data).cx, (*data).cy + 1);
            }
            if window_copy_update_selection(wme, 1, 0) != 0 {
                window_copy_redraw_lines(wme, (*data).cy - 1, 2);
            }
        }

        if norectsel {
            py = screen_hsize((*data).backing) + (*data).cy - (*data).oy;
            px = window_copy_find_length(wme, py);
            if ((*data).cx >= (*data).lastsx && (*data).cx != px) || (*data).cx > px {
                window_copy_update_cursor(wme, px, (*data).cy);
                if window_copy_update_selection(wme, 1, 0) != 0 {
                    window_copy_redraw_lines(wme, (*data).cy, 1);
                }
            }
        }

        if (*data).lineflag == line_sel::LINE_SEL_LEFT_RIGHT {
            py = screen_hsize((*data).backing) + (*data).cy - (*data).oy;
            if (*data).rectflag != 0 {
                px = screen_size_x((*data).backing);
            } else {
                px = window_copy_find_length(wme, py);
            }
            window_copy_update_cursor(wme, px, (*data).cy);
            if window_copy_update_selection(wme, 1, 0) != 0 {
                window_copy_redraw_lines(wme, (*data).cy, 1);
            }
        } else if (*data).lineflag == line_sel::LINE_SEL_RIGHT_LEFT {
            window_copy_update_cursor(wme, 0, (*data).cy);
            if window_copy_update_selection(wme, 1, 0) != 0 {
                window_copy_redraw_lines(wme, (*data).cy, 1);
            }
        }
    }
}

pub unsafe fn window_copy_cursor_jump(wme: *mut window_mode_entry) {
    unsafe {
        let data: *mut window_copy_mode_data = (*wme).data.cast();
        let back_s: *mut screen = (*data).backing;
        let mut gr: grid_reader = zeroed();

        let mut px = (*data).cx + 1;
        let hsize = screen_hsize(back_s);
        let mut py = hsize + (*data).cy - (*data).oy;
        let oldy = (*data).cy;

        grid_reader_start(&raw mut gr, (*back_s).grid, px, py);
        if grid_reader_cursor_jump(&raw mut gr, (*data).jumpchar.as_ref().unwrap()) != 0 {
            grid_reader_get_cursor(&raw mut gr, &raw mut px, &raw mut py);
            window_copy_acquire_cursor_down(
                wme,
                hsize,
                screen_size_y(back_s),
                (*data).oy,
                oldy,
                px,
                py,
                0,
            );
        }
    }
}

pub unsafe fn window_copy_cursor_jump_back(wme: *mut window_mode_entry) {
    unsafe {
        let data: *mut window_copy_mode_data = (*wme).data.cast();
        let back_s: *mut screen = (*data).backing;
        let mut gr: grid_reader = zeroed();

        let mut px = (*data).cx;
        let hsize = screen_hsize(back_s);
        let mut py = hsize + (*data).cy - (*data).oy;
        let oldy = (*data).cy;

        grid_reader_start(&raw mut gr, (*back_s).grid, px, py);
        grid_reader_cursor_left(&raw mut gr, 0);
        if grid_reader_cursor_jump_back(&raw mut gr, (*data).jumpchar.as_ref().unwrap()) != 0 {
            grid_reader_get_cursor(&raw mut gr, &raw mut px, &raw mut py);
            window_copy_acquire_cursor_up(wme, hsize, (*data).oy, oldy, px, py);
        }
    }
}

pub unsafe fn window_copy_cursor_jump_to(wme: *mut window_mode_entry) {
    unsafe {
        let data: *mut window_copy_mode_data = (*wme).data.cast();
        let back_s: *mut screen = (*data).backing;
        let mut gr: grid_reader = zeroed();

        let mut px = (*data).cx + 2;
        let hsize = screen_hsize(back_s);
        let mut py = hsize + (*data).cy - (*data).oy;
        let oldy = (*data).cy;

        grid_reader_start(&raw mut gr, (*back_s).grid, px, py);
        if grid_reader_cursor_jump(&raw mut gr, (*data).jumpchar.as_ref().unwrap()) != 0 {
            grid_reader_cursor_left(&raw mut gr, 1);
            grid_reader_get_cursor(&raw mut gr, &raw mut px, &raw mut py);
            window_copy_acquire_cursor_down(
                wme,
                hsize,
                screen_size_y(back_s),
                (*data).oy,
                oldy,
                px,
                py,
                0,
            );
        }
    }
}

pub unsafe fn window_copy_cursor_jump_to_back(wme: *mut window_mode_entry) {
    unsafe {
        let data: *mut window_copy_mode_data = (*wme).data.cast();
        let back_s: *mut screen = (*data).backing;
        let mut gr: grid_reader = zeroed();

        let mut px = (*data).cx;
        let hsize = screen_hsize(back_s);
        let mut py = hsize + (*data).cy - (*data).oy;
        let oldy = (*data).cy;

        grid_reader_start(&raw mut gr, (*back_s).grid, px, py);
        grid_reader_cursor_left(&raw mut gr, 0);
        grid_reader_cursor_left(&raw mut gr, 0);
        if grid_reader_cursor_jump_back(&raw mut gr, (*data).jumpchar.as_ref().unwrap()) != 0 {
            grid_reader_cursor_right(&raw mut gr, 1, 0);
            grid_reader_get_cursor(&raw mut gr, &raw mut px, &raw mut py);
            window_copy_acquire_cursor_up(wme, hsize, (*data).oy, oldy, px, py);
        }
    }
}

pub unsafe fn window_copy_cursor_next_word(wme: *mut window_mode_entry, separators: *const u8) {
    unsafe {
        let data: *mut window_copy_mode_data = (*wme).data.cast();
        let back_s: *mut screen = (*data).backing;
        let mut gr: grid_reader = zeroed();

        let mut px = (*data).cx;
        let hsize = screen_hsize(back_s);
        let mut py = hsize + (*data).cy - (*data).oy;
        let oldy = (*data).cy;

        grid_reader_start(&raw mut gr, (*back_s).grid, px, py);
        grid_reader_cursor_next_word(&raw mut gr, separators);
        grid_reader_get_cursor(&raw mut gr, &raw mut px, &raw mut py);
        window_copy_acquire_cursor_down(
            wme,
            hsize,
            screen_size_y(back_s),
            (*data).oy,
            oldy,
            px,
            py,
            0,
        );
    }
}

/* Compute the next place where a word ends. */

pub unsafe fn window_copy_cursor_next_word_end_pos(
    wme: *mut window_mode_entry,
    separators: *const u8,
    ppx: *mut u32,
    ppy: *mut u32,
) {
    unsafe {
        let wp: *mut window_pane = (*wme).wp;
        let data: *mut window_copy_mode_data = (*wme).data.cast();
        let oo: *mut options = (*(*wp).window).options;
        let back_s: *mut screen = (*data).backing;
        let mut gr: grid_reader = zeroed();

        let mut px = (*data).cx;
        let hsize = screen_hsize(back_s);
        let mut py = hsize + (*data).cy - (*data).oy;

        grid_reader_start(&raw mut gr, (*back_s).grid, px, py);
        if modekey::try_from(options_get_number_(oo, c"mode-keys") as i32)
            == Ok(modekey::MODEKEY_VI)
        {
            if grid_reader_in_set(&raw mut gr, WHITESPACE) == 0 {
                grid_reader_cursor_right(&raw mut gr, 0, 0);
            }
            grid_reader_cursor_next_word_end(&raw mut gr, separators);
            grid_reader_cursor_left(&raw mut gr, 1);
        } else {
            grid_reader_cursor_next_word_end(&raw mut gr, separators);
        }
        grid_reader_get_cursor(&raw mut gr, &raw mut px, &raw mut py);
        *ppx = px;
        *ppy = py;
    }
}

/* Move to the next place where a word ends. */

pub unsafe fn window_copy_cursor_next_word_end(
    wme: *mut window_mode_entry,
    separators: *const u8,
    no_reset: i32,
) {
    unsafe {
        let wp: *mut window_pane = (*wme).wp;
        let data: *mut window_copy_mode_data = (*wme).data.cast();
        let oo: *mut options = (*(*wp).window).options;
        let back_s: *mut screen = (*data).backing;
        let mut gr: grid_reader = zeroed();

        let mut px = (*data).cx;
        let hsize = screen_hsize(back_s);
        let mut py = hsize + (*data).cy - (*data).oy;
        let oldy = (*data).cy;

        grid_reader_start(&raw mut gr, (*back_s).grid, px, py);
        if modekey::try_from(options_get_number_(oo, c"mode-keys") as i32)
            == Ok(modekey::MODEKEY_VI)
        {
            if grid_reader_in_set(&raw mut gr, WHITESPACE) == 0 {
                grid_reader_cursor_right(&raw mut gr, 0, 0);
            }
            grid_reader_cursor_next_word_end(&raw mut gr, separators);
            grid_reader_cursor_left(&raw mut gr, 1);
        } else {
            grid_reader_cursor_next_word_end(&raw mut gr, separators);
        }
        grid_reader_get_cursor(&raw mut gr, &raw mut px, &raw mut py);
        window_copy_acquire_cursor_down(
            wme,
            hsize,
            screen_size_y(back_s),
            (*data).oy,
            oldy,
            px,
            py,
            no_reset,
        );
    }
}

/* Compute the previous place where a word begins. */

pub unsafe fn window_copy_cursor_previous_word_pos(
    wme: *mut window_mode_entry,
    separators: *const u8,
    ppx: *mut u32,
    ppy: *mut u32,
) {
    unsafe {
        let data: *mut window_copy_mode_data = (*wme).data.cast();
        let back_s: *mut screen = (*data).backing;
        let mut gr: grid_reader = zeroed();

        let mut px = (*data).cx;
        let hsize = screen_hsize(back_s);
        let mut py = hsize + (*data).cy - (*data).oy;

        grid_reader_start(&raw mut gr, (*back_s).grid, px, py);
        grid_reader_cursor_previous_word(
            &raw mut gr,
            separators,
            /* already= */ 0,
            /* stop_at_eol= */ 1,
        );
        grid_reader_get_cursor(&raw mut gr, &raw mut px, &raw mut py);
        *ppx = px;
        *ppy = py;
    }
}

/* Move to the previous place where a word begins. */

pub unsafe fn window_copy_cursor_previous_word(
    wme: *mut window_mode_entry,
    separators: *const u8,
    already: i32,
) {
    unsafe {
        let data: *mut window_copy_mode_data = (*wme).data.cast();
        let w: *mut window = (*(*wme).wp).window;
        let back_s: *mut screen = (*data).backing;
        let mut gr: grid_reader = zeroed();

        let stop_at_eol =
            if modekey::try_from(options_get_number_((*w).options, c"mode-keys") as i32)
                == Ok(modekey::MODEKEY_EMACS)
            {
                1
            } else {
                0
            };

        let mut px = (*data).cx;
        let hsize = screen_hsize(back_s);
        let mut py = hsize + (*data).cy - (*data).oy;
        let oldy = (*data).cy;

        grid_reader_start(&raw mut gr, (*back_s).grid, px, py);
        grid_reader_cursor_previous_word(&raw mut gr, separators, already, stop_at_eol);
        grid_reader_get_cursor(&raw mut gr, &raw mut px, &raw mut py);
        window_copy_acquire_cursor_up(wme, hsize, (*data).oy, oldy, px, py);
    }
}

pub unsafe fn window_copy_cursor_prompt(
    wme: *mut window_mode_entry,
    direction: i32,
    args: *const u8,
) {
    unsafe {
        let data: *mut window_copy_mode_data = (*wme).data.cast();
        let s: *mut screen = (*data).backing;
        let gd: *mut grid = (*s).grid;
        let mut line = (*gd).hsize - (*data).oy + (*data).cy;
        let mut end_line: u32 = 0;
        let mut add: i32 = 0;

        let line_flag = if !args.is_null() && streq_(args, "-o") {
            grid_line_flag::START_OUTPUT
        } else {
            grid_line_flag::START_PROMPT
        };

        if direction == 0 {
            /* up */
            add = -1;
            end_line = 0;
        } else {
            /* down */
            add = 1;
            end_line = (*gd).hsize + (*gd).sy - 1;
        }

        if line == end_line {
            return;
        }
        loop {
            if line == end_line {
                return;
            }
            line += add as u32;

            if (*grid_get_line(gd, line)).flags.intersects(line_flag) {
                break;
            }
        }

        (*data).cx = 0;
        if line > (*gd).hsize {
            (*data).cy = line - (*gd).hsize;
            (*data).oy = 0;
        } else {
            (*data).cy = 0;
            (*data).oy = (*gd).hsize - line;
        }

        window_copy_update_selection(wme, 1, 0);
        window_copy_redraw_screen(wme);
    }
}

pub unsafe fn window_copy_scroll_up(wme: *mut window_mode_entry, mut ny: u32) {
    unsafe {
        let wp: *mut window_pane = (*wme).wp;
        let data: *mut window_copy_mode_data = (*wme).data.cast();
        let s: *mut screen = &raw mut (*data).screen;
        let mut ctx: screen_write_ctx = zeroed();

        if (*data).oy < ny {
            ny = (*data).oy;
        }
        if ny == 0 {
            return;
        }
        (*data).oy -= ny;

        if !(*data).searchmark.is_null() && (*data).timeout == 0 {
            window_copy_search_marks(wme, null_mut(), (*data).searchregex, 1);
        }
        window_copy_update_selection(wme, 0, 0);

        screen_write_start_pane(&raw mut ctx, wp, null_mut());
        screen_write_cursormove(&raw mut ctx, 0, 0, 0);
        screen_write_deleteline(&raw mut ctx, ny, 8);
        window_copy_write_lines(wme, &raw mut ctx, screen_size_y(s) - ny, ny);
        window_copy_write_line(wme, &raw mut ctx, 0);
        if screen_size_y(s) > 1 {
            window_copy_write_line(wme, &raw mut ctx, 1);
        }
        if screen_size_y(s) > 3 {
            window_copy_write_line(wme, &raw mut ctx, screen_size_y(s) - 2);
        }
        if !(*s).sel.is_null() && screen_size_y(s) > ny {
            window_copy_write_line(wme, &raw mut ctx, screen_size_y(s) - ny - 1);
        }
        screen_write_cursormove(&raw mut ctx, (*data).cx as i32, (*data).cy as i32, 0);
        screen_write_stop(&raw mut ctx);
    }
}

pub unsafe fn window_copy_scroll_down(wme: *mut window_mode_entry, mut ny: u32) {
    unsafe {
        let wp: *mut window_pane = (*wme).wp;
        let data: *mut window_copy_mode_data = (*wme).data.cast();
        let s: *mut screen = &raw mut (*data).screen;
        let mut ctx: screen_write_ctx = zeroed();

        if ny > screen_hsize((*data).backing) {
            return;
        }

        if (*data).oy > screen_hsize((*data).backing) - ny {
            ny = screen_hsize((*data).backing) - (*data).oy;
        }
        if ny == 0 {
            return;
        }
        (*data).oy += ny;

        if !(*data).searchmark.is_null() && (*data).timeout == 0 {
            window_copy_search_marks(wme, null_mut(), (*data).searchregex, 1);
        }
        window_copy_update_selection(wme, 0, 0);

        screen_write_start_pane(&raw mut ctx, wp, null_mut());
        screen_write_cursormove(&raw mut ctx, 0, 0, 0);
        screen_write_insertline(&raw mut ctx, ny, 8);
        window_copy_write_lines(wme, &raw mut ctx, 0, ny);
        if !(*s).sel.is_null() && screen_size_y(s) > ny {
            window_copy_write_line(wme, &raw mut ctx, ny);
        } else if ny == 1 {
            window_copy_write_line(wme, &raw mut ctx, 1);
        } /* nuke position */
        screen_write_cursormove(&raw mut ctx, (*data).cx as i32, (*data).cy as i32, 0);
        screen_write_stop(&raw mut ctx);
    }
}

pub unsafe fn window_copy_rectangle_set(wme: *mut window_mode_entry, rectflag: i32) {
    unsafe {
        let data: *mut window_copy_mode_data = (*wme).data.cast();

        (*data).rectflag = rectflag;

        let py = screen_hsize((*data).backing) + (*data).cy - (*data).oy;
        let px = window_copy_find_length(wme, py);
        if (*data).cx > px {
            window_copy_update_cursor(wme, px, (*data).cy);
        }

        window_copy_update_selection(wme, 1, 0);
        window_copy_redraw_screen(wme);
    }
}

pub unsafe fn window_copy_move_mouse(m: *mut mouse_event) {
    unsafe {
        let Some(wp) = cmd_mouse_pane(m, null_mut(), null_mut()) else {
            return;
        };
        let wme = tailq_first(&raw mut (*wp.as_ptr()).modes);
        if wme.is_null() {
            return;
        }
        if (*wme).mode != &WINDOW_COPY_MODE && (*wme).mode != &WINDOW_VIEW_MODE {
            return;
        }

        let mut x = 0;
        let mut y = 0;
        if cmd_mouse_at(wp.as_ptr(), m, &raw mut x, &raw mut y, 0) != 0 {
            return;
        }

        window_copy_update_cursor(wme, x, y);
    }
}

pub unsafe fn window_copy_start_drag(c: *mut client, m: *mut mouse_event) {
    unsafe {
        let mut wp: *mut window_pane;
        let mut wme: *mut window_mode_entry;

        if c.is_null() {
            return;
        }

        let Some(wp) = cmd_mouse_pane(m, null_mut(), null_mut()) else {
            return;
        };
        let wme = tailq_first(&raw mut (*wp.as_ptr()).modes);
        if wme.is_null() {
            return;
        }
        if (*wme).mode != &WINDOW_COPY_MODE && (*wme).mode != &WINDOW_VIEW_MODE {
            return;
        }

        let mut x = 0;
        let mut y = 0;
        if cmd_mouse_at(wp.as_ptr(), m, &raw mut x, &raw mut y, 1) != 0 {
            return;
        }

        (*c).tty.mouse_drag_update = Some(window_copy_drag_update);
        (*c).tty.mouse_drag_release = Some(window_copy_drag_release);

        let data: *mut window_copy_mode_data = (*wme).data.cast();
        let yg = screen_hsize((*data).backing) + y - (*data).oy;
        if x < (*data).selrx || x > (*data).endselrx || yg != (*data).selry {
            (*data).selflag = selflag::SEL_CHAR;
        }
        match (*data).selflag {
            selflag::SEL_WORD => {
                if !(*data).separators.is_null() {
                    window_copy_update_cursor(wme, x, y);
                    window_copy_cursor_previous_word_pos(
                        wme,
                        (*data).separators,
                        &raw mut x,
                        &raw mut y,
                    );
                    y -= screen_hsize((*data).backing) - (*data).oy;
                }
                window_copy_update_cursor(wme, x, y);
            }
            selflag::SEL_LINE => window_copy_update_cursor(wme, 0, y),
            selflag::SEL_CHAR => {
                window_copy_update_cursor(wme, x, y);
                window_copy_start_selection(wme);
            }
        }

        window_copy_redraw_screen(wme);
        window_copy_drag_update(c, m);
    }
}

pub unsafe fn window_copy_drag_update(c: *mut client, m: *mut mouse_event) {
    unsafe {
        let mut wp: *mut window_pane;
        let mut x: u32 = 0;
        let mut y: u32 = 0;

        let mut tv: libc::timeval = libc::timeval {
            tv_sec: 0,
            tv_usec: WINDOW_COPY_DRAG_REPEAT_TIME,
        };

        if c.is_null() {
            return;
        }

        let Some(wp) = cmd_mouse_pane(m, null_mut(), null_mut()) else {
            return;
        };
        let wme: *mut window_mode_entry = tailq_first(&raw mut (*wp.as_ptr()).modes);
        if wme.is_null() {
            return;
        }
        if (*wme).mode != &WINDOW_COPY_MODE && (*wme).mode != &WINDOW_VIEW_MODE {
            return;
        }

        let data: *mut window_copy_mode_data = (*wme).data.cast();
        evtimer_del(&raw mut (*data).dragtimer);

        if cmd_mouse_at(wp.as_ptr(), m, &raw mut x, &raw mut y, 0) != 0 {
            return;
        }
        let old_cx = (*data).cx;
        let old_cy = (*data).cy;

        window_copy_update_cursor(wme, x, y);
        if window_copy_update_selection(wme, 1, 0) != 0 {
            window_copy_redraw_selection(wme, old_cy);
        }
        if old_cy != (*data).cy || old_cx == (*data).cx {
            if y == 0 {
                evtimer_add(&raw mut (*data).dragtimer, &raw mut tv);
                window_copy_cursor_up(wme, 1);
            } else if y == screen_size_y(&(*data).screen) - 1 {
                evtimer_add(&raw mut (*data).dragtimer, &raw mut tv);
                window_copy_cursor_down(wme, 1);
            }
        }
    }
}

pub unsafe fn window_copy_drag_release(c: *mut client, m: *mut mouse_event) {
    unsafe {
        if c.is_null() {
            return;
        }

        let Some(wp) = cmd_mouse_pane(m, null_mut(), null_mut()) else {
            return;
        };
        let wme = tailq_first(&raw mut (*wp.as_ptr()).modes);
        if wme.is_null() {
            return;
        }
        if (*wme).mode != &raw const WINDOW_COPY_MODE && (*wme).mode != &raw const WINDOW_VIEW_MODE
        {
            return;
        }

        let data: *mut window_copy_mode_data = (*wme).data.cast();
        evtimer_del(&raw mut (*data).dragtimer);
    }
}

pub unsafe fn window_copy_jump_to_mark(wme: *mut window_mode_entry) {
    unsafe {
        let data: *mut window_copy_mode_data = (*wme).data.cast();

        let tmx = (*data).cx;
        let tmy = screen_hsize((*data).backing) + (*data).cy - (*data).oy;
        (*data).cx = (*data).mx;
        if (*data).my < screen_hsize((*data).backing) {
            (*data).cy = 0;
            (*data).oy = screen_hsize((*data).backing) - (*data).my;
        } else {
            (*data).cy = (*data).my - screen_hsize((*data).backing);
            (*data).oy = 0;
        }
        (*data).mx = tmx;
        (*data).my = tmy;
        (*data).showmark = 1;
        window_copy_update_selection(wme, 0, 0);
        window_copy_redraw_screen(wme);
    }
}

/* Scroll up if the cursor went off the visible screen. */

pub unsafe fn window_copy_acquire_cursor_up(
    wme: *mut window_mode_entry,
    hsize: u32,
    oy: u32,
    oldy: u32,
    px: u32,
    py: u32,
) {
    unsafe {
        let yy = hsize - oy;
        let mut ny;
        let nd;
        let cy;
        if py < yy {
            ny = yy - py;
            cy = 0;
            nd = 1;
        } else {
            ny = 0;
            cy = py - yy;
            nd = oldy - cy + 1;
        }
        while ny > 0 {
            window_copy_cursor_up(wme, 1);
            ny -= 1;
        }
        window_copy_update_cursor(wme, px, cy);
        if window_copy_update_selection(wme, 1, 0) != 0 {
            window_copy_redraw_lines(wme, cy, nd);
        }
    }
}

/* Scroll down if the cursor went off the visible screen. */

pub unsafe fn window_copy_acquire_cursor_down(
    wme: *mut window_mode_entry,
    hsize: u32,
    sy: u32,
    oy: u32,
    mut oldy: u32,
    px: u32,
    py: u32,
    no_reset: i32,
) {
    unsafe {
        let cy = py - hsize + oy;
        let yy = sy - 1;
        let mut ny;
        let nd;
        if cy > yy {
            ny = cy - yy;
            oldy = yy;
            nd = 1;
        } else {
            ny = 0;
            nd = cy - oldy + 1;
        }
        while ny > 0 {
            window_copy_cursor_down(wme, 1);
            ny -= 1;
        }
        if cy > yy {
            window_copy_update_cursor(wme, px, yy);
        } else {
            window_copy_update_cursor(wme, px, cy);
        }
        if window_copy_update_selection(wme, 1, no_reset) != 0 {
            window_copy_redraw_lines(wme, oldy, nd);
        }
    }
}

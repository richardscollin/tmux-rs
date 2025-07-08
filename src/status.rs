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
    fgetln,
    queue::{tailq_init, tailq_remove},
    tree::rb_foreach,
};
use crate::libc::strncmp;

#[repr(C)]
struct status_prompt_menu {
    c: *mut client,
    start: u32,
    size: u32,
    list: *mut *mut u8,
    flag: u8,
}

pub static PROMPT_TYPE_STRINGS: [SyncCharPtr; 4] = [
    SyncCharPtr::new(c"command"),
    SyncCharPtr::new(c"search"),
    SyncCharPtr::new(c"target"),
    SyncCharPtr::new(c"window-target"),
];

/// Status prompt history.
pub static mut STATUS_PROMPT_HLIST: [*mut *mut u8; PROMPT_NTYPES as usize] =
    [null_mut(); PROMPT_NTYPES as usize];

pub static mut STATUS_PROMPT_HSIZE: [u32; PROMPT_NTYPES as usize] = [0; PROMPT_NTYPES as usize];

/// Find the history file to load/save from/to.
unsafe fn status_prompt_find_history_file() -> *mut u8 {
    unsafe {
        let history_file = options_get_string_(GLOBAL_OPTIONS, c"history-file");
        if *history_file == b'\0' {
            return null_mut();
        }
        if *history_file == b'/' {
            return xstrdup(history_file).as_ptr();
        }

        if *history_file != b'~' || *history_file.add(1) != b'/' {
            return null_mut();
        }

        let Some(home) = NonNull::new(find_home()) else {
            return null_mut();
        };

        format_nul!("{}{}", _s(home.as_ptr()), _s(history_file.add(1)))
    }
}

/// Add loaded history item to the appropriate list.
unsafe fn status_prompt_add_typed_history(mut line: *mut u8) {
    unsafe {
        let mut type_ = prompt_type::PROMPT_TYPE_INVALID;

        let typestr: *mut u8 = strsep(&raw mut line, c!(":"));
        if !line.is_null() {
            type_ = status_prompt_type(typestr);
        }
        if type_ == prompt_type::PROMPT_TYPE_INVALID {
            /*
             * Invalid types are not expected, but this provides backward
             * compatibility with old history files.
             */
            if !line.is_null() {
                line = line.sub(1);
                *(line) = b':';
            }
            status_prompt_add_history(typestr, prompt_type::PROMPT_TYPE_COMMAND as u32);
        } else {
            status_prompt_add_history(line, type_ as u32);
        }
    }
}

/// Load status prompt history from file.
pub unsafe fn status_prompt_load_history() {
    unsafe {
        let mut length: usize = 0;

        let Some(history_file) = NonNull::new(status_prompt_find_history_file()) else {
            return;
        };
        let history_file = history_file.as_ptr();

        log_debug!("loading history from {}", _s(history_file));

        // std::fs::OpenOptions::read(true).open()

        let Some(f) = NonNull::new(libc::fopen(history_file, c!("r"))) else {
            log_debug!("{}: {}", _s(history_file), _s(strerror(errno!())));
            free_(history_file);
            return;
        };
        let f = f.as_ptr();
        free_(history_file);

        loop {
            let line: *mut u8 = fgetln(f, &raw mut length).cast();
            if line.is_null() {
                break;
            }

            if length > 0 {
                if *line.add(length - 1) == b'\n' {
                    *line.add(length - 1) = b'\0';
                    status_prompt_add_typed_history(line);
                } else {
                    let tmp: *mut u8 = xmalloc(length + 1).as_ptr().cast();
                    libc::memcpy(tmp.cast(), line.cast(), length);
                    *tmp.add(length) = b'\0';
                    status_prompt_add_typed_history(tmp.cast());
                    free_(tmp);
                }
            }
        }
        libc::fclose(f);
    }
}

/// Save status prompt history to file.
pub unsafe fn status_prompt_save_history() {
    unsafe {
        let Some(history_file) = NonNull::new(status_prompt_find_history_file()) else {
            return;
        };
        let history_file = history_file.as_ptr();

        log_debug!("saving history to {}", _s(history_file));

        let Some(f) = NonNull::new(fopen(history_file, c!("w"))) else {
            log_debug!("{}: {}", _s(history_file), _s(strerror(errno!())));
            free_(history_file);
            return;
        };
        let f = f.as_ptr();
        free_(history_file);

        for type_ in 0..PROMPT_NTYPES {
            for i in 0..STATUS_PROMPT_HSIZE[type_ as usize] {
                libc::fputs(PROMPT_TYPE_STRINGS[type_ as usize].as_ptr(), f);
                libc::fputc(b':' as i32, f);
                libc::fputs(*STATUS_PROMPT_HLIST[type_ as usize].add(i as usize), f);
                libc::fputc(b'\n' as i32, f);
            }
        }
        fclose(f);
    }
}

/// Status timer callback.
unsafe extern "C-unwind" fn status_timer_callback(_fd: i32, _events: i16, arg: *mut c_void) {
    unsafe {
        let c: *mut client = arg.cast();
        let s: *mut session = (*c).session;

        evtimer_del(&raw mut (*c).status.timer);

        if s.is_null() {
            return;
        }

        if (*c).message_string.is_null() && (*c).prompt_string.is_null() {
            (*c).flags |= client_flag::REDRAWSTATUS;
        }

        let mut tv: timeval = zeroed();
        timerclear(&raw mut tv);
        tv.tv_sec = options_get_number_((*s).options, c"status-interval");

        if tv.tv_sec != 0 {
            evtimer_add(&raw mut (*c).status.timer, &raw const tv);
        }
        log_debug!("client {:p}, status interval {}", c, tv.tv_sec);
    }
}

/// Start status timer for client.
pub unsafe fn status_timer_start(c: *mut client) {
    unsafe {
        let s: *mut session = (*c).session;

        if event_initialized(&raw mut (*c).status.timer) != 0 {
            evtimer_del(&raw mut (*c).status.timer);
        } else {
            evtimer_set(
                &raw mut (*c).status.timer,
                Some(status_timer_callback),
                c.cast(),
            );
        }

        if !s.is_null() && options_get_number_((*s).options, c"status") != 0 {
            status_timer_callback(-1, 0, c.cast());
        }
    }
}

/// Start status timer for all clients.
pub unsafe fn status_timer_start_all() {
    unsafe {
        for c in tailq_foreach(&raw mut CLIENTS).map(NonNull::as_ptr) {
            status_timer_start(c);
        }
    }
}

/// Update status cache.
pub unsafe fn status_update_cache(s: *mut session) {
    unsafe {
        (*s).statuslines = options_get_number_((*s).options, c"status") as u32;
        if (*s).statuslines == 0 {
            (*s).statusat = -1;
        } else if options_get_number_((*s).options, c"status-position") == 0 {
            (*s).statusat = 0;
        } else {
            (*s).statusat = 1;
        }
    }
}

/// Get screen line of status line. -1 means off.
pub unsafe fn status_at_line(c: *mut client) -> i32 {
    unsafe {
        let s: *mut session = (*c).session;

        if (*c)
            .flags
            .intersects(client_flag::STATUSOFF | client_flag::CONTROL)
        {
            return -1;
        }
        if (*s).statusat != 1 {
            return (*s).statusat;
        }
        (*c).tty.sy as i32 - status_line_size(c) as i32
    }
}

/// Get size of status line for client's session. 0 means off.
pub unsafe fn status_line_size(c: *mut client) -> u32 {
    unsafe {
        let s: *mut session = (*c).session;

        if (*c)
            .flags
            .intersects(client_flag::STATUSOFF | client_flag::CONTROL)
        {
            return 0;
        }
        if s.is_null() {
            return options_get_number_(GLOBAL_S_OPTIONS, c"status") as u32;
        }
        (*s).statuslines
    }
}

/// Get the prompt line number for client's session. 1 means at the bottom.
unsafe fn status_prompt_line_at(c: *mut client) -> u32 {
    unsafe {
        let s = (*c).session;

        if (*c)
            .flags
            .intersects(client_flag::STATUSOFF | client_flag::CONTROL)
        {
            return 1;
        }
        options_get_number_((*s).options, c"message-line") as u32
    }
}

/// Get window at window list position.
pub unsafe fn status_get_range(c: *mut client, x: u32, y: u32) -> *mut style_range {
    unsafe {
        let sl = &raw mut (*c).status;

        if y >= (*sl).entries.len() as u32 {
            return null_mut();
        }
        for sr in tailq_foreach(&raw mut (*sl).entries[y as usize].ranges).map(NonNull::as_ptr) {
            if x >= (*sr).start && x < (*sr).end {
                return sr;
            }
        }
        null_mut()
    }
}

/// Free all ranges.
unsafe fn status_free_ranges(srs: *mut style_ranges) {
    unsafe {
        for sr in tailq_foreach(srs).map(NonNull::as_ptr) {
            tailq_remove(srs, sr);
            free_(sr);
        }
    }
}

/// Save old status line.
unsafe fn status_push_screen(c: *mut client) {
    unsafe {
        let sl = &raw mut (*c).status;

        if (*sl).active == &raw mut (*sl).screen {
            (*sl).active = xmalloc_::<screen>().as_ptr();
            screen_init((*sl).active, (*c).tty.sx, status_line_size(c), 0);
        }
        (*sl).references += 1;
    }
}

/// Restore old status line.
unsafe fn status_pop_screen(c: *mut client) {
    unsafe {
        let sl = &raw mut (*c).status;

        (*sl).references -= 1;
        if (*sl).references == 0 {
            screen_free((*sl).active);
            free_((*sl).active);
            (*sl).active = &raw mut (*sl).screen;
        }
    }
}

/// Initialize status line.
pub unsafe fn status_init(c: *mut client) {
    unsafe {
        let sl = &raw mut (*c).status;

        for i in 0..(*sl).entries.len() {
            tailq_init(&raw mut (*sl).entries[i].ranges);
        }

        screen_init(&raw mut (*sl).screen, (*c).tty.sx, 1, 0);
        (*sl).active = &raw mut (*sl).screen;
    }
}

/// Free status line.
pub unsafe fn status_free(c: *mut client) {
    unsafe {
        let sl = &raw mut (*c).status;

        for i in 0..(*sl).entries.len() {
            status_free_ranges(&raw mut (*sl).entries[i].ranges);
            free_((*sl).entries[i].expanded);
        }

        if event_initialized(&raw mut (*sl).timer) != 0 {
            evtimer_del(&raw mut (*sl).timer);
        }

        if (*sl).active != &raw mut (*sl).screen {
            screen_free((*sl).active);
            free_((*sl).active);
        }
        screen_free(&raw mut (*sl).screen);
    }
}

/// Draw status line for client.
pub unsafe fn status_redraw(c: *mut client) -> i32 {
    unsafe {
        let sl = &raw mut (*c).status;
        // status_line_entry *sle;
        let s = (*c).session;
        let mut ctx: screen_write_ctx = zeroed();
        let mut gc: grid_cell = zeroed();

        //u_int lines, i, n;

        let width = (*c).tty.sx;

        let mut force = false;
        let mut changed = false;

        // int flags, force = 0, changed = 0, fg, bg;

        // struct options_entry *o;
        // union options_value *ov;
        // struct format_tree *ft;
        // char *expanded;

        log_debug!("status_redraw enter");

        // Shouldn't get here if not the active screen.
        if (*sl).active != &raw mut (*sl).screen {
            fatalx("not the active screen");
        }

        // No status line?
        let lines = status_line_size(c);
        if (*c).tty.sy == 0 || lines == 0 {
            return 1;
        }

        // Create format tree.
        let mut flags = format_flags::FORMAT_STATUS;
        if (*c).flags.intersects(client_flag::STATUSFORCE) {
            flags |= format_flags::FORMAT_FORCE;
        }
        let ft = format_create(c, null_mut(), FORMAT_NONE, flags);
        format_defaults(ft, c, None, None, None);

        /* Set up default colour. */
        style_apply(&raw mut gc, (*s).options, c!("status-style"), ft);
        let fg = options_get_number_((*s).options, c"status-fg") as i32;
        if !COLOUR_DEFAULT(fg) {
            gc.fg = fg;
        }
        let bg = options_get_number_((*s).options, c"status-bg") as i32;
        if !COLOUR_DEFAULT(bg) {
            gc.bg = bg;
        }
        if grid_cells_equal(&raw const gc, &raw const (*sl).style) == 0 {
            force = true;
            memcpy__(&raw mut (*sl).style, &raw mut gc);
        }

        // Resize the target screen.
        if screen_size_x(&raw mut (*sl).screen) != width
            || screen_size_y(&raw mut (*sl).screen) != lines
        {
            screen_resize(&raw mut (*sl).screen, width, lines, 0);
            changed = true;
            force = true
        }
        screen_write_start(&raw mut ctx, &raw mut (*sl).screen);

        // Write the status lines.
        let o = options_get((*s).options, c!("status-format"));
        if o.is_null() {
            for n in 0..(width * lines) {
                screen_write_putc(&raw mut ctx, &raw mut gc, b' ');
            }
        } else {
            for i in 0..lines {
                screen_write_cursormove(&raw mut ctx, 0, i as i32, 0);

                let ov = options_array_get(o, i);
                if ov.is_null() {
                    for n in 0..width {
                        screen_write_putc(&raw mut ctx, &raw mut gc, b' ');
                    }
                    continue;
                }
                let sle = &raw mut (*sl).entries[i as usize];

                let expanded = format_expand_time(ft, (*ov).string);
                if !force
                    && !(*sle).expanded.is_null()
                    && libc::strcmp(expanded, (*sle).expanded) == 0
                {
                    free_(expanded);
                    continue;
                }
                changed = true;

                for n in 0..width {
                    screen_write_putc(&raw mut ctx, &raw mut gc, b' ');
                }
                screen_write_cursormove(&raw mut ctx, 0, i as i32, 0);

                status_free_ranges(&raw mut (*sle).ranges);
                format_draw(
                    &raw mut ctx,
                    &raw mut gc,
                    width,
                    expanded,
                    &raw mut (*sle).ranges,
                    0,
                );

                free_((*sle).expanded);
                (*sle).expanded = expanded;
            }
        }
        screen_write_stop(&raw mut ctx);

        /* Free the format tree. */
        format_free(ft);

        /* Return if the status line has changed. */
        // log_debug("%s exit: force=%d, changed=%d", __func__, force, changed);
        (force || changed) as i32
    }
}

macro_rules! status_message_set {
   ($c:expr, $delay:expr, $ignore_styles:expr, $ignore_keys:expr, $fmt:literal $(, $args:expr)* $(,)?) => {
        crate::status::status_message_set_($c, $delay, $ignore_styles, $ignore_keys, format_args!($fmt $(, $args)*))
    };
}
pub(crate) use status_message_set;
/// Set a status line message.
pub unsafe fn status_message_set_(
    c: *mut client,
    mut delay: i32,
    ignore_styles: i32,
    ignore_keys: i32,
    args: std::fmt::Arguments,
) {
    unsafe {
        // struct timeval tv;
        // va_list ap;
        // char *s;

        let mut tv: timeval = zeroed();
        let mut s = args.to_string();

        // log_debug("%s: %s", __func__, s);

        if c.is_null() {
            server_add_message!("message: {}", s);
            return;
        }

        status_message_clear(c);
        status_push_screen(c);
        s.push('\0');
        let s = s.leak().as_mut_ptr().cast();
        (*c).message_string = s;
        server_add_message!("{} message: {}", _s((*c).name), _s(s));

        /*
         * With delay -1, the display-time option is used; zero means wait for
         * key press; more than zero is the actual delay time in milliseconds.
         */
        if delay == -1 {
            delay = options_get_number_((*(*c).session).options, c"display-time") as i32;
        }
        if delay > 0 {
            tv.tv_sec = (delay / 1000) as libc::time_t;
            tv.tv_usec = (delay as libc::suseconds_t % 1000) * (1000 as libc::suseconds_t);

            if event_initialized(&raw mut (*c).message_timer) != 0 {
                evtimer_del(&raw mut (*c).message_timer);
            }
            evtimer_set(
                &raw mut (*c).message_timer,
                Some(status_message_callback),
                c.cast(),
            );

            evtimer_add(&raw mut (*c).message_timer, &raw mut tv);
        }

        if delay != 0 {
            (*c).message_ignore_keys = ignore_keys;
        }
        (*c).message_ignore_styles = ignore_styles;

        (*c).tty.flags |= tty_flags::TTY_NOCURSOR | tty_flags::TTY_FREEZE;
        (*c).flags |= client_flag::REDRAWSTATUS;
    }
}

/// Clear status line message.
pub unsafe fn status_message_clear(c: *mut client) {
    unsafe {
        if (*c).message_string.is_null() {
            return;
        }

        free_((*c).message_string);
        (*c).message_string = null_mut();

        if (*c).prompt_string.is_null() {
            (*c).tty.flags &= !(tty_flags::TTY_NOCURSOR | tty_flags::TTY_FREEZE);
        }
        (*c).flags |= CLIENT_ALLREDRAWFLAGS; /* was frozen and may have changed */

        status_pop_screen(c);
    }
}

/// Clear status line message after timer expires.
unsafe extern "C-unwind" fn status_message_callback(_fd: i32, _event: i16, data: *mut c_void) {
    unsafe {
        status_message_clear(data.cast());
    }
}

/// Draw client message on status line of present else on last line.
pub unsafe fn status_message_redraw(c: *mut client) -> i32 {
    unsafe {
        let sl = &raw mut (*c).status;
        let mut ctx: screen_write_ctx = zeroed();
        let s = (*c).session;
        let mut old_screen: screen = zeroed();

        // size_t len;
        // u_int lines, offset, messageline;
        let mut gc: grid_cell = zeroed();
        // struct format_tree *ft;

        if (*c).tty.sx == 0 || (*c).tty.sy == 0 {
            return 0;
        }
        memcpy__(&raw mut old_screen, (*sl).active);

        let mut lines = status_line_size(c);
        if lines <= 1 {
            lines = 1;
        }
        screen_init((*sl).active, (*c).tty.sx, lines, 0);

        let mut messageline = status_prompt_line_at(c);
        if messageline > lines - 1 {
            messageline = lines - 1;
        }

        let mut len = screen_write_strlen!("{}", _s((*c).message_string));
        if len > (*c).tty.sx as usize {
            len = (*c).tty.sx as usize;
        }

        let ft = format_create_defaults(null_mut(), c, null_mut(), null_mut(), null_mut());
        style_apply(&raw mut gc, (*s).options, c!("message-style"), ft);
        format_free(ft);

        screen_write_start(&raw mut ctx, (*sl).active);
        screen_write_fast_copy(
            &raw mut ctx,
            &raw mut (*sl).screen,
            0,
            0,
            (*c).tty.sx,
            lines,
        );
        screen_write_cursormove(&raw mut ctx, 0, messageline as i32, 0);
        for offset in 0..(*c).tty.sx {
            screen_write_putc(&raw mut ctx, &raw const gc, b' ');
        }
        screen_write_cursormove(&raw mut ctx, 0, messageline as i32, 0);
        if (*c).message_ignore_styles != 0 {
            screen_write_nputs!(
                &raw mut ctx,
                len as isize,
                &raw mut gc,
                "{}",
                _s((*c).message_string),
            );
        } else {
            format_draw(
                &raw mut ctx,
                &raw const gc,
                (*c).tty.sx,
                (*c).message_string,
                null_mut(),
                0,
            );
        }
        screen_write_stop(&raw mut ctx);

        if grid_compare((*(*sl).active).grid, old_screen.grid) == 0 {
            screen_free(&raw mut old_screen);
            return 0;
        }
        screen_free(&raw mut old_screen);
        1
    }
}

/// Enable status line prompt.
pub unsafe fn status_prompt_set(
    c: *mut client,
    fs: *mut cmd_find_state,
    msg: *const u8,
    mut input: *const u8,
    inputcb: prompt_input_cb,
    freecb: prompt_free_cb,
    data: *mut c_void,
    flags: i32,
    prompt_type: prompt_type,
) {
    unsafe {
        server_client_clear_overlay(c);

        let ft = if !fs.is_null() {
            format_create_from_state(null_mut(), c, fs)
        } else {
            format_create_defaults(null_mut(), c, null_mut(), null_mut(), null_mut())
        };

        if input.is_null() {
            input = c!("");
        }
        let tmp = if flags & PROMPT_NOFORMAT != 0 {
            xstrdup(input).as_ptr()
        } else {
            format_expand_time(ft, input)
        };

        status_message_clear(c);
        status_prompt_clear(c);
        status_push_screen(c);

        (*c).prompt_string = format_expand_time(ft, msg);

        if (flags & PROMPT_INCREMENTAL) != 0 {
            (*c).prompt_last = xstrdup(tmp).as_ptr();
            (*c).prompt_buffer = utf8_fromcstr(c!(""));
        } else {
            (*c).prompt_last = null_mut();
            (*c).prompt_buffer = utf8_fromcstr(tmp);
        }
        (*c).prompt_index = utf8_strlen((*c).prompt_buffer);

        (*c).prompt_inputcb = inputcb;
        (*c).prompt_freecb = freecb;
        (*c).prompt_data = data;

        libc::memset(
            (&raw mut (*c).prompt_hindex).cast(),
            0,
            size_of::<[u32; 4]>(),
        );

        (*c).prompt_flags = flags;
        (*c).prompt_type = prompt_type;
        (*c).prompt_mode = prompt_mode::PROMPT_ENTRY;

        if (!flags & PROMPT_INCREMENTAL) != 0 {
            (*c).tty.flags |= tty_flags::TTY_NOCURSOR | tty_flags::TTY_FREEZE;
        }
        (*c).flags |= client_flag::REDRAWSTATUS;

        if (flags & PROMPT_INCREMENTAL) != 0 {
            (*c).prompt_inputcb.unwrap()(c, NonNull::new((*c).prompt_data).unwrap(), c!("="), 0);
        }

        free_(tmp);
        format_free(ft);
    }
}

/// Remove status line prompt.
pub unsafe fn status_prompt_clear(c: *mut client) {
    unsafe {
        if (*c).prompt_string.is_null() {
            return;
        }

        if let Some(prompt_freecb) = (*c).prompt_freecb
            && let Some(prompt_data) = NonNull::new((*c).prompt_data)
        {
            prompt_freecb(prompt_data);
        }

        free_((*c).prompt_last);
        (*c).prompt_last = null_mut();

        free_((*c).prompt_string);
        (*c).prompt_string = null_mut();

        free_((*c).prompt_buffer);
        (*c).prompt_buffer = null_mut();

        free_((*c).prompt_saved);
        (*c).prompt_saved = null_mut();

        (*c).tty.flags &= !(tty_flags::TTY_NOCURSOR | tty_flags::TTY_FREEZE);
        (*c).flags |= CLIENT_ALLREDRAWFLAGS; /* was frozen and may have changed */

        status_pop_screen(c);
    }
}

/// Update status line prompt with a new prompt string.
pub unsafe fn status_prompt_update(c: *mut client, msg: *const u8, input: *const u8) {
    unsafe {
        let ft = format_create(c, null_mut(), FORMAT_NONE, format_flags::empty());
        format_defaults(ft, c, None, None, None);

        let tmp = format_expand_time(ft, input);

        free_((*c).prompt_string);
        (*c).prompt_string = format_expand_time(ft, msg);

        free_((*c).prompt_buffer);
        (*c).prompt_buffer = utf8_fromcstr(tmp);
        (*c).prompt_index = utf8_strlen((*c).prompt_buffer);

        libc::memset(
            (&raw mut (*c).prompt_hindex).cast(),
            0,
            size_of::<[u32; 4]>(),
        );

        (*c).flags |= client_flag::REDRAWSTATUS;

        free_(tmp);
        format_free(ft);
    }
}

/// Draw client prompt on status line of present else on last line.
pub unsafe fn status_prompt_redraw(c: *mut client) -> i32 {
    unsafe {
        let sl = &raw mut (*c).status;
        let mut ctx: screen_write_ctx = zeroed();

        let s = (*c).session;
        let mut old_screen: screen = zeroed();

        let mut offset: u32 = 0;

        let mut gc: grid_cell = zeroed();
        let mut cursorgc: grid_cell = zeroed();

        'finished: {
            if (*c).tty.sx == 0 || (*c).tty.sy == 0 {
                return 0;
            }
            memcpy__(&raw mut old_screen, (*sl).active);

            let mut lines = status_line_size(c);
            if lines <= 1 {
                lines = 1;
            }
            screen_init((*sl).active, (*c).tty.sx, lines, 0);

            let mut promptline = status_prompt_line_at(c);
            if promptline > lines - 1 {
                promptline = lines - 1;
            }

            let ft = format_create_defaults(null_mut(), c, null_mut(), null_mut(), null_mut());
            if (*c).prompt_mode == prompt_mode::PROMPT_COMMAND {
                style_apply(&raw mut gc, (*s).options, c!("message-command-style"), ft);
            } else {
                style_apply(&raw mut gc, (*s).options, c!("message-style"), ft);
            }
            format_free(ft);

            memcpy__(&raw mut cursorgc, &raw const gc);
            cursorgc.attr ^= grid_attr::GRID_ATTR_REVERSE;

            let mut start = format_width((*c).prompt_string);
            if start > (*c).tty.sx {
                start = (*c).tty.sx;
            }

            screen_write_start(&raw mut ctx, (*sl).active);
            screen_write_fast_copy(
                &raw mut ctx,
                &raw mut (*sl).screen,
                0,
                0,
                (*c).tty.sx,
                lines,
            );
            screen_write_cursormove(&raw mut ctx, 0, promptline as i32, 0);
            for offset in 0..(*c).tty.sx {
                screen_write_putc(&raw mut ctx, &raw const gc, b' ');
            }
            screen_write_cursormove(&raw mut ctx, 0, promptline as i32, 0);
            format_draw(
                &raw mut ctx,
                &raw const gc,
                start,
                (*c).prompt_string,
                null_mut(),
                0,
            );
            screen_write_cursormove(&raw mut ctx, start as i32, promptline as i32, 0);

            let left = (*c).tty.sx - start;
            if left == 0 {
                break 'finished;
            }

            let pcursor = utf8_strwidth((*c).prompt_buffer, (*c).prompt_index as isize);
            let mut pwidth = utf8_strwidth((*c).prompt_buffer, -1);
            if pcursor >= left {
                /*
                 * The cursor would be outside the screen so start drawing
                 * with it on the right.
                 */
                offset = (pcursor - left) + 1;
                pwidth = left;
            } else {
                offset = 0;
            }
            if pwidth > left {
                pwidth = left;
            }
            (*c).prompt_cursor =
                (start as isize + (*c).prompt_index as isize - offset as isize) as i32;

            let mut width = 0;
            let mut i = 0;
            while (*(*c).prompt_buffer.add(i)).size != 0 {
                if width < offset {
                    width += (*(*c).prompt_buffer.add(i)).width as u32;
                    i += 1;
                    continue;
                }
                if width >= offset + pwidth {
                    break;
                }
                width += (*(*c).prompt_buffer.add(i)).width as u32;
                if width > offset + pwidth {
                    break;
                }

                if i != (*c).prompt_index {
                    utf8_copy(&raw mut gc.data, (*c).prompt_buffer.add(i));
                    screen_write_cell(&raw mut ctx, &raw const gc);
                } else {
                    utf8_copy(&raw mut cursorgc.data, (*c).prompt_buffer.add(i));
                    screen_write_cell(&raw mut ctx, &raw const cursorgc);
                }
                i += 1;
            }
            if (*(*sl).active).cx < screen_size_x((*sl).active) && (*c).prompt_index >= i {
                screen_write_putc(&raw mut ctx, &raw const cursorgc, b' ');
            }
        }
        // finished:
        screen_write_stop(&raw mut ctx);

        if grid_compare((*(*sl).active).grid, old_screen.grid) == 0 {
            screen_free(&raw mut old_screen);
            return 0;
        }
        screen_free(&raw mut old_screen);
        1
    }
}

/// Is this a separator?
unsafe fn status_prompt_in_list(ws: *const u8, ud: *const utf8_data) -> i32 {
    unsafe {
        if (*ud).size != 1 || (*ud).width != 1 {
            return 0;
        }
        !libc::strchr(ws, (*ud).data[0] as i32).is_null() as i32
    }
}

/// Is this a space?
unsafe fn status_prompt_space(ud: *const utf8_data) -> i32 {
    unsafe {
        if (*ud).size != 1 || (*ud).width != 1 {
            return 0;
        }
        ((*ud).data[0] == b' ') as i32
    }
}

/// Translate key from vi to emacs. Return 0 to drop key, 1 to process the key
/// as an emacs key; return 2 to append to the buffer.
unsafe fn status_prompt_translate_key(
    c: *mut client,
    key: key_code,
    new_key: *mut key_code,
) -> i32 {
    unsafe {
        if (*c).prompt_mode == prompt_mode::PROMPT_ENTRY {
            match key {
                code::A_CTRL
                | code::C_CTRL
                | code::E_CTRL
                | code::G_CTRL
                | code::H_CTRL
                | code::TAB
                | code::K_CTRL
                | code::N_CTRL
                | code::P_CTRL
                | code::T_CTRL
                | code::W_CTRL
                | code::Y_CTRL
                | code::LF
                | code::CR
                | code::LEFT_CTRL
                | code::RIGHT_CTRL
                | code::KEYC_BSPACE
                | code::KEYC_DC
                | code::KEYC_DOWN
                | code::KEYC_END
                | code::KEYC_HOME
                | code::KEYC_LEFT
                | code::KEYC_RIGHT
                | code::KEYC_UP => {
                    *new_key = key;
                    return 1;
                }
                code::ESC => {
                    (*c).prompt_mode = prompt_mode::PROMPT_COMMAND;
                    (*c).flags |= client_flag::REDRAWSTATUS;
                    return 0;
                }
                _ => (),
            }
            *new_key = key;
            return 2;
        }

        match key {
            code::KEYC_BSPACE => {
                *new_key = keyc::KEYC_LEFT as u64;
                return 1;
            }
            code::A_UPPER | code::I_UPPER | code::C_UPPER | code::S | code::A => {
                (*c).prompt_mode = prompt_mode::PROMPT_ENTRY;
                (*c).flags |= client_flag::REDRAWSTATUS;
            }
            code::S_UPPER => {
                (*c).prompt_mode = prompt_mode::PROMPT_ENTRY;
                (*c).flags |= client_flag::REDRAWSTATUS;
                *new_key = b'u' as u64 | KEYC_CTRL;
                return 1;
            }
            code::I | code::ESC => {
                (*c).prompt_mode = prompt_mode::PROMPT_ENTRY;
                (*c).flags |= client_flag::REDRAWSTATUS;
                return 0;
            }
            _ => (),
        }

        match key {
            code::A_UPPER | code::DOLLAR => {
                *new_key = keyc::KEYC_END as u64;
                return 1;
            }
            code::I_UPPER | code::ZERO | code::CARET => {
                *new_key = keyc::KEYC_HOME as u64;
                return 1;
            }
            code::C_UPPER | code::D_UPPER => {
                *new_key = b'k' as u64 | KEYC_CTRL;
                return 1;
            }
            code::KEYC_BSPACE | code::X_UPPER => {
                *new_key = keyc::KEYC_BSPACE as u64;
                return 1;
            }
            code::B => {
                *new_key = b'b' as u64 | KEYC_META;
                return 1;
            }
            code::B_UPPER => {
                *new_key = b'B' as u64 | KEYC_VI;
                return 1;
            }
            code::D => {
                *new_key = b'u' as u64 | KEYC_CTRL;
                return 1;
            }
            code::E => {
                *new_key = b'e' as u64 | KEYC_VI;
                return 1;
            }
            code::E_UPPER => {
                *new_key = b'E' as u64 | KEYC_VI;
                return 1;
            }
            code::W => {
                *new_key = b'w' as u64 | KEYC_VI;
                return 1;
            }
            code::W_UPPER => {
                *new_key = b'W' as u64 | KEYC_VI;
                return 1;
            }
            code::P => {
                *new_key = b'y' as u64 | KEYC_CTRL;
                return 1;
            }
            code::Q => {
                *new_key = b'c' as u64 | KEYC_CTRL;
                return 1;
            }
            code::S | code::KEYC_DC | code::X => {
                *new_key = keyc::KEYC_DC as u64;
                return 1;
            }
            code::KEYC_DOWN | code::J => {
                *new_key = keyc::KEYC_DOWN as u64;
                return 1;
            }
            code::KEYC_LEFT | code::H => {
                *new_key = keyc::KEYC_LEFT as u64;
                return 1;
            }
            code::A | code::KEYC_RIGHT | code::L => {
                *new_key = keyc::KEYC_RIGHT as u64;
                return 1;
            }
            code::KEYC_UP | code::K => {
                *new_key = keyc::KEYC_UP as u64;
                return 1;
            }
            code::H_CTRL | code::C_CTRL | code::CR | code::LF => {
                return 1;
            }
            _ => (),
        }

        0
    }
}

/// Paste into prompt.
unsafe fn status_prompt_paste(c: *mut client) -> i32 {
    unsafe {
        // struct paste_buffer *pb;
        // const char *bufdata;
        // size_t size, n, bufsize;
        // u_int i;
        // struct utf8_data *ud, *udp;
        // enum utf8_state more;

        let mut bufsize: usize = 0;
        let mut n: usize = 0;

        let mut ud: *mut utf8_data = null_mut();
        let size = utf8_strlen((*c).prompt_buffer);
        if !(*c).prompt_saved.is_null() {
            ud = (*c).prompt_saved;
            n = utf8_strlen((*c).prompt_saved);
        } else {
            let pb = paste_get_top(null_mut());
            if pb.is_null() {
                return 0;
            }
            let bufdata: *const u8 = paste_buffer_data(pb, &raw mut bufsize).cast();
            let mut udp = xreallocarray_::<utf8_data>(null_mut(), bufsize + 1).as_ptr();
            ud = udp;
            let mut i: u32 = 0;
            while i as usize != bufsize {
                let mut more = utf8_open(udp, *bufdata.add(i as usize));
                if more == utf8_state::UTF8_MORE {
                    while {
                        i += 1;
                        i as usize != bufsize && more == utf8_state::UTF8_MORE
                    } {
                        more = utf8_append(udp, *bufdata.add(i as usize));
                    }
                    if more == utf8_state::UTF8_DONE {
                        udp = udp.add(1);
                        continue;
                    }
                    i -= (*udp).have as u32;
                }
                if *bufdata.add(i as usize) <= 31 || *bufdata.add(i as usize) >= 127 {
                    break;
                }
                utf8_set(udp, *bufdata.add(i as usize));
                udp = udp.add(1);
                i += 1;
            }
            (*udp).size = 0;
            n = udp.offset_from_unsigned(ud);
        }
        if n != 0 {
            (*c).prompt_buffer =
                xreallocarray_::<utf8_data>((*c).prompt_buffer, size + n + 1).as_ptr();
            if (*c).prompt_index == size {
                libc::memcpy(
                    (*c).prompt_buffer.add((*c).prompt_index).cast(),
                    ud.cast(),
                    n * size_of::<utf8_data>(),
                );
                (*c).prompt_index += n;
                (*(*c).prompt_buffer.add((*c).prompt_index)).size = 0;
            } else {
                libc::memmove(
                    (*c).prompt_buffer.add((*c).prompt_index + n).cast(),
                    (*c).prompt_buffer.add((*c).prompt_index).cast(),
                    (size + 1 - (*c).prompt_index) * size_of::<utf8_data>(),
                );
                libc::memcpy(
                    (*c).prompt_buffer.add((*c).prompt_index).cast(),
                    ud.cast(),
                    n * size_of::<utf8_data>(),
                );
                (*c).prompt_index += n;
            }
        }
        if ud != (*c).prompt_saved {
            free_(ud);
        }
        1
    }
}

/// Finish completion.
unsafe fn status_prompt_replace_complete(c: *mut client, mut s: *const u8) -> i32 {
    unsafe {
        let mut word: [u8; 64] = [0; 64];
        let mut allocated: *mut u8 = null_mut();

        let mut n: usize = 0;
        let mut off: usize = 0;
        let mut used: usize = 0;

        let mut last: *mut utf8_data = null_mut();
        let mut ud: *mut utf8_data = null_mut();

        /* Work out where the cursor currently is. */
        let idx = (*c).prompt_index.saturating_sub(1);
        let mut size = utf8_strlen((*c).prompt_buffer);

        /* Find the word we are in. */
        let mut first = (*c).prompt_buffer.add(idx);
        while first.addr() > (*c).prompt_buffer.addr() && status_prompt_space(first) == 0 {
            first = first.sub(1);
        }
        while (*first).size != 0 && status_prompt_space(first) != 0 {
            first = first.add(1);
        }
        last = (*c).prompt_buffer.add(idx);
        while (*last).size != 0 && status_prompt_space(last) == 0 {
            last = last.add(1);
        }
        while last > (*c).prompt_buffer && status_prompt_space(last) != 0 {
            last = last.sub(1);
        }
        if (*last).size != 0 {
            last = last.add(1);
        }
        if last < first {
            return 0;
        }
        if s.is_null() {
            used = 0;
            ud = first;
            while ud < last {
                if used + (*ud).size as usize >= word.len() {
                    break;
                }
                libc::memcpy(
                    (&raw mut word as *mut i8).add(used).cast(),
                    (&raw mut (*ud).data).cast(),
                    (*ud).size as usize,
                );
                used += (*ud).size as usize;
                ud = ud.add(1);
            }
            if ud != last {
                return 0;
            }
            word[used] = b'\0';
        }

        /* Try to complete it. */
        if s.is_null() {
            allocated = status_prompt_complete(
                c,
                (&raw const word).cast(),
                first.offset_from_unsigned((*c).prompt_buffer) as u32,
            );
            if allocated.is_null() {
                return 0;
            }
            s = allocated;
        }

        /* Trim out word. */
        n = size - last.offset_from_unsigned((*c).prompt_buffer) + 1; /* with \0 */
        libc::memmove(first.cast(), last.cast(), n * size_of::<utf8_data>());
        size -= last.offset_from_unsigned(first);

        /* Insert the new word. */
        size += strlen(s);
        off = first.offset_from_unsigned((*c).prompt_buffer);
        (*c).prompt_buffer = xreallocarray_::<utf8_data>((*c).prompt_buffer, size + 1).as_ptr();
        first = (*c).prompt_buffer.add(off);
        libc::memmove(
            first.add(strlen(s)).cast(),
            first.cast(),
            n * size_of::<utf8_data>(),
        );
        for idx in 0..strlen(s) {
            utf8_set(first.add(idx), (*s.add(idx)));
        }
        (*c).prompt_index = first.offset_from_unsigned((*c).prompt_buffer) + strlen(s);

        free_(allocated);
        1
    }
}

/// Prompt forward to the next beginning of a word.
unsafe fn status_prompt_forward_word(c: *mut client, size: usize, vi: i32, separators: *const u8) {
    unsafe {
        let mut idx = (*c).prompt_index;

        /* In emacs mode, skip until the first non-whitespace character. */
        if vi == 0 {
            while idx != size && status_prompt_space((*c).prompt_buffer.add(idx)) != 0 {
                idx += 1;
            }
        }

        /* Can't move forward if we're already at the end. */
        if idx == size {
            (*c).prompt_index = idx;
            return;
        }

        /* Determine the current character class (separators or not). */
        let word_is_separators =
            (status_prompt_in_list(separators, (*c).prompt_buffer.add(idx)) != 0
                && status_prompt_space((*c).prompt_buffer.add(idx)) == 0) as i32;

        /* Skip ahead until the first space or opposite character class. */
        loop {
            idx += 1;
            if status_prompt_space((*c).prompt_buffer.add(idx)) != 0 {
                /* In vi mode, go to the start of the next word. */
                if vi != 0 {
                    while idx != size && status_prompt_space((*c).prompt_buffer.add(idx)) != 0 {
                        idx += 1;
                    }
                }
                break;
            }

            if !(idx != size
                && word_is_separators
                    == status_prompt_in_list(separators, (*c).prompt_buffer.add(idx)))
            {
                break;
            }
        }

        (*c).prompt_index = idx;
    }
}

/// Prompt forward to the next end of a word.
unsafe fn status_prompt_end_word(c: *mut client, size: usize, separators: *const u8) {
    unsafe {
        let mut idx = (*c).prompt_index;
        // int word_is_separators;

        /* Can't move forward if we're already at the end. */
        if idx == size {
            return;
        }

        /* Find the next word. */
        loop {
            idx += 1;
            if idx == size {
                (*c).prompt_index = idx;
                return;
            }
            if status_prompt_space((*c).prompt_buffer.add(idx)) == 0 {
                break;
            }
        }

        /* Determine the character class (separators or not). */
        let word_is_separators = status_prompt_in_list(separators, (*c).prompt_buffer.add(idx));

        /* Skip ahead until the next space or opposite character class. */
        loop {
            idx += 1;
            if idx == size {
                break;
            }
            if !(status_prompt_space((*c).prompt_buffer.add(idx)) == 0
                && word_is_separators
                    == status_prompt_in_list(separators, (*c).prompt_buffer.add(idx)))
            {
                break;
            }
        }

        /* Back up to the previous character to stop at the end of the word. */
        (*c).prompt_index = idx - 1;
    }
}

/// Prompt backward to the previous beginning of a word.
unsafe fn status_prompt_backward_word(c: *mut client, separators: *const u8) {
    unsafe {
        let mut idx = (*c).prompt_index;

        /* Find non-whitespace. */
        while idx != 0 {
            idx -= 1;
            if status_prompt_space((*c).prompt_buffer.add(idx)) == 0 {
                break;
            }
        }
        let word_is_separators = status_prompt_in_list(separators, (*c).prompt_buffer.add(idx));

        /* Find the character before the beginning of the word. */
        while idx != 0 {
            idx -= 1;
            if status_prompt_space((*c).prompt_buffer.add(idx)) != 0
                || word_is_separators
                    != status_prompt_in_list(separators, (*c).prompt_buffer.add(idx))
            {
                /* Go back to the word. */
                idx += 1;
                break;
            }
        }
        (*c).prompt_index = idx;
    }
}

/// Handle keys in prompt.
pub unsafe fn status_prompt_key(c: *mut client, mut key: key_code) -> i32 {
    unsafe {
        let oo = (*(*c).session).options;
        let mut s = null_mut();
        let mut cp = null_mut();
        let mut prefix = b'=';

        let mut histstr: *const u8 = null();
        let mut separators: *const u8 = null();
        let mut keystring: *const u8 = null();

        let mut idx: usize = 0;

        let mut tmp: utf8_data = zeroed();

        let mut word_is_separators: i32 = 0;

        if (*c).prompt_flags & PROMPT_KEY != 0 {
            keystring = key_string_lookup_key(key, 0);
            (*c).prompt_inputcb.unwrap()(c, NonNull::new((*c).prompt_data).unwrap(), keystring, 1);
            status_prompt_clear(c);
            return 0;
        }

        let size: usize = utf8_strlen((*c).prompt_buffer);

        'changed: {
            'append_key: {
                'process_key: {
                    if (*c).prompt_flags & PROMPT_NUMERIC != 0 {
                        if key >= b'0' as u64 && key <= b'9' as u64 {
                            break 'append_key;
                        }
                        s = utf8_tocstr((*c).prompt_buffer);
                        (*c).prompt_inputcb.unwrap()(
                            c,
                            NonNull::new((*c).prompt_data).unwrap(),
                            s,
                            1,
                        );
                        status_prompt_clear(c);
                        free_(s);
                        return 1;
                    }
                    key &= !KEYC_MASK_FLAGS;

                    let keys = modekey::try_from(options_get_number_(
                        (*(*c).session).options,
                        c"status-keys",
                    ) as i32);
                    if keys == Ok(modekey::MODEKEY_VI) {
                        match status_prompt_translate_key(c, key, &raw mut key) {
                            1 => break 'process_key,
                            2 => break 'append_key,
                            _ => return 0,
                        }
                    }
                } // process_key:

                match key {
                    code::KEYC_LEFT | code::B_CTRL => {
                        if (*c).prompt_index > 0 {
                            (*c).prompt_index -= 1;
                        }
                    }
                    code::KEYC_RIGHT | code::F_CTRL => {
                        if (*c).prompt_index < size {
                            (*c).prompt_index += 1;
                        }
                    }
                    code::KEYC_HOME | code::A_CTRL => {
                        if (*c).prompt_index != 0 {
                            (*c).prompt_index = 0;
                        }
                    }
                    code::KEYC_END | code::E_CTRL => {
                        if (*c).prompt_index != size {
                            (*c).prompt_index = size;
                        }
                    }
                    code::TAB => {
                        if status_prompt_replace_complete(c, null_mut()) != 0 {
                            break 'changed;
                        }
                    }
                    code::KEYC_BSPACE | code::H_CTRL => {
                        if (*c).prompt_index != 0 {
                            if (*c).prompt_index == size {
                                (*c).prompt_index -= 1;
                                (*(*c).prompt_buffer.add((*c).prompt_index)).size = 0;
                            } else {
                                libc::memmove(
                                    (*c).prompt_buffer.add((*c).prompt_index - 1).cast(),
                                    (*c).prompt_buffer.add((*c).prompt_index).cast(),
                                    (size + 1 - (*c).prompt_index) * size_of::<utf8_data>(),
                                );
                                (*c).prompt_index -= 1;
                            }
                            break 'changed;
                        }
                    }
                    code::KEYC_DC | code::D_CTRL => {
                        if (*c).prompt_index != size {
                            libc::memmove(
                                (*c).prompt_buffer.add((*c).prompt_index).cast(),
                                (*c).prompt_buffer.add((*c).prompt_index + 1).cast(),
                                (size + 1 - (*c).prompt_index) * size_of::<utf8_data>(),
                            );
                            break 'changed;
                        }
                    }
                    code::U_CTRL => {
                        (*(*c).prompt_buffer).size = 0;
                        (*c).prompt_index = 0;
                        break 'changed;
                    }

                    code::K_CTRL => {
                        if (*c).prompt_index < size {
                            (*(*c).prompt_buffer.add((*c).prompt_index)).size = 0;
                            break 'changed;
                        }
                    }
                    code::W_CTRL => {
                        separators = options_get_string_(oo, c"word-separators");
                        idx = (*c).prompt_index;

                        /* Find non-whitespace. */
                        while idx != 0 {
                            idx -= 1;
                            if status_prompt_space((*c).prompt_buffer.add(idx)) == 0 {
                                break;
                            }
                        }
                        word_is_separators =
                            status_prompt_in_list(separators, (*c).prompt_buffer.add(idx));

                        /* Find the character before the beginning of the word. */
                        while idx != 0 {
                            idx -= 1;
                            if status_prompt_space((*c).prompt_buffer.add(idx)) != 0
                                || word_is_separators
                                    != status_prompt_in_list(
                                        separators,
                                        (*c).prompt_buffer.add(idx),
                                    )
                            {
                                /* Go back to the word. */
                                idx += 1;
                                break;
                            }
                        }

                        free_((*c).prompt_saved);
                        (*c).prompt_saved =
                            xcalloc_::<utf8_data>(((*c).prompt_index - idx) + 1).as_ptr();
                        libc::memcpy(
                            (*c).prompt_saved.cast(),
                            (*c).prompt_buffer.add(idx).cast(),
                            ((*c).prompt_index - idx) * size_of::<utf8_data>(),
                        );

                        libc::memmove(
                            (*c).prompt_buffer.add(idx).cast(),
                            (*c).prompt_buffer.add((*c).prompt_index).cast(),
                            (size + 1 - (*c).prompt_index) * size_of::<utf8_data>(),
                        );
                        libc::memset(
                            (*c).prompt_buffer
                                .add(size - ((*c).prompt_index - idx))
                                .cast(),
                            b'\0' as i32,
                            ((*c).prompt_index - idx) * size_of::<utf8_data>(),
                        );
                        (*c).prompt_index = idx;

                        break 'changed;
                    }
                    code::RIGHT_CTRL | code::F_META => {
                        separators = options_get_string_(oo, c"word-separators");
                        status_prompt_forward_word(c, size, 0, separators);
                        break 'changed;
                    }
                    code::E_UPPER_VI => {
                        status_prompt_end_word(c, size, c!(""));
                        break 'changed;
                    }
                    code::E_VI => {
                        separators = options_get_string_(oo, c"word-separators");
                        status_prompt_end_word(c, size, separators);
                        break 'changed;
                    }
                    code::W_UPPER_VI => {
                        status_prompt_forward_word(c, size, 1, c!(""));
                        break 'changed;
                    }
                    code::W_VI => {
                        separators = options_get_string_(oo, c"word-separators");
                        status_prompt_forward_word(c, size, 1, separators);
                        break 'changed;
                    }
                    code::B_VI => {
                        status_prompt_backward_word(c, c!(""));
                        break 'changed;
                    }
                    code::LEFT_CTRL | code::B_META => {
                        separators = options_get_string_(oo, c"word-separators");
                        status_prompt_backward_word(c, separators);
                        break 'changed;
                    }
                    code::KEYC_UP | code::P_CTRL => {
                        histstr = status_prompt_up_history(
                            (&raw mut (*c).prompt_hindex).cast(),
                            (*c).prompt_type as u32,
                        );
                        if !histstr.is_null() {
                            free_((*c).prompt_buffer);
                            (*c).prompt_buffer = utf8_fromcstr(histstr);
                            (*c).prompt_index = utf8_strlen((*c).prompt_buffer);
                            break 'changed;
                        }
                    }
                    code::KEYC_DOWN | code::N_CTRL => {
                        histstr = status_prompt_down_history(
                            (&raw mut (*c).prompt_hindex).cast(),
                            (*c).prompt_type as u32,
                        );
                        if !histstr.is_null() {
                            free_((*c).prompt_buffer);
                            (*c).prompt_buffer = utf8_fromcstr(histstr);
                            (*c).prompt_index = utf8_strlen((*c).prompt_buffer);
                            break 'changed;
                        }
                    }
                    code::Y_CTRL => {
                        if status_prompt_paste(c) != 0 {
                            break 'changed;
                        }
                    }
                    code::T_CTRL => {
                        idx = (*c).prompt_index;
                        if idx < size {
                            idx += 1;
                        }
                        if idx >= 2 {
                            utf8_copy(&raw mut tmp, (*c).prompt_buffer.add(idx - 2));
                            utf8_copy(
                                (*c).prompt_buffer.add(idx - 2),
                                (*c).prompt_buffer.add(idx - 1),
                            );
                            utf8_copy((*c).prompt_buffer.add(idx - 1), &raw const tmp);
                            (*c).prompt_index = idx;
                            break 'changed;
                        }
                    }
                    code::CR | code::LF => {
                        s = utf8_tocstr((*c).prompt_buffer);
                        if *s != b'\0' {
                            status_prompt_add_history(s, (*c).prompt_type as u32);
                        }
                        if (*c).prompt_inputcb.unwrap()(
                            c,
                            NonNull::new((*c).prompt_data).unwrap(),
                            s,
                            1,
                        ) == 0
                        {
                            status_prompt_clear(c);
                        }
                        free_(s);
                    }
                    code::ESC | code::C_CTRL | code::G_CTRL => {
                        if (*c).prompt_inputcb.unwrap()(
                            c,
                            NonNull::new((*c).prompt_data).unwrap(),
                            null_mut(),
                            1,
                        ) == 0
                        {
                            status_prompt_clear(c);
                        }
                    }
                    code::R_CTRL => {
                        if (!(*c).prompt_flags & PROMPT_INCREMENTAL) == 0 {
                            if (*(*c).prompt_buffer).size == 0 {
                                prefix = b'=';
                                free_((*c).prompt_buffer);
                                (*c).prompt_buffer = utf8_fromcstr((*c).prompt_last);
                                (*c).prompt_index = utf8_strlen((*c).prompt_buffer);
                            } else {
                                prefix = b'-';
                            }
                            break 'changed;
                        }
                    }
                    code::S_CTRL => {
                        if !(*c).prompt_flags & PROMPT_INCREMENTAL == 0 {
                            if (*(*c).prompt_buffer).size == 0 {
                                prefix = b'=';
                                free_((*c).prompt_buffer);
                                (*c).prompt_buffer = utf8_fromcstr((*c).prompt_last);
                                (*c).prompt_index = utf8_strlen((*c).prompt_buffer);
                            } else {
                                prefix = b'+';
                            }
                            break 'changed;
                        }
                    }
                    _ => break 'append_key,
                }

                (*c).flags |= client_flag::REDRAWSTATUS;
                return 0;
            } // append_key:
            if key <= 0x7f {
                utf8_set(&raw mut tmp, key as u8);
            } else if KEYC_IS_UNICODE(key) {
                tmp = utf8_to_data(key as u32);
            } else {
                return 0;
            }

            (*c).prompt_buffer = xreallocarray_((*c).prompt_buffer, size + 2).as_ptr();

            if (*c).prompt_index == size {
                utf8_copy((*c).prompt_buffer.add((*c).prompt_index), &raw const tmp);
                (*c).prompt_index += 1;
                (*(*c).prompt_buffer.add((*c).prompt_index)).size = 0;
            } else {
                libc::memmove(
                    (*c).prompt_buffer.add((*c).prompt_index + 1).cast(),
                    (*c).prompt_buffer.add((*c).prompt_index).cast(),
                    (size + 1 - (*c).prompt_index) * size_of::<utf8_data>(),
                );
                utf8_copy((*c).prompt_buffer.add((*c).prompt_index), &raw const tmp);
                (*c).prompt_index += 1;
            }

            if (*c).prompt_flags & PROMPT_SINGLE != 0 {
                if utf8_strlen((*c).prompt_buffer) != 1 {
                    status_prompt_clear(c);
                } else {
                    s = utf8_tocstr((*c).prompt_buffer);
                    if (*c).prompt_inputcb.unwrap()(
                        c,
                        NonNull::new((*c).prompt_data).unwrap(),
                        s,
                        1,
                    ) == 0
                    {
                        status_prompt_clear(c);
                    }
                    free_(s);
                }
            }
        } // changed:
        (*c).flags |= client_flag::REDRAWSTATUS;
        if (*c).prompt_flags & PROMPT_INCREMENTAL != 0 {
            s = utf8_tocstr((*c).prompt_buffer);
            cp = format_nul!("{}{}", prefix as char, _s(s));
            (*c).prompt_inputcb.unwrap()(c, NonNull::new((*c).prompt_data).unwrap(), cp, 0);
            free_(cp);
            free_(s);
        }
        0
    }
}

/// Get previous line from the history.
unsafe fn status_prompt_up_history(idx: *mut u32, type_: u32) -> *mut u8 {
    unsafe {
        /*
         * History runs from 0 to size - 1. Index is from 0 to size. Zero is
         * empty.
         */

        if STATUS_PROMPT_HSIZE[type_ as usize] == 0
            || *idx.add(type_ as usize) == STATUS_PROMPT_HSIZE[type_ as usize]
        {
            return null_mut();
        }
        *idx.add(type_ as usize) += 1;
        *STATUS_PROMPT_HLIST[type_ as usize]
            .add((STATUS_PROMPT_HSIZE[type_ as usize] - *idx.add(type_ as usize)) as usize)
    }
}

/// Get next line from the history.
unsafe fn status_prompt_down_history(idx: *mut u32, type_: u32) -> *const u8 {
    unsafe {
        if STATUS_PROMPT_HSIZE[type_ as usize] == 0 || *idx.add(type_ as usize) == 0 {
            return c!("");
        }
        *idx.add(type_ as usize) -= 1;
        if *idx.add(type_ as usize) == 0 {
            return c!("");
        }

        *STATUS_PROMPT_HLIST[type_ as usize]
            .add((STATUS_PROMPT_HSIZE[type_ as usize] - *idx.add(type_ as usize)) as usize)
    }
}

/// Add line to the history.
unsafe fn status_prompt_add_history(line: *const u8, type_: u32) {
    unsafe {
        let mut new: u32 = 1;
        let mut newsize: u32 = 0;
        let mut freecount: u32 = 0;
        let mut movesize: usize = 0;

        let oldsize = STATUS_PROMPT_HSIZE[type_ as usize];
        if oldsize > 0
            && libc::strcmp(
                *STATUS_PROMPT_HLIST[type_ as usize].add(oldsize as usize - 1),
                line,
            ) == 0
        {
            new = 0;
        }

        let hlimit = options_get_number_(GLOBAL_OPTIONS, c"prompt-history-limit") as u32;
        if hlimit > oldsize {
            if new == 0 {
                return;
            }
            newsize = oldsize + new;
        } else {
            newsize = hlimit;
            freecount = oldsize + new - newsize;
            if freecount > oldsize {
                freecount = oldsize;
            }
            if freecount == 0 {
                return;
            }
            for i in 0..freecount {
                free_(*STATUS_PROMPT_HLIST[type_ as usize].add(i as usize));
            }
            movesize = (oldsize as isize - freecount as isize) as usize * size_of::<*mut u8>();
            if movesize > 0 {
                libc::memmove(
                    STATUS_PROMPT_HLIST[type_ as usize].cast(),
                    STATUS_PROMPT_HLIST[type_ as usize]
                        .add(freecount as usize)
                        .cast(),
                    movesize,
                );
            }
        }

        if newsize == 0 {
            free_(STATUS_PROMPT_HLIST[type_ as usize]);
            STATUS_PROMPT_HLIST[type_ as usize] = null_mut();
        } else if newsize != oldsize {
            STATUS_PROMPT_HLIST[type_ as usize] =
                xreallocarray_(STATUS_PROMPT_HLIST[type_ as usize], newsize as usize).as_ptr();
        }

        if new == 1 && newsize > 0 {
            *STATUS_PROMPT_HLIST[type_ as usize].add(newsize as usize - 1) = xstrdup(line).as_ptr();
        }
        STATUS_PROMPT_HSIZE[type_ as usize] = newsize;
    }
}

/// Add to completion list.
unsafe fn status_prompt_add_list(list: *mut *mut *mut u8, size: *mut u32, s: *const u8) {
    unsafe {
        for i in 0..*size {
            if libc::strcmp(*(*list).add(i as usize), s) == 0 {
                return;
            }
        }
        *list = xreallocarray_(*list, *size as usize + 1).as_ptr();
        *(*list).add(*size as usize) = xstrdup(s).as_ptr();
        (*size) += 1
    }
}

/// Build completion list.
unsafe fn status_prompt_complete_list(size: *mut u32, s: *const u8, at_start: i32) -> *mut *mut u8 {
    unsafe {
        let mut tmp: *mut u8 = null_mut();
        let mut list: *mut *mut u8 = null_mut();
        let slen = strlen(s);

        let layouts: [*const u8; 8] = [
            c!("even-horizontal"),
            c!("even-vertical"),
            c!("main-horizontal"),
            c!("main-horizontal-mirrored"),
            c!("main-vertical"),
            c!("main-vertical-mirrored"),
            c!("tiled"),
            null_mut(),
        ];

        *size = 0;
        for cmdent in CMD_TABLE {
            if strncmp(cmdent.name.as_ptr(), s, slen) == 0 {
                status_prompt_add_list(&raw mut list, size, cmdent.name.as_ptr());
            }
            if !cmdent.alias.is_null() && strncmp(cmdent.alias.as_ptr(), s, slen) == 0 {
                status_prompt_add_list(&raw mut list, size, cmdent.alias.as_ptr());
            }
        }
        let o = options_get_only(GLOBAL_OPTIONS, c!("command-alias"));
        if !o.is_null() {
            let mut a = options_array_first(o);
            while !a.is_null() {
                'next: {
                    let value = (*options_array_item_value(a)).string;

                    let cp = libc::strchr(value, b'=' as i32);
                    if cp.is_null() {
                        break 'next;
                    }
                    let valuelen = cp.offset_from_unsigned(value);
                    if slen > valuelen || strncmp(value, s, slen) != 0 {
                        break 'next;
                    }

                    tmp = format_nul!("{:.*}", valuelen, _s(value));
                    status_prompt_add_list(&raw mut list, size, tmp);
                    free_(tmp);
                } // next:
                a = options_array_next(a);
            }
        }
        if at_start != 0 {
            return list;
        }
        let mut oe = (&raw mut OPTIONS_TABLE) as *mut options_table_entry;
        while !(*oe).name.is_null() {
            if strncmp((*oe).name, s, slen) == 0 {
                status_prompt_add_list(&raw mut list, size, (*oe).name);
            }
            oe = oe.add(1);
        }
        let mut layout = (&raw const layouts) as *const *const u8;
        while !(*layout).is_null() {
            if strncmp(*layout, s, slen) == 0 {
                status_prompt_add_list(&raw mut list, size, *layout);
            }
            layout = layout.add(1);
        }
        list
    }
}

/// Find longest prefix.
unsafe fn status_prompt_complete_prefix(list: *mut *mut u8, size: u32) -> *mut u8 {
    unsafe {
        if list.is_null() || size == 0 {
            return null_mut();
        }
        let out = xstrdup(*list).as_ptr();
        for i in 1..size {
            let mut j = strlen(*list.add(i as usize));
            if j > strlen(out) {
                j = strlen(out);
            }
            while j > 0 {
                if *out.add(j - 1) != *(*list.add(i as usize)).add(j - 1) {
                    *out.add(j - 1) = b'\0';
                }
                j -= 1;
            }
        }
        out
    }
}

/// Complete word menu callback.
unsafe fn status_prompt_menu_callback(
    _menu: *mut menu,
    mut idx: u32,
    key: key_code,
    data: *mut c_void,
) {
    unsafe {
        let spm: *mut status_prompt_menu = data.cast();
        let c = (*spm).c;
        let mut s: *mut u8 = null_mut();

        if key != KEYC_NONE {
            idx += (*spm).start;
            s = if (*spm).flag == b'\0' {
                xstrdup(*(*spm).list.add(idx as usize)).as_ptr()
            } else {
                format_nul!(
                    "-{}{}",
                    (*spm).flag as char,
                    _s(*(*spm).list.add(idx as usize))
                )
            };
            if (*c).prompt_type == prompt_type::PROMPT_TYPE_WINDOW_TARGET {
                free_((*c).prompt_buffer);
                (*c).prompt_buffer = utf8_fromcstr(s);
                (*c).prompt_index = utf8_strlen((*c).prompt_buffer);
                (*c).flags |= client_flag::REDRAWSTATUS;
            } else if status_prompt_replace_complete(c, s) != 0 {
                (*c).flags |= client_flag::REDRAWSTATUS;
            }
            free_(s);
        }

        for i in 0..(*spm).size {
            free_(*(*spm).list.add(i as usize));
        }
        free_((*spm).list);
    }
}

/// Show complete word menu.
unsafe fn status_prompt_complete_list_menu(
    c: *mut client,
    list: *mut *mut u8,
    size: u32,
    mut offset: u32,
    flag: u8,
) -> i32 {
    unsafe {
        // struct menu *menu;
        // struct menu_item item;
        let mut item: menu_item = zeroed();
        // struct status_prompt_menu *spm;
        let lines = status_line_size(c);

        // height, i;
        // u_int py;

        if size <= 1 {
            return 0;
        }
        if (*c).tty.sy - lines < 3 {
            return 0;
        }

        let spm: *mut status_prompt_menu = xmalloc_::<status_prompt_menu>().as_ptr();
        (*spm).c = c;
        (*spm).size = size;
        (*spm).list = list;
        (*spm).flag = flag;

        let mut height = (*c).tty.sy - lines - 2;
        if height > 10 {
            height = 10;
        }
        if height > size {
            height = size;
        }
        (*spm).start = size - height;

        let menu = menu_create(c!(""));
        for i in (*spm).start..size {
            item.name = SyncCharPtr::from_ptr(*list.add(i as usize));
            item.key = b'0' as u64 + (i as i64 - (*spm).start as i64) as u64;
            item.command = SyncCharPtr::null();
            menu_add_item(menu, &raw mut item, null_mut(), c, null_mut());
        }

        let py = if options_get_number_((*(*c).session).options, c"status-position") == 0 {
            lines
        } else {
            (*c).tty.sy - 3 - height
        };
        offset += utf8_cstrwidth((*c).prompt_string);
        if offset > 2 {
            offset -= 2;
        } else {
            offset = 0;
        }

        if menu_display(
            menu,
            MENU_NOMOUSE | MENU_TAB,
            0,
            null_mut(),
            offset,
            py,
            c,
            box_lines::BOX_LINES_DEFAULT,
            null_mut(),
            null_mut(),
            null_mut(),
            null_mut(),
            Some(status_prompt_menu_callback),
            spm.cast(),
        ) != 0
        {
            menu_free(menu);
            free_(spm);
            return 0;
        }
        1
    }
}

/// Show complete word menu.
unsafe fn status_prompt_complete_window_menu(
    c: *mut client,
    s: *mut session,
    word: *const u8,
    mut offset: u32,
    flag: u8,
) -> *mut u8 {
    unsafe {
        // struct menu_item item;
        // struct status_prompt_menu *spm;
        // struct winlink *wl;
        // char **list = NULL, *tmp;
        let mut item: menu_item = zeroed();
        let mut tmp: *mut u8 = null_mut();
        let mut list: *mut *mut u8 = null_mut();
        let lines = status_line_size(c);

        // u_int py;
        let mut size = 0;

        if (*c).tty.sy - lines < 3 {
            return null_mut();
        }

        let spm: *mut status_prompt_menu = xmalloc_::<status_prompt_menu>().as_ptr();
        (*spm).c = c;
        (*spm).flag = flag;

        let mut height = (*c).tty.sy - lines - 2;
        if height > 10 {
            height = 10;
        }
        (*spm).start = 0;

        let menu = menu_create(c!(""));
        for wl in rb_foreach(&raw mut (*s).windows).map(NonNull::as_ptr) {
            if !word.is_null() && *word != b'\0' {
                tmp = format_nul!("{}", (*wl).idx);
                if strncmp(tmp, word, strlen(word)) != 0 {
                    free_(tmp);
                    continue;
                }
                free_(tmp);
            }

            list = xreallocarray_(list, size + 1).as_ptr();
            if (*c).prompt_type == prompt_type::PROMPT_TYPE_WINDOW_TARGET {
                tmp = format_nul!("{} ({})", (*wl).idx, _s((*(*wl).window).name),);
                *list.add(size) = format_nul!("{}", (*wl).idx);
                size += 1;
            } else {
                tmp = format_nul!(
                    "{}:{} ({})",
                    _s((*s).name),
                    (*wl).idx,
                    _s((*(*wl).window).name),
                );
                *list.add(size) = format_nul!("{}:{}", _s((*s).name), (*wl).idx);
                size += 1;
            }
            item.name = SyncCharPtr::from_ptr(tmp);
            item.key = (b'0' as u64) + size as u64 - 1;
            item.command = SyncCharPtr::null();
            menu_add_item(menu, &raw mut item, null_mut(), c, null_mut());
            free_(tmp);

            if size == height as usize {
                break;
            }
        }
        if size == 0 {
            menu_free(menu);
            free_(spm);
            return null_mut();
        }
        if size == 1 {
            menu_free(menu);
            if flag != b'\0' {
                tmp = format_nul!("-{}{}", flag as char, _s(*list));
                free_(*list);
            } else {
                tmp = *list;
            }
            free_(list);
            free_(spm);
            return tmp;
        }
        if height as usize > size {
            height = size as u32;
        }

        (*spm).size = size as u32;
        (*spm).list = list;

        let py = if options_get_number_((*(*c).session).options, c"status-position") == 0 {
            lines
        } else {
            (*c).tty.sy - 3 - height
        };
        offset += utf8_cstrwidth((*c).prompt_string);
        if offset > 2 {
            offset -= 2;
        } else {
            offset = 0;
        }

        if menu_display(
            menu,
            MENU_NOMOUSE | MENU_TAB,
            0,
            null_mut(),
            offset,
            py,
            c,
            box_lines::BOX_LINES_DEFAULT,
            null_mut(),
            null_mut(),
            null_mut(),
            null_mut(),
            Some(status_prompt_menu_callback),
            spm.cast(),
        ) != 0
        {
            menu_free(menu);
            free_(spm);
            return null_mut();
        }
        null_mut()
    }
}

/// Sort complete list.
unsafe extern "C" fn status_prompt_complete_sort(a: *const c_void, b: *const c_void) -> i32 {
    unsafe {
        let aa: *const *const u8 = a.cast();
        let bb: *const *const u8 = b.cast();

        libc::strcmp(*aa, *bb)
    }
}

/// Complete a session.
unsafe fn status_prompt_complete_session(
    list: *mut *mut *mut u8,
    size: *mut u32,
    s: *const u8,
    flag: u8,
) -> *mut u8 {
    unsafe {
        // char *out, *tmp, n[11];

        let mut tmp = null_mut();
        let mut n: [u8; 11] = [0; 11];

        for loop_ in rb_foreach(&raw mut SESSIONS).map(NonNull::as_ptr) {
            if *s == b'\0' || strncmp((*loop_).name, s, strlen(s)) == 0 {
                *list = xreallocarray_(*list, (*size) as usize + 2).as_ptr();
                *(*list).add(*size as usize) = format_nul!("{}:", _s((*loop_).name));
                (*size) += 1;
            } else if *s == b'$' {
                xsnprintf_!((&raw mut n).cast(), n.len(), "{}", (*loop_).id);
                if *s.add(1) == b'\0' || strncmp((&raw mut n).cast(), s.add(1), strlen(s) - 1) == 0
                {
                    *list = xreallocarray_(*list, (*size) as usize + 2).as_ptr();
                    *(*list).add(*size as usize) =
                        format_nul!("${}:", _s((&raw const n).cast::<u8>()));
                    (*size) += 1
                }
            }
        }
        let mut out = status_prompt_complete_prefix(*list, *size);
        if !out.is_null() && flag != b'\0' {
            tmp = format_nul!("-{}{}", flag as char, _s(out));
            free_(out);
            out = tmp;
        }
        out
    }
}

/// Complete word.
unsafe fn status_prompt_complete(c: *mut client, word: *const u8, mut offset: u32) -> *mut u8 {
    unsafe {
        let mut session: *mut session = null_mut();

        let mut s: *const u8 = null();
        let mut colon: *mut u8 = null_mut();

        let mut flag: u8 = b'\0';

        let mut list: *mut *mut u8 = null_mut();
        let mut copy: *mut u8 = null_mut();
        let mut out: *mut u8 = null_mut();
        let mut size: u32 = 0;

        if *word == b'\0'
            && (*c).prompt_type != prompt_type::PROMPT_TYPE_TARGET
            && (*c).prompt_type != prompt_type::PROMPT_TYPE_WINDOW_TARGET
        {
            return null_mut();
        }

        'found: {
            if (*c).prompt_type != prompt_type::PROMPT_TYPE_TARGET
                && (*c).prompt_type != prompt_type::PROMPT_TYPE_WINDOW_TARGET
                && strncmp(word, c!("-t"), 2) != 0
                && strncmp(word, c!("-s"), 2) != 0
            {
                list = status_prompt_complete_list(&raw mut size, word, (offset == 0) as i32);
                out = if size == 0 {
                    null_mut()
                } else if size == 1 {
                    format_nul!("{} ", _s(*list))
                } else {
                    status_prompt_complete_prefix(list, size)
                };
                break 'found;
            }

            if (*c).prompt_type == prompt_type::PROMPT_TYPE_TARGET
                || (*c).prompt_type == prompt_type::PROMPT_TYPE_WINDOW_TARGET
            {
                s = word;
                flag = b'\0';
            } else {
                s = word.add(2);
                flag = *word.add(1);
                offset += 2;
            }

            /* If this is a window completion, open the window menu. */
            if (*c).prompt_type == prompt_type::PROMPT_TYPE_WINDOW_TARGET {
                out = status_prompt_complete_window_menu(c, (*c).session, s, offset, b'\0');
                break 'found;
            }
            colon = libc::strchr(s, b':' as i32);

            /* If there is no colon, complete as a session. */
            if colon.is_null() {
                out = status_prompt_complete_session(&raw mut list, &raw mut size, s, flag);
                break 'found;
            }

            /* If there is a colon but no period, find session and show a menu. */
            if libc::strchr(colon.add(1), b'.' as i32).is_null() {
                if *s == b':' {
                    session = (*c).session;
                } else {
                    copy = xstrdup(s).as_ptr();
                    *libc::strchr(copy, b':' as i32) = b'\0';
                    session = session_find(copy);
                    free_(copy);
                    if session.is_null() {
                        break 'found;
                    }
                }
                out = status_prompt_complete_window_menu(c, session, colon.add(1), offset, flag);
                if out.is_null() {
                    return null_mut();
                }
            }
        } // found:
        if size != 0 {
            libc::qsort(
                list.cast(),
                size as usize,
                size_of::<*mut i8>(),
                Some(status_prompt_complete_sort),
            );
            for i in 0..size {
                // log_debug("complete %u: %s", i, list[i]);
            }
        }

        if !out.is_null() && libc::strcmp(word, out) == 0 {
            free_(out);
            out = null_mut();
        }
        if !out.is_null() || status_prompt_complete_list_menu(c, list, size, offset, flag) == 0 {
            for i in 0..size {
                free_(*list.add(i as usize));
            }
            free_(list);
        }
        out
    }
}

/// Return the type of the prompt as an enum.
pub unsafe fn status_prompt_type(type_: *const u8) -> prompt_type {
    unsafe {
        for i in 0..PROMPT_NTYPES {
            if libc::strcmp(type_, status_prompt_type_string(i)) == 0 {
                return prompt_type::try_from(i).unwrap();
            }
        }
        prompt_type::PROMPT_TYPE_INVALID
    }
}

/// Accessor for prompt_type_strings.
pub unsafe fn status_prompt_type_string(type_: u32) -> *const u8 {
    if type_ >= PROMPT_NTYPES {
        return c!("invalid");
    }
    PROMPT_TYPE_STRINGS[type_ as usize].as_ptr()
}

mod code {
    use super::*;

    pub const A: u64 = 'a' as u64;
    pub const B: u64 = 'b' as u64;
    pub const D: u64 = 'd' as u64;
    pub const E: u64 = 'e' as u64;
    pub const H: u64 = 'h' as u64;
    pub const I: u64 = 'i' as u64;
    pub const J: u64 = 'j' as u64;
    pub const K: u64 = 'k' as u64;
    pub const L: u64 = 'l' as u64;
    pub const P: u64 = 'p' as u64;
    pub const Q: u64 = 'q' as u64;
    pub const S: u64 = 's' as u64;
    pub const W: u64 = 'w' as u64;
    pub const X: u64 = 'x' as u64;

    pub const DOLLAR: u64 = '$' as u64;
    pub const ZERO: u64 = '0' as u64;
    pub const CARET: u64 = '^' as u64;

    pub const A_UPPER: u64 = 'A' as u64;
    pub const B_UPPER: u64 = 'B' as u64;
    pub const C_UPPER: u64 = 'C' as u64;
    pub const D_UPPER: u64 = 'D' as u64;
    pub const E_UPPER: u64 = 'E' as u64;
    pub const I_UPPER: u64 = 'I' as u64;
    pub const S_UPPER: u64 = 'S' as u64;
    pub const W_UPPER: u64 = 'W' as u64;
    pub const X_UPPER: u64 = 'X' as u64;

    pub const TAB: u64 = b'\x09' as u64;
    pub const KEYC_HOME: u64 = keyc::KEYC_HOME as u64;
    pub const KEYC_END: u64 = keyc::KEYC_END as u64;
    pub const KEYC_UP: u64 = keyc::KEYC_UP as u64;
    pub const KEYC_DOWN: u64 = keyc::KEYC_DOWN as u64;
    pub const KEYC_LEFT: u64 = keyc::KEYC_LEFT as u64;
    pub const KEYC_RIGHT: u64 = keyc::KEYC_RIGHT as u64;
    pub const KEYC_BSPACE: u64 = keyc::KEYC_BSPACE as u64;
    pub const KEYC_DC: u64 = keyc::KEYC_DC as u64;

    pub const A_CTRL: u64 = 'a' as u64 | KEYC_CTRL;
    pub const B_CTRL: u64 = 'b' as u64 | KEYC_CTRL;
    pub const C_CTRL: u64 = 'c' as u64 | KEYC_CTRL;
    pub const D_CTRL: u64 = 'd' as u64 | KEYC_CTRL;
    pub const E_CTRL: u64 = 'e' as u64 | KEYC_CTRL;
    pub const F_CTRL: u64 = 'f' as u64 | KEYC_CTRL;
    pub const G_CTRL: u64 = 'g' as u64 | KEYC_CTRL;
    pub const H_CTRL: u64 = 'h' as u64 | KEYC_CTRL;
    pub const K_CTRL: u64 = 'k' as u64 | KEYC_CTRL;
    pub const N_CTRL: u64 = 'n' as u64 | KEYC_CTRL;
    pub const P_CTRL: u64 = 'p' as u64 | KEYC_CTRL;
    pub const R_CTRL: u64 = 'r' as u64 | KEYC_CTRL;
    pub const S_CTRL: u64 = 's' as u64 | KEYC_CTRL;
    pub const T_CTRL: u64 = 't' as u64 | KEYC_CTRL;
    pub const U_CTRL: u64 = 'u' as u64 | KEYC_CTRL;
    pub const W_CTRL: u64 = 'w' as u64 | KEYC_CTRL;
    pub const Y_CTRL: u64 = 'y' as u64 | KEYC_CTRL;

    pub const LEFT_CTRL: u64 = keyc::KEYC_LEFT as u64 | KEYC_CTRL;
    pub const RIGHT_CTRL: u64 = keyc::KEYC_RIGHT as u64 | KEYC_CTRL;

    pub const B_META: u64 = 'b' as u64 | KEYC_META;
    pub const F_META: u64 = 'f' as u64 | KEYC_META;

    pub const E_UPPER_VI: u64 = 'E' as u64 | KEYC_VI;
    pub const E_VI: u64 = 'e' as u64 | KEYC_VI;
    pub const W_UPPER_VI: u64 = 'W' as u64 | KEYC_VI;
    pub const W_VI: u64 = 'w' as u64 | KEYC_VI;
    pub const B_VI: u64 = 'b' as u64 | KEYC_VI;

    pub const CR: u64 = '\r' as u64;
    pub const LF: u64 = '\n' as u64;
    pub const ESC: u64 = '\x1b' as u64;
}

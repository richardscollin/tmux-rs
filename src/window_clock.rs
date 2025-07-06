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

use crate::compat::{queue::tailq_first, strlcat};

pub static window_clock_mode: window_mode = window_mode {
    name: SyncCharPtr::new(c"clock-mode"),

    init: window_clock_init,
    free: window_clock_free,
    resize: window_clock_resize,
    key: Some(window_clock_key),
    default_format: SyncCharPtr::null(),
    update: None,
    key_table: None,
    command: None,
    formats: None,
};

#[repr(C)]
pub struct window_clock_mode_data {
    pub screen: screen,
    pub tim: time_t,
    pub timer: event,
}

#[rustfmt::skip]
pub static mut window_clock_table: [[[u8; 5]; 5]; 14] = [
    [
        [1, 1, 1, 1, 1], /* 0 */
        [1, 0, 0, 0, 1],
        [1, 0, 0, 0, 1],
        [1, 0, 0, 0, 1],
        [1, 1, 1, 1, 1],
    ],
    [
        [0, 0, 0, 0, 1], /* 1 */
        [0, 0, 0, 0, 1],
        [0, 0, 0, 0, 1],
        [0, 0, 0, 0, 1],
        [0, 0, 0, 0, 1],
    ],
    [
        [1, 1, 1, 1, 1], /* 2 */
        [0, 0, 0, 0, 1],
        [1, 1, 1, 1, 1],
        [1, 0, 0, 0, 0],
        [1, 1, 1, 1, 1],
    ],
    [
        [1, 1, 1, 1, 1], /* 3 */
        [0, 0, 0, 0, 1],
        [1, 1, 1, 1, 1],
        [0, 0, 0, 0, 1],
        [1, 1, 1, 1, 1],
    ],
    [
        [1, 0, 0, 0, 1], /* 4 */
        [1, 0, 0, 0, 1],
        [1, 1, 1, 1, 1],
        [0, 0, 0, 0, 1],
        [0, 0, 0, 0, 1],
    ],
    [
        [1, 1, 1, 1, 1], /* 5 */
        [1, 0, 0, 0, 0],
        [1, 1, 1, 1, 1],
        [0, 0, 0, 0, 1],
        [1, 1, 1, 1, 1],
    ],
    [
        [1, 1, 1, 1, 1], /* 6 */
        [1, 0, 0, 0, 0],
        [1, 1, 1, 1, 1],
        [1, 0, 0, 0, 1],
        [1, 1, 1, 1, 1],
    ],
    [
        [1, 1, 1, 1, 1], /* 7 */
        [0, 0, 0, 0, 1],
        [0, 0, 0, 0, 1],
        [0, 0, 0, 0, 1],
        [0, 0, 0, 0, 1],
    ],
    [
        [1, 1, 1, 1, 1], /* 8 */
        [1, 0, 0, 0, 1],
        [1, 1, 1, 1, 1],
        [1, 0, 0, 0, 1],
        [1, 1, 1, 1, 1],
    ],
    [
        [1, 1, 1, 1, 1], /* 9 */
        [1, 0, 0, 0, 1],
        [1, 1, 1, 1, 1],
        [0, 0, 0, 0, 1],
        [1, 1, 1, 1, 1],
    ],
    [
        [0, 0, 0, 0, 0], /* : */
        [0, 0, 1, 0, 0],
        [0, 0, 0, 0, 0],
        [0, 0, 1, 0, 0],
        [0, 0, 0, 0, 0],
    ],
    [
        [1, 1, 1, 1, 1], /* A */
        [1, 0, 0, 0, 1],
        [1, 1, 1, 1, 1],
        [1, 0, 0, 0, 1],
        [1, 0, 0, 0, 1],
    ],
    [
        [1, 1, 1, 1, 1], /* P */
        [1, 0, 0, 0, 1],
        [1, 1, 1, 1, 1],
        [1, 0, 0, 0, 0],
        [1, 0, 0, 0, 0],
    ],
    [
        [1, 0, 0, 0, 1], /* M */
        [1, 1, 0, 1, 1],
        [1, 0, 1, 0, 1],
        [1, 0, 0, 0, 1],
        [1, 0, 0, 0, 1],
    ],
];

pub unsafe extern "C" fn window_clock_timer_callback(fd: i32, events: i16, arg: *mut c_void) {
    unsafe {
        let wme = arg as *mut window_mode_entry;
        let wp = (*wme).wp;
        let data = (*wme).data as *mut window_clock_mode_data;
        let mut now: libc::tm = zeroed();
        let mut then: libc::tm = zeroed();
        let mut t: time_t;
        let tv: timeval = timeval {
            tv_sec: 1,
            tv_usec: 0,
        };

        evtimer_del(&raw mut (*data).timer);
        evtimer_add(&raw mut (*data).timer, &tv);

        if tailq_first(&raw mut (*wp).modes) != wme {
            return;
        }

        t = libc::time(null_mut());
        libc::gmtime_r(&raw mut t, &raw mut now);
        libc::gmtime_r(&raw mut (*data).tim, &raw mut then);
        if now.tm_min == then.tm_min {
            return;
        }
        (*data).tim = t;

        window_clock_draw_screen(NonNull::new(wme).unwrap());
        (*wp).flags |= window_pane_flags::PANE_REDRAW;
    }
}

pub unsafe fn window_clock_init(
    wme: NonNull<window_mode_entry>,
    _fs: *mut cmd_find_state,
    args: *mut args,
) -> *mut screen {
    unsafe {
        let wp: *mut window_pane = (*wme.as_ptr()).wp;
        let mut tv = timeval {
            tv_sec: 1,
            tv_usec: 0,
        };

        let data = xmalloc_::<window_clock_mode_data>().as_ptr();
        (*wme.as_ptr()).data = data.cast();
        (*data).tim = libc::time(null_mut());

        evtimer_set(
            &raw mut (*data).timer,
            Some(window_clock_timer_callback),
            wme.cast().as_ptr(),
        );
        evtimer_add(&raw mut (*data).timer, &raw mut tv);

        let s = &raw mut (*data).screen;
        screen_init(
            s,
            screen_size_x(&raw mut (*wp).base),
            screen_size_y(&raw mut (*wp).base),
            0,
        );
        (*s).mode &= !mode_flag::MODE_CURSOR;

        window_clock_draw_screen(wme);

        s
    }
}

pub unsafe fn window_clock_free(wme: NonNull<window_mode_entry>) {
    unsafe {
        let data = (*wme.as_ptr()).data as *mut window_clock_mode_data;

        evtimer_del(&raw mut (*data).timer);
        screen_free(&raw mut (*data).screen);
        free_(data);
    }
}

pub unsafe fn window_clock_resize(wme: NonNull<window_mode_entry>, sx: u32, sy: u32) {
    unsafe {
        let data = (*wme.as_ptr()).data as *mut window_clock_mode_data;
        let s = &raw mut (*data).screen;

        screen_resize(s, sx, sy, 0);
        window_clock_draw_screen(wme);
    }
}

pub unsafe fn window_clock_key(
    wme: NonNull<window_mode_entry>,
    c: *mut client,
    s: *mut session,
    wl: *mut winlink,
    key: key_code,
    m: *mut mouse_event,
) {
    unsafe {
        window_pane_reset_mode((*wme.as_ptr()).wp);
    }
}

pub unsafe fn window_clock_draw_screen(wme: NonNull<window_mode_entry>) {
    unsafe {
        let wp = (*wme.as_ptr()).wp;
        let data = (*wme.as_ptr()).data as *mut window_clock_mode_data;
        let mut ctx: screen_write_ctx = zeroed();
        let mut colour: i32;
        let mut style: i32;
        let s = &raw mut (*data).screen;
        const sizeof_tim: usize = 64;
        let mut tim: [u8; 64] = [0; 64];
        let mut x: u32 = 0;
        let mut y: u32 = 0;
        let mut idx: u32 = 0;

        let colour = options_get_number_((*(*wp).window).options, c"clock-mode-colour");
        let style = options_get_number_((*(*wp).window).options, c"clock-mode-style");

        screen_write_start(&raw mut ctx, s);

        let mut t = libc::time(null_mut());
        let tm = libc::localtime(&raw mut t);
        if style == 0 {
            libc::strftime(
                &raw mut tim as _,
                sizeof_tim,
                c!("%l:%M "),
                libc::localtime(&raw mut t),
            );
            if (*tm).tm_hour >= 12 {
                strlcat(&raw mut tim as _, c!("PM"), sizeof_tim);
            } else {
                strlcat(&raw mut tim as _, c!("AM"), sizeof_tim);
            }
        } else {
            libc::strftime(&raw mut tim as _, sizeof_tim, c!("%H:%M"), tm);
        }

        screen_write_clearscreen(&raw mut ctx, 8);

        let mut gc = MaybeUninit::<grid_cell>::uninit();
        let tim_len = strlen(&raw const tim as _) as u32;
        if screen_size_x(s) < 6 * tim_len || screen_size_y(s) < 6 {
            if screen_size_x(s) >= tim_len && screen_size_y(s) != 0 {
                x = (screen_size_x(s) / 2) - (tim_len / 2) as u32;
                y = screen_size_y(s) / 2;
                screen_write_cursormove(&raw mut ctx, x as i32, y as i32, 0);

                gc.write(grid_default_cell);
                (*gc.as_mut_ptr()).flags |= grid_flag::NOPALETTE;
                (*gc.as_mut_ptr()).fg = colour as i32;
                screen_write_puts!(
                    &raw mut ctx,
                    gc.as_mut_ptr(),
                    "{}",
                    _s((&raw const tim).cast::<u8>())
                );
            }

            screen_write_stop(&raw mut ctx);
            return;
        }

        x = (screen_size_x(s) / 2) - 3 * tim_len;
        y = (screen_size_y(s) / 2) - 3;

        gc.write(grid_default_cell);
        (*gc.as_mut_ptr()).flags |= grid_flag::NOPALETTE;
        (*gc.as_mut_ptr()).bg = colour as i32;
        let mut ptr = &raw mut tim as *mut u8;
        while *ptr != b'\0' {
            if *ptr >= b'0' && *ptr <= b'9' {
                idx = (*ptr - b'0') as u32;
            } else if *ptr == b':' {
                idx = 10;
            } else if *ptr == b'A' {
                idx = 11;
            } else if *ptr == b'P' {
                idx = 12;
            } else if *ptr == b'M' {
                idx = 13;
            } else {
                x += 6;
                ptr = ptr.add(1);
                continue;
            }

            for j in 0..5 {
                for i in 0..5 {
                    screen_write_cursormove(&raw mut ctx, (x + i) as i32, (y + j) as i32, 0);
                    if window_clock_table[idx as usize][j as usize][i as usize] != 0 {
                        screen_write_putc(&raw mut ctx, gc.as_ptr(), b' ');
                    }
                }
            }
            x += 6;
            ptr = ptr.add(1);
        }

        screen_write_stop(&raw mut ctx);
    }
}

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
use crate::{colour::colour_split_rgb, compat::b64::b64_ntop};

static mut TTY_LOG_FD: i32 = -1;

#[inline]
unsafe fn tty_use_margin(tty: *const tty) -> bool {
    unsafe { (*(*tty).term).flags.intersects(term_flags::TERM_DECSLRM) }
}

#[inline]
unsafe fn tty_full_width(tty: *const tty, ctx: *const tty_ctx) -> bool {
    unsafe { (*ctx).xoff == 0 && (*ctx).sx >= (*tty).sx }
}

const TTY_BLOCK_INTERVAL: libc::suseconds_t = 100_000; // 100 millis
const TTY_QUERY_TIMEOUT: i32 = 5;
const TTY_REQUEST_LIMIT: i32 = 30;

#[expect(non_snake_case)]
#[inline]
unsafe fn TTY_BLOCK_START(tty: *const tty) -> u32 {
    unsafe { 1 + ((*tty).sx * (*tty).sy) * 8 }
}

#[expect(non_snake_case)]
#[inline]
unsafe fn TTY_BLOCK_STOP(tty: *const tty) -> u32 {
    unsafe { 1 + ((*tty).sx * (*tty).sy) / 8 }
}

pub unsafe fn tty_create_log() {
    unsafe {
        let mut name: [u8; 64] = [0; 64];

        _ = xsnprintf_!(
            (&raw mut name).cast(),
            64,
            "tmux-out-{}.log",
            libc::getpid()
        );

        TTY_LOG_FD = libc::open(
            (&raw const name).cast(),
            libc::O_WRONLY | libc::O_CREAT | libc::O_TRUNC,
            0o644,
        );
        if TTY_LOG_FD != -1 && libc::fcntl(TTY_LOG_FD, libc::F_SETFD, libc::FD_CLOEXEC) == -1 {
            fatal("fcntl failed");
        }
    }
}

pub unsafe fn tty_init(tty: *mut tty, c: *mut client) -> i32 {
    unsafe {
        if libc::isatty((*c).fd) == 0 {
            return -1;
        }

        memset0(tty);
        (*tty).client = c;

        (*tty).cstyle = screen_cursor_style::SCREEN_CURSOR_DEFAULT;
        (*tty).ccolour = -1;
        (*tty).fg = -1;
        (*tty).bg = -1;

        if libc::tcgetattr((*c).fd, &raw mut (*tty).tio) != 0 {
            return -1;
        }

        0
    }
}

pub unsafe fn tty_resize(tty: *mut tty) {
    unsafe {
        let c = (*tty).client;
        let mut ws: libc::winsize = zeroed();
        let mut sx: u32;
        let mut sy: u32;
        let xpixel: u32;
        let ypixel: u32;

        if libc::ioctl((*c).fd, libc::TIOCGWINSZ, &raw mut ws) != -1 {
            sx = ws.ws_col as u32;
            if sx == 0 {
                sx = 80;
                xpixel = 0;
            } else {
                xpixel = ws.ws_xpixel as u32 / sx;
            }
            sy = ws.ws_row as u32;
            if sy == 0 {
                sy = 24;
                ypixel = 0;
            } else {
                ypixel = ws.ws_ypixel as u32 / sy;
            }
        } else {
            sx = 80;
            sy = 24;
            xpixel = 0;
            ypixel = 0;
        }
        // log_debug("%s: %s now %ux%u (%ux%u)", __func__, (*c).name, sx, sy, xpixel, ypixel);
        tty_set_size(tty, sx, sy, xpixel, ypixel);
        tty_invalidate(tty);
    }
}

pub unsafe fn tty_set_size(tty: *mut tty, sx: u32, sy: u32, xpixel: u32, ypixel: u32) {
    unsafe {
        (*tty).sx = sx;
        (*tty).sy = sy;
        (*tty).xpixel = xpixel;
        (*tty).ypixel = ypixel;
    }
}

pub unsafe extern "C-unwind" fn tty_read_callback(_fd: i32, _events: i16, data: *mut c_void) {
    unsafe {
        let tty = data as *mut tty;
        let c = (*tty).client;
        let name = (*c).name;
        let size = EVBUFFER_LENGTH((*tty).in_);

        let nread = evbuffer_read((*tty).in_, (*c).fd, -1);
        if nread == 0 || nread == -1 {
            if nread == 0 {
                log_debug!("{}: read closed", _s(name));
            } else {
                log_debug!("{}: read error: {}", _s(name), _s(strerror(errno!())));
            }
            event_del(&raw mut (*tty).event_in);
            server_client_lost((*tty).client);
            return;
        }
        log_debug!("{}: read {} bytes (already {})", _s(name), nread, size);

        while tty_keys_next(tty) != 0 {}
    }
}

pub unsafe extern "C-unwind" fn tty_timer_callback(_fd: i32, _events: i16, tty: NonNull<tty>) {
    unsafe {
        let tty = tty.as_ptr();
        let c = (*tty).client;
        let mut tv = libc::timeval {
            tv_sec: 0,
            tv_usec: TTY_BLOCK_INTERVAL,
        };

        // log_debug("%s: %zu discarded", (*c).name, (*tty).discarded);

        (*c).flags |= CLIENT_ALLREDRAWFLAGS;
        (*c).discarded += (*tty).discarded;

        if (*tty).discarded < TTY_BLOCK_STOP(tty) as usize {
            (*tty).flags &= !tty_flags::TTY_BLOCK;
            tty_invalidate(tty);
            return;
        }
        (*tty).discarded = 0;
        evtimer_add(&raw mut (*tty).timer, &raw mut tv);
    }
}

pub unsafe fn tty_block_maybe(tty: *mut tty) -> i32 {
    unsafe {
        let c = (*tty).client;
        let size = EVBUFFER_LENGTH((*tty).out);
        let tv = libc::timeval {
            tv_sec: 0,
            tv_usec: TTY_BLOCK_INTERVAL,
        };

        if size == 0 {
            (*tty).flags &= !tty_flags::TTY_NOBLOCK;
        } else if (*tty).flags.intersects(tty_flags::TTY_NOBLOCK) {
            return 0;
        }

        if size < TTY_BLOCK_START(tty) as usize {
            return 0;
        }

        if (*tty).flags.intersects(tty_flags::TTY_BLOCK) {
            return 1;
        }
        (*tty).flags |= tty_flags::TTY_BLOCK;

        // log_debug("%s: can't keep up, %zu discarded", (*c).name, size);

        evbuffer_drain((*tty).out, size);
        (*c).discarded += size;

        (*tty).discarded = 0;
        evtimer_add(&raw mut (*tty).timer, &raw const tv);
        1
    }
}

pub unsafe extern "C-unwind" fn tty_write_callback(_fd: i32, _events: i16, data: *mut c_void) {
    unsafe {
        let tty = data as *mut tty;
        let c = (*tty).client;
        let _size = EVBUFFER_LENGTH((*tty).out);

        let nwrite: i32 = evbuffer_write((*tty).out, (*c).fd);
        if nwrite == -1 {
            return;
        }
        // this log is too noisy
        // log_debug!("{}: wrote {} bytes (of {})", _s((*c).name), nwrite, size);

        if (*c).redraw > 0 {
            if nwrite as usize >= (*c).redraw {
                (*c).redraw = 0;
            } else {
                (*c).redraw -= nwrite as usize;
            }
            log_debug!(
                "{}: waiting for redraw, {} bytes left",
                _s((*c).name),
                (*c).redraw
            );
        } else if tty_block_maybe(tty) != 0 {
            return;
        }

        if EVBUFFER_LENGTH((*tty).out) != 0 {
            event_add(&raw mut (*tty).event_out, null_mut());
        }
    }
}

pub unsafe fn tty_open(tty: *mut tty, cause: *mut *mut u8) -> i32 {
    unsafe {
        let c = (*tty).client;

        (*tty).term = tty_term_create(
            tty,
            (*c).term_name,
            (*c).term_caps,
            (*c).term_ncaps,
            &raw mut (*c).term_features,
            cause,
        );
        if (*tty).term.is_null() {
            tty_close(tty);
            return -1;
        }
        (*tty).flags |= tty_flags::TTY_OPENED;

        (*tty).flags &= !(tty_flags::TTY_NOCURSOR
            | tty_flags::TTY_FREEZE
            | tty_flags::TTY_BLOCK
            | tty_flags::TTY_TIMER);

        event_set(
            &raw mut (*tty).event_in,
            (*c).fd,
            EV_PERSIST | EV_READ,
            Some(tty_read_callback),
            tty.cast(),
        );
        (*tty).in_ = evbuffer_new();
        if (*tty).in_.is_null() {
            fatal("out of memory");
        }

        event_set(
            &raw mut (*tty).event_out,
            (*c).fd,
            EV_WRITE,
            Some(tty_write_callback),
            tty.cast(),
        );
        (*tty).out = evbuffer_new();
        if (*tty).out.is_null() {
            fatal("out of memory");
        }

        evtimer_set(
            &raw mut (*tty).timer,
            tty_timer_callback,
            NonNull::new_unchecked(tty),
        );

        tty_start_tty(tty);
        tty_keys_build(tty);

        0
    }
}

pub unsafe extern "C-unwind" fn tty_start_timer_callback(
    _fd: i32,
    _events: i16,
    tty: NonNull<tty>,
) {
    unsafe {
        let tty = tty.as_ptr();
        let c = (*tty).client;

        log_debug!("{}: start timer fired", _s((*c).name));
        if (*tty)
            .flags
            .intersects(tty_flags::TTY_HAVEDA | tty_flags::TTY_HAVEDA2 | tty_flags::TTY_HAVEXDA)
        {
            tty_update_features(tty);
        }
        (*tty).flags |= TTY_ALL_REQUEST_FLAGS;
    }
}

pub unsafe fn tty_start_tty(tty: *mut tty) {
    unsafe {
        let c = (*tty).client;
        let mut tio: libc::termios = zeroed();
        let tv = libc::timeval {
            tv_sec: TTY_QUERY_TIMEOUT as i64,
            tv_usec: 0,
        };

        setblocking((*c).fd, 0);
        event_add(&raw mut (*tty).event_in, null_mut());

        memcpy__(&raw mut tio, &raw const (*tty).tio);
        tio.c_iflag &= !(libc::IXON
            | libc::IXOFF
            | libc::ICRNL
            | libc::INLCR
            | libc::IGNCR
            | libc::IMAXBEL
            | libc::ISTRIP);
        tio.c_iflag |= libc::IGNBRK;
        tio.c_oflag &= !(libc::OPOST | libc::ONLCR | libc::OCRNL | libc::ONLRET);
        tio.c_lflag &= !(libc::IEXTEN
            | libc::ICANON
            | libc::ECHO
            | libc::ECHOE
            | libc::ECHONL
            | libc::ECHOCTL
            | libc::ECHOPRT
            | libc::ECHOKE
            | libc::ISIG);
        tio.c_cc[libc::VMIN] = 1;
        tio.c_cc[libc::VTIME] = 0;
        if libc::tcsetattr((*c).fd, libc::TCSANOW, &raw mut tio) == 0 {
            libc::tcflush((*c).fd, libc::TCOFLUSH);
        }

        tty_putcode(tty, tty_code_code::TTYC_SMCUP);

        tty_putcode(tty, tty_code_code::TTYC_SMKX);
        tty_putcode(tty, tty_code_code::TTYC_CLEAR);

        if tty_acs_needed(tty) != 0 {
            // log_debug("%s: using capabilities for ACS", (*c).name);
            tty_putcode(tty, tty_code_code::TTYC_ENACS);
        } else {
            // log_debug("%s: using UTF-8 for ACS", (*c).name);
        }

        tty_putcode(tty, tty_code_code::TTYC_CNORM);
        if tty_term_has((*tty).term, tty_code_code::TTYC_KMOUS) {
            tty_puts(tty, c!("\x1b[?1000l\x1b[?1002l\x1b[?1003l"));
            tty_puts(tty, c!("\x1b[?1006l\x1b[?1005l"));
        }
        if tty_term_has((*tty).term, tty_code_code::TTYC_ENBP) {
            tty_putcode(tty, tty_code_code::TTYC_ENBP);
        }

        evtimer_set(
            &raw mut (*tty).start_timer,
            tty_start_timer_callback,
            NonNull::new_unchecked(tty),
        );
        evtimer_add(&raw mut (*tty).start_timer, &raw const tv);

        (*tty).flags |= tty_flags::TTY_STARTED;
        tty_invalidate(tty);

        if (*tty).ccolour != -1 {
            tty_force_cursor_colour(tty, -1);
        }

        (*tty).mouse_drag_flag = 0;
        (*tty).mouse_drag_update = None;
        (*tty).mouse_drag_release = None;
    }
}

pub unsafe fn tty_send_requests(tty: *mut tty) {
    unsafe {
        if !(*tty).flags.intersects(tty_flags::TTY_STARTED) {
            return;
        }

        if (*(*tty).term).flags.intersects(term_flags::TERM_VT100LIKE) {
            // TODO I think the original C code has a bug and it should be as follows, double check
            if !(*tty).flags.intersects(tty_flags::TTY_HAVEDA) {
                tty_puts(tty, c!("\x1b[c"));
            }
            if !(*tty).flags.intersects(tty_flags::TTY_HAVEDA2) {
                tty_puts(tty, c!("\x1b[>c"));
            }
            if !(*tty).flags.intersects(tty_flags::TTY_HAVEXDA) {
                tty_puts(tty, c!("\x1b[>q"));
            }
            tty_puts(tty, c!("\x1b]10;?\x1b\\"));
            tty_puts(tty, c!("\x1b]11;?\x1b\\"));
        } else {
            (*tty).flags |= TTY_ALL_REQUEST_FLAGS;
        }
        (*tty).last_requests = libc::time(null_mut());
    }
}

pub unsafe fn tty_repeat_requests(tty: *mut tty) {
    unsafe {
        let t = libc::time(null_mut());

        if !(*tty).flags.intersects(tty_flags::TTY_STARTED) {
            return;
        }

        if t - (*tty).last_requests <= TTY_REQUEST_LIMIT as i64 {
            return;
        }
        (*tty).last_requests = t;

        if (*(*tty).term).flags.intersects(term_flags::TERM_VT100LIKE) {
            tty_puts(tty, c!("\x1b]10;?\x1b\\"));
            tty_puts(tty, c!("\x1b]11;?\x1b\\"));
        }
    }
}

pub unsafe fn tty_stop_tty(tty: *mut tty) {
    unsafe {
        let c = (*tty).client;
        let ws: libc::winsize = zeroed();

        if !(*tty).flags.intersects(tty_flags::TTY_STARTED) {
            return;
        }
        (*tty).flags &= !tty_flags::TTY_STARTED;

        evtimer_del(&raw mut (*tty).start_timer);

        event_del(&raw mut (*tty).timer);
        (*tty).flags &= !tty_flags::TTY_BLOCK;

        event_del(&raw mut (*tty).event_in);
        event_del(&raw mut (*tty).event_out);

        // Be flexible about error handling and try not kill the server just
        // because the fd is invalid. Things like ssh -t can easily leave us
        // with a dead tty.
        if libc::ioctl((*c).fd, libc::TIOCGWINSZ, &ws) == -1 {
            return;
        }
        if libc::tcsetattr((*c).fd, libc::TCSANOW, &(*tty).tio) == -1 {
            return;
        }

        tty_raw(
            tty,
            tty_term_string_ii(
                (*tty).term,
                tty_code_code::TTYC_CSR,
                0,
                ws.ws_row as i32 - 1,
            ),
        );
        if tty_acs_needed(tty) != 0 {
            tty_raw(tty, tty_term_string((*tty).term, tty_code_code::TTYC_RMACS));
        }
        tty_raw(tty, tty_term_string((*tty).term, tty_code_code::TTYC_SGR0));
        tty_raw(tty, tty_term_string((*tty).term, tty_code_code::TTYC_RMKX));
        tty_raw(tty, tty_term_string((*tty).term, tty_code_code::TTYC_CLEAR));
        if (*tty).cstyle != screen_cursor_style::SCREEN_CURSOR_DEFAULT {
            if tty_term_has((*tty).term, tty_code_code::TTYC_SE) {
                tty_raw(tty, tty_term_string((*tty).term, tty_code_code::TTYC_SE));
            } else if tty_term_has((*tty).term, tty_code_code::TTYC_SS) {
                tty_raw(
                    tty,
                    tty_term_string_i((*tty).term, tty_code_code::TTYC_SS, 0),
                );
            }
        }
        if (*tty).ccolour != -1 {
            tty_raw(tty, tty_term_string((*tty).term, tty_code_code::TTYC_CR));
        }

        tty_raw(tty, tty_term_string((*tty).term, tty_code_code::TTYC_CNORM));
        if tty_term_has((*tty).term, tty_code_code::TTYC_KMOUS) {
            tty_raw(tty, c!("\x1b[?1000l\x1b[?1002l\x1b[?1003l"));
            tty_raw(tty, c!("\x1b[?1006l\x1b[?1005l"));
        }
        if tty_term_has((*tty).term, tty_code_code::TTYC_DSBP) {
            tty_raw(tty, tty_term_string((*tty).term, tty_code_code::TTYC_DSBP));
        }

        if (*(*tty).term).flags.intersects(term_flags::TERM_VT100LIKE) {
            tty_raw(tty, c!("\x1b[?7727l"));
        }
        tty_raw(tty, tty_term_string((*tty).term, tty_code_code::TTYC_DSFCS));
        tty_raw(tty, tty_term_string((*tty).term, tty_code_code::TTYC_DSEKS));

        if tty_use_margin(tty) {
            tty_raw(tty, tty_term_string((*tty).term, tty_code_code::TTYC_DSMG));
        }
        tty_raw(tty, tty_term_string((*tty).term, tty_code_code::TTYC_RMCUP));

        setblocking((*c).fd, 1);
    }
}

pub unsafe fn tty_close(tty: *mut tty) {
    unsafe {
        if event_initialized(&raw mut (*tty).key_timer) != 0 {
            evtimer_del(&raw mut (*tty).key_timer);
        }
        tty_stop_tty(tty);

        if (*tty).flags.intersects(tty_flags::TTY_OPENED) {
            evbuffer_free((*tty).in_);
            event_del(&raw mut (*tty).event_in);
            evbuffer_free((*tty).out);
            event_del(&raw mut (*tty).event_out);

            tty_term_free((*tty).term);
            tty_keys_free(tty);

            (*tty).flags &= !tty_flags::TTY_OPENED;
        }
    }
}

pub unsafe fn tty_free(tty: *mut tty) {
    unsafe {
        tty_close(tty);
    }
}

pub unsafe fn tty_update_features(tty: *mut tty) {
    unsafe {
        let c = (*tty).client;

        if tty_apply_features((*tty).term, (*c).term_features) {
            tty_term_apply_overrides((*tty).term);
        }

        if tty_use_margin(tty) {
            tty_putcode(tty, tty_code_code::TTYC_ENMG);
        }
        if options_get_number_(GLOBAL_OPTIONS, "extended-keys") != 0 {
            tty_puts(tty, tty_term_string((*tty).term, tty_code_code::TTYC_ENEKS));
        }
        if options_get_number_(GLOBAL_OPTIONS, "focus-events") != 0 {
            tty_puts(tty, tty_term_string((*tty).term, tty_code_code::TTYC_ENFCS));
        }
        if (*(*tty).term).flags.intersects(term_flags::TERM_VT100LIKE) {
            tty_puts(tty, c!("\x1b[?7727h"));
        }

        // Features might have changed since the first draw during attach. For
        // example, this happens when DA responses are received.
        server_redraw_client(c);

        tty_invalidate(tty);
    }
}

pub unsafe fn tty_raw(tty: *mut tty, mut s: *const u8) {
    unsafe {
        let c = (*tty).client;

        let mut slen = strlen(s);
        for _ in 0..5 {
            let n = libc::write((*c).fd, s.cast(), slen);
            if n >= 0 {
                s = s.add(n as usize);
                slen -= n as usize;
                if slen == 0 {
                    break;
                }
            } else if n == -1 && errno!() != libc::EAGAIN {
                break;
            }
            libc::usleep(100);
        }
    }
}

pub unsafe fn tty_putcode(tty: *mut tty, code: tty_code_code) {
    unsafe {
        tty_puts(tty, tty_term_string((*tty).term, code));
    }
}

pub unsafe fn tty_putcode_i(tty: *mut tty, code: tty_code_code, a: i32) {
    unsafe {
        if a < 0 {
            return;
        }
        tty_puts(tty, tty_term_string_i((*tty).term, code, a));
    }
}

pub unsafe fn tty_putcode_ii(tty: *mut tty, code: tty_code_code, a: i32, b: i32) {
    unsafe {
        if a < 0 || b < 0 {
            return;
        }
        tty_puts(tty, tty_term_string_ii((*tty).term, code, a, b));
    }
}

pub unsafe fn tty_putcode_iii(tty: *mut tty, code: tty_code_code, a: i32, b: i32, c: i32) {
    unsafe {
        if a < 0 || b < 0 || c < 0 {
            return;
        }
        tty_puts(tty, tty_term_string_iii((*tty).term, code, a, b, c));
    }
}

pub unsafe fn tty_putcode_s(tty: *mut tty, code: tty_code_code, a: *const u8) {
    unsafe {
        if !a.is_null() {
            tty_puts(tty, tty_term_string_s((*tty).term, code, a));
        }
    }
}

pub unsafe fn tty_putcode_ss(tty: *mut tty, code: tty_code_code, a: *const u8, b: *const u8) {
    unsafe {
        if !a.is_null() && !b.is_null() {
            tty_puts(tty, tty_term_string_ss((*tty).term, code, a, b));
        }
    }
}

pub unsafe fn tty_add(tty: *mut tty, buf: *const u8, len: usize) {
    unsafe {
        let c = (*tty).client;

        if (*tty).flags.intersects(tty_flags::TTY_BLOCK) {
            (*tty).discarded += len;
            return;
        }

        evbuffer_add((*tty).out, buf.cast(), len);
        // log_debug("%s: %.*s", (*c).name, (int)len, buf);
        (*c).written += len;

        if TTY_LOG_FD != -1 {
            libc::write(TTY_LOG_FD, buf.cast(), len);
        }
        if (*tty).flags.intersects(tty_flags::TTY_STARTED) {
            event_add(&raw mut (*tty).event_out, null_mut());
        }
    }
}

pub unsafe fn tty_puts(tty: *mut tty, s: *const u8) {
    unsafe {
        if *s != b'\0' {
            tty_add(tty, s, strlen(s));
        }
    }
}

pub unsafe fn tty_putc(tty: *mut tty, ch: u8) {
    unsafe {
        if (*(*tty).term).flags.intersects(term_flags::TERM_NOAM)
            && ch >= 0x20
            && ch != 0x7f
            && (*tty).cy == (*tty).sy - 1
            && (*tty).cx + 1 >= (*tty).sx
        {
            return;
        }

        if (*tty).cell.attr.intersects(grid_attr::GRID_ATTR_CHARSET) {
            let acs = tty_acs_get(tty, ch);
            if !acs.is_null() {
                tty_add(tty, acs, strlen(acs));
            } else {
                tty_add(tty, (&raw const ch).cast(), 1);
            }
        } else {
            tty_add(tty, (&raw const ch).cast(), 1);
        }

        if ch >= 0x20 && ch != 0x7f {
            if (*tty).cx >= (*tty).sx {
                (*tty).cx = 1;
                if (*tty).cy != (*tty).rlower {
                    (*tty).cy += 1;
                }

                // On !am terminals, force the cursor position to where
                // we think it should be after a line wrap - this means
                // it works on sensible terminals as well.
                if (*(*tty).term).flags.intersects(term_flags::TERM_NOAM) {
                    tty_putcode_ii(
                        tty,
                        tty_code_code::TTYC_CUP,
                        (*tty).cy as i32,
                        (*tty).cx as i32,
                    );
                }
            } else {
                (*tty).cx += 1;
            }
        }
    }
}

pub unsafe fn tty_putn(tty: *mut tty, buf: *const c_void, mut len: usize, width: u32) {
    unsafe {
        if (*(*tty).term).flags.intersects(term_flags::TERM_NOAM)
            && (*tty).cy == (*tty).sy - 1
            && (*tty).cx as usize + len >= (*tty).sx as usize
        {
            len = ((*tty).sx - (*tty).cx - 1) as usize;
        }

        tty_add(tty, buf.cast(), len);
        if (*tty).cx + width > (*tty).sx {
            (*tty).cx = ((*tty).cx + width) - (*tty).sx;
            if (*tty).cx <= (*tty).sx {
                (*tty).cy += 1;
            } else {
                (*tty).cx = u32::MAX;
                (*tty).cy = u32::MAX;
            }
        } else {
            (*tty).cx += width;
        }
    }
}

pub unsafe fn tty_set_italics(tty: *mut tty) {
    unsafe {
        if tty_term_has((*tty).term, tty_code_code::TTYC_SITM) {
            let s = options_get_string_(GLOBAL_OPTIONS, "default-terminal");
            if !streq_(s, "screen") && libc::strncmp(s, c!("screen-"), 7) != 0 {
                tty_putcode(tty, tty_code_code::TTYC_SITM);
                return;
            }
        }
        tty_putcode(tty, tty_code_code::TTYC_SMSO);
    }
}

pub unsafe fn tty_set_title(tty: *mut tty, title: *const u8) {
    unsafe {
        if !tty_term_has((*tty).term, tty_code_code::TTYC_TSL)
            || !tty_term_has((*tty).term, tty_code_code::TTYC_FSL)
        {
            return;
        }

        tty_putcode(tty, tty_code_code::TTYC_TSL);
        tty_puts(tty, title);
        tty_putcode(tty, tty_code_code::TTYC_FSL);
    }
}

pub unsafe fn tty_set_path(tty: *mut tty, title: *const u8) {
    unsafe {
        if !tty_term_has((*tty).term, tty_code_code::TTYC_SWD)
            || !tty_term_has((*tty).term, tty_code_code::TTYC_FSL)
        {
            return;
        }

        tty_putcode(tty, tty_code_code::TTYC_SWD);
        tty_puts(tty, title);
        tty_putcode(tty, tty_code_code::TTYC_FSL);
    }
}

pub unsafe fn tty_force_cursor_colour(tty: *mut tty, mut c: i32) {
    unsafe {
        let mut s: [u8; 13] = [0; 13];

        if c != -1 {
            c = colour_force_rgb(c);
        }
        if c == (*tty).ccolour {
            return;
        }
        if c == -1 {
            tty_putcode(tty, tty_code_code::TTYC_CR);
        } else {
            let (r, g, b) = colour_split_rgb(c);
            _ = xsnprintf_!((&raw mut s).cast(), 13, "rgb:{:02x}/{:02x}/{:02x}", r, g, b,);
            tty_putcode_s(tty, tty_code_code::TTYC_CS, (&raw const s).cast::<u8>());
        }
        (*tty).ccolour = c;
    }
}

pub unsafe fn tty_update_cursor(tty: *mut tty, mode: mode_flag, s: *mut screen) -> mode_flag {
    unsafe {
        let mut cstyle: screen_cursor_style;
        let mut ccolour: i32;
        let mut cmode: mode_flag = mode;

        // Set cursor colour if changed.
        if !s.is_null() {
            ccolour = (*s).ccolour;
            if (*s).ccolour == -1 {
                ccolour = (*s).default_ccolour;
            }
            tty_force_cursor_colour(tty, ccolour);
        }

        // If cursor is off, set as invisible.
        if !cmode.intersects(mode_flag::MODE_CURSOR) {
            if (*tty).mode.intersects(mode_flag::MODE_CURSOR) {
                tty_putcode(tty, tty_code_code::TTYC_CIVIS);
            }
            return cmode;
        }

        // Check if blinking or very visible flag changed or style changed.
        if s.is_null() {
            cstyle = (*tty).cstyle;
        } else {
            cstyle = (*s).cstyle;
            if cstyle == screen_cursor_style::SCREEN_CURSOR_DEFAULT {
                if !cmode.intersects(mode_flag::MODE_CURSOR_BLINKING_SET) {
                    if (*s)
                        .default_mode
                        .intersects(mode_flag::MODE_CURSOR_BLINKING)
                    {
                        cmode |= mode_flag::MODE_CURSOR_BLINKING;
                    } else {
                        cmode &= !mode_flag::MODE_CURSOR_BLINKING;
                    }
                }
                cstyle = (*s).default_cstyle;
            }
        }

        // If nothing changed, do nothing.
        let changed = cmode ^ (*tty).mode;
        if !changed.intersects(CURSOR_MODES) && cstyle == (*tty).cstyle {
            return cmode;
        }

        // Set cursor style. If an explicit style has been set with DECSCUSR,
        // set it if supported, otherwise send cvvis for blinking styles.
        //
        // If no style, has been set (SCREEN_CURSOR_DEFAULT), then send cvvis
        // if either the blinking or very visible flags are set.
        tty_putcode(tty, tty_code_code::TTYC_CNORM);
        match cstyle {
            screen_cursor_style::SCREEN_CURSOR_DEFAULT => {
                if (*tty).cstyle != screen_cursor_style::SCREEN_CURSOR_DEFAULT {
                    if tty_term_has((*tty).term, tty_code_code::TTYC_SE) {
                        tty_putcode(tty, tty_code_code::TTYC_SE);
                    } else {
                        tty_putcode_i(tty, tty_code_code::TTYC_SS, 0);
                    }
                }
                if cmode.intersects(
                    mode_flag::MODE_CURSOR_BLINKING | mode_flag::MODE_CURSOR_VERY_VISIBLE,
                ) {
                    tty_putcode(tty, tty_code_code::TTYC_CVVIS);
                }
            }
            screen_cursor_style::SCREEN_CURSOR_BLOCK => {
                if tty_term_has((*tty).term, tty_code_code::TTYC_SS) {
                    if cmode.intersects(mode_flag::MODE_CURSOR_BLINKING) {
                        tty_putcode_i(tty, tty_code_code::TTYC_SS, 1);
                    } else {
                        tty_putcode_i(tty, tty_code_code::TTYC_SS, 2);
                    }
                } else if cmode.intersects(mode_flag::MODE_CURSOR_BLINKING) {
                    tty_putcode(tty, tty_code_code::TTYC_CVVIS);
                }
            }
            screen_cursor_style::SCREEN_CURSOR_UNDERLINE => {
                if tty_term_has((*tty).term, tty_code_code::TTYC_SS) {
                    if cmode.intersects(mode_flag::MODE_CURSOR_BLINKING) {
                        tty_putcode_i(tty, tty_code_code::TTYC_SS, 3);
                    } else {
                        tty_putcode_i(tty, tty_code_code::TTYC_SS, 4);
                    }
                } else if cmode.intersects(mode_flag::MODE_CURSOR_BLINKING) {
                    tty_putcode(tty, tty_code_code::TTYC_CVVIS);
                }
            }
            screen_cursor_style::SCREEN_CURSOR_BAR => {
                if tty_term_has((*tty).term, tty_code_code::TTYC_SS) {
                    if cmode.intersects(mode_flag::MODE_CURSOR_BLINKING) {
                        tty_putcode_i(tty, tty_code_code::TTYC_SS, 5);
                    } else {
                        tty_putcode_i(tty, tty_code_code::TTYC_SS, 6);
                    }
                } else if cmode.intersects(mode_flag::MODE_CURSOR_BLINKING) {
                    tty_putcode(tty, tty_code_code::TTYC_CVVIS);
                }
            }
        }
        (*tty).cstyle = cstyle;
        cmode
    }
}

pub unsafe fn tty_update_mode(tty: *mut tty, mut mode: mode_flag, s: *mut screen) {
    unsafe {
        let term = (*tty).term;
        let c = (*tty).client;

        if (*tty).flags.intersects(tty_flags::TTY_NOCURSOR) {
            mode &= !mode_flag::MODE_CURSOR;
        }

        if tty_update_cursor(tty, mode, s).intersects(mode_flag::MODE_CURSOR_BLINKING) {
            mode |= mode_flag::MODE_CURSOR_BLINKING;
        } else {
            mode &= !mode_flag::MODE_CURSOR_BLINKING;
        }

        let changed = mode ^ (*tty).mode;
        if log_get_level() != 0 && changed.bits() != 0 {
            log_debug!(
                "{}: current mode {}",
                _s((*c).name),
                _s(screen_mode_to_string((*tty).mode)),
            );
            log_debug!(
                "{}: setting mode {}",
                _s((*c).name),
                _s(screen_mode_to_string(mode)),
            );
        }

        if changed.intersects(ALL_MOUSE_MODES) && tty_term_has(term, tty_code_code::TTYC_KMOUS) {
            // If the mouse modes have changed, clear then all and apply
            // again. There are differences in how terminals track the
            // various bits.
            tty_puts(tty, c!("\x1b[?1006l\x1b[?1000l\x1b[?1002l\x1b[?1003l"));
            if mode.intersects(ALL_MOUSE_MODES) {
                tty_puts(tty, c!("\x1b[?1006h"));
            }
            if mode.intersects(mode_flag::MODE_MOUSE_ALL) {
                tty_puts(tty, c!("\x1b[?1000h\x1b[?1002h\x1b[?1003h"));
            } else if mode.intersects(mode_flag::MODE_MOUSE_BUTTON) {
                tty_puts(tty, c!("\x1b[?1000h\x1b[?1002h"));
            } else if mode.intersects(mode_flag::MODE_MOUSE_STANDARD) {
                tty_puts(tty, c!("\x1b[?1000h"));
            }
        }
        (*tty).mode = mode;
    }
}

pub unsafe fn tty_emulate_repeat(
    tty: *mut tty,
    code: tty_code_code,
    code1: tty_code_code,
    mut n: u32,
) {
    unsafe {
        if tty_term_has((*tty).term, code) {
            tty_putcode_i(tty, code, n as i32);
        } else {
            while {
                n -= 1;
                n > 0
            } {
                tty_putcode(tty, code1);
            }
        }
    }
}

pub unsafe fn tty_repeat_space(tty: *mut tty, mut n: u32) {
    const SIZEOF_S: usize = 500;
    static mut S: [u8; SIZEOF_S] = [0; SIZEOF_S];

    unsafe {
        if S[0] != b' ' {
            libc::memset((&raw mut S).cast(), ' ' as i32, SIZEOF_S);
        }

        while n > SIZEOF_S as u32 {
            tty_putn(tty, (&raw mut S).cast(), SIZEOF_S, SIZEOF_S as u32);
            n -= SIZEOF_S as u32;
        }
        if n != 0 {
            tty_putn(tty, (&raw mut S).cast(), n as usize, n);
        }
    }
}

/// Is this window bigger than the terminal?
pub unsafe fn tty_window_bigger(tty: *mut tty) -> bool {
    unsafe {
        let c = (*tty).client;
        let w = (*(*(*c).session).curw).window;

        (*tty).sx < (*w).sx || (*tty).sy - status_line_size(c) < (*w).sy
    }
}

/// What offset should this window be drawn at?
pub unsafe fn tty_window_offset(
    tty: *mut tty,
    ox: *mut u32,
    oy: *mut u32,
    sx: *mut u32,
    sy: *mut u32,
) -> i32 {
    unsafe {
        *ox = (*tty).oox;
        *oy = (*tty).ooy;
        *sx = (*tty).osx;
        *sy = (*tty).osy;

        (*tty).oflag
    }
}

/// What offset should this window be drawn at?
pub unsafe fn tty_window_offset1(
    tty: *mut tty,
    ox: *mut u32,
    oy: *mut u32,
    sx: *mut u32,
    sy: *mut u32,
) -> i32 {
    unsafe {
        let c = (*tty).client;
        let w = (*(*(*c).session).curw).window;
        let wp = server_client_get_pane(c);
        let cx: u32;
        let cy: u32;

        let lines: u32 = status_line_size(c);

        if (*tty).sx >= (*w).sx && (*tty).sy - lines >= (*w).sy {
            *ox = 0;
            *oy = 0;
            *sx = (*w).sx;
            *sy = (*w).sy;

            (*c).pan_window = null_mut();
            return 0;
        }

        *sx = (*tty).sx;
        *sy = (*tty).sy - lines;

        if (*c).pan_window.cast() == w {
            if *sx >= (*w).sx {
                (*c).pan_ox = 0;
            } else if (*c).pan_ox + *sx > (*w).sx {
                (*c).pan_ox = (*w).sx - *sx;
            }
            *ox = (*c).pan_ox;
            if *sy >= (*w).sy {
                (*c).pan_oy = 0;
            } else if (*c).pan_oy + *sy > (*w).sy {
                (*c).pan_oy = (*w).sy - *sy;
            }
            *oy = (*c).pan_oy;
            return 1;
        }

        if !(*(*wp).screen).mode.intersects(mode_flag::MODE_CURSOR) {
            *ox = 0;
            *oy = 0;
        } else {
            cx = (*wp).xoff + (*(*wp).screen).cx;
            cy = (*wp).yoff + (*(*wp).screen).cy;

            if cx < *sx {
                *ox = 0;
            } else if cx > (*w).sx - *sx {
                *ox = (*w).sx - *sx;
            } else {
                *ox = cx - *sx / 2;
            }

            if cy < *sy {
                *oy = 0;
            } else if cy > (*w).sy - *sy {
                *oy = (*w).sy - *sy;
            } else {
                *oy = cy - *sy / 2;
            }
        }

        (*c).pan_window = null_mut();
        1
    }
}

/// Update stored offsets for a window and redraw if necessary.
pub unsafe fn tty_update_window_offset(w: *mut window) {
    unsafe {
        for c in tailq_foreach(&raw mut CLIENTS).map(NonNull::as_ptr) {
            if !(*c).session.is_null()
                && !(*(*c).session).curw.is_null()
                && (*(*(*c).session).curw).window == w
            {
                tty_update_client_offset(c);
            }
        }
    }
}

/// Update stored offsets for a client and redraw if necessary.
pub unsafe fn tty_update_client_offset(c: *mut client) {
    unsafe {
        let mut ox: u32 = 0;
        let mut oy: u32 = 0;
        let mut sx: u32 = 0;
        let mut sy: u32 = 0;

        if !(*c).flags.intersects(client_flag::TERMINAL) {
            return;
        }

        (*c).tty.oflag = tty_window_offset1(
            &raw mut (*c).tty,
            &raw mut ox,
            &raw mut oy,
            &raw mut sx,
            &raw mut sy,
        );
        if ox == (*c).tty.oox && oy == (*c).tty.ooy && sx == (*c).tty.osx && sy == (*c).tty.osy {
            return;
        }

        log_debug!(
            "{}: {} offset has changed ({},{} {}x{} -> {},{} {}x{})",
            "tty_update_client_offset",
            _s((*c).name),
            (*c).tty.oox,
            (*c).tty.ooy,
            (*c).tty.osx,
            (*c).tty.osy,
            ox,
            oy,
            sx,
            sy,
        );

        (*c).tty.oox = ox;
        (*c).tty.ooy = oy;
        (*c).tty.osx = sx;
        (*c).tty.osy = sy;

        (*c).flags |= client_flag::REDRAWWINDOW | client_flag::REDRAWSTATUS;
    }
}

/// Is the region large enough to be worth redrawing once later rather than
/// probably several times now? Currently yes if it is more than 50% of the
/// pane.
pub unsafe fn tty_large_region(_tty: *mut tty, ctx: *const tty_ctx) -> bool {
    unsafe { (*ctx).orlower - (*ctx).orupper >= (*ctx).sy / 2 }
}

/// Return if BCE is needed but the terminal doesn't have it - it'll need to be emulated.
pub unsafe fn tty_fake_bce(tty: *const tty, gc: *const grid_cell, bg: u32) -> bool {
    unsafe {
        if tty_term_flag((*tty).term, tty_code_code::TTYC_BCE) != 0 {
            false
        } else {
            !COLOUR_DEFAULT(bg as i32) || !COLOUR_DEFAULT((*gc).bg)
        }
    }
}

/// Redraw scroll region using data from screen (already updated).
///
/// Used when CSR not supported, or window is a pane that doesn't
/// take up the full width of the terminal.
pub unsafe fn tty_redraw_region(tty: *mut tty, ctx: *const tty_ctx) {
    unsafe {
        let c = (*tty).client;

        // If region is large, schedule a redraw.
        // In most cases this is likely to be followed by some more scrolling.
        if tty_large_region(tty, ctx) {
            log_debug!("tty_redraw_region: {} large redraw", _s((*c).name));
            (*ctx).redraw_cb.unwrap()(ctx);
            return;
        }

        for i in (*ctx).orupper..=(*ctx).orlower {
            tty_draw_pane(tty, ctx, i);
        }
    }
}

/// Is this position visible in the pane?
#[expect(clippy::needless_bool)]
pub unsafe fn tty_is_visible(
    _tty: *mut tty,
    ctx: *const tty_ctx,
    px: u32,
    py: u32,
    nx: u32,
    ny: u32,
) -> bool {
    unsafe {
        let xoff = (*ctx).rxoff + px;
        let yoff = (*ctx).ryoff + py;

        if (*ctx).bigger == 0 {
            true
        } else if xoff + nx <= (*ctx).wox
            || xoff >= (*ctx).wox + (*ctx).wsx
            || yoff + ny <= (*ctx).woy
            || yoff >= (*ctx).woy + (*ctx).wsy
        {
            false
        } else {
            true
        }
    }
}

/// Clamp line position to visible part of pane.
pub unsafe fn tty_clamp_line(
    tty: *mut tty,
    ctx: *const tty_ctx,
    px: u32,
    py: u32,
    nx: u32,
    i: *mut u32,
    x: *mut u32,
    rx: *mut u32,
    ry: *mut u32,
) -> bool {
    unsafe {
        let xoff = (*ctx).rxoff + px;

        if !tty_is_visible(tty, ctx, px, py, nx, 1) {
            return false;
        }
        *ry = (*ctx).yoff + py - (*ctx).woy;

        if xoff >= (*ctx).wox && xoff + nx <= (*ctx).wox + (*ctx).wsx {
            // All visible.
            *i = 0;
            *x = (*ctx).xoff + px - (*ctx).wox;
            *rx = nx;
        } else if xoff < (*ctx).wox && xoff + nx > (*ctx).wox + (*ctx).wsx {
            // Both left and right not visible.
            *i = (*ctx).wox;
            *x = 0;
            *rx = (*ctx).wsx;
        } else if xoff < (*ctx).wox {
            // Left not visible.
            *i = (*ctx).wox - ((*ctx).xoff + px);
            *x = 0;
            *rx = nx - *i;
        } else {
            // Right not visible.
            *i = 0;
            *x = ((*ctx).xoff + px) - (*ctx).wox;
            *rx = (*ctx).wsx - *x;
        }
        if *rx > nx {
            panic!("tty_clamp_line: x too big, {} > {}", *rx, nx);
        }

        true
    }
}

/// Clear a line.
pub unsafe fn tty_clear_line(
    tty: *mut tty,
    defaults: *const grid_cell,
    py: u32,
    px: u32,
    nx: u32,
    bg: u32,
) {
    unsafe {
        let c = (*tty).client;
        let mut r: overlay_ranges = zeroed();
        // struct overlay_ranges r;
        // u_int i;

        // log_debug("%s: %s, %u at %u,%u", __func__, (*c).name, nx, px, py);

        // Nothing to clear.
        if nx == 0 {
            return;
        }

        // If genuine BCE is available, can try escape sequences.
        if (*c).overlay_check.is_none() && !tty_fake_bce(tty, defaults, bg) {
            // Off the end of the line, use EL if available.
            if px + nx >= (*tty).sx && tty_term_has((*tty).term, tty_code_code::TTYC_EL) {
                tty_cursor(tty, px, py);
                tty_putcode(tty, tty_code_code::TTYC_EL);
                return;
            }

            // At the start of the line. Use EL1.
            if px == 0 && tty_term_has((*tty).term, tty_code_code::TTYC_EL1) {
                tty_cursor(tty, px + nx - 1, py);
                tty_putcode(tty, tty_code_code::TTYC_EL1);
                return;
            }

            // Section of line. Use ECH if possible.
            if tty_term_has((*tty).term, tty_code_code::TTYC_ECH) {
                tty_cursor(tty, px, py);
                tty_putcode_i(tty, tty_code_code::TTYC_ECH, nx as i32);
                return;
            }
        }

        // Couldn't use an escape sequence, use spaces. Clear only the visible
        // bit if there is an overlay.
        tty_check_overlay_range(tty, px, py, nx, &raw mut r);
        for i in 0..OVERLAY_MAX_RANGES {
            if r.nx[i] == 0 {
                continue;
            }
            tty_cursor(tty, r.px[i], py);
            tty_repeat_space(tty, r.nx[i]);
        }
    }
}

/// Clear a line, adjusting to visible part of pane.
pub unsafe fn tty_clear_pane_line(
    tty: *mut tty,
    ctx: *const tty_ctx,
    py: u32,
    px: u32,
    nx: u32,
    bg: u32,
) {
    unsafe {
        let c = (*tty).client;

        let mut i = 0;
        let mut x = 0;
        let mut rx = 0;
        let mut ry = 0;

        log_debug!(
            "tty_clear_pane_line: {}, {} at {},{}",
            _s((*c).name),
            nx,
            px,
            py
        );

        if tty_clamp_line(
            tty,
            ctx,
            px,
            py,
            nx,
            &raw mut i,
            &raw mut x,
            &raw mut rx,
            &raw mut ry,
        ) {
            tty_clear_line(tty, &raw const (*ctx).defaults, ry, x, rx, bg);
        }
    }
}

/// Clamp area position to visible part of pane.
pub unsafe fn tty_clamp_area(
    tty: *mut tty,
    ctx: *const tty_ctx,
    px: u32,
    py: u32,
    nx: u32,
    ny: u32,
    i: *mut u32,
    j: *mut u32,
    x: *mut u32,
    y: *mut u32,
    rx: *mut u32,
    ry: *mut u32,
) -> bool {
    unsafe {
        let xoff = (*ctx).rxoff + px;
        let yoff = (*ctx).ryoff + py;

        if !tty_is_visible(tty, ctx, px, py, nx, ny) {
            return false;
        }

        if xoff >= (*ctx).wox && xoff + nx <= (*ctx).wox + (*ctx).wsx {
            // All visible.
            *i = 0;
            *x = (*ctx).xoff + px - (*ctx).wox;
            *rx = nx;
        } else if xoff < (*ctx).wox && xoff + nx > (*ctx).wox + (*ctx).wsx {
            // Both left and right not visible.
            *i = (*ctx).wox;
            *x = 0;
            *rx = (*ctx).wsx;
        } else if xoff < (*ctx).wox {
            // Left not visible.
            *i = (*ctx).wox - ((*ctx).xoff + px);
            *x = 0;
            *rx = nx - *i;
        } else {
            // Right not visible.
            *i = 0;
            *x = ((*ctx).xoff + px) - (*ctx).wox;
            *rx = (*ctx).wsx - *x;
        }
        if *rx > nx {
            panic!("tty_clamp_area: x too big, {} > {}", *rx, nx);
        }

        if yoff >= (*ctx).woy && yoff + ny <= (*ctx).woy + (*ctx).wsy {
            // All visible.
            *j = 0;
            *y = (*ctx).yoff + py - (*ctx).woy;
            *ry = ny;
        } else if yoff < (*ctx).woy && yoff + ny > (*ctx).woy + (*ctx).wsy {
            // Both top and bottom not visible.
            *j = (*ctx).woy;
            *y = 0;
            *ry = (*ctx).wsy;
        } else if yoff < (*ctx).woy {
            // Top not visible.
            *j = (*ctx).woy - ((*ctx).yoff + py);
            *y = 0;
            *ry = ny - *j;
        } else {
            // Bottom not visible.
            *j = 0;
            *y = ((*ctx).yoff + py) - (*ctx).woy;
            *ry = (*ctx).wsy - *y;
        }
        if *ry > ny {
            panic!("tty_clamp_area: y too big, {} > {}", *ry, ny);
        }

        true
    }
}

/// Clear an area, adjusting to visible part of pane.
pub unsafe fn tty_clear_area(
    tty: *mut tty,
    defaults: *const grid_cell,
    py: u32,
    ny: u32,
    px: u32,
    nx: u32,
    bg: u32,
) {
    unsafe {
        let c = (*tty).client;
        const SIZEOF_TMP: usize = 64;
        let mut tmp: [u8; SIZEOF_TMP] = [0; SIZEOF_TMP];

        // log_debug("%s: %s, %u,%u at %u,%u", __func__, (*c).name, nx, ny, px, py);

        // Nothing to clear.
        if nx == 0 || ny == 0 {
            return;
        }

        // If genuine BCE is available, can try escape sequences.
        if (*c).overlay_check.is_none() && !tty_fake_bce(tty, defaults, bg) {
            // Use ED if clearing off the bottom of the terminal.
            if px == 0
                && px + nx >= (*tty).sx
                && py + ny >= (*tty).sy
                && tty_term_has((*tty).term, tty_code_code::TTYC_ED)
            {
                tty_cursor(tty, 0, py);
                tty_putcode(tty, tty_code_code::TTYC_ED);
                return;
            }

            // On VT420 compatible terminals we can use DECFRA if the
            // background colour isn't default (because it doesn't work
            // after SGR 0).
            if (*(*tty).term).flags.intersects(term_flags::TERM_DECFRA)
                && !COLOUR_DEFAULT(bg as i32)
            {
                _ = xsnprintf_!(
                    (&raw mut tmp).cast(),
                    SIZEOF_TMP,
                    "\x1b[32;{};{};{};{}$x",
                    py + 1,
                    px + 1,
                    py + ny,
                    px + nx,
                );
                tty_puts(tty, (&raw const tmp).cast());
                return;
            }

            // Full lines can be scrolled away to clear them.
            if px == 0
                && px + nx >= (*tty).sx
                && ny > 2
                && tty_term_has((*tty).term, tty_code_code::TTYC_CSR)
                && tty_term_has((*tty).term, tty_code_code::TTYC_INDN)
            {
                tty_region(tty, py, py + ny - 1);
                tty_margin_off(tty);
                tty_putcode_i(tty, tty_code_code::TTYC_INDN, ny as i32);
                return;
            }

            // If margins are supported, can just scroll the area off to
            // clear it.
            if nx > 2
                && ny > 2
                && tty_term_has((*tty).term, tty_code_code::TTYC_CSR)
                && tty_use_margin(tty)
                && tty_term_has((*tty).term, tty_code_code::TTYC_INDN)
            {
                tty_region(tty, py, py + ny - 1);
                tty_margin(tty, px, px + nx - 1);
                tty_putcode_i(tty, tty_code_code::TTYC_INDN, ny as i32);
                return;
            }
        }

        // Couldn't use an escape sequence, loop over the lines.
        for yy in py..(py + ny) {
            tty_clear_line(tty, defaults, yy, px, nx, bg);
        }
    }
}

/// Clear an area in a pane.
pub unsafe fn tty_clear_pane_area(
    tty: *mut tty,
    ctx: *const tty_ctx,
    py: u32,
    ny: u32,
    px: u32,
    nx: u32,
    bg: u32,
) {
    unsafe {
        let mut i: u32 = 0;
        let mut j: u32 = 0;
        let mut x: u32 = 0;
        let mut y: u32 = 0;
        let mut rx: u32 = 0;
        let mut ry: u32 = 0;

        if tty_clamp_area(
            tty,
            ctx,
            px,
            py,
            nx,
            ny,
            &raw mut i,
            &raw mut j,
            &raw mut x,
            &raw mut y,
            &raw mut rx,
            &raw mut ry,
        ) {
            tty_clear_area(tty, &raw const (*ctx).defaults, y, ry, x, rx, bg);
        }
    }
}

pub unsafe fn tty_draw_pane(tty: *mut tty, ctx: *const tty_ctx, py: u32) {
    unsafe {
        let s = (*ctx).s;
        let nx = (*ctx).sx;
        let mut i: u32 = 0;
        let mut x: u32 = 0;
        let mut rx: u32 = 0;
        let mut ry: u32 = 0;

        // log_debug("%s: %s %u %d", __func__, (*(*tty).client).name, py, (*ctx).bigger);

        if (*ctx).bigger == 0 {
            tty_draw_line(
                tty,
                s,
                0,
                py,
                nx,
                (*ctx).xoff,
                (*ctx).yoff + py,
                &raw const (*ctx).defaults,
                (*ctx).palette,
            );
            return;
        }
        if tty_clamp_line(
            tty,
            ctx,
            0,
            py,
            nx,
            &raw mut i,
            &raw mut x,
            &raw mut rx,
            &raw mut ry,
        ) {
            tty_draw_line(
                tty,
                s,
                i,
                py,
                rx,
                x,
                ry,
                &raw const (*ctx).defaults,
                (*ctx).palette,
            );
        }
    }
}

pub unsafe fn tty_check_codeset(tty: *mut tty, gc: *const grid_cell) -> *const grid_cell {
    static mut NEW: grid_cell = unsafe { zeroed() };
    unsafe {
        // Characters less than 0x7f are always fine, no matter what.
        if (*gc).data.size == 1 && (*gc).data.data[0] < 0x7f {
            return gc;
        }

        // UTF-8 terminal and a UTF-8 character - fine.
        if (*(*tty).client).flags.intersects(client_flag::UTF8) {
            return gc;
        }
        memcpy__(&raw mut NEW, gc);

        // See if this can be mapped to an ACS character.
        let c = tty_acs_reverse_get(
            tty,
            (&raw const (*gc).data.data).cast(),
            (*gc).data.size as usize,
        );
        if c != -1 {
            utf8_set(&raw mut NEW.data, c as u8);
            NEW.attr |= grid_attr::GRID_ATTR_CHARSET;
            return &raw const NEW;
        }

        // Replace by the right number of underscores.
        NEW.data.size = (*gc).data.width;
        if NEW.data.size > UTF8_SIZE as u8 {
            NEW.data.size = UTF8_SIZE as u8;
        }
        libc::memset(
            (&raw mut NEW.data.data).cast(),
            b'_' as i32,
            NEW.data.size as usize,
        );
        &raw const NEW
    }
}

/// Check if a single character is obstructed by the overlay and return a boolean.
pub unsafe fn tty_check_overlay(tty: *mut tty, px: u32, py: u32) -> bool {
    unsafe {
        let mut r: overlay_ranges = zeroed();

        // A unit width range will always return nx[2] == 0 from a check, even
        // with multiple overlays, so it's sufficient to check just the first
        // two entries.
        tty_check_overlay_range(tty, px, py, 1, &raw mut r);
        r.nx[0] + r.nx[1] != 0
    }
}

/// Return parts of the input range which are visible.
pub unsafe fn tty_check_overlay_range(
    tty: *mut tty,
    px: u32,
    py: u32,
    nx: u32,
    r: *mut overlay_ranges,
) {
    unsafe {
        let c = (*tty).client;

        if let Some(overlay_check) = (*c).overlay_check {
            overlay_check(c, (*c).overlay_data, px, py, nx, r);
        } else {
            (*r).px[0] = px;
            (*r).nx[0] = nx;
            (*r).px[1] = 0;
            (*r).nx[1] = 0;
            (*r).px[2] = 0;
            (*r).nx[2] = 0;
        }
    }
}

pub unsafe fn tty_draw_line(
    tty: *mut tty,
    s: *mut screen,
    px: u32,
    py: u32,
    mut nx: u32,
    atx: u32,
    aty: u32,
    defaults: *const grid_cell,
    palette: *const colour_palette,
) {
    unsafe {
        let gd = (*s).grid;
        let mut gc: grid_cell = zeroed();
        let mut last: grid_cell = zeroed();
        let c = (*tty).client;

        let mut r: overlay_ranges = zeroed();
        let mut cleared = 0;
        let mut wrapped = false;
        const SIZEOF_BUF: usize = 512;
        let mut buf: [u8; SIZEOF_BUF] = [0; SIZEOF_BUF];

        // log_debug("%s: px=%u py=%u nx=%u atx=%u aty=%u", __func__, px, py, nx, atx, aty);
        // log_debug("%s: defaults: fg=%d, bg=%d", __func__, (*defaults).fg, (*defaults).bg);

        // py is the line in the screen to draw.
        // px is the start x and nx is the width to draw.
        // atx,aty is the line on the terminal to draw it.

        let flags = (*tty).flags & tty_flags::TTY_NOCURSOR;
        (*tty).flags |= tty_flags::TTY_NOCURSOR;
        tty_update_mode(tty, (*tty).mode, s);

        tty_region_off(tty);
        tty_margin_off(tty);

        // Clamp the width to cellsize - note this is not cellused, because
        // there may be empty background cells after it (from BCE).
        let mut sx = screen_size_x(s);
        if nx > sx {
            nx = sx;
        }

        let cellsize = (*grid_get_line(gd, (*gd).hsize + py)).cellsize;
        if sx > cellsize {
            sx = cellsize;
        }
        if sx > (*tty).sx {
            sx = (*tty).sx;
        }
        if sx > nx {
            sx = nx;
        }
        let mut ux = 0;

        let gl = if py == 0 {
            null_mut()
        } else {
            grid_get_line(gd, (*gd).hsize + py - 1)
        };
        if gl.is_null()
            || !(*gl).flags.intersects(grid_line_flag::WRAPPED)
            || atx != 0
            || (*tty).cx < (*tty).sx
            || nx < (*tty).sx
        {
            if nx < (*tty).sx
                && atx == 0
                && px + sx != nx
                && tty_term_has((*tty).term, tty_code_code::TTYC_EL1)
                && !tty_fake_bce(tty, defaults, 8)
                && (*c).overlay_check.is_none()
            {
                tty_default_attributes(tty, defaults, palette, 8, (*s).hyperlinks);
                tty_cursor(tty, nx - 1, aty);
                tty_putcode(tty, tty_code_code::TTYC_EL1);
                cleared = 1;
            }
        } else {
            // log_debug("%s: wrapped line %u", __func__, aty);
            wrapped = true;
        }

        memcpy__(&raw mut last, &raw const GRID_DEFAULT_CELL);
        let mut len = 0;
        let mut width = 0;

        for i in 0..sx {
            grid_view_get_cell(gd, px + i, py, &raw mut gc);
            let gcp = tty_check_codeset(tty, &gc);
            if len != 0
                && (!tty_check_overlay(tty, atx + ux + width, aty)
                    || (*gcp).attr.intersects(grid_attr::GRID_ATTR_CHARSET)
                    || (*gcp).flags != last.flags
                    || (*gcp).attr != last.attr
                    || (*gcp).fg != last.fg
                    || (*gcp).bg != last.bg
                    || (*gcp).us != last.us
                    || (*gcp).link != last.link
                    || ux + width + (*gcp).data.width as u32 > nx
                    || (SIZEOF_BUF) - len < (*gcp).data.size as usize)
            {
                tty_attributes(tty, &last, defaults, palette, (*s).hyperlinks);
                if last.flags.intersects(grid_flag::CLEARED) {
                    // log_debug("%s: %zu cleared", __func__, len);
                    tty_clear_line(tty, defaults, aty, atx + ux, width, last.bg as u32);
                } else {
                    if !wrapped || atx != 0 || ux != 0 {
                        tty_cursor(tty, atx + ux, aty);
                    }
                    tty_putn(tty, (&raw const buf).cast(), len, width);
                }
                ux += width;

                len = 0;
                width = 0;
                wrapped = false;
            }

            if (*gcp).flags.intersects(grid_flag::SELECTED) {
                screen_select_cell(s, &raw mut last, gcp);
            } else {
                memcpy__(&raw mut last, gcp);
            }

            tty_check_overlay_range(tty, atx + ux, aty, (*gcp).data.width as u32, &raw mut r);
            let mut hidden = 0;
            for j in 0..OVERLAY_MAX_RANGES {
                hidden += r.nx[j];
            }
            hidden = (*gcp).data.width as u32 - hidden;
            if hidden != 0 && hidden == (*gcp).data.width as u32 {
                if !(*gcp).flags.intersects(grid_flag::PADDING) {
                    ux += (*gcp).data.width as u32;
                }
            } else if hidden != 0 || ux + (*gcp).data.width as u32 > nx {
                if !(*gcp).flags.intersects(grid_flag::PADDING) {
                    tty_attributes(tty, &raw mut last, defaults, palette, (*s).hyperlinks);
                    for j in 0..OVERLAY_MAX_RANGES {
                        if r.nx[j] == 0 {
                            continue;
                        }
                        // Effective width drawn so far.
                        let eux = r.px[j] - atx;
                        if eux < nx {
                            tty_cursor(tty, r.px[j], aty);
                            let nxx = nx - eux;
                            if r.nx[j] > nxx {
                                r.nx[j] = nxx;
                            }
                            tty_repeat_space(tty, r.nx[j]);
                            ux = eux + r.nx[j];
                        }
                    }
                }
            } else if (*gcp).attr.intersects(grid_attr::GRID_ATTR_CHARSET) {
                tty_attributes(tty, &raw mut last, defaults, palette, (*s).hyperlinks);
                tty_cursor(tty, atx + ux, aty);
                for j in 0..(*gcp).data.size {
                    tty_putc(tty, (*gcp).data.data[j as usize]);
                }
                ux += (*gcp).data.width as u32;
            } else if !(*gcp).flags.intersects(grid_flag::PADDING) {
                libc::memcpy(
                    (&raw mut buf as *mut i8).add(len).cast(),
                    (&raw const (*gcp).data.data).cast(),
                    (*gcp).data.size as usize,
                );
                len += (*gcp).data.size as usize;
                width += (*gcp).data.width as u32;
            }
        }
        if len != 0 && ((!last.flags.intersects(grid_flag::CLEARED)) || last.bg != 8) {
            tty_attributes(tty, &raw mut last, defaults, palette, (*s).hyperlinks);
            if last.flags.intersects(grid_flag::CLEARED) {
                // log_debug("%s: %zu cleared (end)", __func__, len);
                tty_clear_line(tty, defaults, aty, atx + ux, width, last.bg as u32);
            } else {
                if !wrapped || atx != 0 || ux != 0 {
                    tty_cursor(tty, atx + ux, aty);
                }
                tty_putn(tty, (&raw const buf).cast(), len, width);
            }
            ux += width;
        }

        if cleared == 0 && ux < nx {
            // log_debug( "%s: %u to end of line (%zu cleared)", __func__, nx - ux, len,);
            tty_default_attributes(tty, defaults, palette, 8, (*s).hyperlinks);
            tty_clear_line(tty, defaults, aty, atx + ux, nx - ux, 8);
        }

        (*tty).flags = ((*tty).flags & !tty_flags::TTY_NOCURSOR) | flags;
        tty_update_mode(tty, (*tty).mode, s);
    }
}

/// Update context for client.
#[cfg(feature = "sixel")]
pub unsafe fn tty_set_client_cb(ttyctx: *mut tty_ctx, c: *mut client) -> i32 {
    unsafe {
        let wp: *mut window_pane = (*ttyctx).arg.cast();

        if (*(*(*c).session).curw).window != (*wp).window {
            return 0;
        }
        if (*wp).layout_cell.is_null() {
            return 0;
        }

        // Set the properties relevant to the current client.
        (*ttyctx).bigger = tty_window_offset(
            &raw mut (*c).tty,
            &raw mut (*ttyctx).wox,
            &raw mut (*ttyctx).woy,
            &raw mut (*ttyctx).wsx,
            &raw mut (*ttyctx).wsy,
        );

        (*ttyctx).yoff = (*wp).yoff;
        (*ttyctx).ryoff = (*wp).yoff;
        if status_at_line(c) == 0 {
            (*ttyctx).yoff += status_line_size(c);
        }

        1
    }
}

#[cfg(feature = "sixel")]
pub unsafe fn tty_draw_images(c: *mut client, wp: *mut window_pane, s: *mut screen) {
    unsafe {
        for im in tailq_foreach::<image, discr_entry>(&raw mut (*s).images).map(NonNull::as_ptr) {
            let mut ttyctx: tty_ctx = zeroed();
            memset0(&raw mut ttyctx);

            // Set the client independent properties.
            ttyctx.ocx = (*im).px;
            ttyctx.ocy = (*im).py;

            ttyctx.orlower = (*s).rlower;
            ttyctx.orupper = (*s).rupper;

            ttyctx.xoff = (*wp).xoff;
            ttyctx.rxoff = (*wp).xoff;
            ttyctx.sx = (*wp).sx;
            ttyctx.sy = (*wp).sy;

            ttyctx.ptr = im.cast();
            ttyctx.arg = wp.cast();
            ttyctx.set_client_cb = Some(tty_set_client_cb);
            ttyctx.allow_invisible_panes = 1;
            tty_write_one(tty_cmd_sixelimage, c, &raw mut ttyctx);
        }
    }
}

pub unsafe fn tty_sync_start(tty: *mut tty) {
    unsafe {
        if (*tty).flags.intersects(tty_flags::TTY_BLOCK) {
            return;
        }
        if (*tty).flags.intersects(tty_flags::TTY_SYNCING) {
            return;
        }
        (*tty).flags |= tty_flags::TTY_SYNCING;

        if tty_term_has((*tty).term, tty_code_code::TTYC_SYNC) {
            // log_debug("%s sync start", (*(*tty).client).name);
            tty_putcode_i(tty, tty_code_code::TTYC_SYNC, 1);
        }
    }
}

pub unsafe fn tty_sync_end(tty: *mut tty) {
    unsafe {
        if (*tty).flags.intersects(tty_flags::TTY_BLOCK) {
            return;
        }
        if !(*tty).flags.intersects(tty_flags::TTY_SYNCING) {
            return;
        }
        (*tty).flags &= !tty_flags::TTY_SYNCING;

        if tty_term_has((*tty).term, tty_code_code::TTYC_SYNC) {
            // log_debug("%s sync end", (*(*tty).client).name);
            tty_putcode_i(tty, tty_code_code::TTYC_SYNC, 2);
        }
    }
}

pub unsafe fn tty_client_ready(ctx: *const tty_ctx, c: *mut client) -> i32 {
    unsafe {
        if (*c).session.is_null() || (*c).tty.term.is_null() {
            return 0;
        }
        if (*c).flags.intersects(client_flag::SUSPENDED) {
            return 0;
        }

        // If invisible panes are allowed (used for passthrough), don't care if
        // redrawing or frozen.
        if (*ctx).allow_invisible_panes != 0 {
            return 1;
        }

        if (*c).flags.intersects(client_flag::REDRAWWINDOW) {
            return 0;
        }
        if (*c).tty.flags.intersects(tty_flags::TTY_FREEZE) {
            return 0;
        }
        1
    }
}

pub unsafe fn tty_write(cmdfn: unsafe fn(*mut tty, *const tty_ctx), ctx: *mut tty_ctx) {
    unsafe {
        let Some(set_client_cb) = (*ctx).set_client_cb else {
            return;
        };

        for c in tailq_foreach(&raw mut CLIENTS).map(NonNull::as_ptr) {
            if tty_client_ready(ctx, c) != 0 {
                let state = set_client_cb(ctx, c);
                if state == -1 {
                    break;
                }
                if state == 0 {
                    continue;
                }
                cmdfn(&raw mut (*c).tty, ctx);
            }
        }
    }
}

/// Only write to the incoming tty instead of every client.
#[cfg(feature = "sixel")]
pub unsafe fn tty_write_one(
    cmdfn: unsafe fn(*mut tty, *const tty_ctx),
    c: *mut client,
    ctx: *mut tty_ctx,
) {
    unsafe {
        let Some(set_client_cb) = (*ctx).set_client_cb else {
            return;
        };
        if set_client_cb(ctx, c) == 1 {
            cmdfn(&raw mut (*c).tty, ctx);
        }
    }
}

pub unsafe fn tty_cmd_insertcharacter(tty: *mut tty, ctx: *const tty_ctx) {
    unsafe {
        let c = (*tty).client;

        if (*ctx).bigger != 0
            || !tty_full_width(tty, ctx)
            || tty_fake_bce(tty, &(*ctx).defaults, (*ctx).bg)
            || (!tty_term_has((*tty).term, tty_code_code::TTYC_ICH)
                && !tty_term_has((*tty).term, tty_code_code::TTYC_ICH1))
            || (*c).overlay_check.is_some()
        {
            tty_draw_pane(tty, ctx, (*ctx).ocy);
            return;
        }

        tty_default_attributes(
            tty,
            &raw const (*ctx).defaults,
            (*ctx).palette,
            (*ctx).bg,
            (*(*ctx).s).hyperlinks,
        );

        tty_cursor_pane(tty, ctx, (*ctx).ocx, (*ctx).ocy);

        tty_emulate_repeat(
            tty,
            tty_code_code::TTYC_ICH,
            tty_code_code::TTYC_ICH1,
            (*ctx).num,
        );
    }
}

pub unsafe fn tty_cmd_deletecharacter(tty: *mut tty, ctx: *const tty_ctx) {
    unsafe {
        let c = (*tty).client;

        if (*ctx).bigger != 0
            || !tty_full_width(tty, ctx)
            || tty_fake_bce(tty, &raw const (*ctx).defaults, (*ctx).bg)
            || (!tty_term_has((*tty).term, tty_code_code::TTYC_DCH)
                && !tty_term_has((*tty).term, tty_code_code::TTYC_DCH1))
            || (*c).overlay_check.is_some()
        {
            tty_draw_pane(tty, ctx, (*ctx).ocy);
            return;
        }

        tty_default_attributes(
            tty,
            &raw const (*ctx).defaults,
            (*ctx).palette,
            (*ctx).bg,
            (*(*ctx).s).hyperlinks,
        );

        tty_cursor_pane(tty, ctx, (*ctx).ocx, (*ctx).ocy);

        tty_emulate_repeat(
            tty,
            tty_code_code::TTYC_DCH,
            tty_code_code::TTYC_DCH1,
            (*ctx).num,
        );
    }
}

pub unsafe fn tty_cmd_clearcharacter(tty: *mut tty, ctx: *const tty_ctx) {
    unsafe {
        tty_default_attributes(
            tty,
            &raw const (*ctx).defaults,
            (*ctx).palette,
            (*ctx).bg,
            (*(*ctx).s).hyperlinks,
        );

        tty_clear_pane_line(tty, ctx, (*ctx).ocy, (*ctx).ocx, (*ctx).num, (*ctx).bg);
    }
}

pub unsafe fn tty_cmd_insertline(tty: *mut tty, ctx: *const tty_ctx) {
    unsafe {
        let c = (*tty).client;

        if (*ctx).bigger != 0
            || !tty_full_width(tty, ctx)
            || tty_fake_bce(tty, &raw const (*ctx).defaults, (*ctx).bg)
            || !tty_term_has((*tty).term, tty_code_code::TTYC_CSR)
            || !tty_term_has((*tty).term, tty_code_code::TTYC_IL1)
            || (*ctx).sx == 1
            || (*ctx).sy == 1
            || (*c).overlay_check.is_some()
        {
            tty_redraw_region(tty, ctx);
            return;
        }

        tty_default_attributes(
            tty,
            &(*ctx).defaults,
            (*ctx).palette,
            (*ctx).bg,
            (*(*ctx).s).hyperlinks,
        );

        tty_region_pane(tty, ctx, (*ctx).orupper, (*ctx).orlower);
        tty_margin_off(tty);
        tty_cursor_pane(tty, ctx, (*ctx).ocx, (*ctx).ocy);

        tty_emulate_repeat(
            tty,
            tty_code_code::TTYC_IL,
            tty_code_code::TTYC_IL1,
            (*ctx).num,
        );
        (*tty).cx = u32::MAX;
        (*tty).cy = u32::MAX;
    }
}

pub unsafe fn tty_cmd_deleteline(tty: *mut tty, ctx: *const tty_ctx) {
    unsafe {
        let c = (*tty).client;

        if (*ctx).bigger != 0
            || !tty_full_width(tty, ctx)
            || tty_fake_bce(tty, &raw const (*ctx).defaults, (*ctx).bg)
            || !tty_term_has((*tty).term, tty_code_code::TTYC_CSR)
            || !tty_term_has((*tty).term, tty_code_code::TTYC_DL1)
            || (*ctx).sx == 1
            || (*ctx).sy == 1
            || (*c).overlay_check.is_some()
        {
            tty_redraw_region(tty, ctx);
            return;
        }

        tty_default_attributes(
            tty,
            &(*ctx).defaults,
            (*ctx).palette,
            (*ctx).bg,
            (*(*ctx).s).hyperlinks,
        );

        tty_region_pane(tty, ctx, (*ctx).orupper, (*ctx).orlower);
        tty_margin_off(tty);
        tty_cursor_pane(tty, ctx, (*ctx).ocx, (*ctx).ocy);

        tty_emulate_repeat(
            tty,
            tty_code_code::TTYC_DL,
            tty_code_code::TTYC_DL1,
            (*ctx).num,
        );
        (*tty).cx = u32::MAX;
        (*tty).cy = u32::MAX;
    }
}

#[expect(dead_code)]
unsafe fn tty_cmd_clearline(tty: *mut tty, ctx: *const tty_ctx) {
    unsafe {
        tty_default_attributes(
            tty,
            &raw const (*ctx).defaults,
            (*ctx).palette,
            (*ctx).bg,
            (*(*ctx).s).hyperlinks,
        );

        tty_clear_pane_line(tty, ctx, (*ctx).ocy, 0, (*ctx).sx, (*ctx).bg);
    }
}

#[expect(dead_code)]
unsafe fn tty_cmd_clearendofline(tty: *mut tty, ctx: *const tty_ctx) {
    unsafe {
        let nx = (*ctx).sx - (*ctx).ocx;

        tty_default_attributes(
            tty,
            &raw const (*ctx).defaults,
            (*ctx).palette,
            (*ctx).bg,
            (*(*ctx).s).hyperlinks,
        );

        tty_clear_pane_line(tty, ctx, (*ctx).ocy, (*ctx).ocx, nx, (*ctx).bg);
    }
}

#[expect(dead_code)]
unsafe fn tty_cmd_clearstartofline(tty: *mut tty, ctx: *const tty_ctx) {
    unsafe {
        tty_default_attributes(
            tty,
            &raw const (*ctx).defaults,
            (*ctx).palette,
            (*ctx).bg,
            (*(*ctx).s).hyperlinks,
        );

        tty_clear_pane_line(tty, ctx, (*ctx).ocy, 0, (*ctx).ocx + 1, (*ctx).bg);
    }
}

pub unsafe fn tty_cmd_reverseindex(tty: *mut tty, ctx: *const tty_ctx) {
    unsafe {
        let c = (*tty).client;

        if (*ctx).ocy != (*ctx).orupper {
            return;
        }

        if (*ctx).bigger != 0
            || (!tty_full_width(tty, ctx) && !tty_use_margin(tty))
            || tty_fake_bce(tty, &raw const (*ctx).defaults, 8)
            || !tty_term_has((*tty).term, tty_code_code::TTYC_CSR)
            || (!tty_term_has((*tty).term, tty_code_code::TTYC_RI)
                && !tty_term_has((*tty).term, tty_code_code::TTYC_RIN))
            || (*ctx).sx == 1
            || (*ctx).sy == 1
            || (*c).overlay_check.is_some()
        {
            tty_redraw_region(tty, ctx);
            return;
        }

        tty_default_attributes(
            tty,
            &raw const (*ctx).defaults,
            (*ctx).palette,
            (*ctx).bg,
            (*(*ctx).s).hyperlinks,
        );

        tty_region_pane(tty, ctx, (*ctx).orupper, (*ctx).orlower);
        tty_margin_pane(tty, ctx);
        tty_cursor_pane(tty, ctx, (*ctx).ocx, (*ctx).orupper);

        if tty_term_has((*tty).term, tty_code_code::TTYC_RI) {
            tty_putcode(tty, tty_code_code::TTYC_RI);
        } else {
            tty_putcode_i(tty, tty_code_code::TTYC_RIN, 1);
        }
    }
}

#[expect(dead_code)]
unsafe fn tty_cmd_linefeed(tty: *mut tty, ctx: *const tty_ctx) {
    unsafe {
        let c = (*tty).client;

        if (*ctx).ocy != (*ctx).orlower {
            return;
        }

        if (*ctx).bigger != 0
            || (!tty_full_width(tty, ctx) && !tty_use_margin(tty))
            || tty_fake_bce(tty, &raw const (*ctx).defaults, 8)
            || !tty_term_has((*tty).term, tty_code_code::TTYC_CSR)
            || (*ctx).sx == 1
            || (*ctx).sy == 1
            || (*c).overlay_check.is_some()
        {
            tty_redraw_region(tty, ctx);
            return;
        }

        tty_default_attributes(
            tty,
            &(*ctx).defaults,
            (*ctx).palette,
            (*ctx).bg,
            (*(*ctx).s).hyperlinks,
        );

        tty_region_pane(tty, ctx, (*ctx).orupper, (*ctx).orlower);
        tty_margin_pane(tty, ctx);

        // If we want to wrap a pane while using margins, the cursor needs to
        // be exactly on the right of the region. If the cursor is entirely off
        // the edge - move it back to the right. Some terminals are funny about
        // this and insert extra spaces, so only use the right if margins are
        // enabled.
        if (*ctx).xoff + (*ctx).ocx > (*tty).rright {
            if !tty_use_margin(tty) {
                tty_cursor(tty, 0, (*ctx).yoff + (*ctx).ocy);
            } else {
                tty_cursor(tty, (*tty).rright, (*ctx).yoff + (*ctx).ocy);
            }
        } else {
            tty_cursor_pane(tty, ctx, (*ctx).ocx, (*ctx).ocy);
        }

        tty_putc(tty, b'\n');
    }
}

pub unsafe fn tty_cmd_scrollup(tty: *mut tty, ctx: *const tty_ctx) {
    unsafe {
        let c = (*tty).client;

        if (*ctx).bigger != 0
            || (!tty_full_width(tty, ctx) && !tty_use_margin(tty))
            || tty_fake_bce(tty, &raw const (*ctx).defaults, 8)
            || !tty_term_has((*tty).term, tty_code_code::TTYC_CSR)
            || (*ctx).sx == 1
            || (*ctx).sy == 1
            || (*c).overlay_check.is_some()
        {
            tty_redraw_region(tty, ctx);
            return;
        }

        tty_default_attributes(
            tty,
            &(*ctx).defaults,
            (*ctx).palette,
            (*ctx).bg,
            (*(*ctx).s).hyperlinks,
        );

        tty_region_pane(tty, ctx, (*ctx).orupper, (*ctx).orlower);
        tty_margin_pane(tty, ctx);

        if (*ctx).num == 1 || !tty_term_has((*tty).term, tty_code_code::TTYC_INDN) {
            if !tty_use_margin(tty) {
                tty_cursor(tty, 0, (*tty).rlower);
            } else {
                tty_cursor(tty, (*tty).rright, (*tty).rlower);
            }
            for _ in 0..(*ctx).num {
                tty_putc(tty, b'\n');
            }
        } else {
            if (*tty).cy == u32::MAX {
                tty_cursor(tty, 0, 0);
            } else {
                tty_cursor(tty, 0, (*tty).cy);
            }
            tty_putcode_i(tty, tty_code_code::TTYC_INDN, (*ctx).num as i32);
        }
    }
}

pub unsafe fn tty_cmd_scrolldown(tty: *mut tty, ctx: *const tty_ctx) {
    unsafe {
        let c = (*tty).client;

        if (*ctx).bigger != 0
            || (!tty_full_width(tty, ctx) && !tty_use_margin(tty))
            || tty_fake_bce(tty, &raw const (*ctx).defaults, 8)
            || !tty_term_has((*tty).term, tty_code_code::TTYC_CSR)
            || (!tty_term_has((*tty).term, tty_code_code::TTYC_RI)
                && !tty_term_has((*tty).term, tty_code_code::TTYC_RIN))
            || (*ctx).sx == 1
            || (*ctx).sy == 1
            || (*c).overlay_check.is_some()
        {
            tty_redraw_region(tty, ctx);
            return;
        }

        tty_default_attributes(
            tty,
            &(*ctx).defaults,
            (*ctx).palette,
            (*ctx).bg,
            (*(*ctx).s).hyperlinks,
        );

        tty_region_pane(tty, ctx, (*ctx).orupper, (*ctx).orlower);
        tty_margin_pane(tty, ctx);
        tty_cursor_pane(tty, ctx, (*ctx).ocx, (*ctx).orupper);

        if tty_term_has((*tty).term, tty_code_code::TTYC_RIN) {
            tty_putcode_i(tty, tty_code_code::TTYC_RIN, (*ctx).num as i32);
        } else {
            for _ in 0..(*ctx).num {
                tty_putcode(tty, tty_code_code::TTYC_RI);
            }
        }
    }
}

pub unsafe fn tty_cmd_clearendofscreen(tty: *mut tty, ctx: *const tty_ctx) {
    unsafe {
        tty_default_attributes(
            tty,
            &raw const (*ctx).defaults,
            (*ctx).palette,
            (*ctx).bg,
            (*(*ctx).s).hyperlinks,
        );

        tty_region_pane(tty, ctx, 0, (*ctx).sy - 1);
        tty_margin_off(tty);

        let mut px = 0;
        let mut nx = (*ctx).sx;
        let mut py = (*ctx).ocy + 1;
        let ny = (*ctx).sy - (*ctx).ocy - 1;

        tty_clear_pane_area(tty, ctx, py, ny, px, nx, (*ctx).bg);

        px = (*ctx).ocx;
        nx = (*ctx).sx - (*ctx).ocx;
        py = (*ctx).ocy;

        tty_clear_pane_line(tty, ctx, py, px, nx, (*ctx).bg);
    }
}

pub unsafe fn tty_cmd_clearstartofscreen(tty: *mut tty, ctx: *const tty_ctx) {
    unsafe {
        tty_default_attributes(
            tty,
            &raw const (*ctx).defaults,
            (*ctx).palette,
            (*ctx).bg,
            (*(*ctx).s).hyperlinks,
        );

        tty_region_pane(tty, ctx, 0, (*ctx).sy - 1);
        tty_margin_off(tty);

        let mut px = 0;
        let mut nx = (*ctx).sx;
        let mut py = 0;
        let ny = (*ctx).ocy;

        tty_clear_pane_area(tty, ctx, py, ny, px, nx, (*ctx).bg);

        px = 0;
        nx = (*ctx).ocx + 1;
        py = (*ctx).ocy;

        tty_clear_pane_line(tty, ctx, py, px, nx, (*ctx).bg);
    }
}

pub unsafe fn tty_cmd_clearscreen(tty: *mut tty, ctx: *const tty_ctx) {
    unsafe {
        tty_default_attributes(
            tty,
            &raw const (*ctx).defaults,
            (*ctx).palette,
            (*ctx).bg,
            (*(*ctx).s).hyperlinks,
        );

        tty_region_pane(tty, ctx, 0, (*ctx).sy - 1);
        tty_margin_off(tty);

        let px = 0;
        let nx = (*ctx).sx;
        let py = 0;
        let ny = (*ctx).sy;

        tty_clear_pane_area(tty, ctx, py, ny, px, nx, (*ctx).bg);
    }
}

pub unsafe fn tty_cmd_alignmenttest(tty: *mut tty, ctx: *const tty_ctx) {
    unsafe {
        if (*ctx).bigger != 0 {
            (*ctx).redraw_cb.unwrap()(ctx);
            return;
        }

        tty_attributes(
            tty,
            &raw const GRID_DEFAULT_CELL,
            &raw const (*ctx).defaults,
            (*ctx).palette,
            (*(*ctx).s).hyperlinks,
        );

        tty_region_pane(tty, ctx, 0, (*ctx).sy - 1);
        tty_margin_off(tty);

        for j in 0..(*ctx).sy {
            tty_cursor_pane(tty, ctx, 0, j);
            for _ in 0..(*ctx).sx {
                tty_putc(tty, b'E');
            }
        }
    }
}

pub unsafe fn tty_cmd_cell(tty: *mut tty, ctx: *const tty_ctx) {
    unsafe {
        let gcp = (*ctx).cell;
        let s = (*ctx).s;
        let mut r: overlay_ranges = zeroed();
        let mut vis: u32 = 0;

        let px = (*ctx).xoff + (*ctx).ocx - (*ctx).wox;
        let py = (*ctx).yoff + (*ctx).ocy - (*ctx).woy;
        if !tty_is_visible(tty, ctx, (*ctx).ocx, (*ctx).ocy, 1, 1)
            || ((*gcp).data.width == 1 && !tty_check_overlay(tty, px, py))
        {
            return;
        }

        // Handle partially obstructed wide characters.
        if (*gcp).data.width > 1 {
            tty_check_overlay_range(tty, px, py, (*gcp).data.width as u32, &raw mut r);
            for i in 0..OVERLAY_MAX_RANGES {
                vis += r.nx[i];
            }
            if vis < (*gcp).data.width as u32 {
                tty_draw_line(
                    tty,
                    s,
                    (*s).cx,
                    (*s).cy,
                    (*gcp).data.width as u32,
                    px,
                    py,
                    &raw const (*ctx).defaults,
                    (*ctx).palette,
                );
                return;
            }
        }

        if (*ctx).xoff + (*ctx).ocx - (*ctx).wox > (*tty).sx - 1
            && (*ctx).ocy == (*ctx).orlower
            && tty_full_width(tty, ctx)
        {
            tty_region_pane(tty, ctx, (*ctx).orupper, (*ctx).orlower);
        }

        tty_margin_off(tty);
        tty_cursor_pane_unless_wrap(tty, ctx, (*ctx).ocx, (*ctx).ocy);

        tty_cell(
            tty,
            (*ctx).cell,
            &raw const (*ctx).defaults,
            (*ctx).palette,
            (*(*ctx).s).hyperlinks,
        );

        if (*ctx).num == 1 {
            tty_invalidate(tty);
        }
    }
}

pub unsafe fn tty_cmd_cells(tty: *mut tty, ctx: *const tty_ctx) {
    unsafe {
        let mut r: overlay_ranges = zeroed();
        let cp: *mut u8 = (*ctx).ptr.cast();

        if !tty_is_visible(tty, ctx, (*ctx).ocx, (*ctx).ocy, (*ctx).num, 1) {
            return;
        }

        if (*ctx).bigger != 0
            && ((*ctx).xoff + (*ctx).ocx < (*ctx).wox
                || (*ctx).xoff + (*ctx).ocx + (*ctx).num > (*ctx).wox + (*ctx).wsx)
        {
            if !(*ctx).wrapped
                || !tty_full_width(tty, ctx)
                || ((*(*tty).term).flags.intersects(term_flags::TERM_NOAM))
                || (*ctx).xoff + (*ctx).ocx != 0
                || (*ctx).yoff + (*ctx).ocy != (*tty).cy + 1
                || (*tty).cx < (*tty).sx
                || (*tty).cy == (*tty).rlower
            {
                tty_draw_pane(tty, ctx, (*ctx).ocy);
            } else {
                (*ctx).redraw_cb.unwrap()(ctx);
            }
            return;
        }

        tty_margin_off(tty);
        tty_cursor_pane_unless_wrap(tty, ctx, (*ctx).ocx, (*ctx).ocy);
        tty_attributes(
            tty,
            (*ctx).cell,
            &raw const (*ctx).defaults,
            (*ctx).palette,
            (*(*ctx).s).hyperlinks,
        );

        // Get tty position from pane position for overlay check.
        let px = (*ctx).xoff + (*ctx).ocx - (*ctx).wox;
        let py = (*ctx).yoff + (*ctx).ocy - (*ctx).woy;

        tty_check_overlay_range(tty, px, py, (*ctx).num, &raw mut r);
        for i in 0..OVERLAY_MAX_RANGES {
            if r.nx[i] == 0 {
                continue;
            }
            // Convert back to pane position for printing.
            let cx = r.px[i] - (*ctx).xoff + (*ctx).wox;
            tty_cursor_pane_unless_wrap(tty, ctx, cx, (*ctx).ocy);
            tty_putn(
                tty,
                cp.add(r.px[i] as usize - px as usize).cast(),
                r.nx[i] as usize,
                r.nx[i],
            );
        }
    }
}

pub unsafe fn tty_cmd_setselection(tty: *mut tty, ctx: *const tty_ctx) {
    unsafe {
        tty_set_selection(
            tty,
            (*ctx).ptr2.cast(),
            (*ctx).ptr.cast(),
            (*ctx).num as usize,
        );
    }
}

pub unsafe fn tty_set_selection(tty: *mut tty, flags: *const u8, buf: *const u8, len: usize) {
    unsafe {
        if !(*tty).flags.intersects(tty_flags::TTY_STARTED) {
            return;
        }
        if !tty_term_has((*tty).term, tty_code_code::TTYC_MS) {
            return;
        }

        let size = 4 * len.div_ceil(3) + 1; /* storage for base64 */
        let encoded: *mut u8 = xmalloc(size).as_ptr().cast();

        b64_ntop(buf.cast(), len, encoded, size);
        (*tty).flags |= tty_flags::TTY_NOBLOCK;
        tty_putcode_ss(tty, tty_code_code::TTYC_MS, flags, encoded);

        free_(encoded);
    }
}

pub unsafe fn tty_cmd_rawstring(tty: *mut tty, ctx: *const tty_ctx) {
    unsafe {
        (*tty).flags |= tty_flags::TTY_NOBLOCK;
        tty_add(tty, (*ctx).ptr.cast(), (*ctx).num as usize);
        tty_invalidate(tty);
    }
}

#[cfg(feature = "sixel")]
pub unsafe fn tty_cmd_sixelimage(tty: *mut tty, ctx: *const tty_ctx) {
    use crate::image_sixel::{sixel_free, sixel_print, sixel_scale, sixel_size_in_cells};

    unsafe {
        let im: *mut image = (*ctx).ptr.cast();
        let si: *mut sixel_image = (*im).data;
        let mut new: *mut sixel_image = null_mut();
        let data: *mut u8;
        let mut size = 0;
        let cx = (*ctx).ocx;
        let cy = (*ctx).ocy;

        let mut i: u32 = 0;
        let mut j: u32 = 0;
        let mut x: u32 = 0;
        let mut y: u32 = 0;
        let mut rx: u32 = 0;
        let mut ry: u32 = 0;
        let mut fallback = 0;

        if !(*(*tty).term).flags.intersects(term_flags::TERM_SIXEL)
            && !tty_term_has((*tty).term, tty_code_code::TTYC_SXL)
        {
            fallback = 1;
        }
        if (*tty).xpixel == 0 || (*tty).ypixel == 0 {
            fallback = 1;
        }

        let (sx, sy) = sixel_size_in_cells(&*si);
        log_debug!("tty_cmd_sixelimage: image is {sx}x{sy}");
        if !tty_clamp_area(
            tty,
            ctx,
            cx,
            cy,
            sx,
            sy,
            &raw mut i,
            &raw mut j,
            &raw mut x,
            &raw mut y,
            &raw mut rx,
            &raw mut ry,
        ) {
            return;
        }
        log_debug!("tty_cmd_sixelimage: clamping to {i},{j}-{rx},{ry} fallback={fallback}");

        if fallback == 1 {
            data = xstrdup((*im).fallback).as_ptr();
            size = strlen(data);
        } else {
            new = sixel_scale(si, (*tty).xpixel, (*tty).ypixel, i, j, rx, ry, 0);
            if new.is_null() {
                return;
            }

            data = sixel_print(new, si, &raw mut size);
        }
        if !data.is_null() {
            log_debug!("tty_cmd_sixelimage: {} bytes: {}", size, _s(data));
            tty_region_off(tty);
            tty_margin_off(tty);
            tty_cursor(tty, x, y);

            (*tty).flags |= tty_flags::TTY_NOBLOCK;
            tty_add(tty, data, size);
            tty_invalidate(tty);
            free_(data);
        }

        if fallback == 0 {
            sixel_free(new);
        }
    }
}

pub unsafe fn tty_cmd_syncstart(tty: *mut tty, ctx: *const tty_ctx) {
    unsafe {
        if (*ctx).num == 0x11 {
            /*
             * This is an overlay and a command that moves the cursor so
             * start synchronized updates.
             */
            tty_sync_start(tty);
        } else if !(*ctx).num & 0x10 != 0
            && ((*ctx).num != 0 || (*(*tty).client).overlay_draw.is_some())
        {
            tty_sync_start(tty);
        } /*
         * This is a pane. If there is an overlay, always start;
         * otherwise, only if requested.
         */
    }
}

pub unsafe fn tty_cell(
    tty: *mut tty,
    gc: *const grid_cell,
    defaults: *const grid_cell,
    palette: *const colour_palette,
    hl: *mut hyperlinks,
) {
    unsafe {
        // Skip last character if terminal is stupid.
        if ((*(*tty).term).flags.intersects(term_flags::TERM_NOAM))
            && (*tty).cy == (*tty).sy - 1
            && (*tty).cx == (*tty).sx - 1
        {
            return;
        }

        // If this is a padding character, do nothing.
        if (*gc).flags.intersects(grid_flag::PADDING) {
            return;
        }

        // Check the output codeset and apply attributes.
        let gcp = tty_check_codeset(tty, gc);
        tty_attributes(tty, gcp, defaults, palette, hl);

        // If it is a single character, write with putc to handle ACS.
        if (*gcp).data.size == 1 {
            tty_attributes(tty, gcp, defaults, palette, hl);
            if (*gcp).data.data[0] < 0x20 || (*gcp).data.data[0] == 0x7f {
                return;
            }
            tty_putc(tty, (*gcp).data.data[0]);
            return;
        }

        // Write the data.
        tty_putn(
            tty,
            (&raw const (*gcp).data.data).cast(),
            (*gcp).data.size as usize,
            (*gcp).data.width as u32,
        );
    }
}

pub unsafe fn tty_reset(tty: *mut tty) {
    unsafe {
        let gc = &raw mut (*tty).cell;

        if grid_cells_equal(gc, &raw const GRID_DEFAULT_CELL) == 0 {
            if (*gc).link != 0 {
                tty_putcode_ss(tty, tty_code_code::TTYC_HLS, c!(""), c!(""));
            }
            if (*gc).attr.intersects(grid_attr::GRID_ATTR_CHARSET) && tty_acs_needed(tty) != 0 {
                tty_putcode(tty, tty_code_code::TTYC_RMACS);
            }
            tty_putcode(tty, tty_code_code::TTYC_SGR0);
            memcpy__(gc, &raw const GRID_DEFAULT_CELL);
        }
        memcpy__(&raw mut (*tty).last_cell, &raw const GRID_DEFAULT_CELL);
    }
}

pub unsafe fn tty_invalidate(tty: *mut tty) {
    unsafe {
        memcpy__(&raw mut (*tty).cell, &raw const GRID_DEFAULT_CELL);
        memcpy__(&raw mut (*tty).last_cell, &raw const GRID_DEFAULT_CELL);

        (*tty).cx = u32::MAX;
        (*tty).cy = u32::MAX;
        (*tty).rupper = u32::MAX;
        (*tty).rlower = u32::MAX;
        (*tty).rright = u32::MAX;
        (*tty).rleft = u32::MAX;

        if (*tty).flags.intersects(tty_flags::TTY_STARTED) {
            if tty_use_margin(tty) {
                tty_putcode(tty, tty_code_code::TTYC_ENMG);
            }
            tty_putcode(tty, tty_code_code::TTYC_SGR0);

            (*tty).mode = mode_flag::all();
            tty_update_mode(tty, mode_flag::MODE_CURSOR, null_mut());

            tty_cursor(tty, 0, 0);
            tty_region_off(tty);
            tty_margin_off(tty);
        } else {
            (*tty).mode = mode_flag::MODE_CURSOR;
        }
    }
}

/// Turn off margin.
pub unsafe fn tty_region_off(tty: *mut tty) {
    unsafe {
        tty_region(tty, 0, (*tty).sy - 1);
    }
}

/// Set region inside pane.
pub unsafe fn tty_region_pane(tty: *mut tty, ctx: *const tty_ctx, rupper: u32, rlower: u32) {
    unsafe {
        tty_region(
            tty,
            (*ctx).yoff + rupper - (*ctx).woy,
            (*ctx).yoff + rlower - (*ctx).woy,
        );
    }
}

/// Set region at absolute position.
pub unsafe fn tty_region(tty: *mut tty, rupper: u32, rlower: u32) {
    unsafe {
        if (*tty).rlower == rlower && (*tty).rupper == rupper {
            return;
        }
        if !tty_term_has((*tty).term, tty_code_code::TTYC_CSR) {
            return;
        }

        (*tty).rupper = rupper;
        (*tty).rlower = rlower;

        // Some terminals (such as PuTTY) do not correctly reset the cursor to
        // 0,0 if it is beyond the last column (they do not reset their wrap
        // flag so further output causes a line feed). As a workaround, do an
        // explicit move to 0 first.
        if (*tty).cx >= (*tty).sx {
            if (*tty).cy == u32::MAX {
                tty_cursor(tty, 0, 0);
            } else {
                tty_cursor(tty, 0, (*tty).cy);
            }
        }

        tty_putcode_ii(
            tty,
            tty_code_code::TTYC_CSR,
            (*tty).rupper as i32,
            (*tty).rlower as i32,
        );
        (*tty).cx = u32::MAX;
        (*tty).cy = u32::MAX;
    }
}

/// Turn off margin.
pub unsafe fn tty_margin_off(tty: *mut tty) {
    unsafe {
        tty_margin(tty, 0, (*tty).sx - 1);
    }
}

/// Set margin inside pane.
pub unsafe fn tty_margin_pane(tty: *mut tty, ctx: *const tty_ctx) {
    unsafe {
        tty_margin(
            tty,
            (*ctx).xoff - (*ctx).wox,
            (*ctx).xoff + (*ctx).sx - 1 - (*ctx).wox,
        );
    }
}

// Set margin at absolute position.

pub unsafe fn tty_margin(tty: *mut tty, rleft: u32, rright: u32) {
    unsafe {
        if !tty_use_margin(tty) {
            return;
        }
        if (*tty).rleft == rleft && (*tty).rright == rright {
            return;
        }

        tty_putcode_ii(
            tty,
            tty_code_code::TTYC_CSR,
            (*tty).rupper as i32,
            (*tty).rlower as i32,
        );

        (*tty).rleft = rleft;
        (*tty).rright = rright;

        if rleft == 0 && rright == (*tty).sx - 1 {
            tty_putcode(tty, tty_code_code::TTYC_CLMG);
        } else {
            tty_putcode_ii(tty, tty_code_code::TTYC_CMG, rleft as i32, rright as i32);
        }
        (*tty).cx = u32::MAX;
        (*tty).cy = u32::MAX;
    }
}

// Move the cursor, unless it would wrap itself when the next character is
// printed.

pub unsafe fn tty_cursor_pane_unless_wrap(tty: *mut tty, ctx: *const tty_ctx, cx: u32, cy: u32) {
    unsafe {
        if !(*ctx).wrapped
            || !tty_full_width(tty, ctx)
            || (*(*tty).term).flags.intersects(term_flags::TERM_NOAM)
            || (*ctx).xoff + cx != 0
            || (*ctx).yoff + cy != (*tty).cy + 1
            || (*tty).cx < (*tty).sx
            || (*tty).cy == (*tty).rlower
        {
            tty_cursor_pane(tty, ctx, cx, cy);
        } else {
            // log_debug("%s: will wrap at %u,%u", __func__, (*tty).cx, (*tty).cy);
        }
    }
}

// Move cursor inside pane.

pub unsafe fn tty_cursor_pane(tty: *mut tty, ctx: *const tty_ctx, cx: u32, cy: u32) {
    unsafe {
        tty_cursor(
            tty,
            (*ctx).xoff + cx - (*ctx).wox,
            (*ctx).yoff + cy - (*ctx).woy,
        );
    }
}

// Move cursor to absolute position.

pub unsafe fn tty_cursor(tty: *mut tty, mut cx: u32, cy: u32) {
    unsafe {
        let term = (*tty).term;

        if (*tty).flags.intersects(tty_flags::TTY_BLOCK) {
            return;
        }

        let thisx = (*tty).cx;
        let thisy = (*tty).cy;

        'out: {
            'absolute: {
                // If in the automargin space, and want to be there, do not move.
                // Otherwise, force the cursor to be in range (and complain).
                if cx == thisx && cy == thisy && cx == (*tty).sx {
                    return;
                }
                if cx > (*tty).sx - 1 {
                    cx = (*tty).sx - 1;
                } // log_debug("%s: x too big %u > %u", __func__, cx, (*tty).sx - 1);

                // No change.
                if cx == thisx && cy == thisy {
                    return;
                }

                // Currently at the very end of the line - use absolute movement.
                if thisx > (*tty).sx - 1 {
                    break 'absolute;
                }

                // Move to home position (0, 0).
                if cx == 0 && cy == 0 && tty_term_has(term, tty_code_code::TTYC_HOME) {
                    tty_putcode(tty, tty_code_code::TTYC_HOME);
                    break 'out;
                }

                // Zero on the next line.
                if cx == 0
                    && cy == thisy + 1
                    && thisy != (*tty).rlower
                    && (!tty_use_margin(tty) || (*tty).rleft == 0)
                {
                    tty_putc(tty, b'\r');
                    tty_putc(tty, b'\n');
                    break 'out;
                }

                // Moving column or row.
                if cy == thisy {
                    // Moving column only, row staying the same.

                    // To left edge.
                    if cx == 0 && (!tty_use_margin(tty) || (*tty).rleft == 0) {
                        tty_putc(tty, b'\r');
                        break 'out;
                    }

                    // One to the left.
                    // TODO underflows on debug rust
                    if cx == thisx.wrapping_sub(1) && tty_term_has(term, tty_code_code::TTYC_CUB1) {
                        tty_putcode(tty, tty_code_code::TTYC_CUB1);
                        break 'out;
                    }

                    // One to the right.
                    if cx == thisx + 1 && tty_term_has(term, tty_code_code::TTYC_CUF1) {
                        tty_putcode(tty, tty_code_code::TTYC_CUF1);
                        break 'out;
                    }

                    // Calculate difference.
                    let change: i32 = thisx as i32 - cx as i32; /* +ve left, -ve right */

                    // Use HPA if change is larger than absolute, otherwise move
                    // the cursor with CUB/CUF.
                    if change.unsigned_abs() > cx && tty_term_has(term, tty_code_code::TTYC_HPA) {
                        tty_putcode_i(tty, tty_code_code::TTYC_HPA, cx as i32);
                        break 'out;
                    } else if change > 0
                        && tty_term_has(term, tty_code_code::TTYC_CUB)
                        && !tty_use_margin(tty)
                    {
                        if change == 2 && tty_term_has(term, tty_code_code::TTYC_CUB1) {
                            tty_putcode(tty, tty_code_code::TTYC_CUB1);
                            tty_putcode(tty, tty_code_code::TTYC_CUB1);
                            break 'out;
                        }
                        tty_putcode_i(tty, tty_code_code::TTYC_CUB, change);
                        break 'out;
                    } else if change < 0
                        && tty_term_has(term, tty_code_code::TTYC_CUF)
                        && !tty_use_margin(tty)
                    {
                        tty_putcode_i(tty, tty_code_code::TTYC_CUF, -change);
                        break 'out;
                    }
                } else if cx == thisx {
                    // Moving row only, column staying the same.

                    // One above.
                    if thisy != (*tty).rupper
                        && cy + 1 == thisy // note avoids underflow in dev
                        && tty_term_has(term, tty_code_code::TTYC_CUU1)
                    {
                        tty_putcode(tty, tty_code_code::TTYC_CUU1);
                        break 'out;
                    }

                    // One below.
                    if thisy != (*tty).rlower
                        && cy == thisy + 1
                        && tty_term_has(term, tty_code_code::TTYC_CUD1)
                    {
                        tty_putcode(tty, tty_code_code::TTYC_CUD1);
                        break 'out;
                    }

                    // Calculate difference.
                    let change: i32 = thisy as i32 - cy as i32; /* +ve up, -ve down */

                    // Try to use VPA if change is larger than absolute or if this
                    // change would cross the scroll region, otherwise use CUU/CUD.
                    if change.unsigned_abs() > cy
                        || (change < 0 && cy as i32 - change > (*tty).rlower as i32)
                        || (change > 0 && cy as i32 - change < (*tty).rupper as i32)
                    {
                        if tty_term_has(term, tty_code_code::TTYC_VPA) {
                            tty_putcode_i(tty, tty_code_code::TTYC_VPA, cy as i32);
                            break 'out;
                        }
                    } else if change > 0 && tty_term_has(term, tty_code_code::TTYC_CUU) {
                        tty_putcode_i(tty, tty_code_code::TTYC_CUU, change);
                        break 'out;
                    } else if change < 0 && tty_term_has(term, tty_code_code::TTYC_CUD) {
                        tty_putcode_i(tty, tty_code_code::TTYC_CUD, -change);
                        break 'out;
                    }
                }
            } // absolute:

            // Absolute movement.
            tty_putcode_ii(tty, tty_code_code::TTYC_CUP, cy as i32, cx as i32);
        } // out:
        (*tty).cx = cx;
        (*tty).cy = cy;
    }
}

pub unsafe fn tty_hyperlink(tty: *mut tty, gc: *const grid_cell, hl: *mut hyperlinks) {
    unsafe {
        if (*gc).link == (*tty).cell.link {
            return;
        }
        (*tty).cell.link = (*gc).link;

        if hl.is_null() {
            return;
        }

        let mut id = null();
        let mut uri = null();
        if (*gc).link == 0 || !hyperlinks_get(hl, (*gc).link, &raw mut uri, null_mut(), &raw mut id)
        {
            tty_putcode_ss(tty, tty_code_code::TTYC_HLS, c!(""), c!(""));
        } else {
            tty_putcode_ss(tty, tty_code_code::TTYC_HLS, id, uri);
        }
    }
}

pub unsafe fn tty_attributes(
    tty: *mut tty,
    gc: *const grid_cell,
    defaults: *const grid_cell,
    palette: *const colour_palette,
    hl: *mut hyperlinks,
) {
    unsafe {
        let tc = &raw mut (*tty).cell;
        let mut gc2: grid_cell = zeroed();

        // Copy cell and update default colours.
        memcpy__(&raw mut gc2, gc);
        if !(*gc).flags.intersects(grid_flag::NOPALETTE) {
            if gc2.fg == 8 {
                gc2.fg = (*defaults).fg;
            }
            if gc2.bg == 8 {
                gc2.bg = (*defaults).bg;
            }
        }

        // Ignore cell if it is the same as the last one.
        if gc2.attr == (*tty).last_cell.attr
            && gc2.fg == (*tty).last_cell.fg
            && gc2.bg == (*tty).last_cell.bg
            && gc2.us == (*tty).last_cell.us
            && gc2.link == (*tty).last_cell.link
        {
            return;
        }

        // If no setab, try to use the reverse attribute as a best-effort for a
        // non-default background. This is a bit of a hack but it doesn't do
        // any serious harm and makes a couple of applications happier.
        if !tty_term_has((*tty).term, tty_code_code::TTYC_SETAB) {
            if gc2.attr.intersects(grid_attr::GRID_ATTR_REVERSE) {
                if gc2.fg != 7 && !COLOUR_DEFAULT(gc2.fg) {
                    gc2.attr &= !grid_attr::GRID_ATTR_REVERSE;
                }
            } else if gc2.bg != 0 && !COLOUR_DEFAULT(gc2.bg) {
                gc2.attr |= grid_attr::GRID_ATTR_REVERSE;
            }
        }

        // Fix up the colours if necessary.
        tty_check_fg(tty, palette, &raw mut gc2);
        tty_check_bg(tty, palette, &raw mut gc2);
        tty_check_us(tty, palette, &raw mut gc2);

        // If any bits are being cleared or the underline colour is now default,
        // reset everything.
        if (*tc).attr.intersects(!gc2.attr) || (*tc).us != gc2.us && gc2.us == 0 {
            tty_reset(tty);
        }

        // Set the colours. This may call tty_reset() (so it comes next) and
        // may add to (NOT remove) the desired attributes.
        tty_colours(tty, &raw mut gc2);

        // Filter out attribute bits already set.
        let changed = gc2.attr & !(*tc).attr;
        (*tc).attr = gc2.attr;

        // Set the attributes.
        if changed.intersects(grid_attr::GRID_ATTR_BRIGHT) {
            tty_putcode(tty, tty_code_code::TTYC_BOLD);
        }
        if changed.intersects(grid_attr::GRID_ATTR_DIM) {
            tty_putcode(tty, tty_code_code::TTYC_DIM);
        }
        if changed.intersects(grid_attr::GRID_ATTR_ITALICS) {
            tty_set_italics(tty);
        }
        if changed.intersects(GRID_ATTR_ALL_UNDERSCORE) {
            if (changed.intersects(grid_attr::GRID_ATTR_UNDERSCORE))
                || !tty_term_has((*tty).term, tty_code_code::TTYC_SMULX)
            {
                tty_putcode(tty, tty_code_code::TTYC_SMUL);
            } else if changed.intersects(grid_attr::GRID_ATTR_UNDERSCORE_2) {
                tty_putcode_i(tty, tty_code_code::TTYC_SMULX, 2);
            } else if changed.intersects(grid_attr::GRID_ATTR_UNDERSCORE_3) {
                tty_putcode_i(tty, tty_code_code::TTYC_SMULX, 3);
            } else if changed.intersects(grid_attr::GRID_ATTR_UNDERSCORE_4) {
                tty_putcode_i(tty, tty_code_code::TTYC_SMULX, 4);
            } else if changed.intersects(grid_attr::GRID_ATTR_UNDERSCORE_5) {
                tty_putcode_i(tty, tty_code_code::TTYC_SMULX, 5);
            }
        }
        if changed.intersects(grid_attr::GRID_ATTR_BLINK) {
            tty_putcode(tty, tty_code_code::TTYC_BLINK);
        }
        if changed.intersects(grid_attr::GRID_ATTR_REVERSE) {
            if tty_term_has((*tty).term, tty_code_code::TTYC_REV) {
                tty_putcode(tty, tty_code_code::TTYC_REV);
            } else if tty_term_has((*tty).term, tty_code_code::TTYC_SMSO) {
                tty_putcode(tty, tty_code_code::TTYC_SMSO);
            }
        }
        if changed.intersects(grid_attr::GRID_ATTR_HIDDEN) {
            tty_putcode(tty, tty_code_code::TTYC_INVIS);
        }
        if changed.intersects(grid_attr::GRID_ATTR_STRIKETHROUGH) {
            tty_putcode(tty, tty_code_code::TTYC_SMXX);
        }
        if changed.intersects(grid_attr::GRID_ATTR_OVERLINE) {
            tty_putcode(tty, tty_code_code::TTYC_SMOL);
        }
        if changed.intersects(grid_attr::GRID_ATTR_CHARSET) && tty_acs_needed(tty) != 0 {
            tty_putcode(tty, tty_code_code::TTYC_SMACS);
        }

        // Set hyperlink if any.
        tty_hyperlink(tty, gc, hl);

        memcpy__(&raw mut (*tty).last_cell, &raw const gc2);
    }
}

pub unsafe fn tty_colours(tty: *mut tty, gc: *const grid_cell) {
    unsafe {
        let tc = &raw mut (*tty).cell;

        /* No changes? Nothing is necessary. */
        if (*gc).fg == (*tc).fg && (*gc).bg == (*tc).bg && (*gc).us == (*tc).us {
            return;
        }

        /*
         * Is either the default colour? This is handled specially because the
         * best solution might be to reset both colours to default, in which
         * case if only one is default need to fall onward to set the other
         * colour.
         */
        if COLOUR_DEFAULT((*gc).fg) || COLOUR_DEFAULT((*gc).bg) {
            if tty_term_flag((*tty).term, tty_code_code::TTYC_AX) == 0 {
                tty_reset(tty);
            } else {
                if COLOUR_DEFAULT((*gc).fg) && !COLOUR_DEFAULT((*tc).fg) {
                    tty_puts(tty, c!("\x1b[39m"));
                    (*tc).fg = (*gc).fg;
                }
                if COLOUR_DEFAULT((*gc).bg) && !COLOUR_DEFAULT((*tc).bg) {
                    tty_puts(tty, c!("\x1b[49m"));
                    (*tc).bg = (*gc).bg;
                }
            }
        } /*
         * If don't have AX, send sgr0. This resets both colours to default.
         * Otherwise, try to set the default colour only as needed.
         */

        /* Set the foreground colour. */
        if !COLOUR_DEFAULT((*gc).fg) && (*gc).fg != (*tc).fg {
            tty_colours_fg(tty, gc);
        }

        /*
         * Set the background colour. This must come after the foreground as
         * tty_colours_fg() can call tty_reset().
         */
        if !COLOUR_DEFAULT((*gc).bg) && (*gc).bg != (*tc).bg {
            tty_colours_bg(tty, gc);
        }

        /* Set the underscore colour. */
        if (*gc).us != (*tc).us {
            tty_colours_us(tty, gc);
        }
    }
}

pub unsafe fn tty_check_fg(tty: *const tty, palette: *const colour_palette, gc: *mut grid_cell) {
    unsafe {
        let mut c: i32;

        // Perform substitution if this pane has a palette. If the bright
        // attribute is set and Nobr is not present, use the bright entry in
        // the palette by changing to the aixterm colour
        if !(*gc).flags.intersects(grid_flag::NOPALETTE) {
            c = (*gc).fg;
            if c < 8
                && (*gc).attr.intersects(grid_attr::GRID_ATTR_BRIGHT)
                && !tty_term_has((*tty).term, tty_code_code::TTYC_NOBR)
            {
                c += 90;
            }
            c = colour_palette_get(ptr_to_ref(palette), c);
            if c != -1 {
                (*gc).fg = c;
            }
        }

        // Is this a 24-bit colour?
        if (*gc).fg & COLOUR_FLAG_RGB != 0 {
            // Not a 24-bit terminal? Translate to 256-colour palette.
            if (*(*tty).term).flags.intersects(term_flags::TERM_RGBCOLOURS) {
                return;
            }
            let (r, g, b) = colour_split_rgb((*gc).fg);
            (*gc).fg = colour_find_rgb(r, g, b);
        }

        // How many colours does this terminal have?
        let colours = if (*(*tty).term).flags.intersects(term_flags::TERM_256COLOURS) {
            256
        } else {
            tty_term_number((*tty).term, tty_code_code::TTYC_COLORS) as u32
        };

        // Is this a 256-colour colour?
        if (*gc).fg & COLOUR_FLAG_256 != 0 {
            // And not a 256 colour mode?
            if colours < 256 {
                (*gc).fg = colour_256to16((*gc).fg);
                if ((*gc).fg & 8) != 0 {
                    (*gc).fg &= 7;
                    if colours >= 16 {
                        (*gc).fg += 90;
                    }
                }
            }
            return;
        }

        // Is this an aixterm colour?
        if (*gc).fg >= 90 && (*gc).fg <= 97 && colours < 16 {
            (*gc).fg -= 90;
            (*gc).attr |= grid_attr::GRID_ATTR_BRIGHT;
        }
    }
}

pub unsafe fn tty_check_bg(tty: *const tty, palette: *const colour_palette, gc: *mut grid_cell) {
    unsafe {
        let c: i32;

        // Perform substitution if this pane has a palette.
        if !(*gc).flags.intersects(grid_flag::NOPALETTE) {
            c = colour_palette_get(ptr_to_ref(palette), (*gc).bg);
            if c != -1 {
                (*gc).bg = c;
            }
        }

        // Is this a 24-bit colour?
        if (*gc).bg & COLOUR_FLAG_RGB != 0 {
            // Not a 24-bit terminal? Translate to 256-colour palette.
            if (*(*tty).term).flags.intersects(term_flags::TERM_RGBCOLOURS) {
                return;
            }
            let (r, g, b) = colour_split_rgb((*gc).bg);
            (*gc).bg = colour_find_rgb(r, g, b);
        }

        // How many colours does this terminal have?
        let colours = if (*(*tty).term).flags.intersects(term_flags::TERM_256COLOURS) {
            256
        } else {
            tty_term_number((*tty).term, tty_code_code::TTYC_COLORS) as u32
        };

        // Is this a 256-colour colour?
        if (*gc).bg & COLOUR_FLAG_256 != 0 {
            // And not a 256 colour mode? Translate to 16-colour
            // palette. Bold background doesn't exist portably, so just
            // discard the bold bit if set.
            if colours < 256 {
                (*gc).bg = colour_256to16((*gc).bg);
                if (*gc).bg & 8 != 0 {
                    (*gc).bg &= 7;
                    if colours >= 16 {
                        (*gc).bg += 90;
                    }
                }
            }
            return;
        }

        // Is this an aixterm colour?
        if (*gc).bg >= 90 && (*gc).bg <= 97 && colours < 16 {
            (*gc).bg -= 90;
        }
    }
}

pub unsafe fn tty_check_us(tty: *const tty, palette: *const colour_palette, gc: *mut grid_cell) {
    unsafe {
        let mut c;

        // Perform substitution if this pane has a palette.
        if !(*gc).flags.intersects(grid_flag::NOPALETTE) {
            c = colour_palette_get(ptr_to_ref(palette), (*gc).us);
            if c != -1 {
                (*gc).us = c;
            }
        }

        // Convert underscore colour if only RGB can be supported.
        if !tty_term_has((*tty).term, tty_code_code::TTYC_SETULC1) {
            c = colour_force_rgb((*gc).us);
            if c == -1 {
                (*gc).us = 8;
            } else {
                (*gc).us = c;
            }
        }
    }
}

pub unsafe fn tty_colours_fg(tty: *mut tty, gc: *const grid_cell) {
    unsafe {
        let tc = &raw mut (*tty).cell;
        let sizeof_s: usize = 32;
        let mut s: [u8; 32] = [0; 32];

        'save: {
            // If the current colour is an aixterm bright colour and the new is not,
            // reset because some terminals do not clear bright correctly.
            if (*tty).cell.fg >= 90 && (*tty).cell.bg <= 97 && ((*gc).fg < 90 || (*gc).fg > 97) {
                tty_reset(tty);
            }

            // Is this a 24-bit or 256-colour colour?
            if (*gc).fg & COLOUR_FLAG_RGB != 0 || (*gc).fg & COLOUR_FLAG_256 != 0 {
                if tty_try_colour(tty, (*gc).fg, c!("38")) == 0 {
                    break 'save;
                }
                // Should not get here, already converted in tty_check_fg.
                return;
            }

            // Is this an aixterm bright colour?
            if (*gc).fg >= 90 && (*gc).fg <= 97 {
                if (*(*tty).term).flags.intersects(term_flags::TERM_256COLOURS) {
                    _ = xsnprintf_!((&raw mut s).cast(), sizeof_s, "\x1b[{}m", (*gc).fg,);
                    tty_puts(tty, (&raw const s).cast());
                } else {
                    tty_putcode_i(tty, tty_code_code::TTYC_SETAF, (*gc).fg - 90 + 8);
                }
                break 'save;
            }

            // Otherwise set the foreground colour.
            tty_putcode_i(tty, tty_code_code::TTYC_SETAF, (*gc).fg);
        } // save:

        // Save the new values in the terminal current cell.
        (*tc).fg = (*gc).fg;
    }
}

pub unsafe fn tty_colours_bg(tty: *mut tty, gc: *const grid_cell) {
    unsafe {
        let tc = &raw mut (*tty).cell;
        let sizeof_s: usize = 32;
        let mut s: [u8; 32] = [0; 32];

        'save: {
            // Is this a 24-bit or 256-colour colour?
            if (*gc).bg & COLOUR_FLAG_RGB != 0 || (*gc).bg & COLOUR_FLAG_256 != 0 {
                if tty_try_colour(tty, (*gc).bg, c!("48")) == 0 {
                    break 'save;
                }
                // Should not get here, already converted in tty_check_bg.
                return;
            }

            // Is this an aixterm bright colour?
            if (*gc).bg >= 90 && (*gc).bg <= 97 {
                if (*(*tty).term).flags.intersects(term_flags::TERM_256COLOURS) {
                    _ = xsnprintf_!((&raw mut s).cast(), sizeof_s, "\x1b[{}m", (*gc).bg + 10,);
                    tty_puts(tty, (&raw const s).cast());
                } else {
                    tty_putcode_i(tty, tty_code_code::TTYC_SETAB, (*gc).bg - 90 + 8);
                }
                break 'save;
            }

            // Otherwise set the background colour.
            tty_putcode_i(tty, tty_code_code::TTYC_SETAB, (*gc).bg);
        } //save:

        // Save the new values in the terminal current cell.
        (*tc).bg = (*gc).bg;
    }
}

pub unsafe fn tty_colours_us(tty: *mut tty, gc: *const grid_cell) {
    unsafe {
        let tc = &raw mut (*tty).cell;
        let mut c: u32;

        'save: {
            // Clear underline colour.
            if COLOUR_DEFAULT((*gc).us) {
                tty_putcode(tty, tty_code_code::TTYC_OL);
                break 'save;
            }

            // If this is not an RGB colour, use Setulc1 if it exists, otherwise
            // convert.
            if !(*gc).us & COLOUR_FLAG_RGB != 0 {
                c = (*gc).us as u32;
                if (!c & COLOUR_FLAG_256 as u32 != 0) && (90..=97).contains(&c) {
                    c -= 82;
                }
                tty_putcode_i(
                    tty,
                    tty_code_code::TTYC_SETULC1,
                    c as i32 & !COLOUR_FLAG_256,
                );
                return;
            }

            // Setulc and setal follows the ncurses(3) one argument "direct colour"
            // capability format. Calculate the colour value.
            let (r, g, b) = colour_split_rgb((*gc).us);
            c = (65536 * r as u32) + (256 * g as u32) + b as u32;

            // Write the colour. Only use setal if the RGB flag is set because the
            // non-RGB version may be wrong.
            if tty_term_has((*tty).term, tty_code_code::TTYC_SETULC) {
                tty_putcode_i(tty, tty_code_code::TTYC_SETULC, c as i32);
            } else if tty_term_has((*tty).term, tty_code_code::TTYC_SETAL)
                && tty_term_has((*tty).term, tty_code_code::TTYC_RGB)
            {
                tty_putcode_i(tty, tty_code_code::TTYC_SETAL, c as i32);
            }
        } // save:

        // Save the new values in the terminal current cell.
        (*tc).us = (*gc).us;
    }
}

pub unsafe fn tty_try_colour(tty: *mut tty, colour: i32, type_: *const u8) -> i32 {
    unsafe {
        if colour & COLOUR_FLAG_256 != 0 {
            if *type_ == b'3' && tty_term_has((*tty).term, tty_code_code::TTYC_SETAF) {
                tty_putcode_i(tty, tty_code_code::TTYC_SETAF, colour & 0xff);
            } else if tty_term_has((*tty).term, tty_code_code::TTYC_SETAB) {
                tty_putcode_i(tty, tty_code_code::TTYC_SETAB, colour & 0xff);
            }
            return 0;
        }

        if colour & COLOUR_FLAG_RGB != 0 {
            let (r, g, b) = colour_split_rgb(colour & 0xffffff);
            if *type_ == b'3' && tty_term_has((*tty).term, tty_code_code::TTYC_SETRGBF) {
                tty_putcode_iii(
                    tty,
                    tty_code_code::TTYC_SETRGBF,
                    r as i32,
                    g as i32,
                    b as i32,
                );
            } else if tty_term_has((*tty).term, tty_code_code::TTYC_SETRGBB) {
                tty_putcode_iii(
                    tty,
                    tty_code_code::TTYC_SETRGBB,
                    r as i32,
                    g as i32,
                    b as i32,
                );
            }
            return 0;
        }

        -1
    }
}

pub unsafe fn tty_window_default_style(gc: *mut grid_cell, wp: *mut window_pane) {
    unsafe {
        memcpy__(gc, &raw const GRID_DEFAULT_CELL);
        (*gc).fg = (*wp).palette.fg;
        (*gc).bg = (*wp).palette.bg;
    }
}

pub unsafe fn tty_default_colours(gc: *mut grid_cell, wp: *mut window_pane) {
    unsafe {
        let oo = (*wp).options;

        memcpy__(gc, &raw const GRID_DEFAULT_CELL);

        if (*wp).flags.intersects(window_pane_flags::PANE_STYLECHANGED) {
            // log_debug("%%%u: style changed", (*wp).id);
            (*wp).flags &= !window_pane_flags::PANE_STYLECHANGED;

            let ft = format_create(
                null_mut(),
                null_mut(),
                (FORMAT_PANE | (*wp).id) as i32,
                format_flags::FORMAT_NOJOBS,
            );
            format_defaults(ft, null_mut(), None, None, NonNull::new(wp));
            tty_window_default_style(&raw mut (*wp).cached_active_gc, wp);
            style_add(
                &raw mut (*wp).cached_active_gc,
                oo,
                c!("window-active-style"),
                ft,
            );
            tty_window_default_style(&raw mut (*wp).cached_gc, wp);
            style_add(&raw mut (*wp).cached_gc, oo, c!("window-style"), ft);
            format_free(ft);
        }

        if (*gc).fg == 8 {
            if wp == (*(*wp).window).active && (*wp).cached_active_gc.fg != 8 {
                (*gc).fg = (*wp).cached_active_gc.fg;
            } else {
                (*gc).fg = (*wp).cached_gc.fg;
            }
        }

        if (*gc).bg == 8 {
            if wp == (*(*wp).window).active && (*wp).cached_active_gc.bg != 8 {
                (*gc).bg = (*wp).cached_active_gc.bg;
            } else {
                (*gc).bg = (*wp).cached_gc.bg;
            }
        }
    }
}

pub unsafe fn tty_default_attributes(
    tty: *mut tty,
    defaults: *const grid_cell,
    palette: *const colour_palette,
    bg: u32,
    hl: *mut hyperlinks,
) {
    unsafe {
        let mut gc: grid_cell = zeroed();
        memcpy__(&raw mut gc, &raw const GRID_DEFAULT_CELL);
        gc.bg = bg as i32;
        tty_attributes(tty, &gc, defaults, palette, hl);
    }
}

pub unsafe extern "C-unwind" fn tty_clipboard_query_callback(
    _fd: i32,
    _events: i16,
    tty: NonNull<tty>,
) {
    unsafe {
        let c = (*tty.as_ptr()).client;

        (*c).flags &= !client_flag::CLIPBOARDBUFFER;
        free_((*c).clipboard_panes);
        (*c).clipboard_panes = null_mut();
        (*c).clipboard_npanes = 0;

        (*tty.as_ptr()).flags &= !tty_flags::TTY_OSC52QUERY;
    }
}

pub unsafe fn tty_clipboard_query(tty: *mut tty) {
    unsafe {
        let tv = libc::timeval {
            tv_sec: TTY_QUERY_TIMEOUT as i64,
            tv_usec: 0,
        };

        if (!(*tty).flags.intersects(tty_flags::TTY_STARTED))
            || ((*tty).flags.intersects(tty_flags::TTY_OSC52QUERY))
        {
            return;
        }
        tty_putcode_ss(tty, tty_code_code::TTYC_MS, c!(""), c!("?"));

        (*tty).flags |= tty_flags::TTY_OSC52QUERY;
        evtimer_set(
            &raw mut (*tty).clipboard_timer,
            tty_clipboard_query_callback,
            NonNull::new_unchecked(tty),
        );
        evtimer_add(&raw mut (*tty).clipboard_timer, &tv);
    }
}

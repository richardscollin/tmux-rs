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
use crate::libc::sscanf;
use crate::*;

pub unsafe fn resize_window(w: *mut window, mut sx: u32, mut sy: u32, xpixel: i32, ypixel: i32) {
    unsafe {
        // Check size limits.
        sx = sx.clamp(WINDOW_MINIMUM, WINDOW_MAXIMUM);
        sy = sy.clamp(WINDOW_MINIMUM, WINDOW_MAXIMUM);

        /* If the window is zoomed, unzoom. */
        let zoomed = (*w).flags.intersects(window_flag::ZOOMED);
        if zoomed {
            window_unzoom(w, 1);
        }

        /* Resize the layout first. */
        layout_resize(w, sx, sy);

        /* Resize the window, it can be no smaller than the layout. */
        if sx < (*(*w).layout_root).sx {
            sx = (*(*w).layout_root).sx;
        }
        if sy < (*(*w).layout_root).sy {
            sy = (*(*w).layout_root).sy;
        }
        window_resize(w, sx, sy, xpixel, ypixel);
        log_debug!(
            "{}: @{} resized to {}x{}; layout {}x{}",
            "resize_window",
            (*w).id,
            sx,
            sy,
            (*(*w).layout_root).sx,
            (*(*w).layout_root).sy,
        );

        /* Restore the window zoom state. */
        if zoomed {
            window_zoom((*w).active);
        }

        tty_update_window_offset(w);
        server_redraw_window(w);
        notify_window(c"window-layout-changed", w);
        notify_window(c"window-resized", w);
        (*w).flags &= !window_flag::RESIZE;
    }
}

pub unsafe fn ignore_client_size(c: *mut client) -> i32 {
    unsafe {
        if (*c).session.is_null() {
            return 1;
        }
        if (*c).flags.intersects(CLIENT_NOSIZEFLAGS) {
            return 1;
        }
        if (*c).flags.intersects(client_flag::IGNORESIZE) {
            /*
             * Ignore flagged clients if there are any attached clients
             * that aren't flagged.
             */
            for loop_ in tailq_foreach(&raw mut CLIENTS).map(NonNull::as_ptr) {
                if (*loop_).session.is_null() {
                    continue;
                }
                if (*loop_).flags.intersects(CLIENT_NOSIZEFLAGS) {
                    continue;
                }
                if !(*loop_).flags.intersects(client_flag::IGNORESIZE) {
                    return 1;
                }
            }
        }
        if (*c).flags.intersects(client_flag::CONTROL)
            && !(*c).flags.intersects(client_flag::SIZECHANGED)
            && !(*c).flags.intersects(client_flag::WINDOWSIZECHANGED)
        {
            return 1;
        }
        0
    }
}

pub unsafe fn clients_with_window(w: *mut window) -> u32 {
    let mut n = 0u32;
    unsafe {
        for loop_ in tailq_foreach(&raw mut CLIENTS).map(NonNull::as_ptr) {
            if ignore_client_size(loop_) != 0 || session_has((*loop_).session, w) == 0 {
                continue;
            }
            n += 1;
            if n > 1 {
                break;
            }
        }
    }
    n
}

#[expect(clippy::type_complexity)]
pub unsafe fn clients_calculate_size(
    type_: window_size_option,
    current: i32,
    c: *mut client,
    s: *mut session,
    w: *mut window,
    skip_client: Option<
        unsafe fn(*mut client, window_size_option, i32, *mut session, *mut window) -> i32,
    >,
    sx: *mut u32,
    sy: *mut u32,
    xpixel: *mut u32,
    ypixel: *mut u32,
) -> i32 {
    let mut cx;
    let mut cy;
    let mut cw;
    let mut n = 0;
    let __func__ = "clients_calculate_size";

    unsafe {
        'skip: {
            /*
             * Start comparing with 0 for largest and UINT_MAX for smallest or
             * latest.
             */
            if type_ == window_size_option::WINDOW_SIZE_LARGEST {
                *sx = 0;
                *sy = 0;
            } else if type_ == window_size_option::WINDOW_SIZE_MANUAL {
                *sx = (*w).manual_sx;
                *sy = (*w).manual_sy;
                log_debug!("{}: manual size {}x{}", __func__, *sx, *sy);
            } else {
                *sx = u32::MAX;
                *sy = u32::MAX;
            }
            *xpixel = 0;
            *ypixel = 0;

            /*
             * For latest, count the number of clients with this window. We only
             * care if there is more than one.
             */
            if type_ == window_size_option::WINDOW_SIZE_LATEST && !w.is_null() {
                n = clients_with_window(w);
            }

            /* Skip setting the size if manual */
            if type_ == window_size_option::WINDOW_SIZE_MANUAL {
                break 'skip;
            }

            /* loop_ over the clients and work out the size. */
            for loop_ in tailq_foreach(&raw mut CLIENTS).map(NonNull::as_ptr) {
                if loop_ != c && ignore_client_size(loop_) != 0 {
                    log_debug!("{}: ignoring {} (1)", __func__, _s((*loop_).name));
                    continue;
                }
                if loop_ != c && skip_client.unwrap()(loop_, type_, current, s, w) != 0 {
                    log_debug!("{}: skipping {} (1)", __func__, _s((*loop_).name));
                    continue;
                }

                /*
                 * If there are multiple clients attached, only accept the
                 * latest client; otherwise let the only client be chosen as
                 * for smallest.
                 */
                if type_ == window_size_option::WINDOW_SIZE_LATEST
                    && n > 1
                    && loop_ != (*w).latest.cast()
                {
                    log_debug!("{}: {} is not latest", __func__, _s((*loop_).name));
                    continue;
                }

                /*
                 * If the client has a per-window size, use this instead if it is
                 * smaller.
                 */
                if !w.is_null() {
                    cw = server_client_get_client_window(loop_, (*w).id);
                } else {
                    cw = null_mut();
                }

                /* Work out this client's size. */
                if !cw.is_null() && (*cw).sx != 0 && (*cw).sy != 0 {
                    cx = (*cw).sx;
                    cy = (*cw).sy;
                } else {
                    cx = (*loop_).tty.sx;
                    cy = (*loop_).tty.sy - status_line_size(loop_);
                }

                /*
                 * If it is larger or smaller than the best so far, update the
                 * new size.
                 */
                if type_ == window_size_option::WINDOW_SIZE_LARGEST {
                    if cx > *sx {
                        *sx = cx;
                    }
                    if cy > *sy {
                        *sy = cy;
                    }
                } else {
                    if cx < *sx {
                        *sx = cx;
                    }
                    if cy < *sy {
                        *sy = cy;
                    }
                }
                if (*loop_).tty.xpixel > *xpixel && (*loop_).tty.ypixel > *ypixel {
                    *xpixel = (*loop_).tty.xpixel;
                    *ypixel = (*loop_).tty.ypixel;
                }
                log_debug!(
                    "{}: after {} ({}x{}), size is {}x{}",
                    __func__,
                    _s((*loop_).name),
                    cx,
                    cy,
                    *sx,
                    *sy,
                );
            }
            if *sx != u32::MAX && *sy != u32::MAX {
                log_debug!("{}: calculated size {}x{}", __func__, *sx, *sy);
            } else {
                log_debug!("{}: no calculated size", __func__);
            }
        }
        // skip:
        /*
         * Do not allow any size to be larger than the per-client window size
         * if one exists.
         */
        if w.is_null() {
            for loop_ in tailq_foreach(&raw mut CLIENTS).map(NonNull::as_ptr) {
                if loop_ != c && ignore_client_size(loop_) != 0 {
                    continue;
                }
                if loop_ != c && skip_client.unwrap()(loop_, type_, current, s, w) != 0 {
                    continue;
                }

                /* Look up per-window size if any. */
                if !(*loop_).flags.intersects(client_flag::WINDOWSIZECHANGED) {
                    continue;
                }
                cw = server_client_get_client_window(loop_, (*w).id);
                if cw.is_null() {
                    continue;
                }

                /* Clamp the size. */
                log_debug!(
                    "{}: {} size for @{} is {}x{}",
                    __func__,
                    _s((*loop_).name),
                    (*w).id,
                    (*cw).sx,
                    (*cw).sy,
                );
                if (*cw).sx != 0 && *sx > (*cw).sx {
                    *sx = (*cw).sx;
                }
                if (*cw).sy != 0 && *sy > (*cw).sy {
                    *sy = (*cw).sy;
                }
            }
        }
        if *sx != u32::MAX && *sy != u32::MAX {
            log_debug!("{}: calculated size {}x{}", __func__, *sx, *sy);
        } else {
            log_debug!("{}: no calculated size", __func__);
        }

        /* Return whether a suitable size was found. */
        if type_ == window_size_option::WINDOW_SIZE_MANUAL {
            log_debug!("{}: type_ is manual", __func__);
            return 1;
        }
        if type_ == window_size_option::WINDOW_SIZE_LARGEST {
            log_debug!("{}: type_ is largest", __func__);
            return (*sx != 0 && *sy != 0) as i32;
        }
        if type_ == window_size_option::WINDOW_SIZE_LATEST {
            log_debug!("{}: type_ is latest", __func__);
        } else {
            log_debug!("{}: type_ is smallest", __func__);
        }
        (*sx != u32::MAX && *sy != u32::MAX) as i32
    }
}

pub unsafe fn default_window_size_skip_client(
    loop_: *mut client,
    type_: window_size_option,
    _current: i32,
    s: *mut session,
    w: *mut window,
) -> i32 {
    unsafe {
        /*
         * Latest checks separately, so do not check here. Otherwise only
         * include clients where the session contains the window or where the
         * session is the given session.
         */
        if type_ == window_size_option::WINDOW_SIZE_LATEST {
            return 0;
        }
        if !w.is_null() && session_has((*loop_).session, w) == 0 {
            return 1;
        }
        if w.is_null() && (*loop_).session != s {
            return 1;
        }
    }
    0
}

#[allow(clippy::too_many_arguments)]
pub unsafe fn default_window_size(
    mut c: *mut client,
    s: *mut session,
    w: *mut window,
    sx: *mut u32,
    sy: *mut u32,
    xpixel: *mut u32,
    ypixel: *mut u32,
    type_: Option<window_size_option>,
) {
    let __func__ = "default_window_size";
    unsafe {
        'done: {
            // const char *value;

            /* Get type_ if not provided. */
            let type_ = type_.unwrap_or_else(|| {
                window_size_option::try_from(
                    options_get_number_(GLOBAL_W_OPTIONS, "window-size") as i32
                )
                .unwrap()
            });

            /*
             * Latest clients can use the given client if suitable. If there is no
             * client and no window, use the default size as for manual type_.
             */
            if type_ == window_size_option::WINDOW_SIZE_LATEST
                && !c.is_null()
                && ignore_client_size(c) == 0
            {
                *sx = (*c).tty.sx;
                *sy = (*c).tty.sy - status_line_size(c);
                *xpixel = (*c).tty.xpixel;
                *ypixel = (*c).tty.ypixel;
                log_debug!("{}: using {}x{} from {}", __func__, *sx, *sy, _s((*c).name));
                break 'done;
            }

            /*
             * Ignore the given client if it is a control client - the creating
             * client should only affect the size if it is not a control client.
             */
            if !c.is_null() && ((*c).flags.intersects(client_flag::CONTROL)) {
                c = null_mut();
            }

            /*
             * Look for a client to base the size on. If none exists (or the type_
             * is manual), use the default-size option.
             */
            if clients_calculate_size(
                type_,
                0,
                c,
                s,
                w,
                Some(default_window_size_skip_client),
                sx,
                sy,
                xpixel,
                ypixel,
            ) == 0
            {
                let value = options_get_string_((*s).options, "default-size");
                if sscanf(value.cast(), c"%ux%u".as_ptr(), sx, sy) != 2 {
                    *sx = 80;
                    *sy = 24;
                }
                log_debug!("{}: using {}x{} from default-size", __func__, *sx, *sy);
            }
        }
        // done:
        /* Make sure the limits are enforced. */
        *sx = (*sx).clamp(WINDOW_MINIMUM, WINDOW_MAXIMUM);
        *sy = (*sy).clamp(WINDOW_MINIMUM, WINDOW_MAXIMUM);
        log_debug!("{}: resulting size is {}x{}", __func__, *sx, *sy);
    }
}

pub unsafe fn recalculate_size_skip_client(
    loop_: *mut client,
    _type_: window_size_option,
    current: i32,
    _s: *mut session,
    w: *mut window,
) -> i32 {
    unsafe {
        /*
         * If the current flag is set, then skip any client where this window
         * is not the current window - this is used for aggressive-resize.
         * Otherwise skip any session that doesn't contain the window.
         */
        if (*(*loop_).session).curw.is_null() {
            return 1;
        }
        if current != 0 {
            return ((*(*(*loop_).session).curw).window != w) as i32;
        }

        (session_has((*loop_).session, w) == 0) as i32
    }
}

pub unsafe fn recalculate_size(w: *mut window, now: i32) {
    let __func__ = "recalculate_size";

    unsafe {
        let mut sx = 0;
        let mut sy = 0;
        let mut xpixel = 0;
        let mut ypixel = 0;
        // u_int sx, sy, xpixel = 0, ypixel = 0;
        // int type, current, changed;

        /*
         * Do not attempt to resize windows which have no pane, they must be on
         * the way to destruction.
         */
        if (*w).active.is_null() {
            return;
        }
        log_debug!("{}: @{} is {}x{}", __func__, (*w).id, (*w).sx, (*w).sy);

        /*
         * type_ is manual, smallest, largest, latest. Current is the
         * aggressive-resize option (do not resize based on clients where the
         * window is not the current window).
         */
        let type_ =
            window_size_option::try_from(options_get_number_((*w).options, "window-size") as i32)
                .unwrap();
        let current = options_get_number_((*w).options, "aggressive-resize") as i32;

        /* Look for a suitable client and get the new size. */
        let mut changed = clients_calculate_size(
            type_,
            current,
            null_mut(),
            null_mut(),
            w,
            Some(recalculate_size_skip_client),
            &raw mut sx,
            &raw mut sy,
            &raw mut xpixel,
            &raw mut ypixel,
        );

        /*
         * Make sure the size has actually changed. If the window has already
         * got a resize scheduled, then use the new size; otherwise the old.
         */
        if (*w).flags.intersects(window_flag::RESIZE) {
            if now == 0 && changed != 0 && (*w).new_sx == sx && (*w).new_sy == sy {
                changed = 0;
            }
        } else if now == 0 && changed != 0 && (*w).sx == sx && (*w).sy == sy {
            changed = 0;
        }

        /*
         * If the size hasn't changed, update the window offset but not the
         * size.
         */
        if changed == 0 {
            log_debug!("{}: @{} no size change", __func__, (*w).id);
            tty_update_window_offset(w);
            return;
        }

        /*
         * If the now flag is set or if the window is sized manually, change
         * the size immediately. Otherwise set the flag and it will be done
         * later.
         */
        log_debug!("{}: @{} new size {}x{}", __func__, (*w).id, sx, sy);
        if now != 0 || type_ == window_size_option::WINDOW_SIZE_MANUAL {
            resize_window(w, sx, sy, xpixel as i32, ypixel as i32);
        } else {
            (*w).new_sx = sx;
            (*w).new_sy = sy;
            (*w).new_xpixel = xpixel;
            (*w).new_ypixel = ypixel;

            (*w).flags |= window_flag::RESIZE;
            tty_update_window_offset(w);
        }
    }
}

pub unsafe fn recalculate_sizes() {
    unsafe {
        recalculate_sizes_now(0);
    }
}

pub unsafe fn recalculate_sizes_now(now: i32) {
    unsafe {
        // struct session *s;
        // struct client *c;
        // struct window *w;

        /*
         * Clear attached count and update saved status line information for
         * each session.
         */
        for s in rb_foreach(&raw mut SESSIONS).map(NonNull::as_ptr) {
            (*s).attached = 0;
            status_update_cache(s);
        }

        /*
         * Increment attached count and check the status line size for each
         * client.
         */
        for c in tailq_foreach(&raw mut CLIENTS).map(NonNull::as_ptr) {
            let s = (*c).session;
            if !s.is_null() && !((*c).flags.intersects(CLIENT_UNATTACHEDFLAGS)) {
                (*s).attached += 1;
            }
            if ignore_client_size(c) != 0 {
                continue;
            }
            if (*c).tty.sy <= (*s).statuslines || ((*c).flags.intersects(client_flag::CONTROL)) {
                (*c).flags |= client_flag::STATUSOFF;
            } else {
                (*c).flags &= !client_flag::STATUSOFF;
            }
        }

        /* Walk each window and adjust the size. */
        for w in rb_foreach(&raw mut WINDOWS) {
            recalculate_size(w.as_ptr(), now);
        }
    }
}

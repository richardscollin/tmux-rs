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
use crate::compat::imsg::{IMSG_HEADER_SIZE, MAX_IMSGSIZE};
use crate::libc::{WEXITSTATUS, WIFEXITED, close, gettimeofday};
use crate::*;
use crate::options_::*;

pub unsafe fn server_redraw_client(c: *mut client) {
    unsafe {
        (*c).flags |= CLIENT_ALLREDRAWFLAGS;
    }
}

pub unsafe fn server_status_client(c: *mut client) {
    unsafe {
        (*c).flags |= client_flag::REDRAWSTATUS;
    }
}

pub unsafe fn server_redraw_session(s: *mut session) {
    unsafe {
        for c in tailq_foreach(&raw mut CLIENTS).map(NonNull::as_ptr) {
            if (*c).session == s {
                server_redraw_client(c);
            }
        }
    }
}

pub unsafe fn server_redraw_session_group(s: *mut session) {
    unsafe {
        let sg = session_group_contains(s);
        if sg.is_null() {
            server_redraw_session(s);
        } else {
            for s in tailq_foreach(&raw mut (*sg).sessions) {
                server_redraw_session(s.as_ptr());
            }
        }
    }
}

pub unsafe fn server_status_session(s: *mut session) {
    unsafe {
        for c in tailq_foreach(&raw mut CLIENTS).map(NonNull::as_ptr) {
            if (*c).session == s {
                server_status_client(c);
            }
        }
    }
}

pub unsafe fn server_status_session_group(s: *mut session) {
    unsafe {
        let sg = session_group_contains(s);
        if sg.is_null() {
            server_status_session(s);
        } else {
            for s in tailq_foreach(&raw mut (*sg).sessions) {
                server_status_session(s.as_ptr());
            }
        }
    }
}

pub unsafe fn server_redraw_window(w: *mut window) {
    unsafe {
        for c in tailq_foreach(&raw mut CLIENTS).map(NonNull::as_ptr) {
            if !(*c).session.is_null() && (*(*(*c).session).curw).window == w {
                server_redraw_client(c);
            }
        }
    }
}

pub unsafe fn server_redraw_window_borders(w: *mut window) {
    unsafe {
        for c in tailq_foreach(&raw mut CLIENTS).map(NonNull::as_ptr) {
            if !(*c).session.is_null() && (*(*(*c).session).curw).window == w {
                (*c).flags |= client_flag::REDRAWBORDERS;
            }
        }
    }
}

pub unsafe fn server_status_window(w: *mut window) {
    unsafe {
        // This is slightly different. We want to redraw the status line of any
        // clients containing this window rather than anywhere it is the
        // current window.

        for s in rb_foreach(&raw mut SESSIONS).map(NonNull::as_ptr) {
            if session_has(s, w) {
                server_status_session(s);
            }
        }
    }
}

pub unsafe fn server_lock() {
    unsafe {
        for c in tailq_foreach(&raw mut CLIENTS).map(NonNull::as_ptr) {
            if !(*c).session.is_null() {
                server_lock_client(c);
            }
        }
    }
}

pub unsafe fn server_lock_session(s: *mut session) {
    unsafe {
        for c in tailq_foreach(&raw mut CLIENTS).map(NonNull::as_ptr) {
            if (*c).session == s {
                server_lock_client(c);
            }
        }
    }
}

pub unsafe fn server_lock_client(c: *mut client) {
    unsafe {
        if (*c).flags.intersects(client_flag::CONTROL) {
            return;
        }

        if (*c).flags.intersects(client_flag::SUSPENDED) {
            return;
        }

        let cmd = options_get_string_((*(*c).session).options, "lock-command");
        if *cmd == b'\0' || strlen(cmd) + 1 > MAX_IMSGSIZE - IMSG_HEADER_SIZE {
            return;
        }

        tty_stop_tty(&raw mut (*c).tty);
        tty_raw(
            &raw mut (*c).tty,
            tty_term_string((*c).tty.term, tty_code_code::TTYC_SMCUP),
        );
        tty_raw(
            &raw mut (*c).tty,
            tty_term_string((*c).tty.term, tty_code_code::TTYC_CLEAR),
        );
        tty_raw(
            &raw mut (*c).tty,
            tty_term_string((*c).tty.term, tty_code_code::TTYC_E3),
        );

        (*c).flags |= client_flag::SUSPENDED;
        proc_send(
            (*c).peer,
            msgtype::MSG_LOCK,
            -1,
            cmd.cast(),
            strlen(cmd) + 1,
        );
    }
}

pub unsafe fn server_kill_pane(wp: *mut window_pane) {
    unsafe {
        let w = (*wp).window;

        if window_count_panes(w) == 1 {
            server_kill_window(w, 1);
            recalculate_sizes();
        } else {
            server_unzoom_window(w);
            server_client_remove_pane(wp);
            layout_close_pane(wp);
            window_remove_pane(w, wp);
            server_redraw_window(w);
        }
    }
}

pub unsafe fn server_kill_window(w: *mut window, renumber: i32) {
    unsafe {
        for s in rb_foreach(&raw mut SESSIONS).map(NonNull::as_ptr) {
            if !session_has(s, w) {
                continue;
            }

            server_unzoom_window(w);
            while let Some(wl) = winlink_find_by_window(&raw mut (*s).windows, w) {
                if session_detach(s, wl.as_ptr()) != 0 {
                    server_destroy_session_group(s);
                    break;
                }
                server_redraw_session_group(s);
            }

            if renumber != 0 {
                server_renumber_session(s);
            }
        }

        recalculate_sizes();
    }
}

pub unsafe fn server_renumber_session(s: *mut session) {
    unsafe {
        if options_get_number_((*s).options, "renumber-windows") != 0 {
            let sg = session_group_contains(s);
            if !sg.is_null() {
                for s in tailq_foreach(&raw mut (*sg).sessions) {
                    session_renumber_windows(s.as_ptr());
                }
            } else {
                session_renumber_windows(s);
            }
        }
    }
}

pub unsafe fn server_renumber_all() {
    unsafe {
        for s in rb_foreach(&raw mut SESSIONS) {
            server_renumber_session(s.as_ptr());
        }
    }
}

pub unsafe fn server_link_window(
    src: *mut session,
    srcwl: *mut winlink,
    dst: *mut session,
    mut dstidx: i32,
    killflag: bool,
    mut selectflag: bool,
    cause: *mut *mut u8,
) -> i32 {
    unsafe {
        let mut dstwl = null_mut();

        let srcsg = session_group_contains(src);
        let dstsg = session_group_contains(dst);
        if src != dst && !srcsg.is_null() && !dstsg.is_null() && srcsg == dstsg {
            *cause = format_nul!("sessions are grouped");
            return -1;
        }

        if dstidx != -1 {
            dstwl = winlink_find_by_index(&raw mut (*dst).windows, dstidx);
        }
        if !dstwl.is_null() {
            if (*dstwl).window == (*srcwl).window {
                *cause = format_nul!("same index: {}", dstidx);
                return -1;
            }
            if killflag {
                // Can't use session_detach as it will destroy session
                // if this makes it empty.
                notify_session_window(c"window-unlinked", dst, (*dstwl).window);
                (*dstwl).flags &= !WINLINK_ALERTFLAGS;
                winlink_stack_remove(&raw mut (*dst).lastw, dstwl);
                winlink_remove(&raw mut (*dst).windows, dstwl);

                // Force select/redraw if current.
                if dstwl == (*dst).curw {
                    selectflag = true;
                    (*dst).curw = null_mut();
                }
            }
        }

        if dstidx == -1 {
            dstidx = -1 - options_get_number_((*dst).options, "base-index") as i32;
        }
        dstwl = session_attach(dst, (*srcwl).window, dstidx, cause);
        if dstwl.is_null() {
            return -1;
        }

        if selectflag {
            session_select(dst, (*dstwl).idx);
        }
        server_redraw_session_group(dst);

        0
    }
}

pub unsafe fn server_unlink_window(s: *mut session, wl: *mut winlink) {
    unsafe {
        if session_detach(s, wl) != 0 {
            server_destroy_session_group(s);
        } else {
            server_redraw_session_group(s);
        }
    }
}

pub unsafe fn server_destroy_pane(wp: *mut window_pane, notify: i32) {
    unsafe {
        let w = (*wp).window;
        let mut ctx: MaybeUninit<screen_write_ctx> = MaybeUninit::<screen_write_ctx>::uninit();
        let ctx = ctx.as_mut_ptr();

        let mut gc: MaybeUninit<grid_cell> = MaybeUninit::<grid_cell>::uninit();
        let gc = gc.as_mut_ptr();

        let sx = screen_size_x(&raw mut (*wp).base);
        let sy = screen_size_y(&raw mut (*wp).base);

        if (*wp).fd != -1 {
            #[cfg(feature = "utempter")]
            {
                utempter_remove_record((*wp).fd);
            }
            bufferevent_free((*wp).event);
            (*wp).event = null_mut();
            close((*wp).fd);
            (*wp).fd = -1;
        }

        let remain_on_exit = options_get_number_((*wp).options, "remain-on-exit");
        if remain_on_exit != 0 && !(*wp).flags.intersects(window_pane_flags::PANE_STATUSREADY) {
            return;
        }
        'out: {
            match remain_on_exit {
                0 => (),
                1 | 2 => {
                    if remain_on_exit == 2
                        && WIFEXITED((*wp).status)
                        && WEXITSTATUS((*wp).status) == 0
                    {
                        break 'out;
                    }
                    if (*wp).flags.intersects(window_pane_flags::PANE_STATUSDRAWN) {
                        return;
                    }
                    (*wp).flags |= window_pane_flags::PANE_STATUSDRAWN;

                    gettimeofday(&raw mut (*wp).dead_time, null_mut());
                    if notify != 0 {
                        notify_pane(c"pane-died", wp);
                    }

                    let s = options_get_string_((*wp).options, "remain-on-exit-format");
                    if *s != b'\0' {
                        screen_write_start_pane(ctx, wp, &raw mut (*wp).base);
                        screen_write_scrollregion(ctx, 0, sy - 1);
                        screen_write_cursormove(ctx, 0, sy as i32 - 1, 0);
                        screen_write_linefeed(ctx, true, 8);
                        memcpy_(gc, &raw const GRID_DEFAULT_CELL, size_of::<grid_cell>());

                        let expanded =
                            format_single(null_mut(), cstr_to_str(s), null_mut(), null_mut(), null_mut(), wp);
                        format_draw(ctx, gc, sx, cstr_to_str(expanded), null_mut(), 0);
                        free_(expanded);

                        screen_write_stop(ctx);
                    }
                    (*wp).base.mode &= !mode_flag::MODE_CURSOR;

                    (*wp).flags |= window_pane_flags::PANE_REDRAW;
                    return;
                }
                _ => (),
            }
        }

        if notify != 0 {
            notify_pane(c"pane-exited", wp);
        }

        server_unzoom_window(w);
        server_client_remove_pane(wp);
        layout_close_pane(wp);
        window_remove_pane(w, wp);

        if tailq_empty(&raw mut (*w).panes) {
            server_kill_window(w, 1);
        } else {
            server_redraw_window(w);
        }
    }
}

pub unsafe fn server_destroy_session_group(sess: *mut session) {
    unsafe {
        let sg = session_group_contains(sess);
        if sg.is_null() {
            server_destroy_session(sess);
            session_destroy(sess, 1, c!("server_destroy_session_group"));
        } else {
            for s in tailq_foreach(&raw mut (*sg).sessions).map(NonNull::as_ptr) {
                server_destroy_session(s);
                session_destroy(s, 1, c!("server_destroy_session_group"));
            }
        }
    }
}

pub unsafe fn server_find_session(
    s: *mut session,
    f: unsafe fn(*mut session, *mut session) -> i32,
) -> *mut session {
    unsafe {
        let mut s_out: *mut session = null_mut();
        for s_loop in rb_foreach(&raw mut SESSIONS).map(NonNull::as_ptr) {
            if s_loop != s && (s_out.is_null() || f(s_loop, s_out) != 0) {
                s_out = s_loop;
            }
        }
        s_out
    }
}

pub unsafe fn server_newer_session(s_loop: *mut session, s_out: *mut session) -> i32 {
    unsafe {
        (timer::new(&raw const (*s_loop).activity_time)
            > timer::new(&raw const (*s_out).activity_time)) as i32
    }
}

pub unsafe fn server_newer_detached_session(s_loop: *mut session, s_out: *mut session) -> i32 {
    unsafe {
        if (*s_loop).attached != 0 {
            return 0;
        }
        server_newer_session(s_loop, s_out)
    }
}

pub unsafe fn server_destroy_session(s: *mut session) {
    unsafe {
        let detach_on_destroy = options_get_number_((*s).options, "detach-on-destroy");

        let mut s_new: *mut session = if detach_on_destroy == 0 {
            server_find_session(s, server_newer_session)
        } else if detach_on_destroy == 2 {
            server_find_session(s, server_newer_detached_session)
        } else if detach_on_destroy == 3 {
            session_previous_session(s)
        } else if detach_on_destroy == 4 {
            session_next_session(s)
        } else {
            null_mut()
        };

        if s_new == s {
            s_new = null_mut();
        }
        for c in tailq_foreach(&raw mut CLIENTS).map(NonNull::as_ptr) {
            if (*c).session != s {
                continue;
            }
            (*c).session = null_mut();
            (*c).last_session = null_mut();
            server_client_set_session(c, s_new);
            if s_new.is_null() {
                (*c).flags |= client_flag::EXIT;
            }
        }
        recalculate_sizes();
    }
}

pub unsafe fn server_check_unattached() {
    unsafe {
        for s in rb_foreach(&raw mut SESSIONS).map(NonNull::as_ptr) {
            if (*s).attached != 0 {
                continue;
            }
            match options_get_number_((*s).options, "destroy-unattached") {
                0 => continue, // off
                1 => (),       // on
                2 => {
                    // keep-last
                    let sg = session_group_contains(s);
                    if sg.is_null() || session_group_count(sg) <= 1 {
                        continue;
                    }
                }
                3 => {
                    // keep-group
                    let sg = session_group_contains(s);
                    if !sg.is_null() && session_group_count(sg) == 1 {
                        continue;
                    }
                }
                _ => (),
            }
            session_destroy(s, 1, c!("server_check_unattached"));
        }
    }
}

pub unsafe fn server_unzoom_window(w: *mut window) {
    unsafe {
        if window_unzoom(w, 1) == 0 {
            server_redraw_window(w);
        }
    }
}

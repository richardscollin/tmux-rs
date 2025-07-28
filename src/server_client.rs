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
use crate::{
    compat::imsg::{IMSG_HEADER_SIZE, imsg_get_fd},
    options_::options_get_number_,
};

/// Compare client windows.
pub fn server_client_window_cmp(cw1: &client_window, cw2: &client_window) -> std::cmp::Ordering {
    cw1.window.cmp(&cw2.window)
}

/// Number of attached clients.
pub unsafe fn server_client_how_many() -> u32 {
    unsafe {
        tailq_foreach(&raw mut CLIENTS)
            .filter(|c| {
                !(*c.as_ptr()).session.is_null()
                    && !(*c.as_ptr()).flags.intersects(CLIENT_UNATTACHEDFLAGS)
            })
            .count() as u32
    }
}

/// Overlay timer callback.
pub unsafe extern "C-unwind" fn server_client_overlay_timer(
    _fd: i32,
    _events: i16,
    c: NonNull<client>,
) {
    unsafe {
        server_client_clear_overlay(c.as_ptr());
    }
}

/// Set an overlay on client.
pub unsafe fn server_client_set_overlay(
    c: *mut client,
    delay: u32,
    checkcb: overlay_check_cb,
    modecb: overlay_mode_cb,
    drawcb: overlay_draw_cb,
    keycb: overlay_key_cb,
    freecb: overlay_free_cb,
    resizecb: overlay_resize_cb,
    data: *mut c_void,
) {
    unsafe {
        if (*c).overlay_draw.is_some() {
            server_client_clear_overlay(c);
        }

        let tv: libc::timeval = libc::timeval {
            tv_sec: delay as i64 / 1000,
            tv_usec: ((delay % 1000) * 1000) as libc::suseconds_t,
        };

        if event_initialized(&raw mut (*c).overlay_timer) != 0 {
            evtimer_del(&raw mut (*c).overlay_timer);
        }
        evtimer_set(
            &raw mut (*c).overlay_timer,
            server_client_overlay_timer,
            NonNull::new_unchecked(c),
        );
        if delay != 0 {
            evtimer_add(&raw mut (*c).overlay_timer, &tv);
        }

        (*c).overlay_check = checkcb;
        (*c).overlay_mode = modecb;
        (*c).overlay_draw = drawcb;
        (*c).overlay_key = keycb;
        (*c).overlay_free = freecb;
        (*c).overlay_resize = resizecb;
        (*c).overlay_data = data;

        if (*c).overlay_check.is_none() {
            (*c).tty.flags |= tty_flags::TTY_FREEZE;
        }
        if (*c).overlay_mode.is_none() {
            (*c).tty.flags |= tty_flags::TTY_NOCURSOR;
        }
        server_redraw_client(c);
    }
}

/// Clear overlay mode on client.
pub unsafe fn server_client_clear_overlay(c: *mut client) {
    unsafe {
        if (*c).overlay_draw.is_none() {
            return;
        }

        if event_initialized(&raw mut (*c).overlay_timer) != 0 {
            evtimer_del(&raw mut (*c).overlay_timer);
        }

        if let Some(overlay_free) = (*c).overlay_free {
            overlay_free(c, (*c).overlay_data);
        }

        (*c).overlay_check = None;
        (*c).overlay_mode = None;
        (*c).overlay_draw = None;
        (*c).overlay_key = None;
        (*c).overlay_free = None;
        (*c).overlay_data = null_mut();

        (*c).tty.flags &= !(tty_flags::TTY_FREEZE | tty_flags::TTY_NOCURSOR);
        server_redraw_client(c);
    }
}

/// Given overlay position and dimensions, return parts of the input range which are visible.
pub unsafe fn server_client_overlay_range(
    x: u32,
    y: u32,
    sx: u32,
    sy: u32,
    px: u32,
    py: u32,
    nx: u32,
    r: *mut overlay_ranges,
) {
    unsafe {
        // Return up to 2 ranges.
        (*r).px[2] = 0;
        (*r).nx[2] = 0;

        // Trivial case of no overlap in the y direction.
        if py < y || py > y + sy - 1 {
            (*r).px[0] = px;
            (*r).nx[0] = nx;
            (*r).px[1] = 0;
            (*r).nx[1] = 0;
            return;
        }

        // Visible bit to the left of the popup.
        if px < x {
            (*r).px[0] = px;
            (*r).nx[0] = x - px;
            if (*r).nx[0] > nx {
                (*r).nx[0] = nx;
            }
        } else {
            (*r).px[0] = 0;
            (*r).nx[0] = 0;
        }

        // Visible bit to the right of the popup.
        let mut ox = x + sx;
        if px > ox {
            ox = px;
        }
        let onx = px + nx;
        if onx > ox {
            (*r).px[1] = ox;
            (*r).nx[1] = onx - ox;
        } else {
            (*r).px[1] = 0;
            (*r).nx[1] = 0;
        }
    }
}

/// Check if this client is inside this server.
pub unsafe fn server_client_check_nested(c: *mut client) -> i32 {
    unsafe {
        let envent = environ_find((*c).environ, c!("TMUX"));
        if envent.is_null() || *transmute_ptr((*envent).value) == b'\0' {
            return 0;
        }

        for wp in rb_foreach(&raw mut ALL_WINDOW_PANES) {
            if libc::strcmp((&raw const (*wp.as_ptr()).tty) as _, (*c).ttyname) == 0 {
                return 1;
            }
        }
        0
    }
}

/// Set client key table.
pub unsafe fn server_client_set_key_table(c: *mut client, mut name: *const u8) {
    unsafe {
        if name.is_null() {
            name = server_client_get_key_table(c);
        }

        key_bindings_unref_table((*c).keytable);
        (*c).keytable = key_bindings_get_table(name, 1);
        (*(*c).keytable).references += 1;
        if libc::gettimeofday(&raw mut (*(*c).keytable).activity_time, null_mut()) != 0 {
            fatal("gettimeofday failed");
        }
    }
}

pub unsafe fn server_client_key_table_activity_diff(c: *mut client) -> u64 {
    unsafe {
        let mut diff: libc::timeval = zeroed();
        timersub(
            &raw const (*c).activity_time,
            &raw const (*(*c).keytable).activity_time,
            &raw mut diff,
        );
        (diff.tv_sec as u64 * 1000u64) + (diff.tv_usec as u64 / 1000u64)
    }
}

/// Get default key table.
pub unsafe fn server_client_get_key_table(c: *mut client) -> *const u8 {
    unsafe {
        let s = (*c).session;
        if s.is_null() {
            return c!("root");
        }

        let name = options_get_string_((*s).options, "key-table");
        if *name == b'\0' {
            return c!("root");
        }
        name
    }
}

/// Is this table the default key table?
pub unsafe fn server_client_is_default_key_table(c: *mut client, table: *mut key_table) -> i32 {
    unsafe { (libc::strcmp((*table).name, server_client_get_key_table(c)) == 0) as i32 }
}

/// Create a new client.
pub unsafe fn server_client_create(fd: i32) -> *mut client {
    unsafe {
        setblocking(fd, 0);

        let c: *mut client = xcalloc1();
        (*c).references = 1;
        (*c).peer = proc_add_peer(SERVER_PROC, fd, Some(server_client_dispatch), c.cast());

        if libc::gettimeofday(&raw mut (*c).creation_time, null_mut()) != 0 {
            fatal("gettimeofday failed");
        }
        memcpy__(&raw mut (*c).activity_time, &raw mut (*c).creation_time);

        (*c).environ = environ_create().as_ptr();

        (*c).fd = -1;
        (*c).out_fd = -1;

        (*c).queue = cmdq_new().as_ptr();
        rb_init(&raw mut (*c).windows);
        rb_init(&raw mut (*c).files);

        (*c).tty.sx = 80;
        (*c).tty.sy = 24;

        status_init(c);
        (*c).flags |= client_flag::FOCUSED;

        (*c).keytable = key_bindings_get_table(c!("root"), 1);
        (*(*c).keytable).references += 1;

        evtimer_set(
            &raw mut (*c).repeat_timer,
            server_client_repeat_timer,
            NonNull::new_unchecked(c),
        );
        evtimer_set(
            &raw mut (*c).click_timer,
            server_client_click_timer,
            NonNull::new_unchecked(c),
        );

        tailq_insert_tail(&raw mut CLIENTS, c);
        log_debug!("new client {:p}", c);
        c
    }
}

/// Open client terminal if needed.
pub unsafe fn server_client_open(c: *mut client, cause: *mut *mut u8) -> i32 {
    unsafe {
        let mut ttynam = _PATH_TTY;

        if (*c).flags.intersects(client_flag::CONTROL) {
            return 0;
        }

        if libc::strcmp((*c).ttyname, ttynam) == 0
            || ((libc::isatty(libc::STDIN_FILENO) != 0
                && ({
                    ttynam = libc::ttyname(libc::STDIN_FILENO);
                    !ttynam.is_null()
                })
                && libc::strcmp((*c).ttyname, ttynam) == 0)
                || (libc::isatty(libc::STDOUT_FILENO) != 0
                    && ({
                        ttynam = libc::ttyname(libc::STDOUT_FILENO);
                        !ttynam.is_null()
                    })
                    && libc::strcmp((*c).ttyname, ttynam) == 0)
                || (libc::isatty(libc::STDERR_FILENO) != 0
                    && ({
                        ttynam = libc::ttyname(libc::STDERR_FILENO);
                        !ttynam.is_null()
                    })
                    && libc::strcmp((*c).ttyname, ttynam) == 0))
        {
            *cause = format_nul!("can't use {}", _s((*c).ttyname));
            return -1;
        }

        if !(*c).flags.intersects(client_flag::TERMINAL) {
            *cause = xstrdup(c!("not a terminal")).as_ptr();
            return -1;
        }

        if tty_open(&raw mut (*c).tty, cause) != 0 {
            return -1;
        }

        0
    }
}

/// Lost an attached client.
pub unsafe fn server_client_attached_lost(c: *mut client) {
    unsafe {
        log_debug!("lost attached client {:p}", c);

        // By this point the session in the client has been cleared so walk all
        // windows to find any with this client as the latest.
        for w in rb_foreach(&raw mut WINDOWS).map(NonNull::as_ptr) {
            if (*w).latest.cast() != c {
                continue;
            }

            let mut found: *mut client = null_mut();
            for loop_ in tailq_foreach(&raw mut CLIENTS).map(NonNull::as_ptr) {
                let s = (*loop_).session;
                if loop_ == c || s.is_null() || (*(*s).curw).window != w {
                    continue;
                }
                if found.is_null()
                    || timer::new(&raw const (*loop_).activity_time)
                        > timer::new(&raw const (*found).activity_time)
                {
                    found = loop_;
                }
            }
            if !found.is_null() {
                server_client_update_latest(found);
            }
        }
    }
}

/// Set client session.
pub unsafe fn server_client_set_session(c: *mut client, s: *mut session) {
    unsafe {
        let old = (*c).session;

        if !s.is_null() && !(*c).session.is_null() && (*c).session != s {
            (*c).last_session = (*c).session;
        } else if s.is_null() {
            (*c).last_session = null_mut();
        }
        (*c).session = s;
        (*c).flags |= client_flag::FOCUSED;

        if !old.is_null() && !(*old).curw.is_null() {
            window_update_focus((*(*old).curw).window);
        }
        if !s.is_null() {
            recalculate_sizes();
            window_update_focus((*(*s).curw).window);
            session_update_activity(s, null_mut());
            libc::gettimeofday(&raw mut (*s).last_attached_time, null_mut());
            (*(*s).curw).flags &= !WINLINK_ALERTFLAGS;
            (*(*(*s).curw).window).latest = c.cast();
            alerts_check_session(s);
            tty_update_client_offset(c);
            status_timer_start(NonNull::new_unchecked(c));
            notify_client(c"client-session-changed", c);
            server_redraw_client(c);
        }

        server_check_unattached();
        server_update_socket();
    }
}

/// Lost a client.
pub unsafe fn server_client_lost(c: *mut client) {
    unsafe {
        (*c).flags |= client_flag::DEAD;

        server_client_clear_overlay(c);
        status_prompt_clear(c);
        status_message_clear(NonNull::new_unchecked(c));

        for cf in rb_foreach(&raw mut (*c).files).map(NonNull::as_ptr) {
            (*cf).error = libc::EINTR;
            file_fire_done(cf);
        }
        for cw in rb_foreach(&raw mut (*c).windows).map(NonNull::as_ptr) {
            rb_remove(&raw mut (*c).windows, cw);
            free_(cw);
        }

        tailq_remove(&raw mut CLIENTS, c);
        log_debug!("lost client {:p}", c);

        if (*c).flags.intersects(client_flag::ATTACHED) {
            server_client_attached_lost(c);
            notify_client(c"client-detached", c);
        }

        if (*c).flags.intersects(client_flag::CONTROL) {
            control_stop(c);
        }
        if (*c).flags.intersects(client_flag::TERMINAL) {
            tty_free(&raw mut (*c).tty);
        }
        free_((*c).ttyname);
        free_((*c).clipboard_panes);

        free_((*c).term_name);
        free_((*c).term_type);
        tty_term_free_list((*c).term_caps, (*c).term_ncaps);

        status_free(c);

        free_((*c).title);
        free_((*c).cwd.cast_mut()); // TODO cast away const

        evtimer_del(&raw mut (*c).repeat_timer);
        evtimer_del(&raw mut (*c).click_timer);

        key_bindings_unref_table((*c).keytable);

        free_((*c).message_string);
        if event_initialized(&raw mut (*c).message_timer) != 0 {
            evtimer_del(&raw mut (*c).message_timer);
        }

        free_((*c).prompt_saved);
        free_((*c).prompt_string);
        free_((*c).prompt_buffer);

        format_lost_client(c);
        environ_free((*c).environ);

        proc_remove_peer((*c).peer);
        (*c).peer = null_mut();

        if (*c).out_fd != -1 {
            libc::close((*c).out_fd);
        }
        if (*c).fd != -1 {
            libc::close((*c).fd);
            (*c).fd = -1;
        }
        server_client_unref(c);

        server_add_accept(0); /* may be more file descriptors now */

        recalculate_sizes();
        server_check_unattached();
        server_update_socket();
    }
}

/// Remove reference from a client.
pub unsafe fn server_client_unref(c: *mut client) {
    unsafe {
        log_debug!("unref client {:p} ({} references)", c, (*c).references);

        (*c).references -= 1;
        if (*c).references == 0 {
            event_once(
                -1,
                EV_TIMEOUT,
                Some(server_client_free),
                c.cast(),
                null_mut(),
            );
        }
    }
}

/// Free dead client.
pub unsafe extern "C-unwind" fn server_client_free(_fd: i32, _events: i16, arg: *mut c_void) {
    unsafe {
        let c: *mut client = arg.cast();
        log_debug!("free client {:p} ({} references)", c, (*c).references);

        cmdq_free((*c).queue);

        if (*c).references == 0 {
            free_((*c).name.cast_mut());
            free_(c);
        }
    }
}

/// Suspend a client.
pub unsafe fn server_client_suspend(c: *mut client) {
    unsafe {
        let s: *mut session = (*c).session;

        if s.is_null() || (*c).flags.intersects(CLIENT_UNATTACHEDFLAGS) {
            return;
        }

        tty_stop_tty(&raw mut (*c).tty);
        (*c).flags |= client_flag::SUSPENDED;
        proc_send((*c).peer, msgtype::MSG_SUSPEND, -1, null_mut(), 0);
    }
}

/// Detach a client.
pub unsafe fn server_client_detach(c: *mut client, msgtype: msgtype) {
    unsafe {
        let s = (*c).session;

        if s.is_null() || (*c).flags.intersects(CLIENT_NODETACHFLAGS) {
            return;
        }

        (*c).flags |= client_flag::EXIT;

        (*c).exit_type = exit_type::CLIENT_EXIT_DETACH;
        (*c).exit_msgtype = msgtype;
        (*c).exit_session = xstrdup((*s).name).as_ptr();
    }
}

/// Execute command to replace a client.
pub unsafe fn server_client_exec(c: *mut client, cmd: *const u8) {
    unsafe {
        let s = (*c).session;
        if *cmd == b'\0' {
            return;
        }
        let cmdsize = strlen(cmd) + 1;

        let mut shell = if !s.is_null() {
            options_get_string_((*s).options, "default-shell")
        } else {
            options_get_string_(GLOBAL_S_OPTIONS, "default-shell")
        };
        if !checkshell(shell) {
            shell = _PATH_BSHELL;
        }
        let shellsize = strlen(shell) + 1;

        let msg: *mut u8 = xmalloc(cmdsize + shellsize).as_ptr().cast();
        libc::memcpy(msg.cast(), cmd.cast(), cmdsize);
        libc::memcpy(msg.add(cmdsize).cast(), shell.cast(), shellsize);

        proc_send(
            (*c).peer,
            msgtype::MSG_EXEC,
            -1,
            msg.cast(),
            cmdsize + shellsize,
        );
        free_(msg);
    }
}

/// Check for mouse keys.
pub unsafe fn server_client_check_mouse(c: *mut client, event: *mut key_event) -> key_code {
    unsafe {
        let m = &raw mut (*event).m;
        let s = (*c).session;
        let fs: *mut session;

        let fwl: *mut winlink;
        let fwp: *mut window_pane;

        // u_int x, y, b, sx, sy, px, py;
        let mut x: u32 = 0;
        let mut y: u32 = 0;
        let mut b: u32 = 0;
        let mut sx: u32 = 0;
        let mut sy: u32 = 0;
        let mut px: u32;
        let mut py: u32;

        let mut ignore = 0;

        let mut key: key_code = 0;
        let mut tv: libc::timeval = zeroed();
        let sr: *mut style_range;

        #[derive(Copy, Clone, Eq, PartialEq)]
        enum type_ {
            NoType,
            Move,
            Down,
            Up,
            Drag,
            Wheel,
            Second,
            Double,
            Triple,
        }
        let mut type_ = type_::NoType;

        #[derive(Copy, Clone, Eq, PartialEq)]
        enum where_ {
            Nowhere,
            Pane,
            Status,
            StatusLeft,
            StatusRight,
            StatusDefault,
            Border,
        }
        let mut where_ = where_::Nowhere;

        'out: {
            'have_event: {
                // log_debug("%s mouse %02x at %u,%u (last %u,%u) (%d)", (*c).name, (*m).b, (*m).x, (*m).y, (*m).lx, (*m).ly, (*c).tty.mouse_drag_flag);

                /* What type of event is this? */
                if (*event).key == keyc::KEYC_DOUBLECLICK as u64 {
                    type_ = type_::Double;
                    x = (*m).x;
                    y = (*m).y;
                    b = (*m).b;
                    ignore = 1;
                    // log_debug("double-click at %u,%u", x, y);
                } else if ((*m).sgr_type != b' ' as u32
                    && MOUSE_DRAG((*m).sgr_b)
                    && MOUSE_RELEASE((*m).sgr_b))
                    || ((*m).sgr_type == b' ' as u32
                        && MOUSE_DRAG((*m).b)
                        && MOUSE_RELEASE((*m).b)
                        && MOUSE_RELEASE((*m).lb))
                {
                    type_ = type_::Move;
                    x = (*m).x;
                    y = (*m).y;
                    b = 0;
                    log_debug!("move at {x},{y}");
                } else if MOUSE_DRAG((*m).b) {
                    type_ = type_::Drag;
                    if (*c).tty.mouse_drag_flag != 0 {
                        x = (*m).x;
                        y = (*m).y;
                        b = (*m).b;
                        if x == (*m).lx && y == (*m).ly {
                            return KEYC_UNKNOWN;
                        }
                        log_debug!("drag update at {x},{y}");
                    } else {
                        x = (*m).lx;
                        y = (*m).ly;
                        b = (*m).lb;
                        log_debug!("drag start at {x},{y}");
                    }
                } else if MOUSE_WHEEL((*m).b) {
                    type_ = type_::Wheel;
                    x = (*m).x;
                    y = (*m).y;
                    b = (*m).b;
                    log_debug!("wheel at {},{}", x, y);
                } else if MOUSE_RELEASE((*m).b) {
                    type_ = type_::Up;
                    x = (*m).x;
                    y = (*m).y;
                    b = (*m).lb;
                    if (*m).sgr_type == b'm' as u32 {
                        b = (*m).sgr_b;
                    }
                    log_debug!("up at {},{}", x, y);
                } else {
                    if (*c).flags.intersects(client_flag::DOUBLECLICK) {
                        evtimer_del(&raw mut (*c).click_timer);
                        (*c).flags &= !client_flag::DOUBLECLICK;
                        if (*m).b == (*c).click_button {
                            type_ = type_::Second;
                            x = (*m).x;
                            y = (*m).y;
                            b = (*m).b;
                            log_debug!("second-click at {},{}", x, y);
                            (*c).flags |= client_flag::TRIPLECLICK;
                        }
                    } else if (*c).flags.intersects(client_flag::TRIPLECLICK) {
                        evtimer_del(&raw mut (*c).click_timer);
                        (*c).flags &= !client_flag::TRIPLECLICK;
                        if (*m).b == (*c).click_button {
                            type_ = type_::Triple;
                            x = (*m).x;
                            y = (*m).y;
                            b = (*m).b;
                            log_debug!("triple-click at {},{}", x, y);
                            break 'have_event;
                        }
                    }

                    /* DOWN is the only remaining event type. */
                    if type_ == type_::NoType {
                        type_ = type_::Down;
                        x = (*m).x;
                        y = (*m).y;
                        b = (*m).b;
                        log_debug!("down at {},{}", x, y);
                        (*c).flags |= client_flag::DOUBLECLICK;
                    }

                    if KEYC_CLICK_TIMEOUT != 0 {
                        memcpy__(&raw mut (*c).click_event, m);
                        (*c).click_button = (*m).b;

                        log_debug!("click timer started");
                        tv.tv_sec = KEYC_CLICK_TIMEOUT as i64 / 1000;
                        tv.tv_usec = ((KEYC_CLICK_TIMEOUT % 1000) * 1000) as libc::suseconds_t;
                        evtimer_del(&raw mut (*c).click_timer);
                        evtimer_add(&raw mut (*c).click_timer, &raw const tv);
                    }
                }
            } // have_event:
            if type_ == type_::NoType {
                return KEYC_UNKNOWN;
            }

            /* Save the session. */
            (*m).s = (*s).id as i32;
            (*m).w = -1;
            (*m).wp = -1;
            (*m).ignore = ignore;

            /* Is this on the status line? */
            (*m).statusat = status_at_line(c);
            (*m).statuslines = status_line_size(c);
            if (*m).statusat != -1
                && y >= (*m).statusat as u32
                && y < (*m).statusat as u32 + (*m).statuslines
            {
                sr = status_get_range(c, x, y - (*m).statusat as u32);
                if sr.is_null() {
                    where_ = where_::StatusDefault;
                } else {
                    match (*sr).type_ {
                        style_range_type::STYLE_RANGE_NONE => return KEYC_UNKNOWN,
                        style_range_type::STYLE_RANGE_LEFT => {
                            log_debug!("mouse range: left");
                            where_ = where_::StatusLeft;
                        }
                        style_range_type::STYLE_RANGE_RIGHT => {
                            log_debug!("mouse range: right");
                            where_ = where_::StatusRight;
                        }
                        style_range_type::STYLE_RANGE_PANE => {
                            fwp = window_pane_find_by_id((*sr).argument);
                            if fwp.is_null() {
                                return KEYC_UNKNOWN;
                            }
                            (*m).wp = (*sr).argument as i32;

                            log_debug!("mouse range: pane %%{}", (*m).wp);
                            where_ = where_::Status;
                        }
                        style_range_type::STYLE_RANGE_WINDOW => {
                            fwl =
                                winlink_find_by_index(&raw mut (*s).windows, (*sr).argument as i32);
                            if fwl.is_null() {
                                return KEYC_UNKNOWN;
                            }
                            (*m).w = (*(*fwl).window).id as i32;

                            log_debug!("mouse range: window @{}", (*m).w);
                            where_ = where_::Status;
                        }
                        style_range_type::STYLE_RANGE_SESSION => {
                            fs = transmute_ptr(session_find_by_id((*sr).argument));
                            if fs.is_null() {
                                return KEYC_UNKNOWN;
                            }
                            (*m).s = (*sr).argument as i32;

                            log_debug!("mouse range: session ${}", (*m).s);
                            where_ = where_::Status;
                        }
                        style_range_type::STYLE_RANGE_USER => where_ = where_::Status,
                    }
                }
            }

            /* Not on status line. Adjust position and check for border or pane. */
            if where_ == where_::Nowhere {
                px = x;
                if (*m).statusat == 0 && y >= (*m).statuslines {
                    py = y - (*m).statuslines;
                } else if (*m).statusat > 0 && y >= (*m).statusat as u32 {
                    py = (*m).statusat as u32 - 1;
                } else {
                    py = y;
                }

                tty_window_offset(
                    &raw mut (*c).tty,
                    &raw mut (*m).ox,
                    &raw mut (*m).oy,
                    &raw mut sx,
                    &raw mut sy,
                );
                // log_debug!("mouse window @%u at %u,%u (%ux%u)", (*(*(*s).curw).window).id, (*m).ox, (*m).oy, sx, sy);
                if px > sx || py > sy {
                    return KEYC_UNKNOWN;
                }
                px += (*m).ox;
                py += (*m).oy;

                let mut wp = null_mut();

                // Try the pane borders if not zoomed.
                if !(*(*(*s).curw).window).flags.intersects(window_flag::ZOOMED)
                    && let Some(wp_) = tailq_foreach::<_, discr_entry>(
                        &raw mut (*(*(*s).curw).window).panes,
                    )
                    .find(|wp| {
                        let wp = wp.as_ptr();
                        ((*wp).xoff + (*wp).sx == px
                            && (*wp).yoff <= 1 + py
                            && (*wp).yoff + (*wp).sy >= py)
                            || ((*wp).yoff + (*wp).sy == py
                                && (*wp).xoff <= 1 + px
                                && (*wp).xoff + (*wp).sx >= px)
                    })
                {
                    wp = wp_.as_ptr();
                    where_ = where_::Border;
                }

                // Otherwise try inside the pane.
                if where_ == where_::Nowhere {
                    wp = window_get_active_at((*(*s).curw).window, px, py);
                    if !wp.is_null() {
                        where_ = where_::Pane;
                    } else {
                        return KEYC_UNKNOWN;
                    }
                }
                if where_ == where_::Pane {
                    log_debug!("mouse {},{} on pane %%{}", x, y, (*wp).id);
                } else if where_ == where_::Border {
                    log_debug!("mouse on pane %%{} border", (*wp).id);
                }
                (*m).wp = (*wp).id as i32;
                (*m).w = (*(*wp).window).id as i32;
            }

            // Stop dragging if needed.
            if !matches!(type_, type_::Drag | type_::Wheel) && (*c).tty.mouse_drag_flag != 0 {
                if let Some(mouse_drag_release) = (*c).tty.mouse_drag_release {
                    mouse_drag_release(c, m);
                }

                (*c).tty.mouse_drag_update = None;
                (*c).tty.mouse_drag_release = None;

                // End a mouse drag by passing a MouseDragEnd key corresponding to the button that started the drag.
                match ((*c).tty.mouse_drag_flag - 1) as u32 {
                    crate::MOUSE_BUTTON_1 => {
                        key = match where_ {
                            where_::Pane => keyc::KEYC_MOUSEDRAGEND1_PANE as u64,
                            where_::Status => keyc::KEYC_MOUSEDRAGEND1_STATUS as u64,
                            where_::StatusLeft => keyc::KEYC_MOUSEDRAGEND1_STATUS_LEFT as u64,
                            where_::StatusRight => keyc::KEYC_MOUSEDRAGEND1_STATUS_RIGHT as u64,
                            where_::StatusDefault => keyc::KEYC_MOUSEDRAGEND1_STATUS_DEFAULT as u64,
                            where_::Border => keyc::KEYC_MOUSEDRAGEND1_BORDER as u64,
                            where_::Nowhere => key,
                        }
                    }
                    crate::MOUSE_BUTTON_2 => {
                        key = match where_ {
                            where_::Pane => keyc::KEYC_MOUSEDRAGEND2_PANE as u64,
                            where_::Status => keyc::KEYC_MOUSEDRAGEND2_STATUS as u64,
                            where_::StatusLeft => keyc::KEYC_MOUSEDRAGEND2_STATUS_LEFT as u64,
                            where_::StatusRight => keyc::KEYC_MOUSEDRAGEND2_STATUS_RIGHT as u64,
                            where_::StatusDefault => keyc::KEYC_MOUSEDRAGEND2_STATUS_DEFAULT as u64,
                            where_::Border => keyc::KEYC_MOUSEDRAGEND2_BORDER as u64,
                            where_::Nowhere => key,
                        }
                    }
                    crate::MOUSE_BUTTON_3 => {
                        key = match where_ {
                            where_::Pane => keyc::KEYC_MOUSEDRAGEND3_PANE as u64,
                            where_::Status => keyc::KEYC_MOUSEDRAGEND3_STATUS as u64,
                            where_::StatusLeft => keyc::KEYC_MOUSEDRAGEND3_STATUS_LEFT as u64,
                            where_::StatusRight => keyc::KEYC_MOUSEDRAGEND3_STATUS_RIGHT as u64,
                            where_::StatusDefault => keyc::KEYC_MOUSEDRAGEND3_STATUS_DEFAULT as u64,
                            where_::Border => keyc::KEYC_MOUSEDRAGEND3_BORDER as u64,
                            where_::Nowhere => key,
                        }
                    }
                    crate::MOUSE_BUTTON_6 => {
                        key = match where_ {
                            where_::Pane => keyc::KEYC_MOUSEDRAGEND6_PANE as u64,
                            where_::Status => keyc::KEYC_MOUSEDRAGEND6_STATUS as u64,
                            where_::StatusLeft => keyc::KEYC_MOUSEDRAGEND6_STATUS_LEFT as u64,
                            where_::StatusRight => keyc::KEYC_MOUSEDRAGEND6_STATUS_RIGHT as u64,
                            where_::StatusDefault => keyc::KEYC_MOUSEDRAGEND6_STATUS_DEFAULT as u64,
                            where_::Border => keyc::KEYC_MOUSEDRAGEND6_BORDER as u64,
                            where_::Nowhere => key,
                        }
                    }
                    crate::MOUSE_BUTTON_7 => {
                        key = match where_ {
                            where_::Pane => keyc::KEYC_MOUSEDRAGEND7_PANE as u64,
                            where_::Status => keyc::KEYC_MOUSEDRAGEND7_STATUS as u64,
                            where_::StatusLeft => keyc::KEYC_MOUSEDRAGEND7_STATUS_LEFT as u64,
                            where_::StatusRight => keyc::KEYC_MOUSEDRAGEND7_STATUS_RIGHT as u64,
                            where_::StatusDefault => keyc::KEYC_MOUSEDRAGEND7_STATUS_DEFAULT as u64,
                            where_::Border => keyc::KEYC_MOUSEDRAGEND7_BORDER as u64,
                            where_::Nowhere => key,
                        }
                    }
                    crate::MOUSE_BUTTON_8 => {
                        key = match where_ {
                            where_::Pane => keyc::KEYC_MOUSEDRAGEND8_PANE as u64,
                            where_::Status => keyc::KEYC_MOUSEDRAGEND8_STATUS as u64,
                            where_::StatusLeft => keyc::KEYC_MOUSEDRAGEND8_STATUS_LEFT as u64,
                            where_::StatusRight => keyc::KEYC_MOUSEDRAGEND8_STATUS_RIGHT as u64,
                            where_::StatusDefault => keyc::KEYC_MOUSEDRAGEND8_STATUS_DEFAULT as u64,
                            where_::Border => keyc::KEYC_MOUSEDRAGEND8_BORDER as u64,
                            where_::Nowhere => key,
                        }
                    }
                    crate::MOUSE_BUTTON_9 => {
                        key = match where_ {
                            where_::Pane => keyc::KEYC_MOUSEDRAGEND9_PANE as u64,
                            where_::Status => keyc::KEYC_MOUSEDRAGEND9_STATUS as u64,
                            where_::StatusLeft => keyc::KEYC_MOUSEDRAGEND9_STATUS_LEFT as u64,
                            where_::StatusRight => keyc::KEYC_MOUSEDRAGEND9_STATUS_RIGHT as u64,
                            where_::StatusDefault => keyc::KEYC_MOUSEDRAGEND9_STATUS_DEFAULT as u64,
                            where_::Border => keyc::KEYC_MOUSEDRAGEND9_BORDER as u64,
                            where_::Nowhere => key,
                        }
                    }
                    crate::MOUSE_BUTTON_10 => {
                        key = match where_ {
                            where_::Pane => keyc::KEYC_MOUSEDRAGEND10_PANE as u64,
                            where_::Status => keyc::KEYC_MOUSEDRAGEND10_STATUS as u64,
                            where_::StatusLeft => keyc::KEYC_MOUSEDRAGEND10_STATUS_LEFT as u64,
                            where_::StatusRight => keyc::KEYC_MOUSEDRAGEND10_STATUS_RIGHT as u64,
                            where_::StatusDefault => {
                                keyc::KEYC_MOUSEDRAGEND10_STATUS_DEFAULT as u64
                            }
                            where_::Border => keyc::KEYC_MOUSEDRAGEND10_BORDER as u64,
                            where_::Nowhere => key,
                        }
                    }
                    crate::MOUSE_BUTTON_11 => {
                        key = match where_ {
                            where_::Pane => keyc::KEYC_MOUSEDRAGEND11_PANE as u64,
                            where_::Status => keyc::KEYC_MOUSEDRAGEND11_STATUS as u64,
                            where_::StatusLeft => keyc::KEYC_MOUSEDRAGEND11_STATUS_LEFT as u64,
                            where_::StatusRight => keyc::KEYC_MOUSEDRAGEND11_STATUS_RIGHT as u64,
                            where_::StatusDefault => {
                                keyc::KEYC_MOUSEDRAGEND11_STATUS_DEFAULT as u64
                            }
                            where_::Border => keyc::KEYC_MOUSEDRAGEND11_BORDER as u64,
                            where_::Nowhere => key,
                        }
                    }
                    _ => key = keyc::KEYC_MOUSE as u64,
                }
                (*c).tty.mouse_drag_flag = 0;
                break 'out;
            }

            // Convert to a key binding.
            key = KEYC_UNKNOWN;
            match type_ {
                type_::NoType => (),
                type_::Move => {
                    key = match where_ {
                        where_::Pane => keyc::KEYC_MOUSEMOVE_PANE as u64,
                        where_::Status => keyc::KEYC_MOUSEMOVE_STATUS as u64,
                        where_::StatusLeft => keyc::KEYC_MOUSEMOVE_STATUS_LEFT as u64,
                        where_::StatusRight => keyc::KEYC_MOUSEMOVE_STATUS_RIGHT as u64,
                        where_::StatusDefault => keyc::KEYC_MOUSEMOVE_STATUS_DEFAULT as u64,
                        where_::Border => keyc::KEYC_MOUSEMOVE_BORDER as u64,
                        where_::Nowhere => key,
                    };
                }
                type_::Drag => {
                    if (*c).tty.mouse_drag_update.is_some() {
                        key = keyc::KEYC_DRAGGING as u64;
                    } else {
                        match MOUSE_BUTTONS(b) {
                            crate::MOUSE_BUTTON_1 => {
                                key = match where_ {
                                    where_::Pane => keyc::KEYC_MOUSEDRAG1_PANE as u64,
                                    where_::Status => keyc::KEYC_MOUSEDRAG1_STATUS as u64,
                                    where_::StatusLeft => keyc::KEYC_MOUSEDRAG1_STATUS_LEFT as u64,
                                    where_::StatusRight => {
                                        keyc::KEYC_MOUSEDRAG1_STATUS_RIGHT as u64
                                    }
                                    where_::StatusDefault => {
                                        keyc::KEYC_MOUSEDRAG1_STATUS_DEFAULT as u64
                                    }
                                    where_::Border => keyc::KEYC_MOUSEDRAG1_BORDER as u64,
                                    where_::Nowhere => key,
                                };
                            }
                            crate::MOUSE_BUTTON_2 => {
                                key = match where_ {
                                    where_::Pane => keyc::KEYC_MOUSEDRAG2_PANE as u64,
                                    where_::Status => keyc::KEYC_MOUSEDRAG2_STATUS as u64,
                                    where_::StatusLeft => keyc::KEYC_MOUSEDRAG2_STATUS_LEFT as u64,
                                    where_::StatusRight => {
                                        keyc::KEYC_MOUSEDRAG2_STATUS_RIGHT as u64
                                    }
                                    where_::StatusDefault => {
                                        keyc::KEYC_MOUSEDRAG2_STATUS_DEFAULT as u64
                                    }
                                    where_::Border => keyc::KEYC_MOUSEDRAG2_BORDER as u64,
                                    where_::Nowhere => key,
                                };
                            }
                            crate::MOUSE_BUTTON_3 => {
                                key = match where_ {
                                    where_::Pane => keyc::KEYC_MOUSEDRAG3_PANE as u64,
                                    where_::Status => keyc::KEYC_MOUSEDRAG3_STATUS as u64,
                                    where_::StatusLeft => keyc::KEYC_MOUSEDRAG3_STATUS_LEFT as u64,
                                    where_::StatusRight => {
                                        keyc::KEYC_MOUSEDRAG3_STATUS_RIGHT as u64
                                    }
                                    where_::StatusDefault => {
                                        keyc::KEYC_MOUSEDRAG3_STATUS_DEFAULT as u64
                                    }
                                    where_::Border => keyc::KEYC_MOUSEDRAG3_BORDER as u64,
                                    where_::Nowhere => key,
                                };
                            }
                            crate::MOUSE_BUTTON_6 => {
                                key = match where_ {
                                    where_::Pane => keyc::KEYC_MOUSEDRAG6_PANE as u64,
                                    where_::Status => keyc::KEYC_MOUSEDRAG6_STATUS as u64,
                                    where_::StatusLeft => keyc::KEYC_MOUSEDRAG6_STATUS_LEFT as u64,
                                    where_::StatusRight => {
                                        keyc::KEYC_MOUSEDRAG6_STATUS_RIGHT as u64
                                    }
                                    where_::StatusDefault => {
                                        keyc::KEYC_MOUSEDRAG6_STATUS_DEFAULT as u64
                                    }
                                    where_::Border => keyc::KEYC_MOUSEDRAG6_BORDER as u64,
                                    where_::Nowhere => key,
                                };
                            }
                            crate::MOUSE_BUTTON_7 => {
                                key = match where_ {
                                    where_::Pane => keyc::KEYC_MOUSEDRAG7_PANE as u64,
                                    where_::Status => keyc::KEYC_MOUSEDRAG7_STATUS as u64,
                                    where_::StatusLeft => keyc::KEYC_MOUSEDRAG7_STATUS_LEFT as u64,
                                    where_::StatusRight => {
                                        keyc::KEYC_MOUSEDRAG7_STATUS_RIGHT as u64
                                    }
                                    where_::StatusDefault => {
                                        keyc::KEYC_MOUSEDRAG7_STATUS_DEFAULT as u64
                                    }
                                    where_::Border => keyc::KEYC_MOUSEDRAG7_BORDER as u64,
                                    where_::Nowhere => key,
                                };
                            }
                            crate::MOUSE_BUTTON_8 => {
                                key = match where_ {
                                    where_::Pane => keyc::KEYC_MOUSEDRAG8_PANE as u64,
                                    where_::Status => keyc::KEYC_MOUSEDRAG8_STATUS as u64,
                                    where_::StatusLeft => keyc::KEYC_MOUSEDRAG8_STATUS_LEFT as u64,
                                    where_::StatusRight => {
                                        keyc::KEYC_MOUSEDRAG8_STATUS_RIGHT as u64
                                    }
                                    where_::StatusDefault => {
                                        keyc::KEYC_MOUSEDRAG8_STATUS_DEFAULT as u64
                                    }
                                    where_::Border => keyc::KEYC_MOUSEDRAG8_BORDER as u64,
                                    where_::Nowhere => key,
                                };
                            }
                            crate::MOUSE_BUTTON_9 => {
                                key = match where_ {
                                    where_::Pane => keyc::KEYC_MOUSEDRAG9_PANE as u64,
                                    where_::Status => keyc::KEYC_MOUSEDRAG9_STATUS as u64,
                                    where_::StatusLeft => keyc::KEYC_MOUSEDRAG9_STATUS_LEFT as u64,
                                    where_::StatusRight => {
                                        keyc::KEYC_MOUSEDRAG9_STATUS_RIGHT as u64
                                    }
                                    where_::StatusDefault => {
                                        keyc::KEYC_MOUSEDRAG9_STATUS_DEFAULT as u64
                                    }
                                    where_::Border => keyc::KEYC_MOUSEDRAG9_BORDER as u64,
                                    where_::Nowhere => key,
                                };
                            }
                            crate::MOUSE_BUTTON_10 => {
                                key = match where_ {
                                    where_::Pane => keyc::KEYC_MOUSEDRAG10_PANE as u64,
                                    where_::Status => keyc::KEYC_MOUSEDRAG10_STATUS as u64,
                                    where_::StatusLeft => keyc::KEYC_MOUSEDRAG10_STATUS_LEFT as u64,
                                    where_::StatusRight => {
                                        keyc::KEYC_MOUSEDRAG10_STATUS_RIGHT as u64
                                    }
                                    where_::StatusDefault => {
                                        keyc::KEYC_MOUSEDRAG10_STATUS_DEFAULT as u64
                                    }
                                    where_::Border => keyc::KEYC_MOUSEDRAG10_BORDER as u64,
                                    where_::Nowhere => key,
                                };
                            }
                            crate::MOUSE_BUTTON_11 => {
                                key = match where_ {
                                    where_::Pane => keyc::KEYC_MOUSEDRAG11_PANE as u64,
                                    where_::Status => keyc::KEYC_MOUSEDRAG11_STATUS as u64,
                                    where_::StatusLeft => keyc::KEYC_MOUSEDRAG11_STATUS_LEFT as u64,
                                    where_::StatusRight => {
                                        keyc::KEYC_MOUSEDRAG11_STATUS_RIGHT as u64
                                    }
                                    where_::StatusDefault => {
                                        keyc::KEYC_MOUSEDRAG11_STATUS_DEFAULT as u64
                                    }
                                    where_::Border => keyc::KEYC_MOUSEDRAG11_BORDER as u64,
                                    where_::Nowhere => key,
                                };
                            }
                            _ => (),
                        }
                    }

                    /*
                     * Begin a drag by setting the flag to a non-zero value that
                     * corresponds to the mouse button in use.
                     */
                    (*c).tty.mouse_drag_flag = MOUSE_BUTTONS(b) as i32 + 1;
                }
                type_::Wheel => {
                    if MOUSE_BUTTONS(b) == MOUSE_WHEEL_UP {
                        key = match where_ {
                            where_::Pane => keyc::KEYC_WHEELUP_PANE as u64,
                            where_::Status => keyc::KEYC_WHEELUP_STATUS as u64,
                            where_::StatusLeft => keyc::KEYC_WHEELUP_STATUS_LEFT as u64,
                            where_::StatusRight => keyc::KEYC_WHEELUP_STATUS_RIGHT as u64,
                            where_::StatusDefault => keyc::KEYC_WHEELUP_STATUS_DEFAULT as u64,
                            where_::Border => keyc::KEYC_WHEELUP_BORDER as u64,
                            where_::Nowhere => key,
                        };
                    } else {
                        key = match where_ {
                            where_::Pane => keyc::KEYC_WHEELDOWN_PANE as u64,
                            where_::Status => keyc::KEYC_WHEELDOWN_STATUS as u64,
                            where_::StatusLeft => keyc::KEYC_WHEELDOWN_STATUS_LEFT as u64,
                            where_::StatusRight => keyc::KEYC_WHEELDOWN_STATUS_RIGHT as u64,
                            where_::StatusDefault => keyc::KEYC_WHEELDOWN_STATUS_DEFAULT as u64,
                            where_::Border => keyc::KEYC_WHEELDOWN_BORDER as u64,
                            where_::Nowhere => key,
                        };
                    }
                }
                type_::Up => {
                    match MOUSE_BUTTONS(b) {
                        crate::MOUSE_BUTTON_1 => {
                            key = match where_ {
                                where_::Pane => keyc::KEYC_MOUSEUP1_PANE as u64,
                                where_::Status => keyc::KEYC_MOUSEUP1_STATUS as u64,
                                where_::StatusLeft => keyc::KEYC_MOUSEUP1_STATUS_LEFT as u64,
                                where_::StatusRight => keyc::KEYC_MOUSEUP1_STATUS_RIGHT as u64,
                                where_::StatusDefault => keyc::KEYC_MOUSEUP1_STATUS_DEFAULT as u64,
                                where_::Border => keyc::KEYC_MOUSEUP1_BORDER as u64,
                                where_::Nowhere => key,
                            };
                        }
                        crate::MOUSE_BUTTON_2 => {
                            key = match where_ {
                                where_::Pane => keyc::KEYC_MOUSEUP2_PANE as u64,
                                where_::Status => keyc::KEYC_MOUSEUP2_STATUS as u64,
                                where_::StatusLeft => keyc::KEYC_MOUSEUP2_STATUS_LEFT as u64,
                                where_::StatusRight => keyc::KEYC_MOUSEUP2_STATUS_RIGHT as u64,
                                where_::StatusDefault => keyc::KEYC_MOUSEUP2_STATUS_DEFAULT as u64,
                                where_::Border => keyc::KEYC_MOUSEUP2_BORDER as u64,
                                where_::Nowhere => key,
                            };
                        }
                        crate::MOUSE_BUTTON_3 => {
                            key = match where_ {
                                where_::Pane => keyc::KEYC_MOUSEUP3_PANE as u64,
                                where_::Status => keyc::KEYC_MOUSEUP3_STATUS as u64,
                                where_::StatusLeft => keyc::KEYC_MOUSEUP3_STATUS_LEFT as u64,
                                where_::StatusRight => keyc::KEYC_MOUSEUP3_STATUS_RIGHT as u64,
                                where_::StatusDefault => keyc::KEYC_MOUSEUP3_STATUS_DEFAULT as u64,
                                where_::Border => keyc::KEYC_MOUSEUP3_BORDER as u64,
                                where_::Nowhere => key,
                            };
                        }
                        crate::MOUSE_BUTTON_6 => {
                            key = match where_ {
                                where_::Pane => keyc::KEYC_MOUSEUP6_PANE as u64,
                                where_::Status => keyc::KEYC_MOUSEUP6_STATUS as u64,
                                where_::StatusLeft => keyc::KEYC_MOUSEUP6_STATUS_LEFT as u64,
                                where_::StatusRight => keyc::KEYC_MOUSEUP6_STATUS_RIGHT as u64,
                                where_::StatusDefault => keyc::KEYC_MOUSEUP6_STATUS_DEFAULT as u64,
                                where_::Border => keyc::KEYC_MOUSEUP6_BORDER as u64,
                                where_::Nowhere => key,
                            };
                        }
                        crate::MOUSE_BUTTON_7 => {
                            key = match where_ {
                                where_::Pane => keyc::KEYC_MOUSEUP7_PANE as u64,
                                where_::Status => keyc::KEYC_MOUSEUP7_STATUS as u64,
                                where_::StatusLeft => keyc::KEYC_MOUSEUP7_STATUS_LEFT as u64,
                                where_::StatusRight => keyc::KEYC_MOUSEUP7_STATUS_RIGHT as u64,
                                where_::StatusDefault => keyc::KEYC_MOUSEUP7_STATUS_DEFAULT as u64,
                                where_::Border => keyc::KEYC_MOUSEUP7_BORDER as u64,
                                where_::Nowhere => key,
                            };
                        }
                        crate::MOUSE_BUTTON_8 => {
                            key = match where_ {
                                where_::Pane => keyc::KEYC_MOUSEUP8_PANE as u64,
                                where_::Status => keyc::KEYC_MOUSEUP8_STATUS as u64,
                                where_::StatusLeft => keyc::KEYC_MOUSEUP8_STATUS_LEFT as u64,
                                where_::StatusRight => keyc::KEYC_MOUSEUP8_STATUS_RIGHT as u64,
                                where_::StatusDefault => keyc::KEYC_MOUSEUP8_STATUS_DEFAULT as u64,
                                where_::Border => keyc::KEYC_MOUSEUP8_BORDER as u64,
                                where_::Nowhere => key,
                            };
                        }
                        crate::MOUSE_BUTTON_9 => {
                            key = match where_ {
                                where_::Pane => keyc::KEYC_MOUSEUP9_PANE as u64,
                                where_::Status => keyc::KEYC_MOUSEUP9_STATUS as u64,
                                where_::StatusLeft => keyc::KEYC_MOUSEUP9_STATUS_LEFT as u64,
                                where_::StatusRight => keyc::KEYC_MOUSEUP9_STATUS_RIGHT as u64,
                                where_::StatusDefault => keyc::KEYC_MOUSEUP9_STATUS_DEFAULT as u64,
                                where_::Border => keyc::KEYC_MOUSEUP9_BORDER as u64,
                                where_::Nowhere => key,
                            };
                        }
                        crate::MOUSE_BUTTON_10 => {
                            // TODO why is this mouseup1 and not mouse up 10, is that a typo?
                            key = match where_ {
                                where_::Pane => keyc::KEYC_MOUSEUP1_PANE as u64,
                                where_::Status => keyc::KEYC_MOUSEUP1_STATUS as u64,
                                where_::StatusLeft => keyc::KEYC_MOUSEUP1_STATUS_LEFT as u64,
                                where_::StatusRight => keyc::KEYC_MOUSEUP1_STATUS_RIGHT as u64,
                                where_::StatusDefault => keyc::KEYC_MOUSEUP1_STATUS_DEFAULT as u64,
                                where_::Border => keyc::KEYC_MOUSEUP1_BORDER as u64,
                                where_::Nowhere => key,
                            };
                        }
                        crate::MOUSE_BUTTON_11 => {
                            key = match where_ {
                                where_::Pane => keyc::KEYC_MOUSEUP11_PANE as u64,
                                where_::Status => keyc::KEYC_MOUSEUP11_STATUS as u64,
                                where_::StatusLeft => keyc::KEYC_MOUSEUP11_STATUS_LEFT as u64,
                                where_::StatusRight => keyc::KEYC_MOUSEUP11_STATUS_RIGHT as u64,
                                where_::StatusDefault => keyc::KEYC_MOUSEUP11_STATUS_DEFAULT as u64,
                                where_::Border => keyc::KEYC_MOUSEUP11_BORDER as u64,
                                where_::Nowhere => key,
                            };
                        }
                        _ => (),
                    }
                }
                type_::Down => match MOUSE_BUTTONS(b) {
                    crate::MOUSE_BUTTON_1 => {
                        key = match where_ {
                            where_::Pane => keyc::KEYC_MOUSEDOWN1_PANE as u64,
                            where_::Status => keyc::KEYC_MOUSEDOWN1_STATUS as u64,
                            where_::StatusLeft => keyc::KEYC_MOUSEDOWN1_STATUS_LEFT as u64,
                            where_::StatusRight => keyc::KEYC_MOUSEDOWN1_STATUS_RIGHT as u64,
                            where_::StatusDefault => keyc::KEYC_MOUSEDOWN1_STATUS_DEFAULT as u64,
                            where_::Border => keyc::KEYC_MOUSEDOWN1_BORDER as u64,
                            where_::Nowhere => key,
                        };
                    }
                    crate::MOUSE_BUTTON_2 => {
                        key = match where_ {
                            where_::Pane => keyc::KEYC_MOUSEDOWN2_PANE as u64,
                            where_::Status => keyc::KEYC_MOUSEDOWN2_STATUS as u64,
                            where_::StatusLeft => keyc::KEYC_MOUSEDOWN2_STATUS_LEFT as u64,
                            where_::StatusRight => keyc::KEYC_MOUSEDOWN2_STATUS_RIGHT as u64,
                            where_::StatusDefault => keyc::KEYC_MOUSEDOWN2_STATUS_DEFAULT as u64,
                            where_::Border => keyc::KEYC_MOUSEDOWN2_BORDER as u64,
                            where_::Nowhere => key,
                        };
                    }
                    crate::MOUSE_BUTTON_3 => {
                        key = match where_ {
                            where_::Pane => keyc::KEYC_MOUSEDOWN3_PANE as u64,
                            where_::Status => keyc::KEYC_MOUSEDOWN3_STATUS as u64,
                            where_::StatusLeft => keyc::KEYC_MOUSEDOWN3_STATUS_LEFT as u64,
                            where_::StatusRight => keyc::KEYC_MOUSEDOWN3_STATUS_RIGHT as u64,
                            where_::StatusDefault => keyc::KEYC_MOUSEDOWN3_STATUS_DEFAULT as u64,
                            where_::Border => keyc::KEYC_MOUSEDOWN3_BORDER as u64,
                            where_::Nowhere => key,
                        };
                    }
                    crate::MOUSE_BUTTON_6 => {
                        key = match where_ {
                            where_::Pane => keyc::KEYC_MOUSEDOWN6_PANE as u64,
                            where_::Status => keyc::KEYC_MOUSEDOWN6_STATUS as u64,
                            where_::StatusLeft => keyc::KEYC_MOUSEDOWN6_STATUS_LEFT as u64,
                            where_::StatusRight => keyc::KEYC_MOUSEDOWN6_STATUS_RIGHT as u64,
                            where_::StatusDefault => keyc::KEYC_MOUSEDOWN6_STATUS_DEFAULT as u64,
                            where_::Border => keyc::KEYC_MOUSEDOWN6_BORDER as u64,
                            where_::Nowhere => key,
                        };
                    }
                    crate::MOUSE_BUTTON_7 => {
                        key = match where_ {
                            where_::Pane => keyc::KEYC_MOUSEDOWN7_PANE as u64,
                            where_::Status => keyc::KEYC_MOUSEDOWN7_STATUS as u64,
                            where_::StatusLeft => keyc::KEYC_MOUSEDOWN7_STATUS_LEFT as u64,
                            where_::StatusRight => keyc::KEYC_MOUSEDOWN7_STATUS_RIGHT as u64,
                            where_::StatusDefault => keyc::KEYC_MOUSEDOWN7_STATUS_DEFAULT as u64,
                            where_::Border => keyc::KEYC_MOUSEDOWN7_BORDER as u64,
                            where_::Nowhere => key,
                        };
                    }
                    crate::MOUSE_BUTTON_8 => {
                        key = match where_ {
                            where_::Pane => keyc::KEYC_MOUSEDOWN8_PANE as u64,
                            where_::Status => keyc::KEYC_MOUSEDOWN8_STATUS as u64,
                            where_::StatusLeft => keyc::KEYC_MOUSEDOWN8_STATUS_LEFT as u64,
                            where_::StatusRight => keyc::KEYC_MOUSEDOWN8_STATUS_RIGHT as u64,
                            where_::StatusDefault => keyc::KEYC_MOUSEDOWN8_STATUS_DEFAULT as u64,
                            where_::Border => keyc::KEYC_MOUSEDOWN8_BORDER as u64,
                            where_::Nowhere => key,
                        };
                    }
                    crate::MOUSE_BUTTON_9 => {
                        key = match where_ {
                            where_::Pane => keyc::KEYC_MOUSEDOWN9_PANE as u64,
                            where_::Status => keyc::KEYC_MOUSEDOWN9_STATUS as u64,
                            where_::StatusLeft => keyc::KEYC_MOUSEDOWN9_STATUS_LEFT as u64,
                            where_::StatusRight => keyc::KEYC_MOUSEDOWN9_STATUS_RIGHT as u64,
                            where_::StatusDefault => keyc::KEYC_MOUSEDOWN9_STATUS_DEFAULT as u64,
                            where_::Border => keyc::KEYC_MOUSEDOWN9_BORDER as u64,
                            where_::Nowhere => key,
                        };
                    }
                    crate::MOUSE_BUTTON_10 => {
                        key = match where_ {
                            where_::Pane => keyc::KEYC_MOUSEDOWN10_PANE as u64,
                            where_::Status => keyc::KEYC_MOUSEDOWN10_STATUS as u64,
                            where_::StatusLeft => keyc::KEYC_MOUSEDOWN10_STATUS_LEFT as u64,
                            where_::StatusRight => keyc::KEYC_MOUSEDOWN10_STATUS_RIGHT as u64,
                            where_::StatusDefault => keyc::KEYC_MOUSEDOWN10_STATUS_DEFAULT as u64,
                            where_::Border => keyc::KEYC_MOUSEDOWN10_BORDER as u64,
                            where_::Nowhere => key,
                        };
                    }
                    crate::MOUSE_BUTTON_11 => {
                        key = match where_ {
                            where_::Pane => keyc::KEYC_MOUSEDOWN11_PANE as u64,
                            where_::Status => keyc::KEYC_MOUSEDOWN11_STATUS as u64,
                            where_::StatusLeft => keyc::KEYC_MOUSEDOWN11_STATUS_LEFT as u64,
                            where_::StatusRight => keyc::KEYC_MOUSEDOWN11_STATUS_RIGHT as u64,
                            where_::StatusDefault => keyc::KEYC_MOUSEDOWN11_STATUS_DEFAULT as u64,
                            where_::Border => keyc::KEYC_MOUSEDOWN11_BORDER as u64,
                            where_::Nowhere => key,
                        };
                    }
                    _ => (),
                },
                type_::Second => match MOUSE_BUTTONS(b) {
                    crate::MOUSE_BUTTON_1 => {
                        key = match where_ {
                            where_::Pane => keyc::KEYC_SECONDCLICK1_PANE as u64,
                            where_::Status => keyc::KEYC_SECONDCLICK1_STATUS as u64,
                            where_::StatusLeft => keyc::KEYC_SECONDCLICK1_STATUS_LEFT as u64,
                            where_::StatusRight => keyc::KEYC_SECONDCLICK1_STATUS_RIGHT as u64,
                            where_::StatusDefault => keyc::KEYC_SECONDCLICK1_STATUS_DEFAULT as u64,
                            where_::Border => keyc::KEYC_SECONDCLICK1_BORDER as u64,
                            where_::Nowhere => key,
                        };
                    }
                    crate::MOUSE_BUTTON_2 => {
                        key = match where_ {
                            where_::Pane => keyc::KEYC_SECONDCLICK2_PANE as u64,
                            where_::Status => keyc::KEYC_SECONDCLICK2_STATUS as u64,
                            where_::StatusLeft => keyc::KEYC_SECONDCLICK2_STATUS_LEFT as u64,
                            where_::StatusRight => keyc::KEYC_SECONDCLICK2_STATUS_RIGHT as u64,
                            where_::StatusDefault => keyc::KEYC_SECONDCLICK2_STATUS_DEFAULT as u64,
                            where_::Border => keyc::KEYC_SECONDCLICK2_BORDER as u64,
                            where_::Nowhere => key,
                        };
                    }
                    crate::MOUSE_BUTTON_3 => {
                        key = match where_ {
                            where_::Pane => keyc::KEYC_SECONDCLICK3_PANE as u64,
                            where_::Status => keyc::KEYC_SECONDCLICK3_STATUS as u64,
                            where_::StatusLeft => keyc::KEYC_SECONDCLICK3_STATUS_LEFT as u64,
                            where_::StatusRight => keyc::KEYC_SECONDCLICK3_STATUS_RIGHT as u64,
                            where_::StatusDefault => keyc::KEYC_SECONDCLICK3_STATUS_DEFAULT as u64,
                            where_::Border => keyc::KEYC_SECONDCLICK3_BORDER as u64,
                            where_::Nowhere => key,
                        };
                    }
                    crate::MOUSE_BUTTON_6 => {
                        key = match where_ {
                            where_::Pane => keyc::KEYC_SECONDCLICK6_PANE as u64,
                            where_::Status => keyc::KEYC_SECONDCLICK6_STATUS as u64,
                            where_::StatusLeft => keyc::KEYC_SECONDCLICK6_STATUS_LEFT as u64,
                            where_::StatusRight => keyc::KEYC_SECONDCLICK6_STATUS_RIGHT as u64,
                            where_::StatusDefault => keyc::KEYC_SECONDCLICK6_STATUS_DEFAULT as u64,
                            where_::Border => keyc::KEYC_SECONDCLICK6_BORDER as u64,
                            where_::Nowhere => key,
                        };
                    }
                    crate::MOUSE_BUTTON_7 => {
                        key = match where_ {
                            where_::Pane => keyc::KEYC_SECONDCLICK7_PANE as u64,
                            where_::Status => keyc::KEYC_SECONDCLICK7_STATUS as u64,
                            where_::StatusLeft => keyc::KEYC_SECONDCLICK7_STATUS_LEFT as u64,
                            where_::StatusRight => keyc::KEYC_SECONDCLICK7_STATUS_RIGHT as u64,
                            where_::StatusDefault => keyc::KEYC_SECONDCLICK7_STATUS_DEFAULT as u64,
                            where_::Border => keyc::KEYC_SECONDCLICK7_BORDER as u64,
                            where_::Nowhere => key,
                        };
                    }
                    crate::MOUSE_BUTTON_8 => {
                        key = match where_ {
                            where_::Pane => keyc::KEYC_SECONDCLICK8_PANE as u64,
                            where_::Status => keyc::KEYC_SECONDCLICK8_STATUS as u64,
                            where_::StatusLeft => keyc::KEYC_SECONDCLICK8_STATUS_LEFT as u64,
                            where_::StatusRight => keyc::KEYC_SECONDCLICK8_STATUS_RIGHT as u64,
                            where_::StatusDefault => keyc::KEYC_SECONDCLICK8_STATUS_DEFAULT as u64,
                            where_::Border => keyc::KEYC_SECONDCLICK8_BORDER as u64,
                            where_::Nowhere => key,
                        };
                    }
                    crate::MOUSE_BUTTON_9 => {
                        key = match where_ {
                            where_::Pane => keyc::KEYC_SECONDCLICK9_PANE as u64,
                            where_::Status => keyc::KEYC_SECONDCLICK9_STATUS as u64,
                            where_::StatusLeft => keyc::KEYC_SECONDCLICK9_STATUS_LEFT as u64,
                            where_::StatusRight => keyc::KEYC_SECONDCLICK9_STATUS_RIGHT as u64,
                            where_::StatusDefault => keyc::KEYC_SECONDCLICK9_STATUS_DEFAULT as u64,
                            where_::Border => keyc::KEYC_SECONDCLICK9_BORDER as u64,
                            where_::Nowhere => key,
                        };
                    }
                    crate::MOUSE_BUTTON_10 => {
                        key = match where_ {
                            where_::Pane => keyc::KEYC_SECONDCLICK10_PANE as u64,
                            where_::Status => keyc::KEYC_SECONDCLICK10_STATUS as u64,
                            where_::StatusLeft => keyc::KEYC_SECONDCLICK10_STATUS_LEFT as u64,
                            where_::StatusRight => keyc::KEYC_SECONDCLICK10_STATUS_RIGHT as u64,
                            where_::StatusDefault => keyc::KEYC_SECONDCLICK10_STATUS_DEFAULT as u64,
                            where_::Border => keyc::KEYC_SECONDCLICK10_BORDER as u64,
                            where_::Nowhere => key,
                        };
                    }
                    crate::MOUSE_BUTTON_11 => {
                        key = match where_ {
                            where_::Pane => keyc::KEYC_SECONDCLICK11_PANE as u64,
                            where_::Status => keyc::KEYC_SECONDCLICK11_STATUS as u64,
                            where_::StatusLeft => keyc::KEYC_SECONDCLICK11_STATUS_LEFT as u64,
                            where_::StatusRight => keyc::KEYC_SECONDCLICK11_STATUS_RIGHT as u64,
                            where_::StatusDefault => keyc::KEYC_SECONDCLICK11_STATUS_DEFAULT as u64,
                            where_::Border => keyc::KEYC_SECONDCLICK11_BORDER as u64,
                            where_::Nowhere => key,
                        };
                    }
                    _ => (),
                },
                type_::Double => match MOUSE_BUTTONS(b) {
                    crate::MOUSE_BUTTON_1 => {
                        key = match where_ {
                            where_::Pane => keyc::KEYC_DOUBLECLICK1_PANE as u64,
                            where_::Status => keyc::KEYC_DOUBLECLICK1_STATUS as u64,
                            where_::StatusLeft => keyc::KEYC_DOUBLECLICK1_STATUS_LEFT as u64,
                            where_::StatusRight => keyc::KEYC_DOUBLECLICK1_STATUS_RIGHT as u64,
                            where_::StatusDefault => keyc::KEYC_DOUBLECLICK1_STATUS_DEFAULT as u64,
                            where_::Border => keyc::KEYC_DOUBLECLICK1_BORDER as u64,
                            where_::Nowhere => key,
                        };
                    }
                    crate::MOUSE_BUTTON_2 => {
                        key = match where_ {
                            where_::Pane => keyc::KEYC_DOUBLECLICK2_PANE as u64,
                            where_::Status => keyc::KEYC_DOUBLECLICK2_STATUS as u64,
                            where_::StatusLeft => keyc::KEYC_DOUBLECLICK2_STATUS_LEFT as u64,
                            where_::StatusRight => keyc::KEYC_DOUBLECLICK2_STATUS_RIGHT as u64,
                            where_::StatusDefault => keyc::KEYC_DOUBLECLICK2_STATUS_DEFAULT as u64,
                            where_::Border => keyc::KEYC_DOUBLECLICK2_BORDER as u64,
                            where_::Nowhere => key,
                        };
                    }
                    crate::MOUSE_BUTTON_3 => {
                        key = match where_ {
                            where_::Pane => keyc::KEYC_DOUBLECLICK3_PANE as u64,
                            where_::Status => keyc::KEYC_DOUBLECLICK3_STATUS as u64,
                            where_::StatusLeft => keyc::KEYC_DOUBLECLICK3_STATUS_LEFT as u64,
                            where_::StatusRight => keyc::KEYC_DOUBLECLICK3_STATUS_RIGHT as u64,
                            where_::StatusDefault => keyc::KEYC_DOUBLECLICK3_STATUS_DEFAULT as u64,
                            where_::Border => keyc::KEYC_DOUBLECLICK3_BORDER as u64,
                            where_::Nowhere => key,
                        };
                    }
                    crate::MOUSE_BUTTON_6 => {
                        key = match where_ {
                            where_::Pane => keyc::KEYC_DOUBLECLICK6_PANE as u64,
                            where_::Status => keyc::KEYC_DOUBLECLICK6_STATUS as u64,
                            where_::StatusLeft => keyc::KEYC_DOUBLECLICK6_STATUS_LEFT as u64,
                            where_::StatusRight => keyc::KEYC_DOUBLECLICK6_STATUS_RIGHT as u64,
                            where_::StatusDefault => keyc::KEYC_DOUBLECLICK6_STATUS_DEFAULT as u64,
                            where_::Border => keyc::KEYC_DOUBLECLICK6_BORDER as u64,
                            where_::Nowhere => key,
                        };
                    }
                    crate::MOUSE_BUTTON_7 => {
                        key = match where_ {
                            where_::Pane => keyc::KEYC_DOUBLECLICK7_PANE as u64,
                            where_::Status => keyc::KEYC_DOUBLECLICK7_STATUS as u64,
                            where_::StatusLeft => keyc::KEYC_DOUBLECLICK7_STATUS_LEFT as u64,
                            where_::StatusRight => keyc::KEYC_DOUBLECLICK7_STATUS_RIGHT as u64,
                            where_::StatusDefault => keyc::KEYC_DOUBLECLICK7_STATUS_DEFAULT as u64,
                            where_::Border => keyc::KEYC_DOUBLECLICK7_BORDER as u64,
                            where_::Nowhere => key,
                        };
                    }
                    crate::MOUSE_BUTTON_8 => {
                        key = match where_ {
                            where_::Pane => keyc::KEYC_DOUBLECLICK8_PANE as u64,
                            where_::Status => keyc::KEYC_DOUBLECLICK8_STATUS as u64,
                            where_::StatusLeft => keyc::KEYC_DOUBLECLICK8_STATUS_LEFT as u64,
                            where_::StatusRight => keyc::KEYC_DOUBLECLICK8_STATUS_RIGHT as u64,
                            where_::StatusDefault => keyc::KEYC_DOUBLECLICK8_STATUS_DEFAULT as u64,
                            where_::Border => keyc::KEYC_DOUBLECLICK8_BORDER as u64,
                            where_::Nowhere => key,
                        };
                    }
                    crate::MOUSE_BUTTON_9 => {
                        key = match where_ {
                            where_::Pane => keyc::KEYC_DOUBLECLICK9_PANE as u64,
                            where_::Status => keyc::KEYC_DOUBLECLICK9_STATUS as u64,
                            where_::StatusLeft => keyc::KEYC_DOUBLECLICK9_STATUS_LEFT as u64,
                            where_::StatusRight => keyc::KEYC_DOUBLECLICK9_STATUS_RIGHT as u64,
                            where_::StatusDefault => keyc::KEYC_DOUBLECLICK9_STATUS_DEFAULT as u64,
                            where_::Border => keyc::KEYC_DOUBLECLICK9_BORDER as u64,
                            where_::Nowhere => key,
                        };
                    }
                    crate::MOUSE_BUTTON_10 => {
                        key = match where_ {
                            where_::Pane => keyc::KEYC_DOUBLECLICK10_PANE as u64,
                            where_::Status => keyc::KEYC_DOUBLECLICK10_STATUS as u64,
                            where_::StatusLeft => keyc::KEYC_DOUBLECLICK10_STATUS_LEFT as u64,
                            where_::StatusRight => keyc::KEYC_DOUBLECLICK10_STATUS_RIGHT as u64,
                            where_::StatusDefault => keyc::KEYC_DOUBLECLICK10_STATUS_DEFAULT as u64,
                            where_::Border => keyc::KEYC_DOUBLECLICK10_BORDER as u64,
                            where_::Nowhere => key,
                        };
                    }
                    crate::MOUSE_BUTTON_11 => {
                        key = match where_ {
                            where_::Pane => keyc::KEYC_DOUBLECLICK11_PANE as u64,
                            where_::Status => keyc::KEYC_DOUBLECLICK11_STATUS as u64,
                            where_::StatusLeft => keyc::KEYC_DOUBLECLICK11_STATUS_LEFT as u64,
                            where_::StatusRight => keyc::KEYC_DOUBLECLICK11_STATUS_RIGHT as u64,
                            where_::StatusDefault => keyc::KEYC_DOUBLECLICK11_STATUS_DEFAULT as u64,
                            where_::Border => keyc::KEYC_DOUBLECLICK11_BORDER as u64,
                            where_::Nowhere => key,
                        };
                    }
                    _ => (),
                },
                type_::Triple => match MOUSE_BUTTONS(b) {
                    crate::MOUSE_BUTTON_1 => {
                        key = match where_ {
                            where_::Pane => keyc::KEYC_TRIPLECLICK1_PANE as u64,
                            where_::Status => keyc::KEYC_TRIPLECLICK1_STATUS as u64,
                            where_::StatusLeft => keyc::KEYC_TRIPLECLICK1_STATUS_LEFT as u64,
                            where_::StatusRight => keyc::KEYC_TRIPLECLICK1_STATUS_RIGHT as u64,
                            where_::StatusDefault => keyc::KEYC_TRIPLECLICK1_STATUS_DEFAULT as u64,
                            where_::Border => keyc::KEYC_TRIPLECLICK1_BORDER as u64,
                            where_::Nowhere => key,
                        };
                    }
                    crate::MOUSE_BUTTON_2 => {
                        key = match where_ {
                            where_::Pane => keyc::KEYC_TRIPLECLICK2_PANE as u64,
                            where_::Status => keyc::KEYC_TRIPLECLICK2_STATUS as u64,
                            where_::StatusLeft => keyc::KEYC_TRIPLECLICK2_STATUS_LEFT as u64,
                            where_::StatusRight => keyc::KEYC_TRIPLECLICK2_STATUS_RIGHT as u64,
                            where_::StatusDefault => keyc::KEYC_TRIPLECLICK2_STATUS_DEFAULT as u64,
                            where_::Border => keyc::KEYC_TRIPLECLICK2_BORDER as u64,
                            where_::Nowhere => key,
                        };
                    }
                    crate::MOUSE_BUTTON_3 => {
                        key = match where_ {
                            where_::Pane => keyc::KEYC_TRIPLECLICK3_PANE as u64,
                            where_::Status => keyc::KEYC_TRIPLECLICK3_STATUS as u64,
                            where_::StatusLeft => keyc::KEYC_TRIPLECLICK3_STATUS_LEFT as u64,
                            where_::StatusRight => keyc::KEYC_TRIPLECLICK3_STATUS_RIGHT as u64,
                            where_::StatusDefault => keyc::KEYC_TRIPLECLICK3_STATUS_DEFAULT as u64,
                            where_::Border => keyc::KEYC_TRIPLECLICK3_BORDER as u64,
                            where_::Nowhere => key,
                        };
                    }
                    crate::MOUSE_BUTTON_6 => {
                        key = match where_ {
                            where_::Pane => keyc::KEYC_TRIPLECLICK6_PANE as u64,
                            where_::Status => keyc::KEYC_TRIPLECLICK6_STATUS as u64,
                            where_::StatusLeft => keyc::KEYC_TRIPLECLICK6_STATUS_LEFT as u64,
                            where_::StatusRight => keyc::KEYC_TRIPLECLICK6_STATUS_RIGHT as u64,
                            where_::StatusDefault => keyc::KEYC_TRIPLECLICK6_STATUS_DEFAULT as u64,
                            where_::Border => keyc::KEYC_TRIPLECLICK6_BORDER as u64,
                            where_::Nowhere => key,
                        };
                    }
                    crate::MOUSE_BUTTON_7 => {
                        key = match where_ {
                            where_::Pane => keyc::KEYC_TRIPLECLICK7_PANE as u64,
                            where_::Status => keyc::KEYC_TRIPLECLICK7_STATUS as u64,
                            where_::StatusLeft => keyc::KEYC_TRIPLECLICK7_STATUS_LEFT as u64,
                            where_::StatusRight => keyc::KEYC_TRIPLECLICK7_STATUS_RIGHT as u64,
                            where_::StatusDefault => keyc::KEYC_TRIPLECLICK7_STATUS_DEFAULT as u64,
                            where_::Border => keyc::KEYC_TRIPLECLICK7_BORDER as u64,
                            where_::Nowhere => key,
                        };
                    }
                    crate::MOUSE_BUTTON_8 => {
                        key = match where_ {
                            where_::Pane => keyc::KEYC_TRIPLECLICK8_PANE as u64,
                            where_::Status => keyc::KEYC_TRIPLECLICK8_STATUS as u64,
                            where_::StatusLeft => keyc::KEYC_TRIPLECLICK8_STATUS_LEFT as u64,
                            where_::StatusRight => keyc::KEYC_TRIPLECLICK8_STATUS_RIGHT as u64,
                            where_::StatusDefault => keyc::KEYC_TRIPLECLICK8_STATUS_DEFAULT as u64,
                            where_::Border => keyc::KEYC_TRIPLECLICK8_BORDER as u64,
                            where_::Nowhere => key,
                        };
                    }
                    crate::MOUSE_BUTTON_9 => {
                        key = match where_ {
                            where_::Pane => keyc::KEYC_TRIPLECLICK9_PANE as u64,
                            where_::Status => keyc::KEYC_TRIPLECLICK9_STATUS as u64,
                            where_::StatusLeft => keyc::KEYC_TRIPLECLICK9_STATUS_LEFT as u64,
                            where_::StatusRight => keyc::KEYC_TRIPLECLICK9_STATUS_RIGHT as u64,
                            where_::StatusDefault => keyc::KEYC_TRIPLECLICK9_STATUS_DEFAULT as u64,
                            where_::Border => keyc::KEYC_TRIPLECLICK9_BORDER as u64,
                            where_::Nowhere => key,
                        };
                    }
                    crate::MOUSE_BUTTON_10 => {
                        key = match where_ {
                            where_::Pane => keyc::KEYC_TRIPLECLICK10_PANE as u64,
                            where_::Status => keyc::KEYC_TRIPLECLICK10_STATUS as u64,
                            where_::StatusLeft => keyc::KEYC_TRIPLECLICK10_STATUS_LEFT as u64,
                            where_::StatusRight => keyc::KEYC_TRIPLECLICK10_STATUS_RIGHT as u64,
                            where_::StatusDefault => keyc::KEYC_TRIPLECLICK10_STATUS_DEFAULT as u64,
                            where_::Border => keyc::KEYC_TRIPLECLICK10_BORDER as u64,
                            where_::Nowhere => key,
                        };
                    }
                    crate::MOUSE_BUTTON_11 => {
                        key = match where_ {
                            where_::Pane => keyc::KEYC_TRIPLECLICK11_PANE as u64,
                            where_::Status => keyc::KEYC_TRIPLECLICK11_STATUS as u64,
                            where_::StatusLeft => keyc::KEYC_TRIPLECLICK11_STATUS_LEFT as u64,
                            where_::StatusRight => keyc::KEYC_TRIPLECLICK11_STATUS_RIGHT as u64,
                            where_::StatusDefault => keyc::KEYC_TRIPLECLICK11_STATUS_DEFAULT as u64,
                            where_::Border => keyc::KEYC_TRIPLECLICK11_BORDER as u64,
                            where_::Nowhere => key,
                        };
                    }
                    _ => (),
                },
            }

            if key == KEYC_UNKNOWN {
                return KEYC_UNKNOWN;
            }
        } // out:

        // Apply modifiers if any.
        if b & MOUSE_MASK_META != 0 {
            key |= KEYC_META;
        }
        if b & MOUSE_MASK_CTRL != 0 {
            key |= KEYC_CTRL;
        }
        if b & MOUSE_MASK_SHIFT != 0 {
            key |= KEYC_SHIFT;
        }

        if log_get_level() != 0 {
            log_debug!("mouse key is {}", _s(key_string_lookup_key(key, 1)));
        }

        key
    }
}

/// Is this a bracket paste key?
pub unsafe fn server_client_is_bracket_pasting(c: *mut client, key: key_code) -> i32 {
    unsafe {
        if key == keyc::KEYC_PASTE_START as u64 {
            (*c).flags |= client_flag::BRACKETPASTING;
            log_debug!("{}: bracket paste on", _s((*c).name));
            return 1;
        }

        if key == keyc::KEYC_PASTE_END as u64 {
            (*c).flags &= !client_flag::BRACKETPASTING;
            log_debug!("{}: bracket paste off", _s((*c).name));
            return 1;
        }

        (*c).flags.intersects(client_flag::BRACKETPASTING) as i32
    }
}

/// Is this fast enough to probably be a paste?
pub unsafe fn server_client_assume_paste(s: *mut session) -> i32 {
    unsafe {
        let mut tv: timeval = zeroed();
        let t: i32 = options_get_number_((*s).options, "assume-paste-time") as i32;

        if t == 0 {
            return 0;
        }

        timersub(
            &raw const (*s).activity_time,
            &raw const (*s).last_activity_time,
            &raw mut tv,
        );
        if tv.tv_sec == 0 && tv.tv_usec < t as libc::suseconds_t * 1000 {
            log_debug!(
                "session {} pasting (flag {})",
                _s((*s).name),
                ((*s).flags & SESSION_PASTING != 0) as i32
            );
            if (*s).flags & SESSION_PASTING != 0 {
                return 1;
            }
            (*s).flags |= SESSION_PASTING;
            return 0;
        }
        log_debug!("session {} not pasting", _s((*s).name));
        (*s).flags &= !SESSION_PASTING;

        0
    }
}

/// Has the latest client changed?
pub unsafe fn server_client_update_latest(c: *mut client) {
    unsafe {
        if (*c).session.is_null() {
            return;
        }
        let w = (*(*(*c).session).curw).window;

        if (*w).latest == c.cast() {
            return;
        }
        (*w).latest = c.cast();

        if window_size_option::try_from(options_get_number_((*w).options, "window-size") as i32)
            == Ok(window_size_option::WINDOW_SIZE_LATEST)
        {
            recalculate_size(w, 0);
        }

        notify_client(c"client-active", c);
    }
}

/// Handle data key input from client. This owns and can modify the key event it is given and is responsible for freeing it.
pub unsafe fn server_client_key_callback(item: *mut cmdq_item, data: *mut c_void) -> cmd_retval {
    unsafe {
        let c = cmdq_get_client(item);
        let event = data as *mut key_event;
        let mut key = (*event).key;
        let m = &raw mut (*event).m;
        let s = (*c).session;

        let mut tv: libc::timeval = zeroed();
        let mut bd: *mut key_binding = null_mut();
        let mut table: *mut key_table = null_mut();
        let mut first: *mut key_table = null_mut();
        let mut wme: *mut window_mode_entry = null_mut();
        let mut fs: cmd_find_state = zeroed();
        let mut wl: *mut winlink = null_mut();
        let mut wp: *mut window_pane = null_mut();

        let mut xtimeout: i32 = 0;
        let mut flags: client_flag = client_flag::empty();
        let mut prefix_delay: u64 = 0;
        let mut key0: key_code = 0;
        let mut prefix: key_code = 0;
        let mut prefix2: key_code = 0;

        'out: {
            'forward_key: {
                /* Check the client is good to accept input. */
                if s.is_null() || (*c).flags.intersects(CLIENT_UNATTACHEDFLAGS) {
                    break 'out;
                }
                wl = (*s).curw;

                /* Update the activity timer. */
                if libc::gettimeofday(&raw mut (*c).activity_time, null_mut()) != 0 {
                    fatal("gettimeofday failed");
                }
                session_update_activity(s, &raw mut (*c).activity_time);

                // Check for mouse keys.
                (*m).valid = 0;
                if key == keyc::KEYC_MOUSE as u64 || key == keyc::KEYC_DOUBLECLICK as u64 {
                    if (*c).flags.intersects(client_flag::READONLY) {
                        break 'out;
                    }
                    key = server_client_check_mouse(c, event);
                    if key == KEYC_UNKNOWN {
                        break 'out;
                    }

                    (*m).valid = 1;
                    (*m).key = key;

                    /*
                     * Mouse drag is in progress, so fire the callback (now that
                     * the mouse event is valid).
                     */
                    if (key & KEYC_MASK_KEY) == keyc::KEYC_DRAGGING as u64 {
                        (*c).tty.mouse_drag_update.unwrap()(c, m);
                        break 'out;
                    }
                    (*event).key = key;
                }

                /* Find affected pane. */
                if !KEYC_IS_MOUSE(key)
                    || cmd_find_from_mouse(&raw mut fs, m, cmd_find_flags::empty()) != 0
                {
                    cmd_find_from_client(&raw mut fs, c, cmd_find_flags::empty());
                }
                wp = fs.wp;

                /* Forward mouse keys if disabled. */
                if KEYC_IS_MOUSE(key) && options_get_number_((*s).options, "mouse") == 0 {
                    break 'forward_key;
                }

                /* Forward if bracket pasting. */
                if server_client_is_bracket_pasting(c, key) != 0 {
                    break 'forward_key;
                }

                /* Treat everything as a regular key when pasting is detected. */
                if !KEYC_IS_MOUSE(key)
                    && (!key & KEYC_SENT) != 0
                    && server_client_assume_paste(s) != 0
                {
                    break 'forward_key;
                }

                /*
                 * Work out the current key table. If the pane is in a mode, use
                 * the mode table instead of the default key table.
                 */
                table = if server_client_is_default_key_table(c, (*c).keytable) != 0
                    && !wp.is_null()
                    && ({
                        wme = tailq_first(&raw mut (*wp).modes);
                        !wme.is_null()
                    })
                    && (*(*wme).mode).key_table.is_some()
                {
                    key_bindings_get_table((*(*wme).mode).key_table.unwrap()(wme), 1)
                } else {
                    (*c).keytable
                };
                first = table;

                'table_changed: loop {
                    /*
                     * The prefix always takes precedence and forces a switch to the prefix
                     * table, unless we are already there.
                     */
                    prefix = options_get_number_((*s).options, "prefix") as key_code;
                    prefix2 = options_get_number_((*s).options, "prefix2") as key_code;
                    key0 = key & (KEYC_MASK_KEY | KEYC_MASK_MODIFIERS);
                    if (key0 == (prefix & (KEYC_MASK_KEY | KEYC_MASK_MODIFIERS))
                        || key0 == (prefix2 & (KEYC_MASK_KEY | KEYC_MASK_MODIFIERS)))
                        && !streq_((*table).name, "prefix")
                    {
                        server_client_set_key_table(c, c!("prefix"));
                        server_status_client(c);
                        break 'out;
                    }
                    flags = (*c).flags;

                    'try_again: loop {
                        /* Log key table. */
                        if wp.is_null() {
                            log_debug!("key table {} (no pane)", _s((*table).name));
                        } else {
                            log_debug!("key table {} (pane %%{})", _s((*table).name), (*wp).id);
                        }
                        if (*c).flags.intersects(client_flag::REPEAT) {
                            log_debug!("currently repeating");
                        }

                        bd =
                            key_bindings_get(NonNull::new(table).expect("just dereferenced"), key0);

                        /*
                         * If prefix-timeout is enabled and we're in the prefix table, see if
                         * the timeout has been exceeded. Revert to the root table if so.
                         */
                        prefix_delay = options_get_number_(GLOBAL_OPTIONS, "prefix-timeout") as u64;
                        if prefix_delay > 0
                            && streq_((*table).name, "prefix")
                            && server_client_key_table_activity_diff(c) > prefix_delay
                        {
                            if !bd.is_null()
                                && (*c).flags.intersects(client_flag::REPEAT)
                                && (*bd).flags & KEY_BINDING_REPEAT != 0
                            {
                                log_debug!("prefix timeout ignored, repeat is active");
                            } else {
                                log_debug!("prefix timeout exceeded");
                                server_client_set_key_table(c, null_mut());
                                table = (*c).keytable;
                                first = (*c).keytable;
                                server_status_client(c);
                                continue 'table_changed;
                            }
                        } /*
                         * If repeating is active and this is a repeating binding,
                         * ignore the timeout.
                         */

                        /* Try to see if there is a key binding in the current table. */
                        if !bd.is_null() {
                            /*
                             * Key was matched in this table. If currently repeating but a
                             * non-repeating binding was found, stop repeating and try
                             * again in the root table.
                             */
                            if (*c).flags.intersects(client_flag::REPEAT)
                                && (*bd).flags & KEY_BINDING_REPEAT == 0
                            {
                                log_debug!(
                                    "found in key table {} (not repeating)",
                                    _s((*table).name)
                                );
                                server_client_set_key_table(c, null_mut());
                                table = (*c).keytable;
                                first = (*c).keytable;
                                (*c).flags &= !client_flag::REPEAT;
                                server_status_client(c);
                                continue 'table_changed;
                            }
                            log_debug!("found in key table {}", _s((*table).name));

                            /*
                             * Take a reference to this table to make sure the key binding
                             * doesn't disappear.
                             */
                            (*table).references += 1;

                            /*
                             * If this is a repeating key, start the timer. Otherwise reset
                             * the client back to the root table.
                             */
                            xtimeout = options_get_number_((*s).options, "repeat-time") as i32;
                            if xtimeout != 0 && (*bd).flags & KEY_BINDING_REPEAT != 0 {
                                (*c).flags |= client_flag::REPEAT;

                                tv.tv_sec = xtimeout as libc::time_t / 1000;
                                tv.tv_usec = (xtimeout as libc::suseconds_t % 1000) * 1000;
                                evtimer_del(&raw mut (*c).repeat_timer);
                                evtimer_add(&raw mut (*c).repeat_timer, &tv);
                            } else {
                                (*c).flags &= !client_flag::REPEAT;
                                server_client_set_key_table(c, null_mut());
                            }
                            server_status_client(c);

                            /* Execute the key binding. */
                            key_bindings_dispatch(bd, item, c, event, &raw mut fs);
                            key_bindings_unref_table(table);
                            break 'out;
                        }

                        /*
                         * No match, try the ANY key.
                         */
                        if key0 != keyc::KEYC_ANY as u64 {
                            key0 = keyc::KEYC_ANY as u64;
                            continue 'try_again;
                        }

                        /*
                         * Binding movement keys is useless since we only turn them on when the
                         * application requests, so don't let them exit the prefix table.
                         */
                        if key == keyc::KEYC_MOUSEMOVE_PANE as u64
                            || key == keyc::KEYC_MOUSEMOVE_STATUS as u64
                            || key == keyc::KEYC_MOUSEMOVE_STATUS_LEFT as u64
                            || key == keyc::KEYC_MOUSEMOVE_STATUS_RIGHT as u64
                            || key == keyc::KEYC_MOUSEMOVE_STATUS_DEFAULT as u64
                            || key == keyc::KEYC_MOUSEMOVE_BORDER as u64
                        {
                            break 'forward_key;
                        }

                        /*
                         * No match in this table. If not in the root table or if repeating
                         * switch the client back to the root table and try again.
                         */
                        log_debug!("not found in key table {}", _s((*table).name));
                        if server_client_is_default_key_table(c, table) == 0
                            || (*c).flags.intersects(client_flag::REPEAT)
                        {
                            log_debug!("trying in root table");
                            server_client_set_key_table(c, null_mut());
                            table = (*c).keytable;
                            if (*c).flags.intersects(client_flag::REPEAT) {
                                first = table;
                            }
                            (*c).flags &= !client_flag::REPEAT;
                            server_status_client(c);
                            continue 'table_changed;
                        }

                        /*
                         * No match in the root table either. If this wasn't the first table
                         * tried, don't pass the key to the pane.
                         */
                        if first != table && !flags.intersects(client_flag::REPEAT) {
                            server_client_set_key_table(c, null_mut());
                            server_status_client(c);
                            break 'out;
                        }

                        break;
                    } // 'try_again
                    break;
                } // 'table_changed
            } // forward_key:
            if (*c).flags.intersects(client_flag::READONLY) {
                break 'out;
            }
            if !wp.is_null() {
                window_pane_key(wp, c, s, wl, key, m);
            }
        } // 'out:
        if !s.is_null() && key != keyc::KEYC_FOCUS_OUT as u64 {
            server_client_update_latest(c);
        }
        free_(event);
        cmd_retval::CMD_RETURN_NORMAL
    }
}

/// Handle a key event.
pub unsafe fn server_client_handle_key(c: *mut client, event: *mut key_event) -> i32 {
    unsafe {
        let s = (*c).session;

        /* Check the client is good to accept input. */
        if s.is_null() || (*c).flags.intersects(CLIENT_UNATTACHEDFLAGS) {
            return 0;
        }

        /*
         * Key presses in overlay mode and the command prompt are a special
         * case. The queue might be blocked so they need to be processed
         * immediately rather than queued.
         */
        if !(*c).flags.intersects(client_flag::READONLY) {
            if !(*c).message_string.is_null() {
                if (*c).message_ignore_keys != 0 {
                    return 0;
                }
                status_message_clear(NonNull::new_unchecked(c));
            }
            if let Some(overlay_key) = (*c).overlay_key {
                match overlay_key(c, (*c).overlay_data, event) {
                    0 => return 0,
                    1 => {
                        server_client_clear_overlay(c);
                        return 0;
                    }
                    _ => (),
                }
            }
            server_client_clear_overlay(c);
            if !(*c).prompt_string.is_null() && status_prompt_key(c, (*event).key) == 0 {
                return 0;
            }
        }

        // Add the key to the queue so it happens after any commands queued by previous keys.
        let item = cmdq_get_callback!(server_client_key_callback, event.cast());
        cmdq_append(c, item.as_ptr());
        1
    }
}

/// Client functions that need to happen every loop.
pub unsafe fn server_client_loop() {
    unsafe {
        // Check for window resize. This is done before redrawing.
        for w in rb_foreach(&raw mut WINDOWS).map(NonNull::as_ptr) {
            server_client_check_window_resize(w);
        }

        // Check clients.
        for c in tailq_foreach(&raw mut CLIENTS).map(NonNull::as_ptr) {
            server_client_check_exit(c);
            if !(*c).session.is_null() {
                server_client_check_modes(c);
                server_client_check_redraw(c);
                server_client_reset_state(c);
            }
        }

        // Any windows will have been redrawn as part of clients, so clear their flags now.
        for w in rb_foreach(&raw mut WINDOWS).map(NonNull::as_ptr) {
            for wp in tailq_foreach::<_, discr_entry>(&raw mut (*w).panes).map(NonNull::as_ptr) {
                if (*wp).fd != -1 {
                    server_client_check_pane_resize(wp);
                    server_client_check_pane_buffer(wp);
                }
                (*wp).flags &= !window_pane_flags::PANE_REDRAW;
            }
            check_window_name(w);
        }
    }
}

/// Check if window needs to be resized.
pub unsafe fn server_client_check_window_resize(w: *mut window) {
    unsafe {
        if !(*w).flags.intersects(window_flag::RESIZE) {
            return;
        }

        let mut wl = null_mut();
        for wl_ in tailq_foreach::<_, discr_wentry>(&raw mut (*w).winlinks) {
            wl = wl_.as_ptr();
            if (*(*wl).session).attached != 0 && (*(*wl).session).curw == wl {
                break;
            }
        }
        if wl.is_null() {
            return;
        }

        log_debug!(
            "{}: resizing window @{}",
            "server_client_check_window_resize",
            (*w).id
        );
        resize_window(
            w,
            (*w).new_sx,
            (*w).new_sy,
            (*w).new_xpixel as i32,
            (*w).new_ypixel as i32,
        );
    }
}

/// Resize timer event.
pub unsafe extern "C-unwind" fn server_client_resize_timer(
    _fd: i32,
    _events: i16,
    wp: NonNull<window_pane>,
) {
    unsafe {
        log_debug!(
            "{}: %%{} resize timer expired",
            "server_client_resize_timer",
            (*wp.as_ptr()).id
        );
        evtimer_del(&raw mut (*wp.as_ptr()).resize_timer);
    }
}

/// Check if pane should be resized.
pub unsafe fn server_client_check_pane_resize(wp: *mut window_pane) {
    unsafe {
        let mut tv: libc::timeval = libc::timeval {
            tv_sec: 0,
            tv_usec: 250000,
        };

        if tailq_empty(&raw mut (*wp).resize_queue) {
            return;
        }

        if event_initialized(&raw mut (*wp).resize_timer) == 0 {
            evtimer_set(
                &raw mut (*wp).resize_timer,
                server_client_resize_timer,
                NonNull::new_unchecked(wp.cast()),
            );
        }
        if evtimer_pending(&raw mut (*wp).resize_timer, null_mut()) != 0 {
            return;
        }

        log_debug!(
            "{}: %%{} needs to be resized",
            "server_client_check_pane_resize",
            (*wp).id
        );
        for r in tailq_foreach(&raw mut (*wp).resize_queue).map(NonNull::as_ptr) {
            log_debug!(
                "queued resize: {}x{} -> {}x{}",
                (*r).osx,
                (*r).osy,
                (*r).sx,
                (*r).sy
            );
        }

        /*
         * There are three cases that matter:
         *
         * - Only one resize. It can just be applied.
         *
         * - Multiple resizes and the ending size is different from the
         *   starting size. We can discard all resizes except the most recent.
         *
         * - Multiple resizes and the ending size is the same as the starting
         *   size. We must resize at least twice to force the application to
         *   redraw. So apply the first and leave the last on the queue for
         *   next time.
         */
        let first = tailq_first(&raw mut (*wp).resize_queue);
        let last = tailq_last(&raw mut (*wp).resize_queue);
        if first == last {
            /* Only one resize. */
            window_pane_send_resize(wp, (*first).sx, (*first).sy);
            tailq_remove(&raw mut (*wp).resize_queue, first);
            free_(first);
        } else if (*last).sx != (*first).osx || (*last).sy != (*first).osy {
            /* Multiple resizes ending up with a different size. */
            window_pane_send_resize(wp, (*last).sx, (*last).sy);
            for r in tailq_foreach(&raw mut (*wp).resize_queue).map(NonNull::as_ptr) {
                tailq_remove(&raw mut (*wp).resize_queue, r);
                free_(r);
            }
        } else {
            /*
             * Multiple resizes ending up with the same size. There will
             * not be more than one to the same size in succession so we
             * can just use the last-but-one on the list and leave the last
             * for later. We reduce the time until the next check to avoid
             * a long delay between the resizes.
             */
            let r = tailq_prev(last);
            window_pane_send_resize(wp, (*r).sx, (*r).sy);
            for r in tailq_foreach(&raw mut (*wp).resize_queue).map(NonNull::as_ptr) {
                if r == last {
                    break;
                }
                tailq_remove(&raw mut (*wp).resize_queue, r);
                free_(r);
            }
            tv.tv_usec = 10000;
        }
        evtimer_add(&raw mut (*wp).resize_timer, &raw const tv);
    }
}

/// Check pane buffer size.
pub unsafe fn server_client_check_pane_buffer(wp: *mut window_pane) {
    unsafe {
        let evb = (*(*wp).event).input;
        let mut minimum: usize = 0;
        let mut wpo: *mut window_pane_offset = null_mut();
        let mut off = 1;
        let mut flag: i32 = 0;
        let mut attached_clients = 0;
        let mut new_size: usize = 0;

        'out: {
            /*
             * Work out the minimum used size. This is the most that can be removed
             * from the buffer.
             */
            minimum = (*wp).offset.used;
            if (*wp).pipe_fd != -1 && (*wp).pipe_offset.used < minimum {
                minimum = (*wp).pipe_offset.used;
            }
            for c in tailq_foreach(&raw mut CLIENTS).map(NonNull::as_ptr) {
                if (*c).session.is_null() {
                    continue;
                }
                attached_clients += 1;

                if !(*c).flags.intersects(client_flag::CONTROL) {
                    off = 0;
                    continue;
                }
                wpo = control_pane_offset(c, wp, &raw mut flag);
                if wpo.is_null() {
                    if flag == 0 {
                        off = 0;
                    }
                    continue;
                }
                if flag == 0 {
                    off = 0;
                }

                window_pane_get_new_data(wp, wpo, &raw mut new_size);
                // log_debug("%s: %s has %zu bytes used and %zu left for %%%u", __func__, (*c).name, (*wpo).used - (*wp).base_offset, new_size, (*wp).id);
                if (*wpo).used < minimum {
                    minimum = (*wpo).used;
                }
            }
            if attached_clients == 0 {
                off = 0;
            }
            minimum -= (*wp).base_offset;
            if minimum == 0 {
                break 'out;
            }

            /* Drain the buffer. */
            log_debug!(
                "{}: %%{} has {} minimum (of {}) bytes used",
                "server_client_check_pane_buffer",
                (*wp).id,
                minimum,
                EVBUFFER_LENGTH(evb)
            );
            evbuffer_drain(evb, minimum);

            /*
             * Adjust the base offset. If it would roll over, all the offsets into
             * the buffer need to be adjusted.
             */
            if (*wp).base_offset > (usize::MAX - minimum) {
                // log_debug("%s: %%%u base offset has wrapped", __func__, (*wp).id);
                (*wp).offset.used -= (*wp).base_offset;
                if (*wp).pipe_fd != -1 {
                    (*wp).pipe_offset.used -= (*wp).base_offset;
                }
                for c in tailq_foreach(&raw mut CLIENTS).map(NonNull::as_ptr) {
                    if (*c).session.is_null() || !(*c).flags.intersects(client_flag::CONTROL) {
                        continue;
                    }
                    wpo = control_pane_offset(c, wp, &raw mut flag);
                    if !wpo.is_null() && flag == 0 {
                        (*wpo).used -= (*wp).base_offset;
                    }
                }
                (*wp).base_offset = minimum;
            } else {
                (*wp).base_offset += minimum;
            }
        } // 'out:

        /*
         * If there is data remaining, and there are no clients able to consume
         * it, do not read any more. This is true when there are attached
         * clients, all of which are control clients which are not able to
         * accept any more data.
         */
        // log_debug("%s: pane %%%u is %s", __func__, (*wp).id, off ? "off" : "on");
        if off != 0 {
            bufferevent_disable((*wp).event, EV_READ);
        } else {
            bufferevent_enable((*wp).event, EV_READ);
        }
    }
}

/// Update cursor position and mode settings. The scroll region and attributes
/// are cleared when idle (waiting for an event) as this is the most likely time
/// a user may interrupt tmux, for example with ~^Z in ssh(1). This is a
/// compromise between excessive resets and likelihood of an interrupt.
///
/// tty_region/tty_reset/tty_update_mode already take care of not resetting
/// things that are already in their default state.
pub unsafe fn server_client_reset_state(c: *mut client) {
    unsafe {
        let tty = &raw mut (*c).tty;
        let w = (*(*(*c).session).curw).window;
        let wp = server_client_get_pane(c);
        let mut s = null_mut();
        let oo = (*(*c).session).options;
        let mut mode = mode_flag::empty();
        let mut cursor = 0;
        let mut flags = tty_flags::empty();
        let mut n: i32 = 0;

        let mut cx = 0;
        let mut cy = 0;
        let mut ox = 0;
        let mut oy = 0;
        let mut sx = 0;
        let mut sy = 0;

        if (*c)
            .flags
            .intersects(client_flag::CONTROL | client_flag::SUSPENDED)
        {
            return;
        }

        /* Disable the block flag. */
        flags = (*tty).flags & tty_flags::TTY_BLOCK;
        (*tty).flags &= !tty_flags::TTY_BLOCK;

        /* Get mode from overlay if any, else from screen. */
        if (*c).overlay_draw.is_some() {
            if let Some(overlay_mode) = (*c).overlay_mode {
                s = overlay_mode(c, (*c).overlay_data, &raw mut cx, &raw mut cy);
            }
        } else {
            s = (*wp).screen;
        }
        if !s.is_null() {
            mode = (*s).mode;
        }
        if log_get_level() != 0 {
            // log_debug( "%s: client %s mode %s", __func__, (*c).name, screen_mode_to_string(mode),);
        }

        /* Reset region and margin. */
        tty_region_off(tty);
        tty_margin_off(tty);

        // Move cursor to pane cursor and offset.
        if !(*c).prompt_string.is_null() {
            n = options_get_number_((*(*c).session).options, "status-position") as i32;
            if n == 0 {
                cy = 0;
            } else {
                n = status_line_size(c) as i32;
                if n == 0 {
                    cy = (*tty).sy - 1;
                } else {
                    cy = (*tty).sy - n as u32;
                }
            }
            cx = (*c).prompt_cursor as u32;
            mode &= !mode_flag::MODE_CURSOR;
        } else if (*c).overlay_draw.is_none() {
            cursor = 0;
            tty_window_offset(tty, &raw mut ox, &raw mut oy, &raw mut sx, &raw mut sy);
            if (*wp).xoff + (*s).cx >= ox
                && (*wp).xoff + (*s).cx <= ox + sx
                && (*wp).yoff + (*s).cy >= oy
                && (*wp).yoff + (*s).cy <= oy + sy
            {
                cursor = 1;

                cx = (*wp).xoff + (*s).cx - ox;
                cy = (*wp).yoff + (*s).cy - oy;

                if status_at_line(c) == 0 {
                    cy += status_line_size(c);
                }
            }
            if cursor == 0 {
                mode &= !mode_flag::MODE_CURSOR;
            }
        }
        // log_debug!("%s: cursor to %u,%u", __func__, cx, cy);
        tty_cursor(tty, cx, cy);

        /*
         * Set mouse mode if requested. To support dragging, always use button
         * mode.
         */
        if options_get_number_(oo, "mouse") != 0 {
            if (*c).overlay_draw.is_none() {
                mode &= !ALL_MOUSE_MODES;
                for loop_ in
                    tailq_foreach::<_, discr_entry>(&raw mut (*w).panes).map(NonNull::as_ptr)
                {
                    if (*(*loop_).screen)
                        .mode
                        .intersects(mode_flag::MODE_MOUSE_ALL)
                    {
                        mode |= mode_flag::MODE_MOUSE_ALL;
                    }
                }
            }
            if !mode.intersects(mode_flag::MODE_MOUSE_ALL) {
                mode |= mode_flag::MODE_MOUSE_BUTTON;
            }
        }

        /* Clear bracketed paste mode if at the prompt. */
        if (*c).overlay_draw.is_none() && !(*c).prompt_string.is_null() {
            mode &= !mode_flag::MODE_BRACKETPASTE;
        }

        /* Set the terminal mode and reset attributes. */
        tty_update_mode(tty, mode, s);
        tty_reset(tty);

        /* All writing must be done, send a sync end (if it was started). */
        tty_sync_end(tty);
        (*tty).flags |= flags;
    }
}

/// Repeat time callback.
pub unsafe extern "C-unwind" fn server_client_repeat_timer(
    _fd: i32,
    _events: i16,
    c: NonNull<client>,
) {
    unsafe {
        let c = c.as_ptr();

        if (*c).flags.intersects(client_flag::REPEAT) {
            server_client_set_key_table(c, null_mut());
            (*c).flags &= !client_flag::REPEAT;
            server_status_client(c);
        }
    }
}

/// Double-click callback.
pub unsafe extern "C-unwind" fn server_client_click_timer(
    _fd: i32,
    _events: i16,
    c: NonNull<client>,
) {
    unsafe {
        let c = c.as_ptr();
        log_debug!("click timer expired");

        if (*c).flags.intersects(client_flag::TRIPLECLICK) {
            // Waiting for a third click that hasn't happened, so this must have been a double click.
            let event = xmalloc_::<key_event>().as_ptr();
            (*event).key = keyc::KEYC_DOUBLECLICK as u64;
            memcpy__(&raw mut (*event).m, &raw const (*c).click_event);
            if server_client_handle_key(c, event) == 0 {
                free_(event);
            }
        }
        (*c).flags &= !(client_flag::DOUBLECLICK | client_flag::TRIPLECLICK);
    }
}

/// Check if client should be exited.
pub unsafe fn server_client_check_exit(c: *mut client) {
    unsafe {
        let name = (*c).exit_session;

        if (*c)
            .flags
            .intersects(client_flag::DEAD | client_flag::EXITED)
        {
            return;
        }
        if !(*c).flags.intersects(client_flag::EXIT) {
            return;
        }

        if (*c).flags.intersects(client_flag::CONTROL) {
            control_discard(c);
            if control_all_done(c) == 0 {
                return;
            }
        }
        for cf in rb_foreach(&raw mut (*c).files).map(NonNull::as_ptr) {
            if EVBUFFER_LENGTH((*cf).buffer) != 0 {
                return;
            }
        }
        (*c).flags |= client_flag::EXITED;

        match (*c).exit_type {
            exit_type::CLIENT_EXIT_RETURN => {
                let msize = if !(*c).exit_message.is_null() {
                    strlen((*c).exit_message) + 1
                } else {
                    0
                };
                let size = size_of::<i32>() + msize;
                let data = xmalloc(size).as_ptr();
                libc::memcpy(data, (&raw mut (*c).retval).cast(), size_of::<i32>());
                if !(*c).exit_message.is_null() {
                    libc::memcpy(
                        data.add(size_of::<i32>()).cast(),
                        (*c).exit_message.cast(),
                        msize,
                    );
                }
                proc_send((*c).peer, msgtype::MSG_EXIT, -1, data, size);
                free_(data);
            }
            exit_type::CLIENT_EXIT_SHUTDOWN => {
                proc_send((*c).peer, msgtype::MSG_SHUTDOWN, -1, null(), 0);
            }
            exit_type::CLIENT_EXIT_DETACH => {
                proc_send(
                    (*c).peer,
                    (*c).exit_msgtype,
                    -1,
                    name.cast(),
                    libc::strlen(name) + 1,
                );
            }
        }
        free_((*c).exit_session);
        free_((*c).exit_message);
    }
}

/// Redraw timer callback.
pub unsafe extern "C-unwind" fn server_client_redraw_timer(_fd: i32, _events: i16, _: *mut c_void) {
    log_debug!("redraw timer fired");
}

/*
 * Check if modes need to be updated. Only modes in the current window are
 * updated and it is done when the status line is redrawn.
 */
pub unsafe fn server_client_check_modes(c: *mut client) {
    unsafe {
        let w = (*(*(*c).session).curw).window;

        if (*c)
            .flags
            .intersects(client_flag::CONTROL | client_flag::SUSPENDED)
        {
            return;
        }
        if !(*c).flags.intersects(client_flag::REDRAWSTATUS) {
            return;
        }
        for wp in tailq_foreach::<_, discr_entry>(&raw mut (*w).panes).map(NonNull::as_ptr) {
            if let Some(wme) = NonNull::new(tailq_first(&raw mut (*wp).modes))
                && let Some(update) = (*(*wme.as_ptr()).mode).update
            {
                update(wme);
            }
        }
    }
}

/// Check for client redraws.
pub unsafe fn server_client_check_redraw(c: *mut client) {
    static mut EV: event = unsafe { zeroed() };
    unsafe {
        let s = (*c).session;
        let tty = &raw mut (*c).tty;
        let w = (*(*(*c).session).curw).window;
        let mut tty_flags_ = tty_flags::empty();
        let mode = (*tty).mode;
        let mut client_flags: client_flag = client_flag::empty();
        let mut redraw = false;
        let mut bit: u32 = 0;
        let tv = libc::timeval {
            tv_sec: 0,
            tv_usec: 1000,
        };
        let mut left: usize = 0;

        if (*c)
            .flags
            .intersects(client_flag::CONTROL | client_flag::SUSPENDED)
        {
            return;
        }
        if (*c).flags.intersects(CLIENT_ALLREDRAWFLAGS) {
            // log_debug("%s: redraw%s%s%s%s%s", (*c).name,
            //           ((*c).flags & CLIENT_REDRAWWINDOW) ? " window" : "",
            //           ((*c).flags & CLIENT_REDRAWSTATUS) ? " status" : "",
            //           ((*c).flags & CLIENT_REDRAWBORDERS) ? " borders" : "",
            //           ((*c).flags & CLIENT_REDRAWOVERLAY) ? " overlay" : "",
            //           ((*c).flags & CLIENT_REDRAWPANES) ? " panes" : "");
        }

        /*
         * If there is outstanding data, defer the redraw until it has been
         * consumed. We can just add a timer to get out of the event loop and
         * end up back here.
         */
        let mut needed = false;
        if (*c).flags.intersects(CLIENT_ALLREDRAWFLAGS) {
            needed = true;
        } else {
            for wp in tailq_foreach::<_, discr_entry>(&raw mut (*w).panes).map(NonNull::as_ptr) {
                if (*wp).flags.intersects(window_pane_flags::PANE_REDRAW) {
                    needed = true;
                    break;
                }
            }
            if needed {
                client_flags |= client_flag::REDRAWPANES;
            }
        }
        if needed
            && ({
                left = EVBUFFER_LENGTH((*tty).out);
                left != 0
            })
        {
            // log_debug("%s: redraw deferred (%zu left)", (*c).name, left);
            if !evtimer_initialized(&raw mut EV) {
                evtimer_set_no_args(&raw mut EV, server_client_redraw_timer);
            }
            if evtimer_pending(&raw mut EV, null_mut()) == 0 {
                log_debug!("redraw timer started");
                evtimer_add(&raw mut EV, &raw const tv);
            }

            if !(*c).flags.intersects(client_flag::REDRAWWINDOW) {
                for wp in tailq_foreach::<_, discr_entry>(&raw mut (*w).panes).map(NonNull::as_ptr)
                {
                    if (*wp).flags.intersects(window_pane_flags::PANE_REDRAW) {
                        // log_debug("%s: pane %%%u needs redraw", (*c).name, (*wp).id);
                        (*c).redraw_panes |= 1 << bit;
                    }
                    bit += 1;
                    if bit == 64 {
                        /*
                         * If more that 64 panes, give up and
                         * just redraw the window.
                         */
                        client_flags &= client_flag::REDRAWPANES;
                        client_flags |= client_flag::REDRAWWINDOW;
                        break;
                    }
                }
                if (*c).redraw_panes != 0 {
                    (*c).flags |= client_flag::REDRAWPANES;
                }
            }
            (*c).flags |= client_flags;
            return;
        } else if needed {
            // log_debug("%s: redraw needed", (*c).name);
        }

        tty_flags_ =
            (*tty).flags & (tty_flags::TTY_BLOCK | tty_flags::TTY_FREEZE | tty_flags::TTY_NOCURSOR);
        (*tty).flags = ((*tty).flags & !(tty_flags::TTY_BLOCK | tty_flags::TTY_FREEZE))
            | tty_flags::TTY_NOCURSOR;

        if !(*c).flags.intersects(client_flag::REDRAWWINDOW) {
            /*
             * If not redrawing the entire window, check whether each pane
             * needs to be redrawn.
             */
            for wp in tailq_foreach::<_, discr_entry>(&raw mut (*w).panes).map(NonNull::as_ptr) {
                redraw = false;
                if (*wp).flags.intersects(window_pane_flags::PANE_REDRAW) {
                    redraw = true;
                } else if (*c).flags.intersects(client_flag::REDRAWPANES) {
                    redraw = ((*c).redraw_panes & (1 << bit)) != 0;
                }
                bit += 1;
                if !redraw {
                    continue;
                }
                // log_debug("%s: redrawing pane %%%u", __func__, (*wp).id);
                screen_redraw_pane(c, wp);
            }
            (*c).redraw_panes = 0;
            (*c).flags &= !client_flag::REDRAWPANES;
        }

        if (*c).flags.intersects(CLIENT_ALLREDRAWFLAGS) {
            if options_get_number_((*s).options, "set-titles") != 0 {
                server_client_set_title(c);
                server_client_set_path(c);
            }
            screen_redraw_screen(c);
        }

        (*tty).flags =
            ((*tty).flags & !tty_flags::TTY_NOCURSOR) | (tty_flags_ & tty_flags::TTY_NOCURSOR);
        tty_update_mode(tty, mode, null_mut());
        (*tty).flags = ((*tty).flags
            & !(tty_flags::TTY_BLOCK | tty_flags::TTY_FREEZE | tty_flags::TTY_NOCURSOR))
            | tty_flags_;

        (*c).flags &= !(CLIENT_ALLREDRAWFLAGS | client_flag::STATUSFORCE);

        if needed {
            /*
             * We would have deferred the redraw unless the output buffer
             * was empty, so we can record how many bytes the redraw
             * generated.
             */
            (*c).redraw = EVBUFFER_LENGTH((*tty).out);
            // log_debug("%s: redraw added %zu bytes", (*c).name, (*c).redraw);
        }
    }
}

/// Set client title.
pub unsafe fn server_client_set_title(c: *mut client) {
    unsafe {
        let s = (*c).session;

        let template = options_get_string_((*s).options, "set-titles-string");

        let ft = format_create(c, null_mut(), FORMAT_NONE, format_flags::empty());
        format_defaults(ft, c, None, None, None);

        let title = format_expand_time(ft, template);
        if (*c).title.is_null() || libc::strcmp(title, (*c).title) != 0 {
            free_((*c).title);
            (*c).title = xstrdup(title).as_ptr();
            tty_set_title(&raw mut (*c).tty, (*c).title);
        }
        free_(title);

        format_free(ft);
    }
}

/// Set client path.
pub unsafe fn server_client_set_path(c: *mut client) {
    unsafe {
        let s = (*c).session;

        if (*s).curw.is_null() {
            return;
        }
        let path = if (*(*(*(*s).curw).window).active).base.path.is_null() {
            c!("")
        } else {
            (*(*(*(*s).curw).window).active).base.path
        };
        if (*c).path.is_null() || libc::strcmp(path, (*c).path) != 0 {
            free_((*c).path);
            (*c).path = xstrdup(path).as_ptr();
            tty_set_path(&raw mut (*c).tty, (*c).path);
        }
    }
}

/// Dispatch message from client.
pub unsafe fn server_client_dispatch(imsg: *mut imsg, arg: *mut c_void) {
    unsafe {
        let c: *mut client = arg.cast();

        if (*c).flags.intersects(client_flag::DEAD) {
            return;
        }

        if imsg.is_null() {
            server_client_lost(c);
            return;
        }

        let datalen = (*imsg).hdr.len - IMSG_HEADER_SIZE as u16;

        match msgtype::try_from((*imsg).hdr.type_).expect("unexpected msgtype") {
            msgtype::MSG_IDENTIFY_CLIENTPID
            | msgtype::MSG_IDENTIFY_CWD
            | msgtype::MSG_IDENTIFY_ENVIRON
            | msgtype::MSG_IDENTIFY_FEATURES
            | msgtype::MSG_IDENTIFY_FLAGS
            | msgtype::MSG_IDENTIFY_LONGFLAGS
            | msgtype::MSG_IDENTIFY_STDIN
            | msgtype::MSG_IDENTIFY_STDOUT
            | msgtype::MSG_IDENTIFY_TERM
            | msgtype::MSG_IDENTIFY_TERMINFO
            | msgtype::MSG_IDENTIFY_TTYNAME
            | msgtype::MSG_IDENTIFY_DONE => server_client_dispatch_identify(c, imsg),
            msgtype::MSG_COMMAND => server_client_dispatch_command(c, imsg),
            msgtype::MSG_RESIZE => {
                if datalen != 0 {
                    fatalx("bad MSG_RESIZE size");
                }

                if !(*c).flags.intersects(client_flag::CONTROL) {
                    server_client_update_latest(c);
                    tty_resize(&raw mut (*c).tty);
                    tty_repeat_requests(&raw mut (*c).tty);
                    recalculate_sizes();
                    if let Some(overlay_resize) = (*c).overlay_resize {
                        overlay_resize(c, (*c).overlay_data);
                    } else {
                        server_client_clear_overlay(c);
                    }
                    server_redraw_client(c);
                    if !(*c).session.is_null() {
                        notify_client(c"client-resized", c);
                    }
                }
            }
            msgtype::MSG_EXITING => {
                if datalen != 0 {
                    fatalx("bad MSG_EXITING size");
                }
                server_client_set_session(c, null_mut());
                recalculate_sizes();
                tty_close(&raw mut (*c).tty);
                proc_send((*c).peer, msgtype::MSG_EXITED, -1, null_mut(), 0);
            }
            msgtype::MSG_WAKEUP | msgtype::MSG_UNLOCK => {
                if datalen != 0 {
                    fatalx("bad MSG_WAKEUP size");
                }

                if !(*c).flags.intersects(client_flag::SUSPENDED) {
                    return;
                }
                (*c).flags &= !client_flag::SUSPENDED;

                if (*c).fd == -1 || (*c).session.is_null() {
                    return;
                } /* exited already */
                let s = (*c).session;

                if libc::gettimeofday(&raw mut (*c).activity_time, null_mut()) != 0 {
                    fatal("gettimeofday failed");
                }

                tty_start_tty(&raw mut (*c).tty);
                server_redraw_client(c);
                recalculate_sizes();

                if !s.is_null() {
                    session_update_activity(s, &raw mut (*c).activity_time);
                }
            }
            msgtype::MSG_SHELL => {
                if datalen != 0 {
                    fatalx("bad MSG_SHELL size");
                }

                server_client_dispatch_shell(c);
            }
            msgtype::MSG_WRITE_READY => file_write_ready(&raw mut (*c).files, imsg),
            msgtype::MSG_READ => file_read_data(&raw mut (*c).files, imsg),
            msgtype::MSG_READ_DONE => file_read_done(&raw mut (*c).files, imsg),
            _ => (),
        };
    }
}

/// Callback when command is not allowed.
pub unsafe fn server_client_read_only(item: *mut cmdq_item, _data: *mut c_void) -> cmd_retval {
    unsafe {
        cmdq_error!(item, "client is read-only");
        cmd_retval::CMD_RETURN_ERROR
    }
}

/// Callback when command is done.
pub unsafe fn server_client_command_done(item: *mut cmdq_item, _data: *mut c_void) -> cmd_retval {
    unsafe {
        let c = cmdq_get_client(item);

        if !(*c).flags.intersects(client_flag::ATTACHED) {
            (*c).flags |= client_flag::EXIT;
        } else if !(*c).flags.intersects(client_flag::EXIT) {
            if (*c).flags.intersects(client_flag::CONTROL) {
                control_ready(c);
            }
            tty_send_requests(&raw mut (*c).tty);
        }
        cmd_retval::CMD_RETURN_NORMAL
    }
}

/// Handle command message.
pub unsafe fn server_client_dispatch_command(c: *mut client, imsg: *mut imsg) {
    unsafe {
        let mut data: msg_command = zeroed();
        let mut buf = null_mut();
        let mut len: usize = 0;
        let mut argc = 0;
        let mut argv: *mut *mut u8 = null_mut();
        let mut cause: *mut u8 = null_mut();
        let mut values = null_mut();
        let mut new_item = null_mut();

        'error: {
            if (*c).flags.intersects(client_flag::EXIT) {
                return;
            }

            if (*imsg).hdr.len as usize - IMSG_HEADER_SIZE < size_of::<msg_command>() {
                fatalx("bad MSG_COMMAND size");
            }
            memcpy__(&raw mut data, (*imsg).data.cast());

            buf = (*imsg).data.cast::<u8>().add(size_of::<msg_command>());
            len = (*imsg).hdr.len as usize - IMSG_HEADER_SIZE - size_of::<msg_command>();
            if len > 0 && *buf.add(len - 1) != b'\0' {
                fatalx("bad MSG_COMMAND string");
            }

            argc = data.argc;
            if cmd_unpack_argv(buf, len, argc, &raw mut argv) != 0 {
                cause = xstrdup(c!("command too long")).as_ptr();
                break 'error;
            }

            if argc == 0 {
                argc = 1;
                argv = xcalloc1();
                *argv = xstrdup(c!("new-session")).as_ptr();
            }

            values = args_from_vector(argc, argv);
            let cmdlist = match cmd_parse_from_arguments(values, argc as u32, None) {
                Ok(cmdlist) => cmdlist,
                Err(err) => {
                    cause = err;
                    break 'error;
                }
            };
            args_free_values(values, argc as u32);
            free_(values);
            cmd_free_argv(argc, argv);

            if (*c).flags.intersects(client_flag::READONLY)
                && !cmd_list_all_have(cmdlist, cmd_flag::CMD_READONLY)
            {
                new_item = cmdq_get_callback!(server_client_read_only, null_mut()).as_ptr();
            } else {
                new_item = cmdq_get_command(cmdlist, null_mut());
            }
            cmdq_append(c, new_item);
            cmdq_append(
                c,
                cmdq_get_callback!(server_client_command_done, null_mut()).as_ptr(),
            );

            cmd_list_free(cmdlist);
            return;
        }
        // error:
        cmd_free_argv(argc, argv);

        cmdq_append(c, cmdq_get_error(cause).as_ptr());
        free_(cause);

        (*c).flags |= client_flag::EXIT;
    }
}

/// Handle identify message.
pub unsafe fn server_client_dispatch_identify(c: *mut client, imsg: *mut imsg) {
    unsafe {
        let mut feat: i32 = 0;
        let mut flags: i32 = 0;
        let mut longflags: u64 = 0;

        if (*c).flags.intersects(client_flag::IDENTIFIED) {
            fatalx("out-of-order identify message");
        }

        let data = (*imsg).data;
        let datalen = (*imsg).hdr.len - IMSG_HEADER_SIZE as u16;

        match msgtype::try_from((*imsg).hdr.type_).expect("unexpectd msgtype") {
            msgtype::MSG_IDENTIFY_FEATURES => {
                if datalen != size_of::<i32>() as u16 {
                    fatalx("bad MSG_IDENTIFY_FEATURES size");
                }
                memcpy__(&raw mut feat, data.cast());
                (*c).term_features |= feat;
                // log_debug("client %p IDENTIFY_FEATURES %s", c, tty_get_features(feat));
            }
            msgtype::MSG_IDENTIFY_FLAGS => {
                if datalen != size_of::<i32>() as u16 {
                    fatalx("bad MSG_IDENTIFY_FLAGS size");
                }
                memcpy__(&raw mut flags, data.cast());
                (*c).flags |= client_flag::from_bits(flags as u64).expect("invalid identify flags");
                // log_debug("client %p IDENTIFY_FLAGS %#x", c, flags);
            }
            msgtype::MSG_IDENTIFY_LONGFLAGS => {
                if datalen != size_of::<u64>() as u16 {
                    fatalx("bad MSG_IDENTIFY_LONGFLAGS size");
                }
                memcpy__(&raw mut longflags, data.cast());
                (*c).flags |=
                    client_flag::from_bits(longflags).expect("invalid identify longflags");
                // log_debug("client %p IDENTIFY_LONGFLAGS %#llx", c, (unsigned long long)longflags);
            }
            msgtype::MSG_IDENTIFY_TERM => {
                if datalen == 0 || *data.cast::<u8>().add((datalen - 1) as usize) != b'\0' {
                    fatalx("bad MSG_IDENTIFY_TERM string");
                }
                if *data.cast::<u8>() == b'\0' {
                    (*c).term_name = xstrdup(c!("unknown")).as_ptr();
                } else {
                    (*c).term_name = xstrdup(data.cast()).as_ptr();
                }
                // log_debug("client %p IDENTIFY_TERM %s", c, data);
            }
            msgtype::MSG_IDENTIFY_TERMINFO => {
                if datalen == 0 || *data.cast::<u8>().add((datalen - 1) as usize) != b'\0' {
                    fatalx("bad MSG_IDENTIFY_TERMINFO string");
                }
                (*c).term_caps =
                    xreallocarray_((*c).term_caps, (*c).term_ncaps as usize + 1).as_ptr();
                *(*c).term_caps.add((*c).term_ncaps as usize) = xstrdup(data.cast()).as_ptr();
                (*c).term_ncaps += 1;
                // log_debug("client %p IDENTIFY_TERMINFO %s", c, data);
            }
            msgtype::MSG_IDENTIFY_TTYNAME => {
                if datalen == 0 || *data.cast::<u8>().add((datalen - 1) as usize) != b'\0' {
                    fatalx("bad MSG_IDENTIFY_TTYNAME string");
                }
                (*c).ttyname = xstrdup(data.cast()).as_ptr();
                // log_debug("client %p IDENTIFY_TTYNAME %s", c, data);
            }
            msgtype::MSG_IDENTIFY_CWD => {
                if datalen == 0 || *data.cast::<u8>().add((datalen - 1) as usize) != b'\0' {
                    // fatalx("bad MSG_IDENTIFY_CWD string");
                }
                if libc::access(data.cast(), libc::X_OK) == 0 {
                    (*c).cwd = xstrdup(data.cast()).as_ptr();
                } else if let Some(home) = NonNull::new(find_home()) {
                    (*c).cwd = xstrdup(home.as_ptr()).as_ptr();
                } else {
                    (*c).cwd = xstrdup(c!("/")).as_ptr();
                }
                // log_debug("client %p IDENTIFY_CWD %s", c, data);
            }
            msgtype::MSG_IDENTIFY_STDIN => {
                if datalen != 0 {
                    fatalx("bad MSG_IDENTIFY_STDIN size");
                }
                (*c).fd = imsg_get_fd(imsg);
                // log_debug("client %p IDENTIFY_STDIN %d", c, (*c).fd);
            }
            msgtype::MSG_IDENTIFY_STDOUT => {
                if datalen != 0 {
                    fatalx("bad MSG_IDENTIFY_STDOUT size");
                }
                (*c).out_fd = imsg_get_fd(imsg);
                // log_debug("client %p IDENTIFY_STDOUT %d", c, (*c).out_fd);
            }
            msgtype::MSG_IDENTIFY_ENVIRON => {
                if datalen == 0 || *data.cast::<u8>().add((datalen - 1) as usize) != b'\0' {
                    fatalx("bad MSG_IDENTIFY_ENVIRON string");
                }
                if !libc::strchr(data.cast(), b'=' as i32).is_null() {
                    environ_put((*c).environ, data.cast(), 0);
                }
                // log_debug("client %p IDENTIFY_ENVIRON %s", c, data);
            }
            msgtype::MSG_IDENTIFY_CLIENTPID => {
                if datalen != size_of::<i32>() as u16 {
                    fatalx("bad MSG_IDENTIFY_CLIENTPID size");
                }
                memcpy__(&raw mut (*c).pid, data.cast());
                // log_debug("client %p IDENTIFY_CLIENTPID %ld", c, (long)(*c).pid);
            }
            _ => (),
        }

        if (*imsg).hdr.type_ != msgtype::MSG_IDENTIFY_DONE as u32 {
            return;
        }
        (*c).flags |= client_flag::IDENTIFIED;

        let mut name = if *(*c).ttyname != b'\0' {
            xstrdup((*c).ttyname).as_ptr()
        } else {
            format_nul!("client-{}", (*c).pid)
        };
        (*c).name = name;
        // log_debug("client %p name is %s", c, (*c).name);

        // #[cfg(feature = "cygwin")] // I don't think rust even works on cygwin
        // {
        //     (*c).fd = open((*c).ttyname, O_RDWR | O_NOCTTY);
        // }

        if (*c).flags.intersects(client_flag::CONTROL) {
            control_start(c);
        } else if (*c).fd != -1 {
            if tty_init(&raw mut (*c).tty, c) != 0 {
                libc::close((*c).fd);
                (*c).fd = -1;
            } else {
                tty_resize(&raw mut (*c).tty);
                (*c).flags |= client_flag::TERMINAL;
            }
            libc::close((*c).out_fd);
            (*c).out_fd = -1;
        }

        /*
         * If this is the first client, load configuration files. Any later
         * clients are allowed to continue with their command even if the
         * config has not been loaded - they might have been run from inside it
         */
        if !(*c).flags.intersects(client_flag::EXIT)
            && !CFG_FINISHED.load(atomic::Ordering::Acquire)
            && c == tailq_first(&raw mut CLIENTS)
        {
            start_cfg();
        }
    }
}

/// Handle shell message.
pub unsafe fn server_client_dispatch_shell(c: *mut client) {
    unsafe {
        let mut shell = options_get_string_(GLOBAL_S_OPTIONS, "default-shell");
        if !checkshell(shell) {
            shell = _PATH_BSHELL;
        }
        proc_send(
            (*c).peer,
            msgtype::MSG_SHELL,
            -1,
            shell.cast(),
            strlen(shell) + 1,
        );

        proc_kill_peer((*c).peer);
    }
}

/// Get client working directory.
pub unsafe fn server_client_get_cwd(c: *mut client, mut s: *mut session) -> *const u8 {
    unsafe {
        if !CFG_FINISHED.load(atomic::Ordering::Acquire) && !CFG_CLIENT.is_null() {
            (*CFG_CLIENT).cwd
        } else if (!c.is_null() && (*c).session.is_null() && !(*c).cwd.is_null())
            || (!s.is_null() && !(*s).cwd.is_null())
        {
            (*c).cwd
        } else if !c.is_null()
            && ({
                s = (*c).session;
                !s.is_null()
            })
            && !(*s).cwd.is_null()
        {
            (*s).cwd
        } else if let Some(home) = NonNull::new(find_home()) {
            home.as_ptr()
        } else {
            c!("/")
        }
    }
}

/// Get control client flags.
pub unsafe fn server_client_control_flags(c: *mut client, next: *const u8) -> client_flag {
    unsafe {
        if streq_(next, "pause-after") {
            (*c).pause_age = 0;
            client_flag::CONTROL_PAUSEAFTER
        } else if libc::sscanf(
            next.cast(),
            c"pause-after=%u".as_ptr(),
            &raw mut (*c).pause_age,
        ) == 1
        {
            (*c).pause_age *= 1000;
            client_flag::CONTROL_PAUSEAFTER
        } else if streq_(next, "no-output") {
            client_flag::CONTROL_NOOUTPUT
        } else if streq_(next, "wait-exit") {
            client_flag::CONTROL_WAITEXIT
        } else {
            client_flag::empty()
        }
    }
}

/// Set client flags.
pub unsafe fn server_client_set_flags(c: *mut client, flags: *const u8) {
    unsafe {
        let mut next = null_mut();
        let mut flag: client_flag = client_flag::empty();
        let mut not = false;

        let copy = xstrdup(flags).as_ptr();
        let mut s = copy;
        while {
            next = strsep(&raw mut s, c!(","));
            next.is_null()
        } {
            not = *next == b'!';
            if not {
                next = next.add(1);
            }

            if (*c).flags.intersects(client_flag::CONTROL) {
                flag = server_client_control_flags(c, next);
            } else {
                flag = client_flag::empty();
            }
            if streq_(next, "read-only") {
                flag = client_flag::READONLY;
            } else if streq_(next, "ignore-size") {
                flag = client_flag::IGNORESIZE;
            } else if streq_(next, "active-pane") {
                flag = client_flag::ACTIVEPANE;
            }
            if flag == client_flag::empty() {
                continue;
            }

            // log_debug("client %s set flag %s", (*c).name, next);
            if not {
                if (*c).flags.intersects(client_flag::READONLY) {
                    flag &= !client_flag::READONLY;
                }
                (*c).flags &= !flag;
            } else {
                (*c).flags |= flag;
            }
            if flag == client_flag::CONTROL_NOOUTPUT {
                control_reset_offsets(c);
            }
        }
        free_(copy);
        proc_send(
            (*c).peer,
            msgtype::MSG_FLAGS,
            -1,
            (&raw mut (*c).flags).cast(),
            size_of::<client_flag>(),
        );
    }
}

/// Get client flags. This is only flags useful to show to users.
pub unsafe fn server_client_get_flags(c: *mut client) -> *const u8 {
    unsafe {
        const SIZEOF_S: usize = 256;
        const SIZEOF_TMP: usize = 32;
        static mut S: [u8; SIZEOF_S] = [0; SIZEOF_S];
        static mut TMP: [u8; SIZEOF_TMP] = [0; SIZEOF_TMP];

        S[0] = b'\0';
        if (*c).flags.intersects(client_flag::ATTACHED) {
            strlcat((&raw mut S).cast(), c!("attached,"), SIZEOF_S);
        }
        if (*c).flags.intersects(client_flag::FOCUSED) {
            strlcat((&raw mut S).cast(), c!("focused,"), SIZEOF_S);
        }
        if (*c).flags.intersects(client_flag::CONTROL) {
            strlcat((&raw mut S).cast(), c!("control-mode,"), SIZEOF_S);
        }
        if (*c).flags.intersects(client_flag::IGNORESIZE) {
            strlcat((&raw mut S).cast(), c!("ignore-size,"), SIZEOF_S);
        }
        if (*c).flags.intersects(client_flag::CONTROL_NOOUTPUT) {
            strlcat((&raw mut S).cast(), c!("no-output,"), SIZEOF_S);
        }
        if (*c).flags.intersects(client_flag::CONTROL_WAITEXIT) {
            strlcat((&raw mut S).cast(), c!("wait-exit,"), SIZEOF_S);
        }
        if (*c).flags.intersects(client_flag::CONTROL_PAUSEAFTER) {
            _ = xsnprintf_!(
                (&raw mut TMP).cast(),
                SIZEOF_TMP,
                "pause-after={},",
                (*c).pause_age / 1000,
            );
            strlcat((&raw mut S).cast(), (&raw mut TMP).cast(), SIZEOF_S);
        }
        if (*c).flags.intersects(client_flag::READONLY) {
            strlcat((&raw mut S).cast(), c!("read-only,"), SIZEOF_S);
        }
        if (*c).flags.intersects(client_flag::ACTIVEPANE) {
            strlcat((&raw mut S).cast(), c!("active-pane,"), SIZEOF_S);
        }
        if (*c).flags.intersects(client_flag::SUSPENDED) {
            strlcat((&raw mut S).cast(), c!("suspended,"), SIZEOF_S);
        }
        if (*c).flags.intersects(client_flag::UTF8) {
            strlcat((&raw mut S).cast(), c!("UTF-8,"), SIZEOF_S);
        }
        if S[0] != b'\0' {
            S[strlen((&raw const S).cast()) - 1] = b'\0';
        }
        (&raw const S) as *const u8
    }
}

/// Get client window.
pub unsafe fn server_client_get_client_window(c: *mut client, id: u32) -> *mut client_window {
    unsafe {
        let mut cw: client_window = client_window {
            window: id,
            ..zeroed()
        };

        rb_find(&raw mut (*c).windows, &raw mut cw)
    }
}

/// Add client window.
pub unsafe fn server_client_add_client_window(c: *mut client, id: u32) -> NonNull<client_window> {
    unsafe {
        if let Some(cw) = NonNull::new(server_client_get_client_window(c, id)) {
            cw
        } else {
            let cw: &mut client_window = xcalloc1();
            cw.window = id;
            rb_insert(&raw mut (*c).windows, cw);
            NonNull::new(cw).unwrap()
        }
    }
}

/// Get client active pane.
pub unsafe fn server_client_get_pane(c: *mut client) -> *mut window_pane {
    unsafe {
        let s = (*c).session;

        if s.is_null() {
            return null_mut();
        }

        if !(*c).flags.intersects(client_flag::ACTIVEPANE) {
            return (*(*(*s).curw).window).active;
        }
        let cw = server_client_get_client_window(c, (*(*(*s).curw).window).id);
        if cw.is_null() {
            return (*(*(*s).curw).window).active;
        }
        (*cw).pane
    }
}

// Set client active pane.
pub unsafe fn server_client_set_pane(c: *mut client, wp: *mut window_pane) {
    unsafe {
        let s = (*c).session;

        if s.is_null() {
            return;
        }

        let cw = server_client_add_client_window(c, (*(*(*s).curw).window).id).as_ptr();
        (*cw).pane = wp;
        // log_debug("%s pane now %%%u", (*c).name, (*wp).id);
    }
}

/// Remove pane from client lists.
pub unsafe fn server_client_remove_pane(wp: *mut window_pane) {
    unsafe {
        let w = (*wp).window;

        for c in tailq_foreach(&raw mut CLIENTS).map(NonNull::as_ptr) {
            let cw = server_client_get_client_window(c, (*w).id);
            if !cw.is_null() && (*cw).pane == wp {
                rb_remove(&raw mut (*c).windows, cw);
                free_(cw);
            }
        }
    }
}

/// Print to a client.
pub unsafe fn server_client_print(c: *mut client, parse: i32, evb: *mut evbuffer) {
    unsafe {
        let data = EVBUFFER_DATA(evb);
        let mut size = EVBUFFER_LENGTH(evb);
        let mut msg = null_mut();

        'out: {
            if parse == 0 {
                utf8_stravisx(
                    &raw mut msg,
                    data.cast(),
                    size,
                    vis_flags::VIS_OCTAL | vis_flags::VIS_CSTYLE | vis_flags::VIS_NOSLASH,
                );
                // log_debug("%s: %s", __func__, msg);
            } else {
                msg = EVBUFFER_DATA(evb).cast();
                if *msg.add(size - 1) != b'\0' {
                    evbuffer_add(evb, c!("").cast(), 1);
                }
            }

            if c.is_null() {
                break 'out;
            }

            if (*c).session.is_null() || (*c).flags.intersects(client_flag::CONTROL) {
                if !(*c).flags.intersects(client_flag::UTF8) {
                    let sanitized = utf8_sanitize(msg);
                    if (*c).flags.intersects(client_flag::CONTROL) {
                        control_write!(c, "{}", _s(sanitized));
                    } else {
                        file_print!(c, "{}\n", _s(sanitized));
                    }
                    free_(sanitized);
                } else if (*c).flags.intersects(client_flag::CONTROL) {
                    control_write!(c, "{}", _s(msg));
                } else {
                    file_print!(c, "{}\n", _s(msg));
                }
                break 'out;
            }

            let wp = server_client_get_pane(c);
            let wme = tailq_first(&raw mut (*wp).modes);
            if wme.is_null() || !std::ptr::eq((*wme).mode, &raw const WINDOW_VIEW_MODE) {
                window_pane_set_mode(
                    wp,
                    null_mut(),
                    &raw const WINDOW_VIEW_MODE,
                    null_mut(),
                    null_mut(),
                );
            }
            if parse != 0 {
                loop {
                    let line =
                        evbuffer_readln(evb, null_mut(), evbuffer_eol_style::EVBUFFER_EOL_LF);
                    if !line.is_null() {
                        window_copy_add!(wp, 1, "{}", _s(line));
                        free_(line);
                    }
                    if line.is_null() {
                        break;
                    }
                }

                size = EVBUFFER_LENGTH(evb);
                if size != 0 {
                    let line = EVBUFFER_DATA(evb);
                    window_copy_add!(wp, 1, "{:1$}", _s(line), size);
                }
            } else {
                window_copy_add!(wp, 0, "{}", _s(msg));
            }
        } // out:
        if parse == 0 {
            free_(msg);
        }
    }
}

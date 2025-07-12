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

use std::cmp::Ordering;

use crate::compat::{
    RB_GENERATE,
    queue::{tailq_empty, tailq_foreach, tailq_init, tailq_insert_tail, tailq_remove},
    tree::{
        rb_empty, rb_find, rb_foreach, rb_init, rb_initializer, rb_insert, rb_max, rb_min, rb_next,
        rb_prev, rb_remove, rb_root,
    },
};

RB_GENERATE!(sessions, session, entry, discr_entry, session_cmp);
RB_GENERATE!(
    session_groups,
    session_group,
    entry,
    discr_entry,
    session_group_cmp
);

pub static mut SESSIONS: sessions = unsafe { zeroed() };

pub static mut NEXT_SESSION_ID: u32 = 0;

pub static mut SESSION_GROUPS: session_groups = rb_initializer();

pub fn session_cmp(s1: &session, s2: &session) -> Ordering {
    unsafe { i32_to_ordering(libc::strcmp(s1.name, s2.name)) }
}

pub fn session_group_cmp(s1: &session_group, s2: &session_group) -> Ordering {
    unsafe { i32_to_ordering(libc::strcmp(s1.name, s2.name)) }
}

pub unsafe fn session_alive(s: *mut session) -> bool {
    unsafe { rb_foreach(&raw mut SESSIONS).any(|s_loop| s_loop.as_ptr() == s) }
}

/// Find session by name.
pub unsafe fn session_find(name: *mut u8) -> *mut session {
    let mut s = MaybeUninit::<session>::uninit();
    let s = s.as_mut_ptr();

    unsafe {
        (*s).name = name;
        rb_find(&raw mut SESSIONS, s)
    }
}

/// Find session by id parsed from a string.
pub unsafe fn session_find_by_id_str(s: *const u8) -> *mut session {
    unsafe {
        if *s != b'$' {
            return null_mut();
        }

        let Ok(id) = strtonum(s.add(1), 0, u32::MAX) else {
            return null_mut();
        };
        transmute_ptr(session_find_by_id(id))
    }
}

/// Find session by id.
pub unsafe fn session_find_by_id(id: u32) -> Option<NonNull<session>> {
    unsafe { rb_foreach(&raw mut SESSIONS).find(|s| (*s.as_ptr()).id == id) }
}

impl session {
    unsafe fn create(
        prefix: *const u8,
        name: *const u8,
        cwd: *const u8,
        env: *mut environ,
        oo: *mut options,
        tio: *mut termios,
    ) -> Box<Self> {
        unsafe {
            let mut s: Box<session> = Box::new(zeroed());
            s.references = 1;
            s.flags = 0;

            s.cwd = xstrdup(cwd).as_ptr();

            tailq_init(&raw mut s.lastw);
            rb_init(&raw mut s.windows);

            s.environ = env;
            s.options = oo;

            status_update_cache(s.as_mut());

            s.tio = null_mut();
            if !tio.is_null() {
                s.tio = xmalloc_::<termios>().as_ptr();
                memcpy__(s.tio, tio);
            }

            if !name.is_null() {
                s.name = xstrdup(name).as_ptr();
                s.id = NEXT_SESSION_ID;
                NEXT_SESSION_ID += 1;
            } else {
                loop {
                    s.id = NEXT_SESSION_ID;
                    NEXT_SESSION_ID += 1;
                    free_(s.name);
                    s.name = if !prefix.is_null() {
                        format_nul!("{}-{}", _s(prefix), s.id)
                    } else {
                        format_nul!("{}", s.id)
                    };

                    if rb_find(&raw mut SESSIONS, s.as_mut()).is_null() {
                        break;
                    }
                }
            }
            rb_insert(&raw mut SESSIONS, s.as_mut());

            log_debug!("new session {} ${}", _s(s.name), s.id);

            if libc::gettimeofday(&raw mut s.creation_time, null_mut()) != 0 {
                fatal("gettimeofday failed");
            }
            session_update_activity(s.as_mut(), &raw mut s.creation_time);

            s
        }
    }
}

/// Create a new session.
pub unsafe fn session_create(
    prefix: *const u8,
    name: *const u8,
    cwd: *const u8,
    env: *mut environ,
    oo: *mut options,
    tio: *mut termios,
) -> *mut session {
    unsafe { Box::leak(session::create(prefix, name, cwd, env, oo, tio)) }
}

/// Add a reference to a session.
pub unsafe fn session_add_ref(s: *mut session, from: *const u8) {
    let __func__ = "session_add_ref";
    unsafe {
        (*s).references += 1;
        log_debug!(
            "{}: {} {}, now {}",
            __func__,
            _s((*s).name),
            _s(from),
            (*s).references
        );
    }
}

/// Remove a reference from a session.
pub unsafe fn session_remove_ref(s: *mut session, from: *const u8) {
    let __func__ = "session_remove_ref";
    unsafe {
        (*s).references -= 1;
        log_debug!(
            "{}: {} {}, now {}",
            __func__,
            _s((*s).name),
            _s(from),
            (*s).references
        );

        if (*s).references == 0 {
            event_once(-1, EV_TIMEOUT, Some(session_free), s.cast(), null_mut());
        }
    }
}

/// Free session.
pub unsafe extern "C-unwind" fn session_free(_fd: i32, _events: i16, arg: *mut c_void) {
    unsafe {
        let s = arg as *mut session;

        log_debug!(
            "session {} freed ({} references)",
            _s((*s).name),
            (*s).references
        );

        if (*s).references == 0 {
            environ_free((*s).environ);
            options_free((*s).options);
            free_((*s).name);
            free_(s);
        }
    }
}

/// Destroy a session.
pub unsafe fn session_destroy(s: *mut session, notify: i32, from: *const u8) {
    let __func__ = c!("session_destroy");
    unsafe {
        log_debug!("session {} destroyed ({})", _s((*s).name), _s(from));

        if (*s).curw.is_null() {
            return;
        }
        (*s).curw = null_mut();

        rb_remove(&raw mut SESSIONS, s);
        if notify != 0 {
            notify_session(c"session-closed", s);
        }

        free_((*s).tio);

        if event_initialized(&raw mut (*s).lock_timer) != 0 {
            event_del(&raw mut (*s).lock_timer);
        }

        session_group_remove(s);

        while !tailq_empty(&raw mut (*s).lastw) {
            winlink_stack_remove(&raw mut (*s).lastw, tailq_first(&raw mut (*s).lastw));
        }
        while !rb_empty(&raw mut (*s).windows) {
            let wl = rb_root(&raw mut (*s).windows);
            notify_session_window(c"window-unlinked", s, (*wl).window);
            winlink_remove(&raw mut (*s).windows, wl);
        }

        free_((*s).cwd);

        session_remove_ref(s, __func__);
    }
}

/// Sanitize session name.
pub unsafe fn session_check_name(name: *const u8) -> *mut u8 {
    unsafe {
        let mut new_name = null_mut();
        if *name == b'\0' {
            return null_mut();
        }
        let copy = xstrdup(name).as_ptr();
        let mut cp = copy;
        while *cp != b'\0' {
            if *cp == b':' || *cp == b'.' {
                *cp = b'_';
            }
            cp = cp.add(1);
        }
        utf8_stravis(
            &raw mut new_name,
            copy,
            vis_flags::VIS_OCTAL | vis_flags::VIS_CSTYLE | vis_flags::VIS_TAB | vis_flags::VIS_NL,
        );
        free_(copy);
        new_name
    }
}

/// Lock session if it has timed out.
pub unsafe extern "C-unwind" fn session_lock_timer(_fd: i32, _events: i16, s: NonNull<session>) {
    unsafe {
        if (*s.as_ptr()).attached == 0 {
            return;
        }

        log_debug!(
            "session {} locked, activity time {}",
            _s((*s.as_ptr()).name),
            (*s.as_ptr()).activity_time.tv_sec,
        );

        server_lock_session(s.as_ptr());
        recalculate_sizes();
    }
}

/// Update activity time.
pub unsafe fn session_update_activity(s: *mut session, from: *mut timeval) {
    unsafe {
        let last = &raw mut (*s).last_activity_time;

        memcpy__(last, &raw mut (*s).activity_time);
        if from.is_null() {
            libc::gettimeofday(&raw mut (*s).activity_time, null_mut());
        } else {
            memcpy__(&raw mut (*s).activity_time, from);
        }

        log_debug!(
            "session ${} {} activity {}.{:06} (last {}.{:06})",
            (*s).id,
            _s((*s).name),
            (*s).activity_time.tv_sec,
            (*s).activity_time.tv_usec,
            (*last).tv_sec,
            (*last).tv_usec,
        );

        if evtimer_initialized(&raw mut (*s).lock_timer) {
            evtimer_del(&raw mut (*s).lock_timer);
        } else {
            evtimer_set(
                &raw mut (*s).lock_timer,
                session_lock_timer,
                NonNull::new(s).unwrap(),
            );
        }

        if (*s).attached != 0 {
            let tv = timeval {
                tv_sec: options_get_number_((*s).options, c"lock-after-time"),
                tv_usec: 0,
            };

            if tv.tv_sec != 0 {
                evtimer_add(&raw mut (*s).lock_timer, &tv);
            }
        }
    }
}

/// Find the next usable session.
pub unsafe fn session_next_session(s: *mut session) -> *mut session {
    unsafe {
        if rb_empty(&raw mut SESSIONS) || !session_alive(s) {
            return null_mut();
        }

        let mut s2 = rb_next(s);
        if s2.is_null() {
            s2 = rb_min(&raw mut SESSIONS);
        }
        if s2 == s {
            return null_mut();
        }

        s2
    }
}

/// Find the previous usable session.
pub unsafe fn session_previous_session(s: *mut session) -> *mut session {
    unsafe {
        if rb_empty(&raw mut SESSIONS) || !session_alive(s) {
            return null_mut();
        }

        let mut s2 = rb_prev(s);
        if s2.is_null() {
            s2 = rb_max(&raw mut SESSIONS);
        }
        if s2 == s {
            return null_mut();
        }
        s2
    }
}

/// Attach a window to a session.
pub unsafe fn session_attach(
    s: *mut session,
    w: *mut window,
    idx: i32,
    cause: *mut *mut u8,
) -> *mut winlink {
    unsafe {
        let wl = winlink_add(&raw mut (*s).windows, idx);

        if wl.is_null() {
            *cause = format_nul!("index in use: {}", idx);
            return null_mut();
        }
        (*wl).session = s;
        winlink_set_window(wl, w);
        notify_session_window(c"window-linked", s, w);

        session_group_synchronize_from(s);
        wl
    }
}

/// Detach a window from a session.
pub unsafe fn session_detach(s: *mut session, wl: *mut winlink) -> i32 {
    unsafe {
        if (*s).curw == wl && session_last(s) != 0 && session_previous(s, 0) != 0 {
            session_next(s, 0);
        }

        (*wl).flags &= !WINLINK_ALERTFLAGS;
        notify_session_window(c"window-unlinked", s, (*wl).window);
        winlink_stack_remove(&raw mut (*s).lastw, wl);
        winlink_remove(&raw mut (*s).windows, wl);

        session_group_synchronize_from(s);

        if rb_empty(&raw mut (*s).windows) {
            return 1;
        }
        0
    }
}

/// Return if session has window.
pub unsafe fn session_has(s: *mut session, w: *mut window) -> i32 {
    unsafe {
        tailq_foreach::<_, discr_wentry>(&raw mut (*w).winlinks)
            .any(|wl| (*wl.as_ptr()).session == s) as i32
    }
}

/// Return 1 if a window is linked outside this session (not including session groups). The window must be in this session!
pub unsafe fn session_is_linked(s: *mut session, w: *mut window) -> i32 {
    unsafe {
        let sg = session_group_contains(s);
        if sg.is_null() {
            return ((*w).references != session_group_count(sg)) as i32;
        }
        ((*w).references != 1) as i32
    }
}

pub unsafe fn session_next_alert(mut wl: *mut winlink) -> *mut winlink {
    unsafe {
        while !wl.is_null() {
            if (*wl).flags.intersects(WINLINK_ALERTFLAGS) {
                break;
            }
            wl = winlink_next(wl);
        }
    }
    wl
}

/// Move session to next window.
pub unsafe fn session_next(s: *mut session, alert: i32) -> i32 {
    unsafe {
        if (*s).curw.is_null() {
            return -1;
        }

        let mut wl = winlink_next((*s).curw);
        if alert != 0 {
            wl = session_next_alert(wl);
        }
        if wl.is_null() {
            wl = rb_min(&raw mut (*s).windows);
            if alert != 0
                && ({
                    (wl = session_next_alert(wl));
                    wl.is_null()
                })
            {
                return -1;
            }
        }
        session_set_current(s, wl)
    }
}

pub unsafe fn session_previous_alert(mut wl: *mut winlink) -> *mut winlink {
    unsafe {
        while !wl.is_null() {
            if (*wl).flags.intersects(WINLINK_ALERTFLAGS) {
                break;
            }
            wl = winlink_previous(wl);
        }
        wl
    }
}

/// Move session to previous window.
pub unsafe fn session_previous(s: *mut session, alert: i32) -> i32 {
    unsafe {
        if (*s).curw.is_null() {
            return -1;
        }

        let mut wl = winlink_previous((*s).curw);
        if alert != 0 {
            wl = session_previous_alert(wl);
        }
        if wl.is_null() {
            wl = rb_max(&raw mut (*s).windows);
            if alert != 0
                && ({
                    (wl = session_previous_alert(wl));
                    wl.is_null()
                })
            {
                return -1;
            }
        }
        session_set_current(s, wl)
    }
}

/// Move session to specific window.
pub unsafe fn session_select(s: *mut session, idx: i32) -> i32 {
    unsafe {
        let wl = winlink_find_by_index(&raw mut (*s).windows, idx);
        session_set_current(s, wl)
    }
}

/// Move session to last used window.
pub unsafe fn session_last(s: *mut session) -> i32 {
    unsafe {
        let wl = tailq_first(&raw mut (*s).lastw);
        if wl.is_null() {
            return -1;
        }
        if wl == (*s).curw {
            return 1;
        }

        session_set_current(s, wl)
    }
}

/// Set current winlink to wl.
pub unsafe fn session_set_current(s: *mut session, wl: *mut winlink) -> i32 {
    unsafe {
        let old: *mut winlink = (*s).curw;

        if wl.is_null() {
            return -1;
        }
        if wl == (*s).curw {
            return 1;
        }

        winlink_stack_remove(&raw mut (*s).lastw, wl);
        winlink_stack_push(&raw mut (*s).lastw, (*s).curw);
        (*s).curw = wl;
        if options_get_number_(GLOBAL_OPTIONS, c"focus-events") != 0 {
            if !old.is_null() {
                window_update_focus((*old).window);
            }
            window_update_focus((*wl).window);
        }
        winlink_clear_flags(wl);
        window_update_activity(NonNull::new_unchecked((*wl).window));
        tty_update_window_offset((*wl).window);
        notify_session(c"session-window-changed", s);
        0
    }
}

/// Find the session group containing a session.
pub unsafe fn session_group_contains(target: *mut session) -> *mut session_group {
    unsafe {
        for sg in rb_foreach(&raw mut SESSION_GROUPS) {
            for s in tailq_foreach(&raw mut (*sg.as_ptr()).sessions) {
                if s.as_ptr() == target {
                    return sg.as_ptr();
                }
            }
        }

        null_mut()
    }
}

/// Find session group by name.
pub unsafe fn session_group_find(name: *const u8) -> *mut session_group {
    unsafe {
        let mut sg = MaybeUninit::<session_group>::uninit();
        let sg = sg.as_mut_ptr();

        (*sg).name = name;
        rb_find(&raw mut SESSION_GROUPS, sg)
    }
}

/// Create a new session group.
pub unsafe fn session_group_new(name: *const u8) -> *mut session_group {
    unsafe {
        let mut sg = session_group_find(name);
        if !sg.is_null() {
            return sg;
        }

        sg = xcalloc1::<session_group>();
        (*sg).name = xstrdup(name).as_ptr();
        tailq_init(&raw mut (*sg).sessions);

        rb_insert(&raw mut SESSION_GROUPS, sg);
        sg
    }
}

/// Add a session to a session group.
pub unsafe fn session_group_add(sg: *mut session_group, s: *mut session) {
    unsafe {
        if session_group_contains(s).is_null() {
            tailq_insert_tail(&raw mut (*sg).sessions, s);
        }
    }
}

/// Remove a session from its group and destroy the group if empty.
pub unsafe fn session_group_remove(s: *mut session) {
    unsafe {
        let sg = session_group_contains(s);

        if sg.is_null() {
            return;
        }
        tailq_remove(&raw mut (*sg).sessions, s);
        if tailq_empty(&raw mut (*sg).sessions) {
            rb_remove(&raw mut SESSION_GROUPS, sg);
            free_((*sg).name.cast_mut());
            free_(sg);
        }
    }
}

/// Count number of sessions in session group.
pub unsafe fn session_group_count(sg: *mut session_group) -> u32 {
    unsafe { tailq_foreach(&raw mut (*sg).sessions).count() as u32 }
}

/// Count number of clients attached to sessions in session group.
pub unsafe fn session_group_attached_count(sg: *mut session_group) -> u32 {
    unsafe {
        tailq_foreach(&raw mut (*sg).sessions)
            .map(|s| (*s.as_ptr()).attached)
            .sum()
    }
}

/// Synchronize a session to its session group.
pub unsafe fn session_group_synchronize_to(s: *mut session) {
    unsafe {
        let sg = session_group_contains(s);
        if sg.is_null() {
            return;
        }

        let mut target = null_mut();
        for target_ in tailq_foreach(&raw mut (*sg).sessions).map(|e| e.as_ptr()) {
            target = target_;
            if target != s {
                break;
            }
        }
        if !target.is_null() {
            session_group_synchronize1(target, s);
        }
    }
}

/// Synchronize a session group to a session.
pub unsafe fn session_group_synchronize_from(target: *mut session) {
    unsafe {
        let sg = session_group_contains(target);
        if sg.is_null() {
            return;
        }

        for s in tailq_foreach(&raw mut (*sg).sessions).map(|e| e.as_ptr()) {
            if s != target {
                session_group_synchronize1(target, s);
            }
        }
    }
}

/*
 * Synchronize a session with a target session. This means destroying all
 * winlinks then recreating them, then updating the current window, last window
 * stack and alerts.
 */
pub unsafe fn session_group_synchronize1(target: *mut session, s: *mut session) {
    let mut old_windows = MaybeUninit::<winlinks>::uninit();
    let mut old_lastw = MaybeUninit::<winlink_stack>::uninit();

    unsafe {
        /* Don't do anything if the session is empty (it'll be destroyed). */
        let ww: *mut winlinks = &raw mut (*target).windows;
        if rb_empty(ww) {
            return;
        }

        /* If the current window has vanished, move to the next now. */
        if !(*s).curw.is_null()
            && winlink_find_by_index(ww, (*(*s).curw).idx).is_null()
            && session_last(s) != 0
            && session_previous(s, 0) != 0
        {
            session_next(s, 0);
        }

        /* Save the old pointer and reset it. */
        memcpy__(old_windows.as_mut_ptr(), &raw mut (*s).windows);
        rb_init(&raw mut (*s).windows);

        /* Link all the windows from the target. */
        for wl in rb_foreach(ww).map(|e| e.as_ptr()) {
            let wl2 = winlink_add(&raw mut (*s).windows, (*wl).idx);
            (*wl2).session = s;
            winlink_set_window(wl2, (*wl).window);
            notify_session_window(c"window-linked", s, (*wl2).window);
            (*wl2).flags |= (*wl).flags & WINLINK_ALERTFLAGS;
        }

        /* Fix up the current window. */
        if !(*s).curw.is_null() {
            (*s).curw = winlink_find_by_index(&raw mut (*s).windows, (*(*s).curw).idx);
        } else {
            (*s).curw = winlink_find_by_index(&raw mut (*s).windows, (*(*target).curw).idx);
        }

        /* Fix up the last window stack. */
        memcpy__(old_lastw.as_mut_ptr(), &raw mut (*s).lastw);
        tailq_init(&raw mut (*s).lastw);

        for wl in tailq_foreach::<_, discr_sentry>(old_lastw.as_mut_ptr()).map(|e| e.as_ptr()) {
            if let Some(wl2) = NonNull::new(winlink_find_by_index(&raw mut (*s).windows, (*wl).idx))
            {
                tailq_insert_tail::<_, discr_sentry>(&raw mut (*s).lastw, wl2.as_ptr());
                (*wl2.as_ptr()).flags |= winlink_flags::WINLINK_VISITED;
            }
        }

        /* Then free the old winlinks list. */
        while !rb_empty(old_windows.as_mut_ptr()) {
            let wl = rb_root(old_windows.as_mut_ptr());
            let wl2 = winlink_find_by_window_id(&raw mut (*s).windows, (*(*wl).window).id);
            if wl2.is_null() {
                notify_session_window(c"window-unlinked", s, (*wl).window);
            }
            winlink_remove(old_windows.as_mut_ptr(), wl);
        }
    }
}

/// Renumber the windows across winlinks attached to a specific session.
pub unsafe fn session_renumber_windows(s: *mut session) {
    unsafe {
        let mut old_wins = MaybeUninit::<winlinks>::uninit();
        let mut old_lastw = MaybeUninit::<winlink_stack>::uninit();
        let mut marked_idx = -1;

        // Save and replace old window list.
        memcpy__(old_wins.as_mut_ptr(), &raw mut (*s).windows);
        rb_init(&raw mut (*s).windows);

        // Start renumbering from the base-index if it's set.
        let mut new_idx = options_get_number_((*s).options, c"base-index") as i32;
        let mut new_curw_idx = 0;

        // Go through the winlinks and assign new indexes.
        for wl in rb_foreach(old_wins.as_mut_ptr()).map(|e| e.as_ptr()) {
            let wl_new = winlink_add(&raw mut (*s).windows, new_idx);
            (*wl_new).session = s;
            winlink_set_window(wl_new, (*wl).window);
            (*wl_new).flags |= (*wl).flags & WINLINK_ALERTFLAGS;

            if wl == MARKED_PANE.wl {
                marked_idx = (*wl_new).idx;
            }
            if wl == (*s).curw {
                new_curw_idx = (*wl_new).idx;
            }

            new_idx += 1;
        }

        // Fix the stack of last windows now.
        memcpy__(old_lastw.as_mut_ptr(), &raw mut (*s).lastw);
        tailq_init(&raw mut (*s).lastw);
        for wl in tailq_foreach::<_, discr_sentry>(old_lastw.as_mut_ptr()).map(|e| e.as_ptr()) {
            (*wl).flags &= !winlink_flags::WINLINK_VISITED;

            if let Some(wl_new) = winlink_find_by_window(&raw mut (*s).windows, (*wl).window) {
                tailq_insert_tail::<_, discr_sentry>(&raw mut (*s).lastw, wl_new.as_ptr());
                (*wl_new.as_ptr()).flags |= winlink_flags::WINLINK_VISITED;
            }
        }

        // Set the current window.
        if marked_idx != -1 {
            MARKED_PANE.wl = winlink_find_by_index(&raw mut (*s).windows, marked_idx);
            if MARKED_PANE.wl.is_null() {
                server_clear_marked();
            }
        }
        (*s).curw = winlink_find_by_index(&raw mut (*s).windows, new_curw_idx);

        // Free the old winlinks (reducing window references too).
        for wl in rb_foreach(old_wins.as_mut_ptr()).map(|e| e.as_ptr()) {
            winlink_remove(old_wins.as_mut_ptr(), wl);
        }
    }
}

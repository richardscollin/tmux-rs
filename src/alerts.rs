// Copyright (c) 2015 Nicholas Marriott <nicholas.marriott@gmail.com>
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
    queue::{tailq_foreach, tailq_head, tailq_insert_tail, tailq_remove},
    tree::rb_foreach,
};

use std::sync::atomic;

static ALERTS_FIRED: atomic::AtomicI32 = atomic::AtomicI32::new(0);

static mut ALERTS_LIST: tailq_head<window> = compat::TAILQ_HEAD_INITIALIZER!(ALERTS_LIST);

unsafe extern "C" fn alerts_timer(_fd: i32, _events: i16, arg: *mut c_void) {
    let w = arg as *mut window;

    unsafe {
        log_debug!("@{} alerts timer expired", (*w).id);
        alerts_queue(NonNull::new_unchecked(w), window_flag::SILENCE);
    }
}

unsafe extern "C" fn alerts_callback(_fd: c_int, _events: c_short, _arg: *mut c_void) {
    unsafe {
        for w in tailq_foreach::<_, crate::discr_alerts_entry>(&raw mut ALERTS_LIST) {
            let alerts = alerts_check_all(w);

            let w = w.as_ptr();
            log_debug!("@{} alerts check, alerts {:#x}", (*w).id, alerts);

            (*w).alerts_queued = 0;
            tailq_remove::<_, crate::discr_alerts_entry>(&raw mut ALERTS_LIST, w);

            (*w).flags &= !WINDOW_ALERTFLAGS;
            window_remove_ref(w, c!("alerts_callback"));
        }
        ALERTS_FIRED.store(0, atomic::Ordering::Release);
    }
}

unsafe fn alerts_action_applies(wl: *mut winlink, name: &'static CStr) -> c_int {
    unsafe {
        match alert_option::try_from(options_get_number_((*(*wl).session).options, name) as i32) {
            Ok(alert_option::ALERT_ANY) => 1,
            Ok(alert_option::ALERT_CURRENT) => (wl == (*(*wl).session).curw) as i32,
            Ok(alert_option::ALERT_OTHER) => (wl != (*(*wl).session).curw) as i32,
            _ => 0,
        }
    }
}

unsafe fn alerts_check_all(w: NonNull<window>) -> window_flag {
    unsafe { alerts_check_bell(w) | alerts_check_activity(w) | alerts_check_silence(w) }
}

pub(crate) unsafe fn alerts_check_session(s: *mut session) {
    unsafe {
        for wl in rb_foreach(&raw mut (*s).windows).map(NonNull::as_ptr) {
            alerts_check_all(NonNull::new_unchecked((*wl).window));
        }
    }
}

unsafe fn alerts_enabled(w: *mut window, flags: window_flag) -> c_int {
    unsafe {
        if flags.intersects(window_flag::BELL)
            && options_get_number_((*w).options, c"monitor-bell") != 0
        {
            return 1;
        }
        if flags.intersects(window_flag::ACTIVITY)
            && options_get_number_((*w).options, c"monitor-activity") != 0
        {
            return 1;
        }
        if flags.intersects(window_flag::SILENCE)
            && options_get_number_((*w).options, c"monitor-silence") != 0
        {
            return 1;
        }
    }

    0
}

pub(crate) unsafe fn alerts_reset_all() {
    unsafe {
        for w in rb_foreach(&raw mut WINDOWS) {
            alerts_reset(w);
        }
    }
}

unsafe fn alerts_reset(w: NonNull<window>) {
    let w = w.as_ptr();
    unsafe {
        if event_initialized(&raw const (*w).alerts_timer) == 0 {
            evtimer_set(&raw mut (*w).alerts_timer, Some(alerts_timer), w as _);
        }

        (*w).flags &= !window_flag::SILENCE;
        event_del(&raw mut (*w).alerts_timer);

        let mut tv = timeval {
            tv_sec: options_get_number_((*w).options, c"monitor-silence"),
            tv_usec: 0,
        };

        log_debug!("@{} alerts timer reset {}", (*w).id, tv.tv_sec);
        if tv.tv_sec != 0 {
            event_add(&raw mut (*w).alerts_timer, &raw mut tv);
        }
    }
}

pub(crate) unsafe fn alerts_queue(w: NonNull<window>, flags: window_flag) {
    unsafe {
        alerts_reset(w);
        let w = w.as_ptr();

        if ((*w).flags & flags) != flags {
            (*w).flags |= flags;
            log_debug!("@{} alerts flags added {:#x}", (*w).id, flags);
        }

        if alerts_enabled(w, flags) != 0 {
            if (*w).alerts_queued == 0 {
                (*w).alerts_queued = 1;
                tailq_insert_tail::<_, discr_alerts_entry>(&raw mut ALERTS_LIST, w);
                window_add_ref(w, c!("alerts_queue"));
            }

            if ALERTS_FIRED.load(atomic::Ordering::Acquire) == 0 {
                log_debug!("alerts check queued (by @{})", (*w).id);
                event_once(
                    -1,
                    EV_TIMEOUT,
                    Some(alerts_callback),
                    null_mut(),
                    null_mut(),
                );
                ALERTS_FIRED.store(1, atomic::Ordering::Release);
            }
        }
    }
}

unsafe fn alerts_check_bell(w: NonNull<window>) -> window_flag {
    unsafe {
        let w = w.as_ptr();
        if !(*w).flags.intersects(window_flag::BELL) {
            return window_flag::empty();
        }
        if options_get_number_((*w).options, c"monitor-bell") == 0 {
            return window_flag::empty();
        }

        for wl in tailq_foreach::<_, crate::discr_wentry>(&raw mut (*w).winlinks) {
            (*(*wl.as_ptr()).session).flags &= !SESSION_ALERTED;
        }

        for wl in
            tailq_foreach::<_, crate::discr_wentry>(&raw mut (*w).winlinks).map(NonNull::as_ptr)
        {
            /*
             * Bells are allowed even if there is an existing bell (so do
             * not check WINLINK_BELL).
             */
            let s = (*wl).session;
            if (*s).curw != wl || (*s).attached == 0 {
                (*wl).flags |= winlink_flags::WINLINK_BELL;
                server_status_session(s);
            }
            if alerts_action_applies(wl, c"bell-action") == 0 {
                continue;
            }
            notify_winlink(c"alert-bell", wl);

            if (*s).flags & SESSION_ALERTED != 0 {
                continue;
            }
            (*s).flags |= SESSION_ALERTED;

            alerts_set_message(wl, c"Bell", c"visual-bell");
        }
    }
    window_flag::BELL
}

unsafe fn alerts_check_activity(w: NonNull<window>) -> window_flag {
    unsafe {
        let w = w.as_ptr();
        if !(*w).flags.intersects(window_flag::ACTIVITY) {
            return window_flag::empty();
        }
        if options_get_number_((*w).options, c"monitor-activity") == 0 {
            return window_flag::empty();
        }

        for wl in
            tailq_foreach::<_, crate::discr_wentry>(&raw mut (*w).winlinks).map(NonNull::as_ptr)
        {
            (*(*wl).session).flags &= !SESSION_ALERTED;
        }

        for wl in
            tailq_foreach::<_, crate::discr_wentry>(&raw mut (*w).winlinks).map(NonNull::as_ptr)
        {
            let s = (*wl).session;
            if (*s).curw != wl || (*s).attached == 0 {
                (*wl).flags |= winlink_flags::WINLINK_ACTIVITY;
                server_status_session(s);
            }
            if alerts_action_applies(wl, c"activity-action") == 0 {
                continue;
            }
            notify_winlink(c"alert-activity", wl);

            if (*s).flags & SESSION_ALERTED != 0 {
                continue;
            }
            (*s).flags |= SESSION_ALERTED;

            alerts_set_message(wl, c"Activity", c"visual-activity");
        }
    }
    window_flag::ACTIVITY
}

unsafe fn alerts_check_silence(w: NonNull<window>) -> window_flag {
    unsafe {
        let w = w.as_ptr();
        if !(*w).flags.intersects(window_flag::SILENCE) {
            return window_flag::empty();
        }
        if options_get_number_((*w).options, c"monitor-silence") == 0 {
            return window_flag::empty();
        }

        for wl in
            tailq_foreach::<_, crate::discr_wentry>(&raw mut (*w).winlinks).map(NonNull::as_ptr)
        {
            (*(*wl).session).flags &= !SESSION_ALERTED;
        }

        for wl in
            tailq_foreach::<_, crate::discr_wentry>(&raw mut (*w).winlinks).map(NonNull::as_ptr)
        {
            if (*wl).flags.intersects(winlink_flags::WINLINK_SILENCE) {
                continue;
            }
            let s = (*wl).session;
            if (*s).curw != wl || (*s).attached == 0 {
                (*wl).flags |= winlink_flags::WINLINK_SILENCE;
                server_status_session(s);
            }
            if alerts_action_applies(wl, c"silence-action") == 0 {
                continue;
            }
            notify_winlink(c"alert-silence", wl);

            if (*s).flags & SESSION_ALERTED != 0 {
                continue;
            }
            (*s).flags |= SESSION_ALERTED;

            alerts_set_message(wl, c"Silence", c"visual-silence");
        }
    }

    window_flag::SILENCE
}

unsafe fn alerts_set_message(wl: *mut winlink, type_: &'static CStr, option: &'static CStr) {
    unsafe {
        let visual =
            visual_option::try_from(options_get_number_((*(*wl).session).options, option) as i32)
                .unwrap();

        for c in tailq_foreach(&raw mut CLIENTS).map(NonNull::as_ptr) {
            if (*c).session != (*wl).session || (*c).flags.intersects(client_flag::CONTROL) {
                continue;
            }

            if visual == visual_option::VISUAL_OFF || visual == visual_option::VISUAL_BOTH {
                tty_putcode(&raw mut (*c).tty, tty_code_code::TTYC_BEL);
            }
            if visual == visual_option::VISUAL_OFF {
                continue;
            }
            if (*(*c).session).curw == wl {
                status_message_set!(
                    c,
                    -1,
                    1,
                    0,
                    "{} in current window",
                    _s(type_.as_ptr().cast::<u8>())
                );
            } else {
                status_message_set!(
                    c,
                    -1,
                    1,
                    0,
                    "{} in window {}",
                    _s(type_.as_ptr().cast::<u8>()),
                    (*wl).idx
                );
            }
        }
    }
}

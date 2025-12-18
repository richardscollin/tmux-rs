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
use crate::options_::{options_get_number___};

/// Alert option values
#[repr(i32)]
#[derive(Copy, Clone, num_enum::TryFromPrimitive)]
pub(crate) enum alert_option {
    ALERT_NONE,
    ALERT_ANY,
    ALERT_CURRENT,
    ALERT_OTHER,
}

static ALERTS_FIRED: atomic::AtomicI32 = atomic::AtomicI32::new(0);

thread_local! {
    static ALERTS_LIST: RefCell<LinkedList<NonNull<window>>> = const { RefCell::new(LinkedList::new()) };
}

unsafe extern "C-unwind" fn alerts_timer(_fd: i32, _events: i16, w: NonNull<window>) {
    unsafe {
        log_debug!("@{} alerts timer expired", (*w.as_ptr()).id);
        alerts_queue(w, window_flag::SILENCE);
    }
}

unsafe extern "C-unwind" fn alerts_callback(_fd: c_int, _events: c_short, _arg: *mut c_void) {
    unsafe {
        ALERTS_LIST.with_borrow_mut(|alerts_list| {
            while let Some(w) = alerts_list.pop_front() {
                let alerts = alerts_check_all(&*w.as_ptr());

                let w = w.as_ptr();
                log_debug!("@{} alerts check, alerts {:#x}", (*w).id, alerts);

                (*w).alerts_queued = 0;

                (*w).flags &= !WINDOW_ALERTFLAGS;
                window_remove_ref(w, c!("alerts_callback"));
            }
        });
        ALERTS_FIRED.store(0, atomic::Ordering::Release);
    }
}

fn alerts_action_applies(wl: &winlink, name: &str) -> bool {
    unsafe {
        match alert_option::try_from(options_get_number___::<i32>(&*(*wl.session).options, name)) {
            Ok(alert_option::ALERT_ANY) => true,
            Ok(alert_option::ALERT_CURRENT) => std::ptr::eq(wl, (*wl.session).curw),
            Ok(alert_option::ALERT_OTHER) => !std::ptr::eq(wl, (*wl.session).curw),
            _ => false,
        }
    }
}

fn alerts_check_all(w: &window) -> window_flag {
    alerts_check_bell(w) | alerts_check_activity(w) | alerts_check_silence(w)
}

pub(crate) fn alerts_check_session(s: &session) {
    unsafe {
        for wl in rb_foreach_const(&raw const s.windows).map(NonNull::as_ptr) {
            alerts_check_all(&*(*wl).window);
        }
    }
}

fn alerts_enabled(w: &window, flags: window_flag) -> bool {
    unsafe {
        if flags.intersects(window_flag::BELL)
            && options_get_number___::<i64>(&*w.options, "monitor-bell") != 0
        {
            return true;
        }
        if flags.intersects(window_flag::ACTIVITY)
            && options_get_number___::<i64>(&*w.options, "monitor-activity") != 0
        {
            return true;
        }
        if flags.intersects(window_flag::SILENCE)
            && options_get_number___::<i64>(&*w.options, "monitor-silence") != 0
        {
            return true;
        }
    }

    false
}

pub(crate) unsafe fn alerts_reset_all() {
    unsafe {
        for w in rb_foreach(&raw mut WINDOWS) {
            alerts_reset(w);
        }
    }
}

unsafe fn alerts_reset(w: NonNull<window>) {
    unsafe {
        if event_initialized(&raw const (*w.as_ptr()).alerts_timer) == 0 {
            evtimer_set(&raw mut (*w.as_ptr()).alerts_timer, alerts_timer, w);
        }

        let w = w.as_ptr();
        (*w).flags &= !window_flag::SILENCE;
        event_del(&raw mut (*w).alerts_timer);

        let mut tv = timeval {
            tv_sec: options_get_number___(&*(*w).options, "monitor-silence"),
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

        if alerts_enabled(&*w, flags) {
            if (*w).alerts_queued == 0 {
                (*w).alerts_queued = 1;
                ALERTS_LIST.with_borrow_mut(|alerts_list| {
                    alerts_list.push_back(NonNull::new_unchecked(w));
                });
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

fn alerts_check_bell(w: &window) -> window_flag {
    unsafe {
        if !w.flags.intersects(window_flag::BELL) {
            return window_flag::empty();
        }
        if options_get_number___::<i64>(&*w.options, "monitor-bell") == 0 {
            return window_flag::empty();
        }

        for wl in tailq_foreach_const::<_, crate::discr_wentry>(&raw const w.winlinks) {
            (*(*wl.as_ptr()).session).flags &= !SESSION_ALERTED;
        }

        for wl in tailq_foreach_const::<_, crate::discr_wentry>(&raw const w.winlinks)
            .map(NonNull::as_ptr)
        {
            // Bells are allowed even if there is an existing bell (so do
            // not check WINLINK_BELL).
            let s = (*wl).session;
            if (*s).curw != wl || (*s).attached == 0 {
                (*wl).flags |= winlink_flags::WINLINK_BELL;
                server_status_session(s);
            }
            if !alerts_action_applies(&*wl, "bell-action") {
                continue;
            }
            notify_winlink(c"alert-bell", wl);

            if (*s).flags & SESSION_ALERTED != 0 {
                continue;
            }
            (*s).flags |= SESSION_ALERTED;

            alerts_set_message(&*wl, "Bell", "visual-bell");
        }
    }
    window_flag::BELL
}

fn alerts_check_activity(w: &window) -> window_flag {
    unsafe {
        if !w.flags.intersects(window_flag::ACTIVITY) {
            return window_flag::empty();
        }
        if options_get_number___::<i64>(&*w.options, "monitor-activity") == 0 {
            return window_flag::empty();
        }

        for wl in tailq_foreach_const::<_, crate::discr_wentry>(&raw const w.winlinks)
            .map(NonNull::as_ptr)
        {
            (*(*wl).session).flags &= !SESSION_ALERTED;
        }

        for wl in tailq_foreach_const::<_, crate::discr_wentry>(&raw const w.winlinks)
            .map(NonNull::as_ptr)
        {
            let s = (*wl).session;
            if (*s).curw != wl || (*s).attached == 0 {
                (*wl).flags |= winlink_flags::WINLINK_ACTIVITY;
                server_status_session(s);
            }
            if !alerts_action_applies(&*wl, "activity-action") {
                continue;
            }
            notify_winlink(c"alert-activity", wl);

            if (*s).flags & SESSION_ALERTED != 0 {
                continue;
            }
            (*s).flags |= SESSION_ALERTED;

            alerts_set_message(&*wl, "Activity", "visual-activity");
        }
    }
    window_flag::ACTIVITY
}

fn alerts_check_silence(w: &window) -> window_flag {
    unsafe {
        if !w.flags.intersects(window_flag::SILENCE) {
            return window_flag::empty();
        }
        if options_get_number___::<i64>(&*w.options, "monitor-silence") == 0 {
            return window_flag::empty();
        }

        for wl in tailq_foreach_const::<_, crate::discr_wentry>(&raw const w.winlinks)
            .map(NonNull::as_ptr)
        {
            (*(*wl).session).flags &= !SESSION_ALERTED;
        }

        for wl in tailq_foreach_const::<_, crate::discr_wentry>(&raw const w.winlinks)
            .map(NonNull::as_ptr)
        {
            if (*wl).flags.intersects(winlink_flags::WINLINK_SILENCE) {
                continue;
            }
            let s = (*wl).session;
            if (*s).curw != wl || (*s).attached == 0 {
                (*wl).flags |= winlink_flags::WINLINK_SILENCE;
                server_status_session(s);
            }
            if !alerts_action_applies(&*wl, "silence-action") {
                continue;
            }
            notify_winlink(c"alert-silence", wl);

            if (*s).flags & SESSION_ALERTED != 0 {
                continue;
            }
            (*s).flags |= SESSION_ALERTED;

            alerts_set_message(&*wl, "Silence", "visual-silence");
        }
    }

    window_flag::SILENCE
}

fn alerts_set_message(wl: &winlink, type_: &str, option: &str) {
    unsafe {
        let visual =
            visual_option::try_from(options_get_number___::<i32>(&*(*wl.session).options, option));

        for c in tailq_foreach(&raw mut CLIENTS).map(NonNull::as_ptr) {
            if (*c).session != wl.session || (*c).flags.intersects(client_flag::CONTROL) {
                continue;
            }

            if matches!(
                visual,
                Ok(visual_option::VISUAL_OFF) | Ok(visual_option::VISUAL_BOTH)
            ) {
                tty_putcode(&raw mut (*c).tty, tty_code_code::TTYC_BEL);
            }
            if matches!(visual, Ok(visual_option::VISUAL_OFF)) {
                continue;
            }
            if std::ptr::eq((*(*c).session).curw, wl) {
                status_message_set!(c, -1, 1, false, "{type_} in current window",);
            } else {
                status_message_set!(c, -1, 1, false, "{type_} in window {}", wl.idx);
            }
        }
    }
}

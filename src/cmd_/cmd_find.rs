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
use crate::compat::{
    queue::{tailq_first, tailq_foreach},
    strlcat,
    tree::{rb_foreach, rb_foreach_const, rb_max, rb_min},
};
use crate::libc::strcmp;
use crate::*;

static CMD_FIND_SESSION_TABLE: [[&str; 2]; 0] = [];

static CMD_FIND_WINDOW_TABLE: [[&str; 2]; 5] = [
    ["{start}", "^"],
    ["{last}", "!"],
    ["{end}", "$"],
    ["{next}", "+"],
    ["{previous}", "-"],
];

static CMD_FIND_PANE_TABLE: [[&str; 2]; 15] = [
    ["{last}", "!"],
    ["{next}", "+"],
    ["{previous}", "-"],
    ["{top}", "top"],
    ["{bottom}", "bottom"],
    ["{left}", "left"],
    ["{right}", "right"],
    ["{top-left}", "top-left"],
    ["{top-right}", "top-right"],
    ["{bottom-left}", "bottom-left"],
    ["{bottom-right}", "bottom-right"],
    ["{up-of}", "{up-of}"],
    ["{down-of}", "{down-of}"],
    ["{left-of}", "{left-of}"],
    ["{right-of}", "{right-of}"],
];

pub unsafe fn cmd_find_inside_pane(c: *mut client) -> *mut window_pane {
    let __func__ = "cmd_find_inside_pane";
    unsafe {
        if c.is_null() {
            return null_mut();
        }

        let mut wp: *mut window_pane = null_mut();
        for wp_ in rb_foreach(&raw mut ALL_WINDOW_PANES) {
            wp = wp_.as_ptr();
            if (*wp).fd != -1 && strcmp((*wp).tty.as_ptr(), (*c).ttyname) == 0 {
                break;
            }
        }

        if wp.is_null() {
            let envent = environ_find((*c).environ, c!("TMUX_PANE"));
            if !envent.is_null() {
                wp = window_pane_find_by_id_str(cstr_to_str(transmute_ptr((*envent).value)));
            }
        }
        if !wp.is_null() {
            log_debug!(
                "{}: got pane %{} ({})",
                __func__,
                (*wp).id,
                _s((*wp).tty.as_ptr())
            );
        }
        wp
    }
}

pub unsafe fn cmd_find_client_better(c: *const client, than: *const client) -> i32 {
    if than.is_null() {
        return 1;
    }
    unsafe {
        timer::new(&raw const (*c).activity_time).cmp(&timer::new(&raw const (*than).activity_time))
            as i32
    }
}

pub unsafe fn cmd_find_best_client(mut s: *const session) -> *mut client {
    unsafe {
        if (*s).attached == 0 {
            s = null();
        }

        let mut c = null_mut();
        for c_loop in tailq_foreach(&raw mut CLIENTS).map(NonNull::as_ptr) {
            if (*c_loop).session.is_null() {
                continue;
            }
            if !s.is_null() && (*c_loop).session != s as *mut session {
                continue;
            }
            if cmd_find_client_better(c_loop, c) != 0 {
                c = c_loop;
            }
        }

        c
    }
}

pub unsafe fn cmd_find_session_better(
    s: *const session,
    than: *const session,
    flags: cmd_find_flags,
) -> i32 {
    if than.is_null() {
        return 1;
    }

    unsafe {
        if flags.intersects(cmd_find_flags::CMD_FIND_PREFER_UNATTACHED) {
            let attached = (*than).attached != 0;
            if attached && (*s).attached == 0 {
                return 1;
            } else if !attached && (*s).attached != 0 {
                return 0;
            }
        }
        (timer::new(&raw const (*s).activity_time) > timer::new(&raw const (*than).activity_time))
            as i32
    }
}

pub unsafe fn cmd_find_best_session(
    slist: *mut *mut session,
    ssize: u32,
    flags: cmd_find_flags,
) -> *mut session {
    unsafe {
        log_debug!("{}: {} sessions to try", "cmd_find_best_session", ssize);

        let mut s = null_mut();
        if !slist.is_null() {
            for i in 0..ssize {
                if cmd_find_session_better(*slist.add(i as usize), s, flags) != 0 {
                    s = *slist.add(i as usize);
                }
            }
        } else {
            for s_loop in rb_foreach(&raw mut SESSIONS).map(std::ptr::NonNull::as_ptr) {
                if cmd_find_session_better(s_loop, s, flags) != 0 {
                    s = s_loop;
                }
            }
        }

        s
    }
}

pub unsafe fn cmd_find_best_session_with_window(fs: *mut cmd_find_state) -> i32 {
    let __func__ = "cmd_find_best_session_with_window";
    unsafe {
        let mut slist: *mut *mut session = null_mut();
        log_debug!("{}: window is @{}", __func__, (*(*fs).w).id);

        'fail: {
            let mut ssize: u32 = 0;
            for s in rb_foreach(&raw mut SESSIONS).map(NonNull::as_ptr) {
                if !session_has(s, (*fs).w) {
                    continue;
                }
                slist = xreallocarray_(slist, ssize as usize + 1).as_ptr();
                *slist.add(ssize as usize) = s;
                ssize += 1;
            }
            if ssize == 0 {
                break 'fail;
            }
            (*fs).s = cmd_find_best_session(slist, ssize, (*fs).flags);
            if (*fs).s.is_null() {
                break 'fail;
            }
            free_(slist);
            return cmd_find_best_winlink_with_window(fs);
        }

        // fail:
        free_(slist);
        -1
    }
}

pub unsafe fn cmd_find_best_winlink_with_window(fs: *mut cmd_find_state) -> i32 {
    let __func__ = "cmd_find_best_winlink_with_window";
    unsafe {
        log_debug!("{}: window is @{}", __func__, (*(*fs).w).id);

        let mut wl = null_mut();
        if !(*(*fs).s).curw.is_null() && (*(*(*fs).s).curw).window == (*fs).w {
            wl = (*(*fs).s).curw;
        } else {
            for wl_loop in rb_foreach(&raw mut (*(*fs).s).windows).map(NonNull::as_ptr) {
                if (*wl_loop).window == (*fs).w {
                    wl = wl_loop;
                    break;
                }
            }
        }
        if wl.is_null() {
            return -1;
        }
        (*fs).wl = wl;
        (*fs).idx = (*(*fs).wl).idx;
    }
    0
}

pub fn cmd_find_map_table<'a>(table: &[[&'static str; 2]], s: &'a str) -> &'a str {
    for entry in table {
        if s == entry[0] {
            return entry[1];
        }
    }
    s
}

pub unsafe fn cmd_find_get_session(fs: *mut cmd_find_state, session: &str) -> i32 {
    let __func__ = "cmd_find_get_session";
    unsafe {
        log_debug!("{}: {}", __func__, session);

        if session.starts_with('$') {
            (*fs).s = session_find_by_id_str(session);
            if (*fs).s.is_null() {
                return -1;
            }
            return 0;
        }

        (*fs).s = session_find(session); // TODO this is invalid casting away const
        if !(*fs).s.is_null() {
            return 0;
        }

        let c = cmd_find_client(null_mut(), Some(session), 1);
        if !c.is_null() && !(*c).session.is_null() {
            (*fs).s = (*c).session;
            return 0;
        }

        if (*fs)
            .flags
            .intersects(cmd_find_flags::CMD_FIND_EXACT_SESSION)
        {
            return -1;
        }

        let session_c = CString::new(session).unwrap();

        let mut s: *mut session = null_mut();
        for s_loop in rb_foreach(&raw mut SESSIONS).map(NonNull::as_ptr) {
            if libc::strncmp(
                session_c.as_ptr().cast(),
                CString::new((*s_loop).name.to_string())
                    .unwrap()
                    .as_ptr()
                    .cast(),
                session.len(),
            ) == 0
            {
                if !s.is_null() {
                    return -1;
                }
                s = s_loop;
            }
        }
        if !s.is_null() {
            (*fs).s = s;
            return 0;
        }

        s = null_mut();
        for s_loop in rb_foreach(&raw mut SESSIONS).map(NonNull::as_ptr) {
            if libc::fnmatch(
                session_c.as_ptr().cast(),
                CString::new((*s_loop).name.to_string())
                    .unwrap()
                    .as_ptr()
                    .cast(),
                0,
            ) == 0
            {
                if !s.is_null() {
                    return -1;
                }
                s = s_loop;
            }
        }
        if !s.is_null() {
            (*fs).s = s;
            return 0;
        }
    }
    -1
}

pub unsafe fn cmd_find_get_window(fs: *mut cmd_find_state, window: &str, only: bool) -> i32 {
    let __func__ = "cmd_find_get_window";
    unsafe {
        log_debug!("{}: {}", __func__, window);

        if window.starts_with('@') {
            (*fs).w = window_find_by_id_str(window);
            if (*fs).w.is_null() {
                return -1;
            }
            return cmd_find_best_session_with_window(fs);
        }

        (*fs).s = (*(*fs).current).s;

        if cmd_find_get_window_with_session(fs, window) == 0 {
            return 0;
        }

        if !only && cmd_find_get_session(fs, window) == 0 {
            (*fs).wl = (*(*fs).s).curw;
            (*fs).w = (*(*fs).wl).window;
            if !(*fs)
                .flags
                .intersects(cmd_find_flags::CMD_FIND_WINDOW_INDEX)
            {
                (*fs).idx = (*(*fs).wl).idx;
            }
            return 0;
        }
    }
    -1
}

pub unsafe fn cmd_find_get_window_with_session(fs: *mut cmd_find_state, window: &str) -> i32 {
    let __func__ = "cmd_find_get_window_with_session";
    unsafe {
        log_debug!("{}: {}", __func__, window);

        let exact = (*fs)
            .flags
            .intersects(cmd_find_flags::CMD_FIND_EXACT_WINDOW);

        (*fs).wl = (*(*fs).s).curw;
        (*fs).w = (*(*fs).wl).window;

        if window.starts_with('@') {
            (*fs).w = window_find_by_id_str(window);
            if (*fs).w.is_null() || !session_has((*fs).s, (*fs).w) {
                return -1;
            }
            return cmd_find_best_winlink_with_window(fs);
        }

        if !exact && (window.starts_with('+') || window.starts_with('-')) {
            let n = if window.len() > 1 {
                strtonum_(&window[1..], 1, i32::MAX).unwrap_or_default()
            } else {
                1
            };
            let s = (*fs).s;
            if (*fs)
                .flags
                .intersects(cmd_find_flags::CMD_FIND_WINDOW_INDEX)
            {
                if window.starts_with('+') {
                    if i32::MAX - (*(*s).curw).idx < n {
                        return -1;
                    }
                    (*fs).idx = (*(*s).curw).idx + n;
                } else {
                    if n > (*(*s).curw).idx {
                        return -1;
                    }
                    (*fs).idx = (*(*s).curw).idx - n;
                }
                return 0;
            }
            if window.starts_with('+') {
                (*fs).wl = winlink_next_by_number((*s).curw, s, n);
            } else {
                (*fs).wl = winlink_previous_by_number((*s).curw, s, n);
            }
            if !(*fs).wl.is_null() {
                (*fs).idx = (*(*fs).wl).idx;
                (*fs).w = (*(*fs).wl).window;
                return 0;
            }
        }

        if !exact {
            match window {
                "!" => {
                    (*fs).wl = tailq_first(&raw mut (*(*fs).s).lastw);
                    if (*fs).wl.is_null() {
                        return -1;
                    }
                    (*fs).idx = (*(*fs).wl).idx;
                    (*fs).w = (*(*fs).wl).window;
                    return 0;
                }
                "^" => {
                    (*fs).wl = rb_min(&raw mut (*(*fs).s).windows);
                    if (*fs).wl.is_null() {
                        return -1;
                    }
                    (*fs).idx = (*(*fs).wl).idx;
                    (*fs).w = (*(*fs).wl).window;
                    return 0;
                }
                "$" => {
                    (*fs).wl = rb_max(&raw mut (*(*fs).s).windows);
                    if (*fs).wl.is_null() {
                        return -1;
                    }
                    (*fs).idx = (*(*fs).wl).idx;
                    (*fs).w = (*(*fs).wl).window;
                    return 0;
                }
                _ => (),
            }
        }

        #[expect(clippy::allow_attributes)]
        #[allow(
            clippy::collapsible_if,
            reason = "collapsing doesn't work with if let; false positive"
        )]
        if !window.starts_with('+') && !window.starts_with('-') {
            if let Ok(idx) = strtonum_(window, 0, i32::MAX) {
                (*fs).wl = winlink_find_by_index(&raw mut (*(*fs).s).windows, idx);
                if !(*fs).wl.is_null() {
                    (*fs).idx = (*(*fs).wl).idx;
                    (*fs).w = (*(*fs).wl).window;
                    return 0;
                }
                if (*fs)
                    .flags
                    .intersects(cmd_find_flags::CMD_FIND_WINDOW_INDEX)
                {
                    (*fs).idx = idx;
                    return 0;
                }
            }
        }

        (*fs).wl = null_mut();
        for wl in rb_foreach(&raw mut (*(*fs).s).windows).map(NonNull::as_ptr) {
            if streq_((*(*wl).window).name, window) {
                if !(*fs).wl.is_null() {
                    return -1;
                }
                (*fs).wl = wl;
            }
        }

        if !(*fs).wl.is_null() {
            (*fs).idx = (*(*fs).wl).idx;
            (*fs).w = (*(*fs).wl).window;
            return 0;
        }

        if exact {
            return -1;
        }
        let window_c = CString::new(window).unwrap();

        (*fs).wl = null_mut();
        for wl in rb_foreach(&raw mut (*(*fs).s).windows).map(NonNull::as_ptr) {
            #[expect(clippy::disallowed_methods)]
            if libc::strncmp(window.as_ptr().cast(), (*(*wl).window).name, window.len()) == 0 {
                if !(*fs).wl.is_null() {
                    return -1;
                }
                (*fs).wl = wl;
            }
        }

        if !(*fs).wl.is_null() {
            (*fs).idx = (*(*fs).wl).idx;
            (*fs).w = (*(*fs).wl).window;
            return 0;
        }

        (*fs).wl = null_mut();
        for wl in rb_foreach(&raw mut (*(*fs).s).windows).map(NonNull::as_ptr) {
            if libc::fnmatch(window_c.as_ptr().cast(), (*(*wl).window).name, 0) == 0 {
                if !(*fs).wl.is_null() {
                    return -1;
                }
                (*fs).wl = wl;
            }
        }

        if !(*fs).wl.is_null() {
            (*fs).idx = (*(*fs).wl).idx;
            (*fs).w = (*(*fs).wl).window;
            return 0;
        }
    }
    -1
}

pub unsafe fn cmd_find_get_pane(fs: *mut cmd_find_state, pane: &str, only: bool) -> i32 {
    let __func__ = "cmd_find_get_pane";
    unsafe {
        log_debug!("{}: {}", __func__, pane);

        if pane.starts_with('%') {
            (*fs).wp = window_pane_find_by_id_str(pane);
            if (*fs).wp.is_null() {
                return -1;
            }
            (*fs).w = (*(*fs).wp).window;
            return cmd_find_best_session_with_window(fs);
        }

        (*fs).s = (*(*fs).current).s;
        (*fs).wl = (*(*fs).current).wl;
        (*fs).idx = (*(*fs).current).idx;
        (*fs).w = (*(*fs).current).w;

        if cmd_find_get_pane_with_window(fs, pane) == 0 {
            return 0;
        }

        if !only && cmd_find_get_window(fs, pane, false) == 0 {
            (*fs).wp = (*(*fs).w).active;
            return 0;
        }
    }
    -1
}

pub unsafe fn cmd_find_get_pane_with_session(fs: *mut cmd_find_state, pane: &str) -> i32 {
    let __func__ = "cmd_find_get_pane_with_session";
    unsafe {
        log_debug!("{}: {}", __func__, pane);

        if pane.starts_with('%') {
            (*fs).wp = window_pane_find_by_id_str(pane);
            if (*fs).wp.is_null() {
                return -1;
            }
            (*fs).w = (*(*fs).wp).window;
            return cmd_find_best_winlink_with_window(fs);
        }

        (*fs).wl = (*(*fs).s).curw;
        (*fs).idx = (*(*fs).wl).idx;
        (*fs).w = (*(*fs).wl).window;

        cmd_find_get_pane_with_window(fs, pane)
    }
}

pub unsafe fn cmd_find_get_pane_with_window(fs: *mut cmd_find_state, pane: &str) -> i32 {
    let __func__ = "cmd_find_get_pane_with_window";
    unsafe {
        log_debug!("{}: {}", __func__, pane);

        if pane.starts_with('%') {
            (*fs).wp = window_pane_find_by_id_str(pane);
            if (*fs).wp.is_null() {
                return -1;
            }
            if (*(*fs).wp).window != (*fs).w {
                return -1;
            }
            return 0;
        }

        match pane {
            "!" => {
                (*fs).wp = tailq_first(&raw mut (*(*fs).w).last_panes);
                if (*fs).wp.is_null() {
                    return -1;
                }
                return 0;
            }
            "{up-of}" => {
                (*fs).wp = window_pane_find_up((*(*fs).w).active);
                if (*fs).wp.is_null() {
                    return -1;
                }
                return 0;
            }
            "{down-of}" => {
                (*fs).wp = window_pane_find_down((*(*fs).w).active);
                if (*fs).wp.is_null() {
                    return -1;
                }
                return 0;
            }
            "{left-of}" => {
                (*fs).wp = window_pane_find_left((*(*fs).w).active);
                if (*fs).wp.is_null() {
                    return -1;
                }
                return 0;
            }
            "{right-of}" => {
                (*fs).wp = window_pane_find_right((*(*fs).w).active);
                if (*fs).wp.is_null() {
                    return -1;
                }
                return 0;
            }
            _ => (),
        }

        if pane.starts_with('+') || pane.starts_with('-') {
            let n = if pane.len() > 1 {
                strtonum_(&pane[1..], 1, i32::MAX).unwrap_or_default() as u32
            } else {
                1
            };
            let wp = (*(*fs).w).active;
            if pane.starts_with('+') {
                (*fs).wp = window_pane_next_by_number((*fs).w, wp, n);
            } else {
                (*fs).wp = window_pane_previous_by_number((*fs).w, wp, n);
            }
            if !(*fs).wp.is_null() {
                return 0;
            }
        }

        if let Ok(idx) = strtonum_(pane, 0, i32::MAX) {
            (*fs).wp = window_pane_at_index((*fs).w, idx as u32);
            if !(*fs).wp.is_null() {
                return 0;
            }
        }

        (*fs).wp = window_find_string((*fs).w, pane);
        if !(*fs).wp.is_null() {
            return 0;
        }
    }
    -1
}

pub unsafe fn cmd_find_clear_state(fs: *mut cmd_find_state, flags: cmd_find_flags) {
    unsafe {
        memset0(fs);

        (*fs).flags = flags;

        (*fs).idx = -1;
    }
}

pub unsafe fn cmd_find_empty_state(fs: *mut cmd_find_state) -> i32 {
    unsafe {
        ((*fs).s.is_null() && (*fs).wl.is_null() && (*fs).w.is_null() && (*fs).wp.is_null()) as i32
    }
}

pub unsafe fn cmd_find_valid_state(fs: *const cmd_find_state) -> bool {
    unsafe {
        if (*fs).s.is_null() || (*fs).wl.is_null() || (*fs).w.is_null() || (*fs).wp.is_null() {
            return false;
        }

        if !session_alive((*fs).s) {
            return false;
        }

        if !rb_foreach_const(&raw const (*(*fs).s).windows)
            .any(|wl| (*wl.as_ptr()).window == (*fs).w && wl.as_ptr() == (*fs).wl)
        {
            return false;
        }

        if (*fs).w != (*(*fs).wl).window {
            return false;
        }

        window_has_pane((*fs).w, (*fs).wp)
    }
}

pub unsafe fn cmd_find_copy_state(dst: *mut cmd_find_state, src: *mut cmd_find_state) {
    unsafe {
        (*dst).s = (*src).s;
        (*dst).wl = (*src).wl;
        (*dst).idx = (*src).idx;
        (*dst).w = (*src).w;
        (*dst).wp = (*src).wp;
    }
}

pub unsafe fn cmd_find_log_state(prefix: *const u8, fs: *const cmd_find_state) {
    unsafe {
        if !(*fs).s.is_null() {
            log_debug!("{}: s=${} {}", _s(prefix), (*(*fs).s).id, (*(*fs).s).name);
        } else {
            log_debug!("{}: s=none", _s(prefix));
        }
        if !(*fs).wl.is_null() {
            log_debug!("{}: wl=%u {}", _s(prefix), (*(*fs).wl).idx);
        } else {
            log_debug!("{}: wl=none", _s(prefix));
        }
        if !(*fs).wp.is_null() {
            log_debug!("{}: wp=%%{}", _s(prefix), (*(*fs).wp).id);
        } else {
            log_debug!("{}: wp=none", _s(prefix));
        }
        if (*fs).idx != -1 {
            log_debug!("{}: idx={}", _s(prefix), (*fs).idx);
        } else {
            log_debug!("{}: idx=none", _s(prefix));
        }
    }
}

pub unsafe fn cmd_find_from_session(
    fs: *mut cmd_find_state,
    s: *mut session,
    flags: cmd_find_flags,
) {
    unsafe {
        cmd_find_clear_state(fs, flags);

        (*fs).s = s;
        (*fs).wl = (*(*fs).s).curw;
        (*fs).w = (*(*fs).wl).window;
        (*fs).wp = (*(*fs).w).active;

        cmd_find_log_state(c!("cmd_find_from_session"), fs);
    }
}

pub unsafe fn cmd_find_from_winlink(
    fs: *mut cmd_find_state,
    wl: *mut winlink,
    flags: cmd_find_flags,
) {
    unsafe {
        cmd_find_clear_state(fs, flags);

        (*fs).s = (*wl).session;
        (*fs).wl = wl;
        (*fs).w = (*wl).window;
        (*fs).wp = (*(*wl).window).active;

        cmd_find_log_state(c!("cmd_find_from_winlink"), fs);
    }
}

pub unsafe fn cmd_find_from_session_window(
    fs: *mut cmd_find_state,
    s: *mut session,
    w: *mut window,
    flags: cmd_find_flags,
) -> i32 {
    unsafe {
        cmd_find_clear_state(fs, flags);

        (*fs).s = s;
        (*fs).w = w;
        if cmd_find_best_winlink_with_window(fs) != 0 {
            cmd_find_clear_state(fs, flags);
            return -1;
        }
        (*fs).wp = (*(*fs).w).active;

        cmd_find_log_state(c!("cmd_find_from_session_window"), fs);
    }
    0
}

pub unsafe fn cmd_find_from_window(
    fs: *mut cmd_find_state,
    w: *mut window,
    flags: cmd_find_flags,
) -> i32 {
    unsafe {
        cmd_find_clear_state(fs, flags);

        (*fs).w = w;
        if cmd_find_best_session_with_window(fs) != 0 {
            cmd_find_clear_state(fs, flags);
            return -1;
        }
        if cmd_find_best_winlink_with_window(fs) != 0 {
            cmd_find_clear_state(fs, flags);
            return -1;
        }
        (*fs).wp = (*(*fs).w).active;

        cmd_find_log_state(c!("cmd_find_from_window"), fs);
        0
    }
}

pub unsafe fn cmd_find_from_winlink_pane(
    fs: *mut cmd_find_state,
    wl: *mut winlink,
    wp: *mut window_pane,
    flags: cmd_find_flags,
) {
    unsafe {
        cmd_find_clear_state(fs, flags);

        (*fs).s = (*wl).session;
        (*fs).wl = wl;
        (*fs).idx = (*(*fs).wl).idx;
        (*fs).w = (*(*fs).wl).window;
        (*fs).wp = wp;

        cmd_find_log_state(c!("cmd_find_from_winlink_pane"), fs);
    }
}

pub unsafe fn cmd_find_from_pane(
    fs: *mut cmd_find_state,
    wp: *mut window_pane,
    flags: cmd_find_flags,
) -> i32 {
    unsafe {
        if cmd_find_from_window(fs, (*wp).window, flags) != 0 {
            return -1;
        }
        (*fs).wp = wp;

        cmd_find_log_state(c!("cmd_find_from_pane"), fs);
    }

    0
}

pub unsafe fn cmd_find_from_nothing(fs: *mut cmd_find_state, flags: cmd_find_flags) -> i32 {
    unsafe {
        cmd_find_clear_state(fs, flags);

        (*fs).s = cmd_find_best_session(null_mut(), 0, flags);
        if (*fs).s.is_null() {
            cmd_find_clear_state(fs, flags);
            return -1;
        }
        (*fs).wl = (*(*fs).s).curw;
        (*fs).idx = (*(*fs).wl).idx;
        (*fs).w = (*(*fs).wl).window;
        (*fs).wp = (*(*fs).w).active;

        cmd_find_log_state(c!("cmd_find_from_nothing"), fs);
    }
    0
}

pub unsafe fn cmd_find_from_mouse(
    fs: *mut cmd_find_state,
    m: *mut mouse_event,
    flags: cmd_find_flags,
) -> i32 {
    unsafe {
        cmd_find_clear_state(fs, flags);

        if !(*m).valid {
            return -1;
        }

        (*fs).wp = transmute_ptr(cmd_mouse_pane(m, &raw mut (*fs).s, &raw mut (*fs).wl));
        if (*fs).wp.is_null() {
            cmd_find_clear_state(fs, flags);
            return -1;
        }
        (*fs).w = (*(*fs).wl).window;

        cmd_find_log_state(c!("cmd_find_from_mouse"), fs);
    }
    0
}

pub unsafe fn cmd_find_from_client(
    fs: *mut cmd_find_state,
    c: *mut client,
    flags: cmd_find_flags,
) -> i32 {
    let __func__ = c!("cmd_find_from_client");
    unsafe {
        'unknown_pane: {
            if c.is_null() {
                return cmd_find_from_nothing(fs, flags);
            }

            if !(*c).session.is_null() {
                cmd_find_clear_state(fs, flags);

                (*fs).wp = server_client_get_pane(c);
                if (*fs).wp.is_null() {
                    cmd_find_from_session(fs, (*c).session, flags);
                    return 0;
                }
                (*fs).s = (*c).session;
                (*fs).wl = (*(*fs).s).curw;
                (*fs).w = (*(*fs).wl).window;

                cmd_find_log_state(__func__, fs);
                return 0;
            }
            cmd_find_clear_state(fs, flags);

            let wp = cmd_find_inside_pane(c);
            if wp.is_null() {
                break 'unknown_pane;
            }

            (*fs).w = (*wp).window;
            if cmd_find_best_session_with_window(fs) != 0 {
                break 'unknown_pane;
            }
            (*fs).wl = (*(*fs).s).curw;
            (*fs).w = (*(*fs).wl).window;
            (*fs).wp = (*(*fs).w).active;

            cmd_find_log_state(__func__, fs);
            return 0;
        }
        // unknown_pane
        cmd_find_from_nothing(fs, flags)
    }
}

pub unsafe fn cmd_find_target(
    fs: *mut cmd_find_state,
    item: *mut cmdq_item,
    target: Option<&str>,
    type_: cmd_find_type,
    mut flags: cmd_find_flags,
) -> i32 {
    let __func__ = "cmd_find_target";

    macro_rules! current {
        ($fs:expr, $flags:expr) => {
            cmd_find_copy_state($fs, (*$fs).current);
            if $flags.intersects(cmd_find_flags::CMD_FIND_WINDOW_INDEX) {
                (*$fs).idx = -1;
            }
            found!($fs)
        };
    }

    unsafe {
        let m: *mut mouse_event;
        let mut current: cmd_find_state = zeroed();

        let sizeof_tmp = 256;
        let mut tmp: [u8; 256] = [0; 256];

        let mut window_only = false;
        let mut pane_only = false;

        if flags.intersects(cmd_find_flags::CMD_FIND_CANFAIL) {
            flags |= cmd_find_flags::CMD_FIND_QUIET;
        }

        let s = match type_ {
            cmd_find_type::CMD_FIND_PANE => "pane",
            cmd_find_type::CMD_FIND_WINDOW => "window",
            cmd_find_type::CMD_FIND_SESSION => "session",
        };

        tmp[0] = b'\0';
        if flags.intersects(cmd_find_flags::CMD_FIND_PREFER_UNATTACHED) {
            strlcat(tmp.as_mut_ptr(), c!("PREFER_UNATTACHED,"), sizeof_tmp);
        }
        if flags.intersects(cmd_find_flags::CMD_FIND_QUIET) {
            strlcat(tmp.as_mut_ptr(), c!("QUIET,"), sizeof_tmp);
        }
        if flags.intersects(cmd_find_flags::CMD_FIND_WINDOW_INDEX) {
            strlcat(tmp.as_mut_ptr(), c!("WINDOW_INDEX,"), sizeof_tmp);
        }
        if flags.intersects(cmd_find_flags::CMD_FIND_DEFAULT_MARKED) {
            strlcat(tmp.as_mut_ptr(), c!("DEFAULT_MARKED,"), sizeof_tmp);
        }
        if flags.intersects(cmd_find_flags::CMD_FIND_EXACT_SESSION) {
            strlcat(tmp.as_mut_ptr(), c!("EXACT_SESSION,"), sizeof_tmp);
        }
        if flags.intersects(cmd_find_flags::CMD_FIND_EXACT_WINDOW) {
            strlcat(tmp.as_mut_ptr(), c!("EXACT_WINDOW,"), sizeof_tmp);
        }
        if flags.intersects(cmd_find_flags::CMD_FIND_CANFAIL) {
            strlcat(tmp.as_mut_ptr(), c!("CANFAIL,"), sizeof_tmp);
        }
        if tmp[0] != b'\0' {
            tmp[strlen(tmp.as_mut_ptr()) - 1] = b'\0';
        } else {
            strlcat(tmp.as_mut_ptr(), c!("NONE"), sizeof_tmp);
        }
        log_debug!(
            "{}: target {}, type {}, item {:p}, flags {}",
            __func__,
            target.unwrap_or("none"),
            s,
            item,
            _s(tmp.as_ptr()),
        );

        cmd_find_clear_state(fs, flags);

        if server_check_marked() && flags.intersects(cmd_find_flags::CMD_FIND_DEFAULT_MARKED) {
            (*fs).current = &raw mut MARKED_PANE;
            log_debug!("{}: current is marked pane", __func__);
        } else if cmd_find_valid_state(cmdq_get_current(item)) {
            (*fs).current = cmdq_get_current(item);
            log_debug!("{}: current is from queue", __func__);
        } else if cmd_find_from_client(&raw mut current, cmdq_get_client(item), flags) == 0 {
            (*fs).current = &raw mut current;
            log_debug!("{}: current is from client", __func__);
        } else {
            if !flags.intersects(cmd_find_flags::CMD_FIND_QUIET) {
                cmdq_error!(item, "no current target");
            }
            return_error!(fs, flags);
        }
        if !cmd_find_valid_state((*fs).current) {
            fatalx("invalid current find state");
        }

        // An empty or NULL target is the current.
        let Some(target) = target else {
            current!(fs, flags);
        };
        if target.is_empty() {
            current!(fs, flags);
        }

        // Mouse target is a plain = or {mouse}.
        if target == "=" || target == "{mouse}" {
            m = &raw mut (*cmdq_get_event(item)).m;
            match type_ {
                cmd_find_type::CMD_FIND_PANE => {
                    (*fs).wp =
                        transmute_ptr(cmd_mouse_pane(m, &raw mut (*fs).s, &raw mut (*fs).wl));
                    if !(*fs).wp.is_null() {
                        (*fs).w = (*(*fs).wl).window;
                    } else {
                        // FALLTHROUGH; copied from below
                        (*fs).wl = transmute_ptr(cmd_mouse_window(m, &raw mut (*fs).s));
                        if (*fs).wl.is_null() && !(*fs).s.is_null() {
                            (*fs).wl = (*(*fs).s).curw;
                        }
                        if !(*fs).wl.is_null() {
                            (*fs).w = (*(*fs).wl).window;
                            (*fs).wp = (*(*fs).w).active;
                        }
                    }
                }
                cmd_find_type::CMD_FIND_WINDOW | cmd_find_type::CMD_FIND_SESSION => {
                    (*fs).wl = transmute_ptr(cmd_mouse_window(m, &raw mut (*fs).s));
                    if (*fs).wl.is_null() && !(*fs).s.is_null() {
                        (*fs).wl = (*(*fs).s).curw;
                    }
                    if !(*fs).wl.is_null() {
                        (*fs).w = (*(*fs).wl).window;
                        (*fs).wp = (*(*fs).w).active;
                    }
                }
            }
            if (*fs).wp.is_null() {
                if !flags.intersects(cmd_find_flags::CMD_FIND_QUIET) {
                    cmdq_error!(item, "no mouse target");
                }
                return_error!(fs, flags);
            }
            found!(fs);
        }

        if target == "~" || target == "{marked}" {
            if !server_check_marked() {
                if !flags.intersects(cmd_find_flags::CMD_FIND_QUIET) {
                    cmdq_error!(item, "no marked target");
                }
                return_error!(fs, flags);
            }
            cmd_find_copy_state(fs, &raw mut MARKED_PANE);
            found!(fs);
        }

        let copy = target; // No need to make copy due to rust slice
        let colon = copy.find(':');
        let period = if let Some(colon) = colon {
            copy[colon + 1..].find('.').map(|i| colon + 1 + i)
        } else {
            copy.find('.')
        };

        let mut session: Option<&str> = None;
        let mut window: Option<&str> = None;
        let mut pane: Option<&str> = None;
        match (colon, period) {
            (Some(colon), Some(period)) => {
                session = Some(&copy[..colon]);
                window = Some(&copy[colon + 1..period]);
                window_only = true;
                pane = Some(&copy[period + 1..]);
                pane_only = true;
            }
            (Some(colon), None) => {
                session = Some(&copy[..colon]);
                window = Some(&copy[colon + 1..]);
                window_only = true;
            }
            (None, Some(period)) => {
                window = Some(&copy[..period]);
                pane = Some(&copy[period + 1..]);
                pane_only = true;
            }
            (None, None) => match copy.chars().next() {
                Some('$') => session = Some(copy),
                Some('@') => window = Some(copy),
                Some('%') => pane = Some(copy),
                _ => match type_ {
                    cmd_find_type::CMD_FIND_SESSION => session = Some(copy),
                    cmd_find_type::CMD_FIND_WINDOW => window = Some(copy),
                    cmd_find_type::CMD_FIND_PANE => pane = Some(copy),
                },
            },
        }

        if session.is_some_and(|s| s.starts_with('=')) {
            session = session.map(|s| &s[1..]);
            (*fs).flags |= cmd_find_flags::CMD_FIND_EXACT_SESSION;
        }
        if window.is_some_and(|w| w.starts_with('=')) {
            window = window.map(|w| &w[1..]);
            (*fs).flags |= cmd_find_flags::CMD_FIND_EXACT_WINDOW;
        }

        if session.is_some_and(str::is_empty) {
            session = None;
        }
        if window.is_some_and(str::is_empty) {
            window = None;
        }
        if pane.is_some_and(str::is_empty) {
            pane = None;
        }

        if session.is_some() {
            session = Some(cmd_find_map_table(
                &CMD_FIND_SESSION_TABLE,
                session.unwrap(),
            ));
        }
        if window.is_some() {
            window = Some(cmd_find_map_table(&CMD_FIND_WINDOW_TABLE, window.unwrap()));
        }
        if pane.is_some() {
            pane = Some(cmd_find_map_table(&CMD_FIND_PANE_TABLE, pane.unwrap()));
        }

        if session.is_some() || window.is_some() || pane.is_some() {
            log_debug!(
                "{}: target {} is {}{}{}{}{}{}",
                __func__,
                target,
                if session.is_none() { "" } else { "session " },
                session.unwrap_or_default(),
                if window.is_none() { "" } else { "window " },
                window.unwrap_or_default(),
                if pane.is_none() { "" } else { "pane " },
                pane.unwrap_or_default(),
            );
        }

        if pane.is_some() && flags.intersects(cmd_find_flags::CMD_FIND_WINDOW_INDEX) {
            if !flags.intersects(cmd_find_flags::CMD_FIND_QUIET) {
                cmdq_error!(item, "can't specify pane here");
            }
            return_error!(fs, flags);
        }

        if let Some(session) = session {
            if cmd_find_get_session(fs, session) != 0 {
                no_session!(item, session, fs, flags);
            }

            match (window, pane) {
                (None, None) => {
                    (*fs).wl = (*(*fs).s).curw;
                    (*fs).idx = -1;
                    (*fs).w = (*(*fs).wl).window;
                    (*fs).wp = (*(*fs).w).active;
                    found!(fs);
                }
                (Some(window), None) => {
                    if cmd_find_get_window_with_session(fs, window) != 0 {
                        no_window!(item, window, fs, flags);
                    }
                    if !(*fs).wl.is_null() {
                        (*fs).wp = (*(*(*fs).wl).window).active;
                    }
                    found!(fs);
                }
                (None, Some(pane)) => {
                    if cmd_find_get_pane_with_session(fs, pane) != 0 {
                        no_pane!(item, pane, fs, flags);
                    }
                    found!(fs);
                }
                (Some(window), Some(pane)) => {
                    if cmd_find_get_window_with_session(fs, window) != 0 {
                        no_window!(item, window, fs, flags);
                    }
                    if cmd_find_get_pane_with_window(fs, pane) != 0 {
                        no_pane!(item, pane, fs, flags);
                    }
                    found!(fs);
                }
            }
        }

        match (window, pane) {
            (Some(window), Some(pane)) => {
                if cmd_find_get_window(fs, window, window_only) != 0 {
                    no_window!(item, window, fs, flags);
                }
                if cmd_find_get_pane_with_window(fs, pane) != 0 {
                    no_pane!(item, pane, fs, flags);
                }
                found!(fs);
            }
            (Some(window), None) => {
                if cmd_find_get_window(fs, window, window_only) != 0 {
                    no_window!(item, window, fs, flags);
                }
                if !(*fs).wl.is_null() {
                    (*fs).wp = (*(*(*fs).wl).window).active;
                }
                found!(fs);
            }
            (None, Some(pane)) => {
                if cmd_find_get_pane(fs, pane, pane_only) != 0 {
                    no_pane!(item, pane, fs, flags);
                }
                found!(fs);
            }
            (None, None) => {
                current!(fs, flags);
            }
        }

        macro_rules! found {
            ($fs:expr) => {
                (*$fs).current = null_mut();
                cmd_find_log_state(c!("cmd_find_target"), $fs);
                return 0;
            };
        }
        use found;

        macro_rules! no_pane {
            ($item:expr, $pane:expr, $fs:expr, $flags:expr) => {
                if !$flags.intersects(cmd_find_flags::CMD_FIND_QUIET) {
                    cmdq_error!($item, "can't find pane: {}", $pane);
                }
                return_error!($fs, $flags);
            };
        }
        use no_pane;

        macro_rules! no_session {
            ($item:expr, $session:expr, $fs:expr, $flags:expr) => {
                if !$flags.intersects(cmd_find_flags::CMD_FIND_QUIET) {
                    cmdq_error!($item, "can't find session: {}", $session);
                }
                return_error!($fs, $flags);
            };
        }
        use no_session;

        macro_rules! no_window {
            ($item:expr, $window:expr, $fs:expr, $flags:expr) => {
                if !$flags.intersects(cmd_find_flags::CMD_FIND_QUIET) {
                    cmdq_error!($item, "can't find window: {}", $window);
                }
                return_error!($fs, $flags);
            };
        }
        use no_window;

        macro_rules! return_error {
            ($fs:expr, $flags:expr) => {
                (*$fs).current = null_mut();
                log_debug!("cmd_find_target: error");

                if $flags.intersects(cmd_find_flags::CMD_FIND_CANFAIL) {
                    return 0;
                } else {
                    return -1;
                }
            };
        }
        use return_error;
    }
}

pub unsafe fn cmd_find_current_client(item: *mut cmdq_item, quiet: i32) -> *mut client {
    let __func__ = "cmd_find_current_client";
    unsafe {
        let mut c: *mut client = null_mut();
        let wp;
        let mut fs: cmd_find_state = zeroed();

        if !item.is_null() {
            c = cmdq_get_client(item);
        }
        if !c.is_null() && !(*c).session.is_null() {
            return c;
        }

        let mut found: *mut client = null_mut();
        if !c.is_null()
            && ({
                wp = cmd_find_inside_pane(c);
                !wp.is_null()
            })
        {
            cmd_find_clear_state(&raw mut fs, cmd_find_flags::CMD_FIND_QUIET);
            fs.w = (*wp).window;
            if cmd_find_best_session_with_window(&raw mut fs) == 0 {
                found = cmd_find_best_client(fs.s);
            }
        } else {
            let s = cmd_find_best_session(null_mut(), 0, cmd_find_flags::CMD_FIND_QUIET);
            if !s.is_null() {
                found = cmd_find_best_client(s);
            }
        }
        if found.is_null() && !item.is_null() && quiet == 0 {
            cmdq_error!(item, "no current client");
        }
        log_debug!("{}: no target, return {:p}", __func__, found);
        found
    }
}

pub unsafe fn cmd_find_client(
    item: *mut cmdq_item,
    target: Option<&str>,
    quiet: i32,
) -> *mut client {
    let __func__ = "cmd_find_client";
    unsafe {
        // struct client *c;
        // char *copy;
        // size_t size;

        // A NULL argument means the current client.
        let Some(target) = target else {
            return cmd_find_current_client(item, quiet);
        };

        // Trim a single trailing colon if any.
        let copy = target.strip_suffix(':').unwrap_or(target);

        let mut c = null_mut();
        // Check name and path of each client.
        for c_ in tailq_foreach(&raw mut CLIENTS) {
            c = c_.as_ptr();
            if (*c).session.is_null() {
                continue;
            }
            if streq_((*c).name, copy) {
                break;
            }

            if *(*c).ttyname == b'\0' {
                continue;
            }
            if streq_((*c).ttyname, copy) {
                break;
            }
            if libc::strncmp((*c).ttyname, _PATH_DEV, SIZEOF_PATH_DEV - 1) != 0 {
                continue;
            }
            if streq_((*c).ttyname.add(SIZEOF_PATH_DEV - 1), copy) {
                break;
            }

            continue;
        }

        if c.is_null() && quiet == 0 {
            cmdq_error!(item, "can't find client: {}", copy);
        }

        log_debug!("{}: target {}, return {:p}", __func__, target, c);
        c
    }
}

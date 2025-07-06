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

use crate::libc::strcmp;

use crate::compat::{
    queue::{tailq_first, tailq_foreach},
    strlcat,
    tree::{rb_foreach, rb_foreach_const, rb_max, rb_min},
};

static mut CMD_FIND_SESSION_TABLE: [[*const u8; 2]; 1] = [[null_mut(), null_mut()]];

static mut CMD_FIND_WINDOW_TABLE: [[*const u8; 2]; 6] = [
    [c!("{start}"), c!("^")],
    [c!("{last}"), c!("!")],
    [c!("{end}"), c!("$")],
    [c!("{next}"), c!("+")],
    [c!("{previous}"), c!("-")],
    [null(), null()],
];

static mut CMD_FIND_PANE_TABLE: [[*const u8; 2]; 16] = [
    [c!("{last}"), c!("!")],
    [c!("{next}"), c!("+")],
    [c!("{previous}"), c!("-")],
    [c!("{top}"), c!("top")],
    [c!("{bottom}"), c!("bottom")],
    [c!("{left}"), c!("left")],
    [c!("{right}"), c!("right")],
    [c!("{top-left}"), c!("top-left")],
    [c!("{top-right}"), c!("top-right")],
    [c!("{bottom-left}"), c!("bottom-left")],
    [c!("{bottom-right}"), c!("bottom-right")],
    [c!("{up-of}"), c!("{up-of}")],
    [c!("{down-of}"), c!("{down-of}")],
    [c!("{left-of}"), c!("{left-of}")],
    [c!("{right-of}.as_ptr()"), c!("{right-of}")],
    [null(), null()],
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
                wp = window_pane_find_by_id_str(transmute_ptr((*envent).value));
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

pub unsafe fn cmd_find_client_better(c: *mut client, than: *mut client) -> i32 {
    if than.is_null() {
        return 1;
    }
    unsafe {
        timer::new(&raw const (*c).activity_time).cmp(&timer::new(&raw const (*than).activity_time))
            as i32
    }
}

pub unsafe fn cmd_find_best_client(mut s: *mut session) -> *mut client {
    unsafe {
        if (*s).attached == 0 {
            s = null_mut();
        }

        let mut c = null_mut();
        for c_loop in tailq_foreach(&raw mut CLIENTS).map(NonNull::as_ptr) {
            if (*c_loop).session.is_null() {
                continue;
            }
            if !s.is_null() && (*c_loop).session != s {
                continue;
            }
            if cmd_find_client_better(c_loop, c) != 0 {
                c = c_loop;
            }
        }

        c
    }
}

pub unsafe fn cmd_find_session_better(s: *mut session, than: *mut session, flags: i32) -> i32 {
    if than.is_null() {
        return 1;
    }

    unsafe {
        if flags & CMD_FIND_PREFER_UNATTACHED != 0 {
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
    flags: i32,
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
            for s_loop in rb_foreach(&raw mut SESSIONS).map(|e| e.as_ptr()) {
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
                if session_has(s, (*fs).w) == 0 {
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

pub unsafe fn cmd_find_map_table(table: *const [*const u8; 2], s: *const u8) -> *const u8 {
    unsafe {
        let mut i = 0;
        while !(*table.add(i))[0].is_null() {
            if unsafe { strcmp(s, (*table.add(i))[0]) == 0 } {
                return (*table.add(i))[1];
            }
            i += 1;
        }
        s
    }
}

pub unsafe fn cmd_find_get_session(fs: *mut cmd_find_state, session: *const u8) -> i32 {
    let __func__ = "cmd_find_get_session";
    unsafe {
        log_debug!("{}: {}", __func__, _s(session));

        if *session == b'$' as _ {
            (*fs).s = session_find_by_id_str(session);
            if (*fs).s.is_null() {
                return -1;
            }
            return 0;
        }

        (*fs).s = session_find(session.cast_mut()); // TODO this is invalid casting away const
        if !(*fs).s.is_null() {
            return 0;
        }

        let c = cmd_find_client(null_mut(), session, 1);
        if !c.is_null() && !(*c).session.is_null() {
            (*fs).s = (*c).session;
            return 0;
        }

        if (*fs).flags & CMD_FIND_EXACT_SESSION != 0 {
            return -1;
        }

        let mut s: *mut session = null_mut();
        for s_loop in rb_foreach(&raw mut SESSIONS).map(NonNull::as_ptr) {
            if libc::strncmp(session, (*s_loop).name, strlen(session)) == 0 {
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
            if libc::fnmatch(session, (*s_loop).name, 0) == 0 {
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

pub unsafe fn cmd_find_get_window(fs: *mut cmd_find_state, window: *const u8, only: i32) -> i32 {
    let __func__ = "cmd_find_get_window";
    unsafe {
        log_debug!("{}: {}", __func__, _s(window));

        if *window == b'@' {
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

        if only == 0 && cmd_find_get_session(fs, window) == 0 {
            (*fs).wl = (*(*fs).s).curw;
            (*fs).w = (*(*fs).wl).window;
            if !(*fs).flags & CMD_FIND_WINDOW_INDEX != 0 {
                (*fs).idx = (*(*fs).wl).idx;
            }
            return 0;
        }
    }
    -1
}

pub unsafe fn cmd_find_get_window_with_session(fs: *mut cmd_find_state, window: *const u8) -> i32 {
    let __func__ = "cmd_find_get_window_with_session";
    unsafe {
        let mut errstr: *const u8 = null();
        let mut n = 0i32;
        let mut exact = 0i32;
        let mut s = null_mut();

        log_debug!("{}: {}", __func__, _s(window));
        exact = (*fs).flags & CMD_FIND_EXACT_WINDOW;

        (*fs).wl = (*(*fs).s).curw;
        (*fs).w = (*(*fs).wl).window;

        if *window == b'@' as _ {
            (*fs).w = window_find_by_id_str(window);
            if (*fs).w.is_null() || session_has((*fs).s, (*fs).w) == 0 {
                return -1;
            }
            return cmd_find_best_winlink_with_window(fs);
        }

        if exact == 0 && (*window == b'+' as _ || *window == b'-' as _) {
            n = if *window.add(1) != b'\0' as _ {
                strtonum(window.add(1), 1, i32::MAX).unwrap_or_default()
            } else {
                1
            };
            s = (*fs).s;
            if (*fs).flags & CMD_FIND_WINDOW_INDEX != 0 {
                if *window == b'+' as _ {
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
            if *window == b'+' as _ {
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

        if exact == 0 {
            if streq_(window, "!") {
                (*fs).wl = tailq_first(&raw mut (*(*fs).s).lastw);
                if (*fs).wl.is_null() {
                    return -1;
                }
                (*fs).idx = (*(*fs).wl).idx;
                (*fs).w = (*(*fs).wl).window;
                return 0;
            } else if streq_(window, "^") {
                (*fs).wl = rb_min(&raw mut (*(*fs).s).windows);
                if (*fs).wl.is_null() {
                    return -1;
                }
                (*fs).idx = (*(*fs).wl).idx;
                (*fs).w = (*(*fs).wl).window;
                return 0;
            } else if streq_(window, "$") {
                (*fs).wl = rb_max(&raw mut (*(*fs).s).windows);
                if (*fs).wl.is_null() {
                    return -1;
                }
                (*fs).idx = (*(*fs).wl).idx;
                (*fs).w = (*(*fs).wl).window;
                return 0;
            }
        }

        #[allow(
            clippy::collapsible_if,
            reason = "collapsing doesn't work with if let; false positive"
        )]
        if *window != b'+' as _ && *window != b'-' {
            if let Ok(idx) = strtonum(window, 0, i32::MAX) {
                (*fs).wl = winlink_find_by_index(&raw mut (*(*fs).s).windows, idx);
                if !(*fs).wl.is_null() {
                    (*fs).idx = (*(*fs).wl).idx;
                    (*fs).w = (*(*fs).wl).window;
                    return 0;
                }
                if (*fs).flags & CMD_FIND_WINDOW_INDEX != 0 {
                    (*fs).idx = idx;
                    return 0;
                }
            }
        }

        (*fs).wl = null_mut();
        for wl in rb_foreach(&raw mut (*(*fs).s).windows).map(NonNull::as_ptr) {
            if strcmp(window, (*(*wl).window).name) == 0 {
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

        if exact != 0 {
            return -1;
        }

        (*fs).wl = null_mut();
        for wl in rb_foreach(&raw mut (*(*fs).s).windows).map(NonNull::as_ptr) {
            if libc::strncmp(window, (*(*wl).window).name, strlen(window)) == 0 {
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
            if libc::fnmatch(window, (*(*wl).window).name, 0) == 0 {
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

pub unsafe fn cmd_find_get_pane(fs: *mut cmd_find_state, pane: *const u8, only: i32) -> i32 {
    let __func__ = "cmd_find_get_pane";
    unsafe {
        log_debug!("{}: {}", __func__, _s(pane));

        if *pane == b'%' as _ {
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

        if only == 0 && cmd_find_get_window(fs, pane, 0) == 0 {
            (*fs).wp = (*(*fs).w).active;
            return 0;
        }
    }
    -1
}

pub unsafe fn cmd_find_get_pane_with_session(fs: *mut cmd_find_state, pane: *const u8) -> i32 {
    let __func__ = "cmd_find_get_pane_with_session";
    unsafe {
        log_debug!("{}: {}", __func__, _s(pane));

        if *pane == b'%' as _ {
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

pub unsafe fn cmd_find_get_pane_with_window(fs: *mut cmd_find_state, pane: *const u8) -> i32 {
    let __func__ = "cmd_find_get_pane_with_window";
    unsafe {
        let mut n = 0u32;
        let mut errstr: *const u8 = null();

        log_debug!("{}: {}", __func__, _s(pane));

        if *pane == b'%' as _ {
            (*fs).wp = window_pane_find_by_id_str(pane);
            if (*fs).wp.is_null() {
                return -1;
            }
            if (*(*fs).wp).window != (*fs).w {
                return -1;
            }
            return 0;
        }

        if streq_(pane, "!") {
            (*fs).wp = tailq_first(&raw mut (*(*fs).w).last_panes);
            if (*fs).wp.is_null() {
                return -1;
            }
            return 0;
        } else if streq_(pane, "{up-of}") {
            (*fs).wp = window_pane_find_up((*(*fs).w).active);
            if (*fs).wp.is_null() {
                return -1;
            }
            return 0;
        } else if streq_(pane, "{down-of}") {
            (*fs).wp = window_pane_find_down((*(*fs).w).active);
            if (*fs).wp.is_null() {
                return -1;
            }
            return 0;
        } else if streq_(pane, "{left-of}") {
            (*fs).wp = window_pane_find_left((*(*fs).w).active);
            if (*fs).wp.is_null() {
                return -1;
            }
            return 0;
        } else if streq_(pane, "{right-of}") {
            (*fs).wp = window_pane_find_right((*(*fs).w).active);
            if (*fs).wp.is_null() {
                return -1;
            }
            return 0;
        }

        if *pane == b'+' as _ || *pane == b'-' as _ {
            n = if *pane.add(1) != b'\0' as _ {
                strtonum(pane.add(1), 1, i32::MAX).unwrap_or_default() as u32
            } else {
                1
            };
            let wp = (*(*fs).w).active;
            if *pane == b'+' as _ {
                (*fs).wp = window_pane_next_by_number((*fs).w, wp, n);
            } else {
                (*fs).wp = window_pane_previous_by_number((*fs).w, wp, n);
            }
            if !(*fs).wp.is_null() {
                return 0;
            }
        }

        if let Ok(idx) = strtonum(pane, 0, i32::MAX) {
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

pub unsafe fn cmd_find_clear_state(fs: *mut cmd_find_state, flags: i32) {
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

        let Some(wl) = rb_foreach_const(&raw const (*(*fs).s).windows)
            .find(|wl| (*wl.as_ptr()).window == (*fs).w && wl.as_ptr() == (*fs).wl)
        else {
            return false;
        };

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
            log_debug!(
                "{}: s=${} {}",
                _s(prefix),
                (*(*fs).s).id,
                _s((*(*fs).s).name)
            );
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

pub unsafe fn cmd_find_from_session(fs: *mut cmd_find_state, s: *mut session, flags: i32) {
    unsafe {
        cmd_find_clear_state(fs, flags);

        (*fs).s = s;
        (*fs).wl = (*(*fs).s).curw;
        (*fs).w = (*(*fs).wl).window;
        (*fs).wp = (*(*fs).w).active;

        cmd_find_log_state(c!("cmd_find_from_session"), fs);
    }
}

pub unsafe fn cmd_find_from_winlink(fs: *mut cmd_find_state, wl: *mut winlink, flags: i32) {
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
    flags: i32,
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

pub unsafe fn cmd_find_from_window(fs: *mut cmd_find_state, w: *mut window, flags: i32) -> i32 {
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
    flags: i32,
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

pub unsafe fn cmd_find_from_pane(fs: *mut cmd_find_state, wp: *mut window_pane, flags: i32) -> i32 {
    unsafe {
        if cmd_find_from_window(fs, (*wp).window, flags) != 0 {
            return -1;
        }
        (*fs).wp = wp;

        cmd_find_log_state(c!("cmd_find_from_pane"), fs);
    }

    0
}

pub unsafe fn cmd_find_from_nothing(fs: *mut cmd_find_state, flags: i32) -> i32 {
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

pub unsafe fn cmd_find_from_mouse(fs: *mut cmd_find_state, m: *mut mouse_event, flags: i32) -> i32 {
    unsafe {
        cmd_find_clear_state(fs, flags);

        if (*m).valid == 0 {
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

pub unsafe fn cmd_find_from_client(fs: *mut cmd_find_state, c: *mut client, flags: i32) -> i32 {
    let __func__ = c!("cmd_find_from_client");
    unsafe {
        // struct window_pane *wp;

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
        // unknown_pane:
        cmd_find_from_nothing(fs, flags)
    }
}

pub unsafe fn cmd_find_target(
    fs: *mut cmd_find_state,
    item: *mut cmdq_item,
    target: *const u8,
    type_: cmd_find_type,
    mut flags: i32,
) -> i32 {
    let __func__ = "cmd_find_target";
    unsafe {
        let mut m: *mut mouse_event = null_mut();
        let mut current: cmd_find_state = zeroed();

        let mut colon: *mut u8 = null_mut();
        let mut period: *mut u8 = null_mut();
        let mut copy: *mut u8 = null_mut();
        let sizeof_tmp = 256;
        let mut tmp: [u8; 256] = [0; 256];

        let mut session: *const u8 = null();
        let mut window: *const u8 = null();
        let mut pane: *const u8 = null();
        let mut s: *const u8 = null();

        let mut window_only = 0;
        let mut pane_only = 0;

        'error: {
            'no_pane: {
                'no_window: {
                    'no_session: {
                        'found: {
                            'current: {
                                if flags & CMD_FIND_CANFAIL != 0 {
                                    flags |= CMD_FIND_QUIET;
                                }

                                s = match type_ {
                                    cmd_find_type::CMD_FIND_PANE => c!("pane"),
                                    cmd_find_type::CMD_FIND_WINDOW => c!("window"),
                                    cmd_find_type::CMD_FIND_SESSION => c!("session"),
                                };

                                tmp[0] = b'\0';
                                if flags & CMD_FIND_PREFER_UNATTACHED != 0 {
                                    strlcat(tmp.as_mut_ptr(), c!("PREFER_UNATTACHED,"), sizeof_tmp);
                                }
                                if flags & CMD_FIND_QUIET != 0 {
                                    strlcat(tmp.as_mut_ptr(), c!("QUIET,"), sizeof_tmp);
                                }
                                if flags & CMD_FIND_WINDOW_INDEX != 0 {
                                    strlcat(tmp.as_mut_ptr(), c!("WINDOW_INDEX,"), sizeof_tmp);
                                }
                                if flags & CMD_FIND_DEFAULT_MARKED != 0 {
                                    strlcat(tmp.as_mut_ptr(), c!("DEFAULT_MARKED,"), sizeof_tmp);
                                }
                                if flags & CMD_FIND_EXACT_SESSION != 0 {
                                    strlcat(tmp.as_mut_ptr(), c!("EXACT_SESSION,"), sizeof_tmp);
                                }
                                if flags & CMD_FIND_EXACT_WINDOW != 0 {
                                    strlcat(tmp.as_mut_ptr(), c!("EXACT_WINDOW,"), sizeof_tmp);
                                }
                                if flags & CMD_FIND_CANFAIL != 0 {
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
                                    if target.is_null() {
                                        _s(c!("none"))
                                    } else {
                                        _s(target)
                                    },
                                    _s(s),
                                    item,
                                    _s(tmp.as_ptr()),
                                );

                                cmd_find_clear_state(fs, flags);

                                if server_check_marked() && (flags & CMD_FIND_DEFAULT_MARKED != 0) {
                                    (*fs).current = &raw mut MARKED_PANE;
                                    log_debug!("{}: current is marked pane", __func__);
                                } else if cmd_find_valid_state(cmdq_get_current(item)) {
                                    (*fs).current = cmdq_get_current(item);
                                    log_debug!("{}: current is from queue", __func__);
                                } else if cmd_find_from_client(
                                    &raw mut current,
                                    cmdq_get_client(item),
                                    flags,
                                ) == 0
                                {
                                    (*fs).current = &raw mut current;
                                    log_debug!("{}: current is from client", __func__);
                                } else {
                                    if !flags & CMD_FIND_QUIET != 0 {
                                        cmdq_error!(item, "no current target");
                                    }
                                    break 'error;
                                }
                                if !cmd_find_valid_state((*fs).current) {
                                    fatalx("invalid current find state");
                                }

                                /* An empty or NULL target is the current. */
                                if target.is_null() || *target == b'\0' as _ {
                                    break 'current;
                                }

                                /* Mouse target is a plain = or {mouse}. */
                                if streq_(target, "=") || streq_(target, "{mouse}") {
                                    m = &raw mut (*cmdq_get_event(item)).m;
                                    match type_ {
                                        cmd_find_type::CMD_FIND_PANE => {
                                            (*fs).wp = transmute_ptr(cmd_mouse_pane(
                                                m,
                                                &raw mut (*fs).s,
                                                &raw mut (*fs).wl,
                                            ));
                                            if !(*fs).wp.is_null() {
                                                (*fs).w = (*(*fs).wl).window;
                                            } else {
                                                /* FALLTHROUGH; copied from below */
                                                (*fs).wl = transmute_ptr(cmd_mouse_window(
                                                    m,
                                                    &raw mut (*fs).s,
                                                ));
                                                if (*fs).wl.is_null() && !(*fs).s.is_null() {
                                                    (*fs).wl = (*(*fs).s).curw;
                                                }
                                                if !(*fs).wl.is_null() {
                                                    (*fs).w = (*(*fs).wl).window;
                                                    (*fs).wp = (*(*fs).w).active;
                                                }
                                            }
                                        }
                                        cmd_find_type::CMD_FIND_WINDOW
                                        | cmd_find_type::CMD_FIND_SESSION => {
                                            (*fs).wl = transmute_ptr(cmd_mouse_window(
                                                m,
                                                &raw mut (*fs).s,
                                            ));
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
                                        if !flags & CMD_FIND_QUIET != 0 {
                                            cmdq_error!(item, "no mouse target");
                                        }
                                        break 'error;
                                    }
                                    break 'found;
                                }

                                if streq_(target, "~") || streq_(target, "{marked}") {
                                    if !server_check_marked() {
                                        if !flags & CMD_FIND_QUIET != 0 {
                                            cmdq_error!(item, "no marked target");
                                        }
                                        break 'error;
                                    }
                                    cmd_find_copy_state(fs, &raw mut MARKED_PANE);
                                    break 'found;
                                }

                                copy = xstrdup(target).as_ptr();
                                colon = libc::strchr(copy, b':' as _);
                                if !colon.is_null() {
                                    *colon = b'\0' as _;
                                    colon = colon.add(1);
                                }
                                if colon.is_null() {
                                    period = libc::strchr(copy, b'.' as _);
                                } else {
                                    period = libc::strchr(colon, b'.' as _);
                                }
                                if !period.is_null() {
                                    *period = b'\0' as _;
                                    period = period.add(1);
                                }

                                session = null_mut();
                                window = null_mut();
                                pane = null_mut();
                                if !colon.is_null() && !period.is_null() {
                                    session = copy;
                                    window = colon;
                                    window_only = 1;
                                    pane = period;
                                    pane_only = 1;
                                } else if !colon.is_null() && period.is_null() {
                                    session = copy;
                                    window = colon;
                                    window_only = 1;
                                } else if colon.is_null() && !period.is_null() {
                                    window = copy;
                                    pane = period;
                                    pane_only = 1;
                                } else if *copy == b'$' as _ {
                                    session = copy;
                                } else if *copy == b'@' as _ {
                                    window = copy;
                                } else if *copy == b'%' as _ {
                                    pane = copy;
                                } else {
                                    match type_ {
                                        cmd_find_type::CMD_FIND_SESSION => session = copy,
                                        cmd_find_type::CMD_FIND_WINDOW => window = copy,
                                        cmd_find_type::CMD_FIND_PANE => pane = copy,
                                    }
                                }

                                if !session.is_null() && *session == b'=' as _ {
                                    session = session.add(1);
                                    (*fs).flags |= CMD_FIND_EXACT_SESSION;
                                }
                                if !window.is_null() && *window == b'=' as _ {
                                    window = window.add(1);
                                    (*fs).flags |= CMD_FIND_EXACT_WINDOW;
                                }

                                if !session.is_null() && *session == b'\0' as _ {
                                    session = null_mut();
                                }
                                if !window.is_null() && *window == b'\0' as _ {
                                    window = null_mut();
                                }
                                if !pane.is_null() && *pane == b'\0' as _ {
                                    pane = null_mut();
                                }

                                if !session.is_null() {
                                    session = cmd_find_map_table(
                                        &raw const CMD_FIND_SESSION_TABLE as *const _,
                                        session,
                                    );
                                }
                                if !window.is_null() {
                                    window = cmd_find_map_table(
                                        &raw const CMD_FIND_WINDOW_TABLE as *const _,
                                        window,
                                    );
                                }
                                if !pane.is_null() {
                                    pane = cmd_find_map_table(
                                        &raw const CMD_FIND_PANE_TABLE as *const _,
                                        pane,
                                    );
                                }

                                if !session.is_null() || !window.is_null() || !pane.is_null() {
                                    log_debug!(
                                        "{}: target {} is {}{}{}{}{}{}",
                                        __func__,
                                        _s(target),
                                        if session.is_null() { "" } else { "session " },
                                        _s(if session.is_null() { c!("") } else { session }),
                                        if window.is_null() { "" } else { "window " },
                                        _s(if window.is_null() { c!("") } else { window }),
                                        if pane.is_null() { "" } else { "pane " },
                                        _s(if pane.is_null() { c!("") } else { pane }),
                                    );
                                }

                                if !pane.is_null() && (flags & CMD_FIND_WINDOW_INDEX != 0) {
                                    if !flags & CMD_FIND_QUIET != 0 {
                                        cmdq_error!(item, "can't specify pane here");
                                    }
                                    break 'error;
                                }

                                if !session.is_null() {
                                    if cmd_find_get_session(fs, session) != 0 {
                                        break 'no_session;
                                    }

                                    if window.is_null() && pane.is_null() {
                                        (*fs).wl = (*(*fs).s).curw;
                                        (*fs).idx = -1;
                                        (*fs).w = (*(*fs).wl).window;
                                        (*fs).wp = (*(*fs).w).active;
                                        break 'found;
                                    }

                                    if !window.is_null() && pane.is_null() {
                                        if cmd_find_get_window_with_session(fs, window) != 0 {
                                            break 'no_window;
                                        }
                                        if !(*fs).wl.is_null() {
                                            (*fs).wp = (*(*(*fs).wl).window).active;
                                        }
                                        break 'found;
                                    }

                                    if window.is_null() && !pane.is_null() {
                                        if cmd_find_get_pane_with_session(fs, pane) != 0 {
                                            break 'no_pane;
                                        }
                                        break 'found;
                                    }

                                    if cmd_find_get_window_with_session(fs, window) != 0 {
                                        break 'no_window;
                                    }
                                    if cmd_find_get_pane_with_window(fs, pane) != 0 {
                                        break 'no_pane;
                                    }
                                    break 'found;
                                }

                                if !window.is_null() && !pane.is_null() {
                                    if cmd_find_get_window(fs, window, window_only) != 0 {
                                        break 'no_window;
                                    }
                                    if cmd_find_get_pane_with_window(fs, pane) != 0 {
                                        break 'no_pane;
                                    }
                                    break 'found;
                                }

                                if !window.is_null() && pane.is_null() {
                                    if cmd_find_get_window(fs, window, window_only) != 0 {
                                        break 'no_window;
                                    }
                                    if !(*fs).wl.is_null() {
                                        (*fs).wp = (*(*(*fs).wl).window).active;
                                    }
                                    break 'found;
                                }

                                if window.is_null() && !pane.is_null() {
                                    if cmd_find_get_pane(fs, pane, pane_only) != 0 {
                                        break 'no_pane;
                                    }
                                    break 'found;
                                }

                                //
                            }
                            // current:
                            cmd_find_copy_state(fs, (*fs).current);
                            if flags & CMD_FIND_WINDOW_INDEX != 0 {
                                (*fs).idx = -1;
                            }
                            break 'found;
                        }
                        // found:
                        (*fs).current = null_mut();
                        cmd_find_log_state(c!("cmd_find_target"), fs);

                        free_(copy);
                        return 0;
                    }
                    // no_session:
                    if !flags & CMD_FIND_QUIET != 0 {
                        cmdq_error!(item, "can't find session: {}", _s(session));
                    }
                    break 'error;
                }
                // no_window:
                if !flags & CMD_FIND_QUIET != 0 {
                    cmdq_error!(item, "can't find window: {}", _s(window));
                }
                break 'error;
            }
            // no_pane:
            if !flags & CMD_FIND_QUIET != 0 {
                cmdq_error!(item, "can't find pane: {}", _s(pane));
            }
            break 'error;
        }

        // error:
        (*fs).current = null_mut();
        log_debug!("{}: error", __func__);

        free_(copy);
        if flags & CMD_FIND_CANFAIL != 0 {
            return 0;
        }
        -1
    }
}

pub unsafe fn cmd_find_current_client(item: *mut cmdq_item, quiet: i32) -> *mut client {
    let __func__ = "cmd_find_current_client";
    unsafe {
        let mut c: *mut client = null_mut();
        let mut found: *mut client = null_mut();
        let mut s = null_mut();
        let mut wp = null_mut();
        let mut fs: cmd_find_state = zeroed();

        if !item.is_null() {
            c = cmdq_get_client(item);
        }
        if !c.is_null() && !(*c).session.is_null() {
            return c;
        }

        found = null_mut();
        if !c.is_null()
            && ({
                wp = cmd_find_inside_pane(c);
                !wp.is_null()
            })
        {
            cmd_find_clear_state(&raw mut fs, CMD_FIND_QUIET);
            fs.w = (*wp).window;
            if cmd_find_best_session_with_window(&raw mut fs) == 0 {
                found = cmd_find_best_client(fs.s);
            }
        } else {
            s = cmd_find_best_session(null_mut(), 0, CMD_FIND_QUIET);
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

pub unsafe fn cmd_find_client(item: *mut cmdq_item, target: *const u8, quiet: i32) -> *mut client {
    let __func__ = "cmd_find_client";
    unsafe {
        // struct client *c;
        // char *copy;
        // size_t size;

        /* A NULL argument means the current client. */
        if target.is_null() {
            return cmd_find_current_client(item, quiet);
        }
        let copy = xstrdup(target).as_ptr();

        /* Trim a single trailing colon if any. */
        let size = strlen(copy);
        if size != 0 && *copy.add(size - 1) == b':' as _ {
            *copy.add(size - 1) = b'\0' as _;
        }

        let mut c = null_mut();
        /* Check name and path of each client. */
        for c_ in tailq_foreach(&raw mut CLIENTS) {
            c = c_.as_ptr();
            if (*c).session.is_null() {
                continue;
            }
            if strcmp(copy, (*c).name) == 0 {
                break;
            }

            if *(*c).ttyname == b'\0' as _ {
                continue;
            }
            if strcmp(copy, (*c).ttyname) == 0 {
                break;
            }
            if libc::strncmp((*c).ttyname, _PATH_DEV, SIZEOF_PATH_DEV - 1) != 0 {
                continue;
            }
            if strcmp(copy, (*c).ttyname.add(SIZEOF_PATH_DEV - 1)) == 0 {
                break;
            }

            continue;
        }

        if c.is_null() && quiet == 0 {
            cmdq_error!(item, "can't find client: {}", _s(copy));
        }

        free_(copy);
        log_debug!("{}: target {}, return {:p}", __func__, _s(target), c);
        c
    }
}

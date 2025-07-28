// Copyright (c) 2012 Nicholas Marriott <nicholas.marriott@gmail.com>
// Copyright (c) 2012 George Nachman <tmux@georgester.com>
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

macro_rules! CONTROL_SHOULD_NOTIFY_CLIENT {
    ($c:expr) => {
        !$c.is_null() && (*$c).flags.intersects(client_flag::CONTROL)
    };
}

pub unsafe fn control_notify_pane_mode_changed(pane: c_int) {
    unsafe {
        for c in tailq_foreach(&raw mut CLIENTS).map(NonNull::as_ptr) {
            {
                if !CONTROL_SHOULD_NOTIFY_CLIENT!(c) {
                    continue;
                }

                control_write!(c, "%pane-mode-changed %{}", pane);
            }
        }
    }
}

pub unsafe fn control_notify_window_layout_changed(w: *mut window) {
    let template = c!(
        "%layout-change #{window_id} #{window_layout} #{window_visible_layout} #{window_raw_flags}"
    );

    unsafe {
        for c in tailq_foreach(&raw mut CLIENTS).map(NonNull::as_ptr) {
            {
                if !CONTROL_SHOULD_NOTIFY_CLIENT!(c) || (*c).session.is_null() {
                    continue;
                }
                let s = (*c).session;

                if winlink_find_by_window_id(&raw mut (*s).windows, (*w).id).is_null() {
                    continue;
                }

                // When the last pane in a window is closed it won't have a
                // layout root and we don't need to inform the client about the
                // layout change because the whole window will go away soon.
                if (*w).layout_root.is_null() {
                    continue;
                }

                if let Some(wl) = winlink_find_by_window(&raw mut (*s).windows, w) {
                    let cp =
                        format_single(null_mut(), template, c, null_mut(), wl.as_ptr(), null_mut());
                    control_write!(c, "{}", _s(cp));
                    free_(cp);
                }
            }
        }
    }
}

pub unsafe fn control_notify_window_pane_changed(w: *mut window) {
    unsafe {
        for c in tailq_foreach(&raw mut CLIENTS).map(NonNull::as_ptr) {
            {
                if !CONTROL_SHOULD_NOTIFY_CLIENT!(c) {
                    continue;
                }

                control_write!(
                    c,
                    "%window-pane-changed @{} %{}",
                    (*w).id,
                    (*(*w).active).id,
                );
            }
        }
    }
}

pub unsafe fn control_notify_window_unlinked(_s: *mut session, w: *mut window) {
    unsafe {
        for c in tailq_foreach(&raw mut CLIENTS).map(NonNull::as_ptr) {
            {
                if !CONTROL_SHOULD_NOTIFY_CLIENT!(c) || (*c).session.is_null() {
                    continue;
                }
                let cs = (*c).session;

                if !winlink_find_by_window_id(&raw mut (*cs).windows, (*w).id).is_null() {
                    control_write!(c, "%window-close @{}", (*w).id);
                } else {
                    control_write!(c, "%unlinked-window-close @{}", (*w).id);
                }
            }
        }
    }
}

pub unsafe fn control_notify_window_linked(_s: *mut session, w: *mut window) {
    unsafe {
        for c in tailq_foreach(&raw mut CLIENTS).map(NonNull::as_ptr) {
            {
                if !CONTROL_SHOULD_NOTIFY_CLIENT!(c) || (*c).session.is_null() {
                    continue;
                }
                let cs = (*c).session;

                if !winlink_find_by_window_id(&raw mut (*cs).windows, (*w).id).is_null() {
                    control_write!(c, "%window-add @{}", (*w).id);
                } else {
                    control_write!(c, "%unlinked-window-add @{}", (*w).id);
                }
            }
        }
    }
}

pub unsafe fn control_notify_window_renamed(w: *mut window) {
    unsafe {
        for c in tailq_foreach(&raw mut CLIENTS).map(NonNull::as_ptr) {
            {
                if !CONTROL_SHOULD_NOTIFY_CLIENT!(c) || (*c).session.is_null() {
                    continue;
                }
                let cs = (*c).session;

                if !winlink_find_by_window_id(&raw mut (*cs).windows, (*w).id).is_null() {
                    control_write!(c, "%window-renamed @{} {}", (*w).id, _s((*w).name));
                } else {
                    control_write!(c, "%unlinked-window-renamed @{} {}", (*w).id, _s((*w).name),);
                }
            }
        }
    }
}

pub unsafe fn control_notify_client_session_changed(cc: *mut client) {
    unsafe {
        if (*cc).session.is_null() {
            return;
        }
        let s = (*cc).session;

        for c in tailq_foreach(&raw mut CLIENTS).map(NonNull::as_ptr) {
            {
                if !CONTROL_SHOULD_NOTIFY_CLIENT!(c) || (*c).session.is_null() {
                    continue;
                }

                if cc == c {
                    control_write!(c, "%session-changed ${} {}", (*s).id, _s((*s).name));
                } else {
                    control_write!(
                        c,
                        "%client-session-changed {} ${} {}",
                        _s((*cc).name),
                        (*s).id,
                        _s((*s).name),
                    );
                }
            }
        }
    }
}

pub unsafe fn control_notify_client_detached(cc: *mut client) {
    unsafe {
        for c in tailq_foreach(&raw mut CLIENTS).map(NonNull::as_ptr) {
            {
                if CONTROL_SHOULD_NOTIFY_CLIENT!(c) {
                    control_write!(c, "%client-detached {}", _s((*cc).name));
                }
            }
        }
    }
}

pub unsafe fn control_notify_session_renamed(s: *mut session) {
    unsafe {
        for c in tailq_foreach(&raw mut CLIENTS).map(NonNull::as_ptr) {
            {
                if !CONTROL_SHOULD_NOTIFY_CLIENT!(c) {
                    continue;
                }

                control_write!(c, "%session-renamed ${} {}", (*s).id, _s((*s).name));
            }
        }
    }
}

pub unsafe fn control_notify_session_created(_: *mut session) {
    unsafe {
        for c in tailq_foreach(&raw mut CLIENTS).map(NonNull::as_ptr) {
            {
                if !CONTROL_SHOULD_NOTIFY_CLIENT!(c) {
                    continue;
                }

                control_write!(c, "%sessions-changed");
            }
        }
    }
}

pub unsafe fn control_notify_session_closed(_: *mut session) {
    unsafe {
        for c in tailq_foreach(&raw mut CLIENTS).map(NonNull::as_ptr) {
            {
                if !CONTROL_SHOULD_NOTIFY_CLIENT!(c) {
                    continue;
                }

                control_write!(c, "%sessions-changed");
            }
        }
    }
}

pub unsafe fn control_notify_session_window_changed(s: *mut session) {
    unsafe {
        for c in tailq_foreach(&raw mut CLIENTS).map(NonNull::as_ptr) {
            {
                if !CONTROL_SHOULD_NOTIFY_CLIENT!(c) {
                    continue;
                }

                control_write!(
                    c,
                    "%session-window-changed ${} @{}",
                    (*s).id,
                    (*(*(*s).curw).window).id,
                );
            }
        }
    }
}

pub unsafe fn control_notify_paste_buffer_changed(name: *const u8) {
    unsafe {
        for c in tailq_foreach(&raw mut CLIENTS).map(NonNull::as_ptr) {
            {
                if !CONTROL_SHOULD_NOTIFY_CLIENT!(c) {
                    continue;
                }

                control_write!(c, "%paste-buffer-changed {}", _s(name));
            }
        }
    }
}

pub unsafe fn control_notify_paste_buffer_deleted(name: *const u8) {
    unsafe {
        for c in tailq_foreach(&raw mut CLIENTS).map(NonNull::as_ptr) {
            {
                if !CONTROL_SHOULD_NOTIFY_CLIENT!(c) {
                    continue;
                }

                control_write!(c, "%paste-buffer-deleted {}", _s(name));
            }
        }
    }
}

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
use crate::event_::{event_add, event_initialized};
use crate::libc::{gettimeofday, memcpy, strchr, strcmp, strcspn, strlen, strncmp};
use crate::*;
use crate::options_::*;

pub unsafe extern "C-unwind" fn name_time_callback(
    _fd: c_int,
    _events: c_short,
    w: NonNull<window>,
) {
    unsafe {
        log_debug!("@{} timer expired", (*w.as_ptr()).id);
    }
}

pub unsafe fn name_time_expired(w: *mut window, tv: *mut timeval) -> c_int {
    unsafe {
        let mut offset: MaybeUninit<timeval> = MaybeUninit::<timeval>::uninit();

        timersub(tv, &raw mut (*w).name_time, offset.as_mut_ptr());
        let offset = offset.assume_init_ref();

        if offset.tv_sec != 0 || offset.tv_usec > NAME_INTERVAL {
            0
        } else {
            (NAME_INTERVAL - offset.tv_usec) as c_int
        }
    }
}

pub unsafe fn check_window_name(w: *mut window) {
    unsafe {
        let mut tv: timeval = zeroed();
        let mut next: timeval = zeroed();

        if (*w).active.is_null() {
            return;
        }

        if options_get_number_((*w).options, "automatic-rename") == 0 {
            return;
        }

        if !(*(*w).active)
            .flags
            .intersects(window_pane_flags::PANE_CHANGED)
        {
            // log_debug!("@{} pane not changed", (*w).id);
            return;
        }
        log_debug!("@{} pane changed", (*w).id);

        gettimeofday(&raw mut tv, null_mut());
        let left = name_time_expired(w, &raw mut tv);
        if left != 0 {
            if event_initialized(&raw mut (*w).name_event) == 0 {
                evtimer_set(
                    &raw mut (*w).name_event,
                    name_time_callback,
                    NonNull::new_unchecked(w),
                );
            }
            if evtimer_pending(&raw mut (*w).name_event, null_mut()) == 0 {
                log_debug!("@{} timer queued ({})", (*w).id, left);
                timerclear(&raw mut next);
                next.tv_usec = left as libc::suseconds_t;
                event_add(&raw mut (*w).name_event, &raw const next);
            } else {
                log_debug!("@{} timer already queued ({})", (*w).id, left);
            }
            return;
        }
        memcpy(
            &raw mut (*w).name_time as _,
            &raw const tv as _,
            size_of::<timeval>(),
        );
        if event_initialized(&raw mut (*w).name_event) != 0 {
            evtimer_del(&raw mut (*w).name_event);
        }

        (*(*w).active).flags &= !window_pane_flags::PANE_CHANGED;

        let name = format_window_name(w);
        if strcmp(name, (*w).name) != 0 {
            log_debug!("@{} name {} (was {})", (*w).id, _s(name), _s((*w).name));
            window_set_name(w, name);
            server_redraw_window_borders(w);
            server_status_window(w);
        } else {
            log_debug!("@{} not changed (still {})", (*w).id, _s((*w).name));
        }

        free(name as _);
    }
}

pub unsafe fn default_window_name(w: *mut window) -> String {
    unsafe {
        if (*w).active.is_null() {
            return String::new();
        }

        let cmd =
            CString::new(cmd_stringify_argv((*(*w).active).argc, (*(*w).active).argv)).unwrap();
        if !cmd.is_empty() {
            parse_window_name(cmd.as_ptr().cast())
        } else {
            parse_window_name((*(*w).active).shell)
        }
    }
}

unsafe fn format_window_name(w: *mut window) -> *const u8 {
    unsafe {
        let ft = format_create(
            null_mut(),
            null_mut(),
            (FORMAT_WINDOW | (*w).id) as i32,
            format_flags::empty(),
        );
        format_defaults_window(ft, w);
        format_defaults_pane(ft, (*w).active);

        let fmt = options_get_string_((*w).options, "automatic-rename-format");
        let name = format_expand(ft, fmt);

        format_free(ft);
        name
    }
}

pub unsafe fn parse_window_name(in_: *const u8) -> String {
    unsafe {
        let sizeof_exec: usize = 6; // sizeof "exec "
        let copy: *mut u8 = xstrdup(in_).cast().as_ptr();
        let mut name = copy;
        if *name == b'"' {
            name = name.wrapping_add(1);
        }
        *name.add(strcspn(name, c!("\""))) = b'\0';

        if strncmp(name, c!("exec "), sizeof_exec - 1) == 0 {
            name = name.wrapping_add(sizeof_exec - 1);
        }

        while *name == b' ' || *name == b'-' {
            name = name.wrapping_add(1);
        }

        let mut ptr = strchr(name, b' ' as _);
        if !ptr.is_null() {
            *ptr = b'\0' as _;
        }

        if *name != b'\0' {
            ptr = name.add(strlen(name) - 1);
            while ptr > name
                && !(*ptr as u8).is_ascii_alphanumeric()
                && !(*ptr as u8).is_ascii_punctuation()
            {
                *ptr = b'\0';
                ptr = ptr.wrapping_sub(1);
            }
        }

        let tmp = if *name == b'/' {
            basename(cstr_to_str(name)).to_string()
        } else {
            cstr_to_str(name).to_string()
        };
        free(copy as _);
        tmp
    }
}

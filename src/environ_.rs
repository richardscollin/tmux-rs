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

use crate::compat::{
    RB_GENERATE,
    tree::{rb_find, rb_foreach, rb_init, rb_insert, rb_min, rb_next, rb_remove},
};
use crate::xmalloc::xcalloc_;

pub type environ = rb_head<environ_entry>;
RB_GENERATE!(environ, environ_entry, entry, discr_entry, environ_cmp);

pub unsafe fn environ_cmp(
    envent1: *const environ_entry,
    envent2: *const environ_entry,
) -> std::cmp::Ordering {
    unsafe {
        i32_to_ordering(libc::strcmp(
            transmute_ptr((*envent1).name),
            transmute_ptr((*envent2).name),
        ))
    }
}

pub fn environ_create() -> NonNull<environ> {
    unsafe {
        let env = xcalloc1::<environ>();
        rb_init(env);
        NonNull::new_unchecked(env)
    }
}

pub unsafe fn environ_free(env: *mut environ) {
    unsafe {
        for envent in rb_foreach(env).map(NonNull::as_ptr) {
            rb_remove(env, envent);
            free_(transmute_ptr((*envent).name));
            free_(transmute_ptr((*envent).value));
            free_(envent);
        }
        free_(env);
    }
}

pub unsafe fn environ_first(env: *mut environ) -> *mut environ_entry {
    unsafe { rb_min(env) }
}

pub unsafe fn environ_next(envent: *mut environ_entry) -> *mut environ_entry {
    unsafe { rb_next(envent) }
}

pub unsafe fn environ_copy(srcenv: *mut environ, dstenv: *mut environ) {
    unsafe {
        for envent in rb_foreach(srcenv).map(NonNull::as_ptr) {
            if let Some(value) = (*envent).value {
                environ_set!(
                    dstenv,
                    (*envent).name.unwrap().as_ptr(),
                    (*envent).flags,
                    "{}",
                    _s(value.as_ptr()),
                );
            } else {
                environ_clear(dstenv, transmute_ptr((*envent).name));
            }
        }
    }
}

pub unsafe fn environ_find(
    env: *mut environ,
    name: *const c_char,
) -> *mut environ_entry {
    let mut envent: MaybeUninit<environ_entry> = MaybeUninit::uninit();
    let envent = envent.as_mut_ptr();

    unsafe {
        (*envent).name = NonNull::new(name.cast_mut());
        // std::ptr::write(&raw mut (*envent).name, name);
    }

    unsafe { rb_find(env, envent) }
}

macro_rules! environ_set {
   ($env:expr, $name:expr, $flags:expr, $fmt:literal $(, $args:expr)* $(,)?) => {
        crate::environ_::environ_set_($env, $name, $flags, format_args!($fmt $(, $args)*))
    };
}
pub(crate) use environ_set;
pub unsafe fn environ_set_(
    env: *mut environ,
    name: *const c_char,
    flags: c_int,
    args: std::fmt::Arguments,
) {
    unsafe {
        let mut envent = environ_find(env, name);
        let mut s = args.to_string();
        s.push('\0');
        let s = NonNull::new(s.leak().as_mut_ptr().cast());

        if !envent.is_null() {
            (*envent).flags = flags;
            free_(transmute_ptr((*envent).value));
            (*envent).value = s;
        } else {
            envent = xmalloc_::<environ_entry>().as_ptr();
            (*envent).name = Some(xstrdup(name).cast());
            (*envent).flags = flags;
            (*envent).value = s;
            rb_insert(env, envent);
        }
    }
}

pub unsafe fn environ_clear(env: *mut environ, name: *const c_char) {
    unsafe {
        let mut envent = environ_find(env, name);
        if !envent.is_null() {
            free_(transmute_ptr((*envent).value));
            (*envent).value = None;
        } else {
            envent = xmalloc_::<environ_entry>().as_ptr();
            (*envent).name = Some(xstrdup(name).cast());
            (*envent).flags = 0;
            (*envent).value = None;
            rb_insert(env, envent);
        }
    }
}

pub unsafe fn environ_put(env: *mut environ, var: *const c_char, flags: c_int) {
    unsafe {
        let mut value = libc::strchr(var, b'=' as c_int);
        if value.is_null() {
            return;
        }
        value = value.add(1);

        let name: *mut c_char = xstrdup(var).cast().as_ptr();
        *name.add(libc::strcspn(name, c"=".as_ptr())) = b'\0' as c_char;

        environ_set!(env, name, flags, "{}", _s(value));
        free_(name);
    }
}

pub unsafe fn environ_unset(env: *mut environ, name: *const c_char) {
    unsafe {
        let envent = environ_find(env, name);
        if envent.is_null() {
            return;
        }
        rb_remove(env, envent);
        free_(transmute_ptr((*envent).name));
        free_(transmute_ptr((*envent).value));
        free_(envent);
    }
}

pub unsafe fn environ_update(oo: *mut options, src: *mut environ, dst: *mut environ) {
    unsafe {
        let mut found: i32 = 0;

        let o = options_get(oo, c"update-environment".as_ptr());
        if o.is_null() {
            return;
        }
        let mut a = options_array_first(o);
        while !a.is_null() {
            let ov = options_array_item_value(a);
            found = 0;
            for envent in rb_foreach(src).map(NonNull::as_ptr) {
                if libc::fnmatch((*ov).string, transmute_ptr((*envent).name), 0) == 0 {
                    environ_set!(
                        dst,
                        transmute_ptr((*envent).name),
                        0,
                        "{}",
                        _s(transmute_ptr((*envent).value)),
                    );
                    found = 1;
                }
            }
            if found == 0 {
                environ_clear(dst, (*ov).string);
            }
            a = options_array_next(a);
        }
    }
}

pub unsafe fn environ_push(env: *mut environ) {
    unsafe {
        let mut envent: *mut environ_entry;

        environ = xcalloc_::<*mut c_char>(1).as_ptr();
        for envent in rb_foreach(env).map(NonNull::as_ptr) {
            if (*envent).value.is_some()
                && *(*envent).name.unwrap().as_ptr() != b'\0' as c_char
                && !(*envent).flags & ENVIRON_HIDDEN != 0
            {
                libc::setenv(
                    transmute_ptr((*envent).name),
                    transmute_ptr((*envent).value),
                    1,
                );
            }
        }
    }
}

macro_rules! environ_log {
   ($env:expr, $fmt:literal $(, $args:expr)* $(,)?) => {
        crate::environ_::environ_log_($env, format_args!($fmt $(, $args)*))
    };
}
pub(crate) use environ_log;

pub unsafe fn environ_log_(env: *mut environ, args: std::fmt::Arguments) {
    unsafe {
        let prefix = args.to_string();

        for envent in rb_foreach(env).map(NonNull::as_ptr) {
            if (*envent).value.is_some() && *(*envent).name.unwrap().as_ptr() != b'\0' as c_char {
                log_debug!(
                    "{}{}={}",
                    prefix,
                    _s(transmute_ptr((*envent).name)),
                    _s(transmute_ptr((*envent).value))
                );
            }
        }
    }
}

pub unsafe fn environ_for_session(s: *mut session, no_term: c_int) -> *mut environ {
    let env: *mut environ = environ_create().as_ptr();

    unsafe {
        environ_copy(global_environ, env);
        if !s.is_null() {
            environ_copy((*s).environ, env);
        }

        if no_term == 0 {
            let value = options_get_string_(global_options, c"default-terminal");
            environ_set!(env, c"TERM".as_ptr(), 0, "{}", _s(value));
            environ_set!(env, c"TERM_PROGRAM".as_ptr(), 0, "{}", "tmux");
            environ_set!(env, c"TERM_PROGRAM_VERSION".as_ptr(), 0, "{}", getversion());
        }

        #[cfg(feature = "systemd")]
        {
            environ_clear(env, c"LISTEN_PID".as_ptr());
            environ_clear(env, c"LISTEN_FDS".as_ptr());
            environ_clear(env, c"LISTEN_FDNAMES".as_ptr());
        }

        let idx = if !s.is_null() { (*s).id as i32 } else { -1 };

        environ_set!(
            env,
            c"TMUX".as_ptr(),
            0,
            "{},{},{}",
            _s(socket_path),
            std::process::id(),
            idx,
        );

        env
    }
}

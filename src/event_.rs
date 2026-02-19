#![allow(clippy::transmute_ptr_to_ptr)]
#![allow(non_upper_case_globals)]
#![allow(unused)]

use std::{
    ffi::{c_int, c_short, c_void},
    option::Option,
    ptr::NonNull,
};

use ::libc::timeval;

macro_rules! evbuffer_add_printf {
   ($buf:expr, $fmt:literal $(, $args:expr)* $(,)?) => {
        crate::event_::evbuffer_add_vprintf($buf, format_args!($fmt $(, $args)*))
    };
}
pub(crate) use evbuffer_add_printf;
#[expect(clippy::disallowed_methods)]
pub unsafe fn evbuffer_add_vprintf(buf: *mut evbuffer, args: std::fmt::Arguments) -> i32 {
    let s = args.to_string(); // TODO this is doing unecessary allocating and freeing
    unsafe { evbuffer_add(buf, s.as_ptr().cast(), s.len()) }
}

pub const EVLOOP_NO_EXIT_ON_EMPTY: i32 = 0x04;
pub const EVLOOP_NONBLOCK: i32 = 0x02;
pub const EVLOOP_ONCE: i32 = 0x01;

pub const EV_CLOSED: i16 = 0x80;
pub const EV_ET: i16 = 0x20;
pub const EV_FINALIZE: i16 = 0x40;
pub const EV_PERSIST: i16 = 0x10;
pub const EV_READ: i16 = 0x02;
pub const EV_SIGNAL: i16 = 0x08;
pub const EV_TIMEOUT: i16 = 0x01;
pub const EV_WRITE: i16 = 0x04;

// Shared opaque struct markers
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct evbuffer {
    _unused: [u8; 0],
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct event_base {
    _unused: [u8; 0],
}

// Shared types
#[repr(u32)]
pub(crate) enum evbuffer_eol_style {
    EVBUFFER_EOL_ANY = 0,
    EVBUFFER_EOL_CRLF = 1,
    EVBUFFER_EOL_CRLF_STRICT = 2,
    EVBUFFER_EOL_LF = 3,
    EVBUFFER_EOL_NUL = 4,
}

pub type bufferevent_data_cb =
    Option<unsafe extern "C-unwind" fn(bev: *mut bufferevent, ctx: *mut c_void)>;
pub type bufferevent_event_cb =
    Option<unsafe extern "C-unwind" fn(bev: *mut bufferevent, what: c_short, ctx: *mut c_void)>;
pub type event_log_cb = Option<unsafe extern "C-unwind" fn(severity: c_int, msg: *const u8)>;

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct event_watermark {
    pub low: usize,
    pub high: usize,
}

// --- Backend selection ---

#[cfg(not(feature = "event-tokio"))]
mod event_libevent;
#[cfg(not(feature = "event-tokio"))]
pub use event_libevent::*;

#[cfg(feature = "event-tokio")]
mod event_tokio;
#[cfg(feature = "event-tokio")]
pub use event_tokio::*;

// /usr/include/event2/event.h

// #define evtimer_set(ev, cb, arg)	event_set((ev), -1, 0, (cb), (arg))
pub unsafe fn evtimer_set<T>(
    ev: *mut event,
    cb: unsafe extern "C-unwind" fn(_: c_int, _: c_short, _: NonNull<T>),
    arg: NonNull<T>,
) {
    unsafe {
        event_set(
            ev,
            -1,
            0,
            std::mem::transmute::<
                Option<unsafe extern "C-unwind" fn(_: c_int, _: c_short, _: NonNull<T>)>,
                Option<unsafe extern "C-unwind" fn(_: c_int, _: c_short, _: *mut c_void)>,
            >(Some(cb)),
            arg.as_ptr().cast(),
        );
    }
}

pub unsafe fn evtimer_set_no_args(
    ev: *mut event,
    cb: unsafe extern "C-unwind" fn(_: c_int, _: c_short, _: *mut c_void),
) {
    unsafe { event_set(ev, -1, 0, Some(cb), std::ptr::null_mut()) }
}

// #define evtimer_add(ev, tv)		event_add((ev), (tv))
pub unsafe fn evtimer_add(ev: *mut event, tv: *const timeval) -> c_int {
    unsafe { event_add(ev, tv) }
}

pub unsafe fn evtimer_initialized(ev: *mut event) -> bool {
    unsafe { event_initialized(ev) != 0 }
}

// #define evtimer_del(ev)			event_del(ev)
pub unsafe fn evtimer_del(ev: *mut event) -> c_int {
    unsafe { event_del(ev) }
}

// #define evtimer_pending(ev, tv)		event_pending((ev), EV_TIMEOUT, (tv))
pub unsafe fn evtimer_pending(ev: *const event, tv: *mut libc::timeval) -> c_int {
    unsafe { event_pending(ev, EV_TIMEOUT, tv) }
}

// #define signal_add(ev, tv)		event_add((ev), (tv))
#[inline]
pub unsafe fn signal_add(ev: *mut event, tv: *const timeval) -> i32 {
    unsafe { event_add(ev, tv) }
}

// #define signal_set(ev, x, cb, arg)				 event_set((ev), (x), EV_SIGNAL|EV_PERSIST, (cb), (arg))
#[inline]
pub unsafe fn signal_set(
    ev: *mut event,
    x: i32,
    cb: Option<unsafe extern "C-unwind" fn(c_int, c_short, *mut c_void)>,
    arg: *mut c_void,
) {
    unsafe { event_set(ev, x, EV_SIGNAL | EV_PERSIST, cb, arg) }
}

// #define signal_del(ev)			event_del(ev)
// #define signal_pending(ev, tv)		event_pending((ev), EV_SIGNAL, (tv))
// #define signal_initialized(ev)		event_initialized(ev)

#[expect(non_snake_case)]
#[inline]
pub unsafe fn EVBUFFER_LENGTH(x: *mut evbuffer) -> usize {
    unsafe { evbuffer_get_length(x) }
}

#[expect(non_snake_case)]
#[inline]
pub unsafe fn EVBUFFER_DATA(x: *mut evbuffer) -> *mut u8 {
    unsafe { evbuffer_pullup(x, -1) }
}

#[expect(non_snake_case)]
#[inline]
pub unsafe fn EVBUFFER_OUTPUT(x: *mut bufferevent) -> *mut evbuffer {
    unsafe { bufferevent_get_output(x) }
}

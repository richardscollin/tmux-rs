#![allow(non_upper_case_globals)]
use ::core::{
    ffi::{c_int, c_short, c_void},
    option::Option,
};
use ::libc::timeval;

macro_rules! evbuffer_add_printf {
   ($buf:expr, $fmt:literal $(, $args:expr)* $(,)?) => {
        crate::event_::evbuffer_add_vprintf($buf, format_args!($fmt $(, $args)*))
    };
}
pub(crate) use evbuffer_add_printf;
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

// /usr/include/event2/event.h

// #define evtimer_set(ev, cb, arg)	event_set((ev), -1, 0, (cb), (arg))
pub unsafe fn evtimer_set(
    ev: *mut event,
    cb: Option<unsafe extern "C-unwind" fn(_: c_int, _: c_short, _: *mut c_void)>,
    arg: *mut c_void,
) {
    unsafe {
        event_set(ev, -1, 0, cb, arg);
    }
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

#[allow(non_snake_case)]
#[inline]
pub unsafe fn EVBUFFER_LENGTH(x: *mut evbuffer) -> usize {
    unsafe { evbuffer_get_length(x) }
}

#[allow(non_snake_case)]
#[inline]
pub unsafe fn EVBUFFER_DATA(x: *mut evbuffer) -> *mut u8 {
    unsafe { evbuffer_pullup(x, -1) }
}

#[allow(non_snake_case)]
#[inline]
pub unsafe fn EVBUFFER_OUTPUT(x: *mut bufferevent) -> *mut evbuffer {
    unsafe { bufferevent_get_output(x) }
}

#[repr(C)]
pub struct __BindgenUnionField<T>(::core::marker::PhantomData<T>);
impl<T> __BindgenUnionField<T> {
    #[inline]
    pub const fn new() -> Self {
        __BindgenUnionField(::core::marker::PhantomData)
    }
    #[inline]
    pub unsafe fn as_ref(&self) -> &T {
        unsafe { ::core::mem::transmute(self) }
    }
    #[inline]
    pub unsafe fn as_mut(&mut self) -> &mut T {
        unsafe { ::core::mem::transmute(self) }
    }
}
impl<T> ::core::default::Default for __BindgenUnionField<T> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}
impl<T> ::core::clone::Clone for __BindgenUnionField<T> {
    #[inline]
    fn clone(&self) -> Self {
        *self
    }
}
impl<T> ::core::marker::Copy for __BindgenUnionField<T> {}
impl<T> ::core::fmt::Debug for __BindgenUnionField<T> {
    fn fmt(&self, fmt: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
        fmt.write_str("__BindgenUnionField")
    }
}
impl<T> ::core::hash::Hash for __BindgenUnionField<T> {
    fn hash<H: ::core::hash::Hasher>(&self, _state: &mut H) {}
}
impl<T> ::core::cmp::PartialEq for __BindgenUnionField<T> {
    fn eq(&self, _other: &__BindgenUnionField<T>) -> bool {
        true
    }
}
impl<T> ::core::cmp::Eq for __BindgenUnionField<T> {}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct evbuffer {
    _unused: [u8; 0],
}

#[repr(u32)]
pub(crate) enum evbuffer_eol_style {
    EVBUFFER_EOL_ANY = 0,
    EVBUFFER_EOL_CRLF = 1,
    EVBUFFER_EOL_CRLF_STRICT = 2,
    EVBUFFER_EOL_LF = 3,
    EVBUFFER_EOL_NUL = 4,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct event_base {
    _unused: [u8; 0],
}
pub type bufferevent_data_cb =
    Option<unsafe extern "C-unwind" fn(bev: *mut bufferevent, ctx: *mut c_void)>;
pub type bufferevent_event_cb =
    Option<unsafe extern "C-unwind" fn(bev: *mut bufferevent, what: c_short, ctx: *mut c_void)>;
#[repr(C)]
#[derive(Copy, Clone)]
pub struct event_callback {
    pub evcb_active_next: event_callback__bindgen_ty_1,
    pub evcb_flags: i16,
    pub evcb_pri: u8,
    pub evcb_closure: u8,
    pub evcb_cb_union: event_callback__bindgen_ty_2,
    pub evcb_arg: *mut c_void,
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct event_callback__bindgen_ty_1 {
    pub tqe_next: *mut event_callback,
    pub tqe_prev: *mut *mut event_callback,
}
#[repr(C)]
#[derive(Copy, Clone)]
pub union event_callback__bindgen_ty_2 {
    pub evcb_callback:
        Option<unsafe extern "C-unwind" fn(arg1: c_int, arg2: c_short, arg3: *mut c_void)>,
    pub evcb_selfcb:
        Option<unsafe extern "C-unwind" fn(arg1: *mut event_callback, arg2: *mut c_void)>,
    pub evcb_evfinalize: Option<unsafe extern "C-unwind" fn(arg1: *mut event, arg2: *mut c_void)>,
    pub evcb_cbfinalize:
        Option<unsafe extern "C-unwind" fn(arg1: *mut event_callback, arg2: *mut c_void)>,
}
#[repr(C)]
pub struct event {
    pub ev_evcallback: event_callback,
    pub ev_timeout_pos: event__bindgen_ty_1,
    pub ev_fd: c_int,
    pub ev_base: *mut event_base,
    pub ev_: event__bindgen_ty_2,
    pub ev_events: c_short,
    pub ev_res: c_short,
    pub ev_timeout: timeval,
}
#[repr(C)]
#[derive(Copy, Clone)]
pub union event__bindgen_ty_1 {
    pub ev_next_with_common_timeout: event__bindgen_ty_1__bindgen_ty_1,
    pub min_heap_idx: c_int,
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct event__bindgen_ty_1__bindgen_ty_1 {
    pub tqe_next: *mut event,
    pub tqe_prev: *mut *mut event,
}
#[repr(C)]
pub struct event__bindgen_ty_2 {
    pub ev_io: __BindgenUnionField<event__bindgen_ty_2__bindgen_ty_1>,
    pub ev_signal: __BindgenUnionField<event__bindgen_ty_2__bindgen_ty_2>,
    pub bindgen_union_field: [u64; 4usize],
}
#[repr(C)]
pub struct event__bindgen_ty_2__bindgen_ty_1 {
    pub ev_io_next: event__bindgen_ty_2__bindgen_ty_1__bindgen_ty_1,
    pub ev_timeout: timeval,
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct event__bindgen_ty_2__bindgen_ty_1__bindgen_ty_1 {
    pub le_next: *mut event,
    pub le_prev: *mut *mut event,
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct event__bindgen_ty_2__bindgen_ty_2 {
    pub ev_signal_next: event__bindgen_ty_2__bindgen_ty_2__bindgen_ty_1,
    pub ev_ncalls: c_short,
    pub ev_pncalls: *mut c_short,
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct event__bindgen_ty_2__bindgen_ty_2__bindgen_ty_1 {
    pub le_next: *mut event,
    pub le_prev: *mut *mut event,
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct event_watermark {
    pub low: usize,
    pub high: usize,
}
#[repr(C)]
pub struct bufferevent {
    pub ev_base: *mut event_base,
    pub be_ops: *mut bufferevent_ops,
    pub ev_read: event,
    pub ev_write: event,
    pub input: *mut evbuffer,
    pub output: *mut evbuffer,
    pub wm_read: event_watermark,
    pub wm_write: event_watermark,
    pub readcb: bufferevent_data_cb,
    pub writecb: bufferevent_data_cb,
    pub errorcb: bufferevent_event_cb,
    pub cbarg: *mut c_void,
    pub timeout_read: timeval,
    pub timeout_write: timeval,
    pub enabled: c_short,
}
pub type event_log_cb = Option<unsafe extern "C-unwind" fn(severity: c_int, msg: *const u8)>;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct bufferevent_ops {
    pub _address: u8,
}

#[link(name = "event_core")]
unsafe extern "C" {
    pub fn evbuffer_new() -> *mut evbuffer;
    pub fn evbuffer_free(buf: *mut evbuffer);
    pub fn evbuffer_get_length(buf: *const evbuffer) -> usize;
    pub fn evbuffer_add(buf: *mut evbuffer, data: *const c_void, datlen: usize) -> c_int;

    pub fn evbuffer_write(buffer: *mut evbuffer, fd: i32) -> i32;
    pub fn evbuffer_read(buffer: *mut evbuffer, fd: i32, howmuch: i32) -> i32;
    pub fn evbuffer_readline(buffer: *mut evbuffer) -> *mut u8;
    pub fn evbuffer_readln(
        buffer: *mut evbuffer,
        n_read_out: *mut usize,
        eol_style: evbuffer_eol_style,
    ) -> *mut u8;
    pub fn evbuffer_drain(buf: *mut evbuffer, len: usize) -> c_int;
    pub fn evbuffer_pullup(buf: *mut evbuffer, size: isize) -> *mut u8;
    pub fn bufferevent_free(bufev: *mut bufferevent);
    pub fn bufferevent_write(bufev: *mut bufferevent, data: *const c_void, size: usize) -> c_int;
    pub fn bufferevent_write_buffer(bufev: *mut bufferevent, buf: *mut evbuffer) -> c_int;
    pub fn bufferevent_get_output(bufev: *mut bufferevent) -> *mut evbuffer;
    pub fn bufferevent_enable(bufev: *mut bufferevent, event: i16) -> c_int;
    pub fn bufferevent_disable(bufev: *mut bufferevent, event: i16) -> c_int;
    pub fn bufferevent_setwatermark(
        bufev: *mut bufferevent,
        events: i16,
        lowmark: usize,
        highmark: usize,
    );
    pub fn bufferevent_new(
        fd: c_int,
        readcb: bufferevent_data_cb,
        writecb: bufferevent_data_cb,
        errorcb: bufferevent_event_cb,
        cbarg: *mut c_void,
    ) -> *mut bufferevent;
    pub fn event_init() -> *mut event_base;
    pub fn event_reinit(base: *mut event_base) -> c_int;
    pub fn event_set_log_callback(cb: event_log_cb);
    pub fn event_add(ev: *mut event, timeout: *const timeval) -> c_int;
    pub fn event_del(arg1: *mut event) -> c_int;
    pub fn event_active(ev: *mut event, res: c_int, ncalls: c_short);
    pub fn event_pending(ev: *const event, events: c_short, tv: *mut timeval) -> c_int;
    pub fn event_initialized(ev: *const event) -> c_int;
    pub fn event_get_version() -> *const u8;
    pub fn event_loop(arg1: c_int) -> c_int;
    pub fn event_once(
        arg1: c_int,
        arg2: c_short,
        arg3: Option<unsafe extern "C-unwind" fn(arg1: c_int, arg2: c_short, arg3: *mut c_void)>,
        arg4: *mut c_void,
        arg5: *const timeval,
    ) -> c_int;
    pub fn event_get_method() -> *const u8;
    pub fn event_set(
        arg1: *mut event,
        arg2: c_int,
        arg3: c_short,
        arg4: Option<unsafe extern "C-unwind" fn(arg1: c_int, arg2: c_short, arg3: *mut c_void)>,
        arg5: *mut c_void,
    );
}

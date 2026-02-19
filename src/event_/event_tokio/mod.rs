mod bufferevent_impl;
mod evbuffer_impl;
mod event_impl;

use std::ffi::{c_int, c_short, c_void};

use ::libc::timeval;

use super::{
    bufferevent_data_cb, bufferevent_event_cb, evbuffer, evbuffer_eol_style, event_base,
    event_log_cb, event_watermark,
};

/// Tokio-backend event struct.
///
/// This replaces the libevent `struct event` with a simplified layout.
/// No code outside event_.rs accesses event fields directly, so the
/// internal layout is free to differ from libevent's.
#[repr(C)]
pub struct event {
    /// Unique id assigned by event_set (0 = uninitialized).
    pub(crate) id: u64,
    /// File descriptor, or -1 for timers.
    pub(crate) ev_fd: c_int,
    /// Event flags (EV_READ, EV_WRITE, EV_SIGNAL, EV_PERSIST, EV_TIMEOUT).
    pub(crate) ev_events: c_short,
    /// Pending result flags (set by event_active).
    pub(crate) ev_res: c_short,
    /// Callback function.
    pub(crate) ev_callback:
        Option<unsafe extern "C-unwind" fn(arg1: c_int, arg2: c_short, arg3: *mut c_void)>,
    /// Callback argument.
    pub(crate) ev_arg: *mut c_void,
    /// Pointer to the event base this event is registered with.
    pub(crate) ev_base: *mut event_base,
    /// Timeout value.
    pub(crate) ev_timeout: timeval,
    /// Whether this event is currently added (has a watcher task).
    pub(crate) added: bool,
}

/// Tokio-backend bufferevent struct.
///
/// Field names for `input`, `output`, `readcb`, `writecb`, `errorcb`,
/// `cbarg`, and `enabled` are preserved since they are accessed from
/// ~10 files throughout the codebase.
#[repr(C)]
pub struct bufferevent {
    pub ev_base: *mut event_base,
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

// Re-export all public API functions
pub use bufferevent_impl::*;
pub use evbuffer_impl::*;
pub use event_impl::*;

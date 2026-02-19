use std::ffi::{c_int, c_short, c_void};

use ::libc::timeval;

use super::super::{
    EV_PERSIST, EV_READ, EV_WRITE, bufferevent_data_cb, bufferevent_event_cb, evbuffer, event_base,
    event_watermark,
};
use super::evbuffer_impl::{evbuffer_add, evbuffer_drain, evbuffer_get_length, evbuffer_new};
use super::{bufferevent, event};

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn bufferevent_new(
    fd: c_int,
    readcb: bufferevent_data_cb,
    writecb: bufferevent_data_cb,
    errorcb: bufferevent_event_cb,
    cbarg: *mut c_void,
) -> *mut bufferevent {
    let input = unsafe { evbuffer_new() };
    let output = unsafe { evbuffer_new() };

    let bev = Box::new(bufferevent {
        ev_base: std::ptr::null_mut(),
        ev_read: unsafe { std::mem::zeroed::<event>() },
        ev_write: unsafe { std::mem::zeroed::<event>() },
        input,
        output,
        wm_read: event_watermark { low: 0, high: 0 },
        wm_write: event_watermark { low: 0, high: 0 },
        readcb,
        writecb,
        errorcb,
        cbarg,
        timeout_read: timeval {
            tv_sec: 0,
            tv_usec: 0,
        },
        timeout_write: timeval {
            tv_sec: 0,
            tv_usec: 0,
        },
        enabled: 0,
    });

    let ptr = Box::into_raw(bev);

    // Set up internal read/write events with the fd.
    // Both use EV_PERSIST so they keep firing (matching libevent's bufferevent).
    // The write event is disabled when the output buffer is empty to prevent
    // busy-looping (see bufferevent_writecb).
    unsafe {
        super::event_impl::event_set(
            &raw mut (*ptr).ev_read,
            fd,
            EV_READ | EV_PERSIST,
            Some(bufferevent_readcb),
            ptr as *mut c_void,
        );
        super::event_impl::event_set(
            &raw mut (*ptr).ev_write,
            fd,
            EV_WRITE | EV_PERSIST,
            Some(bufferevent_writecb),
            ptr as *mut c_void,
        );
    }

    ptr
}

/// Internal read callback: reads data from fd into input buffer,
/// then calls the user's read callback.
unsafe extern "C-unwind" fn bufferevent_readcb(fd: c_int, _events: c_short, arg: *mut c_void) {
    let bev = arg as *mut bufferevent;
    if bev.is_null() {
        return;
    }

    // Read from fd into the input evbuffer
    let n = unsafe { super::evbuffer_impl::evbuffer_read((*bev).input, fd, 4096) };
    if n > 0 {
        if let Some(cb) = unsafe { (*bev).readcb } {
            unsafe { cb(bev, (*bev).cbarg) };
        }
    } else if n == 0
        || (n < 0 && std::io::Error::last_os_error().kind() != std::io::ErrorKind::WouldBlock)
    {
        // EOF or error
        if let Some(cb) = unsafe { (*bev).errorcb } {
            let what: c_short = EV_READ | 0x01; // BEV_EVENT_READING | BEV_EVENT_EOF (simplified)
            unsafe { cb(bev, what, (*bev).cbarg) };
        }
    }
}

/// Internal write callback: writes data from output buffer to fd,
/// then calls the user's write callback if buffer is drained.
unsafe extern "C-unwind" fn bufferevent_writecb(fd: c_int, _events: c_short, arg: *mut c_void) {
    let bev = arg as *mut bufferevent;
    if bev.is_null() {
        return;
    }

    let output = unsafe { (*bev).output };
    if unsafe { evbuffer_get_length(output) } > 0 {
        let n = unsafe { super::evbuffer_impl::evbuffer_write(output, fd) };
        if n < 0 {
            if std::io::Error::last_os_error().kind() == std::io::ErrorKind::WouldBlock {
                return;
            }
            if let Some(cb) = unsafe { (*bev).errorcb } {
                let what: c_short = EV_WRITE | 0x01;
                unsafe { cb(bev, what, (*bev).cbarg) };
            }
            return;
        }
    }

    // If output buffer is empty, stop watching writability to prevent
    // busy-looping (the fd is almost always writable). The write event
    // will be re-added when bufferevent_write() queues new data.
    if unsafe { evbuffer_get_length(output) } == 0 {
        unsafe { super::event_impl::event_del(&raw mut (*bev).ev_write) };
        if let Some(cb) = unsafe { (*bev).writecb } {
            unsafe { cb(bev, (*bev).cbarg) };
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn bufferevent_free(bufev: *mut bufferevent) {
    if bufev.is_null() {
        return;
    }
    unsafe {
        // Delete internal events
        super::event_impl::event_del(&raw mut (*bufev).ev_read);
        super::event_impl::event_del(&raw mut (*bufev).ev_write);

        // Free evbuffers
        super::evbuffer_impl::evbuffer_free((*bufev).input);
        super::evbuffer_impl::evbuffer_free((*bufev).output);

        // Free the bufferevent itself
        let _ = Box::from_raw(bufev);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn bufferevent_write(
    bufev: *mut bufferevent,
    data: *const c_void,
    size: usize,
) -> c_int {
    if bufev.is_null() {
        return -1;
    }
    let output = unsafe { (*bufev).output };
    let rc = unsafe { evbuffer_add(output, data, size) };
    if rc != 0 {
        return rc;
    }

    // If writing is enabled, ensure the write event is active
    if (unsafe { (*bufev).enabled } & EV_WRITE) != 0 {
        unsafe {
            super::event_impl::event_add(&raw mut (*bufev).ev_write, std::ptr::null());
        };
    }

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn bufferevent_write_buffer(
    bufev: *mut bufferevent,
    buf: *mut evbuffer,
) -> c_int {
    if bufev.is_null() || buf.is_null() {
        return -1;
    }
    // Move all data from buf into the output buffer
    let len = unsafe { evbuffer_get_length(buf) };
    if len == 0 {
        return 0;
    }
    let data = unsafe { super::evbuffer_impl::evbuffer_pullup(buf, -1) };
    if data.is_null() {
        return -1;
    }
    let rc = unsafe { evbuffer_add((*bufev).output, data as *const c_void, len) };
    if rc != 0 {
        return rc;
    }
    unsafe { evbuffer_drain(buf, len) };

    // If writing is enabled, ensure the write event is active
    if (unsafe { (*bufev).enabled } & EV_WRITE) != 0 {
        unsafe {
            super::event_impl::event_add(&raw mut (*bufev).ev_write, std::ptr::null());
        };
    }

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn bufferevent_get_output(bufev: *mut bufferevent) -> *mut evbuffer {
    if bufev.is_null() {
        return std::ptr::null_mut();
    }
    unsafe { (*bufev).output }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn bufferevent_enable(bufev: *mut bufferevent, events: i16) -> c_int {
    if bufev.is_null() {
        return -1;
    }
    unsafe {
        (*bufev).enabled |= events;

        if (events & EV_READ) != 0 {
            super::event_impl::event_add(&raw mut (*bufev).ev_read, std::ptr::null());
        }
        if (events & EV_WRITE) != 0 {
            super::event_impl::event_add(&raw mut (*bufev).ev_write, std::ptr::null());
        }
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn bufferevent_disable(bufev: *mut bufferevent, events: i16) -> c_int {
    if bufev.is_null() {
        return -1;
    }
    unsafe {
        (*bufev).enabled &= !events;

        if (events & EV_READ) != 0 {
            super::event_impl::event_del(&raw mut (*bufev).ev_read);
        }
        if (events & EV_WRITE) != 0 {
            super::event_impl::event_del(&raw mut (*bufev).ev_write);
        }
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn bufferevent_setwatermark(
    bufev: *mut bufferevent,
    events: i16,
    lowmark: usize,
    highmark: usize,
) {
    if bufev.is_null() {
        return;
    }
    unsafe {
        if (events & EV_READ) != 0 {
            (*bufev).wm_read.low = lowmark;
            (*bufev).wm_read.high = highmark;
        }
        if (events & EV_WRITE) != 0 {
            (*bufev).wm_write.low = lowmark;
            (*bufev).wm_write.high = highmark;
        }
    }
}

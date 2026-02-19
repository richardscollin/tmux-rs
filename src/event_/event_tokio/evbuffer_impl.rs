use std::ffi::{c_int, c_void};

use super::super::{evbuffer, evbuffer_eol_style};

/// Internal evbuffer backed by a `Vec<u8>`.
///
/// Allocated on the heap; callers hold `*mut evbuffer` which is really
/// a `*mut EvbufferInner` behind a cast.
pub(crate) struct EvbufferInner {
    data: Vec<u8>,
}

impl EvbufferInner {
    fn new() -> Self {
        Self { data: Vec::new() }
    }
}

/// Cast `*mut evbuffer` to `*mut EvbufferInner`.
///
/// # Safety
///
/// `buf` must have been created by `evbuffer_new` (i.e. it's really
/// a `Box<EvbufferInner>` pointer).
unsafe fn inner(buf: *mut evbuffer) -> &'static mut EvbufferInner {
    unsafe { &mut *(buf as *mut EvbufferInner) }
}

unsafe fn inner_const(buf: *const evbuffer) -> &'static EvbufferInner {
    unsafe { &*(buf as *const EvbufferInner) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn evbuffer_new() -> *mut evbuffer {
    let inner = Box::new(EvbufferInner::new());
    Box::into_raw(inner) as *mut evbuffer
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn evbuffer_free(buf: *mut evbuffer) {
    if !buf.is_null() {
        let _ = unsafe { Box::from_raw(buf as *mut EvbufferInner) };
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn evbuffer_get_length(buf: *const evbuffer) -> usize {
    if buf.is_null() {
        return 0;
    }
    unsafe { inner_const(buf) }.data.len()
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn evbuffer_add(
    buf: *mut evbuffer,
    data: *const c_void,
    datlen: usize,
) -> c_int {
    if buf.is_null() || (data.is_null() && datlen != 0) {
        return -1;
    }
    if datlen == 0 {
        return 0;
    }
    let slice = unsafe { std::slice::from_raw_parts(data as *const u8, datlen) };
    unsafe { inner(buf) }.data.extend_from_slice(slice);
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn evbuffer_drain(buf: *mut evbuffer, len: usize) -> c_int {
    if buf.is_null() {
        return -1;
    }
    let inner = unsafe { inner(buf) };
    let drain_len = len.min(inner.data.len());
    inner.data.drain(..drain_len);
    0
}

/// Returns a pointer to the first `size` bytes of the buffer.
/// If `size` is -1, returns all bytes. The pointer is valid until
/// the next buffer mutation.
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn evbuffer_pullup(buf: *mut evbuffer, size: isize) -> *mut u8 {
    if buf.is_null() {
        return std::ptr::null_mut();
    }
    let inner = unsafe { inner(buf) };
    if inner.data.is_empty() {
        return std::ptr::null_mut();
    }
    // size == -1 means "all data"; any other value is capped at data.len()
    inner.data.as_mut_ptr()
}

/// Synchronous write from evbuffer to a file descriptor.
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn evbuffer_write(buffer: *mut evbuffer, fd: i32) -> i32 {
    if buffer.is_null() {
        return -1;
    }
    let inner = unsafe { inner(buffer) };
    if inner.data.is_empty() {
        return 0;
    }
    let n = unsafe { libc::write(fd, inner.data.as_ptr() as *const c_void, inner.data.len()) };
    if n > 0 {
        inner.data.drain(..n as usize);
    }
    n as i32
}

/// Synchronous read from a file descriptor into evbuffer.
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn evbuffer_read(buffer: *mut evbuffer, fd: i32, howmuch: i32) -> i32 {
    if buffer.is_null() {
        return -1;
    }
    let howmuch = if howmuch <= 0 { 4096 } else { howmuch as usize };
    let inner = unsafe { inner(buffer) };
    let old_len = inner.data.len();
    inner.data.resize(old_len + howmuch, 0);
    let n = unsafe {
        libc::read(
            fd,
            inner.data[old_len..].as_mut_ptr() as *mut c_void,
            howmuch,
        )
    };
    if n >= 0 {
        inner.data.truncate(old_len + n as usize);
        n as i32
    } else {
        inner.data.truncate(old_len);
        -1
    }
}

/// Read a line terminated by `\r\n` or `\n`, returning a malloc'd C string.
/// The caller is responsible for calling `free()` on the result.
/// Returns null if no complete line is available.
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn evbuffer_readline(buffer: *mut evbuffer) -> *mut u8 {
    if buffer.is_null() {
        return std::ptr::null_mut();
    }
    let inner = unsafe { inner(buffer) };

    // Find first \n
    let pos = match memchr::memchr(b'\n', &inner.data) {
        Some(p) => p,
        None => return std::ptr::null_mut(),
    };

    // Determine line length (strip trailing \r if present)
    let line_len = if pos > 0 && inner.data[pos - 1] == b'\r' {
        pos - 1
    } else {
        pos
    };

    // Allocate with malloc (caller will free)
    let out = unsafe { libc::malloc(line_len + 1) } as *mut u8;
    if out.is_null() {
        return std::ptr::null_mut();
    }
    unsafe {
        std::ptr::copy_nonoverlapping(inner.data.as_ptr(), out, line_len);
        *out.add(line_len) = 0; // null terminate
    }

    // Drain the line + newline from the buffer
    inner.data.drain(..=pos);

    out
}

/// Read a line with a specific EOL style, returning a malloc'd string.
/// Sets `*n_read_out` to the length of the returned string (excluding NUL).
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn evbuffer_readln(
    buffer: *mut evbuffer,
    n_read_out: *mut usize,
    eol_style: evbuffer_eol_style,
) -> *mut u8 {
    if buffer.is_null() {
        return std::ptr::null_mut();
    }
    let inner = unsafe { inner(buffer) };

    // Find end-of-line based on style
    let (line_len, drain_len) = match eol_style {
        evbuffer_eol_style::EVBUFFER_EOL_LF => match memchr::memchr(b'\n', &inner.data) {
            Some(pos) => (pos, pos + 1),
            None => return std::ptr::null_mut(),
        },
        evbuffer_eol_style::EVBUFFER_EOL_CRLF_STRICT => {
            match memchr::memmem::find(&inner.data, b"\r\n") {
                Some(pos) => (pos, pos + 2),
                None => return std::ptr::null_mut(),
            }
        }
        evbuffer_eol_style::EVBUFFER_EOL_CRLF => {
            // Accept \r\n or bare \n
            match memchr::memchr(b'\n', &inner.data) {
                Some(pos) => {
                    let line_len = if pos > 0 && inner.data[pos - 1] == b'\r' {
                        pos - 1
                    } else {
                        pos
                    };
                    (line_len, pos + 1)
                }
                None => return std::ptr::null_mut(),
            }
        }
        evbuffer_eol_style::EVBUFFER_EOL_ANY => {
            // Any sequence of \r and \n
            match memchr::memchr2(b'\r', b'\n', &inner.data) {
                Some(pos) => {
                    let line_len = pos;
                    let mut end = pos;
                    while end < inner.data.len()
                        && (inner.data[end] == b'\r' || inner.data[end] == b'\n')
                    {
                        end += 1;
                    }
                    (line_len, end)
                }
                None => return std::ptr::null_mut(),
            }
        }
        evbuffer_eol_style::EVBUFFER_EOL_NUL => match memchr::memchr(0, &inner.data) {
            Some(pos) => (pos, pos + 1),
            None => return std::ptr::null_mut(),
        },
    };

    // Allocate with malloc
    let out = unsafe { libc::malloc(line_len + 1) } as *mut u8;
    if out.is_null() {
        return std::ptr::null_mut();
    }
    unsafe {
        std::ptr::copy_nonoverlapping(inner.data.as_ptr(), out, line_len);
        *out.add(line_len) = 0; // null terminate
    }

    if !n_read_out.is_null() {
        unsafe { *n_read_out = line_len };
    }

    inner.data.drain(..drain_len);

    out
}

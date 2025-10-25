// Author: Tatu Ylonen <ylo@cs.hut.fi>
// Copyright (c) 1995 Tatu Ylonen <ylo@cs.hut.fi>, Espoo, Finland
//                    All rights reserved
// Versions of malloc and friends that check their results, and never return
// failure (they call fatalx if they encounter an error).
//
// As far as I am concerned, the code I have written for this software
// can be used freely for any purpose.  Any derived versions of this
// software must be clearly marked as such, and if the derived work is
// incompatible with the protocol description in the RFC file, it must be
// called by a name other than "ssh" or "Secure Shell".
#![expect(clippy::panic)]
use std::{
    ffi::{CStr, c_void},
    mem::MaybeUninit,
    ptr::NonNull,
};

use crate::{
    compat::{reallocarray::reallocarray, recallocarray::recallocarray},
    fatalx,
};

pub fn xmalloc(size: usize) -> NonNull<c_void> {
    debug_assert_ne!(size, 0, "xmalloc: zero size");

    // Allocate using max_align_t to have the same allignment as malloc.
    // We allocate a bit too much when size is not a multiple of max_align_t.
    let count = size.div_ceil(size_of::<libc::max_align_t>());
    let alloc = vec![MaybeUninit::<libc::max_align_t>::uninit(); count].into_boxed_slice();
    NonNull::new(Box::into_raw(alloc))
        .expect("box pointer is not null")
        .cast()
}

pub fn xcalloc(nmemb: usize, size: usize) -> NonNull<c_void> {
    debug_assert!(size != 0 && nmemb != 0, "xcalloc: zero size");

    NonNull::new(unsafe { ::libc::calloc(nmemb, size) })
        .unwrap_or_else(|| panic!("xcalloc: allocating {nmemb} * {size}"))
}

pub fn xcalloc_<T>(nmemb: usize) -> NonNull<T> {
    xcalloc(nmemb, size_of::<T>()).cast()
}

pub unsafe fn xcalloc1<'a, T>() -> &'a mut T {
    let mut ptr: NonNull<T> = xcalloc(1, size_of::<T>()).cast();
    unsafe { ptr.as_mut() }
}

pub unsafe fn xrealloc(ptr: *mut c_void, size: usize) -> NonNull<c_void> {
    unsafe { xrealloc_(ptr, size) }
}

pub unsafe fn xrealloc_<T>(ptr: *mut T, size: usize) -> NonNull<T> {
    unsafe { xreallocarray_old(ptr, 1, size) }
}

pub unsafe fn xreallocarray(ptr: *mut c_void, nmemb: usize, size: usize) -> NonNull<c_void> {
    unsafe { xreallocarray_old(ptr, nmemb, size) }
}

pub unsafe fn xreallocarray_old<T>(ptr: *mut T, nmemb: usize, size: usize) -> NonNull<T> {
    unsafe {
        if nmemb == 0 || size == 0 {
            fatalx("xreallocarray: zero size");
        }

        match NonNull::new(reallocarray(ptr as _, nmemb, size)) {
            None => fatalx("xreallocarray: allocating "),
            Some(new_ptr) => new_ptr.cast(),
        }
    }
}

pub unsafe fn xreallocarray_<T>(ptr: *mut T, nmemb: usize) -> NonNull<T> {
    let size = size_of::<T>();
    unsafe {
        if nmemb == 0 || size == 0 {
            fatalx("xreallocarray: zero size");
        }

        match NonNull::new(reallocarray(ptr as _, nmemb, size)) {
            None => fatalx("xreallocarray: allocating"),
            Some(new_ptr) => new_ptr.cast(),
        }
    }
}

pub unsafe fn xrecallocarray(
    ptr: *mut c_void,
    oldnmemb: usize,
    nmemb: usize,
    size: usize,
) -> NonNull<c_void> {
    unsafe { xrecallocarray_(ptr, oldnmemb, nmemb, size) }
}

pub unsafe fn xrecallocarray_<T>(
    ptr: *mut T,
    oldnmemb: usize,
    nmemb: usize,
    size: usize,
) -> NonNull<T> {
    if nmemb == 0 || size == 0 {
        panic!("xrecallocarray: zero size");
    }

    NonNull::new(unsafe { recallocarray(ptr as *mut c_void, oldnmemb, nmemb, size) })
        .unwrap_or_else(|| panic!("xrecallocarray: allocating {nmemb} * {size}"))
        .cast()
}

pub unsafe fn xrecallocarray__<T>(ptr: *mut T, oldnmemb: usize, nmemb: usize) -> NonNull<T> {
    let size = size_of::<T>();
    if nmemb == 0 || size == 0 {
        panic!("xrecallocarray: zero size");
    }

    NonNull::new(unsafe { recallocarray(ptr as *mut c_void, oldnmemb, nmemb, size) })
        .unwrap_or_else(|| panic!("xrecallocarray: allocating {nmemb} * {size}"))
        .cast()
}

pub unsafe fn xstrdup(str: *const u8) -> NonNull<u8> {
    NonNull::new(unsafe { crate::libc::strdup(str) }).unwrap()
}

pub fn xstrdup_(str: &CStr) -> NonNull<u8> {
    unsafe { xstrdup(str.as_ptr().cast()) }
}

pub fn xstrdup__(str: &str) -> *mut u8 {
    let mut out = str.to_string();
    out.push('\0');
    out.leak().as_mut_ptr()
}
pub fn xstrdup___(str: Option<&str>) -> *mut u8 {
    let Some(str) = str else {
        return std::ptr::null_mut();
    };
    xstrdup__(str)
}

pub unsafe fn xstrndup(str: *const u8, maxlen: usize) -> NonNull<u8> {
    NonNull::new(unsafe { crate::libc::strndup(str, maxlen) }).unwrap()
}

macro_rules! format_nul {
   ($fmt:literal $(, $args:expr)* $(,)?) => {
        crate::xmalloc::format_nul_(format_args!($fmt $(, $args)*))
    };
}
pub(crate) use format_nul;
pub(crate) fn format_nul_(args: std::fmt::Arguments) -> *mut u8 {
    let mut s = args.to_string();
    s.push('\0');
    s.leak().as_mut_ptr()
}

macro_rules! xsnprintf_ {
   ($out:expr, $len:expr, $fmt:literal $(, $args:expr)* $(,)?) => {
        crate::xmalloc::xsnprintf__($out, $len, format_args!($fmt $(, $args)*))
    };
}
pub(crate) use xsnprintf_;
pub(crate) unsafe fn xsnprintf__(
    out: *mut u8,
    len: usize,
    args: std::fmt::Arguments,
) -> std::io::Result<usize> {
    use std::io::Write;

    struct WriteAdapter {
        buffer: *mut u8,
        length: usize,
        written: usize,
    }
    impl WriteAdapter {
        fn new(buffer: *mut u8, length: usize) -> Self {
            Self {
                buffer,
                length,
                written: 0,
            }
        }
    }

    impl std::io::Write for WriteAdapter {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            let remaining = self.length - self.written;
            let write_amount = buf.len().min(remaining);

            unsafe {
                std::ptr::copy_nonoverlapping(
                    buf.as_ptr(),
                    self.buffer.add(self.written).cast(),
                    write_amount,
                );
            }
            self.written += write_amount;

            Ok(write_amount)
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    let mut adapter = WriteAdapter::new(out, len);
    adapter.write_fmt(args)?;
    if adapter.write(&[0])? == 0 {
        return Err(std::io::ErrorKind::WriteZero.into());
    }

    Ok(adapter.written)
}

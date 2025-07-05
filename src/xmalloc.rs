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

use ::core::{
    ffi::{CStr, c_char, c_int, c_void},
    mem::MaybeUninit,
    num::NonZero,
    ptr::NonNull,
};

use ::libc::{calloc, malloc, strdup, strndup};

use crate::{
    compat::{reallocarray, recallocarray},
    fatalx, vasprintf, vsnprintf,
};

pub extern "C" fn xmalloc(size: usize) -> NonNull<c_void> {
    debug_assert!(size != 0, "xmalloc: zero size");

    NonNull::new(unsafe { malloc(size) }).unwrap_or_else(|| panic!("xmalloc: allocating {size}"))
}

// note this function definition is safe
#[inline]
fn malloc_(size: NonZero<usize>) -> *mut c_void {
    unsafe { malloc(size.get()) }
}

pub fn xmalloc_<T>() -> NonNull<T> {
    let size = size_of::<T>();
    debug_assert!(size != 0);
    let nz_size = NonZero::<usize>::try_from(size).unwrap();
    NonNull::new(malloc_(nz_size))
        .unwrap_or_else(|| panic!("xmalloc: allocating {size} bytes"))
        .cast()
}

pub fn xmalloc__<'a, T>() -> &'a mut MaybeUninit<T> {
    let size = size_of::<T>();
    debug_assert!(size != 0);
    let nz_size = NonZero::<usize>::try_from(size).unwrap();

    let ptr: NonNull<T> = NonNull::new(malloc_(nz_size))
        .unwrap_or_else(|| panic!("xmalloc: allocating {size} bytes"))
        .cast();

    // from `NonNull::as_uninit_mut`
    unsafe { &mut *ptr.cast().as_ptr() }
}

#[inline]
pub fn calloc_(nmemb: usize, size: usize) -> *mut c_void {
    unsafe { calloc(nmemb, size) }
}

pub extern "C" fn xcalloc(nmemb: usize, size: usize) -> NonNull<c_void> {
    debug_assert!(size != 0 && nmemb != 0, "xcalloc: zero size");

    NonNull::new(calloc_(nmemb, size))
        .unwrap_or_else(|| panic!("xcalloc: allocating {nmemb} * {size}"))
}

pub fn xcalloc_<T>(nmemb: usize) -> NonNull<T> {
    xcalloc(nmemb, size_of::<T>()).cast()
}

pub unsafe fn xcalloc1<'a, T>() -> &'a mut T {
    let mut ptr: NonNull<T> = xcalloc(1, size_of::<T>()).cast();
    unsafe { ptr.as_mut() }
}

pub fn xcalloc1__<'a, T>() -> &'a mut MaybeUninit<T> {
    let size = size_of::<T>();
    debug_assert!(size != 0, "xcalloc: zero size");

    let ptr: *mut T = unsafe { calloc(1, size).cast() };
    if ptr.is_null() {
        panic!("bad xcalloc1_: out of memory");
    }

    unsafe { &mut *ptr.cast::<MaybeUninit<T>>() }
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

pub unsafe fn xstrdup(str: *const c_char) -> NonNull<c_char> {
    NonNull::new(unsafe { strdup(str) }).unwrap()
}

pub fn xstrdup_(str: &CStr) -> NonNull<c_char> {
    unsafe { xstrdup(str.as_ptr()) }
}

pub fn xstrdup__<'a>(str: &CStr) -> &'a CStr {
    unsafe { CStr::from_ptr(xstrdup(str.as_ptr()).as_ptr()) }
}

pub unsafe fn xstrndup(str: *const c_char, maxlen: usize) -> NonNull<c_char> {
    NonNull::new(unsafe { strndup(str, maxlen) }).unwrap()
}

// #[allow(improper_ctypes_definitions, reason = "must be extern C to use c variadics")]
// pub unsafe fn xasprintf__(args: std::fmt::Arguments<'_>) -> NonNull<c_char> {}

macro_rules! format_nul {
   ($fmt:literal $(, $args:expr)* $(,)?) => {
        crate::xmalloc::format_nul_(format_args!($fmt $(, $args)*))
    };
}
pub(crate) use format_nul;
pub(crate) fn format_nul_(args: std::fmt::Arguments) -> *mut c_char {
    let mut s = args.to_string();
    s.push('\0');
    s.leak().as_mut_ptr().cast()
}

macro_rules! xsnprintf_ {
   ($out:expr, $len:expr, $fmt:literal $(, $args:expr)* $(,)?) => {
        crate::xmalloc::xsnprintf__($out, $len, format_args!($fmt $(, $args)*))
    };
}
pub(crate) use xsnprintf_;
pub(crate) unsafe fn xsnprintf__(
    out: *mut c_char,
    len: usize,
    args: std::fmt::Arguments,
) -> std::io::Result<usize> {
    use std::io::Write;

    struct WriteAdapter {
        buffer: *mut c_char,
        length: usize,
        written: usize,
    }
    impl WriteAdapter {
        fn new(buffer: *mut c_char, length: usize) -> Self {
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

pub unsafe fn free_<T>(p: *mut T) {
    unsafe { libc::free(p as *mut c_void) }
}

pub unsafe fn memcpy_<T>(dest: *mut T, src: *const T, n: usize) -> *mut T {
    unsafe { libc::memcpy(dest as *mut c_void, src as *const c_void, n).cast() }
}

pub unsafe fn memcpy__<T>(dest: *mut T, src: *const T) -> *mut T {
    unsafe { libc::memcpy(dest as *mut c_void, src as *const c_void, size_of::<T>()).cast() }
}

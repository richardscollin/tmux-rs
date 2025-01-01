#![feature(c_variadic)]
#![allow(unused_variables)]
#![allow(clippy::missing_safety_doc)]
use core::ffi::{VaList, c_char, c_int, c_void};

use libc::{__errno_location, calloc, malloc, reallocarray, strdup, strerror, strndup};

unsafe extern "C" {
    fn vsnprintf(_: *mut c_char, _: usize, _: *const c_char, _: VaList) -> c_int;
    fn vasprintf(_: *mut *mut c_char, _: *const c_char, _: VaList) -> c_int;
    fn fatalx(_: *const c_char, _: ...) -> !;
}

#[unsafe(no_mangle)]
pub extern "C" fn xmalloc(size: usize) -> *mut c_void {
    unsafe {
        if size == 0 {
            fatalx(c"xmalloc: zero size".as_ptr());
        }

        let ptr = malloc(size);
        if ptr.is_null() {
            fatalx(
                c"xmalloc: allocating %zu bytes: %s".as_ptr(),
                size,
                strerror(*__errno_location()),
            );
        }

        ptr
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn xcalloc(nmemb: usize, size: usize) -> *mut c_void {
    unsafe {
        if size == 0 || nmemb == 0 {
            fatalx(c"xcalloc: zero size".as_ptr());
        }

        let ptr = calloc(nmemb, size);
        if ptr.is_null() {
            fatalx(
                c"xcalloc: allocating %zu * %zu bytes: %s".as_ptr(),
                nmemb,
                size,
                strerror(*__errno_location()),
            );
        }

        ptr
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xrealloc(ptr: *mut c_void, size: usize) -> *mut c_void {
    unsafe { xreallocarray(ptr, 1, size) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xreallocarray(ptr: *mut c_void, nmemb: usize, size: usize) -> *mut c_void {
    unsafe {
        if nmemb == 0 || size == 0 {
            fatalx(c"xreallocarray: zero size".as_ptr());
        }

        let new_ptr = reallocarray(ptr, nmemb, size);
        if new_ptr.is_null() {
            fatalx(
                c"xreallocarray: allocating %zu * %zu bytes: %s".as_ptr(),
                nmemb,
                size,
                strerror(*__errno_location()),
            );
        }

        new_ptr
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xrecallocarray(
    ptr: *mut c_void,
    oldnmemb: usize,
    nmemb: usize,
    size: usize,
) -> *mut c_void {
    /*
    unsafe {
        if nmemb == 0 || size == 0 {
            fatalx(c"xrecallocarray: zero size".as_ptr());
        }

        let mut new_ptr = recallocarray(ptr, oldnmemb, nmemb, size);
        if new_ptr.is_null() {
            fatalx(
                c"xrecallocarray: allocating %zu * %zu bytes: %s".as_ptr(),
                nmemb,
                size,
                strerror(*__errno_location()),
            );
        }

        new_ptr
    }
    */
    todo!()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xstrdup(str: *const c_char) -> *mut c_char {
    unsafe {
        let cp = strdup(str);

        if cp.is_null() {
            fatalx(c"xstrdup: %s".as_ptr(), strerror(*__errno_location()));
        }

        cp
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xstrndup(str: *const c_char, maxlen: usize) -> *mut c_char {
    unsafe {
        let cp = strndup(str, maxlen);

        if cp.is_null() {
            fatalx(c"xstrndup: %s".as_ptr(), strerror(*__errno_location()));
        }

        cp
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xasprintf(
    ret: *mut *mut c_char,
    fmt: *const c_char,
    mut args: ...
) -> c_int {
    unsafe { xvasprintf(ret, fmt, args.as_va_list()) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xvasprintf(
    ret: *mut *mut c_char,
    fmt: *const c_char,
    args: VaList,
) -> c_int {
    unsafe {
        let i = vasprintf(ret, fmt, args);

        if i == -1 {
            fatalx(c"xasprintf: %s".as_ptr(), strerror(*__errno_location()));
        }

        i
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xsnprintf(
    str: *mut c_char,
    len: usize,
    fmt: *const c_char,
    mut args: ...
) -> c_int {
    unsafe { xvsnprintf(str, len, fmt, args.as_va_list()) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xvsnprintf(
    str: *mut c_char,
    len: usize,
    fmt: *const c_char,
    args: VaList,
) -> c_int {
    unsafe {
        if len > i32::MAX as usize {
            fatalx(c"xsnprintf: len > INT_MAX".as_ptr());
        }

        let i = vsnprintf(str, len, fmt, args);
        if i < 0 || i >= len as c_int {
            fatalx(c"xsnprintf: overflow".as_ptr());
        }

        i
    }
}

// Copyright (c) 2008, 2017 Otto Moerbeek <otto@drijf.net>
//
// Permission to use, copy, modify, and distribute this software for any
// purpose with or without fee is hereby granted, provided that the above
// copyright notice and this permission notice appear in all copies.
//
// THE SOFTWARE IS PROVIDED "AS IS" AND THE AUTHOR DISCLAIMS ALL WARRANTIES
// WITH REGARD TO THIS SOFTWARE INCLUDING ALL IMPLIED WARRANTIES OF
// MERCHANTABILITY AND FITNESS. IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR
// ANY SPECIAL, DIRECT, INDIRECT, OR CONSEQUENTIAL DAMAGES OR ANY DAMAGES
// WHATSOEVER RESULTING FROM LOSS OF USE, DATA OR PROFITS, WHETHER IN AN
// ACTION OF CONTRACT, NEGLIGENCE OR OTHER TORTIOUS ACTION, ARISING OUT OF
// OR IN CONNECTION WITH THE USE OR PERFORMANCE OF THIS SOFTWARE.

#[cfg(not(target_os = "linux"))]
pub unsafe fn reallocarray(
    optr: *mut core::ffi::c_void,
    nmemb: usize,
    size: usize,
) -> *mut core::ffi::c_void {
    const MUL_NO_OVERFLOW: usize = 1usize << (size_of::<usize>() * 4);

    unsafe {
        if (nmemb >= MUL_NO_OVERFLOW || size >= MUL_NO_OVERFLOW)
            && nmemb > 0
            && usize::MAX / nmemb < size
        {
            crate::errno!() = libc::ENOMEM;
            return core::ptr::null_mut();
        }
        libc::realloc(optr, size * nmemb)
    }
}

#[cfg(target_os = "linux")]
pub(crate) use libc::reallocarray;

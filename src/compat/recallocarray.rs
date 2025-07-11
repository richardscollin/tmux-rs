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
use core::ffi::c_void;
use core::ptr::null_mut;

pub unsafe fn recallocarray(
    ptr: *mut c_void,
    oldnmemb: usize,
    newnmemb: usize,
    size: usize,
) -> *mut c_void {
    const MUL_NO_OVERFLOW: usize = 1usize << (size_of::<usize>() * 4);

    unsafe extern "C" {
        fn getpagesize() -> i32;
    }

    unsafe {
        if ptr.is_null() {
            return libc::calloc(newnmemb, size);
        }

        if (newnmemb >= MUL_NO_OVERFLOW || size >= MUL_NO_OVERFLOW)
            && newnmemb > 0
            && usize::MAX / newnmemb < size
        {
            crate::errno!() = libc::ENOMEM;
            return null_mut();
        }
        let newsize = newnmemb * size;

        if (oldnmemb >= MUL_NO_OVERFLOW || size >= MUL_NO_OVERFLOW)
            && oldnmemb > 0
            && usize::MAX / oldnmemb < size
        {
            crate::errno!() = libc::EINVAL;
            return null_mut();
        }
        let oldsize = oldnmemb * size;

        /*
         * Don't bother too much if we're shrinking just a bit,
         * we do not shrink for series of small steps, oh well.
         */
        if (newsize <= oldsize) {
            let d = oldsize - newsize;

            if (d < oldsize / 2 && d < getpagesize() as usize) {
                libc::memset((ptr as *mut u8).add(newsize).cast(), 0, d);
                return ptr;
            }
        }

        let newptr = libc::malloc(newsize);
        if newptr.is_null() {
            return null_mut();
        }

        if (newsize > oldsize) {
            libc::memcpy(newptr, ptr, oldsize);
            libc::memset(
                (newptr as *mut u8).add(oldsize).cast(),
                0,
                newsize - oldsize,
            );
        } else {
            libc::memcpy(newptr, ptr, newsize);
        }

        libc::memset(ptr, 0, oldsize);
        libc::free(ptr);

        newptr
    }
}

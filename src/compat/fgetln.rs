// Copyright (c) 2015 Joerg Jung <jung@openbsd.org>
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
use core::ptr::null_mut;

/// portable fgetln() version, NOT reentrant
pub unsafe fn fgetln(fp: *mut libc::FILE, len: *mut usize) -> *mut u8 {
    unsafe {
        static mut buf: *mut u8 = null_mut();
        static mut bufsz: usize = 0;
        let mut r = 0usize;

        if fp.is_null() || len.is_null() {
            crate::errno!() = libc::EINVAL;
            return null_mut();
        }
        if buf.is_null() {
            buf = libc::calloc(1, libc::BUFSIZ as usize).cast();
            if buf.is_null() {
                return null_mut();
            }
            bufsz = libc::BUFSIZ as usize;
        }

        let mut c = libc::fgetc(fp);
        while c != libc::EOF {
            *buf.add(r) = c as u8;
            r += 1;
            if (r == bufsz) {
                let p = super::reallocarray(buf.cast(), 2, bufsz);
                if p.is_null() {
                    let e = crate::errno!();
                    libc::free(buf.cast());
                    crate::errno!() = e;
                    buf = null_mut();
                    bufsz = 0;
                    return null_mut();
                }
                buf = p.cast();
                bufsz *= 2;
            }
            if c == b'\n' as i32 {
                break;
            }
            c = libc::fgetc(fp);
        }

        *len = r;
        if r == 0 { null_mut() } else { buf }
    }
}

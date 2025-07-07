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
        static mut BUF: *mut u8 = null_mut();
        static mut BUFSZ: usize = 0;
        let mut r = 0usize;

        if fp.is_null() || len.is_null() {
            crate::errno!() = libc::EINVAL;
            return null_mut();
        }
        if BUF.is_null() {
            BUF = libc::calloc(1, libc::BUFSIZ as usize).cast();
            if BUF.is_null() {
                return null_mut();
            }
            BUFSZ = libc::BUFSIZ as usize;
        }

        let mut c = libc::fgetc(fp);
        while c != libc::EOF {
            *BUF.add(r) = c as u8;
            r += 1;
            if r == BUFSZ {
                let p = super::reallocarray(BUF.cast(), 2, BUFSZ);
                if p.is_null() {
                    let e = crate::errno!();
                    libc::free(BUF.cast());
                    crate::errno!() = e;
                    BUF = null_mut();
                    BUFSZ = 0;
                    return null_mut();
                }
                BUF = p.cast();
                BUFSZ *= 2;
            }
            if c == b'\n' as i32 {
                break;
            }
            c = libc::fgetc(fp);
        }

        *len = r;
        if r == 0 { null_mut() } else { BUF }
    }
}

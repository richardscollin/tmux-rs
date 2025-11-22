// Copyright (c) 2019 Nicholas Marriott <nicholas.marriott@gmail.com>
//
// Permission to use, copy, modify, and distribute this software for any
// purpose with or without fee is hereby granted, provided that the above
// copyright notice and this permission notice appear in all copies.
//
// THE SOFTWARE IS PROVIDED "AS IS" AND THE AUTHOR DISCLAIMS ALL WARRANTIES
// WITH REGARD TO THIS SOFTWARE INCLUDING ALL IMPLIED WARRANTIES OF
// MERCHANTABILITY AND FITNESS. IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR
// ANY SPECIAL, DIRECT, INDIRECT, OR CONSEQUENTIAL DAMAGES OR ANY DAMAGES
// WHATSOEVER RESULTING FROM LOSS OF MIND, USE, DATA OR PROFITS, WHETHER
// IN AN ACTION OF CONTRACT, NEGLIGENCE OR OTHER TORTIOUS ACTION, ARISING
// OUT OF OR IN CONNECTION WITH THE USE OR PERFORMANCE OF THIS SOFTWARE.
use core::ffi::c_int;

use xmalloc::xrealloc_;

use crate::libc::{memcpy, regcomp, regex_t, regexec, regfree, regmatch_t, strlen};
use crate::*;

unsafe fn regsub_copy(
    buf: *mut *mut u8,
    len: *mut isize,
    text: *const u8,
    start: usize,
    end: usize,
) {
    let add: usize = end - start;
    unsafe {
        *buf = xrealloc_(*buf, (*len) as usize + add + 1).as_ptr();
        memcpy((*buf).add(*len as usize) as _, text.add(start) as _, add);
        (*len) += add as isize;
    }
}

pub unsafe fn regsub_expand(
    buf: *mut *mut u8,
    len: *mut isize,
    with: *const u8,
    text: *const u8,
    m: *mut regmatch_t,
    n: c_uint,
) {
    unsafe {
        let mut cp = with;
        while *cp != b'\0' {
            if *cp == b'\\' {
                cp = cp.add(1);
                if *cp >= b'0' as _ && *cp <= b'9' as _ {
                    let i = (*cp - b'0') as u32;
                    if i < n && (*m.add(i as _)).rm_so != (*m.add(i as _)).rm_eo {
                        regsub_copy(
                            buf,
                            len,
                            text,
                            (*m.add(i as _)).rm_so as usize,
                            (*m.add(i as _)).rm_eo as usize,
                        );
                        continue;
                    }
                }
            }
            *buf = xrealloc_(*buf, (*len) as usize + 2).as_ptr();
            *(*buf).add((*len) as usize) = *cp;
            (*len) += 1;

            cp = cp.add(1);
        }
    }
}

pub unsafe fn regsub(
    pattern: *const u8,
    with: *const u8,
    text: *const u8,
    flags: c_int,
) -> *mut u8 {
    unsafe {
        let mut r: regex_t = zeroed();
        let mut m: [regmatch_t; 10] = zeroed(); // TODO can use uninit
        let mut len: isize = 0;
        let mut empty = 0;
        let mut buf = null_mut();

        if *text == b'\0' {
            return xstrdup(c!("")).cast().as_ptr();
        }
        if regcomp(&raw mut r, pattern, flags) != 0 {
            return null_mut();
        }

        let mut start: isize = 0;
        let mut last: isize = 0;
        let end: isize = strlen(text) as _;

        while start <= end {
            if regexec(
                &raw mut r,
                text.add(start as _) as _,
                m.len(),
                m.as_mut_ptr(),
                0,
            ) != 0
            {
                regsub_copy(
                    &raw mut buf,
                    &raw mut len,
                    text,
                    start as usize,
                    end as usize,
                );
                break;
            }

            // Append any text not part of this match (from the end of the
            // last match).
            regsub_copy(
                &raw mut buf,
                &raw mut len,
                text,
                last as usize,
                (m[0].rm_so as isize + start) as usize,
            );

            // If the last match was empty and this one isn't (it is either
            // later or has matched text), expand this match. If it is
            // empty, move on one character and try again from there.
            if empty != 0 || start + m[0].rm_so as isize != last || m[0].rm_so != m[0].rm_eo {
                regsub_expand(
                    &raw mut buf,
                    &raw mut len,
                    with,
                    text.offset(start),
                    m.as_mut_ptr(),
                    m.len() as u32,
                );

                last = start + m[0].rm_eo as isize;
                start += m[0].rm_eo as isize;
                empty = 0;
            } else {
                last = start + m[0].rm_eo as isize;
                start += (m[0].rm_eo + 1) as isize;
                empty = 1;
            }

            // Stop now if anchored to start.
            if *pattern == b'^' {
                regsub_copy(
                    &raw mut buf,
                    &raw mut len,
                    text,
                    start as usize,
                    end as usize,
                );
                break;
            }
        }
        *buf.offset(len) = b'\0' as _;

        regfree(&raw mut r);
        buf
    }
}

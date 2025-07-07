// Copyright (c) 1989, 1993
// The Regents of the University of California.  All rights reserved.
//
// Redistribution and use in source and binary forms, with or without
// modification, are permitted provided that the following conditions
// are met:
// 1. Redistributions of source code must retain the above copyright
//    notice, this list of conditions and the following disclaimer.
// 2. Redistributions in binary form must reproduce the above copyright
//    notice, this list of conditions and the following disclaimer in the
//    documentation and/or other materials provided with the distribution.
// 3. Neither the name of the University nor the names of its contributors
//    may be used to endorse or promote products derived from this software
//    without specific prior written permission.
//
// THIS SOFTWARE IS PROVIDED BY THE REGENTS AND CONTRIBUTORS ``AS IS'' AND
// ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
// IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE
// ARE DISCLAIMED.  IN NO EVENT SHALL THE REGENTS OR CONTRIBUTORS BE LIABLE
// FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
// DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS
// OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION)
// HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT
// LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY
// OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF
// SUCH DAMAGE.
use core::ffi::c_int;

// documentation from vis(3bsd)
bitflags::bitflags! {
    #[repr(transparent)]
    #[derive(Copy, Clone, Eq, PartialEq)]
    pub(crate) struct vis_flags: i32 {
        /// Use a three digit octal sequence.  The form is '\ddd' where d represents an octal digit.
        const VIS_OCTAL   = 0x0001;

        /// Use C-style backslash sequences to represent standard non-printable characters.
        /// The following sequences are used to represent the indicated characters:
        /// \a - BEL (007)
        /// \b - BS  (010)
        /// \t - HT  (011)
        /// \n - NL  (012)
        /// \v - VT  (013)
        /// \f - NP  (014)
        /// \r - CR  (015)
        /// \s - SP  (040)
        /// \0 - NUL (000)
        const VIS_CSTYLE  = 0x0002;

        /// encode tab
        const VIS_TAB     = 0x0008;

        /// encode newline
        const VIS_NL      = 0x0010;

        /// inhibit the doubling of backslashes and the backslash before the default format
        /// (that is, control characters are represented by ‘^C’ and meta characters as ‘M-C’).
        /// with this flag set, the encoding is ambiguous and non-invertible.
        const VIS_NOSLASH = 0x0040;

        /// encode the magic characters *, ?, [, and # recognized by glob(3)
        const VIS_GLOB    = 0x1000;

        /// encode double quote
        const VIS_DQ      = 0x0200;
    }
}

/// copies into dst a string which represents the character c. If c needs no encoding, it is copied in unaltered.
/// The string is null terminated, and a pointer to the end of the string is returned.
pub unsafe fn vis_(dst: *mut u8, c: c_int, flag: vis_flags, nextc: c_int) -> *mut u8 {
    unsafe {
        if flag.intersects(vis_flags::VIS_CSTYLE) {
            match c as u8 {
                b'\0' if !matches!(nextc as u8, b'0'..=b'7') => encode_cstyle(dst, b'0'),
                b'\t' if flag.intersects(vis_flags::VIS_TAB) => encode_cstyle(dst, b't'),
                b'\n' if flag.intersects(vis_flags::VIS_NL) => encode_cstyle(dst, b'n'),
                b'\\' if !flag.intersects(vis_flags::VIS_NOSLASH) => encode_cstyle(dst, b'\\'),
                b'*' | b'?' | b'[' | b'#' if flag.intersects(vis_flags::VIS_GLOB) => {
                    encode_cstyle(dst, c as u8)
                }
                7..9 | 11..14 => {
                    const CSTYLE: [u8; 7] = [b'a', b'b', 0, 0, b'v', b'f', b'r'];
                    encode_cstyle(dst, CSTYLE[c as usize - 7])
                }
                0..7 | 14..32 | 92 | 127.. => encode_octal(dst, c),
                _ => encode_passthrough(dst, c),
            }
        } else {
            match c as u8 {
                b'\t' if flag.intersects(vis_flags::VIS_TAB) => encode_octal(dst, c),
                b'\n' if flag.intersects(vis_flags::VIS_NL) => encode_octal(dst, c),
                b'*' | b'?' | b'[' | b'#' if flag.intersects(vis_flags::VIS_GLOB) => {
                    encode_octal(dst, c)
                }
                b'\\' if !flag.intersects(vis_flags::VIS_NOSLASH) => encode_octal(dst, c),
                0..9 | 11..32 | 127.. => encode_octal(dst, c),
                _ => encode_passthrough(dst, c),
            }
        }
    }
}

#[inline]
unsafe fn encode_passthrough(dst: *mut u8, ch: i32) -> *mut u8 {
    unsafe {
        *dst = ch as u8;
        *dst.add(1) = b'\0';
        dst.add(1)
    }
}

#[inline]
unsafe fn encode_cstyle(dst: *mut u8, ch: u8) -> *mut u8 {
    unsafe {
        *dst = b'\\';
        *dst.add(1) = ch;
        *dst.add(2) = b'\0';
        dst.add(2)
    }
}

#[inline]
unsafe fn encode_octal(dst: *mut u8, c: i32) -> *mut u8 {
    unsafe {
        let c = c as u8;
        let ones_place = c % 8;
        let eights_place = (c / 8) % 8;
        let sixty_four_place = c / 64;
        *dst = b'\\';
        *dst.add(1) = sixty_four_place + b'0';
        *dst.add(2) = eights_place + b'0';
        *dst.add(3) = ones_place + b'0';
        *dst.add(4) = b'\0';
        dst.add(4)
    }
}

pub unsafe fn strvis(mut dst: *mut u8, mut src: *const u8, flag: vis_flags) -> i32 {
    unsafe {
        let start = dst;

        while *src != 0 {
            dst = vis_(dst, *src as i32, flag, *src.add(1) as i32);
            src = src.add(1);
        }
        *dst = 0;

        dst.offset_from(start) as i32
    }
}

pub unsafe fn strnvis(mut dst: *mut u8, mut src: *const u8, dlen: usize, flag: vis_flags) -> i32 {
    unsafe {
        let mut i = 0;

        while *src != 0 && i < dlen {
            let tmp = vis_(dst, *src as i32, flag, *src.add(1) as i32);
            i += dst.offset_from_unsigned(dst);
            dst = tmp;
            src = src.add(1);
        }
        *dst = 0;

        i as i32
    }
}

pub unsafe fn stravis(outp: *mut *mut u8, src: *const u8, flag: vis_flags) -> i32 {
    unsafe {
        let buf: *mut u8 = libc::calloc(4, crate::libc::strlen(src) + 1).cast();
        if buf.is_null() {
            return -1;
        }
        let len = strvis(buf, src, flag);
        let serrno = crate::errno!();
        *outp = libc::realloc(buf.cast(), len as usize + 1).cast();
        if (*outp).is_null() {
            *outp = buf;
            crate::errno!() = serrno;
        }

        len
    }
}

pub unsafe fn vis(dst: *mut u8, c: c_int, flag: vis_flags, nextc: c_int) -> *mut u8 {
    unsafe { vis_(dst, c, flag, nextc) }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_vis() {
        let mut c_dst_arr: [u8; 16] = [0; 16];
        let mut rs_dst_arr: [u8; 16] = [0; 16];

        let c_dst = &raw mut c_dst_arr as *mut u8;
        let rs_dst = &raw mut rs_dst_arr as *mut u8;

        unsafe {
            for f1 in [
                vis_flags::VIS_OCTAL,
                vis_flags::VIS_CSTYLE,
                vis_flags::VIS_OCTAL | vis_flags::VIS_CSTYLE,
            ] {
                for f2 in [
                    vis_flags::VIS_GLOB, //
                    vis_flags::VIS_TAB | vis_flags::VIS_NL,
                    vis_flags::VIS_TAB,
                    vis_flags::VIS_NL,
                    vis_flags::VIS_DQ,
                    vis_flags::VIS_NOSLASH,
                ] {
                    for ch in 0..=u8::MAX {
                        for nextc in [b'\0' as i32, b'0' as i32] {
                            let flag = f1 | f2;
                            let rs_out = vis_(rs_dst, ch as i32, flag, nextc);
                            let c_out = vis(c_dst, ch as i32, flag, nextc);

                            assert_eq!(
                                c_dst_arr,
                                rs_dst_arr,
                                "mismatch when encoding vis(_, _, _, {ch}) => {} != {}",
                                crate::_s(c_dst),
                                crate::_s(rs_dst)
                            );

                            assert_eq!(rs_out.offset_from(rs_dst), c_out.offset_from(c_dst));

                            c_dst_arr.fill(0);
                            rs_dst_arr.fill(0);
                        }
                    }
                }
            }
        }
    }
}

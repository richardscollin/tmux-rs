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

// it's a bit silly to have a single variant enum, but
// it calls attention that our implementation of vis
// is slightly different than the standard and that the
// VIS_OCTAL and VIS_CSTYLE flags are set unconditionally
#[derive(Copy, Clone)]
pub enum VisMode {
    CombinedCStyleOctal,
}

// Use C-style backslash sequences to represent standard non-printable characters.
// The following sequences are used to represent the indicated characters:
// \a - BEL (007)
// \b - BS  (010)
// \t - HT  (011)
// \n - NL  (012)
// \v - VT  (013)
// \f - NP  (014)
// \r - CR  (015)
// \s - SP  (040)
// \0 - NUL (000)

// documentation from vis(3bsd)
bitflags::bitflags! {
    #[repr(transparent)]
    #[derive(Copy, Clone, Debug, Eq, PartialEq)]
    pub(crate) struct vis_flags: i32 {
        // tmux-rs assumes both VIS_OCTAL and VIS_CSTYLE are set unconditionally.
        // these hex values should match against system libbsd for tests (not required for functionality)

        /// encode tab
        const VIS_TAB     = 0x0008;

        /// encode newline
        const VIS_NL      = 0x0010;

        /// inhibit the doubling of backslashes and the backslash before the default format
        /// (that is, control characters are represented by ‘^C’ and meta characters as ‘M-C’).
        /// with this flag set, the encoding is ambiguous and non-invertible.
        const VIS_NOSLASH = 0x0040;

        /// encode double quote
        const VIS_DQ      = 0x8000;
    }
}

/// copies into dst a string which represents the character c. If c needs no encoding, it is copied in unaltered.
/// The string is null terminated, and a pointer to the end of the string is returned.
pub unsafe fn vis_(
    dst: *mut u8,
    c: c_int,
    _mode: VisMode,
    flag: vis_flags,
    nextc: c_int,
) -> *mut u8 {
    unsafe {
        match c as u8 {
            b'\0' if !matches!(nextc as u8, b'0'..=b'7') => encode_cstyle(dst, b'0'),
            b'\t' if flag.intersects(vis_flags::VIS_TAB) => encode_cstyle(dst, b't'),
            b'\n' if flag.intersects(vis_flags::VIS_NL) => encode_cstyle(dst, b'n'),
            b'"' if flag.intersects(vis_flags::VIS_DQ) => encode_cstyle(dst, b'"'),
            b'\\' if !flag.intersects(vis_flags::VIS_NOSLASH) => encode_cstyle(dst, b'\\'),
            7..9 | 11..14 => {
                const CSTYLE: [u8; 7] = [b'a', b'b', 0, 0, b'v', b'f', b'r'];
                encode_cstyle(dst, CSTYLE[c as usize - 7])
            }
            0..7 | 14..32 | 127.. => encode_octal(dst, c),
            _ => encode_passthrough(dst, c),
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

pub unsafe fn strvis(mut dst: *mut u8, mut src: *const u8, mode: VisMode, flag: vis_flags) -> i32 {
    unsafe {
        let start = dst;

        while *src != 0 {
            dst = vis_(dst, *src as i32, mode, flag, *src.add(1) as i32);
            src = src.add(1);
        }
        *dst = 0;

        dst.offset_from(start) as i32
    }
}

pub unsafe fn strnvis(
    mut dst: *mut u8,
    mut src: *const u8,
    dlen: usize,
    mode: VisMode,
    flag: vis_flags,
) -> i32 {
    unsafe {
        let mut i = 0;

        while *src != 0 && i < dlen {
            let tmp = vis_(dst, *src as i32, mode, flag, *src.add(1) as i32);
            i += dst.offset_from_unsigned(dst);
            dst = tmp;
            src = src.add(1);
        }
        *dst = 0;

        i as i32
    }
}

pub unsafe fn stravis(outp: *mut *mut u8, src: *const u8, mode: VisMode, flag: vis_flags) -> i32 {
    unsafe {
        let buf: *mut u8 = libc::calloc(4, crate::libc::strlen(src) + 1).cast();
        if buf.is_null() {
            return -1;
        }
        let len = strvis(buf, src, mode, flag);
        let serrno = crate::errno!();
        *outp = libc::realloc(buf.cast(), len as usize + 1).cast();
        if (*outp).is_null() {
            *outp = buf;
            crate::errno!() = serrno;
        }

        len
    }
}

#[cfg(test)]
mod test {
    use super::*;

    pub unsafe fn vis_rs(dst: *mut u8, c: c_int, flag: vis_flags, nextc: c_int) -> *mut u8 {
        unsafe { vis_(dst, c, VisMode::CombinedCStyleOctal, flag, nextc) }
    }

    pub unsafe fn vis_c(dst: *mut u8, c: c_int, flag: vis_flags, nextc: c_int) -> *mut u8 {
        #[link(name = "bsd")]
        unsafe extern "C" {
            pub unsafe fn vis(dst: *mut u8, c: c_int, flag: i32, nextc: c_int) -> *mut u8;
        }

        const VIS_OCTAL: i32 = 0x0001;
        const VIS_CSTYLE: i32 = 0x0002;
        unsafe { vis(dst, c, flag.bits() | VIS_OCTAL | VIS_CSTYLE, nextc) }
    }

    #[test]
    fn test_vis() {
        let mut c_dst_arr: [u8; 16] = [0; 16];
        let mut rs_dst_arr: [u8; 16] = [0; 16];

        let c_dst = &raw mut c_dst_arr as *mut u8;
        let rs_dst = &raw mut rs_dst_arr as *mut u8;

        unsafe {
            for flag in [
                vis_flags::VIS_TAB | vis_flags::VIS_NL,
                vis_flags::VIS_TAB,
                vis_flags::VIS_NL,
                vis_flags::VIS_DQ,
                vis_flags::VIS_NOSLASH,
            ] {
                for ch in 0..=u8::MAX {
                    for nextc in [b'\0' as i32, b'0' as i32] {
                        let c_out = vis_c(c_dst, ch as i32, flag, nextc);
                        let rs_out = vis_rs(rs_dst, ch as i32, flag, nextc);

                        assert_eq!(
                            c_dst_arr,
                            rs_dst_arr,
                            "mismatch when encoding vis(_, {ch}, {:?}, {nextc}) => {} != {}",
                            flag,
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

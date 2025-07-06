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

// generated using c2rust on unvis.c
// TODO refactor

use ::core::ffi::c_char;

#[unsafe(no_mangle)]
pub unsafe fn unvis(mut cp: *mut u8, mut c: u8, mut astate: *mut i32, mut flag: i32) -> i32 {
    unsafe {
        if flag & 1 != 0 {
            if *astate == 5 || *astate == 6 {
                *astate = 0;
                return 1;
            }
            return if *astate == 0 { 3 } else { -1 };
        }
        match *astate {
            0 => {
                *cp = 0;
                if c == b'\\' {
                    *astate = 1;
                    return 0;
                }
                *cp = c;
                1
            }
            1 => {
                match c as libc::c_int {
                    92 => {
                        *cp = c;
                        *astate = 0;
                        return 1;
                    }
                    48..=55 => {
                        *cp = c - b'0';
                        *astate = 5;
                        return 0;
                    }
                    77 => {
                        *cp = 0o200i32 as u8;
                        *astate = 2;
                        return 0;
                    }
                    94 => {
                        *astate = 4;
                        return 0;
                    }
                    110 => {
                        *cp = b'\n';
                        *astate = 0;
                        return 1;
                    }
                    114 => {
                        *cp = b'\r';
                        *astate = 0;
                        return 1;
                    }
                    98 => {
                        *cp = '\u{8}' as u8;
                        *astate = 0;
                        return 1;
                    }
                    97 => {
                        *cp = '\u{7}' as u8;
                        *astate = 0;
                        return 1;
                    }
                    118 => {
                        *cp = '\u{b}' as u8;
                        *astate = 0;
                        return 1;
                    }
                    116 => {
                        *cp = b'\t';
                        *astate = 0;
                        return 1;
                    }
                    102 => {
                        *cp = '\u{c}' as u8;
                        *astate = 0;
                        return 1;
                    }
                    115 => {
                        *cp = b' ';
                        *astate = 0;
                        return 1;
                    }
                    69 => {
                        *cp = '\u{1b}' as u8;
                        *astate = 0;
                        return 1;
                    }
                    10 => {
                        *astate = 0;
                        return 3;
                    }
                    36 => {
                        *astate = 0;
                        return 3;
                    }
                    _ => {}
                }
                *astate = 0;
                -1
            }
            2 => {
                if c == b'-' {
                    *astate = 3;
                } else if c == b'^' {
                    *astate = 4;
                } else {
                    *astate = 0;
                    return -1;
                }
                0
            }
            3 => {
                *astate = 0;
                *cp = *cp | c;
                1
            }
            4 => {
                if c == b'?' {
                    *cp = (*cp as libc::c_int | 0o177 as libc::c_int) as u8;
                } else {
                    *cp = (*cp as libc::c_int | c as libc::c_int & 0o37 as libc::c_int) as u8;
                }
                *astate = 0;
                1
            }
            5 => {
                if c >= b'0' && c <= b'7' {
                    *cp = (((*cp as libc::c_int) << 3 as libc::c_int) + (c - b'0') as i32) as u8;
                    *astate = 6;
                    return 0;
                }
                *astate = 0;
                2
            }
            6 => {
                *astate = 0 as libc::c_int;
                if c >= b'0' && c <= b'7' {
                    *cp = (((*cp as libc::c_int) << 3 as libc::c_int) + (c - b'0') as i32) as u8;
                    return 1;
                }
                2
            }
            _ => {
                *astate = 0;
                -1
            }
        }
    }
}
#[unsafe(no_mangle)]
pub unsafe fn strunvis(mut dst: *mut u8, mut src: *const u8) -> i32 {
    unsafe {
        let mut c: u8 = 0;
        let mut start: *mut u8 = dst;
        let mut state: i32 = 0;
        loop {
            let fresh0 = src;
            src = src.offset(1);
            c = *fresh0;
            if c == 0 {
                break;
            }
            loop {
                match unvis(dst, c, &mut state, 0 as libc::c_int) {
                    1 => {
                        dst = dst.offset(1);
                        dst;
                        break;
                    }
                    2 => {
                        dst = dst.offset(1);
                        dst;
                    }
                    0 | 3 => {
                        break;
                    }
                    _ => {
                        *dst = b'\0';
                        return -1;
                    }
                }
            }
        }
        if unvis(dst, c, &mut state, 1 as libc::c_int) == 1 as libc::c_int {
            dst = dst.offset(1);
            dst;
        }
        *dst = b'\0';
        dst.offset_from(start) as i32
    }
}
#[unsafe(no_mangle)]
pub unsafe fn strnunvis(mut dst: *mut u8, mut src: *const u8, mut sz: usize) -> isize {
    unsafe {
        let mut c: u8 = 0;
        let mut p: u8 = 0;
        let mut start: *mut u8 = dst;
        let mut end: *mut u8 = dst.add(sz).offset(-1);
        let mut state: i32 = 0;
        if sz > 0 {
            *end = b'\0';
        }
        loop {
            let fresh1 = src;
            src = src.offset(1);
            c = *fresh1;
            if c == 0 {
                break;
            }
            loop {
                match unvis(&mut p, c, &mut state, 0 as libc::c_int) {
                    1 => {
                        if dst < end {
                            *dst = p;
                        }
                        dst = dst.offset(1);
                        dst;
                        break;
                    }
                    2 => {
                        if dst < end {
                            *dst = p;
                        }
                        dst = dst.offset(1);
                        dst;
                    }
                    0 | 3 => {
                        break;
                    }
                    _ => {
                        if dst <= end {
                            *dst = b'\0';
                        }
                        return -1;
                    }
                }
            }
        }
        if unvis(&mut p, c, &mut state, 1 as libc::c_int) == 1 as libc::c_int {
            if dst < end {
                *dst = p;
            }
            dst = dst.offset(1);
            dst;
        }
        if dst <= end {
            *dst = b'\0';
        }
        dst.offset_from(start)
    }
}

// Copyright (c) 2008 Nicholas Marriott <nicholas.marriott@gmail.com>
// Copyright (c) 2016 Avi Halachmi <avihpit@yahoo.com>
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
use crate::*;
use core::ffi::{CStr, c_double, c_int, c_uchar};
use std::{
    io::Write as _,
    ptr::{null, null_mut},
};

use crate::libc::{free, sscanf, strcasecmp, strncasecmp, strncmp};
use xmalloc::xstrndup;

const COLOUR_FLAG_256: i32 = 0x01000000;
const COLOUR_FLAG_RGB: i32 = 0x02000000;

fn colour_dist_sq(r1: i32, g1: i32, b1: i32, r2: i32, g2: i32, b2: i32) -> i32 {
    (r1 - r2) * (r1 - r2) + (g1 - g2) * (g1 - g2) + (b1 - b2) * (b1 - b2)
}

fn colour_to_6cube(v: i32) -> i32 {
    if v < 48 {
        0
    } else if v < 114 {
        1
    } else {
        (v - 35) / 40
    }
}

/// Convert an RGB triplet to the xterm(1) 256 colour palette.
///
/// xterm provides a 6x6x6 colour cube (16 - 231) and 24 greys (232 - 255). We
/// map our RGB colour to the closest in the cube, also work out the closest
/// grey, and use the nearest of the two.
///
/// Note that the xterm has much lower resolution for darker colours (they are
/// not evenly spread out), so our 6 levels are not evenly spread: 0x0, 0x5f
/// (95), 0x87 (135), 0xaf (175), 0xd7 (215) and 0xff (255). Greys are more
/// evenly spread (8, 18, 28 ... 238).
pub fn colour_find_rgb(r: u8, g: u8, b: u8) -> i32 {
    // convert to i32 to better match c's integer promotion rules
    let r = r as i32;
    let g = g as i32;
    let b = b as i32;

    const q2c: [i32; 6] = [0x00, 0x5f, 0x87, 0xaf, 0xd7, 0xff];

    // Map RGB to 6x6x6 cube.
    let qr = colour_to_6cube(r);
    let qg = colour_to_6cube(g);
    let qb = colour_to_6cube(b);
    let cr = q2c[qr as usize];
    let cg = q2c[qg as usize];
    let cb = q2c[qb as usize];

    // If we have hit the colour exactly, return early.
    if cr == r && cg == g && cb == b {
        return (16 + (36 * qr) + (6 * qg) + qb) | COLOUR_FLAG_256;
    }

    // Work out the closest grey (average of RGB).
    let grey_avg = (r + g + b) / 3;
    let grey_idx = if grey_avg > 238 {
        23
    } else {
        (grey_avg - 3) / 10
    };
    let grey = 8 + (10 * grey_idx);

    // Is grey or 6x6x6 colour closest?
    let d = colour_dist_sq(cr, cg, cb, r, g, b);
    let idx = if colour_dist_sq(grey, grey, grey, r, g, b) < d {
        232 + grey_idx
    } else {
        16 + (36 * qr) + (6 * qg) + qb
    };

    idx | COLOUR_FLAG_256
}

/// Join RGB into a colour.
pub fn colour_join_rgb(r: u8, g: u8, b: u8) -> i32 {
    (((r as i32) << 16) | ((g as i32) << 8) | (b as i32)) | COLOUR_FLAG_RGB
}

/// Split colour into RGB.
#[inline]
pub fn colour_split_rgb(c: i32) -> (u8 /* red */, u8 /* green */, u8 /* blue */) {
    (
        ((c >> 16) & 0xff) as u8,
        ((c >> 8) & 0xff) as u8,
        (c & 0xff) as u8,
    )
}

/// Force colour to RGB if not already.
pub fn colour_force_rgb(c: i32) -> i32 {
    if c & COLOUR_FLAG_RGB != 0 {
        c
    } else if c & COLOUR_FLAG_256 != 0 || (0..=7).contains(&c) {
        colour_256_to_rgb(c)
    } else if (90..=97).contains(&c) {
        colour_256_to_rgb(8 + c - 90)
    } else {
        -1
    }
}

/// Convert colour to a string.
#[allow(
    static_mut_refs,
    reason = "TODO need to find a better way to make use of the write macro without invoking ub"
)]
pub unsafe fn colour_tostring(c: i32) -> *const u8 {
    // TODO this function returns a static buffer
    // this means it's not thread safe and multiple
    // concurrent calls to this function would result in bugs
    // consider fixing / reworking the interface
    static mut buf: [u8; 32] = [0; 32];

    if c == -1 {
        return c!("none");
    }

    if c & COLOUR_FLAG_RGB != 0 {
        let (r, g, b) = colour_split_rgb(c);
        write!(unsafe { buf.as_mut_slice() }, "#{r:02x}{g:02x}{b:02x}\0").unwrap();
        return &raw const buf as _;
    }

    if c & COLOUR_FLAG_256 != 0 {
        write!(unsafe { buf.as_mut_slice() }, "colour{}\0", c & 0xff).unwrap();
        return &raw const buf as _;
    }

    match c {
        0 => c!("black"),
        1 => c!("red"),
        2 => c!("green"),
        3 => c!("yellow"),
        4 => c!("blue"),
        5 => c!("magenta"),
        6 => c!("cyan"),
        7 => c!("white"),
        8 => c!("default"),
        9 => c!("terminal"),
        90 => c!("brightblack"),
        91 => c!("brightred"),
        92 => c!("brightgreen"),
        93 => c!("brightyellow"),
        94 => c!("brightblue"),
        95 => c!("brightmagenta"),
        96 => c!("brightcyan"),
        97 => c!("brightwhite"),
        _ => c!("invalid"),
    }
}

/// Convert colour from string.
pub unsafe fn colour_fromstring(s: *const u8) -> c_int {
    unsafe {
        if *s == b'#' && libc::strlen(s) == 7 {
            let mut cp = s.wrapping_add(1);
            while (*cp).is_ascii_hexdigit() {
                cp = cp.wrapping_add(1);
            }
            if *cp != 0 {
                return -1;
            }

            let mut r: u8 = 0;
            let mut g: u8 = 0;
            let mut b: u8 = 0;

            let n = sscanf(
                s.wrapping_add(1).cast(),
                c"%2hhx%2hhx%2hhx".as_ptr(),
                &raw mut r,
                &raw mut g,
                &raw mut b,
            );
            if n != 3 {
                return -1;
            }
            return colour_join_rgb(r, g, b);
        }

        if strcaseeq_(s, "colour") {
            let mut errstr: *const u8 = null();
            let Ok(n) = strtonum(s.add(6), 0i32, 255) else {
                return -1;
            };
            return n | COLOUR_FLAG_256;
        }

        if strcaseeq_(s, "color") {
            let mut errstr: *const u8 = null();
            let Ok(n) = strtonum(s.add(5), 0i32, 255) else {
                return -1;
            };
            return n | COLOUR_FLAG_256;
        }

        if strcaseeq_(s, "default") {
            8
        } else if strcaseeq_(s, "terminal") {
            9
        } else if strcaseeq_(s, "black") || streq_(s, "0") {
            0
        } else if strcaseeq_(s, "red") || streq_(s, "1") {
            1
        } else if strcaseeq_(s, "green") || streq_(s, "2") {
            2
        } else if strcaseeq_(s, "yellow") || streq_(s, "3") {
            3
        } else if strcaseeq_(s, "blue") || streq_(s, "4") {
            4
        } else if strcaseeq_(s, "magenta") || streq_(s, "5") {
            5
        } else if strcaseeq_(s, "cyan") || streq_(s, "6") {
            6
        } else if strcaseeq_(s, "white") || streq_(s, "7") {
            7
        } else if strcaseeq_(s, "brightblack") || streq_(s, "90") {
            90
        } else if strcaseeq_(s, "brightred") || streq_(s, "91") {
            91
        } else if strcaseeq_(s, "brightgreen") || streq_(s, "92") {
            92
        } else if strcaseeq_(s, "brightyellow") || streq_(s, "93") {
            93
        } else if strcaseeq_(s, "brightblue") || streq_(s, "94") {
            94
        } else if strcaseeq_(s, "brightmagenta") || streq_(s, "95") {
            95
        } else if strcaseeq_(s, "brightcyan") || streq_(s, "96") {
            96
        } else if strcaseeq_(s, "brightwhite") || streq_(s, "97") {
            97
        } else {
            colour_byname(s)
        }
    }
}

/// Convert 256 colour to RGB colour.
fn colour_256_to_rgb(c: i32) -> i32 {
    const table: [i32; 256] = [
        0x000000, 0x800000, 0x008000, 0x808000, 0x000080, 0x800080, 0x008080, 0xc0c0c0, 0x808080,
        0xff0000, 0x00ff00, 0xffff00, 0x0000ff, 0xff00ff, 0x00ffff, 0xffffff, 0x000000, 0x00005f,
        0x000087, 0x0000af, 0x0000d7, 0x0000ff, 0x005f00, 0x005f5f, 0x005f87, 0x005faf, 0x005fd7,
        0x005fff, 0x008700, 0x00875f, 0x008787, 0x0087af, 0x0087d7, 0x0087ff, 0x00af00, 0x00af5f,
        0x00af87, 0x00afaf, 0x00afd7, 0x00afff, 0x00d700, 0x00d75f, 0x00d787, 0x00d7af, 0x00d7d7,
        0x00d7ff, 0x00ff00, 0x00ff5f, 0x00ff87, 0x00ffaf, 0x00ffd7, 0x00ffff, 0x5f0000, 0x5f005f,
        0x5f0087, 0x5f00af, 0x5f00d7, 0x5f00ff, 0x5f5f00, 0x5f5f5f, 0x5f5f87, 0x5f5faf, 0x5f5fd7,
        0x5f5fff, 0x5f8700, 0x5f875f, 0x5f8787, 0x5f87af, 0x5f87d7, 0x5f87ff, 0x5faf00, 0x5faf5f,
        0x5faf87, 0x5fafaf, 0x5fafd7, 0x5fafff, 0x5fd700, 0x5fd75f, 0x5fd787, 0x5fd7af, 0x5fd7d7,
        0x5fd7ff, 0x5fff00, 0x5fff5f, 0x5fff87, 0x5fffaf, 0x5fffd7, 0x5fffff, 0x870000, 0x87005f,
        0x870087, 0x8700af, 0x8700d7, 0x8700ff, 0x875f00, 0x875f5f, 0x875f87, 0x875faf, 0x875fd7,
        0x875fff, 0x878700, 0x87875f, 0x878787, 0x8787af, 0x8787d7, 0x8787ff, 0x87af00, 0x87af5f,
        0x87af87, 0x87afaf, 0x87afd7, 0x87afff, 0x87d700, 0x87d75f, 0x87d787, 0x87d7af, 0x87d7d7,
        0x87d7ff, 0x87ff00, 0x87ff5f, 0x87ff87, 0x87ffaf, 0x87ffd7, 0x87ffff, 0xaf0000, 0xaf005f,
        0xaf0087, 0xaf00af, 0xaf00d7, 0xaf00ff, 0xaf5f00, 0xaf5f5f, 0xaf5f87, 0xaf5faf, 0xaf5fd7,
        0xaf5fff, 0xaf8700, 0xaf875f, 0xaf8787, 0xaf87af, 0xaf87d7, 0xaf87ff, 0xafaf00, 0xafaf5f,
        0xafaf87, 0xafafaf, 0xafafd7, 0xafafff, 0xafd700, 0xafd75f, 0xafd787, 0xafd7af, 0xafd7d7,
        0xafd7ff, 0xafff00, 0xafff5f, 0xafff87, 0xafffaf, 0xafffd7, 0xafffff, 0xd70000, 0xd7005f,
        0xd70087, 0xd700af, 0xd700d7, 0xd700ff, 0xd75f00, 0xd75f5f, 0xd75f87, 0xd75faf, 0xd75fd7,
        0xd75fff, 0xd78700, 0xd7875f, 0xd78787, 0xd787af, 0xd787d7, 0xd787ff, 0xd7af00, 0xd7af5f,
        0xd7af87, 0xd7afaf, 0xd7afd7, 0xd7afff, 0xd7d700, 0xd7d75f, 0xd7d787, 0xd7d7af, 0xd7d7d7,
        0xd7d7ff, 0xd7ff00, 0xd7ff5f, 0xd7ff87, 0xd7ffaf, 0xd7ffd7, 0xd7ffff, 0xff0000, 0xff005f,
        0xff0087, 0xff00af, 0xff00d7, 0xff00ff, 0xff5f00, 0xff5f5f, 0xff5f87, 0xff5faf, 0xff5fd7,
        0xff5fff, 0xff8700, 0xff875f, 0xff8787, 0xff87af, 0xff87d7, 0xff87ff, 0xffaf00, 0xffaf5f,
        0xffaf87, 0xffafaf, 0xffafd7, 0xffafff, 0xffd700, 0xffd75f, 0xffd787, 0xffd7af, 0xffd7d7,
        0xffd7ff, 0xffff00, 0xffff5f, 0xffff87, 0xffffaf, 0xffffd7, 0xffffff, 0x080808, 0x121212,
        0x1c1c1c, 0x262626, 0x303030, 0x3a3a3a, 0x444444, 0x4e4e4e, 0x585858, 0x626262, 0x6c6c6c,
        0x767676, 0x808080, 0x8a8a8a, 0x949494, 0x9e9e9e, 0xa8a8a8, 0xb2b2b2, 0xbcbcbc, 0xc6c6c6,
        0xd0d0d0, 0xdadada, 0xe4e4e4, 0xeeeeee,
    ];

    table[c as u8 as usize] | COLOUR_FLAG_RGB
}

pub fn colour_256to16(c: i32) -> i32 {
    const table: [u8; 256] = [
        0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 0, 4, 4, 4, 12, 12, 2, 6, 4, 4, 12,
        12, 2, 2, 6, 4, 12, 12, 2, 2, 2, 6, 12, 12, 10, 10, 10, 10, 14, 12, 10, 10, 10, 10, 10, 14,
        1, 5, 4, 4, 12, 12, 3, 8, 4, 4, 12, 12, 2, 2, 6, 4, 12, 12, 2, 2, 2, 6, 12, 12, 10, 10, 10,
        10, 14, 12, 10, 10, 10, 10, 10, 14, 1, 1, 5, 4, 12, 12, 1, 1, 5, 4, 12, 12, 3, 3, 8, 4, 12,
        12, 2, 2, 2, 6, 12, 12, 10, 10, 10, 10, 14, 12, 10, 10, 10, 10, 10, 14, 1, 1, 1, 5, 12, 12,
        1, 1, 1, 5, 12, 12, 1, 1, 1, 5, 12, 12, 3, 3, 3, 7, 12, 12, 10, 10, 10, 10, 14, 12, 10, 10,
        10, 10, 10, 14, 9, 9, 9, 9, 13, 12, 9, 9, 9, 9, 13, 12, 9, 9, 9, 9, 13, 12, 9, 9, 9, 9, 13,
        12, 11, 11, 11, 11, 7, 12, 10, 10, 10, 10, 10, 14, 9, 9, 9, 9, 9, 13, 9, 9, 9, 9, 9, 13, 9,
        9, 9, 9, 9, 13, 9, 9, 9, 9, 9, 13, 9, 9, 9, 9, 9, 13, 11, 11, 11, 11, 11, 15, 0, 0, 0, 0,
        0, 0, 8, 8, 8, 8, 8, 8, 7, 7, 7, 7, 7, 7, 15, 15, 15, 15, 15, 15,
    ];
    table[c as u8 as usize] as i32
}

pub unsafe fn colour_byname(name: *const u8) -> i32 {
    const COLOURS: [(&CStr, i32); 578] = [
        (c"AliceBlue", 0xf0f8ff),
        (c"AntiqueWhite", 0xfaebd7),
        (c"AntiqueWhite1", 0xffefdb),
        (c"AntiqueWhite2", 0xeedfcc),
        (c"AntiqueWhite3", 0xcdc0b0),
        (c"AntiqueWhite4", 0x8b8378),
        (c"BlanchedAlmond", 0xffebcd),
        (c"BlueViolet", 0x8a2be2),
        (c"CadetBlue", 0x5f9ea0),
        (c"CadetBlue1", 0x98f5ff),
        (c"CadetBlue2", 0x8ee5ee),
        (c"CadetBlue3", 0x7ac5cd),
        (c"CadetBlue4", 0x53868b),
        (c"CornflowerBlue", 0x6495ed),
        (c"DarkBlue", 0x00008b),
        (c"DarkCyan", 0x008b8b),
        (c"DarkGoldenrod", 0xb8860b),
        (c"DarkGoldenrod1", 0xffb90f),
        (c"DarkGoldenrod2", 0xeead0e),
        (c"DarkGoldenrod3", 0xcd950c),
        (c"DarkGoldenrod4", 0x8b6508),
        (c"DarkGray", 0xa9a9a9),
        (c"DarkGreen", 0x006400),
        (c"DarkGrey", 0xa9a9a9),
        (c"DarkKhaki", 0xbdb76b),
        (c"DarkMagenta", 0x8b008b),
        (c"DarkOliveGreen", 0x556b2f),
        (c"DarkOliveGreen1", 0xcaff70),
        (c"DarkOliveGreen2", 0xbcee68),
        (c"DarkOliveGreen3", 0xa2cd5a),
        (c"DarkOliveGreen4", 0x6e8b3d),
        (c"DarkOrange", 0xff8c00),
        (c"DarkOrange1", 0xff7f00),
        (c"DarkOrange2", 0xee7600),
        (c"DarkOrange3", 0xcd6600),
        (c"DarkOrange4", 0x8b4500),
        (c"DarkOrchid", 0x9932cc),
        (c"DarkOrchid1", 0xbf3eff),
        (c"DarkOrchid2", 0xb23aee),
        (c"DarkOrchid3", 0x9a32cd),
        (c"DarkOrchid4", 0x68228b),
        (c"DarkRed", 0x8b0000),
        (c"DarkSalmon", 0xe9967a),
        (c"DarkSeaGreen", 0x8fbc8f),
        (c"DarkSeaGreen1", 0xc1ffc1),
        (c"DarkSeaGreen2", 0xb4eeb4),
        (c"DarkSeaGreen3", 0x9bcd9b),
        (c"DarkSeaGreen4", 0x698b69),
        (c"DarkSlateBlue", 0x483d8b),
        (c"DarkSlateGray", 0x2f4f4f),
        (c"DarkSlateGray1", 0x97ffff),
        (c"DarkSlateGray2", 0x8deeee),
        (c"DarkSlateGray3", 0x79cdcd),
        (c"DarkSlateGray4", 0x528b8b),
        (c"DarkSlateGrey", 0x2f4f4f),
        (c"DarkTurquoise", 0x00ced1),
        (c"DarkViolet", 0x9400d3),
        (c"DeepPink", 0xff1493),
        (c"DeepPink1", 0xff1493),
        (c"DeepPink2", 0xee1289),
        (c"DeepPink3", 0xcd1076),
        (c"DeepPink4", 0x8b0a50),
        (c"DeepSkyBlue", 0x00bfff),
        (c"DeepSkyBlue1", 0x00bfff),
        (c"DeepSkyBlue2", 0x00b2ee),
        (c"DeepSkyBlue3", 0x009acd),
        (c"DeepSkyBlue4", 0x00688b),
        (c"DimGray", 0x696969),
        (c"DimGrey", 0x696969),
        (c"DodgerBlue", 0x1e90ff),
        (c"DodgerBlue1", 0x1e90ff),
        (c"DodgerBlue2", 0x1c86ee),
        (c"DodgerBlue3", 0x1874cd),
        (c"DodgerBlue4", 0x104e8b),
        (c"FloralWhite", 0xfffaf0),
        (c"ForestGreen", 0x228b22),
        (c"GhostWhite", 0xf8f8ff),
        (c"GreenYellow", 0xadff2f),
        (c"HotPink", 0xff69b4),
        (c"HotPink1", 0xff6eb4),
        (c"HotPink2", 0xee6aa7),
        (c"HotPink3", 0xcd6090),
        (c"HotPink4", 0x8b3a62),
        (c"IndianRed", 0xcd5c5c),
        (c"IndianRed1", 0xff6a6a),
        (c"IndianRed2", 0xee6363),
        (c"IndianRed3", 0xcd5555),
        (c"IndianRed4", 0x8b3a3a),
        (c"LavenderBlush", 0xfff0f5),
        (c"LavenderBlush1", 0xfff0f5),
        (c"LavenderBlush2", 0xeee0e5),
        (c"LavenderBlush3", 0xcdc1c5),
        (c"LavenderBlush4", 0x8b8386),
        (c"LawnGreen", 0x7cfc00),
        (c"LemonChiffon", 0xfffacd),
        (c"LemonChiffon1", 0xfffacd),
        (c"LemonChiffon2", 0xeee9bf),
        (c"LemonChiffon3", 0xcdc9a5),
        (c"LemonChiffon4", 0x8b8970),
        (c"LightBlue", 0xadd8e6),
        (c"LightBlue1", 0xbfefff),
        (c"LightBlue2", 0xb2dfee),
        (c"LightBlue3", 0x9ac0cd),
        (c"LightBlue4", 0x68838b),
        (c"LightCoral", 0xf08080),
        (c"LightCyan", 0xe0ffff),
        (c"LightCyan1", 0xe0ffff),
        (c"LightCyan2", 0xd1eeee),
        (c"LightCyan3", 0xb4cdcd),
        (c"LightCyan4", 0x7a8b8b),
        (c"LightGoldenrod", 0xeedd82),
        (c"LightGoldenrod1", 0xffec8b),
        (c"LightGoldenrod2", 0xeedc82),
        (c"LightGoldenrod3", 0xcdbe70),
        (c"LightGoldenrod4", 0x8b814c),
        (c"LightGoldenrodYellow", 0xfafad2),
        (c"LightGray", 0xd3d3d3),
        (c"LightGreen", 0x90ee90),
        (c"LightGrey", 0xd3d3d3),
        (c"LightPink", 0xffb6c1),
        (c"LightPink1", 0xffaeb9),
        (c"LightPink2", 0xeea2ad),
        (c"LightPink3", 0xcd8c95),
        (c"LightPink4", 0x8b5f65),
        (c"LightSalmon", 0xffa07a),
        (c"LightSalmon1", 0xffa07a),
        (c"LightSalmon2", 0xee9572),
        (c"LightSalmon3", 0xcd8162),
        (c"LightSalmon4", 0x8b5742),
        (c"LightSeaGreen", 0x20b2aa),
        (c"LightSkyBlue", 0x87cefa),
        (c"LightSkyBlue1", 0xb0e2ff),
        (c"LightSkyBlue2", 0xa4d3ee),
        (c"LightSkyBlue3", 0x8db6cd),
        (c"LightSkyBlue4", 0x607b8b),
        (c"LightSlateBlue", 0x8470ff),
        (c"LightSlateGray", 0x778899),
        (c"LightSlateGrey", 0x778899),
        (c"LightSteelBlue", 0xb0c4de),
        (c"LightSteelBlue1", 0xcae1ff),
        (c"LightSteelBlue2", 0xbcd2ee),
        (c"LightSteelBlue3", 0xa2b5cd),
        (c"LightSteelBlue4", 0x6e7b8b),
        (c"LightYellow", 0xffffe0),
        (c"LightYellow1", 0xffffe0),
        (c"LightYellow2", 0xeeeed1),
        (c"LightYellow3", 0xcdcdb4),
        (c"LightYellow4", 0x8b8b7a),
        (c"LimeGreen", 0x32cd32),
        (c"MediumAquamarine", 0x66cdaa),
        (c"MediumBlue", 0x0000cd),
        (c"MediumOrchid", 0xba55d3),
        (c"MediumOrchid1", 0xe066ff),
        (c"MediumOrchid2", 0xd15fee),
        (c"MediumOrchid3", 0xb452cd),
        (c"MediumOrchid4", 0x7a378b),
        (c"MediumPurple", 0x9370db),
        (c"MediumPurple1", 0xab82ff),
        (c"MediumPurple2", 0x9f79ee),
        (c"MediumPurple3", 0x8968cd),
        (c"MediumPurple4", 0x5d478b),
        (c"MediumSeaGreen", 0x3cb371),
        (c"MediumSlateBlue", 0x7b68ee),
        (c"MediumSpringGreen", 0x00fa9a),
        (c"MediumTurquoise", 0x48d1cc),
        (c"MediumVioletRed", 0xc71585),
        (c"MidnightBlue", 0x191970),
        (c"MintCream", 0xf5fffa),
        (c"MistyRose", 0xffe4e1),
        (c"MistyRose1", 0xffe4e1),
        (c"MistyRose2", 0xeed5d2),
        (c"MistyRose3", 0xcdb7b5),
        (c"MistyRose4", 0x8b7d7b),
        (c"NavajoWhite", 0xffdead),
        (c"NavajoWhite1", 0xffdead),
        (c"NavajoWhite2", 0xeecfa1),
        (c"NavajoWhite3", 0xcdb38b),
        (c"NavajoWhite4", 0x8b795e),
        (c"NavyBlue", 0x000080),
        (c"OldLace", 0xfdf5e6),
        (c"OliveDrab", 0x6b8e23),
        (c"OliveDrab1", 0xc0ff3e),
        (c"OliveDrab2", 0xb3ee3a),
        (c"OliveDrab3", 0x9acd32),
        (c"OliveDrab4", 0x698b22),
        (c"OrangeRed", 0xff4500),
        (c"OrangeRed1", 0xff4500),
        (c"OrangeRed2", 0xee4000),
        (c"OrangeRed3", 0xcd3700),
        (c"OrangeRed4", 0x8b2500),
        (c"PaleGoldenrod", 0xeee8aa),
        (c"PaleGreen", 0x98fb98),
        (c"PaleGreen1", 0x9aff9a),
        (c"PaleGreen2", 0x90ee90),
        (c"PaleGreen3", 0x7ccd7c),
        (c"PaleGreen4", 0x548b54),
        (c"PaleTurquoise", 0xafeeee),
        (c"PaleTurquoise1", 0xbbffff),
        (c"PaleTurquoise2", 0xaeeeee),
        (c"PaleTurquoise3", 0x96cdcd),
        (c"PaleTurquoise4", 0x668b8b),
        (c"PaleVioletRed", 0xdb7093),
        (c"PaleVioletRed1", 0xff82ab),
        (c"PaleVioletRed2", 0xee799f),
        (c"PaleVioletRed3", 0xcd6889),
        (c"PaleVioletRed4", 0x8b475d),
        (c"PapayaWhip", 0xffefd5),
        (c"PeachPuff", 0xffdab9),
        (c"PeachPuff1", 0xffdab9),
        (c"PeachPuff2", 0xeecbad),
        (c"PeachPuff3", 0xcdaf95),
        (c"PeachPuff4", 0x8b7765),
        (c"PowderBlue", 0xb0e0e6),
        (c"RebeccaPurple", 0x663399),
        (c"RosyBrown", 0xbc8f8f),
        (c"RosyBrown1", 0xffc1c1),
        (c"RosyBrown2", 0xeeb4b4),
        (c"RosyBrown3", 0xcd9b9b),
        (c"RosyBrown4", 0x8b6969),
        (c"RoyalBlue", 0x4169e1),
        (c"RoyalBlue1", 0x4876ff),
        (c"RoyalBlue2", 0x436eee),
        (c"RoyalBlue3", 0x3a5fcd),
        (c"RoyalBlue4", 0x27408b),
        (c"SaddleBrown", 0x8b4513),
        (c"SandyBrown", 0xf4a460),
        (c"SeaGreen", 0x2e8b57),
        (c"SeaGreen1", 0x54ff9f),
        (c"SeaGreen2", 0x4eee94),
        (c"SeaGreen3", 0x43cd80),
        (c"SeaGreen4", 0x2e8b57),
        (c"SkyBlue", 0x87ceeb),
        (c"SkyBlue1", 0x87ceff),
        (c"SkyBlue2", 0x7ec0ee),
        (c"SkyBlue3", 0x6ca6cd),
        (c"SkyBlue4", 0x4a708b),
        (c"SlateBlue", 0x6a5acd),
        (c"SlateBlue1", 0x836fff),
        (c"SlateBlue2", 0x7a67ee),
        (c"SlateBlue3", 0x6959cd),
        (c"SlateBlue4", 0x473c8b),
        (c"SlateGray", 0x708090),
        (c"SlateGray1", 0xc6e2ff),
        (c"SlateGray2", 0xb9d3ee),
        (c"SlateGray3", 0x9fb6cd),
        (c"SlateGray4", 0x6c7b8b),
        (c"SlateGrey", 0x708090),
        (c"SpringGreen", 0x00ff7f),
        (c"SpringGreen1", 0x00ff7f),
        (c"SpringGreen2", 0x00ee76),
        (c"SpringGreen3", 0x00cd66),
        (c"SpringGreen4", 0x008b45),
        (c"SteelBlue", 0x4682b4),
        (c"SteelBlue1", 0x63b8ff),
        (c"SteelBlue2", 0x5cacee),
        (c"SteelBlue3", 0x4f94cd),
        (c"SteelBlue4", 0x36648b),
        (c"VioletRed", 0xd02090),
        (c"VioletRed1", 0xff3e96),
        (c"VioletRed2", 0xee3a8c),
        (c"VioletRed3", 0xcd3278),
        (c"VioletRed4", 0x8b2252),
        (c"WebGray", 0x808080),
        (c"WebGreen", 0x008000),
        (c"WebGrey", 0x808080),
        (c"WebMaroon", 0x800000),
        (c"WebPurple", 0x800080),
        (c"WhiteSmoke", 0xf5f5f5),
        (c"X11Gray", 0xbebebe),
        (c"X11Green", 0x00ff00),
        (c"X11Grey", 0xbebebe),
        (c"X11Maroon", 0xb03060),
        (c"X11Purple", 0xa020f0),
        (c"YellowGreen", 0x9acd32),
        (c"alice blue", 0xf0f8ff),
        (c"antique white", 0xfaebd7),
        (c"aqua", 0x00ffff),
        (c"aquamarine", 0x7fffd4),
        (c"aquamarine1", 0x7fffd4),
        (c"aquamarine2", 0x76eec6),
        (c"aquamarine3", 0x66cdaa),
        (c"aquamarine4", 0x458b74),
        (c"azure", 0xf0ffff),
        (c"azure1", 0xf0ffff),
        (c"azure2", 0xe0eeee),
        (c"azure3", 0xc1cdcd),
        (c"azure4", 0x838b8b),
        (c"beige", 0xf5f5dc),
        (c"bisque", 0xffe4c4),
        (c"bisque1", 0xffe4c4),
        (c"bisque2", 0xeed5b7),
        (c"bisque3", 0xcdb79e),
        (c"bisque4", 0x8b7d6b),
        (c"black", 0x000000),
        (c"blanched almond", 0xffebcd),
        (c"blue violet", 0x8a2be2),
        (c"blue", 0x0000ff),
        (c"blue1", 0x0000ff),
        (c"blue2", 0x0000ee),
        (c"blue3", 0x0000cd),
        (c"blue4", 0x00008b),
        (c"brown", 0xa52a2a),
        (c"brown1", 0xff4040),
        (c"brown2", 0xee3b3b),
        (c"brown3", 0xcd3333),
        (c"brown4", 0x8b2323),
        (c"burlywood", 0xdeb887),
        (c"burlywood1", 0xffd39b),
        (c"burlywood2", 0xeec591),
        (c"burlywood3", 0xcdaa7d),
        (c"burlywood4", 0x8b7355),
        (c"cadet blue", 0x5f9ea0),
        (c"chartreuse", 0x7fff00),
        (c"chartreuse1", 0x7fff00),
        (c"chartreuse2", 0x76ee00),
        (c"chartreuse3", 0x66cd00),
        (c"chartreuse4", 0x458b00),
        (c"chocolate", 0xd2691e),
        (c"chocolate1", 0xff7f24),
        (c"chocolate2", 0xee7621),
        (c"chocolate3", 0xcd661d),
        (c"chocolate4", 0x8b4513),
        (c"coral", 0xff7f50),
        (c"coral1", 0xff7256),
        (c"coral2", 0xee6a50),
        (c"coral3", 0xcd5b45),
        (c"coral4", 0x8b3e2f),
        (c"cornflower blue", 0x6495ed),
        (c"cornsilk", 0xfff8dc),
        (c"cornsilk1", 0xfff8dc),
        (c"cornsilk2", 0xeee8cd),
        (c"cornsilk3", 0xcdc8b1),
        (c"cornsilk4", 0x8b8878),
        (c"crimson", 0xdc143c),
        (c"cyan", 0x00ffff),
        (c"cyan1", 0x00ffff),
        (c"cyan2", 0x00eeee),
        (c"cyan3", 0x00cdcd),
        (c"cyan4", 0x008b8b),
        (c"dark blue", 0x00008b),
        (c"dark cyan", 0x008b8b),
        (c"dark goldenrod", 0xb8860b),
        (c"dark gray", 0xa9a9a9),
        (c"dark green", 0x006400),
        (c"dark grey", 0xa9a9a9),
        (c"dark khaki", 0xbdb76b),
        (c"dark magenta", 0x8b008b),
        (c"dark olive green", 0x556b2f),
        (c"dark orange", 0xff8c00),
        (c"dark orchid", 0x9932cc),
        (c"dark red", 0x8b0000),
        (c"dark salmon", 0xe9967a),
        (c"dark sea green", 0x8fbc8f),
        (c"dark slate blue", 0x483d8b),
        (c"dark slate gray", 0x2f4f4f),
        (c"dark slate grey", 0x2f4f4f),
        (c"dark turquoise", 0x00ced1),
        (c"dark violet", 0x9400d3),
        (c"deep pink", 0xff1493),
        (c"deep sky blue", 0x00bfff),
        (c"dim gray", 0x696969),
        (c"dim grey", 0x696969),
        (c"dodger blue", 0x1e90ff),
        (c"firebrick", 0xb22222),
        (c"firebrick1", 0xff3030),
        (c"firebrick2", 0xee2c2c),
        (c"firebrick3", 0xcd2626),
        (c"firebrick4", 0x8b1a1a),
        (c"floral white", 0xfffaf0),
        (c"forest green", 0x228b22),
        (c"fuchsia", 0xff00ff),
        (c"gainsboro", 0xdcdcdc),
        (c"ghost white", 0xf8f8ff),
        (c"gold", 0xffd700),
        (c"gold1", 0xffd700),
        (c"gold2", 0xeec900),
        (c"gold3", 0xcdad00),
        (c"gold4", 0x8b7500),
        (c"goldenrod", 0xdaa520),
        (c"goldenrod1", 0xffc125),
        (c"goldenrod2", 0xeeb422),
        (c"goldenrod3", 0xcd9b1d),
        (c"goldenrod4", 0x8b6914),
        (c"green yellow", 0xadff2f),
        (c"green", 0x00ff00),
        (c"green1", 0x00ff00),
        (c"green2", 0x00ee00),
        (c"green3", 0x00cd00),
        (c"green4", 0x008b00),
        (c"honeydew", 0xf0fff0),
        (c"honeydew1", 0xf0fff0),
        (c"honeydew2", 0xe0eee0),
        (c"honeydew3", 0xc1cdc1),
        (c"honeydew4", 0x838b83),
        (c"hot pink", 0xff69b4),
        (c"indian red", 0xcd5c5c),
        (c"indigo", 0x4b0082),
        (c"ivory", 0xfffff0),
        (c"ivory1", 0xfffff0),
        (c"ivory2", 0xeeeee0),
        (c"ivory3", 0xcdcdc1),
        (c"ivory4", 0x8b8b83),
        (c"khaki", 0xf0e68c),
        (c"khaki1", 0xfff68f),
        (c"khaki2", 0xeee685),
        (c"khaki3", 0xcdc673),
        (c"khaki4", 0x8b864e),
        (c"lavender blush", 0xfff0f5),
        (c"lavender", 0xe6e6fa),
        (c"lawn green", 0x7cfc00),
        (c"lemon chiffon", 0xfffacd),
        (c"light blue", 0xadd8e6),
        (c"light coral", 0xf08080),
        (c"light cyan", 0xe0ffff),
        (c"light goldenrod yellow", 0xfafad2),
        (c"light goldenrod", 0xeedd82),
        (c"light gray", 0xd3d3d3),
        (c"light green", 0x90ee90),
        (c"light grey", 0xd3d3d3),
        (c"light pink", 0xffb6c1),
        (c"light salmon", 0xffa07a),
        (c"light sea green", 0x20b2aa),
        (c"light sky blue", 0x87cefa),
        (c"light slate blue", 0x8470ff),
        (c"light slate gray", 0x778899),
        (c"light slate grey", 0x778899),
        (c"light steel blue", 0xb0c4de),
        (c"light yellow", 0xffffe0),
        (c"lime green", 0x32cd32),
        (c"lime", 0x00ff00),
        (c"linen", 0xfaf0e6),
        (c"magenta", 0xff00ff),
        (c"magenta1", 0xff00ff),
        (c"magenta2", 0xee00ee),
        (c"magenta3", 0xcd00cd),
        (c"magenta4", 0x8b008b),
        (c"maroon", 0xb03060),
        (c"maroon1", 0xff34b3),
        (c"maroon2", 0xee30a7),
        (c"maroon3", 0xcd2990),
        (c"maroon4", 0x8b1c62),
        (c"medium aquamarine", 0x66cdaa),
        (c"medium blue", 0x0000cd),
        (c"medium orchid", 0xba55d3),
        (c"medium purple", 0x9370db),
        (c"medium sea green", 0x3cb371),
        (c"medium slate blue", 0x7b68ee),
        (c"medium spring green", 0x00fa9a),
        (c"medium turquoise", 0x48d1cc),
        (c"medium violet red", 0xc71585),
        (c"midnight blue", 0x191970),
        (c"mint cream", 0xf5fffa),
        (c"misty rose", 0xffe4e1),
        (c"moccasin", 0xffe4b5),
        (c"navajo white", 0xffdead),
        (c"navy blue", 0x000080),
        (c"navy", 0x000080),
        (c"old lace", 0xfdf5e6),
        (c"olive drab", 0x6b8e23),
        (c"olive", 0x808000),
        (c"orange red", 0xff4500),
        (c"orange", 0xffa500),
        (c"orange1", 0xffa500),
        (c"orange2", 0xee9a00),
        (c"orange3", 0xcd8500),
        (c"orange4", 0x8b5a00),
        (c"orchid", 0xda70d6),
        (c"orchid1", 0xff83fa),
        (c"orchid2", 0xee7ae9),
        (c"orchid3", 0xcd69c9),
        (c"orchid4", 0x8b4789),
        (c"pale goldenrod", 0xeee8aa),
        (c"pale green", 0x98fb98),
        (c"pale turquoise", 0xafeeee),
        (c"pale violet red", 0xdb7093),
        (c"papaya whip", 0xffefd5),
        (c"peach puff", 0xffdab9),
        (c"peru", 0xcd853f),
        (c"pink", 0xffc0cb),
        (c"pink1", 0xffb5c5),
        (c"pink2", 0xeea9b8),
        (c"pink3", 0xcd919e),
        (c"pink4", 0x8b636c),
        (c"plum", 0xdda0dd),
        (c"plum1", 0xffbbff),
        (c"plum2", 0xeeaeee),
        (c"plum3", 0xcd96cd),
        (c"plum4", 0x8b668b),
        (c"powder blue", 0xb0e0e6),
        (c"purple", 0xa020f0),
        (c"purple1", 0x9b30ff),
        (c"purple2", 0x912cee),
        (c"purple3", 0x7d26cd),
        (c"purple4", 0x551a8b),
        (c"rebecca purple", 0x663399),
        (c"red", 0xff0000),
        (c"red1", 0xff0000),
        (c"red2", 0xee0000),
        (c"red3", 0xcd0000),
        (c"red4", 0x8b0000),
        (c"rosy brown", 0xbc8f8f),
        (c"royal blue", 0x4169e1),
        (c"saddle brown", 0x8b4513),
        (c"salmon", 0xfa8072),
        (c"salmon1", 0xff8c69),
        (c"salmon2", 0xee8262),
        (c"salmon3", 0xcd7054),
        (c"salmon4", 0x8b4c39),
        (c"sandy brown", 0xf4a460),
        (c"sea green", 0x2e8b57),
        (c"seashell", 0xfff5ee),
        (c"seashell1", 0xfff5ee),
        (c"seashell2", 0xeee5de),
        (c"seashell3", 0xcdc5bf),
        (c"seashell4", 0x8b8682),
        (c"sienna", 0xa0522d),
        (c"sienna1", 0xff8247),
        (c"sienna2", 0xee7942),
        (c"sienna3", 0xcd6839),
        (c"sienna4", 0x8b4726),
        (c"silver", 0xc0c0c0),
        (c"sky blue", 0x87ceeb),
        (c"slate blue", 0x6a5acd),
        (c"slate gray", 0x708090),
        (c"slate grey", 0x708090),
        (c"snow", 0xfffafa),
        (c"snow1", 0xfffafa),
        (c"snow2", 0xeee9e9),
        (c"snow3", 0xcdc9c9),
        (c"snow4", 0x8b8989),
        (c"spring green", 0x00ff7f),
        (c"steel blue", 0x4682b4),
        (c"tan", 0xd2b48c),
        (c"tan1", 0xffa54f),
        (c"tan2", 0xee9a49),
        (c"tan3", 0xcd853f),
        (c"tan4", 0x8b5a2b),
        (c"teal", 0x008080),
        (c"thistle", 0xd8bfd8),
        (c"thistle1", 0xffe1ff),
        (c"thistle2", 0xeed2ee),
        (c"thistle3", 0xcdb5cd),
        (c"thistle4", 0x8b7b8b),
        (c"tomato", 0xff6347),
        (c"tomato1", 0xff6347),
        (c"tomato2", 0xee5c42),
        (c"tomato3", 0xcd4f39),
        (c"tomato4", 0x8b3626),
        (c"turquoise", 0x40e0d0),
        (c"turquoise1", 0x00f5ff),
        (c"turquoise2", 0x00e5ee),
        (c"turquoise3", 0x00c5cd),
        (c"turquoise4", 0x00868b),
        (c"violet red", 0xd02090),
        (c"violet", 0xee82ee),
        (c"web gray", 0x808080),
        (c"web green", 0x008000),
        (c"web grey", 0x808080),
        (c"web maroon", 0x800000),
        (c"web purple", 0x800080),
        (c"wheat", 0xf5deb3),
        (c"wheat1", 0xffe7ba),
        (c"wheat2", 0xeed8ae),
        (c"wheat3", 0xcdba96),
        (c"wheat4", 0x8b7e66),
        (c"white smoke", 0xf5f5f5),
        (c"white", 0xffffff),
        (c"x11 gray", 0xbebebe),
        (c"x11 green", 0x00ff00),
        (c"x11 grey", 0xbebebe),
        (c"x11 maroon", 0xb03060),
        (c"x11 purple", 0xa020f0),
        (c"yellow green", 0x9acd32),
        (c"yellow", 0xffff00),
        (c"yellow1", 0xffff00),
        (c"yellow2", 0xeeee00),
        (c"yellow3", 0xcdcd00),
        (c"yellow4", 0x8b8b00),
    ];

    unsafe {
        if strncmp(name, c!("grey"), 4) == 0 || strncmp(name, c!("gray"), 4) == 0 {
            if *name.add(4) == 0 {
                return -1;
            }

            let mut errstr: *const u8 = null();
            let Ok(c) = strtonum(name.add(4), 0, 100) else {
                return -1;
            };
            let c = (2.55f32 * (c as f32)).round() as i32;

            if !(0..=255).contains(&c) {
                return -1;
            }

            let c = c as u8;
            return colour_join_rgb(c, c, c);
        }

        for (color_name, color_hex) in &COLOURS {
            if strcasecmp(color_name.as_ptr().cast(), name) == 0 {
                return color_hex | COLOUR_FLAG_RGB;
            }
        }
    }

    -1
}

pub unsafe fn colour_palette_init(p: *mut colour_palette) {
    unsafe {
        *p = colour_palette {
            fg: 8,
            bg: 8,
            palette: null_mut(),
            default_palette: null_mut(),
        };
    }
}

/// Clear palette.
pub unsafe fn colour_palette_clear(p: *mut colour_palette) {
    unsafe {
        if !p.is_null() {
            (*p).fg = 8;
            (*p).bg = 8;
            free((*p).palette as _);
            (*p).palette = null_mut();
        }
    }
}

/// Free a palette
pub unsafe fn colour_palette_free(p: *mut colour_palette) {
    if let Some(p) = std::ptr::NonNull::new(p) {
        let p = p.as_ptr();
        unsafe {
            free((*p).palette as _);
            (*p).palette = null_mut();
            free((*p).default_palette as _);
            (*p).default_palette = null_mut();
        }
    }
}

/// Get a colour from a palette.
pub unsafe fn colour_palette_get(p: *const colour_palette, mut c: i32) -> i32 {
    unsafe {
        if p.is_null() {
            return -1;
        } else if (90..=97).contains(&c) {
            c = 8 + c - 90;
        } else if c & COLOUR_FLAG_256 != 0 {
            c &= !COLOUR_FLAG_256;
        } else if c >= 8 {
            return -1;
        }

        let c = c as usize;

        if !(*p).palette.is_null() && *(*p).palette.add(c) != -1 {
            *(*p).palette.add(c)
        } else if !(*p).default_palette.is_null() && *(*p).default_palette.add(c) != -1 {
            *(*p).default_palette.add(c)
        } else {
            -1
        }
    }
}

pub unsafe fn colour_palette_set(p: *mut colour_palette, n: i32, c: i32) -> i32 {
    unsafe {
        if p.is_null() || n > 255 {
            return 0;
        }

        if c == -1 && (*p).palette.is_null() {
            return 0;
        }

        if c != -1 && (*p).palette.is_null() {
            if (*p).palette.is_null() {
                (*p).palette = xcalloc_(256).as_ptr();
            }
            for i in 0..256 {
                *(*p).palette.add(i) = -1;
            }
        }
        *(*p).palette.add(n as usize) = c;

        1
    }
}

pub unsafe fn colour_palette_from_option(p: *mut colour_palette, oo: *mut options) {
    unsafe {
        if p.is_null() {
            return;
        }

        let o = options_get(oo, c!("pane-colours"));

        let mut a = options_array_first(o);
        if a.is_null() {
            if !(*p).default_palette.is_null() {
                free((*p).default_palette as _);
                (*p).default_palette = null_mut();
            }
            return;
        }

        if (*p).default_palette.is_null() {
            (*p).default_palette = xcalloc_::<c_int>(256).as_ptr();
        }
        for i in 0..256 {
            *(*p).default_palette.add(i) = -1;
        }

        while !a.is_null() {
            let n = options_array_item_index(a);
            if n < 256 {
                let c = (*options_array_item_value(a)).number as i32;
                *(*p).default_palette.add(n as usize) = c;
            }
            a = options_array_next(a);
        }
    }
}

// below has the auto generated code I haven't bothered to translate yet
pub unsafe fn colour_parse_x11(mut p: *const u8) -> c_int {
    unsafe {
        let mut c: f64 = 0.0;
        let mut m: f64 = 0.0;
        let mut y: f64 = 0.0;
        let mut k: f64 = 0.0;

        let mut r: u32 = 0;
        let mut g: u32 = 0;
        let mut b: u32 = 0;

        let mut len = strlen(p);
        let mut colour: i32 = -1;
        let mut copy: *mut u8 = null_mut();
        if len == 12
            && sscanf(
                p.cast(),
                c"rgb:%02x/%02x/%02x".as_ptr(),
                &raw mut r,
                &raw mut g,
                &raw mut b,
            ) == 3
            || len == 7
                && sscanf(
                    p.cast(),
                    c"#%02x%02x%02x".as_ptr(),
                    &raw mut r,
                    &raw mut g,
                    &raw mut b,
                ) == 3
            || sscanf(
                p.cast(),
                c"%d,%d,%d.as_ptr()".as_ptr(),
                &raw mut r,
                &raw mut g,
                &raw mut b,
            ) == 3
        {
            colour = colour_join_rgb(r as u8, g as u8, b as u8);
        } else if len == 18
            && sscanf(
                p.cast(),
                c"rgb:%04x/%04x/%04x".as_ptr(),
                &raw mut r,
                &raw mut g,
                &raw mut b,
            ) == 3 as c_int
            || len == 13
                && sscanf(
                    p.cast(),
                    c"#%04x%04x%04x".as_ptr(),
                    &raw mut r,
                    &raw mut g,
                    &raw mut b,
                ) == 3 as c_int
        {
            colour = colour_join_rgb(
                (r >> 8 as c_int) as c_uchar,
                (g >> 8 as c_int) as c_uchar,
                (b >> 8 as c_int) as c_uchar,
            );
        } else if (sscanf(
            p.cast(),
            c"cmyk:%lf/%lf/%lf/%lf".as_ptr(),
            &raw mut c,
            &raw mut m,
            &raw mut y,
            &raw mut k,
        ) == 4
            || sscanf(
                p.cast(),
                c"cmy:%lf/%lf/%lf".as_ptr(),
                &raw mut c,
                &raw mut m,
                &raw mut y,
            ) == 3 as c_int)
            && c >= 0.0
            && c <= 1.0
            && m >= 0.0
            && m <= 1.0
            && y >= 0.0
            && y <= 1.0
            && k >= 0.0
            && k <= 1.0
        {
            colour = colour_join_rgb(
                ((1f64 - c) * (1f64 - k) * 255f64) as u8,
                ((1f64 - m) * (1f64 - k) * 255f64) as u8,
                ((1f64 - y) * (1f64 - k) * 255f64) as u8,
            );
        } else {
            while len != 0 && *p == b' ' {
                p = p.offset(1);
                len = len.wrapping_sub(1);
            }
            while len != 0 && *p.add(len - 1) == b' ' {
                len = len.wrapping_sub(1);
            }
            copy = xstrndup(p, len).cast().as_ptr();
            colour = colour_byname(copy);
            free(copy as _);
        }
        log_debug!(
            "{}: {} = {}",
            "colour_parseX11",
            _s(p),
            _s(colour_tostring(colour))
        );
        colour
    }
}

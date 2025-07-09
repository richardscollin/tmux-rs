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

    const Q2C: [i32; 6] = [0x00, 0x5f, 0x87, 0xaf, 0xd7, 0xff];

    // Map RGB to 6x6x6 cube.
    let qr = colour_to_6cube(r);
    let qg = colour_to_6cube(g);
    let qb = colour_to_6cube(b);
    let cr = Q2C[qr as usize];
    let cg = Q2C[qg as usize];
    let cb = Q2C[qb as usize];

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
    static mut BUF: [u8; 32] = [0; 32];

    if c == -1 {
        return c!("none");
    }

    if c & COLOUR_FLAG_RGB != 0 {
        let (r, g, b) = colour_split_rgb(c);
        write!(unsafe { BUF.as_mut_slice() }, "#{r:02x}{g:02x}{b:02x}\0").unwrap();
        return &raw const BUF as _;
    }

    if c & COLOUR_FLAG_256 != 0 {
        write!(unsafe { BUF.as_mut_slice() }, "colour{}\0", c & 0xff).unwrap();
        return &raw const BUF as _;
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
pub unsafe fn colour_fromstring_(s: &str) -> c_int {
    let orig = s;
    unsafe {
        if s.chars().next().is_some_and(|c| c == '#') && s.len() == 7 {
            let cp = s.trim_start_matches(|c: char| c.is_ascii_hexdigit());
            if cp.is_empty() {
                return -1;
            }

            if s.len() < 7 {
                return -1;
            }
            if let Ok(r) = u8::from_str_radix(&s[1..3], 16)
                && let Ok(g) = u8::from_str_radix(&s[3..5], 16)
                && let Ok(b) = u8::from_str_radix(&s[5..6], 16)
            {
                return colour_join_rgb(r, g, b);
            } else {
                return -1;
            }
        }

        if s.eq_ignore_ascii_case("colour") {
            if s.len() == 6 {
                return -1;
            }
            let Ok(n) = strtonum_(&s[6..], 0i32, 255) else {
                return -1;
            };
            return n | COLOUR_FLAG_256;
        }

        if s.eq_ignore_ascii_case("color") {
            if s.len() == 5 {
                return -1;
            }
            let Ok(n) = strtonum_(&s[5..], 0i32, 255) else {
                return -1;
            };
            return n | COLOUR_FLAG_256;
        }

        if s.eq_ignore_ascii_case("default") {
            8
        } else if s.eq_ignore_ascii_case("terminal") {
            9
        } else if s.eq_ignore_ascii_case("black") || s == "0" {
            0
        } else if s.eq_ignore_ascii_case("red") || s == "1" {
            1
        } else if s.eq_ignore_ascii_case("green") || s == "2" {
            2
        } else if s.eq_ignore_ascii_case("yellow") || s == "3" {
            3
        } else if s.eq_ignore_ascii_case("blue") || s == "4" {
            4
        } else if s.eq_ignore_ascii_case("magenta") || s == "5" {
            5
        } else if s.eq_ignore_ascii_case("cyan") || s == "6" {
            6
        } else if s.eq_ignore_ascii_case("white") || s == "7" {
            7
        } else if s.eq_ignore_ascii_case("brightblack") || s == "90" {
            90
        } else if s.eq_ignore_ascii_case("brightred") || s == "91" {
            91
        } else if s.eq_ignore_ascii_case("brightgreen") || s == "92" {
            92
        } else if s.eq_ignore_ascii_case("brightyellow") || s == "93" {
            93
        } else if s.eq_ignore_ascii_case("brightblue") || s == "94" {
            94
        } else if s.eq_ignore_ascii_case("brightmagenta") || s == "95" {
            95
        } else if s.eq_ignore_ascii_case("brightcyan") || s == "96" {
            96
        } else if s.eq_ignore_ascii_case("brightwhite") || s == "97" {
            97
        } else {
            colour_byname(s)
        }
    }
}

/// Convert colour from string.
pub unsafe fn colour_fromstring(s: *const u8) -> c_int {
    unsafe { colour_fromstring_(cstr_to_str(s)) }
}

/// Convert 256 colour to RGB colour.
fn colour_256_to_rgb(c: i32) -> i32 {
    const TABLE: [i32; 256] = [
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

    TABLE[c as u8 as usize] | COLOUR_FLAG_RGB
}

pub fn colour_256to16(c: i32) -> i32 {
    const TABLE: [u8; 256] = [
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
    TABLE[c as u8 as usize] as i32
}

pub unsafe fn colour_byname(name: &str) -> i32 {
    const COLOURS: [(&str, i32); 578] = [
        ("AliceBlue", 0xf0f8ff),
        ("AntiqueWhite", 0xfaebd7),
        ("AntiqueWhite1", 0xffefdb),
        ("AntiqueWhite2", 0xeedfcc),
        ("AntiqueWhite3", 0xcdc0b0),
        ("AntiqueWhite4", 0x8b8378),
        ("BlanchedAlmond", 0xffebcd),
        ("BlueViolet", 0x8a2be2),
        ("CadetBlue", 0x5f9ea0),
        ("CadetBlue1", 0x98f5ff),
        ("CadetBlue2", 0x8ee5ee),
        ("CadetBlue3", 0x7ac5cd),
        ("CadetBlue4", 0x53868b),
        ("CornflowerBlue", 0x6495ed),
        ("DarkBlue", 0x00008b),
        ("DarkCyan", 0x008b8b),
        ("DarkGoldenrod", 0xb8860b),
        ("DarkGoldenrod1", 0xffb90f),
        ("DarkGoldenrod2", 0xeead0e),
        ("DarkGoldenrod3", 0xcd950c),
        ("DarkGoldenrod4", 0x8b6508),
        ("DarkGray", 0xa9a9a9),
        ("DarkGreen", 0x006400),
        ("DarkGrey", 0xa9a9a9),
        ("DarkKhaki", 0xbdb76b),
        ("DarkMagenta", 0x8b008b),
        ("DarkOliveGreen", 0x556b2f),
        ("DarkOliveGreen1", 0xcaff70),
        ("DarkOliveGreen2", 0xbcee68),
        ("DarkOliveGreen3", 0xa2cd5a),
        ("DarkOliveGreen4", 0x6e8b3d),
        ("DarkOrange", 0xff8c00),
        ("DarkOrange1", 0xff7f00),
        ("DarkOrange2", 0xee7600),
        ("DarkOrange3", 0xcd6600),
        ("DarkOrange4", 0x8b4500),
        ("DarkOrchid", 0x9932cc),
        ("DarkOrchid1", 0xbf3eff),
        ("DarkOrchid2", 0xb23aee),
        ("DarkOrchid3", 0x9a32cd),
        ("DarkOrchid4", 0x68228b),
        ("DarkRed", 0x8b0000),
        ("DarkSalmon", 0xe9967a),
        ("DarkSeaGreen", 0x8fbc8f),
        ("DarkSeaGreen1", 0xc1ffc1),
        ("DarkSeaGreen2", 0xb4eeb4),
        ("DarkSeaGreen3", 0x9bcd9b),
        ("DarkSeaGreen4", 0x698b69),
        ("DarkSlateBlue", 0x483d8b),
        ("DarkSlateGray", 0x2f4f4f),
        ("DarkSlateGray1", 0x97ffff),
        ("DarkSlateGray2", 0x8deeee),
        ("DarkSlateGray3", 0x79cdcd),
        ("DarkSlateGray4", 0x528b8b),
        ("DarkSlateGrey", 0x2f4f4f),
        ("DarkTurquoise", 0x00ced1),
        ("DarkViolet", 0x9400d3),
        ("DeepPink", 0xff1493),
        ("DeepPink1", 0xff1493),
        ("DeepPink2", 0xee1289),
        ("DeepPink3", 0xcd1076),
        ("DeepPink4", 0x8b0a50),
        ("DeepSkyBlue", 0x00bfff),
        ("DeepSkyBlue1", 0x00bfff),
        ("DeepSkyBlue2", 0x00b2ee),
        ("DeepSkyBlue3", 0x009acd),
        ("DeepSkyBlue4", 0x00688b),
        ("DimGray", 0x696969),
        ("DimGrey", 0x696969),
        ("DodgerBlue", 0x1e90ff),
        ("DodgerBlue1", 0x1e90ff),
        ("DodgerBlue2", 0x1c86ee),
        ("DodgerBlue3", 0x1874cd),
        ("DodgerBlue4", 0x104e8b),
        ("FloralWhite", 0xfffaf0),
        ("ForestGreen", 0x228b22),
        ("GhostWhite", 0xf8f8ff),
        ("GreenYellow", 0xadff2f),
        ("HotPink", 0xff69b4),
        ("HotPink1", 0xff6eb4),
        ("HotPink2", 0xee6aa7),
        ("HotPink3", 0xcd6090),
        ("HotPink4", 0x8b3a62),
        ("IndianRed", 0xcd5c5c),
        ("IndianRed1", 0xff6a6a),
        ("IndianRed2", 0xee6363),
        ("IndianRed3", 0xcd5555),
        ("IndianRed4", 0x8b3a3a),
        ("LavenderBlush", 0xfff0f5),
        ("LavenderBlush1", 0xfff0f5),
        ("LavenderBlush2", 0xeee0e5),
        ("LavenderBlush3", 0xcdc1c5),
        ("LavenderBlush4", 0x8b8386),
        ("LawnGreen", 0x7cfc00),
        ("LemonChiffon", 0xfffacd),
        ("LemonChiffon1", 0xfffacd),
        ("LemonChiffon2", 0xeee9bf),
        ("LemonChiffon3", 0xcdc9a5),
        ("LemonChiffon4", 0x8b8970),
        ("LightBlue", 0xadd8e6),
        ("LightBlue1", 0xbfefff),
        ("LightBlue2", 0xb2dfee),
        ("LightBlue3", 0x9ac0cd),
        ("LightBlue4", 0x68838b),
        ("LightCoral", 0xf08080),
        ("LightCyan", 0xe0ffff),
        ("LightCyan1", 0xe0ffff),
        ("LightCyan2", 0xd1eeee),
        ("LightCyan3", 0xb4cdcd),
        ("LightCyan4", 0x7a8b8b),
        ("LightGoldenrod", 0xeedd82),
        ("LightGoldenrod1", 0xffec8b),
        ("LightGoldenrod2", 0xeedc82),
        ("LightGoldenrod3", 0xcdbe70),
        ("LightGoldenrod4", 0x8b814c),
        ("LightGoldenrodYellow", 0xfafad2),
        ("LightGray", 0xd3d3d3),
        ("LightGreen", 0x90ee90),
        ("LightGrey", 0xd3d3d3),
        ("LightPink", 0xffb6c1),
        ("LightPink1", 0xffaeb9),
        ("LightPink2", 0xeea2ad),
        ("LightPink3", 0xcd8c95),
        ("LightPink4", 0x8b5f65),
        ("LightSalmon", 0xffa07a),
        ("LightSalmon1", 0xffa07a),
        ("LightSalmon2", 0xee9572),
        ("LightSalmon3", 0xcd8162),
        ("LightSalmon4", 0x8b5742),
        ("LightSeaGreen", 0x20b2aa),
        ("LightSkyBlue", 0x87cefa),
        ("LightSkyBlue1", 0xb0e2ff),
        ("LightSkyBlue2", 0xa4d3ee),
        ("LightSkyBlue3", 0x8db6cd),
        ("LightSkyBlue4", 0x607b8b),
        ("LightSlateBlue", 0x8470ff),
        ("LightSlateGray", 0x778899),
        ("LightSlateGrey", 0x778899),
        ("LightSteelBlue", 0xb0c4de),
        ("LightSteelBlue1", 0xcae1ff),
        ("LightSteelBlue2", 0xbcd2ee),
        ("LightSteelBlue3", 0xa2b5cd),
        ("LightSteelBlue4", 0x6e7b8b),
        ("LightYellow", 0xffffe0),
        ("LightYellow1", 0xffffe0),
        ("LightYellow2", 0xeeeed1),
        ("LightYellow3", 0xcdcdb4),
        ("LightYellow4", 0x8b8b7a),
        ("LimeGreen", 0x32cd32),
        ("MediumAquamarine", 0x66cdaa),
        ("MediumBlue", 0x0000cd),
        ("MediumOrchid", 0xba55d3),
        ("MediumOrchid1", 0xe066ff),
        ("MediumOrchid2", 0xd15fee),
        ("MediumOrchid3", 0xb452cd),
        ("MediumOrchid4", 0x7a378b),
        ("MediumPurple", 0x9370db),
        ("MediumPurple1", 0xab82ff),
        ("MediumPurple2", 0x9f79ee),
        ("MediumPurple3", 0x8968cd),
        ("MediumPurple4", 0x5d478b),
        ("MediumSeaGreen", 0x3cb371),
        ("MediumSlateBlue", 0x7b68ee),
        ("MediumSpringGreen", 0x00fa9a),
        ("MediumTurquoise", 0x48d1cc),
        ("MediumVioletRed", 0xc71585),
        ("MidnightBlue", 0x191970),
        ("MintCream", 0xf5fffa),
        ("MistyRose", 0xffe4e1),
        ("MistyRose1", 0xffe4e1),
        ("MistyRose2", 0xeed5d2),
        ("MistyRose3", 0xcdb7b5),
        ("MistyRose4", 0x8b7d7b),
        ("NavajoWhite", 0xffdead),
        ("NavajoWhite1", 0xffdead),
        ("NavajoWhite2", 0xeecfa1),
        ("NavajoWhite3", 0xcdb38b),
        ("NavajoWhite4", 0x8b795e),
        ("NavyBlue", 0x000080),
        ("OldLace", 0xfdf5e6),
        ("OliveDrab", 0x6b8e23),
        ("OliveDrab1", 0xc0ff3e),
        ("OliveDrab2", 0xb3ee3a),
        ("OliveDrab3", 0x9acd32),
        ("OliveDrab4", 0x698b22),
        ("OrangeRed", 0xff4500),
        ("OrangeRed1", 0xff4500),
        ("OrangeRed2", 0xee4000),
        ("OrangeRed3", 0xcd3700),
        ("OrangeRed4", 0x8b2500),
        ("PaleGoldenrod", 0xeee8aa),
        ("PaleGreen", 0x98fb98),
        ("PaleGreen1", 0x9aff9a),
        ("PaleGreen2", 0x90ee90),
        ("PaleGreen3", 0x7ccd7c),
        ("PaleGreen4", 0x548b54),
        ("PaleTurquoise", 0xafeeee),
        ("PaleTurquoise1", 0xbbffff),
        ("PaleTurquoise2", 0xaeeeee),
        ("PaleTurquoise3", 0x96cdcd),
        ("PaleTurquoise4", 0x668b8b),
        ("PaleVioletRed", 0xdb7093),
        ("PaleVioletRed1", 0xff82ab),
        ("PaleVioletRed2", 0xee799f),
        ("PaleVioletRed3", 0xcd6889),
        ("PaleVioletRed4", 0x8b475d),
        ("PapayaWhip", 0xffefd5),
        ("PeachPuff", 0xffdab9),
        ("PeachPuff1", 0xffdab9),
        ("PeachPuff2", 0xeecbad),
        ("PeachPuff3", 0xcdaf95),
        ("PeachPuff4", 0x8b7765),
        ("PowderBlue", 0xb0e0e6),
        ("RebeccaPurple", 0x663399),
        ("RosyBrown", 0xbc8f8f),
        ("RosyBrown1", 0xffc1c1),
        ("RosyBrown2", 0xeeb4b4),
        ("RosyBrown3", 0xcd9b9b),
        ("RosyBrown4", 0x8b6969),
        ("RoyalBlue", 0x4169e1),
        ("RoyalBlue1", 0x4876ff),
        ("RoyalBlue2", 0x436eee),
        ("RoyalBlue3", 0x3a5fcd),
        ("RoyalBlue4", 0x27408b),
        ("SaddleBrown", 0x8b4513),
        ("SandyBrown", 0xf4a460),
        ("SeaGreen", 0x2e8b57),
        ("SeaGreen1", 0x54ff9f),
        ("SeaGreen2", 0x4eee94),
        ("SeaGreen3", 0x43cd80),
        ("SeaGreen4", 0x2e8b57),
        ("SkyBlue", 0x87ceeb),
        ("SkyBlue1", 0x87ceff),
        ("SkyBlue2", 0x7ec0ee),
        ("SkyBlue3", 0x6ca6cd),
        ("SkyBlue4", 0x4a708b),
        ("SlateBlue", 0x6a5acd),
        ("SlateBlue1", 0x836fff),
        ("SlateBlue2", 0x7a67ee),
        ("SlateBlue3", 0x6959cd),
        ("SlateBlue4", 0x473c8b),
        ("SlateGray", 0x708090),
        ("SlateGray1", 0xc6e2ff),
        ("SlateGray2", 0xb9d3ee),
        ("SlateGray3", 0x9fb6cd),
        ("SlateGray4", 0x6c7b8b),
        ("SlateGrey", 0x708090),
        ("SpringGreen", 0x00ff7f),
        ("SpringGreen1", 0x00ff7f),
        ("SpringGreen2", 0x00ee76),
        ("SpringGreen3", 0x00cd66),
        ("SpringGreen4", 0x008b45),
        ("SteelBlue", 0x4682b4),
        ("SteelBlue1", 0x63b8ff),
        ("SteelBlue2", 0x5cacee),
        ("SteelBlue3", 0x4f94cd),
        ("SteelBlue4", 0x36648b),
        ("VioletRed", 0xd02090),
        ("VioletRed1", 0xff3e96),
        ("VioletRed2", 0xee3a8c),
        ("VioletRed3", 0xcd3278),
        ("VioletRed4", 0x8b2252),
        ("WebGray", 0x808080),
        ("WebGreen", 0x008000),
        ("WebGrey", 0x808080),
        ("WebMaroon", 0x800000),
        ("WebPurple", 0x800080),
        ("WhiteSmoke", 0xf5f5f5),
        ("X11Gray", 0xbebebe),
        ("X11Green", 0x00ff00),
        ("X11Grey", 0xbebebe),
        ("X11Maroon", 0xb03060),
        ("X11Purple", 0xa020f0),
        ("YellowGreen", 0x9acd32),
        ("alice blue", 0xf0f8ff),
        ("antique white", 0xfaebd7),
        ("aqua", 0x00ffff),
        ("aquamarine", 0x7fffd4),
        ("aquamarine1", 0x7fffd4),
        ("aquamarine2", 0x76eec6),
        ("aquamarine3", 0x66cdaa),
        ("aquamarine4", 0x458b74),
        ("azure", 0xf0ffff),
        ("azure1", 0xf0ffff),
        ("azure2", 0xe0eeee),
        ("azure3", 0xc1cdcd),
        ("azure4", 0x838b8b),
        ("beige", 0xf5f5dc),
        ("bisque", 0xffe4c4),
        ("bisque1", 0xffe4c4),
        ("bisque2", 0xeed5b7),
        ("bisque3", 0xcdb79e),
        ("bisque4", 0x8b7d6b),
        ("black", 0x000000),
        ("blanched almond", 0xffebcd),
        ("blue violet", 0x8a2be2),
        ("blue", 0x0000ff),
        ("blue1", 0x0000ff),
        ("blue2", 0x0000ee),
        ("blue3", 0x0000cd),
        ("blue4", 0x00008b),
        ("brown", 0xa52a2a),
        ("brown1", 0xff4040),
        ("brown2", 0xee3b3b),
        ("brown3", 0xcd3333),
        ("brown4", 0x8b2323),
        ("burlywood", 0xdeb887),
        ("burlywood1", 0xffd39b),
        ("burlywood2", 0xeec591),
        ("burlywood3", 0xcdaa7d),
        ("burlywood4", 0x8b7355),
        ("cadet blue", 0x5f9ea0),
        ("chartreuse", 0x7fff00),
        ("chartreuse1", 0x7fff00),
        ("chartreuse2", 0x76ee00),
        ("chartreuse3", 0x66cd00),
        ("chartreuse4", 0x458b00),
        ("chocolate", 0xd2691e),
        ("chocolate1", 0xff7f24),
        ("chocolate2", 0xee7621),
        ("chocolate3", 0xcd661d),
        ("chocolate4", 0x8b4513),
        ("coral", 0xff7f50),
        ("coral1", 0xff7256),
        ("coral2", 0xee6a50),
        ("coral3", 0xcd5b45),
        ("coral4", 0x8b3e2f),
        ("cornflower blue", 0x6495ed),
        ("cornsilk", 0xfff8dc),
        ("cornsilk1", 0xfff8dc),
        ("cornsilk2", 0xeee8cd),
        ("cornsilk3", 0xcdc8b1),
        ("cornsilk4", 0x8b8878),
        ("crimson", 0xdc143c),
        ("cyan", 0x00ffff),
        ("cyan1", 0x00ffff),
        ("cyan2", 0x00eeee),
        ("cyan3", 0x00cdcd),
        ("cyan4", 0x008b8b),
        ("dark blue", 0x00008b),
        ("dark cyan", 0x008b8b),
        ("dark goldenrod", 0xb8860b),
        ("dark gray", 0xa9a9a9),
        ("dark green", 0x006400),
        ("dark grey", 0xa9a9a9),
        ("dark khaki", 0xbdb76b),
        ("dark magenta", 0x8b008b),
        ("dark olive green", 0x556b2f),
        ("dark orange", 0xff8c00),
        ("dark orchid", 0x9932cc),
        ("dark red", 0x8b0000),
        ("dark salmon", 0xe9967a),
        ("dark sea green", 0x8fbc8f),
        ("dark slate blue", 0x483d8b),
        ("dark slate gray", 0x2f4f4f),
        ("dark slate grey", 0x2f4f4f),
        ("dark turquoise", 0x00ced1),
        ("dark violet", 0x9400d3),
        ("deep pink", 0xff1493),
        ("deep sky blue", 0x00bfff),
        ("dim gray", 0x696969),
        ("dim grey", 0x696969),
        ("dodger blue", 0x1e90ff),
        ("firebrick", 0xb22222),
        ("firebrick1", 0xff3030),
        ("firebrick2", 0xee2c2c),
        ("firebrick3", 0xcd2626),
        ("firebrick4", 0x8b1a1a),
        ("floral white", 0xfffaf0),
        ("forest green", 0x228b22),
        ("fuchsia", 0xff00ff),
        ("gainsboro", 0xdcdcdc),
        ("ghost white", 0xf8f8ff),
        ("gold", 0xffd700),
        ("gold1", 0xffd700),
        ("gold2", 0xeec900),
        ("gold3", 0xcdad00),
        ("gold4", 0x8b7500),
        ("goldenrod", 0xdaa520),
        ("goldenrod1", 0xffc125),
        ("goldenrod2", 0xeeb422),
        ("goldenrod3", 0xcd9b1d),
        ("goldenrod4", 0x8b6914),
        ("green yellow", 0xadff2f),
        ("green", 0x00ff00),
        ("green1", 0x00ff00),
        ("green2", 0x00ee00),
        ("green3", 0x00cd00),
        ("green4", 0x008b00),
        ("honeydew", 0xf0fff0),
        ("honeydew1", 0xf0fff0),
        ("honeydew2", 0xe0eee0),
        ("honeydew3", 0xc1cdc1),
        ("honeydew4", 0x838b83),
        ("hot pink", 0xff69b4),
        ("indian red", 0xcd5c5c),
        ("indigo", 0x4b0082),
        ("ivory", 0xfffff0),
        ("ivory1", 0xfffff0),
        ("ivory2", 0xeeeee0),
        ("ivory3", 0xcdcdc1),
        ("ivory4", 0x8b8b83),
        ("khaki", 0xf0e68c),
        ("khaki1", 0xfff68f),
        ("khaki2", 0xeee685),
        ("khaki3", 0xcdc673),
        ("khaki4", 0x8b864e),
        ("lavender blush", 0xfff0f5),
        ("lavender", 0xe6e6fa),
        ("lawn green", 0x7cfc00),
        ("lemon chiffon", 0xfffacd),
        ("light blue", 0xadd8e6),
        ("light coral", 0xf08080),
        ("light cyan", 0xe0ffff),
        ("light goldenrod yellow", 0xfafad2),
        ("light goldenrod", 0xeedd82),
        ("light gray", 0xd3d3d3),
        ("light green", 0x90ee90),
        ("light grey", 0xd3d3d3),
        ("light pink", 0xffb6c1),
        ("light salmon", 0xffa07a),
        ("light sea green", 0x20b2aa),
        ("light sky blue", 0x87cefa),
        ("light slate blue", 0x8470ff),
        ("light slate gray", 0x778899),
        ("light slate grey", 0x778899),
        ("light steel blue", 0xb0c4de),
        ("light yellow", 0xffffe0),
        ("lime green", 0x32cd32),
        ("lime", 0x00ff00),
        ("linen", 0xfaf0e6),
        ("magenta", 0xff00ff),
        ("magenta1", 0xff00ff),
        ("magenta2", 0xee00ee),
        ("magenta3", 0xcd00cd),
        ("magenta4", 0x8b008b),
        ("maroon", 0xb03060),
        ("maroon1", 0xff34b3),
        ("maroon2", 0xee30a7),
        ("maroon3", 0xcd2990),
        ("maroon4", 0x8b1c62),
        ("medium aquamarine", 0x66cdaa),
        ("medium blue", 0x0000cd),
        ("medium orchid", 0xba55d3),
        ("medium purple", 0x9370db),
        ("medium sea green", 0x3cb371),
        ("medium slate blue", 0x7b68ee),
        ("medium spring green", 0x00fa9a),
        ("medium turquoise", 0x48d1cc),
        ("medium violet red", 0xc71585),
        ("midnight blue", 0x191970),
        ("mint cream", 0xf5fffa),
        ("misty rose", 0xffe4e1),
        ("moccasin", 0xffe4b5),
        ("navajo white", 0xffdead),
        ("navy blue", 0x000080),
        ("navy", 0x000080),
        ("old lace", 0xfdf5e6),
        ("olive drab", 0x6b8e23),
        ("olive", 0x808000),
        ("orange red", 0xff4500),
        ("orange", 0xffa500),
        ("orange1", 0xffa500),
        ("orange2", 0xee9a00),
        ("orange3", 0xcd8500),
        ("orange4", 0x8b5a00),
        ("orchid", 0xda70d6),
        ("orchid1", 0xff83fa),
        ("orchid2", 0xee7ae9),
        ("orchid3", 0xcd69c9),
        ("orchid4", 0x8b4789),
        ("pale goldenrod", 0xeee8aa),
        ("pale green", 0x98fb98),
        ("pale turquoise", 0xafeeee),
        ("pale violet red", 0xdb7093),
        ("papaya whip", 0xffefd5),
        ("peach puff", 0xffdab9),
        ("peru", 0xcd853f),
        ("pink", 0xffc0cb),
        ("pink1", 0xffb5c5),
        ("pink2", 0xeea9b8),
        ("pink3", 0xcd919e),
        ("pink4", 0x8b636c),
        ("plum", 0xdda0dd),
        ("plum1", 0xffbbff),
        ("plum2", 0xeeaeee),
        ("plum3", 0xcd96cd),
        ("plum4", 0x8b668b),
        ("powder blue", 0xb0e0e6),
        ("purple", 0xa020f0),
        ("purple1", 0x9b30ff),
        ("purple2", 0x912cee),
        ("purple3", 0x7d26cd),
        ("purple4", 0x551a8b),
        ("rebecca purple", 0x663399),
        ("red", 0xff0000),
        ("red1", 0xff0000),
        ("red2", 0xee0000),
        ("red3", 0xcd0000),
        ("red4", 0x8b0000),
        ("rosy brown", 0xbc8f8f),
        ("royal blue", 0x4169e1),
        ("saddle brown", 0x8b4513),
        ("salmon", 0xfa8072),
        ("salmon1", 0xff8c69),
        ("salmon2", 0xee8262),
        ("salmon3", 0xcd7054),
        ("salmon4", 0x8b4c39),
        ("sandy brown", 0xf4a460),
        ("sea green", 0x2e8b57),
        ("seashell", 0xfff5ee),
        ("seashell1", 0xfff5ee),
        ("seashell2", 0xeee5de),
        ("seashell3", 0xcdc5bf),
        ("seashell4", 0x8b8682),
        ("sienna", 0xa0522d),
        ("sienna1", 0xff8247),
        ("sienna2", 0xee7942),
        ("sienna3", 0xcd6839),
        ("sienna4", 0x8b4726),
        ("silver", 0xc0c0c0),
        ("sky blue", 0x87ceeb),
        ("slate blue", 0x6a5acd),
        ("slate gray", 0x708090),
        ("slate grey", 0x708090),
        ("snow", 0xfffafa),
        ("snow1", 0xfffafa),
        ("snow2", 0xeee9e9),
        ("snow3", 0xcdc9c9),
        ("snow4", 0x8b8989),
        ("spring green", 0x00ff7f),
        ("steel blue", 0x4682b4),
        ("tan", 0xd2b48c),
        ("tan1", 0xffa54f),
        ("tan2", 0xee9a49),
        ("tan3", 0xcd853f),
        ("tan4", 0x8b5a2b),
        ("teal", 0x008080),
        ("thistle", 0xd8bfd8),
        ("thistle1", 0xffe1ff),
        ("thistle2", 0xeed2ee),
        ("thistle3", 0xcdb5cd),
        ("thistle4", 0x8b7b8b),
        ("tomato", 0xff6347),
        ("tomato1", 0xff6347),
        ("tomato2", 0xee5c42),
        ("tomato3", 0xcd4f39),
        ("tomato4", 0x8b3626),
        ("turquoise", 0x40e0d0),
        ("turquoise1", 0x00f5ff),
        ("turquoise2", 0x00e5ee),
        ("turquoise3", 0x00c5cd),
        ("turquoise4", 0x00868b),
        ("violet red", 0xd02090),
        ("violet", 0xee82ee),
        ("web gray", 0x808080),
        ("web green", 0x008000),
        ("web grey", 0x808080),
        ("web maroon", 0x800000),
        ("web purple", 0x800080),
        ("wheat", 0xf5deb3),
        ("wheat1", 0xffe7ba),
        ("wheat2", 0xeed8ae),
        ("wheat3", 0xcdba96),
        ("wheat4", 0x8b7e66),
        ("white smoke", 0xf5f5f5),
        ("white", 0xffffff),
        ("x11 gray", 0xbebebe),
        ("x11 green", 0x00ff00),
        ("x11 grey", 0xbebebe),
        ("x11 maroon", 0xb03060),
        ("x11 purple", 0xa020f0),
        ("yellow green", 0x9acd32),
        ("yellow", 0xffff00),
        ("yellow1", 0xffff00),
        ("yellow2", 0xeeee00),
        ("yellow3", 0xcdcd00),
        ("yellow4", 0x8b8b00),
    ];

    unsafe {
        if name.starts_with("grey") || name.starts_with("gray") {
            if name.len() == 4 {
                return -1;
            }

            let Ok(c) = strtonum_(&name[4..], 0, 100) else {
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
            if color_name.eq_ignore_ascii_case(name) {
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
            colour = colour_byname(cstr_to_str(copy));
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

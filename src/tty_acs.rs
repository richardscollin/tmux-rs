// Copyright (c) 2010 Nicholas Marriott <nicholas.marriott@gmail.com>
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

#[repr(C)]
pub struct tty_acs_entry {
    pub key: u8,
    pub string: &'static [u8; 4],
}
impl tty_acs_entry {
    pub const fn new(key: u8, string: &'static [u8; 4]) -> Self {
        Self { key, string }
    }
}

static TTY_ACS_TABLE: [tty_acs_entry; 36] = [
    tty_acs_entry::new(b'+', &[0o342, 0o206, 0o222, 0o000]), // arrow pointing right
    tty_acs_entry::new(b',', &[0o342, 0o206, 0o220, 0o000]), // arrow pointing left
    tty_acs_entry::new(b'-', &[0o342, 0o206, 0o221, 0o000]), // arrow pointing up
    tty_acs_entry::new(b'.', &[0o342, 0o206, 0o223, 0o000]), // arrow pointing down
    tty_acs_entry::new(b'0', &[0o342, 0o226, 0o256, 0o000]), // solid square block
    tty_acs_entry::new(b'`', &[0o342, 0o227, 0o206, 0o000]), // diamond
    tty_acs_entry::new(b'a', &[0o342, 0o226, 0o222, 0o000]), // checker board (stipple)
    tty_acs_entry::new(b'b', &[0o342, 0o220, 0o211, 0o000]),
    tty_acs_entry::new(b'c', &[0o342, 0o220, 0o214, 0o000]),
    tty_acs_entry::new(b'd', &[0o342, 0o220, 0o215, 0o000]),
    tty_acs_entry::new(b'e', &[0o342, 0o220, 0o212, 0o000]),
    tty_acs_entry::new(b'f', &[0o302, 0o260, 0o000, 0o000]), // degree symbol
    tty_acs_entry::new(b'g', &[0o302, 0o261, 0o000, 0o000]), // plus/minus
    tty_acs_entry::new(b'h', &[0o342, 0o220, 0o244, 0o000]),
    tty_acs_entry::new(b'i', &[0o342, 0o220, 0o213, 0o000]),
    tty_acs_entry::new(b'j', &[0o342, 0o224, 0o230, 0o000]), // lower right corner
    tty_acs_entry::new(b'k', &[0o342, 0o224, 0o220, 0o000]), // upper right corner
    tty_acs_entry::new(b'l', &[0o342, 0o224, 0o214, 0o000]), // upper left corner
    tty_acs_entry::new(b'm', &[0o342, 0o224, 0o224, 0o000]), // lower left corner
    tty_acs_entry::new(b'n', &[0o342, 0o224, 0o274, 0o000]), // large plus or crossover
    tty_acs_entry::new(b'o', &[0o342, 0o216, 0o272, 0o000]), // scan line 1
    tty_acs_entry::new(b'p', &[0o342, 0o216, 0o273, 0o000]), // scan line 3
    tty_acs_entry::new(b'q', &[0o342, 0o224, 0o200, 0o000]), // horizontal line
    tty_acs_entry::new(b'r', &[0o342, 0o216, 0o274, 0o000]), // scan line 7
    tty_acs_entry::new(b's', &[0o342, 0o216, 0o275, 0o000]), // scan line 9
    tty_acs_entry::new(b't', &[0o342, 0o224, 0o234, 0o000]), // tee pointing right
    tty_acs_entry::new(b'u', &[0o342, 0o224, 0o244, 0o000]), // tee pointing left
    tty_acs_entry::new(b'v', &[0o342, 0o224, 0o264, 0o000]), // tee pointing up
    tty_acs_entry::new(b'w', &[0o342, 0o224, 0o254, 0o000]), // tee pointing down
    tty_acs_entry::new(b'x', &[0o342, 0o224, 0o202, 0o000]), // vertical line
    tty_acs_entry::new(b'y', &[0o342, 0o211, 0o244, 0o000]), // less-than-or-equal-to
    tty_acs_entry::new(b'z', &[0o342, 0o211, 0o245, 0o000]), // greater-than-or-equal-to
    tty_acs_entry::new(b'{', &[0o317, 0o200, 0o000, 0o000]), // greek pi
    tty_acs_entry::new(b'|', &[0o342, 0o211, 0o240, 0o000]), // not-equal
    tty_acs_entry::new(b'}', &[0o302, 0o243, 0o000, 0o000]), // UK pound sign
    tty_acs_entry::new(b'~', &[0o302, 0o267, 0o000, 0o000]), // bullet
];

#[repr(C)]
pub struct tty_acs_reverse_entry {
    pub string: &'static [u8; 4],
    pub key: u8,
}
impl tty_acs_reverse_entry {
    const fn new(string: &'static [u8; 4], key: u8) -> Self {
        Self { string, key }
    }
}

static TTY_ACS_REVERSE2: [tty_acs_reverse_entry; 1] = [tty_acs_reverse_entry::new(
    &[0o302, 0o267, 0o000, 0o000],
    b'~',
)];

static TTY_ACS_REVERSE3: [tty_acs_reverse_entry; 32] = [
    tty_acs_reverse_entry::new(&[0o342, 0o224, 0o200, 0o000], b'q'),
    tty_acs_reverse_entry::new(&[0o342, 0o224, 0o201, 0o000], b'q'),
    tty_acs_reverse_entry::new(&[0o342, 0o224, 0o202, 0o000], b'x'),
    tty_acs_reverse_entry::new(&[0o342, 0o224, 0o203, 0o000], b'x'),
    tty_acs_reverse_entry::new(&[0o342, 0o224, 0o214, 0o000], b'l'),
    tty_acs_reverse_entry::new(&[0o342, 0o224, 0o217, 0o000], b'k'),
    tty_acs_reverse_entry::new(&[0o342, 0o224, 0o220, 0o000], b'k'),
    tty_acs_reverse_entry::new(&[0o342, 0o224, 0o223, 0o000], b'l'),
    tty_acs_reverse_entry::new(&[0o342, 0o224, 0o224, 0o000], b'm'),
    tty_acs_reverse_entry::new(&[0o342, 0o224, 0o227, 0o000], b'm'),
    tty_acs_reverse_entry::new(&[0o342, 0o224, 0o230, 0o000], b'j'),
    tty_acs_reverse_entry::new(&[0o342, 0o224, 0o233, 0o000], b'j'),
    tty_acs_reverse_entry::new(&[0o342, 0o224, 0o234, 0o000], b't'),
    tty_acs_reverse_entry::new(&[0o342, 0o224, 0o243, 0o000], b't'),
    tty_acs_reverse_entry::new(&[0o342, 0o224, 0o244, 0o000], b'u'),
    tty_acs_reverse_entry::new(&[0o342, 0o224, 0o253, 0o000], b'u'),
    tty_acs_reverse_entry::new(&[0o342, 0o224, 0o263, 0o000], b'w'),
    tty_acs_reverse_entry::new(&[0o342, 0o224, 0o264, 0o000], b'v'),
    tty_acs_reverse_entry::new(&[0o342, 0o224, 0o273, 0o000], b'v'),
    tty_acs_reverse_entry::new(&[0o342, 0o224, 0o274, 0o000], b'n'),
    tty_acs_reverse_entry::new(&[0o342, 0o225, 0o213, 0o000], b'n'),
    tty_acs_reverse_entry::new(&[0o342, 0o225, 0o220, 0o000], b'q'),
    tty_acs_reverse_entry::new(&[0o342, 0o225, 0o221, 0o000], b'x'),
    tty_acs_reverse_entry::new(&[0o342, 0o225, 0o224, 0o000], b'l'),
    tty_acs_reverse_entry::new(&[0o342, 0o225, 0o227, 0o000], b'k'),
    tty_acs_reverse_entry::new(&[0o342, 0o225, 0o232, 0o000], b'm'),
    tty_acs_reverse_entry::new(&[0o342, 0o225, 0o235, 0o000], b'j'),
    tty_acs_reverse_entry::new(&[0o342, 0o225, 0o240, 0o000], b't'),
    tty_acs_reverse_entry::new(&[0o342, 0o225, 0o243, 0o000], b'u'),
    tty_acs_reverse_entry::new(&[0o342, 0o225, 0o246, 0o000], b'w'),
    tty_acs_reverse_entry::new(&[0o342, 0o225, 0o251, 0o000], b'v'),
    tty_acs_reverse_entry::new(&[0o342, 0o225, 0o254, 0o000], b'n'),
];

/// UTF-8 double borders.
static TTY_ACS_DOUBLE_BORDERS_LIST: [utf8_data; 13] = [
    utf8_data::new([0o000, 0o000, 0o000, 0o000], 0, 0, 0),
    utf8_data::new([0o342, 0o225, 0o221, 0o000], 0, 3, 1), // U+2551
    utf8_data::new([0o342, 0o225, 0o220, 0o000], 0, 3, 1), // U+2550
    utf8_data::new([0o342, 0o225, 0o224, 0o000], 0, 3, 1), // U+2554
    utf8_data::new([0o342, 0o225, 0o227, 0o000], 0, 3, 1), // U+2557
    utf8_data::new([0o342, 0o225, 0o232, 0o000], 0, 3, 1), // U+255A
    utf8_data::new([0o342, 0o225, 0o235, 0o000], 0, 3, 1), // U+255D
    utf8_data::new([0o342, 0o225, 0o246, 0o000], 0, 3, 1), // U+2566
    utf8_data::new([0o342, 0o225, 0o251, 0o000], 0, 3, 1), // U+2569
    utf8_data::new([0o342, 0o225, 0o240, 0o000], 0, 3, 1), // U+2560
    utf8_data::new([0o342, 0o225, 0o243, 0o000], 0, 3, 1), // U+2563
    utf8_data::new([0o342, 0o225, 0o254, 0o000], 0, 3, 1), // U+256C
    utf8_data::new([0o302, 0o267, 0o000, 0o000], 0, 2, 1), // U+00B7
];

/// UTF-8 heavy borders.
static TTY_ACS_HEAVY_BORDERS_LIST: [utf8_data; 13] = [
    utf8_data::new([0o000, 0o000, 0o000, 0o000], 0, 0, 0),
    utf8_data::new([0o342, 0o224, 0o203, 0o000], 0, 3, 1), // U+2503
    utf8_data::new([0o342, 0o224, 0o201, 0o000], 0, 3, 1), // U+2501
    utf8_data::new([0o342, 0o224, 0o217, 0o000], 0, 3, 1), // U+250F
    utf8_data::new([0o342, 0o224, 0o223, 0o000], 0, 3, 1), // U+2513
    utf8_data::new([0o342, 0o224, 0o227, 0o000], 0, 3, 1), // U+2517
    utf8_data::new([0o342, 0o224, 0o233, 0o000], 0, 3, 1), // U+251B
    utf8_data::new([0o342, 0o224, 0o263, 0o000], 0, 3, 1), // U+2533
    utf8_data::new([0o342, 0o224, 0o273, 0o000], 0, 3, 1), // U+253B
    utf8_data::new([0o342, 0o224, 0o243, 0o000], 0, 3, 1), // U+2523
    utf8_data::new([0o342, 0o224, 0o253, 0o000], 0, 3, 1), // U+252B
    utf8_data::new([0o342, 0o225, 0o213, 0o000], 0, 3, 1), // U+254B
    utf8_data::new([0o302, 0o267, 0o000, 0o000], 0, 2, 1), // U+00B7
];

/// UTF-8 rounded borders.
static TTY_ACS_ROUNDED_BORDERS_LIST: [utf8_data; 13] = [
    utf8_data::new([0o000, 0o000, 0o000, 0o000], 0, 0, 0),
    utf8_data::new([0o342, 0o224, 0o202, 0o000], 0, 3, 1), // U+2502
    utf8_data::new([0o342, 0o224, 0o200, 0o000], 0, 3, 1), // U+2500
    utf8_data::new([0o342, 0o225, 0o255, 0o000], 0, 3, 1), // U+256D
    utf8_data::new([0o342, 0o225, 0o256, 0o000], 0, 3, 1), // U+256E
    utf8_data::new([0o342, 0o225, 0o260, 0o000], 0, 3, 1), // U+2570
    utf8_data::new([0o342, 0o225, 0o257, 0o000], 0, 3, 1), // U+256F
    utf8_data::new([0o342, 0o224, 0o263, 0o000], 0, 3, 1), // U+2533
    utf8_data::new([0o342, 0o224, 0o273, 0o000], 0, 3, 1), // U+253B
    utf8_data::new([0o342, 0o224, 0o234, 0o000], 0, 3, 1), // U+2524
    utf8_data::new([0o342, 0o224, 0o244, 0o000], 0, 3, 1), // U+251C
    utf8_data::new([0o342, 0o225, 0o213, 0o000], 0, 3, 1), // U+254B
    utf8_data::new([0o302, 0o267, 0o000, 0o000], 0, 2, 1), // U+00B7
];

pub fn tty_acs_double_borders(cell_type: cell_type) -> &'static utf8_data {
    &TTY_ACS_DOUBLE_BORDERS_LIST[cell_type as usize]
}

pub fn tty_acs_heavy_borders(cell_type: cell_type) -> &'static utf8_data {
    &TTY_ACS_HEAVY_BORDERS_LIST[cell_type as usize]
}

/// Get cell border character for rounded style.
pub fn tty_acs_rounded_borders(cell_type: cell_type) -> &'static utf8_data {
    &TTY_ACS_ROUNDED_BORDERS_LIST[cell_type as usize]
}

pub fn tty_acs_cmp(test: &u8, entry: &tty_acs_entry) -> std::cmp::Ordering {
    test.cmp(&entry.key)
}

pub unsafe fn tty_acs_reverse_cmp(
    key: *const u8,
    entry: *const tty_acs_reverse_entry,
) -> std::cmp::Ordering {
    unsafe { i32_to_ordering(libc::strcmp(key, (*entry).string.as_ptr().cast())) }
}

/// Should this terminal use ACS instead of UTF-8 line drawing?
pub unsafe fn tty_acs_needed(tty: *const tty) -> bool {
    unsafe {
        if tty.is_null() {
            return false;
        }

        if tty_term_has((*tty).term, tty_code_code::TTYC_U8)
            && tty_term_number((*tty).term, tty_code_code::TTYC_U8) == 0
        {
            return true;
        }

        if (*(*tty).client).flags.intersects(client_flag::UTF8) {
            return false;
        }
        true
    }
}

/// Retrieve ACS to output as UTF-8.
pub unsafe fn tty_acs_get(tty: *mut tty, ch: u8) -> *const u8 {
    unsafe {
        // Use the ACS set instead of UTF-8 if needed.
        if tty_acs_needed(tty) {
            if (*(*tty).term).acs[ch as usize][0] == b'\0' {
                return null();
            }
            return &raw const (*(*tty).term).acs[ch as usize][0];
        }

        let Ok(entry) = TTY_ACS_TABLE.binary_search_by(|e| tty_acs_cmp(&ch, e).reverse()) else {
            return null_mut();
        };

        TTY_ACS_TABLE[entry].string.as_ptr().cast()
    }
}

/// Reverse UTF-8 into ACS.
pub unsafe fn tty_acs_reverse_get(_tty: *const tty, s: *const u8, slen: usize) -> i32 {
    unsafe {
        let table = if slen == 2 {
            TTY_ACS_REVERSE2.as_slice()
        } else if slen == 3 {
            TTY_ACS_REVERSE3.as_slice()
        } else {
            return -1;
        };
        let Ok(entry) = table.binary_search_by(|e| tty_acs_reverse_cmp(s, e).reverse()) else {
            return -1;
        };
        table[entry].key as _
    }
}

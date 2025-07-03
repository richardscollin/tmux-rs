// Copyright (c) 2007 Nicholas Marriott <nicholas.marriott@gmail.com>
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

use crate::compat::strlcat;
use libc::{memcpy, snprintf, sscanf, strcasecmp, tolower};

unsafe impl Sync for key_string_table_entry {}
#[repr(C)]
#[derive(Copy, Clone)]
struct key_string_table_entry {
    string: *const c_char,
    key: key_code,
}

impl key_string_table_entry {
    const fn new(string: &'static CStr, key: key_code) -> Self {
        Self {
            string: string.as_ptr(),
            key,
        }
    }
}

// #define KEYC_MOUSE_KEY(name)
// 	KEYC_ ## name ## _PANE,
// 	KEYC_ ## name ## _STATUS,
// 	KEYC_ ## name ## _STATUS_LEFT,
// 	KEYC_ ## name ## _STATUS_RIGHT,
// 	KEYC_ ## name ## _STATUS_DEFAULT,
// 	KEYC_ ## name ## _BORDER
// #define KEYC_MOUSE_STRING(name, s)
// 	{ #s "Pane", KEYC_ ## name ## _PANE },
// 	{ #s "Status", KEYC_ ## name ## _STATUS },
// 	{ #s "StatusLeft", KEYC_ ## name ## _STATUS_LEFT },
// 	{ #s "StatusRight", KEYC_ ## name ## _STATUS_RIGHT },
// 	{ #s "StatusDefault", KEYC_ ## name ## _STATUS_DEFAULT },
// 	{ #s "Border", KEYC_ ## name ## _BORDER }
macro_rules! KEYC_MOUSE_STRING {
    ($name:ident, $s:literal) => {
        ::paste::paste! {
            [
                key_string_table_entry{string: concat!($s, "Pane\0").as_ptr().cast(), key: keyc::[<KEYC_ $name _PANE>] as u64},
                key_string_table_entry{string: concat!($s, "Status\0").as_ptr().cast(), key: keyc::[<KEYC_ $name _STATUS>] as u64 },
                key_string_table_entry{string: concat!($s, "StatusLeft\0").as_ptr().cast(), key: keyc::[<KEYC_ $name _STATUS_LEFT>] as u64},
                key_string_table_entry{string: concat!($s, "StatusRight\0").as_ptr().cast(), key: keyc::[<KEYC_ $name _STATUS_RIGHT>] as u64},
                key_string_table_entry{string: concat!($s, "StatusDefault\0").as_ptr().cast(), key: keyc::[<KEYC_ $name _STATUS_DEFAULT>] as u64 },
                key_string_table_entry{string: concat!($s, "Border\0").as_ptr().cast(), key: keyc::[<KEYC_ $name _BORDER>] as u64},
            ]
        }
    };
}

macro_rules! concat_array {
    ($out:ident, $out_i: ident, $in:expr) => {
        let tmp = $in;
        let mut tmp_i = 0usize;
        while tmp_i < tmp.len() {
            $out[$out_i] = tmp[tmp_i];
            $out_i += 1;
            tmp_i += 1;
        }
    };
}

/*
* N. B. the order of the enum variants is incremental
    KEYC_MOUSEDOWN1_PANE,
    KEYC_MOUSEDOWN1_STATUS,
    KEYC_MOUSEDOWN1_STATUS_LEFT,
    KEYC_MOUSEDOWN1_STATUS_RIGHT,
    KEYC_MOUSEDOWN1_STATUS_DEFAULT,
    KEYC_MOUSEDOWN1_BORDER,
*/
macro_rules! KEYC_MOUSE_STRING_I {
    ($name:ident, $s:literal, $i:literal) => {
        ::paste::paste! {
            [
                key_string_table_entry{string: concat!($s, $i, "Pane\0").as_ptr().cast(), key: keyc::[<KEYC_ $name $i _PANE>] as u64},
                key_string_table_entry{string: concat!($s, $i, "Status\0").as_ptr().cast(), key: keyc::[<KEYC_ $name $i _STATUS>] as u64 },
                key_string_table_entry{string: concat!($s, $i, "StatusLeft\0").as_ptr().cast(), key: keyc::[<KEYC_ $name $i _STATUS_LEFT>] as u64},
                key_string_table_entry{string: concat!($s, $i, "StatusRight\0").as_ptr().cast(), key: keyc::[<KEYC_ $name $i _STATUS_RIGHT>] as u64},
                key_string_table_entry{string: concat!($s, $i, "StatusDefault\0").as_ptr().cast(), key: keyc::[<KEYC_ $name $i _STATUS_DEFAULT>] as u64 },
                key_string_table_entry{string: concat!($s, $i, "Border\0").as_ptr().cast(), key: keyc::[<KEYC_ $name $i _BORDER>] as u64},
            ]
        }
    };
}

macro_rules! KEYC_MOUSE_STRING11 {
    ($out:ident, $out_i: ident, $name:ident, $s:literal) => {
        concat_array!($out, $out_i, KEYC_MOUSE_STRING_I!($name, $s, 1));
        concat_array!($out, $out_i, KEYC_MOUSE_STRING_I!($name, $s, 2));
        concat_array!($out, $out_i, KEYC_MOUSE_STRING_I!($name, $s, 3));
        // yes, there's no 4 or 5
        concat_array!($out, $out_i, KEYC_MOUSE_STRING_I!($name, $s, 6));
        concat_array!($out, $out_i, KEYC_MOUSE_STRING_I!($name, $s, 7));
        concat_array!($out, $out_i, KEYC_MOUSE_STRING_I!($name, $s, 8));
        concat_array!($out, $out_i, KEYC_MOUSE_STRING_I!($name, $s, 9));
        concat_array!($out, $out_i, KEYC_MOUSE_STRING_I!($name, $s, 10));
        concat_array!($out, $out_i, KEYC_MOUSE_STRING_I!($name, $s, 11));
    };
}

static key_string_table: [key_string_table_entry; 469] = const {
    let mut out_i: usize = 0;
    let mut out: [key_string_table_entry; 469] = unsafe { zeroed() };

    let function_keys = [
        key_string_table_entry::new(c"F1", keyc::KEYC_F1 as u64 | KEYC_IMPLIED_META),
        key_string_table_entry::new(c"F2", keyc::KEYC_F2 as u64 | KEYC_IMPLIED_META),
        key_string_table_entry::new(c"F3", keyc::KEYC_F3 as u64 | KEYC_IMPLIED_META),
        key_string_table_entry::new(c"F4", keyc::KEYC_F4 as u64 | KEYC_IMPLIED_META),
        key_string_table_entry::new(c"F5", keyc::KEYC_F5 as u64 | KEYC_IMPLIED_META),
        key_string_table_entry::new(c"F6", keyc::KEYC_F6 as u64 | KEYC_IMPLIED_META),
        key_string_table_entry::new(c"F7", keyc::KEYC_F7 as u64 | KEYC_IMPLIED_META),
        key_string_table_entry::new(c"F8", keyc::KEYC_F8 as u64 | KEYC_IMPLIED_META),
        key_string_table_entry::new(c"F9", keyc::KEYC_F9 as u64 | KEYC_IMPLIED_META),
        key_string_table_entry::new(c"F10", keyc::KEYC_F10 as u64 | KEYC_IMPLIED_META),
        key_string_table_entry::new(c"F11", keyc::KEYC_F11 as u64 | KEYC_IMPLIED_META),
        key_string_table_entry::new(c"F12", keyc::KEYC_F12 as u64 | KEYC_IMPLIED_META),
        key_string_table_entry::new(c"IC", keyc::KEYC_IC as u64 | KEYC_IMPLIED_META),
        key_string_table_entry::new(c"Insert", keyc::KEYC_IC as u64 | KEYC_IMPLIED_META),
        key_string_table_entry::new(c"DC", keyc::KEYC_DC as u64 | KEYC_IMPLIED_META),
        key_string_table_entry::new(c"Delete", keyc::KEYC_DC as u64 | KEYC_IMPLIED_META),
        key_string_table_entry::new(c"Home", keyc::KEYC_HOME as u64 | KEYC_IMPLIED_META),
        key_string_table_entry::new(c"End", keyc::KEYC_END as u64 | KEYC_IMPLIED_META),
        key_string_table_entry::new(c"NPage", keyc::KEYC_NPAGE as u64 | KEYC_IMPLIED_META),
        key_string_table_entry::new(c"PageDown", keyc::KEYC_NPAGE as u64 | KEYC_IMPLIED_META),
        key_string_table_entry::new(c"PgDn", keyc::KEYC_NPAGE as u64 | KEYC_IMPLIED_META),
        key_string_table_entry::new(c"PPage", keyc::KEYC_PPAGE as u64 | KEYC_IMPLIED_META),
        key_string_table_entry::new(c"PageUp", keyc::KEYC_PPAGE as u64 | KEYC_IMPLIED_META),
        key_string_table_entry::new(c"PgUp", keyc::KEYC_PPAGE as u64 | KEYC_IMPLIED_META),
        key_string_table_entry::new(c"BTab", keyc::KEYC_BTAB as u64),
        key_string_table_entry::new(c"Space", ' ' as key_code),
        key_string_table_entry::new(c"BSpace", keyc::KEYC_BSPACE as u64),
        /*
         * C0 control characters, with the exception of Tab, Enter,
         * and Esc, should never appear as keys. We still render them,
         * so to be able to spot them in logs in case of an abnormality.
         */
        key_string_table_entry::new(c"[NUL]", c0::C0_NUL as u64),
        key_string_table_entry::new(c"[SOH]", c0::C0_SOH as u64),
        key_string_table_entry::new(c"[STX]", c0::C0_STX as u64),
        key_string_table_entry::new(c"[ETX]", c0::C0_ETX as u64),
        key_string_table_entry::new(c"[EOT]", c0::C0_EOT as u64),
        key_string_table_entry::new(c"[ENQ]", c0::C0_ENQ as u64),
        key_string_table_entry::new(c"[ASC]", c0::C0_ASC as u64),
        key_string_table_entry::new(c"[BEL]", c0::C0_BEL as u64),
        key_string_table_entry::new(c"[BS]", c0::C0_BS as u64),
        key_string_table_entry::new(c"Tab", c0::C0_HT as u64),
        key_string_table_entry::new(c"[LF]", c0::C0_LF as u64),
        key_string_table_entry::new(c"[VT]", c0::C0_VT as u64),
        key_string_table_entry::new(c"[FF]", c0::C0_FF as u64),
        key_string_table_entry::new(c"Enter", c0::C0_CR as u64),
        key_string_table_entry::new(c"[SO]", c0::C0_SO as u64),
        key_string_table_entry::new(c"[SI]", c0::C0_SI as u64),
        key_string_table_entry::new(c"[DLE]", c0::C0_DLE as u64),
        key_string_table_entry::new(c"[DC1]", c0::C0_DC1 as u64),
        key_string_table_entry::new(c"[DC2]", c0::C0_DC2 as u64),
        key_string_table_entry::new(c"[DC3]", c0::C0_DC3 as u64),
        key_string_table_entry::new(c"[DC4]", c0::C0_DC4 as u64),
        key_string_table_entry::new(c"[NAK]", c0::C0_NAK as u64),
        key_string_table_entry::new(c"[SYN]", c0::C0_SYN as u64),
        key_string_table_entry::new(c"[ETB]", c0::C0_ETB as u64),
        key_string_table_entry::new(c"[CAN]", c0::C0_CAN as u64),
        key_string_table_entry::new(c"[EM]", c0::C0_EM as u64),
        key_string_table_entry::new(c"[SUB]", c0::C0_SUB as u64),
        key_string_table_entry::new(c"Escape", c0::C0_ESC as u64),
        key_string_table_entry::new(c"[FS]", c0::C0_FS as u64),
        key_string_table_entry::new(c"[GS]", c0::C0_GS as u64),
        key_string_table_entry::new(c"[RS]", c0::C0_RS as u64),
        key_string_table_entry::new(c"[US]", c0::C0_US as u64),
        /* Arrow keys. */
        key_string_table_entry::new(
            c"Up",
            keyc::KEYC_UP as u64 | KEYC_CURSOR | KEYC_IMPLIED_META,
        ),
        key_string_table_entry::new(
            c"Down",
            keyc::KEYC_DOWN as u64 | KEYC_CURSOR | KEYC_IMPLIED_META,
        ),
        key_string_table_entry::new(
            c"Left",
            keyc::KEYC_LEFT as u64 | KEYC_CURSOR | KEYC_IMPLIED_META,
        ),
        key_string_table_entry::new(
            c"Right",
            keyc::KEYC_RIGHT as u64 | KEYC_CURSOR | KEYC_IMPLIED_META,
        ),
        /* Numeric keypad. */
        key_string_table_entry::new(c"KP/", keyc::KEYC_KP_SLASH as u64 | KEYC_KEYPAD),
        key_string_table_entry::new(c"KP*", keyc::KEYC_KP_STAR as u64 | KEYC_KEYPAD),
        key_string_table_entry::new(c"KP-", keyc::KEYC_KP_MINUS as u64 | KEYC_KEYPAD),
        key_string_table_entry::new(c"KP7", keyc::KEYC_KP_SEVEN as u64 | KEYC_KEYPAD),
        key_string_table_entry::new(c"KP8", keyc::KEYC_KP_EIGHT as u64 | KEYC_KEYPAD),
        key_string_table_entry::new(c"KP9", keyc::KEYC_KP_NINE as u64 | KEYC_KEYPAD),
        key_string_table_entry::new(c"KP+", keyc::KEYC_KP_PLUS as u64 | KEYC_KEYPAD),
        key_string_table_entry::new(c"KP4", keyc::KEYC_KP_FOUR as u64 | KEYC_KEYPAD),
        key_string_table_entry::new(c"KP5", keyc::KEYC_KP_FIVE as u64 | KEYC_KEYPAD),
        key_string_table_entry::new(c"KP6", keyc::KEYC_KP_SIX as u64 | KEYC_KEYPAD),
        key_string_table_entry::new(c"KP1", keyc::KEYC_KP_ONE as u64 | KEYC_KEYPAD),
        key_string_table_entry::new(c"KP2", keyc::KEYC_KP_TWO as u64 | KEYC_KEYPAD),
        key_string_table_entry::new(c"KP3", keyc::KEYC_KP_THREE as u64 | KEYC_KEYPAD),
        key_string_table_entry::new(c"KPEnter", keyc::KEYC_KP_ENTER as u64 | KEYC_KEYPAD),
        key_string_table_entry::new(c"KP0", keyc::KEYC_KP_ZERO as u64 | KEYC_KEYPAD),
        key_string_table_entry::new(c"KP.", keyc::KEYC_KP_PERIOD as u64 | KEYC_KEYPAD),
    ];

    concat_array!(out, out_i, function_keys);

    // Mouse keys.
    KEYC_MOUSE_STRING11!(out, out_i, MOUSEDOWN, "MouseDown");
    KEYC_MOUSE_STRING11!(out, out_i, MOUSEUP, "MouseUp");
    KEYC_MOUSE_STRING11!(out, out_i, MOUSEDRAG, "MouseDrag");
    KEYC_MOUSE_STRING11!(out, out_i, MOUSEDRAGEND, "MouseDragEnd");
    concat_array!(out, out_i, KEYC_MOUSE_STRING!(WHEELUP, "WheelUp"));
    concat_array!(out, out_i, KEYC_MOUSE_STRING!(WHEELDOWN, "WheelDown"));
    KEYC_MOUSE_STRING11!(out, out_i, SECONDCLICK, "SecondClick");
    KEYC_MOUSE_STRING11!(out, out_i, DOUBLECLICK, "DoubleClick");
    KEYC_MOUSE_STRING11!(out, out_i, TRIPLECLICK, "TripleClick");

    out
};

/// Find key string in table.
pub unsafe fn key_string_search_table(string: *const c_char) -> key_code {
    unsafe {
        for key_string in &key_string_table {
            if strcasecmp(string, key_string.string) == 0 {
                return key_string.key;
            }
        }

        let mut user = 0u32;
        if sscanf(string, c"User%u".as_ptr(), &raw mut user) == 1 && user < KEYC_NUSER as u32 {
            return KEYC_USER + user as u64;
        }
    }

    KEYC_UNKNOWN
}

/// Find modifiers.
pub unsafe fn key_string_get_modifiers(string: *mut *const c_char) -> key_code {
    unsafe {
        let mut modifiers: key_code = 0;

        while **string as u8 != b'\0' && *(*string).add(1) as u8 == b'-' {
            match **string as u8 {
                b'C' | b'c' => {
                    modifiers |= KEYC_CTRL;
                }
                b'M' | b'm' => {
                    modifiers |= KEYC_META;
                }
                b'S' | b's' => {
                    modifiers |= KEYC_SHIFT;
                }
                _ => {
                    *string = null_mut();
                    return 0;
                }
            }
            (*string) = (*string).add(2);
        }

        modifiers
    }
}

// TODO
const MB_LEN_MAX: usize = 16;

/* Lookup a string and convert to a key value. */

pub unsafe fn key_string_lookup_string(mut string: *const c_char) -> key_code {
    unsafe {
        let mut key: key_code = 0;
        let mut modifiers: key_code = 0;
        let mut u: u32 = 0;
        let i: u32 = 0;
        let mut ud: utf8_data = zeroed();
        let mut uc: utf8_char = 0;
        let mlen = 0i32;

        let mut m = [MaybeUninit::<c_char>::uninit(); MB_LEN_MAX + 1];

        /* Is this no key or any key? */
        if strcasecmp(string, c"None".as_ptr()) == 0 {
            return KEYC_NONE;
        }
        if strcasecmp(string, c"Any".as_ptr()) == 0 {
            return keyc::KEYC_ANY as key_code;
        }

        /* Is this a hexadecimal value? */
        if *string == b'0' as c_char && *string.add(1) == b'x' as i8 {
            if sscanf(string.add(2), c"%x".as_ptr(), &raw mut u) != 1 {
                return KEYC_UNKNOWN;
            }
            if u < 32 {
                return u as u64;
            }
            let mlen = wctomb(m.as_mut_slice().as_mut_ptr().cast(), u as i32);
            if mlen <= 0 || mlen > MB_LEN_MAX as i32 {
                return KEYC_UNKNOWN;
            }
            m[mlen as usize].write(b'\0' as c_char);

            let udp: *mut utf8_data = utf8_fromcstr(m.as_slice().as_ptr().cast());
            if udp.is_null()
                || (*udp).size == 0
                || (*udp.add(1)).size != 0
                || utf8_from_data(udp, &raw mut uc) != utf8_state::UTF8_DONE
            {
                free_(udp);
                return KEYC_UNKNOWN;
            }
            free_(udp);
            return uc as u64;
        }

        /* Check for short Ctrl key. */
        if *string == b'^' as c_char && *string.add(1) != b'\0' as i8 {
            if *string.add(2) == b'\0' as i8 {
                return tolower(*string.add(1) as _) as u64 | KEYC_CTRL;
            }
            modifiers |= KEYC_CTRL;
            string = string.add(1);
        }

        // Check for modifiers.
        modifiers |= key_string_get_modifiers(&raw mut string);
        if string.is_null() || *string == b'\0' as c_char {
            return KEYC_UNKNOWN;
        }

        /* Is this a standard ASCII key? */
        if *string.add(1) == b'\0' as c_char && *string as u8 <= 127 {
            key = *string as u8 as u64;
            if key < 32 {
                return KEYC_UNKNOWN;
            }
        } else {
            /* Try as a UTF-8 key. */
            let mut more: utf8_state = utf8_open(&raw mut ud, *string as u8);
            if more == utf8_state::UTF8_MORE {
                if strlen(string) != ud.size as usize {
                    return KEYC_UNKNOWN;
                }
                for i in 1..ud.size {
                    more = utf8_append(&raw mut ud, *string.add(i as usize) as u8);
                }
                if more != utf8_state::UTF8_DONE {
                    return KEYC_UNKNOWN;
                }
                if utf8_from_data(&raw const ud, &raw mut uc) != utf8_state::UTF8_DONE {
                    return KEYC_UNKNOWN;
                }
                return uc as u64 | modifiers;
            }

            /* Otherwise look the key up in the table. */
            key = key_string_search_table(string);
            if key == KEYC_UNKNOWN {
                return KEYC_UNKNOWN;
            }
            if !modifiers & KEYC_META != 0 {
                key &= !KEYC_IMPLIED_META;
            }
        }

        key | modifiers
    }
}

/// Convert a key code into string format, with prefix if necessary.
pub unsafe fn key_string_lookup_key(
    mut key: key_code,
    with_flags: i32,
) -> *const c_char {
    let sizeof_out: usize = 64;
    static mut out: [c_char; 64] = [0; 64];
    unsafe {
        let saved = key;
        let sizeof_tmp: usize = 8;
        let mut tmp: [c_char; 8] = [0; 8];
        let mut s = null();
        let mut ud: utf8_data = zeroed();
        let mut off: usize = 0;

        out[0] = b'\0' as i8;

        'out: {
            'append: {
                /* Literal keys are themselves. */
                if key & KEYC_LITERAL != 0 {
                    snprintf(
                        &raw mut out as *mut i8,
                        sizeof_out,
                        c"%c".as_ptr(),
                        (key & 0xff) as i32,
                    );
                    break 'out;
                }

                /* Fill in the modifiers. */
                if key & KEYC_CTRL != 0 {
                    strlcat(&raw mut out as *mut i8, c"C-".as_ptr(), sizeof_out);
                }
                if key & KEYC_META != 0 {
                    strlcat(&raw mut out as *mut i8, c"M-".as_ptr(), sizeof_out);
                }
                if key & KEYC_SHIFT != 0 {
                    strlcat(&raw mut out as *mut i8, c"S-".as_ptr(), sizeof_out);
                }
                key &= KEYC_MASK_KEY;

                /* Handle no key. */
                if key == KEYC_NONE {
                    s = c"None".as_ptr();
                    break 'append;
                }

                /* Handle special keys. */
                if key == KEYC_UNKNOWN {
                    s = c"Unknown".as_ptr();
                    break 'append;
                }
                if key == keyc::KEYC_ANY as u64 {
                    s = c"Any".as_ptr();
                    break 'append;
                }
                if key == keyc::KEYC_FOCUS_IN as u64 {
                    s = c"FocusIn".as_ptr();
                    break 'append;
                }
                if key == keyc::KEYC_FOCUS_OUT as u64 {
                    s = c"FocusOut".as_ptr();
                    break 'append;
                }
                if key == keyc::KEYC_PASTE_START as u64 {
                    s = c"PasteStart".as_ptr();
                    break 'append;
                }
                if key == keyc::KEYC_PASTE_END as u64 {
                    s = c"PasteEnd".as_ptr();
                    break 'append;
                }
                if key == keyc::KEYC_MOUSE as u64 {
                    s = c"Mouse".as_ptr();
                    break 'append;
                }
                if key == keyc::KEYC_DRAGGING as u64 {
                    s = c"Dragging".as_ptr();
                    break 'append;
                }
                if key == keyc::KEYC_MOUSEMOVE_PANE as u64 {
                    s = c"MouseMovePane".as_ptr();
                    break 'append;
                }
                if key == keyc::KEYC_MOUSEMOVE_STATUS as u64 {
                    s = c"MouseMoveStatus".as_ptr();
                    break 'append;
                }
                if key == keyc::KEYC_MOUSEMOVE_STATUS_LEFT as u64 {
                    s = c"MouseMoveStatusLeft".as_ptr();
                    break 'append;
                }
                if key == keyc::KEYC_MOUSEMOVE_STATUS_RIGHT as u64 {
                    s = c"MouseMoveStatusRight".as_ptr();
                    break 'append;
                }
                if key == keyc::KEYC_MOUSEMOVE_BORDER as u64 {
                    s = c"MouseMoveBorder".as_ptr();
                    break 'append;
                }
                if key >= KEYC_USER && key < KEYC_USER_END {
                    snprintf(
                        &raw mut tmp as *mut c_char,
                        sizeof_tmp,
                        c"User%u".as_ptr(),
                        (key - KEYC_USER) as u8 as u32,
                    );
                    strlcat(
                        &raw mut out as *mut c_char,
                        &raw const tmp as *const c_char,
                        sizeof_out,
                    );
                    break 'out;
                }

                // Try the key against the string table.
                if let Some(i) = key_string_table
                    .iter()
                    .position(|e| key == e.key & KEYC_MASK_KEY)
                {
                    strlcat(
                        &raw mut out as *mut c_char,
                        key_string_table[i].string,
                        sizeof_out,
                    );
                    break 'out;
                }

                /* Is this a Unicode key? */
                if KEYC_IS_UNICODE(key) {
                    utf8_to_data(key as u32, &raw mut ud);
                    off = strlen(&raw const out as *const c_char);
                    memcpy(
                        &raw mut out[off] as *mut c_void,
                        &raw const ud.data as *const c_void,
                        ud.size as usize,
                    );
                    out[off + ud.size as usize] = b'\0' as c_char;
                    break 'out;
                }

                /* Invalid keys are errors. */
                if key > 255 {
                    snprintf(
                        &raw mut out as *mut c_char,
                        sizeof_out,
                        c"Invalid#%llx".as_ptr(),
                        saved,
                    );
                    break 'out;
                }

                /* Printable ASCII keys. */
                if key > 32 && key <= 126 {
                    tmp[0] = key as c_char;
                    tmp[1] = b'\0' as c_char;
                } else if key == 127 {
                    xsnprintf_!(&raw mut tmp as *mut c_char, sizeof_tmp, "C-?");
                } else if key >= 128 {
                    xsnprintf_!(&raw mut tmp as *mut c_char, sizeof_tmp, "\\{:o}", key,);
                }

                strlcat(
                    &raw mut out as *mut c_char,
                    &raw const tmp as *const c_char,
                    sizeof_out,
                );
                break 'out;
            }
            // append:
            strlcat(&raw mut out as *mut c_char, s, sizeof_out);
        }
        // out:
        if with_flags != 0 && (saved & KEYC_MASK_FLAGS) != 0 {
            strlcat(&raw mut out as *mut c_char, c"[".as_ptr(), sizeof_out);
            if saved & KEYC_LITERAL != 0 {
                strlcat(&raw mut out as *mut c_char, c"L".as_ptr(), sizeof_out);
            }
            if saved & KEYC_KEYPAD != 0 {
                strlcat(&raw mut out as *mut c_char, c"K".as_ptr(), sizeof_out);
            }
            if saved & KEYC_CURSOR != 0 {
                strlcat(&raw mut out as *mut c_char, c"C".as_ptr(), sizeof_out);
            }
            if saved & KEYC_IMPLIED_META != 0 {
                strlcat(&raw mut out as *mut c_char, c"I".as_ptr(), sizeof_out);
            }
            if saved & KEYC_BUILD_MODIFIERS != 0 {
                strlcat(&raw mut out as *mut c_char, c"B".as_ptr(), sizeof_out);
            }
            if saved & KEYC_SENT != 0 {
                strlcat(&raw mut out as *mut c_char, c"S".as_ptr(), sizeof_out);
            }
            strlcat(&raw mut out as *mut c_char, c"]".as_ptr(), sizeof_out);
        }
        &raw const out as *const i8
    }
}

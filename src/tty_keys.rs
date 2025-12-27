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
use crate::compat::b64::b64_pton;
use crate::*;
use crate::options_::*;

// Handle keys input from the outside terminal. tty_default_*_keys[] are a base
// table of supported keys which are looked up in terminfo(5) and translated
// into a ternary tree.

// A key tree entry.
#[repr(C)]
pub struct tty_key {
    ch: u8,
    key: key_code,

    left: *mut tty_key,
    right: *mut tty_key,

    next: *mut tty_key,
}

/// Default raw keys.
#[repr(C)]
struct tty_default_key_raw {
    string: SyncCharPtr,
    key: key_code,
}
impl tty_default_key_raw {
    const fn new(string: &'static CStr, key: key_code) -> Self {
        Self {
            string: SyncCharPtr::new(string),
            key,
        }
    }
}

static TTY_DEFAULT_RAW_KEYS: [tty_default_key_raw; 100] = [
    // Application escape.
    tty_default_key_raw::new(c"\x1bO[", '\x1b' as u64),
    // Numeric keypad. Just use the vt100 escape sequences here and always
    // put the terminal into keypad_xmit mode. Translation of numbers
    // mode/applications mode is done in input-keys.c.
    tty_default_key_raw::new(c"\x1bOo", keyc::KEYC_KP_SLASH as u64 | KEYC_KEYPAD),
    tty_default_key_raw::new(c"\x1bOj", keyc::KEYC_KP_STAR as u64 | KEYC_KEYPAD),
    tty_default_key_raw::new(c"\x1bOm", keyc::KEYC_KP_MINUS as u64 | KEYC_KEYPAD),
    tty_default_key_raw::new(c"\x1bOw", keyc::KEYC_KP_SEVEN as u64 | KEYC_KEYPAD),
    tty_default_key_raw::new(c"\x1bOx", keyc::KEYC_KP_EIGHT as u64 | KEYC_KEYPAD),
    tty_default_key_raw::new(c"\x1bOy", keyc::KEYC_KP_NINE as u64 | KEYC_KEYPAD),
    tty_default_key_raw::new(c"\x1bOk", keyc::KEYC_KP_PLUS as u64 | KEYC_KEYPAD),
    tty_default_key_raw::new(c"\x1bOt", keyc::KEYC_KP_FOUR as u64 | KEYC_KEYPAD),
    tty_default_key_raw::new(c"\x1bOu", keyc::KEYC_KP_FIVE as u64 | KEYC_KEYPAD),
    tty_default_key_raw::new(c"\x1bOv", keyc::KEYC_KP_SIX as u64 | KEYC_KEYPAD),
    tty_default_key_raw::new(c"\x1bOq", keyc::KEYC_KP_ONE as u64 | KEYC_KEYPAD),
    tty_default_key_raw::new(c"\x1bOr", keyc::KEYC_KP_TWO as u64 | KEYC_KEYPAD),
    tty_default_key_raw::new(c"\x1bOs", keyc::KEYC_KP_THREE as u64 | KEYC_KEYPAD),
    tty_default_key_raw::new(c"\x1bOM", keyc::KEYC_KP_ENTER as u64 | KEYC_KEYPAD),
    tty_default_key_raw::new(c"\x1bOp", keyc::KEYC_KP_ZERO as u64 | KEYC_KEYPAD),
    tty_default_key_raw::new(c"\x1bOn", keyc::KEYC_KP_PERIOD as u64 | KEYC_KEYPAD),
    // Arrow keys.
    tty_default_key_raw::new(c"\x1bOA", keyc::KEYC_UP as u64 | KEYC_CURSOR),
    tty_default_key_raw::new(c"\x1bOB", keyc::KEYC_DOWN as u64 | KEYC_CURSOR),
    tty_default_key_raw::new(c"\x1bOC", keyc::KEYC_RIGHT as u64 | KEYC_CURSOR),
    tty_default_key_raw::new(c"\x1bOD", keyc::KEYC_LEFT as u64 | KEYC_CURSOR),
    tty_default_key_raw::new(c"\x1b[A", keyc::KEYC_UP as u64 | KEYC_CURSOR),
    tty_default_key_raw::new(c"\x1b[B", keyc::KEYC_DOWN as u64 | KEYC_CURSOR),
    tty_default_key_raw::new(c"\x1b[C", keyc::KEYC_RIGHT as u64 | KEYC_CURSOR),
    tty_default_key_raw::new(c"\x1b[D", keyc::KEYC_LEFT as u64 | KEYC_CURSOR),
    // Meta arrow keys. These do not get the IMPLIED_META flag so they
    // don't match the xterm-style meta keys in the output tree - Escape+Up
    // should stay as Escape+Up and not become M-Up.
    tty_default_key_raw::new(
        c"\x1b\x1bOA",
        keyc::KEYC_UP as u64 | KEYC_CURSOR | KEYC_META,
    ),
    tty_default_key_raw::new(
        c"\x1b\x1bOB",
        keyc::KEYC_DOWN as u64 | KEYC_CURSOR | KEYC_META,
    ),
    tty_default_key_raw::new(
        c"\x1b\x1bOC",
        keyc::KEYC_RIGHT as u64 | KEYC_CURSOR | KEYC_META,
    ),
    tty_default_key_raw::new(
        c"\x1b\x1bOD",
        keyc::KEYC_LEFT as u64 | KEYC_CURSOR | KEYC_META,
    ),
    tty_default_key_raw::new(
        c"\x1b\x1b[A",
        keyc::KEYC_UP as u64 | KEYC_CURSOR | KEYC_META,
    ),
    tty_default_key_raw::new(
        c"\x1b\x1b[B",
        keyc::KEYC_DOWN as u64 | KEYC_CURSOR | KEYC_META,
    ),
    tty_default_key_raw::new(
        c"\x1b\x1b[C",
        keyc::KEYC_RIGHT as u64 | KEYC_CURSOR | KEYC_META,
    ),
    tty_default_key_raw::new(
        c"\x1b\x1b[D",
        keyc::KEYC_LEFT as u64 | KEYC_CURSOR | KEYC_META,
    ),
    // Other xterm keys.
    tty_default_key_raw::new(c"\x1bOH", keyc::KEYC_HOME as u64),
    tty_default_key_raw::new(c"\x1bOF", keyc::KEYC_END as u64),
    tty_default_key_raw::new(
        c"\x1b\x1bOH",
        keyc::KEYC_HOME as u64 | KEYC_META | KEYC_IMPLIED_META,
    ),
    tty_default_key_raw::new(
        c"\x1b\x1bOF",
        keyc::KEYC_END as u64 | KEYC_META | KEYC_IMPLIED_META,
    ),
    tty_default_key_raw::new(c"\x1b[H", keyc::KEYC_HOME as u64),
    tty_default_key_raw::new(c"\x1b[F", keyc::KEYC_END as u64),
    tty_default_key_raw::new(
        c"\x1b\x1b[H",
        keyc::KEYC_HOME as u64 | KEYC_META | KEYC_IMPLIED_META,
    ),
    tty_default_key_raw::new(
        c"\x1b\x1b[F",
        keyc::KEYC_END as u64 | KEYC_META | KEYC_IMPLIED_META,
    ),
    // rxvt arrow keys.
    tty_default_key_raw::new(c"\x1bOa", keyc::KEYC_UP as u64 | KEYC_CTRL),
    tty_default_key_raw::new(c"\x1bOb", keyc::KEYC_DOWN as u64 | KEYC_CTRL),
    tty_default_key_raw::new(c"\x1bOc", keyc::KEYC_RIGHT as u64 | KEYC_CTRL),
    tty_default_key_raw::new(c"\x1bOd", keyc::KEYC_LEFT as u64 | KEYC_CTRL),
    tty_default_key_raw::new(c"\x1b[a", keyc::KEYC_UP as u64 | KEYC_SHIFT),
    tty_default_key_raw::new(c"\x1b[b", keyc::KEYC_DOWN as u64 | KEYC_SHIFT),
    tty_default_key_raw::new(c"\x1b[c", keyc::KEYC_RIGHT as u64 | KEYC_SHIFT),
    tty_default_key_raw::new(c"\x1b[d", keyc::KEYC_LEFT as u64 | KEYC_SHIFT),
    // rxvt function keys.
    tty_default_key_raw::new(c"\x1b[11~", keyc::KEYC_F1 as u64),
    tty_default_key_raw::new(c"\x1b[12~", keyc::KEYC_F2 as u64),
    tty_default_key_raw::new(c"\x1b[13~", keyc::KEYC_F3 as u64),
    tty_default_key_raw::new(c"\x1b[14~", keyc::KEYC_F4 as u64),
    tty_default_key_raw::new(c"\x1b[15~", keyc::KEYC_F5 as u64),
    tty_default_key_raw::new(c"\x1b[17~", keyc::KEYC_F6 as u64),
    tty_default_key_raw::new(c"\x1b[18~", keyc::KEYC_F7 as u64),
    tty_default_key_raw::new(c"\x1b[19~", keyc::KEYC_F8 as u64),
    tty_default_key_raw::new(c"\x1b[20~", keyc::KEYC_F9 as u64),
    tty_default_key_raw::new(c"\x1b[21~", keyc::KEYC_F10 as u64),
    tty_default_key_raw::new(c"\x1b[23~", keyc::KEYC_F1 as u64 | KEYC_SHIFT),
    tty_default_key_raw::new(c"\x1b[24~", keyc::KEYC_F2 as u64 | KEYC_SHIFT),
    tty_default_key_raw::new(c"\x1b[25~", keyc::KEYC_F3 as u64 | KEYC_SHIFT),
    tty_default_key_raw::new(c"\x1b[26~", keyc::KEYC_F4 as u64 | KEYC_SHIFT),
    tty_default_key_raw::new(c"\x1b[28~", keyc::KEYC_F5 as u64 | KEYC_SHIFT),
    tty_default_key_raw::new(c"\x1b[29~", keyc::KEYC_F6 as u64 | KEYC_SHIFT),
    tty_default_key_raw::new(c"\x1b[31~", keyc::KEYC_F7 as u64 | KEYC_SHIFT),
    tty_default_key_raw::new(c"\x1b[32~", keyc::KEYC_F8 as u64 | KEYC_SHIFT),
    tty_default_key_raw::new(c"\x1b[33~", keyc::KEYC_F9 as u64 | KEYC_SHIFT),
    tty_default_key_raw::new(c"\x1b[34~", keyc::KEYC_F10 as u64 | KEYC_SHIFT),
    tty_default_key_raw::new(c"\x1b[23$", keyc::KEYC_F11 as u64 | KEYC_SHIFT),
    tty_default_key_raw::new(c"\x1b[24$", keyc::KEYC_F12 as u64 | KEYC_SHIFT),
    tty_default_key_raw::new(c"\x1b[11^", keyc::KEYC_F1 as u64 | KEYC_CTRL),
    tty_default_key_raw::new(c"\x1b[12^", keyc::KEYC_F2 as u64 | KEYC_CTRL),
    tty_default_key_raw::new(c"\x1b[13^", keyc::KEYC_F3 as u64 | KEYC_CTRL),
    tty_default_key_raw::new(c"\x1b[14^", keyc::KEYC_F4 as u64 | KEYC_CTRL),
    tty_default_key_raw::new(c"\x1b[15^", keyc::KEYC_F5 as u64 | KEYC_CTRL),
    tty_default_key_raw::new(c"\x1b[17^", keyc::KEYC_F6 as u64 | KEYC_CTRL),
    tty_default_key_raw::new(c"\x1b[18^", keyc::KEYC_F7 as u64 | KEYC_CTRL),
    tty_default_key_raw::new(c"\x1b[19^", keyc::KEYC_F8 as u64 | KEYC_CTRL),
    tty_default_key_raw::new(c"\x1b[20^", keyc::KEYC_F9 as u64 | KEYC_CTRL),
    tty_default_key_raw::new(c"\x1b[21^", keyc::KEYC_F10 as u64 | KEYC_CTRL),
    tty_default_key_raw::new(c"\x1b[23^", keyc::KEYC_F11 as u64 | KEYC_CTRL),
    tty_default_key_raw::new(c"\x1b[24^", keyc::KEYC_F12 as u64 | KEYC_CTRL),
    tty_default_key_raw::new(c"\x1b[11@", keyc::KEYC_F1 as u64 | KEYC_CTRL | KEYC_SHIFT),
    tty_default_key_raw::new(c"\x1b[12@", keyc::KEYC_F2 as u64 | KEYC_CTRL | KEYC_SHIFT),
    tty_default_key_raw::new(c"\x1b[13@", keyc::KEYC_F3 as u64 | KEYC_CTRL | KEYC_SHIFT),
    tty_default_key_raw::new(c"\x1b[14@", keyc::KEYC_F4 as u64 | KEYC_CTRL | KEYC_SHIFT),
    tty_default_key_raw::new(c"\x1b[15@", keyc::KEYC_F5 as u64 | KEYC_CTRL | KEYC_SHIFT),
    tty_default_key_raw::new(c"\x1b[17@", keyc::KEYC_F6 as u64 | KEYC_CTRL | KEYC_SHIFT),
    tty_default_key_raw::new(c"\x1b[18@", keyc::KEYC_F7 as u64 | KEYC_CTRL | KEYC_SHIFT),
    tty_default_key_raw::new(c"\x1b[19@", keyc::KEYC_F8 as u64 | KEYC_CTRL | KEYC_SHIFT),
    tty_default_key_raw::new(c"\x1b[20@", keyc::KEYC_F9 as u64 | KEYC_CTRL | KEYC_SHIFT),
    tty_default_key_raw::new(c"\x1b[21@", keyc::KEYC_F10 as u64 | KEYC_CTRL | KEYC_SHIFT),
    tty_default_key_raw::new(c"\x1b[23@", keyc::KEYC_F11 as u64 | KEYC_CTRL | KEYC_SHIFT),
    tty_default_key_raw::new(c"\x1b[24@", keyc::KEYC_F12 as u64 | KEYC_CTRL | KEYC_SHIFT),
    // Focus tracking.
    tty_default_key_raw::new(c"\x1b[I", keyc::KEYC_FOCUS_IN as u64),
    tty_default_key_raw::new(c"\x1b[O", keyc::KEYC_FOCUS_OUT as u64),
    // Paste keys.
    tty_default_key_raw::new(c"\x1b[200~", keyc::KEYC_PASTE_START as u64),
    tty_default_key_raw::new(c"\x1b[201~", keyc::KEYC_PASTE_END as u64),
    // Extended keys.
    tty_default_key_raw::new(c"\x1b[1;5Z", '\x09' as u64 | KEYC_CTRL | KEYC_SHIFT),
];

/// Default xterm keys.
#[repr(C)]
struct tty_default_key_xterm {
    template: SyncCharPtr,
    key: key_code,
}
impl tty_default_key_xterm {
    const fn new(template: &'static CStr, key: keyc) -> Self {
        Self {
            template: SyncCharPtr::new(template),
            key: key as key_code,
        }
    }
}

static TTY_DEFAULT_XTERM_KEYS: [tty_default_key_xterm; 30] = [
    tty_default_key_xterm::new(c"\x1b[1;_P", keyc::KEYC_F1),
    tty_default_key_xterm::new(c"\x1bO1;_P", keyc::KEYC_F1),
    tty_default_key_xterm::new(c"\x1bO_P", keyc::KEYC_F1),
    tty_default_key_xterm::new(c"\x1b[1;_Q", keyc::KEYC_F2),
    tty_default_key_xterm::new(c"\x1bO1;_Q", keyc::KEYC_F2),
    tty_default_key_xterm::new(c"\x1bO_Q", keyc::KEYC_F2),
    tty_default_key_xterm::new(c"\x1b[1;_R", keyc::KEYC_F3),
    tty_default_key_xterm::new(c"\x1bO1;_R", keyc::KEYC_F3),
    tty_default_key_xterm::new(c"\x1bO_R", keyc::KEYC_F3),
    tty_default_key_xterm::new(c"\x1b[1;_S", keyc::KEYC_F4),
    tty_default_key_xterm::new(c"\x1bO1;_S", keyc::KEYC_F4),
    tty_default_key_xterm::new(c"\x1bO_S", keyc::KEYC_F4),
    tty_default_key_xterm::new(c"\x1b[15;_~", keyc::KEYC_F5),
    tty_default_key_xterm::new(c"\x1b[17;_~", keyc::KEYC_F6),
    tty_default_key_xterm::new(c"\x1b[18;_~", keyc::KEYC_F7),
    tty_default_key_xterm::new(c"\x1b[19;_~", keyc::KEYC_F8),
    tty_default_key_xterm::new(c"\x1b[20;_~", keyc::KEYC_F9),
    tty_default_key_xterm::new(c"\x1b[21;_~", keyc::KEYC_F10),
    tty_default_key_xterm::new(c"\x1b[23;_~", keyc::KEYC_F11),
    tty_default_key_xterm::new(c"\x1b[24;_~", keyc::KEYC_F12),
    tty_default_key_xterm::new(c"\x1b[1;_A", keyc::KEYC_UP),
    tty_default_key_xterm::new(c"\x1b[1;_B", keyc::KEYC_DOWN),
    tty_default_key_xterm::new(c"\x1b[1;_C", keyc::KEYC_RIGHT),
    tty_default_key_xterm::new(c"\x1b[1;_D", keyc::KEYC_LEFT),
    tty_default_key_xterm::new(c"\x1b[1;_H", keyc::KEYC_HOME),
    tty_default_key_xterm::new(c"\x1b[1;_F", keyc::KEYC_END),
    tty_default_key_xterm::new(c"\x1b[5;_~", keyc::KEYC_PPAGE),
    tty_default_key_xterm::new(c"\x1b[6;_~", keyc::KEYC_NPAGE),
    tty_default_key_xterm::new(c"\x1b[2;_~", keyc::KEYC_IC),
    tty_default_key_xterm::new(c"\x1b[3;_~", keyc::KEYC_DC),
];

static TTY_DEFAULT_XTERM_MODIFIERS: [key_code; 10] = [
    0,
    0,
    KEYC_SHIFT,
    KEYC_META | KEYC_IMPLIED_META,
    KEYC_SHIFT | KEYC_META | KEYC_IMPLIED_META,
    KEYC_CTRL,
    KEYC_SHIFT | KEYC_CTRL,
    KEYC_META | KEYC_IMPLIED_META | KEYC_CTRL,
    KEYC_SHIFT | KEYC_META | KEYC_IMPLIED_META | KEYC_CTRL,
    KEYC_META | KEYC_IMPLIED_META,
];

// Default terminfo(5) keys. Any keys that have builtin modifiers (that is,
// where the key itself contains the modifiers) has the KEYC_XTERM flag set so
// a leading escape is not treated as meta (and probably removed).
#[repr(C)]
struct tty_default_key_code {
    code: tty_code_code,
    key: key_code,
}
impl tty_default_key_code {
    const fn new(code: tty_code_code, key: key_code) -> Self {
        Self { code, key }
    }
}

static TTY_DEFAULT_CODE_KEYS: [tty_default_key_code; 136] = [
    // Function keys.
    tty_default_key_code::new(tty_code_code::TTYC_KF1, keyc::KEYC_F1 as key_code),
    tty_default_key_code::new(tty_code_code::TTYC_KF2, keyc::KEYC_F2 as key_code),
    tty_default_key_code::new(tty_code_code::TTYC_KF3, keyc::KEYC_F3 as key_code),
    tty_default_key_code::new(tty_code_code::TTYC_KF4, keyc::KEYC_F4 as key_code),
    tty_default_key_code::new(tty_code_code::TTYC_KF5, keyc::KEYC_F5 as key_code),
    tty_default_key_code::new(tty_code_code::TTYC_KF6, keyc::KEYC_F6 as key_code),
    tty_default_key_code::new(tty_code_code::TTYC_KF7, keyc::KEYC_F7 as key_code),
    tty_default_key_code::new(tty_code_code::TTYC_KF8, keyc::KEYC_F8 as key_code),
    tty_default_key_code::new(tty_code_code::TTYC_KF9, keyc::KEYC_F9 as key_code),
    tty_default_key_code::new(tty_code_code::TTYC_KF10, keyc::KEYC_F10 as key_code),
    tty_default_key_code::new(tty_code_code::TTYC_KF11, keyc::KEYC_F11 as key_code),
    tty_default_key_code::new(tty_code_code::TTYC_KF12, keyc::KEYC_F12 as key_code),
    tty_default_key_code::new(
        tty_code_code::TTYC_KF13,
        keyc::KEYC_F1 as key_code | KEYC_SHIFT,
    ),
    tty_default_key_code::new(
        tty_code_code::TTYC_KF14,
        keyc::KEYC_F2 as key_code | KEYC_SHIFT,
    ),
    tty_default_key_code::new(
        tty_code_code::TTYC_KF15,
        keyc::KEYC_F3 as key_code | KEYC_SHIFT,
    ),
    tty_default_key_code::new(
        tty_code_code::TTYC_KF16,
        keyc::KEYC_F4 as key_code | KEYC_SHIFT,
    ),
    tty_default_key_code::new(
        tty_code_code::TTYC_KF17,
        keyc::KEYC_F5 as key_code | KEYC_SHIFT,
    ),
    tty_default_key_code::new(
        tty_code_code::TTYC_KF18,
        keyc::KEYC_F6 as key_code | KEYC_SHIFT,
    ),
    tty_default_key_code::new(
        tty_code_code::TTYC_KF19,
        keyc::KEYC_F7 as key_code | KEYC_SHIFT,
    ),
    tty_default_key_code::new(
        tty_code_code::TTYC_KF20,
        keyc::KEYC_F8 as key_code | KEYC_SHIFT,
    ),
    tty_default_key_code::new(
        tty_code_code::TTYC_KF21,
        keyc::KEYC_F9 as key_code | KEYC_SHIFT,
    ),
    tty_default_key_code::new(
        tty_code_code::TTYC_KF22,
        keyc::KEYC_F10 as key_code | KEYC_SHIFT,
    ),
    tty_default_key_code::new(
        tty_code_code::TTYC_KF23,
        keyc::KEYC_F11 as key_code | KEYC_SHIFT,
    ),
    tty_default_key_code::new(
        tty_code_code::TTYC_KF24,
        keyc::KEYC_F12 as key_code | KEYC_SHIFT,
    ),
    tty_default_key_code::new(
        tty_code_code::TTYC_KF25,
        keyc::KEYC_F1 as key_code | KEYC_CTRL,
    ),
    tty_default_key_code::new(
        tty_code_code::TTYC_KF26,
        keyc::KEYC_F2 as key_code | KEYC_CTRL,
    ),
    tty_default_key_code::new(
        tty_code_code::TTYC_KF27,
        keyc::KEYC_F3 as key_code | KEYC_CTRL,
    ),
    tty_default_key_code::new(
        tty_code_code::TTYC_KF28,
        keyc::KEYC_F4 as key_code | KEYC_CTRL,
    ),
    tty_default_key_code::new(
        tty_code_code::TTYC_KF29,
        keyc::KEYC_F5 as key_code | KEYC_CTRL,
    ),
    tty_default_key_code::new(
        tty_code_code::TTYC_KF30,
        keyc::KEYC_F6 as key_code | KEYC_CTRL,
    ),
    tty_default_key_code::new(
        tty_code_code::TTYC_KF31,
        keyc::KEYC_F7 as key_code | KEYC_CTRL,
    ),
    tty_default_key_code::new(
        tty_code_code::TTYC_KF32,
        keyc::KEYC_F8 as key_code | KEYC_CTRL,
    ),
    tty_default_key_code::new(
        tty_code_code::TTYC_KF33,
        keyc::KEYC_F9 as key_code | KEYC_CTRL,
    ),
    tty_default_key_code::new(
        tty_code_code::TTYC_KF34,
        keyc::KEYC_F10 as key_code | KEYC_CTRL,
    ),
    tty_default_key_code::new(
        tty_code_code::TTYC_KF35,
        keyc::KEYC_F11 as key_code | KEYC_CTRL,
    ),
    tty_default_key_code::new(
        tty_code_code::TTYC_KF36,
        keyc::KEYC_F12 as key_code | KEYC_CTRL,
    ),
    tty_default_key_code::new(
        tty_code_code::TTYC_KF37,
        keyc::KEYC_F1 as key_code | KEYC_SHIFT | KEYC_CTRL,
    ),
    tty_default_key_code::new(
        tty_code_code::TTYC_KF38,
        keyc::KEYC_F2 as key_code | KEYC_SHIFT | KEYC_CTRL,
    ),
    tty_default_key_code::new(
        tty_code_code::TTYC_KF39,
        keyc::KEYC_F3 as key_code | KEYC_SHIFT | KEYC_CTRL,
    ),
    tty_default_key_code::new(
        tty_code_code::TTYC_KF40,
        keyc::KEYC_F4 as key_code | KEYC_SHIFT | KEYC_CTRL,
    ),
    tty_default_key_code::new(
        tty_code_code::TTYC_KF41,
        keyc::KEYC_F5 as key_code | KEYC_SHIFT | KEYC_CTRL,
    ),
    tty_default_key_code::new(
        tty_code_code::TTYC_KF42,
        keyc::KEYC_F6 as key_code | KEYC_SHIFT | KEYC_CTRL,
    ),
    tty_default_key_code::new(
        tty_code_code::TTYC_KF43,
        keyc::KEYC_F7 as key_code | KEYC_SHIFT | KEYC_CTRL,
    ),
    tty_default_key_code::new(
        tty_code_code::TTYC_KF44,
        keyc::KEYC_F8 as key_code | KEYC_SHIFT | KEYC_CTRL,
    ),
    tty_default_key_code::new(
        tty_code_code::TTYC_KF45,
        keyc::KEYC_F9 as key_code | KEYC_SHIFT | KEYC_CTRL,
    ),
    tty_default_key_code::new(
        tty_code_code::TTYC_KF46,
        keyc::KEYC_F10 as key_code | KEYC_SHIFT | KEYC_CTRL,
    ),
    tty_default_key_code::new(
        tty_code_code::TTYC_KF47,
        keyc::KEYC_F11 as key_code | KEYC_SHIFT | KEYC_CTRL,
    ),
    tty_default_key_code::new(
        tty_code_code::TTYC_KF48,
        keyc::KEYC_F12 as key_code | KEYC_SHIFT | KEYC_CTRL,
    ),
    tty_default_key_code::new(
        tty_code_code::TTYC_KF49,
        keyc::KEYC_F1 as key_code | KEYC_META | KEYC_IMPLIED_META,
    ),
    tty_default_key_code::new(
        tty_code_code::TTYC_KF50,
        keyc::KEYC_F2 as key_code | KEYC_META | KEYC_IMPLIED_META,
    ),
    tty_default_key_code::new(
        tty_code_code::TTYC_KF51,
        keyc::KEYC_F3 as key_code | KEYC_META | KEYC_IMPLIED_META,
    ),
    tty_default_key_code::new(
        tty_code_code::TTYC_KF52,
        keyc::KEYC_F4 as key_code | KEYC_META | KEYC_IMPLIED_META,
    ),
    tty_default_key_code::new(
        tty_code_code::TTYC_KF53,
        keyc::KEYC_F5 as key_code | KEYC_META | KEYC_IMPLIED_META,
    ),
    tty_default_key_code::new(
        tty_code_code::TTYC_KF54,
        keyc::KEYC_F6 as key_code | KEYC_META | KEYC_IMPLIED_META,
    ),
    tty_default_key_code::new(
        tty_code_code::TTYC_KF55,
        keyc::KEYC_F7 as key_code | KEYC_META | KEYC_IMPLIED_META,
    ),
    tty_default_key_code::new(
        tty_code_code::TTYC_KF56,
        keyc::KEYC_F8 as key_code | KEYC_META | KEYC_IMPLIED_META,
    ),
    tty_default_key_code::new(
        tty_code_code::TTYC_KF57,
        keyc::KEYC_F9 as key_code | KEYC_META | KEYC_IMPLIED_META,
    ),
    tty_default_key_code::new(
        tty_code_code::TTYC_KF58,
        keyc::KEYC_F10 as key_code | KEYC_META | KEYC_IMPLIED_META,
    ),
    tty_default_key_code::new(
        tty_code_code::TTYC_KF59,
        keyc::KEYC_F11 as key_code | KEYC_META | KEYC_IMPLIED_META,
    ),
    tty_default_key_code::new(
        tty_code_code::TTYC_KF60,
        keyc::KEYC_F12 as key_code | KEYC_META | KEYC_IMPLIED_META,
    ),
    tty_default_key_code::new(
        tty_code_code::TTYC_KF61,
        keyc::KEYC_F1 as key_code | KEYC_META | KEYC_IMPLIED_META | KEYC_SHIFT,
    ),
    tty_default_key_code::new(
        tty_code_code::TTYC_KF62,
        keyc::KEYC_F2 as key_code | KEYC_META | KEYC_IMPLIED_META | KEYC_SHIFT,
    ),
    tty_default_key_code::new(
        tty_code_code::TTYC_KF63,
        keyc::KEYC_F3 as key_code | KEYC_META | KEYC_IMPLIED_META | KEYC_SHIFT,
    ),
    tty_default_key_code::new(tty_code_code::TTYC_KICH1, keyc::KEYC_IC as key_code),
    tty_default_key_code::new(tty_code_code::TTYC_KDCH1, keyc::KEYC_DC as key_code),
    tty_default_key_code::new(tty_code_code::TTYC_KHOME, keyc::KEYC_HOME as key_code),
    tty_default_key_code::new(tty_code_code::TTYC_KEND, keyc::KEYC_END as key_code),
    tty_default_key_code::new(tty_code_code::TTYC_KNP, keyc::KEYC_NPAGE as key_code),
    tty_default_key_code::new(tty_code_code::TTYC_KPP, keyc::KEYC_PPAGE as key_code),
    tty_default_key_code::new(tty_code_code::TTYC_KCBT, keyc::KEYC_BTAB as key_code),
    // Arrow keys from terminfo.
    tty_default_key_code::new(
        tty_code_code::TTYC_KCUU1,
        keyc::KEYC_UP as key_code | KEYC_CURSOR,
    ),
    tty_default_key_code::new(
        tty_code_code::TTYC_KCUD1,
        keyc::KEYC_DOWN as key_code | KEYC_CURSOR,
    ),
    tty_default_key_code::new(
        tty_code_code::TTYC_KCUB1,
        keyc::KEYC_LEFT as key_code | KEYC_CURSOR,
    ),
    tty_default_key_code::new(
        tty_code_code::TTYC_KCUF1,
        keyc::KEYC_RIGHT as key_code | KEYC_CURSOR,
    ),
    // Key and modifier capabilities.
    tty_default_key_code::new(
        tty_code_code::TTYC_KDC2,
        keyc::KEYC_DC as key_code | KEYC_SHIFT,
    ),
    tty_default_key_code::new(
        tty_code_code::TTYC_KDC3,
        keyc::KEYC_DC as key_code | KEYC_META | KEYC_IMPLIED_META,
    ),
    tty_default_key_code::new(
        tty_code_code::TTYC_KDC4,
        keyc::KEYC_DC as key_code | KEYC_SHIFT | KEYC_META | KEYC_IMPLIED_META,
    ),
    tty_default_key_code::new(
        tty_code_code::TTYC_KDC5,
        keyc::KEYC_DC as key_code | KEYC_CTRL,
    ),
    tty_default_key_code::new(
        tty_code_code::TTYC_KDC6,
        keyc::KEYC_DC as key_code | KEYC_SHIFT | KEYC_CTRL,
    ),
    tty_default_key_code::new(
        tty_code_code::TTYC_KDC7,
        keyc::KEYC_DC as key_code | KEYC_META | KEYC_IMPLIED_META | KEYC_CTRL,
    ),
    tty_default_key_code::new(
        tty_code_code::TTYC_KIND,
        keyc::KEYC_DOWN as key_code | KEYC_SHIFT,
    ),
    tty_default_key_code::new(
        tty_code_code::TTYC_KDN2,
        keyc::KEYC_DOWN as key_code | KEYC_SHIFT,
    ),
    tty_default_key_code::new(
        tty_code_code::TTYC_KDN3,
        keyc::KEYC_DOWN as key_code | KEYC_META | KEYC_IMPLIED_META,
    ),
    tty_default_key_code::new(
        tty_code_code::TTYC_KDN4,
        keyc::KEYC_DOWN as key_code | KEYC_SHIFT | KEYC_META | KEYC_IMPLIED_META,
    ),
    tty_default_key_code::new(
        tty_code_code::TTYC_KDN5,
        keyc::KEYC_DOWN as key_code | KEYC_CTRL,
    ),
    tty_default_key_code::new(
        tty_code_code::TTYC_KDN6,
        keyc::KEYC_DOWN as key_code | KEYC_SHIFT | KEYC_CTRL,
    ),
    tty_default_key_code::new(
        tty_code_code::TTYC_KDN7,
        keyc::KEYC_DOWN as key_code | KEYC_META | KEYC_IMPLIED_META | KEYC_CTRL,
    ),
    tty_default_key_code::new(
        tty_code_code::TTYC_KEND2,
        keyc::KEYC_END as key_code | KEYC_SHIFT,
    ),
    tty_default_key_code::new(
        tty_code_code::TTYC_KEND3,
        keyc::KEYC_END as key_code | KEYC_META | KEYC_IMPLIED_META,
    ),
    tty_default_key_code::new(
        tty_code_code::TTYC_KEND4,
        keyc::KEYC_END as key_code | KEYC_SHIFT | KEYC_META | KEYC_IMPLIED_META,
    ),
    tty_default_key_code::new(
        tty_code_code::TTYC_KEND5,
        keyc::KEYC_END as key_code | KEYC_CTRL,
    ),
    tty_default_key_code::new(
        tty_code_code::TTYC_KEND6,
        keyc::KEYC_END as key_code | KEYC_SHIFT | KEYC_CTRL,
    ),
    tty_default_key_code::new(
        tty_code_code::TTYC_KEND7,
        keyc::KEYC_END as key_code | KEYC_META | KEYC_IMPLIED_META | KEYC_CTRL,
    ),
    tty_default_key_code::new(
        tty_code_code::TTYC_KHOM2,
        keyc::KEYC_HOME as key_code | KEYC_SHIFT,
    ),
    tty_default_key_code::new(
        tty_code_code::TTYC_KHOM3,
        keyc::KEYC_HOME as key_code | KEYC_META | KEYC_IMPLIED_META,
    ),
    tty_default_key_code::new(
        tty_code_code::TTYC_KHOM4,
        keyc::KEYC_HOME as key_code | KEYC_SHIFT | KEYC_META | KEYC_IMPLIED_META,
    ),
    tty_default_key_code::new(
        tty_code_code::TTYC_KHOM5,
        keyc::KEYC_HOME as key_code | KEYC_CTRL,
    ),
    tty_default_key_code::new(
        tty_code_code::TTYC_KHOM6,
        keyc::KEYC_HOME as key_code | KEYC_SHIFT | KEYC_CTRL,
    ),
    tty_default_key_code::new(
        tty_code_code::TTYC_KHOM7,
        keyc::KEYC_HOME as key_code | KEYC_META | KEYC_IMPLIED_META | KEYC_CTRL,
    ),
    tty_default_key_code::new(
        tty_code_code::TTYC_KIC2,
        keyc::KEYC_IC as key_code | KEYC_SHIFT,
    ),
    tty_default_key_code::new(
        tty_code_code::TTYC_KIC3,
        keyc::KEYC_IC as key_code | KEYC_META | KEYC_IMPLIED_META,
    ),
    tty_default_key_code::new(
        tty_code_code::TTYC_KIC4,
        keyc::KEYC_IC as key_code | KEYC_SHIFT | KEYC_META | KEYC_IMPLIED_META,
    ),
    tty_default_key_code::new(
        tty_code_code::TTYC_KIC5,
        keyc::KEYC_IC as key_code | KEYC_CTRL,
    ),
    tty_default_key_code::new(
        tty_code_code::TTYC_KIC6,
        keyc::KEYC_IC as key_code | KEYC_SHIFT | KEYC_CTRL,
    ),
    tty_default_key_code::new(
        tty_code_code::TTYC_KIC7,
        keyc::KEYC_IC as key_code | KEYC_META | KEYC_IMPLIED_META | KEYC_CTRL,
    ),
    tty_default_key_code::new(
        tty_code_code::TTYC_KLFT2,
        keyc::KEYC_LEFT as key_code | KEYC_SHIFT,
    ),
    tty_default_key_code::new(
        tty_code_code::TTYC_KLFT3,
        keyc::KEYC_LEFT as key_code | KEYC_META | KEYC_IMPLIED_META,
    ),
    tty_default_key_code::new(
        tty_code_code::TTYC_KLFT4,
        keyc::KEYC_LEFT as key_code | KEYC_SHIFT | KEYC_META | KEYC_IMPLIED_META,
    ),
    tty_default_key_code::new(
        tty_code_code::TTYC_KLFT5,
        keyc::KEYC_LEFT as key_code | KEYC_CTRL,
    ),
    tty_default_key_code::new(
        tty_code_code::TTYC_KLFT6,
        keyc::KEYC_LEFT as key_code | KEYC_SHIFT | KEYC_CTRL,
    ),
    tty_default_key_code::new(
        tty_code_code::TTYC_KLFT7,
        keyc::KEYC_LEFT as key_code | KEYC_META | KEYC_IMPLIED_META | KEYC_CTRL,
    ),
    tty_default_key_code::new(
        tty_code_code::TTYC_KNXT2,
        keyc::KEYC_NPAGE as key_code | KEYC_SHIFT,
    ),
    tty_default_key_code::new(
        tty_code_code::TTYC_KNXT3,
        keyc::KEYC_NPAGE as key_code | KEYC_META | KEYC_IMPLIED_META,
    ),
    tty_default_key_code::new(
        tty_code_code::TTYC_KNXT4,
        keyc::KEYC_NPAGE as key_code | KEYC_SHIFT | KEYC_META | KEYC_IMPLIED_META,
    ),
    tty_default_key_code::new(
        tty_code_code::TTYC_KNXT5,
        keyc::KEYC_NPAGE as key_code | KEYC_CTRL,
    ),
    tty_default_key_code::new(
        tty_code_code::TTYC_KNXT6,
        keyc::KEYC_NPAGE as key_code | KEYC_SHIFT | KEYC_CTRL,
    ),
    tty_default_key_code::new(
        tty_code_code::TTYC_KNXT7,
        keyc::KEYC_NPAGE as key_code | KEYC_META | KEYC_IMPLIED_META | KEYC_CTRL,
    ),
    tty_default_key_code::new(
        tty_code_code::TTYC_KPRV2,
        keyc::KEYC_PPAGE as key_code | KEYC_SHIFT,
    ),
    tty_default_key_code::new(
        tty_code_code::TTYC_KPRV3,
        keyc::KEYC_PPAGE as key_code | KEYC_META | KEYC_IMPLIED_META,
    ),
    tty_default_key_code::new(
        tty_code_code::TTYC_KPRV4,
        keyc::KEYC_PPAGE as key_code | KEYC_SHIFT | KEYC_META | KEYC_IMPLIED_META,
    ),
    tty_default_key_code::new(
        tty_code_code::TTYC_KPRV5,
        keyc::KEYC_PPAGE as key_code | KEYC_CTRL,
    ),
    tty_default_key_code::new(
        tty_code_code::TTYC_KPRV6,
        keyc::KEYC_PPAGE as key_code | KEYC_SHIFT | KEYC_CTRL,
    ),
    tty_default_key_code::new(
        tty_code_code::TTYC_KPRV7,
        keyc::KEYC_PPAGE as key_code | KEYC_META | KEYC_IMPLIED_META | KEYC_CTRL,
    ),
    tty_default_key_code::new(
        tty_code_code::TTYC_KRIT2,
        keyc::KEYC_RIGHT as key_code | KEYC_SHIFT,
    ),
    tty_default_key_code::new(
        tty_code_code::TTYC_KRIT3,
        keyc::KEYC_RIGHT as key_code | KEYC_META | KEYC_IMPLIED_META,
    ),
    tty_default_key_code::new(
        tty_code_code::TTYC_KRIT4,
        keyc::KEYC_RIGHT as key_code | KEYC_SHIFT | KEYC_META | KEYC_IMPLIED_META,
    ),
    tty_default_key_code::new(
        tty_code_code::TTYC_KRIT5,
        keyc::KEYC_RIGHT as key_code | KEYC_CTRL,
    ),
    tty_default_key_code::new(
        tty_code_code::TTYC_KRIT6,
        keyc::KEYC_RIGHT as key_code | KEYC_SHIFT | KEYC_CTRL,
    ),
    tty_default_key_code::new(
        tty_code_code::TTYC_KRIT7,
        keyc::KEYC_RIGHT as key_code | KEYC_META | KEYC_IMPLIED_META | KEYC_CTRL,
    ),
    tty_default_key_code::new(
        tty_code_code::TTYC_KRI,
        keyc::KEYC_UP as key_code | KEYC_SHIFT,
    ),
    tty_default_key_code::new(
        tty_code_code::TTYC_KUP2,
        keyc::KEYC_UP as key_code | KEYC_SHIFT,
    ),
    tty_default_key_code::new(
        tty_code_code::TTYC_KUP3,
        keyc::KEYC_UP as key_code | KEYC_META | KEYC_IMPLIED_META,
    ),
    tty_default_key_code::new(
        tty_code_code::TTYC_KUP4,
        keyc::KEYC_UP as key_code | KEYC_SHIFT | KEYC_META | KEYC_IMPLIED_META,
    ),
    tty_default_key_code::new(
        tty_code_code::TTYC_KUP5,
        keyc::KEYC_UP as key_code | KEYC_CTRL,
    ),
    tty_default_key_code::new(
        tty_code_code::TTYC_KUP6,
        keyc::KEYC_UP as key_code | KEYC_SHIFT | KEYC_CTRL,
    ),
    tty_default_key_code::new(
        tty_code_code::TTYC_KUP7,
        keyc::KEYC_UP as key_code | KEYC_META | KEYC_IMPLIED_META | KEYC_CTRL,
    ),
];

/// Add key to tree.
unsafe fn tty_keys_add(tty: *mut tty, s: *const u8, key: key_code) {
    unsafe {
        let mut size: usize = 0;

        let keystr = key_string_lookup_key(key, 1);
        let tk = tty_keys_find(tty, s, strlen(s), &raw mut size);
        if tk.is_null() {
            log_debug!("new key {}: 0x{:x} ({})", _s(s), key, _s(keystr));
            tty_keys_add1(&raw mut (*tty).key_tree, s, key);
        } else {
            log_debug!("replacing key {}: 0x{:x} ({})", _s(s), key, _s(keystr));
            (*tk).key = key;
        }
    }
}

/// Add next node to the tree.
unsafe fn tty_keys_add1(mut tkp: *mut *mut tty_key, mut s: *const u8, key: key_code) {
    unsafe {
        // Allocate a tree entry if there isn't one already.
        let mut tk = *tkp;
        if tk.is_null() {
            *tkp = xcalloc1() as *mut tty_key;
            tk = *tkp;
            (*tk).ch = *s;
            (*tk).key = KEYC_UNKNOWN;
        }

        // Find the next entry.
        if *s == (*tk).ch {
            // Move forward in string.
            s = s.add(1);

            // If this is the end of the string, no more is necessary.
            if *s == b'\0' {
                (*tk).key = key;
                return;
            }

            // Use the child tree for the next character.
            tkp = &raw mut (*tk).next;
        } else if *s < (*tk).ch {
            tkp = &raw mut (*tk).left;
        } else if *s > (*tk).ch {
            tkp = &raw mut (*tk).right;
        }

        // And recurse to add it.
        tty_keys_add1(tkp, s, key);
    }
}

/// Initialise a key tree from the table.
pub unsafe fn tty_keys_build(tty: *mut tty) {
    unsafe {
        let mut copy: [u8; 16] = [0; 16];

        if !(*tty).key_tree.is_null() {
            tty_keys_free(tty);
        }
        (*tty).key_tree = null_mut();

        for tdkx in &TTY_DEFAULT_XTERM_KEYS {
            for (j, tty_default_xterm_modifiers_j) in TTY_DEFAULT_XTERM_MODIFIERS
                .iter()
                .copied()
                .enumerate()
                .skip(2)
            {
                strlcpy(
                    copy.as_mut_ptr(),
                    tdkx.template.as_ptr(),
                    size_of::<[u8; 16]>(),
                );
                copy[libc::strcspn(copy.as_ptr(), c!("_"))] = b'0' + j as u8;

                let key = tdkx.key | tty_default_xterm_modifiers_j;
                tty_keys_add(tty, copy.as_ptr(), key);
            }
        }

        for tdkr in &TTY_DEFAULT_RAW_KEYS {
            let s = tdkr.string.as_ptr();
            if *s != 0 {
                tty_keys_add(tty, s, tdkr.key);
            }
        }

        for tdkc in &TTY_DEFAULT_CODE_KEYS {
            let s = tty_term_string((*tty).term, tdkc.code);
            if *s != 0 {
                tty_keys_add(tty, s, tdkc.key);
            }
        }

        let o = options_get(GLOBAL_OPTIONS, "user-keys");
        if !o.is_null() {
            let mut a = options_array_first(o);
            while !a.is_null() {
                let i = options_array_item_index(a) as u64;
                let ov = options_array_item_value(a);
                tty_keys_add(tty, (*ov).string, KEYC_USER + i);
                a = options_array_next(a);
            }
        }
    }
}

/// Free the entire key tree.
pub unsafe fn tty_keys_free(tty: *mut tty) {
    unsafe {
        tty_keys_free1((*tty).key_tree);
    }
}

/// Free a single key.
unsafe fn tty_keys_free1(tk: *mut tty_key) {
    unsafe {
        if !(*tk).next.is_null() {
            tty_keys_free1((*tk).next);
        }
        if !(*tk).left.is_null() {
            tty_keys_free1((*tk).left);
        }
        if !(*tk).right.is_null() {
            tty_keys_free1((*tk).right);
        }
        free_(tk);
    }
}

/// Lookup a key in the tree.
pub unsafe fn tty_keys_find(
    tty: *mut tty,
    buf: *const u8,
    len: usize,
    size: *mut usize,
) -> *mut tty_key {
    unsafe {
        *size = 0;
        tty_keys_find1((*tty).key_tree, buf, len, size)
    }
}

unsafe fn tty_keys_find1(
    mut tk: *mut tty_key,
    mut buf: *const u8,
    mut len: usize,
    size: *mut usize,
) -> *mut tty_key {
    unsafe {
        // If no data, no match
        if len == 0 {
            return null_mut();
        }

        // If the node is NULL, this is the end of the tree. No match
        if tk.is_null() {
            return null_mut();
        }

        // Pick the next in the sequence
        if (*tk).ch == *buf {
            // Move forward in the string
            buf = buf.add(1);
            len -= 1;
            *size += 1;

            // At the end of the string, return the current node
            if len == 0 || ((*tk).next.is_null() && (*tk).key != KEYC_UNKNOWN) {
                return tk;
            }

            // Move into the next tree for the following character
            tk = (*tk).next;
        } else if *buf < (*tk).ch {
            tk = (*tk).left;
        } else if *buf > (*tk).ch {
            tk = (*tk).right;
        }

        // Move to the next in the tree
        tty_keys_find1(tk, buf, len, size)
    }
}

unsafe fn tty_keys_next1(
    tty: *mut tty,
    buf: *const u8,
    len: usize,
    key: *mut key_code,
    size: *mut usize,
    expired: i32,
) -> i32 {
    unsafe {
        let c = (*tty).client;
        let mut ud: utf8_data = zeroed();
        let mut more: utf8_state;
        let mut uc: utf8_char = zeroed();

        // log_debug!("{}: next key is {} (%.*s) (expired=%d)", _s((*c).name), len, len as i32, buf, expired);

        // Is this a known key?
        let tk = tty_keys_find(tty, buf, len, size);
        if !tk.is_null() && (*tk).key != KEYC_UNKNOWN {
            let mut tk1 = tk;
            loop {
                log_debug!("{}: keys in list: %#{}", _s((*c).name), (*tk1).key);
                tk1 = (*tk1).next;
                if tk1.is_null() {
                    break;
                }
            }
            if !(*tk).next.is_null() && expired == 0 {
                return 1;
            }
            *key = (*tk).key;
            return 0;
        }

        // Is this valid UTF-8?
        more = utf8_open(&mut ud, *buf);
        if more == utf8_state::UTF8_MORE {
            *size = ud.size as usize;
            if len < ud.size as usize {
                if expired == 0 {
                    return 1;
                }
                return -1;
            }
            for i in 1..ud.size {
                more = utf8_append(&mut ud, *buf.add(i as usize));
            }
            if more != utf8_state::UTF8_DONE {
                return -1;
            }

            if utf8_from_data(&raw const ud, &raw mut uc) != utf8_state::UTF8_DONE {
                return -1;
            }
            *key = uc as u64;

            // log_debug!("{}: UTF-8 key %.*s %#llx".as_ptr(), (*c).name, ud.size as i32, ud.data, *key);
            return 0;
        }

        -1
    }
}

// Process at least one key in the buffer. Return 0 if no keys present.

pub unsafe fn tty_keys_next(tty: *mut tty) -> i32 {
    unsafe {
        let c = (*tty).client;
        let mut tv: timeval = zeroed();
        let mut size: usize = 0;
        let mut expired = 0;
        let mut key: key_code = 0;
        let mut onlykey: key_code;
        let mut m: mouse_event = zeroed();

        // Get key buffer.
        let buf = EVBUFFER_DATA((*tty).in_);
        let len = EVBUFFER_LENGTH((*tty).in_);
        if len == 0 {
            return 0;
        }
        // log_debug("%s: keys are %zu (%.*s)", (*c).name, len, (int)len, buf);

        let mut start = true;
        'first_key: loop {
            'discard_key: {
                'complete_key: {
                    'partial_key: {
                        if start {
                            start = false;

                            // Is this a clipboard response?
                            match tty_keys_clipboard(tty, buf.cast(), len, &raw mut size) {
                                0 => {
                                    // yes
                                    key = KEYC_UNKNOWN;
                                    break 'complete_key;
                                }
                                -1 => (),                // no, or not valid
                                1 => break 'partial_key, // partial
                                _ => (),
                            }

                            // Is this a primary device attributes response?
                            match tty_keys_device_attributes(tty, buf.cast(), len, &raw mut size) {
                                0 =>
                                // yes
                                {
                                    key = KEYC_UNKNOWN;
                                    break 'complete_key;
                                }
                                -1 => (),                // no, or not valid
                                1 => break 'partial_key, // partial
                                _ => (),
                            }

                            // Is this a secondary device attributes response?
                            match tty_keys_device_attributes2(tty, buf.cast(), len, &raw mut size) {
                                0 => {
                                    // yes
                                    key = KEYC_UNKNOWN;
                                    break 'complete_key;
                                }
                                -1 => (),                // no, or not valid
                                1 => break 'partial_key, // partial
                                _ => (),
                            }

                            // Is this an extended device attributes response?
                            match tty_keys_extended_device_attributes(
                                tty,
                                buf.cast(),
                                len,
                                &raw mut size,
                            ) {
                                0 => {
                                    // yes
                                    key = KEYC_UNKNOWN;
                                    break 'complete_key;
                                }
                                -1 => (), // no, or not valid
                                1 => break 'partial_key,
                                _ => (),
                            }

                            // Is this a colours response?
                            match tty_keys_colours(
                                tty,
                                buf.cast(),
                                len,
                                &raw mut size,
                                &raw mut (*tty).fg,
                                &raw mut (*tty).bg,
                            ) {
                                0 => {
                                    // yes
                                    key = KEYC_UNKNOWN;
                                    break 'complete_key;
                                }
                                -1 => (), // no, or not valid
                                1 => break 'partial_key,
                                _ => (),
                            }

                            // Is this a mouse key press?
                            #[expect(unused_assignments)]
                            match tty_keys_mouse(tty, buf.cast(), len, &raw mut size, &raw mut m) {
                                0 => {
                                    // yes
                                    key = keyc::KEYC_MOUSE as u64;
                                    break 'complete_key;
                                }
                                -1 => (), // no, or not valid
                                -2 => {
                                    // yes, but we don't care.
                                    key = keyc::KEYC_MOUSE as u64;
                                    break 'discard_key;
                                }
                                1 => break 'partial_key,
                                _ => (),
                            }

                            // Is this an extended key press?
                            match tty_keys_extended_key(
                                tty,
                                buf.cast(),
                                len,
                                &raw mut size,
                                &raw mut key,
                            ) {
                                0 => {
                                    // yes
                                    break 'complete_key;
                                }
                                -1 => (), // no, or not valid
                                1 => break 'partial_key,
                                _ => (),
                            }
                        } // if start

                        // 'first_key:
                        // Try to lookup complete key.
                        let n = tty_keys_next1(
                            tty,
                            buf.cast(),
                            len,
                            &raw mut key,
                            &raw mut size,
                            expired,
                        );
                        if n == 0 {
                            // found
                            break 'complete_key;
                        }
                        if n == 1 {
                            break 'partial_key;
                        }

                        // If not a complete key, look for key with an escape prefix (meta
                        // modifier).
                        if *buf == b'\x1b' && len > 1 {
                            // Look for a key without the escape.
                            let n = tty_keys_next1(
                                tty,
                                buf.add(1).cast(),
                                len - 1,
                                &raw mut key,
                                &raw mut size,
                                expired,
                            );
                            if n == 0 {
                                // found
                                if key & KEYC_IMPLIED_META != 0 {
                                    // We want the escape key as well as the xterm
                                    // key, because the xterm sequence implicitly
                                    // includes the escape (so if we see
                                    // \x1b\x1b[1;3D we know it is an Escape
                                    // followed by M-Left, not just M-Left).
                                    key = b'\x1b' as u64;
                                    size = 1;
                                    break 'complete_key;
                                }
                                key |= KEYC_META;
                                size += 1;
                                break 'complete_key;
                            }
                            if n == 1 {
                                // partial
                                break 'partial_key;
                            }
                        }

                        // At this point, we know the key is not partial (with or without
                        // escape). So pass it through even if the timer has not expired.
                        if *buf == b'\x1b' && len >= 2 {
                            key = *buf.add(1) as u64 | KEYC_META;
                            size = 2;
                        } else {
                            key = *buf as u64;
                            size = 1;
                        }

                        // C-Space is special.
                        if (key & KEYC_MASK_KEY) == c0::C0_NUL as u64 {
                            key = b' ' as u64 | KEYC_CTRL | (key & KEYC_META);
                        }

                        // Fix up all C0 control codes that don't have a dedicated key into
                        // corresponding Ctrl keys. Convert characters in the A-Z range into
                        // lowercase, so ^A becomes a|CTRL.
                        onlykey = key & KEYC_MASK_KEY;
                        if onlykey < 0x20
                            && onlykey != c0::C0_HT as u64
                            && onlykey != c0::C0_CR as u64
                            && onlykey != c0::C0_ESC as u64
                        {
                            onlykey |= 0x40;
                            if onlykey >= b'A' as u64 && onlykey <= b'Z' as u64 {
                                onlykey |= 0x20;
                            }
                            key = onlykey | KEYC_CTRL | (key & KEYC_META);
                        }

                        break 'complete_key;
                    } // partial_key:
                    // log_debug("%s: partial key %.*s", (*c).name, len as i32, buf);

                    // If timer is going, check for expiration.
                    if (*tty).flags.intersects(tty_flags::TTY_TIMER) {
                        if evtimer_initialized(&raw mut (*tty).key_timer)
                            && evtimer_pending(&raw mut (*tty).key_timer, null_mut()) == 0
                        {
                            expired = 1;
                            continue 'first_key;
                        }
                        return 0;
                    }

                    // Get the time period.
                    let mut delay = options_get_number___(&*GLOBAL_OPTIONS, "escape-time");
                    if delay == 0 {
                        delay = 1;
                    }
                    tv.tv_sec = delay / 1000;
                    tv.tv_usec = ((delay % 1000) * 1000) as libc::suseconds_t;

                    // Start the timer.
                    if event_initialized(&raw const (*tty).key_timer) != 0 {
                        evtimer_del(&raw mut (*tty).key_timer);
                    }
                    evtimer_set(
                        &raw mut (*tty).key_timer,
                        tty_keys_callback,
                        NonNull::new(tty).unwrap(),
                    );
                    evtimer_add(&raw mut (*tty).key_timer, &raw const tv);

                    (*tty).flags |= tty_flags::TTY_TIMER;
                    return 0;
                }
                // complete_key:
                // log_debug("%s: complete key %.*s %#llx", (*c).name, (int)size, buf, key);

                // Check for backspace key using termios VERASE - the terminfo
                // kbs entry is extremely unreliable, so cannot be safely
                // used. termios should have a better idea.

                let bspace: libc::cc_t = (*tty).tio.c_cc[libc::VERASE];
                if bspace != libc::_POSIX_VDISABLE && (key & KEYC_MASK_KEY) as libc::cc_t == bspace
                {
                    key = (key & KEYC_MASK_MODIFIERS) | keyc::KEYC_BSPACE as u64;
                }

                // Remove data from buffer.
                evbuffer_drain((*tty).in_, size);

                // Remove key timer.
                if event_initialized(&raw const (*tty).key_timer) != 0 {
                    evtimer_del(&raw mut (*tty).key_timer);
                }
                (*tty).flags &= !tty_flags::TTY_TIMER;

                // Check for focus events.
                if key == keyc::KEYC_FOCUS_OUT as u64 {
                    (*c).flags &= !client_flag::FOCUSED;
                    window_update_focus((*(*(*c).session).curw).window);
                    notify_client(c"client-focus-out", c);
                } else if key == keyc::KEYC_FOCUS_IN as u64 {
                    (*c).flags |= client_flag::FOCUSED;
                    notify_client(c"client-focus-in", c);
                    window_update_focus((*(*(*c).session).curw).window);
                }

                // Fire the key.
                if key != KEYC_UNKNOWN {
                    let event = Box::leak(Box::new(key_event { key, m })) as *mut key_event;
                    if server_client_handle_key(c, event) == 0 {
                        free_(event);
                    }
                }

                return 1;
            }
            // discard_key:

            // log_debug("%s: discard key %.*s %#llx", c->name, (int)size, buf, key);

            // Remove data from buffer.
            evbuffer_drain((*tty).in_, size);

            return 1;
        }
    }
}

/// Key timer callback.
unsafe extern "C-unwind" fn tty_keys_callback(_fd: i32, _events: i16, tty: NonNull<tty>) {
    unsafe {
        if (*tty.as_ptr()).flags.intersects(tty_flags::TTY_TIMER) {
            while tty_keys_next(tty.as_ptr()) != 0 {}
        }
    }
}

/// Handle extended key input. This has two forms: \x1b[27;m;k~ and \x1b[k;mu,
/// where k is key as a number and m is a modifier. Returns 0 for success, -1
/// for failure, 1 for partial;
unsafe fn tty_keys_extended_key(
    tty: *mut tty,
    buf: *const u8,
    len: usize,
    size: *mut usize,
    key: *mut key_code,
) -> i32 {
    unsafe {
        let c = (*tty).client;
        let end: usize = 0;
        let mut number: u32 = 0;
        let mut modifiers: u32 = 0;
        const SIZE_OF_TMP: usize = 64;
        let mut tmp: [u8; 64] = [0; 64];

        let mut ud: utf8_data = zeroed();
        let mut uc: utf8_char = zeroed();

        *size = 0;

        // First two bytes are always \x1b[.
        if *buf != b'\x1b' {
            return -1;
        }
        if len == 1 {
            return 1;
        }
        if *buf.add(1) != b'[' {
            return -1;
        }
        if len == 2 {
            return 1;
        }

        // Look for a terminator. Stop at either '~' or anything that isn't a
        // number or ';'.
        for end in 2..len.min(SIZE_OF_TMP) {
            if *buf.add(end) == b'~' {
                break;
            }
            if !(*buf.add(end)).is_ascii_digit() && *buf.add(end) != b';' {
                break;
            }
        }
        if end == len {
            return 1;
        }
        if end == SIZE_OF_TMP || (*buf.add(end) != b'~' && *buf.add(end) != b'u') {
            return -1;
        }

        // Copy to the buffer.
        libc::memcpy(tmp.as_mut_ptr().cast(), buf.add(2).cast(), end);
        tmp[end] = 0;

        // Try to parse either form of key.
        if *buf.add(end) == b'~' {
            if libc::sscanf(
                tmp.as_ptr().cast(),
                c"27;%u;%u".as_ptr(),
                &raw mut modifiers,
                &raw mut number,
            ) != 2
            {
                return -1;
            }
        } else if libc::sscanf(
            tmp.as_ptr().cast(),
            c"%u;%u".as_ptr(),
            &raw mut number,
            &raw mut modifiers,
        ) != 2
        {
            return -1;
        }
        *size = end + 1;

        // Store the key.

        let bspace: libc::cc_t = (*tty).tio.c_cc[libc::VERASE];

        let mut nkey: key_code = if bspace != libc::_POSIX_VDISABLE && number == bspace as u32 {
            keyc::KEYC_BSPACE as key_code
        } else {
            number as key_code
        };

        // Convert UTF-32 codepoint into internal representation.
        if nkey != keyc::KEYC_BSPACE as key_code && (nkey & !0x7f) != 0 {
            if utf8_fromwc(nkey as wchar_t, &raw mut ud) == utf8_state::UTF8_DONE
                && utf8_from_data(&raw const ud, &raw mut uc) == utf8_state::UTF8_DONE
            {
                nkey = uc as key_code;
            } else {
                return -1;
            }
        }

        // Update the modifiers.
        if modifiers > 0 {
            modifiers -= 1;
            if (modifiers & 1) != 0 {
                nkey |= KEYC_SHIFT;
            }
            if (modifiers & 2) != 0 {
                nkey |= KEYC_META | KEYC_IMPLIED_META; /* Alt */
            }
            if (modifiers & 4) != 0 {
                nkey |= KEYC_CTRL;
            }
            if (modifiers & 8) != 0 {
                nkey |= KEYC_META | KEYC_IMPLIED_META; /* Meta */
            }
        }

        // Convert S-Tab into Backtab.
        if (nkey & KEYC_MASK_KEY) == b'\x09' as key_code && (nkey & KEYC_SHIFT) != 0 {
            nkey = (keyc::KEYC_BTAB as u64) | (nkey & !KEYC_MASK_KEY & !KEYC_SHIFT);
        }

        // Deal with the Shift modifier when present alone. The problem is that
        // in mode 2 some terminals would report shifted keys, like S-a, as
        // just A, and some as S-A.
        //
        // Because we need an unambiguous internal representation, and because
        // restoring the Shift modifier when it's missing would require knowing
        // the keyboard layout, and because S-A would cause a lot of issues
        // downstream, we choose to lose the Shift for all printable
        // characters.
        //
        // That still leaves some ambiguity, such as C-S-A vs. C-A, but that's
        // OK, and applications can handle that.
        let onlykey: key_code = nkey & KEYC_MASK_KEY;
        if ((onlykey > 0x20 && onlykey < 0x7f) || KEYC_IS_UNICODE(nkey))
            && (nkey & KEYC_MASK_MODIFIERS) == KEYC_SHIFT
        {
            nkey &= !KEYC_SHIFT;
        }

        if log_get_level() != 0 {
            log_debug!(
                "{0}: extended key {2:1$} is {3:#x} ({4})",
                _s((*c).name),
                *size,
                _s(buf),
                nkey,
                _s(key_string_lookup_key(nkey, 1))
            );
        }

        *key = nkey;
        0
    }
}

/// Handle mouse key input. Returns 0 for success, -1 for failure, 1 for partial
/// (probably a mouse sequence but need more data), -2 if an invalid mouse
/// sequence.
unsafe fn tty_keys_mouse(
    tty: *mut tty,
    buf: *const u8,
    len: usize,
    size: *mut usize,
    m: *mut mouse_event,
) -> i32 {
    unsafe {
        let c = (*tty).client;
        let mut x: u32 = 0;
        let mut y: u32 = 0;
        let mut b: u32 = 0;
        let mut sgr_b: u32 = 0;
        let mut sgr_type: u8 = b' ';
        let mut ch: u8;

        // Standard mouse sequences are \x1b[M followed by three characters
        // indicating button, X and Y, all based at 32 with 1,1 top-left.
        //
        // UTF-8 mouse sequences are similar but the three are expressed as
        // UTF-8 characters.
        //
        // SGR extended mouse sequences are \x1b[< followed by three numbers in
        // decimal and separated by semicolons indicating button, X and Y. A
        // trailing 'M' is click or scroll and trailing 'm' release. All are
        // based at 0 with 1,1 top-left.

        *size = 0;

        // First two bytes are always \x1b[.
        if *buf != b'\x1b' {
            return -1;
        }
        if len == 1 {
            return 1;
        }
        if *buf.add(1) != b'[' {
            return -1;
        }
        if len == 2 {
            return 1;
        }

        // Third byte is M in old standard (and UTF-8 extension which we do not
        // support), < in SGR extension.
        if *buf.add(2) == b'M' {
            // Read the three inputs.
            *size = 3;
            for i in 0..3 {
                if len <= *size {
                    return 1;
                }
                ch = *buf.add(*size);
                *size += 1;
                if i == 0 {
                    b = ch as u32;
                } else if i == 1 {
                    x = ch as u32;
                } else {
                    y = ch as u32;
                }
            }
            log_debug!("{0}: mouse input: {2:1$}", _s((*c).name), *size, _s(buf));

            // Check and return the mouse input.
            if b < MOUSE_PARAM_BTN_OFF || x < MOUSE_PARAM_POS_OFF || y < MOUSE_PARAM_POS_OFF {
                return -2;
            }
            b -= MOUSE_PARAM_BTN_OFF;
            x -= MOUSE_PARAM_POS_OFF;
            y -= MOUSE_PARAM_POS_OFF;
        } else if *buf.add(2) == b'<' {
            // Read the three inputs.
            *size = 3;
            loop {
                if len <= *size {
                    return 1;
                }
                ch = *buf.add(*size);
                *size += 1;
                if ch == b';' {
                    break;
                }
                if !ch.is_ascii_digit() {
                    return -1;
                }
                sgr_b = 10 * sgr_b + (ch - b'0') as u32;
            }
            loop {
                if len <= *size {
                    return 1;
                }
                ch = *buf.add(*size);
                *size += 1;
                if ch == b';' {
                    break;
                }
                if !ch.is_ascii_digit() {
                    return -1;
                }
                x = 10 * x + (ch - b'0') as u32;
            }
            loop {
                if len <= *size {
                    return 1;
                }
                ch = *buf.add(*size);
                *size += 1;
                if ch == b'M' || ch == b'm' {
                    break;
                }
                if !ch.is_ascii_digit() {
                    return -1;
                }
                y = 10 * y + (ch - b'0') as u32;
            }
            log_debug!(
                "{0}: mouse input (SGR): {2:1$}",
                _s((*c).name),
                *size,
                _s(buf)
            );

            // Check and return the mouse input.
            if x < 1 || y < 1 {
                return -2;
            }
            x -= 1;
            y -= 1;
            b = sgr_b;

            // Type is M for press, m for release.
            sgr_type = ch;
            if sgr_type == b'm' {
                b = 3;
            }

            // Some terminals (like PuTTY 0.63) mistakenly send
            // button-release events for scroll-wheel button-press event.
            // Discard it before it reaches any program running inside
            // tmux.
            if sgr_type == b'm' && MOUSE_WHEEL(sgr_b) {
                return -2;
            }
        } else {
            return -1;
        }

        // Fill mouse event.
        (*m).lx = (*tty).mouse_last_x;
        (*m).x = x;
        (*m).ly = (*tty).mouse_last_y;
        (*m).y = y;
        (*m).lb = (*tty).mouse_last_b;
        (*m).b = b;
        (*m).sgr_type = sgr_type as u32;
        (*m).sgr_b = sgr_b;

        // Update last mouse state.
        (*tty).mouse_last_x = x;
        (*tty).mouse_last_y = y;
        (*tty).mouse_last_b = b;

        0
    }
}

// Handle OSC 52 clipboard input. Returns 0 for success, -1 for failure, 1 for
// partial.

unsafe fn tty_keys_clipboard(
    tty: *mut tty,
    mut buf: *const u8,
    len: usize,
    size: *mut usize,
) -> i32 {
    unsafe {
        let c = (*tty).client;
        let mut wp: *mut window_pane;
        let mut end: usize;
        let mut terminator: usize = 0;

        *size = 0;

        // First five bytes are always \x1b]52;.
        if *buf != b'\x1b' {
            return -1;
        }
        if len == 1 {
            return 1;
        }
        if *buf.add(1) != b']' {
            return -1;
        }
        if len == 2 {
            return 1;
        }
        if *buf.add(2) != b'5' {
            return -1;
        }
        if len == 3 {
            return 1;
        }
        if *buf.add(3) != b'2' {
            return -1;
        }
        if len == 4 {
            return 1;
        }
        if *buf.add(4) != b';' {
            return -1;
        }
        if len == 5 {
            return 1;
        }

        // Find the terminator if any.
        end = 5;
        while end < len {
            if *buf.add(end) == b'\x07' {
                terminator = 1;
                break;
            }
            if end > 5 && *buf.add(end - 1) == b'\x1b' && *buf.add(end) == b'\\' {
                terminator = 2;
                break;
            }
            end += 1;
        }
        if end == len {
            return 1;
        }
        *size = end + 1;

        // Skip the initial part.
        buf = buf.add(5);
        end -= 5;

        // Adjust end so that it points to the start of the terminator.
        end -= terminator - 1;

        // Get the second argument.
        while end != 0 && *buf != b';' {
            buf = buf.add(1);
            end -= 1;
        }
        if end == 0 || end == 1 {
            return 0;
        }
        buf = buf.add(1);
        end -= 1;

        // If we did not request this, ignore it.
        if !(*tty).flags.intersects(tty_flags::TTY_OSC52QUERY) {
            return 0;
        }
        (*tty).flags &= !tty_flags::TTY_OSC52QUERY;
        evtimer_del(&raw mut (*tty).clipboard_timer);

        // It has to be a string so copy it.
        let copy: *mut u8 = xmalloc(end + 1).as_ptr().cast();
        libc::memcpy(copy.cast(), buf.cast(), end);
        *copy.add(end) = b'\0';

        // Convert from base64.
        let needed: usize = (end / 4) * 3;
        let out: *mut u8 = xmalloc(needed).as_ptr().cast();
        let outlen: i32 = b64_pton(copy, out.cast(), len);
        if outlen == -1 {
            free_(out);
            free_(copy);
            return 0;
        }
        free_(copy);

        // Create a new paste buffer and forward to panes.
        // log_debug( c!("%s: %.*s\0"), __func__, outlen, out);
        if (*c).flags.intersects(client_flag::CLIPBOARDBUFFER) {
            paste_add(null_mut(), out, outlen as usize);
            (*c).flags &= !client_flag::CLIPBOARDBUFFER;
        }
        let mut i: u32 = 0;
        while i < (*c).clipboard_npanes {
            wp = window_pane_find_by_id(*(*c).clipboard_panes.add(i as usize));
            if !wp.is_null() {
                input_reply_clipboard((*wp).event, out, outlen as usize, c!("\x1b\\"));
            }
            i += 1;
        }
        free_((*c).clipboard_panes);
        (*c).clipboard_panes = null_mut();
        (*c).clipboard_npanes = 0;

        0
    }
}

// Handle primary device attributes input. Returns 0 for success, -1 for
// failure, 1 for partial.

unsafe fn tty_keys_device_attributes(
    tty: *mut tty,
    buf: *const u8,
    len: usize,
    size: *mut usize,
) -> i32 {
    unsafe {
        let c = (*tty).client;
        let features = &raw mut (*c).term_features;
        let mut n: u32 = 0;
        let mut tmp: [u8; 128] = [0; 128];
        let mut endptr: *mut u8 = null_mut();
        let mut p: [u32; 32] = [0; 32];

        *size = 0;
        if (*tty).flags.intersects(tty_flags::TTY_HAVEDA) {
            return -1;
        }

        // First three bytes are always \x1b[?.
        if *buf != b'\x1b' {
            return -1;
        }
        if len == 1 {
            return 1;
        }
        if *buf.add(1) != b'[' {
            return -1;
        }
        if len == 2 {
            return 1;
        }
        if *buf.add(2) != b'?' {
            return -1;
        }
        if len == 3 {
            return 1;
        }

        // Copy the rest up to a c.
        let mut found = false;
        let mut i: usize = 0;
        for j in 0..tmp.len() {
            i = j;
            if 3 + i == len {
                return 1;
            }
            if *buf.add(3 + i) == b'c' {
                found = true;
                break;
            }
            tmp[i] = *buf.add(3 + i);
        }
        if !found {
            return -1;
        }
        tmp[i] = b'\0';
        *size = 4 + i;

        // Convert all arguments to numbers.
        let mut cp = tmp.as_mut_ptr();
        let mut next: *mut u8;
        while {
            next = strsep(&raw mut cp, c!(";"));
            !next.is_null()
        } {
            p[n as usize] = libc::strtoul(next, &raw mut endptr, 10) as u32;
            if *endptr != b'\0' {
                p[n as usize] = 0;
            }
            n += 1;
            if n == p.len() as u32 {
                break;
            }
        }

        // Add terminal features.
        if matches!(p[0], 61..=65) {
            // level 1-5
            for i in 1..n {
                // log_debug( c!("%s: DA feature: %d\0"), (*c).name, p[i as usize]);
                if p[i as usize] == 4 {
                    tty_add_features(features, "sixel", c!(","));
                }
                if p[i as usize] == 21 {
                    tty_add_features(features, "margins", c!(","));
                }
                if p[i as usize] == 28 {
                    tty_add_features(features, "rectfill", c!(","));
                }
            }
        }
        // log_debug( c!("%s: received primary DA %.*s\0"), (*c).name, *size as i32, buf);

        tty_update_features(tty);
        (*tty).flags |= tty_flags::TTY_HAVEDA;

        0
    }
}

/// Handle secondary device attributes input. Returns 0 for success, -1 for failure, 1 for partial.
unsafe fn tty_keys_device_attributes2(
    tty: *mut tty,
    buf: *const u8,
    len: usize,
    size: *mut usize,
) -> i32 {
    unsafe {
        let c = (*tty).client;
        let features = &raw mut (*c).term_features;
        let mut n: u32 = 0;
        let mut tmp: [u8; 128] = [0; 128];
        let mut endptr: *mut u8 = null_mut();
        let mut p: [u32; 32] = [0; 32];

        *size = 0;
        if (*tty).flags.intersects(tty_flags::TTY_HAVEDA2) {
            return -1;
        }

        // First three bytes are always \x1b[>.
        if *buf != b'\x1b' {
            return -1;
        }
        if len == 1 {
            return 1;
        }
        if *buf.add(1) != b'[' {
            return -1;
        }
        if len == 2 {
            return 1;
        }
        if *buf.add(2) != b'>' {
            return -1;
        }
        if len == 3 {
            return 1;
        }

        // Copy the rest up to a c.
        let mut found = false;
        let mut i: usize = 0;
        for j in 0..tmp.len() {
            i = j;
            if 3 + i == len {
                return 1;
            }
            if *buf.add(3 + i) == b'c' {
                found = true;
                break;
            }
            tmp[i] = *buf.add(3 + i);
        }
        if !found {
            return -1;
        }
        tmp[i] = b'\0';
        *size = 4 + i;

        // Convert all arguments to numbers.
        let mut cp = tmp.as_mut_ptr();
        let mut next: *mut u8;
        while {
            next = strsep(&raw mut cp, c!(";"));
            !next.is_null()
        } {
            p[n as usize] = libc::strtoul(next, &raw mut endptr, 10) as u32;
            if *endptr != b'\0' {
                p[n as usize] = 0;
            }
            n += 1;
            if n == p.len() as u32 {
                break;
            }
        }

        // Add terminal features. We add DECSLRM and DECFRA for some
        // identification codes here, notably 64 will catch VT520, even though
        // we can't use level 5 from DA because of VTE.
        match p[0] as u8 {
            b'M' => {
                // mintty
                tty_default_features(features, c!("mintty"), 0);
            }
            b'T' => {
                // tmux
                tty_default_features(features, c!("tmux"), 0);
            }
            b'U' => {
                // rxvt-unicode
                tty_default_features(features, c!("rxvt-unicode"), 0);
            }
            _ => {}
        }
        // log_debug( c!("%s: received secondary DA %.*s\0"), (*c).name, *size as i32, buf);

        tty_update_features(tty);
        (*tty).flags |= tty_flags::TTY_HAVEDA2;

        0
    }
}

// Handle extended device attributes input. Returns 0 for success, -1 for
// failure, 1 for partial.

unsafe fn tty_keys_extended_device_attributes(
    tty: *mut tty,
    buf: *const u8,
    len: usize,
    size: *mut usize,
) -> i32 {
    unsafe {
        let c = (*tty).client;
        let features = &raw mut (*c).term_features;
        let mut i: usize = 0;
        let mut tmp: [u8; 128] = [0; 128];

        *size = 0;
        if (*tty).flags.intersects(tty_flags::TTY_HAVEXDA) {
            return -1;
        }

        // First four bytes are always \x1bP>|.
        if *buf != b'\x1b' {
            return -1;
        }
        if len == 1 {
            return 1;
        }
        if *buf.add(1) != b'P' {
            return -1;
        }
        if len == 2 {
            return 1;
        }
        if *buf.add(2) != b'>' {
            return -1;
        }
        if len == 3 {
            return 1;
        }
        if *buf.add(3) != b'|' {
            return -1;
        }
        if len == 4 {
            return 1;
        }

        // Copy the rest up to \x1b\.
        let mut found = false;
        for j in 0..tmp.len() - 1 {
            i = j;
            if 4 + i == len {
                return 1;
            }
            if *buf.add(4 + i - 1) == b'\x1b' && *buf.add(4 + i) == b'\\' {
                found = true;
                break;
            }
            tmp[i] = *buf.add(4 + i);
        }
        if !found {
            return -1;
        }
        tmp[i - 1] = b'\0';
        *size = 5 + i;

        // Add terminal features.
        if libc::strncmp(tmp.as_ptr(), c!("iTerm2 "), 7) == 0 {
            tty_default_features(features, c!("iTerm2"), 0);
        } else if libc::strncmp(tmp.as_ptr(), c!("tmux "), 5) == 0 {
            tty_default_features(features, c!("tmux"), 0);
        } else if libc::strncmp(tmp.as_ptr(), c!("XTerm("), 6) == 0 {
            tty_default_features(features, c!("XTerm"), 0);
        } else if libc::strncmp(tmp.as_ptr(), c!("mintty "), 7) == 0 {
            tty_default_features(features, c!("mintty"), 0);
        }
        // log_debug( c!("%s: received extended DA %.*s\0"), (*c).name, *size as i32, buf);

        free_((*c).term_type);
        (*c).term_type = xstrdup(tmp.as_ptr()).as_ptr();

        tty_update_features(tty);
        (*tty).flags |= tty_flags::TTY_HAVEXDA;

        0
    }
}

// Handle foreground or background input. Returns 0 for success, -1 for
// failure, 1 for partial.

pub unsafe fn tty_keys_colours(
    tty: *mut tty,
    buf: *const u8,
    len: usize,
    size: *mut usize,
    fg: *mut i32,
    bg: *mut i32,
) -> i32 {
    unsafe {
        let c = (*tty).client;
        let mut tmp: [u8; 128] = [0; 128];

        *size = 0;

        // First four bytes are always \x1b]1 and 0 or 1 and ;.
        if *buf != b'\x1b' {
            return -1;
        }
        if len == 1 {
            return 1;
        }
        if *buf.add(1) != b']' {
            return -1;
        }
        if len == 2 {
            return 1;
        }
        if *buf.add(2) != b'1' {
            return -1;
        }
        if len == 3 {
            return 1;
        }
        if *buf.add(3) != b'0' && *buf.add(3) != b'1' {
            return -1;
        }
        if len == 4 {
            return 1;
        }
        if *buf.add(4) != b';' {
            return -1;
        }
        if len == 5 {
            return 1;
        }

        let mut found = false;
        let mut i: usize = 0;
        // Copy the rest up to \x1b\ or \x07.
        for j in 0..tmp.len() - 1 {
            i = j;
            if 5 + i == len {
                return 1;
            }
            if *buf.add(5 + i - 1) == b'\x1b' && *buf.add(5 + i) == b'\\' {
                found = true;
                break;
            }
            if *buf.add(5 + i) == b'\x07' {
                found = true;
                break;
            }
            tmp[i] = *buf.add(5 + i);
        }
        if !found {
            return -1;
        }
        if tmp[i - 1] == b'\x1b' {
            tmp[i - 1] = b'\0';
        } else {
            tmp[i] = b'\0';
        }
        *size = 6 + i;

        let n: i32 = colour_parse_x11(tmp.as_ptr());
        if n != -1 && *buf.add(3) == b'0' {
            if !c.is_null() {
                // log_debug( c!("%s fg is %s\0"), (*c).name, colour_tostring(n));
            } else {
                // log_debug( c!("fg is %s\0"), colour_tostring(n));
            }
            *fg = n;
        } else if n != -1 {
            if !c.is_null() {
                // log_debug( c!("%s bg is %s\0"), (*c).name, colour_tostring(n));
            } else {
                // log_debug( c!("bg is %s\0"), colour_tostring(n));
            }
            *bg = n;
        }

        0
    }
}

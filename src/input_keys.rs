// Copyright (c) 2007 Nicholas Marriott <nichu8gmail.com>
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

use std::cmp::Ordering;

use crate::*;

use crate::compat::{
    RB_GENERATE,
    tree::{rb_find, rb_foreach, rb_initializer, rb_insert},
};

// Entry in the key tree.
pub struct input_key_entry {
    pub key: key_code,
    pub data: *const u8,

    pub entry: rb_entry<input_key_entry>,
}
pub type input_key_tree = rb_head<input_key_entry>;

impl input_key_entry {
    const fn new(key: key_code, data: &'static CStr) -> Self {
        Self {
            key,
            data: data.as_ptr().cast(),
            entry: unsafe { zeroed() },
        }
    }
}

/// Input key comparison function.
pub fn input_key_cmp(ike1: &input_key_entry, ike2: &input_key_entry) -> Ordering {
    ike1.key.cmp(&ike2.key)
}

RB_GENERATE!(
    input_key_tree,
    input_key_entry,
    entry,
    discr_entry,
    input_key_cmp
);
static mut INPUT_KEY_TREE: input_key_tree = rb_initializer();

const INPUT_KEY_DEFAULTS_LEN: usize = 83;

static mut INPUT_KEY_DEFAULTS: [input_key_entry; 83] = [
    /* Paste keys. */
    input_key_entry::new(keyc::KEYC_PASTE_START as u64, c"\xb11[200~"),
    input_key_entry::new(keyc::KEYC_PASTE_END as u64, c"\xb11[201~"),
    /* Function keys. */
    input_key_entry::new(keyc::KEYC_F1 as u64, c"\xb11OP"),
    input_key_entry::new(keyc::KEYC_F2 as u64, c"\xb11OQ"),
    input_key_entry::new(keyc::KEYC_F3 as u64, c"\xb11OR"),
    input_key_entry::new(keyc::KEYC_F4 as u64, c"\xb11OS"),
    input_key_entry::new(keyc::KEYC_F5 as u64, c"\xb11[15~"),
    input_key_entry::new(keyc::KEYC_F6 as u64, c"\xb11[17~"),
    input_key_entry::new(keyc::KEYC_F7 as u64, c"\xb11[18~"),
    input_key_entry::new(keyc::KEYC_F8 as u64, c"\xb11[19~"),
    input_key_entry::new(keyc::KEYC_F9 as u64, c"\xb11[20~"),
    input_key_entry::new(keyc::KEYC_F10 as u64, c"\xb11[21~"),
    input_key_entry::new(keyc::KEYC_F11 as u64, c"\xb11[23~"),
    input_key_entry::new(keyc::KEYC_F12 as u64, c"\xb11[24~"),
    input_key_entry::new(keyc::KEYC_IC as u64, c"\xb11[2~"),
    input_key_entry::new(keyc::KEYC_DC as u64, c"\xb11[3~"),
    input_key_entry::new(keyc::KEYC_HOME as u64, c"\xb11[1~"),
    input_key_entry::new(keyc::KEYC_END as u64, c"\xb11[4~"),
    input_key_entry::new(keyc::KEYC_NPAGE as u64, c"\xb11[6~"),
    input_key_entry::new(keyc::KEYC_PPAGE as u64, c"\xb11[5~"),
    input_key_entry::new(keyc::KEYC_BTAB as u64, c"\xb11[Z"),
    /* Arrow keys. */
    input_key_entry::new(keyc::KEYC_UP as u64 | KEYC_CURSOR, c"\xb11OA"),
    input_key_entry::new(keyc::KEYC_DOWN as u64 | KEYC_CURSOR, c"\xb11OB"),
    input_key_entry::new(keyc::KEYC_RIGHT as u64 | KEYC_CURSOR, c"\xb11OC"),
    input_key_entry::new(keyc::KEYC_LEFT as u64 | KEYC_CURSOR, c"\xb11OD"),
    input_key_entry::new(keyc::KEYC_UP as u64, c"\xb11[A"),
    input_key_entry::new(keyc::KEYC_DOWN as u64, c"\xb11[B"),
    input_key_entry::new(keyc::KEYC_RIGHT as u64, c"\xb11[C"),
    input_key_entry::new(keyc::KEYC_LEFT as u64, c"\xb11[D"),
    /* Keypad keys. */
    input_key_entry::new(keyc::KEYC_KP_SLASH as u64 | KEYC_KEYPAD, c"\xb11Oo"),
    input_key_entry::new(keyc::KEYC_KP_STAR as u64 | KEYC_KEYPAD, c"\xb11Oj"),
    input_key_entry::new(keyc::KEYC_KP_MINUS as u64 | KEYC_KEYPAD, c"\xb11Om"),
    input_key_entry::new(keyc::KEYC_KP_SEVEN as u64 | KEYC_KEYPAD, c"\xb11Ow"),
    input_key_entry::new(keyc::KEYC_KP_EIGHT as u64 | KEYC_KEYPAD, c"\xb11Ox"),
    input_key_entry::new(keyc::KEYC_KP_NINE as u64 | KEYC_KEYPAD, c"\xb11Oy"),
    input_key_entry::new(keyc::KEYC_KP_PLUS as u64 | KEYC_KEYPAD, c"\xb11Ok"),
    input_key_entry::new(keyc::KEYC_KP_FOUR as u64 | KEYC_KEYPAD, c"\xb11Ot"),
    input_key_entry::new(keyc::KEYC_KP_FIVE as u64 | KEYC_KEYPAD, c"\xb11Ou"),
    input_key_entry::new(keyc::KEYC_KP_SIX as u64 | KEYC_KEYPAD, c"\xb11Ov"),
    input_key_entry::new(keyc::KEYC_KP_ONE as u64 | KEYC_KEYPAD, c"\xb11Oq"),
    input_key_entry::new(keyc::KEYC_KP_TWO as u64 | KEYC_KEYPAD, c"\xb11Or"),
    input_key_entry::new(keyc::KEYC_KP_THREE as u64 | KEYC_KEYPAD, c"\xb11Os"),
    input_key_entry::new(keyc::KEYC_KP_ENTER as u64 | KEYC_KEYPAD, c"\xb11OM"),
    input_key_entry::new(keyc::KEYC_KP_ZERO as u64 | KEYC_KEYPAD, c"\xb11Op"),
    input_key_entry::new(keyc::KEYC_KP_PERIOD as u64 | KEYC_KEYPAD, c"\xb11On"),
    input_key_entry::new(keyc::KEYC_KP_SLASH as u64, c"/"),
    input_key_entry::new(keyc::KEYC_KP_STAR as u64, c"*"),
    input_key_entry::new(keyc::KEYC_KP_MINUS as u64, c"-"),
    input_key_entry::new(keyc::KEYC_KP_SEVEN as u64, c"7"),
    input_key_entry::new(keyc::KEYC_KP_EIGHT as u64, c"8"),
    input_key_entry::new(keyc::KEYC_KP_NINE as u64, c"9"),
    input_key_entry::new(keyc::KEYC_KP_PLUS as u64, c"+"),
    input_key_entry::new(keyc::KEYC_KP_FOUR as u64, c"4"),
    input_key_entry::new(keyc::KEYC_KP_FIVE as u64, c"5"),
    input_key_entry::new(keyc::KEYC_KP_SIX as u64, c"6"),
    input_key_entry::new(keyc::KEYC_KP_ONE as u64, c"1"),
    input_key_entry::new(keyc::KEYC_KP_TWO as u64, c"2"),
    input_key_entry::new(keyc::KEYC_KP_THREE as u64, c"3"),
    input_key_entry::new(keyc::KEYC_KP_ENTER as u64, c"\n"),
    input_key_entry::new(keyc::KEYC_KP_ZERO as u64, c"0"),
    input_key_entry::new(keyc::KEYC_KP_PERIOD as u64, c"."),
    /* Keys with an embedded modifier. */
    input_key_entry::new(keyc::KEYC_F1 as u64 | KEYC_BUILD_MODIFIERS, c"\xb11[1;_P"),
    input_key_entry::new(keyc::KEYC_F2 as u64 | KEYC_BUILD_MODIFIERS, c"\xb11[1;_Q"),
    input_key_entry::new(keyc::KEYC_F3 as u64 | KEYC_BUILD_MODIFIERS, c"\xb11[1;_R"),
    input_key_entry::new(keyc::KEYC_F4 as u64 | KEYC_BUILD_MODIFIERS, c"\xb11[1;_S"),
    input_key_entry::new(keyc::KEYC_F5 as u64 | KEYC_BUILD_MODIFIERS, c"\xb11[15;_~"),
    input_key_entry::new(keyc::KEYC_F6 as u64 | KEYC_BUILD_MODIFIERS, c"\xb11[17;_~"),
    input_key_entry::new(keyc::KEYC_F7 as u64 | KEYC_BUILD_MODIFIERS, c"\xb11[18;_~"),
    input_key_entry::new(keyc::KEYC_F8 as u64 | KEYC_BUILD_MODIFIERS, c"\xb11[19;_~"),
    input_key_entry::new(keyc::KEYC_F9 as u64 | KEYC_BUILD_MODIFIERS, c"\xb11[20;_~"),
    input_key_entry::new(keyc::KEYC_F10 as u64 | KEYC_BUILD_MODIFIERS, c"\xb11[21;_~"),
    input_key_entry::new(keyc::KEYC_F11 as u64 | KEYC_BUILD_MODIFIERS, c"\xb11[23;_~"),
    input_key_entry::new(keyc::KEYC_F12 as u64 | KEYC_BUILD_MODIFIERS, c"\xb11[24;_~"),
    input_key_entry::new(keyc::KEYC_UP as u64 | KEYC_BUILD_MODIFIERS, c"\xb11[1;_A"),
    input_key_entry::new(keyc::KEYC_DOWN as u64 | KEYC_BUILD_MODIFIERS, c"\xb11[1;_B"),
    input_key_entry::new(
        keyc::KEYC_RIGHT as u64 | KEYC_BUILD_MODIFIERS,
        c"\xb11[1;_C",
    ),
    input_key_entry::new(keyc::KEYC_LEFT as u64 | KEYC_BUILD_MODIFIERS, c"\xb11[1;_D"),
    input_key_entry::new(keyc::KEYC_HOME as u64 | KEYC_BUILD_MODIFIERS, c"\xb11[1;_H"),
    input_key_entry::new(keyc::KEYC_END as u64 | KEYC_BUILD_MODIFIERS, c"\xb11[1;_F"),
    input_key_entry::new(
        keyc::KEYC_PPAGE as u64 | KEYC_BUILD_MODIFIERS,
        c"\xb11[5;_~",
    ),
    input_key_entry::new(
        keyc::KEYC_NPAGE as u64 | KEYC_BUILD_MODIFIERS,
        c"\xb11[6;_~",
    ),
    input_key_entry::new(keyc::KEYC_IC as u64 | KEYC_BUILD_MODIFIERS, c"\xb11[2;_~"),
    input_key_entry::new(keyc::KEYC_DC as u64 | KEYC_BUILD_MODIFIERS, c"\xb11[3;_~"),
];

static INPUT_KEY_MODIFIERS: [key_code; 9] = [
    0,
    0,
    KEYC_SHIFT,
    KEYC_META | KEYC_IMPLIED_META,
    KEYC_SHIFT | KEYC_META | KEYC_IMPLIED_META,
    KEYC_CTRL,
    KEYC_SHIFT | KEYC_CTRL,
    KEYC_META | KEYC_IMPLIED_META | KEYC_CTRL,
    KEYC_SHIFT | KEYC_META | KEYC_IMPLIED_META | KEYC_CTRL,
];

/// Look for key in tree.
pub unsafe fn input_key_get(key: key_code) -> *mut input_key_entry {
    unsafe {
        let mut entry = MaybeUninit::<input_key_entry>::uninit();
        (*entry.as_mut_ptr()).key = key;
        rb_find(&raw mut INPUT_KEY_TREE, entry.as_mut_ptr())
    }
}

pub unsafe fn input_key_split2(c: u32, dst: *mut u8) -> usize {
    unsafe {
        if c > 0x7f {
            *dst = (c >> 6) as u8 | 0xc0;
            *dst.add(1) = (c as u8 & 0x3f) | 0x80;

            2
        } else {
            *dst = c as u8;

            1
        }
    }
}

#[expect(clippy::needless_range_loop)]
/// Build input key tree.
pub unsafe extern "C-unwind" fn input_key_build() {
    unsafe {
        for i in 0..INPUT_KEY_DEFAULTS_LEN {
            let ike = &raw mut INPUT_KEY_DEFAULTS[i];
            if !(*ike).key & KEYC_BUILD_MODIFIERS != 0 {
                rb_insert(&raw mut INPUT_KEY_TREE, ike);
                continue;
            }

            for (j, input_key_modifiers_j) in
                INPUT_KEY_MODIFIERS.iter().cloned().enumerate().skip(2)
            {
                let key = (*ike).key & !KEYC_BUILD_MODIFIERS;
                let data = xstrdup((*ike).data).as_ptr();
                *data.add(libc::strcspn(data, c!("_"))) = b'0' + j as u8;

                let new = xcalloc1::<input_key_entry>();
                new.key = key | input_key_modifiers_j;
                new.data = data;
                rb_insert(&raw mut INPUT_KEY_TREE, new);
            }
        }

        for ike in rb_foreach(&raw mut INPUT_KEY_TREE).map(NonNull::as_ptr) {
            // log_debug_!( "{}:{} : 0x{:x} ({}) is {}", file!(), line!(), (*ike).key, PercentS(key_string_lookup_key((*ike).key, 1)), PercentS((*ike).data),);
        }
    }
}

/// Translate a key code into an output key sequence for a pane.
pub unsafe fn input_key_pane(wp: *mut window_pane, key: key_code, m: *mut mouse_event) -> i32 {
    unsafe {
        if log_get_level() != 0 {
            // log_debug(  c!("writing key 0x%llx (%s) to %%%u"), key, key_string_lookup_key(key, 1), (*wp).id,);
        }

        if KEYC_IS_MOUSE(key) {
            if !m.is_null() && (*m).wp != -1 && (*m).wp as u32 == (*wp).id {
                input_key_mouse(wp, m);
            }
            return 0;
        }
        input_key((*wp).screen, (*wp).event, key)
    }
}

pub unsafe fn input_key_write(
    from: *const u8,
    bev: *mut bufferevent,
    data: *const u8,
    size: usize,
) {
    unsafe {
        log_debug!("input_key_write {0}: {2:1$}", _s(from), size, _s(data));
        bufferevent_write(bev, data.cast(), size);
    }
}

pub unsafe fn input_key_extended(bev: *mut bufferevent, mut key: key_code) -> i32 {
    let __func__ = c!("input_key_extended");
    unsafe {
        let sizeof_tmp = 64;
        let mut tmp = MaybeUninit::<[u8; 64]>::uninit();
        let mut wc: wchar_t = 0;

        const KEYC_SHIFT_OR_META: u64 = KEYC_SHIFT | KEYC_META;
        const KEYC_SHIFT_OR_CTRL: u64 = KEYC_SHIFT | KEYC_CTRL;
        const KEYC_META_OR_CTRL: u64 = KEYC_META | KEYC_CTRL;
        const KEYC_SHIFT_OR_META_OR_CTRL: u64 = KEYC_SHIFT | KEYC_META | KEYC_CTRL;

        let modifier = match key & KEYC_MASK_MODIFIERS {
            KEYC_SHIFT => b'2',
            KEYC_META => b'3',
            KEYC_SHIFT_OR_META => b'4',
            KEYC_CTRL => b'5',
            KEYC_SHIFT_OR_CTRL => b'6',
            KEYC_META_OR_CTRL => b'7',
            KEYC_SHIFT_OR_META_OR_CTRL => b'8',
            _ => return -1,
        };

        if KEYC_IS_UNICODE(key) {
            let ud = utf8_to_data((key & KEYC_MASK_KEY) as u32);
            if utf8_towc(&ud, &raw mut wc) == utf8_state::UTF8_DONE {
                key = wc as u64;
            } else {
                return -1;
            }
        } else {
            key &= KEYC_MASK_KEY;
        }

        if options_get_number_(GLOBAL_OPTIONS, c"extended-keys-format") == 1 {
            xsnprintf_!(
                tmp.as_mut_ptr().cast(),
                sizeof_tmp,
                "\x1b[27;{};{}~",
                modifier as char,
                key,
            );
        } else {
            xsnprintf_!(
                tmp.as_mut_ptr().cast(),
                sizeof_tmp,
                "\x1b[{};{}",
                key,
                modifier as char,
            );
        }

        input_key_write(
            __func__,
            bev,
            tmp.as_ptr().cast(),
            strlen(tmp.as_ptr().cast()),
        );
        0
    }
}

#[expect(
    clippy::manual_c_str_literals,
    reason = "false positive if c string contains NUL"
)]
static STANDARD_MAP: [SyncCharPtr; 2] = [
    SyncCharPtr::from_ptr(c!("1!9(0)=+;:'\",<.>/-8? 2")),
    SyncCharPtr::from_ptr(b"119900=+;;'',,..\x1f\x1f\x7f\x7f\0\0\0".as_ptr().cast()),
];

/// Outputs the key in the "standard" mode. This is by far the most
/// complicated output mode, with a lot of remapping in order to
/// emulate quirks of terminals that today can be only found in museums.
pub unsafe fn input_key_vt10x(bev: *mut bufferevent, mut key: key_code) -> i32 {
    let __func__ = c!("input_key_vt10x");
    unsafe {
        let mut ud: utf8_data = zeroed(); // TODO use uninit

        log_debug!("{}: key in {}", _s(__func__), key);

        if key & KEYC_META != 0 {
            input_key_write(__func__, bev, c!("\x1b"), 1);
        }

        /*
         * There's no way to report modifiers for unicode keys in standard mode
         * so lose the modifiers.
         */
        if KEYC_IS_UNICODE(key) {
            ud = utf8_to_data(key as u32);
            input_key_write(__func__, bev, ud.data.as_ptr().cast(), ud.size as usize);
            return 0;
        }

        /* Prevent TAB and RET from being swallowed by C0 remapping logic. */
        let onlykey: key_code = key & KEYC_MASK_KEY;
        if onlykey == b'\r' as u64 || onlykey == b'\t' as u64 {
            key &= !KEYC_CTRL;
        }

        /*
         * Convert keys with Ctrl modifier into corresponding C0 control codes,
         * with the exception of *some* keys, which are remapped into printable
         * ASCII characters.
         *
         * There is no special handling for Shift modifier, which is pretty
         * much redundant anyway, as no terminal will send <base key>|SHIFT,
         * but only <shifted key>|SHIFT.
         */
        if key & KEYC_CTRL != 0 {
            let p = libc::strchr(STANDARD_MAP[0].as_ptr(), onlykey as i32);
            key = if !p.is_null() {
                *STANDARD_MAP[1]
                    .as_ptr()
                    .add(p.addr() - STANDARD_MAP[0].as_ptr().addr()) as u64
            } else if onlykey >= b'3' as u64 && onlykey <= b'7' as u64 {
                onlykey - b'\x18' as u64
            } else if onlykey >= b'@' as u64 && onlykey <= b'~' as u64 {
                onlykey & 0x1f
            } else {
                return -1;
            };
        }

        log_debug!("{}: key out {}", _s(__func__), key);

        ud.data[0] = (key & 0x7f) as u8;
        input_key_write(__func__, bev, ud.data.as_ptr().cast(), 1);

        0
    }
}

/// Pick keys that are reported as vt10x keys in modifyOtherKeys=1 mode.
pub unsafe fn input_key_mode1(bev: *mut bufferevent, key: key_code) -> i32 {
    unsafe {
        log_debug!("{}: key in {}", "input_key_mode1", key);

        // As per https://invisible-island.net/xterm/modified-keys-us-pc105.html.
        let onlykey = key & KEYC_MASK_KEY;
        if (key & (KEYC_META | KEYC_CTRL)) == KEYC_CTRL
            && (onlykey == ' ' as u64
                || onlykey == '/' as u64
                || onlykey == '@' as u64
                || onlykey == '^' as u64
                || (onlykey >= '2' as u64 && onlykey <= '8' as u64)
                || (onlykey >= '@' as u64 && onlykey <= '~' as u64))
        {
            return input_key_vt10x(bev, key);
        }

        // A regular key + Meta. In the absence of a standard to back this, we mimic what iTerm 2 does.
        if (key & (KEYC_CTRL | KEYC_META)) == KEYC_META {
            return input_key_vt10x(bev, key);
        }
    }

    -1
}

/// Translate a key code into an output key sequence.
pub unsafe fn input_key(s: *mut screen, bev: *mut bufferevent, mut key: key_code) -> i32 {
    let __func__ = c!("input_key");
    unsafe {
        let mut ike: *mut input_key_entry = null_mut();
        let mut ud: utf8_data = zeroed();

        /* Mouse keys need a pane. */
        if KEYC_IS_MOUSE(key) {
            return 0;
        }

        /* Literal keys go as themselves (can't be more than eight bits). */
        if key & KEYC_LITERAL != 0 {
            ud.data[0] = key as u8;
            input_key_write(__func__, bev, ud.data.as_ptr().cast(), 1);
            return 0;
        }

        /* Is this backspace? */
        if (key & KEYC_MASK_KEY) == keyc::KEYC_BSPACE as u64 {
            let mut newkey = options_get_number_(GLOBAL_OPTIONS, c"backspace") as key_code;
            if newkey >= 0x7f {
                newkey = '\x7f' as u64;
            }
            key = newkey | (key & (KEYC_MASK_MODIFIERS | KEYC_MASK_FLAGS));
        }

        /* Is this backtab? */
        if (key & KEYC_MASK_KEY) == keyc::KEYC_BTAB as u64 {
            if (*s).mode.intersects(EXTENDED_KEY_MODES) {
                /* When in xterm extended mode, remap into S-Tab. */
                key = '\x09' as u64 | (key & !KEYC_MASK_KEY) | KEYC_SHIFT;
            } else {
                /* Otherwise clear modifiers. */
                key &= !KEYC_MASK_MODIFIERS;
            }
        }

        /*
         * A trivial case, that is a 7-bit key, excluding C0 control characters
         * that can't be entered from the keyboard, and no modifiers; or a UTF-8
         * key and no modifiers.
         */
        if (key & !KEYC_MASK_KEY) == 0 {
            if key == c0::C0_HT as u64
                || key == c0::C0_CR as u64
                || key == c0::C0_ESC as u64
                || (key >= 0x20 && key <= 0x7f)
            {
                ud.data[0] = key as u8;
                input_key_write(__func__, bev, ud.data.as_ptr().cast(), 1);
                return 0;
            }
            if KEYC_IS_UNICODE(key) {
                ud = utf8_to_data(key as u32);
                input_key_write(__func__, bev, ud.data.as_ptr().cast(), ud.size as usize);
                return 0;
            }
        }

        /*
         * Look up the standard VT10x keys in the tree. If not in application
         * keypad or cursor mode, remove the respective flags from the key.
         */
        if !(*s).mode.intersects(mode_flag::MODE_KKEYPAD) {
            key &= !KEYC_KEYPAD;
        }
        if !(*s).mode.intersects(mode_flag::MODE_KCURSOR) {
            key &= !KEYC_CURSOR;
        }
        if ike.is_null() {
            ike = input_key_get(key);
        }
        if ike.is_null() && (key & KEYC_META != 0) && (!key & KEYC_IMPLIED_META != 0) {
            ike = input_key_get(key & !KEYC_META);
        }
        if ike.is_null() && (key & KEYC_CURSOR != 0) {
            ike = input_key_get(key & !KEYC_CURSOR);
        }
        if ike.is_null() && (key & KEYC_KEYPAD != 0) {
            ike = input_key_get(key & !KEYC_KEYPAD);
        }
        if !ike.is_null() {
            log_debug!(
                "{}: found key 0x{}: \"{}\"",
                _s(__func__),
                key,
                _s((*ike).data)
            );
            if (key == keyc::KEYC_PASTE_START as u64 || key == keyc::KEYC_PASTE_END as u64)
                && !(*s).mode.intersects(mode_flag::MODE_BRACKETPASTE)
            {
                return 0;
            }
            if (key & KEYC_META != 0) && (!key & KEYC_IMPLIED_META != 0) {
                input_key_write(__func__, bev, c!("\x1b"), 1);
            }
            input_key_write(__func__, bev, (*ike).data, strlen((*ike).data));
            return 0;
        }

        /* Ignore internal function key codes. */
        if (key >= KEYC_BASE && key < keyc::KEYC_BASE_END as u64)
            || (key >= KEYC_USER && key < KEYC_USER_END)
        {
            log_debug!("{}: ignoring key 0x{}", _s(__func__), key);
            return 0;
        }

        /*
         * No builtin key sequence; construct an extended key sequence
         * depending on the client mode.
         *
         * If something invalid reaches here, an invalid output may be
         * produced. For example Ctrl-Shift-2 is invalid (as there's
         * no way to enter it). The correct form is Ctrl-Shift-@, at
         * least in US English keyboard layout.
         */
        match (*s).mode & EXTENDED_KEY_MODES {
            mode_flag::MODE_KEYS_EXTENDED_2 =>
            /*
             * The simplest mode to handle - *all* modified keys are
             * reported in the extended form.
             */
            {
                input_key_extended(bev, key)
            }
            mode_flag::MODE_KEYS_EXTENDED => {
                /*
                 * Some keys are still reported in standard mode, to maintain
                 * compatibility with applications unaware of extended keys.
                 */
                if input_key_mode1(bev, key) == -1 {
                    return input_key_extended(bev, key);
                }
                0
            }
            _ =>
            /* The standard mode. */
            {
                input_key_vt10x(bev, key)
            }
        }
    }
}

/// Get mouse event string.
pub unsafe fn input_key_get_mouse(
    s: *mut screen,
    m: *mut mouse_event,
    x: u32,
    y: u32,
    rbuf: *mut *const u8,
    rlen: *mut usize,
) -> i32 {
    static mut BUF: [u8; 40] = [0; 40];
    let len = 0usize;

    unsafe {
        let sizeof_buf = 40;
        *rbuf = null_mut();
        *rlen = 0;

        /* If this pane is not in button or all mode, discard motion events. */
        if MOUSE_DRAG((*m).b) && !(*s).mode.intersects(MOTION_MOUSE_MODES) {
            return 0;
        }
        if !(*s).mode.intersects(ALL_MOUSE_MODES) {
            return 0;
        }

        /*
         * If this event is a release event and not in all mode, discard it.
         * In SGR mode we can tell absolutely because a release is normally
         * shown by the last character. Without SGR, we check if the last
         * buttons was also a release.
         */
        if (*m).sgr_type != b' ' as u32 {
            if MOUSE_DRAG((*m).sgr_b)
                && MOUSE_RELEASE((*m).sgr_b)
                && !(*s).mode.intersects(mode_flag::MODE_MOUSE_ALL)
            {
                return 0;
            }
        } else if MOUSE_DRAG((*m).b)
            && MOUSE_RELEASE((*m).b)
            && MOUSE_RELEASE((*m).lb)
            && !(*s).mode.intersects(mode_flag::MODE_MOUSE_ALL)
        {
            return 0;
        }

        /*
         * Use the SGR (1006) extension only if the application requested it
         * and the underlying terminal also sent the event in this format (this
         * is because an old style mouse release event cannot be converted into
         * the new SGR format, since the released button is unknown). Otherwise
         * pretend that tmux doesn't speak this extension, and fall back to the
         * UTF-8 (1005) extension if the application requested, or to the
         * legacy format.
         */
        let mut len: usize = 0;
        if (*m).sgr_type != ' ' as u32 && (*s).mode.intersects(mode_flag::MODE_MOUSE_SGR) {
            len = xsnprintf_!(
                &raw mut BUF as *mut u8,
                sizeof_buf,
                "\x1b[<{};{};{}{}",
                (*m).sgr_b,
                x + 1,
                y + 1,
                (*m).sgr_type,
            )
            .unwrap() as usize;
        } else if (*s).mode.intersects(mode_flag::MODE_MOUSE_UTF8) {
            if (*m).b > (MOUSE_PARAM_UTF8_MAX - MOUSE_PARAM_BTN_OFF)
                || x > (MOUSE_PARAM_UTF8_MAX - MOUSE_PARAM_POS_OFF)
                || y > (MOUSE_PARAM_UTF8_MAX - MOUSE_PARAM_POS_OFF)
            {
                return 0;
            }
            len = xsnprintf_!(&raw mut BUF as *mut u8, sizeof_buf, "\x1b[M").unwrap() as usize;
            len += input_key_split2((*m).b + MOUSE_PARAM_BTN_OFF, &raw mut BUF[len] as _);
            len += input_key_split2(x + MOUSE_PARAM_POS_OFF, &raw mut BUF[len] as _);
            len += input_key_split2(y + MOUSE_PARAM_POS_OFF, &raw mut BUF[len] as _);
        } else {
            if (*m).b + MOUSE_PARAM_BTN_OFF > MOUSE_PARAM_MAX {
                return 0;
            }

            len = xsnprintf_!(&raw mut BUF as *mut u8, sizeof_buf, "\x1b[M").unwrap() as usize;
            BUF[len] = ((*m).b + MOUSE_PARAM_BTN_OFF) as u8;
            len += 1;

            /*
             * The incoming x and y may be out of the range which can be
             * supported by the "normal" mouse protocol. Clamp the
             * coordinates to the supported range.
             */
            if x + MOUSE_PARAM_POS_OFF > MOUSE_PARAM_MAX {
                BUF[len] = MOUSE_PARAM_MAX as u8;
                len += 1;
            } else {
                BUF[len] = x as u8 + MOUSE_PARAM_POS_OFF as u8;
                len += 1;
            }
            if y + MOUSE_PARAM_POS_OFF > MOUSE_PARAM_MAX {
                BUF[len] = MOUSE_PARAM_MAX as u8;
                len += 1;
            } else {
                BUF[len] = y as u8 + MOUSE_PARAM_POS_OFF as u8;
                len += 1;
            }
        }

        *rbuf = &raw const BUF as *const u8;
        *rlen = len;
    }
    1
}

/// Translate mouse and output.
pub unsafe fn input_key_mouse(wp: *mut window_pane, m: *mut mouse_event) {
    let __func__ = c!("input_key_mouse");
    unsafe {
        let s = (*wp).screen;
        let mut x = 0;
        let mut y = 0;
        let mut buf = null();
        let mut len: usize = 0;

        /* Ignore events if no mouse mode or the pane is not visible. */
        if (*m).ignore != 0 || !(*s).mode.intersects(ALL_MOUSE_MODES) {
            return;
        }
        if cmd_mouse_at(wp, m, &raw mut x, &raw mut y, 0) != 0 {
            return;
        }
        if window_pane_visible(wp) == 0 {
            return;
        }
        if input_key_get_mouse(s, m, x, y, &raw mut buf, &raw mut len) == 0 {
            return;
        }
        log_debug!("writing mouse {1:0$} to %{2}", len, _s(buf), (*wp).id);
        input_key_write(__func__, (*wp).event, buf, len);
    }
}

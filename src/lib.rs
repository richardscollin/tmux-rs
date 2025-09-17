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
#![expect(rustdoc::broken_intra_doc_links, reason = "github markdown callout")]
#![cfg_attr(doc, doc = include_str!("../README.md"))]
#![allow(
    non_camel_case_types,
    reason = "this lint is here instead of in Cargo.toml because of a bug in rust analyzer"
)]

use std::{
    borrow::Cow,
    cmp,
    ffi::{
        CStr, CString, c_int, c_long, c_longlong, c_short, c_uchar, c_uint, c_ulonglong, c_void,
    },
    mem::{MaybeUninit, size_of, zeroed},
    ptr::{NonNull, addr_of, addr_of_mut, null, null_mut},
    sync::{
        Mutex,
        atomic::{self, AtomicBool, AtomicI32, AtomicU32},
    },
};

mod compat;
use compat::{queue::*, strlcat, strlcpy, strtonum, strtonum_, tree::*, vis_flags};

mod ncurses_;
use ncurses_::*;

pub(crate) mod libc;
pub(crate) use libc::errno;
pub(crate) use libc::*;
pub(crate) use libc::{free_, memcpy_, memcpy__, streq_};

#[cfg(feature = "sixel")]
mod image_;
#[cfg(feature = "sixel")]
mod image_sixel;
#[cfg(feature = "sixel")]
use image_sixel::sixel_image;

#[cfg(feature = "utempter")]
mod utempter;

// libevent2
mod event_;
use event_::*;

#[inline]
const fn transmute_ptr<T>(value: Option<NonNull<T>>) -> *mut T {
    match value {
        Some(ptr) => ptr.as_ptr(),
        None => null_mut(),
    }
}

#[inline]
const unsafe fn ptr_to_ref<'a, T>(value: *const T) -> Option<&'a T> {
    unsafe { if value.is_null() { None } else { Some(&*value) } }
}

#[inline]
const unsafe fn ptr_to_mut_ref<'a, T>(value: *mut T) -> Option<&'a mut T> {
    unsafe {
        if value.is_null() {
            None
        } else {
            Some(&mut *value)
        }
    }
}

use compat::imsg::imsg; // TODO move

// use crate::tmux_protocol_h::*;

type bitstr_t = u8;

unsafe fn bit_alloc(nbits: u32) -> *mut u8 {
    unsafe { libc::calloc(nbits.div_ceil(8) as usize, 1).cast() }
}
unsafe fn bit_set(bits: *mut u8, i: u32) {
    unsafe {
        let byte_index = i / 8;
        let bit_index = i % 8;
        *bits.add(byte_index as usize) |= 1 << bit_index;
    }
}

#[inline]
unsafe fn bit_clear(bits: *mut u8, i: u32) {
    unsafe {
        let byte_index = i / 8;
        let bit_index = i % 8;
        *bits.add(byte_index as usize) &= !(1 << bit_index);
    }
}

/// clear bits start..=stop in bitstring
unsafe fn bit_nclear(bits: *mut u8, start: u32, stop: u32) {
    unsafe {
        // TODO this is written inefficiently, assuming the compiler will optimize it. if it doesn't rewrite it
        for i in start..=stop {
            bit_clear(bits, i);
        }
    }
}

unsafe fn bit_test(bits: *const u8, i: u32) -> bool {
    unsafe {
        let byte_index = i / 8;
        let bit_index = i % 8;
        (*bits.add(byte_index as usize) & (1 << bit_index)) != 0
    }
}

const TTY_NAME_MAX: usize = 32;

// discriminant structs
struct discr_alerts_entry;
struct discr_all_entry;
struct discr_by_uri_entry;
struct discr_by_inner_entry;
struct discr_entry;
struct discr_name_entry;
struct discr_pending_entry;
struct discr_sentry;
struct discr_time_entry;
struct discr_tree_entry;
struct discr_wentry;

// /usr/include/paths.h
const _PATH_TTY: *const u8 = c!("/dev/tty");

const _PATH_BSHELL: *const u8 = c!("/bin/sh");
const _PATH_BSHELL_STR: &str = "/bin/sh";

const _PATH_DEFPATH: *const u8 = c!("/usr/bin:/bin");
const _PATH_DEV: *const u8 = c!("/dev/");
const _PATH_DEVNULL: *const u8 = c!("/dev/null");
const _PATH_VI: &str = "/usr/bin/vi";

const SIZEOF_PATH_DEV: usize = 6;

macro_rules! env_or {
    ($key:literal, $default:expr) => {
        match std::option_env!($key) {
            Some(value) => value,
            None => $default,
        }
    };
}

const TMUX_VERSION: &str = env_or!("TMUX_VERSION", env!("CARGO_PKG_VERSION"));
const TMUX_CONF: &str = env_or!(
    "TMUX_CONF",
    "/etc/tmux.conf:~/.tmux.conf:$XDG_CONFIG_HOME/tmux/tmux.conf:~/.config/tmux/tmux.conf"
);
const TMUX_SOCK: &str = env_or!("TMUX_SOCK", "$TMUX_TMPDIR:/tmp/");
const TMUX_TERM: &str = env_or!("TMUX_TERM", "screen");
const TMUX_LOCK_CMD: &str = env_or!("TMUX_LOCK_CMD", "lock -np");

/// Minimum layout cell size, NOT including border lines.
const PANE_MINIMUM: u32 = 1;

/// Automatic name refresh interval, in microseconds. Must be < 1 second.
const NAME_INTERVAL: libc::suseconds_t = 500000;

/// Default pixel cell sizes.
const DEFAULT_XPIXEL: u32 = 16;
const DEFAULT_YPIXEL: u32 = 32;

/// Alert option values
#[repr(i32)]
#[derive(Copy, Clone, num_enum::TryFromPrimitive)]
enum alert_option {
    ALERT_NONE,
    ALERT_ANY,
    ALERT_CURRENT,
    ALERT_OTHER,
}

/// Visual option values
#[repr(i32)]
#[derive(Copy, Clone, Eq, PartialEq, num_enum::TryFromPrimitive)]
enum visual_option {
    VISUAL_OFF,
    VISUAL_ON,
    VISUAL_BOTH,
}

// No key or unknown key.
const KEYC_NONE: c_ulonglong = 0x000ff000000000;
const KEYC_UNKNOWN: c_ulonglong = 0x000fe000000000;

// Base for special (that is, not Unicode) keys. An enum must be at most a
// signed int, so these are based in the highest Unicode PUA.
const KEYC_BASE: c_ulonglong = 0x0000000010e000;
const KEYC_USER: c_ulonglong = 0x0000000010f000;
const KEYC_USER_END: c_ulonglong = KEYC_USER + KEYC_NUSER;

// Key modifier bits
const KEYC_META: c_ulonglong = 0x00100000000000;
const KEYC_CTRL: c_ulonglong = 0x00200000000000;
const KEYC_SHIFT: c_ulonglong = 0x00400000000000;

// Key flag bits.
const KEYC_LITERAL: c_ulonglong = 0x01000000000000;
const KEYC_KEYPAD: c_ulonglong = 0x02000000000000;
const KEYC_CURSOR: c_ulonglong = 0x04000000000000;
const KEYC_IMPLIED_META: c_ulonglong = 0x08000000000000;
const KEYC_BUILD_MODIFIERS: c_ulonglong = 0x10000000000000;
const KEYC_VI: c_ulonglong = 0x20000000000000;
const KEYC_SENT: c_ulonglong = 0x40000000000000;

// Masks for key bits.
const KEYC_MASK_MODIFIERS: c_ulonglong = 0x00f00000000000;
const KEYC_MASK_FLAGS: c_ulonglong = 0xff000000000000;
const KEYC_MASK_KEY: c_ulonglong = 0x000fffffffffff;

const KEYC_NUSER: c_ulonglong = 1000;

#[expect(non_snake_case)]
#[inline(always)]
fn KEYC_IS_MOUSE(key: key_code) -> bool {
    const KEYC_MOUSE: c_ulonglong = keyc::KEYC_MOUSE as c_ulonglong;
    const KEYC_BSPACE: c_ulonglong = keyc::KEYC_BSPACE as c_ulonglong;

    (key & KEYC_MASK_KEY) >= KEYC_MOUSE && (key & KEYC_MASK_KEY) < KEYC_BSPACE
}

#[expect(non_snake_case)]
#[inline(always)]
fn KEYC_IS_UNICODE(key: key_code) -> bool {
    let masked = key & KEYC_MASK_KEY;

    const KEYC_BASE_END: c_ulonglong = keyc::KEYC_BASE_END as c_ulonglong;
    masked > 0x7f
        && !(KEYC_BASE..KEYC_BASE_END).contains(&masked)
        && !(KEYC_USER..KEYC_USER_END).contains(&masked)
}

const KEYC_CLICK_TIMEOUT: i32 = 300;

/// A single key. This can be ASCII or Unicode or one of the keys between
/// KEYC_BASE and KEYC_BASE_END.
type key_code = core::ffi::c_ulonglong;

// skipped C0 control characters

// C0 control characters
#[repr(u64)]
#[derive(Copy, Clone)]
enum c0 {
    C0_NUL,
    C0_SOH,
    C0_STX,
    C0_ETX,
    C0_EOT,
    C0_ENQ,
    C0_ASC,
    C0_BEL,
    C0_BS,
    C0_HT,
    C0_LF,
    C0_VT,
    C0_FF,
    C0_CR,
    C0_SO,
    C0_SI,
    C0_DLE,
    C0_DC1,
    C0_DC2,
    C0_DC3,
    C0_DC4,
    C0_NAK,
    C0_SYN,
    C0_ETB,
    C0_CAN,
    C0_EM,
    C0_SUB,
    C0_ESC,
    C0_FS,
    C0_GS,
    C0_RS,
    C0_US,
}

// idea write a custom top level macro
// which allows me to annotate a variant
// that should be converted to mouse key
// enum mouse_keys {
// KEYC_MOUSE,
//
// #[keyc_mouse_key]
// MOUSEMOVE,
// }
include!("keyc_mouse_key.rs");

/// Termcap codes.
#[repr(u32)]
#[derive(Copy, Clone, num_enum::TryFromPrimitive)]
enum tty_code_code {
    TTYC_ACSC,
    TTYC_AM,
    TTYC_AX,
    TTYC_BCE,
    TTYC_BEL,
    TTYC_BIDI,
    TTYC_BLINK,
    TTYC_BOLD,
    TTYC_CIVIS,
    TTYC_CLEAR,
    TTYC_CLMG,
    TTYC_CMG,
    TTYC_CNORM,
    TTYC_COLORS,
    TTYC_CR,
    TTYC_CS,
    TTYC_CSR,
    TTYC_CUB,
    TTYC_CUB1,
    TTYC_CUD,
    TTYC_CUD1,
    TTYC_CUF,
    TTYC_CUF1,
    TTYC_CUP,
    TTYC_CUU,
    TTYC_CUU1,
    TTYC_CVVIS,
    TTYC_DCH,
    TTYC_DCH1,
    TTYC_DIM,
    TTYC_DL,
    TTYC_DL1,
    TTYC_DSBP,
    TTYC_DSEKS,
    TTYC_DSFCS,
    TTYC_DSMG,
    TTYC_E3,
    TTYC_ECH,
    TTYC_ED,
    TTYC_EL,
    TTYC_EL1,
    TTYC_ENACS,
    TTYC_ENBP,
    TTYC_ENEKS,
    TTYC_ENFCS,
    TTYC_ENMG,
    TTYC_FSL,
    TTYC_HLS,
    TTYC_HOME,
    TTYC_HPA,
    TTYC_ICH,
    TTYC_ICH1,
    TTYC_IL,
    TTYC_IL1,
    TTYC_INDN,
    TTYC_INVIS,
    TTYC_KCBT,
    TTYC_KCUB1,
    TTYC_KCUD1,
    TTYC_KCUF1,
    TTYC_KCUU1,
    TTYC_KDC2,
    TTYC_KDC3,
    TTYC_KDC4,
    TTYC_KDC5,
    TTYC_KDC6,
    TTYC_KDC7,
    TTYC_KDCH1,
    TTYC_KDN2,
    TTYC_KDN3,
    TTYC_KDN4,
    TTYC_KDN5,
    TTYC_KDN6,
    TTYC_KDN7,
    TTYC_KEND,
    TTYC_KEND2,
    TTYC_KEND3,
    TTYC_KEND4,
    TTYC_KEND5,
    TTYC_KEND6,
    TTYC_KEND7,
    TTYC_KF1,
    TTYC_KF10,
    TTYC_KF11,
    TTYC_KF12,
    TTYC_KF13,
    TTYC_KF14,
    TTYC_KF15,
    TTYC_KF16,
    TTYC_KF17,
    TTYC_KF18,
    TTYC_KF19,
    TTYC_KF2,
    TTYC_KF20,
    TTYC_KF21,
    TTYC_KF22,
    TTYC_KF23,
    TTYC_KF24,
    TTYC_KF25,
    TTYC_KF26,
    TTYC_KF27,
    TTYC_KF28,
    TTYC_KF29,
    TTYC_KF3,
    TTYC_KF30,
    TTYC_KF31,
    TTYC_KF32,
    TTYC_KF33,
    TTYC_KF34,
    TTYC_KF35,
    TTYC_KF36,
    TTYC_KF37,
    TTYC_KF38,
    TTYC_KF39,
    TTYC_KF4,
    TTYC_KF40,
    TTYC_KF41,
    TTYC_KF42,
    TTYC_KF43,
    TTYC_KF44,
    TTYC_KF45,
    TTYC_KF46,
    TTYC_KF47,
    TTYC_KF48,
    TTYC_KF49,
    TTYC_KF5,
    TTYC_KF50,
    TTYC_KF51,
    TTYC_KF52,
    TTYC_KF53,
    TTYC_KF54,
    TTYC_KF55,
    TTYC_KF56,
    TTYC_KF57,
    TTYC_KF58,
    TTYC_KF59,
    TTYC_KF6,
    TTYC_KF60,
    TTYC_KF61,
    TTYC_KF62,
    TTYC_KF63,
    TTYC_KF7,
    TTYC_KF8,
    TTYC_KF9,
    TTYC_KHOM2,
    TTYC_KHOM3,
    TTYC_KHOM4,
    TTYC_KHOM5,
    TTYC_KHOM6,
    TTYC_KHOM7,
    TTYC_KHOME,
    TTYC_KIC2,
    TTYC_KIC3,
    TTYC_KIC4,
    TTYC_KIC5,
    TTYC_KIC6,
    TTYC_KIC7,
    TTYC_KICH1,
    TTYC_KIND,
    TTYC_KLFT2,
    TTYC_KLFT3,
    TTYC_KLFT4,
    TTYC_KLFT5,
    TTYC_KLFT6,
    TTYC_KLFT7,
    TTYC_KMOUS,
    TTYC_KNP,
    TTYC_KNXT2,
    TTYC_KNXT3,
    TTYC_KNXT4,
    TTYC_KNXT5,
    TTYC_KNXT6,
    TTYC_KNXT7,
    TTYC_KPP,
    TTYC_KPRV2,
    TTYC_KPRV3,
    TTYC_KPRV4,
    TTYC_KPRV5,
    TTYC_KPRV6,
    TTYC_KPRV7,
    TTYC_KRI,
    TTYC_KRIT2,
    TTYC_KRIT3,
    TTYC_KRIT4,
    TTYC_KRIT5,
    TTYC_KRIT6,
    TTYC_KRIT7,
    TTYC_KUP2,
    TTYC_KUP3,
    TTYC_KUP4,
    TTYC_KUP5,
    TTYC_KUP6,
    TTYC_KUP7,
    TTYC_MS,
    TTYC_NOBR,
    TTYC_OL,
    TTYC_OP,
    TTYC_RECT,
    TTYC_REV,
    TTYC_RGB,
    TTYC_RI,
    TTYC_RIN,
    TTYC_RMACS,
    TTYC_RMCUP,
    TTYC_RMKX,
    TTYC_SE,
    TTYC_SETAB,
    TTYC_SETAF,
    TTYC_SETAL,
    TTYC_SETRGBB,
    TTYC_SETRGBF,
    TTYC_SETULC,
    TTYC_SETULC1,
    TTYC_SGR0,
    TTYC_SITM,
    TTYC_SMACS,
    TTYC_SMCUP,
    TTYC_SMKX,
    TTYC_SMOL,
    TTYC_SMSO,
    TTYC_SMUL,
    TTYC_SMULX,
    TTYC_SMXX,
    TTYC_SXL,
    TTYC_SS,
    TTYC_SWD,
    TTYC_SYNC,
    TTYC_TC,
    TTYC_TSL,
    TTYC_U8,
    TTYC_VPA,
    TTYC_XT,
}

const WHITESPACE: *const u8 = c!(" ");

#[repr(i32)]
#[derive(Copy, Clone, Eq, PartialEq, num_enum::TryFromPrimitive)]
enum modekey {
    MODEKEY_EMACS = 0,
    MODEKEY_VI = 1,
}

bitflags::bitflags! {
    /// Grid flags.
    #[repr(transparent)]
    #[derive(Copy, Clone, Eq, PartialEq)]
    struct mode_flag : i32 {
        const MODE_CURSOR = 0x1;
        const MODE_INSERT = 0x2;
        const MODE_KCURSOR = 0x4;
        const MODE_KKEYPAD = 0x8;
        const MODE_WRAP = 0x10;
        const MODE_MOUSE_STANDARD = 0x20;
        const MODE_MOUSE_BUTTON = 0x40;
        const MODE_CURSOR_BLINKING = 0x80;
        const MODE_MOUSE_UTF8 = 0x100;
        const MODE_MOUSE_SGR = 0x200;
        const MODE_BRACKETPASTE = 0x400;
        const MODE_FOCUSON = 0x800;
        const MODE_MOUSE_ALL = 0x1000;
        const MODE_ORIGIN = 0x2000;
        const MODE_CRLF = 0x4000;
        const MODE_KEYS_EXTENDED = 0x8000;
        const MODE_CURSOR_VERY_VISIBLE = 0x10000;
        const MODE_CURSOR_BLINKING_SET = 0x20000;
        const MODE_KEYS_EXTENDED_2 = 0x40000;
    }
}

#[expect(dead_code)]
const ALL_MODES: i32 = 0xffffff;
const ALL_MOUSE_MODES: mode_flag = mode_flag::MODE_MOUSE_STANDARD
    .union(mode_flag::MODE_MOUSE_BUTTON)
    .union(mode_flag::MODE_MOUSE_ALL);
const MOTION_MOUSE_MODES: mode_flag = mode_flag::MODE_MOUSE_BUTTON.union(mode_flag::MODE_MOUSE_ALL);
const CURSOR_MODES: mode_flag = mode_flag::MODE_CURSOR
    .union(mode_flag::MODE_CURSOR_BLINKING)
    .union(mode_flag::MODE_CURSOR_VERY_VISIBLE);
const EXTENDED_KEY_MODES: mode_flag =
    mode_flag::MODE_KEYS_EXTENDED.union(mode_flag::MODE_KEYS_EXTENDED_2);

// Mouse protocol constants.
const MOUSE_PARAM_MAX: u32 = 0xff;
const MOUSE_PARAM_UTF8_MAX: u32 = 0x7ff;
const MOUSE_PARAM_BTN_OFF: u32 = 0x20;
const MOUSE_PARAM_POS_OFF: u32 = 0x21;

// A single UTF-8 character.
type utf8_char = c_uint;

// An expanded UTF-8 character. UTF8_SIZE must be big enough to hold combining
// characters as well. It can't be more than 32 bytes without changes to how
// characters are stored.
const UTF8_SIZE: usize = 21;

#[repr(C)]
#[derive(Copy, Clone)]
struct utf8_data {
    data: [u8; UTF8_SIZE],

    have: u8,
    size: u8, /* TODO check the codebase for things checking if size == 0, which is the sentinal value */
    /// 0xff if invalid
    width: u8,
}

impl utf8_data {
    const fn new<const N: usize>(data: [u8; N], have: u8, size: u8, width: u8) -> Self {
        if N >= UTF8_SIZE {
            panic!("invalid size");
        }

        let mut padded_data = [0u8; UTF8_SIZE];
        let mut i = 0usize;
        while i < N {
            padded_data[i] = data[i];
            i += 1;
        }

        Self {
            data: padded_data,
            have,
            size,
            width,
        }
    }

    fn initialized_slice(&self) -> &[u8] {
        &self.data[..self.size as usize]
    }
}

#[repr(i32)]
#[derive(Copy, Clone, Eq, PartialEq)]
enum utf8_state {
    UTF8_MORE,
    UTF8_DONE,
    UTF8_ERROR,
}

// Colour flags.
const COLOUR_FLAG_256: i32 = 0x01000000;
const COLOUR_FLAG_RGB: i32 = 0x02000000;

/// Special colours.
#[expect(non_snake_case)]
#[inline]
fn COLOUR_DEFAULT(c: i32) -> bool {
    c == 8 || c == 9
}

// Replacement palette.
#[repr(C)]
#[derive(Clone)]
struct colour_palette {
    fg: i32,
    bg: i32,

    palette: Option<Box<[i32]>>,
    default_palette: Option<Box<[i32]>>,
}

// Grid attributes. Anything above 0xff is stored in an extended cell.
bitflags::bitflags! {
    /// Grid flags.
    #[repr(transparent)]
    #[derive(Copy, Clone, Eq, PartialEq)]
    struct grid_attr : u16 {
        const GRID_ATTR_BRIGHT = 0x1;
        const GRID_ATTR_DIM = 0x2;
        const GRID_ATTR_UNDERSCORE = 0x4;
        const GRID_ATTR_BLINK = 0x8;
        const GRID_ATTR_REVERSE = 0x10;
        const GRID_ATTR_HIDDEN = 0x20;
        const GRID_ATTR_ITALICS = 0x40;
        const GRID_ATTR_CHARSET = 0x80; // alternative character set
        const GRID_ATTR_STRIKETHROUGH = 0x100;
        const GRID_ATTR_UNDERSCORE_2 = 0x200;
        const GRID_ATTR_UNDERSCORE_3 = 0x400;
        const GRID_ATTR_UNDERSCORE_4 = 0x800;
        const GRID_ATTR_UNDERSCORE_5 = 0x1000;
        const GRID_ATTR_OVERLINE = 0x2000;
    }
}

/// All underscore attributes.
const GRID_ATTR_ALL_UNDERSCORE: grid_attr = grid_attr::GRID_ATTR_UNDERSCORE
    .union(grid_attr::GRID_ATTR_UNDERSCORE_2)
    .union(grid_attr::GRID_ATTR_UNDERSCORE_3)
    .union(grid_attr::GRID_ATTR_UNDERSCORE_4)
    .union(grid_attr::GRID_ATTR_UNDERSCORE_5);

bitflags::bitflags! {
    /// Grid flags.
    #[repr(transparent)]
    #[derive(Copy, Clone, Eq, PartialEq)]
    struct grid_flag : u8 {
        const FG256 = 0x1;
        const BG256 = 0x2;
        const PADDING = 0x4;
        const EXTENDED = 0x8;
        const SELECTED = 0x10;
        const NOPALETTE = 0x20;
        const CLEARED = 0x40;
    }
}

bitflags::bitflags! {
    /// Grid line flags.
    #[repr(transparent)]
    #[derive(Copy, Clone, Eq, PartialEq)]
    struct grid_line_flag: i32 {
        const WRAPPED      = 1 << 0; // 0x1
        const EXTENDED     = 1 << 1; // 0x2
        const DEAD         = 1 << 2; // 0x4
        const START_PROMPT = 1 << 3; // 0x8
        const START_OUTPUT = 1 << 4; // 0x10
    }
}

bitflags::bitflags! {
    /// Grid string flags.
    #[repr(transparent)]
    #[derive(Copy, Clone, Eq, PartialEq)]
    struct grid_string_flags: i32 {
        const GRID_STRING_WITH_SEQUENCES = 0x1;
        const GRID_STRING_ESCAPE_SEQUENCES = 0x2;
        const GRID_STRING_TRIM_SPACES = 0x4;
        const GRID_STRING_USED_ONLY = 0x8;
        const GRID_STRING_EMPTY_CELLS = 0x10;
    }
}

/// Cell positions.
#[repr(i32)]
#[derive(Copy, Clone, Eq, PartialEq)]
enum cell_type {
    CELL_INSIDE = 0,
    CELL_TOPBOTTOM = 1,
    CELL_LEFTRIGHT = 2,
    CELL_TOPLEFT = 3,
    CELL_TOPRIGHT = 4,
    CELL_BOTTOMLEFT = 5,
    CELL_BOTTOMRIGHT = 6,
    CELL_TOPJOIN = 7,
    CELL_BOTTOMJOIN = 8,
    CELL_LEFTJOIN = 9,
    CELL_RIGHTJOIN = 10,
    CELL_JOIN = 11,
    CELL_OUTSIDE = 12,
}
use cell_type::*; // TODO remove

// Cell borders.
const CELL_BORDERS: [u8; 13] = [
    b' ', b'x', b'q', b'l', b'k', b'm', b'j', b'w', b'v', b't', b'u', b'n', b'~',
];
const SIMPLE_BORDERS: [u8; 13] = [
    b' ', b'|', b'-', b'+', b'+', b'+', b'+', b'+', b'+', b'+', b'+', b'+', b'.',
];
const PADDED_BORDERS: [u8; 13] = [b' '; 13];

/// Grid cell data.
#[repr(C)]
#[derive(Copy, Clone)]
struct grid_cell {
    data: utf8_data,
    attr: grid_attr,
    flags: grid_flag,
    fg: i32,
    bg: i32,
    us: i32,
    link: u32,
}

impl grid_cell {
    const fn new(
        data: utf8_data,
        attr: grid_attr,
        flags: grid_flag,
        fg: i32,
        bg: i32,
        us: i32,
        link: u32,
    ) -> Self {
        Self {
            data,
            attr,
            flags,
            fg,
            bg,
            us,
            link,
        }
    }
}

/// Grid extended cell entry.
#[repr(C)]
struct grid_extd_entry {
    data: utf8_char,
    attr: u16,
    flags: u8,
    fg: i32,
    bg: i32,
    us: i32,
    link: u32,
}

#[derive(Copy, Clone)]
#[repr(C, align(4))]
struct grid_cell_entry_data {
    attr: u8,
    fg: u8,
    bg: u8,
    data: u8,
}

#[repr(C)]
union grid_cell_entry_union {
    offset: u32,
    data: grid_cell_entry_data,
}

#[repr(C)]
struct grid_cell_entry {
    union_: grid_cell_entry_union,
    flags: grid_flag,
}

/// Grid line.
#[repr(C)]
struct grid_line {
    celldata: *mut grid_cell_entry,
    cellused: u32,
    cellsize: u32,

    extddata: *mut grid_extd_entry,
    extdsize: u32,

    flags: grid_line_flag,
    time: time_t,
}

const GRID_HISTORY: i32 = 0x1; // scroll lines into history

/// Entire grid of cells.
#[repr(C)]
struct grid {
    flags: i32,

    sx: u32,
    sy: u32,

    hscrolled: u32,
    hsize: u32,
    hlimit: u32,

    linedata: *mut grid_line,
}

/// Virtual cursor in a grid.
#[repr(C)]
struct grid_reader {
    gd: *mut grid,
    cx: u32,
    cy: u32,
}

/// Style alignment.
#[repr(i32)]
#[derive(Copy, Clone, Eq, PartialEq)]
enum style_align {
    STYLE_ALIGN_DEFAULT,
    STYLE_ALIGN_LEFT,
    STYLE_ALIGN_CENTRE,
    STYLE_ALIGN_RIGHT,
    STYLE_ALIGN_ABSOLUTE_CENTRE,
}

/// Style list.
#[repr(i32)]
#[derive(Copy, Clone, Eq, PartialEq)]
enum style_list {
    STYLE_LIST_OFF,
    STYLE_LIST_ON,
    STYLE_LIST_FOCUS,
    STYLE_LIST_LEFT_MARKER,
    STYLE_LIST_RIGHT_MARKER,
}

/// Style range.
#[repr(i32)]
#[derive(Copy, Clone, Eq, PartialEq)]
enum style_range_type {
    STYLE_RANGE_NONE,
    STYLE_RANGE_LEFT,
    STYLE_RANGE_RIGHT,
    STYLE_RANGE_PANE,
    STYLE_RANGE_WINDOW,
    STYLE_RANGE_SESSION,
    STYLE_RANGE_USER,
}

impl_tailq_entry!(style_range, entry, tailq_entry<style_range>);
// #[derive(crate::compat::TailQEntry)]
#[repr(C)]
struct style_range {
    type_: style_range_type,
    argument: u32,
    string: [u8; 16],
    start: u32,
    /// not included
    end: u32,

    // #[entry]
    entry: tailq_entry<style_range>,
}
type style_ranges = tailq_head<style_range>;

/// Style default.
#[repr(i32)]
#[derive(Copy, Clone, Eq, PartialEq)]
enum style_default_type {
    STYLE_DEFAULT_BASE,
    STYLE_DEFAULT_PUSH,
    STYLE_DEFAULT_POP,
}

/// Style option.
#[repr(C)]
#[derive(Copy, Clone)]
struct style {
    gc: grid_cell,
    ignore: i32,

    fill: i32,
    align: style_align,
    list: style_list,

    range_type: style_range_type,
    range_argument: u32,
    range_string: [u8; 16],

    default_type: style_default_type,
}

#[cfg(feature = "sixel")]
impl crate::compat::queue::Entry<image, discr_all_entry> for image {
    unsafe fn entry(this: *mut Self) -> *mut tailq_entry<image> {
        unsafe { &raw mut (*this).all_entry }
    }
}
#[cfg(feature = "sixel")]
impl crate::compat::queue::Entry<image, discr_entry> for image {
    unsafe fn entry(this: *mut Self) -> *mut tailq_entry<image> {
        unsafe { &raw mut (*this).entry }
    }
}
#[cfg(feature = "sixel")]
#[repr(C)]
#[derive(Copy, Clone)]
struct image {
    s: *mut screen,
    data: *mut sixel_image,
    fallback: *mut u8,
    px: u32,
    py: u32,
    sx: u32,
    sy: u32,

    all_entry: tailq_entry<image>,
    entry: tailq_entry<image>,
}

#[cfg(feature = "sixel")]
type images = tailq_head<image>;

/// Cursor style.
#[repr(i32)]
#[derive(Copy, Clone, Eq, PartialEq)]
enum screen_cursor_style {
    SCREEN_CURSOR_DEFAULT,
    SCREEN_CURSOR_BLOCK,
    SCREEN_CURSOR_UNDERLINE,
    SCREEN_CURSOR_BAR,
}

/// Virtual screen.
#[repr(C)]
#[derive(Clone)]
struct screen {
    title: *mut u8,
    path: *mut u8,
    titles: *mut screen_titles,

    /// grid data
    grid: *mut grid,

    /// cursor x
    cx: u32,
    /// cursor y
    cy: u32,

    /// cursor style
    cstyle: screen_cursor_style,
    default_cstyle: screen_cursor_style,
    /// cursor colour
    ccolour: i32,
    /// default cursor colour
    default_ccolour: i32,

    /// scroll region top
    rupper: u32,
    /// scroll region bottom
    rlower: u32,

    mode: mode_flag,
    default_mode: mode_flag,

    saved_cx: u32,
    saved_cy: u32,
    saved_grid: *mut grid,
    saved_cell: grid_cell,
    saved_flags: i32,

    tabs: *mut bitstr_t,
    sel: *mut screen_sel,

    #[cfg(feature = "sixel")]
    images: images,

    write_list: *mut screen_write_cline,

    hyperlinks: *mut hyperlinks,
}

const SCREEN_WRITE_SYNC: i32 = 0x1;

// Screen write context.
type screen_write_init_ctx_cb = Option<unsafe fn(*mut screen_write_ctx, *mut tty_ctx)>;
#[repr(C)]
struct screen_write_ctx {
    wp: *mut window_pane,
    s: *mut screen,

    flags: i32,

    init_ctx_cb: screen_write_init_ctx_cb,

    arg: *mut c_void,

    item: *mut screen_write_citem,
    scrolled: u32,
    bg: u32,
}

/// Box border lines option.
#[repr(i32)]
#[derive(Copy, Clone, Default, Eq, PartialEq, num_enum::TryFromPrimitive)]
enum box_lines {
    #[default]
    BOX_LINES_DEFAULT = -1,
    BOX_LINES_SINGLE,
    BOX_LINES_DOUBLE,
    BOX_LINES_HEAVY,
    BOX_LINES_SIMPLE,
    BOX_LINES_ROUNDED,
    BOX_LINES_PADDED,
    BOX_LINES_NONE,
}

/// Pane border lines option.
#[repr(i32)]
#[derive(Copy, Clone, Default, Eq, PartialEq, num_enum::TryFromPrimitive)]
enum pane_lines {
    #[default]
    PANE_LINES_SINGLE,
    PANE_LINES_DOUBLE,
    PANE_LINES_HEAVY,
    PANE_LINES_SIMPLE,
    PANE_LINES_NUMBER,
}

#[repr(i32)]
#[derive(Copy, Clone, num_enum::TryFromPrimitive)]
enum pane_border_indicator {
    PANE_BORDER_OFF,
    PANE_BORDER_COLOUR,
    PANE_BORDER_ARROWS,
    PANE_BORDER_BOTH,
}

// Mode returned by window_pane_mode function.
const WINDOW_PANE_NO_MODE: i32 = 0;
const WINDOW_PANE_COPY_MODE: i32 = 1;
const WINDOW_PANE_VIEW_MODE: i32 = 2;

// Screen redraw context.
#[repr(C)]
struct screen_redraw_ctx {
    c: *mut client,

    statuslines: u32,
    statustop: i32,

    pane_status: pane_status,
    pane_lines: pane_lines,

    no_pane_gc: grid_cell,
    no_pane_gc_set: i32,

    sx: u32,
    sy: u32,
    ox: u32,
    oy: u32,
}

unsafe fn screen_size_x(s: *const screen) -> u32 {
    unsafe { (*(*s).grid).sx }
}
unsafe fn screen_size_y(s: *const screen) -> u32 {
    unsafe { (*(*s).grid).sy }
}
unsafe fn screen_hsize(s: *const screen) -> u32 {
    unsafe { (*(*s).grid).hsize }
}
unsafe fn screen_hlimit(s: *const screen) -> u32 {
    unsafe { (*(*s).grid).hlimit }
}

/// Menu.
#[repr(C)]
#[derive(Default)]
struct menu_item {
    name: SyncCharPtr,
    key: key_code,
    command: SyncCharPtr,
}
impl menu_item {
    const fn new(name: &'static CStr, key: key_code, command: *const u8) -> Self {
        Self {
            name: SyncCharPtr::new(name),
            key,
            command: SyncCharPtr(command),
        }
    }
}

#[repr(C)]
struct menu {
    title: *const u8,
    items: Vec<menu_item>,
    width: u32,
}
type menu_choice_cb = Option<unsafe fn(*mut menu, u32, key_code, *mut c_void)>;

#[expect(clippy::type_complexity)]
/// Window mode. Windows can be in several modes and this is used to call the
/// right function to handle input and output.
#[repr(C)]
struct window_mode {
    name: SyncCharPtr,
    default_format: SyncCharPtr,

    init: unsafe fn(NonNull<window_mode_entry>, *mut cmd_find_state, *mut args) -> *mut screen,
    free: unsafe fn(NonNull<window_mode_entry>),
    resize: unsafe fn(NonNull<window_mode_entry>, u32, u32),
    update: Option<unsafe fn(NonNull<window_mode_entry>)>,
    key: Option<
        unsafe fn(
            NonNull<window_mode_entry>,
            *mut client,
            *mut session,
            *mut winlink,
            key_code,
            *mut mouse_event,
        ),
    >,

    key_table: Option<unsafe fn(*mut window_mode_entry) -> *const u8>,
    command: Option<
        unsafe fn(
            NonNull<window_mode_entry>,
            *mut client,
            *mut session,
            *mut winlink,
            *mut args,
            *mut mouse_event,
        ),
    >,
    formats: Option<unsafe fn(*mut window_mode_entry, *mut format_tree)>,
}

// Active window mode.
impl_tailq_entry!(window_mode_entry, entry, tailq_entry<window_mode_entry>);
#[repr(C)]
struct window_mode_entry {
    wp: *mut window_pane,
    swp: *mut window_pane,

    mode: *const window_mode,
    data: *mut c_void,

    screen: *mut screen,
    prefix: u32,

    // #[entry]
    entry: tailq_entry<window_mode_entry>,
}

/// Offsets into pane buffer.
#[repr(C)]
#[derive(Copy, Clone)]
struct window_pane_offset {
    used: usize,
}

impl_tailq_entry!(window_pane_resize, entry, tailq_entry<window_pane_resize>);
/// Queued pane resize.
#[repr(C)]
struct window_pane_resize {
    sx: u32,
    sy: u32,

    osx: u32,
    osy: u32,

    entry: tailq_entry<window_pane_resize>,
}
type window_pane_resizes = tailq_head<window_pane_resize>;

bitflags::bitflags! {
    #[repr(transparent)]
    #[derive(Copy, Clone, Eq, PartialEq)]
    struct window_pane_flags : i32 {
        const PANE_REDRAW = 0x1;
        const PANE_DROP = 0x2;
        const PANE_FOCUSED = 0x4;
        const PANE_VISITED = 0x8;
        /* 0x10 unused */
        /* 0x20 unused */
        const PANE_INPUTOFF = 0x40;
        const PANE_CHANGED = 0x80;
        const PANE_EXITED = 0x100;
        const PANE_STATUSREADY = 0x200;
        const PANE_STATUSDRAWN = 0x400;
        const PANE_EMPTY = 0x800;
        const PANE_STYLECHANGED = 0x1000;
        const PANE_UNSEENCHANGES = 0x2000;
    }
}

/// Child window structure.
#[repr(C)]
struct window_pane {
    id: u32,
    active_point: u32,

    window: *mut window,
    options: *mut options,

    layout_cell: *mut layout_cell,
    saved_layout_cell: *mut layout_cell,

    sx: u32,
    sy: u32,

    xoff: u32,
    yoff: u32,

    flags: window_pane_flags,

    argc: i32,
    argv: *mut *mut u8,
    shell: *mut u8,
    cwd: *mut u8,

    pid: pid_t,
    tty: [u8; TTY_NAME_MAX],
    status: i32,
    dead_time: timeval,

    fd: i32,
    event: *mut bufferevent,

    offset: window_pane_offset,
    base_offset: usize,

    resize_queue: window_pane_resizes,
    resize_timer: event,

    ictx: *mut input_ctx,

    cached_gc: grid_cell,
    cached_active_gc: grid_cell,
    palette: colour_palette,

    pipe_fd: i32,
    pipe_event: *mut bufferevent,
    pipe_offset: window_pane_offset,

    screen: *mut screen,
    base: screen,

    status_screen: screen,
    status_size: usize,

    modes: tailq_head<window_mode_entry>,

    searchstr: *mut u8,
    searchregex: i32,

    border_gc_set: i32,
    border_gc: grid_cell,

    control_bg: i32,
    control_fg: i32,

    /// link in list of all panes
    entry: tailq_entry<window_pane>,
    /// link in list of last visited
    sentry: tailq_entry<window_pane>,
    tree_entry: rb_entry<window_pane>,
}
type window_panes = tailq_head<window_pane>;
type window_pane_tree = rb_head<window_pane>;

impl Entry<window_pane, discr_entry> for window_pane {
    unsafe fn entry(this: *mut Self) -> *mut tailq_entry<window_pane> {
        unsafe { &raw mut (*this).entry }
    }
}
impl Entry<window_pane, discr_sentry> for window_pane {
    unsafe fn entry(this: *mut Self) -> *mut tailq_entry<window_pane> {
        unsafe { &raw mut (*this).sentry }
    }
}

bitflags::bitflags! {
    #[repr(transparent)]
    #[derive(Copy, Clone, Eq, PartialEq)]
    struct window_flag: i32 {
        const BELL = 0x1;
        const ACTIVITY = 0x2;
        const SILENCE = 0x4;
        const ZOOMED = 0x8;
        const WASZOOMED = 0x10;
        const RESIZE = 0x20;
    }
}
const WINDOW_ALERTFLAGS: window_flag = window_flag::BELL
    .union(window_flag::ACTIVITY)
    .union(window_flag::SILENCE);

/// Window structure.
#[repr(C)]
struct window {
    id: u32,
    latest: *mut c_void,

    name: *mut u8,
    name_event: event,
    name_time: timeval,

    alerts_timer: event,
    offset_timer: event,

    activity_time: timeval,

    active: *mut window_pane,
    last_panes: window_panes,
    panes: window_panes,

    lastlayout: i32,
    layout_root: *mut layout_cell,
    saved_layout_root: *mut layout_cell,
    old_layout: *mut u8,

    sx: u32,
    sy: u32,
    manual_sx: u32,
    manual_sy: u32,
    xpixel: u32,
    ypixel: u32,

    new_sx: u32,
    new_sy: u32,
    new_xpixel: u32,
    new_ypixel: u32,

    fill_character: *mut utf8_data,
    flags: window_flag,

    alerts_queued: i32,
    alerts_entry: tailq_entry<window>,

    options: *mut options,

    references: u32,
    winlinks: tailq_head<winlink>,
    entry: rb_entry<window>,
}
type windows = rb_head<window>;
// crate::compat::impl_rb_tree_protos!(windows, window);

impl crate::compat::queue::Entry<window, discr_alerts_entry> for window {
    unsafe fn entry(this: *mut Self) -> *mut tailq_entry<window> {
        unsafe { &raw mut (*this).alerts_entry }
    }
}

bitflags::bitflags! {
    #[repr(transparent)]
    #[derive(Copy, Clone, Eq, PartialEq)]
    struct winlink_flags: i32 {
        const WINLINK_BELL = 0x1;
        const WINLINK_ACTIVITY = 0x2;
        const WINLINK_SILENCE = 0x4;
        const WINLINK_VISITED = 0x8;
    }
}
const WINLINK_ALERTFLAGS: winlink_flags = winlink_flags::WINLINK_BELL
    .union(winlink_flags::WINLINK_ACTIVITY)
    .union(winlink_flags::WINLINK_SILENCE);

#[repr(C)]
#[derive(Copy, Clone)]
struct winlink {
    idx: i32,
    session: *mut session,
    window: *mut window,

    flags: winlink_flags,

    entry: rb_entry<winlink>,

    wentry: tailq_entry<winlink>,
    sentry: tailq_entry<winlink>,
}

impl crate::compat::queue::Entry<winlink, discr_wentry> for winlink {
    unsafe fn entry(this: *mut Self) -> *mut tailq_entry<winlink> {
        unsafe { &raw mut (*this).wentry }
    }
}

impl crate::compat::queue::Entry<winlink, discr_sentry> for winlink {
    unsafe fn entry(this: *mut Self) -> *mut tailq_entry<winlink> {
        unsafe { &raw mut (*this).sentry }
    }
}

type winlinks = rb_head<winlink>;
// crate::compat::impl_rb_tree_protos!(winlinks, winlink);
type winlink_stack = tailq_head<winlink>;
// crate::compat::impl_rb_tree_protos!(winlink_stack, winlink);

/// Window size option.
#[repr(i32)]
#[derive(Copy, Clone, Eq, PartialEq, num_enum::TryFromPrimitive)]
enum window_size_option {
    WINDOW_SIZE_LARGEST,
    WINDOW_SIZE_SMALLEST,
    WINDOW_SIZE_MANUAL,
    WINDOW_SIZE_LATEST,
}

/// Pane border status option.
#[repr(i32)]
#[derive(Copy, Clone, Eq, PartialEq, num_enum::TryFromPrimitive)]
enum pane_status {
    PANE_STATUS_OFF,
    PANE_STATUS_TOP,
    PANE_STATUS_BOTTOM,
}

/// Layout direction.
#[repr(i32)]
#[derive(Copy, Clone, Eq, PartialEq, num_enum::TryFromPrimitive)]
enum layout_type {
    LAYOUT_LEFTRIGHT,
    LAYOUT_TOPBOTTOM,
    LAYOUT_WINDOWPANE,
}

/// Layout cells queue.
type layout_cells = tailq_head<layout_cell>;

impl_tailq_entry!(layout_cell, entry, tailq_entry<layout_cell>);
/// Layout cell.
#[repr(C)]
struct layout_cell {
    type_: layout_type,

    parent: *mut layout_cell,

    sx: u32,
    sy: u32,

    xoff: u32,
    yoff: u32,

    wp: *mut window_pane,
    cells: layout_cells,

    entry: tailq_entry<layout_cell>,
}

bitflags::bitflags! {
    #[repr(transparent)]
    #[derive(Copy, Clone)]
    struct environ_flags: i32 {
        const ENVIRON_HIDDEN = 0x1;
    }
}
const ENVIRON_HIDDEN: environ_flags = environ_flags::ENVIRON_HIDDEN;

/// Environment variable.
#[repr(C)]
struct environ_entry {
    name: Option<NonNull<u8>>,
    value: Option<NonNull<u8>>,

    flags: environ_flags,
    entry: rb_entry<environ_entry>,
}

/// Client session.
#[repr(C)]
struct session_group {
    name: *const u8,
    sessions: tailq_head<session>,

    entry: rb_entry<session_group>,
}
type session_groups = rb_head<session_group>;

const SESSION_PASTING: i32 = 0x1;
const SESSION_ALERTED: i32 = 0x2;

#[repr(C)]
struct session {
    id: u32,
    name: *mut u8,
    cwd: *mut u8,

    creation_time: timeval,
    last_attached_time: timeval,
    activity_time: timeval,
    last_activity_time: timeval,

    lock_timer: event,

    curw: *mut winlink,
    lastw: winlink_stack,
    windows: winlinks,

    statusat: i32,
    statuslines: u32,

    options: *mut options,

    flags: i32,

    attached: u32,

    tio: *mut termios,

    environ: *mut environ,

    references: i32,

    gentry: tailq_entry<session>,
    entry: rb_entry<session>,
}
type sessions = rb_head<session>;
impl_tailq_entry!(session, gentry, tailq_entry<session>);

const MOUSE_MASK_BUTTONS: u32 = 195;
const MOUSE_MASK_SHIFT: u32 = 4;
const MOUSE_MASK_META: u32 = 8;
const MOUSE_MASK_CTRL: u32 = 16;
const MOUSE_MASK_DRAG: u32 = 32;
const MOUSE_MASK_MODIFIERS: u32 = MOUSE_MASK_SHIFT | MOUSE_MASK_META | MOUSE_MASK_CTRL;

// Mouse wheel type.
const MOUSE_WHEEL_UP: u32 = 64;
const MOUSE_WHEEL_DOWN: u32 = 65;

// Mouse button type.
const MOUSE_BUTTON_1: u32 = 0;
const MOUSE_BUTTON_2: u32 = 1;
const MOUSE_BUTTON_3: u32 = 2;
const MOUSE_BUTTON_6: u32 = 66;
const MOUSE_BUTTON_7: u32 = 67;
const MOUSE_BUTTON_8: u32 = 128;
const MOUSE_BUTTON_9: u32 = 129;
const MOUSE_BUTTON_10: u32 = 130;
const MOUSE_BUTTON_11: u32 = 131;

// Mouse helpers.
#[expect(non_snake_case)]
#[inline]
fn MOUSE_BUTTONS(b: u32) -> u32 {
    b & MOUSE_MASK_BUTTONS
}
#[expect(non_snake_case)]
#[inline]
fn MOUSE_WHEEL(b: u32) -> bool {
    ((b) & MOUSE_MASK_BUTTONS) == MOUSE_WHEEL_UP || ((b) & MOUSE_MASK_BUTTONS) == MOUSE_WHEEL_DOWN
}
#[expect(non_snake_case)]
#[inline]
fn MOUSE_DRAG(b: u32) -> bool {
    b & MOUSE_MASK_DRAG != 0
}
#[expect(non_snake_case)]
#[inline]
fn MOUSE_RELEASE(b: u32) -> bool {
    b & MOUSE_MASK_BUTTONS == 3
}

/// Mouse input.
#[repr(C)]
#[derive(Copy, Clone)]
struct mouse_event {
    valid: bool,
    ignore: i32,

    key: key_code,

    statusat: i32,
    statuslines: u32,

    x: u32,
    y: u32,
    b: u32,

    lx: u32,
    ly: u32,
    lb: u32,

    ox: u32,
    oy: u32,

    s: i32,
    w: i32,
    wp: i32,

    sgr_type: u32,
    sgr_b: u32,
}

/// Key event.
#[repr(C)]
struct key_event {
    key: key_code,
    m: mouse_event,
}

bitflags::bitflags! {
    #[repr(transparent)]
    #[derive(Copy, Clone)]
    struct term_flags: i32 {
        const TERM_256COLOURS = 0x1;
        const TERM_NOAM = 0x2;
        const TERM_DECSLRM = 0x4;
        const TERM_DECFRA = 0x8;
        const TERM_RGBCOLOURS = 0x10;
        const TERM_VT100LIKE = 0x20;
        const TERM_SIXEL = 0x40;
    }
}

/// Terminal definition.
#[repr(C)]
struct tty_term {
    name: *mut u8,
    tty: *mut tty,
    features: i32,

    acs: [[u8; 2]; c_uchar::MAX as usize + 1],

    codes: *mut tty_code,

    flags: term_flags,

    entry: list_entry<tty_term>,
}
type tty_terms = list_head<tty_term>;
impl ListEntry<tty_term, discr_entry> for tty_term {
    unsafe fn field(this: *mut Self) -> *mut list_entry<tty_term> {
        unsafe { &raw mut (*this).entry }
    }
}

bitflags::bitflags! {
    #[repr(transparent)]
    #[derive(Copy, Clone)]
    struct tty_flags: i32 {
        const TTY_NOCURSOR = 0x1;
        const TTY_FREEZE = 0x2;
        const TTY_TIMER = 0x4;
        const TTY_NOBLOCK = 0x8;
        const TTY_STARTED = 0x10;
        const TTY_OPENED = 0x20;
        const TTY_OSC52QUERY = 0x40;
        const TTY_BLOCK = 0x80;
        const TTY_HAVEDA = 0x100; // Primary DA.
        const TTY_HAVEXDA = 0x200;
        const TTY_SYNCING = 0x400;
        const TTY_HAVEDA2 = 0x800; // Secondary DA.
    }
}
const TTY_ALL_REQUEST_FLAGS: tty_flags = tty_flags::TTY_HAVEDA
    .union(tty_flags::TTY_HAVEDA2)
    .union(tty_flags::TTY_HAVEXDA);

/// Client terminal.
#[repr(C)]
struct tty {
    client: *mut client,
    start_timer: event,
    clipboard_timer: event,
    last_requests: time_t,

    sx: u32,
    sy: u32,

    xpixel: u32,
    ypixel: u32,

    cx: u32,
    cy: u32,
    cstyle: screen_cursor_style,
    ccolour: i32,

    oflag: i32,
    oox: u32,
    ooy: u32,
    osx: u32,
    osy: u32,

    mode: mode_flag,
    fg: i32,
    bg: i32,

    rlower: u32,
    rupper: u32,

    rleft: u32,
    rright: u32,

    event_in: event,
    in_: *mut evbuffer,
    event_out: event,
    out: *mut evbuffer,
    timer: event,
    discarded: usize,

    tio: termios,

    cell: grid_cell,
    last_cell: grid_cell,

    flags: tty_flags,

    term: *mut tty_term,

    mouse_last_x: u32,
    mouse_last_y: u32,
    mouse_last_b: u32,
    mouse_drag_flag: i32,
    mouse_drag_update: Option<unsafe fn(*mut client, *mut mouse_event)>,
    mouse_drag_release: Option<unsafe fn(*mut client, *mut mouse_event)>,

    key_timer: event,
    key_tree: *mut tty_key,
}

type tty_ctx_redraw_cb = Option<unsafe fn(*const tty_ctx)>;
type tty_ctx_set_client_cb = Option<unsafe fn(*mut tty_ctx, *mut client) -> i32>;

#[repr(C)]
struct tty_ctx {
    s: *mut screen,

    redraw_cb: tty_ctx_redraw_cb,
    set_client_cb: tty_ctx_set_client_cb,
    arg: *mut c_void,

    cell: *const grid_cell,
    wrapped: bool,

    num: u32,
    ptr: *mut c_void,
    ptr2: *mut c_void,

    allow_invisible_panes: i32,

    // Cursor and region position before the screen was updated - this is
    // where the command should be applied; the values in the screen have
    // already been updated.
    ocx: u32,
    ocy: u32,

    orupper: u32,
    orlower: u32,

    // Target region (usually pane) offset and size.
    xoff: u32,
    yoff: u32,
    rxoff: u32,
    ryoff: u32,
    sx: u32,
    sy: u32,

    // The background colour used for clearing (erasing).
    bg: u32,

    // The default colours and palette.
    defaults: grid_cell,
    palette: *const colour_palette,

    // Containing region (usually window) offset and size.
    bigger: i32,
    wox: u32,
    woy: u32,
    wsx: u32,
    wsy: u32,
}

// Saved message entry.
impl_tailq_entry!(message_entry, entry, tailq_entry<message_entry>);
#[repr(C)]
struct message_entry {
    msg: *mut u8,
    msg_num: u32,
    msg_time: timeval,

    entry: tailq_entry<message_entry>,
}
type message_list = tailq_head<message_entry>;

/// Argument type.
#[repr(i32)]
#[derive(Copy, Clone, Eq, PartialEq)]
enum args_type {
    ARGS_NONE,
    ARGS_STRING,
    ARGS_COMMANDS,
}

#[repr(C)]
union args_value_union {
    string: *mut u8,
    cmdlist: *mut cmd_list,
}

impl_tailq_entry!(args_value, entry, tailq_entry<args_value>);
/// Argument value.
#[repr(C)]
struct args_value {
    type_: args_type,
    union_: args_value_union,
    cached: *mut u8,
    // #[entry]
    entry: tailq_entry<args_value>,
}
type args_tree = rb_head<args_entry>;

/// Arguments parsing type.
#[repr(C)]
#[derive(Eq, PartialEq)]
enum args_parse_type {
    ARGS_PARSE_INVALID,
    ARGS_PARSE_STRING,
    ARGS_PARSE_COMMANDS_OR_STRING,
    #[expect(dead_code)]
    ARGS_PARSE_COMMANDS,
}

type args_parse_cb = Option<unsafe fn(*mut args, u32, *mut *mut u8) -> args_parse_type>;
#[repr(C)]
struct args_parse {
    template: SyncCharPtr,
    lower: i32,
    upper: i32,
    cb: args_parse_cb,
}

impl args_parse {
    const fn new(template: &'static CStr, lower: i32, upper: i32, cb: args_parse_cb) -> Self {
        Self {
            template: SyncCharPtr::new(template),
            lower,
            upper,
            cb,
        }
    }
}

/// Command find structures.
#[repr(C)]
#[derive(Copy, Clone, Default)]
enum cmd_find_type {
    #[default]
    CMD_FIND_PANE,
    CMD_FIND_WINDOW,
    CMD_FIND_SESSION,
}

#[repr(C)]
#[derive(Copy, Clone)]
struct cmd_find_state {
    flags: cmd_find_flags,
    current: *mut cmd_find_state,

    s: *mut session,
    wl: *mut winlink,
    w: *mut window,
    wp: *mut window_pane,
    idx: i32,
}

bitflags::bitflags! {
    // Command find flags.
    #[repr(transparent)]
    #[derive(Copy, Clone, Default, Eq, PartialEq)]
    struct cmd_find_flags: i32 {
        const CMD_FIND_PREFER_UNATTACHED = 0x1;
        const CMD_FIND_QUIET = 0x2;
        const CMD_FIND_WINDOW_INDEX = 0x4;
        const CMD_FIND_DEFAULT_MARKED = 0x8;
        const CMD_FIND_EXACT_SESSION = 0x10;
        const CMD_FIND_EXACT_WINDOW = 0x20;
        const CMD_FIND_CANFAIL = 0x40;
    }
}

/// List of commands.
#[repr(C)]
struct cmd_list {
    references: i32,
    group: u32,
    list: *mut cmds,
}

// Command return values.
#[repr(i32)]
#[derive(Copy, Clone, Eq, PartialEq)]
enum cmd_retval {
    CMD_RETURN_ERROR = -1,
    CMD_RETURN_NORMAL = 0,
    CMD_RETURN_WAIT,
    CMD_RETURN_STOP,
}

// Command parse result.
#[repr(i32)]
#[derive(Copy, Clone, Default, Eq, PartialEq)]
enum cmd_parse_status {
    #[default]
    CMD_PARSE_ERROR,
    CMD_PARSE_SUCCESS,
}

type cmd_parse_result = Result<*mut cmd_list /* cmdlist */, *mut u8 /* error */>;

bitflags::bitflags! {
    #[repr(transparent)]
    #[derive(Copy, Clone, Eq, PartialEq)]
    struct cmd_parse_input_flags: i32 {
        const CMD_PARSE_QUIET = 0x1;
        const CMD_PARSE_PARSEONLY = 0x2;
        const CMD_PARSE_NOALIAS = 0x4;
        const CMD_PARSE_VERBOSE = 0x8;
        const CMD_PARSE_ONEGROUP = 0x10;
    }
}

#[repr(transparent)]
struct AtomicCmdParseInputFlags(std::sync::atomic::AtomicI32);
impl From<cmd_parse_input_flags> for AtomicCmdParseInputFlags {
    fn from(value: cmd_parse_input_flags) -> Self {
        Self(std::sync::atomic::AtomicI32::new(value.bits()))
    }
}
impl AtomicCmdParseInputFlags {
    fn intersects(&self, rhs: cmd_parse_input_flags) -> bool {
        cmd_parse_input_flags::from_bits(self.0.load(std::sync::atomic::Ordering::SeqCst))
            .unwrap()
            .intersects(rhs)
    }
}
impl std::ops::BitOrAssign<cmd_parse_input_flags> for &AtomicCmdParseInputFlags {
    fn bitor_assign(&mut self, rhs: cmd_parse_input_flags) {
        self.0
            .fetch_or(rhs.bits(), std::sync::atomic::Ordering::SeqCst);
    }
}
impl std::ops::BitAndAssign<cmd_parse_input_flags> for &AtomicCmdParseInputFlags {
    fn bitand_assign(&mut self, rhs: cmd_parse_input_flags) {
        self.0
            .fetch_and(rhs.bits(), std::sync::atomic::Ordering::SeqCst);
    }
}

#[repr(C)]
struct cmd_parse_input<'a> {
    flags: AtomicCmdParseInputFlags,

    file: Option<&'a str>,
    line: AtomicU32, // work around borrow checker

    item: *mut cmdq_item,
    c: *mut client,
    fs: cmd_find_state,
}

bitflags::bitflags! {
    /// Command queue flags.
    #[repr(transparent)]
    #[derive(Copy, Clone, Eq, PartialEq)]
    struct cmdq_state_flags: i32 {
        const CMDQ_STATE_REPEAT = 0x1;
        const CMDQ_STATE_CONTROL = 0x2;
        const CMDQ_STATE_NOHOOKS = 0x4;
    }
}

// Command queue callback.
type cmdq_cb = Option<unsafe fn(*mut cmdq_item, *mut c_void) -> cmd_retval>;

// Command definition flag.
#[repr(C)]
#[derive(Copy, Clone, Default)]
struct cmd_entry_flag {
    flag: u8,
    type_: cmd_find_type,
    flags: cmd_find_flags,
}

impl cmd_entry_flag {
    const fn new(flag: u8, type_: cmd_find_type, flags: cmd_find_flags) -> Self {
        Self { flag, type_, flags }
    }

    const fn zeroed() -> Self {
        Self {
            flag: b'\0',
            type_: cmd_find_type::CMD_FIND_PANE,
            flags: cmd_find_flags::empty(),
        }
    }
}

bitflags::bitflags! {
    #[repr(transparent)]
    #[derive(Copy, Clone, Eq, PartialEq)]
    struct cmd_flag: i32 {
        const CMD_STARTSERVER = 0x1;
        const CMD_READONLY = 0x2;
        const CMD_AFTERHOOK = 0x4;
        const CMD_CLIENT_CFLAG = 0x8;
        const CMD_CLIENT_TFLAG = 0x10;
        const CMD_CLIENT_CANFAIL = 0x20;
    }
}

// Command definition.
#[repr(C)]
struct cmd_entry {
    name: SyncCharPtr,
    alias: SyncCharPtr,

    args: args_parse,
    usage: SyncCharPtr,

    source: cmd_entry_flag,
    target: cmd_entry_flag,

    flags: cmd_flag,

    exec: unsafe fn(*mut cmd, *mut cmdq_item) -> cmd_retval,
}

// Status line.
const STATUS_LINES_LIMIT: usize = 5;
#[repr(C)]
struct status_line_entry {
    expanded: *mut u8,
    ranges: style_ranges,
}
#[repr(C)]
struct status_line {
    timer: event,

    screen: screen,
    active: *mut screen,
    references: c_int,

    style: grid_cell,
    entries: [status_line_entry; STATUS_LINES_LIMIT],
}

/// Prompt type.
const PROMPT_NTYPES: u32 = 4;
#[repr(u32)]
#[derive(Copy, Clone, Default, Eq, PartialEq, num_enum::TryFromPrimitive)]
enum prompt_type {
    #[default]
    PROMPT_TYPE_COMMAND = 0,
    PROMPT_TYPE_SEARCH,
    PROMPT_TYPE_TARGET,
    PROMPT_TYPE_WINDOW_TARGET,
    PROMPT_TYPE_INVALID = 0xff,
}

// File in client.
type client_file_cb = Option<unsafe fn(*mut client, *mut u8, i32, i32, *mut evbuffer, *mut c_void)>;
#[repr(C)]
struct client_file {
    c: *mut client,
    peer: *mut tmuxpeer,
    tree: *mut client_files,

    references: i32,
    stream: i32,

    path: *mut u8,
    buffer: *mut evbuffer,
    event: *mut bufferevent,

    fd: i32,
    error: i32,
    closed: i32,

    cb: client_file_cb,
    data: *mut c_void,

    entry: rb_entry<client_file>,
}
type client_files = rb_head<client_file>;
RB_GENERATE!(client_files, client_file, entry, discr_entry, file_cmp);

// Client window.
#[repr(C)]
struct client_window {
    window: u32,
    pane: *mut window_pane,

    sx: u32,
    sy: u32,

    entry: rb_entry<client_window>,
}
type client_windows = rb_head<client_window>;
RB_GENERATE!(
    client_windows,
    client_window,
    entry,
    discr_entry,
    server_client_window_cmp
);

// Visible areas not obstructed by overlays.
const OVERLAY_MAX_RANGES: usize = 3;
#[repr(C)]
struct overlay_ranges {
    px: [u32; OVERLAY_MAX_RANGES],
    nx: [u32; OVERLAY_MAX_RANGES],
}

type prompt_input_cb = Option<unsafe fn(*mut client, NonNull<c_void>, *const u8, i32) -> i32>;
type prompt_free_cb = Option<unsafe fn(NonNull<c_void>)>;

type overlay_check_cb =
    Option<unsafe fn(*mut client, *mut c_void, u32, u32, u32, *mut overlay_ranges)>;
type overlay_mode_cb =
    Option<unsafe fn(*mut client, *mut c_void, *mut u32, *mut u32) -> *mut screen>;
type overlay_draw_cb = Option<unsafe fn(*mut client, *mut c_void, *mut screen_redraw_ctx)>;
type overlay_key_cb = Option<unsafe fn(*mut client, *mut c_void, *mut key_event) -> i32>;
type overlay_free_cb = Option<unsafe fn(*mut client, *mut c_void)>;
type overlay_resize_cb = Option<unsafe fn(*mut client, *mut c_void)>;

bitflags::bitflags! {
    #[repr(transparent)]
    #[derive(Copy, Clone, Eq, PartialEq)]
    struct client_flag: u64 {
        const TERMINAL           = 0x0000000001u64;
        const LOGIN              = 0x0000000002u64;
        const EXIT               = 0x0000000004u64;
        const REDRAWWINDOW       = 0x0000000008u64;
        const REDRAWSTATUS       = 0x0000000010u64;
        const REPEAT             = 0x0000000020u64;
        const SUSPENDED          = 0x0000000040u64;
        const ATTACHED           = 0x0000000080u64;
        const EXITED             = 0x0000000100u64;
        const DEAD               = 0x0000000200u64;
        const REDRAWBORDERS      = 0x0000000400u64;
        const READONLY           = 0x0000000800u64;
        const NOSTARTSERVER      = 0x0000001000u64;
        const CONTROL            = 0x0000002000u64;
        const CONTROLCONTROL     = 0x0000004000u64;
        const FOCUSED            = 0x0000008000u64;
        const UTF8               = 0x0000010000u64;
        const IGNORESIZE         = 0x0000020000u64;
        const IDENTIFIED         = 0x0000040000u64;
        const STATUSFORCE        = 0x0000080000u64;
        const DOUBLECLICK        = 0x0000100000u64;
        const TRIPLECLICK        = 0x0000200000u64;
        const SIZECHANGED        = 0x0000400000u64;
        const STATUSOFF          = 0x0000800000u64;
        const REDRAWSTATUSALWAYS = 0x0001000000u64;
        const REDRAWOVERLAY      = 0x0002000000u64;
        const CONTROL_NOOUTPUT   = 0x0004000000u64;
        const DEFAULTSOCKET      = 0x0008000000u64;
        const STARTSERVER        = 0x0010000000u64;
        const REDRAWPANES        = 0x0020000000u64;
        const NOFORK             = 0x0040000000u64;
        const ACTIVEPANE         = 0x0080000000u64;
        const CONTROL_PAUSEAFTER = 0x0100000000u64;
        const CONTROL_WAITEXIT   = 0x0200000000u64;
        const WINDOWSIZECHANGED  = 0x0400000000u64;
        const CLIPBOARDBUFFER    = 0x0800000000u64;
        const BRACKETPASTING     = 0x1000000000u64;
    }
}

const CLIENT_ALLREDRAWFLAGS: client_flag = client_flag::REDRAWWINDOW
    .union(client_flag::REDRAWSTATUS)
    .union(client_flag::REDRAWSTATUSALWAYS)
    .union(client_flag::REDRAWBORDERS)
    .union(client_flag::REDRAWOVERLAY)
    .union(client_flag::REDRAWPANES);
const CLIENT_UNATTACHEDFLAGS: client_flag = client_flag::DEAD
    .union(client_flag::SUSPENDED)
    .union(client_flag::EXIT);
const CLIENT_NODETACHFLAGS: client_flag = client_flag::DEAD.union(client_flag::EXIT);
const CLIENT_NOSIZEFLAGS: client_flag = client_flag::DEAD
    .union(client_flag::SUSPENDED)
    .union(client_flag::EXIT);

const PROMPT_SINGLE: i32 = 0x1;
const PROMPT_NUMERIC: i32 = 0x2;
const PROMPT_INCREMENTAL: i32 = 0x4;
const PROMPT_NOFORMAT: i32 = 0x8;
const PROMPT_KEY: i32 = 0x8;

impl_tailq_entry!(client, entry, tailq_entry<client>);
#[repr(C)]
struct client {
    name: *const u8,
    peer: *mut tmuxpeer,
    queue: *mut cmdq_list,

    windows: client_windows,

    control_state: *mut control_state,
    pause_age: c_uint,

    pid: pid_t,
    fd: c_int,
    out_fd: c_int,
    event: event,
    retval: c_int,

    creation_time: timeval,
    activity_time: timeval,

    environ: *mut environ,
    jobs: *mut format_job_tree,

    title: *mut u8,
    path: *mut u8,
    cwd: *const u8,

    term_name: *mut u8,
    term_features: c_int,
    term_type: *mut u8,
    term_caps: *mut *mut u8,
    term_ncaps: c_uint,

    ttyname: *mut u8,
    tty: tty,

    written: usize,
    discarded: usize,
    redraw: usize,

    repeat_timer: event,

    click_timer: event,
    click_button: c_uint,
    click_event: mouse_event,

    status: status_line,

    flags: client_flag,

    exit_type: exit_type,
    exit_msgtype: msgtype,
    exit_session: *mut u8,
    exit_message: *mut u8,

    keytable: *mut key_table,

    redraw_panes: u64,

    message_ignore_keys: c_int,
    message_ignore_styles: c_int,
    message_string: *mut u8,
    message_timer: event,

    prompt_string: *mut u8,
    prompt_buffer: *mut utf8_data,
    prompt_last: *mut u8,
    prompt_index: usize,
    prompt_inputcb: prompt_input_cb,
    prompt_freecb: prompt_free_cb,
    prompt_data: *mut c_void,
    prompt_hindex: [c_uint; 4],
    prompt_mode: prompt_mode,
    prompt_saved: *mut utf8_data,

    prompt_flags: c_int,
    prompt_type: prompt_type,
    prompt_cursor: c_int,

    session: *mut session,
    last_session: *mut session,

    references: c_int,

    pan_window: *mut c_void,
    pan_ox: c_uint,
    pan_oy: c_uint,

    overlay_check: overlay_check_cb,
    overlay_mode: overlay_mode_cb,
    overlay_draw: overlay_draw_cb,
    overlay_key: overlay_key_cb,
    overlay_free: overlay_free_cb,
    overlay_resize: overlay_resize_cb,
    overlay_data: *mut c_void,
    overlay_timer: event,

    files: client_files,

    clipboard_panes: *mut c_uint,
    clipboard_npanes: c_uint,

    // #[entry]
    entry: tailq_entry<client>,
}
type clients = tailq_head<client>;

/// Control mode subscription type.
#[repr(i32)]
enum control_sub_type {
    CONTROL_SUB_SESSION,
    CONTROL_SUB_PANE,
    CONTROL_SUB_ALL_PANES,
    CONTROL_SUB_WINDOW,
    CONTROL_SUB_ALL_WINDOWS,
}

const KEY_BINDING_REPEAT: i32 = 0x1;

/// Key binding and key table.
#[repr(C)]
struct key_binding {
    key: key_code,
    cmdlist: *mut cmd_list,
    note: *mut u8,

    flags: i32,

    entry: rb_entry<key_binding>,
}
type key_bindings = rb_head<key_binding>;

#[repr(C)]
struct key_table {
    name: *mut u8,
    activity_time: timeval,
    key_bindings: key_bindings,
    default_key_bindings: key_bindings,

    references: u32,

    entry: rb_entry<key_table>,
}
type key_tables = rb_head<key_table>;

// Option data.
type options_array = rb_head<options_array_item>;

#[repr(C)]
#[derive(Copy, Clone)]
union options_value {
    string: *mut u8,
    number: c_longlong,
    style: style,
    array: options_array,
    cmdlist: *mut cmd_list,
}

// Option table entries.
#[repr(i32)]
#[derive(Clone, Copy, PartialEq, Eq)]
enum options_table_type {
    OPTIONS_TABLE_STRING,
    OPTIONS_TABLE_NUMBER,
    OPTIONS_TABLE_KEY,
    OPTIONS_TABLE_COLOUR,
    OPTIONS_TABLE_FLAG,
    OPTIONS_TABLE_CHOICE,
    OPTIONS_TABLE_COMMAND,
}

const OPTIONS_TABLE_NONE: i32 = 0;
const OPTIONS_TABLE_SERVER: i32 = 0x1;
const OPTIONS_TABLE_SESSION: i32 = 0x2;
const OPTIONS_TABLE_WINDOW: i32 = 0x4;
const OPTIONS_TABLE_PANE: i32 = 0x8;

const OPTIONS_TABLE_IS_ARRAY: i32 = 0x1;
const OPTIONS_TABLE_IS_HOOK: i32 = 0x2;
const OPTIONS_TABLE_IS_STYLE: i32 = 0x4;

unsafe impl Send for options_table_entry {}
unsafe impl Sync for options_table_entry {}

#[repr(C)]
struct options_table_entry {
    name: *const u8,
    alternative_name: *const u8,
    type_: options_table_type,
    scope: i32,
    flags: i32,
    minimum: u32,
    maximum: u32,

    choices: &'static [&'static str],

    default_str: Option<&'static str>,
    default_num: c_longlong,
    default_arr: *const *const u8,

    separator: *const u8,
    pattern: *const u8,

    text: *const u8,
    unit: *const u8,
}

impl options_table_entry {
    pub const fn const_default() -> Self {
        Self {
            name: null(),
            alternative_name: null(),
            type_: options_table_type::OPTIONS_TABLE_STRING,
            scope: 0,
            flags: 0,
            minimum: 0,
            maximum: 0,
            choices: &[],
            default_str: None,
            default_num: 0,
            default_arr: null(),
            separator: null(),
            pattern: null(),
            text: null(),
            unit: null(),
        }
    }
}

#[repr(C)]
struct options_name_map_str {
    from: &'static str,
    to: &'static str,
}
impl options_name_map_str {
    const fn new(from: &'static str, to: &'static str) -> Self {
        Self { from, to }
    }
}

#[repr(C)]
struct options_name_map {
    from: *const u8,
    to: *const u8,
}
impl options_name_map {
    const fn new(from: &'static CStr, to: &'static CStr) -> Self {
        Self {
            from: from.as_ptr().cast(),
            to: to.as_ptr().cast(),
        }
    }
}

// Common command usages.
const CMD_TARGET_PANE_USAGE: &CStr = c"[-t target-pane]";
const CMD_TARGET_WINDOW_USAGE: &CStr = c"[-t target-window]";
// const CMD_TARGET_SESSION_USAGE: &CStr = c"[-t target-session]";
// const CMD_TARGET_CLIENT_USAGE: &CStr = c"[-t target-client]";
// const CMD_SRCDST_PANE_USAGE: &CStr = c"[-s src-pane] [-t dst-pane]";
// const CMD_SRCDST_WINDOW_USAGE: &CStr = c"[-s src-window] [-t dst-window]";
// const CMD_SRCDST_SESSION_USAGE: &CStr = c"[-s src-session] [-t dst-session]";
// const CMD_SRCDST_CLIENT_USAGE: &CStr = c"[-s src-client] [-t dst-client]";
const CMD_BUFFER_USAGE: &CStr = c"[-b buffer-name]";

bitflags::bitflags! {
    #[repr(transparent)]
    #[derive(Copy, Clone, Eq, PartialEq)]
    struct spawn_flags: i32 {
        const SPAWN_KILL = 0x1;
        const SPAWN_DETACHED = 0x2;
        const SPAWN_RESPAWN = 0x4;
        const SPAWN_BEFORE = 0x8;
        const SPAWN_NONOTIFY = 0x10;
        const SPAWN_FULLSIZE = 0x20;
        const SPAWN_EMPTY = 0x40;
        const SPAWN_ZOOM = 0x80;
    }
}

// TODO inline these and remove the definitions
const SPAWN_KILL: spawn_flags = spawn_flags::SPAWN_KILL;
const SPAWN_DETACHED: spawn_flags = spawn_flags::SPAWN_DETACHED;
const SPAWN_RESPAWN: spawn_flags = spawn_flags::SPAWN_RESPAWN;
const SPAWN_BEFORE: spawn_flags = spawn_flags::SPAWN_BEFORE;
const SPAWN_NONOTIFY: spawn_flags = spawn_flags::SPAWN_NONOTIFY;
const SPAWN_FULLSIZE: spawn_flags = spawn_flags::SPAWN_FULLSIZE;
const SPAWN_EMPTY: spawn_flags = spawn_flags::SPAWN_EMPTY;
const SPAWN_ZOOM: spawn_flags = spawn_flags::SPAWN_ZOOM;

/// Spawn common context.
#[repr(C)]
struct spawn_context {
    item: *mut cmdq_item,

    s: *mut session,
    wl: *mut winlink,
    tc: *mut client,

    wp0: *mut window_pane,
    lc: *mut layout_cell,

    name: *const u8,
    argv: *mut *mut u8,
    argc: i32,
    environ: *mut environ,

    idx: i32,
    cwd: *const u8,

    flags: spawn_flags,
}

/// Mode tree sort order.
#[repr(C)]
#[derive(Default)]
struct mode_tree_sort_criteria {
    field: u32,
    reversed: bool,
}

const WINDOW_MINIMUM: u32 = PANE_MINIMUM;
const WINDOW_MAXIMUM: u32 = 10_000;

#[repr(i32)]
enum exit_type {
    #[expect(dead_code)]
    CLIENT_EXIT_RETURN,
    CLIENT_EXIT_SHUTDOWN,
    CLIENT_EXIT_DETACH,
}

#[repr(i32)]
#[derive(Copy, Clone, Eq, PartialEq)]
enum prompt_mode {
    PROMPT_ENTRY,
    PROMPT_COMMAND,
}

mod tmux;

pub use crate::tmux::tmux_main;
use crate::tmux::{
    GLOBAL_ENVIRON, GLOBAL_OPTIONS, GLOBAL_S_OPTIONS, GLOBAL_W_OPTIONS, PTM_FD, SHELL_COMMAND,
    SOCKET_PATH, START_TIME, checkshell_, find_cwd, find_home, get_timer, getversion, setblocking,
    shell_argv0,
};

mod proc;
use crate::proc::{
    proc_add_peer, proc_clear_signals, proc_exit, proc_flush_peer, proc_fork_and_daemon,
    proc_get_peer_uid, proc_kill_peer, proc_loop, proc_remove_peer, proc_send, proc_set_signals,
    proc_start, proc_toggle_log, tmuxpeer, tmuxproc,
};

mod cfg_;
use crate::cfg_::{
    CFG_CLIENT, CFG_FILES, CFG_FINISHED, CFG_QUIET, cfg_print_causes, cfg_show_causes,
    load_cfg_from_buffer, start_cfg,
};

mod paste;
use crate::paste::{
    paste_add, paste_buffer, paste_buffer_created, paste_buffer_data, paste_buffer_data_,
    paste_buffer_name, paste_buffer_order, paste_free, paste_get_name, paste_get_top,
    paste_is_empty, paste_make_sample, paste_rename, paste_replace, paste_set, paste_walk,
};

mod format;
use crate::format::format_add;
use crate::format::{
    FORMAT_NONE, FORMAT_PANE, FORMAT_WINDOW, format_add_cb, format_add_tv, format_create,
    format_create_defaults, format_create_from_state, format_create_from_target, format_defaults,
    format_defaults_pane, format_defaults_paste_buffer, format_defaults_window, format_each,
    format_expand, format_expand_time, format_flags, format_free, format_get_pane,
    format_grid_hyperlink, format_grid_line, format_grid_word, format_job_tree, format_log_debug,
    format_lost_client, format_merge, format_pretty_time, format_single, format_single_from_state,
    format_single_from_target, format_skip, format_tidy_jobs, format_tree, format_true,
};

mod format_draw_;
use crate::format_draw_::{format_draw, format_trim_left, format_trim_right, format_width};

mod notify;
use crate::notify::{
    notify_client, notify_hook, notify_pane, notify_paste_buffer, notify_session,
    notify_session_window, notify_window, notify_winlink,
};

mod options_;
use crate::options_::options_set_string;
use crate::options_::{
    options, options_array_assign, options_array_clear, options_array_first, options_array_get,
    options_array_item, options_array_item_index, options_array_item_value, options_array_next,
    options_array_set, options_create, options_default, options_default_to_string, options_empty,
    options_entry, options_first, options_free, options_from_string, options_get, options_get_,
    options_get_number, options_get_number_, options_get_only, options_get_parent,
    options_get_string, options_get_string_, options_is_array, options_is_string, options_match,
    options_name, options_next, options_owner, options_parse_get, options_push_changes,
    options_remove_or_default, options_scope_from_flags, options_scope_from_name,
    options_set_number, options_set_parent, options_string_to_style, options_table_entry,
    options_to_string,
};

mod options_table;
use crate::options_table::{OPTIONS_OTHER_NAMES, OPTIONS_TABLE};

bitflags::bitflags! {
    #[repr(transparent)]
    #[derive(Copy, Clone, Eq, PartialEq)]
    struct job_flag: i32 {
        const JOB_NOWAIT = 1;
        const JOB_KEEPWRITE = 2;
        const JOB_PTY = 4;
        const JOB_DEFAULTSHELL = 8;
    }
}
mod job_;
use crate::job_::{
    job, job_check_died, job_free, job_get_data, job_get_event, job_get_status, job_kill_all,
    job_print_summary, job_resize, job_run, job_still_running, job_transfer,
};

mod environ_;
use crate::environ_::{
    environ, environ_clear, environ_copy, environ_create, environ_find, environ_first,
    environ_for_session, environ_free, environ_next, environ_push, environ_put, environ_unset,
    environ_update,
};
use crate::environ_::{environ_log, environ_set};

mod tty_;
use crate::tty_::{
    tty_attributes, tty_cell, tty_clipboard_query, tty_close, tty_cmd_alignmenttest, tty_cmd_cell,
    tty_cmd_cells, tty_cmd_clearcharacter, tty_cmd_clearendofscreen, tty_cmd_clearscreen,
    tty_cmd_clearstartofscreen, tty_cmd_deletecharacter, tty_cmd_deleteline,
    tty_cmd_insertcharacter, tty_cmd_insertline, tty_cmd_rawstring, tty_cmd_reverseindex,
    tty_cmd_scrolldown, tty_cmd_scrollup, tty_cmd_setselection, tty_cmd_syncstart, tty_create_log,
    tty_cursor, tty_default_colours, tty_draw_line, tty_free, tty_init, tty_margin_off, tty_open,
    tty_putc, tty_putcode, tty_putn, tty_puts, tty_raw, tty_region_off, tty_repeat_requests,
    tty_reset, tty_resize, tty_send_requests, tty_set_path, tty_set_selection, tty_set_size,
    tty_set_title, tty_start_tty, tty_stop_tty, tty_sync_end, tty_sync_start,
    tty_update_client_offset, tty_update_features, tty_update_mode, tty_update_window_offset,
    tty_window_bigger, tty_window_offset, tty_write,
};

mod tty_term_;
use crate::tty_term_::{
    TTY_TERMS, tty_code, tty_term_apply, tty_term_apply_overrides, tty_term_create,
    tty_term_describe, tty_term_flag, tty_term_free, tty_term_free_list, tty_term_has,
    tty_term_ncodes, tty_term_number, tty_term_read_list, tty_term_string, tty_term_string_i,
    tty_term_string_ii, tty_term_string_iii, tty_term_string_s, tty_term_string_ss,
};

mod tty_features;
use crate::tty_features::{
    tty_add_features, tty_apply_features, tty_default_features, tty_get_features,
};

mod tty_acs;
use crate::tty_acs::{
    tty_acs_double_borders, tty_acs_get, tty_acs_heavy_borders, tty_acs_needed,
    tty_acs_reverse_get, tty_acs_rounded_borders,
};

mod tty_keys;
use crate::tty_keys::{tty_key, tty_keys_build, tty_keys_colours, tty_keys_free, tty_keys_next};

mod arguments;

// unsafe fn args_get(_: *mut args, _: c_uchar) -> *const c_char;
unsafe fn args_get_(args: *mut args, flag: char) -> *const u8 {
    debug_assert!(flag.is_ascii());
    unsafe { args_get(args, flag as u8) }
}

use crate::arguments::{
    args, args_command_state, args_copy, args_count, args_create, args_entry, args_escape,
    args_first, args_first_value, args_free, args_free_value, args_free_values, args_from_vector,
    args_get, args_has, args_has_count, args_make_commands, args_make_commands_free,
    args_make_commands_get_command, args_make_commands_now, args_make_commands_prepare, args_next,
    args_next_value, args_parse, args_percentage, args_percentage_and_expand, args_print, args_set,
    args_string, args_string_percentage, args_strtonum, args_strtonum_and_expand, args_to_vector,
    args_value, args_values,
};

mod cmd_;
use crate::cmd_::cmd_attach_session::cmd_attach_session;
use crate::cmd_::cmd_find::{
    cmd_find_best_client, cmd_find_clear_state, cmd_find_client, cmd_find_copy_state,
    cmd_find_empty_state, cmd_find_from_client, cmd_find_from_mouse, cmd_find_from_nothing,
    cmd_find_from_pane, cmd_find_from_session, cmd_find_from_session_window, cmd_find_from_window,
    cmd_find_from_winlink, cmd_find_from_winlink_pane, cmd_find_target, cmd_find_valid_state,
};
use crate::cmd_::cmd_log_argv;
use crate::cmd_::{
    CMD_TABLE, cmd, cmd_append_argv, cmd_copy_argv, cmd_free_argv, cmd_get_alias, cmd_get_args,
    cmd_get_entry, cmd_get_group, cmd_get_source, cmd_list_all_have, cmd_list_any_have,
    cmd_list_append, cmd_list_append_all, cmd_list_copy, cmd_list_first, cmd_list_free,
    cmd_list_move, cmd_list_new, cmd_list_next, cmd_list_print, cmd_mouse_at, cmd_mouse_pane,
    cmd_mouse_window, cmd_pack_argv, cmd_parse, cmd_print, cmd_stringify_argv,
    cmd_template_replace, cmd_unpack_argv, cmds,
};

mod cmd_parse;
use crate::cmd_::cmd_queue::{
    cmdq_add_format, cmdq_add_formats, cmdq_append, cmdq_continue, cmdq_copy_state, cmdq_error,
    cmdq_free, cmdq_free_state, cmdq_get_callback, cmdq_get_client, cmdq_get_command,
    cmdq_get_current, cmdq_get_error, cmdq_get_event, cmdq_get_flags, cmdq_get_name,
    cmdq_get_source, cmdq_get_state, cmdq_get_target, cmdq_get_target_client, cmdq_guard,
    cmdq_insert_after, cmdq_insert_hook, cmdq_item, cmdq_list, cmdq_merge_formats, cmdq_new,
    cmdq_new_state, cmdq_next, cmdq_print, cmdq_print_data, cmdq_running, cmdq_state,
};
use crate::cmd_::cmd_wait_for::cmd_wait_for_flush;
use crate::cmd_parse::{
    cmd_parse_and_append, cmd_parse_from_arguments, cmd_parse_from_buffer, cmd_parse_from_file,
    cmd_parse_from_string, cmd_parse_state, *,
};

mod client_;
use crate::client_::client_main;

mod key_bindings_;
use crate::key_bindings_::{
    key_bindings_add, key_bindings_dispatch, key_bindings_first, key_bindings_first_table,
    key_bindings_get, key_bindings_get_default, key_bindings_get_table, key_bindings_init,
    key_bindings_next, key_bindings_next_table, key_bindings_remove, key_bindings_remove_table,
    key_bindings_reset, key_bindings_unref_table,
};

mod key_string;
use crate::key_string::{key_string_lookup_key, key_string_lookup_string};

mod alerts;
use crate::alerts::{alerts_check_session, alerts_queue, alerts_reset_all};

mod file;
use crate::file::{
    file_can_print, file_cancel, file_cmp, file_error, file_fire_done, file_print,
    file_print_buffer, file_read, file_read_cancel, file_read_data, file_read_done, file_read_open,
    file_write, file_write_close, file_write_data, file_write_left, file_write_open,
    file_write_ready,
};

mod server;
use crate::server::{
    CLIENTS, CURRENT_TIME, MARKED_PANE, SERVER_PROC, server_add_accept, server_add_message,
    server_check_marked, server_clear_marked, server_is_marked, server_set_marked, server_start,
    server_update_socket,
};

mod server_client;
use crate::server_client::{
    server_client_add_client_window, server_client_check_nested, server_client_clear_overlay,
    server_client_create, server_client_detach, server_client_exec,
    server_client_get_client_window, server_client_get_cwd, server_client_get_flags,
    server_client_get_key_table, server_client_get_pane, server_client_handle_key,
    server_client_how_many, server_client_loop, server_client_lost, server_client_open,
    server_client_overlay_range, server_client_print, server_client_remove_pane,
    server_client_set_flags, server_client_set_key_table, server_client_set_overlay,
    server_client_set_pane, server_client_set_session, server_client_suspend, server_client_unref,
    server_client_window_cmp,
};

mod server_fn;
use crate::server_fn::{
    server_check_unattached, server_destroy_pane, server_destroy_session, server_kill_pane,
    server_kill_window, server_link_window, server_lock, server_lock_client, server_lock_session,
    server_redraw_client, server_redraw_session, server_redraw_session_group, server_redraw_window,
    server_redraw_window_borders, server_renumber_all, server_status_client, server_status_session,
    server_status_session_group, server_status_window, server_unlink_window, server_unzoom_window,
};

mod status;
use crate::status::{
    STATUS_PROMPT_HLIST, STATUS_PROMPT_HSIZE, status_at_line, status_free, status_get_range,
    status_init, status_line_size, status_message_clear, status_message_redraw, status_message_set,
    status_prompt_clear, status_prompt_key, status_prompt_load_history, status_prompt_redraw,
    status_prompt_save_history, status_prompt_set, status_prompt_type, status_prompt_type_string,
    status_prompt_update, status_redraw, status_timer_start, status_timer_start_all,
    status_update_cache,
};

mod resize;
use crate::resize::{
    default_window_size, recalculate_size, recalculate_sizes, recalculate_sizes_now, resize_window,
};

mod input;
use crate::input::{
    input_ctx, input_free, input_init, input_parse_buffer, input_parse_pane, input_parse_screen,
    input_pending, input_reply_clipboard, input_reset,
};

mod input_keys;
use crate::input_keys::{input_key, input_key_build, input_key_get_mouse, input_key_pane};

mod colour;
use crate::colour::{
    colour_256to16, colour_find_rgb, colour_force_rgb, colour_fromstring, colour_fromstring_,
    colour_join_rgb, colour_palette_clear, colour_palette_free, colour_palette_from_option,
    colour_palette_get, colour_palette_init, colour_palette_set, colour_parse_x11,
    colour_split_rgb, colour_tostring,
};

mod attributes;
use crate::attributes::{attributes_fromstring, attributes_tostring};

mod grid_;
use crate::grid_::{
    GRID_DEFAULT_CELL, grid_adjust_lines, grid_cells_equal, grid_cells_look_equal, grid_clear,
    grid_clear_history, grid_clear_lines, grid_collect_history, grid_compare, grid_create,
    grid_destroy, grid_duplicate_lines, grid_empty_line, grid_get_cell, grid_get_line,
    grid_line_length, grid_move_cells, grid_move_lines, grid_peek_line, grid_reflow,
    grid_remove_history, grid_scroll_history, grid_scroll_history_region, grid_set_cell,
    grid_set_cells, grid_set_padding, grid_string_cells, grid_unwrap_position, grid_wrap_position,
};

mod grid_reader_;
use crate::grid_reader_::{
    grid_reader_cursor_back_to_indentation, grid_reader_cursor_end_of_line,
    grid_reader_cursor_jump, grid_reader_cursor_jump_back, grid_reader_cursor_left,
    grid_reader_cursor_next_word, grid_reader_cursor_next_word_end,
    grid_reader_cursor_previous_word, grid_reader_cursor_right, grid_reader_cursor_start_of_line,
    grid_reader_get_cursor, grid_reader_in_set, grid_reader_start,
};

mod grid_view;
use crate::grid_view::{
    grid_view_clear, grid_view_clear_history, grid_view_delete_cells, grid_view_delete_lines,
    grid_view_delete_lines_region, grid_view_get_cell, grid_view_insert_cells,
    grid_view_insert_lines, grid_view_insert_lines_region, grid_view_scroll_region_down,
    grid_view_scroll_region_up, grid_view_set_cell, grid_view_set_cells, grid_view_set_padding,
    grid_view_string_cells,
};

mod screen_write;
use crate::screen_write::{
    screen_write_alignmenttest, screen_write_alternateoff, screen_write_alternateon,
    screen_write_backspace, screen_write_box, screen_write_carriagereturn, screen_write_cell,
    screen_write_citem, screen_write_clearcharacter, screen_write_clearendofline,
    screen_write_clearendofscreen, screen_write_clearhistory, screen_write_clearline,
    screen_write_clearscreen, screen_write_clearstartofline, screen_write_clearstartofscreen,
    screen_write_cline, screen_write_collect_add, screen_write_collect_end,
    screen_write_cursordown, screen_write_cursorleft, screen_write_cursormove,
    screen_write_cursorright, screen_write_cursorup, screen_write_deletecharacter,
    screen_write_deleteline, screen_write_fast_copy, screen_write_free_list,
    screen_write_fullredraw, screen_write_hline, screen_write_insertcharacter,
    screen_write_insertline, screen_write_linefeed, screen_write_make_list, screen_write_menu,
    screen_write_mode_clear, screen_write_mode_set, screen_write_preview, screen_write_putc,
    screen_write_rawstring, screen_write_reset, screen_write_reverseindex, screen_write_scrolldown,
    screen_write_scrollregion, screen_write_scrollup, screen_write_setselection,
    screen_write_start, screen_write_start_callback, screen_write_start_pane, screen_write_stop,
    screen_write_vline,
};
use crate::screen_write::{
    screen_write_nputs, screen_write_puts, screen_write_strlen, screen_write_text,
    screen_write_vnputs, screen_write_vnputs_,
};

mod screen_redraw;
use crate::screen_redraw::{screen_redraw_pane, screen_redraw_screen};

mod screen_;
use crate::screen_::{
    screen_alternate_off, screen_alternate_on, screen_check_selection, screen_clear_selection,
    screen_free, screen_hide_selection, screen_init, screen_mode_to_string, screen_pop_title,
    screen_push_title, screen_reinit, screen_reset_hyperlinks, screen_reset_tabs, screen_resize,
    screen_resize_cursor, screen_sel, screen_select_cell, screen_set_cursor_colour,
    screen_set_cursor_style, screen_set_path, screen_set_selection, screen_set_title,
    screen_titles,
};

mod window_;
use crate::window_::{
    ALL_WINDOW_PANES, WINDOWS, window_add_pane, window_add_ref, window_count_panes, window_create,
    window_destroy_panes, window_find_by_id, window_find_by_id_str, window_find_string,
    window_get_active_at, window_has_pane, window_lost_pane, window_pane_at_index,
    window_pane_default_cursor, window_pane_destroy_ready, window_pane_exited,
    window_pane_find_by_id, window_pane_find_by_id_str, window_pane_find_down,
    window_pane_find_left, window_pane_find_right, window_pane_find_up, window_pane_get_new_data,
    window_pane_index, window_pane_key, window_pane_mode, window_pane_next_by_number,
    window_pane_previous_by_number, window_pane_reset_mode, window_pane_reset_mode_all,
    window_pane_resize, window_pane_search, window_pane_send_resize, window_pane_set_event,
    window_pane_set_mode, window_pane_stack_remove, window_pane_start_input,
    window_pane_update_used_data, window_pane_visible, window_pop_zoom, window_printable_flags,
    window_push_zoom, window_redraw_active_switch, window_remove_pane, window_remove_ref,
    window_resize, window_set_active_pane, window_set_fill_character, window_set_name,
    window_unzoom, window_update_activity, window_update_focus, window_zoom, winlink_add,
    winlink_clear_flags, winlink_count, winlink_find_by_index, winlink_find_by_window,
    winlink_find_by_window_id, winlink_next, winlink_next_by_number, winlink_previous,
    winlink_previous_by_number, winlink_remove, winlink_set_window, winlink_shuffle_up,
    winlink_stack_push, winlink_stack_remove,
};

mod layout;
use crate::layout::{
    layout_assign_pane, layout_close_pane, layout_count_cells, layout_create_cell,
    layout_destroy_cell, layout_fix_offsets, layout_fix_panes, layout_free, layout_free_cell,
    layout_init, layout_make_leaf, layout_make_node, layout_print_cell, layout_resize,
    layout_resize_adjust, layout_resize_layout, layout_resize_pane, layout_resize_pane_to,
    layout_search_by_border, layout_set_size, layout_split_pane, layout_spread_cell,
    layout_spread_out,
};

mod layout_custom;
use crate::layout_custom::{layout_dump, layout_parse};

mod layout_set;
use crate::layout_set::{
    layout_set_lookup, layout_set_next, layout_set_previous, layout_set_select,
};

mod mode_tree;
use crate::mode_tree::{
    mode_tree_add, mode_tree_build, mode_tree_collapse_current, mode_tree_count_tagged,
    mode_tree_data, mode_tree_down, mode_tree_draw, mode_tree_draw_as_parent,
    mode_tree_each_tagged, mode_tree_expand, mode_tree_expand_current, mode_tree_free,
    mode_tree_get_current, mode_tree_get_current_name, mode_tree_item, mode_tree_key,
    mode_tree_no_tag, mode_tree_remove, mode_tree_resize, mode_tree_run_command,
    mode_tree_set_current, mode_tree_start, mode_tree_up, mode_tree_zoom,
};

mod window_buffer;
use crate::window_buffer::WINDOW_BUFFER_MODE;

mod window_tree;
use crate::window_tree::WINDOW_TREE_MODE;

mod window_clock;
use crate::window_clock::{WINDOW_CLOCK_MODE, WINDOW_CLOCK_TABLE};

mod window_client;
use crate::window_client::WINDOW_CLIENT_MODE;

mod window_copy;
use crate::window_copy::window_copy_add;
use crate::window_copy::{
    WINDOW_COPY_MODE, WINDOW_VIEW_MODE, window_copy_get_line, window_copy_get_word,
    window_copy_pagedown, window_copy_pageup, window_copy_start_drag,
};

mod window_customize;
use crate::window_customize::WINDOW_CUSTOMIZE_MODE;

mod names;
use crate::names::{check_window_name, default_window_name, parse_window_name};

mod control;
use crate::control::control_write;
use crate::control::{
    control_add_sub, control_all_done, control_continue_pane, control_discard, control_pane_offset,
    control_pause_pane, control_ready, control_remove_sub, control_reset_offsets,
    control_set_pane_off, control_set_pane_on, control_start, control_state, control_stop,
    control_write_output,
};

mod control_notify;
use crate::control_notify::{
    control_notify_client_detached, control_notify_client_session_changed,
    control_notify_pane_mode_changed, control_notify_paste_buffer_changed,
    control_notify_paste_buffer_deleted, control_notify_session_closed,
    control_notify_session_created, control_notify_session_renamed,
    control_notify_session_window_changed, control_notify_window_layout_changed,
    control_notify_window_linked, control_notify_window_pane_changed,
    control_notify_window_renamed, control_notify_window_unlinked,
};

mod session_;
use crate::session_::{
    NEXT_SESSION_ID, SESSIONS, session_add_ref, session_alive, session_attach, session_check_name,
    session_create, session_destroy, session_detach, session_find, session_find_by_id,
    session_find_by_id_str, session_group_add, session_group_attached_count,
    session_group_contains, session_group_count, session_group_find, session_group_new,
    session_group_synchronize_from, session_group_synchronize_to, session_has, session_is_linked,
    session_last, session_next, session_next_session, session_previous, session_previous_session,
    session_remove_ref, session_renumber_windows, session_select, session_set_current,
    session_update_activity,
};

mod utf8;
use crate::utf8::{
    utf8_append, utf8_build_one, utf8_copy, utf8_cstrhas, utf8_cstrwidth, utf8_from_data,
    utf8_fromcstr, utf8_fromwc, utf8_in_table, utf8_isvalid, utf8_open, utf8_padcstr,
    utf8_rpadcstr, utf8_sanitize, utf8_set, utf8_stravis, utf8_stravisx, utf8_strlen, utf8_strvis,
    utf8_strwidth, utf8_to_data, utf8_tocstr, utf8_towc,
};

mod osdep;
use crate::osdep::{osdep_event_init, osdep_get_cwd, osdep_get_name};

mod utf8_combined;
use crate::utf8_combined::{utf8_has_zwj, utf8_is_modifier, utf8_is_vs, utf8_is_zwj};

#[macro_use] // log_debug
mod log;
use crate::log::{fatal, fatalx, log_add_level, log_close, log_get_level, log_open, log_toggle};
use crate::log::{fatalx_, log_debug};

bitflags::bitflags! {
    #[repr(transparent)]
    #[derive(Copy, Clone, Eq, PartialEq)]
    struct menu_flags: i32 {
        const MENU_NOMOUSE = 0x1;
        const MENU_TAB = 0x2;
        const MENU_STAYOPEN = 0x4;
    }
}
const MENU_NOMOUSE: menu_flags = menu_flags::MENU_NOMOUSE;
const MENU_TAB: menu_flags = menu_flags::MENU_TAB;
const MENU_STAYOPEN: menu_flags = menu_flags::MENU_STAYOPEN;

mod menu_;
use crate::menu_::{
    menu_add_item, menu_add_items, menu_check_cb, menu_create, menu_data, menu_display,
    menu_draw_cb, menu_free, menu_free_cb, menu_key_cb, menu_mode_cb, menu_prepare,
};

const POPUP_CLOSEEXIT: i32 = 0x1;
const POPUP_CLOSEEXITZERO: i32 = 0x2;
const POPUP_INTERNAL: i32 = 0x4;
mod popup;
use crate::popup::{popup_display, popup_editor};

mod style_;
use crate::style_::{style_add, style_apply, style_copy, style_parse, style_set, style_tostring};

mod spawn;
use crate::spawn::{spawn_pane, spawn_window};

mod regsub;
use crate::regsub::regsub;

mod server_acl;
use crate::server_acl::{
    server_acl_display, server_acl_get_uid, server_acl_init, server_acl_join,
    server_acl_user_allow, server_acl_user_allow_write, server_acl_user_deny,
    server_acl_user_deny_write, server_acl_user_find,
};

mod hyperlinks_;
use crate::hyperlinks_::{
    hyperlinks, hyperlinks_copy, hyperlinks_free, hyperlinks_get, hyperlinks_init, hyperlinks_put,
    hyperlinks_reset,
};

mod xmalloc;
use crate::xmalloc::{
    format_nul, xcalloc, xcalloc_, xcalloc1, xmalloc, xmalloc_, xrealloc, xrealloc_, xreallocarray,
    xreallocarray_, xrecallocarray, xsnprintf_, xstrdup, xstrdup_, xstrdup__, xstrdup___, xstrndup,
};

mod tmux_protocol;
use crate::tmux_protocol::{
    PROTOCOL_VERSION, msg_command, msg_read_cancel, msg_read_data, msg_read_done, msg_read_open,
    msg_write_close, msg_write_data, msg_write_open, msg_write_ready, msgtype,
};

unsafe impl Sync for SyncCharPtr {}
#[repr(transparent)]
#[derive(Copy, Clone, Default)]
struct SyncCharPtr(*const u8);
impl SyncCharPtr {
    const fn new(value: &'static CStr) -> Self {
        Self(value.as_ptr().cast())
    }
    const fn from_ptr(value: *const u8) -> Self {
        Self(value)
    }
    const fn null() -> Self {
        Self(null())
    }
    const fn as_ptr(&self) -> *const u8 {
        self.0
    }
    const fn is_null(&self) -> bool {
        self.0.is_null()
    }
}

unsafe fn _s(ptr: impl ToU8Ptr) -> DisplayCStrPtr {
    DisplayCStrPtr(ptr.to_u8_ptr())
}
trait ToU8Ptr {
    fn to_u8_ptr(self) -> *const u8;
}
impl ToU8Ptr for *mut u8 {
    fn to_u8_ptr(self) -> *const u8 {
        self.cast()
    }
}
impl ToU8Ptr for *const u8 {
    fn to_u8_ptr(self) -> *const u8 {
        self
    }
}
impl ToU8Ptr for *mut i8 {
    fn to_u8_ptr(self) -> *const u8 {
        self.cast()
    }
}
impl ToU8Ptr for *const i8 {
    fn to_u8_ptr(self) -> *const u8 {
        self.cast()
    }
}
impl ToU8Ptr for SyncCharPtr {
    fn to_u8_ptr(self) -> *const u8 {
        self.as_ptr()
    }
}
// TODO struct should have some sort of lifetime
/// Display wrapper for a *c_char pointer
#[repr(transparent)]
struct DisplayCStrPtr(*const u8);
impl std::fmt::Display for DisplayCStrPtr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.0.is_null() {
            return f.write_str("(null)");
        }

        // TODO alignment

        let len = if let Some(width) = f.precision() {
            unsafe { libc::strnlen(self.0, width) }
        } else if let Some(width) = f.width() {
            unsafe { libc::strnlen(self.0, width) }
        } else {
            unsafe { libc::strlen(self.0) }
        };

        let s: &[u8] = unsafe { std::slice::from_raw_parts(self.0, len) };
        let s = std::str::from_utf8(s).unwrap_or("%s-invalid-utf8");
        f.write_str(s)
    }
}

// TOOD make usable in const context
// https://stackoverflow.com/a/63904992
macro_rules! function_name {
    () => {{
        fn f() {}
        fn type_name_of<T>(_: T) -> &'static str {
            std::any::type_name::<T>()
        }
        let name = type_name_of(f);

        // Find and cut the rest of the path
        match &name[..name.len() - 3].rfind(':') {
            Some(pos) => &name[pos + 1..name.len() - 3],
            None => &name[..name.len() - 3],
        }
    }};
}
pub(crate) use function_name;

const fn concat_array<const N: usize, const M: usize, const O: usize, T: Copy>(
    a1: [T; N],
    a2: [T; M],
) -> [T; O] {
    let mut out: [MaybeUninit<T>; O] = [MaybeUninit::uninit(); O];

    let mut i: usize = 0;
    while i < a1.len() {
        out[i].write(a1[i]);
        i += 1;
    }
    while i < a1.len() + a2.len() {
        out[i].write(a2[i - a1.len()]);
        i += 1;
    }

    assert!(a1.len() + a2.len() == out.len());
    assert!(i == out.len());

    unsafe { std::mem::transmute_copy(&out) }
    // TODO once stabilized switch to:
    // unsafe { MaybeUninit::array_assume_init(out) }
}

pub(crate) fn i32_to_ordering(value: i32) -> std::cmp::Ordering {
    match value {
        ..0 => std::cmp::Ordering::Less,
        0 => std::cmp::Ordering::Equal,
        1.. => std::cmp::Ordering::Greater,
    }
}

pub(crate) unsafe fn cstr_to_str<'a>(ptr: *const u8) -> &'a str {
    unsafe {
        let len = libc::strlen(ptr);

        let bytes = std::slice::from_raw_parts(ptr.cast::<u8>(), len);

        std::str::from_utf8(bytes).expect("bad cstr_to_str")
    }
}

// ideally we could just use c string literal until we transition to &str everywhere
// unfortunately, some platforms people use have
macro_rules! c {
    ($s:literal) => {{
        const S: &str = concat!($s, "\0");
        #[allow(clippy::allow_attributes)]
        #[allow(unused_unsafe)]
        unsafe { std::ffi::CStr::from_bytes_with_nul_unchecked(S.as_bytes()) }
            .as_ptr()
            .cast::<u8>()
    }};
}
pub(crate) use c;

macro_rules! impl_ord {
    ($ty:ty as $func:ident) => {
        impl Ord for $ty {
            fn cmp(&self, other: &Self) -> std::cmp::Ordering {
                $func(&self, &other)
            }
        }
        impl PartialEq for $ty {
            fn eq(&self, other: &Self) -> bool {
                self.cmp(other).is_eq()
            }
        }
        impl Eq for $ty {}
        impl PartialOrd for $ty {
            fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
                Some(self.cmp(other))
            }
        }
    };
}
pub(crate) use impl_ord;

macro_rules! const_unwrap_result {
    ($e:expr) => {
        match $e {
            Ok(value) => value,
            _ => panic!("const_unwrap_result"),
        }
    };
}
pub(crate) use const_unwrap_result;

macro_rules! cstring_concat {
    ($($e:expr),* $(,)?) => {
        const_unwrap_result!(::core::ffi::CStr::from_bytes_with_nul(concat!($($e),*, "\0").as_bytes()))
    };
}
pub(crate) use cstring_concat;

trait Reverseable {
    fn maybe_reverse(self, reversed: bool) -> Self;
}
impl Reverseable for cmp::Ordering {
    fn maybe_reverse(self, reversed: bool) -> Self {
        if reversed { self.reverse() } else { self }
    }
}

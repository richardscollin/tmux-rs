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
#![cfg_attr(
    fuzzing,
    allow(
        private_interfaces,
        reason = "we use the fuzzing config flag to mark modules public which otherwise wouldn't be in order to fuzz internal implementations"
    )
)]
#![allow(
    non_camel_case_types,
    reason = "this lint is here instead of in Cargo.toml because of a bug in rust analyzer"
)]

mod libc;
pub(crate) use crate::libc::errno;
pub(crate) use crate::libc::*;
pub(crate) use crate::libc::{free_, memcpy_, memcpy__, streq_};

// libevent2
mod event_;
use crate::event_::*;

mod ncurses_;
use crate::ncurses_::*;

macro_rules! cfg_pub_mods {
    ($( mod $mod_name:ident; )*) => {
        $(
            #[cfg(fuzzing)]
            pub mod $mod_name;

            #[cfg(not(fuzzing))]
            mod $mod_name;
        )*
    };
}

cfg_pub_mods! {
    mod alerts;
    mod arguments;
    mod attributes;
    mod bitstr;
    mod cfg_;
    mod client_;
    mod cmd_;
    mod cmd_parse;
    mod colour;
    mod compat;
    mod control;
    mod control_notify;
    mod environ_;
    mod file;
    mod format;
    mod format_draw_;
    mod grid_;
    mod grid_reader_;
    mod grid_view;
    mod hyperlinks_;
    mod input;
    mod input_keys;
    mod job_;
    mod key_bindings_;
    mod key_string;
    mod layout;
    mod layout_custom;
    mod layout_set;
    mod menu_;
    mod mode_tree;
    mod names;
    mod notify;
    mod options_;
    mod options_table;
    mod osdep;
    mod paste;
    mod popup;
    mod proc;
    mod regsub;
    mod resize;
    mod screen_;
    mod screen_redraw;
    mod screen_write;
    mod server;
    mod server_acl;
    mod server_client;
    mod server_fn;
    mod session_;
    mod spawn;
    mod status;
    mod style_;
    mod tmux;
    mod tmux_protocol;
    mod tty_;
    mod tty_acs;
    mod tty_features;
    mod tty_keys;
    mod tty_term_;
    mod utf8;
    mod utf8_combined;
    mod window_;
    mod window_buffer;
    mod window_client;
    mod window_clock;
    mod window_copy;
    mod window_customize;
    mod window_tree;
    mod xmalloc;
}

#[macro_use] // log_debug
mod log;
use std::{
    borrow::Cow,
    cell::RefCell,
    cmp,
    collections::LinkedList,
    ffi::{
        CStr, CString, c_int, c_long, c_longlong, c_short, c_uchar, c_uint, c_ulonglong, c_void,
    },
    mem::{MaybeUninit, size_of, zeroed},
    ptr::{NonNull, addr_of, addr_of_mut, null, null_mut},
    sync::{
        Mutex,
        atomic::{self, AtomicBool, AtomicU32, AtomicU64},
    },
};

use crate::log::*;
pub use crate::tmux::tmux_main;
use crate::{
    alerts::*,
    arguments::*,
    attributes::*,
    bitstr::*,
    cfg_::*,
    client_::*,
    cmd_::{
        cmd_attach_session::cmd_attach_session, cmd_find::*, cmd_log_argv, cmd_queue::*,
        cmd_wait_for::cmd_wait_for_flush, *,
    },
    cmd_parse::*,
    colour::*,
    compat::{imsg::imsg, queue::*, strtonum, tree::*, *}, /* strtonum need to disambiguate from libc on macos */
    control::{control_write, *},
    control_notify::*,
    environ_::*,
    file::*,
    format::*,
    format_draw_::*,
    grid_::*,
    grid_reader_::*,
    grid_view::*,
    hyperlinks_::*,
    input::*,
    input_keys::*,
    job_::*,
    key_bindings_::*,
    key_string::*,
    layout::*,
    layout_custom::*,
    layout_set::*,
    menu_::*,
    mode_tree::*,
    names::*,
    notify::*,
    options_::{options, options_array_item},
    options_table::*,
    osdep::*,
    paste::*,
    popup::*,
    proc::*,
    regsub::regsub,
    resize::*,
    screen_::*,
    screen_redraw::*,
    screen_write::*,
    server::*,
    server_acl::*,
    server_client::*,
    server_fn::*,
    session_::*,
    spawn::*,
    status::*,
    style_::*,
    tmux::*,
    tmux_protocol::*,
    tty_::*,
    tty_acs::*,
    tty_features::*,
    tty_keys::*,
    tty_term_::*,
    utf8::*,
    utf8_combined::*,
    window_::*,
    window_buffer::WINDOW_BUFFER_MODE,
    window_client::WINDOW_CLIENT_MODE,
    window_clock::{WINDOW_CLOCK_MODE, WINDOW_CLOCK_TABLE},
    window_copy::{window_copy_add, *},
    window_customize::WINDOW_CUSTOMIZE_MODE,
    window_tree::WINDOW_TREE_MODE,
    xmalloc::*,
};

#[cfg(feature = "sixel")]
mod image_;
#[cfg(feature = "sixel")]
mod image_sixel;
#[cfg(feature = "sixel")]
use image_sixel::sixel_image;

#[cfg(feature = "utempter")]
mod utempter;

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

// /usr/include/paths.h
const _PATH_TTY: *const u8 = c!("/dev/tty");
const _PATH_BSHELL: *const u8 = c!("/bin/sh");
const _PATH_BSHELL_STR: &str = "/bin/sh";
const _PATH_DEFPATH: *const u8 = c!("/usr/bin:/bin");
const _PATH_DEV: *const u8 = c!("/dev/");
const _PATH_DEVNULL: *const u8 = c!("/dev/null");
const _PATH_VI: &str = "/usr/bin/vi";
const SIZEOF_PATH_DEV: usize = 6;
const TTY_NAME_MAX: usize = 32;

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

// discriminant structs
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

/// Minimum layout cell size, NOT including border lines.
const PANE_MINIMUM: u32 = 1;

/// Automatic name refresh interval, in microseconds. Must be < 1 second.
const NAME_INTERVAL: libc::suseconds_t = 500000;

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
/// `KEYC_BASE` and `KEYC_BASE_END`.
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

// Colour flags.
const COLOUR_FLAG_256: i32 = 0x01000000;
const COLOUR_FLAG_RGB: i32 = 0x02000000;

/// Special colours.
#[expect(non_snake_case)]
#[inline]
fn COLOUR_DEFAULT(c: i32) -> bool {
    c == 8 || c == 9
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

/// Cell borders.
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

    tabs: Option<BitStr>,
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
    name: &'static str,
    default_format: Option<&'static str>,

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

    options: *mut options,

    references: u32,
    winlinks: tailq_head<winlink>,
    entry: rb_entry<window>,
}
type windows = rb_head<window>;

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
    name: Cow<'static, str>,
    sessions: tailq_head<session>,

    entry: rb_entry<session_group>,
}
type session_groups = rb_head<session_group>;

const SESSION_PASTING: i32 = 0x1;
const SESSION_ALERTED: i32 = 0x2;

#[repr(C)]
struct session {
    id: u32,
    name: Cow<'static, str>,
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
    template: &'static str,
    lower: i32,
    upper: i32,
    cb: args_parse_cb,
}

impl args_parse {
    const fn new(template: &'static str, lower: i32, upper: i32, cb: args_parse_cb) -> Self {
        Self {
            template,
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
#[derive(Copy, Clone, Default)]
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
#[derive(Default)]
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
#[derive(Default)]
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
    name: &'static str,
    alias: Option<&'static str>,

    args: args_parse,
    usage: &'static str,

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

bitflags::bitflags! {
    #[repr(transparent)]
    #[derive(Copy, Clone, Default, Eq, PartialEq)]
    struct prompt_flags: u32 {
        const PROMPT_SINGLE = 0x1;
        const PROMPT_NUMERIC = 0x2;
        const PROMPT_INCREMENTAL = 0x4;
        const PROMPT_NOFORMAT = 0x8;
        const PROMPT_KEY = 0x10;
    }
}

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

    prompt_flags: prompt_flags,
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

unsafe impl Sync for options_table_entry {}

#[repr(C)]
struct options_table_entry {
    name: &'static str,
    alternative_name: *mut u8,
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
            name: "",
            alternative_name: null_mut(),
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
    from: &'static str,
    to: &'static str,
}
impl options_name_map {
    const fn new(from: &'static str, to: &'static str) -> Self {
        Self { from, to }
    }
}

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

bitflags::bitflags! {
    #[repr(transparent)]
    #[derive(Copy, Clone, Default, Eq, PartialEq)]
    struct job_flag: i32 {
        const JOB_NOWAIT = 1;
        const JOB_KEEPWRITE = 2;
        const JOB_PTY = 4;
        const JOB_DEFAULTSHELL = 8;
    }
}

// unsafe fn args_get(_: *mut args, _: c_uchar) -> *const c_char;
unsafe fn args_get_(args: *mut args, flag: char) -> *const u8 {
    debug_assert!(flag.is_ascii());
    unsafe { args_get(args, flag as u8) }
}

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
/// Display wrapper for a *`c_char` pointer
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
    unsafe { cstr_to_str_(ptr).unwrap() }
}

pub(crate) unsafe fn cstr_to_str_<'a>(ptr: *const u8) -> Option<&'a str> {
    unsafe {
        if ptr.is_null() {
            return None;
        }
        let len = libc::strlen(ptr);

        let bytes = std::slice::from_raw_parts(ptr.cast::<u8>(), len);

        Some(std::str::from_utf8(bytes).expect("bad cstr_to_str"))
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

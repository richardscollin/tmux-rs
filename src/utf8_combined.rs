// Copyright (c) 2023 Nicholas Marriott <nicholas.marriott@gmail.com>
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
use core::ffi::c_void;

use crate::libc::memcmp;

use crate::{utf8_data, utf8_state, utf8_towc, wchar_t};

pub unsafe fn utf8_has_zwj(ud: *const utf8_data) -> bool {
    unsafe {
        if (*ud).size < 3 {
            return false;
        }

        memcmp(
            &raw const (*ud).data[((*ud).size - 3) as usize] as *const c_void,
            b"\xe2\x80\x8d\x00" as *const u8 as *const c_void,
            3,
        ) == 0
    }
}

pub unsafe fn utf8_is_zwj(ud: *const utf8_data) -> bool {
    unsafe {
        if (*ud).size != 3 {
            return false;
        }
        memcmp(
            &raw const (*ud).data as *const u8 as *const c_void,
            b"\xe2\x80\x8d\x00" as *const u8 as *const c_void,
            3,
        ) == 0
    }
}

pub unsafe fn utf8_is_vs(ud: *const utf8_data) -> bool {
    unsafe {
        if (*ud).size != 3 {
            return false;
        }
        memcmp(
            &raw const (*ud).data as *const u8 as *const c_void,
            b"\xef\xbf\x8f\x00" as *const u8 as *const c_void,
            3,
        ) == 0
    }
}

pub unsafe fn utf8_is_modifier(ud: *const utf8_data) -> bool {
    let mut wc: wchar_t = 0;
    unsafe {
        if utf8_towc(ud, &raw mut wc) != utf8_state::UTF8_DONE {
            return false;
        }
    }
    matches!(
        wc,
        0x1F1E6..=0x1F1FF | 0x1F3FB..=0x1F3FF
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum hanguljamo_state {
    NotHangulJamo,
    Choseong,
    Composable,
    NotComposable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum hanguljamo_subclass {
    NotHangulJamo,
    Choseong,
    OldChoseong,
    ChoseongFiller,
    JungseongFiller,
    Jungseong,
    OldJungseong,
    Jongseong,
    OldJongseong,
    ExtendedOldChoseong,
    ExtendedOldJungseong,
    ExtendedOldJongseong,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum hanguljamo_class {
    NotHangulJamo,
    Choseong,
    Jungseong,
    Jongseong,
}

fn hanguljamo_get_subclass(s: &[u8]) -> hanguljamo_subclass {
    match s[0] {
        0xE1 => match s[1] {
            0x84 => {
                if s[2] >= 0x80 && s[2] <= 0x92 {
                    return hanguljamo_subclass::Choseong;
                }
                if s[2] >= 0x93 && s[2] <= 0xBF {
                    return hanguljamo_subclass::OldChoseong;
                }
            }
            0x85 => {
                if s[2] == 0x9F {
                    return hanguljamo_subclass::ChoseongFiller;
                }
                if s[2] == 0xA0 {
                    return hanguljamo_subclass::JungseongFiller;
                }
                if s[2] >= 0x80 && s[2] <= 0x9E {
                    return hanguljamo_subclass::OldChoseong;
                }
                if s[2] >= 0xA1 && s[2] <= 0xB5 {
                    return hanguljamo_subclass::Jungseong;
                }
                if s[2] >= 0xB6 && s[2] <= 0xBF {
                    return hanguljamo_subclass::OldJungseong;
                }
            }
            0x86 => {
                if s[2] >= 0x80 && s[2] <= 0xA7 {
                    return hanguljamo_subclass::OldJungseong;
                }
                if s[2] >= 0xA8 && s[2] <= 0xBF {
                    return hanguljamo_subclass::Jongseong;
                }
            }
            0x87 => {
                if s[2] >= 0x80 && s[2] <= 0x82 {
                    return hanguljamo_subclass::Jongseong;
                }
                if s[2] >= 0x83 && s[2] <= 0xBF {
                    return hanguljamo_subclass::OldJongseong;
                }
            }
            _ => {}
        },
        0xEA => {
            if s[1] == 0xA5 && s[2] >= 0xA0 && s[2] <= 0xBC {
                return hanguljamo_subclass::ExtendedOldChoseong;
            }
        }
        0xED => {
            if s[1] == 0x9E && s[2] >= 0xB0 && s[2] <= 0xBF {
                return hanguljamo_subclass::ExtendedOldJungseong;
            }
            if s[1] == 0x9F {
                if s[2] >= 0x80 && s[2] <= 0x86 {
                    return hanguljamo_subclass::ExtendedOldJungseong;
                }
                if s[2] >= 0x8B && s[2] <= 0xBB {
                    return hanguljamo_subclass::ExtendedOldJongseong;
                }
            }
        }
        _ => {}
    }
    hanguljamo_subclass::NotHangulJamo
}

fn hanguljamo_get_class(s: &[u8]) -> hanguljamo_class {
    match hanguljamo_get_subclass(s) {
        hanguljamo_subclass::Choseong
        | hanguljamo_subclass::ChoseongFiller
        | hanguljamo_subclass::OldChoseong
        | hanguljamo_subclass::ExtendedOldChoseong => hanguljamo_class::Choseong,
        hanguljamo_subclass::Jungseong
        | hanguljamo_subclass::JungseongFiller
        | hanguljamo_subclass::OldJungseong
        | hanguljamo_subclass::ExtendedOldJungseong => hanguljamo_class::Jungseong,
        hanguljamo_subclass::Jongseong
        | hanguljamo_subclass::OldJongseong
        | hanguljamo_subclass::ExtendedOldJongseong => hanguljamo_class::Jongseong,
        hanguljamo_subclass::NotHangulJamo => hanguljamo_class::NotHangulJamo,
    }
}

pub unsafe fn hanguljamo_check_state(
    p_ud: *const utf8_data,
    ud: *const utf8_data,
) -> hanguljamo_state {
    unsafe {
        if (*ud).size != 3 {
            return hanguljamo_state::NotHangulJamo;
        }

        match hanguljamo_get_class(&(&(*ud).data)[..3]) {
            hanguljamo_class::Choseong => hanguljamo_state::Choseong,
            hanguljamo_class::Jungseong => {
                if ((*p_ud).size as usize) < 3 {
                    return hanguljamo_state::NotComposable;
                }
                let off = (*p_ud).size as usize - 3;
                if hanguljamo_get_class(&(&(*p_ud).data)[off..off + 3])
                    == hanguljamo_class::Choseong
                {
                    hanguljamo_state::Composable
                } else {
                    hanguljamo_state::NotComposable
                }
            }
            hanguljamo_class::Jongseong => {
                if ((*p_ud).size as usize) < 3 {
                    return hanguljamo_state::NotComposable;
                }
                let off = (*p_ud).size as usize - 3;
                if hanguljamo_get_class(&(&(*p_ud).data)[off..off + 3])
                    == hanguljamo_class::Jungseong
                {
                    hanguljamo_state::Composable
                } else {
                    hanguljamo_state::NotComposable
                }
            }
            hanguljamo_class::NotHangulJamo => hanguljamo_state::NotHangulJamo,
        }
    }
}

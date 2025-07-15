// Copyright (c) 2008 Nicholas Marriott <nicholas.marriott@gmail.com>
//
// Permission u8, copy, modify, and distribute this software for any
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

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::fmt::{self, Display};
use std::slice;

use crate::*;

use crate::libc::{memcpy, memset};

use crate::compat::{
    tree::{rb_find, rb_initializer, rb_insert},
    vis,
};
use crate::xmalloc::xreallocarray;

#[cfg(feature = "utf8proc")]
unsafe extern "C" {
    fn utf8proc_wcwidth(_: wchar_t) -> i32;
    fn utf8proc_mbtowc(_: *mut wchar_t, _: *const u8, _: usize) -> i32;
    fn utf8proc_wctomb(_: *mut char, _: wchar_t) -> i32;
}

static UTF8_FORCE_WIDE: [wchar_t; 162] = [
    0x0261D, 0x026F9, 0x0270A, 0x0270B, 0x0270C, 0x0270D, 0x1F1E6, 0x1F1E7, 0x1F1E8, 0x1F1E9,
    0x1F1EA, 0x1F1EB, 0x1F1EC, 0x1F1ED, 0x1F1EE, 0x1F1EF, 0x1F1F0, 0x1F1F1, 0x1F1F2, 0x1F1F3,
    0x1F1F4, 0x1F1F5, 0x1F1F6, 0x1F1F7, 0x1F1F8, 0x1F1F9, 0x1F1FA, 0x1F1FB, 0x1F1FC, 0x1F1FD,
    0x1F1FE, 0x1F1FF, 0x1F385, 0x1F3C2, 0x1F3C3, 0x1F3C4, 0x1F3C7, 0x1F3CA, 0x1F3CB, 0x1F3CC,
    0x1F3FB, 0x1F3FC, 0x1F3FD, 0x1F3FE, 0x1F3FF, 0x1F442, 0x1F443, 0x1F446, 0x1F447, 0x1F448,
    0x1F449, 0x1F44A, 0x1F44B, 0x1F44C, 0x1F44D, 0x1F44E, 0x1F44F, 0x1F450, 0x1F466, 0x1F467,
    0x1F468, 0x1F469, 0x1F46B, 0x1F46C, 0x1F46D, 0x1F46E, 0x1F470, 0x1F471, 0x1F472, 0x1F473,
    0x1F474, 0x1F475, 0x1F476, 0x1F477, 0x1F478, 0x1F47C, 0x1F481, 0x1F482, 0x1F483, 0x1F485,
    0x1F486, 0x1F487, 0x1F48F, 0x1F491, 0x1F4AA, 0x1F574, 0x1F575, 0x1F57A, 0x1F590, 0x1F595,
    0x1F596, 0x1F645, 0x1F646, 0x1F647, 0x1F64B, 0x1F64C, 0x1F64D, 0x1F64E, 0x1F64F, 0x1F6A3,
    0x1F6B4, 0x1F6B5, 0x1F6B6, 0x1F6C0, 0x1F6CC, 0x1F90C, 0x1F90F, 0x1F918, 0x1F919, 0x1F91A,
    0x1F91B, 0x1F91C, 0x1F91D, 0x1F91E, 0x1F91F, 0x1F926, 0x1F930, 0x1F931, 0x1F932, 0x1F933,
    0x1F934, 0x1F935, 0x1F936, 0x1F937, 0x1F938, 0x1F939, 0x1F93D, 0x1F93E, 0x1F977, 0x1F9B5,
    0x1F9B6, 0x1F9B8, 0x1F9B9, 0x1F9BB, 0x1F9CD, 0x1F9CE, 0x1F9CF, 0x1F9D1, 0x1F9D2, 0x1F9D3,
    0x1F9D4, 0x1F9D5, 0x1F9D6, 0x1F9D7, 0x1F9D8, 0x1F9D9, 0x1F9DA, 0x1F9DB, 0x1F9DC, 0x1F9DD,
    0x1FAC3, 0x1FAC4, 0x1FAC5, 0x1FAF0, 0x1FAF1, 0x1FAF2, 0x1FAF3, 0x1FAF4, 0x1FAF5, 0x1FAF6,
    0x1FAF7, 0x1FAF8,
];

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct utf8_item_index {
    pub index: u32,
}

#[derive(Clone, Copy)] // TODO investigate manual clone
pub struct utf8_item_data {
    data: [MaybeUninit<u8>; UTF8_SIZE],
    size: u8,
}

impl utf8_item_data {
    fn new(bytes: &[u8]) -> Self {
        assert!(bytes.len() <= UTF8_SIZE);

        let mut data = [MaybeUninit::new(0); UTF8_SIZE];
        for (i, ch) in bytes.iter().enumerate() {
            data[i] = MaybeUninit::new(*ch);
        }
        Self {
            data,
            size: bytes.len() as u8,
        }
    }
}

impl Display for utf8_item_data {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(
            std::str::from_utf8(self.initialized_slice())
                .unwrap_or("invalid utf8 in utf8_item_data"),
        )
    }
}

/// once stabilized use: <https://doc.rust-lang.org/std/primitive.slice.html#method.assume_init_ref>
unsafe fn assume_init_ref<T>(data: &[MaybeUninit<T>]) -> &[T] {
    unsafe { std::slice::from_raw_parts(data.as_ptr().cast(), data.len()) }
}
impl utf8_item_data {
    fn initialized_slice(&self) -> &[u8] {
        // SAFETY: type invariant utf8_item_data.data should be initialized until self.size
        unsafe { assume_init_ref(&self.data[..self.size as usize]) }
    }
}

impl_ord!(utf8_item_data as utf8_data_cmp);

fn utf8_data_cmp(ui1: &utf8_item_data, ui2: &utf8_item_data) -> std::cmp::Ordering {
    ui1.initialized_slice().cmp(ui2.initialized_slice())
}

thread_local! {
    static UTF8_DATA_TREE: RefCell<BTreeMap<utf8_item_data, utf8_item_index>> = const { RefCell::new(BTreeMap::new()) };
    static UTF8_INDEX_TREE: RefCell<BTreeMap<utf8_item_index, utf8_item_data>> = const { RefCell::new(BTreeMap::new()) };
}

static mut UTF8_NEXT_INDEX: u32 = 0;

fn utf8_get_size(uc: utf8_char) -> u8 {
    (((uc) >> 24) & 0x1f) as u8
}
fn utf8_get_width(uc: utf8_char) -> u8 {
    (((uc) >> 29) - 1) as u8
}
fn utf8_set_size(size: u8) -> utf8_char {
    (size as utf8_char) << 24
}
fn utf8_set_width(width: u8) -> utf8_char {
    (width as utf8_char + 1) << 29
}

pub fn utf8_item_by_data(ud: &utf8_item_data) -> Option<utf8_item_index> {
    UTF8_DATA_TREE.with(|tree| tree.borrow().get(ud).copied())
}

pub fn utf8_item_by_index(index: u32) -> Option<utf8_item_data> {
    let ui = utf8_item_index { index };

    UTF8_INDEX_TREE.with(|tree| tree.borrow().get(&ui).copied())
}

pub unsafe fn utf8_put_item(ud: &utf8_item_data) -> Result<u32, ()> {
    unsafe {
        let ui = utf8_item_by_data(ud);
        if let Some(ui) = ui {
            let index = ui.index;
            log_debug!("utf8_put_item: found {ud} = {index}");
            return Ok(index);
        }

        if UTF8_NEXT_INDEX == 0xffffff + 1 {
            return Err(());
        }

        let ui_index = utf8_item_index {
            index: UTF8_NEXT_INDEX,
        };
        UTF8_NEXT_INDEX += 1;

        let ui_data = *ud;
        UTF8_INDEX_TREE.with(|tree| tree.borrow_mut().insert(ui_index, ui_data));
        UTF8_DATA_TREE.with(|tree| tree.borrow_mut().insert(ui_data, ui_index));

        let index = ui_index.index;
        log_debug!("utf8_put_item: added {ud} = {index}");
        Ok(index)
    }
}

pub unsafe extern "C" fn utf8_table_cmp(vp1: *const c_void, vp2: *const c_void) -> i32 {
    let wc1 = vp1 as *const wchar_t;
    let wc2 = vp2 as *const wchar_t;
    unsafe { wchar_t::cmp(&*wc1, &*wc2) as i8 as i32 }
}

pub fn utf8_in_table(find: wchar_t, table: &[wchar_t]) -> bool {
    table.binary_search(&find).is_ok()
}

pub unsafe fn utf8_from_data(ud: &utf8_data) -> Result<utf8_char, utf8_char> {
    unsafe {
        let mut index: u32 = 0;
        'fail: {
            if ud.width > 2 {
                fatalx_!("invalid UTF-8 width: {}", ud.width);
            }

            if ud.size > UTF8_SIZE as u8 {
                break 'fail;
            }
            if ud.size <= 3 {
                index =
                    ((ud.data[2] as u32) << 16) | ((ud.data[1] as u32) << 8) | (ud.data[0] as u32);
            } else {
                match utf8_put_item(&utf8_item_data::new(ud.initialized_slice())) {
                    Ok(value) => index = value,
                    Err(()) => break 'fail,
                }
            }
            let uc = utf8_set_size(ud.size) | utf8_set_width(ud.width) | index;
            log_debug!(
                "utf8_from_data: ({} {} {}) -> {:08x}",
                ud.width,
                ud.size,
                String::from_utf8_lossy(ud.initialized_slice()),
                uc,
            );
            return Ok(uc);
        }

        // fail:
        if ud.width == 0 {
            Err(utf8_set_size(0) | utf8_set_width(0))
        } else if ud.width == 1 {
            Err(utf8_set_size(1) | utf8_set_width(1) | 0x20)
        } else {
            Err(utf8_set_size(1) | utf8_set_width(1) | 0x2020)
        }
    }
}

pub fn utf8_to_data(uc: utf8_char) -> utf8_data {
    let mut ud = utf8_data {
        data: [0; UTF8_SIZE],
        size: utf8_get_size(uc),
        have: utf8_get_size(uc),
        width: utf8_get_width(uc),
    };

    if ud.size <= 3 {
        ud.data[2] = (uc >> 16) as u8;
        ud.data[1] = ((uc >> 8) & 0xff) as u8;
        ud.data[0] = (uc & 0xff) as u8;
    } else {
        let index = uc & 0xffffff;
        if let Some(ui) = utf8_item_by_index(index) {
            ud.data[..ud.size as usize].copy_from_slice(ui.initialized_slice());
        } else {
            ud.data[..ud.size as usize].fill(b' ');
        }
    }

    log_debug!(
        "utf8_to_data: {:08x} -> ({} {} {})",
        uc,
        ud.width,
        ud.size,
        String::from_utf8_lossy(ud.initialized_slice())
    );

    ud
}

pub fn utf8_build_one(ch: c_uchar) -> u32 {
    utf8_set_size(1) | utf8_set_width(1) | ch as u32
}

pub unsafe fn utf8_set(ud: *mut utf8_data, ch: c_uchar) {
    static EMPTY: utf8_data = utf8_data {
        data: unsafe { zeroed() },
        have: 1,
        size: 1,
        width: 1,
    };

    unsafe {
        memcpy__(ud, &raw const EMPTY);
        (*ud).data[0] = ch;
    }
}

pub unsafe fn utf8_copy(to: *mut utf8_data, from: *const utf8_data) {
    unsafe {
        memcpy__(to, from);

        for i in (*to).size..(UTF8_SIZE as u8) {
            (*to).data[i as usize] = b'\0';
        }
    }
}

pub unsafe fn utf8_width(ud: *mut utf8_data, width: *mut i32) -> utf8_state {
    unsafe {
        let mut wc: wchar_t = 0;

        if utf8_towc(ud, &raw mut wc) != utf8_state::UTF8_DONE {
            return utf8_state::UTF8_ERROR;
        }
        if utf8_in_table(wc, UTF8_FORCE_WIDE.as_slice()) {
            *width = 2;
            return utf8_state::UTF8_DONE;
        }
        if cfg!(feature = "utf8proc") {
            #[cfg(feature = "utf8proc")]
            {
                *width = utf8proc_wcwidth(wc);
                log_debug!("utf8proc_wcwidth({:05X}) returned {}", wc, *width);
            }
        } else {
            *width = wcwidth(wc);
            log_debug!("wcwidth({:05X}) returned {}", wc, *width);
            if *width < 0 {
                *width = if wc >= 0x80 && wc <= 0x9f { 0 } else { 1 };
            }
        }
        if *width >= 0 && *width <= 0xff {
            return utf8_state::UTF8_DONE;
        }
        utf8_state::UTF8_ERROR
    }
}

pub unsafe fn utf8_towc(ud: *const utf8_data, wc: *mut wchar_t) -> utf8_state {
    unsafe {
        #[cfg(feature = "utf8proc")]
        let value = utf8proc_mbtowc(wc, (*ud).data.as_ptr().cast(), (*ud).size as usize);
        #[cfg(not(feature = "utf8proc"))]
        let value = mbtowc(wc, (*ud).data.as_ptr().cast(), (*ud).size as usize);

        match value {
            -1 => {
                log_debug!(
                    "UTF-8 {}, mbtowc() {}",
                    String::from_utf8_lossy((*ud).initialized_slice()),
                    errno!(),
                );
                mbtowc(null_mut(), null(), MB_CUR_MAX());
                return utf8_state::UTF8_ERROR;
            }
            0 => return utf8_state::UTF8_ERROR,
            _ => (),
        }
        log_debug!(
            "UTF-8 {1:0$} is {2:5X}",
            (*ud).size as usize,
            _s((&raw const (*ud).data).cast::<u8>()),
            *wc as u32,
        );
    }

    utf8_state::UTF8_DONE
}

pub unsafe fn utf8_fromwc(wc: wchar_t, ud: *mut utf8_data) -> utf8_state {
    unsafe {
        let mut width: i32 = 0;

        #[cfg(feature = "utf8proc")]
        let size = utf8proc_wctomb((*ud).data.as_mut_ptr().cast(), wc);
        #[cfg(not(feature = "utf8proc"))]
        let size = wctomb((*ud).data.as_mut_ptr().cast(), wc);

        if size < 0 {
            log_debug!("UTF-8 {}, wctomb() {}", wc, errno!());
            wctomb(null_mut(), 0);
            return utf8_state::UTF8_ERROR;
        }
        if size == 0 {
            return utf8_state::UTF8_ERROR;
        }
        (*ud).have = size as u8;
        (*ud).size = size as u8;
        if utf8_width(ud, &raw mut width) == utf8_state::UTF8_DONE {
            (*ud).width = width as u8;
            return utf8_state::UTF8_DONE;
        }
    }
    utf8_state::UTF8_ERROR
}

pub unsafe fn utf8_open(ud: *mut utf8_data, ch: c_uchar) -> utf8_state {
    unsafe {
        memset(ud.cast(), 0, size_of::<utf8_data>());

        (*ud).size = match ch {
            0xc2..=0xdf => 2,
            0xe0..=0xef => 3,
            0xf0..=0xf4 => 4,
            _ => return utf8_state::UTF8_ERROR,
        };

        utf8_append(ud, ch);
    }

    utf8_state::UTF8_MORE
}

pub unsafe fn utf8_append(ud: *mut utf8_data, ch: c_uchar) -> utf8_state {
    unsafe {
        let mut width: i32 = 0;

        if (*ud).have >= (*ud).size {
            fatalx("UTF-8 character overflow");
        }
        if (*ud).size > UTF8_SIZE as u8 {
            fatalx("UTF-8 character size too large");
        }

        if (*ud).have != 0 && (ch & 0xc0) != 0x80 {
            (*ud).width = 0xff;
        }

        (*ud).data[(*ud).have as usize] = ch;
        (*ud).have += 1;
        if (*ud).have != (*ud).size {
            return utf8_state::UTF8_MORE;
        }

        if (*ud).width == 0xff {
            return utf8_state::UTF8_ERROR;
        }
        if utf8_width(ud, &raw mut width) != utf8_state::UTF8_DONE {
            return utf8_state::UTF8_ERROR;
        }
        (*ud).width = width as u8;
    }
    utf8_state::UTF8_DONE
}

pub unsafe fn utf8_strvis(
    mut dst: *mut u8,
    mut src: *const u8,
    len: usize,
    flag: vis_flags,
) -> i32 {
    unsafe {
        let mut ud: utf8_data = zeroed();
        let start = dst;
        let end = src.add(len);
        let mut more: utf8_state;

        while src < end {
            more = utf8_open(&raw mut ud, (*src));
            if more == utf8_state::UTF8_MORE {
                src = src.add(1);
                while src < end && more == utf8_state::UTF8_MORE {
                    more = utf8_append(&raw mut ud, (*src));
                }
                if more == utf8_state::UTF8_DONE {
                    /* UTF-8 character finished. */
                    for i in 0..ud.size {
                        *dst = ud.data[i as usize];
                        dst = dst.add(1);
                    }
                    continue;
                }
                /* Not a complete, valid UTF-8 character. */
                src = src.sub(ud.have as usize);
            }
            if flag.intersects(vis_flags::VIS_DQ) && *src == b'$' && src < end.sub(1) {
                if (*src.add(1)).is_ascii_alphabetic() || *src.add(1) == b'_' || *src.add(1) == b'{'
                {
                    *dst = b'\\';
                    dst = dst.add(1);
                }
                *dst = b'$';
                dst = dst.add(1);
            } else if src < end.sub(1) {
                dst = vis(dst, *src as i32, flag, *src.add(1) as i32);
            } else if src < end {
                dst = vis(dst, *src as i32, flag, b'\0' as i32);
            }
            src = src.add(1);
        }
        *dst = b'\0';
        (dst.addr() - start.addr()) as i32
    }
}

pub unsafe fn utf8_stravis(dst: *mut *mut u8, src: *const u8, flag: vis_flags) -> i32 {
    unsafe {
        let buf = xreallocarray(null_mut(), 4, strlen(src) + 1);
        let len = utf8_strvis(buf.as_ptr().cast(), src, strlen(src), flag);

        *dst = xrealloc(buf.as_ptr(), len as usize + 1).as_ptr().cast();
        len
    }
}

pub unsafe fn utf8_stravisx(
    dst: *mut *mut u8,
    src: *const u8,
    srclen: usize,
    flag: vis_flags,
) -> i32 {
    unsafe {
        let buf = xreallocarray(null_mut(), 4, srclen + 1);
        let len = utf8_strvis(buf.as_ptr().cast(), src, srclen, flag);

        *dst = xrealloc(buf.as_ptr(), len as usize + 1).as_ptr().cast();
        len
    }
}

pub unsafe fn utf8_isvalid(mut s: *const u8) -> bool {
    unsafe {
        let mut ud: utf8_data = zeroed();

        let end = s.add(strlen(s));
        while s < end {
            let mut more = utf8_open(&raw mut ud, (*s));
            if more == utf8_state::UTF8_MORE {
                while {
                    s = s.add(1);
                    s < end && more == utf8_state::UTF8_MORE
                } {
                    more = utf8_append(&raw mut ud, (*s));
                }
                if more == utf8_state::UTF8_DONE {
                    continue;
                }
                return false;
            }
            if *s < 0x20 || *s > 0x7e {
                return false;
            }
            s = s.add(1);
        }
    }

    true
}

pub unsafe fn utf8_sanitize(mut src: *const u8) -> *mut u8 {
    unsafe {
        let mut dst: *mut u8 = null_mut();
        let mut n: usize = 0;
        let mut ud: utf8_data = zeroed();

        while *src != b'\0' {
            dst = xreallocarray_(dst, n + 1).as_ptr();
            let mut more = utf8_open(&raw mut ud, (*src));
            if more == utf8_state::UTF8_MORE {
                while {
                    src = src.add(1);
                    *src != b'\0' && more == utf8_state::UTF8_MORE
                } {
                    more = utf8_append(&raw mut ud, (*src));
                }
                if more == utf8_state::UTF8_DONE {
                    dst = xreallocarray_(dst, n + ud.width as usize).as_ptr();
                    for _ in 0..ud.width {
                        *dst.add(n) = b'_';
                        n += 1;
                    }
                    continue;
                }
                src = src.sub(ud.have as usize);
            }
            if *src > 0x1f && *src < 0x7f {
                *dst.add(n) = *src;
                n += 1;
            } else {
                *dst.add(n) = b'_';
                n += 1;
            }
            src = src.add(1);
        }
        dst = xreallocarray_(dst, n + 1).as_ptr();
        *dst.add(n) = b'\0';
        dst
    }
}

pub unsafe fn utf8_strlen(s: *const utf8_data) -> usize {
    let mut i = 0;

    unsafe {
        while (*s.add(i)).size != 0 {
            i += 1;
        }
    }

    i
}
pub fn utf8_strlen_(s: &[utf8_data]) -> usize {
    s.len()
}

pub unsafe fn utf8_strwidth(s: *const utf8_data, n: isize) -> u32 {
    unsafe {
        let mut width: u32 = 0;

        let mut i: isize = 0;
        while (*s.add(i as usize)).size != 0 {
            if n != -1 && n == i {
                break;
            }
            width += (*s.add(i as usize)).width as u32;
            i += 1;
        }

        width
    }
}

pub unsafe fn utf8_fromcstr(mut src: *const u8) -> Box<[utf8_data]> {
    unsafe {
        let mut dst: Vec<utf8_data> = Vec::new();
        let mut n = 0;

        while *src != b'\0' {
            dst.reserve(1);
            let mut more = utf8_open(dst.as_mut_ptr().add(n), (*src));
            dst.set_len(dst.len() + 1);
            if more == utf8_state::UTF8_MORE {
                while {
                    src = src.add(1);
                    *src != b'\0' && more == utf8_state::UTF8_MORE
                } {
                    more = utf8_append(dst.as_mut_ptr().add(n), (*src));
                }
                if more == utf8_state::UTF8_DONE {
                    n += 1;
                    continue;
                }
                src = src.sub(dst[n].have as usize);
            }
            utf8_set(dst.as_mut_ptr().add(n), (*src));
            n += 1;
            src = src.add(1);
        }

        // TODO, this is very hacky and should be removed once
        // all the code uses &[utf8_data] or Box<[utf8_data]> instead of *mut utf8_data
        //
        // The idea is we write an extra nul field without changing the size
        // that way old code which looks for the size == 0 to terminate will still work
        // but new code can use the length from the box slice
        // note there could be some safety issues if more fields are appended later
        // this should be removed and just use the len from the box slice
        dst.reserve(1);
        (*dst.as_mut_ptr().add(n)).size = 0;
        // dst.set_len(dst.len() + 1); // don't increase the length so that a sentinal isn't included

        dst.into_boxed_slice()
    }
}

pub unsafe fn utf8_tocstr(mut src: *mut utf8_data) -> *mut u8 {
    unsafe {
        let mut dst = null_mut::<u8>();
        let mut n: usize = 0;

        while (*src).size != 0 {
            dst = xreallocarray_(dst, n + (*src).size as usize).as_ptr();
            memcpy(
                dst.add(n).cast(),
                (*src).data.as_ptr().cast(),
                (*src).size as usize,
            );
            n += (*src).size as usize;
            src = src.add(1);
        }
        dst = xreallocarray_(dst, n + 1).as_ptr();
        *dst.add(n) = b'\0';
        dst
    }
}

pub unsafe fn utf8_cstrwidth(mut s: *const u8) -> u32 {
    unsafe {
        let mut tmp: utf8_data = zeroed();

        let mut width: u32 = 0;
        while *s != b'\0' {
            let mut more = utf8_open(&raw mut tmp, (*s));
            if more == utf8_state::UTF8_MORE {
                while {
                    s = s.add(1);
                    *s != b'\0' && more == utf8_state::UTF8_MORE
                } {
                    more = utf8_append(&raw mut tmp, (*s));
                }
                if more == utf8_state::UTF8_DONE {
                    width += tmp.width as u32;
                    continue;
                }
                s = s.sub(tmp.have as usize);
            }
            if *s > 0x1f && *s != 0x7f {
                width += 1;
            }
            s = s.add(1);
        }
        width
    }
}

pub unsafe fn utf8_padcstr(s: *const u8, width: u32) -> *mut u8 {
    unsafe {
        let n = utf8_cstrwidth(s);
        if n >= width {
            return xstrdup(s).as_ptr();
        }

        let mut slen = strlen(s);
        let out: *mut u8 = xmalloc(slen + 1 + (width - n) as usize).as_ptr().cast();
        memcpy(out.cast(), s.cast(), slen);
        let mut i = n;
        while i < width {
            *out.add(slen) = b' ';
            slen += 1;
            i += 1;
        }
        *out.add(slen) = b'\0';
        out
    }
}

pub unsafe fn utf8_rpadcstr(s: *const u8, width: u32) -> *mut u8 {
    unsafe {
        let n = utf8_cstrwidth(s);
        if n >= width {
            return xstrdup(s).as_ptr();
        }

        let slen = strlen(s);
        let out: *mut u8 = xmalloc(slen + 1 + (width - n) as usize).as_ptr().cast();
        let mut i = 0;
        while i < width {
            *out.add(i as usize) = b' ';
            i += 1;
        }
        memcpy(out.add(i as usize).cast(), s.cast(), slen);
        *out.add(i as usize + slen) = b'\0';
        out
    }
}

// TODO change to bool
pub unsafe fn utf8_cstrhas(s: *const u8, ud: &utf8_data) -> i32 {
    unsafe {
        utf8_fromcstr(s)
            .iter()
            .any(|loop_| loop_.initialized_slice() == ud.initialized_slice()) as i32
    }
}

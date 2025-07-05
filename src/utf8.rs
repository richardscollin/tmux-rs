// Copyright (c) 2008 Nicholas Marriott <nicholas.marriott@gmail.com>
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

use libc::{memcpy, memset};

use crate::compat::{
    tree::{rb_find, rb_initializer, rb_insert},
    vis,
};
use crate::xmalloc::xreallocarray;

#[cfg(feature = "utf8proc")]
unsafe extern "C" {
    fn utf8proc_wcwidth(_: wchar_t) -> i32;
    fn utf8proc_mbtowc(_: *mut wchar_t, _: *const c_char, _: usize) -> i32;
    fn utf8proc_wctomb(_: *mut char, _: wchar_t) -> i32;
}

static utf8_force_wide: [wchar_t; 162] = [
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

#[repr(C)]
pub struct utf8_item {
    pub index_entry: rb_entry<utf8_item>,
    pub index: u32,

    pub data_entry: rb_entry<utf8_item>,
    pub data: [c_char; UTF8_SIZE],
    pub size: c_uchar,
}

pub fn utf8_data_cmp(ui1: &utf8_item, ui2: &utf8_item) -> std::cmp::Ordering {
    ui1.size
        .cmp(&ui2.size)
        .then_with(|| ui1.data[..ui1.size as usize].cmp(&ui2.data[..ui2.size as usize]))
}
pub type utf8_data_tree = rb_head<utf8_item>;
RB_GENERATE!(
    utf8_data_tree,
    utf8_item,
    data_entry,
    discr_data_entry,
    utf8_data_cmp
);
static mut utf8_data_tree: utf8_data_tree = rb_initializer();

pub fn utf8_index_cmp(ui1: &utf8_item, ui2: &utf8_item) -> std::cmp::Ordering {
    ui1.index.cmp(&ui2.index)
}
pub type utf8_index_tree = rb_head<utf8_item>;
RB_GENERATE!(
    utf8_index_tree,
    utf8_item,
    index_entry,
    discr_index_entry,
    utf8_index_cmp
);
static mut utf8_index_tree: utf8_index_tree = rb_initializer();

static mut utf8_next_index: u32 = 0;

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

pub unsafe fn utf8_item_by_data(data: *const [i8; UTF8_SIZE], size: usize) -> *mut utf8_item {
    unsafe {
        let mut ui = MaybeUninit::<utf8_item>::uninit();
        let ui = ui.as_mut_ptr();

        memcpy(
            (*ui).data.as_mut_ptr().cast(),
            (&raw const data).cast(),
            size,
        );
        (*ui).size = size as u8;

        rb_find::<_, discr_data_entry>(&raw mut utf8_data_tree, ui)
    }
}

pub unsafe fn utf8_item_by_index(index: u32) -> *mut utf8_item {
    unsafe {
        let mut ui = MaybeUninit::<utf8_item>::uninit();
        let ui = ui.as_mut_ptr();

        (*ui).index = index;

        rb_find::<_, discr_index_entry>(&raw mut utf8_index_tree, ui)
    }
}

pub unsafe fn utf8_put_item(data: *const [c_char; UTF8_SIZE], size: usize, index: *mut u32) -> i32 {
    unsafe {
        let ui = utf8_item_by_data(data, size);
        if !ui.is_null() {
            *index = (*ui).index;
            log_debug!(
                "utf8_put_item: found {1:0$} = {2}",
                size,
                _s((&raw const data) as *const c_char),
                *index,
            );
            return 0;
        }

        if utf8_next_index == 0xffffff + 1 {
            return -1;
        }

        let ui: &mut utf8_item = xcalloc1();
        ui.index = utf8_next_index;
        utf8_next_index += 1;
        rb_insert::<_, discr_index_entry>(&raw mut utf8_index_tree, ui);

        memcpy(ui.data.as_mut_ptr().cast(), data.cast(), size);
        ui.size = size as u8;
        rb_insert::<_, discr_data_entry>(&raw mut utf8_data_tree, ui);

        *index = ui.index;
        log_debug!(
            "utf8_put_item: added {1:0$} = {2}",
            size,
            _s((&raw const data).cast()),
            *index,
        );
        0
    }
}

pub unsafe extern "C" fn utf8_table_cmp(vp1: *const c_void, vp2: *const c_void) -> i32 {
    let wc1 = vp1 as *const wchar_t;
    let wc2 = vp2 as *const wchar_t;
    unsafe { wchar_t::cmp(&*wc1, &*wc2) as i8 as i32 }
}

pub unsafe fn utf8_in_table(find: wchar_t, table: *const wchar_t, count: u32) -> i32 {
    unsafe {
        let found = bsearch_(
            &raw const find,
            table,
            count as usize,
            size_of::<wchar_t>(),
            utf8_table_cmp,
        );
        !found.is_null() as i32
    }
}

pub unsafe fn utf8_from_data(ud: *const utf8_data, uc: *mut utf8_char) -> utf8_state {
    unsafe {
        let mut index: u32 = 0;
        'fail: {
            if (*ud).width > 2 {
                fatalx_!("invalid UTF-8 width: {}", (*ud).width);
            }

            if (*ud).size > UTF8_SIZE as u8 {
                break 'fail;
            }
            if (*ud).size <= 3 {
                index = (((*ud).data[2] as u32) << 16)
                    | (((*ud).data[1] as u32) << 8)
                    | ((*ud).data[0] as u32);
            } else if utf8_put_item(
                (&raw const (*ud).data).cast(),
                (*ud).size as usize,
                &raw mut index,
            ) != 0
            {
                break 'fail;
            }
            *uc = utf8_set_size((*ud).size) | utf8_set_width((*ud).width) | index;
            log_debug!(
                "utf8_from_data: ({0} {1} {3:2$}) -> {4:08x}",
                (*ud).width,
                (*ud).size,
                (*ud).size as usize,
                _s((&raw const (*ud).data).cast()),
                *uc,
            );
            return utf8_state::UTF8_DONE;
        }

        // fail:
        *uc = if (*ud).width == 0 {
            utf8_set_size(0) | utf8_set_width(0)
        } else if (*ud).width == 1 {
            utf8_set_size(1) | utf8_set_width(1) | 0x20
        } else {
            utf8_set_size(1) | utf8_set_width(1) | 0x2020
        };
        utf8_state::UTF8_ERROR
    }
}

pub unsafe fn utf8_to_data(uc: utf8_char, ud: *mut utf8_data) {
    unsafe {
        core::ptr::write(ud, zeroed());
        (*ud).size = utf8_get_size(uc);
        (*ud).have = utf8_get_size(uc);
        (*ud).width = utf8_get_width(uc);

        if (*ud).size <= 3 {
            (*ud).data[2] = (uc >> 16) as u8;
            (*ud).data[1] = ((uc >> 8) & 0xff) as u8;
            (*ud).data[0] = (uc & 0xff) as u8;
        } else {
            let index = uc & 0xffffff;
            let ui = utf8_item_by_index(index);
            if ui.is_null() {
                memset(
                    (*ud).data.as_mut_ptr().cast(),
                    b' ' as i32,
                    (*ud).size as usize,
                );
            } else {
                memcpy(
                    (*ud).data.as_mut_ptr().cast(),
                    (*ui).data.as_mut_ptr().cast(),
                    (*ud).size as usize,
                );
            }
        }

        log_debug!(
            "utf8_to_data: {0:08x} -> ({1} {2} {4:3$})",
            uc,
            (*ud).width,
            (*ud).size,
            (*ud).size as usize,
            _s((&raw const (*ud).data).cast()),
        );
    }
}

pub fn utf8_build_one(ch: c_uchar) -> u32 {
    utf8_set_size(1) | utf8_set_width(1) | ch as u32
}

pub unsafe fn utf8_set(ud: *mut utf8_data, ch: c_uchar) {
    static empty: utf8_data = utf8_data {
        data: unsafe { zeroed() },
        have: 1,
        size: 1,
        width: 1,
    };

    unsafe {
        memcpy__(ud, &raw const empty);
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
        if utf8_in_table(wc, utf8_force_wide.as_ptr(), utf8_force_wide.len() as u32) != 0 {
            *width = 2;
            return utf8_state::UTF8_DONE;
        }
        if cfg!(feature = "utf8proc") {
            #[cfg(feature = "utf8proc")]
            {
                *width = utf8proc_wcwidth(wc);
                log_debug_c(
                    c"utf8proc_wcwidth(%05X) returned %d".as_ptr(),
                    wc as u32,
                    *width,
                );
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
                    "UTF-8 {1:0$}, mbtowc() {2}",
                    (*ud).size as usize,
                    _s((&raw const (*ud).data).cast()),
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
            _s((&raw const (*ud).data).cast()),
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
    mut dst: *mut c_char,
    mut src: *const c_char,
    len: usize,
    flag: vis_flags,
) -> i32 {
    unsafe {
        let mut ud: utf8_data = zeroed();
        let start = dst;
        let end = src.add(len);
        let mut more: utf8_state;

        while src < end {
            more = utf8_open(&raw mut ud, *src as u8);
            if more == utf8_state::UTF8_MORE {
                src = src.add(1);
                while src < end && more == utf8_state::UTF8_MORE {
                    more = utf8_append(&raw mut ud, *src as u8);
                }
                if more == utf8_state::UTF8_DONE {
                    /* UTF-8 character finished. */
                    for i in 0..ud.size {
                        *dst = ud.data[i as usize] as i8;
                        dst = dst.add(1);
                    }
                    continue;
                }
                /* Not a complete, valid UTF-8 character. */
                src = src.sub(ud.have as usize);
            }
            if flag.intersects(vis_flags::VIS_DQ) && *src == b'$' as c_char && src < end.sub(1) {
                if (*src.add(1) as u8).is_ascii_alphabetic()
                    || *src.add(1) == b'_' as c_char
                    || *src.add(1) == b'{' as c_char
                {
                    *dst = b'\\' as c_char;
                    dst = dst.add(1);
                }
                *dst = b'$' as c_char;
                dst = dst.add(1);
            } else if src < end.sub(1) {
                dst = vis(dst, *src as i32, flag, *src.add(1) as i32);
            } else if src < end {
                dst = vis(dst, *src as i32, flag, b'\0' as i32);
            }
            src = src.add(1);
        }
        *dst = b'\0' as c_char;
        (dst.addr() - start.addr()) as i32
    }
}

pub unsafe fn utf8_stravis(dst: *mut *mut c_char, src: *const c_char, flag: vis_flags) -> i32 {
    unsafe {
        let buf = xreallocarray(null_mut(), 4, strlen(src) + 1);
        let len = utf8_strvis(buf.as_ptr().cast(), src, strlen(src), flag);

        *dst = xrealloc(buf.as_ptr(), len as usize + 1).as_ptr().cast();
        len
    }
}

pub unsafe fn utf8_stravisx(
    dst: *mut *mut c_char,
    src: *const c_char,
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

pub unsafe fn utf8_isvalid(mut s: *const c_char) -> bool {
    unsafe {
        let mut ud: utf8_data = zeroed();

        let end = s.add(strlen(s));
        while s < end {
            let mut more = utf8_open(&raw mut ud, *s as u8);
            if more == utf8_state::UTF8_MORE {
                while {
                    s = s.add(1);
                    s < end && more == utf8_state::UTF8_MORE
                } {
                    more = utf8_append(&raw mut ud, *s as u8);
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

pub unsafe fn utf8_sanitize(mut src: *const c_char) -> *mut c_char {
    unsafe {
        let mut dst: *mut c_char = null_mut();
        let mut n: usize = 0;
        let mut ud: utf8_data = zeroed();

        while *src != b'\0' as c_char {
            dst = xreallocarray_(dst, n + 1).as_ptr();
            let mut more = utf8_open(&raw mut ud, *src as u8);
            if more == utf8_state::UTF8_MORE {
                while {
                    src = src.add(1);
                    *src != b'\0' as c_char && more == utf8_state::UTF8_MORE
                } {
                    more = utf8_append(&raw mut ud, *src as u8);
                }
                if more == utf8_state::UTF8_DONE {
                    dst = xreallocarray_(dst, n + ud.width as usize).as_ptr();
                    for _ in 0..ud.width {
                        *dst.add(n) = b'_' as c_char;
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
                *dst.add(n) = b'_' as c_char;
                n += 1;
            }
            src = src.add(1);
        }
        dst = xreallocarray_(dst, n + 1).as_ptr();
        *dst.add(n) = b'\0' as c_char;
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

pub unsafe fn utf8_fromcstr(mut src: *const c_char) -> *mut utf8_data {
    unsafe {
        let mut dst: *mut utf8_data = null_mut();
        let mut n = 0;

        while *src != b'\0' as c_char {
            dst = xreallocarray_(dst, n + 1).as_ptr();
            let mut more = utf8_open(dst.add(n), *src as u8);
            if more == utf8_state::UTF8_MORE {
                while {
                    src = src.add(1);
                    *src != b'\0' as c_char && more == utf8_state::UTF8_MORE
                } {
                    more = utf8_append(dst.add(n), *src as u8);
                }
                if more == utf8_state::UTF8_DONE {
                    n += 1;
                    continue;
                }
                src = src.sub((*dst.add(n)).have as usize);
            }
            utf8_set(dst.add(n), *src as u8);
            n += 1;
            src = src.add(1);
        }
        dst = xreallocarray_(dst, n + 1).as_ptr();
        (*dst.add(n)).size = 0;

        dst
    }
}

pub unsafe fn utf8_tocstr(mut src: *mut utf8_data) -> *mut c_char {
    unsafe {
        let mut dst = null_mut::<c_char>();
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
        *dst.add(n) = b'\0' as c_char;
        dst
    }
}

pub unsafe fn utf8_cstrwidth(mut s: *const c_char) -> u32 {
    unsafe {
        let mut tmp: utf8_data = zeroed();

        let mut width: u32 = 0;
        while *s != b'\0' as c_char {
            let mut more = utf8_open(&raw mut tmp, *s as u8);
            if more == utf8_state::UTF8_MORE {
                while {
                    s = s.add(1);
                    *s != b'\0' as c_char && more == utf8_state::UTF8_MORE
                } {
                    more = utf8_append(&raw mut tmp, *s as u8);
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

pub unsafe fn utf8_padcstr(s: *const c_char, width: u32) -> *mut c_char {
    unsafe {
        let n = utf8_cstrwidth(s);
        if n >= width {
            return xstrdup(s).as_ptr();
        }

        let mut slen = strlen(s);
        let out: *mut c_char = xmalloc(slen + 1 + (width - n) as usize).as_ptr().cast();
        memcpy(out.cast(), s.cast(), slen);
        let mut i = n;
        while i < width {
            *out.add(slen) = b' ' as c_char;
            slen += 1;
            i += 1;
        }
        *out.add(slen) = b'\0' as c_char;
        out
    }
}

pub unsafe fn utf8_rpadcstr(s: *const c_char, width: u32) -> *mut c_char {
    unsafe {
        let n = utf8_cstrwidth(s);
        if n >= width {
            return xstrdup(s).as_ptr();
        }

        let slen = strlen(s);
        let out: *mut c_char = xmalloc(slen + 1 + (width - n) as usize).as_ptr().cast();
        let mut i = 0;
        while i < width {
            *out.add(i as usize) = b' ' as c_char;
            i += 1;
        }
        memcpy(out.add(i as usize).cast(), s.cast(), slen);
        *out.add(i as usize + slen) = b'\0' as c_char;
        out
    }
}

pub unsafe fn utf8_cstrhas(s: *const c_char, ud: *const utf8_data) -> i32 {
    let mut found: i32 = 0;

    unsafe {
        let copy = utf8_fromcstr(s);
        let mut loop_ = copy;
        while (*loop_).size != 0 {
            if (*loop_).size != (*ud).size {
                loop_ = loop_.add(1);
                continue;
            }
            if memcmp(
                (*loop_).data.as_ptr().cast(),
                (*ud).data.as_ptr().cast(),
                (*loop_).size as usize,
            ) == 0
            {
                found = 1;
                break;
            }
            loop_ = loop_.add(1);
        }

        free_(copy);

        found
    }
}

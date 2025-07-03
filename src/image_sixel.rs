// Copyright (c) 2019 Nicholas Marriott <nicholas.marriott@gmail.com>
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

use crate::compat::strtonum;
use crate::xmalloc::xrecallocarray__;

pub const SIXEL_COLOUR_REGISTERS: u32 = 1024;
const SIXEL_WIDTH_LIMIT: u32 = 10000;
const SIXEL_HEIGHT_LIMIT: u32 = 10000;

#[repr(C)]
struct sixel_line {
    x: u32,
    data: *mut u16,
}

#[repr(C)]
pub struct sixel_image {
    x: u32,
    y: u32,
    xpixel: u32,
    ypixel: u32,

    colours: *mut u32,
    ncolours: u32,

    dx: u32,
    dy: u32,
    dc: u32,

    lines: *mut sixel_line,
}

unsafe fn sixel_parse_expand_lines(si: *mut sixel_image, y: u32) -> i32 {
    unsafe {
        if y <= (*si).y {
            return 0;
        }
        if y > SIXEL_HEIGHT_LIMIT {
            return 1;
        }
        (*si).lines = xrecallocarray__((*si).lines, (*si).y as usize, y as usize).as_ptr();
        (*si).y = y;
        0
    }
}

unsafe fn sixel_parse_expand_line(
    si: *mut sixel_image,
    sl: *mut sixel_line,
    x: u32,
) -> i32 {
    unsafe {
        if x <= (*sl).x {
            return 0;
        }
        if x > SIXEL_WIDTH_LIMIT {
            return 1;
        }
        if (x > (*si).x) {
            (*si).x = x;
        }
        (*sl).data = xrecallocarray__((*sl).data, (*sl).x as usize, (*si).x as usize).as_ptr();
        (*sl).x = (*si).x;
        0
    }
}

unsafe fn sixel_get_pixel(si: *mut sixel_image, x: u32, y: u32) -> u32 {
    unsafe {
        if y >= (*si).y {
            return 0;
        }
        let sl = (*si).lines.add(y as usize);
        if x >= (*sl).x {
            return 0;
        }
        *(*sl).data.add(x as usize) as u32
    }
}

unsafe fn sixel_set_pixel(si: *mut sixel_image, x: u32, y: u32, c: u32) -> i32 {
    unsafe {
        if sixel_parse_expand_lines(si, y + 1) != 0 {
            return 1;
        }
        let sl = (*si).lines.add(y as usize);
        if sixel_parse_expand_line(si, sl, x + 1) != 0 {
            return 1;
        }
        *(*sl).data.add(x as usize) = c as u16;

        0
    }
}

unsafe fn sixel_parse_write(si: *mut sixel_image, ch: u32) -> i32 {
    if sixel_parse_expand_lines(si, (*si).dy + 6) != 0 {
        return 1;
    }
    let mut sl = (*si).lines.add((*si).dy as usize);

    for i in 0..6 {
        if sixel_parse_expand_line(si, sl, (*si).dx + 1) != 0 {
            return 1;
        }
        if ch & (1 << i) != 0 {
            *(*sl).data.add((*si).dx as usize) = (*si).dc as u16;
        }
        sl = sl.add(1);
    }
    return 0;
}

unsafe fn sixel_parse_attributes(
    si: *mut sixel_image,
    cp: *const c_char,
    end: *const c_char,
) -> *const c_char {
    unsafe {
        let mut endptr: *mut c_char = null_mut();

        let mut last = cp;
        while last != end {
            if (*last != b';' as i8 && (*last < b'0' as i8 || *last > b'9' as i8)) {
                break;
            }
            last = last.add(1);
        }
        libc::strtoul(cp, &raw mut endptr, 10);
        if endptr.cast_const() == last || *endptr != b';' as i8 {
            return last;
        }
        libc::strtoul(endptr.add(1), &raw mut endptr, 10);
        if endptr.cast_const() == last {
            return last;
        }
        if (*endptr != b';' as i8) {
            // log_debug("%s: missing ;", __func__);
            return null_mut();
        }

        let x = libc::strtoul(endptr.add(1), &raw mut endptr, 10) as u32;
        if (endptr.cast_const() == last || *endptr != b';' as i8) {
            // log_debug("%s: missing ;", __func__);
            return null_mut();
        }
        if x > SIXEL_WIDTH_LIMIT {
            // log_debug("%s: image is too wide", __func__);
            return null_mut();
        }
        let y = libc::strtoul(endptr.add(1), &raw mut endptr, 10) as u32;
        if endptr.cast_const() != last {
            // log_debug("%s: extra ;", __func__);
            return null_mut();
        }
        if y > SIXEL_HEIGHT_LIMIT {
            // log_debug("%s: image is too tall", __func__);
            return null_mut();
        }

        (*si).x = x;
        sixel_parse_expand_lines(si, y);

        last
    }
}

unsafe fn sixel_parse_colour(
    si: *mut sixel_image,
    cp: *const c_char,
    end: *const c_char,
) -> *const c_char {
    unsafe {
        let mut endptr: *mut c_char = null_mut();

        let mut last = cp;
        while (last != end) {
            if (*last != b';' as i8 && (*last < b'0' as i8 || *last > b'9' as i8)) {
                break;
            }
            last = last.add(1);
        }

        let mut c = libc::strtoul(cp, &raw mut endptr, 10) as u32;
        if c > SIXEL_COLOUR_REGISTERS {
            // log_debug("%s: too many colours", __func__);
            return null_mut();
        }
        (*si).dc = c + 1;
        if endptr.cast_const() == last || *endptr != b';' as i8 {
            return last;
        }

        let mut type_ = libc::strtoul(endptr.add(1), &raw mut endptr, 10) as u32;
        if endptr.cast_const() == last || *endptr != b';' as i8 {
            // log_debug("%s: missing ;", __func__);
            return null_mut();
        }
        let mut r = libc::strtoul(endptr.add(1), &raw mut endptr, 10) as u32;
        if (endptr.cast_const() == last || *endptr != b';' as i8) {
            // log_debug("%s: missing ;", __func__);
            return null_mut();
        }
        let mut g = libc::strtoul(endptr.add(1), &raw mut endptr, 10) as u32;
        if (endptr.cast_const() == last || *endptr != b';' as i8) {
            // log_debug("%s: missing ;", __func__);
            return null_mut();
        }
        let mut b = libc::strtoul(endptr.add(1), &raw mut endptr, 10) as u32;
        if (endptr.cast_const() != last) {
            // log_debug("%s: missing ;", __func__);
            return null_mut();
        }

        if (type_ != 1 && type_ != 2) {
            // log_debug("%s: invalid type_ %d", __func__, type_);
            return null_mut();
        }
        if (c + 1 > (*si).ncolours) {
            (*si).colours =
                xrecallocarray__((*si).colours, (*si).ncolours as usize, c as usize + 1).as_ptr();
            (*si).ncolours = c + 1;
        }
        *(*si).colours.add(c as usize) = (type_ << 24) | (r << 16) | (g << 8) | b;
        last
    }
}

unsafe fn sixel_parse_repeat(
    si: *mut sixel_image,
    cp: *const c_char,
    end: *const c_char,
) -> *const c_char {
    unsafe {
        const size_of_tmp: usize = 32;
        let mut tmp: [c_char; size_of_tmp] = [0; size_of_tmp];

        let mut n: u32 = 0;

        let mut errstr: *const c_char = null();

        let mut last = cp;
        while (last != end) {
            if (*last < b'0' as i8 || *last > b'9' as i8) {
                break;
            }
            tmp[n as usize] = *last;
            n += 1;
            last = last.add(1);
            if n == (size_of_tmp as u32) - 1 {
                // log_debug("%s: repeat not terminated", __func__);
                return null_mut();
            }
        }
        if n == 0 || last == end {
            // log_debug("%s: repeat not terminated", __func__);
            return null_mut();
        }
        tmp[n as usize] = b'\0' as i8;

        n = strtonum(
            (&raw const tmp) as *const i8,
            1,
            SIXEL_WIDTH_LIMIT as i64,
            &raw mut errstr,
        ) as u32;
        if n == 0 || !errstr.is_null() {
            // log_debug("%s: repeat too wide", __func__);
            return null_mut();
        }

        let ch = (*last) - 0x3f;
        last = last.add(1);
        for i in 0..n {
            if sixel_parse_write(si, ch as u32) != 0 {
                //log_debug("%s: width limit reached", __func__);
                return null_mut();
            }
            (*si).dx += 1;
        }
        return last;
    }
}

pub unsafe fn sixel_parse(
    buf: *const c_char,
    len: usize,
    xpixel: u32,
    ypixel: u32,
) -> *mut sixel_image {
    unsafe {
        // struct sixel_image *si;
        // const char *cp = buf, *end = buf + len;
        // char ch;

        let mut si = null_mut();
        let mut cp = buf;
        let mut end = buf.add(len);

        'bad: {
            if (len == 0 || len == 1 || *cp != b'q' as i8) {
                // log_debug("%s: empty image", __func__);
                return null_mut();
            }
            cp = cp.add(1);

            si = xcalloc1::<sixel_image>() as *mut sixel_image;
            (*si).xpixel = xpixel;
            (*si).ypixel = ypixel;

            while cp != end {
                let ch = *cp as u8;
                cp = cp.add(1);
                match ch {
                    b'"' => {
                        cp = sixel_parse_attributes(si, cp, end);
                        if cp.is_null() {
                            break 'bad;
                        }
                    }
                    b'#' => {
                        cp = sixel_parse_colour(si, cp, end);
                        if cp.is_null() {
                            break 'bad;
                        }
                    }
                    b'!' => {
                        cp = sixel_parse_repeat(si, cp, end);
                        if cp.is_null() {
                            break 'bad;
                        }
                    }
                    b'-' => {
                        (*si).dx = 0;
                        (*si).dy += 6;
                    }
                    b'$' => (*si).dx = 0,
                    _ => {
                        if !(ch < 0x20) {
                            if (ch < 0x3f || ch > 0x7e) {
                                break 'bad;
                            }
                            if sixel_parse_write(si, (ch - 0x3f) as u32) != 0 {
                                // log_debug("%s: width limit reached", __func__);
                                break 'bad;
                            }
                            (*si).dx += 1;
                        }
                    }
                }
            }

            if ((*si).x == 0 || (*si).y == 0) {
                break 'bad;
            }
            return si;
        } // 'bad:
        free_(si);
        return null_mut();
    }
}

pub unsafe fn sixel_free(si: *mut sixel_image) {
    unsafe {
        for y in 0..(*si).y {
            free_((*(*si).lines.add(y as usize)).data);
        }
        free_((*si).lines);

        free_((*si).colours);
        free_(si);
    }
}

unsafe fn sixel_log(si: *mut sixel_image) {
    unsafe {
        let mut s: [c_char; SIXEL_WIDTH_LIMIT as usize + 1] = [0; SIXEL_WIDTH_LIMIT as usize + 1];
        let mut cx: u32 = 0;
        let mut cy: u32 = 0;

        sixel_size_in_cells(si, &raw mut cx, &raw mut cy);
        // log_debug("%s: image %ux%u (%ux%u)", __func__, (*si).x, (*si).y, cx, cy);
        for i in 0..(*si).ncolours {
            // log_debug("%s: colour %u is %07x", __func__, i, (*si).colours[i]);
        }

        let mut xx: u32 = 0;
        for y in 0..(*si).y {
            let sl = (*si).lines.add(y as usize);
            for x in 0..(*si).x {
                s[x as usize] = if (x >= (*sl).x) {
                    b'_' as i8
                } else if (*(*sl).data.add(x as usize) != 0) {
                    b'0' as i8 + ((*(*sl).data.add(x as usize) - 1) % 10) as i8
                } else {
                    b'.' as i8
                };
                xx = x;
            }
            s[xx as usize] = b'\0' as c_char;
            // log_debug("%s: %4u: %s", __func__, y, s);
        }
    }
}

pub unsafe fn sixel_size_in_cells(si: *mut sixel_image, x: *mut u32, y: *mut u32) {
    unsafe {
        if (((*si).x % (*si).xpixel) == 0) {
            *x = ((*si).x / (*si).xpixel);
        } else {
            *x = 1 + ((*si).x / (*si).xpixel);
        }
        if (((*si).y % (*si).ypixel) == 0) {
            *y = ((*si).y / (*si).ypixel);
        } else {
            *y = 1 + ((*si).y / (*si).ypixel);
        }
    }
}

pub unsafe fn sixel_scale(
    si: *mut sixel_image,
    mut xpixel: u32,
    mut ypixel: u32,
    ox: u32,
    oy: u32,
    mut sx: u32,
    mut sy: u32,
    colours: i32,
) -> *mut sixel_image {
    unsafe {
        /*
         * We want to get the section of the image at ox,oy in image cells and
         * map it onto the same size in terminal cells, remembering that we
         * can only draw vertical sections of six pixels.
         */

        let mut cx: u32 = 0;
        let mut cy: u32 = 0;

        sixel_size_in_cells(si, &raw mut cx, &raw mut cy);
        if ox >= cx {
            return null_mut();
        }
        if oy >= cy {
            return null_mut();
        }
        if ox + sx >= cx {
            sx = cx - ox;
        }
        if (oy + sy >= cy) {
            sy = cy - oy;
        }

        if (xpixel == 0) {
            xpixel = (*si).xpixel;
        }
        if (ypixel == 0) {
            ypixel = (*si).ypixel;
        }

        let pox = ox * (*si).xpixel;
        let poy = oy * (*si).ypixel;
        let psx = sx * (*si).xpixel;
        let psy = sy * (*si).ypixel;

        let tsx = sx * xpixel;
        let tsy = ((sy * ypixel) / 6) * 6;

        let new = xcalloc1::<sixel_image>() as *mut sixel_image;
        (*new).xpixel = xpixel;
        (*new).ypixel = ypixel;

        for y in 0..tsy {
            let py: u32 = poy + ((y as f64) * psy as f64 / tsy as f64) as u32;
            for x in 0..tsx {
                let px: u32 = pox + (x as f64 * psx as f64 / tsx as f64) as u32;
                sixel_set_pixel(new, x, y, sixel_get_pixel(si, px, py));
            }
        }

        if colours != 0 {
            (*new).colours = xmalloc((*si).ncolours as usize * size_of::<u32>())
                .as_ptr()
                .cast();
            for i in 0..(*si).ncolours {
                *(*new).colours.add(i as usize) = *(*si).colours.add(i as usize);
            }
            (*new).ncolours = (*si).ncolours;
        }
        new
    }
}

unsafe fn sixel_print_add(
    buf: *mut *mut c_char,
    len: *mut usize,
    used: *mut usize,
    s: *const c_char,
    slen: usize,
) {
    unsafe {
        if (*used + slen >= *len + 1) {
            (*len) *= 2;
            *buf = xrealloc_(*buf, *len).as_ptr()
        }
        libc::memcpy(buf.add(*used).cast(), s.cast(), slen);
        (*used) += slen;
    }
}

unsafe fn sixel_print_repeat(
    buf: *mut *mut c_char,
    len: *mut usize,
    used: *mut usize,
    count: u32,
    ch: c_char,
) {
    unsafe {
        if (count == 1) {
            sixel_print_add(buf, len, used, &raw const ch, 1);
        } else if (count == 2) {
            sixel_print_add(buf, len, used, &raw const ch, 1);
            sixel_print_add(buf, len, used, &raw const ch, 1);
        } else if (count == 3) {
            sixel_print_add(buf, len, used, &raw const ch, 1);
            sixel_print_add(buf, len, used, &raw const ch, 1);
            sixel_print_add(buf, len, used, &raw const ch, 1);
        } else if (count != 0) {
            let mut tmp: [c_char; 16] = [0; 16];
            let tmplen = xsnprintf(
                (&raw mut tmp) as *mut i8,
                16usize,
                c"!%u%c".as_ptr(),
                count,
                ch as i32,
            ) as usize;
            sixel_print_add(buf, len, used, (&raw mut tmp) as *mut i8, tmplen);
        }
    }
}

unsafe fn sixel_print(
    si: *mut sixel_image,
    map: *mut sixel_image,
    size: *mut usize,
) -> *const c_char {
    unsafe {
        let mut buf: *mut c_char = null_mut();
        const size_of_tmp: usize = 64;
        let mut tmp: [c_char; size_of_tmp] = [0; size_of_tmp];
        let mut contains: *mut c_char = null_mut();
        let mut data: c_char = b'\0' as i8;
        let mut last = 0;

        let mut len: usize = 0;
        let mut used: usize = 0;
        let mut tmplen: usize = 0;

        let (colours, ncolours) = if !map.is_null() {
            ((*map).colours, (*map).ncolours)
        } else {
            ((*si).colours, (*si).ncolours)
        };

        if ncolours == 0 {
            return null_mut();
        }
        contains = xcalloc(1, ncolours as usize).as_ptr().cast();

        len = 8192;
        buf = xmalloc(len).as_ptr().cast();

        sixel_print_add(
            &raw mut buf,
            &raw mut len,
            &raw mut used,
            c"\x1bPq".as_ptr(),
            3,
        );

        tmplen = xsnprintf(
            (&raw mut tmp).cast(),
            size_of_tmp,
            c"\"1;1;%u;%u".as_ptr(),
            (*si).x,
            (*si).y,
        ) as usize;
        sixel_print_add(
            &raw mut buf,
            &raw mut len,
            &raw mut used,
            (&raw mut tmp).cast(),
            tmplen,
        );

        for i in 0..ncolours {
            let c = *colours.add(i as usize);
            tmplen = xsnprintf(
                (&raw mut tmp).cast(),
                size_of_tmp,
                c"#%u;%u;%u;%u;%u".as_ptr(),
                i,
                c >> 24,
                (c >> 16) & 0xff,
                (c >> 8) & 0xff,
                c & 0xff,
            ) as usize;
            sixel_print_add(
                &raw mut buf,
                &raw mut len,
                &raw mut used,
                (&raw mut tmp).cast(),
                tmplen,
            );
        }

        let mut y = 0;
        while y < (*si).y {
            libc::memset(contains.cast(), 0, ncolours as usize);
            for x in 0..(*si).x {
                for i in 0..6 {
                    if (y + i >= (*si).y) {
                        break;
                    }
                    let sl = (*si).lines.add((y + i) as usize);
                    if (x < (*sl).x && *(*sl).data.add(x as usize) != 0) {
                        *contains.add(*(*sl).data.add(x as usize) as usize - 1) = 1;
                    }
                }
            }

            for c in 0..ncolours {
                if *contains.add(c as usize) == 0 {
                    continue;
                }
                tmplen = xsnprintf((&raw mut tmp).cast(), size_of_tmp, c"#%u".as_ptr(), c) as usize;
                sixel_print_add(
                    &raw mut buf,
                    &raw mut len,
                    &raw mut used,
                    (&raw mut tmp).cast(),
                    tmplen,
                );

                let mut count = 0;
                for x in 0..(*si).x {
                    data = 0;
                    for i in 0..6 {
                        if (y + i >= (*si).y) {
                            break;
                        }
                        let sl = (*si).lines.add((y + i) as usize);
                        if (x < (*sl).x && *(*sl).data.add(x as usize) as u32 == c + 1) {
                            data |= (1 << i);
                        }
                    }
                    data += 0x3f;
                    if (data != last) {
                        sixel_print_repeat(&raw mut buf, &raw mut len, &raw mut used, count, last);
                        last = data;
                        count = 1;
                    } else {
                        count += 1;
                    }
                }
                sixel_print_repeat(&raw mut buf, &raw mut len, &raw mut used, count, data);
                sixel_print_add(&raw mut buf, &raw mut len, &raw mut used, c"$".as_ptr(), 1);
            }

            if *buf.add(used - 1) == b'$' as i8 {
                used -= 1;
            }
            if *buf.add(used - 1) != b'-' as i8 {
                sixel_print_add(&raw mut buf, &raw mut len, &raw mut used, c"-".as_ptr(), 1);
            }

            y += 6;
        }
        if *buf.add(used - 1) == b'$' as i8 || *buf.add(used - 1) == b'-' as i8 {
            used -= 1;
        }

        sixel_print_add(
            &raw mut buf,
            &raw mut len,
            &raw mut used,
            c"\x1b\\".as_ptr(),
            2,
        );

        *buf.add(used) = b'\0' as i8;
        if !size.is_null() {
            *size = used;
        }

        free_(contains);
        buf
    }
}

unsafe fn sixel_to_screen(si: *mut sixel_image) -> *mut screen {
    unsafe {
        let mut ctx: screen_write_ctx = zeroed();
        let mut gc: grid_cell = zeroed();

        let mut x: u32 = 0;
        let mut y: u32 = 0;
        let mut sx: u32 = 0;
        let mut sy: u32 = 0;

        sixel_size_in_cells(si, &raw mut sx, &raw mut sy);

        let s = xmalloc_::<screen>().as_ptr();
        screen_init(s, sx, sy, 0);

        memcpy__(&raw mut gc, &raw const grid_default_cell);
        gc.attr |= (GRID_ATTR_CHARSET | GRID_ATTR_DIM);
        utf8_set(&raw mut gc.data, b'~');

        screen_write_start(&raw mut ctx, s);
        if (sx == 1 || sy == 1) {
            for y in 0..sy {
                for x in 0..sx {
                    grid_view_set_cell((*s).grid, x, y, &gc);
                }
            }
        } else {
            screen_write_box(
                &raw mut ctx,
                sx,
                sy,
                box_lines::BOX_LINES_DEFAULT,
                null(),
                null(),
            );
            for y in 1..(sy - 1) {
                for x in 1..(sx - 1) {
                    grid_view_set_cell((*s).grid, x, y, &raw const gc);
                }
            }
        }
        screen_write_stop(&raw mut ctx);
        s
    }
}

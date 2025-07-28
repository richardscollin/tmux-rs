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

static mut all_images: images = TAILQ_HEAD_INITIALIZER!(all_images);

static mut all_images_count: u32 = 0;

unsafe fn image_free(im: NonNull<image>) {
    unsafe {
        let im = im.as_ptr();
        let mut s = (*im).s;

        tailq_remove::<_, discr_all_entry>(&raw mut all_images, im);
        all_images_count -= 1;

        tailq_remove::<_, discr_entry>(&raw mut (*s).images, im);
        crate::image_sixel::sixel_free((*im).data);
        free_((*im).fallback);
        free_(im);
    }
}

pub unsafe fn image_free_all(s: *mut screen) -> i32 {
    unsafe {
        let mut redraw = !tailq_empty(&raw mut (*s).images);

        for im in tailq_foreach::<image, discr_entry>(&raw mut (*s).images) {
            image_free(im);
        }
        redraw as i32
    }
}

/// Create text placeholder for an image.

pub unsafe fn image_fallback(ret: *mut *mut c_char, sx: u32, sy: u32) {
    unsafe {
        let mut label: *mut c_char = format_nul!("SIXEL IMAGE ({}x{})\r\n", sx, sy);

        // Allocate first line.
        let lsize: u32 = libc::strlen(label) as u32 + 1;
        let mut size: u32 = if sx < lsize - 3 { lsize - 1 } else { sx + 2 };

        // Remaining lines. Every placeholder line has \r\n at the end.
        size += (sx + 2) * (sy - 1) + 1;
        let mut buf: *mut c_char = xmalloc(size as usize).as_ptr().cast();
        *ret = buf;

        // Render first line.
        if (sx < lsize - 3) {
            libc::memcpy(buf.cast(), label.cast(), lsize as usize);
            buf = buf.add(lsize as usize - 1);
        } else {
            libc::memcpy(buf.cast(), label.cast(), lsize as usize - 3);
            buf = buf.add(lsize as usize - 3);
            libc::memset(buf.cast(), b'+' as i32, (sx - lsize + 3) as usize);
            buf = buf.add((sx - lsize + 3) as usize);
            libc::snprintf(buf, 3, c!("\r\n"));
            buf = buf.add(2);
        }

        // Remaining lines.
        for py in 1..sy {
            libc::memset(buf.cast(), b'+' as i32, sx as usize);
            buf = buf.add(sx as usize);
            libc::snprintf(buf, 3, c!("\r\n"));
            buf = buf.add(2);
        }

        free_(label);
    }
}

pub unsafe fn image_store(s: *mut screen, si: *mut sixel_image) -> *mut image {
    unsafe {
        let mut im = xcalloc1::<image>() as *mut image;
        (*im).s = s;
        (*im).data = si;

        (*im).px = (*s).cx;
        (*im).py = (*s).cy;
        crate::image_sixel::sixel_size_in_cells(si, &raw mut (*im).sx, &raw mut (*im).sy);

        image_fallback(&raw mut (*im).fallback, (*im).sx, (*im).sy);

        tailq_insert_tail::<image, discr_entry>(&raw mut (*s).images, im);
        tailq_insert_tail::<image, discr_all_entry>(&raw mut all_images, im);
        all_images_count += 1;
        if all_images_count == 10 {
            image_free(tailq_first(&raw mut all_images));
        }

        im
    }
}

pub unsafe fn image_check_line(s: *mut screen, py: u32, ny: u32) -> bool {
    unsafe {
        let mut redraw = false;

        for im in tailq_foreach::<_, discr_entry>(&raw mut (*s).images).map(NonNull::as_ptr) {
            if (py + ny > (*im).py && py < (*im).py + (*im).sy) {
                image_free(im);
                redraw = true;
            }
        }
        redraw
    }
}

pub unsafe fn image_check_area(s: *mut screen, px: u32, py: u32, nx: u32, ny: u32) -> bool {
    unsafe {
        let mut redraw = false;

        for im in tailq_foreach::<_, discr_entry>(&raw mut (*s).images).map(NonNull::as_ptr) {
            if (py + ny <= (*im).py || py >= (*im).py + (*im).sy) {
                continue;
            }
            if (px + nx <= (*im).px || px >= (*im).px + (*im).sx) {
                continue;
            }
            image_free(im);
            redraw = true;
        }
        redraw
    }
}

pub unsafe fn image_scroll_up(s: *mut screen, lines: u32) -> i32 {
    unsafe {
        let mut redraw = false;

        for im in tailq_foreach::<_, discr_entry>(&raw mut (*s).images).map(NonNull::as_ptr) {
            if ((*im).py >= lines) {
                (*im).py -= lines;
                redraw = true;
                continue;
            }
            if ((*im).py + (*im).sy <= lines) {
                image_free(im);
                redraw = true;
                continue;
            }
            let sx = (*im).sx;
            let sy = ((*im).py + (*im).sy) - lines;

            let new =
                crate::image_sixel::sixel_scale((*im).data, 0, 0, 0, (*im).sy - sy, sx, sy, 1);
            crate::image_sixel::sixel_free((*im).data);
            (*im).data = new;

            (*im).py = 0;
            crate::image_sixel::sixel_size_in_cells((*im).data, &(*im).sx, &(*im).sy);

            free_((*im).fallback);
            image_fallback(&raw mut (*im).fallback, (*im).sx, (*im).sy);
            redraw = true;
        }
        redraw.into()
    }
}

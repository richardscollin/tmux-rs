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

static mut ALL_IMAGES: images = TAILQ_HEAD_INITIALIZER!(ALL_IMAGES);

static mut ALL_IMAGES_COUNT: u32 = 0;

unsafe fn image_free(im: NonNull<image>) {
    unsafe {
        let im = im.as_ptr();
        let s = (*im).s;

        tailq_remove::<_, discr_all_entry>(&raw mut ALL_IMAGES, im);
        ALL_IMAGES_COUNT -= 1;

        tailq_remove::<_, discr_entry>(&raw mut (*s).images, im);
        crate::image_sixel::sixel_free((*im).data);
        free_((*im).fallback);
        free_(im);
    }
}

pub unsafe fn image_free_all(s: *mut screen) -> bool {
    unsafe {
        let redraw = !tailq_empty(&raw mut (*s).images);

        for im in tailq_foreach::<_, discr_entry>(&raw mut (*s).images) {
            image_free(im);
        }
        redraw
    }
}

/// Create text placeholder for an image.
pub fn image_fallback(sx: u32, sy: u32) -> CString {
    let sx = sx as usize;
    let sy = sy as usize;

    let label = CString::new(format!("SIXEL IMAGE ({sx}x{sy})\r\n")).unwrap();

    // Allocate first line.
    let lsize = label.to_bytes_with_nul().len();
    let size = if sx < lsize - 3 { lsize - 1 } else { sx + 2 };
    // Remaining lines. Every placeholder line has \r\n at the end.
    let size = size + (sx + 2) * (sy - 1) + 1;

    let mut buf: Vec<u8> = Vec::with_capacity(size);

    // Render first line.
    if sx < lsize - 3 {
        buf.extend_from_slice(label.as_bytes());
    } else {
        buf.extend_from_slice(&label.as_bytes()[..(lsize - 3)]);
        buf.extend(std::iter::repeat_n(b'+', sx - lsize + 3));
        buf.extend_from_slice("\r\n".as_bytes());
    }

    // Remaining lines.
    for _ in 1..sy {
        buf.extend(std::iter::repeat_n(b'+', sx));
        buf.extend_from_slice("\r\n".as_bytes());
    }

    CString::new(buf).unwrap()
}

pub unsafe fn image_store(s: *mut screen, si: *mut sixel_image) -> *mut image {
    unsafe {
        let mut im = Box::new(image {
            s,
            data: si,
            px: (*s).cx,
            py: (*s).cy,
            sx: 0,
            sy: 0,
            fallback: null_mut(),
            all_entry: zeroed(),
            entry: zeroed(),
        });

        (im.sx, im.sy) = crate::image_sixel::sixel_size_in_cells(&*si);

        im.fallback = image_fallback(im.sx, im.sy).into_raw().cast();

        tailq_insert_tail::<image, discr_entry>(&raw mut (*s).images, &mut *im);
        tailq_insert_tail::<image, discr_all_entry>(&raw mut ALL_IMAGES, &mut *im);
        ALL_IMAGES_COUNT += 1;
        if ALL_IMAGES_COUNT == 10 {
            image_free(NonNull::new(tailq_first::<image>(&raw mut ALL_IMAGES)).unwrap());
        }

        Box::leak(im)
    }
}

pub unsafe fn image_check_line(s: *mut screen, py: u32, ny: u32) -> bool {
    unsafe {
        let mut redraw = false;

        for im in tailq_foreach::<_, discr_entry>(&raw mut (*s).images) {
            if py + ny > (*im.as_ptr()).py && py < (*im.as_ptr()).py + (*im.as_ptr()).sy {
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

        for im in tailq_foreach::<_, discr_entry>(&raw mut (*s).images) {
            if py + ny <= (*im.as_ptr()).py || py >= (*im.as_ptr()).py + (*im.as_ptr()).sy {
                continue;
            }
            if px + nx <= (*im.as_ptr()).px || px >= (*im.as_ptr()).px + (*im.as_ptr()).sx {
                continue;
            }
            image_free(im);
            redraw = true;
        }
        redraw
    }
}

pub unsafe fn image_scroll_up(s: *mut screen, lines: u32) -> bool {
    unsafe {
        let mut redraw = false;

        for im in tailq_foreach::<_, discr_entry>(&raw mut (*s).images) {
            if (*im.as_ptr()).py >= lines {
                (*im.as_ptr()).py -= lines;
                redraw = true;
                continue;
            }
            if (*im.as_ptr()).py + (*im.as_ptr()).sy <= lines {
                image_free(im);
                redraw = true;
                continue;
            }
            let sx = (*im.as_ptr()).sx;
            let sy = ((*im.as_ptr()).py + (*im.as_ptr()).sy) - lines;

            let new = crate::image_sixel::sixel_scale(
                (*im.as_ptr()).data,
                0,
                0,
                0,
                (*im.as_ptr()).sy - sy,
                sx,
                sy,
                1,
            );
            crate::image_sixel::sixel_free((*im.as_ptr()).data);
            (*im.as_ptr()).data = new;

            (*im.as_ptr()).py = 0;
            ((*im.as_ptr()).sx, (*im.as_ptr()).sy) =
                crate::image_sixel::sixel_size_in_cells(&*(*im.as_ptr()).data);

            free_((*im.as_ptr()).fallback);
            (*im.as_ptr()).fallback = image_fallback((*im.as_ptr()).sx, (*im.as_ptr()).sy)
                .into_raw()
                .cast();
            redraw = true;
        }
        redraw
    }
}

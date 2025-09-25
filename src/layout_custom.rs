// Copyright (c) 2010 Nicholas Marriott <nicholas.marriott@gmail.com>
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
use crate::libc::sscanf;
use crate::*;

pub unsafe fn layout_find_bottomright(mut lc: *mut layout_cell) -> *mut layout_cell {
    unsafe {
        if (*lc).type_ == layout_type::LAYOUT_WINDOWPANE {
            return lc;
        }
        lc = tailq_last(&raw mut (*lc).cells);
        layout_find_bottomright(lc)
    }
}

pub unsafe fn layout_checksum(mut layout: *const u8) -> u16 {
    unsafe {
        let mut csum = 0u16;
        while *layout != b'\0' as _ {
            csum = (csum >> 1) + ((csum & 1) << 15);
            csum += *layout as u16;
            layout = layout.add(1);
        }
        csum
    }
}

/// Dump layout as a string.
pub unsafe fn layout_dump(root: *mut layout_cell) -> *mut u8 {
    unsafe {
        let mut layout: MaybeUninit<[u8; 8192]> = MaybeUninit::<[u8; 8192]>::uninit();
        let layout = layout.as_mut_ptr() as *mut u8;

        *layout = b'\0' as _;
        if layout_append(root, layout, 8192) != 0 {
            return null_mut();
        }

        format_nul!("{:04x},{}", layout_checksum(layout), _s(layout),)
    }
}

pub unsafe fn layout_append(lc: *mut layout_cell, buf: *mut u8, len: usize) -> i32 {
    unsafe {
        let sizeof_tmp = 64;
        let mut tmp = MaybeUninit::<[u8; 64]>::uninit();
        let tmp = tmp.as_mut_ptr() as *mut u8;

        let mut brackets = c!("][");

        if len == 0 {
            return -1;
        }

        let tmplen = if !(*lc).wp.is_null() {
            xsnprintf_!(
                tmp,
                sizeof_tmp,
                "{}x{},{},{},{}",
                (*lc).sx,
                (*lc).sy,
                (*lc).xoff,
                (*lc).yoff,
                (*(*lc).wp).id,
            )
            .unwrap()
        } else {
            xsnprintf_!(
                tmp,
                sizeof_tmp,
                "{}x{},{},{}",
                (*lc).sx,
                (*lc).sy,
                (*lc).xoff,
                (*lc).yoff,
            )
            .unwrap()
        };

        if tmplen > sizeof_tmp - 1 {
            return -1;
        }
        if strlcat(buf, tmp, len) >= len {
            return -1;
        }

        if ((*lc).type_) == layout_type::LAYOUT_LEFTRIGHT {
            brackets = c!("}{");
        }

        match (*lc).type_ {
            layout_type::LAYOUT_LEFTRIGHT | layout_type::LAYOUT_TOPBOTTOM => {
                if strlcat(buf, brackets.add(1), len) >= len {
                    return -1;
                }
                for lcchild in tailq_foreach(&raw mut (*lc).cells) {
                    if layout_append(lcchild.as_ptr(), buf, len) != 0 {
                        return -1;
                    }
                    if strlcat(buf, c!(","), len) >= len {
                        return -1;
                    }
                }
                *buf.add(strlen(buf) - 1) = *brackets;
            }
            layout_type::LAYOUT_WINDOWPANE => (),
        }
    }
    0
}

/// Check layout sizes fit.
pub unsafe fn layout_check(lc: *mut layout_cell) -> bool {
    unsafe {
        let mut n = 0u32;

        match (*lc).type_ {
            layout_type::LAYOUT_WINDOWPANE => (),
            layout_type::LAYOUT_LEFTRIGHT => {
                for lcchild in tailq_foreach(&raw mut (*lc).cells).map(NonNull::as_ptr) {
                    if (*lcchild).sy != (*lc).sy {
                        return false;
                    }
                    if !layout_check(lcchild) {
                        return false;
                    }
                    n += (*lcchild).sx + 1;
                }
                if n - 1 != (*lc).sx {
                    return false;
                }
            }
            layout_type::LAYOUT_TOPBOTTOM => {
                for lcchild in tailq_foreach(&raw mut (*lc).cells).map(NonNull::as_ptr) {
                    if (*lcchild).sx != (*lc).sx {
                        return false;
                    }
                    if !layout_check(lcchild) {
                        return false;
                    }
                    n += (*lcchild).sy + 1;
                }
                if n - 1 != (*lc).sy {
                    return false;
                }
            }
        }
    }
    true
}

pub unsafe fn layout_parse(w: *mut window, mut layout: *const u8, cause: *mut *mut u8) -> i32 {
    let __func__ = c!("layout_parse");
    unsafe {
        let mut lc: *mut layout_cell;
        let mut csum: u16 = 0;

        'fail: {
            // Check validity.
            if sscanf(layout.cast(), c"%hx,".as_ptr(), &raw mut csum) != 1 {
                *cause = xstrdup_(c"invalid layout").as_ptr();
                return -1;
            }
            layout = layout.add(5);
            if csum != layout_checksum(layout) {
                *cause = xstrdup_(c"invalid layout").as_ptr();
                return -1;
            }

            // Build the layout.
            lc = layout_construct(null_mut(), &raw mut layout);
            if lc.is_null() {
                *cause = xstrdup_(c"invalid layout").as_ptr();
                return -1;
            }
            if *layout != b'\0' as _ {
                *cause = xstrdup_(c"invalid layout").as_ptr();
                break 'fail;
            }

            // Check this window will fit into the layout.
            loop {
                let npanes = window_count_panes(w);
                let ncells = layout_count_cells(lc);
                if npanes > ncells {
                    *cause = format_nul!("have {} panes but need {}", npanes, ncells);
                    break 'fail;
                }
                if npanes == ncells {
                    break;
                }

                // Fewer panes than cells - close the bottom right.
                let lcchild = layout_find_bottomright(lc);
                layout_destroy_cell(w, lcchild, &raw mut lc);
            }

            // It appears older versions of tmux were able to generate layouts with
            // an incorrect top cell size - if it is larger than the top child then
            // correct that (if this is still wrong the check code will catch it).
            let mut sy = 0;
            let mut sx = 0;
            match (*lc).type_ {
                layout_type::LAYOUT_WINDOWPANE => (),
                layout_type::LAYOUT_LEFTRIGHT => {
                    for lcchild in tailq_foreach(&raw mut (*lc).cells).map(NonNull::as_ptr) {
                        sy = (*lcchild).sy + 1;
                        sx += (*lcchild).sx + 1;
                        continue;
                    }
                }
                layout_type::LAYOUT_TOPBOTTOM => {
                    for lcchild in tailq_foreach(&raw mut (*lc).cells).map(NonNull::as_ptr) {
                        sx = (*lcchild).sx + 1;
                        sy += (*lcchild).sy + 1;
                        continue;
                    }
                }
            }
            if (*lc).type_ != layout_type::LAYOUT_WINDOWPANE && ((*lc).sx != sx || (*lc).sy != sy) {
                log_debug!("fix layout {},{} to {},{}", (*lc).sx, (*lc).sy, sx, sy);
                layout_print_cell(lc, __func__, 0);
                (*lc).sx = sx - 1;
                (*lc).sy = sy - 1;
            }

            // Check the new layout.
            if !layout_check(lc) {
                *cause = xstrdup_(c"size mismatch after applying layout").as_ptr();
                break 'fail;
            }

            // Resize to the layout size.
            window_resize(w, (*lc).sx, (*lc).sy, -1, -1);

            // Destroy the old layout and swap to the new.
            layout_free_cell((*w).layout_root);
            (*w).layout_root = lc;

            // Assign the panes into the cells.
            let mut wp = tailq_first(&raw mut (*w).panes);
            layout_assign(&raw mut wp, lc);

            // Update pane offsets and sizes.
            layout_fix_offsets(w);
            layout_fix_panes(w, null_mut());
            recalculate_sizes();

            layout_print_cell(lc, __func__, 0);

            notify_window(c"window-layout-changed", w);

            return 0;
        }
        // fail:
        layout_free_cell(lc);
        -1
    }
}

// Assign panes into cells.

unsafe fn layout_assign(wp: *mut *mut window_pane, lc: *mut layout_cell) {
    unsafe {
        match (*lc).type_ {
            layout_type::LAYOUT_WINDOWPANE => {
                layout_make_leaf(lc, *wp);
                *wp = tailq_next::<_, _, discr_entry>(*wp);
            }
            layout_type::LAYOUT_LEFTRIGHT | layout_type::LAYOUT_TOPBOTTOM => {
                for lcchild in tailq_foreach(&raw mut (*lc).cells).map(NonNull::as_ptr) {
                    layout_assign(wp, lcchild);
                }
            }
        }
    }
}

/// Construct a cell from all or part of a layout tree.
unsafe fn layout_construct(lcparent: *mut layout_cell, layout: *mut *const u8) -> *mut layout_cell {
    unsafe {
        let lc;
        let mut sx = 0u32;
        let mut sy = 0u32;
        let mut xoff = 0u32;
        let mut yoff = 0u32;

        'fail: {
            if !(**layout).is_ascii_digit() {
                return null_mut();
            }
            if sscanf(
                (*layout).cast(),
                c"%ux%u,%u,%u".as_ptr(),
                &raw mut sx,
                &raw mut sy,
                &raw mut xoff,
                &raw mut yoff,
            ) != 4
            {
                return null_mut();
            }

            while isdigit(**layout as i32) != 0 {
                (*layout) = (*layout).add(1);
            }
            if **layout != b'x' as _ {
                return null_mut();
            }
            (*layout) = (*layout).add(1);
            while isdigit(**layout as i32) != 0 {
                (*layout) = (*layout).add(1);
            }
            if **layout != b',' as _ {
                return null_mut();
            }
            (*layout) = (*layout).add(1);
            while isdigit(**layout as i32) != 0 {
                (*layout) = (*layout).add(1);
            }
            if **layout != b',' as _ {
                return null_mut();
            }
            (*layout) = (*layout).add(1);
            while isdigit(**layout as i32) != 0 {
                (*layout) = (*layout).add(1);
            }
            if **layout == b',' as _ {
                let saved = *layout;
                (*layout) = (*layout).add(1);
                while isdigit(**layout as i32) != 0 {
                    (*layout) = (*layout).add(1);
                }
                if **layout == b'x' as _ {
                    *layout = saved;
                }
            }

            lc = layout_create_cell(lcparent);
            (*lc).sx = sx;
            (*lc).sy = sy;
            (*lc).xoff = xoff;
            (*lc).yoff = yoff;

            match **layout {
                b',' | b'}' | b']' | b'\0' => return lc,
                b'{' => (*lc).type_ = layout_type::LAYOUT_LEFTRIGHT,
                b'[' => (*lc).type_ = layout_type::LAYOUT_TOPBOTTOM,
                _ => break 'fail,
            }

            loop {
                (*layout) = (*layout).add(1);
                let lcchild = layout_construct(lc, layout);
                if lcchild.is_null() {
                    break 'fail;
                }
                tailq_insert_tail(&raw mut (*lc).cells, lcchild);
                if **layout != b',' as _ {
                    break;
                }
            }

            match (*lc).type_ {
                layout_type::LAYOUT_LEFTRIGHT => {
                    if **layout != b'}' as _ {
                        break 'fail;
                    }
                }
                layout_type::LAYOUT_TOPBOTTOM => {
                    if **layout != b']' as _ {
                        break 'fail;
                    }
                }
                _ => break 'fail,
            }
            (*layout) = (*layout).add(1);

            return lc;
        }
        // fail:
        layout_free_cell(lc);
        null_mut()
    }
}

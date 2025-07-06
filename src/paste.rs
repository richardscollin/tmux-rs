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

use std::cmp::Ordering;

use crate::*;

use crate::compat::{
    strlcpy,
    tree::{
        rb_find, rb_foreach_reverse, rb_initializer, rb_insert, rb_min, rb_next, rb_remove, rb_root,
    },
};
use crate::xmalloc::xreallocarray;

#[repr(C)]
pub struct paste_buffer {
    pub data: *mut u8,
    pub size: usize,

    pub name: *mut u8,
    pub created: time_t,
    pub automatic: i32,
    pub order: u32,

    pub name_entry: rb_entry<paste_buffer>,
    pub time_entry: rb_entry<paste_buffer>,
}

static mut paste_next_index: u32 = 0;
static mut paste_next_order: u32 = 0;
static mut paste_num_automatic: u32 = 0;

type paste_name_tree = rb_head<paste_buffer>;
type paste_time_tree = rb_head<paste_buffer>;

static mut paste_by_name: paste_name_tree = rb_initializer();
static mut paste_by_time: paste_time_tree = rb_initializer();

RB_GENERATE!(
    paste_name_tree,
    paste_buffer,
    name_entry,
    discr_name_entry,
    paste_cmp_names
);
fn paste_cmp_names(a: *const paste_buffer, b: *const paste_buffer) -> Ordering {
    unsafe { i32_to_ordering(libc::strcmp((*a).name, (*b).name)) }
}

RB_GENERATE!(
    paste_time_tree,
    paste_buffer,
    time_entry,
    discr_time_entry,
    paste_cmp_times
);
fn paste_cmp_times(a: *const paste_buffer, b: *const paste_buffer) -> Ordering {
    unsafe {
        let x = (*a).order;
        let y = (*b).order;

        u32::cmp(&x, &y)
    }
}

pub unsafe fn paste_buffer_name(pb: NonNull<paste_buffer>) -> *const u8 {
    unsafe { (*pb.as_ptr()).name }
}

pub unsafe fn paste_buffer_order(pb: NonNull<paste_buffer>) -> u32 {
    unsafe { (*pb.as_ptr()).order }
}

pub unsafe fn paste_buffer_created(pb: NonNull<paste_buffer>) -> time_t {
    unsafe { (*pb.as_ptr()).created }
}

pub unsafe fn paste_buffer_data(pb: *mut paste_buffer, size: *mut usize) -> *const u8 {
    unsafe {
        if !size.is_null() {
            *size = (*pb).size;
        }
        (*pb).data
    }
}
// all usages seen pass in a param and don't use null, so we can remove the check
pub unsafe fn paste_buffer_data_(pb: NonNull<paste_buffer>, size: &mut usize) -> *const u8 {
    unsafe {
        *size = (*pb.as_ptr()).size;
        (*pb.as_ptr()).data
    }
}

pub unsafe fn paste_walk(pb: *mut paste_buffer) -> *mut paste_buffer {
    unsafe {
        if pb.is_null() {
            return rb_min::<_, discr_time_entry>(&raw mut paste_by_time);
        }
        rb_next::<_, discr_time_entry>(pb)
    }
}

pub unsafe fn paste_is_empty() -> i32 {
    unsafe { rb_root(&raw mut paste_by_time).is_null() as i32 }
}

pub unsafe fn paste_get_top(name: *mut *const u8) -> *mut paste_buffer {
    unsafe {
        let mut pb = rb_min::<_, discr_time_entry>(&raw mut paste_by_time);
        while !pb.is_null() && (*pb).automatic == 0 {
            pb = rb_next::<_, discr_time_entry>(pb);
        }
        if pb.is_null() {
            return null_mut();
        }
        if !name.is_null() {
            *name = (*pb).name;
        }

        pb
    }
}

pub unsafe fn paste_get_name(name: *const u8) -> *mut paste_buffer {
    unsafe {
        let mut pbfind = MaybeUninit::<paste_buffer>::uninit();

        if name.is_null() || *name == b'\0' {
            return null_mut();
        }

        (*pbfind.as_mut_ptr()).name = name.cast_mut();
        rb_find::<_, discr_name_entry>(&raw mut paste_by_name, pbfind.as_ptr())
    }
}

pub unsafe fn paste_free(pb: NonNull<paste_buffer>) {
    unsafe {
        let pb = pb.as_ptr();
        notify_paste_buffer((*pb).name, 1);

        rb_remove::<_, discr_name_entry>(&raw mut paste_by_name, pb);
        rb_remove::<_, discr_time_entry>(&raw mut paste_by_time, pb);
        if (*pb).automatic != 0 {
            paste_num_automatic -= 1;
        }

        free_((*pb).data);
        free_((*pb).name);
        free_(pb);
    }
}

pub unsafe fn paste_add(mut prefix: *const u8, data: *mut u8, size: usize) {
    unsafe {
        if prefix.is_null() {
            prefix = c!("buffer");
        }

        if size == 0 {
            free_(data);
            return;
        }

        let limit = options_get_number_(global_options, c"buffer-limit");
        for pb in rb_foreach_reverse::<_, discr_time_entry>(&raw mut paste_by_time) {
            if (paste_num_automatic as i64) < limit {
                break;
            }
            if (*pb.as_ptr()).automatic != 0 {
                paste_free(pb);
            }
        }

        let pb = xmalloc_::<paste_buffer>().as_ptr();

        (*pb).name = null_mut();
        loop {
            free_((*pb).name);
            let tmp = paste_next_index;
            (*pb).name = format_nul!("{}{}", _s(prefix), tmp);
            paste_next_index += 1;
            if paste_get_name((*pb).name).is_null() {
                break;
            }
        }

        (*pb).data = data;
        (*pb).size = size;

        (*pb).automatic = 1;
        paste_num_automatic += 1;

        (*pb).created = libc::time(null_mut());

        (*pb).order = paste_next_order;
        paste_next_order += 1;
        rb_insert::<_, discr_name_entry>(&raw mut paste_by_name, pb);
        rb_insert::<_, discr_time_entry>(&raw mut paste_by_time, pb);

        notify_paste_buffer((*pb).name, 0);
    }
}

pub unsafe fn paste_rename(oldname: *const u8, newname: *const u8, cause: *mut *mut u8) -> i32 {
    unsafe {
        if !cause.is_null() {
            *cause = null_mut();
        }

        if oldname.is_null() || *oldname == b'\0' {
            if !cause.is_null() {
                *cause = xstrdup_(c"no buffer").as_ptr();
            }
            return -1;
        }
        if newname.is_null() || *newname == b'\0' {
            if !cause.is_null() {
                *cause = xstrdup_(c"new name is empty").as_ptr();
            }
            return -1;
        }

        let pb = paste_get_name(oldname);
        if pb.is_null() {
            if !cause.is_null() {
                *cause = format_nul!("no buffer {}", _s(oldname));
            }
            return -1;
        }

        if let Some(pb_new) = NonNull::new(paste_get_name(newname)) {
            paste_free(pb_new);
        }

        rb_remove::<_, discr_name_entry>(&raw mut paste_by_name, pb);

        free_((*pb).name);
        (*pb).name = xstrdup(newname).as_ptr();

        if (*pb).automatic != 0 {
            paste_num_automatic -= 1;
        }
        (*pb).automatic = 0;

        rb_insert::<_, discr_name_entry>(&raw mut paste_by_name, pb);

        notify_paste_buffer(oldname, 1);
        notify_paste_buffer(newname, 0);
    }
    0
}

pub unsafe fn paste_set(data: *mut u8, size: usize, name: *const u8, cause: *mut *mut u8) -> i32 {
    unsafe {
        if !cause.is_null() {
            *cause = null_mut();
        }

        if size == 0 {
            free_(data);
            return 0;
        }
        if name.is_null() {
            paste_add(null_mut(), data, size);
            return 0;
        }

        if *name == b'\0' as _ {
            if !cause.is_null() {
                *cause = xstrdup_(c"empty buffer name").as_ptr();
            }
            return -1;
        }

        let pb = xmalloc_::<paste_buffer>().as_ptr();

        (*pb).name = xstrdup(name).as_ptr();

        (*pb).data = data;
        (*pb).size = size;

        (*pb).automatic = 0;
        (*pb).order = paste_next_order;
        paste_next_order += 1;

        (*pb).created = libc::time(null_mut());

        if let Some(old) = NonNull::new(paste_get_name(name)) {
            paste_free(old);
        }

        rb_insert::<_, discr_name_entry>(&raw mut paste_by_name, pb);
        rb_insert::<_, discr_time_entry>(&raw mut paste_by_time, pb);

        notify_paste_buffer(name, 0);
    }
    0
}

pub unsafe fn paste_replace(pb: NonNull<paste_buffer>, data: *mut u8, size: usize) {
    unsafe {
        free_((*pb.as_ptr()).data);
        (*pb.as_ptr()).data = data;
        (*pb.as_ptr()).size = size;

        notify_paste_buffer((*pb.as_ptr()).name, 0);
    }
}

pub unsafe fn paste_make_sample(pb: *mut paste_buffer) -> *mut u8 {
    unsafe {
        let width = 200;

        let mut len = (*pb).size;
        if len > width {
            len = width;
        }
        let buf: *mut u8 = xreallocarray(null_mut(), len, 4 + 4).cast().as_ptr();

        let used = utf8_strvis(
            buf,
            (*pb).data,
            len,
            vis_flags::VIS_OCTAL | vis_flags::VIS_CSTYLE | vis_flags::VIS_TAB | vis_flags::VIS_NL,
        );
        if (*pb).size > width || used > width as i32 {
            strlcpy(buf.add(width), c!("..."), 4);
        }
        buf
    }
}

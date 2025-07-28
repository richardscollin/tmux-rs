// Copyright (c) 2021 Will <author@will.party>
// Copyright (c) 2022 Jeff Chiang <pobomp@gmail.com>
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

const MAX_HYPERLINKS: u32 = 5000;

static mut HYPERLINKS_NEXT_EXTERNAL_ID: c_longlong = 1;
static mut GLOBAL_HYPERLINKS_COUNT: u32 = 0;

impl_tailq_entry!(hyperlinks_uri, list_entry, tailq_entry<hyperlinks_uri>);
#[repr(C)]
pub struct hyperlinks_uri {
    pub tree: *mut hyperlinks,

    pub inner: u32,
    pub internal_id: *mut u8,
    pub external_id: *mut u8,
    pub uri: *mut u8,

    // #[entry]
    pub list_entry: tailq_entry<hyperlinks_uri>,

    pub by_inner_entry: rb_entry<hyperlinks_uri>,
    pub by_uri_entry: rb_entry<hyperlinks_uri>,
}

pub type hyperlinks_by_uri_tree = rb_head<hyperlinks_uri>;
pub type hyperlinks_by_inner_tree = rb_head<hyperlinks_uri>;

pub type hyperlinks_list = tailq_head<hyperlinks_uri>;

static mut GLOBAL_HYPERLINKS: hyperlinks_list = TAILQ_HEAD_INITIALIZER!(GLOBAL_HYPERLINKS);

#[repr(C)]
pub struct hyperlinks {
    pub next_inner: u32,
    pub by_inner: hyperlinks_by_inner_tree,
    pub by_uri: hyperlinks_by_uri_tree,
    pub references: u32,
}

fn hyperlinks_by_uri_cmp(left: &hyperlinks_uri, right: &hyperlinks_uri) -> cmp::Ordering {
    unsafe {
        if *left.internal_id == b'\0' as _ || *right.internal_id == b'\0' as _ {
            if *left.internal_id != b'\0' as _ {
                return cmp::Ordering::Less;
            }
            if *right.internal_id != b'\0' as _ {
                return cmp::Ordering::Greater;
            }
            return left.inner.cmp(&right.inner);
        }

        i32_to_ordering(libc::strcmp(left.internal_id, right.internal_id))
            .then_with(|| i32_to_ordering(crate::libc::strcmp(left.uri, right.uri)))
    }
}

RB_GENERATE!(
    hyperlinks_by_uri_tree,
    hyperlinks_uri,
    by_uri_entry,
    discr_by_uri_entry,
    hyperlinks_by_uri_cmp
);

fn hyperlinks_by_inner_cmp(left: &hyperlinks_uri, right: &hyperlinks_uri) -> cmp::Ordering {
    left.inner.cmp(&right.inner)
}

RB_GENERATE!(
    hyperlinks_by_inner_tree,
    hyperlinks_uri,
    by_inner_entry,
    discr_by_inner_entry,
    hyperlinks_by_inner_cmp
);

unsafe fn hyperlinks_remove(hlu: *mut hyperlinks_uri) {
    unsafe {
        let hl = (*hlu).tree;

        tailq_remove::<_, _>(&raw mut GLOBAL_HYPERLINKS, hlu);
        GLOBAL_HYPERLINKS_COUNT -= 1;

        rb_remove::<_, discr_by_inner_entry>(&raw mut (*hl).by_inner, hlu);
        rb_remove::<_, discr_by_uri_entry>(&raw mut (*hl).by_uri, hlu);

        free_((*hlu).internal_id);
        free_((*hlu).external_id);
        free_((*hlu).uri);
        free_(hlu);
    }
}

pub unsafe fn hyperlinks_put(
    hl: *mut hyperlinks,
    uri_in: *const u8,
    mut internal_id_in: *const u8,
) -> u32 {
    unsafe {
        let mut uri = null_mut();
        let mut internal_id = null_mut();

        /*
         * Anonymous URI are stored with an empty internal ID and the tree
         * comparator will make sure they never match each other (so each
         * anonymous URI is unique).
         */
        if internal_id_in.is_null() {
            internal_id_in = c!("");
        }

        utf8_stravis(
            &raw mut uri,
            uri_in,
            vis_flags::VIS_OCTAL | vis_flags::VIS_CSTYLE,
        );
        utf8_stravis(
            &raw mut internal_id,
            internal_id_in,
            vis_flags::VIS_OCTAL | vis_flags::VIS_CSTYLE,
        );

        if *internal_id_in != b'\0' as _ {
            let mut find = MaybeUninit::<hyperlinks_uri>::uninit();
            let find = find.as_mut_ptr();
            (*find).uri = uri;
            (*find).internal_id = internal_id;

            let hlu = rb_find::<_, discr_by_uri_entry>(&raw mut (*hl).by_uri, find);
            if !hlu.is_null() {
                free_(uri);
                free_(internal_id);
                return (*hlu).inner;
            }
        }

        let id = HYPERLINKS_NEXT_EXTERNAL_ID;
        let external_id: *mut u8 = format_nul!("tmux{:X}", id);
        HYPERLINKS_NEXT_EXTERNAL_ID += 1;

        let hlu = xcalloc1::<hyperlinks_uri>() as *mut hyperlinks_uri;
        (*hlu).inner = (*hl).next_inner;
        (*hl).next_inner += 1;
        (*hlu).internal_id = internal_id;
        (*hlu).external_id = external_id;
        (*hlu).uri = uri;
        (*hlu).tree = hl;
        rb_insert::<_, discr_by_uri_entry>(&raw mut (*hl).by_uri, hlu);
        rb_insert::<_, discr_by_inner_entry>(&raw mut (*hl).by_inner, hlu);

        tailq_insert_tail(&raw mut GLOBAL_HYPERLINKS, hlu);
        GLOBAL_HYPERLINKS_COUNT += 1;
        if GLOBAL_HYPERLINKS_COUNT == MAX_HYPERLINKS {
            hyperlinks_remove(tailq_first(&raw mut GLOBAL_HYPERLINKS));
        }

        (*hlu).inner
    }
}

pub unsafe fn hyperlinks_get(
    hl: *mut hyperlinks,
    inner: u32,
    uri_out: *mut *const u8,
    internal_id_out: *mut *const u8,
    external_id_out: *mut *const u8,
) -> bool {
    unsafe {
        let mut find = MaybeUninit::<hyperlinks_uri>::uninit();
        let find = find.as_mut_ptr();
        (*find).inner = inner;

        let hlu = rb_find::<_, discr_by_inner_entry>(&raw mut (*hl).by_inner, find);
        if hlu.is_null() {
            return false;
        }
        if !internal_id_out.is_null() {
            *internal_id_out = (*hlu).internal_id;
        }
        if !external_id_out.is_null() {
            *external_id_out = (*hlu).external_id;
        }
        *uri_out = (*hlu).uri as _;
        true
    }
}

pub unsafe fn hyperlinks_init() -> *mut hyperlinks {
    unsafe {
        let hl = xcalloc_::<hyperlinks>(1).as_ptr();
        (*hl).next_inner = 1;
        rb_init(&raw mut (*hl).by_uri);
        rb_init(&raw mut (*hl).by_inner);
        (*hl).references = 1;
        hl
    }
}

pub unsafe fn hyperlinks_copy(hl: *mut hyperlinks) -> *mut hyperlinks {
    unsafe {
        (*hl).references += 1;
    }
    hl
}

pub unsafe fn hyperlinks_reset(hl: *mut hyperlinks) {
    unsafe {
        for hlu in rb_foreach::<_, discr_by_inner_entry>(&raw mut (*hl).by_inner) {
            hyperlinks_remove(hlu.as_ptr());
        }
    }
}

pub unsafe fn hyperlinks_free(hl: *mut hyperlinks) {
    unsafe {
        (*hl).references -= 1;
        if (*hl).references == 0 {
            hyperlinks_reset(hl);
            free_(hl);
        }
    }
}

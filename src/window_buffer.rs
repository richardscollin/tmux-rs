// Copyright (c) 2017 Nicholas Marriott <nicholas.marriott@gmail.com>
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
use crate::libc::{memmem, strcmp, strstr};
use crate::*;

const WINDOW_BUFFER_DEFAULT_COMMAND: *const u8 = c!("paste-buffer -p -b '%%'");
const WINDOW_BUFFER_DEFAULT_FORMAT: *const u8 = c!("#{t/p:buffer_created}: #{buffer_sample}");

const WINDOW_BUFFER_DEFAULT_KEY_FORMAT: &CStr = cstring_concat!(
    "#{?#{e|<:#{line},10},", //
    "#{line}",
    ",",
    "#{?#{e|<:#{line},36},",
    "M-#{a:#{e|+:97,#{e|-:#{line},10}}}",
    ",",
    "",
    "}",
    "}"
);

static WINDOW_BUFFER_MENU_ITEMS: [menu_item; 11] = [
    menu_item::new(c"Paste", 'p' as u64, null_mut()),
    menu_item::new(c"Paste Tagged", 'P' as u64, null_mut()),
    menu_item::new(c"", KEYC_NONE, null_mut()),
    menu_item::new(c"Tag", 't' as u64, null_mut()),
    menu_item::new(c"Tag All", '\x14' as u64, null_mut()),
    menu_item::new(c"Tag None", 'T' as u64, null_mut()),
    menu_item::new(c"", KEYC_NONE, null_mut()),
    menu_item::new(c"Delete", 'd' as u64, null_mut()),
    menu_item::new(c"Delete Tagged", 'D' as u64, null_mut()),
    menu_item::new(c"", KEYC_NONE, null_mut()),
    menu_item::new(c"Cancel", 'q' as u64, null_mut()),
];

pub static WINDOW_BUFFER_MODE: window_mode = window_mode {
    name: SyncCharPtr::new(c"buffer-mode"),
    default_format: SyncCharPtr::from_ptr(WINDOW_BUFFER_DEFAULT_FORMAT),

    init: window_buffer_init,
    free: window_buffer_free,
    resize: window_buffer_resize,
    update: Some(window_buffer_update),
    key: Some(window_buffer_key),
    key_table: None,
    command: None,
    formats: None,
};

enum_try_from!(
    window_buffer_sort_type,
    u32,
    window_buffer_sort_type::WINDOW_BUFFER_BY_SIZE
);
#[expect(dead_code, reason = "enum_try_from transmutes from u32 to enum")]
#[repr(u32)]
enum window_buffer_sort_type {
    WINDOW_BUFFER_BY_TIME,
    WINDOW_BUFFER_BY_NAME,
    WINDOW_BUFFER_BY_SIZE,
}

const WINDOW_BUFFER_SORT_LIST_LEN: u32 = 3;
static mut WINDOW_BUFFER_SORT_LIST: [SyncCharPtr; 3] = [
    SyncCharPtr::new(c"time"),
    SyncCharPtr::new(c"name"),
    SyncCharPtr::new(c"size"),
];

pub struct window_buffer_itemdata {
    pub name: *mut u8,
    pub order: u32,
    pub size: usize,
}

pub struct window_buffer_modedata {
    pub wp: *mut window_pane,
    pub fs: cmd_find_state,

    pub data: *mut mode_tree_data,
    pub command: *mut u8,
    pub format: *mut u8,
    pub key_format: *mut u8,

    pub item_list: *mut *mut window_buffer_itemdata,
    pub item_size: u32,
}

pub struct window_buffer_editdata {
    pub wp_id: u32,
    pub name: *mut u8,
    pub pb: *mut paste_buffer,
}

pub unsafe fn window_buffer_add_item(
    data: *mut window_buffer_modedata,
) -> *mut window_buffer_itemdata {
    unsafe {
        (*data).item_list =
            xreallocarray_((*data).item_list, (*data).item_size as usize + 1).as_ptr();
        let item = xcalloc1::<window_buffer_itemdata>();
        *(*data).item_list.add((*data).item_size as usize) = item;
        (*data).item_size += 1;
        item
    }
}

pub unsafe fn window_buffer_free_item(item: *mut window_buffer_itemdata) {
    unsafe {
        free_((*item).name);
        free_(item);
    }
}

pub unsafe fn window_buffer_build(
    modedata: NonNull<c_void>,
    sort_crit: *mut mode_tree_sort_criteria,
    _tag: *mut u64,
    filter: *const u8,
) {
    unsafe {
        let data: NonNull<window_buffer_modedata> = modedata.cast();
        let mut item: *mut window_buffer_itemdata;
        let data = data.as_ptr();
        // char *text, *cp;
        // struct format_tree *ft;
        let mut s = None;
        let mut wl = None;
        let mut wp = None;

        for i in 0..(*data).item_size {
            window_buffer_free_item(*(*data).item_list.add(i as usize));
        }
        free_((*data).item_list);
        (*data).item_list = null_mut();
        (*data).item_size = 0;

        let mut pb = paste_walk(null_mut());
        while let Some(pb_non_null) = NonNull::new(pb) {
            let item = window_buffer_add_item(data);
            (*item).name = xstrdup(paste_buffer_name(pb_non_null)).as_ptr();
            paste_buffer_data(pb, &raw mut (*item).size); // I'm sure if we follow alias rules on item.size here, so keep using older function
            (*item).order = paste_buffer_order(pb_non_null);
            pb = paste_walk(pb);
        }

        {
            trait Reverseable {
                fn maybe_reverse(self, reversed: bool) -> Self;
            }
            impl Reverseable for cmp::Ordering {
                fn maybe_reverse(self, reversed: bool) -> Self {
                    if reversed { self.reverse() } else { self }
                }
            }

            let tmp = std::slice::from_raw_parts_mut((*data).item_list, (*data).item_size as usize);

            // TODO double check this ordering is correct
            match window_buffer_sort_type::try_from((*sort_crit).field) {
                Ok(window_buffer_sort_type::WINDOW_BUFFER_BY_TIME) => {
                    tmp.sort_by(|a, b| {
                        ((**b).order)
                            .cmp(&(**a).order)
                            .then_with(|| i32_to_ordering(strcmp((**a).name, (**b).name)))
                            .maybe_reverse((*sort_crit).reversed)
                    });
                }
                Ok(window_buffer_sort_type::WINDOW_BUFFER_BY_SIZE) => {
                    tmp.sort_by(|a, b| {
                        ((**b).size)
                            .cmp(&(**a).size)
                            .then_with(|| i32_to_ordering(strcmp((**a).name, (**b).name)))
                            .maybe_reverse((*sort_crit).reversed)
                    });
                }
                Ok(window_buffer_sort_type::WINDOW_BUFFER_BY_NAME) | Err(_) => {
                    tmp.sort_by(|a, b| {
                        i32_to_ordering(strcmp((**a).name, (**b).name))
                            .maybe_reverse((*sort_crit).reversed)
                    });
                }
            }
        }

        if cmd_find_valid_state(&raw mut (*data).fs) {
            s = NonNull::new((*data).fs.s);
            wl = NonNull::new((*data).fs.wl);
            wp = NonNull::new((*data).fs.wp);
        }

        for i in 0..(*data).item_size {
            item = *(*data).item_list.add(i as usize);

            pb = paste_get_name((*item).name);
            if pb.is_null() {
                continue;
            }
            let ft = format_create(null_mut(), null_mut(), FORMAT_NONE, format_flags::empty());
            format_defaults(ft, null_mut(), s, wl, wp);
            format_defaults_paste_buffer(ft, pb);

            if !filter.is_null() {
                let cp = format_expand(ft, filter);
                if format_true(cp) == 0 {
                    free_(cp);
                    format_free(ft);
                    continue;
                }
                free_(cp);
            }

            let text = format_expand(ft, (*data).format);
            mode_tree_add(
                (*data).data.cast(),
                null_mut(),
                item.cast(),
                (*item).order as u64,
                (*item).name,
                text,
                -1,
            );
            free_(text);

            format_free(ft);
        }
    }
}

pub unsafe fn window_buffer_draw(
    _modedata: *mut c_void,
    itemdata: Option<NonNull<c_void>>,
    ctx: *mut screen_write_ctx,
    sx: u32,
    sy: u32,
) {
    unsafe {
        let item: Option<NonNull<window_buffer_itemdata>> = itemdata.map(NonNull::cast);
        let cx = (*(*ctx).s).cx;
        let cy = (*(*ctx).s).cy;

        let Some(pb) = NonNull::new(paste_get_name((*item.unwrap().as_ptr()).name)) else {
            return;
        };

        let mut psize: usize = 0;
        let mut buf: *mut u8 = null_mut();
        let mut end = paste_buffer_data_(pb, &mut psize);
        let pdata = end;
        for i in 0..sy {
            let start = end;
            while end != pdata.add(psize) && *end != b'\n' {
                end = end.add(1);
            }
            buf = xreallocarray(buf.cast(), 4, end.offset_from(start) as usize + 1)
                .as_ptr()
                .cast();
            utf8_strvis(
                buf,
                start,
                end.offset_from(start) as usize,
                vis_flags::VIS_OCTAL | vis_flags::VIS_CSTYLE | vis_flags::VIS_TAB,
            );
            if *buf != b'\0' {
                screen_write_cursormove(ctx, cx as i32, (cy + i) as i32, 0);
                screen_write_nputs!(
                    ctx,
                    sx as isize,
                    &raw const GRID_DEFAULT_CELL,
                    "{}",
                    _s(buf),
                );
            }

            if end == pdata.add(psize) {
                break;
            }
            end = end.add(1);
        }
        free_(buf);
    }
}

pub unsafe fn window_buffer_search(
    _modedata: *mut c_void,
    itemdata: NonNull<c_void>,
    ss: *const u8,
) -> bool {
    unsafe {
        let item: NonNull<window_buffer_itemdata> = itemdata.cast();
        let Some(pb) = NonNull::new(paste_get_name((*item.as_ptr()).name)) else {
            return false;
        };
        if !strstr((*item.as_ptr()).name, ss).is_null() {
            return true;
        }
        let mut bufsize = 0;
        let bufdata = paste_buffer_data_(pb, &mut bufsize);
        !memmem(bufdata.cast(), bufsize, ss.cast(), strlen(ss)).is_null()
    }
}

pub unsafe fn window_buffer_menu(modedata: NonNull<c_void>, c: *mut client, key: key_code) {
    unsafe {
        let data: NonNull<window_buffer_modedata> = modedata.cast();
        let wp: *mut window_pane = (*data.as_ptr()).wp;

        if let Some(wme) = NonNull::new(tailq_first(&raw mut (*wp).modes))
            && (*wme.as_ptr()).data == modedata.as_ptr()
        {
            window_buffer_key(wme, c, null_mut(), null_mut(), key, null_mut());
        }
    }
}

pub unsafe fn window_buffer_get_key(
    modedata: NonNull<c_void>,
    itemdata: NonNull<c_void>,
    line: u32,
) -> key_code {
    unsafe {
        let data: NonNull<window_buffer_modedata> = modedata.cast();
        let item: NonNull<window_buffer_itemdata> = itemdata.cast();
        let mut s = None;
        let mut wl = None;
        let mut wp = None;

        if cmd_find_valid_state(&raw mut (*data.as_ptr()).fs) {
            s = NonNull::new((*data.as_ptr()).fs.s);
            wl = NonNull::new((*data.as_ptr()).fs.wl);
            wp = NonNull::new((*data.as_ptr()).fs.wp);
        }
        let Some(pb) = NonNull::new(paste_get_name((*item.as_ptr()).name)) else {
            return KEYC_NONE;
        };

        let ft = format_create(null_mut(), null_mut(), FORMAT_NONE, format_flags::empty());
        format_defaults(ft, null_mut(), None, None, None);
        format_defaults(ft, null_mut(), s, wl, wp);
        format_defaults_paste_buffer(ft, pb.as_ptr());
        format_add!(ft, c!("line"), "{line}");

        let expanded = format_expand(ft, (*data.as_ptr()).key_format);
        let key = key_string_lookup_string(expanded);
        free_(expanded);
        format_free(ft);
        key
    }
}

pub unsafe fn window_buffer_init(
    wme: NonNull<window_mode_entry>,
    fs: *mut cmd_find_state,
    args: *mut args,
) -> *mut screen {
    unsafe {
        let mut s = null_mut();
        let wp = (*wme.as_ptr()).wp;
        let data = xcalloc1::<window_buffer_modedata>();
        (*wme.as_ptr()).data = data as *mut window_buffer_modedata as *mut c_void;
        data.wp = wp;
        cmd_find_copy_state(&raw mut data.fs, fs);

        if args.is_null() || !args_has(args, 'F') {
            data.format = xstrdup(WINDOW_BUFFER_DEFAULT_FORMAT).as_ptr();
        } else {
            data.format = xstrdup(args_get_(args, 'F')).as_ptr();
        }
        if args.is_null() || !args_has(args, 'K') {
            data.key_format = xstrdup_(WINDOW_BUFFER_DEFAULT_KEY_FORMAT).as_ptr();
        } else {
            data.key_format = xstrdup(args_get_(args, 'K')).as_ptr();
        }
        if args.is_null() || args_count(args) == 0 {
            data.command = xstrdup(WINDOW_BUFFER_DEFAULT_COMMAND).as_ptr();
        } else {
            data.command = xstrdup(args_string(args, 0)).as_ptr();
        }

        data.data = mode_tree_start(
            wp,
            args,
            Some(window_buffer_build),
            Some(window_buffer_draw),
            Some(window_buffer_search),
            Some(window_buffer_menu),
            None,
            Some(window_buffer_get_key),
            data as *mut window_buffer_modedata as *mut c_void,
            WINDOW_BUFFER_MENU_ITEMS.as_slice(),
            &raw mut WINDOW_BUFFER_SORT_LIST as *mut *const u8,
            WINDOW_BUFFER_SORT_LIST_LEN,
            &raw mut s,
        );
        mode_tree_zoom(data.data, args);

        mode_tree_build(data.data);
        mode_tree_draw(data.data);

        s
    }
}

pub unsafe fn window_buffer_free(wme: NonNull<window_mode_entry>) {
    unsafe {
        let data = (*wme.as_ptr()).data as *mut window_buffer_modedata;

        if data.is_null() {
            return;
        }

        mode_tree_free((*data).data);

        for i in 0..(*data).item_size {
            window_buffer_free_item(*(*data).item_list.add(i as usize));
        }
        free_((*data).item_list);

        free_((*data).format);
        free_((*data).key_format);
        free_((*data).command);

        free_(data);
    }
}

pub unsafe fn window_buffer_resize(wme: NonNull<window_mode_entry>, sx: u32, sy: u32) {
    unsafe {
        let data = (*wme.as_ptr()).data as *mut window_buffer_modedata;
        mode_tree_resize((*data).data, sx, sy);
    }
}

pub unsafe fn window_buffer_update(wme: NonNull<window_mode_entry>) {
    unsafe {
        let data = (*wme.as_ptr()).data as *mut window_buffer_modedata;

        mode_tree_build((*data).data);
        mode_tree_draw((*data).data);
        (*(*data).wp).flags |= window_pane_flags::PANE_REDRAW;
    }
}

pub unsafe fn window_buffer_do_delete(
    modedata: NonNull<c_void>,
    itemdata: NonNull<c_void>,
    _c: *mut client,
    _key: key_code,
) {
    unsafe {
        let data: NonNull<window_buffer_modedata> = modedata.cast();
        let item: NonNull<window_buffer_itemdata> = itemdata.cast();

        if item == mode_tree_get_current((*data.as_ptr()).data).cast()
            && mode_tree_down((*data.as_ptr()).data, 0) == 0
        {
            // If we were unable to select the item further down we are at
            // the end of the list. Move one element up instead, to make
            // sure that we preserve a valid selection or we risk having
            // the tree build logic reset it to the first item.
            mode_tree_up((*data.as_ptr()).data, 0);
        }

        if let Some(pb) = NonNull::new(paste_get_name((*item.as_ptr()).name)) {
            paste_free(pb);
        }
    }
}

pub unsafe fn window_buffer_do_paste(
    modedata: NonNull<c_void>,
    itemdata: NonNull<c_void>,
    c: *mut client,
    _key: key_code,
) {
    unsafe {
        let data: NonNull<window_buffer_modedata> = modedata.cast();
        let item: NonNull<window_buffer_itemdata> = itemdata.cast();

        if !paste_get_name((*item.as_ptr()).name).is_null() {
            mode_tree_run_command(
                c,
                null_mut(),
                (*data.as_ptr()).command,
                (*item.as_ptr()).name,
            );
        }
    }
}

pub unsafe fn window_buffer_finish_edit(ed: *mut window_buffer_editdata) {
    unsafe {
        free_((*ed).name);
        free_(ed);
    }
}

pub unsafe fn window_buffer_edit_close_cb(buf: *mut u8, mut len: usize, arg: *mut c_void) {
    unsafe {
        let ed = arg as *mut window_buffer_editdata;

        if buf.is_null() || len == 0 {
            window_buffer_finish_edit(ed);
            return;
        }

        let pb = paste_get_name((*ed).name);
        if pb.is_null() || pb != (*ed).pb {
            window_buffer_finish_edit(ed);
            return;
        }
        let pb = NonNull::new(pb).expect("just checked");

        let mut oldlen = 0;
        let oldbuf = paste_buffer_data_(pb, &mut oldlen);
        if oldlen != 0 && *oldbuf.add(oldlen - 1) != b'\n' && *buf.add(len - 1) == b'\n' {
            len -= 1;
        }
        if len != 0 {
            paste_replace(pb, buf, len);
        }

        let wp = window_pane_find_by_id((*ed).wp_id);
        if !wp.is_null() {
            let wme = tailq_first(&raw mut (*wp).modes);
            if (*wme).mode == &raw const WINDOW_BUFFER_MODE {
                let data = (*wme).data as *mut window_buffer_modedata;
                mode_tree_build((*data).data);
                mode_tree_draw((*data).data);
            }
            (*wp).flags |= window_pane_flags::PANE_REDRAW;
        }
        window_buffer_finish_edit(ed);
    }
}

pub unsafe fn window_buffer_start_edit(
    data: *mut window_buffer_modedata,
    item: *mut window_buffer_itemdata,
    c: *mut client,
) {
    unsafe {
        // struct paste_buffer *pb;
        // const char *buf;
        // size_t len;
        // struct window_buffer_editdata *ed;

        let Some(pb) = NonNull::new(paste_get_name((*item).name)) else {
            return;
        };
        let mut len = 0;
        let buf = paste_buffer_data_(pb, &mut len);

        let ed = xcalloc1::<window_buffer_editdata>();
        ed.wp_id = (*(*data).wp).id;
        ed.name = xstrdup(paste_buffer_name(pb)).as_ptr();
        ed.pb = pb.as_ptr();
        let ed = ed as *mut window_buffer_editdata;

        if popup_editor(c, buf, len, Some(window_buffer_edit_close_cb), ed.cast()) != 0 {
            window_buffer_finish_edit(ed);
        }
    }
}

pub unsafe fn window_buffer_key(
    wme: NonNull<window_mode_entry>,
    c: *mut client,
    _s: *mut session,
    _wl: *mut winlink,
    mut key: key_code,
    m: *mut mouse_event,
) {
    unsafe {
        let wp = (*wme.as_ptr()).wp;
        let data = (*wme.as_ptr()).data as *mut window_buffer_modedata;
        let mtd: *mut mode_tree_data = (*data).data;
        let mut finished;

        'out: {
            if paste_is_empty() != 0 {
                finished = true;
                break 'out;
            }

            finished = mode_tree_key(mtd, c, &raw mut key, m, null_mut(), null_mut()) != 0;
            match key as u8 {
                b'e' => {
                    let item: NonNull<window_buffer_itemdata> = mode_tree_get_current(mtd).cast();
                    window_buffer_start_edit(data, item.as_ptr(), c);
                }
                b'd' => {
                    let item = mode_tree_get_current(mtd);
                    window_buffer_do_delete(NonNull::new(data.cast()).unwrap(), item, c, key);
                    mode_tree_build(mtd);
                }
                b'D' => {
                    mode_tree_each_tagged(mtd, Some(window_buffer_do_delete), c, key, 0);
                    mode_tree_build(mtd);
                }
                b'P' => {
                    mode_tree_each_tagged(mtd, Some(window_buffer_do_paste), c, key, 0);
                    finished = true;
                }
                b'p' | b'\r' => {
                    let item = mode_tree_get_current(mtd);
                    window_buffer_do_paste(NonNull::new(data.cast()).unwrap(), item, c, key);
                    finished = true;
                }
                _ => (),
            }
        }
        // out:
        if finished || paste_is_empty() != 0 {
            window_pane_reset_mode(wp);
        } else {
            mode_tree_draw(mtd);
            (*wp).flags |= window_pane_flags::PANE_REDRAW;
        }
    }
}

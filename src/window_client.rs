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

use crate::*;

use crate::libc::{qsort, strcmp};

use crate::compat::queue::tailq_first;

static WINDOW_CLIENT_DEFAULT_COMMAND: &CStr = c"detach-client -t '%%'";
static WINDOW_CLIENT_DEFAULT_FORMAT: &CStr = c"#{t/p:client_activity}: session #{session_name}";
static WINDOW_CLIENT_DEFAULT_KEY_FORMAT: &CStr =
    c"#{?#{e|<:#{line},10},#{line},#{?#{e|<:#{line},36},M-#{a:#{e|+:97,#{e|-:#{line},10}}},}}";

static WINDOW_CLIENT_MENU_ITEMS: [menu_item; 8] = [
    menu_item::new(c"Detach", b'd' as _, null()),
    menu_item::new(c"Detach Tagged", b'D' as _, null()),
    menu_item::new(c"", KEYC_NONE, null()),
    menu_item::new(c"Tag", b't' as _, null()),
    menu_item::new(c"Tag All", b'\x14' as _, null()),
    menu_item::new(c"Tag None", b'T' as _, null()),
    menu_item::new(c"", KEYC_NONE, null()),
    menu_item::new(c"Cancel", b'q' as _, null()),
];

pub static WINDOW_CLIENT_MODE: window_mode = window_mode {
    name: SyncCharPtr::new(c"client-mode"),
    default_format: SyncCharPtr::new(WINDOW_CLIENT_DEFAULT_FORMAT),

    init: window_client_init,
    free: window_client_free,
    resize: window_client_resize,
    update: Some(window_client_update),
    key: Some(window_client_key),
    key_table: None,
    command: None,
    formats: None,
};

enum_try_from!(
    window_client_sort_type,
    u32,
    window_client_sort_type::WINDOW_CLIENT_BY_ACTIVITY_TIME
);
#[repr(u32)]
pub enum window_client_sort_type {
    WINDOW_CLIENT_BY_NAME,
    WINDOW_CLIENT_BY_SIZE,
    WINDOW_CLIENT_BY_CREATION_TIME,
    WINDOW_CLIENT_BY_ACTIVITY_TIME,
}
const WINDOW_CLIENT_SORT_LIST_LEN: u32 = 4;
static mut WINDOW_CLIENT_SORT_LIST: [*const u8; 4] =
    [c!("name"), c!("size"), c!("creation"), c!("activity")];

static mut WINDOW_CLIENT_SORT: *mut mode_tree_sort_criteria = null_mut();

#[repr(C)]
pub struct window_client_itemdata {
    c: *mut client,
}

#[repr(C)]
pub struct window_client_modedata {
    wp: *mut window_pane,

    data: *mut mode_tree_data,
    format: *mut u8,
    key_format: *mut u8,
    command: *mut u8,

    item_list: *mut *mut window_client_itemdata,
    item_size: u32,
}

pub unsafe fn window_client_add_item(
    data: *mut window_client_modedata,
) -> *mut window_client_itemdata {
    unsafe {
        (*data).item_list =
            xreallocarray_((*data).item_list, (*data).item_size as usize + 1).as_ptr();
        let item = xcalloc1::<window_client_itemdata>() as *mut window_client_itemdata;
        *(*data).item_list.add((*data).item_size as usize) = item;
        (*data).item_size += 1;

        item
    }
}

pub unsafe fn window_client_free_item(item: *mut window_client_itemdata) {
    unsafe {
        server_client_unref((*item).c);
        free_(item);
    }
}

pub unsafe extern "C" fn window_client_cmp(a0: *const c_void, b0: *const c_void) -> i32 {
    unsafe {
        let a: *const *const window_client_itemdata = a0 as _;
        let b: *const *const window_client_itemdata = b0 as _;
        let itema: *const window_client_itemdata = *a;
        let itemb: *const window_client_itemdata = *b;
        let ca = (*itema).c;
        let cb = (*itemb).c;
        let mut result: i32 = 0;

        match window_client_sort_type::try_from((*WINDOW_CLIENT_SORT).field) {
            Ok(window_client_sort_type::WINDOW_CLIENT_BY_SIZE) => {
                result = (*ca).tty.sx.wrapping_sub((*cb).tty.sx) as i32;
                if result == 0 {
                    result = (*ca).tty.sy.wrapping_sub((*cb).tty.sy) as i32;
                }
            }
            Ok(window_client_sort_type::WINDOW_CLIENT_BY_CREATION_TIME) => {
                if timer::new(&raw const (*ca).creation_time)
                    > timer::new(&raw const (*cb).creation_time)
                {
                    result = -1;
                } else if timer::new(&raw mut (*ca).creation_time)
                    < timer::new(&raw mut (*cb).creation_time)
                {
                    result = 1;
                }
            }
            Ok(window_client_sort_type::WINDOW_CLIENT_BY_ACTIVITY_TIME) => {
                if timer::new(&raw mut (*ca).activity_time)
                    > timer::new(&raw mut (*cb).activity_time)
                {
                    result = -1;
                } else if timer::new(&raw mut (*ca).activity_time)
                    < timer::new(&raw mut (*cb).activity_time)
                {
                    result = 1;
                }
            }
            _ => (),
        }

        /* Use WINDOW_CLIENT_BY_NAME as default order and tie breaker. */
        if result == 0 {
            result = strcmp((*ca).name, (*cb).name);
        }

        if (*WINDOW_CLIENT_SORT).reversed != 0 {
            result = -result;
        }

        result
    }
}

pub unsafe fn window_client_build(
    modedata: NonNull<c_void>,
    sort_crit: *mut mode_tree_sort_criteria,
    _tag: *mut u64,
    filter: *const u8,
) {
    unsafe {
        let data: NonNull<window_client_modedata> = modedata.cast();
        let data = data.as_ptr();

        for i in 0..(*data).item_size {
            window_client_free_item(*(*data).item_list.add(i as usize));
        }
        free_((*data).item_list);
        (*data).item_list = null_mut();
        (*data).item_size = 0;

        for c in crate::compat::queue::tailq_foreach(&raw mut CLIENTS).map(NonNull::as_ptr) {
            if (*c).session.is_null() || (*c).flags.intersects(CLIENT_UNATTACHEDFLAGS) {
                continue;
            }

            let item = window_client_add_item(data);
            (*item).c = c;

            (*c).references += 1;
        }

        WINDOW_CLIENT_SORT = sort_crit;
        qsort(
            (*data).item_list.cast(),
            (*data).item_size as usize,
            size_of::<window_client_itemdata>(),
            Some(window_client_cmp),
        );

        for i in 0..(*data).item_size {
            let item = *(*data).item_list.add(i as usize);
            let c = (*item).c;

            if !filter.is_null() {
                let cp = format_single(null_mut(), filter, c, null_mut(), null_mut(), null_mut());
                if format_true(cp) == 0 {
                    free_(cp);
                    continue;
                }
                free_(cp);
            }

            let text = format_single(
                null_mut(),
                (*data).format,
                c,
                null_mut(),
                null_mut(),
                null_mut(),
            );
            mode_tree_add(
                (*data).data,
                null_mut(),
                item.cast(),
                c as u64,
                (*c).name,
                text,
                -1,
            );
            free_(text);
        }
    }
}

pub unsafe fn window_client_draw(
    _modedata: *mut c_void,
    itemdata: Option<NonNull<c_void>>,
    ctx: *mut screen_write_ctx,
    sx: u32,
    sy: u32,
) {
    unsafe {
        let item: Option<NonNull<window_client_itemdata>> = itemdata.map(NonNull::cast);
        let c = (*item.unwrap().as_ptr()).c;
        let s = (*ctx).s;

        let cx = (*s).cx;
        let cy = (*s).cy;

        if (*c).session.is_null() || (*c).flags.intersects(CLIENT_UNATTACHEDFLAGS) {
            return;
        }
        let wp = (*(*(*(*c).session).curw).window).active;

        let mut lines = status_line_size(c);
        if lines >= sy {
            lines = 0;
        }
        let at = if status_at_line(c) == 0 { lines } else { 0 };

        screen_write_cursormove(ctx, cx as i32, (cy + at) as i32, 0);
        screen_write_preview(ctx, &raw mut (*wp).base, sx, sy - 2 - lines);

        if at != 0 {
            screen_write_cursormove(ctx, cx as i32, (cy + 2) as i32, 0);
        } else {
            screen_write_cursormove(ctx, cx as i32, (cy + sy - 1 - lines) as i32, 0);
        }
        screen_write_hline(ctx, sx, 0, 0, box_lines::BOX_LINES_DEFAULT, null());

        if at != 0 {
            screen_write_cursormove(ctx, cx as i32, cy as i32, 0);
        } else {
            screen_write_cursormove(ctx, cx as i32, (cy + sy - lines) as i32, 0);
        }
        screen_write_fast_copy(ctx, &raw mut (*c).status.screen, 0, 0, sx, lines);
    }
}

pub unsafe fn window_client_menu(modedata: NonNull<c_void>, c: *mut client, key: key_code) {
    unsafe {
        let data: NonNull<window_client_modedata> = modedata.cast();
        let wp: *mut window_pane = (*data.as_ptr()).wp;

        if let Some(wme) = NonNull::new(tailq_first(&raw mut (*wp).modes)) {
            if (*wme.as_ptr()).data == modedata.as_ptr() {
                window_client_key(wme, c, null_mut(), null_mut(), key, null_mut());
            }
        }
    }
}

pub unsafe fn window_client_get_key(
    modedata: NonNull<c_void>,
    itemdata: NonNull<c_void>,
    line: u32,
) -> key_code {
    unsafe {
        let data: NonNull<window_client_modedata> = modedata.cast();
        let item: NonNull<window_client_itemdata> = itemdata.cast();

        let ft = format_create(null_mut(), null_mut(), FORMAT_NONE, format_flags::empty());
        format_defaults(ft, (*item.as_ptr()).c, None, None, None);
        format_add!(ft, c!("line"), "{line}");

        let expanded = format_expand(ft, (*data.as_ptr()).key_format);
        let key = key_string_lookup_string(expanded);
        free_(expanded);
        format_free(ft);
        key
    }
}

pub unsafe fn window_client_init(
    wme: NonNull<window_mode_entry>,
    _fs: *mut cmd_find_state,
    args: *mut args,
) -> *mut screen {
    unsafe {
        let wp: *mut window_pane = (*wme.as_ptr()).wp;
        let mut s: *mut screen = null_mut();

        let data: *mut window_client_modedata =
            xcalloc1::<window_client_modedata>() as *mut window_client_modedata;
        (*wme.as_ptr()).data = data.cast();
        (*data).wp = wp;

        if args.is_null() || !args_has_(args, 'F') {
            (*data).format = xstrdup_(WINDOW_CLIENT_DEFAULT_FORMAT).as_ptr();
        } else {
            (*data).format = xstrdup(args_get_(args, 'F')).as_ptr();
        }
        if args.is_null() || !args_has_(args, 'K') {
            (*data).key_format = xstrdup_(WINDOW_CLIENT_DEFAULT_KEY_FORMAT).as_ptr();
        } else {
            (*data).key_format = xstrdup(args_get_(args, 'K')).as_ptr();
        }
        if args.is_null() || args_count(args) == 0 {
            (*data).command = xstrdup_(WINDOW_CLIENT_DEFAULT_COMMAND).as_ptr();
        } else {
            (*data).command = xstrdup(args_string(args, 0)).as_ptr();
        }

        (*data).data = mode_tree_start(
            wp,
            args,
            Some(window_client_build),
            Some(window_client_draw),
            None,
            Some(window_client_menu),
            None,
            Some(window_client_get_key),
            data.cast(),
            WINDOW_CLIENT_MENU_ITEMS.as_slice(),
            &raw mut WINDOW_CLIENT_SORT_LIST as *mut *const u8,
            WINDOW_CLIENT_SORT_LIST_LEN,
            &raw mut s,
        );
        mode_tree_zoom((*data).data, args);

        mode_tree_build((*data).data);
        mode_tree_draw((*data).data);

        s
    }
}

pub unsafe fn window_client_free(wme: NonNull<window_mode_entry>) {
    unsafe {
        let data: *mut window_client_modedata = (*wme.as_ptr()).data as *mut window_client_modedata;

        if data.is_null() {
            return;
        }

        mode_tree_free((*data).data);

        for i in 0..(*data).item_size {
            window_client_free_item(*(*data).item_list.add(i as usize));
        }
        free_((*data).item_list);

        free_((*data).format);
        free_((*data).key_format);
        free_((*data).command);

        free_(data);
    }
}

pub unsafe fn window_client_resize(wme: NonNull<window_mode_entry>, sx: u32, sy: u32) {
    unsafe {
        let data = (*wme.as_ptr()).data as *mut window_client_modedata;

        mode_tree_resize((*data).data, sx, sy);
    }
}

pub unsafe fn window_client_update(wme: NonNull<window_mode_entry>) {
    unsafe {
        let data = (*wme.as_ptr()).data as *mut window_client_modedata;

        mode_tree_build((*data).data);
        mode_tree_draw((*data).data);
        (*(*data).wp).flags |= window_pane_flags::PANE_REDRAW;
    }
}

pub unsafe fn window_client_do_detach(
    modedata: NonNull<c_void>,
    itemdata: NonNull<c_void>,
    _c: *mut client,
    key: key_code,
) {
    let data: NonNull<window_client_modedata> = modedata.cast();
    let item: NonNull<window_client_itemdata> = itemdata.cast();

    // TODO I'm not conviced this NonNull (item) is correct here

    unsafe {
        if item == mode_tree_get_current((*data.as_ptr()).data).cast() {
            mode_tree_down((*data.as_ptr()).data, 0);
        }
        if key == 'd' as _ || key == 'D' as _ {
            server_client_detach((*item.as_ptr()).c, msgtype::MSG_DETACH);
        } else if key == 'x' as _ || key == 'X' as _ {
            server_client_detach((*item.as_ptr()).c, msgtype::MSG_DETACHKILL);
        } else if key == 'z' as _ || key == 'Z' as _ {
            server_client_suspend((*item.as_ptr()).c);
        }
    }
}

pub unsafe fn window_client_key(
    wme: NonNull<window_mode_entry>,
    c: *mut client,
    _: *mut session,
    _wl: *mut winlink,
    mut key: key_code,
    m: *mut mouse_event,
) {
    unsafe {
        let wp = (*wme.as_ptr()).wp;
        let data = (*wme.as_ptr()).data as *mut window_client_modedata;
        let mtd: *mut mode_tree_data = (*data).data;

        let mut finished = mode_tree_key(mtd, c, &raw mut key, m, null_mut(), null_mut()) != 0;
        match key as u8 {
            b'd' | b'x' | b'z' => {
                let item: NonNull<window_client_itemdata> = mode_tree_get_current(mtd).cast();
                window_client_do_detach(NonNull::new(data.cast()).unwrap(), item.cast(), c, key);
                mode_tree_build(mtd);
            }
            b'D' | b'X' | b'Z' => {
                mode_tree_each_tagged(mtd, Some(window_client_do_detach), c, key, 0);
                mode_tree_build(mtd);
            }
            b'\r' => {
                let item: NonNull<window_client_itemdata> = mode_tree_get_current(mtd).cast();
                mode_tree_run_command(
                    c,
                    null_mut(),
                    (*data).command,
                    (*(*item.as_ptr()).c).ttyname,
                );
                finished = true;
            }
            _ => (),
        }

        if finished || server_client_how_many() == 0 {
            window_pane_reset_mode(wp);
        } else {
            mode_tree_draw(mtd);
            (*wp).flags |= window_pane_flags::PANE_REDRAW;
        }
    }
}

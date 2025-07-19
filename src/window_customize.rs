// Copyright (c) 2020 Nicholas Marriott <nicholas.marriott@gmail.com>
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

use super::*;

use crate::compat::{strlcat, tree::rb_empty};

static WINDOW_CUSTOMIZE_DEFAULT_FORMAT: &CStr = cstring_concat!(
    "#{?is_option,",
    "#{?option_is_global,,#[reverse](#{option_scope})#[default] }",
    "#[ignore]",
    "#{option_value}#{?option_unit, #{option_unit},}",
    ",",
    "#{key}",
    "}"
);

static WINDOW_CUSTOMIZE_MENU_ITEMS: [menu_item; 8] = [
    menu_item::new(c"Select", '\r' as key_code, null_mut()),
    menu_item::new(c"Expand", keyc::KEYC_RIGHT as key_code, null_mut()),
    menu_item::new(c"", KEYC_NONE, null_mut()),
    menu_item::new(c"Tag", 't' as key_code, null_mut()),
    menu_item::new(c"Tag All", '\x14' as key_code, null_mut()),
    menu_item::new(c"Tag None", 'T' as key_code, null_mut()),
    menu_item::new(c"", KEYC_NONE, null_mut()),
    menu_item::new(c"Cancel", 'q' as key_code, null_mut()),
];

pub static WINDOW_CUSTOMIZE_MODE: window_mode = window_mode {
    name: SyncCharPtr::new(c"options-mode"),
    default_format: SyncCharPtr::new(WINDOW_CUSTOMIZE_DEFAULT_FORMAT),

    init: window_customize_init,
    free: window_customize_free,
    resize: window_customize_resize,
    key: Some(window_customize_key),
    update: None,
    key_table: None,
    command: None,
    formats: None,
};

#[repr(i32)]
#[derive(Copy, Clone, Eq, PartialEq)]
enum window_customize_scope {
    WINDOW_CUSTOMIZE_NONE,
    WINDOW_CUSTOMIZE_KEY,
    WINDOW_CUSTOMIZE_SERVER,
    WINDOW_CUSTOMIZE_GLOBAL_SESSION,
    WINDOW_CUSTOMIZE_SESSION,
    WINDOW_CUSTOMIZE_GLOBAL_WINDOW,
    WINDOW_CUSTOMIZE_WINDOW,
    WINDOW_CUSTOMIZE_PANE,
}

#[repr(i32)]
#[derive(Copy, Clone, Eq, PartialEq)]
enum window_customize_change {
    WINDOW_CUSTOMIZE_UNSET,
    WINDOW_CUSTOMIZE_RESET,
}

#[repr(C)]
pub struct window_customize_itemdata {
    data: *mut window_customize_modedata,
    scope: window_customize_scope,

    table: *mut u8,
    key: key_code,

    oo: *mut options,
    name: *mut u8,
    idx: i32,
}

#[repr(C)]
pub struct window_customize_modedata {
    wp: *mut window_pane,
    dead: i32,
    references: i32,

    data: *mut mode_tree_data,
    format: *mut u8,
    hide_global: i32,

    item_list: *mut *mut window_customize_itemdata,
    item_size: u32,

    fs: cmd_find_state,
    change: window_customize_change,
}

unsafe fn window_customize_get_tag(
    o: *mut options_entry,
    idx: i32,
    oe: *const options_table_entry,
) -> u64 {
    unsafe {
        if let Some(oe) = NonNull::new(oe.cast_mut()) {
            let offset = oe.offset_from_unsigned(
                NonNull::new((&raw const OPTIONS_TABLE) as *mut options_table_entry).unwrap(),
            ) as u64;
            (2u64 << 62) | (offset << 32) | ((idx as u64 + 1) << 1) | 1
        } else {
            o.addr() as u64
        }
    }
}

unsafe fn window_customize_get_tree(
    scope: window_customize_scope,
    fs: *mut cmd_find_state,
) -> *mut options {
    unsafe {
        match scope {
            window_customize_scope::WINDOW_CUSTOMIZE_NONE
            | window_customize_scope::WINDOW_CUSTOMIZE_KEY => null_mut(),
            window_customize_scope::WINDOW_CUSTOMIZE_SERVER => GLOBAL_OPTIONS,
            window_customize_scope::WINDOW_CUSTOMIZE_GLOBAL_SESSION => GLOBAL_S_OPTIONS,
            window_customize_scope::WINDOW_CUSTOMIZE_SESSION => (*(*fs).s).options,
            window_customize_scope::WINDOW_CUSTOMIZE_GLOBAL_WINDOW => GLOBAL_W_OPTIONS,
            window_customize_scope::WINDOW_CUSTOMIZE_WINDOW => (*(*fs).w).options,
            window_customize_scope::WINDOW_CUSTOMIZE_PANE => (*(*fs).wp).options,
        }
    }
}

unsafe fn window_customize_check_item(
    data: *mut window_customize_modedata,
    item: *mut window_customize_itemdata,
    mut fsp: *mut cmd_find_state,
) -> bool {
    unsafe {
        let mut fs: cmd_find_state = zeroed();

        if fsp.is_null() {
            fsp = &raw mut fs;
        }

        if cmd_find_valid_state(&raw mut (*data).fs) {
            cmd_find_copy_state(fsp, &raw mut (*data).fs);
        } else {
            cmd_find_from_pane(fsp, (*data).wp, 0);
        }

        (*item).oo == window_customize_get_tree((*item).scope, fsp)
    }
}

unsafe fn window_customize_get_key(
    item: *mut window_customize_itemdata,
    ktp: *mut *mut key_table,
    bdp: *mut *mut key_binding,
) -> i32 {
    unsafe {
        let Some(kt) = NonNull::new(key_bindings_get_table((*item).table, 0)) else {
            return 0;
        };

        let Some(bd) = NonNull::new(key_bindings_get(kt, (*item).key)) else {
            return 0;
        };

        if !ktp.is_null() {
            *ktp = kt.as_ptr();
        }
        if !bdp.is_null() {
            *bdp = bd.as_ptr();
        }
        1
    }
}

unsafe fn window_customize_scope_text(
    scope: window_customize_scope,
    fs: *mut cmd_find_state,
) -> *mut u8 {
    unsafe {
        let mut s: *mut u8 = null_mut();
        let mut idx: u32 = 0;

        match scope {
            window_customize_scope::WINDOW_CUSTOMIZE_PANE => {
                window_pane_index((*fs).wp, &raw mut idx);
                format_nul!("pane {}", idx)
            }
            window_customize_scope::WINDOW_CUSTOMIZE_SESSION => {
                format_nul!("session {}", _s((*(*fs).s).name))
            }
            window_customize_scope::WINDOW_CUSTOMIZE_WINDOW => {
                format_nul!("window {}", (*(*fs).wl).idx)
            }
            _ => xstrdup_(c"").as_ptr(),
        }
    }
}

unsafe fn window_customize_add_item(
    data: *mut window_customize_modedata,
) -> *mut window_customize_itemdata {
    unsafe {
        let mut item: *mut window_customize_itemdata = null_mut();

        (*data).item_list =
            xreallocarray_((*data).item_list, (*data).item_size as usize + 1).as_ptr();
        item = xcalloc1() as *mut window_customize_itemdata;
        *(*data).item_list.add((*data).item_size as usize) = item;
        (*data).item_size += 1;

        item
    }
}

unsafe fn window_customize_free_item(item: *mut window_customize_itemdata) {
    unsafe {
        free_((*item).table);
        free_((*item).name);
        free_(item);
    }
}

unsafe fn window_customize_build_array(
    data: *mut window_customize_modedata,
    top: *mut mode_tree_item,
    scope: window_customize_scope,
    o: *mut options_entry,
    ft: *mut format_tree,
) {
    unsafe {
        let oe = options_table_entry(o);
        let oo = options_owner(o);

        let mut ai = options_array_first(o);
        while !ai.is_null() {
            let idx = options_array_item_index(ai);
            let mut name: *mut u8 = null_mut();

            name = format_nul!("{}[{}]", _s(options_name(o)), idx);
            format_add!(ft, c!("option_name"), "{}", _s(name));
            let value: *mut u8 = options_to_string(o, idx as i32, 0);
            format_add!(ft, c!("option_value"), "{}", _s(value));

            let item = window_customize_add_item(data);
            (*item).scope = scope;
            (*item).oo = oo;
            (*item).name = xstrdup(options_name(o)).as_ptr();
            (*item).idx = idx as i32;

            let text: *mut u8 = format_expand(ft, (*data).format);
            let tag = window_customize_get_tag(o, idx as i32, oe);
            mode_tree_add((*data).data, top, item.cast(), tag, name, text, -1);
            free_(text);

            free_(name);
            free_(value);

            ai = options_array_next(ai);
        }
    }
}

unsafe fn window_customize_build_option(
    data: *mut window_customize_modedata,
    top: *mut mode_tree_item,
    scope: window_customize_scope,
    o: *mut options_entry,
    ft: *mut format_tree,
    filter: *const u8,
    fs: *mut cmd_find_state,
) {
    unsafe {
        let oe = options_table_entry(o);
        let oo = options_owner(o);
        let name: *const u8 = options_name(o);

        let mut global: i32 = 0;
        let mut array: i32 = 0;

        if !oe.is_null() && (*oe).flags & OPTIONS_TABLE_IS_HOOK != 0 {
            return;
        }
        if !oe.is_null() && (*oe).flags & OPTIONS_TABLE_IS_ARRAY != 0 {
            array = 1;
        }

        if scope == window_customize_scope::WINDOW_CUSTOMIZE_SERVER
            || scope == window_customize_scope::WINDOW_CUSTOMIZE_GLOBAL_SESSION
            || scope == window_customize_scope::WINDOW_CUSTOMIZE_GLOBAL_WINDOW
        {
            global = 1;
        }
        if (*data).hide_global != 0 && global != 0 {
            return;
        }

        format_add!(ft, c!("option_name"), "{}", _s(name));
        format_add!(ft, c!("option_is_global"), "{global}");
        format_add!(ft, c!("option_is_array"), "{array}");

        let mut text = window_customize_scope_text(scope, fs);
        format_add!(ft, c!("option_scope"), "{}", _s(text));
        free_(text);

        if !oe.is_null() && !(*oe).unit.is_null() {
            format_add!(ft, c!("option_unit"), "{}", _s((*oe).unit));
        } else {
            format_add!(ft, c!("option_unit"), "{}", "");
        }

        if array == 0 {
            let value = options_to_string(o, -1, 0);
            format_add!(ft, c!("option_value"), "{}", _s(value));
            free_(value);
        }

        if !filter.is_null() {
            let expanded = format_expand(ft, filter);
            if format_true(expanded) == 0 {
                free_(expanded);
                return;
            }
            free_(expanded);
        }
        let item = window_customize_add_item(data);
        (*item).oo = oo;
        (*item).scope = scope;
        (*item).name = xstrdup(name).as_ptr();
        (*item).idx = -1;

        if array != 0 {
            text = null_mut();
        } else {
            text = format_expand(ft, (*data).format);
        }
        let tag = window_customize_get_tag(o, -1, oe);
        let top = mode_tree_add((*data).data, top, item.cast(), tag, name, text, 0);
        free_(text);

        if array != 0 {
            window_customize_build_array(data, top, scope, o, ft);
        }
    }
}

unsafe fn window_customize_find_user_options(
    oo: *mut options,
    list: *mut *mut *const u8,
    size: *mut u32,
) {
    unsafe {
        let mut o = options_first(oo);
        while !o.is_null() {
            let name = options_name(o);
            if *name != b'@' {
                o = options_next(o);
                continue;
            }
            let mut i = 0;
            for j in 0..(*size) {
                i = j;
                if libc::strcmp(*(*list).add(i as usize), name) == 0 {
                    break;
                }
            }
            if i != *size {
                o = options_next(o);
                continue;
            }
            *list = xreallocarray_(*list, (*size) as usize + 1).as_ptr();
            *(*list).add(*size as usize) = name;
            (*size) += 1;

            o = options_next(o);
        }
    }
}

unsafe fn window_customize_build_options(
    data: *mut window_customize_modedata,
    title: *const u8,
    tag: u64,
    scope0: window_customize_scope,
    oo0: *mut options,
    scope1: window_customize_scope,
    oo1: *mut options,
    scope2: window_customize_scope,
    oo2: *mut options,
    ft: *mut format_tree,
    filter: *const u8,
    fs: *mut cmd_find_state,
) {
    unsafe {
        let mut o = null_mut();
        let mut list = null_mut();
        let mut size: u32 = 0;

        let top = mode_tree_add(
            (*data).data,
            null_mut(),
            null_mut(),
            tag,
            title,
            null_mut(),
            0,
        );
        mode_tree_no_tag(top);

        // We get the options from the first tree, but build it using the
        // values from the other two. Any tree can have user options so we need
        // to build a separate list of them.

        window_customize_find_user_options(oo0, &raw mut list, &raw mut size);
        if !oo1.is_null() {
            window_customize_find_user_options(oo1, &raw mut list, &raw mut size);
        }
        if !oo2.is_null() {
            window_customize_find_user_options(oo2, &raw mut list, &raw mut size);
        }

        for i in 0..size {
            if !oo2.is_null() {
                o = options_get(oo2, *list.add(i as usize));
            }
            if o.is_null() && !oo1.is_null() {
                o = options_get(oo1, *list.add(i as usize));
            }
            if o.is_null() {
                o = options_get(oo0, *list.add(i as usize));
            }
            let scope = if options_owner(o) == oo2 {
                scope2
            } else if options_owner(o) == oo1 {
                scope1
            } else {
                scope0
            };
            window_customize_build_option(data, top, scope, o, ft, filter, fs);
        }
        free_(list);

        let mut loop_ = options_first(oo0);
        while !loop_.is_null() {
            let name: *const u8 = options_name(loop_);
            if *name == b'@' {
                loop_ = options_next(loop_);
                continue;
            }
            if !oo2.is_null() {
                o = options_get(oo2, name);
            } else if !oo1.is_null() {
                o = options_get(oo1, name);
            } else {
                o = loop_;
            }
            let scope = if options_owner(o) == oo2 {
                scope2
            } else if options_owner(o) == oo1 {
                scope1
            } else {
                scope0
            };
            window_customize_build_option(data, top, scope, o, ft, filter, fs);
            loop_ = options_next(loop_);
        }
    }
}

unsafe fn window_customize_build_keys(
    data: *mut window_customize_modedata,
    kt: *mut key_table,
    mut ft: *mut format_tree,
    filter: *const u8,
    fs: *mut cmd_find_state,
    number: i32,
) {
    unsafe {
        // struct mode_tree_item *top, *child, *mti;
        // struct window_customize_itemdata *item;
        // struct key_binding *bd;
        // char *title, *text, *tmp, *expanded;
        // const char *flag;
        // uint64_t tag;

        let mut text: *mut u8 = null_mut();
        let mut title: *mut u8 = null_mut();
        let tag: u64 = (1u64 << 62) | ((number as u64) << 54) | 1;

        title = format_nul!("Key Table - {}", _s((*kt).name));
        let top = mode_tree_add(
            (*data).data,
            null_mut(),
            null_mut(),
            tag,
            title,
            null_mut(),
            0,
        );
        mode_tree_no_tag(top);
        free_(title);

        ft = format_create_from_state(null_mut(), null_mut(), fs);
        format_add!(ft, c!("is_option"), "0");
        format_add!(ft, c!("is_key"), "1");

        let mut bd = key_bindings_first(kt);
        while !bd.is_null() {
            format_add!(ft, c!("key"), "{}", _s(key_string_lookup_key((*bd).key, 0)),);
            if !(*bd).note.is_null() {
                format_add!(ft, c!("key_note"), "{}", _s((*bd).note));
            }
            if !filter.is_null() {
                let expanded = format_expand(ft, filter);
                if format_true(expanded) == 0 {
                    free_(expanded);
                    continue;
                }
                free_(expanded);
            }

            let item = window_customize_add_item(data);
            (*item).scope = window_customize_scope::WINDOW_CUSTOMIZE_KEY;
            (*item).table = xstrdup((*kt).name).as_ptr();
            (*item).key = (*bd).key;
            (*item).name = xstrdup(key_string_lookup_key((*item).key, 0)).as_ptr();
            (*item).idx = -1;

            let expanded = format_expand(ft, (*data).format);
            let child = mode_tree_add(
                (*data).data,
                top,
                item.cast(),
                bd as u64,
                expanded,
                null_mut(),
                0,
            );
            free_(expanded);

            let tmp = cmd_list_print(&mut *(*bd).cmdlist, 0);
            text = format_nul!("#[ignore]{}", _s(tmp));
            free_(tmp);
            let mut mti = mode_tree_add(
                (*data).data,
                child,
                item.cast(),
                tag | ((*bd).key << 3) | 1,
                c!("Command"),
                text,
                -1,
            );
            mode_tree_draw_as_parent(mti);
            mode_tree_no_tag(mti);
            free_(text);

            if !(*bd).note.is_null() {
                text = format_nul!("#[ignore]{}", _s((*bd).note));
            } else {
                text = xstrdup(c!("")).as_ptr();
            }
            mti = mode_tree_add(
                (*data).data,
                child,
                item.cast(),
                tag | ((*bd).key << 3) | (1 << 1) | 1,
                c!("Note"),
                text,
                -1,
            );
            mode_tree_draw_as_parent(mti);
            mode_tree_no_tag(mti);
            free_(text);

            let flag = if (*bd).flags & KEY_BINDING_REPEAT != 0 {
                c!("on")
            } else {
                c!("off")
            };
            mti = mode_tree_add(
                (*data).data,
                child,
                item.cast(),
                tag | ((*bd).key << 3) | (2 << 1) | 1,
                c!("Repeat"),
                flag,
                -1,
            );
            mode_tree_draw_as_parent(mti);
            mode_tree_no_tag(mti);

            bd = key_bindings_next(kt, bd);
        }

        format_free(ft);
    }
}

unsafe fn window_customize_build(
    modedata: NonNull<c_void>,
    _: *mut mode_tree_sort_criteria,
    _: *mut u64,
    filter: *const u8,
) {
    unsafe {
        let data: NonNull<window_customize_modedata> = modedata.cast();
        let data = data.as_ptr();
        let mut fs: cmd_find_state = zeroed();

        for i in 0..(*data).item_size {
            window_customize_free_item(*(*data).item_list.add(i as usize));
        }
        free_((*data).item_list);
        (*data).item_list = null_mut();
        (*data).item_size = 0;

        if cmd_find_valid_state(&raw mut (*data).fs) {
            cmd_find_copy_state(&raw mut fs, &raw mut (*data).fs);
        } else {
            cmd_find_from_pane(&raw mut fs, (*data).wp, 0);
        }

        let mut ft = format_create_from_state(null_mut(), null_mut(), &raw mut fs);
        format_add!(ft, c!("is_option"), "1");
        format_add!(ft, c!("is_key"), "0");

        window_customize_build_options(
            data,
            c!("Server Options"),
            (3u64 << 62) | ((OPTIONS_TABLE_SERVER as u64) << 1) | 1,
            window_customize_scope::WINDOW_CUSTOMIZE_SERVER,
            GLOBAL_OPTIONS,
            window_customize_scope::WINDOW_CUSTOMIZE_NONE,
            null_mut(),
            window_customize_scope::WINDOW_CUSTOMIZE_NONE,
            null_mut(),
            ft,
            filter,
            &raw mut fs,
        );
        window_customize_build_options(
            data,
            c!("Session Options"),
            (3u64 << 62) | ((OPTIONS_TABLE_SESSION as u64) << 1) | 1,
            window_customize_scope::WINDOW_CUSTOMIZE_GLOBAL_SESSION,
            GLOBAL_S_OPTIONS,
            window_customize_scope::WINDOW_CUSTOMIZE_SESSION,
            (*fs.s).options,
            window_customize_scope::WINDOW_CUSTOMIZE_NONE,
            null_mut(),
            ft,
            filter,
            &raw mut fs,
        );
        window_customize_build_options(
            data,
            c!("Window & Pane Options"),
            (3u64 << 62) | ((OPTIONS_TABLE_WINDOW as u64) << 1) | 1,
            window_customize_scope::WINDOW_CUSTOMIZE_GLOBAL_WINDOW,
            GLOBAL_W_OPTIONS,
            window_customize_scope::WINDOW_CUSTOMIZE_WINDOW,
            (*fs.w).options,
            window_customize_scope::WINDOW_CUSTOMIZE_PANE,
            (*fs.wp).options,
            ft,
            filter,
            &raw mut fs,
        );

        format_free(ft);
        ft = format_create_from_state(null_mut(), null_mut(), &raw mut fs);

        let mut i = 0;
        let mut kt = key_bindings_first_table();
        while !kt.is_null() {
            if !rb_empty(&raw mut (*kt).key_bindings) {
                window_customize_build_keys(data, kt, ft, filter, &raw mut fs, i);
                i += 1;
                if i == 256 {
                    break;
                }
            }
            kt = key_bindings_next_table(kt);
        }

        format_free(ft);
    }
}

unsafe fn window_customize_draw_key(
    _: *mut window_customize_modedata,
    item: *mut window_customize_itemdata,
    ctx: *mut screen_write_ctx,
    sx: u32,
    sy: u32,
) {
    unsafe {
        let s = (*ctx).s;
        let cx = (*s).cx;
        let cy = (*s).cy;

        let mut kt: *mut key_table = null_mut();
        let mut bd: *mut key_binding = null_mut();
        let mut period = c!("");

        if item.is_null() || window_customize_get_key(item, &raw mut kt, &raw mut bd) == 0 {
            return;
        }

        let mut note: *const u8 = (*bd).note;
        if note.is_null() {
            note = c!("There is no note for this key.");
        }
        if *note != b'\0' && *note.add(libc::strlen(note) - 1) != b'.' {
            period = c!(".");
        }
        if !screen_write_text!(
            ctx,
            cx,
            sx,
            sy,
            0,
            &GRID_DEFAULT_CELL,
            "{}{}",
            _s(note),
            _s(period),
        ) {
            return;
        }
        screen_write_cursormove(ctx, cx as i32, (*s).cy as i32 + 1, 0); /* skip line */
        if (*s).cy >= cy + sy - 1 {
            return;
        }

        if !screen_write_text!(
            ctx,
            cx,
            sx,
            sy - ((*s).cy - cy),
            0,
            &raw const GRID_DEFAULT_CELL,
            "This key is in the {} table.",
            _s((*kt).name),
        ) {
            return;
        }
        if !screen_write_text!(
            ctx,
            cx,
            sx,
            sy - ((*s).cy - cy),
            0,
            &raw const GRID_DEFAULT_CELL,
            "This key {} repeat.",
            if (*bd).flags & KEY_BINDING_REPEAT != 0 {
                "does"
            } else {
                "does not"
            },
        ) {
            return;
        }
        screen_write_cursormove(ctx, cx as i32, (*s).cy as i32 + 1, 0); /* skip line */
        if (*s).cy >= cy + sy - 1 {
            return;
        }

        let cmd = cmd_list_print(&mut *(*bd).cmdlist, 0);
        if !screen_write_text!(
            ctx,
            cx,
            sx,
            sy - ((*s).cy - cy),
            0,
            &raw const GRID_DEFAULT_CELL,
            "Command: {}",
            _s(cmd),
        ) {
            free_(cmd);
            return;
        }
        let default_bd = key_bindings_get_default(kt, (*bd).key);
        if !default_bd.is_null() {
            let default_cmd = cmd_list_print(&mut *(*default_bd).cmdlist, 0);
            if libc::strcmp(cmd, default_cmd) != 0
                && !screen_write_text!(
                    ctx,
                    cx,
                    sx,
                    sy - ((*s).cy - cy),
                    0,
                    &GRID_DEFAULT_CELL,
                    "The default is: {}",
                    _s(default_cmd),
                )
            {
                free_(default_cmd);
                free_(cmd);
                return;
            }
            free_(default_cmd);
        }
        free_(cmd);
    }
}

unsafe fn window_customize_draw_option(
    data: *mut window_customize_modedata,
    item: *mut window_customize_itemdata,
    ctx: *mut screen_write_ctx,
    sx: u32,
    sy: u32,
) {
    unsafe {
        let s = (*ctx).s;
        let cx = (*s).cx;
        let cy = (*s).cy;

        // int idx;
        // struct options_entry *o, *parent;
        // struct options *go, *wo;
        // const struct options_table_entry *oe;
        // struct grid_cell gc;
        // const char **choice, *text, *name;
        let mut gc: grid_cell = zeroed();
        let mut space: *const u8 = c!("");
        let mut unit: *const u8 = c!("");

        let mut expanded: *mut u8 = null_mut();
        let mut value: *mut u8 = null_mut();
        let mut default_value: *mut u8 = null_mut();
        // char choices[256] = "";

        let mut fs: cmd_find_state = zeroed();
        let ft = null_mut();

        'out: {
            if !window_customize_check_item(data, item, &raw mut fs) {
                return;
            }
            let name: *mut u8 = (*item).name;
            let idx = (*item).idx;

            let o = options_get((*item).oo, name);
            if o.is_null() {
                return;
            }
            let oe = options_table_entry(o);

            if !oe.is_null() && !(*oe).unit.is_null() {
                space = c!(" ");
                unit = (*oe).unit;
            }
            let ft = format_create_from_state(null_mut(), null_mut(), &raw mut fs);

            let mut text = if oe.is_null() || (*oe).text.is_null() {
                c!("This option doesn't have a description.")
            } else {
                (*oe).text
            };

            if !screen_write_text!(
                ctx,
                cx,
                sx,
                sy,
                0,
                &raw const GRID_DEFAULT_CELL,
                "{}",
                _s(text),
            ) {
                break 'out;
            }
            screen_write_cursormove(ctx, cx as i32, (*s).cy as i32 + 1, 0); /* skip line */
            if (*s).cy >= cy + sy - 1 {
                break 'out;
            }

            if oe.is_null() {
                text = c!("user");
            } else if ((*oe).scope & (OPTIONS_TABLE_WINDOW | OPTIONS_TABLE_PANE))
                == (OPTIONS_TABLE_WINDOW | OPTIONS_TABLE_PANE)
            {
                text = c!("window and pane");
            } else if (*oe).scope & OPTIONS_TABLE_WINDOW != 0 {
                text = c!("window");
            } else if (*oe).scope & OPTIONS_TABLE_SESSION != 0 {
                text = c!("session");
            } else {
                text = c!("server");
            }
            if !screen_write_text!(
                ctx,
                cx,
                sx,
                sy - ((*s).cy - cy),
                0,
                &raw const GRID_DEFAULT_CELL,
                "This is a {} option.",
                _s(text),
            ) {
                break 'out;
            }
            if !oe.is_null() && (*oe).flags & OPTIONS_TABLE_IS_ARRAY != 0 {
                if idx != -1 {
                    if !screen_write_text!(
                        ctx,
                        cx,
                        sx,
                        sy - ((*s).cy - cy),
                        0,
                        &raw const GRID_DEFAULT_CELL,
                        "This is an array option, index {idx}."
                    ) {
                        break 'out;
                    }
                } else if !screen_write_text!(
                    ctx,
                    cx,
                    sx,
                    sy - ((*s).cy - cy),
                    0,
                    &raw const GRID_DEFAULT_CELL,
                    "This is an array option.",
                ) {
                    break 'out;
                }
                if idx == -1 {
                    break 'out;
                }
            }
            screen_write_cursormove(ctx, cx as i32, (*s).cy as i32 + 1, 0); /* skip line */
            if (*s).cy >= cy + sy - 1 {
                break 'out;
            }

            value = options_to_string(o, idx, 0);
            if !oe.is_null() && idx == -1 {
                default_value = options_default_to_string(oe).as_ptr();
                if libc::strcmp(default_value, value) == 0 {
                    free_(default_value);
                    default_value = null_mut();
                }
            }
            if !screen_write_text!(
                ctx,
                cx,
                sx,
                sy - ((*s).cy - cy),
                0,
                &raw const GRID_DEFAULT_CELL,
                "Option value: {}{}{}",
                _s(value),
                _s(space),
                _s(unit),
            ) {
                break 'out;
            }
            if oe.is_null() || (*oe).type_ == options_table_type::OPTIONS_TABLE_STRING {
                expanded = format_expand(ft, value);
                if libc::strcmp(expanded, value) != 0
                    && !screen_write_text!(
                        ctx,
                        cx,
                        sx,
                        sy - ((*s).cy - cy),
                        0,
                        &raw const GRID_DEFAULT_CELL,
                        "This expands to: {}",
                        _s(expanded),
                    )
                {
                    break 'out;
                }
                free_(expanded);
            }

            const SIZEOF_CHOICES: usize = 256;
            let mut choices: [u8; SIZEOF_CHOICES] = [0; SIZEOF_CHOICES];
            if !oe.is_null() && (*oe).type_ == options_table_type::OPTIONS_TABLE_CHOICE {
                let mut choice = (*oe).choices;
                while !(*choice).is_null() {
                    strlcat(choices.as_mut_ptr(), *choice, SIZEOF_CHOICES);
                    strlcat(choices.as_mut_ptr(), c!(", "), SIZEOF_CHOICES);
                    choice = choice.add(1);
                }
                choices[libc::strlen(choices.as_ptr()) - 2] = b'\0';
                if !screen_write_text!(
                    ctx,
                    cx,
                    sx,
                    sy - ((*s).cy - cy),
                    0,
                    &raw const GRID_DEFAULT_CELL,
                    "Available values are: {}",
                    _s((&raw const choices) as *const u8),
                ) {
                    break 'out;
                }
            }
            if !oe.is_null() && (*oe).type_ == options_table_type::OPTIONS_TABLE_COLOUR {
                if !screen_write_text!(
                    ctx,
                    cx,
                    sx,
                    sy - ((*s).cy - cy),
                    1,
                    &raw const GRID_DEFAULT_CELL,
                    "This is a colour option: ",
                ) {
                    break 'out;
                }
                memcpy__(&raw mut gc, &raw const GRID_DEFAULT_CELL);
                gc.fg = options_get_number((*item).oo, name) as i32;
                if !screen_write_text!(
                    ctx,
                    cx,
                    sx,
                    sy - ((*s).cy - cy),
                    0,
                    &raw const gc,
                    "EXAMPLE",
                ) {
                    break 'out;
                }
            }
            if !oe.is_null() && (*oe).flags & OPTIONS_TABLE_IS_STYLE != 0 {
                if !screen_write_text!(
                    ctx,
                    cx,
                    sx,
                    sy - ((*s).cy - cy),
                    1,
                    &raw const GRID_DEFAULT_CELL,
                    "This is a style option: "
                ) {
                    break 'out;
                }
                style_apply(&raw mut gc, (*item).oo, name, ft);
                if !screen_write_text!(ctx, cx, sx, sy - ((*s).cy - cy), 0, &raw mut gc, "EXAMPLE")
                {
                    break 'out;
                }
            }
            if !default_value.is_null()
                && !screen_write_text!(
                    ctx,
                    cx,
                    sx,
                    sy - ((*s).cy - cy),
                    0,
                    &raw const GRID_DEFAULT_CELL,
                    "The default is: {}{}{}",
                    _s(default_value),
                    _s(space),
                    _s(unit),
                )
            {
                break 'out;
            }

            screen_write_cursormove(ctx, cx as i32, (*s).cy as i32 + 1, 0); /* skip line */
            if (*s).cy > cy + sy - 1 {
                break 'out;
            }
            let (wo, go) = if !oe.is_null() && (*oe).flags & OPTIONS_TABLE_IS_ARRAY != 0 {
                (null_mut(), null_mut())
            } else {
                match (*item).scope {
                    window_customize_scope::WINDOW_CUSTOMIZE_PANE => {
                        let wo = options_get_parent((*item).oo);
                        (wo, options_get_parent(wo))
                    }
                    window_customize_scope::WINDOW_CUSTOMIZE_WINDOW
                    | window_customize_scope::WINDOW_CUSTOMIZE_SESSION => {
                        (null_mut(), options_get_parent((*item).oo))
                    }
                    _ => (null_mut(), null_mut()),
                }
            };
            if !wo.is_null() && options_owner(o) != wo {
                let parent = options_get_only(wo, name);
                if !parent.is_null() {
                    value = options_to_string(parent, -1, 0);
                    if !screen_write_text!(
                        ctx,
                        (*s).cx,
                        sx,
                        sy - ((*s).cy - cy),
                        0,
                        &raw const GRID_DEFAULT_CELL,
                        "Window value (from window {}): {}{}{}",
                        (*fs.wl).idx,
                        _s(value),
                        _s(space),
                        _s(unit),
                    ) {
                        break 'out;
                    }
                }
            }
            if !go.is_null() && options_owner(o) != go {
                let parent = options_get_only(go, name);
                if !parent.is_null() {
                    value = options_to_string(parent, -1, 0);
                    if !screen_write_text!(
                        ctx,
                        (*s).cx,
                        sx,
                        sy - ((*s).cy - cy),
                        0,
                        &raw const GRID_DEFAULT_CELL,
                        "Global value: {}{}{}",
                        _s(value),
                        _s(space),
                        _s(unit),
                    ) {
                        break 'out;
                    }
                }
            }
        } // 'out:
        free_(value);
        free_(default_value);
        format_free(ft);
    }
}

unsafe fn window_customize_draw(
    modedata: *mut c_void,
    itemdata: Option<NonNull<c_void>>,
    ctx: *mut screen_write_ctx,
    sx: u32,
    sy: u32,
) {
    unsafe {
        let data = modedata as *mut window_customize_modedata;
        let item: Option<NonNull<window_customize_itemdata>> = itemdata.map(NonNull::cast);

        let Some(item) = item else {
            return;
        };

        if (*item.as_ptr()).scope == window_customize_scope::WINDOW_CUSTOMIZE_KEY {
            window_customize_draw_key(data, item.as_ptr(), ctx, sx, sy);
        } else {
            window_customize_draw_option(data, item.as_ptr(), ctx, sx, sy);
        }
    }
}

unsafe fn window_customize_menu(modedata: NonNull<c_void>, c: *mut client, key: key_code) {
    unsafe {
        let data: NonNull<window_customize_modedata> = modedata.cast();
        let wp: *mut window_pane = (*data.as_ptr()).wp;

        let Some(wme) = NonNull::new(tailq_first(&raw mut (*wp).modes)) else {
            return;
        };

        if (*wme.as_ptr()).data != modedata.as_ptr() {
            return;
        }

        window_customize_key(wme, c, null_mut(), null_mut(), key, null_mut());
    }
}

unsafe fn window_customize_height(_modedata: *mut c_void, _height: u32) -> u32 {
    12
}

pub unsafe fn window_customize_init(
    wme: NonNull<window_mode_entry>,
    fs: *mut cmd_find_state,
    args: *mut args,
) -> *mut screen {
    unsafe {
        let wp = (*wme.as_ptr()).wp;
        let mut s: *mut screen = null_mut();

        let data: *mut window_customize_modedata = xcalloc1() as *mut window_customize_modedata;
        (*wme.as_ptr()).data = data.cast();
        (*data).wp = wp;
        (*data).references = 1;

        memcpy__(&raw mut (*data).fs, fs);

        if args.is_null() || !args_has_(args, 'F') {
            (*data).format = xstrdup_(WINDOW_CUSTOMIZE_DEFAULT_FORMAT).as_ptr();
        } else {
            (*data).format = xstrdup(args_get_(args, 'F')).as_ptr();
        }

        (*data).data = mode_tree_start(
            wp,
            args,
            Some(window_customize_build),
            Some(window_customize_draw),
            None,
            Some(window_customize_menu),
            Some(window_customize_height),
            None,
            data.cast(),
            WINDOW_CUSTOMIZE_MENU_ITEMS.as_slice(),
            null_mut(),
            0,
            &raw mut s,
        );
        mode_tree_zoom((*data).data, args);

        mode_tree_build((*data).data);
        mode_tree_draw((*data).data);

        s
    }
}

pub unsafe fn window_customize_destroy(data: *mut window_customize_modedata) {
    unsafe {
        (*data).references -= 1;
        if (*data).references != 0 {
            return;
        }

        for i in 0..(*data).item_size {
            window_customize_free_item(*(*data).item_list.add(i as usize));
        }
        free_((*data).item_list);

        free_((*data).format);

        free_(data);
    }
}

pub unsafe fn window_customize_free(wme: NonNull<window_mode_entry>) {
    unsafe {
        let data: *mut window_customize_modedata = (*wme.as_ptr()).data.cast();

        if data.is_null() {
            return;
        }

        (*data).dead = 1;
        mode_tree_free((*data).data);
        window_customize_destroy(data);
    }
}

pub unsafe fn window_customize_resize(wme: NonNull<window_mode_entry>, sx: u32, sy: u32) {
    unsafe {
        let data: *mut window_customize_modedata = (*wme.as_ptr()).data.cast();

        mode_tree_resize((*data).data, sx, sy);
    }
}

pub unsafe fn window_customize_free_callback(modedata: NonNull<c_void>) {
    unsafe {
        window_customize_destroy(modedata.cast().as_ptr());
    }
}

pub unsafe fn window_customize_free_item_callback(itemdata: NonNull<c_void>) {
    unsafe {
        let item: NonNull<window_customize_itemdata> = itemdata.cast();
        let data: *mut window_customize_modedata = (*item.as_ptr()).data;

        window_customize_free_item(item.as_ptr());
        window_customize_destroy(data);
    }
}

pub unsafe fn window_customize_set_option_callback(
    c: *mut client,
    itemdata: NonNull<c_void>,
    s: *const u8,
    done: i32,
) -> i32 {
    unsafe {
        let item: NonNull<window_customize_itemdata> = itemdata.cast();
        let item = item.as_ptr();
        let data: *mut window_customize_modedata = (*item).data.cast();

        let oo: *mut options = (*item).oo;
        let name: *mut u8 = (*item).name;

        let mut cause: *mut u8 = null_mut();
        let mut idx: i32 = (*item).idx;

        'fail: {
            if s.is_null() || *s == b'\0' || (*data).dead != 0 {
                return 0;
            }
            if item.is_null() || !window_customize_check_item(data, item, null_mut()) {
                return 0;
            }
            let o = options_get(oo, name);
            if o.is_null() {
                return 0;
            }
            let oe = options_table_entry(o);

            if !oe.is_null() && (*oe).flags & OPTIONS_TABLE_IS_ARRAY != 0 {
                if idx == -1 {
                    for idx_ in 0..i32::MAX {
                        idx = idx_;
                        if options_array_get(o, idx as u32).is_null() {
                            break;
                        }
                    }
                }
                if options_array_set(o, idx as u32, Some(cstr_to_str(s)), 0, &raw mut cause) != 0 {
                    break 'fail;
                }
            } else if options_from_string(oo, oe, name, s, 0, &raw mut cause) != 0 {
                break 'fail;
            }

            options_push_changes((*item).name);
            mode_tree_build((*data).data);
            mode_tree_draw((*data).data);
            (*(*data).wp).flags |= window_pane_flags::PANE_REDRAW;

            return 0;
        } // 'fail:
        *cause = (*cause).to_ascii_uppercase();
        status_message_set!(c, -1, 1, 0, "{}", _s(cause));
        free_(cause);
        0
    }
}

pub unsafe fn window_customize_set_option(
    c: *mut client,
    data: *mut window_customize_modedata,
    item: *mut window_customize_itemdata,
    global: i32,
    mut pane: i32,
) {
    unsafe {
        // struct options_entry *o;
        // const struct options_table_entry *oe;
        // struct options *oo;
        // struct window_customize_itemdata *new_item;
        let mut flag: i32 = 0;
        let idx = (*item).idx;
        let mut scope = window_customize_scope::WINDOW_CUSTOMIZE_NONE;

        let mut choice: u32;
        let name = (*item).name;
        let mut space = c!("");
        let mut oo: *mut options = null_mut();

        // char *prompt, *value, *text;
        // struct cmd_find_state fs;
        let mut value = null_mut();
        let mut fs: cmd_find_state = zeroed();

        if item.is_null() || !window_customize_check_item(data, item, &raw mut fs) {
            return;
        }
        let o = options_get((*item).oo, name);
        if o.is_null() {
            return;
        }

        let oe = options_table_entry(o);
        if !oe.is_null() && !(*oe).scope & OPTIONS_TABLE_PANE != 0 {
            pane = 0;
        }
        if !oe.is_null() && (*oe).flags & OPTIONS_TABLE_IS_ARRAY != 0 {
            scope = (*item).scope;
            oo = (*item).oo;
        } else {
            if global != 0 {
                match (*item).scope {
                    window_customize_scope::WINDOW_CUSTOMIZE_NONE
                    | window_customize_scope::WINDOW_CUSTOMIZE_KEY
                    | window_customize_scope::WINDOW_CUSTOMIZE_SERVER
                    | window_customize_scope::WINDOW_CUSTOMIZE_GLOBAL_SESSION
                    | window_customize_scope::WINDOW_CUSTOMIZE_GLOBAL_WINDOW => {
                        scope = (*item).scope
                    }
                    window_customize_scope::WINDOW_CUSTOMIZE_SESSION => {
                        scope = window_customize_scope::WINDOW_CUSTOMIZE_GLOBAL_SESSION
                    }
                    window_customize_scope::WINDOW_CUSTOMIZE_WINDOW
                    | window_customize_scope::WINDOW_CUSTOMIZE_PANE => {
                        scope = window_customize_scope::WINDOW_CUSTOMIZE_GLOBAL_WINDOW
                    }
                }
            } else {
                match (*item).scope {
                    window_customize_scope::WINDOW_CUSTOMIZE_NONE
                    | window_customize_scope::WINDOW_CUSTOMIZE_KEY
                    | window_customize_scope::WINDOW_CUSTOMIZE_SERVER
                    | window_customize_scope::WINDOW_CUSTOMIZE_SESSION => scope = (*item).scope,
                    window_customize_scope::WINDOW_CUSTOMIZE_WINDOW
                    | window_customize_scope::WINDOW_CUSTOMIZE_PANE => {
                        if pane != 0 {
                            scope = window_customize_scope::WINDOW_CUSTOMIZE_PANE;
                        } else {
                            scope = window_customize_scope::WINDOW_CUSTOMIZE_WINDOW;
                        }
                    }
                    window_customize_scope::WINDOW_CUSTOMIZE_GLOBAL_SESSION => {
                        scope = window_customize_scope::WINDOW_CUSTOMIZE_SESSION
                    }
                    window_customize_scope::WINDOW_CUSTOMIZE_GLOBAL_WINDOW => {
                        if pane != 0 {
                            scope = window_customize_scope::WINDOW_CUSTOMIZE_PANE;
                        } else {
                            scope = window_customize_scope::WINDOW_CUSTOMIZE_WINDOW;
                        }
                    }
                }
            }

            if scope == (*item).scope {
                oo = (*item).oo;
            } else {
                oo = window_customize_get_tree(scope, &raw mut fs);
            }
        }

        if !oe.is_null() && (*oe).type_ == options_table_type::OPTIONS_TABLE_FLAG {
            flag = options_get_number(oo, name) as i32;
            options_set_number(oo, name, (flag == 0) as i64);
        } else if !oe.is_null() && (*oe).type_ == options_table_type::OPTIONS_TABLE_CHOICE {
            choice = options_get_number(oo, name) as u32;
            if (*(*oe).choices.add(choice as usize + 1)).is_null() {
                choice = 0;
            } else {
                choice += 1;
            }
            options_set_number(oo, name, choice as i64);
        } else {
            let text = window_customize_scope_text(scope, &raw mut fs);
            if *text != b'\0' {
                space = c!(", for ");
            } else if scope != window_customize_scope::WINDOW_CUSTOMIZE_SERVER {
                space = c!(", global");
            }
            let prompt = if !oe.is_null() && (*oe).flags & OPTIONS_TABLE_IS_ARRAY != 0 {
                if idx == -1 {
                    format_nul!("({}[+]{}{}) ", _s(name), _s(space), _s(text))
                } else {
                    format_nul!("({}[{}]{}{}) ", _s(name), idx, _s(space), _s(text))
                }
            } else {
                format_nul!("({}{}{}) ", _s(name), _s(space), _s(text))
            };
            free_(text);

            value = options_to_string(o, idx, 0);

            let new_item =
                xcalloc1::<window_customize_itemdata>() as *mut window_customize_itemdata;
            (*new_item).data = data;
            (*new_item).scope = scope;
            (*new_item).oo = oo;
            (*new_item).name = xstrdup(name).as_ptr();
            (*new_item).idx = idx;

            (*data).references += 1;
            status_prompt_set(
                c,
                null_mut(),
                prompt,
                value,
                Some(window_customize_set_option_callback),
                Some(window_customize_free_item_callback),
                new_item.cast(),
                PROMPT_NOFORMAT,
                prompt_type::PROMPT_TYPE_COMMAND,
            );

            free_(prompt);
            free_(value);
        }
    }
}

pub unsafe fn window_customize_unset_option(
    data: *mut window_customize_modedata,
    item: *mut window_customize_itemdata,
) {
    unsafe {
        if item.is_null() || !window_customize_check_item(data, item, null_mut()) {
            return;
        }

        let o = options_get((*item).oo, (*item).name);
        if o.is_null() {
            return;
        }
        if (*item).idx != -1 && item.cast() == mode_tree_get_current((*data).data).as_ptr() {
            mode_tree_up((*data).data, 0);
        }
        options_remove_or_default(o, (*item).idx, null_mut());
    }
}

pub unsafe fn window_customize_reset_option(
    data: *mut window_customize_modedata,
    item: *mut window_customize_itemdata,
) {
    unsafe {
        if item.is_null() || !window_customize_check_item(data, item, null_mut()) {
            return;
        }
        if (*item).idx != -1 {
            return;
        }

        let mut oo = (*item).oo;
        while !oo.is_null() {
            let o = options_get_only((*item).oo, (*item).name);
            if !o.is_null() {
                options_remove_or_default(o, -1, null_mut());
            }
            oo = options_get_parent(oo);
        }
    }
}

pub unsafe fn window_customize_set_command_callback(
    c: *mut client,
    itemdata: NonNull<c_void>,
    s: *const u8,
    _done: i32,
) -> i32 {
    unsafe {
        let item: NonNull<window_customize_itemdata> = itemdata.cast();
        let item = item.as_ptr();
        let data: *mut window_customize_modedata = (*item).data;
        let mut bd: *mut key_binding = null_mut();
        let mut error: *mut u8 = null_mut();

        'fail: {
            if s.is_null() || *s == b'\0' || (*data).dead != 0 {
                return 0;
            }
            if item.is_null() || window_customize_get_key(item, null_mut(), &raw mut bd) == 0 {
                return 0;
            }

            let cmdlist = match cmd_parse_from_string(cstr_to_str(s), None) {
                Ok(cmdlist) => cmdlist,
                Err(pr_error) => {
                    error = pr_error;
                    break 'fail;
                }
            };
            cmd_list_free((*bd).cmdlist);
            (*bd).cmdlist = cmdlist;

            mode_tree_build((*data).data);
            mode_tree_draw((*data).data);
            (*(*data).wp).flags |= window_pane_flags::PANE_REDRAW;

            return 0;
        }
        // 'fail:
        *error = (*error).to_ascii_uppercase();
        status_message_set!(c, -1, 1, 0, "{}", _s(error));
        free_(error);
        0
    }
}

pub unsafe fn window_customize_set_note_callback(
    _c: *mut client,
    itemdata: NonNull<c_void>,
    s: *const u8,
    _done: i32,
) -> i32 {
    unsafe {
        let item: NonNull<window_customize_itemdata> = itemdata.cast();
        let item = item.as_ptr();
        let data: *mut window_customize_modedata = (*item).data;
        let mut bd = null_mut();

        if s.is_null() || *s == b'\0' || (*data).dead != 0 {
            return 0;
        }
        if item.is_null() || window_customize_get_key(item, null_mut(), &raw mut bd) == 0 {
            return 0;
        }

        free_((*bd).note);
        (*bd).note = xstrdup(s).as_ptr();

        mode_tree_build((*data).data);
        mode_tree_draw((*data).data);
        (*(*data).wp).flags |= window_pane_flags::PANE_REDRAW;

        0
    }
}

pub unsafe fn window_customize_set_key(
    c: *mut client,
    data: *mut window_customize_modedata,
    item: *mut window_customize_itemdata,
) {
    unsafe {
        let key = (*item).key;
        let mut prompt: *mut u8 = null_mut();
        let mut value: *mut u8 = null_mut();
        let mut bd: *mut key_binding = null_mut();

        if item.is_null() || window_customize_get_key(item, null_mut(), &raw mut bd) == 0 {
            return;
        }

        let s = mode_tree_get_current_name((*data).data);
        if streq_(s, "Repeat") {
            (*bd).flags ^= KEY_BINDING_REPEAT;
        } else if streq_(s, "Command") {
            prompt = format_nul!("({}) ", _s(key_string_lookup_key(key, 0)));
            value = cmd_list_print(&mut *(*bd).cmdlist, 0);

            let new_item =
                xcalloc1::<window_customize_itemdata>() as *mut window_customize_itemdata;
            (*new_item).data = data;
            (*new_item).scope = (*item).scope;
            (*new_item).table = xstrdup((*item).table).as_ptr();
            (*new_item).key = key;

            (*data).references += 1;
            status_prompt_set(
                c,
                null_mut(),
                prompt,
                value,
                Some(window_customize_set_command_callback),
                Some(window_customize_free_item_callback),
                new_item.cast(),
                PROMPT_NOFORMAT,
                prompt_type::PROMPT_TYPE_COMMAND,
            );
            free_(prompt);
            free_(value);
        } else if streq_(s, "Note") {
            prompt = format_nul!("({}) ", _s(key_string_lookup_key(key, 0)));

            let new_item =
                xcalloc1::<window_customize_itemdata>() as *mut window_customize_itemdata;
            (*new_item).data = data;
            (*new_item).scope = (*item).scope;
            (*new_item).table = xstrdup((*item).table).as_ptr();
            (*new_item).key = key;

            (*data).references += 1;
            status_prompt_set(
                c,
                null_mut(),
                prompt,
                if (*bd).note.is_null() {
                    c!("")
                } else {
                    (*bd).note
                },
                Some(window_customize_set_note_callback),
                Some(window_customize_free_item_callback),
                new_item.cast(),
                PROMPT_NOFORMAT,
                prompt_type::PROMPT_TYPE_COMMAND,
            );
            free_(prompt);
        }
    }
}

pub unsafe fn window_customize_unset_key(
    data: *mut window_customize_modedata,
    item: *mut window_customize_itemdata,
) {
    unsafe {
        let mut kt: *mut key_table = null_mut();
        let mut bd: *mut key_binding = null_mut();

        if item.is_null() || window_customize_get_key(item, &raw mut kt, &raw mut bd) == 0 {
            return;
        }

        if item == mode_tree_get_current((*data).data).as_ptr().cast() {
            mode_tree_collapse_current((*data).data);
            mode_tree_up((*data).data, 0);
        }
        key_bindings_remove((*kt).name, (*bd).key);
    }
}

pub unsafe fn window_customize_reset_key(
    data: *mut window_customize_modedata,
    item: *mut window_customize_itemdata,
) {
    unsafe {
        let mut kt: *mut key_table = null_mut();
        let mut bd: *mut key_binding = null_mut();

        if item.is_null() || window_customize_get_key(item, &raw mut kt, &raw mut bd) == 0 {
            return;
        }

        let dd: *mut key_binding = key_bindings_get_default(kt, (*bd).key);
        if !dd.is_null() && (*bd).cmdlist == (*dd).cmdlist {
            return;
        }
        if dd.is_null() && item == mode_tree_get_current((*data).data).as_ptr().cast() {
            mode_tree_collapse_current((*data).data);
            mode_tree_up((*data).data, 0);
        }
        key_bindings_reset((*kt).name, (*bd).key);
    }
}

pub unsafe fn window_customize_change_each(
    modedata: NonNull<c_void>,
    itemdata: NonNull<c_void>,
    _c: *mut client,
    _key: key_code,
) {
    unsafe {
        let data: NonNull<window_customize_modedata> = modedata.cast();
        let item: NonNull<window_customize_itemdata> = itemdata.cast();

        let data = data.as_ptr();
        let item = item.as_ptr();

        match (*data).change {
            window_customize_change::WINDOW_CUSTOMIZE_UNSET => {
                if (*item).scope == window_customize_scope::WINDOW_CUSTOMIZE_KEY {
                    window_customize_unset_key(data, item);
                } else {
                    window_customize_unset_option(data, item);
                }
            }
            window_customize_change::WINDOW_CUSTOMIZE_RESET => {
                if (*item).scope == window_customize_scope::WINDOW_CUSTOMIZE_KEY {
                    window_customize_reset_key(data, item);
                } else {
                    window_customize_reset_option(data, item);
                }
            }
        }
        if (*item).scope != window_customize_scope::WINDOW_CUSTOMIZE_KEY {
            options_push_changes((*item).name);
        }
    }
}

pub unsafe fn window_customize_change_current_callback(
    c: *mut client,
    modedata: NonNull<c_void>,
    s: *const u8,
    _done: i32,
) -> i32 {
    unsafe {
        let data: *mut window_customize_modedata = modedata.cast().as_ptr();
        let mut item: *mut window_customize_itemdata = null_mut();

        if s.is_null() || *s == b'\0' || (*data).dead != 0 {
            return 0;
        }
        if !(*s).eq_ignore_ascii_case(&b'y') || *s.add(1) != b'\0' {
            return 0;
        }

        item = mode_tree_get_current((*data).data).as_ptr().cast();
        match (*data).change {
            window_customize_change::WINDOW_CUSTOMIZE_UNSET => {
                if (*item).scope == window_customize_scope::WINDOW_CUSTOMIZE_KEY {
                    window_customize_unset_key(data, item);
                } else {
                    window_customize_unset_option(data, item);
                }
            }
            window_customize_change::WINDOW_CUSTOMIZE_RESET => {
                if (*item).scope == window_customize_scope::WINDOW_CUSTOMIZE_KEY {
                    window_customize_reset_key(data, item);
                } else {
                    window_customize_reset_option(data, item);
                }
            }
        }
        if (*item).scope != window_customize_scope::WINDOW_CUSTOMIZE_KEY {
            options_push_changes((*item).name);
        }
        mode_tree_build((*data).data);
        mode_tree_draw((*data).data);
        (*(*data).wp).flags |= window_pane_flags::PANE_REDRAW;

        0
    }
}

pub unsafe fn window_customize_change_tagged_callback(
    c: *mut client,
    modedata: NonNull<c_void>,
    s: *const u8,
    _done: i32,
) -> i32 {
    unsafe {
        let data: *mut window_customize_modedata = modedata.cast().as_ptr();

        if s.is_null() || *s == b'\0' || (*data).dead != 0 {
            return 0;
        }
        if !(*s).eq_ignore_ascii_case(&b'y') || *s.add(1) != b'\0' {
            return 0;
        }

        mode_tree_each_tagged(
            (*data).data,
            Some(window_customize_change_each),
            c,
            KEYC_NONE,
            0,
        );
        mode_tree_build((*data).data);
        mode_tree_draw((*data).data);
        (*(*data).wp).flags |= window_pane_flags::PANE_REDRAW;

        0
    }
}

pub unsafe fn window_customize_key(
    wme: NonNull<window_mode_entry>,
    c: *mut client,
    s: *mut session,
    wl: *mut winlink,
    mut key: key_code,
    m: *mut mouse_event,
) {
    unsafe {
        let wp: *mut window_pane = (*wme.as_ptr()).wp;
        let data: *mut window_customize_modedata = (*wme.as_ptr()).data.cast();
        let mut item: *mut window_customize_itemdata =
            mode_tree_get_current((*data).data).cast().as_ptr();
        let mut prompt = null_mut();
        let finished: i32 = mode_tree_key((*data).data, c, &raw mut key, m, null_mut(), null_mut());

        let new_item: NonNull<window_customize_itemdata> =
            mode_tree_get_current((*data).data).cast();
        if item != new_item.as_ptr() {
            item = new_item.as_ptr();
        }

        match key as u8 {
            b'\r' | b's' => {
                if !item.is_null() {
                    if (*item).scope == window_customize_scope::WINDOW_CUSTOMIZE_KEY {
                        window_customize_set_key(c, data, item);
                    } else {
                        window_customize_set_option(c, data, item, 0, 1);
                        options_push_changes((*item).name);
                    }
                    mode_tree_build((*data).data);
                }
            }
            b'w' => {
                if !(item.is_null()
                    || (*item).scope == window_customize_scope::WINDOW_CUSTOMIZE_KEY)
                {
                    window_customize_set_option(c, data, item, 0, 0);
                    options_push_changes((*item).name);
                    mode_tree_build((*data).data);
                }
            }
            b'S' | b'W' => {
                if !(item.is_null()
                    || (*item).scope == window_customize_scope::WINDOW_CUSTOMIZE_KEY)
                {
                    window_customize_set_option(c, data, item, 1, 0);
                    options_push_changes((*item).name);
                    mode_tree_build((*data).data);
                }
            }
            b'd' => {
                if !(item.is_null() || (*item).idx != -1) {
                    prompt = format_nul!("Reset {} to default? ", _s((*item).name));
                    (*data).references += 1;
                    (*data).change = window_customize_change::WINDOW_CUSTOMIZE_RESET;
                    status_prompt_set(
                        c,
                        null_mut(),
                        prompt,
                        c!(""),
                        Some(window_customize_change_current_callback),
                        Some(window_customize_free_callback),
                        data.cast(),
                        PROMPT_SINGLE | PROMPT_NOFORMAT,
                        prompt_type::PROMPT_TYPE_COMMAND,
                    );
                    free_(prompt);
                }
            }
            b'D' => {
                let tagged = mode_tree_count_tagged((*data).data);
                if tagged != 0 {
                    prompt = format_nul!("Reset {} tagged to default? ", tagged);
                    (*data).references += 1;
                    (*data).change = window_customize_change::WINDOW_CUSTOMIZE_RESET;
                    status_prompt_set(
                        c,
                        null_mut(),
                        prompt,
                        c!(""),
                        Some(window_customize_change_tagged_callback),
                        Some(window_customize_free_callback),
                        data.cast(),
                        PROMPT_SINGLE | PROMPT_NOFORMAT,
                        prompt_type::PROMPT_TYPE_COMMAND,
                    );
                    free_(prompt);
                }
            }
            b'u' => {
                if !item.is_null() {
                    let idx = (*item).idx;
                    prompt = if idx != -1 {
                        format_nul!("Unset {}[{}]? ", _s((*item).name), idx)
                    } else {
                        format_nul!("Unset {}? ", _s((*item).name))
                    };
                    (*data).references += 1;
                    (*data).change = window_customize_change::WINDOW_CUSTOMIZE_UNSET;
                    status_prompt_set(
                        c,
                        null_mut(),
                        prompt,
                        c!(""),
                        Some(window_customize_change_current_callback),
                        Some(window_customize_free_callback),
                        data.cast(),
                        PROMPT_SINGLE | PROMPT_NOFORMAT,
                        prompt_type::PROMPT_TYPE_COMMAND,
                    );
                    free_(prompt);
                }
            }
            b'U' => {
                let tagged = mode_tree_count_tagged((*data).data);
                if tagged != 0 {
                    prompt = format_nul!("Unset {} tagged? ", tagged);
                    (*data).references += 1;
                    (*data).change = window_customize_change::WINDOW_CUSTOMIZE_UNSET;
                    status_prompt_set(
                        c,
                        null_mut(),
                        prompt,
                        c!(""),
                        Some(window_customize_change_tagged_callback),
                        Some(window_customize_free_callback),
                        data.cast(),
                        PROMPT_SINGLE | PROMPT_NOFORMAT,
                        prompt_type::PROMPT_TYPE_COMMAND,
                    );
                    free_(prompt);
                }
            }
            b'H' => {
                (*data).hide_global = !(*data).hide_global;
                mode_tree_build((*data).data);
            }
            _ => (),
        }
        if finished != 0 {
            window_pane_reset_mode(wp);
        } else {
            mode_tree_draw((*data).data);
            (*wp).flags |= window_pane_flags::PANE_REDRAW;
        }
    }
}

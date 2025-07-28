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

use crate::compat::tree::rb_foreach;
use crate::{cmd_::cmd_queue::cmdq_get_callback1, options_::options_get_number_};

const WINDOW_TREE_DEFAULT_COMMAND: &str = "switch-client -Zt '%%'";
const WINDOW_TREE_DEFAULT_FORMAT: &CStr = cstring_concat!(
    "#{?pane_format,",
    "#{?pane_marked,#[reverse],}",
    "#{pane_current_command}#{?pane_active,*,}#{?pane_marked,M,}",
    "#{?#{&&:#{pane_title},#{!=:#{pane_title},#{host_short}}},: ",
    "\"#{pane_title}\",}",
    ",",
    "#{?window_format,",
    "#{?window_marked_flag,#[reverse],}",
    "#{window_name}#{window_flags}",
    "#{?#{&&:#{==:#{window_panes},1},#{&&:#{pane_title},#{!=:#{pane_title},#{",
    "host_short}}}},: \"#{pane_title}\",}",
    ",",
    "#{session_windows} windows",
    "#{?session_grouped, ",
    "(group #{session_group}: ",
    "#{session_group_list}),",
    "}",
    "#{?session_attached, (attached),}",
    "}",
    "}"
);

const WINDOW_TREE_DEFAULT_KEY_FORMAT: &CStr = cstring_concat!(
    "#{?#{e|<:#{line},10},",
    "#{line}",
    ",",
    "#{?#{e|<:#{line},36},",
    "M-#{a:#{e|+:97,#{e|-:#{line},10}}}",
    ",",
    "",
    "}",
    "}"
);

static WINDOW_TREE_MENU_ITEMS: [menu_item; 12] = [
    menu_item::new(c"Select", b'\r' as key_code, null()),
    menu_item::new(c"Expand", keyc::KEYC_RIGHT as key_code, null()),
    menu_item::new(c"Mark", 'm' as key_code, null()),
    menu_item::new(c"", KEYC_NONE, null()),
    menu_item::new(c"Tag", b't' as key_code, null()),
    menu_item::new(c"Tag All", b'\x14' as key_code, null()),
    menu_item::new(c"Tag None", b'T' as key_code, null()),
    menu_item::new(c"", KEYC_NONE, null()),
    menu_item::new(c"Kill", b'x' as key_code, null()),
    menu_item::new(c"Kill Tagged", b'X' as key_code, null()),
    menu_item::new(c"", KEYC_NONE, null()),
    menu_item::new(c"Cancel", b'q' as key_code, null()),
];

pub static WINDOW_TREE_MODE: window_mode = window_mode {
    name: SyncCharPtr::new(c"tree-mode"),
    default_format: SyncCharPtr::from_ptr(WINDOW_TREE_DEFAULT_FORMAT.as_ptr().cast()),

    init: window_tree_init,
    free: window_tree_free,
    resize: window_tree_resize,
    update: Some(window_tree_update),
    key: Some(window_tree_key),
    key_table: None,
    command: None,
    formats: None,
};

enum_try_from!(
    window_tree_sort_type,
    i32,
    window_tree_sort_type::WINDOW_TREE_BY_TIME
);
#[repr(i32)]
enum window_tree_sort_type {
    WINDOW_TREE_BY_INDEX,
    WINDOW_TREE_BY_NAME,
    WINDOW_TREE_BY_TIME,
}

const WINDOW_TREE_SORT_LIST_LEN: usize = 3;

static mut WINDOW_TREE_SORT_LIST: [SyncCharPtr; WINDOW_TREE_SORT_LIST_LEN] = [
    SyncCharPtr::new(c"index"),
    SyncCharPtr::new(c"name"),
    SyncCharPtr::new(c"time"),
];

static mut WINDOW_TREE_SORT: *mut mode_tree_sort_criteria = null_mut();

#[repr(i32)]
#[derive(Eq, PartialEq)]
enum window_tree_type {
    WINDOW_TREE_NONE,
    WINDOW_TREE_SESSION,
    WINDOW_TREE_WINDOW,
    WINDOW_TREE_PANE,
}

#[repr(C)]
struct window_tree_itemdata {
    type_: window_tree_type,
    session: i32,
    winlink: i32,
    pane: i32,
}

#[repr(C)]
struct window_tree_modedata {
    wp: *mut window_pane,
    dead: i32,
    references: i32,

    data: *mut mode_tree_data,
    format: *mut u8,
    key_format: *mut u8,
    command: *mut u8,
    squash_groups: i32,

    item_list: *mut *mut window_tree_itemdata,
    item_size: u32,

    entered: *const u8,

    fs: cmd_find_state,
    type_: window_tree_type,

    offset: i32,

    left: i32,
    right: i32,
    start: u32,
    end: u32,
    each: u32,
}

unsafe fn window_tree_pull_item(
    item: NonNull<window_tree_itemdata>,
    sp: *mut Option<NonNull<session>>,
    wlp: *mut Option<NonNull<winlink>>,
    wp: *mut Option<NonNull<window_pane>>,
) {
    unsafe {
        *wp = None;
        *wlp = None;
        *sp = session_find_by_id((*item.as_ptr()).session as u32);
        if (*sp).is_none() {
            return;
        }

        if (*item.as_ptr()).type_ == window_tree_type::WINDOW_TREE_SESSION {
            *wlp = std::mem::transmute::<*mut winlink, Option<NonNull<winlink>>>(
                (*(*sp).unwrap().as_ptr()).curw,
            );
            *wp = std::mem::transmute::<*mut window_pane, Option<NonNull<window_pane>>>(
                (*(*(*wlp).unwrap().as_ptr()).window).active,
            );
            return;
        }

        *wlp =
            std::mem::transmute::<*mut winlink, Option<NonNull<winlink>>>(winlink_find_by_index(
                &raw mut (*transmute_ptr(*sp)).windows,
                (*item.as_ptr()).winlink,
            ));
        if (*wlp).is_none() {
            *sp = None;
            return;
        }

        if (*item.as_ptr()).type_ == window_tree_type::WINDOW_TREE_WINDOW {
            *wp = std::mem::transmute::<*mut window_pane, Option<NonNull<window_pane>>>(
                (*(*(*wlp).unwrap().as_ptr()).window).active,
            );
            return;
        }

        *wp = std::mem::transmute::<*mut window_pane, Option<NonNull<window_pane>>>(
            window_pane_find_by_id((*item.as_ptr()).pane as u32),
        );
        if !window_has_pane((*(*wlp).unwrap().as_ptr()).window, transmute_ptr(*wp)) {
            *wp = None;
        }
        if (*wp).is_none() {
            *sp = None;
            *wlp = None;
        }
    }
}

unsafe fn window_tree_add_item(data: NonNull<window_tree_modedata>) -> *mut window_tree_itemdata {
    unsafe {
        let data = data.as_ptr();
        (*data).item_list =
            xreallocarray_((*data).item_list, (*data).item_size as usize + 1).as_ptr();
        let item: *mut window_tree_itemdata = xcalloc1() as *mut window_tree_itemdata;
        *(*data).item_list.add((*data).item_size as usize) = item;
        (*data).item_size += 1;
        item
    }
}

unsafe fn window_tree_free_item(item: *mut window_tree_itemdata) {
    unsafe {
        free_(item);
    }
}

unsafe extern "C" fn window_tree_cmp_session(a0: *const c_void, b0: *const c_void) -> i32 {
    unsafe {
        let a: *mut *mut session = a0 as *mut *mut session;
        let b: *mut *mut session = b0 as *mut *mut session;
        let sa = *a;
        let sb = *b;

        let mut result: i32 = 0;
        match window_tree_sort_type::try_from((*WINDOW_TREE_SORT).field as i32) {
            Ok(window_tree_sort_type::WINDOW_TREE_BY_INDEX) => {
                result = ((*sa).id as i32).wrapping_sub((*sb).id as i32)
            }
            Ok(window_tree_sort_type::WINDOW_TREE_BY_TIME) => {
                if timer::new(&raw const (*sa).activity_time)
                    > timer::new(&raw const (*sb).activity_time)
                {
                    result = -1;
                } else if timer::new(&raw const (*sa).activity_time)
                    < timer::new(&raw const (*sb).activity_time)
                {
                    result = 1;
                } else {
                    result = libc::strcmp((*sa).name, (*sb).name);
                }
            }
            Ok(window_tree_sort_type::WINDOW_TREE_BY_NAME) => {
                result = libc::strcmp((*sa).name, (*sb).name)
            }
            Err(_) => (),
        }

        if (*WINDOW_TREE_SORT).reversed != 0 {
            result = -result;
        }

        result
    }
}

unsafe extern "C" fn window_tree_cmp_window(a0: *const c_void, b0: *const c_void) -> i32 {
    unsafe {
        let a = a0 as *mut *mut winlink;
        let b = b0 as *mut *mut winlink;
        let wla: *mut winlink = *a;
        let wlb: *mut winlink = *b;
        let wa = (*wla).window;
        let wb = (*wlb).window;
        let mut result: i32 = 0;

        match window_tree_sort_type::try_from((*WINDOW_TREE_SORT).field as i32) {
            Ok(window_tree_sort_type::WINDOW_TREE_BY_INDEX) => result = (*wla).idx - (*wlb).idx,
            Ok(window_tree_sort_type::WINDOW_TREE_BY_TIME) => {
                if timer::new(&raw const (*wa).activity_time)
                    > timer::new(&raw const (*wb).activity_time)
                {
                    result = -1;
                } else if timer::new(&raw const (*wa).activity_time)
                    < timer::new(&raw const (*wb).activity_time)
                {
                    result = 1;
                } else {
                    result = libc::strcmp((*wa).name, (*wb).name);
                }
            }
            Ok(window_tree_sort_type::WINDOW_TREE_BY_NAME) => {
                result = libc::strcmp((*wa).name, (*wb).name)
            }
            Err(_) => (),
        }

        if (*WINDOW_TREE_SORT).reversed != 0 {
            result = -result;
        }
        result
    }
}

unsafe extern "C" fn window_tree_cmp_pane(a0: *const c_void, b0: *const c_void) -> i32 {
    unsafe {
        let a = a0 as *mut *mut window_pane;
        let b = b0 as *mut *mut window_pane;
        let mut result: i32;
        let mut ai: u32 = 0;
        let mut bi: u32 = 0;

        if (*WINDOW_TREE_SORT).field == window_tree_sort_type::WINDOW_TREE_BY_TIME as u32 {
            result = ((**a).active_point as i32).wrapping_sub((**b).active_point as i32);
        } else {
            // Panes don't have names, so use number order for any other sort field.
            window_pane_index(*a, &raw mut ai);
            window_pane_index(*b, &raw mut bi);
            result = ai as i32 - bi as i32;
        }
        if (*WINDOW_TREE_SORT).reversed != 0 {
            result = -result;
        }
        result
    }
}

unsafe fn window_tree_build_pane(
    s: *mut session,
    wl: *mut winlink,
    wp: *mut window_pane,
    modedata: NonNull<c_void>,
    parent: *mut mode_tree_item,
) {
    unsafe {
        let data: NonNull<window_tree_modedata> = modedata.cast();
        let mut idx: u32 = 0;

        window_pane_index(wp, &raw mut idx);

        let item = window_tree_add_item(data);
        (*item).type_ = window_tree_type::WINDOW_TREE_PANE;
        (*item).session = (*s).id as i32;
        (*item).winlink = (*wl).idx;
        (*item).pane = (*wp).id as i32;

        let text: *mut u8 =
            format_single(null_mut(), (*data.as_ptr()).format, null_mut(), s, wl, wp);
        let name = format_nul!("{idx}");

        mode_tree_add(
            (*data.as_ptr()).data,
            parent,
            item.cast(),
            wp as u64,
            name,
            text,
            -1,
        );
        free_(text);
        free_(name);
    }
}

unsafe fn window_tree_filter_pane(
    s: *mut session,
    wl: *mut winlink,
    wp: *mut window_pane,
    filter: *const u8,
) -> i32 {
    unsafe {
        if filter.is_null() {
            return 1;
        }

        let cp: *mut u8 = format_single(null_mut(), filter, null_mut(), s, wl, wp);
        let result = format_true(cp);
        free_(cp);

        result
    }
}

unsafe fn window_tree_build_window(
    s: *mut session,
    wl: *mut winlink,
    modedata: NonNull<c_void>,
    sort_crit: *mut mode_tree_sort_criteria,
    parent: *mut mode_tree_item,
    filter: *const u8,
) -> i32 {
    unsafe {
        let data: NonNull<window_tree_modedata> = modedata.cast();
        let item: *mut window_tree_itemdata;

        let mti: *mut mode_tree_item;
        let name: *mut u8;
        let text: *mut u8;

        let mut l: *mut *mut window_pane;

        let mut n: u32;
        let expanded: i32;

        'empty: {
            item = window_tree_add_item(data);
            (*item).type_ = window_tree_type::WINDOW_TREE_WINDOW;
            (*item).session = (*s).id as i32;
            (*item).winlink = (*wl).idx;
            (*item).pane = -1;

            text = format_single(
                null_mut(),
                (*data.as_ptr()).format,
                null_mut(),
                s,
                wl,
                null_mut(),
            );
            name = format_nul!("{}", (*wl).idx);

            if matches!(
                (*data.as_ptr()).type_,
                window_tree_type::WINDOW_TREE_SESSION | window_tree_type::WINDOW_TREE_WINDOW
            ) {
                expanded = 0;
            } else {
                expanded = 1;
            }
            mti = mode_tree_add(
                (*data.as_ptr()).data,
                parent,
                item.cast(),
                wl as u64,
                name,
                text,
                expanded,
            );
            free_(text);
            free_(name);

            let wp = tailq_first(&raw mut (*(*wl).window).panes);
            if wp.is_null() {
                break 'empty;
            }
            if tailq_next::<_, window_pane, discr_entry>(wp).is_null() {
                if window_tree_filter_pane(s, wl, wp, filter) == 0 {
                    break 'empty;
                }
                return 1;
            }

            l = null_mut();
            n = 0;

            for wp in
                tailq_foreach::<_, discr_entry>(&raw mut (*(*wl).window).panes).map(NonNull::as_ptr)
            {
                if window_tree_filter_pane(s, wl, wp, filter) == 0 {
                    continue;
                }
                l = xreallocarray_(l, n as usize + 1).as_ptr();
                *l.add(n as usize) = wp;
                n += 1;
            }
            if n == 0 {
                break 'empty;
            }

            WINDOW_TREE_SORT = sort_crit;
            libc::qsort(
                l.cast(),
                n as usize,
                size_of::<*mut window_pane>(),
                Some(window_tree_cmp_pane),
            );

            for i in 0..n {
                window_tree_build_pane(s, wl, *l.add(i as usize), modedata, mti);
            }
            free_(l);
            return 1;
        }
        // empty:
        window_tree_free_item(item);
        (*data.as_ptr()).item_size -= 1;
        mode_tree_remove((*data.as_ptr()).data, mti);
        0
    }
}

unsafe fn window_tree_build_session(
    s: *mut session,
    modedata: NonNull<c_void>,
    sort_crit: *mut mode_tree_sort_criteria,
    filter: *const u8,
) {
    unsafe {
        let data: NonNull<window_tree_modedata> = modedata.cast();

        let item = window_tree_add_item(data);
        let data = data.as_ptr();
        (*item).type_ = window_tree_type::WINDOW_TREE_SESSION;
        (*item).session = (*s).id as i32;
        (*item).winlink = -1;
        (*item).pane = -1;

        let text = format_single(
            null_mut(),
            (*data).format,
            null_mut(),
            s,
            null_mut(),
            null_mut(),
        );

        let expanded = if (*data).type_ == window_tree_type::WINDOW_TREE_SESSION {
            0
        } else {
            1
        };
        let mti = mode_tree_add(
            (*data).data,
            null_mut(),
            item.cast(),
            s as u64,
            (*s).name,
            text,
            expanded,
        );
        free_(text);

        let mut l: *mut *mut winlink = null_mut();
        let mut n = 0;
        for wl in rb_foreach(&raw mut (*s).windows).map(NonNull::as_ptr) {
            l = xreallocarray_(l, n + 1).as_ptr();
            *l.add(n) = wl;
            n += 1;
        }
        WINDOW_TREE_SORT = sort_crit;
        libc::qsort(
            l.cast(),
            n,
            size_of::<&mut winlink>(),
            Some(window_tree_cmp_window),
        );

        let mut empty = 0;
        for i in 0..n {
            if window_tree_build_window(s, *l.add(i), modedata, sort_crit, mti, filter) == 0 {
                empty += 1;
            }
        }
        if empty == n {
            window_tree_free_item(item);
            (*data).item_size -= 1;
            mode_tree_remove((*data).data, mti);
        }
        free_(l);
    }
}

unsafe fn window_tree_build(
    modedata: NonNull<c_void>,
    sort_crit: *mut mode_tree_sort_criteria,
    tag: *mut u64,
    filter: *const u8,
) {
    unsafe {
        let data: NonNull<window_tree_modedata> = modedata.cast();
        let data = data.as_ptr();

        let mut sg: *mut session_group;

        // u_int n, i;
        let current = session_group_contains((*data).fs.s);

        for i in 0..(*data).item_size {
            window_tree_free_item(*(*data).item_list.add(i as usize));
        }
        free_((*data).item_list);
        (*data).item_list = null_mut();
        (*data).item_size = 0;

        let mut l: *mut *mut session = null_mut();
        let mut n: u32 = 0;
        for s in rb_foreach(&raw mut SESSIONS).map(NonNull::as_ptr) {
            if (*data).squash_groups != 0
                && ({
                    sg = session_group_contains(s);
                    !sg.is_null()
                })
                && ((sg == current && s != (*data).fs.s)
                    || (sg != current && s != tailq_first(&raw mut (*sg).sessions)))
            {
                continue;
            }
            l = xreallocarray_(l, n as usize + 1).as_ptr();
            *l.add(n as usize) = s;
            n += 1;
        }
        WINDOW_TREE_SORT = sort_crit;
        libc::qsort(
            l.cast(),
            n as usize,
            size_of::<*mut session>(),
            Some(window_tree_cmp_session),
        );

        for i in 0..n {
            window_tree_build_session(*l.add(i as usize), modedata, sort_crit, filter);
        }
        free_(l);

        match (*data).type_ {
            window_tree_type::WINDOW_TREE_NONE => (),
            window_tree_type::WINDOW_TREE_SESSION => *tag = (*data).fs.s as u64,
            window_tree_type::WINDOW_TREE_WINDOW => *tag = (*data).fs.wl as u64,
            window_tree_type::WINDOW_TREE_PANE => {
                if window_count_panes((*(*data).fs.wl).window) == 1 {
                    *tag = (*data).fs.wl as u64;
                } else {
                    *tag = (*data).fs.wp as u64;
                }
            }
        }
    }
}

unsafe fn window_tree_draw_label(
    ctx: *mut screen_write_ctx,
    px: u32,
    py: u32,
    sx: u32,
    sy: u32,
    gc: *mut grid_cell,
    label: *const u8,
) {
    unsafe {
        let len = strlen(label);
        if sx == 0 || sy == 1 || len as u32 > sx {
            return;
        }
        let ox = (sx - len as u32).div_ceil(2);
        let oy = sy.div_ceil(2);

        if ox > 1 && (ox + len as u32) < sx - 1 && sy >= 3 {
            screen_write_cursormove(ctx, (px + ox - 1) as i32, (py + oy - 1) as i32, 0);
            screen_write_box(
                ctx,
                len as u32 + 2,
                3,
                box_lines::BOX_LINES_DEFAULT,
                null_mut(),
                null_mut(),
            );
        }
        screen_write_cursormove(ctx, (px + ox) as i32, (py + oy) as i32, 0);
        screen_write_puts!(ctx, gc, "{}", _s(label));
    }
}

unsafe fn window_tree_draw_session(
    data: *mut window_tree_modedata,
    s: *mut session,
    ctx: *mut screen_write_ctx,
    sx: u32,
    sy: u32,
) {
    unsafe {
        let oo = (*s).options;

        let cx: u32 = (*(*ctx).s).cx;
        let cy: u32 = (*(*ctx).s).cy;
        let mut loop_: u32;
        let mut visible: u32;
        let each: u32;
        let mut width: u32;
        let mut offset: u32;

        let mut start: u32;
        let mut end: u32;
        let remaining: u32;
        let mut i: u32;

        let mut gc: grid_cell = zeroed();
        let mut label: *mut u8;

        let total = winlink_count(&raw mut (*s).windows);

        memcpy__(&raw mut gc, &raw const GRID_DEFAULT_CELL);
        let colour = options_get_number_(oo, "display-panes-colour");
        let active_colour = options_get_number_(oo, "display-panes-active-colour");

        if sx / total < 24 {
            visible = sx / 24;
            if visible == 0 {
                visible = 1;
            }
        } else {
            visible = total;
        }

        let mut current: u32 = 0;
        for wl in rb_foreach(&raw mut (*s).windows).map(NonNull::as_ptr) {
            if wl == (*s).curw {
                break;
            }
            current += 1;
        }

        if current < visible {
            start = 0;
            end = visible;
        } else if current >= total - visible {
            start = total - visible;
            end = total;
        } else {
            start = current - (visible / 2);
            end = start + visible;
        }

        if (*data).offset < -(start as i32) {
            (*data).offset = -(start as i32);
        }
        if (*data).offset > (total - end) as i32 {
            (*data).offset = (total - end) as i32;
        }
        start += (*data).offset as u32;
        end += (*data).offset as u32;

        let mut left = start != 0;
        let mut right = end != total;
        if ((left && right) && sx <= 6) || ((left || right) && sx <= 3) {
            left = false;
            right = false;
        }
        if left && right {
            each = (sx - 6) / visible;
            remaining = (sx - 6) - (visible * each);
        } else if left || right {
            each = (sx - 3) / visible;
            remaining = (sx - 3) - (visible * each);
        } else {
            each = sx / visible;
            remaining = sx - (visible * each);
        }
        if each == 0 {
            return;
        }

        if left {
            (*data).left = (cx + 2) as i32;
            screen_write_cursormove(ctx, (cx + 2) as i32, cy as i32, 0);
            screen_write_vline(ctx, sy, 0, 0);
            screen_write_cursormove(ctx, cx as i32, (cy + sy / 2) as i32, 0);
            screen_write_puts!(ctx, &raw const GRID_DEFAULT_CELL, "<");
        } else {
            (*data).left = -1;
        }
        if right {
            (*data).right = (cx + sx - 3) as i32;
            screen_write_cursormove(ctx, (cx + sx - 3) as i32, cy as i32, 0);
            screen_write_vline(ctx, sy, 0, 0);
            screen_write_cursormove(ctx, (cx + sx - 1) as i32, (cy + sy / 2) as i32, 0);
            screen_write_puts!(ctx, &raw const GRID_DEFAULT_CELL, ">");
        } else {
            (*data).right = -1;
        }

        (*data).start = start;
        (*data).end = end;
        (*data).each = each;

        loop_ = 0;
        i = 0;
        for wl in rb_foreach(&raw mut (*s).windows).map(NonNull::as_ptr) {
            if loop_ == end {
                break;
            }
            if loop_ < start {
                loop_ += 1;
                continue;
            }
            let w = (*wl).window;

            if wl == (*s).curw {
                gc.fg = active_colour as i32;
            } else {
                gc.fg = colour as i32;
            }

            if left {
                offset = 3 + (i * each);
            } else {
                offset = i * each;
            }
            if loop_ == end - 1 {
                width = each + remaining;
            } else {
                width = each - 1;
            }

            screen_write_cursormove(ctx, (cx + offset) as i32, cy as i32, 0);
            screen_write_preview(ctx, &raw mut (*(*w).active).base, width, sy);

            label = format_nul!(" {}:{} ", (*wl).idx, _s((*w).name));
            if strlen(label) > width as usize {
                label = format_nul!(" {} ", (*wl).idx);
            }
            window_tree_draw_label(ctx, cx + offset, cy, width, sy, &raw mut gc, label);
            free_(label);

            if loop_ != end - 1 {
                screen_write_cursormove(ctx, (cx + offset + width) as i32, cy as i32, 0);
                screen_write_vline(ctx, sy, 0, 0);
            }
            loop_ += 1;

            i += 1;
        }
    }
}

unsafe fn window_tree_draw_window(
    data: *mut window_tree_modedata,
    s: *mut session,
    w: *mut window,
    ctx: *mut screen_write_ctx,
    sx: u32,
    sy: u32,
) {
    unsafe {
        let oo = (*s).options;
        // struct window_pane *wp;
        let cx = (*(*ctx).s).cx;
        let cy = (*(*ctx).s).cy;
        // u_int loop_, total, visible, each, width, offset;
        // u_int current, start, end, remaining, i, pane_idx;
        // struct grid_cell gc;
        let mut gc: grid_cell = zeroed();
        // int colour, active_colour, left, right;
        // char *label;

        let total = window_count_panes(w);

        memcpy__(&raw mut gc, &raw const GRID_DEFAULT_CELL);
        let colour: i32 = options_get_number_(oo, "display-panes-colour") as i32;
        let active_colour: i32 = options_get_number_(oo, "display-panes-active-colour") as i32;

        let visible = if sx / total < 24 {
            if sx / 24 != 0 { sx / 24 } else { 1 }
        } else {
            total
        };

        let mut current: u32 = 0;
        for wp in tailq_foreach::<_, discr_entry>(&raw mut (*w).panes).map(NonNull::as_ptr) {
            if wp == (*w).active {
                break;
            }
            current += 1;
        }

        let (mut start, mut end) = if current < visible {
            (0, visible)
        } else if current >= total - visible {
            (total - visible, total)
        } else {
            let start = current - (visible / 2);
            (start, start + visible)
        };

        if (*data).offset < -(start as i32) {
            (*data).offset = -(start as i32);
        }
        if (*data).offset > (total - end) as i32 {
            (*data).offset = (total - end) as i32;
        }
        start += (*data).offset as u32;
        end += (*data).offset as u32;

        let mut left = start != 0;
        let mut right = end != total;
        if ((left && right) && sx <= 6) || ((left || right) && sx <= 3) {
            left = false;
            right = false;
        }

        let each;
        let remaining;
        if left && right {
            each = (sx - 6) / visible;
            remaining = (sx - 6) - (visible * each);
        } else if left || right {
            each = (sx - 3) / visible;
            remaining = (sx - 3) - (visible * each);
        } else {
            each = sx / visible;
            remaining = sx - (visible * each);
        }

        if each == 0 {
            return;
        }

        if left {
            (*data).left = (cx + 2) as i32;
            screen_write_cursormove(ctx, (cx + 2) as i32, cy as i32, 0);
            screen_write_vline(ctx, sy, 0, 0);
            screen_write_cursormove(ctx, cx as i32, (cy + sy / 2) as i32, 0);
            screen_write_puts!(ctx, &raw const GRID_DEFAULT_CELL, "<");
        } else {
            (*data).left = -1;
        }
        if right {
            (*data).right = (cx + sx - 3) as i32;
            screen_write_cursormove(ctx, (cx + sx - 3) as i32, cy as i32, 0);
            screen_write_vline(ctx, sy, 0, 0);
            screen_write_cursormove(ctx, (cx + sx - 1) as i32, (cy + sy / 2) as i32, 0);
            screen_write_puts!(ctx, &raw const GRID_DEFAULT_CELL, ">");
        } else {
            (*data).right = -1;
        }

        (*data).start = start;
        (*data).end = end;
        (*data).each = each;

        let mut i = 0;
        let mut loop_ = 0;
        for wp in tailq_foreach::<_, discr_entry>(&raw mut (*w).panes).map(NonNull::as_ptr) {
            if loop_ == end {
                break;
            }
            if loop_ < start {
                loop_ += 1;
                continue;
            }

            if wp == (*w).active {
                gc.fg = active_colour;
            } else {
                gc.fg = colour;
            }

            let offset = if left { 3 + (i * each) } else { i * each };
            let width = if loop_ == end - 1 {
                each + remaining
            } else {
                each - 1
            };

            screen_write_cursormove(ctx, (cx + offset) as i32, cy as i32, 0);
            screen_write_preview(ctx, &raw mut (*wp).base, width, sy);

            let mut pane_idx: u32 = 0;

            if window_pane_index(wp, &raw mut pane_idx) != 0 {
                pane_idx = loop_;
            }
            let label = format_nul!(" {} ", pane_idx);
            window_tree_draw_label(ctx, cx + offset, cy, each, sy, &raw mut gc, label);
            free_(label);

            if loop_ != end - 1 {
                screen_write_cursormove(ctx, (cx + offset + width) as i32, cy as i32, 0);
                screen_write_vline(ctx, sy, 0, 0);
            }
            loop_ += 1;

            i += 1;
        }
    }
}

unsafe fn window_tree_draw(
    modedata: *mut c_void,
    itemdata: Option<NonNull<c_void>>,
    ctx: *mut screen_write_ctx,
    sx: u32,
    sy: u32,
) {
    unsafe {
        let item: Option<NonNull<window_tree_itemdata>> = itemdata.map(NonNull::cast);
        let mut sp: Option<NonNull<session>> = None;
        let mut wlp: Option<NonNull<winlink>> = None;
        let mut wp: Option<NonNull<window_pane>> = None;

        window_tree_pull_item(item.unwrap(), &raw mut sp, &raw mut wlp, &raw mut wp);
        let Some(wp) = wp else {
            return;
        };

        match (*item.unwrap().as_ptr()).type_ {
            window_tree_type::WINDOW_TREE_NONE => (),
            window_tree_type::WINDOW_TREE_SESSION => {
                window_tree_draw_session(modedata.cast(), transmute_ptr(sp), ctx, sx, sy)
            }
            window_tree_type::WINDOW_TREE_WINDOW => window_tree_draw_window(
                modedata.cast(),
                transmute_ptr(sp),
                (*transmute_ptr(wlp)).window,
                ctx,
                sx,
                sy,
            ),
            window_tree_type::WINDOW_TREE_PANE => {
                screen_write_preview(ctx, &raw mut (*wp.as_ptr()).base, sx, sy)
            }
        };
    }
}

unsafe fn window_tree_search(
    _modedata: *mut c_void,
    itemdata: NonNull<c_void>,
    ss: *const u8,
) -> bool {
    unsafe {
        let item: NonNull<window_tree_itemdata> = itemdata.cast();
        let mut s: Option<NonNull<session>> = None;
        let mut wl: Option<NonNull<winlink>> = None;
        let mut wp: Option<NonNull<window_pane>> = None;
        window_tree_pull_item(item, &raw mut s, &raw mut wl, &raw mut wp);

        match (*item.as_ptr()).type_ {
            window_tree_type::WINDOW_TREE_NONE => return false,
            window_tree_type::WINDOW_TREE_SESSION => {
                if let Some(s) = s {
                    return !libc::strstr((*s.as_ptr()).name, ss).is_null();
                }
            }
            window_tree_type::WINDOW_TREE_WINDOW => {
                if s.is_some()
                    && let Some(wl) = wl
                {
                    return !libc::strstr((*(*wl.as_ptr()).window).name, ss).is_null();
                }
            }
            window_tree_type::WINDOW_TREE_PANE => {
                if s.is_some()
                    && wl.is_some()
                    && let Some(wp) = wp
                {
                    let cmd: *mut u8 =
                        osdep_get_name((*wp.as_ptr()).fd, (&raw const (*wp.as_ptr()).tty).cast());
                    if cmd.is_null() || *cmd == b'\0' {
                        return false;
                    } else {
                        let retval = !libc::strstr(cmd, ss).is_null();
                        free_(cmd);
                        return retval;
                    }
                }
            }
        }

        false
    }
}

unsafe fn window_tree_menu(modedata: NonNull<c_void>, c: *mut client, key: key_code) {
    unsafe {
        let data: NonNull<window_tree_modedata> = modedata.cast();
        let wp: NonNull<window_pane> = NonNull::new_unchecked((*data.as_ptr()).wp);
        if let Some(wme) = NonNull::new(tailq_first(&raw mut (*wp.as_ptr()).modes))
            && (*wme.as_ptr()).data == modedata.as_ptr()
        {
            window_tree_key(wme, c, null_mut(), null_mut(), key, null_mut());
        }
    }
}

unsafe fn window_tree_get_key(
    modedata: NonNull<c_void>,
    itemdata: NonNull<c_void>,
    line: u32,
) -> key_code {
    unsafe {
        let data: NonNull<window_tree_modedata> = modedata.cast();
        let item: NonNull<window_tree_itemdata> = itemdata.cast();
        // struct format_tree *ft;

        let mut s = None;
        let mut wl = None;
        let mut wp = None;

        let ft = format_create(null_mut(), null_mut(), FORMAT_NONE, format_flags::empty());
        window_tree_pull_item(item, &raw mut s, &raw mut wl, &raw mut wp);
        if (*item.as_ptr()).type_ == window_tree_type::WINDOW_TREE_SESSION {
            format_defaults(ft, null_mut(), s, None, None);
        } else if (*item.as_ptr()).type_ == window_tree_type::WINDOW_TREE_WINDOW {
            format_defaults(ft, null_mut(), s, wl, None);
        } else {
            format_defaults(ft, null_mut(), s, wl, wp);
        }
        format_add!(ft, c!("line"), "{line}");

        let expanded = format_expand(ft, (*data.as_ptr()).key_format);
        let key = key_string_lookup_string(expanded);
        free_(expanded);
        format_free(ft);
        key
    }
}

unsafe fn window_tree_init(
    wme: NonNull<window_mode_entry>,
    fs: *mut cmd_find_state,
    args: *mut args,
) -> *mut screen {
    unsafe {
        let wp: *mut window_pane = (*wme.as_ptr()).wp;
        // window_tree_modedata *data;
        // screen *s;
        let mut s = null_mut();

        let data: *mut window_tree_modedata = xcalloc1::<'static, window_tree_modedata>();
        (*wme.as_ptr()).data = data.cast();
        (*data).wp = wp;
        (*data).references = 1;

        if args_has_(args, 's') {
            (*data).type_ = window_tree_type::WINDOW_TREE_SESSION;
        } else if args_has_(args, 'w') {
            (*data).type_ = window_tree_type::WINDOW_TREE_WINDOW;
        } else {
            (*data).type_ = window_tree_type::WINDOW_TREE_PANE;
        }
        memcpy__(&raw mut (*data).fs, fs);

        if args.is_null() || !args_has_(args, 'F') {
            (*data).format = xstrdup_(WINDOW_TREE_DEFAULT_FORMAT).as_ptr();
        } else {
            (*data).format = xstrdup(args_get_(args, 'F')).as_ptr();
        }
        if args.is_null() || !args_has_(args, 'K') {
            (*data).key_format = xstrdup_(WINDOW_TREE_DEFAULT_KEY_FORMAT).as_ptr();
        } else {
            (*data).key_format = xstrdup(args_get_(args, 'K')).as_ptr();
        }
        if args.is_null() || args_count(args) == 0 {
            (*data).command = xstrdup__(WINDOW_TREE_DEFAULT_COMMAND);
        } else {
            (*data).command = xstrdup(args_string(args, 0)).as_ptr();
        }
        (*data).squash_groups = !args_has(args, b'G');

        (*data).data = mode_tree_start(
            wp,
            args,
            Some(window_tree_build),
            Some(window_tree_draw),
            Some(window_tree_search),
            Some(window_tree_menu),
            None,
            Some(window_tree_get_key),
            data.cast(),
            WINDOW_TREE_MENU_ITEMS.as_slice(),
            (&raw mut WINDOW_TREE_SORT_LIST).cast(),
            WINDOW_TREE_SORT_LIST_LEN as u32,
            &raw mut s,
        );
        mode_tree_zoom((*data).data, args);

        mode_tree_build((*data).data);
        mode_tree_draw((*data).data);

        (*data).type_ = window_tree_type::WINDOW_TREE_NONE;

        s
    }
}

unsafe fn window_tree_destroy(data: NonNull<window_tree_modedata>) {
    unsafe {
        let data = data.as_ptr();
        (*data).references -= 1;
        if (*data).references != 0 {
            return;
        }

        for i in 0..(*data).item_size {
            window_tree_free_item(*(*data).item_list.add(i as usize));
        }
        free_((*data).item_list);

        free_((*data).format);
        free_((*data).key_format);
        free_((*data).command);

        free_(data);
    }
}

unsafe fn window_tree_free(wme: NonNull<window_mode_entry>) {
    unsafe {
        if let Some(data) = NonNull::new((*wme.as_ptr()).data.cast::<window_tree_modedata>()) {
            (*data.as_ptr()).dead = 1;
            mode_tree_free((*data.as_ptr()).data);
            window_tree_destroy(data);
        }
    }
}

unsafe fn window_tree_resize(wme: NonNull<window_mode_entry>, sx: u32, sy: u32) {
    unsafe {
        let data: *mut window_tree_modedata = (*wme.as_ptr()).data.cast();
        mode_tree_resize((*data).data, sx, sy);
    }
}

unsafe fn window_tree_update(wme: NonNull<window_mode_entry>) {
    unsafe {
        let data: *mut window_tree_modedata = (*wme.as_ptr()).data.cast();

        mode_tree_build((*data).data);
        mode_tree_draw((*data).data);
        (*(*data).wp).flags |= window_pane_flags::PANE_REDRAW;
    }
}

unsafe fn window_tree_get_target(
    item: NonNull<window_tree_itemdata>,
    fs: *mut cmd_find_state,
) -> *mut u8 {
    unsafe {
        let mut s = None;
        let mut wl = None;
        let mut wp = None;

        window_tree_pull_item(item, &raw mut s, &raw mut wl, &raw mut wp);

        let mut target: *mut u8 = null_mut();
        match (*item.as_ptr()).type_ {
            window_tree_type::WINDOW_TREE_NONE => (),
            window_tree_type::WINDOW_TREE_SESSION => {
                if let Some(s) = s {
                    target = format_nul!("={}:", _s((*s.as_ptr()).name));
                }
            }
            window_tree_type::WINDOW_TREE_WINDOW => {
                if let Some(s) = s
                    && let Some(wl) = wl
                {
                    target = format_nul!("={}:{}.", _s((*s.as_ptr()).name), (*wl.as_ptr()).idx);
                }
            }
            window_tree_type::WINDOW_TREE_PANE => {
                if let Some(s) = s
                    && let Some(wl) = wl
                    && let Some(wp) = wp
                {
                    target = format_nul!(
                        "={}:{}.%{}",
                        _s((*s.as_ptr()).name),
                        (*wl.as_ptr()).idx,
                        (*wp.as_ptr()).id
                    );
                }
            }
        }

        if target.is_null() {
            cmd_find_clear_state(fs, cmd_find_flags::empty());
        } else {
            cmd_find_from_winlink_pane(
                fs,
                transmute_ptr(wl),
                transmute_ptr(wp),
                cmd_find_flags::empty(),
            );
        }

        target
    }
}

unsafe fn window_tree_command_each(
    modedata: NonNull<c_void>,
    itemdata: NonNull<c_void>,
    c: *mut client,
    _key: key_code,
) {
    unsafe {
        let item: NonNull<window_tree_itemdata> = itemdata.cast();
        let mut fs: cmd_find_state = zeroed();

        if let Some(name) = NonNull::new(window_tree_get_target(item, &raw mut fs)) {
            let data: NonNull<window_tree_modedata> = modedata.cast();
            mode_tree_run_command(c, &raw mut fs, (*data.as_ptr()).entered, name.as_ptr());
            free_(name.as_ptr());
        }
    }
}

unsafe fn window_tree_command_done(_: *mut cmdq_item, modedata: *mut c_void) -> cmd_retval {
    unsafe {
        let data: NonNull<window_tree_modedata> = NonNull::new(modedata.cast()).unwrap();

        if (*data.as_ptr()).dead == 0 {
            mode_tree_build((*data.as_ptr()).data);
            mode_tree_draw((*data.as_ptr()).data);
            (*(*data.as_ptr()).wp).flags |= window_pane_flags::PANE_REDRAW;
        }
        window_tree_destroy(data);

        cmd_retval::CMD_RETURN_NORMAL
    }
}

unsafe fn window_tree_command_callback(
    c: *mut client,
    data: NonNull<window_tree_modedata>,
    s: *const u8,
    _done: i32,
) -> i32 {
    unsafe {
        if s.is_null() || *s == b'\0' || (*data.as_ptr()).dead != 0 {
            return 0;
        }

        (*data.as_ptr()).entered = s;
        mode_tree_each_tagged(
            (*data.as_ptr()).data,
            Some(window_tree_command_each),
            c,
            KEYC_NONE,
            1,
        );
        (*data.as_ptr()).entered = null_mut();

        (*data.as_ptr()).references += 1;
        let data = data.as_ptr();
        cmdq_append(
            c,
            cmdq_get_callback!(window_tree_command_done, data.cast()).as_ptr(),
        );

        0
    }
}

unsafe fn window_tree_command_free(modedata: NonNull<window_tree_modedata>) {
    unsafe {
        window_tree_destroy(modedata);
    }
}

unsafe fn window_tree_kill_each(
    _: NonNull<c_void>,
    itemdata: NonNull<c_void>,
    _: *mut client,
    _: key_code,
) {
    unsafe {
        let item: NonNull<window_tree_itemdata> = itemdata.cast();

        let mut s = None;
        let mut wl = None;
        let mut wp = None;
        window_tree_pull_item(item, &raw mut s, &raw mut wl, &raw mut wp);

        match (*item.as_ptr()).type_ {
            window_tree_type::WINDOW_TREE_NONE => (),
            window_tree_type::WINDOW_TREE_SESSION => {
                if let Some(s) = s {
                    server_destroy_session(s.as_ptr());
                    session_destroy(s.as_ptr(), 1, c!("window_tree_kill_each"));
                }
            }
            window_tree_type::WINDOW_TREE_WINDOW => {
                if let Some(wl) = wl {
                    server_kill_window((*wl.as_ptr()).window, 0);
                }
            }
            window_tree_type::WINDOW_TREE_PANE => {
                if let Some(wp) = wp {
                    server_kill_pane(wp.as_ptr());
                }
            }
        }
    }
}

unsafe fn window_tree_kill_current_callback(
    c: *mut client,
    data: NonNull<window_tree_modedata>,
    s: *const u8,
    _: i32,
) -> i32 {
    unsafe {
        let mtd: *mut mode_tree_data = (*data.as_ptr()).data;

        if s.is_null() || *s == b'\0' || (*data.as_ptr()).dead != 0 {
            return 0;
        }
        if !(*s).eq_ignore_ascii_case(&b'y') || *s.add(1) != b'\0' {
            return 0;
        }

        window_tree_kill_each(data.cast(), mode_tree_get_current(mtd), c, KEYC_NONE);
        server_renumber_all();

        (*data.as_ptr()).references += 1;
        cmdq_append(
            c,
            cmdq_get_callback!(window_tree_command_done, data.as_ptr().cast()).as_ptr(),
        );

        0
    }
}

unsafe fn window_tree_kill_tagged_callback(
    c: *mut client,
    data: NonNull<window_tree_modedata>,
    s: *const u8,
    _: i32,
) -> i32 {
    unsafe {
        let mtd: *mut mode_tree_data = (*data.as_ptr()).data;

        if s.is_null() || *s == b'\0' || (*data.as_ptr()).dead != 0 {
            return 0;
        }
        if !(*s).eq_ignore_ascii_case(&b'y') || *s.add(1) != b'\0' {
            return 0;
        }

        mode_tree_each_tagged(mtd, Some(window_tree_kill_each), c, KEYC_NONE, 1);
        server_renumber_all();

        (*data.as_ptr()).references += 1;
        cmdq_append(
            c,
            cmdq_get_callback1(
                "window_tree_command_done",
                Some(window_tree_command_done),
                data.cast().as_ptr(),
            )
            .as_ptr(),
        );

        0
    }
}

unsafe fn window_tree_mouse(
    data: *mut window_tree_modedata,
    key: key_code,
    mut x: u32,
    item: NonNull<window_tree_itemdata>,
) -> key_code {
    unsafe {
        let mut s = None;
        let mut wl = None;
        let mut wp = None;

        if key != keyc::KEYC_MOUSEDOWN1_PANE as u64 {
            return KEYC_NONE;
        }

        if (*data).left != -1 && x <= (*data).left as u32 {
            return '<' as key_code;
        }
        if (*data).right != -1 && x >= (*data).right as u32 {
            return '>' as key_code;
        }

        if (*data).left != -1 {
            x -= (*data).left as u32;
        } else {
            x = x.saturating_sub(1);
        }
        if x == 0 || (*data).end == 0 {
            x = 0;
        } else {
            x /= (*data).each;
            if (*data).start + x >= (*data).end {
                x = (*data).end - 1;
            }
        }

        window_tree_pull_item(item, &raw mut s, &raw mut wl, &raw mut wp);
        if (*item.as_ptr()).type_ == window_tree_type::WINDOW_TREE_SESSION {
            let Some(s) = s else {
                return KEYC_NONE;
            };
            mode_tree_expand_current((*data).data);

            for (loop_, wl_) in rb_foreach(&raw mut (*s.as_ptr()).windows).enumerate() {
                wl = Some(wl_);
                if loop_ as u32 == (*data).start + x {
                    break;
                }
            }
            if let Some(wl) = wl {
                mode_tree_set_current((*data).data, wl.addr().get() as u64);
            }
            return '\r' as key_code;
        }
        if (*item.as_ptr()).type_ == window_tree_type::WINDOW_TREE_WINDOW {
            let Some(wl) = wl else {
                return KEYC_NONE;
            };
            mode_tree_expand_current((*data).data);
            for (loop_, wp_) in
                tailq_foreach::<_, discr_entry>(&raw mut (*(*wl.as_ptr()).window).panes).enumerate()
            {
                wp = Some(wp_);
                if loop_ as u32 == (*data).start + x {
                    break;
                }
            }
            if let Some(wp) = wp {
                mode_tree_set_current((*data).data, wp.addr().get() as u64);
            }
            return '\r' as key_code;
        }
        KEYC_NONE
    }
}

unsafe fn window_tree_key(
    wme: NonNull<window_mode_entry>,
    c: *mut client,
    _: *mut session,
    _: *mut winlink,
    mut key: key_code,
    m: *mut mouse_event,
) {
    unsafe {
        let wp = (*wme.as_ptr()).wp;
        let data = (*wme.as_ptr()).data as *mut window_tree_modedata;

        let mut prompt: *mut u8 = null_mut();

        let mut fs: cmd_find_state = zeroed();
        let fsp = &raw mut (*data).fs;

        let tagged: u32;
        let mut x: u32 = 0;
        let mut y: u32 = 0;
        let mut idx: u32 = 0;

        let mut ns = None;
        let mut nwl = None;
        let mut nwp = None;

        let mut item: NonNull<window_tree_itemdata> = mode_tree_get_current((*data).data).cast();

        let mut finished = mode_tree_key((*data).data, c, &raw mut key, m, &raw mut x, &raw mut y);

        'again: loop {
            let new_item: NonNull<window_tree_itemdata> =
                mode_tree_get_current((*data).data).cast();
            if item != new_item {
                item = new_item;
                (*data).offset = 0;
            }
            if KEYC_IS_MOUSE(key) && !m.is_null() {
                key = window_tree_mouse(data, key, x, item);
                continue 'again;
            }

            match key as u8 {
                b'<' => (*data).offset -= 1,
                b'>' => (*data).offset += 1,
                b'H' => {
                    mode_tree_expand((*data).data, (*fsp).s as u64);
                    mode_tree_expand((*data).data, (*fsp).wl as u64);
                    if mode_tree_set_current((*data).data, (*wme.as_ptr()).wp as u64) == 0 {
                        mode_tree_set_current((*data).data, (*fsp).wl as u64);
                    }
                }
                b'm' => {
                    window_tree_pull_item(item, &raw mut ns, &raw mut nwl, &raw mut nwp);
                    server_set_marked(transmute_ptr(ns), transmute_ptr(nwl), transmute_ptr(nwp));
                    mode_tree_build((*data).data);
                }
                b'M' => {
                    server_clear_marked();
                    mode_tree_build((*data).data);
                }
                b'x' => {
                    window_tree_pull_item(item, &raw mut ns, &raw mut nwl, &raw mut nwp);
                    // TODO there were breaks here which would have broken out
                    match (*item.as_ptr()).type_ {
                        window_tree_type::WINDOW_TREE_NONE => (),
                        window_tree_type::WINDOW_TREE_SESSION => {
                            if let Some(ns) = ns {
                                prompt = format_nul!("Kill session {}? ", _s((*ns.as_ptr()).name));
                            }
                        }
                        window_tree_type::WINDOW_TREE_WINDOW => {
                            if let Some(nwl) = nwl {
                                prompt = format_nul!("Kill window {}? ", (*nwl.as_ptr()).idx);
                            }
                        }
                        window_tree_type::WINDOW_TREE_PANE => {
                            if let Some(nwp) = nwp
                                && window_pane_index(nwp.as_ptr(), &raw mut idx) == 0
                            {
                                prompt = format_nul!("Kill pane {}? ", idx);
                            }
                        }
                    }
                    if prompt.is_null() {
                        break;
                    }
                    (*data).references += 1;
                    status_prompt_set(
                        c,
                        null_mut(),
                        prompt,
                        c!(""),
                        window_tree_kill_current_callback,
                        window_tree_command_free,
                        data,
                        PROMPT_SINGLE | PROMPT_NOFORMAT,
                        prompt_type::PROMPT_TYPE_COMMAND,
                    );
                    free_(prompt);
                }
                b'X' => {
                    tagged = mode_tree_count_tagged((*data).data);
                    if tagged == 0 {
                        break;
                    }
                    prompt = format_nul!("Kill {} tagged? ", tagged);
                    (*data).references += 1;
                    status_prompt_set(
                        c,
                        null_mut(),
                        prompt,
                        c!(""),
                        window_tree_kill_tagged_callback,
                        window_tree_command_free,
                        data,
                        PROMPT_SINGLE | PROMPT_NOFORMAT,
                        prompt_type::PROMPT_TYPE_COMMAND,
                    );
                    free_(prompt);
                }
                b':' => {
                    tagged = mode_tree_count_tagged((*data).data);
                    prompt = if tagged != 0 {
                        format_nul!("({} tagged) ", tagged)
                    } else {
                        format_nul!("(current) ")
                    };
                    (*data).references += 1;
                    status_prompt_set(
                        c,
                        null_mut(),
                        prompt,
                        c!(""),
                        window_tree_command_callback,
                        window_tree_command_free,
                        data,
                        PROMPT_NOFORMAT,
                        prompt_type::PROMPT_TYPE_COMMAND,
                    );
                    free_(prompt);
                }
                b'\r' => {
                    if let Some(name) = NonNull::new(window_tree_get_target(item, &raw mut fs)) {
                        mode_tree_run_command(c, null_mut(), (*data).command, name.as_ptr());
                        free_(name.as_ptr());
                    }
                    finished = 1;
                }
                _ => (),
            }

            if finished != 0 {
                window_pane_reset_mode(wp);
            } else {
                mode_tree_draw((*data).data);
                (*wp).flags |= window_pane_flags::PANE_REDRAW;
            }
            break 'again;
        }
    }
}

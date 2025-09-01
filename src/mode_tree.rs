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
use crate::libc::strstr;
use crate::*;

pub type mode_tree_build_cb = Option<
    unsafe fn(_: NonNull<c_void>, _: *mut mode_tree_sort_criteria, _: *mut u64, _: *const u8),
>;
pub type mode_tree_draw_cb = Option<
    unsafe fn(
        _: *mut c_void,
        _: Option<NonNull<c_void>>,
        _: *mut screen_write_ctx,
        _: c_uint,
        _: c_uint,
    ),
>;
pub type mode_tree_search_cb =
    Option<unsafe fn(_: *mut c_void, _: NonNull<c_void>, _: *const u8) -> bool>;
pub type mode_tree_menu_cb = Option<unsafe fn(_: NonNull<c_void>, _: *mut client, _: key_code)>;
pub type mode_tree_height_cb = Option<unsafe fn(_: *mut c_void, _: c_uint) -> c_uint>;
pub type mode_tree_key_cb =
    Option<unsafe fn(_: NonNull<c_void>, _: NonNull<c_void>, _: c_uint) -> key_code>;
pub type mode_tree_each_cb =
    Option<unsafe fn(_: NonNull<c_void>, _: NonNull<c_void>, _: *mut client, _: key_code)>;

#[repr(C)]
#[derive(Copy, Clone, Eq, PartialEq)]
enum mode_tree_search_dir {
    MODE_TREE_SEARCH_FORWARD,
    MODE_TREE_SEARCH_BACKWARD,
}

pub type mode_tree_list = tailq_head<mode_tree_item>;

#[repr(C)]
pub struct mode_tree_data {
    dead: i32,
    references: u32,
    zoomed: i32,

    wp: *mut window_pane,
    modedata: *mut c_void,
    menu: &'static [menu_item],

    sort_list: &'static [&'static str],
    sort_crit: mode_tree_sort_criteria,

    buildcb: mode_tree_build_cb,
    drawcb: mode_tree_draw_cb,
    searchcb: mode_tree_search_cb,
    menucb: mode_tree_menu_cb,
    heightcb: mode_tree_height_cb,
    keycb: mode_tree_key_cb,

    children: mode_tree_list,
    saved: mode_tree_list,

    line_list: Vec<mode_tree_line>,

    depth: u32,

    width: u32,
    height: u32,

    offset: u32,
    current: u32,

    screen: screen,

    preview: bool,
    search: *mut u8,
    filter: *mut u8,
    no_matches: i32,
    search_dir: mode_tree_search_dir,
}

#[repr(C)]
pub struct mode_tree_item {
    parent: *mut mode_tree_item,
    itemdata: *mut c_void,
    line: u32,

    key: key_code,
    keystr: *mut u8,
    keylen: usize,

    tag: u64,
    name: *mut u8,
    text: *mut u8,

    expanded: bool,
    tagged: i32,

    draw_as_parent: i32,
    no_tag: i32,

    children: mode_tree_list,
    entry: tailq_entry<mode_tree_item>,
}
impl_tailq_entry!(mode_tree_item, entry, tailq_entry<mode_tree_item>);

#[repr(C)]
struct mode_tree_line {
    item: *mut mode_tree_item,
    depth: u32,
    last: i32,
    flat: i32,
}

#[repr(C)]
struct mode_tree_menu {
    data: *mut mode_tree_data,
    c: *mut client,
    line: u32,
}

static MODE_TREE_MENU_ITEMS: [menu_item; 4] = [
    menu_item::new(c"Scroll Left", '<' as u64, null_mut()),
    menu_item::new(c"Scroll Right", '>' as u64, null_mut()),
    menu_item::new(c"", KEYC_NONE, null_mut()),
    menu_item::new(c"Cancel", 'q' as u64, null_mut()),
];

unsafe fn mode_tree_find_item(mtl: *mut mode_tree_list, tag: u64) -> *mut mode_tree_item {
    unsafe {
        for mti in tailq_foreach(mtl).map(NonNull::as_ptr) {
            if (*mti).tag == tag {
                return mti;
            }

            if let Some(child) = NonNull::new(mode_tree_find_item(&raw mut (*mti).children, tag)) {
                return child.as_ptr();
            }
        }
        null_mut()
    }
}

unsafe fn mode_tree_free_item(mti: *mut mode_tree_item) {
    unsafe {
        mode_tree_free_items(&raw mut (*mti).children);

        free_((*mti).name);
        free_((*mti).text);
        free_((*mti).keystr);

        free_(mti);
    }
}

unsafe fn mode_tree_free_items(mtl: *mut mode_tree_list) {
    unsafe {
        for mti in tailq_foreach(mtl).map(NonNull::as_ptr) {
            tailq_remove(mtl, mti);
            mode_tree_free_item(mti);
        }
    }
}

unsafe fn mode_tree_check_selected(mtd: *mut mode_tree_data) {
    unsafe {
        // If the current line would now be off screen reset the offset to the
        // last visible line.
        if (*mtd).current > (*mtd).height - 1 {
            (*mtd).offset = (*mtd).current - (*mtd).height + 1;
        }
    }
}

unsafe fn mode_tree_clear_lines(mtd: *mut mode_tree_data) {
    unsafe {
        (*mtd).line_list = Vec::new();
    }
}

unsafe fn mode_tree_build_lines(mtd: *mut mode_tree_data, mtl: *mut mode_tree_list, depth: u32) {
    unsafe {
        let mut flat = 1;

        (*mtd).depth = depth;
        for mti in tailq_foreach(mtl).map(NonNull::as_ptr) {
            (*mtd).line_list.push(mode_tree_line {
                item: mti,
                depth,
                last: (mti == tailq_last(mtl)) as i32,
                flat: 0,
            });

            (*mti).line = (*mtd).line_list.len() as u32 - 1;
            if !tailq_empty(&raw const (*mti).children) {
                flat = 0;
            }
            if (*mti).expanded {
                mode_tree_build_lines(mtd, &raw mut (*mti).children, depth + 1);
            }

            if let Some(keycb) = (*mtd).keycb {
                (*mti).key = keycb(
                    NonNull::new((*mtd).modedata).unwrap(),
                    NonNull::new((*mti).itemdata).unwrap(),
                    (*mti).line,
                );
                if (*mti).key == KEYC_UNKNOWN {
                    (*mti).key = KEYC_NONE;
                }
            } else if (*mti).line < 10 {
                (*mti).key = (b'0' as u32 + (*mti).line) as u64;
            } else if (*mti).line < 36 {
                (*mti).key = KEYC_META | (b'a' as u32 + (*mti).line - 10) as u64;
            } else {
                (*mti).key = KEYC_NONE;
            }
            if (*mti).key != KEYC_NONE {
                (*mti).keystr = xstrdup(key_string_lookup_key((*mti).key, 0)).as_ptr();
                (*mti).keylen = strlen((*mti).keystr);
            } else {
                (*mti).keystr = null_mut();
                (*mti).keylen = 0;
            }
        }
        for mti in tailq_foreach(mtl).map(NonNull::as_ptr) {
            for line in (*mtd).line_list.iter_mut() {
                if line.item == mti {
                    line.flat = flat;
                }
            }
        }
    }
}

unsafe fn mode_tree_clear_tagged(mtl: *mut mode_tree_list) {
    unsafe {
        for mti in tailq_foreach(mtl).map(NonNull::as_ptr) {
            (*mti).tagged = 0;
            mode_tree_clear_tagged(&raw mut (*mti).children);
        }
    }
}

pub unsafe fn mode_tree_up(mtd: *mut mode_tree_data, wrap: i32) {
    unsafe {
        if (*mtd).current == 0 {
            if wrap != 0 {
                (*mtd).current = (*mtd).line_list.len() as u32 - 1;
                if (*mtd).line_list.len() as u32 >= (*mtd).height {
                    (*mtd).offset = (*mtd).line_list.len() as u32 - (*mtd).height;
                }
            }
        } else {
            (*mtd).current -= 1;
            if (*mtd).current < (*mtd).offset {
                (*mtd).offset -= 1;
            }
        }
    }
}

pub unsafe fn mode_tree_down(mtd: *mut mode_tree_data, wrap: i32) -> i32 {
    unsafe {
        if (*mtd).current == (*mtd).line_list.len() as u32 - 1 {
            if wrap != 0 {
                (*mtd).current = 0;
                (*mtd).offset = 0;
            } else {
                return 0;
            }
        } else {
            (*mtd).current += 1;
            if (*mtd).current > (*mtd).offset + (*mtd).height - 1 {
                (*mtd).offset += 1;
            }
        }

        1
    }
}

pub unsafe fn mode_tree_get_current(mtd: *mut mode_tree_data) -> NonNull<c_void> {
    NonNull::new(unsafe { (*(&mut (*mtd).line_list)[(*mtd).current as usize].item).itemdata })
        .unwrap()
}

pub unsafe fn mode_tree_get_current_name(mtd: *mut mode_tree_data) -> *const u8 {
    unsafe { (*(&(*mtd).line_list)[(*mtd).current as usize].item).name }
}

pub unsafe fn mode_tree_expand_current(mtd: *mut mode_tree_data) {
    unsafe {
        if !(*(&(*mtd).line_list)[(*mtd).current as usize].item).expanded {
            (*(&mut (*mtd).line_list)[(*mtd).current as usize].item).expanded = true;
            mode_tree_build(mtd);
        }
    }
}

pub unsafe fn mode_tree_collapse_current(mtd: *mut mode_tree_data) {
    unsafe {
        if (*(&(*mtd).line_list)[(*mtd).current as usize].item).expanded {
            (*(&mut (*mtd).line_list)[(*mtd).current as usize].item).expanded = false;
            mode_tree_build(mtd);
        }
    }
}

pub unsafe fn mode_tree_get_tag(mtd: &mode_tree_data, tag: u64) -> Option<usize> {
    unsafe {
        mtd.line_list
            .iter()
            .position(|line| (*line.item).tag == tag)
    }
}

pub unsafe fn mode_tree_expand(mtd: *mut mode_tree_data, tag: u64) {
    unsafe {
        let Some(found) = mode_tree_get_tag(&*mtd, tag) else {
            return;
        };
        if !(*(&(*mtd).line_list)[found].item).expanded {
            (*(&mut (*mtd).line_list)[found].item).expanded = true;
            mode_tree_build(mtd);
        }
    }
}

pub unsafe fn mode_tree_set_current(mtd: *mut mode_tree_data, tag: u64) -> i32 {
    unsafe {
        if let Some(found) = mode_tree_get_tag(&*mtd, tag) {
            (*mtd).current = found as u32;
            // TODO does this same problem exist in upstream?
            if (*mtd).current > (*mtd).height.saturating_sub(1) {
                (*mtd).offset = (*mtd).current - (*mtd).height + 1;
            } else {
                (*mtd).offset = 0;
            }
            return 1;
        }
        (*mtd).current = 0;
        (*mtd).offset = 0;
        0
    }
}

pub unsafe fn mode_tree_count_tagged(mtd: *mut mode_tree_data) -> u32 {
    unsafe {
        (*mtd)
            .line_list
            .iter()
            .filter(|line| (*line.item).tagged != 0)
            .count() as u32
    }
}

pub unsafe fn mode_tree_each_tagged(
    mtd: *mut mode_tree_data,
    cb: mode_tree_each_cb,
    c: *mut client,
    key: key_code,
    current: i32,
) {
    unsafe {
        let mut fired = false;
        for line in (*mtd).line_list.iter_mut() {
            let mti = line.item;
            if (*mti).tagged != 0 {
                fired = true;
                cb.unwrap()(
                    NonNull::new((*mtd).modedata).unwrap(),
                    NonNull::new((*mti).itemdata).unwrap(),
                    c,
                    key,
                );
            }
        }
        if !fired && current != 0 {
            let mti = (&(*mtd).line_list)[(*mtd).current as usize].item;
            cb.unwrap()(
                NonNull::new((*mtd).modedata).unwrap(),
                NonNull::new((*mti).itemdata).unwrap(),
                c,
                key,
            );
        }
    }
}

pub unsafe fn mode_tree_start(
    wp: *mut window_pane,
    args: *mut args,
    buildcb: mode_tree_build_cb,
    drawcb: mode_tree_draw_cb,
    searchcb: mode_tree_search_cb,
    menucb: mode_tree_menu_cb,
    heightcb: mode_tree_height_cb,
    keycb: mode_tree_key_cb,
    modedata: *mut c_void,
    menu: &'static [menu_item],
    sort_list: &'static [&'static str],
    s: *mut *mut screen,
) -> *mut mode_tree_data {
    unsafe {
        let mut mtd = Box::new(mode_tree_data {
            references: 1,
            wp,
            modedata,
            menu,
            sort_list,
            preview: !args_has(args, 'N'),

            buildcb,
            drawcb,
            searchcb,
            menucb,
            heightcb,
            keycb,
            dead: 0,
            zoomed: 0,
            sort_crit: Default::default(),
            children: zeroed(),
            saved: zeroed(),
            line_list: Default::default(),
            depth: Default::default(),
            width: Default::default(),
            height: Default::default(),
            offset: Default::default(),
            current: Default::default(),
            screen: zeroed(),
            search: Default::default(),
            filter: if args_has(args, 'f') {
                xstrdup(args_get_(args, 'f')).as_ptr()
            } else {
                null_mut()
            },
            no_matches: Default::default(),
            search_dir: zeroed(),
        });

        let sort = args_get_(args, 'O');
        if !sort.is_null()
            && let Some(pos) = mtd.sort_list.iter().position(|e| strcaseeq_(sort, e))
        {
            mtd.sort_crit.field = pos as u32;
        }
        mtd.sort_crit.reversed = args_has(args, 'r');

        tailq_init(&mut mtd.children);

        *s = &raw mut mtd.screen;
        screen_init(
            *s,
            screen_size_x(&raw mut (*wp).base),
            screen_size_y(&raw mut (*wp).base),
            0,
        );
        (*(*s)).mode &= !mode_flag::MODE_CURSOR;

        Box::leak(mtd)
    }
}

pub unsafe fn mode_tree_zoom(mtd: *mut mode_tree_data, args: *mut args) {
    unsafe {
        let wp: *mut window_pane = (*mtd).wp;

        if args_has(args, 'Z') {
            (*mtd).zoomed = ((*(*wp).window).flags & window_flag::ZOOMED).bits();
            if (*mtd).zoomed == 0 && window_zoom(wp) == 0 {
                server_redraw_window((*wp).window);
            }
        } else {
            (*mtd).zoomed = -1;
        }
    }
}

pub unsafe fn mode_tree_set_height(mtd: *mut mode_tree_data) {
    unsafe {
        let s: *mut screen = &raw mut (*mtd).screen;

        if let Some(heightcb) = (*mtd).heightcb {
            let height = heightcb(mtd.cast(), screen_size_y(s));
            if height < screen_size_y(s) {
                (*mtd).height = screen_size_y(s) - height;
            }
        } else {
            (*mtd).height = (screen_size_y(s) / 3) * 2;
            if (*mtd).height > (*mtd).line_list.len() as u32 {
                (*mtd).height = screen_size_y(s) / 2;
            }
        }
        if (*mtd).height < 10 {
            (*mtd).height = screen_size_y(s);
        }
        if screen_size_y(s) - (*mtd).height < 2 {
            (*mtd).height = screen_size_y(s);
        }
    }
}

pub unsafe fn mode_tree_build(mtd: *mut mode_tree_data) {
    unsafe {
        let s = &raw mut (*mtd).screen;

        let mut tag = if !(*mtd).line_list.is_empty() {
            (*(&(*mtd).line_list)[(*mtd).current as usize].item).tag
        } else {
            u64::MAX
        };

        tailq_concat(&raw mut (*mtd).saved, &raw mut (*mtd).children);
        tailq_init(&raw mut (*mtd).children);

        (*mtd).buildcb.unwrap()(
            NonNull::new((*mtd).modedata).unwrap(),
            &raw mut (*mtd).sort_crit,
            &raw mut tag,
            (*mtd).filter,
        );
        (*mtd).no_matches = tailq_empty(&raw mut (*mtd).children) as i32;
        if (*mtd).no_matches != 0 {
            (*mtd).buildcb.unwrap()(
                NonNull::new((*mtd).modedata).unwrap(),
                &raw mut (*mtd).sort_crit,
                &raw mut tag,
                null_mut(),
            );
        }

        mode_tree_free_items(&raw mut (*mtd).saved);
        tailq_init(&raw mut (*mtd).saved);

        mode_tree_clear_lines(mtd);
        mode_tree_build_lines(mtd, &raw mut (*mtd).children, 0);

        if !(*mtd).line_list.is_empty() && tag == u64::MAX {
            tag = (*(&(*mtd).line_list)[(*mtd).current as usize].item).tag;
        }
        mode_tree_set_current(mtd, tag);

        (*mtd).width = screen_size_x(s);
        if (*mtd).preview {
            mode_tree_set_height(mtd);
        } else {
            (*mtd).height = screen_size_y(s);
        }
        mode_tree_check_selected(mtd);
    }
}

pub unsafe fn mode_tree_remove_ref(mtd: *mut mode_tree_data) {
    unsafe {
        (*mtd).references -= 1;
        if (*mtd).references == 0 {
            free_(mtd);
        }
    }
}

pub unsafe fn mode_tree_free(mtd: *mut mode_tree_data) {
    unsafe {
        let wp = (*mtd).wp;

        if (*mtd).zoomed == 0 {
            server_unzoom_window((*wp).window);
        }

        mode_tree_free_items(&raw mut (*mtd).children);
        mode_tree_clear_lines(mtd);
        screen_free(&raw mut (*mtd).screen);

        free_((*mtd).search);
        free_((*mtd).filter);

        (*mtd).dead = 1;
        mode_tree_remove_ref(mtd);
    }
}

pub unsafe fn mode_tree_resize(mtd: *mut mode_tree_data, sx: u32, sy: u32) {
    unsafe {
        let s: *mut screen = &raw mut (*mtd).screen;

        screen_resize(s, sx, sy, 0);

        mode_tree_build(mtd);
        mode_tree_draw(&mut *mtd);

        (*(*mtd).wp).flags |= window_pane_flags::PANE_REDRAW;
    }
}

pub unsafe fn mode_tree_add(
    mtd: *mut mode_tree_data,
    parent: *mut mode_tree_item,
    itemdata: *mut c_void,
    tag: u64,
    name: *const u8,
    text: *const u8,
    expanded: Option<bool>,
) -> *mut mode_tree_item {
    unsafe {
        // log_debug("%s: %llu, %s %s", __func__, (unsigned long long)tag, name, (text == NULL ? "" : text));

        let mti: *mut mode_tree_item = xcalloc1::<mode_tree_item>() as *mut mode_tree_item;
        (*mti).parent = parent;
        (*mti).itemdata = itemdata;

        (*mti).tag = tag;
        (*mti).name = xstrdup(name).as_ptr();
        if !text.is_null() {
            (*mti).text = xstrdup(text).as_ptr();
        }

        let saved = mode_tree_find_item(&raw mut (*mtd).saved, tag);
        if !saved.is_null() {
            if parent.is_null() || (*parent).expanded {
                (*mti).tagged = (*saved).tagged;
            }
            (*mti).expanded = (*saved).expanded;
        } else {
            (*mti).expanded = expanded.unwrap_or(true);
        }

        tailq_init(&raw mut (*mti).children);

        if !parent.is_null() {
            tailq_insert_tail(&raw mut (*parent).children, mti);
        } else {
            tailq_insert_tail(&raw mut (*mtd).children, mti);
        }

        mti
    }
}

pub unsafe fn mode_tree_draw_as_parent(mti: *mut mode_tree_item) {
    unsafe {
        (*mti).draw_as_parent = 1;
    }
}

pub unsafe fn mode_tree_no_tag(mti: *mut mode_tree_item) {
    unsafe {
        (*mti).no_tag = 1;
    }
}

pub unsafe fn mode_tree_remove(mtd: *mut mode_tree_data, mti: *mut mode_tree_item) {
    unsafe {
        let parent: *mut mode_tree_item = (*mti).parent;

        if !parent.is_null() {
            tailq_remove(&raw mut (*parent).children, mti);
        } else {
            tailq_remove(&raw mut (*mtd).children, mti);
        }
        mode_tree_free_item(mti);
    }
}

pub unsafe fn mode_tree_draw(mtd: &mut mode_tree_data) {
    unsafe {
        let wp = mtd.wp;
        let s = &raw mut mtd.screen;
        let oo = (*(*wp).window).options;
        let mut ctx: screen_write_ctx = zeroed();

        let mut gc0: grid_cell = zeroed();
        let mut gc: grid_cell = zeroed();

        'done: {
            if mtd.line_list.is_empty() {
                return;
            }

            memcpy__(&raw mut gc0, &raw const GRID_DEFAULT_CELL);
            memcpy__(&raw mut gc, &raw const GRID_DEFAULT_CELL);
            style_apply(&raw mut gc, oo, c!("mode-style"), null_mut());

            let w = mtd.width;
            let h = mtd.height;

            screen_write_start(&raw mut ctx, s);
            screen_write_clearscreen(&raw mut ctx, 8);

            let mut keylen: i32 = 0;
            for line in mtd.line_list.iter() {
                let mti = line.item;
                if (*mti).key == KEYC_NONE {
                    continue;
                }
                if (*mti).keylen as i32 + 3 > keylen {
                    keylen = (*mti).keylen as i32 + 3;
                }
            }

            for i in 0..mtd.line_list.len() {
                // line = &mtd.line_list[i] // but we don't want to borrow
                if (i as u32) < mtd.offset {
                    continue;
                }
                if i as u32 > mtd.offset + h - 1 {
                    break;
                }
                let mti = mtd.line_list[i].item;

                screen_write_cursormove(&raw mut ctx, 0, i as i32 - mtd.offset as i32, 0);

                let pad = keylen - 2 - (*mti).keylen as i32;
                let key = if (*mti).key != KEYC_NONE {
                    format_nul!("({0}){2:>1$}", _s((*mti).keystr), pad as usize, "")
                } else {
                    xstrdup_(c"").as_ptr()
                };

                let symbol = if mtd.line_list[i].flat != 0 {
                    c!("")
                } else if tailq_empty(&raw const (*mti).children) {
                    c!("  ")
                } else if (*mti).expanded {
                    c!("- ")
                } else {
                    c!("+ ")
                };

                let start: *mut u8;
                if mtd.line_list[i].depth == 0 {
                    start = xstrdup(symbol).as_ptr();
                } else {
                    let size = (4 * mtd.line_list[i].depth as usize) + 32;

                    start = xcalloc(1, size).as_ptr().cast();
                    for _ in 1..mtd.line_list[i].depth {
                        if !(*mti).parent.is_null()
                            && mtd.line_list[(*(*mti).parent).line as usize].last != 0
                        {
                            strlcat(start, c!("    "), size);
                        } else {
                            strlcat(start, c!("\x01x\x01   "), size);
                        }
                    }
                    if mtd.line_list[i].last != 0 {
                        strlcat(start, c!("\x01mq\x01> "), size);
                    } else {
                        strlcat(start, c!("\x01tq\x01> "), size);
                    }
                    strlcat(start, symbol, size);
                }

                let tag = if (*mti).tagged != 0 { c!("*") } else { c!("") };
                let text = format_nul!(
                    "{1:<0$}{2}{3}{4}{5}",
                    keylen as usize,
                    _s(key),
                    _s(start),
                    _s((*mti).name),
                    _s(tag),
                    if !(*mti).text.is_null() { ": " } else { "" },
                );
                let mut width = utf8_cstrwidth(text);
                if width > w {
                    width = w;
                }
                free_(start);

                if (*mti).tagged != 0 {
                    gc.attr ^= grid_attr::GRID_ATTR_BRIGHT;
                    gc0.attr ^= grid_attr::GRID_ATTR_BRIGHT;
                }

                if i as u32 != mtd.current {
                    screen_write_clearendofline(&raw mut ctx, 8);
                    screen_write_nputs!(&raw mut ctx, w as isize, &raw mut gc0, "{}", _s(text),);
                    if !(*mti).text.is_null() {
                        format_draw(
                            &raw mut ctx,
                            &raw mut gc0,
                            w - width,
                            (*mti).text,
                            null_mut(),
                            0,
                        );
                    }
                } else {
                    screen_write_clearendofline(&raw mut ctx, gc.bg as u32);
                    screen_write_nputs!(&raw mut ctx, w as isize, &raw mut gc, "{}", _s(text));
                    if !(*mti).text.is_null() {
                        format_draw(
                            &raw mut ctx,
                            &raw mut gc,
                            w - width,
                            (*mti).text,
                            null_mut(),
                            0,
                        );
                    }
                }
                free_(text);
                free_(key);

                if (*mti).tagged != 0 {
                    gc.attr ^= grid_attr::GRID_ATTR_BRIGHT;
                    gc0.attr ^= grid_attr::GRID_ATTR_BRIGHT;
                }
            }

            let sy = screen_size_y(s);
            if !mtd.preview || sy <= 4 || h <= 4 || sy - h <= 4 || w <= 4 {
                break 'done;
            }

            let line = &mtd.line_list[mtd.current as usize];
            let mut mti = line.item;
            if (*mti).draw_as_parent != 0 {
                mti = (*mti).parent;
            }

            screen_write_cursormove(&raw mut ctx, 0, h as i32, 0);
            screen_write_box(
                &raw mut ctx,
                w,
                sy - h,
                box_lines::BOX_LINES_DEFAULT,
                null(),
                null(),
            );

            let text = if !mtd.sort_list.is_empty() {
                format_nul!(
                    " {} (sort: {}{})",
                    _s((*mti).name),
                    mtd.sort_list[mtd.sort_crit.field as usize],
                    if mtd.sort_crit.reversed {
                        ", reversed"
                    } else {
                        ""
                    },
                )
            } else {
                format_nul!(" {}", _s((*mti).name))
            };
            if w - 2 >= strlen(text) as u32 {
                screen_write_cursormove(&raw mut ctx, 1, h as i32, 0);
                screen_write_puts!(&raw mut ctx, &raw mut gc0, "{}", _s(text));

                let n = if mtd.no_matches != 0 {
                    "no matches".len()
                } else {
                    "active".len()
                };

                if !mtd.filter.is_null() && w as usize - 2 >= strlen(text) + 10 + n + 2 {
                    screen_write_puts!(&raw mut ctx, &raw mut gc0, " (filter: ");
                    if mtd.no_matches != 0 {
                        screen_write_puts!(&raw mut ctx, &raw mut gc, "no matches");
                    } else {
                        screen_write_puts!(&raw mut ctx, &raw mut gc0, "active");
                    }
                    screen_write_puts!(&raw mut ctx, &raw mut gc0, ") ");
                } else {
                    screen_write_puts!(&raw mut ctx, &raw mut gc0, " ");
                }
            }
            free_(text);

            let box_x = w - 4;
            let box_y = sy - h - 2;

            if box_x != 0 && box_y != 0 {
                screen_write_cursormove(&raw mut ctx, 2, h as i32 + 1, 0);
                mtd.drawcb.unwrap()(
                    mtd.modedata,
                    NonNull::new((*mti).itemdata),
                    &raw mut ctx,
                    box_x,
                    box_y,
                );
            }
        }
        // done:
        screen_write_cursormove(&raw mut ctx, 0, mtd.current as i32 - mtd.offset as i32, 0);
        screen_write_stop(&raw mut ctx);
    }
}

pub unsafe fn mode_tree_search_backward(mtd: *mut mode_tree_data) -> *mut mode_tree_item {
    unsafe {
        if (*mtd).search.is_null() {
            return null_mut();
        }

        let last = (&(*mtd).line_list)[(*mtd).current as usize].item;
        let mut mti = last;

        loop {
            let mut prev = tailq_prev(mti);
            if !prev.is_null() {
                // Point to the last child in the previous subtree.
                while !tailq_empty(&raw mut (*prev).children) {
                    prev = tailq_last(&raw mut (*prev).children);
                }
                mti = prev;
            } else {
                // If prev is NULL, jump to the parent.
                mti = (*mti).parent;
            }

            if mti.is_null() {
                // Point to the last child in the last root subtree.
                prev = tailq_last(&raw mut (*mtd).children);
                while !tailq_empty(&raw mut (*prev).children) {
                    prev = tailq_last(&raw mut (*prev).children);
                }
                mti = prev;
            }
            if mti == last {
                break;
            }

            let Some(searchcb) = (*mtd).searchcb else {
                if !strstr((*mti).name, (*mtd).search).is_null() {
                    return mti;
                }
                continue;
            };
            if searchcb(
                (*mtd).modedata,
                NonNull::new((*mti).itemdata).unwrap(),
                (*mtd).search,
            ) {
                return mti;
            }
        }
        null_mut()
    }
}

pub unsafe fn mode_tree_search_forward(mtd: *mut mode_tree_data) -> *mut mode_tree_item {
    unsafe {
        if (*mtd).search.is_null() {
            return null_mut();
        }

        let last = (&(*mtd).line_list)[(*mtd).current as usize].item;
        let mut mti = last;
        loop {
            if !tailq_empty(&raw mut (*mti).children) {
                mti = tailq_first(&raw mut (*mti).children);
            } else if let Some(next) = NonNull::new(tailq_next(mti)) {
                mti = next.as_ptr();
            } else {
                loop {
                    mti = (*mti).parent;
                    if mti.is_null() {
                        break;
                    }

                    if let Some(next) = NonNull::new(tailq_next(mti)) {
                        mti = next.as_ptr();
                        break;
                    }
                }
            }
            if mti.is_null() {
                mti = tailq_first(&raw mut (*mtd).children);
            }
            if mti == last {
                break;
            }

            let Some(searchcb) = (*mtd).searchcb else {
                if !strstr((*mti).name, (*mtd).search).is_null() {
                    return mti;
                }
                continue;
            };
            if searchcb(
                (*mtd).modedata,
                NonNull::new((*mti).itemdata).unwrap(),
                (*mtd).search,
            ) {
                return mti;
            }
        }
        null_mut()
    }
}

pub unsafe fn mode_tree_search_set(mtd: *mut mode_tree_data) {
    unsafe {
        let mti = match (*mtd).search_dir {
            mode_tree_search_dir::MODE_TREE_SEARCH_FORWARD => mode_tree_search_forward(mtd),
            mode_tree_search_dir::MODE_TREE_SEARCH_BACKWARD => mode_tree_search_backward(mtd),
        };
        if mti.is_null() {
            return;
        }
        let tag = (*mti).tag;

        let mut loop_ = (*mti).parent;
        while !loop_.is_null() {
            (*loop_).expanded = true;
            loop_ = (*loop_).parent;
        }

        mode_tree_build(mtd);
        mode_tree_set_current(mtd, tag);
        mode_tree_draw(&mut *mtd);
        (*(*mtd).wp).flags |= window_pane_flags::PANE_REDRAW;
    }
}

pub unsafe fn mode_tree_search_callback(
    _c: *mut client,
    mtd: NonNull<mode_tree_data>,
    s: *const u8,
    _done: i32,
) -> i32 {
    unsafe {
        let mtd: *mut mode_tree_data = mtd.as_ptr();

        if (*mtd).dead != 0 {
            return 0;
        }

        free_((*mtd).search);
        if s.is_null() || *s == b'\0' {
            (*mtd).search = null_mut();
            return 0;
        }
        (*mtd).search = xstrdup(s).as_ptr();
        mode_tree_search_set(mtd);

        0
    }
}

pub unsafe fn mode_tree_search_free(data: NonNull<mode_tree_data>) {
    unsafe {
        mode_tree_remove_ref(data.cast().as_ptr());
    }
}

pub unsafe fn mode_tree_filter_callback(
    _c: *mut client,
    data: NonNull<mode_tree_data>,
    s: *const u8,
    _done: i32,
) -> i32 {
    unsafe {
        let mtd: *mut mode_tree_data = data.as_ptr();

        if (*mtd).dead != 0 {
            return 0;
        }

        if !(*mtd).filter.is_null() {
            free_((*mtd).filter);
        }
        if s.is_null() || *s == b'\0' {
            (*mtd).filter = null_mut();
        } else {
            (*mtd).filter = xstrdup(s).as_ptr();
        }

        mode_tree_build(mtd);
        mode_tree_draw(&mut *mtd);
        (*(*mtd).wp).flags |= window_pane_flags::PANE_REDRAW;

        0
    }
}

pub unsafe fn mode_tree_filter_free(data: NonNull<mode_tree_data>) {
    unsafe {
        mode_tree_remove_ref(data.cast().as_ptr());
    }
}

pub unsafe fn mode_tree_menu_callback(
    _menu: *mut menu,
    _idx: u32,
    key: key_code,
    data: *mut c_void,
) {
    unsafe {
        let mtm: *mut mode_tree_menu = data.cast();
        let mtd: *mut mode_tree_data = (*mtm).data.cast();

        'out: {
            if (*mtd).dead != 0 || key == KEYC_NONE {
                break 'out;
            }

            if (*mtm).line >= (*mtd).line_list.len() as u32 {
                break 'out;
            }
            (*mtd).current = (*mtm).line;
            (*mtd).menucb.unwrap()(NonNull::new((*mtd).modedata).unwrap(), (*mtm).c, key);
        }
        // out:
        mode_tree_remove_ref(mtd);
        free_(mtm);
    }
}

pub unsafe fn mode_tree_display_menu(
    mtd: *mut mode_tree_data,
    c: *mut client,
    mut x: u32,
    y: u32,
    outside: i32,
) {
    unsafe {
        let line = if (*mtd).offset + y > (*mtd).line_list.len() as u32 - 1 {
            (*mtd).current
        } else {
            (*mtd).offset + y
        };
        let mti = (&(*mtd).line_list)[line as usize].item;

        let (items, title) = if outside == 0 {
            (
                (*mtd).menu,
                format_nul!("#[align=centre]{}", _s((*mti).name)),
            )
        } else {
            (MODE_TREE_MENU_ITEMS.as_slice(), xstrdup_(c"").as_ptr())
        };
        let menu = menu_create(title);
        menu_add_items(menu, items, null_mut(), c, null_mut());
        free_(title);

        let mtm = xmalloc_::<mode_tree_menu>().as_ptr();
        (*mtm).data = mtd;
        (*mtm).c = c;
        (*mtm).line = line;
        (*mtd).references += 1;

        if x >= ((*menu).width + 4) / 2 {
            x -= ((*menu).width + 4) / 2;
        } else {
            x = 0;
        }
        if menu_display(
            menu,
            menu_flags::empty(),
            0,
            null_mut(),
            x,
            y,
            c,
            box_lines::BOX_LINES_DEFAULT,
            null_mut(),
            null_mut(),
            null_mut(),
            null_mut(),
            Some(mode_tree_menu_callback),
            mtm.cast(),
        ) != 0
        {
            menu_free(menu);
        }
    }
}

pub unsafe fn mode_tree_key(
    mtd: *mut mode_tree_data,
    c: *mut client,
    key: *mut key_code,
    m: *mut mouse_event,
    xp: *mut u32,
    yp: *mut u32,
) -> i32 {
    unsafe {
        let mut x: u32 = 0;
        let mut y: u32 = 0;

        if KEYC_IS_MOUSE(*key) && !m.is_null() {
            if cmd_mouse_at((*mtd).wp, m, &raw mut x, &raw mut y, 0) != 0 {
                *key = KEYC_NONE;
                return 0;
            }
            if !xp.is_null() {
                *xp = x;
            }
            if !yp.is_null() {
                *yp = y;
            }
            if x > (*mtd).width || y > (*mtd).height {
                if *key == keyc::KEYC_MOUSEDOWN3_PANE as u64 {
                    mode_tree_display_menu(mtd, c, x, y, 1);
                }
                if !(*mtd).preview {
                    *key = KEYC_NONE;
                }
                return 0;
            }
            if (*mtd).offset + y < (*mtd).line_list.len() as u32 {
                if *key == keyc::KEYC_MOUSEDOWN1_PANE as u64
                    || *key == keyc::KEYC_MOUSEDOWN3_PANE as u64
                    || *key == keyc::KEYC_DOUBLECLICK1_PANE as u64
                {
                    (*mtd).current = (*mtd).offset + y;
                }
                if *key == keyc::KEYC_DOUBLECLICK1_PANE as u64 {
                    *key = b'\r' as u64;
                } else {
                    if *key == keyc::KEYC_MOUSEDOWN3_PANE as u64 {
                        mode_tree_display_menu(mtd, c, x, y, 0);
                    }
                    *key = KEYC_NONE;
                }
            } else {
                if *key == keyc::KEYC_MOUSEDOWN3_PANE as u64 {
                    mode_tree_display_menu(mtd, c, x, y, 0);
                }
                *key = KEYC_NONE;
            }
            return 0;
        }

        let line = &((&(*mtd).line_list)[(*mtd).current as usize]);
        let mut current = line.item;

        if let Some(choice) = (*mtd)
            .line_list
            .iter()
            .position(|line| *key == (*line.item).key)
        {
            if (choice as u32) > (*mtd).line_list.len() as u32 - 1 {
                *key = KEYC_NONE;
                return 0;
            }
            (*mtd).current = choice as u32;
            *key = b'\r' as u64;
            return 0;
        }

        mod code {
            use super::*;

            pub const Q: u64 = 'q' as u64;
            pub const ESC: u64 = '\x1b' as u64;
            pub const G_CTRL: u64 = 'g' as u64 | KEYC_CTRL;

            pub const K: u64 = 'k' as u64;
            pub const P_CTRL: u64 = 'p' as u64 | KEYC_CTRL;

            pub const J: u64 = 'j' as u64;

            pub const F: u64 = 'f' as u64;
            pub const V: u64 = 'v' as u64;

            pub const N: u64 = 'n' as u64;
            pub const N_UPPER: u64 = 'N' as u64;
            pub const N_CTRL: u64 = 'n' as u64 | KEYC_CTRL;

            pub const B_CTRL: u64 = 'b' as u64 | KEYC_CTRL;

            pub const F_CTRL: u64 = 'f' as u64 | KEYC_CTRL;

            pub const G: u64 = 'g' as u64;
            pub const G_UPPER: u64 = 'G' as u64;

            pub const T: u64 = 't' as u64;
            pub const T_CTRL: u64 = 't' as u64 | KEYC_CTRL;
            pub const T_UPPER: u64 = 'T' as u64;

            pub const O_UPPER: u64 = 'O' as u64;
            pub const R: u64 = 'r' as u64;

            pub const L: u64 = 'l' as u64;
            pub const H: u64 = 'h' as u64;
            pub const MINUS: u64 = '-' as u64;
            pub const PLUS: u64 = '+' as u64;

            pub const MINUS_META: u64 = '-' as u64 | KEYC_META;
            pub const PLUS_META: u64 = '+' as u64 | KEYC_META;

            pub const QUESTION_MARK: u64 = '?' as u64;
            pub const SLASH: u64 = '/' as u64;
            pub const S_CTRL: u64 = 's' as u64 | KEYC_CTRL;

            pub const KEYC_UP: u64 = keyc::KEYC_UP as u64;
            pub const KEYC_DOWN: u64 = keyc::KEYC_DOWN as u64;

            pub const KEYC_WHEELUP_PANE: u64 = keyc::KEYC_WHEELUP_PANE as u64;
            pub const KEYC_WHEELDOWN_PANE: u64 = keyc::KEYC_WHEELDOWN_PANE as u64;

            pub const KEYC_PPAGE: u64 = keyc::KEYC_PPAGE as u64;
            pub const KEYC_NPAGE: u64 = keyc::KEYC_NPAGE as u64;

            pub const KEYC_HOME: u64 = keyc::KEYC_HOME as u64;
            pub const KEYC_END: u64 = keyc::KEYC_END as u64;
            pub const KEYC_LEFT: u64 = keyc::KEYC_LEFT as u64;
            pub const KEYC_RIGHT: u64 = keyc::KEYC_RIGHT as u64;
        }

        match *key {
            code::Q | code::ESC | code::G_CTRL => return 1,

            code::KEYC_UP | code::K | code::KEYC_WHEELUP_PANE | code::P_CTRL => {
                mode_tree_up(mtd, 1);
            }

            code::KEYC_DOWN | code::J | code::KEYC_WHEELDOWN_PANE | code::N_CTRL => {
                mode_tree_down(mtd, 1);
            }

            code::KEYC_PPAGE | code::B_CTRL => {
                for _ in 0..(*mtd).height {
                    if (*mtd).current == 0 {
                        break;
                    }
                    mode_tree_up(mtd, 1);
                }
            }
            code::KEYC_NPAGE | code::F_CTRL => {
                for _ in 0..(*mtd).height {
                    if (*mtd).current == (*mtd).line_list.len() as u32 - 1 {
                        break;
                    }
                    mode_tree_down(mtd, 1);
                }
            }
            code::G | code::KEYC_HOME => {
                (*mtd).current = 0;
                (*mtd).offset = 0;
            }
            code::G_UPPER | code::KEYC_END => {
                (*mtd).current = (*mtd).line_list.len() as u32 - 1;
                if (*mtd).current > (*mtd).height - 1 {
                    (*mtd).offset = (*mtd).current - (*mtd).height + 1;
                } else {
                    (*mtd).offset = 0;
                }
            }
            code::T => {
                // Do not allow parents and children to both be tagged: untag
                // all parents and children of current.
                if (*current).no_tag == 0 {
                    if (*current).tagged == 0 {
                        let mut parent = (*current).parent;
                        while !parent.is_null() {
                            (*parent).tagged = 0;
                            parent = (*parent).parent;
                        }
                        mode_tree_clear_tagged(&raw mut (*current).children);
                        (*current).tagged = 1;
                    } else {
                        (*current).tagged = 0;
                    }
                    if !m.is_null() {
                        mode_tree_down(mtd, 0);
                    }
                }
            }
            code::T_UPPER => {
                for line in (*mtd).line_list.iter_mut() {
                    (*line.item).tagged = 0;
                }
            }
            code::T_CTRL => {
                for line in (*mtd).line_list.iter_mut() {
                    if ((*line.item).parent.is_null() && (*line.item).no_tag == 0)
                        || (!(*line.item).parent.is_null() && (*(*line.item).parent).no_tag != 0)
                    {
                        (*line.item).tagged = 1;
                    } else {
                        (*line.item).tagged = 0;
                    }
                }
            }
            #[expect(clippy::needless_borrow, reason = "false positive")]
            code::O_UPPER => {
                (*mtd).sort_crit.field += 1;
                if (*mtd).sort_crit.field >= (&(*mtd).sort_list).len() as u32 {
                    (*mtd).sort_crit.field = 0;
                }
                mode_tree_build(mtd);
            }
            code::R => {
                (*mtd).sort_crit.reversed = !(*mtd).sort_crit.reversed;
                mode_tree_build(mtd);
            }
            code::KEYC_LEFT | code::H | code::MINUS => {
                if line.flat != 0 || !(*current).expanded {
                    current = (*current).parent;
                }
                if current.is_null() {
                    mode_tree_up(mtd, 0);
                } else {
                    (*current).expanded = false;
                    (*mtd).current = (*current).line;
                    mode_tree_build(mtd);
                }
            }
            code::KEYC_RIGHT | code::L | code::PLUS => {
                if line.flat != 0 || (*current).expanded {
                    mode_tree_down(mtd, 0);
                } else if line.flat == 0 {
                    (*current).expanded = true;
                    mode_tree_build(mtd);
                }
            }
            code::MINUS_META => {
                for mti in tailq_foreach(&raw mut (*mtd).children).map(NonNull::as_ptr) {
                    (*mti).expanded = false;
                }
                mode_tree_build(mtd);
            }
            code::PLUS_META => {
                for mti in tailq_foreach(&raw mut (*mtd).children).map(NonNull::as_ptr) {
                    (*mti).expanded = true;
                }
                mode_tree_build(mtd);
            }
            code::QUESTION_MARK | code::SLASH | code::S_CTRL => {
                (*mtd).references += 1;
                status_prompt_set(
                    c,
                    null_mut(),
                    c!("(search) "),
                    c!(""),
                    mode_tree_search_callback,
                    mode_tree_search_free,
                    mtd,
                    PROMPT_NOFORMAT,
                    prompt_type::PROMPT_TYPE_SEARCH,
                );
            }
            code::N => {
                (*mtd).search_dir = mode_tree_search_dir::MODE_TREE_SEARCH_FORWARD;
                mode_tree_search_set(mtd);
            }
            code::N_UPPER => {
                (*mtd).search_dir = mode_tree_search_dir::MODE_TREE_SEARCH_BACKWARD;
                mode_tree_search_set(mtd);
            }
            code::F => {
                (*mtd).references += 1;
                status_prompt_set(
                    c,
                    null_mut(),
                    c!("(filter) "),
                    (*mtd).filter,
                    mode_tree_filter_callback,
                    mode_tree_filter_free,
                    mtd,
                    PROMPT_NOFORMAT,
                    prompt_type::PROMPT_TYPE_SEARCH,
                );
            }
            code::V => {
                (*mtd).preview = !(*mtd).preview;
                mode_tree_build(mtd);
                if (*mtd).preview {
                    mode_tree_check_selected(mtd);
                }
            }
            _ => (),
        };
        0
    }
}

pub unsafe fn mode_tree_run_command(
    c: *mut client,
    fs: *mut cmd_find_state,
    template: *const u8,
    name: *const u8,
) {
    unsafe {
        let mut error: *mut u8 = null_mut();

        let command = cmd_template_replace(template, name, 1);
        if !command.is_null() && *command != b'\0' {
            let state = cmdq_new_state(fs, null_mut(), cmdq_state_flags::empty());
            let status = cmd_parse_and_append(cstr_to_str(command), None, c, state, &raw mut error);
            if status == cmd_parse_status::CMD_PARSE_ERROR {
                if !c.is_null() {
                    *error = (*error).to_ascii_uppercase();
                    status_message_set!(c, -1, 1, false, "{}", _s(error));
                }
                free_(error);
            }
            cmdq_free_state(state);
        }
        free_(command);
    }
}

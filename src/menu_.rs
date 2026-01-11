// Copyright (c) 2019 Nicholas Marriott <nicholas.marriott@gmail.com>
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
use crate::options_::*;

bitflags::bitflags! {
    #[repr(transparent)]
    #[derive(Copy, Clone, Eq, PartialEq)]
    pub struct menu_flags: i32 {
        const MENU_NOMOUSE = 0x1;
        const MENU_TAB = 0x2;
        const MENU_STAYOPEN = 0x4;
    }
}
pub const MENU_NOMOUSE: menu_flags = menu_flags::MENU_NOMOUSE;
pub const MENU_TAB: menu_flags = menu_flags::MENU_TAB;
pub const MENU_STAYOPEN: menu_flags = menu_flags::MENU_STAYOPEN;

#[repr(C)]
pub struct menu_data {
    pub item: *mut cmdq_item,
    pub flags: menu_flags,

    pub style: grid_cell,
    pub border_style: grid_cell,
    pub selected_style: grid_cell,
    pub border_lines: box_lines,

    pub fs: cmd_find_state,
    pub s: screen,

    pub px: u32,
    pub py: u32,

    pub menu: *mut menu,
    pub choice: i32,

    pub cb: menu_choice_cb,
    pub data: *mut c_void,
}

pub unsafe fn menu_add_items(
    menu: *mut menu,
    items: &[menu_item],
    qitem: *mut cmdq_item,
    c: *mut client,
    fs: *mut cmd_find_state,
) {
    for loop_ in items {
        unsafe {
            menu_add_item(menu, Some(loop_), qitem, c, fs);
        }
    }
}

pub unsafe fn menu_add_item(
    menu: *mut menu,
    item: Option<&menu_item>,
    qitem: *mut cmdq_item,
    c: *mut client,
    fs: *mut cmd_find_state,
) {
    unsafe {
        let line = item.is_none() || item.as_ref().unwrap().name.is_empty();
        if line && (*menu).items.is_empty() {
            return;
        }

        (*menu).items.push(menu_item::default());

        if line {
            return;
        }

        let s0 = if !fs.is_null() {
            format_single_from_state(qitem, &item.as_ref().unwrap().name, c, fs)
        } else {
            format_single(
                qitem,
                &item.as_ref().unwrap().name,
                c,
                null_mut(),
                null_mut(),
                null_mut(),
            )
        };

        if *s0 == b'\0' {
            (*menu).items.pop();
            return;
        }
        let mut max_width = (*c).tty.sx - 4;

        let mut key = null();
        let slen: usize = strlen(s0);
        if *s0 != b'-' && item.as_ref().unwrap().key != KEYC_UNKNOWN && item.as_ref().unwrap().key != KEYC_NONE {
            key = key_string_lookup_key(item.as_ref().unwrap().key, 0);
            let keylen: usize = strlen(key) + 3;

            if keylen <= max_width as usize / 4 {
                max_width -= keylen as u32;
            } else if keylen >= max_width as usize || slen >= max_width as usize - keylen {
                key = null_mut();
            }
        }

        let suffix = if slen > max_width as usize {
            max_width -= 1;
            c!(">")
        } else {
            c!("")
        };
        let trimmed = format_trim_right(s0, max_width);
        let name: String = if !key.is_null() {
            format!(
                "{}{}#[default] #[align=right]({})",
                _s(trimmed),
                _s(suffix),
                _s(key),
            )
        } else {
            format!("{}{}", _s(trimmed), _s(suffix))
        };
        free_(trimmed);

        let new_item = (*menu).items.last_mut().unwrap();

        new_item.name = Cow::Owned(name);
        free_(s0);

        let cmd: *const u8 = item.as_ref().unwrap().command.as_ptr();
        let s1: *mut u8 = if !cmd.is_null() {
            if !fs.is_null() {
                format_single_from_state(qitem, cstr_to_str(cmd), c, fs)
            } else {
                format_single(qitem, cstr_to_str(cmd), c, null_mut(), null_mut(), null_mut())
            }
        } else {
            null_mut()
        };
        new_item.command = SyncCharPtr::from_ptr(s1);
        new_item.key = item.as_ref().unwrap().key;

        let mut width = format_width(&new_item.name);
        if new_item.name.starts_with('-') {
            width -= 1;
        }
        if width > (*menu).width {
            (*menu).width = width;
        }
    }
}

pub fn menu_create(title: &str) -> Box<menu> {
    Box::new(menu {
        title: title.to_string(),
        items: Vec::new(),
        width: unsafe { format_width(title) },
    })
}

pub unsafe fn menu_free(menu: *mut menu) {
    unsafe {
        for item in (*menu).items.drain(..) {
            drop(item.name);
            free_(item.command.as_ptr().cast_mut());
        }
        (*menu).items = Vec::new();
        (*menu).title = String::new();
        free_(menu);
    }
}

pub unsafe fn menu_mode_cb(
    _c: *mut client,
    data: *mut c_void,
    cx: *mut u32,
    cy: *mut u32,
) -> *mut screen {
    unsafe {
        let md = data as *mut menu_data;

        *cx = (*md).px + 2;
        if (*md).choice == -1 {
            *cy = (*md).py;
        } else {
            *cy = (*md).py + 1 + (*md).choice as u32;
        }

        &raw mut (*md).s
    }
}

pub unsafe fn menu_check_cb(
    _c: *mut client,
    data: *mut c_void,
    px: u32,
    py: u32,
    nx: u32,
    r: *mut overlay_ranges,
) {
    unsafe {
        let md = data as *mut menu_data;
        let menu = (*md).menu;

        server_client_overlay_range(
            (*md).px,
            (*md).py,
            (*menu).width + 4,
            (*menu).items.len() as u32 + 2,
            px,
            py,
            nx,
            r,
        );
    }
}

pub unsafe fn menu_draw_cb(c: *mut client, data: *mut c_void, _rctx: *mut screen_redraw_ctx) {
    unsafe {
        let md = data as *mut menu_data;
        let tty = &raw mut (*c).tty;
        let s = &raw mut (*md).s;
        let menu = (*md).menu;
        let mut ctx = MaybeUninit::<screen_write_ctx>::uninit();
        let ctx = ctx.as_mut_ptr();
        // u_int i;
        let px = (*md).px;
        let py = (*md).py;

        screen_write_start(ctx, s);
        screen_write_clearscreen(ctx, 8);

        if (*md).border_lines != box_lines::BOX_LINES_NONE {
            screen_write_box(
                ctx,
                (*menu).width + 4,
                (*menu).items.len() as u32 + 2,
                (*md).border_lines,
                &raw mut (*md).border_style,
                Some(&(*menu).title),
            );
        }

        screen_write_menu(
            ctx,
            menu,
            (*md).choice,
            (*md).border_lines,
            &raw mut (*md).style,
            &raw mut (*md).border_style,
            &raw mut (*md).selected_style,
        );
        screen_write_stop(ctx);

        for i in 0..screen_size_y(&raw mut (*md).s) {
            tty_draw_line(
                tty,
                s,
                0,
                i,
                (*menu).width + 4,
                px,
                py + i,
                &raw const GRID_DEFAULT_CELL,
                null_mut(),
            );
        }
    }
}

pub unsafe fn menu_free_cb(_c: *mut client, data: *mut c_void) {
    unsafe {
        let md = data as *mut menu_data;

        if !(*md).item.is_null() {
            cmdq_continue((*md).item);
        }

        if let Some(cb) = (*md).cb {
            cb((*md).menu, u32::MAX, KEYC_NONE, (*md).data);
        }

        screen_free(&raw mut (*md).s);
        menu_free((*md).menu);
        free_(md);
    }
}

pub unsafe fn menu_key_cb(c: *mut client, data: *mut c_void, mut event: *mut key_event) -> i32 {
    unsafe {
        let md = data as *mut menu_data;
        let menu = (*md).menu;
        let m = &raw mut (*event).m;
        let count = (*menu).items.len();
        let mut old = (*md).choice;

        let mut error = null_mut();

        'chosen: {
            if KEYC_IS_MOUSE((*event).key) {
                if (*md).flags.intersects(menu_flags::MENU_NOMOUSE) {
                    if MOUSE_BUTTONS((*m).b) != MOUSE_BUTTON_1 {
                        return 1;
                    }
                    return 0;
                }
                if (*m).x < (*md).px
                    || (*m).x > (*md).px + 4 + (*menu).width
                    || (*m).y < (*md).py + 1
                    || (*m).y > (*md).py + 1 + count as u32 - 1
                {
                    if !(*md).flags.intersects(menu_flags::MENU_STAYOPEN) {
                        if MOUSE_RELEASE((*m).b) {
                            return 1;
                        }
                    } else if !MOUSE_RELEASE((*m).b) && !MOUSE_WHEEL((*m).b) && !MOUSE_DRAG((*m).b)
                    {
                        return 1;
                    }
                    if (*md).choice != -1 {
                        (*md).choice = -1;
                        (*c).flags |= client_flag::REDRAWOVERLAY;
                    }
                    return 0;
                }
                if !(*md).flags.intersects(MENU_STAYOPEN) {
                    if MOUSE_RELEASE((*m).b) {
                        break 'chosen;
                    }
                } else if !MOUSE_WHEEL((*m).b) && !MOUSE_DRAG((*m).b) {
                    break 'chosen;
                }
                (*md).choice = (*m).y as i32 - ((*md).py as i32 + 1);
                if (*md).choice != old {
                    (*c).flags |= client_flag::REDRAWOVERLAY;
                }
                return 0;
            }
            for i in 0..count {
                let name = &(&mut (*menu).items)[i].name;
                if name.is_empty() || name.starts_with('-') {
                    continue;
                }
                if (*event).key == (&(*menu).items)[i].key {
                    (*md).choice = i as i32;
                    break 'chosen;
                }
            }

            const G: u64 = 'g' as u64;
            const J: u64 = 'j' as u64;
            const K: u64 = 'k' as u64;
            const Q: u64 = 'q' as u64;

            const UP: u64 = keyc::KEYC_UP as u64;
            const DOWN: u64 = keyc::KEYC_DOWN as u64;
            const BTAB: u64 = keyc::KEYC_BTAB as u64;
            const BSPACE: u64 = keyc::KEYC_BSPACE as u64;
            const HOME: u64 = keyc::KEYC_HOME as u64;
            const END: u64 = keyc::KEYC_END as u64;
            const NPAGE: u64 = keyc::KEYC_NPAGE as u64;
            const PPAGE: u64 = keyc::KEYC_PPAGE as u64;

            const TAB: u64 = b'\x09' as u64;
            const ESCAPE: u64 = b'\x1b' as u64;
            const RETURN: u64 = b'\r' as u64;

            const G_UPPER: u64 = 'G' as u64;
            const CTRL_B: u64 = 'b' as u64 | KEYC_CTRL;
            const CTRL_C: u64 = 'c' as u64 | KEYC_CTRL;
            const CTRL_G: u64 = 'g' as u64 | KEYC_CTRL;
            const CTRL_F: u64 = 'f' as u64 | KEYC_CTRL;

            'match_: {
                match (*event).key & !KEYC_MASK_FLAGS {
                    BTAB | UP | K => {
                        if old == -1 {
                            old = 0;
                        }
                        loop {
                            if (*md).choice == -1 || (*md).choice == 0 {
                                (*md).choice = count as i32 - 1;
                            } else {
                                (*md).choice -= 1;
                            }
                            let name = &(&(*menu).items)[(*md).choice as usize].name;
                            if !((name.is_empty() || name.starts_with('-')) && (*md).choice != old) {
                                break;
                            }
                        }
                        (*c).flags |= client_flag::REDRAWOVERLAY;
                        return 0;
                    }
                    BSPACE => {
                        if !(*md).flags.intersects(menu_flags::MENU_TAB) {
                            break 'match_;
                        }
                        return 1;
                    }
                    key @ (TAB | DOWN | J) => {
                        if key == TAB {
                            if !(*md).flags.intersects(menu_flags::MENU_TAB) {
                                break 'match_;
                            }
                            if (*md).choice == count as i32 - 1 {
                                return 1;
                            }
                        }
                        if old == -1 {
                            old = 0;
                        }
                        loop {
                            if (*md).choice == -1 || (*md).choice == count as i32 - 1 {
                                (*md).choice = 0;
                            } else {
                                (*md).choice += 1;
                            }
                            let name = &(&(*menu).items)[(*md).choice as usize].name;
                            if !((name.is_empty() || name.starts_with('-')) && (*md).choice != old) {
                                break;
                            }
                        }
                        (*c).flags |= client_flag::REDRAWOVERLAY;
                        return 0;
                    }
                    PPAGE | CTRL_B => {
                        if (*md).choice < 6 {
                            (*md).choice = 0;
                        } else {
                            let mut i = 5;
                            while i > 0 {
                                (*md).choice -= 1;
                                let name = &(&(*menu).items)[(*md).choice as usize].name;
                                if (*md).choice != 0 && (!name.is_empty() && !name.starts_with('-')) {
                                    i -= 1;
                                } else if (*md).choice == 0 {
                                    break;
                                }
                            }
                        }
                        (*c).flags |= client_flag::REDRAWOVERLAY;
                    }
                    NPAGE => {
                        let mut name: &str;
                        if (*md).choice > count as i32 - 6 {
                            (*md).choice = count as i32 - 1;
                            name = &(&mut (*menu).items)[(*md).choice as usize].name;
                        } else {
                            let mut i = 5;
                            loop {
                                (*md).choice += 1;
                                name = &(&mut (*menu).items)[(*md).choice as usize].name;
                                if (*md).choice != count as i32 - 1
                                    && (!name.is_empty() && !name.starts_with('-'))
                                {
                                    i -= 1;
                                } else if (*md).choice == count as i32 - 1 {
                                    break;
                                }
                                if i <= 0 {
                                    break;
                                }
                            }
                        }
                        while name.is_empty() || name.starts_with('-') {
                            (*md).choice -= 1;
                            name = &(&(*menu).items)[(*md).choice as usize].name;
                        }
                        (*c).flags |= client_flag::REDRAWOVERLAY;
                    }
                    G | HOME => {
                        (*md).choice = 0;
                        let mut name = &(&(*menu).items)[(*md).choice as usize].name;
                        while name.is_empty() || name.starts_with('-') {
                            (*md).choice += 1;
                            name = &(&(*menu).items)[(*md).choice as usize].name;
                        }
                        (*c).flags |= client_flag::REDRAWOVERLAY;
                    }
                    G_UPPER | END => {
                        (*md).choice = count as i32 - 1;
                        let mut name = &(&mut (*menu).items)[(*md).choice as usize].name;
                        while name.is_empty() || name.starts_with('-') {
                            (*md).choice -= 1;
                            name = &(&mut (*menu).items)[(*md).choice as usize].name;
                        }
                        (*c).flags |= client_flag::REDRAWOVERLAY;
                    }
                    CTRL_F => (),
                    RETURN => break 'chosen,
                    ESCAPE | CTRL_C | CTRL_G | Q => return 1,
                    _ => (),
                }
            } // 'match_
            return 0;
        } // 'chosen

        if (*md).choice == -1 {
            return 1;
        }
        let item = &mut (&mut (*menu).items)[(*md).choice as usize];
        // TODO previously was is_null check here (but doesn't make sense for rust type), should it now be is_empty?
        if item.name.starts_with('-') {
            if (*md).flags.intersects(MENU_STAYOPEN) {
                return 0;
            }
            return 1;
        }
        if let Some(cb) = (*md).cb {
            cb((*md).menu, (*md).choice as u32, item.key, (*md).data);
            (*md).cb = None;
            return 1;
        }

        if !(*md).item.is_null() {
            event = cmdq_get_event((*md).item);
        } else {
            event = null_mut();
        }
        let state = cmdq_new_state(&raw mut (*md).fs, event, cmdq_state_flags::empty());

        let ptr = item.command.as_ptr();
        let cmd_str =
            std::str::from_utf8(std::slice::from_raw_parts(ptr.cast(), libc::strlen(ptr))).unwrap();
        let status = cmd_parse_and_append(cmd_str, None, c, state, &raw mut error);
        if status == cmd_parse_status::CMD_PARSE_ERROR {
            cmdq_append(c, cmdq_get_error(error).as_ptr());
            free_(error);
        }
        cmdq_free_state(state);
    }

    1
}

pub unsafe fn menu_set_style(
    c: *mut client,
    gc: *mut grid_cell,
    style: *const u8,
    option: *const u8,
) {
    unsafe {
        let o = (*(*(*(*c).session).curw).window).options;

        memcpy__(gc, &raw const GRID_DEFAULT_CELL);
        style_apply(gc, o, option, null_mut());
        if !style.is_null() {
            let mut sytmp = MaybeUninit::<style>::uninit();
            let sytmp = sytmp.as_mut_ptr();

            style_set(sytmp, &raw const GRID_DEFAULT_CELL);
            if style_parse(sytmp, gc, style) == 0 {
                (*gc).fg = (*sytmp).gc.fg;
                (*gc).bg = (*sytmp).gc.bg;
            }
        }
        (*gc).attr = grid_attr::empty();
    }
}

pub unsafe fn menu_prepare(
    menu: *mut menu,
    flags: menu_flags,
    mut starting_choice: i32,
    item: *mut cmdq_item,
    mut px: u32,
    mut py: u32,
    c: *mut client,
    mut lines: box_lines,
    style: *const u8,
    selected_style: *const u8,
    border_style: *const u8,
    fs: *mut cmd_find_state,
    cb: menu_choice_cb,
    data: *mut c_void,
) -> *mut menu_data {
    unsafe {
        let mut choice;
        let mut name: &str;

        let o = (*(*(*(*c).session).curw).window).options;

        if (*c).tty.sx < (*menu).width + 4 || (*c).tty.sy < (*menu).items.len() as u32 + 2 {
            return null_mut();
        }
        if px + (*menu).width + 4 > (*c).tty.sx {
            px = (*c).tty.sx - (*menu).width - 4;
        }
        if py + (*menu).items.len() as u32 + 2 > (*c).tty.sy {
            py = (*c).tty.sy - (*menu).items.len() as u32 - 2;
        }

        if lines == box_lines::BOX_LINES_DEFAULT {
            lines =
                box_lines::try_from(options_get_number_(o, "menu-border-lines") as i32).unwrap();
        }

        let md = xcalloc1::<menu_data>() as *mut menu_data;
        (*md).item = item;
        (*md).flags = flags;
        (*md).border_lines = lines;

        menu_set_style(c, &raw mut (*md).style, style, c!("menu-style"));
        menu_set_style(
            c,
            &raw mut (*md).selected_style,
            selected_style,
            c!("menu-selected-style"),
        );
        menu_set_style(
            c,
            &raw mut (*md).border_style,
            border_style,
            c!("menu-border-style"),
        );

        if !fs.is_null() {
            cmd_find_copy_state(&raw mut (*md).fs, fs);
        }
        screen_init(
            &raw mut (*md).s,
            (*menu).width + 4,
            (*menu).items.len() as u32 + 2,
            0,
        );
        if !(*md).flags.intersects(menu_flags::MENU_NOMOUSE) {
            (*md).s.mode |= mode_flag::MODE_MOUSE_ALL | mode_flag::MODE_MOUSE_BUTTON;
        }
        (*md).s.mode &= !mode_flag::MODE_CURSOR;

        (*md).px = px;
        (*md).py = py;

        (*md).menu = menu;
        (*md).choice = -1;

        if (*md).flags.intersects(MENU_NOMOUSE) {
            if starting_choice >= (*menu).items.len() as i32 {
                starting_choice = (*menu).items.len() as i32 - 1;
                choice = starting_choice + 1;
                loop {
                    name = &(&(*menu).items)[choice as usize - 1].name;
                    if !name.is_empty() && !name.starts_with('-') {
                        (*md).choice = choice - 1;
                        break;
                    }
                    choice -= 1;
                    if choice == 0 {
                        choice = (*menu).items.len() as i32;
                    }
                    if choice == starting_choice + 1 {
                        break;
                    }
                }
            } else if starting_choice >= 0 {
                choice = starting_choice;
                loop {
                    name = &(&(*menu).items)[choice as usize].name;
                    if !name.is_empty() && !name.starts_with('-') {
                        (*md).choice = choice;
                        break;
                    }
                    choice += 1;
                    if choice == (*menu).items.len() as i32 {
                        choice = 0;
                    }
                    if choice == starting_choice {
                        break;
                    }
                }
            }
        }

        (*md).cb = cb;
        (*md).data = data;
        md
    }
}

pub unsafe fn menu_display(
    menu: *mut menu,
    flags: menu_flags,
    starting_choice: i32,
    item: *mut cmdq_item,
    px: u32,
    py: u32,
    c: *mut client,
    lines: box_lines,
    style: *const u8,
    selected_style: *const u8,
    border_style: *const u8,
    fs: *mut cmd_find_state,
    cb: menu_choice_cb,
    data: *mut c_void,
) -> i32 {
    unsafe {
        let md = menu_prepare(
            menu,
            flags,
            starting_choice,
            item,
            px,
            py,
            c,
            lines,
            style,
            selected_style,
            border_style,
            fs,
            cb,
            data,
        );
        if md.is_null() {
            return -1;
        }
        server_client_set_overlay(
            c,
            0,
            None,
            Some(menu_mode_cb),
            Some(menu_draw_cb),
            Some(menu_key_cb),
            Some(menu_free_cb),
            None,
            md.cast(),
        );
    }
    0
}

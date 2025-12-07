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
use crate::*;
use crate::options_::*;

bitflags::bitflags! {
    #[repr(transparent)]
    #[derive(Copy, Clone, Eq, PartialEq)]
    pub struct popup_flag : i32 {
        const POPUP_CLOSEEXIT = 0x1;
        const POPUP_CLOSEEXITZERO = 0x2;
        const POPUP_INTERNAL = 0x4;
    }
}

pub type popup_close_cb = Option<unsafe fn(_: i32, _: *mut c_void)>;
pub type popup_finish_edit_cb = Option<unsafe fn(_: *mut u8, _: usize, _: *mut c_void)>;

#[repr(i32)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum dragging_state {
    Off,
    Move,
    Size,
}

#[repr(C)]
pub struct popup_data {
    pub c: *mut client,
    pub item: *mut cmdq_item,
    pub flags: popup_flag,
    pub title: *mut u8,

    pub border_cell: grid_cell,
    pub border_lines: box_lines,

    pub s: screen,
    pub defaults: grid_cell,
    pub palette: colour_palette,

    pub job: *mut job,
    pub ictx: *mut input_ctx,
    pub status: i32,
    pub cb: popup_close_cb,
    pub arg: *mut c_void,

    pub menu: *mut menu,
    pub md: *mut menu_data,
    pub close: i32,

    // Current position and size
    pub px: u32,
    pub py: u32,
    pub sx: u32,
    pub sy: u32,

    // Preferred position and size
    pub ppx: u32,
    pub ppy: u32,
    pub psx: u32,
    pub psy: u32,

    pub dragging: dragging_state,
    pub dx: u32,
    pub dy: u32,

    pub lx: u32,
    pub ly: u32,
    pub lb: u32,
}

#[repr(C)]
pub struct popup_editor {
    pub path: *mut u8,
    pub cb: popup_finish_edit_cb,
    pub arg: *mut c_void,
}

static POPUP_MENU_ITEMS: [menu_item; 8] = [
    menu_item::new(c"Close", 'q' as u64, null_mut()),
    menu_item::new(
        c"#{?buffer_name,Paste #[underscore]#{buffer_name},}",
        'p' as u64,
        null_mut(),
    ),
    menu_item::new(c"", KEYC_NONE, null_mut()),
    menu_item::new(c"Fill Space", 'F' as u64, null_mut()),
    menu_item::new(c"Centre", 'C' as u64, null_mut()),
    menu_item::new(c"", KEYC_NONE, null_mut()),
    menu_item::new(c"To Horizontal Pane", 'h' as u64, null_mut()),
    menu_item::new(c"To Vertical Pane", 'v' as u64, null_mut()),
];

static POPUP_INTERNAL_MENU_ITEMS: [menu_item; 4] = [
    menu_item::new(c"Close", 'q' as u64, null_mut()),
    menu_item::new(c"", KEYC_NONE, null_mut()),
    menu_item::new(c"Fill Space", 'F' as u64, null_mut()),
    menu_item::new(c"Centre", 'C' as u64, null_mut()),
];

pub unsafe fn popup_redraw_cb(ttyctx: *const tty_ctx) {
    unsafe {
        let pd = (*ttyctx).arg.cast::<popup_data>();
        (*(*pd).c).flags |= client_flag::REDRAWOVERLAY;
    }
}

pub unsafe fn popup_set_client_cb(ttyctx: *mut tty_ctx, c: *mut client) -> i32 {
    unsafe {
        let pd = (*ttyctx).arg.cast::<popup_data>();

        if c != (*pd).c {
            return 0;
        }
        if (*(*pd).c).flags.intersects(client_flag::REDRAWOVERLAY) {
            return 0;
        }

        (*ttyctx).bigger = 0;
        (*ttyctx).wox = 0;
        (*ttyctx).woy = 0;
        (*ttyctx).wsx = (*c).tty.sx;
        (*ttyctx).wsy = (*c).tty.sy;

        if (*pd).border_lines == box_lines::BOX_LINES_NONE {
            (*ttyctx).xoff = (*pd).px;
            (*ttyctx).rxoff = (*pd).px;
            (*ttyctx).yoff = (*pd).py;
            (*ttyctx).ryoff = (*pd).py;
        } else {
            (*ttyctx).xoff = (*pd).px + 1;
            (*ttyctx).rxoff = (*pd).px + 1;
            (*ttyctx).yoff = (*pd).py + 1;
            (*ttyctx).ryoff = (*pd).py + 1;
        }

        1
    }
}

pub unsafe fn popup_init_ctx_cb(ctx: *mut screen_write_ctx, ttyctx: *mut tty_ctx) {
    unsafe {
        let pd = (*ctx).arg.cast::<popup_data>();

        memcpy__(&raw mut (*ttyctx).defaults, &raw const (*pd).defaults);
        (*ttyctx).palette = &raw const (*pd).palette;
        (*ttyctx).redraw_cb = Some(popup_redraw_cb);
        (*ttyctx).set_client_cb = Some(popup_set_client_cb);
        (*ttyctx).arg = pd.cast();
    }
}

pub unsafe fn popup_mode_cb(
    c: *mut client,
    data: *mut c_void,
    cx: *mut u32,
    cy: *mut u32,
) -> *mut screen {
    unsafe {
        let pd = data.cast::<popup_data>();

        if !(*pd).md.is_null() {
            return menu_mode_cb(c, (*pd).md.cast(), cx, cy);
        }

        if (*pd).border_lines == box_lines::BOX_LINES_NONE {
            *cx = (*pd).px + (*pd).s.cx;
            *cy = (*pd).py + (*pd).s.cy;
        } else {
            *cx = (*pd).px + 1 + (*pd).s.cx;
            *cy = (*pd).py + 1 + (*pd).s.cy;
        }
        &raw mut (*pd).s
    }
}

/// Return parts of the input range which are not obstructed by the popup.
pub unsafe fn popup_check_cb(
    c: *mut client,
    data: *mut c_void,
    px: u32,
    py: u32,
    nx: u32,
    r: *mut overlay_ranges,
) {
    unsafe {
        let pd = data.cast::<popup_data>();
        let mut or = MaybeUninit::<[overlay_ranges; 2]>::uninit();
        let or: *mut overlay_ranges = or.as_mut_ptr().cast();

        let mut k = 0;

        if !(*pd).md.is_null() {
            // Check each returned range for the menu against the popup
            menu_check_cb(c, (*pd).md.cast(), px, py, nx, r);

            for i in 0..2 {
                server_client_overlay_range(
                    (*pd).px,
                    (*pd).py,
                    (*pd).sx,
                    (*pd).sy,
                    (*r).px[i],
                    py,
                    (*r).nx[i],
                    or.add(i),
                );
            }

            // or has up to OVERLAY_MAX_RANGES non-overlapping ranges,
            // ordered from left to right. Collect them in the output.
            for i in 0..2 {
                // Each or[i] only has 2 ranges
                for j in 0..2 {
                    if (*or.add(i)).nx[j] > 0 {
                        (*r).px[k] = (*or.add(i)).px[j];
                        (*r).nx[k] = (*or.add(i)).nx[j];
                        k += 1;
                    }
                }
            }

            // Zero remaining ranges if any
            for i in k..OVERLAY_MAX_RANGES {
                (*r).px[i] = 0;
                (*r).nx[i] = 0;
            }

            return;
        }

        server_client_overlay_range((*pd).px, (*pd).py, (*pd).sx, (*pd).sy, px, py, nx, r);
    }
}

pub unsafe fn popup_draw_cb(c: *mut client, data: *mut c_void, rctx: *mut screen_redraw_ctx) {
    unsafe {
        let pd = data.cast::<popup_data>();
        let tty = &mut (*c).tty;
        let mut s = MaybeUninit::<screen>::uninit();
        let mut ctx = MaybeUninit::<screen_write_ctx>::uninit();
        let (px, py) = ((*pd).px, (*pd).py);
        let palette = &raw mut (*pd).palette;
        let mut defaults = MaybeUninit::<grid_cell>::uninit();
        let defaults = defaults.as_mut_ptr();

        screen_init(s.as_mut_ptr(), (*pd).sx, (*pd).sy, 0);
        screen_write_start(ctx.as_mut_ptr(), s.as_mut_ptr());
        screen_write_clearscreen(ctx.as_mut_ptr(), 8);

        if (*pd).border_lines == box_lines::BOX_LINES_NONE {
            screen_write_cursormove(ctx.as_mut_ptr(), 0, 0, 0);
            screen_write_fast_copy(ctx.as_mut_ptr(), &raw mut (*pd).s, 0, 0, (*pd).sx, (*pd).sy);
        } else if (*pd).sx > 2 && (*pd).sy > 2 {
            screen_write_box(
                ctx.as_mut_ptr(),
                (*pd).sx,
                (*pd).sy,
                (*pd).border_lines,
                &(*pd).border_cell,
                (*pd).title,
            );
            screen_write_cursormove(ctx.as_mut_ptr(), 1, 1, 0);
            screen_write_fast_copy(
                ctx.as_mut_ptr(),
                &raw mut (*pd).s,
                0,
                0,
                (*pd).sx - 2,
                (*pd).sy - 2,
            );
        }
        screen_write_stop(ctx.as_mut_ptr());

        memcpy__(defaults, &raw const (*pd).defaults);
        if (*defaults).fg == 8 {
            (*defaults).fg = (*palette).fg;
        }
        if (*defaults).bg == 8 {
            (*defaults).bg = (*palette).bg;
        }

        if !(*pd).md.is_null() {
            (*c).overlay_check = Some(menu_check_cb);
            (*c).overlay_data = (*pd).md.cast();
        } else {
            (*c).overlay_check = None;
            (*c).overlay_data = null_mut();
        }

        for i in 0..(*pd).sy {
            tty_draw_line(
                tty,
                s.as_mut_ptr(),
                0,
                i,
                (*pd).sx,
                px,
                py + i,
                defaults,
                palette,
            );
        }

        screen_free(s.as_mut_ptr());

        if !(*pd).md.is_null() {
            (*c).overlay_check = None;
            (*c).overlay_data = null_mut();
            menu_draw_cb(c, (*pd).md.cast(), rctx);
        }

        (*c).overlay_check = Some(popup_check_cb);
        (*c).overlay_data = pd.cast();
    }
}

pub fn popup_free_cb(c: *mut client, data: *mut c_void) {
    unsafe {
        let pd = data as *mut popup_data;
        let item = (*pd).item;

        if !(*pd).md.is_null() {
            menu_free_cb(c, (*pd).md.cast());
        }

        if let Some(cb) = (*pd).cb {
            cb((*pd).status, (*pd).arg);
        }

        if !item.is_null() {
            let client = cmdq_get_client(item);
            if !client.is_null() && (*client).session.is_null() {
                (*client).retval = (*pd).status;
            }
            cmdq_continue(item);
        }
        server_client_unref((*pd).c);

        if !(*pd).job.is_null() {
            job_free((*pd).job);
        }
        input_free((*pd).ictx);

        screen_free(&mut (*pd).s);
        colour_palette_free(Some(&mut (*pd).palette));

        free_((*pd).title);
        free_(pd);
    }
}

pub fn popup_resize_cb(_c: *mut client, data: *mut c_void) {
    unsafe {
        let pd = data as *mut popup_data;
        if pd.is_null() {
            return;
        }

        let tty = &raw mut (*(*pd).c).tty;

        if !(*pd).md.is_null() {
            menu_free_cb(_c, (*pd).md.cast());
        }

        // Adjust position and size
        (*pd).sy = (*pd).psy.min((*tty).sy);
        (*pd).sx = (*pd).psx.min((*tty).sx);

        (*pd).py = if (*pd).ppy + (*pd).sy > (*tty).sy {
            (*tty).sy - (*pd).sy
        } else {
            (*pd).ppy
        };

        (*pd).px = if (*pd).ppx + (*pd).sx > (*tty).sx {
            (*tty).sx - (*pd).sx
        } else {
            (*pd).ppx
        };

        // Avoid zero size screens
        if (*pd).border_lines == box_lines::BOX_LINES_NONE {
            screen_resize(&mut (*pd).s, (*pd).sx, (*pd).sy, 0);
            if !(*pd).job.is_null() {
                job_resize((*pd).job, (*pd).sx, (*pd).sy);
            }
        } else if (*pd).sx > 2 && (*pd).sy > 2 {
            screen_resize(&mut (*pd).s, (*pd).sx - 2, (*pd).sy - 2, 0);
            if !(*pd).job.is_null() {
                job_resize((*pd).job, (*pd).sx - 2, (*pd).sy - 2);
            }
        }
    }
}

pub fn popup_make_pane(pd: *mut popup_data, type_: layout_type) {
    unsafe {
        let c = (*pd).c;
        let s = (*c).session;
        let w = (*(*s).curw).window;
        let wp = (*w).active;

        window_unzoom(w, 1);

        let lc = layout_split_pane(wp, type_, -1, spawn_flags::empty());
        let hlimit = options_get_number_((*s).options, "history-limit") as u32;
        let new_wp = window_add_pane((*wp).window, null_mut(), hlimit, spawn_flags::empty());
        layout_assign_pane(lc, new_wp, 0);

        (*new_wp).fd = job_transfer(
            (*pd).job,
            &mut (*new_wp).pid,
            (*new_wp).tty.as_mut_ptr(),
            TTY_NAME_MAX,
        );
        (*pd).job = null_mut();

        screen_set_title(&raw mut (*pd).s, (*new_wp).base.title);
        screen_free(&raw mut (*new_wp).base);
        memcpy__(&raw mut (*new_wp).base, &raw const (*pd).s);
        screen_resize(&raw mut (*new_wp).base, (*new_wp).sx, (*new_wp).sy, 1);
        screen_init(&raw mut (*pd).s, 1, 1, 0);

        let mut shell = options_get_string_((*s).options, "default-shell");
        if !checkshell_(shell) {
            shell = _PATH_BSHELL;
        }
        (*new_wp).shell = xstrdup(shell).as_ptr();

        window_pane_set_event(new_wp);
        window_set_active_pane(w, new_wp, 1);
        (*new_wp).flags |= window_pane_flags::PANE_CHANGED;

        (*pd).close = 1;
    }
}

pub fn popup_menu_done(_menu: *mut menu, _choice: u32, key: key_code, data: *mut c_void) {
    unsafe {
        let pd = data as *mut popup_data;
        let c = (*pd).c;

        (*pd).md = null_mut();
        (*pd).menu = null_mut();
        server_redraw_client((*pd).c);

        match key as u8 {
            b'p' => {
                if let Some(pb) = NonNull::new(paste_get_top(null_mut())) {
                    let mut len: usize = 0;
                    let buf = paste_buffer_data_(pb, &mut len);
                    bufferevent_write(job_get_event((*pd).job), buf as *const c_void, len);
                }
            }
            b'F' => {
                (*pd).sx = (*c).tty.sx;
                (*pd).sy = (*c).tty.sy;
                (*pd).px = 0;
                (*pd).py = 0;
                server_redraw_client(c);
            }
            b'C' => {
                (*pd).px = (*c).tty.sx / 2 - (*pd).sx / 2;
                (*pd).py = (*c).tty.sy / 2 - (*pd).sy / 2;
                server_redraw_client(c);
            }
            b'h' => popup_make_pane(pd, layout_type::LAYOUT_LEFTRIGHT),
            b'v' => popup_make_pane(pd, layout_type::LAYOUT_TOPBOTTOM),
            b'q' => (*pd).close = 1,
            _ => {}
        }
    }
}

pub unsafe fn popup_handle_drag(c: *mut client, pd: *mut popup_data, m: *mut mouse_event) {
    unsafe {
        let px: u32;
        let py: u32;

        if !MOUSE_DRAG((*m).b) {
            (*pd).dragging = dragging_state::Off;
        } else if (*pd).dragging == dragging_state::Move {
            if (*m).x < (*pd).dx {
                px = 0;
            } else if (*m).x - (*pd).dx + (*pd).sx > (*c).tty.sx {
                px = (*c).tty.sx - (*pd).sx;
            } else {
                px = (*m).x - (*pd).dx;
            }
            if (*m).y < (*pd).dy {
                py = 0;
            } else if (*m).y - (*pd).dy + (*pd).sy > (*c).tty.sy {
                py = (*c).tty.sy - (*pd).sy;
            } else {
                py = (*m).y - (*pd).dy;
            }
            (*pd).px = px;
            (*pd).py = py;
            (*pd).dx = (*m).x - (*pd).px;
            (*pd).dy = (*m).y - (*pd).py;
            (*pd).ppx = px;
            (*pd).ppy = py;
            server_redraw_client(c);
        } else if (*pd).dragging == dragging_state::Size {
            if (*pd).border_lines == box_lines::BOX_LINES_NONE {
                if (*m).x < (*pd).px + 1 {
                    return;
                }
                if (*m).y < (*pd).py + 1 {
                    return;
                }
            } else {
                if (*m).x < (*pd).px + 3 {
                    return;
                }
                if (*m).y < (*pd).py + 3 {
                    return;
                }
            }
            (*pd).sx = (*m).x - (*pd).px;
            (*pd).sy = (*m).y - (*pd).py;
            (*pd).psx = (*pd).sx;
            (*pd).psy = (*pd).sy;

            if (*pd).border_lines == box_lines::BOX_LINES_NONE {
                screen_resize(&raw mut (*pd).s, (*pd).sx, (*pd).sy, 0);
                if !(*pd).job.is_null() {
                    job_resize((*pd).job, (*pd).sx, (*pd).sy);
                }
            } else {
                screen_resize(&raw mut (*pd).s, (*pd).sx - 2, (*pd).sy - 2, 0);
                if !(*pd).job.is_null() {
                    job_resize((*pd).job, (*pd).sx - 2, (*pd).sy - 2);
                }
            }
            server_redraw_client(c);
        }
    }
}

pub unsafe fn popup_key_cb(c: *mut client, data: *mut c_void, event: *mut key_event) -> i32 {
    unsafe {
        let pd = data as *mut popup_data;
        let m = &raw mut (*event).m;
        let mut buf = null();
        let mut len = 0;

        'menu: {
            'out: {
                #[repr(i32)]
                #[derive(Copy, Clone, Eq, PartialEq)]
                enum Border {
                    None,
                    Left,
                    Right,
                    Top,
                    Bottom,
                }
                let mut border = Border::None;

                if !(*pd).md.is_null() {
                    if menu_key_cb(c, (*pd).md.cast(), event) == 1 {
                        (*pd).md = null_mut();
                        (*pd).menu = null_mut();
                        if (*pd).close != 0 {
                            server_client_clear_overlay(c);
                        } else {
                            server_redraw_client(c);
                        }
                    }
                    return 0;
                }

                if KEYC_IS_MOUSE((*event).key) {
                    if (*pd).dragging != dragging_state::Off {
                        popup_handle_drag(c, pd, m);
                        break 'out;
                    }
                    if (*m).x < (*pd).px
                        || (*m).x > (*pd).px + (*pd).sx - 1
                        || (*m).y < (*pd).py
                        || (*m).y > (*pd).py + (*pd).sy - 1
                    {
                        if MOUSE_BUTTONS((*m).b) == MOUSE_BUTTON_3 {
                            break 'menu;
                        }
                        return 0;
                    }
                    if (*pd).border_lines != box_lines::BOX_LINES_NONE {
                        if (*m).x == (*pd).px {
                            border = Border::Left;
                        } else if (*m).x == (*pd).px + (*pd).sx - 1 {
                            border = Border::Right;
                        } else if (*m).y == (*pd).py {
                            border = Border::Top;
                        } else if (*m).y == (*pd).py + (*pd).sy - 1 {
                            border = Border::Bottom;
                        }
                    }
                    if ((*m).b & MOUSE_MASK_MODIFIERS) == 0
                        && MOUSE_BUTTONS((*m).b) == MOUSE_BUTTON_3
                        && (border == Border::Left || border == Border::Top)
                    {
                        break 'menu;
                    }
                    if (((*m).b & MOUSE_MASK_MODIFIERS) == MOUSE_MASK_META)
                        || border != Border::None
                    {
                        if !MOUSE_DRAG((*m).b) {
                            break 'out;
                        }
                        if MOUSE_BUTTONS((*m).lb) == MOUSE_BUTTON_1 {
                            (*pd).dragging = dragging_state::Move;
                        } else if MOUSE_BUTTONS((*m).lb) == MOUSE_BUTTON_3 {
                            (*pd).dragging = dragging_state::Size;
                        }
                        (*pd).dx = (*m).lx - (*pd).px;
                        (*pd).dy = (*m).ly - (*pd).py;
                        break 'out;
                    }
                }
                if ((!(*pd)
                    .flags
                    .intersects(popup_flag::POPUP_CLOSEEXIT | popup_flag::POPUP_CLOSEEXITZERO))
                    || (*pd).job.is_null())
                    && ((*event).key == b'\x1b' as u64 || (*event).key == (b'c' as u64 | KEYC_CTRL))
                {
                    return 1;
                }
                if !(*pd).job.is_null() {
                    if KEYC_IS_MOUSE((*event).key) {
                        // Must be inside, checked already.
                        let (px, py) = if (*pd).border_lines == box_lines::BOX_LINES_NONE {
                            ((*m).x - (*pd).px, (*m).y - (*pd).py)
                        } else {
                            ((*m).x - (*pd).px - 1, (*m).y - (*pd).py - 1)
                        };
                        if input_key_get_mouse(
                            &raw mut (*pd).s,
                            m,
                            px,
                            py,
                            &raw mut buf,
                            &raw mut len,
                        ) == 0
                        {
                            return 0;
                        }
                        bufferevent_write(job_get_event((*pd).job), buf.cast(), len);
                        return 0;
                    }
                    input_key(&raw mut (*pd).s, job_get_event((*pd).job), (*event).key);
                }
                return 0;
            }
            // menu:
            (*pd).menu = menu_create(c!(""));
            if (*pd).flags.intersects(popup_flag::POPUP_INTERNAL) {
                menu_add_items(
                    (*pd).menu,
                    POPUP_INTERNAL_MENU_ITEMS.as_slice(),
                    null_mut(),
                    c,
                    null_mut(),
                );
            } else {
                menu_add_items((*pd).menu, &POPUP_MENU_ITEMS, null_mut(), c, null_mut());
            }
            let x = (*m).x.saturating_sub(((*(*pd).menu).width + 4) / 2);
            (*pd).md = menu_prepare(
                (*pd).menu,
                menu_flags::empty(),
                0,
                null_mut(),
                x,
                (*m).y,
                c,
                box_lines::BOX_LINES_DEFAULT,
                null_mut(),
                null_mut(),
                null_mut(),
                null_mut(),
                Some(popup_menu_done),
                pd.cast(),
            );
            (*c).flags |= client_flag::REDRAWOVERLAY;
        }
        // out:
        (*pd).lx = (*m).x;
        (*pd).ly = (*m).y;
        (*pd).lb = (*m).b;
        0
    }
}

pub unsafe fn popup_job_update_cb(job: *mut job) {
    unsafe {
        let pd = job_get_data(job) as *mut popup_data;
        let evb = (*job_get_event(job)).input;
        let c = (*pd).c;
        let s = &raw mut (*pd).s;
        let data = EVBUFFER_DATA(evb);
        let size = EVBUFFER_LENGTH(evb);

        if size == 0 {
            return;
        }

        if !(*pd).md.is_null() {
            (*c).overlay_check = Some(menu_check_cb);
            (*c).overlay_data = (*pd).md.cast();
        } else {
            (*c).overlay_check = None;
            (*c).overlay_data = null_mut();
        }
        input_parse_screen(
            (*pd).ictx,
            s,
            Some(popup_init_ctx_cb),
            pd.cast(),
            data,
            size,
        );
        (*c).overlay_check = Some(popup_check_cb);
        (*c).overlay_data = pd.cast();

        evbuffer_drain(evb, size);
    }
}

pub unsafe fn popup_job_complete_cb(job: *mut job) {
    unsafe {
        let pd = job_get_data(job) as *mut popup_data;
        let status = job_get_status((*pd).job);

        if WIFEXITED(status) {
            (*pd).status = WEXITSTATUS(status);
        } else if WIFSIGNALED(status) {
            (*pd).status = WTERMSIG(status);
        } else {
            (*pd).status = 0;
        }
        (*pd).job = null_mut();

        if (*pd).flags.intersects(popup_flag::POPUP_CLOSEEXIT)
            || ((*pd).flags.intersects(popup_flag::POPUP_CLOSEEXITZERO) && (*pd).status == 0)
        {
            server_client_clear_overlay((*pd).c);
        }
    }
}

pub unsafe fn popup_display(
    flags: popup_flag,
    mut lines: box_lines,
    item: *mut cmdq_item,
    px: c_uint,
    py: c_uint,
    sx: c_uint,
    sy: c_uint,
    env: *mut environ,
    shellcmd: *const u8,
    argc: c_int,
    argv: *mut *mut u8,
    cwd: *const u8,
    title: *const u8,
    c: *mut client,
    s: *mut session,
    style: *const u8,
    border_style: *const u8,
    cb: popup_close_cb,
    arg: *mut c_void,
) -> c_int {
    unsafe {
        let o = if !s.is_null() {
            (*(*(*s).curw).window).options
        } else {
            (*(*(*(*c).session).curw).window).options
        };

        lines = if lines == box_lines::BOX_LINES_DEFAULT {
            (options_get_number_(o, "popup-border-lines") as i32)
                .try_into()
                .unwrap_or(box_lines::BOX_LINES_ROUNDED) // TODO
        } else {
            lines
        };

        let (jx, jy) = if lines == box_lines::BOX_LINES_NONE {
            if sx < 1 || sy < 1 {
                return -1;
            }
            (sx, sy)
        } else {
            if sx < 3 || sy < 3 {
                return -1;
            }
            (sx - 2, sy - 2)
        };

        if (*c).tty.sx < sx || (*c).tty.sy < sy {
            return -1;
        }

        let pd = xcalloc1::<popup_data>() as *mut popup_data;
        (*pd).item = item;
        (*pd).flags = flags;
        if !title.is_null() {
            (*pd).title = xstrdup(title).as_ptr();
        }

        (*pd).c = c;
        (*(*pd).c).references += 1;

        (*pd).cb = cb;
        (*pd).arg = arg;
        (*pd).status = 128 + SIGHUP;

        (*pd).border_lines = lines;
        memcpy__(&raw mut (*pd).border_cell, &raw const GRID_DEFAULT_CELL);
        style_apply(
            &raw mut (*pd).border_cell,
            o,
            c!("popup-border-style"),
            null_mut(),
        );

        if !border_style.is_null() {
            let mut sytmp = MaybeUninit::<style>::uninit();
            style_set(sytmp.as_mut_ptr(), &raw const GRID_DEFAULT_CELL);
            if style_parse(sytmp.as_mut_ptr(), &raw mut (*pd).border_cell, border_style) == 0 {
                (*pd).border_cell.fg = (*sytmp.as_ptr()).gc.fg;
                (*pd).border_cell.bg = (*sytmp.as_ptr()).gc.bg;
            }
        }
        (*pd).border_cell.attr = grid_attr::empty();

        screen_init(&raw mut (*pd).s, jx, jy, 0);
        (*pd).palette = colour_palette_init();
        colour_palette_from_option(Some(&mut (*pd).palette), GLOBAL_W_OPTIONS);

        memcpy__(&raw mut (*pd).defaults, &raw const GRID_DEFAULT_CELL);
        style_apply(
            &raw mut (*pd).defaults,
            o,
            c!("popup-style").cast(),
            null_mut(),
        );
        if !style.is_null() {
            let mut sytmp = MaybeUninit::<style>::uninit();
            style_set(sytmp.as_mut_ptr(), &raw const GRID_DEFAULT_CELL);
            if style_parse(sytmp.as_mut_ptr(), &raw mut (*pd).defaults, style) == 0 {
                (*pd).defaults.fg = (*sytmp.as_ptr()).gc.fg;
                (*pd).defaults.bg = (*sytmp.as_ptr()).gc.bg;
            }
        }
        (*pd).defaults.attr = grid_attr::empty();

        (*pd).px = px;
        (*pd).py = py;
        (*pd).sx = sx;
        (*pd).sy = sy;

        (*pd).ppx = px;
        (*pd).ppy = py;
        (*pd).psx = sx;
        (*pd).psy = sy;

        (*pd).job = job_run(
            shellcmd,
            argc,
            argv,
            env,
            s,
            cwd,
            Some(popup_job_update_cb),
            Some(popup_job_complete_cb),
            None,
            pd.cast(),
            job_flag::JOB_NOWAIT
                | job_flag::JOB_PTY
                | job_flag::JOB_KEEPWRITE
                | job_flag::JOB_DEFAULTSHELL,
            jx as i32,
            jy as i32,
        );
        (*pd).ictx = input_init(null_mut(), job_get_event((*pd).job), &raw mut (*pd).palette);

        server_client_set_overlay(
            c,
            0,
            Some(popup_check_cb),
            Some(popup_mode_cb),
            Some(popup_draw_cb),
            Some(popup_key_cb),
            Some(popup_free_cb),
            Some(popup_resize_cb),
            pd.cast(),
        );
        0
    }
}

pub unsafe fn popup_editor_free(pe: *mut popup_editor) {
    unsafe {
        unlink((*pe).path.cast());
        free_((*pe).path);
        free_(pe);
    }
}

pub unsafe fn popup_editor_close_cb(status: i32, arg: *mut c_void) {
    unsafe {
        let pe = arg as *mut popup_editor;
        let mut buf: *mut u8 = null_mut();
        let mut len: libc::off_t = 0;

        if status != 0 {
            ((*pe).cb.unwrap())(null_mut(), 0, (*pe).arg);
            popup_editor_free(pe);
            return;
        }

        let f = fopen((*pe).path, c!("r"));
        if !f.is_null() {
            fseeko(f, 0, SEEK_END);
            len = ftello(f);
            fseeko(f, 0, SEEK_SET);

            // TODO SIZE_MAX is used in C, this check is essentially useless
            if len == 0
                || len as usize > usize::MAX
                || {
                    buf = malloc(len as usize).cast();
                    buf.is_null()
                }
                || fread(buf.cast(), len as usize, 1, f) != 1
            {
                free_(buf);
                buf = null_mut();
                len = 0;
            }
            fclose(f);
        }
        ((*pe).cb.unwrap())(buf, len as usize, (*pe).arg); // callback now owns buffer
        popup_editor_free(pe);
    }
}

// https://blog.orhun.dev/zero-deps-random-in-rust/
pub fn random_seed() -> u64 {
    use std::collections::hash_map::RandomState;
    use std::hash::{BuildHasher, Hasher};
    RandomState::new().build_hasher().finish()
}

fn create_temp_file() -> std::path::PathBuf {
    let mut seed = random_seed();
    let mut filename = String::from("tmux.");
    for _ in 0..8 {
        let ch = (b'a' + (seed % 26) as u8) as char;
        filename.push(ch);
        seed = seed.wrapping_mul(1103515245).wrapping_add(12345);
    }
    std::env::temp_dir().join(filename)
}

pub unsafe fn popup_editor(
    c: *mut client,
    buf: &[u8],
    cb: popup_finish_edit_cb,
    arg: *mut c_void,
) -> c_int {
    unsafe {
        let editor = options_get_string_(GLOBAL_OPTIONS, "editor");
        if *editor == b'\0' {
            return -1;
        }

        let path = create_temp_file();
        if std::fs::write(&path, buf).is_err() {
            return -1;
        }

        let pe = xcalloc1::<popup_editor>();
        pe.path = xstrdup__(
            path.to_str()
                .expect("fixme: temporary path SHOULD be valid string; or should store path buf"),
        );
        pe.cb = cb;
        pe.arg = arg;

        let sx = (*c).tty.sx * 9 / 10;
        let sy = (*c).tty.sy * 9 / 10;
        let px = ((*c).tty.sx / 2).wrapping_sub(sx / 2);
        let py = ((*c).tty.sy / 2).wrapping_sub(sy / 2);

        let cmd = format_nul!("{} {}", _s(editor), path.display());
        if popup_display(
            popup_flag::POPUP_INTERNAL | popup_flag::POPUP_CLOSEEXIT,
            box_lines::BOX_LINES_DEFAULT,
            null_mut(),
            px,
            py,
            sx,
            sy,
            null_mut(),
            cmd,
            0,
            null_mut(),
            c!("/tmp/"),
            null(),
            c,
            null_mut(),
            null(),
            null(),
            Some(popup_editor_close_cb),
            pe as *mut popup_editor as *mut c_void,
        ) != 0
        {
            popup_editor_free(pe);
            free_(cmd);
            return -1;
        }
        free_(cmd);
        0
    }
}

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
use crate::compat::HOST_NAME_MAX;
use crate::libc::{
    FIONREAD, FNM_CASEFOLD, TIOCSWINSZ, close, fnmatch, free, gethostname, gettimeofday, ioctl,
    isspace, memset, regcomp, regex_t, regexec, regfree, strcasecmp, strlen, winsize,
};
#[cfg(feature = "utempter")]
use crate::utempter::utempter_remove_record;
use crate::*;

pub static mut WINDOWS: windows = unsafe { std::mem::zeroed() };

pub static mut ALL_WINDOW_PANES: window_pane_tree = unsafe { std::mem::zeroed() };

#[repr(C)]
#[derive(Copy, Clone)]
pub struct window_pane_input_data {
    item: *mut cmdq_item,
    wp: u32,
    file: *mut client_file,
}

RB_GENERATE!(windows, window, entry, discr_entry, window_cmp);
RB_GENERATE!(winlinks, winlink, entry, discr_entry, winlink_cmp);
RB_GENERATE!(
    window_pane_tree,
    window_pane,
    tree_entry,
    discr_tree_entry,
    window_pane_cmp
);

pub fn window_cmp(w1: &window, w2: &window) -> cmp::Ordering {
    w1.id.cmp(&w2.id)
}

pub fn winlink_cmp(wl1: &winlink, wl2: &winlink) -> cmp::Ordering {
    wl1.idx.cmp(&wl2.idx)
}

pub fn window_pane_cmp(wp1: &window_pane, wp2: &window_pane) -> cmp::Ordering {
    wp1.id.cmp(&wp2.id)
}

pub unsafe fn winlink_find_by_window(
    wwl: *mut winlinks,
    w: *mut window,
) -> Option<NonNull<winlink>> {
    unsafe {
        for wl in rb_foreach(wwl) {
            if (*wl.as_ptr()).window == w {
                return Some(wl);
            }
        }
        None
    }
}

pub unsafe fn winlink_find_by_index(wwl: *mut winlinks, idx: i32) -> *mut winlink {
    unsafe {
        if idx < 0 {
            fatalx("bad index");
        }

        let mut wl: winlink = std::mem::zeroed();
        wl.idx = idx;

        rb_find(wwl, &raw mut wl)
    }
}

pub unsafe fn winlink_find_by_window_id(wwl: *mut winlinks, id: u32) -> *mut winlink {
    unsafe {
        for wl in rb_foreach(wwl).map(NonNull::as_ptr) {
            if (*(*wl).window).id == id {
                return wl;
            }
        }

        null_mut()
    }
}

unsafe fn winlink_next_index(wwl: *mut winlinks, idx: i32) -> i32 {
    let mut i = idx;

    loop {
        if unsafe { winlink_find_by_index(wwl, i).is_null() } {
            return i;
        }

        if i == i32::MAX {
            i = 0;
        } else {
            i += 1;
        }

        if i == idx {
            break;
        }
    }

    -1
}

pub unsafe fn winlink_count(wwl: *mut winlinks) -> u32 {
    unsafe { rb_foreach(wwl).count() as u32 }
}

pub unsafe fn winlink_add(wwl: *mut winlinks, mut idx: i32) -> *mut winlink {
    unsafe {
        if idx < 0 {
            idx = winlink_next_index(wwl, -idx - 1);
            if idx == -1 {
                return null_mut();
            }
        } else if !winlink_find_by_index(wwl, idx).is_null() {
            return null_mut();
        }

        let wl: *mut winlink = xcalloc_::<winlink>(1).as_ptr();
        (*wl).idx = idx;
        rb_insert(wwl, wl);

        wl
    }
}

pub unsafe fn winlink_set_window(wl: *mut winlink, w: *mut window) {
    unsafe {
        if !(*wl).window.is_null() {
            tailq_remove::<_, discr_wentry>(&raw mut (*(*wl).window).winlinks, wl);
            window_remove_ref((*wl).window, c!("winlink_set_window"));
        }
        tailq_insert_tail::<_, discr_wentry>(&raw mut (*w).winlinks, wl);
        (*wl).window = w;
        window_add_ref(w, c!("winlink_set_window"));
    }
}

pub unsafe fn winlink_remove(wwl: *mut winlinks, wl: *mut winlink) {
    unsafe {
        let w = (*wl).window;

        if !w.is_null() {
            tailq_remove::<_, discr_wentry>(&raw mut (*w).winlinks, wl);
            window_remove_ref(w, c!("winlink_remove"));
        }

        rb_remove(wwl, wl);
        free(wl as _);
    }
}

pub unsafe fn winlink_next(wl: *mut winlink) -> *mut winlink {
    unsafe { rb_next(wl) }
}

pub unsafe fn winlink_previous(wl: *mut winlink) -> *mut winlink {
    unsafe { rb_prev(wl) }
}

pub unsafe fn winlink_next_by_number(
    mut wl: *mut winlink,
    s: *mut session,
    n: i32,
) -> *mut winlink {
    unsafe {
        for _ in 0..n {
            wl = rb_next(wl);
            if wl.is_null() {
                wl = rb_min(&raw mut (*s).windows);
            }
        }
    }

    wl
}

pub unsafe fn winlink_previous_by_number(
    mut wl: *mut winlink,
    s: *mut session,
    n: i32,
) -> *mut winlink {
    unsafe {
        for _ in 0..n {
            wl = rb_prev(wl);
            if wl.is_null() {
                wl = rb_min(&raw mut (*s).windows);
            }
        }
    }

    wl
}

pub unsafe fn winlink_stack_push(stack: *mut winlink_stack, wl: *mut winlink) {
    if wl.is_null() {
        return;
    }

    unsafe {
        winlink_stack_remove(stack, wl);
        tailq_insert_head::<_, discr_sentry>(stack, wl);
        (*wl).flags |= winlink_flags::WINLINK_VISITED;
    }
}

pub unsafe fn winlink_stack_remove(stack: *mut winlink_stack, wl: *mut winlink) {
    unsafe {
        if !wl.is_null() && (*wl).flags.intersects(winlink_flags::WINLINK_VISITED) {
            tailq_remove::<_, discr_sentry>(stack, wl);
            (*wl).flags &= !winlink_flags::WINLINK_VISITED;
        }
    }
}

pub unsafe fn window_find_by_id_str(s: *const u8) -> *mut window {
    unsafe {
        if *s != b'@' {
            return null_mut();
        }

        let Ok(id) = strtonum(s.wrapping_add(1), 0, u32::MAX) else {
            return null_mut();
        };

        window_find_by_id(id)
    }
}

pub unsafe fn window_find_by_id(id: u32) -> *mut window {
    unsafe {
        let mut w: window = std::mem::zeroed();

        w.id = id;
        rb_find(&raw mut WINDOWS, &raw mut w)
    }
}

pub unsafe fn window_update_activity(w: NonNull<window>) {
    unsafe {
        gettimeofday(&raw mut (*w.as_ptr()).activity_time, null_mut());
        alerts_queue(w, window_flag::ACTIVITY);
    }
}

pub unsafe fn window_create(sx: u32, sy: u32, mut xpixel: u32, mut ypixel: u32) -> *mut window {
    static NEXT_WINDOW_ID: AtomicU32 = AtomicU32::new(0);

    if xpixel == 0 {
        xpixel = DEFAULT_XPIXEL;
    }
    if ypixel == 0 {
        ypixel = DEFAULT_YPIXEL;
    }
    unsafe {
        let w: *mut window = xcalloc_::<window>(1).as_ptr();
        (*w).name = xstrdup(c!("")).as_ptr();
        (*w).flags = window_flag::empty();

        tailq_init(&raw mut (*w).panes);
        tailq_init(&raw mut (*w).last_panes);
        (*w).active = null_mut();

        (*w).lastlayout = -1;
        (*w).layout_root = null_mut();

        (*w).sx = sx;
        (*w).sy = sy;
        (*w).manual_sx = sx;
        (*w).manual_sy = sy;
        (*w).xpixel = xpixel;
        (*w).ypixel = ypixel;

        (*w).options = options_create(GLOBAL_W_OPTIONS);

        (*w).references = 0;
        tailq_init(&raw mut (*w).winlinks);

        (*w).id = NEXT_WINDOW_ID.fetch_add(1, atomic::Ordering::Relaxed);
        rb_insert(&raw mut WINDOWS, w);

        window_set_fill_character(NonNull::new_unchecked(w));
        window_update_activity(NonNull::new_unchecked(w));

        log_debug!(
            "{}: @{} create {}x{} ({}x{})",
            "window_create",
            (*w).id,
            sx,
            sy,
            (*w).xpixel,
            (*w).ypixel,
        );
        w
    }
}

unsafe fn window_destroy(w: *mut window) {
    unsafe {
        log_debug!(
            "window @{} destroyed ({} references)",
            (*w).id,
            (*w).references
        );

        window_unzoom(w, 0);
        rb_remove(&raw mut WINDOWS, w);

        if !(*w).layout_root.is_null() {
            layout_free_cell((*w).layout_root);
        }
        if !(*w).saved_layout_root.is_null() {
            layout_free_cell((*w).saved_layout_root);
        }
        free((*w).old_layout as _);

        window_destroy_panes(w);

        if event_initialized(&raw mut (*w).name_event) != 0 {
            event_del(&raw mut (*w).name_event);
        }

        if event_initialized(&raw mut (*w).alerts_timer) != 0 {
            event_del(&raw mut (*w).alerts_timer);
        }
        if event_initialized(&raw mut (*w).offset_timer) != 0 {
            event_del(&raw mut (*w).offset_timer);
        }

        options_free((*w).options);
        free((*w).fill_character as _);

        free((*w).name as _);
        free(w as _);
    }
}

pub unsafe fn window_pane_destroy_ready(wp: *mut window_pane) -> i32 {
    let mut n = 0;
    unsafe {
        if (*wp).pipe_fd != -1 {
            if EVBUFFER_LENGTH((*(*wp).pipe_event).output) != 0 {
                return 0;
            }
            if ioctl((*wp).fd, FIONREAD, &raw mut n) != -1 && n > 0 {
                return 0;
            }
        }

        if !(*wp).flags.intersects(window_pane_flags::PANE_EXITED) {
            return 0;
        }
    }

    1
}

pub unsafe fn window_add_ref(w: *mut window, from: *const u8) {
    unsafe {
        (*w).references += 1;
        log_debug!(
            "{}: @{} {}, now {}",
            "window_add_ref",
            (*w).id,
            _s(from),
            (*w).references,
        );
    }
}

pub unsafe fn window_remove_ref(w: *mut window, from: *const u8) {
    unsafe {
        (*w).references -= 1;
        log_debug!(
            "{}: @{} {}, now {}",
            "window_remove_ref",
            (*w).id,
            _s(from),
            (*w).references,
        );

        if (*w).references == 0 {
            window_destroy(w);
        }
    }
}

pub unsafe fn window_set_name(w: *mut window, new_name: *const u8) {
    unsafe {
        free_((*w).name);
        utf8_stravis(
            &raw mut (*w).name,
            new_name,
            vis_flags::VIS_OCTAL | vis_flags::VIS_CSTYLE | vis_flags::VIS_TAB | vis_flags::VIS_NL,
        );
        notify_window(c"window-renamed", w);
    }
}

pub unsafe fn window_resize(w: *mut window, sx: u32, sy: u32, mut xpixel: i32, mut ypixel: i32) {
    if xpixel == 0 {
        xpixel = DEFAULT_XPIXEL as i32;
    }
    if ypixel == 0 {
        ypixel = DEFAULT_YPIXEL as i32;
    }

    unsafe {
        log_debug!(
            "{}: @{} resize {}x{} ({}x{})",
            "window_resize",
            (*w).id,
            sx,
            sy,
            if xpixel == -1 {
                (*w).xpixel
            } else {
                xpixel as u32
            },
            if ypixel == -1 {
                (*w).ypixel
            } else {
                ypixel as u32
            },
        );

        (*w).sx = sx;
        (*w).sy = sy;
        if xpixel != -1 {
            (*w).xpixel = xpixel as u32;
        }
        if ypixel != -1 {
            (*w).ypixel = ypixel as u32;
        }
    }
}

pub unsafe fn window_pane_send_resize(wp: *mut window_pane, sx: u32, sy: u32) {
    unsafe {
        let w = (*wp).window;
        let mut ws: winsize = core::mem::zeroed();

        if (*wp).fd == -1 {
            return;
        }

        log_debug!(
            "{}: %%{} resize to {},{}",
            "window_pane_send_resize",
            (*wp).id,
            sx,
            sy,
        );

        memset(&raw mut ws as _, 0, size_of::<winsize>());

        ws.ws_col = sx as u16;
        ws.ws_row = sy as u16;
        ws.ws_xpixel = (*w).xpixel as u16 * ws.ws_col;
        ws.ws_ypixel = (*w).ypixel as u16 * ws.ws_row;

        // TODO sun ifdef

        if ioctl((*wp).fd, TIOCSWINSZ, &ws) == -1 {
            fatal("ioctl failed");
        }
    }
}

pub unsafe fn window_has_pane(w: *mut window, wp: *mut window_pane) -> bool {
    unsafe { tailq_foreach::<_, discr_entry>(&raw mut (*w).panes).any(|wp1| wp1.as_ptr() == wp) }
}

pub unsafe fn window_update_focus(w: *mut window) {
    unsafe {
        if !w.is_null() {
            log_debug!("{}: @{}", "window_update_focus", (*w).id);
            window_pane_update_focus((*w).active);
        }
    }
}

pub unsafe fn window_pane_update_focus(wp: *mut window_pane) {
    unsafe {
        let mut focused = false;

        if !wp.is_null() && !(*wp).flags.intersects(window_pane_flags::PANE_EXITED) {
            if wp != (*(*wp).window).active {
                focused = false;
            } else {
                for c in tailq_foreach(&raw mut CLIENTS).map(NonNull::as_ptr) {
                    if !(*c).session.is_null()
                        && (*(*c).session).attached != 0
                        && (*c).flags.intersects(client_flag::FOCUSED)
                        && (*(*(*c).session).curw).window == (*wp).window
                    {
                        focused = true;
                        break;
                    }
                }
            }
            if !focused && (*wp).flags.intersects(window_pane_flags::PANE_FOCUSED) {
                log_debug!("{}: %%{} focus out", "window_pane_update_focus", (*wp).id);
                if (*wp).base.mode.intersects(mode_flag::MODE_FOCUSON) {
                    bufferevent_write((*wp).event, c!("\x1b[O") as _, 3);
                }
                notify_pane(c"pane-focus-out", wp);
                (*wp).flags &= !window_pane_flags::PANE_FOCUSED;
            } else if focused && !(*wp).flags.intersects(window_pane_flags::PANE_FOCUSED) {
                log_debug!("{}: %%{} focus in", "window_pane_update_focus", (*wp).id);
                if (*wp).base.mode.intersects(mode_flag::MODE_FOCUSON) {
                    bufferevent_write((*wp).event, c!("\x1b[I") as _, 3);
                }
                notify_pane(c"pane-focus-in", wp);
                (*wp).flags |= window_pane_flags::PANE_FOCUSED;
            } else {
                log_debug!(
                    "{}: %%{} focus unchanged",
                    "window_pane_update_focus",
                    (*wp).id,
                );
            }
        }
    }
}

pub unsafe fn window_set_active_pane(w: *mut window, wp: *mut window_pane, notify: i32) -> i32 {
    static NEXT_ACTIVE_POINT: AtomicU32 = AtomicU32::new(0);

    let lastwp: *mut window_pane;
    unsafe {
        log_debug!("{}: pane %%{}", "window_set_active_pane", (*wp).id);

        if wp == (*w).active {
            return 0;
        }
        lastwp = (*w).active;

        window_pane_stack_remove(&raw mut (*w).last_panes, wp);
        window_pane_stack_push(&raw mut (*w).last_panes, lastwp);

        (*w).active = wp;
        (*(*w).active).active_point = NEXT_ACTIVE_POINT.fetch_add(1, atomic::Ordering::Relaxed);
        (*(*w).active).flags |= window_pane_flags::PANE_CHANGED;

        if options_get_number_(GLOBAL_OPTIONS, "focus-events") != 0 {
            window_pane_update_focus(lastwp);
            window_pane_update_focus((*w).active);
        }

        tty_update_window_offset(w);

        if notify != 0 {
            notify_window(c"window-pane-changed", w);
        }
    }
    1
}

fn window_pane_get_palette(wp: Option<&window_pane>, c: i32) -> i32 {
    if let Some(wp) = wp {
        colour_palette_get(Some(&wp.palette), c)
    } else {
        -1
    }
}

pub unsafe fn window_redraw_active_switch(w: *mut window, mut wp: *mut window_pane) {
    unsafe {
        if wp == (*w).active {
            return;
        }

        loop {
            // If the active and inactive styles or palettes are different,
            // need to redraw the panes.
            let gc1 = &raw mut (*wp).cached_gc;
            let gc2 = &raw mut (*wp).cached_active_gc;
            if grid_cells_look_equal(gc1, gc2) == 0 {
                (*wp).flags |= window_pane_flags::PANE_REDRAW;
            } else {
                let mut c1 = window_pane_get_palette(ptr_to_ref(wp), (*gc1).fg);
                let mut c2 = window_pane_get_palette(ptr_to_ref(wp), (*gc2).fg);
                if c1 != c2 {
                    (*wp).flags |= window_pane_flags::PANE_REDRAW;
                } else {
                    c1 = window_pane_get_palette(ptr_to_ref(wp), (*gc1).bg);
                    c2 = window_pane_get_palette(ptr_to_ref(wp), (*gc2).bg);
                    if c1 != c2 {
                        (*wp).flags |= window_pane_flags::PANE_REDRAW;
                    }
                }
            }
            if wp == (*w).active {
                break;
            }
            wp = (*w).active;
        }
    }
}

pub unsafe fn window_get_active_at(w: *mut window, x: u32, y: u32) -> *mut window_pane {
    unsafe {
        for wp in tailq_foreach::<_, discr_entry>(&raw mut (*w).panes).map(NonNull::as_ptr) {
            if !window_pane_visible(wp) {
                continue;
            }
            if x < (*wp).xoff || x > (*wp).xoff + (*wp).sx {
                continue;
            }
            if y < (*wp).yoff || y > (*wp).yoff + (*wp).sy {
                continue;
            }
            return wp;
        }
        null_mut()
    }
}

pub unsafe fn window_find_string(w: *mut window, s: *const u8) -> *mut window_pane {
    unsafe {
        let mut top: u32 = 0;
        let mut bottom: u32 = (*w).sy - 1;

        let mut x = (*w).sx / 2;
        let mut y = (*w).sy / 2;

        let status: Result<pane_status, _> =
            (options_get_number_((*w).options, "pane-border-status") as i32).try_into();
        match status {
            Ok(pane_status::PANE_STATUS_TOP) => top += 1,
            Ok(pane_status::PANE_STATUS_BOTTOM) => bottom -= 1,
            _ => (),
        }

        if strcasecmp(s, c!("top")) == 0 {
            y = top;
        } else if strcasecmp(s, c!("bottom")) == 0 {
            y = bottom;
        } else if strcasecmp(s, c!("left")) == 0 {
            x = 0;
        } else if strcasecmp(s, c!("right")) == 0 {
            x = (*w).sx - 1;
        } else if strcasecmp(s, c!("top-left")) == 0 {
            x = 0;
            y = top;
        } else if strcasecmp(s, c!("top-right")) == 0 {
            x = (*w).sx - 1;
            y = top;
        } else if strcasecmp(s, c!("bottom-left")) == 0 {
            x = 0;
            y = bottom;
        } else if strcasecmp(s, c!("bottom-right")) == 0 {
            x = (*w).sx - 1;
            y = bottom;
        } else {
            return null_mut();
        }

        window_get_active_at(w, x, y)
    }
}

pub unsafe fn window_zoom(wp: *mut window_pane) -> i32 {
    unsafe {
        let w = (*wp).window;

        if (*w).flags.intersects(window_flag::ZOOMED) {
            return -1;
        }

        if window_count_panes(w) == 1 {
            return -1;
        }

        if (*w).active != wp {
            window_set_active_pane(w, wp, 1);
        }

        for wp1 in tailq_foreach::<_, discr_entry>(&raw mut (*w).panes).map(NonNull::as_ptr) {
            (*wp1).saved_layout_cell = (*wp1).layout_cell;
            (*wp1).layout_cell = null_mut();
        }

        (*w).saved_layout_root = (*w).layout_root;
        layout_init(w, wp);
        (*w).flags |= window_flag::ZOOMED;
        notify_window(c"window-layout-changed", w);

        0
    }
}

pub unsafe fn window_unzoom(w: *mut window, notify: i32) -> i32 {
    unsafe {
        if !(*w).flags.intersects(window_flag::ZOOMED) {
            return -1;
        }

        (*w).flags &= !window_flag::ZOOMED;
        layout_free(w);
        (*w).layout_root = (*w).saved_layout_root;
        (*w).saved_layout_root = null_mut();

        for wp in tailq_foreach::<_, discr_entry>(&raw mut (*w).panes).map(NonNull::as_ptr) {
            (*wp).layout_cell = (*wp).saved_layout_cell;
            (*wp).saved_layout_cell = null_mut();
        }
        layout_fix_panes(w, null_mut());

        if notify != 0 {
            notify_window(c"window-layout-changed", w);
        }

        0
    }
}

pub unsafe fn window_push_zoom(w: *mut window, always: bool, flag: bool) -> bool {
    unsafe {
        log_debug!(
            "{}: @{} {}",
            "window_push_zoom",
            (*w).id,
            (flag && (*w).flags.intersects(window_flag::ZOOMED)) as i32,
        );
        if flag && (always || (*w).flags.intersects(window_flag::ZOOMED)) {
            (*w).flags |= window_flag::WASZOOMED;
        } else {
            (*w).flags &= !window_flag::WASZOOMED;
        }

        window_unzoom(w, 1) == 0
    }
}

pub unsafe fn window_pop_zoom(w: *mut window) -> i32 {
    unsafe {
        log_debug!(
            "{}: @{} {}",
            "window_pop_zoom",
            (*w).id,
            (*w).flags.intersects(window_flag::WASZOOMED) as i32,
        );
        if (*w).flags.intersects(window_flag::WASZOOMED) {
            return if window_zoom((*w).active) == 0 { 1 } else { 0 };
        }
    }

    0
}

pub unsafe fn window_add_pane(
    w: *mut window,
    mut other: *mut window_pane,
    hlimit: u32,
    flags: spawn_flags,
) -> *mut window_pane {
    let func = "window_add_pane";
    unsafe {
        if other.is_null() {
            other = (*w).active;
        }

        let wp = window_pane_create(w, (*w).sx, (*w).sy, hlimit);
        if tailq_empty(&raw mut (*w).panes) {
            log_debug!("{}: @{} at start", func, (*w).id);
            tailq_insert_head::<_, discr_entry>(&raw mut (*w).panes, wp);
        } else if flags.intersects(SPAWN_BEFORE) {
            log_debug!("{}: @{} before %%{}", func, (*w).id, (*wp).id);
            if flags.intersects(SPAWN_FULLSIZE) {
                tailq_insert_head::<_, discr_entry>(&raw mut (*w).panes, wp);
            } else {
                tailq_insert_before::<_, discr_entry>(other, wp);
            }
        } else {
            log_debug!("{}: @{} after %%{}", func, (*w).id, (*wp).id);
            if flags.intersects(SPAWN_FULLSIZE) {
                tailq_insert_tail::<_, discr_entry>(&raw mut (*w).panes, wp);
            } else {
                tailq_insert_after::<_, discr_entry>(&raw mut (*w).panes, other, wp);
            }
        }

        wp
    }
}

pub unsafe fn window_lost_pane(w: *mut window, wp: *mut window_pane) {
    unsafe {
        log_debug!("{}: @{} pane %%{}", "window_lost_pane", (*w).id, (*wp).id);

        if wp == MARKED_PANE.wp {
            server_clear_marked();
        }

        window_pane_stack_remove(&raw mut (*w).last_panes, wp);
        if wp == (*w).active {
            (*w).active = tailq_first(&raw mut (*w).last_panes);
            if (*w).active.is_null() {
                (*w).active = tailq_prev::<_, _, discr_entry>(wp);
                if (*w).active.is_null() {
                    (*w).active = tailq_next::<_, _, discr_entry>(wp);
                }
            }
            if !(*w).active.is_null() {
                window_pane_stack_remove(&raw mut (*w).last_panes, (*w).active);
                (*(*w).active).flags |= window_pane_flags::PANE_CHANGED;
                notify_window(c"window-pane-changed", w);
                window_update_focus(w);
            }
        }
    }
}

pub unsafe fn window_remove_pane(w: *mut window, wp: *mut window_pane) {
    unsafe {
        window_lost_pane(w, wp);

        tailq_remove::<_, discr_entry>(&raw mut (*w).panes, wp);
        window_pane_destroy(wp);
    }
}

pub unsafe fn window_pane_at_index(w: *mut window, idx: u32) -> *mut window_pane {
    unsafe {
        let mut n: u32 = options_get_number_((*w).options, "pane-base-index") as _;

        for wp in tailq_foreach::<_, discr_entry>(&raw mut (*w).panes).map(NonNull::as_ptr) {
            if n == idx {
                return wp;
            }
            n += 1;
        }

        null_mut()
    }
}

pub unsafe fn window_pane_next_by_number(
    w: *mut window,
    mut wp: *mut window_pane,
    n: u32,
) -> *mut window_pane {
    unsafe {
        for _ in 0..n {
            wp = tailq_next::<_, _, discr_entry>(wp);
            if wp.is_null() {
                wp = tailq_first(&raw mut (*w).panes);
            }
        }
    }

    wp
}

pub unsafe fn window_pane_previous_by_number(
    w: *mut window,
    mut wp: *mut window_pane,
    n: u32,
) -> *mut window_pane {
    unsafe {
        for _ in 0..n {
            wp = tailq_prev::<_, _, discr_entry>(wp);
            if wp.is_null() {
                wp = tailq_last(&raw mut (*w).panes);
            }
        }
    }

    wp
}

pub unsafe fn window_pane_index(wp: *mut window_pane, i: *mut u32) -> i32 {
    unsafe {
        let w = (*wp).window;

        *i = options_get_number_((*w).options, "pane-base-index") as _;
        for wq in tailq_foreach::<_, discr_entry>(&raw mut (*w).panes).map(NonNull::as_ptr) {
            if wp == wq {
                return 0;
            }
            (*i) += 1;
        }
        -1
    }
}

pub unsafe fn window_count_panes(w: *mut window) -> u32 {
    unsafe { tailq_foreach::<_, discr_entry>(&raw mut (*w).panes).count() as u32 }
}

pub unsafe fn window_destroy_panes(w: *mut window) {
    let mut wp: *mut window_pane;
    unsafe {
        while !tailq_empty(&raw mut (*w).last_panes) {
            wp = tailq_first(&raw mut (*w).last_panes);
            window_pane_stack_remove(&raw mut (*w).last_panes, wp);
        }

        while !tailq_empty(&raw mut (*w).panes) {
            wp = tailq_first(&raw mut (*w).panes);
            tailq_remove::<_, discr_entry>(&raw mut (*w).panes, wp);
            window_pane_destroy(wp);
        }
    }
}

pub unsafe fn window_printable_flags(wl: *mut winlink, escape: i32) -> *const u8 {
    static mut FLAGS: [u8; 32] = [0; 32];

    unsafe {
        let s = (*wl).session;

        let mut pos = 0;
        if (*wl).flags.intersects(winlink_flags::WINLINK_ACTIVITY) {
            FLAGS[pos] = b'#';
            pos += 1;
            if escape != 0 {
                FLAGS[pos] = b'#';
                pos += 1;
            }
        }
        if (*wl).flags.intersects(winlink_flags::WINLINK_BELL) {
            FLAGS[pos] = b'!';
            pos += 1;
        }
        if (*wl).flags.intersects(winlink_flags::WINLINK_SILENCE) {
            FLAGS[pos] = b'~';
            pos += 1;
        }
        if wl == (*s).curw {
            FLAGS[pos] = b'*';
            pos += 1;
        }
        if wl == tailq_first(&raw mut (*s).lastw) {
            FLAGS[pos] = b'-';
            pos += 1;
        }
        if server_check_marked() && wl == MARKED_PANE.wl {
            FLAGS[pos] = b'M';
            pos += 1;
        }
        if (*(*wl).window).flags.intersects(window_flag::ZOOMED) {
            FLAGS[pos] = b'Z';
            pos += 1;
        }
        FLAGS[pos] = b'\0';
        &raw mut FLAGS as *mut u8
    }
}

pub unsafe fn window_pane_find_by_id_str(s: *const u8) -> *mut window_pane {
    unsafe {
        if *s != b'%' {
            return null_mut();
        }

        match strtonum(s.add(1), 0, u32::MAX) {
            Ok(id) => window_pane_find_by_id(id),
            Err(_errstr) => null_mut(),
        }
    }
}

pub unsafe fn window_pane_find_by_id(id: u32) -> *mut window_pane {
    unsafe {
        let mut wp: window_pane = zeroed();
        wp.id = id;
        rb_find(&raw mut ALL_WINDOW_PANES, &raw mut wp)
    }
}

pub unsafe fn window_pane_create(
    w: *mut window,
    sx: u32,
    sy: u32,
    hlimit: u32,
) -> *mut window_pane {
    static NEXT_WINDOW_PANE_ID: AtomicU32 = AtomicU32::new(0);

    unsafe {
        let mut host: [u8; HOST_NAME_MAX + 1] = zeroed();
        let wp: *mut window_pane = xcalloc_::<window_pane>(1).as_ptr();
        (*wp).window = w;
        (*wp).options = options_create((*w).options);
        (*wp).flags = window_pane_flags::PANE_STYLECHANGED;

        (*wp).id = NEXT_WINDOW_PANE_ID.fetch_add(1, atomic::Ordering::Relaxed);

        rb_insert(&raw mut ALL_WINDOW_PANES, wp);

        (*wp).fd = -1;

        tailq_init(&raw mut (*wp).modes);

        tailq_init(&raw mut (*wp).resize_queue);

        (*wp).sx = sx;
        (*wp).sy = sy;

        (*wp).pipe_fd = -1;

        (*wp).control_bg = -1;
        (*wp).control_fg = -1;

        (*wp).palette = colour_palette_init();
        colour_palette_from_option(Some(&mut (*wp).palette), (*wp).options);

        screen_init(&raw mut (*wp).base, sx, sy, hlimit);
        (*wp).screen = &raw mut (*wp).base;
        window_pane_default_cursor(wp);

        screen_init(&raw mut (*wp).status_screen, 1, 1, 0);

        if gethostname(host.as_mut_ptr(), size_of_val(&host)) == 0 {
            screen_set_title(&raw mut (*wp).base, host.as_ptr());
        }

        wp
    }
}

unsafe fn window_pane_destroy(wp: *mut window_pane) {
    unsafe {
        window_pane_reset_mode_all(wp);
        free((*wp).searchstr as _);

        if (*wp).fd != -1 {
            #[cfg(feature = "utempter")]
            {
                utempter_remove_record((*wp).fd);
            }
            bufferevent_free((*wp).event);
            close((*wp).fd);
        }
        if !(*wp).ictx.is_null() {
            input_free((*wp).ictx);
        }

        screen_free(&raw mut (*wp).status_screen);

        screen_free(&raw mut (*wp).base);

        if (*wp).pipe_fd != -1 {
            bufferevent_free((*wp).pipe_event);
            close((*wp).pipe_fd);
        }

        if event_initialized(&raw mut (*wp).resize_timer) != 0 {
            event_del(&raw mut (*wp).resize_timer);
        }
        for r in tailq_foreach(&raw mut (*wp).resize_queue).map(NonNull::as_ptr) {
            tailq_remove::<_, ()>(&raw mut (*wp).resize_queue, r);
            free_(r);
        }

        rb_remove(&raw mut ALL_WINDOW_PANES, wp);

        options_free((*wp).options);
        free((*wp).cwd as _);
        free((*wp).shell as _);
        cmd_free_argv((*wp).argc, (*wp).argv);
        colour_palette_free(Some(&mut (*wp).palette));
        free(wp as _);
    }
}

unsafe extern "C-unwind" fn window_pane_read_callback(_bufev: *mut bufferevent, data: *mut c_void) {
    unsafe {
        let wp: *mut window_pane = data as _;
        let evb: *mut evbuffer = (*(*wp).event).input;
        let wpo: *mut window_pane_offset = &raw mut (*wp).pipe_offset;
        let size = EVBUFFER_LENGTH(evb);
        let mut new_size: usize = 0;

        if (*wp).pipe_fd != -1 {
            let new_data = window_pane_get_new_data(wp, wpo, &raw mut new_size);
            if new_size > 0 {
                bufferevent_write((*wp).pipe_event, new_data, new_size);
                window_pane_update_used_data(wp, wpo, new_size);
            }
        }

        log_debug!("%%{} has {} bytes", (*wp).id, size);
        for c in tailq_foreach(&raw mut CLIENTS).map(NonNull::as_ptr) {
            if !(*c).session.is_null() && (*c).flags.intersects(client_flag::CONTROL) {
                control_write_output(c, wp);
            }
        }
        input_parse_pane(wp);
        bufferevent_disable((*wp).event, EV_READ);
    }
}

unsafe extern "C-unwind" fn window_pane_error_callback(
    _bufev: *mut bufferevent,
    _what: c_short,
    data: *mut c_void,
) {
    let wp: *mut window_pane = data as _;
    unsafe {
        log_debug!("%%{} error", (*wp).id);
        (*wp).flags |= window_pane_flags::PANE_EXITED;

        if window_pane_destroy_ready(wp) != 0 {
            server_destroy_pane(wp, 1);
        }
    }
}

pub unsafe fn window_pane_set_event(wp: *mut window_pane) {
    unsafe {
        setblocking((*wp).fd, 0);

        (*wp).event = bufferevent_new(
            (*wp).fd,
            Some(window_pane_read_callback),
            None,
            Some(window_pane_error_callback),
            wp as _,
        );
        if (*wp).event.is_null() {
            fatalx("out of memory");
        }
        (*wp).ictx = input_init(wp, (*wp).event, &raw mut (*wp).palette);

        bufferevent_enable((*wp).event, EV_READ | EV_WRITE);
    }
}

pub unsafe fn window_pane_resize(wp: *mut window_pane, sx: u32, sy: u32) {
    unsafe {
        if sx == (*wp).sx && sy == (*wp).sy {
            return;
        }

        let r: *mut window_pane_resize = xmalloc_::<window_pane_resize>().as_ptr();
        (*r).sx = sx;
        (*r).sy = sy;
        (*r).osx = (*wp).sx;
        (*r).osy = (*wp).sy;
        tailq_insert_tail(&raw mut (*wp).resize_queue, r);

        (*wp).sx = sx;
        (*wp).sy = sy;

        log_debug!(
            "{}: %%{} resize {}x{}",
            "window_pane_resize",
            (*wp).id,
            sx,
            sy,
        );
        screen_resize(
            &raw mut (*wp).base,
            sx,
            sy,
            (*wp).base.saved_grid.is_null() as i32,
        );

        if let Some(wme) = NonNull::new(tailq_first(&raw mut (*wp).modes)) {
            ((*(*wme.as_ptr()).mode).resize)(wme, sx, sy);
        }
    }
}

pub unsafe fn window_pane_set_mode(
    wp: *mut window_pane,
    swp: *mut window_pane,
    mode: *const window_mode,
    fs: *mut cmd_find_state,
    args: *mut args,
) -> i32 {
    unsafe {
        if !tailq_empty(&raw mut (*wp).modes) && (*tailq_first(&raw mut (*wp).modes)).mode == mode {
            return 1;
        }

        let mut wme: *mut window_mode_entry = null_mut();
        for wme_ in tailq_foreach(&raw mut (*wp).modes).map(NonNull::as_ptr) {
            wme = wme_;
            if (*wme).mode == mode {
                break;
            }
        }

        if !wme.is_null() {
            tailq_remove::<_, ()>(&raw mut (*wp).modes, wme);
            tailq_insert_head(&raw mut (*wp).modes, wme);
        } else {
            wme = xcalloc_::<window_mode_entry>(1).as_ptr();
            (*wme).wp = wp;
            (*wme).swp = swp;
            (*wme).mode = mode;
            (*wme).prefix = 1;
            tailq_insert_head(&raw mut (*wp).modes, wme);
            (*wme).screen = ((*(*wme).mode).init)(NonNull::new_unchecked(wme), fs, args);
        }

        (*wp).screen = (*wme).screen;
        (*wp).flags |= window_pane_flags::PANE_REDRAW | window_pane_flags::PANE_CHANGED;

        server_redraw_window_borders((*wp).window);
        server_status_window((*wp).window);
        notify_pane(c"pane-mode-changed", wp);

        0
    }
}

pub unsafe fn window_pane_reset_mode(wp: *mut window_pane) {
    let func = "window_pane_reset_mode";
    unsafe {
        if tailq_empty(&raw mut (*wp).modes) {
            return;
        }

        let wme = tailq_first(&raw mut (*wp).modes);
        tailq_remove::<_, ()>(&raw mut (*wp).modes, wme);
        ((*(*wme).mode).free)(NonNull::new(wme).unwrap());
        free(wme as _);

        if let Some(next) = NonNull::new(tailq_first(&raw mut (*wp).modes)) {
            log_debug!(
                "{}: next mode is {}",
                func,
                _s((*(*next.as_ptr()).mode).name.as_ptr())
            );
            (*wp).screen = (*next.as_ptr()).screen;
            ((*(*next.as_ptr()).mode).resize)(next, (*wp).sx, (*wp).sy);
        } else {
            (*wp).flags &= !window_pane_flags::PANE_UNSEENCHANGES;
            log_debug!("{}: no next mode", func);
            (*wp).screen = &raw mut (*wp).base;
        }
        (*wp).flags |= window_pane_flags::PANE_REDRAW | window_pane_flags::PANE_CHANGED;

        server_redraw_window_borders((*wp).window);
        server_status_window((*wp).window);
        notify_pane(c"pane-mode-changed", wp);
    }
}

pub unsafe fn window_pane_reset_mode_all(wp: *mut window_pane) {
    unsafe {
        while !tailq_empty(&raw mut (*wp).modes) {
            window_pane_reset_mode(wp);
        }
    }
}

unsafe fn window_pane_copy_key(wp: *mut window_pane, key: key_code) {
    unsafe {
        for loop_ in
            tailq_foreach::<_, discr_entry>(&raw mut (*(*wp).window).panes).map(NonNull::as_ptr)
        {
            if loop_ != wp
                && tailq_empty(&raw mut (*loop_).modes)
                && (*loop_).fd != -1
                && !(*loop_).flags.intersects(window_pane_flags::PANE_INPUTOFF)
                && window_pane_visible(loop_)
                && options_get_number_((*loop_).options, "synchronize-panes") != 0
            {
                input_key_pane(loop_, key, null_mut());
            }
        }
    }
}

pub unsafe fn window_pane_key(
    wp: *mut window_pane,
    c: *mut client,
    s: *mut session,
    wl: *mut winlink,
    mut key: key_code,
    m: *mut mouse_event,
) -> i32 {
    if KEYC_IS_MOUSE(key) && m.is_null() {
        return -1;
    }
    unsafe {
        if let Some(wme) = NonNull::new(tailq_first(&raw mut (*wp).modes))
            && let Some(key_fn) = (*(*wme.as_ptr()).mode).key
            && !c.is_null()
        {
            key &= !KEYC_MASK_FLAGS;
            key_fn(wme, c, s, wl, key, m);
            return 0;
        }

        if (*wp).fd == -1 || (*wp).flags.intersects(window_pane_flags::PANE_INPUTOFF) {
            return 0;
        }

        if input_key_pane(wp, key, m) != 0 {
            return -1;
        }

        if KEYC_IS_MOUSE(key) {
            return 0;
        }
        if options_get_number_((*wp).options, "synchronize-panes") != 0 {
            window_pane_copy_key(wp, key);
        }
    }

    0
}

pub unsafe fn window_pane_visible(wp: *mut window_pane) -> bool {
    unsafe {
        if !(*(*wp).window).flags.intersects(window_flag::ZOOMED) {
            return true;
        }
        wp == (*(*wp).window).active
    }
}

pub unsafe fn window_pane_exited(wp: *mut window_pane) -> bool {
    unsafe { (*wp).fd == -1 || (*wp).flags.intersects(window_pane_flags::PANE_EXITED) }
}

pub unsafe fn window_pane_search(
    wp: *mut window_pane,
    term: *const u8,
    regex: i32,
    ignore: i32,
) -> u32 {
    unsafe {
        let s: *mut screen = &raw mut (*wp).base;
        let mut r: regex_t = zeroed();
        let mut new: *mut u8 = null_mut();
        let mut flags = 0;

        if regex == 0 {
            if ignore != 0 {
                flags |= FNM_CASEFOLD;
            }
            new = format_nul!("*{}*", _s(term));
        } else {
            if ignore != 0 {
                flags |= REG_ICASE;
            }
            if regcomp(&raw mut r, term, flags | REG_EXTENDED) != 0 {
                return 0;
            }
        }

        let mut i = 0;
        for j in 0..screen_size_y(s) {
            i = j;

            let line = grid_view_string_cells((*s).grid, 0, i, screen_size_x(s));
            for n in (1..=strlen(line)).rev() {
                if isspace(line.add(n - 1) as c_uchar as c_int) == 0 {
                    break;
                }
                *line.add(n - 1) = b'\0' as _;
            }

            log_debug!("{}: {}", "window_pane_search", _s(line));
            let found = if regex == 0 {
                fnmatch(new, line, flags) == 0
            } else {
                regexec(&r, line, 0, null_mut(), 0) == 0
            };
            free(line as _);

            if found {
                break;
            }
        }

        if regex == 0 {
            free(new as _);
        } else {
            regfree(&raw mut r);
        }

        if i == screen_size_y(s) {
            return 0;
        }

        i + 1
    }
}

/// Get MRU pane from a list.
unsafe fn window_pane_choose_best(list: *mut *mut window_pane, size: u32) -> *mut window_pane {
    if size == 0 {
        return null_mut();
    }

    unsafe {
        let mut best = *list;
        for i in 1..size {
            let next = *list.add(i as usize);
            if (*next).active_point > (*best).active_point {
                best = next;
            }
        }
        best
    }
}

/// Find the pane directly above another. We build a list of those adjacent to top edge and then choose the best.
pub unsafe fn window_pane_find_up(wp: *mut window_pane) -> *mut window_pane {
    unsafe {
        if wp.is_null() {
            return null_mut();
        }
        let w = (*wp).window;
        let status: pane_status = (options_get_number_((*w).options, "pane-border-status") as i32)
            .try_into()
            .unwrap();

        let mut list: *mut *mut window_pane = null_mut();
        let mut size = 0;

        let mut edge = (*wp).yoff;
        match status {
            pane_status::PANE_STATUS_TOP => {
                if edge == 1 {
                    edge = (*w).sy + 1;
                }
            }
            pane_status::PANE_STATUS_BOTTOM => {
                if edge == 0 {
                    edge = (*w).sy;
                }
            }
            _ => {
                if edge == 0 {
                    edge = (*w).sy + 1;
                }
            }
        }

        let left = (*wp).xoff;
        let right = (*wp).xoff + (*wp).sx;

        for next in tailq_foreach::<_, discr_entry>(&raw mut (*w).panes).map(NonNull::as_ptr) {
            if next == wp {
                continue;
            }
            if (*next).yoff + (*next).sy + 1 != edge {
                continue;
            }
            let end = (*next).xoff + (*next).sx - 1;

            let mut found = 0;
            #[expect(clippy::if_same_then_else)]
            if (*next).xoff < left && end > right {
                found = 1;
            } else if (*next).xoff >= left && (*next).xoff <= right {
                found = 1;
            } else if end >= left && end <= right {
                found = 1;
            }
            if found == 0 {
                continue;
            }
            list = xreallocarray_::<*mut window_pane>(list, size + 1).as_ptr();
            *list.add(size) = next;
            size += 1;
        }

        let best = window_pane_choose_best(list, size as u32);
        free(list as _);
        best
    }
}

/// Find the pane directly below another.
pub unsafe fn window_pane_find_down(wp: *mut window_pane) -> *mut window_pane {
    unsafe {
        if wp.is_null() {
            return null_mut();
        }
        let w = (*wp).window;
        let status: pane_status = (options_get_number_((*w).options, "pane-border-status") as i32)
            .try_into()
            .unwrap();

        let mut list: *mut *mut window_pane = null_mut();
        let mut size = 0;

        let mut edge = (*wp).yoff + (*wp).sy + 1;
        match status {
            pane_status::PANE_STATUS_TOP => {
                if edge >= (*w).sy {
                    edge = 1;
                }
            }
            pane_status::PANE_STATUS_BOTTOM => {
                if edge >= (*w).sy - 1 {
                    edge = 0;
                }
            }
            _ => {
                if edge >= (*w).sy {
                    edge = 0;
                }
            }
        }

        let left = (*wp).xoff;
        let right = (*wp).xoff + (*wp).sx;

        for next in tailq_foreach::<_, discr_entry>(&raw mut (*w).panes).map(NonNull::as_ptr) {
            if next == wp {
                continue;
            }
            if (*next).yoff != edge {
                continue;
            }
            let end = (*next).xoff + (*next).sx - 1;

            let mut found = 0;
            #[expect(clippy::if_same_then_else)]
            if (*next).xoff < left && end > right {
                found = 1;
            } else if (*next).xoff >= left && (*next).xoff <= right {
                found = 1;
            } else if end >= left && end <= right {
                found = 1;
            }
            if found == 0 {
                continue;
            }
            list = xreallocarray_::<*mut window_pane>(list, size + 1).as_ptr();
            *list.add(size) = next;
            size += 1;
        }

        let best = window_pane_choose_best(list, size as u32);
        free(list as _);
        best
    }
}

/// Find the pane directly to the left of another.
pub unsafe fn window_pane_find_left(wp: *mut window_pane) -> *mut window_pane {
    if wp.is_null() {
        return null_mut();
    }
    unsafe {
        let w = (*wp).window;

        let mut list: *mut *mut window_pane = null_mut();
        let mut size = 0;

        let mut edge = (*wp).xoff;
        if edge == 0 {
            edge = (*w).sx + 1;
        }

        let top = (*wp).yoff;
        let bottom = (*wp).yoff + (*wp).sy;

        for next in tailq_foreach::<_, discr_entry>(&raw mut (*w).panes).map(NonNull::as_ptr) {
            if next == wp {
                continue;
            }
            if (*next).xoff + (*next).sx + 1 != edge {
                continue;
            }
            let end = (*next).yoff + (*next).sy - 1;

            let mut found = false;
            #[expect(clippy::if_same_then_else)]
            if (*next).yoff < top && end > bottom {
                found = true;
            } else if (*next).yoff >= top && (*next).yoff <= bottom {
                found = true;
            } else if end >= top && end <= bottom {
                found = true;
            }
            if !found {
                continue;
            }
            list = xreallocarray_::<*mut window_pane>(list, size + 1).as_ptr();
            *list.add(size) = next;
            size += 1;
        }

        let best = window_pane_choose_best(list, size as u32);
        free(list as _);
        best
    }
}

/// Find the pane directly to the right of another.
pub unsafe fn window_pane_find_right(wp: *mut window_pane) -> *mut window_pane {
    if wp.is_null() {
        return null_mut();
    }
    unsafe {
        let w = (*wp).window;

        let mut list: *mut *mut window_pane = null_mut();
        let mut size = 0;

        let mut edge = (*wp).xoff + (*wp).sx + 1;
        if edge >= (*w).sx {
            edge = 0;
        }

        let top = (*wp).yoff;
        let bottom = (*wp).yoff + (*wp).sy;

        for next in tailq_foreach::<_, discr_entry>(&raw mut (*w).panes).map(NonNull::as_ptr) {
            if next == wp {
                continue;
            }
            if (*next).xoff != edge {
                continue;
            }
            let end = (*next).yoff + (*next).sy - 1;

            let mut found = false;
            #[expect(clippy::if_same_then_else)]
            if (*next).yoff < top && end > bottom {
                found = true;
            } else if (*next).yoff >= top && (*next).yoff <= bottom {
                found = true;
            } else if end >= top && end <= bottom {
                found = true;
            }
            if !found {
                continue;
            }
            list = xreallocarray_::<*mut window_pane>(list, size + 1).as_ptr();
            *list.add(size) = next;
            size += 1;
        }

        let best = window_pane_choose_best(list, size as _);
        free(list as _);
        best
    }
}

pub unsafe fn window_pane_stack_push(stack: *mut window_panes, wp: *mut window_pane) {
    unsafe {
        if !wp.is_null() {
            window_pane_stack_remove(stack, wp);
            tailq_insert_head::<_, discr_sentry>(stack, wp);
            (*wp).flags |= window_pane_flags::PANE_VISITED;
        }
    }
}

pub unsafe fn window_pane_stack_remove(stack: *mut window_panes, wp: *mut window_pane) {
    unsafe {
        if !wp.is_null() && (*wp).flags.intersects(window_pane_flags::PANE_VISITED) {
            tailq_remove::<_, crate::discr_sentry>(stack, wp);
            (*wp).flags &= !window_pane_flags::PANE_VISITED;
        }
    }
}

/// Clear alert flags for a winlink
pub unsafe fn winlink_clear_flags(wl: *mut winlink) {
    unsafe {
        (*(*wl).window).flags &= !WINDOW_ALERTFLAGS;
        for loop_ in tailq_foreach::<_, crate::discr_wentry>(&raw mut (*(*wl).window).winlinks)
            .map(NonNull::as_ptr)
        {
            if (*loop_).flags.intersects(WINLINK_ALERTFLAGS) {
                (*loop_).flags &= !WINLINK_ALERTFLAGS;
                server_status_session((*loop_).session);
            }
        }
    }
}

/// Shuffle window indexes up.
pub unsafe fn winlink_shuffle_up(s: *mut session, mut wl: *mut winlink, before: bool) -> i32 {
    if wl.is_null() {
        return -1;
    }
    unsafe {
        let idx = if before { (*wl).idx } else { (*wl).idx + 1 };

        // Find the next free index.
        let mut last = idx;
        for i in idx..i32::MAX {
            last = i;
            if winlink_find_by_index(&raw mut (*s).windows, last).is_null() {
                break;
            }
        }
        if last == i32::MAX {
            return -1;
        }

        // Move everything from last - 1 to idx up a bit.
        while last > idx {
            wl = winlink_find_by_index(&raw mut (*s).windows, last - 1);
            rb_remove(&raw mut (*s).windows, wl);
            (*wl).idx += 1;
            rb_insert(&raw mut (*s).windows, wl);
            last -= 1;
        }

        idx
    }
}

unsafe fn window_pane_input_callback(
    c: *mut client,
    _path: *mut u8,
    error: i32,
    closed: i32,
    buffer: *mut evbuffer,
    data: *mut c_void,
) {
    unsafe {
        let cdata: *mut window_pane_input_data = data as *mut window_pane_input_data;
        let buf: *mut c_uchar = EVBUFFER_DATA(buffer);
        let len: usize = EVBUFFER_LENGTH(buffer);

        let wp = window_pane_find_by_id((*cdata).wp);
        if !(*cdata).file.is_null() && (wp.is_null() || (*c).flags.intersects(client_flag::DEAD)) {
            if wp.is_null() {
                (*c).retval = 1;
                (*c).flags |= client_flag::EXIT;
            }
            file_cancel((*cdata).file);
        } else if (*cdata).file.is_null() || closed != 0 || error != 0 {
            cmdq_continue((*cdata).item);
            server_client_unref(c);
            free(cdata as _);
        } else {
            input_parse_buffer(wp, buf, len);
            evbuffer_drain(buffer, len);
        }
    }
}

pub unsafe fn window_pane_start_input(
    wp: *mut window_pane,
    item: *mut cmdq_item,
    cause: *mut *mut u8,
) -> i32 {
    unsafe {
        let c: *mut client = cmdq_get_client(item);

        if !(*wp).flags.intersects(window_pane_flags::PANE_EMPTY) {
            *cause = xstrdup(c!("pane is not empty")).cast().as_ptr();
            return -1;
        }
        if (*c)
            .flags
            .intersects(client_flag::DEAD | client_flag::EXITED)
        {
            return 1;
        }
        if !(*c).session.is_null() {
            return 1;
        }

        let cdata: *mut window_pane_input_data = xmalloc_::<window_pane_input_data>().as_ptr();
        (*cdata).item = item;
        (*cdata).wp = (*wp).id;
        (*cdata).file = file_read(c, c!("-"), Some(window_pane_input_callback), cdata as _);
        (*c).references += 1;

        0
    }
}

pub unsafe fn window_pane_get_new_data(
    wp: *mut window_pane,
    wpo: *mut window_pane_offset,
    size: *mut usize,
) -> *mut c_void {
    unsafe {
        let used = (*wpo).used - (*wp).base_offset;

        *size = EVBUFFER_LENGTH((*(*wp).event).input) - used;
        EVBUFFER_DATA((*(*wp).event).input).add(used) as _
    }
}

pub unsafe fn window_pane_update_used_data(
    wp: *mut window_pane,
    wpo: *mut window_pane_offset,
    mut size: usize,
) {
    unsafe {
        let used = (*wpo).used - (*wp).base_offset;

        if size > EVBUFFER_LENGTH((*(*wp).event).input) - used {
            size = EVBUFFER_LENGTH((*(*wp).event).input) - used;
        }
        (*wpo).used += size;
    }
}

pub unsafe fn window_set_fill_character(w: NonNull<window>) {
    let w = w.as_ptr();
    unsafe {
        free((*w).fill_character as _);
        (*w).fill_character = null_mut();

        let value = options_get_string_((*w).options, "fill-character");
        if *value != b'\0' as _ && utf8_isvalid(value) {
            let ud = utf8_fromcstr(value);
            if !ud.is_null() && (*ud).width == 1 {
                (*w).fill_character = ud;
            }
        }
    }
}

pub unsafe fn window_pane_default_cursor(wp: *mut window_pane) {
    unsafe {
        let s = (*wp).screen;

        let c: i32 = options_get_number_((*wp).options, "cursor-colour") as i32;
        (*s).default_ccolour = c;

        let c: i32 = options_get_number_((*wp).options, "cursor-style") as i32;
        (*s).default_mode = mode_flag::empty();
        screen_set_cursor_style(
            c as u32,
            &raw mut (*s).default_cstyle,
            &raw mut (*s).default_mode,
        );
    }
}

pub unsafe fn window_pane_mode(wp: *mut window_pane) -> i32 {
    unsafe {
        if !tailq_first(&raw mut (*wp).modes).is_null() {
            if (*tailq_first(&raw mut (*wp).modes)).mode.addr()
                == (&raw const WINDOW_COPY_MODE).addr()
            {
                return WINDOW_PANE_COPY_MODE;
            }
            if (*tailq_first(&raw mut (*wp).modes)).mode.addr()
                == (&raw const WINDOW_VIEW_MODE).addr()
            {
                return WINDOW_PANE_VIEW_MODE;
            }
        }
        WINDOW_PANE_NO_MODE
    }
}

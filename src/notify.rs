// Copyright (c) 2012 George Nachman <tmux@georgester.com>
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

#[repr(C)]
pub struct notify_entry {
    pub name: *mut u8,
    pub fs: cmd_find_state,
    pub formats: *mut format_tree,

    pub client: *mut client,
    pub session: *mut session,
    pub window: *mut window,
    pub pane: i32,
    pub pbname: *mut u8,
}

pub unsafe fn notify_insert_one_hook(
    item: *mut cmdq_item,
    ne: *mut notify_entry,
    cmdlist: *mut cmd_list,
    state: *mut cmdq_state,
) -> *mut cmdq_item {
    unsafe {
        if cmdlist.is_null() {
            return item;
        }
        if log_get_level() != 0 {
            let s = cmd_list_print(&mut *cmdlist, 0);
            log_debug!(
                "{}: hook {}: {}",
                "notify_insert_one_hook",
                _s((*ne).name),
                _s(s)
            );
            free_(s);
        }
        let new_item = cmdq_get_command(cmdlist, state);
        cmdq_insert_after(item, new_item)
    }
}

pub unsafe fn notify_insert_hook(mut item: *mut cmdq_item, ne: *mut notify_entry) {
    let __func__ = "notify_insert_hook";
    unsafe {
        log_debug!("{}: inserting hook {}", __func__, _s((*ne).name));

        let mut fs: cmd_find_state = zeroed();

        cmd_find_clear_state(&raw mut fs, 0);
        if cmd_find_empty_state(&raw mut (*ne).fs) != 0 || !cmd_find_valid_state(&raw mut (*ne).fs)
        {
            cmd_find_from_nothing(&raw mut fs, 0);
        } else {
            cmd_find_copy_state(&raw mut fs, &raw mut (*ne).fs);
        }

        let mut oo = if fs.s.is_null() {
            global_s_options
        } else {
            (*fs.s).options
        };
        let mut o = options_get(oo, (*ne).name);
        if o.is_null() && !fs.wp.is_null() {
            oo = (*fs.wp).options;
            o = options_get(oo, (*ne).name);
        }
        if o.is_null() && !fs.wl.is_null() {
            oo = (*(*fs.wl).window).options;
            o = options_get(oo, (*ne).name);
        }
        if o.is_null() {
            log_debug!("{}: hook {} not found", __func__, _s((*ne).name));
            return;
        }

        let state = cmdq_new_state(
            &raw mut fs,
            null_mut(),
            cmdq_state_flags::CMDQ_STATE_NOHOOKS,
        );
        cmdq_add_formats(state, (*ne).formats);

        if *(*ne).name == b'@' {
            let value = options_get_string(oo, (*ne).name);
            match cmd_parse_from_string(cstr_to_str(value), None) {
                Err(error) => {
                    log_debug!(
                        "{}: can't parse hook {}: {}",
                        __func__,
                        _s((*ne).name),
                        _s(error)
                    );
                    free_(error);
                }
                Ok(cmdlist) => {
                    notify_insert_one_hook(item, ne, cmdlist, state);
                }
            }
        } else {
            let mut a = options_array_first(o);
            while !a.is_null() {
                let cmdlist = (*options_array_item_value(a)).cmdlist;
                item = notify_insert_one_hook(item, ne, cmdlist, state);
                a = options_array_next(a);
            }
        }

        cmdq_free_state(state);
    }
}

// notify_callback
// notify_add

pub unsafe fn notify_callback(item: *mut cmdq_item, data: *mut c_void) -> cmd_retval {
    let __func__ = c!("notify_callback");
    unsafe {
        let ne = data as *mut notify_entry;

        log_debug!("{}: {}", _s(__func__), _s((*ne).name));

        if streq_((*ne).name, "pane-mode-changed") {
            control_notify_pane_mode_changed((*ne).pane);
        }
        if streq_((*ne).name, "window-layout-changed") {
            control_notify_window_layout_changed((*ne).window);
        }
        if streq_((*ne).name, "window-pane-changed") {
            control_notify_window_pane_changed((*ne).window);
        }
        if streq_((*ne).name, "window-unlinked") {
            control_notify_window_unlinked((*ne).session, (*ne).window);
        }
        if streq_((*ne).name, "window-linked") {
            control_notify_window_linked((*ne).session, (*ne).window);
        }
        if streq_((*ne).name, "window-renamed") {
            control_notify_window_renamed((*ne).window);
        }
        if streq_((*ne).name, "client-session-changed") {
            control_notify_client_session_changed((*ne).client);
        }
        if streq_((*ne).name, "client-detached") {
            control_notify_client_detached((*ne).client);
        }
        if streq_((*ne).name, "session-renamed") {
            control_notify_session_renamed((*ne).session);
        }
        if streq_((*ne).name, "session-created") {
            control_notify_session_created((*ne).session);
        }
        if streq_((*ne).name, "session-closed") {
            control_notify_session_closed((*ne).session);
        }
        if streq_((*ne).name, "session-window-changed") {
            control_notify_session_window_changed((*ne).session);
        }
        if streq_((*ne).name, "paste-buffer-changed") {
            control_notify_paste_buffer_changed((*ne).pbname);
        }
        if streq_((*ne).name, "paste-buffer-deleted") {
            control_notify_paste_buffer_deleted((*ne).pbname);
        }

        notify_insert_hook(item, ne);

        if !(*ne).client.is_null() {
            server_client_unref((*ne).client);
        }
        if !(*ne).session.is_null() {
            session_remove_ref((*ne).session, __func__);
        }
        if !(*ne).window.is_null() {
            window_remove_ref((*ne).window, __func__);
        }

        if !(*ne).fs.s.is_null() {
            session_remove_ref((*ne).fs.s, __func__);
        }

        format_free((*ne).formats);
        free_((*ne).name);
        free_((*ne).pbname);
        free_(ne);
    }

    cmd_retval::CMD_RETURN_NORMAL
}

pub unsafe fn notify_add(
    name: &'static CStr,
    fs: *mut cmd_find_state,
    c: *mut client,
    s: *mut session,
    w: *mut window,
    wp: *mut window_pane,
    pbname: *const u8,
) {
    let __func__ = c!("notify_add");
    unsafe {
        let item = cmdq_running(null_mut());
        if !item.is_null() && cmdq_get_flags(item).intersects(cmdq_state_flags::CMDQ_STATE_NOHOOKS)
        {
            return;
        }

        let ne = xcalloc1::<notify_entry>() as *mut notify_entry;
        (*ne).name = xstrdup(name.as_ptr().cast()).as_ptr();

        (*ne).client = c;
        (*ne).session = s;
        (*ne).window = w;
        (*ne).pane = if !wp.is_null() { (*wp).id as i32 } else { -1 };
        (*ne).pbname = if !pbname.is_null() {
            xstrdup(pbname).as_ptr()
        } else {
            null_mut()
        };

        (*ne).formats = format_create(null_mut(), null_mut(), 0, format_flags::FORMAT_NOJOBS);
        format_add!((*ne).formats, c!("hook"), "{}", _s(name.as_ptr()));
        if !c.is_null() {
            format_add!((*ne).formats, c!("hook_client"), "{}", _s((*c).name),);
        }
        if !s.is_null() {
            format_add!((*ne).formats, c!("hook_session"), "${}", (*s).id);
            format_add!((*ne).formats, c!("hook_session_name"), "{}", _s((*s).name),);
        }
        if !w.is_null() {
            format_add!((*ne).formats, c!("hook_window"), "@{}", (*w).id,);
            format_add!((*ne).formats, c!("hook_window_name"), "{}", _s((*w).name),);
        }
        if !wp.is_null() {
            format_add!((*ne).formats, c!("hook_pane"), "%%{}", (*wp).id,);
        }
        format_log_debug((*ne).formats, __func__);

        if !c.is_null() {
            (*c).references += 1;
        }
        if !s.is_null() {
            session_add_ref(s, __func__);
        }
        if !w.is_null() {
            window_add_ref(w, __func__);
        }

        cmd_find_copy_state(&raw mut (*ne).fs, fs);
        if !(*ne).fs.s.is_null() {
            session_add_ref((*ne).fs.s, __func__);
        } /* cmd_find_valid_state needs session */

        cmdq_append(
            null_mut(),
            cmdq_get_callback!(notify_callback, ne.cast()).as_ptr(),
        );
    }
}

pub unsafe fn notify_hook(item: *mut cmdq_item, name: *mut u8) {
    let __func__ = c!("notify_hook");
    unsafe {
        let target = cmdq_get_target(item);
        let mut ne: notify_entry = zeroed();

        ne.name = name;
        cmd_find_copy_state(&raw mut ne.fs, target);

        ne.client = cmdq_get_client(item);
        ne.session = (*target).s;
        ne.window = (*target).w;
        ne.pane = if !(*target).wp.is_null() {
            (*(*target).wp).id as i32
        } else {
            -1
        };

        ne.formats = format_create(null_mut(), null_mut(), 0, format_flags::FORMAT_NOJOBS);
        format_add!(ne.formats, c!("hook"), "{}", _s(name));
        format_log_debug(ne.formats, __func__);

        notify_insert_hook(item, &raw mut ne);
        format_free(ne.formats);
    }
}

pub unsafe fn notify_client(name: &'static CStr, c: *mut client) {
    unsafe {
        let mut fs: cmd_find_state = zeroed(); // TODO use uninit

        cmd_find_from_client(&raw mut fs, c, 0);
        notify_add(
            name,
            &raw mut fs,
            c,
            null_mut(),
            null_mut(),
            null_mut(),
            null_mut(),
        );
    }
}

pub unsafe fn notify_session(name: &'static CStr, s: *mut session) {
    unsafe {
        let mut fs = zeroed(); // TODO use uninit

        if session_alive(s) {
            cmd_find_from_session(&raw mut fs, s, 0);
        } else {
            cmd_find_from_nothing(&raw mut fs, 0);
        }
        notify_add(
            name,
            &raw mut fs,
            null_mut(),
            s,
            null_mut(),
            null_mut(),
            null_mut(),
        );
    }
}

pub unsafe fn notify_winlink(name: &'static CStr, wl: *mut winlink) {
    unsafe {
        let mut fs: cmd_find_state = zeroed();

        cmd_find_from_winlink(&raw mut fs, wl, 0);
        notify_add(
            name,
            &raw mut fs,
            null_mut(),
            (*wl).session,
            (*wl).window,
            null_mut(),
            null_mut(),
        );
    }
}

pub unsafe fn notify_session_window(name: &'static CStr, s: *mut session, w: *mut window) {
    unsafe {
        let mut fs: cmd_find_state = zeroed();

        cmd_find_from_session_window(&raw mut fs, s, w, 0);
        notify_add(name, &raw mut fs, null_mut(), s, w, null_mut(), null_mut());
    }
}

pub unsafe fn notify_window(name: &'static CStr, w: *mut window) {
    unsafe {
        let mut fs: cmd_find_state = zeroed();

        cmd_find_from_window(&raw mut fs, w, 0);
        notify_add(
            name,
            &raw mut fs,
            null_mut(),
            null_mut(),
            w,
            null_mut(),
            null_mut(),
        );
    }
}

pub unsafe fn notify_pane(name: &'static CStr, wp: *mut window_pane) {
    unsafe {
        let mut fs: cmd_find_state = zeroed();

        cmd_find_from_pane(&raw mut fs, wp, 0);
        notify_add(
            name,
            &raw mut fs,
            null_mut(),
            null_mut(),
            null_mut(),
            wp,
            null_mut(),
        );
    }
}

pub unsafe fn notify_paste_buffer(pbname: *const u8, deleted: i32) {
    unsafe {
        let mut fs: cmd_find_state = zeroed();

        cmd_find_clear_state(&raw mut fs, 0);
        if deleted != 0 {
            notify_add(
                c"paste-buffer-deleted",
                &raw mut fs,
                null_mut(),
                null_mut(),
                null_mut(),
                null_mut(),
                pbname,
            );
        } else {
            notify_add(
                c"paste-buffer-changed",
                &raw mut fs,
                null_mut(),
                null_mut(),
                null_mut(),
                null_mut(),
                pbname,
            );
        }
    }
}

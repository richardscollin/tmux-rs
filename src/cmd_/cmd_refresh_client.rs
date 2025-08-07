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
use crate::libc::{sscanf, strchr};
use crate::*;

pub static CMD_REFRESH_CLIENT_ENTRY: cmd_entry = cmd_entry {
    name: SyncCharPtr::new(c"refresh-client"),
    alias: SyncCharPtr::new(c"refresh"),

    args: args_parse::new(c"A:B:cC:Df:r:F:l::LRSt:U", 0, 1, None),
    usage: SyncCharPtr::new(c"[-cDlLRSU] [-A pane:state] [-B name:what:format] [-C XxY] [-f flags] [-r pane:report] [-t target-client] [adjustment]"),

    flags: cmd_flag::CMD_AFTERHOOK.union(cmd_flag::CMD_CLIENT_TFLAG),
    exec: cmd_refresh_client_exec,
    source: cmd_entry_flag::zeroed(),
    target: cmd_entry_flag::zeroed(),
};

pub unsafe fn cmd_refresh_client_update_subscription(tc: *mut client, value: *const u8) {
    unsafe {
        let subid = -1;
        let copy;
        'out: {
            let name = xstrdup(value).as_ptr();
            copy = name;
            let mut split = strchr(copy, ':' as i32);
            if split.is_null() {
                control_remove_sub(tc, copy);
                break 'out;
            }
            *split = b'\0' as _;
            split = split.add(1);

            let what = split;
            split = strchr(what, ':' as i32);
            if split.is_null() {
                break 'out;
            }
            *split = b'\0';
            split = split.add(1);

            let subtype = if streq_(what, "%*") {
                control_sub_type::CONTROL_SUB_ALL_PANES
            } else if sscanf(what.cast(), c"%%%d".as_ptr(), &subid) == 1 && subid >= 0 {
                control_sub_type::CONTROL_SUB_PANE
            } else if streq_(what, "@*") {
                control_sub_type::CONTROL_SUB_ALL_WINDOWS
            } else if sscanf(what.cast(), c"@%d".as_ptr(), &subid) == 1 && subid >= 0 {
                control_sub_type::CONTROL_SUB_WINDOW
            } else {
                control_sub_type::CONTROL_SUB_SESSION
            };
            control_add_sub(tc, name, subtype, subid, split);
        }

        // out:
        free_(copy);
    }
}

pub unsafe fn cmd_refresh_client_control_client_size(
    self_: *mut cmd,
    item: *mut cmdq_item,
) -> cmd_retval {
    let __func__ = "cmd_refresh_client_control_client_size";
    unsafe {
        let args = cmd_get_args(self_);
        let tc = cmdq_get_target_client(item);
        let size = args_get(args, b'C');
        let mut w: u32 = 0;
        let mut x: u32 = 0;
        let mut y: u32 = 0;
        // u_int w, x, y;
        // struct client_window *cw;

        if sscanf(
            size.cast(),
            c"@%u:%ux%u".as_ptr(),
            &raw mut w,
            &raw mut x,
            &raw mut y,
        ) == 3
        {
            if !(WINDOW_MINIMUM..=WINDOW_MAXIMUM).contains(&x)
                || !(WINDOW_MINIMUM..=WINDOW_MAXIMUM).contains(&y)
            {
                cmdq_error!(item, "size too small or too big");
                return cmd_retval::CMD_RETURN_ERROR;
            }
            log_debug!(
                "{}: client {} window @{}: size {}x{}",
                __func__,
                _s((*tc).name),
                w,
                x,
                y
            );
            let cw = server_client_add_client_window(tc, w).as_ptr();
            (*cw).sx = x;
            (*cw).sy = y;
            (*tc).flags |= client_flag::WINDOWSIZECHANGED;
            recalculate_sizes_now(1);
            return cmd_retval::CMD_RETURN_NORMAL;
        }
        if sscanf(size.cast(), c"@%u:".as_ptr(), &w) == 1 {
            let cw = server_client_get_client_window(tc, w);
            if !cw.is_null() {
                log_debug!(
                    "{}: client {} window @{}: no size",
                    __func__,
                    _s((*tc).name),
                    w
                );
                (*cw).sx = 0;
                (*cw).sy = 0;
                recalculate_sizes_now(1);
            }
            return cmd_retval::CMD_RETURN_NORMAL;
        }

        if sscanf(size.cast(), c"%u,%u".as_ptr(), &x, &y) != 2
            && sscanf(size.cast(), c"%ux%u".as_ptr(), &x, &y) != 2
        {
            cmdq_error!(item, "bad size argument");
            return cmd_retval::CMD_RETURN_ERROR;
        }
        if !(WINDOW_MINIMUM..=WINDOW_MAXIMUM).contains(&x)
            || !(WINDOW_MINIMUM..=WINDOW_MAXIMUM).contains(&y)
        {
            cmdq_error!(item, "size too small or too big");
            return cmd_retval::CMD_RETURN_ERROR;
        }
        tty_set_size(&raw mut (*tc).tty, x, y, 0, 0);
        (*tc).flags |= client_flag::SIZECHANGED;
        recalculate_sizes_now(1);
    }
    cmd_retval::CMD_RETURN_NORMAL
}

pub unsafe fn cmd_refresh_client_update_offset(tc: *mut client, value: *const u8) {
    unsafe {
        let mut pane: u32 = 0;

        if *value != b'%' {
            return;
        }
        let copy = xstrdup(value).as_ptr();
        'out: {
            let mut split = strchr(copy, ':' as i32);
            if split.is_null() {
                break 'out;
            }
            *split = b'\0';
            split = split.add(1);

            if sscanf(copy.cast(), c"%%%u".as_ptr(), &raw mut pane) != 1 {
                break 'out;
            }
            let wp = window_pane_find_by_id(pane);
            if wp.is_null() {
                break 'out;
            }

            if streq_(split, "on") {
                control_set_pane_on(tc, wp);
            } else if streq_(split, "off") {
                control_set_pane_off(tc, wp);
            } else if streq_(split, "continue") {
                control_continue_pane(tc, wp);
            } else if streq_(split, "pause") {
                control_pause_pane(tc, wp);
            }
        }

        // out:
        free_(copy);
    }
}

pub unsafe fn cmd_refresh_client_clipboard(self_: *mut cmd, item: *mut cmdq_item) -> cmd_retval {
    unsafe {
        let args = cmd_get_args(self_);
        let tc = cmdq_get_target_client(item);
        let mut fs: cmd_find_state = zeroed();
        // const char *p;
        // u_int i;
        // struct cmd_find_state fs;

        let p = args_get_(args, 'l');
        if p.is_null() {
            if (*tc).flags.intersects(client_flag::CLIPBOARDBUFFER) {
                return cmd_retval::CMD_RETURN_NORMAL;
            }
            (*tc).flags |= client_flag::CLIPBOARDBUFFER;
        } else {
            if cmd_find_target(
                &raw mut fs,
                item,
                p,
                cmd_find_type::CMD_FIND_PANE,
                cmd_find_flags::empty(),
            ) != 0
            {
                return cmd_retval::CMD_RETURN_ERROR;
            }
            let mut i = 0;
            for j in 0..(*tc).clipboard_npanes {
                i = j;
                if *(*tc).clipboard_panes.add(i as usize) == (*fs.wp).id {
                    break;
                }
            }
            if i != (*tc).clipboard_npanes {
                return cmd_retval::CMD_RETURN_NORMAL;
            }
            (*tc).clipboard_panes =
                xreallocarray_((*tc).clipboard_panes, (*tc).clipboard_npanes as usize + 1).as_ptr();
            *(*tc).clipboard_panes.add((*tc).clipboard_npanes as usize) = (*fs.wp).id;
            (*tc).clipboard_npanes += 1;
        }
        tty_clipboard_query(&raw mut (*tc).tty);
    }
    cmd_retval::CMD_RETURN_NORMAL
}

pub unsafe fn cmd_refresh_report(tty: *mut tty, value: *const u8) {
    unsafe {
        let pane: u32 = 0;
        let mut size: usize = 0;

        if *value != b'%' as _ {
            return;
        }
        let copy = xstrdup(value).as_ptr();
        'out: {
            let mut split = strchr(copy, ':' as i32);
            if split.is_null() {
                break 'out;
            }
            *split = b'\0' as _;
            split = split.add(1);

            if sscanf(copy.cast(), c"%%%u".as_ptr(), &pane) != 1 {
                break 'out;
            }
            let wp = window_pane_find_by_id(pane);
            if wp.is_null() {
                break 'out;
            }

            tty_keys_colours(
                tty,
                split,
                strlen(split),
                &raw mut size,
                &raw mut (*wp).control_fg,
                &raw mut (*wp).control_bg,
            );
        }
        // out:
        free_(copy);
    }
}

pub unsafe fn cmd_refresh_client_exec(self_: *mut cmd, item: *mut cmdq_item) -> cmd_retval {
    unsafe {
        let args = cmd_get_args(self_);
        let tc = cmdq_get_target_client(item);
        let tty = &raw mut (*tc).tty;

        'not_control_client: {
            if args_has(args, 'c')
                || args_has(args, 'L')
                || args_has(args, 'R')
                || args_has(args, 'U')
                || args_has(args, 'D')
            {
                let adjust = if args_count(args) == 0 {
                    1
                } else {
                    match strtonum(args_string(args, 0), 1, i32::MAX) {
                        Ok(n) => n as u32,
                        Err(errstr) => {
                            cmdq_error!(item, "adjustment {}", _s(errstr.as_ptr()));
                            return cmd_retval::CMD_RETURN_ERROR;
                        }
                    }
                };

                if args_has(args, 'c') {
                    (*tc).pan_window = null_mut();
                } else {
                    let w = (*(*(*tc).session).curw).window;
                    if (*tc).pan_window != w.cast() {
                        (*tc).pan_window = w.cast();
                        (*tc).pan_ox = (*tty).oox;
                        (*tc).pan_oy = (*tty).ooy;
                    }
                    if args_has(args, 'L') {
                        if (*tc).pan_ox > adjust {
                            (*tc).pan_ox -= adjust;
                        } else {
                            (*tc).pan_ox = 0;
                        }
                    } else if args_has(args, 'R') {
                        (*tc).pan_ox += adjust;
                        if (*tc).pan_ox > (*w).sx - (*tty).osx {
                            (*tc).pan_ox = (*w).sx - (*tty).osx;
                        }
                    } else if args_has(args, 'U') {
                        if (*tc).pan_oy > adjust {
                            (*tc).pan_oy -= adjust;
                        } else {
                            (*tc).pan_oy = 0;
                        }
                    } else if args_has(args, 'D') {
                        (*tc).pan_oy += adjust;
                        if (*tc).pan_oy > (*w).sy - (*tty).osy {
                            (*tc).pan_oy = (*w).sy - (*tty).osy;
                        }
                    }
                }
                tty_update_client_offset(tc);
                server_redraw_client(tc);
                return cmd_retval::CMD_RETURN_NORMAL;
            }

            if args_has(args, 'l') {
                return cmd_refresh_client_clipboard(self_, item);
            }

            if args_has(args, 'F') {
                server_client_set_flags(tc, args_get(args, b'F'));
            } /* -F is an alias for -f */
            if args_has(args, 'f') {
                server_client_set_flags(tc, args_get(args, b'f'));
            }
            if args_has(args, 'r') {
                cmd_refresh_report(tty, args_get(args, b'r'));
            }

            if args_has(args, 'A') {
                if !(*tc).flags.intersects(client_flag::CONTROL) {
                    break 'not_control_client;
                }
                let mut av = args_first_value(args, b'A');
                while !av.is_null() {
                    cmd_refresh_client_update_offset(tc, (*av).union_.string);
                    av = args_next_value(av);
                }
                return cmd_retval::CMD_RETURN_NORMAL;
            }
            if args_has(args, 'B') {
                if !(*tc).flags.intersects(client_flag::CONTROL) {
                    break 'not_control_client;
                }
                let mut av = args_first_value(args, b'B');
                while !av.is_null() {
                    cmd_refresh_client_update_subscription(tc, (*av).union_.string);
                    av = args_next_value(av);
                }
                return cmd_retval::CMD_RETURN_NORMAL;
            }
            if args_has(args, 'C') {
                if !(*tc).flags.intersects(client_flag::CONTROL) {
                    break 'not_control_client;
                }
                return cmd_refresh_client_control_client_size(self_, item);
            }

            if args_has(args, 'S') {
                (*tc).flags |= client_flag::STATUSFORCE;
                server_status_client(tc);
            } else {
                (*tc).flags |= client_flag::STATUSFORCE;
                server_redraw_client(tc);
            }
            return cmd_retval::CMD_RETURN_NORMAL;
        }
        // not_control_client:
        cmdq_error!(item, "not a control client");
        cmd_retval::CMD_RETURN_ERROR
    }
}

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
use crate::compat::queue::tailq_foreach;
use crate::libc::strtol;
use crate::options_::options_find_choice;
use crate::*;

pub static CMD_DISPLAY_MENU_ENTRY: cmd_entry = cmd_entry {
    name: "display-menu",
    alias: Some("menu"),

    args: args_parse::new(
        c"b:c:C:H:s:S:MOt:T:x:y:",
        1,
        -1,
        Some(cmd_display_menu_args_parse),
    ),
    usage: "[-MO] [-b border-lines] [-c target-client] [-C starting-choice] [-H selected-style] [-s style] [-S border-style] [-t target-pane][-T title] [-x position] [-y position] name key command ...",
    target: cmd_entry_flag::new(b't', cmd_find_type::CMD_FIND_PANE, cmd_find_flags::empty()),

    flags: cmd_flag::CMD_AFTERHOOK.union(cmd_flag::CMD_CLIENT_CFLAG),
    exec: cmd_display_menu_exec,
    source: cmd_entry_flag::zeroed(),
};

pub static CMD_DISPLAY_POPUP_ENTRY: cmd_entry = cmd_entry {
    name: "display-popup",
    alias: Some("popup"),

    args: args_parse::new(c"Bb:Cc:d:e:Eh:s:S:t:T:w:x:y:", 0, -1, None),
    usage: "[-BCE] [-b border-lines] [-c target-client] [-d start-directory] [-e environment] [-h height] [-s style] [-S border-style] [-t target-pane][-T title] [-w width] [-x position] [-y position] [shell-command]",
    target: cmd_entry_flag::new(b't', cmd_find_type::CMD_FIND_PANE, cmd_find_flags::empty()),

    flags: cmd_flag::CMD_AFTERHOOK.union(cmd_flag::CMD_CLIENT_CFLAG),
    exec: cmd_display_popup_exec,
    source: cmd_entry_flag::zeroed(),
};

unsafe fn cmd_display_menu_args_parse(
    args: *mut args,
    idx: u32,
    _cause: *mut *mut u8,
) -> args_parse_type {
    let mut i: u32 = 0;

    let mut type_;
    loop {
        type_ = args_parse_type::ARGS_PARSE_STRING;
        if i == idx {
            break;
        }

        unsafe {
            if *args_string(args, i) == b'\0' as _ {
                i += 1;
                continue;
            }
            i += 1;
        }

        type_ = args_parse_type::ARGS_PARSE_STRING;
        if i == idx {
            break;
        }
        i += 1;

        type_ = args_parse_type::ARGS_PARSE_COMMANDS_OR_STRING;
        if i == idx {
            break;
        }
        i += 1;
    }
    type_
}

unsafe fn cmd_display_menu_get_position(
    tc: *mut client,
    item: *mut cmdq_item,
    args: *mut args,
    px: *mut u32,
    py: *mut u32,
    w: u32,
    h: u32,
) -> i32 {
    unsafe {
        let tty = &raw mut (*tc).tty;
        let target = cmdq_get_target(item);
        let event = cmdq_get_event(item);
        let s = (*tc).session;
        let wl = (*target).wl;
        let wp = (*target).wp;
        let mut sr = null_mut();
        let mut line: u32 = 0;
        let mut ox: u32 = 0;
        let mut oy: u32 = 0;
        let mut sx: u32 = 0;
        let mut sy: u32 = 0;

        // Work out the position from the -x and -y arguments. This is the
        // bottom-left position.

        // If the popup is too big, stop now.
        if w > (*tty).sx || h > (*tty).sy {
            return 0;
        }

        // Create format with mouse position if any.
        let ft = format_create_from_target(item);
        if (*event).m.valid {
            format_add!(ft, c!("popup_mouse_x"), "{}", (*event).m.x);
            format_add!(ft, c!("popup_mouse_y"), "{}", (*event).m.y);
        }

        // If there are any status lines, add this window position and the
        // status line position.
        let mut top = status_at_line(tc);
        if top != -1 {
            let lines = status_line_size(tc);
            if top == 0 {
                top = lines as i32;
            } else {
                top = 0;
            }
            let position = options_get_number_((*s).options, "status-position");

            for line_ in 0..lines {
                line = line_;
                let ranges = &raw mut (*tc).status.entries[line as usize].ranges;
                for sr_ in tailq_foreach(ranges) {
                    sr = sr_.as_ptr();
                    if (*sr).type_ != style_range_type::STYLE_RANGE_WINDOW {
                        continue;
                    }
                    if (*sr).argument == (*wl).idx as u32 {
                        break;
                    }
                    continue;
                }
                if !sr.is_null() {
                    break;
                }
            }

            if !sr.is_null() {
                format_add!(ft, c!("popup_window_status_line_x"), "{}", (*sr).start,);
                if position == 0 {
                    format_add!(ft, c!("popup_window_status_line_y"), "{}", line + 1 + h,);
                } else {
                    format_add!(
                        ft,
                        c!("popup_window_status_line_y"),
                        "{}",
                        (*tty).sy - lines + line,
                    );
                }
            }

            if position == 0 {
                format_add!(ft, c!("popup_status_line_y"), "{}", lines + h,);
            } else {
                format_add!(ft, c!("popup_status_line_y"), "{}", (*tty).sy - lines,);
            }
        } else {
            top = 0;
        }

        // Popup width and height.
        format_add!(ft, c!("popup_width"), "{w}");
        format_add!(ft, c!("popup_height"), "{h}");

        // Position so popup is in the centre.
        let mut n: c_long = ((*tty).sx - 1) as c_long / 2 - w as c_long / 2;
        if n < 0 {
            format_add!(ft, c!("popup_centre_x"), "0");
        } else {
            format_add!(ft, c!("popup_centre_x"), "{n}");
        }
        n = (((*tty).sy - 1) / 2 + h / 2) as i64;
        if n >= (*tty).sy as i64 {
            format_add!(ft, c!("popup_centre_y"), "{}", (*tty).sy - h,);
        } else {
            format_add!(ft, c!("popup_centre_y"), "{n}");
        }

        // Position of popup relative to mouse.
        if (*event).m.valid {
            n = (*event).m.x as c_long - w as c_long / 2;
            if n < 0 {
                format_add!(ft, c!("popup_mouse_centre_x"), "0");
            } else {
                format_add!(ft, c!("popup_mouse_centre_x"), "{n}");
            }
            n = ((*event).m.y - h / 2) as i64;
            if n + h as c_long >= (*tty).sy as i64 {
                format_add!(ft, c!("popup_mouse_centre_y"), "{}", (*tty).sy - h,);
            } else {
                format_add!(ft, c!("popup_mouse_centre_y"), "{n}");
            }
            n = (*event).m.y as c_long + h as c_long;
            if n >= (*tty).sy as c_long {
                format_add!(ft, c!("popup_mouse_top"), "{}", (*tty).sy - 1,);
            } else {
                format_add!(ft, c!("popup_mouse_top"), "{n}");
            }
            n = ((*event).m.y - h) as c_long;
            if n < 0 {
                format_add!(ft, c!("popup_mouse_bottom"), "0");
            } else {
                format_add!(ft, c!("popup_mouse_bottom"), "{n}");
            }
        }

        // Position in pane.
        tty_window_offset(
            &raw mut (*tc).tty,
            &raw mut ox,
            &raw mut oy,
            &raw mut sx,
            &raw mut sy,
        );
        n = top as i64 + (*wp).yoff as i64 - oy as i64 + h as i64;
        if n >= (*tty).sy as i64 {
            format_add!(ft, c!("popup_pane_top"), "{}", (*tty).sy - h,);
        } else {
            format_add!(ft, c!("popup_pane_top"), "{n}");
        }
        format_add!(
            ft,
            c!("popup_pane_bottom"),
            "{}",
            top + (*wp).yoff as i32 + (*wp).sy as i32 - oy as i32,
        );
        format_add!(ft, c!("popup_pane_left"), "{}", (*wp).xoff - ox,);
        n = (*wp).xoff as c_long + (*wp).sx as i64 - ox as i64 - w as i64;
        if n < 0 {
            format_add!(ft, c!("popup_pane_right"), "0");
        } else {
            format_add!(ft, c!("popup_pane_right"), "{n}");
        }

        // Expand horizontal position.
        let mut xp = args_get_(args, 'x');
        if xp.is_null() || streq_(xp, "C") {
            xp = c!("#{popup_centre_x}");
        } else if streq_(xp, "R") {
            xp = c!("#{popup_pane_right}");
        } else if streq_(xp, "P") {
            xp = c!("#{popup_pane_left}");
        } else if streq_(xp, "M") {
            xp = c!("#{popup_mouse_centre_x}");
        } else if streq_(xp, "W") {
            xp = c!("#{popup_window_status_line_x}");
        }
        let p = format_expand(ft, xp);
        n = strtol(p, null_mut(), 10);
        if n + w as i64 >= (*tty).sx as i64 {
            n = (*tty).sx as i64 - w as i64;
        } else if n < 0 {
            n = 0;
        }
        *px = n as u32;
        log_debug!(
            "{}: -x: {} = {} = {} (-w {})",
            "cmd_display_menu_get_position",
            _s(xp),
            _s(p),
            *px,
            w,
        );
        free_(p);

        // Expand vertical position
        let mut yp = args_get_(args, 'y');
        if yp.is_null() || streq_(yp, "C") {
            yp = c!("#{popup_centre_y}");
        } else if streq_(yp, "P") {
            yp = c!("#{popup_pane_bottom}");
        } else if streq_(yp, "M") {
            yp = c!("#{popup_mouse_top}");
        } else if streq_(yp, "S") {
            yp = c!("#{popup_status_line_y}");
        } else if streq_(yp, "W") {
            yp = c!("#{popup_window_status_line_y}");
        }
        let p = format_expand(ft, yp);
        n = strtol(p, null_mut(), 10);
        if n < h as i64 {
            n = 0;
        } else {
            n -= h as i64;
        }
        if n + h as i64 >= (*tty).sy as i64 {
            n = (*tty).sy as i64 - h as i64;
        } else if n < 0 {
            n = 0;
        }
        *py = n as u32;
        log_debug!(
            "{}: -y: {} = {} = {} (-h {})",
            "cmd_display_menu_get_position",
            _s(yp),
            _s(p),
            *py,
            h,
        );
        free_(p);

        format_free(ft);
        1
    }
}

unsafe fn cmd_display_menu_exec(self_: *mut cmd, item: *mut cmdq_item) -> cmd_retval {
    unsafe {
        let args = cmd_get_args(self_);
        let target = cmdq_get_target(item);
        let event = cmdq_get_event(item);
        let tc = cmdq_get_target_client(item);
        let mut menu_item: menu_item = zeroed();

        let style = args_get_(args, 's');
        let border_style = args_get_(args, 'S');
        let selected_style = args_get_(args, 'H');
        let lines = box_lines::BOX_LINES_DEFAULT;

        let mut cause = null_mut();
        let mut flags = menu_flags::empty();
        let mut starting_choice: i32 = 0;
        let mut px: u32 = 0;
        let mut py: u32 = 0;
        let count = args_count(args);
        let o = (*(*(*(*target).s).curw).window).options;

        if (*tc).overlay_draw.is_some() {
            return cmd_retval::CMD_RETURN_NORMAL;
        }

        if args_has(args, 'C') {
            if streq_(args_get(args, b'C'), "-") {
                starting_choice = -1;
            } else {
                starting_choice =
                    args_strtonum(args, b'C', 0, u32::MAX as i64, &raw mut cause) as i32;
                if !cause.is_null() {
                    cmdq_error!(item, "starting choice {}", _s(cause));
                    free_(cause);
                    return cmd_retval::CMD_RETURN_ERROR;
                }
            }
        }

        let title = if args_has(args, 'T') {
            format_single_from_target(item, args_get(args, b'T'))
        } else {
            xstrdup_(c"").as_ptr()
        };
        let menu = menu_create(title);
        free_(title);

        let mut i: u32 = 0;
        while i != count {
            let name = args_string(args, i);
            i += 1;
            if *name == b'\0' as _ {
                menu_add_item(menu, null_mut(), item, tc, target);
                continue;
            }

            if count - i < 2 {
                cmdq_error!(item, "not enough arguments");
                menu_free(menu);
                return cmd_retval::CMD_RETURN_ERROR;
            }
            let key = args_string(args, i);
            i += 1;

            menu_item.name = SyncCharPtr::from_ptr(name);
            menu_item.key = key_string_lookup_string(key);
            menu_item.command = SyncCharPtr::from_ptr(args_string(args, i));
            i += 1;

            menu_add_item(menu, &raw mut menu_item, item, tc, target);
        }
        if menu.is_null() {
            cmdq_error!(item, "invalid menu arguments");
            return cmd_retval::CMD_RETURN_ERROR;
        }
        if (*menu).items.is_empty() {
            menu_free(menu);
            return cmd_retval::CMD_RETURN_NORMAL;
        }
        if cmd_display_menu_get_position(
            tc,
            item,
            args,
            &raw mut px,
            &raw mut py,
            (*menu).width + 4,
            (*menu).items.len() as u32 + 2,
        ) == 0
        {
            menu_free(menu);
            return cmd_retval::CMD_RETURN_NORMAL;
        }

        let value = args_get_(args, 'b');
        if !value.is_null() {
            let oe = options_get(o, c!("menu-border-lines"));
            if let Err(cause) = options_find_choice(options_table_entry(oe), value) {
                cmdq_error!(item, "menu-border-lines {}", cause.to_str().unwrap());
                return cmd_retval::CMD_RETURN_ERROR;
            }
        }

        if args_has(args, 'O') {
            flags |= menu_flags::MENU_STAYOPEN;
        }
        if !(*event).m.valid && !args_has(args, 'M') {
            flags |= menu_flags::MENU_NOMOUSE;
        }
        if menu_display(
            menu,
            flags,
            starting_choice,
            item,
            px,
            py,
            tc,
            lines,
            style,
            selected_style,
            border_style,
            target,
            None,
            null_mut(),
        ) != 0
        {
            return cmd_retval::CMD_RETURN_NORMAL;
        }
        cmd_retval::CMD_RETURN_WAIT
    }
}

unsafe fn cmd_display_popup_exec(self_: *mut cmd, item: *mut cmdq_item) -> cmd_retval {
    unsafe {
        let args = cmd_get_args(self_);
        let target = cmdq_get_target(item);
        let s = (*target).s;
        let tc = cmdq_get_target_client(item);
        let tty = &raw mut (*tc).tty;
        let style = args_get(args, b's');
        let border_style = args_get(args, b'S');
        let mut cause: *mut u8 = null_mut();
        let mut argc = 0;
        let mut lines = box_lines::BOX_LINES_DEFAULT as i32;
        let mut px = 0;
        let mut py = 0;
        let count = args_count(args);
        let mut env = null_mut();
        let o = (*(*(*s).curw).window).options;

        if args_has(args, 'C') {
            server_client_clear_overlay(tc);
            return cmd_retval::CMD_RETURN_NORMAL;
        }
        if (*tc).overlay_draw.is_some() {
            return cmd_retval::CMD_RETURN_NORMAL;
        }

        let mut h: u32 = (*tty).sy / 2;
        if args_has(args, 'h') {
            h = args_percentage(
                args,
                b'h',
                1,
                (*tty).sy as i64,
                (*tty).sy as i64,
                &raw mut cause,
            ) as u32;
            if !cause.is_null() {
                cmdq_error!(item, "height {}", _s(cause));
                free_(cause);
                return cmd_retval::CMD_RETURN_ERROR;
            }
        }

        let mut w = (*tty).sx / 2;
        if args_has(args, 'w') {
            w = args_percentage(
                args,
                b'w',
                1,
                (*tty).sx as i64,
                (*tty).sx as i64,
                &raw mut cause,
            ) as u32;
            if !cause.is_null() {
                cmdq_error!(item, "width {}", _s(cause));
                free_(cause);
                return cmd_retval::CMD_RETURN_ERROR;
            }
        }

        if w > (*tty).sx {
            w = (*tty).sx;
        }
        if h > (*tty).sy {
            h = (*tty).sy;
        }
        if cmd_display_menu_get_position(tc, item, args, &raw mut px, &raw mut py, w, h) == 0 {
            return cmd_retval::CMD_RETURN_NORMAL;
        }

        let mut value = args_get(args, b'b');
        if args_has(args, 'B') {
            lines = box_lines::BOX_LINES_NONE as i32;
        } else if !value.is_null() {
            let oe = options_get(o, c!("popup-border-lines"));
            lines = match options_find_choice(options_table_entry(oe), value) {
                Ok(ok) => ok,
                Err(cause) => {
                    cmdq_error!(item, "popup-border-lines {}", cause.to_str().unwrap());
                    return cmd_retval::CMD_RETURN_ERROR;
                }
            };
        }

        value = args_get(args, b'd');
        let cwd = if !value.is_null() {
            format_single_from_target(item, value)
        } else {
            xstrdup(server_client_get_cwd(tc, s)).as_ptr()
        };
        let mut shellcmd = null();
        if count == 0 {
            shellcmd = options_get_string_((*s).options, "default-command");
        } else if count == 1 {
            shellcmd = args_string(args, 0);
        }

        let mut argv = null_mut();

        if count <= 1 && (shellcmd.is_null() || *shellcmd == b'\0' as _) {
            shellcmd = null_mut();
            let mut shell = options_get_string_((*s).options, "default-shell");
            if !checkshell_(shell) {
                shell = _PATH_BSHELL;
            }
            cmd_append_argv(&raw mut argc, &raw mut argv, shell);
        } else {
            args_to_vector(args, &raw mut argc, &raw mut argv);
        }

        if args_has(args, 'e') {
            env = environ_create().as_ptr();
            let mut av = args_first_value(args, b'e');
            while !av.is_null() {
                environ_put(env, (*av).union_.string, environ_flags::empty());
                av = args_next_value(av);
            }
        }

        let title = if args_has(args, 'T') {
            format_single_from_target(item, args_get(args, b'T'))
        } else {
            xstrdup_(c"").as_ptr()
        };
        let mut flags = popup_flag::empty();
        if args_has_count(args, b'E') > 1 {
            flags |= popup_flag::POPUP_CLOSEEXITZERO;
        } else if args_has(args, 'E') {
            flags |= popup_flag::POPUP_CLOSEEXIT;
        }
        if popup_display(
            flags,
            box_lines::try_from(lines).unwrap(),
            item,
            px,
            py,
            w,
            h,
            env,
            shellcmd,
            argc,
            argv,
            cwd,
            title,
            tc,
            s,
            style,
            border_style,
            None,
            null_mut(),
        ) != 0
        {
            cmd_free_argv(argc, argv);
            if !env.is_null() {
                environ_free(env);
            }
            free_(cwd);
            free_(title);
            return cmd_retval::CMD_RETURN_NORMAL;
        }
        if !env.is_null() {
            environ_free(env);
        }
        free_(cwd);
        free_(title);
        cmd_free_argv(argc, argv);

        cmd_retval::CMD_RETURN_WAIT
    }
}

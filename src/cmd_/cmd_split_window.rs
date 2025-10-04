// Copyright (c) 2009 Nicholas Marriott <nicholas.marriott@gmail.com>
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

const SPLIT_WINDOW_TEMPLATE: *const u8 = c!("#{session_name}:#{window_index}.#{pane_index}");

pub static CMD_SPLIT_WINDOW_ENTRY: cmd_entry = cmd_entry {
    name: "split-window",
    alias: Some("splitw"),

    args: args_parse::new(c"bc:de:fF:hIl:p:Pt:vZ", 0, -1, None),
    usage: SyncCharPtr::new(c"[-bdefhIPvZ] [-c start-directory] [-e environment] [-F format] [-l size] [-t target-pane][shell-command]"),

    target: cmd_entry_flag::new(b't', cmd_find_type::CMD_FIND_PANE, cmd_find_flags::empty()),

    flags: cmd_flag::empty(),
    exec: cmd_split_window_exec,
    source: cmd_entry_flag::zeroed(),
};

unsafe fn cmd_split_window_exec(self_: *mut cmd, item: *mut cmdq_item) -> cmd_retval {
    unsafe {
        let args = cmd_get_args(self_);
        let current = cmdq_get_current(item);
        let target = cmdq_get_target(item);
        let tc = cmdq_get_target_client(item);
        let s = (*target).s;
        let wl = (*target).wl;
        let w = (*wl).window;
        let wp = (*target).wp;
        let mut cause = null_mut();
        let count = args_count(args);
        let mut curval = 0;

        let mut type_ = layout_type::LAYOUT_TOPBOTTOM;
        if args_has(args, 'h') {
            type_ = layout_type::LAYOUT_LEFTRIGHT;
        }

        // If the 'p' flag is dropped then this bit can be moved into 'l'.
        #[expect(clippy::collapsible_else_if)]
        if args_has(args, 'l') || args_has(args, 'p') {
            if args_has(args, 'f') {
                if type_ == layout_type::LAYOUT_TOPBOTTOM {
                    curval = (*w).sy;
                } else {
                    curval = (*w).sx;
                }
            } else {
                if type_ == layout_type::LAYOUT_TOPBOTTOM {
                    curval = (*wp).sy;
                } else {
                    curval = (*wp).sx;
                }
            }
        }

        let mut size: i32 = -1;
        if args_has(args, 'l') {
            size = args_percentage_and_expand(
                args,
                b'l',
                0,
                i32::MAX as i64,
                curval as _,
                item,
                &raw mut cause,
            ) as _;
        } else if args_has(args, 'p') {
            size = args_strtonum_and_expand(args, b'p', 0, 100, item, &raw mut cause) as _;
            if cause.is_null() {
                size = curval as i32 * size / 100;
            }
        }
        if !cause.is_null() {
            cmdq_error!(item, "size {}", _s(cause));
            free_(cause);
            return cmd_retval::CMD_RETURN_ERROR;
        }

        window_push_zoom((*wp).window, true, args_has(args, 'Z'));
        let mut input = args_has(args, 'I') && count == 0;

        let mut flags = spawn_flags::empty();
        if args_has(args, 'b') {
            flags |= SPAWN_BEFORE;
        }
        if args_has(args, 'f') {
            flags |= SPAWN_FULLSIZE;
        }
        if input || (count == 1 && *args_string(args, 0) == b'\0' as _) {
            flags |= SPAWN_EMPTY;
        }

        let lc = layout_split_pane(wp, type_, size, flags);
        if lc.is_null() {
            cmdq_error!(item, "no space for new pane");
            return cmd_retval::CMD_RETURN_ERROR;
        }

        let mut sc: spawn_context = zeroed();
        sc.item = item;
        sc.s = s;
        sc.wl = wl;

        sc.wp0 = wp;
        sc.lc = lc;

        args_to_vector(args, &raw mut sc.argc, &raw mut sc.argv);
        sc.environ = environ_create().as_ptr();

        let mut av = args_first_value(args, b'e');
        while !av.is_null() {
            environ_put(sc.environ, (*av).union_.string, environ_flags::empty());
            av = args_next_value(av);
        }

        sc.idx = -1;
        sc.cwd = args_get_(args, 'c');

        sc.flags = flags;
        if args_has(args, 'd') {
            sc.flags |= SPAWN_DETACHED;
        }
        if args_has(args, 'Z') {
            sc.flags |= SPAWN_ZOOM;
        }

        let new_wp = spawn_pane(&raw mut sc, &raw mut cause);
        if new_wp.is_null() {
            cmdq_error!(item, "create pane failed: {}", _s(cause));
            free_(cause);
            if !sc.argv.is_null() {
                cmd_free_argv(sc.argc, sc.argv);
            }
            environ_free(sc.environ);
            return cmd_retval::CMD_RETURN_ERROR;
        }
        if input {
            match window_pane_start_input(new_wp, item, &raw mut cause) {
                -1 => {
                    server_client_remove_pane(new_wp);
                    layout_close_pane(new_wp);
                    window_remove_pane((*wp).window, new_wp);
                    cmdq_error!(item, "{}", _s(cause));
                    free_(cause);
                    if !sc.argv.is_null() {
                        cmd_free_argv(sc.argc, sc.argv);
                    }
                    environ_free(sc.environ);
                    return cmd_retval::CMD_RETURN_ERROR;
                }
                1 => {
                    input = false;
                }
                _ => (),
            }
        }
        if !args_has(args, 'd') {
            cmd_find_from_winlink_pane(current, wl, new_wp, cmd_find_flags::empty());
        }
        window_pop_zoom((*wp).window);
        server_redraw_window((*wp).window);
        server_status_session(s);

        if args_has(args, 'P') {
            let mut template = args_get_(args, 'F');
            if template.is_null() {
                template = SPLIT_WINDOW_TEMPLATE;
            }
            let cp = format_single(item, template, tc, s, wl, new_wp);
            cmdq_print!(item, "{}", _s(cp));
            free_(cp);
        }

        let mut fs: cmd_find_state = zeroed(); // TODO use uninit
        cmd_find_from_winlink_pane(&raw mut fs, wl, new_wp, cmd_find_flags::empty());
        cmdq_insert_hook!(s, item, &raw mut fs, "after-split-window");

        if !sc.argv.is_null() {
            cmd_free_argv(sc.argc, sc.argv);
        }
        environ_free(sc.environ);
        if input {
            return cmd_retval::CMD_RETURN_WAIT;
        }
        cmd_retval::CMD_RETURN_NORMAL
    }
}

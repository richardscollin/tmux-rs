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
use crate::*;

use crate::compat::tree::rb_foreach;

const NEW_WINDOW_TEMPLATE: *const u8 = c!("#{session_name}:#{window_index}.#{pane_index}");

pub static cmd_new_window_entry: cmd_entry = cmd_entry {
    name: SyncCharPtr::new(c"new-window"),
    alias: SyncCharPtr::new(c"neww"),

    args: args_parse::new(c"abc:de:F:kn:PSt:", 0, -1, None),
    usage: SyncCharPtr::new(c"[-abdkPS] [-c start-directory] [-e environment] [-F format] [-n window-name] [-t target-window] [shell-command]"),

    target: cmd_entry_flag::new(b't', cmd_find_type::CMD_FIND_WINDOW, CMD_FIND_WINDOW_INDEX),

    flags: cmd_flag::empty(),
    exec: cmd_new_window_exec,
    source: cmd_entry_flag::zeroed(),
};

unsafe fn cmd_new_window_exec(self_: *mut cmd, item: *mut cmdq_item) -> cmd_retval {
    unsafe {
        let args = cmd_get_args(self_);
        let c = cmdq_get_client(item);
        let current = cmdq_get_current(item);
        let target = cmdq_get_target(item);
        let mut sc: spawn_context = zeroed();
        let tc = cmdq_get_target_client(item);
        let s = (*target).s;
        let wl = (*target).wl;
        let mut new_wl: *mut winlink = null_mut();
        let mut idx = (*target).idx;
        // before;
        let mut cause = null_mut();

        /*
         * If -S and -n are given and -t is not and a single window with this
         * name already exists, select it.
         */
        let name = args_get(args, b'n');
        if args_has_(args, 'S') && !name.is_null() && (*target).idx == -1 {
            let expanded = format_single(item, name, c, s, null_mut(), null_mut());
            for wl in rb_foreach(&raw mut (*s).windows).map(NonNull::as_ptr) {
                if libc::strcmp((*(*wl).window).name, expanded) != 0 {
                    continue;
                }
                if new_wl.is_null() {
                    new_wl = wl;
                    continue;
                }
                cmdq_error!(item, "multiple windows named {}", _s(name));
                free_(expanded);
                return cmd_retval::CMD_RETURN_ERROR;
            }

            free_(expanded);
            if !new_wl.is_null() {
                if args_has_(args, 'd') {
                    return cmd_retval::CMD_RETURN_NORMAL;
                }
                if session_set_current(s, new_wl) == 0 {
                    server_redraw_session(s);
                }
                if !c.is_null() && !(*c).session.is_null() {
                    (*(*(*s).curw).window).latest = c as _;
                }
                recalculate_sizes();
                return cmd_retval::CMD_RETURN_NORMAL;
            }
        }

        let before = args_has(args, b'b');
        if args_has_(args, 'a') || before != 0 {
            idx = winlink_shuffle_up(s, wl, before);
            if idx == -1 {
                idx = (*target).idx;
            }
        }

        sc.item = item;
        sc.s = s;
        sc.tc = tc;

        sc.name = args_get(args, b'n');
        args_to_vector(args, &raw mut sc.argc, &raw mut sc.argv);
        sc.environ = environ_create().as_ptr();

        let mut av = args_first_value(args, b'e');
        while !av.is_null() {
            environ_put(sc.environ, (*av).union_.string, 0);
            av = args_next_value(av);
        }

        sc.idx = idx;
        sc.cwd = args_get_(args, 'c');

        sc.flags = 0;
        if args_has_(args, 'd') {
            sc.flags |= SPAWN_DETACHED;
        }
        if args_has_(args, 'k') {
            sc.flags |= SPAWN_KILL;
        }

        let new_wl = spawn_window(&raw mut sc, &raw mut cause);
        if new_wl.is_null() {
            cmdq_error!(item, "create window failed: {}", _s(cause));
            free_(cause);
            if !sc.argv.is_null() {
                cmd_free_argv(sc.argc, sc.argv);
            }
            environ_free(sc.environ);
            return cmd_retval::CMD_RETURN_ERROR;
        }
        if !args_has_(args, 'd') || new_wl == (*s).curw {
            cmd_find_from_winlink(current, new_wl, 0);
            server_redraw_session_group(s);
        } else {
            server_status_session_group(s);
        }

        if args_has_(args, 'P') {
            let mut template = args_get_(args, 'F');
            if template.is_null() {
                template = NEW_WINDOW_TEMPLATE;
            }
            let cp = format_single(item, template, tc, s, new_wl, (*(*new_wl).window).active);
            cmdq_print!(item, "{}", _s(cp));
            free_(cp);
        }

        let mut fs: cmd_find_state = zeroed(); //TODO can be uninit
        cmd_find_from_winlink(&raw mut fs, new_wl, 0);
        cmdq_insert_hook!(s, item, &raw mut fs, "after-new-window");

        if !sc.argv.is_null() {
            cmd_free_argv(sc.argc, sc.argv);
        }
        environ_free(sc.environ);
        cmd_retval::CMD_RETURN_NORMAL
    }
}

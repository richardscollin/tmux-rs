// Copyright (c) 2008 Nicholas Marriott <nicholas.marriott@gmail.com>
// Copyright (c) 2011 Marcel P. Partap <mpartap@gmx.net>
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

pub static CMD_RESPAWN_PANE_ENTRY: cmd_entry = cmd_entry {
    name: SyncCharPtr::new(c"respawn-pane"),
    alias: SyncCharPtr::new(c"respawnp"),

    args: args_parse::new(c"c:e:kt:", 0, -1, None),
    usage: SyncCharPtr::new(
        c"[-k] [-c start-directory] [-e environment] [-t target-pane] [shell-command]",
    ),

    target: cmd_entry_flag::new(b't', cmd_find_type::CMD_FIND_PANE, 0),

    flags: cmd_flag::empty(),
    exec: cmd_respawn_pane_exec,
    source: cmd_entry_flag::zeroed(),
};

unsafe fn cmd_respawn_pane_exec(self_: *mut cmd, item: *mut cmdq_item) -> cmd_retval {
    unsafe {
        let args = cmd_get_args(self_);
        let target = cmdq_get_target(item);
        let mut sc: spawn_context = unsafe { zeroed() };
        let s = (*target).s;
        let wl = (*target).wl;
        let wp = (*target).wp;
        let mut cause = null_mut();

        sc.item = item;
        sc.s = s;
        sc.wl = wl;

        sc.wp0 = wp;

        args_to_vector(args, &raw mut sc.argc, &raw mut sc.argv);
        sc.environ = environ_create().as_ptr();

        let mut av = args_first_value(args, b'e');
        while !av.is_null() {
            environ_put(sc.environ, (*av).union_.string, 0);
            av = args_next_value(av);
        }

        sc.idx = -1;
        sc.cwd = args_get(args, b'c');

        sc.flags = SPAWN_RESPAWN;
        if (args_has(args, b'k')) != 0 {
            sc.flags |= SPAWN_KILL;
        }

        if spawn_pane(&raw mut sc, &raw mut cause).is_null() {
            cmdq_error!(item, "respawn pane failed: {}", _s(cause));
            free_(cause);
            if !sc.argv.is_null() {
                cmd_free_argv(sc.argc, sc.argv);
            }
            environ_free(sc.environ);
            return cmd_retval::CMD_RETURN_ERROR;
        }

        (*wp).flags |= window_pane_flags::PANE_REDRAW;
        server_redraw_window_borders((*wp).window);
        server_status_window((*wp).window);

        if !sc.argv.is_null() {
            cmd_free_argv(sc.argc, sc.argv);
        }
        environ_free(sc.environ);
        cmd_retval::CMD_RETURN_NORMAL
    }
}

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

pub static cmd_kill_pane_entry: cmd_entry = cmd_entry {
    name: SyncCharPtr::new(c"kill-pane"),
    alias: SyncCharPtr::new(c"killp"),

    args: args_parse::new(c"at:", 0, 0, None),
    usage: SyncCharPtr::new(c"[-a] [-t target-client]"),

    target: cmd_entry_flag::new(b't', cmd_find_type::CMD_FIND_PANE, 0),

    flags: cmd_flag::CMD_AFTERHOOK,
    exec: cmd_kill_pane_exec,
    source: cmd_entry_flag::zeroed(),
};

unsafe fn cmd_kill_pane_exec(self_: *mut cmd, item: *mut cmdq_item) -> cmd_retval {
    unsafe {
        let args = cmd_get_args(self_);
        let target = cmdq_get_target(item);
        let wl = (*target).wl;
        let wp = (*target).wp;

        if args_has(args, b'a') != 0 {
            server_unzoom_window((*wl).window);
            for loopwp in
                tailq_foreach::<_, discr_entry>(&raw mut (*(*wl).window).panes).map(NonNull::as_ptr)
            {
                if loopwp == wp {
                    continue;
                }
                server_client_remove_pane(loopwp);
                layout_close_pane(loopwp);
                window_remove_pane((*wl).window, loopwp);
            }
            server_redraw_window((*wl).window);
            return cmd_retval::CMD_RETURN_NORMAL;
        }

        server_kill_pane(wp);
        cmd_retval::CMD_RETURN_NORMAL
    }
}

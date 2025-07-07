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

use crate::compat::queue::{tailq_insert_tail, tailq_remove};

pub static CMD_SWAP_WINDOW_ENTRY: cmd_entry = cmd_entry {
    name: SyncCharPtr::new(c"swap-window"),
    alias: SyncCharPtr::new(c"swapw"),

    args: args_parse::new(c"ds:t:", 0, 0, None),
    usage: SyncCharPtr::new(c"[-d] [-s src-window] [-t dst-window]"),

    source: cmd_entry_flag::new(
        b's',
        cmd_find_type::CMD_FIND_WINDOW,
        CMD_FIND_DEFAULT_MARKED,
    ),
    target: cmd_entry_flag::new(b't', cmd_find_type::CMD_FIND_WINDOW, 0),

    flags: cmd_flag::empty(),
    exec: cmd_swap_window_exec,
};

unsafe fn cmd_swap_window_exec(self_: *mut cmd, item: *mut cmdq_item) -> cmd_retval {
    unsafe {
        let args = cmd_get_args(self_);
        let source = cmdq_get_source(item);
        let target = cmdq_get_target(item);
        let src = (*source).s;
        let dst = (*target).s;
        let wl_src = (*source).wl;
        let wl_dst = (*target).wl;

        let sg_src = session_group_contains(src);
        let sg_dst = session_group_contains(dst);

        if src != dst && !sg_src.is_null() && !sg_dst.is_null() && sg_src == sg_dst {
            cmdq_error!(item, "can't move window, sessions are grouped");
            return cmd_retval::CMD_RETURN_ERROR;
        }

        if (*wl_dst).window == (*wl_src).window {
            return cmd_retval::CMD_RETURN_NORMAL;
        }

        let w_dst = (*wl_dst).window;
        tailq_remove::<_, discr_wentry>(&raw mut (*w_dst).winlinks, wl_dst);
        let w_src = (*wl_src).window;
        tailq_remove::<_, discr_wentry>(&raw mut (*w_src).winlinks, wl_src);

        (*wl_dst).window = w_src;
        tailq_insert_tail::<_, discr_wentry>(&raw mut (*w_src).winlinks, wl_dst);
        (*wl_src).window = w_dst;
        tailq_insert_tail::<_, discr_wentry>(&raw mut (*w_dst).winlinks, wl_src);

        if args_has(args, b'd') != 0 {
            session_select(dst, (*wl_dst).idx);
            if src != dst {
                session_select(src, (*wl_src).idx);
            }
        }
        session_group_synchronize_from(src);
        server_redraw_session_group(src);
        if src != dst {
            session_group_synchronize_from(dst);
            server_redraw_session_group(dst);
        }
        recalculate_sizes();

        cmd_retval::CMD_RETURN_NORMAL
    }
}

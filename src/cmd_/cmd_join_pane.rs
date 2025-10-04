// Copyright (c) 2011 George Nachman <tmux@georgester.com>
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

use crate::compat::queue::{tailq_insert_after, tailq_insert_before, tailq_remove};
use crate::*;

pub static CMD_JOIN_PANE_ENTRY: cmd_entry = cmd_entry {
    name: "join-pane",
    alias: Some("joinp"),

    args: args_parse::new(c"bdfhvp:l:s:t:", 0, 0, None),
    usage: "[-bdfhv] [-l size] [-s src-pane] [-t dst-pane]",

    source: cmd_entry_flag::new(
        b's',
        cmd_find_type::CMD_FIND_PANE,
        cmd_find_flags::CMD_FIND_DEFAULT_MARKED,
    ),
    target: cmd_entry_flag::new(b't', cmd_find_type::CMD_FIND_PANE, cmd_find_flags::empty()),

    flags: cmd_flag::empty(),
    exec: cmd_join_pane_exec,
};

pub static CMD_MOVE_PANE_ENTRY: cmd_entry = cmd_entry {
    name: "move-pane",
    alias: Some("movep"),

    args: args_parse::new(c"bdfhvp:l:s:t:", 0, 0, None),
    usage: "[-bdfhv] [-l size] [-s src-pane] [-t dst-pane]",

    source: cmd_entry_flag::new(
        b's',
        cmd_find_type::CMD_FIND_PANE,
        cmd_find_flags::CMD_FIND_DEFAULT_MARKED,
    ),
    target: cmd_entry_flag::new(b't', cmd_find_type::CMD_FIND_PANE, cmd_find_flags::empty()),

    flags: cmd_flag::empty(),
    exec: cmd_join_pane_exec,
};

unsafe fn cmd_join_pane_exec(self_: *mut cmd, item: *mut cmdq_item) -> cmd_retval {
    unsafe {
        let args = cmd_get_args(self_);
        let current = cmdq_get_current(item);
        let target = cmdq_get_target(item);
        let source = cmdq_get_source(item);
        let mut cause = null_mut();
        let mut type_: layout_type;

        let mut curval: u32 = 0;

        let dst_s = (*target).s;
        let dst_wl = (*target).wl;
        let dst_wp = (*target).wp;
        let dst_w = (*dst_wl).window;
        let dst_idx = (*dst_wl).idx;
        server_unzoom_window(dst_w);

        let src_wl = (*source).wl;
        let src_wp = (*source).wp;
        let src_w = (*src_wl).window;
        server_unzoom_window(src_w);

        if src_wp == dst_wp {
            cmdq_error!(item, "source and target panes must be different");
            return cmd_retval::CMD_RETURN_ERROR;
        }

        type_ = layout_type::LAYOUT_TOPBOTTOM;
        if args_has(args, 'h') {
            type_ = layout_type::LAYOUT_LEFTRIGHT;
        }

        // If the 'p' flag is dropped then this bit can be moved into 'l'.
        if args_has(args, 'l') || args_has(args, 'p') {
            #[expect(clippy::collapsible_else_if)]
            if args_has(args, 'f') {
                if type_ == layout_type::LAYOUT_TOPBOTTOM {
                    curval = (*dst_w).sy;
                } else {
                    curval = (*dst_w).sx;
                }
            } else {
                if type_ == layout_type::LAYOUT_TOPBOTTOM {
                    curval = (*dst_wp).sy;
                } else {
                    curval = (*dst_wp).sx;
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
                curval as i64,
                item,
                &raw mut cause,
            ) as _;
        } else if args_has(args, 'p') {
            size = args_strtonum_and_expand(args, b'l', 0, 100, item, &raw mut cause) as _;
            if cause.is_null() {
                size = curval as i32 * size / 100;
            }
        }
        if !cause.is_null() {
            cmdq_error!(item, "size {}", _s(cause));
            free_(cause);
            return cmd_retval::CMD_RETURN_ERROR;
        }

        let mut flags: spawn_flags = spawn_flags::empty();
        if args_has(args, 'b') {
            flags |= SPAWN_BEFORE;
        }
        if args_has(args, 'f') {
            flags |= SPAWN_FULLSIZE;
        }

        let lc: *mut layout_cell = layout_split_pane(dst_wp, type_, size, flags);
        if lc.is_null() {
            cmdq_error!(item, "create pane failed: pane too small");
            return cmd_retval::CMD_RETURN_ERROR;
        }

        layout_close_pane(src_wp);

        server_client_remove_pane(src_wp);
        window_lost_pane(src_w, src_wp);
        tailq_remove::<_, discr_entry>(&raw mut (*src_w).panes, src_wp);

        (*src_wp).window = dst_w;
        options_set_parent((*src_wp).options, (*dst_w).options);
        (*src_wp).flags |= window_pane_flags::PANE_STYLECHANGED;
        if flags.intersects(SPAWN_BEFORE) {
            tailq_insert_before::<_, discr_entry>(dst_wp, src_wp);
        } else {
            tailq_insert_after::<_, discr_entry>(&raw mut (*dst_w).panes, dst_wp, src_wp);
        }
        layout_assign_pane(lc, src_wp, 0);
        colour_palette_from_option(Some(&mut (*src_wp).palette), (*src_wp).options);

        recalculate_sizes();

        server_redraw_window(src_w);
        server_redraw_window(dst_w);

        if !args_has(args, 'd') {
            window_set_active_pane(dst_w, src_wp, 1);
            session_select(dst_s, dst_idx);
            cmd_find_from_session(current, dst_s, cmd_find_flags::empty());
            server_redraw_session(dst_s);
        } else {
            server_status_session(dst_s);
        }

        if window_count_panes(src_w) == 0 {
            server_kill_window(src_w, 1);
        } else {
            notify_window(c"window-layout-changed", src_w);
        }
        notify_window(c"window-layout-changed", dst_w);

        cmd_retval::CMD_RETURN_NORMAL
    }
}

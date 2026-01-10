// Copyright (c) 2008 Nicholas Marriott <nicholas.marriott@gmail.com>
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
use crate::options_::*;

pub static CMD_MOVE_WINDOW_ENTRY: cmd_entry = cmd_entry {
    name: "move-window",
    alias: Some("movew"),

    args: args_parse::new("abdkrs:t:", 0, 0, None),
    usage: "[-abdkr] [-s src-window] [-t dst-window]",

    source: cmd_entry_flag::new(
        b's',
        cmd_find_type::CMD_FIND_WINDOW,
        cmd_find_flags::empty(),
    ),

    flags: cmd_flag::empty(),
    exec: cmd_move_window_exec,
    target: cmd_entry_flag::zeroed(),
};

pub static CMD_LINK_WINDOW_ENTRY: cmd_entry = cmd_entry {
    name: "link-window",
    alias: Some("linkw"),

    args: args_parse::new("abdks:t:", 0, 0, None),
    usage: "[-abdk] [-s src-window] [-t dst-window]",

    source: cmd_entry_flag::new(
        b's',
        cmd_find_type::CMD_FIND_WINDOW,
        cmd_find_flags::empty(),
    ),

    flags: cmd_flag::empty(),
    exec: cmd_move_window_exec,
    target: cmd_entry_flag::zeroed(),
};

unsafe fn cmd_move_window_exec(self_: *mut cmd, item: *mut cmdq_item) -> cmd_retval {
    unsafe {
        let args = cmd_get_args(self_);
        let source = cmdq_get_source(item);
        let mut target = zeroed();
        let tflag = args_get(args, b't');
        let src = (*source).s;
        let wl = (*source).wl;
        let mut cause = null_mut();

        if args_has(args, 'r') {
            if cmd_find_target(
                &raw mut target,
                item,
                cstr_to_str_(tflag),
                cmd_find_type::CMD_FIND_SESSION,
                cmd_find_flags::CMD_FIND_QUIET,
            ) != 0
            {
                return cmd_retval::CMD_RETURN_ERROR;
            }

            session_renumber_windows(target.s);
            recalculate_sizes();
            server_status_session(target.s);

            return cmd_retval::CMD_RETURN_NORMAL;
        }
        if cmd_find_target(
            &raw mut target,
            item,
            cstr_to_str_(tflag),
            cmd_find_type::CMD_FIND_WINDOW,
            cmd_find_flags::CMD_FIND_WINDOW_INDEX,
        ) != 0
        {
            return cmd_retval::CMD_RETURN_ERROR;
        }
        let dst = target.s;
        let mut idx = target.idx;

        let kflag = args_has(args, 'k');
        let dflag = args_has(args, 'd');
        let sflag = args_has(args, 's');

        let before = args_has(args, 'b');
        if args_has(args, 'a') || before {
            if !target.wl.is_null() {
                idx = winlink_shuffle_up(dst, target.wl, before);
            } else {
                idx = winlink_shuffle_up(dst, (*dst).curw, before);
            }
            if idx == -1 {
                return cmd_retval::CMD_RETURN_ERROR;
            }
        }

        if server_link_window(src, wl, dst, idx, kflag, !dflag, &raw mut cause) != 0 {
            cmdq_error!(item, "{}", _s(cause));
            free_(cause);
            return cmd_retval::CMD_RETURN_ERROR;
        }
        if std::ptr::eq(cmd_get_entry(self_), &CMD_MOVE_WINDOW_ENTRY) {
            server_unlink_window(src, wl);
        }

        // Renumber the winlinks in the src session only, the destination
        // session already has the correct winlink id to us, either
        // automatically or specified by -s.
        if !sflag && options_get_number___::<i64>(&*(*src).options, "renumber-windows") != 0 {
            session_renumber_windows(src);
        }

        recalculate_sizes();

        cmd_retval::CMD_RETURN_NORMAL
    }
}

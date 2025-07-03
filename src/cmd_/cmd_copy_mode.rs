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

use super::*;

pub static mut cmd_copy_mode_entry: cmd_entry = cmd_entry {
    name: c"copy-mode".as_ptr(),
    alias: null_mut(),

    args: args_parse::new(c"deHMs:t:uq", 0, 0, None),
    usage: c"[-deHMuq] [-s src-pane] [-t target-pane]".as_ptr(),

    source: cmd_entry_flag::new(b's', cmd_find_type::CMD_FIND_PANE, 0),
    target: cmd_entry_flag::new(b't', cmd_find_type::CMD_FIND_PANE, 0),

    flags: cmd_flag::CMD_AFTERHOOK,
    exec: Some(cmd_copy_mode_exec),
};

pub static mut cmd_clock_mode_entry: cmd_entry = cmd_entry {
    name: c"clock-mode".as_ptr(),
    alias: null_mut(),

    args: args_parse::new(c"t:", 0, 0, None),
    usage: CMD_TARGET_PANE_USAGE.as_ptr(),

    target: cmd_entry_flag::new(b't', cmd_find_type::CMD_FIND_PANE, 0),
    source: unsafe { zeroed() },

    flags: cmd_flag::CMD_AFTERHOOK,
    exec: Some(cmd_copy_mode_exec),
};

unsafe fn cmd_copy_mode_exec(self_: *mut cmd, item: *mut cmdq_item) -> cmd_retval {
    unsafe {
        let args = cmd_get_args(self_);
        let event = cmdq_get_event(item);
        let source = cmdq_get_source(item);
        let target = cmdq_get_target(item);
        let c = cmdq_get_client(item);
        let mut s = null_mut();
        let wp = (*target).wp;

        if args_has(args, b'q') != 0 {
            window_pane_reset_mode_all(wp);
            return cmd_retval::CMD_RETURN_NORMAL;
        }

        if args_has(args, b'M') != 0 {
            let wp = cmd_mouse_pane(&raw mut (*event).m, &raw mut s, null_mut());
            if wp.is_none() {
                return cmd_retval::CMD_RETURN_NORMAL;
            }
            if c.is_null() || (*c).session != s {
                return cmd_retval::CMD_RETURN_NORMAL;
            }
        }

        if cmd_get_entry(self_) == &raw mut cmd_clock_mode_entry {
            window_pane_set_mode(
                wp,
                null_mut(),
                &raw const window_clock_mode,
                null_mut(),
                null_mut(),
            );
            return cmd_retval::CMD_RETURN_NORMAL;
        }

        let swp = if args_has(args, b's') != 0 {
            (*source).wp
        } else {
            wp
        };
        if window_pane_set_mode(wp, swp, &raw const window_copy_mode, null_mut(), args) == 0
            && args_has(args, b'M') != 0
        {
            window_copy_start_drag(c, &raw mut (*event).m);
        }
        if args_has(args, b'u') != 0 {
            window_copy_pageup(wp, 0);
        }
        if args_has(args, b'd') != 0 {
            window_copy_pagedown(wp, 0, args_has(args, b'e'));
        }

        cmd_retval::CMD_RETURN_NORMAL
    }
}

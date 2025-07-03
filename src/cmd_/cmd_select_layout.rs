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

pub static mut cmd_select_layout_entry: cmd_entry = cmd_entry {
    name: c"select-layout".as_ptr(),
    alias: c"selectl".as_ptr(),

    args: args_parse::new(c"Enopt:", 0, 1, None),
    usage: c"[-Enop] [-t target-pane] [layout-name]".as_ptr(),

    target: cmd_entry_flag::new(b't', cmd_find_type::CMD_FIND_PANE, 0),

    flags: cmd_flag::CMD_AFTERHOOK,
    exec: Some(cmd_select_layout_exec),
    ..unsafe { zeroed() }
};

pub static mut cmd_next_layout_entry: cmd_entry = cmd_entry {
    name: c"next-layout".as_ptr(),
    alias: c"nextl".as_ptr(),

    args: args_parse::new(c"t:", 0, 0, None),
    usage: CMD_TARGET_WINDOW_USAGE.as_ptr(),

    target: cmd_entry_flag::new(b't', cmd_find_type::CMD_FIND_WINDOW, 0),

    flags: cmd_flag::CMD_AFTERHOOK,
    exec: Some(cmd_select_layout_exec),
    ..unsafe { zeroed() }
};

pub static mut cmd_previous_layout_entry: cmd_entry = cmd_entry {
    name: c"previous-layout".as_ptr(),
    alias: c"prevl".as_ptr(),

    args: args_parse::new(c"t:", 0, 0, None),
    usage: CMD_TARGET_WINDOW_USAGE.as_ptr(),

    target: cmd_entry_flag::new(b't', cmd_find_type::CMD_FIND_WINDOW, 0),

    flags: cmd_flag::CMD_AFTERHOOK,
    exec: Some(cmd_select_layout_exec),
    ..unsafe { zeroed() }
};

unsafe fn cmd_select_layout_exec(self_: *mut cmd, item: *mut cmdq_item) -> cmd_retval {
    unsafe {
        let args = cmd_get_args(self_);
        let target = cmdq_get_target(item);
        let wl = (*target).wl;
        let w = (*wl).window;
        let wp = (*target).wp;

        server_unzoom_window(w);

        let mut oldlayout = null_mut();

        'error: {
            'changed: {
                let mut next = cmd_get_entry(self_) == &raw mut cmd_next_layout_entry;
                if args_has_(args, 'n') {
                    next = true;
                }
                let mut previous = cmd_get_entry(self_) == &raw mut cmd_previous_layout_entry;
                if args_has_(args, 'p') {
                    previous = true;
                }

                oldlayout = (*w).old_layout;
                (*w).old_layout = layout_dump((*w).layout_root);

                if next || previous {
                    if next {
                        layout_set_next(w);
                    } else {
                        layout_set_previous(w);
                    }
                    break 'changed;
                }

                if args_has_(args, 'E') {
                    layout_spread_out(wp);
                    break 'changed;
                }

                let layoutname = if args_count(args) != 0 {
                    args_string(args, 0)
                } else if args_has_(args, 'o') {
                    oldlayout
                } else {
                    null()
                };

                if !args_has_(args, 'o') {
                    let layout = if layoutname.is_null() {
                        (*w).lastlayout
                    } else {
                        layout_set_lookup(layoutname)
                    };
                    if layout != -1 {
                        layout_set_select(w, layout as u32);
                        break 'changed;
                    }
                }

                if !layoutname.is_null() {
                    let mut cause = null_mut();
                    if layout_parse(w, layoutname, &raw mut cause) == -1 {
                        cmdq_error!(item, "{}: {}", _s(cause), _s(layoutname));
                        free_(cause);
                        break 'error;
                    }
                    break 'changed;
                }

                free_(oldlayout);
                return cmd_retval::CMD_RETURN_NORMAL;
            }

            // changed:
            free_(oldlayout);
            recalculate_sizes();
            server_redraw_window(w);
            notify_window(c"window-layout-changed", w);
            return cmd_retval::CMD_RETURN_NORMAL;
        }

        // error:
        free_((*w).old_layout);
        (*w).old_layout = oldlayout;
        cmd_retval::CMD_RETURN_ERROR
    }
}

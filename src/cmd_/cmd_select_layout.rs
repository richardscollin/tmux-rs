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

pub static CMD_SELECT_LAYOUT_ENTRY: cmd_entry = cmd_entry {
    name: "select-layout",
    alias: Some("selectl"),

    args: args_parse::new("Enopt:", 0, 1, None),
    usage: "[-Enop] [-t target-pane] [layout-name]",

    target: cmd_entry_flag::new(b't', cmd_find_type::CMD_FIND_PANE, cmd_find_flags::empty()),

    flags: cmd_flag::CMD_AFTERHOOK,
    exec: cmd_select_layout_exec,
    source: cmd_entry_flag::zeroed(),
};

pub static CMD_NEXT_LAYOUT_ENTRY: cmd_entry = cmd_entry {
    name: "next-layout",
    alias: Some("nextl"),

    args: args_parse::new("t:", 0, 0, None),
    usage: "[-t target-window]",

    target: cmd_entry_flag::new(
        b't',
        cmd_find_type::CMD_FIND_WINDOW,
        cmd_find_flags::empty(),
    ),

    flags: cmd_flag::CMD_AFTERHOOK,
    exec: cmd_select_layout_exec,
    source: cmd_entry_flag::zeroed(),
};

pub static CMD_PREVIOUS_LAYOUT_ENTRY: cmd_entry = cmd_entry {
    name: "previous-layout",
    alias: Some("prevl"),

    args: args_parse::new("t:", 0, 0, None),
    usage: "[-t target-window]",

    target: cmd_entry_flag::new(
        b't',
        cmd_find_type::CMD_FIND_WINDOW,
        cmd_find_flags::empty(),
    ),

    flags: cmd_flag::CMD_AFTERHOOK,
    exec: cmd_select_layout_exec,
    source: cmd_entry_flag::zeroed(),
};

unsafe fn cmd_select_layout_exec(self_: *mut cmd, item: *mut cmdq_item) -> cmd_retval {
    unsafe {
        let args = cmd_get_args(self_);
        let target = cmdq_get_target(item);
        let wl = (*target).wl;
        let w = (*wl).window;
        let wp = (*target).wp;

        server_unzoom_window(w);

        let oldlayout;
        'error: {
            'changed: {
                let mut next = std::ptr::eq(cmd_get_entry(self_), &CMD_NEXT_LAYOUT_ENTRY);
                if args_has(args, 'n') {
                    next = true;
                }
                let mut previous = std::ptr::eq(cmd_get_entry(self_), &CMD_PREVIOUS_LAYOUT_ENTRY);
                if args_has(args, 'p') {
                    previous = true;
                }

                oldlayout = (*w).old_layout;
                (*w).old_layout = layout_dump((*w).layout_root)
                    .map(|s| CString::new(s).unwrap().into_raw().cast())
                    .unwrap_or_default();

                if next || previous {
                    if next {
                        layout_set_next(w);
                    } else {
                        layout_set_previous(w);
                    }
                    break 'changed;
                }

                if args_has(args, 'E') {
                    layout_spread_out(wp);
                    break 'changed;
                }

                let layoutname = if args_count(args) != 0 {
                    args_string(args, 0)
                } else if args_has(args, 'o') {
                    oldlayout
                } else {
                    null()
                };

                if !args_has(args, 'o') {
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

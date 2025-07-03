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

const LIST_WINDOWS_TEMPLATE: &CStr = c"#{window_index}: #{window_name}#{window_raw_flags} (#{window_panes} panes) [#{window_width}x#{window_height}] [layout #{window_layout}] #{window_id}#{?window_active, (active),}";
const LIST_WINDOWS_WITH_SESSION_TEMPLATE: &CStr = c"#{session_name}:#{window_index}: #{window_name}#{window_raw_flags} (#{window_panes} panes) [#{window_width}x#{window_height}] ";

pub static mut cmd_list_windows_entry: cmd_entry = cmd_entry {
    name: c"list-windows".as_ptr(),
    alias: c"lsw".as_ptr(),

    args: args_parse::new(c"F:f:at:", 0, 0, None),
    usage: c"[-a] [-F format] [-f filter] [-t target-session]".as_ptr(),

    target: cmd_entry_flag::new(b't', cmd_find_type::CMD_FIND_SESSION, 0),

    flags: cmd_flag::CMD_AFTERHOOK,
    exec: Some(cmd_list_windows_exec),
    ..unsafe { zeroed() }
};

unsafe fn cmd_list_windows_exec(self_: *mut cmd, item: *mut cmdq_item) -> cmd_retval {
    unsafe {
        let args = cmd_get_args(self_);
        let target = cmdq_get_target(item);

        if args_has_(args, 'a') {
            cmd_list_windows_server(self_, item);
        } else {
            cmd_list_windows_session(self_, NonNull::new_unchecked((*target).s), item, 0);
        }

        cmd_retval::CMD_RETURN_NORMAL
    }
}

unsafe fn cmd_list_windows_server(self_: *mut cmd, item: *mut cmdq_item) {
    unsafe {
        for s in rb_foreach(&raw mut sessions) {
            cmd_list_windows_session(self_, s, item, 1);
        }
    }
}

unsafe fn cmd_list_windows_session(
    self_: *mut cmd,
    s: NonNull<session>,
    item: *mut cmdq_item,
    type_: i32,
) {
    unsafe {
        let args = cmd_get_args(self_);
        let mut flag = 0;

        let mut template = args_get_(args, 'F');
        if template.is_null() {
            match type_ {
                0 => {
                    template = LIST_WINDOWS_TEMPLATE.as_ptr();
                }
                1 => {
                    template = LIST_WINDOWS_WITH_SESSION_TEMPLATE.as_ptr();
                }
                _ => (),
            }
        }
        let filter = args_get_(args, 'f');

        for (n, wl) in rb_foreach(&raw mut (*s.as_ptr()).windows).enumerate() {
            let ft = format_create(
                cmdq_get_client(item),
                item,
                FORMAT_NONE,
                format_flags::empty(),
            );
            format_add!(ft, c"line".as_ptr(), "{n}");
            format_defaults(ft, null_mut(), Some(s), Some(wl), None);

            if !filter.is_null() {
                let expanded = format_expand(ft, filter);
                flag = format_true(expanded);
                free_(expanded);
            } else {
                flag = 1;
            }
            if flag != 0 {
                let line = format_expand(ft, template);
                cmdq_print!(item, "{}", _s(line));
                free_(line);
            }

            format_free(ft);
        }
    }
}

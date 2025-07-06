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

use crate::compat::{queue::tailq_foreach, tree::rb_foreach};

pub static cmd_list_panes_entry: cmd_entry = cmd_entry {
    name: SyncCharPtr::new(c"list-panes"),
    alias: SyncCharPtr::new(c"lsp"),

    args: args_parse::new(c"asF:f:t:", 0, 0, None),
    usage: SyncCharPtr::new(c"[-as] [-F format] [-f filter] [-t target-window]"),

    target: cmd_entry_flag::new(b't', cmd_find_type::CMD_FIND_WINDOW, 0),

    flags: cmd_flag::CMD_AFTERHOOK,
    exec: cmd_list_panes_exec,
    source: cmd_entry_flag::zeroed(),
};

unsafe fn cmd_list_panes_exec(self_: *mut cmd, item: *mut cmdq_item) -> cmd_retval {
    unsafe {
        let args = cmd_get_args(self_);
        let target = cmdq_get_target(item);
        let s = (*target).s;
        let wl = (*target).wl;

        if args_has_(args, 'a') {
            cmd_list_panes_server(self_, item);
        } else if args_has_(args, 's') {
            cmd_list_panes_session(self_, s, item, 1);
        } else {
            cmd_list_panes_window(self_, s, wl, item, 0);
        }

        cmd_retval::CMD_RETURN_NORMAL
    }
}

unsafe fn cmd_list_panes_server(self_: *mut cmd, item: *mut cmdq_item) {
    unsafe {
        for s in rb_foreach(&raw mut sessions).map(NonNull::as_ptr) {
            cmd_list_panes_session(self_, s, item, 2);
        }
    }
}

unsafe fn cmd_list_panes_session(
    self_: *mut cmd,
    s: *mut session,
    item: *mut cmdq_item,
    type_: i32,
) {
    unsafe {
        for wl in rb_foreach(&raw mut (*s).windows).map(NonNull::as_ptr) {
            cmd_list_panes_window(self_, s, wl, item, type_);
        }
    }
}

fn cmd_list_panes_window(
    self_: *mut cmd,
    s: *mut session,
    wl: *mut winlink,
    item: *mut cmdq_item,
    type_: i32,
) {
    unsafe {
        let args = cmd_get_args(self_);

        let mut template = args_get_(args, 'F');
        if template.is_null() {
            match type_ {
                0 => {
                    template = concat!(
                        "#{pane_index}: ",
                        "[#{pane_width}x#{pane_height}] [history ",
                        "#{history_size}/#{history_limit}, ",
                        "#{history_bytes} bytes] #{pane_id}",
                        "#{?pane_active, (active),}#{?pane_dead, (dead),}\0"
                    )
                    .as_ptr()
                    .cast();
                }
                1 => {
                    template = concat!(
                        "#{window_index}.#{pane_index}: ",
                        "[#{pane_width}x#{pane_height}] [history ",
                        "#{history_size}/#{history_limit}, ",
                        "#{history_bytes} bytes] #{pane_id}",
                        "#{?pane_active, (active),}#{?pane_dead, (dead),}\0"
                    )
                    .as_ptr()
                    .cast();
                }
                2 => {
                    template = concat!(
                        "#{session_name}:#{window_index}.",
                        "#{pane_index}: [#{pane_width}x#{pane_height}] ",
                        "[history #{history_size}/#{history_limit}, ",
                        "#{history_bytes} bytes] #{pane_id}",
                        "#{?pane_active, (active),}#{?pane_dead, (dead),}\0"
                    )
                    .as_ptr()
                    .cast();
                }
                _ => (),
            }
        }
        let filter = args_get_(args, 'f');

        for (n, wp) in tailq_foreach::<_, discr_entry>(&raw mut (*(*wl).window).panes).enumerate() {
            let ft = format_create(
                cmdq_get_client(item),
                item,
                FORMAT_NONE,
                format_flags::empty(),
            );
            format_add!(ft, c!("line"), "{n}");
            format_defaults(ft, null_mut(), NonNull::new(s), NonNull::new(wl), Some(wp));

            let mut flag = 0;
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

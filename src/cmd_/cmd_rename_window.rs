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
use crate::options_::*;

pub static CMD_RENAME_WINDOW_ENTRY: cmd_entry = cmd_entry {
    name: "rename-window",
    alias: Some("renamew"),

    args: args_parse::new("t:", 1, 1, None),
    usage: "[-t target-window] new-name",

    target: cmd_entry_flag::new(
        b't',
        cmd_find_type::CMD_FIND_WINDOW,
        cmd_find_flags::empty(),
    ),
    source: cmd_entry_flag::zeroed(),

    flags: cmd_flag::CMD_AFTERHOOK,
    exec: cmd_rename_window_exec,
};

unsafe fn cmd_rename_window_exec(self_: *mut cmd, item: *mut cmdq_item) -> cmd_retval {
    unsafe {
        let args = cmd_get_args(self_);
        let target = cmdq_get_target(item);
        let wl = (*target).wl;

        let newname = format_single_from_target(item, args_string(args, 0));
        window_set_name((*wl).window, newname);
        options_set_number((*(*wl).window).options, c!("automatic-rename"), 0);

        server_redraw_window_borders((*wl).window);
        server_status_window((*wl).window);
        free_(newname);
    }

    cmd_retval::CMD_RETURN_NORMAL
}

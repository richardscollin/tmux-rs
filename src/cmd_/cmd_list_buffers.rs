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

pub static CMD_LIST_BUFFERS_ENTRY: cmd_entry = cmd_entry {
    name: SyncCharPtr::new(c"list-buffers"),
    alias: SyncCharPtr::new(c"lsb"),

    args: args_parse::new(c"F:f:", 0, 0, None),
    usage: SyncCharPtr::new(c"[-F format] [-f filter]"),

    flags: cmd_flag::CMD_AFTERHOOK,
    exec: cmd_list_buffers_exec,
    source: cmd_entry_flag::zeroed(),
    target: cmd_entry_flag::zeroed(),
};

unsafe fn cmd_list_buffers_exec(self_: *mut cmd, item: *mut cmdq_item) -> cmd_retval {
    unsafe {
        let args = cmd_get_args(self_);

        let mut template: *const u8 = args_get(args, b'F');
        if template.is_null() {
            template = c!("#{buffer_name}: #{buffer_size} bytes: \"#{buffer_sample}\"");
        }
        let filter = args_get(args, b'f');

        let mut pb = null_mut();
        while {
            pb = paste_walk(pb);
            !pb.is_null()
        } {
            let ft = format_create(
                cmdq_get_client(item),
                item,
                FORMAT_NONE,
                format_flags::empty(),
            );
            format_defaults_paste_buffer(ft, pb);

            let flag;
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

        cmd_retval::CMD_RETURN_NORMAL
    }
}

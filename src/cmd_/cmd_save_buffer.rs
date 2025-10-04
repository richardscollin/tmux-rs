// Copyright (c) 2009 Tiago Cunha <me@tiagocunha.org>
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
use crate::libc::{O_APPEND, O_TRUNC};
use crate::*;

pub static CMD_SAVE_BUFFER_ENTRY: cmd_entry = cmd_entry {
    name: "save-buffer",
    alias: Some("saveb"),

    args: args_parse::new(c"ab:", 1, 1, None),
    usage: SyncCharPtr::new(c"[-a] [-b buffer-name] path"),

    flags: cmd_flag::CMD_AFTERHOOK,
    exec: cmd_save_buffer_exec,
    source: cmd_entry_flag::zeroed(),
    target: cmd_entry_flag::zeroed(),
};

pub static CMD_SHOW_BUFFER_ENTRY: cmd_entry = cmd_entry {
    name: "show-buffer",
    alias: Some("showb"),

    args: args_parse::new(c"b:", 0, 0, None),
    usage: SyncCharPtr::new(c"[-b buffer-name]"),

    flags: cmd_flag::CMD_AFTERHOOK,
    exec: cmd_save_buffer_exec,
    source: cmd_entry_flag::zeroed(),
    target: cmd_entry_flag::zeroed(),
};

unsafe fn cmd_save_buffer_done(
    _c: *mut client,
    path: *mut u8,
    error: i32,
    closed: i32,
    _buffer: *mut evbuffer,
    data: *mut c_void,
) {
    let item = data as *mut cmdq_item;

    if closed == 0 {
        return;
    }

    unsafe {
        if error != 0 {
            cmdq_error!(item, "{}: {}", _s(path), strerror(error));
        }
        cmdq_continue(item);
    }
}

unsafe fn cmd_save_buffer_exec(self_: *mut cmd, item: *mut cmdq_item) -> cmd_retval {
    unsafe {
        let args = cmd_get_args(self_);
        let c = cmdq_get_client(item);
        let bufname = args_get_(args, 'b');
        let path;
        let evb;

        let pb = if bufname.is_null() {
            let Some(pb) = NonNull::new(paste_get_top(null_mut())) else {
                cmdq_error!(item, "no buffers");
                return cmd_retval::CMD_RETURN_ERROR;
            };
            pb
        } else {
            let Some(pb) = NonNull::new(paste_get_name(bufname)) else {
                cmdq_error!(item, "no buffer {}", _s(bufname));
                return cmd_retval::CMD_RETURN_ERROR;
            };
            pb
        };
        let mut bufsize: usize = 0;
        let bufdata = paste_buffer_data_(pb, &mut bufsize);

        if std::ptr::eq(cmd_get_entry(self_), &CMD_SHOW_BUFFER_ENTRY) {
            if !(*c).session.is_null() || (*c).flags.intersects(client_flag::CONTROL) {
                evb = evbuffer_new();
                if evb.is_null() {
                    fatalx("out of memory");
                }
                evbuffer_add(evb, bufdata as _, bufsize);
                cmdq_print_data(item, 1, evb);
                evbuffer_free(evb);
                return cmd_retval::CMD_RETURN_NORMAL;
            }
            path = xstrdup_(c"-").as_ptr();
        } else {
            path = format_single_from_target(item, args_string(args, 0));
        }
        let flags = if args_has(args, 'a') {
            O_APPEND
        } else {
            O_TRUNC
        };
        file_write(
            cmdq_get_client(item),
            path,
            flags,
            bufdata as _,
            bufsize,
            Some(cmd_save_buffer_done),
            item as _,
        );
        free_(path);

        cmd_retval::CMD_RETURN_WAIT
    }
}

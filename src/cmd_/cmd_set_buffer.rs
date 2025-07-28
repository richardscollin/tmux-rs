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

pub static CMD_SET_BUFFER_ENTRY: cmd_entry = cmd_entry {
    name: SyncCharPtr::new(c"set-buffer"),
    alias: SyncCharPtr::new(c"setb"),

    args: args_parse::new(c"ab:t:n:w", 0, 1, None),
    usage: SyncCharPtr::new(c"[-aw] [-b buffer-name] [-n new-buffer-name] [-t target-client] data"),

    flags: cmd_flag::CMD_AFTERHOOK
        .union(cmd_flag::CMD_CLIENT_TFLAG)
        .union(cmd_flag::CMD_CLIENT_CANFAIL),
    exec: cmd_set_buffer_exec,
    source: cmd_entry_flag::zeroed(),
    target: cmd_entry_flag::zeroed(),
};

pub static CMD_DELETE_BUFFER_ENTRY: cmd_entry = cmd_entry {
    name: SyncCharPtr::new(c"delete-buffer"),
    alias: SyncCharPtr::new(c"deleteb"),

    args: args_parse::new(c"b:", 0, 0, None),
    usage: SyncCharPtr::new(CMD_BUFFER_USAGE),

    flags: cmd_flag::CMD_AFTERHOOK,
    exec: cmd_set_buffer_exec,
    source: cmd_entry_flag::zeroed(),
    target: cmd_entry_flag::zeroed(),
};

unsafe fn cmd_set_buffer_exec(self_: *mut cmd, item: *mut cmdq_item) -> cmd_retval {
    unsafe {
        let args = cmd_get_args(self_);
        let tc = cmdq_get_target_client(item);
        let mut pb;
        let mut cause = null_mut();
        let olddata;

        let mut bufname = args_get_(args, 'b');
        if bufname.is_null() {
            pb = null_mut();
        } else {
            pb = paste_get_name(bufname);
        }

        if std::ptr::eq(cmd_get_entry(self_), &CMD_DELETE_BUFFER_ENTRY) {
            if pb.is_null() {
                if !bufname.is_null() {
                    cmdq_error!(item, "unknown buffer: {}", _s(bufname));
                    return cmd_retval::CMD_RETURN_ERROR;
                }
                pb = paste_get_top(&raw mut bufname);
            }
            if pb.is_null() {
                cmdq_error!(item, "no buffer");
                return cmd_retval::CMD_RETURN_ERROR;
            }
            paste_free(NonNull::new_unchecked(pb));
            return cmd_retval::CMD_RETURN_NORMAL;
        }

        if args_has_(args, 'n') {
            if pb.is_null() {
                if !bufname.is_null() {
                    cmdq_error!(item, "unknown buffer: {}", _s(bufname));
                    return cmd_retval::CMD_RETURN_ERROR;
                }
                pb = paste_get_top(&raw mut bufname);
            }
            if pb.is_null() {
                cmdq_error!(item, "no buffer");
                return cmd_retval::CMD_RETURN_ERROR;
            }
            if paste_rename(bufname, args_get_(args, 'n'), &raw mut cause) != 0 {
                cmdq_error!(item, "{}", _s(cause));
                free_(cause);
                return cmd_retval::CMD_RETURN_ERROR;
            }
            return cmd_retval::CMD_RETURN_NORMAL;
        }

        if args_count(args) != 1 {
            cmdq_error!(item, "no data specified");
            return cmd_retval::CMD_RETURN_ERROR;
        }
        let newsize = strlen(args_string(args, 0));
        if newsize == 0 {
            return cmd_retval::CMD_RETURN_NORMAL;
        }

        let mut bufsize = 0;
        let mut bufdata = null_mut();

        if let Some(pb_non_null) = NonNull::new(pb)
            && args_has_(args, 'a')
        {
            olddata = paste_buffer_data_(pb_non_null, &mut bufsize);
            bufdata = xmalloc(bufsize).as_ptr().cast();
            memcpy_(bufdata, olddata, bufsize);
        }

        bufdata = xrealloc_(bufdata, bufsize + newsize).as_ptr();
        memcpy_(bufdata.add(bufsize), args_string(args, 0), newsize);
        bufsize += newsize;

        if paste_set(bufdata, bufsize, bufname, &raw mut cause) != 0 {
            cmdq_error!(item, "{}", _s(cause));
            free_(bufdata);
            free_(cause);
            return cmd_retval::CMD_RETURN_ERROR;
        }
        if args_has_(args, 'w') && !tc.is_null() {
            tty_set_selection(&raw mut (*tc).tty, c!(""), bufdata, bufsize);
        }

        cmd_retval::CMD_RETURN_NORMAL
    }
}

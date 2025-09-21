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
use crate::*;

pub static CMD_LOAD_BUFFER_ENTRY: cmd_entry = cmd_entry {
    name: SyncCharPtr::new(c"load-buffer"),
    alias: SyncCharPtr::new(c"loadb"),

    args: args_parse::new(c"b:t:w", 1, 1, None),
    usage: SyncCharPtr::new(c"[-b buffer-name] [-t target-client] path"),

    flags: cmd_flag::CMD_AFTERHOOK
        .union(cmd_flag::CMD_CLIENT_TFLAG)
        .union(cmd_flag::CMD_CLIENT_CANFAIL),
    exec: cmd_load_buffer_exec,
    source: cmd_entry_flag::zeroed(),
    target: cmd_entry_flag::zeroed(),
};

#[repr(C)]
pub struct cmd_load_buffer_data {
    pub client: *mut client,
    pub item: *mut cmdq_item,
    pub name: *mut u8,
}

unsafe fn cmd_load_buffer_done(
    _c: *mut client,
    path: *mut u8,
    error: i32,
    closed: i32,
    buffer: *mut evbuffer,
    data: *mut c_void,
) {
    unsafe {
        let cdata = data as *mut cmd_load_buffer_data;
        let tc = (*cdata).client;
        let item = (*cdata).item;
        let bdata = EVBUFFER_DATA(buffer);
        let bsize = EVBUFFER_LENGTH(buffer);

        if closed == 0 {
            return;
        }

        if error != 0 {
            cmdq_error!(item, "{}: {}", _s(path), _s(strerror(error)));
        } else if bsize != 0 {
            let copy = xmalloc(bsize).as_ptr();
            memcpy_(copy, bdata as _, bsize);
            let mut cause = null_mut();
            if paste_set(copy as _, bsize, (*cdata).name, &raw mut cause) != 0 {
                cmdq_error!(item, "{}", _s(cause));
                free_(cause);
                free_(copy);
            } else if !tc.is_null()
                && !(*tc).session.is_null()
                && !(*tc).flags.intersects(client_flag::DEAD)
            {
                tty_set_selection(&raw mut (*tc).tty, c!(""), copy as _, bsize);
            }
            if !tc.is_null() {
                server_client_unref(tc);
            }
        }
        cmdq_continue(item);

        free_((*cdata).name);
        free_(cdata);
    }
}

unsafe fn cmd_load_buffer_exec(self_: *mut cmd, item: *mut cmdq_item) -> cmd_retval {
    unsafe {
        let args = cmd_get_args(self_);
        let tc = cmdq_get_target_client(item);
        let bufname = args_get(args, b'b');

        let cdata = xcalloc_::<cmd_load_buffer_data>(1).as_ptr();
        (*cdata).item = item;
        if !bufname.is_null() {
            (*cdata).name = xstrdup(bufname).as_ptr();
        }
        if args_has(args, 'w') && !tc.is_null() {
            (*cdata).client = tc;
            (*(*cdata).client).references += 1;
        }

        let mut path = format_single_from_target(item, args_string(args, 0));
        nul_terminate(&mut path);
        file_read(
            cmdq_get_client(item),
            path.as_ptr().cast(),
            Some(cmd_load_buffer_done),
            cdata.cast(),
        );
    }

    cmd_retval::CMD_RETURN_WAIT
}

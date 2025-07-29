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

pub static CMD_PASTE_BUFFER_ENTRY: cmd_entry = cmd_entry {
    name: SyncCharPtr::new(c"paste-buffer"),
    alias: SyncCharPtr::new(c"pasteb"),

    args: args_parse::new(c"db:prs:t:", 0, 0, None),
    usage: SyncCharPtr::new(c"[-dpr] [-s separator] [-b buffer-name] [-t target-pane]"),

    target: cmd_entry_flag::new(b't', cmd_find_type::CMD_FIND_PANE, cmd_find_flags::empty()),

    flags: cmd_flag::CMD_AFTERHOOK,
    exec: cmd_paste_buffer_exec,
    source: cmd_entry_flag::zeroed(),
};

unsafe fn cmd_paste_buffer_exec(self_: *mut cmd, item: *mut cmdq_item) -> cmd_retval {
    unsafe {
        let args = cmd_get_args(self_);
        let target = cmdq_get_target(item);
        let wp = (*target).wp;
        let bracket = args_has(args, b'p') != 0;

        if window_pane_exited(wp) {
            cmdq_error!(item, "target pane has exited");
            return cmd_retval::CMD_RETURN_ERROR;
        }

        let mut bufname = null();
        if args_has(args, b'b') != 0 {
            bufname = args_get(args, b'b');
        }

        let pb ;
        if bufname.is_null() {
            pb = paste_get_top(null_mut());
        } else {
            pb = paste_get_name(bufname);
            if pb.is_null() {
                cmdq_error!(item, "no buffer {}", _s(bufname));
                return cmd_retval::CMD_RETURN_ERROR;
            }
        }

        if let Some(pb) = NonNull::new(pb)
            && !(*wp).flags.intersects(window_pane_flags::PANE_INPUTOFF)
        {
            let mut sepstr = args_get(args, b's');
            if sepstr.is_null() {
                if args_has(args, b'r') != 0 {
                    sepstr = c!("\n");
                } else {
                    sepstr = c!("\r");
                }
            }
            let seplen = strlen(sepstr);

            if bracket
                && (*(*wp).screen)
                    .mode
                    .intersects(mode_flag::MODE_BRACKETPASTE)
            {
                bufferevent_write((*wp).event, c!("\x1b[200~").cast(), 6);
            }

            let mut bufsize: usize = 0;
            let mut bufdata = paste_buffer_data_(pb, &mut bufsize);
            let bufend = bufdata.add(bufsize);

            loop {
                let line: *mut u8 =
                    libc::memchr(bufdata as _, b'\n' as i32, bufend.addr() - bufdata.addr()).cast();
                if line.is_null() {
                    break;
                }

                bufferevent_write((*wp).event, bufdata.cast(), line.addr() - bufdata.addr());
                bufferevent_write((*wp).event, sepstr.cast(), seplen);

                bufdata = line.add(1);
            }
            if bufdata != bufend {
                bufferevent_write((*wp).event, bufdata.cast(), bufend.addr() - bufdata.addr());
            }

            if bracket
                && (*(*wp).screen)
                    .mode
                    .intersects(mode_flag::MODE_BRACKETPASTE)
            {
                bufferevent_write((*wp).event, c!("\x1b[201~").cast(), 6);
            }
        }

        if let Some(non_null_pb) = NonNull::new(pb)
            && args_has_(args, 'd')
        {
            paste_free(non_null_pb);
        }

        cmd_retval::CMD_RETURN_NORMAL
    }
}

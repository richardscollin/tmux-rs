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
use crate::compat::queue::{list_foreach, tailq_foreach_reverse};
use crate::*;

const SHOW_MESSAGES_TEMPLATE: *const u8 = c!("#{t/p:message_time}: #{message_text}");

pub static CMD_SHOW_MESSAGES_ENTRY: cmd_entry = cmd_entry {
    name: SyncCharPtr::new(c"show-messages"),
    alias: SyncCharPtr::new(c"showmsgs"),

    args: args_parse::new(c"JTt:", 0, 0, None),
    usage: SyncCharPtr::new(c"[-JT] [-t target-client]"),

    flags: cmd_flag::CMD_AFTERHOOK.union(cmd_flag::CMD_CLIENT_TFLAG),
    exec: cmd_show_messages_exec,
    source: cmd_entry_flag::zeroed(),
    target: cmd_entry_flag::zeroed(),
};

unsafe fn cmd_show_messages_terminals(
    self_: *mut cmd,
    item: *mut cmdq_item,
    mut blank: i32,
) -> c_int {
    unsafe {
        let args = cmd_get_args(self_);
        let tc = cmdq_get_target_client(item);

        let mut n = 0u32;
        for term in list_foreach::<_, discr_entry>(&raw mut TTY_TERMS).map(NonNull::as_ptr) {
            if args_has(args, 't') && term != (*tc).tty.term {
                continue;
            }
            if blank != 0 {
                cmdq_print!(item, "");
                blank = 0;
            }
            cmdq_print!(
                item,
                "Terminal {}: {} for {}, flags=0x{:x}:",
                n,
                _s((*term).name),
                _s((*(*(*term).tty).client).name),
                (*term).flags,
            );
            n += 1;
            for i in 0..tty_term_ncodes() {
                cmdq_print!(
                    item,
                    "{}",
                    _s(tty_term_describe(term, tty_code_code::try_from(i).unwrap())),
                );
            }
        }
        (n != 0) as i32
    }
}

unsafe fn cmd_show_messages_exec(self_: *mut cmd, item: *mut cmdq_item) -> cmd_retval {
    unsafe {
        let args = cmd_get_args(self_);

        let mut done = false;
        let mut blank = 0;
        if args_has(args, 'T') {
            blank = cmd_show_messages_terminals(self_, item, blank);
            done = true;
        }
        if args_has(args, 'J') {
            job_print_summary(item, blank);
            done = true;
        }
        if done {
            return cmd_retval::CMD_RETURN_NORMAL;
        }

        let ft = format_create_from_target(item);

        for msg in tailq_foreach_reverse(&raw mut crate::server::MESSAGE_LOG).map(NonNull::as_ptr) {
            format_add!(ft, c!("message_text"), "{}", _s((*msg).msg));
            format_add!(ft, c!("message_number"), "{}", (*msg).msg_num,);
            format_add_tv(ft, c!("message_time"), &raw mut (*msg).msg_time);

            let s = format_expand(ft, SHOW_MESSAGES_TEMPLATE);
            cmdq_print!(item, "{}", _s(s));
            free_(s);
        }
        format_free(ft);

        cmd_retval::CMD_RETURN_NORMAL
    }
}

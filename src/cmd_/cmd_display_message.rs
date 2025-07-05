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

const DISPLAY_MESSAGE_TEMPLATE: &CStr = c"[#{session_name}] #{window_index}:#{window_name}, current pane #{pane_index} - (%H:%M %d-%b-%y)";

pub static cmd_display_message_entry: cmd_entry = cmd_entry {
    name: SyncCharPtr::new(c"display-message"),
    alias: SyncCharPtr::new(c"display"),

    args: args_parse::new(c"ac:d:lINpt:F:v", 0, 1, None),
    usage: SyncCharPtr::new(
        c"[-aIlNpv] [-c target-client] [-d delay] [-F format] [-t target-pane] [message]",
    ),

    target: cmd_entry_flag::new(b't', cmd_find_type::CMD_FIND_PANE, CMD_FIND_CANFAIL),

    flags: cmd_flag::CMD_AFTERHOOK
        .union(cmd_flag::CMD_CLIENT_CFLAG)
        .union(cmd_flag::CMD_CLIENT_CANFAIL),
    exec: cmd_display_message_exec,
    source: cmd_entry_flag::zeroed(),
};

unsafe fn cmd_display_message_each(key: *const c_char, value: *const c_char, arg: *mut c_void) {
    let item = arg as *mut cmdq_item;

    unsafe {
        cmdq_print!(item, "{}={}", _s(key), _s(value));
    }
}

unsafe fn cmd_display_message_exec(self_: *mut cmd, item: *mut cmdq_item) -> cmd_retval {
    unsafe {
        let args = cmd_get_args(self_);
        let target = cmdq_get_target(item);
        let tc = cmdq_get_target_client(item);
        let s = (*target).s;
        let wl = (*target).wl;
        let wp = (*target).wp;
        let mut cause: *mut c_char = null_mut();
        let mut delay = -1;
        let nflag = args_has(args, b'N');
        let count = args_count(args);

        if args_has_(args, 'I') {
            if wp.is_null() {
                return cmd_retval::CMD_RETURN_NORMAL;
            }
            match window_pane_start_input(wp, item, &raw mut cause) {
                -1 => {
                    cmdq_error!(item, "{}", _s(cause));
                    free_(cause);
                    return cmd_retval::CMD_RETURN_ERROR;
                }
                1 => {
                    return cmd_retval::CMD_RETURN_NORMAL;
                }
                0 => {
                    return cmd_retval::CMD_RETURN_WAIT;
                }
                _ => (),
            }
        }

        if args_has_(args, 'F') && count != 0 {
            cmdq_error!(item, "only one of -F or argument must be given");
            return cmd_retval::CMD_RETURN_ERROR;
        }

        if args_has_(args, 'd') {
            delay = args_strtonum(args, b'd', 0, u32::MAX as i64, &raw mut cause);
            if !cause.is_null() {
                cmdq_error!(item, "delay {}", _s(cause));
                free_(cause);
                return cmd_retval::CMD_RETURN_ERROR;
            }
        }

        let mut template = if count != 0 {
            args_string(args, 0)
        } else {
            args_get_(args, 'F')
        };
        if template.is_null() {
            template = DISPLAY_MESSAGE_TEMPLATE.as_ptr();
        }

        /*
         * -c is intended to be the client where the message should be
         * displayed if -p is not given. But it makes sense to use it for the
         * formats too, assuming it matches the session. If it doesn't, use the
         * best client for the session.
         */
        let c = if !tc.is_null() && (*tc).session == s {
            tc
        } else if !s.is_null() {
            cmd_find_best_client(s)
        } else {
            null_mut()
        };

        let flags = if args_has_(args, 'v') {
            format_flags::FORMAT_VERBOSE
        } else {
            format_flags::empty()
        };
        let ft = format_create(cmdq_get_client(item), item, FORMAT_NONE, flags);
        format_defaults(ft, c, NonNull::new(s), NonNull::new(wl), NonNull::new(wp));

        if args_has_(args, 'a') {
            format_each(ft, Some(cmd_display_message_each), item as _);
            return cmd_retval::CMD_RETURN_NORMAL;
        }

        let msg = if args_has_(args, 'l') {
            xstrdup(template).as_ptr()
        } else {
            format_expand_time(ft, template)
        };

        if cmdq_get_client(item).is_null() {
            cmdq_error!(item, "{}", _s(msg));
        } else if args_has_(args, 'p') {
            cmdq_print!(item, "{}", _s(msg));
        } else if !tc.is_null() && (*tc).flags.intersects(client_flag::CONTROL) {
            let evb = evbuffer_new();
            if evb.is_null() {
                fatalx("out of memory");
            }
            evbuffer_add_printf!(evb, "%message {}", _s(msg));
            server_client_print(tc, 0, evb);
            evbuffer_free(evb);
        } else if !tc.is_null() {
            status_message_set!(tc, delay as i32, 0, nflag, "{}", _s(msg));
        }
        free_(msg);

        format_free(ft);

        cmd_retval::CMD_RETURN_NORMAL
    }
}

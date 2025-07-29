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

pub static CMD_SHOW_ENVIRONMENT_ENTRY: cmd_entry = cmd_entry {
    name: SyncCharPtr::new(c"show-environment"),
    alias: SyncCharPtr::new(c"showenv"),

    args: args_parse::new(c"hgst:", 0, 1, None),
    usage: SyncCharPtr::new(c"[-hgs] [-t target-session] [name]"),

    target: cmd_entry_flag::new(
        b't',
        cmd_find_type::CMD_FIND_SESSION,
        cmd_find_flags::CMD_FIND_CANFAIL,
    ),

    flags: cmd_flag::CMD_AFTERHOOK,
    exec: cmd_show_environment_exec,
    source: cmd_entry_flag::zeroed(),
};

unsafe fn cmd_show_environment_escape(envent: *mut environ_entry) -> *mut u8 {
    unsafe {
        let mut value = transmute_ptr((*envent).value);
        let ret: *mut u8 = xmalloc(strlen(value) * 2 + 1).as_ptr().cast(); /* at most twice the size */
        let mut out = ret;

        let mut c ;
        while {
            c = *value;
            value = value.add(1);
            c != b'\0'
        } {
            /* POSIX interprets $ ` " and \ in double quotes. */
            if c == b'$' as _ || c == b'`' as _ || c == b'"' as _ || c == b'\\' as _ {
                *out = b'\\' as _;
                out = out.add(1);
            }
            *out = c;
            out = out.add(1);
        }
        *out = b'\0';

        ret
    }
}

unsafe fn cmd_show_environment_print(
    self_: *mut cmd,
    item: *mut cmdq_item,
    envent: *mut environ_entry,
) {
    unsafe {
        let args = cmd_get_args(self_);

        if !args_has_(args, 'h') && ((*envent).flags & ENVIRON_HIDDEN != 0) {
            return;
        }
        if args_has_(args, 'h') && (!(*envent).flags & ENVIRON_HIDDEN != 0) {
            return;
        }

        if !args_has_(args, 's') {
            if let Some(value) = (*envent).value {
                cmdq_print!(
                    item,
                    "{}={}",
                    _s(transmute_ptr((*envent).name)),
                    _s(value.as_ptr())
                );
            } else {
                cmdq_print!(item, "-{}", _s(transmute_ptr((*envent).name)));
            }
            return;
        }

        if (*envent).value.is_some() {
            let escaped = cmd_show_environment_escape(envent);
            cmdq_print!(
                item,
                "{}=\"{}\"; export {};",
                _s(transmute_ptr((*envent).name)),
                _s(escaped),
                _s(transmute_ptr((*envent).name)),
            );
            free_(escaped);
        } else {
            cmdq_print!(item, "unset {};", _s(transmute_ptr((*envent).name)));
        }
    }
}

unsafe fn cmd_show_environment_exec(self_: *mut cmd, item: *mut cmdq_item) -> cmd_retval {
    unsafe {
        let args = cmd_get_args(self_);
        let target = cmdq_get_target(item);
        let env: *mut environ;
        let name = args_string(args, 0);

        let mut tflag = args_get_(args, 't');
        if !tflag.is_null() && (*target).s.is_null() {
            cmdq_error!(item, "no such session: {}", _s(tflag));
            return cmd_retval::CMD_RETURN_ERROR;
        }

        if args_has_(args, 'g') {
            env = GLOBAL_ENVIRON;
        } else {
            if (*target).s.is_null() {
                tflag = args_get_(args, 't');
                if !tflag.is_null() {
                    cmdq_error!(item, "no such session: {}", _s(tflag));
                } else {
                    cmdq_error!(item, "no current session");
                }
                return cmd_retval::CMD_RETURN_ERROR;
            }
            env = (*(*target).s).environ;
        }

        let mut envent;
        if !name.is_null() {
            envent = environ_find(env, name);
            if envent.is_null() {
                cmdq_error!(item, "unknown variable: {}", _s(name));
                return cmd_retval::CMD_RETURN_ERROR;
            }
            cmd_show_environment_print(self_, item, envent);
            return cmd_retval::CMD_RETURN_NORMAL;
        }

        envent = environ_first(env);
        while !envent.is_null() {
            cmd_show_environment_print(self_, item, envent);
            envent = environ_next(envent);
        }
        cmd_retval::CMD_RETURN_NORMAL
    }
}

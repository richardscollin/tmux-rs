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

pub static CMD_SET_ENVIRONMENT_ENTRY: cmd_entry = cmd_entry {
    name: SyncCharPtr::new(c"set-environment"),
    alias: SyncCharPtr::new(c"setenv"),

    args: args_parse::new(c"Fhgrt:u", 1, 2, None),
    usage: SyncCharPtr::new(c"[-Fhgru] [-t target-session] name [value]"),

    target: cmd_entry_flag::new(
        b't',
        cmd_find_type::CMD_FIND_SESSION,
        cmd_find_flags::CMD_FIND_CANFAIL,
    ),

    flags: cmd_flag::CMD_AFTERHOOK,
    exec: cmd_set_environment_exec,
    source: cmd_entry_flag::zeroed(),
};

unsafe fn cmd_set_environment_exec(self_: *mut cmd, item: *mut cmdq_item) -> cmd_retval {
    unsafe {
        let args = cmd_get_args(self_);
        let target = cmdq_get_target(item);
        let env: *mut environ;
        let name = args_string(args, 0);
        let tflag;

        if *name == b'\0' as _ {
            cmdq_error!(item, "empty variable name");
            return cmd_retval::CMD_RETURN_ERROR;
        }
        if !strchr_(name, '=').is_null() {
            cmdq_error!(item, "variable name contains =");
            return cmd_retval::CMD_RETURN_ERROR;
        }

        let mut value;
        let expanded;
        if args_count(args) < 2 {
            value = None;
        } else {
            value = args_string_(args, 1);
        }
        if let Some(v) = value
            && args_has(args, 'F')
        {
            // note args_string_ returned value is a backed by nul terminated str
            expanded = format_single_from_target(item, v.as_ptr().cast());
            value = Some(expanded.as_str());
        }
        if args_has(args, 'g') {
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

        if args_has(args, 'u') {
            if value.is_some() {
                cmdq_error!(item, "can't specify a value with -u");
                return cmd_retval::CMD_RETURN_ERROR;
            }
            environ_unset(env, name);
        } else if args_has(args, 'r') {
            if value.is_some() {
                cmdq_error!(item, "can't specify a value with -r");
                return cmd_retval::CMD_RETURN_ERROR;
            }
            environ_clear(env, name);
        } else {
            let Some(value) = value else {
                cmdq_error!(item, "no value specified");
                return cmd_retval::CMD_RETURN_ERROR;
            };

            if args_has(args, 'h') {
                environ_set!(env, name, ENVIRON_HIDDEN, "{value}");
            } else {
                environ_set!(env, name, environ_flags::empty(), "{value}");
            }
        }

        cmd_retval::CMD_RETURN_NORMAL
    }
}

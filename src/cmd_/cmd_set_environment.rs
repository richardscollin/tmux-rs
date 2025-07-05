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

pub static cmd_set_environment_entry: cmd_entry = cmd_entry {
    name: SyncCharPtr::new(c"set-environment"),
    alias: SyncCharPtr::new(c"setenv"),

    args: args_parse::new(c"Fhgrt:u", 1, 2, None),
    usage: SyncCharPtr::new(c"[-Fhgru] [-t target-session] name [value]"),

    target: cmd_entry_flag::new(b't', cmd_find_type::CMD_FIND_SESSION, CMD_FIND_CANFAIL),

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
        let mut value = null();
        let tflag;
        let mut expanded = null_mut();
        let mut retval = cmd_retval::CMD_RETURN_NORMAL;

        'out: {
            if *name == b'\0' as _ {
                cmdq_error!(item, "empty variable name");
                return cmd_retval::CMD_RETURN_ERROR;
            }
            if !strchr_(name, '=').is_null() {
                cmdq_error!(item, "variable name contains =");
                return cmd_retval::CMD_RETURN_ERROR;
            }

            if args_count(args) < 2 {
                value = null_mut();
            } else {
                value = args_string(args, 1);
            }
            if !value.is_null() && args_has_(args, 'F') {
                expanded = format_single_from_target(item, value);
                value = expanded;
            }
            if args_has_(args, 'g') {
                env = global_environ;
            } else {
                if (*target).s.is_null() {
                    tflag = args_get_(args, 't');
                    if !tflag.is_null() {
                        cmdq_error!(item, "no such session: {}", _s(tflag));
                    } else {
                        cmdq_error!(item, "no current session");
                    }
                    retval = cmd_retval::CMD_RETURN_ERROR;
                    break 'out;
                }
                env = (*(*target).s).environ;
            }

            if args_has_(args, 'u') {
                if !value.is_null() {
                    cmdq_error!(item, "can't specify a value with -u");
                    retval = cmd_retval::CMD_RETURN_ERROR;
                    break 'out;
                }
                environ_unset(env, name);
            } else if args_has_(args, 'r') {
                if !value.is_null() {
                    cmdq_error!(item, "can't specify a value with -r");
                    retval = cmd_retval::CMD_RETURN_ERROR;
                    break 'out;
                }
                environ_clear(env, name);
            } else {
                if value.is_null() {
                    cmdq_error!(item, "no value specified");
                    retval = cmd_retval::CMD_RETURN_ERROR;
                    break 'out;
                }

                if args_has_(args, 'h') {
                    environ_set!(env, name, ENVIRON_HIDDEN, "{}", _s(value));
                } else {
                    environ_set!(env, name, 0, "{}", _s(value));
                }
            }
        }

        //out:
        free_(expanded);
        retval
    }
}

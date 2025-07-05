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

pub static cmd_unbind_key_entry: cmd_entry = cmd_entry {
    name: SyncCharPtr::new(c"unbind-key"),
    alias: SyncCharPtr::new(c"unbind"),

    args: args_parse::new(c"anqT:", 0, 1, None),
    usage: SyncCharPtr::new(c"[-anq] [-T key-table] key"),

    flags: cmd_flag::CMD_AFTERHOOK,
    exec: cmd_unbind_key_exec,
    source: cmd_entry_flag::zeroed(),
    target: cmd_entry_flag::zeroed(),
};

unsafe fn cmd_unbind_key_exec(self_: *mut cmd, item: *mut cmdq_item) -> cmd_retval {
    unsafe {
        let args = cmd_get_args(self_);
        let mut tablename: *const c_char = null_mut();
        let keystr = args_string(args, 0);
        let quiet = args_has(args, b'q');

        if args_has(args, b'a') != 0 {
            if !keystr.is_null() {
                if quiet == 0 {
                    cmdq_error!(item, "key given with -a");
                }
                return cmd_retval::CMD_RETURN_ERROR;
            }

            tablename = args_get(args, b'T');
            if tablename.is_null() {
                if args_has(args, b'n') != 0 {
                    tablename = c"root".as_ptr();
                } else {
                    tablename = c"prefix".as_ptr();
                }
            }
            if key_bindings_get_table(tablename, 0).is_null() {
                if quiet == 0 {
                    cmdq_error!(item, "table {} doesn't exist", _s(tablename));
                }
                return cmd_retval::CMD_RETURN_ERROR;
            }

            key_bindings_remove_table(tablename);
            return cmd_retval::CMD_RETURN_NORMAL;
        }

        if keystr.is_null() {
            if quiet == 0 {
                cmdq_error!(item, "missing key");
            }
            return cmd_retval::CMD_RETURN_ERROR;
        }

        let key = key_string_lookup_string(keystr);
        if key == KEYC_NONE || key == KEYC_UNKNOWN {
            if quiet == 0 {
                cmdq_error!(item, "unknown key unbind: {}", _s(keystr));
            }
            return cmd_retval::CMD_RETURN_ERROR;
        }

        if args_has(args, b'T') != 0 {
            tablename = args_get(args, b'T');
            if key_bindings_get_table(tablename, 0).is_null() {
                if quiet == 0 {
                    cmdq_error!(item, "table {} doesn't exist", _s(tablename));
                }
                return cmd_retval::CMD_RETURN_ERROR;
            }
        } else if args_has(args, b'n') != 0 {
            tablename = c"root".as_ptr();
        } else {
            tablename = c"prefix".as_ptr();
        }
        key_bindings_remove(tablename, key);
        cmd_retval::CMD_RETURN_NORMAL
    }
}

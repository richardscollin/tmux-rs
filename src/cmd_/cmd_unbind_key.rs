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

pub static CMD_UNBIND_KEY_ENTRY: cmd_entry = cmd_entry {
    name: "unbind-key",
    alias: Some("unbind"),

    args: args_parse::new("anqT:", 0, 1, None),
    usage: "[-anq] [-T key-table] key",

    flags: cmd_flag::CMD_AFTERHOOK,
    exec: cmd_unbind_key_exec,
    source: cmd_entry_flag::zeroed(),
    target: cmd_entry_flag::zeroed(),
};

unsafe fn cmd_unbind_key_exec(self_: *mut cmd, item: *mut cmdq_item) -> cmd_retval {
    unsafe {
        let args = cmd_get_args(self_);
        let mut tablename: *const u8;
        let keystr = args_string(args, 0);
        let quiet = args_has(args, 'q');

        if args_has(args, 'a') {
            if !keystr.is_null() {
                if !quiet {
                    cmdq_error!(item, "key given with -a");
                }
                return cmd_retval::CMD_RETURN_ERROR;
            }

            tablename = args_get(args, b'T');
            if tablename.is_null() {
                if args_has(args, 'n') {
                    tablename = c!("root");
                } else {
                    tablename = c!("prefix");
                }
            }
            if key_bindings_get_table(tablename, false).is_null() {
                if !quiet {
                    cmdq_error!(item, "table {} doesn't exist", _s(tablename));
                }
                return cmd_retval::CMD_RETURN_ERROR;
            }

            key_bindings_remove_table(tablename);
            return cmd_retval::CMD_RETURN_NORMAL;
        }

        if keystr.is_null() {
            if !quiet {
                cmdq_error!(item, "missing key");
            }
            return cmd_retval::CMD_RETURN_ERROR;
        }

        let key = key_string_lookup_string(keystr);
        if key == KEYC_NONE || key == KEYC_UNKNOWN {
            if !quiet {
                cmdq_error!(item, "unknown key unbind: {}", _s(keystr));
            }
            return cmd_retval::CMD_RETURN_ERROR;
        }

        if args_has(args, 'T') {
            tablename = args_get(args, b'T');
            if key_bindings_get_table(tablename, false).is_null() {
                if !quiet {
                    cmdq_error!(item, "table {} doesn't exist", _s(tablename));
                }
                return cmd_retval::CMD_RETURN_ERROR;
            }
        } else if args_has(args, 'n') {
            tablename = c!("root");
        } else {
            tablename = c!("prefix");
        }
        key_bindings_remove(tablename, key);
        cmd_retval::CMD_RETURN_NORMAL
    }
}

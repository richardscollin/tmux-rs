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
use crate::compat::strlcat;
use crate::libc::strcmp;
use crate::*;
use crate::options_::options_get_number___;

pub static CMD_LIST_KEYS_ENTRY: cmd_entry = cmd_entry {
    name: "list-keys",
    alias: Some("lsk"),

    args: args_parse::new("1aNP:T:", 0, 1, None),
    usage: "[-1aN] [-P prefix-string] [-T key-table] [key]",

    flags: cmd_flag::CMD_STARTSERVER.union(cmd_flag::CMD_AFTERHOOK),
    exec: cmd_list_keys_exec,
    source: cmd_entry_flag::zeroed(),
    target: cmd_entry_flag::zeroed(),
};

pub static CMD_LIST_COMMANDS_ENTRY: cmd_entry = cmd_entry {
    name: "list-commands",
    alias: Some("lscm"),

    args: args_parse::new("F:", 0, 1, None),
    usage: "[-F format] [command]",

    flags: cmd_flag::CMD_STARTSERVER.union(cmd_flag::CMD_AFTERHOOK),
    exec: cmd_list_keys_exec,
    source: cmd_entry_flag::zeroed(),
    target: cmd_entry_flag::zeroed(),
};

unsafe fn cmd_list_keys_get_width(tablename: *const u8, only: key_code) -> u32 {
    unsafe {
        let mut keywidth = 0u32;

        let table = key_bindings_get_table(tablename, false);
        if table.is_null() {
            return 0;
        }
        let mut bd = key_bindings_first(table);
        while !bd.is_null() {
            if (only != KEYC_UNKNOWN && (*bd).key != only)
                || KEYC_IS_MOUSE((*bd).key)
                || (*bd).note.is_null()
                || *(*bd).note == b'\0'
            {
                bd = key_bindings_next(table, bd);
                continue;
            }
            let width = utf8_cstrwidth(key_string_lookup_key((*bd).key, 0));
            if width > keywidth {
                keywidth = width;
            }

            bd = key_bindings_next(table, bd);
        }
        keywidth
    }
}

unsafe fn cmd_list_keys_print_notes(
    item: *mut cmdq_item,
    args: *mut args,
    tablename: *const u8,
    keywidth: u32,
    only: key_code,
    prefix: *const u8,
) -> i32 {
    unsafe {
        let tc = cmdq_get_target_client(item);
        let mut found = 0;

        let table = key_bindings_get_table(tablename, false);
        if table.is_null() {
            return 0;
        }
        let mut bd = key_bindings_first(table);
        while !bd.is_null() {
            if (only != KEYC_UNKNOWN && (*bd).key != only)
                || KEYC_IS_MOUSE((*bd).key)
                || (((*bd).note.is_null() || *(*bd).note == b'\0') && !args_has(args, 'a'))
            {
                bd = key_bindings_next(table, bd);
                continue;
            }
            found = 1;
            let key = key_string_lookup_key((*bd).key, 0);

            let note = if (*bd).note.is_null() || *(*bd).note == b'\0' {
                cmd_list_print(&*(*bd).cmdlist, 1)
            } else {
                xstrdup((*bd).note).as_ptr()
            };

            let tmp = utf8_padcstr(key, keywidth + 1);
            if args_has(args, '1') && !tc.is_null() {
                status_message_set!(tc, -1, 1, false, "{}{}{}", _s(prefix), _s(tmp), _s(note));
            } else {
                cmdq_print!(item, "{}{}{}", _s(prefix), _s(tmp), _s(note));
            }
            free_(tmp);
            free_(note);

            if args_has(args, '1') {
                break;
            }
            bd = key_bindings_next(table, bd);
        }
        found
    }
}

unsafe fn cmd_list_keys_get_prefix(args: *mut args, prefix: *mut key_code) -> NonNull<u8> {
    unsafe {
        *prefix = options_get_number___::<i64>(&*GLOBAL_S_OPTIONS, "prefix") as _;
        if !args_has(args, 'P') {
            if *prefix != KEYC_NONE {
                let s = format_nul!("{} ", _s(key_string_lookup_key(*prefix, 0)));
                NonNull::new(s).unwrap()
            } else {
                xstrdup_(c"")
            }
        } else {
            xstrdup(args_get_(args, 'P'))
        }
    }
}

unsafe fn cmd_list_keys_exec(self_: *mut cmd, item: *mut cmdq_item) -> cmd_retval {
    unsafe {
        let args = cmd_get_args(self_);
        let tc = cmdq_get_target_client(item);
        let mut table: *mut key_table;
        let mut width: i32;
        let mut prefix: key_code = 0;
        let mut keywidth: i32;
        let mut found = 0;
        let mut only: key_code = KEYC_UNKNOWN;

        if std::ptr::eq(cmd_get_entry(self_), &CMD_LIST_COMMANDS_ENTRY) {
            return cmd_list_keys_commands(self_, item);
        }

        'out: {
            let keystr = args_string(args, 0);
            if !keystr.is_null() {
                only = key_string_lookup_string(keystr);
                if only == KEYC_UNKNOWN {
                    cmdq_error!(item, "invalid key: {}", _s(keystr));
                    return cmd_retval::CMD_RETURN_ERROR;
                }
                only &= KEYC_MASK_KEY | KEYC_MASK_MODIFIERS;
            }

            let tablename = args_get(args, b'T');
            if !tablename.is_null() && key_bindings_get_table(tablename, false).is_null() {
                cmdq_error!(item, "table {} doesn't exist", _s(tablename));
                return cmd_retval::CMD_RETURN_ERROR;
            }

            if args_has(args, 'N') {
                let start;
                if tablename.is_null() {
                    start = cmd_list_keys_get_prefix(args, &raw mut prefix).as_ptr();
                    keywidth = cmd_list_keys_get_width(c!("root"), only) as _;
                    if prefix != KEYC_NONE {
                        width = cmd_list_keys_get_width(c!("prefix"), only) as _;
                        if width == 0 {
                            prefix = KEYC_NONE;
                        } else if width > keywidth {
                            keywidth = width;
                        }
                    }
                    let empty = utf8_padcstr(c!(""), utf8_cstrwidth(start));

                    found = cmd_list_keys_print_notes(
                        item,
                        args,
                        c!("root"),
                        keywidth as _,
                        only,
                        empty,
                    );
                    if prefix != KEYC_NONE
                        && cmd_list_keys_print_notes(
                            item,
                            args,
                            c!("prefix"),
                            keywidth as _,
                            only,
                            start,
                        ) != 0
                    {
                        found = 1;
                    }
                    free_(empty);
                } else {
                    start = if args_has(args, 'P') {
                        xstrdup(args_get_(args, 'P')).as_ptr()
                    } else {
                        xstrdup(c!("")).as_ptr()
                    };
                    keywidth = cmd_list_keys_get_width(tablename, only) as _;
                    found = cmd_list_keys_print_notes(
                        item,
                        args,
                        tablename,
                        keywidth as _,
                        only,
                        start,
                    );
                }
                free_(start);
                break 'out;
            }

            let mut repeat = 0;
            let mut tablewidth = 0;
            keywidth = 0;
            table = key_bindings_first_table();
            while !table.is_null() {
                if !tablename.is_null() && strcmp((*table).name, tablename) != 0 {
                    table = key_bindings_next_table(table);
                    continue;
                }
                let mut bd = key_bindings_first(table);
                while !bd.is_null() {
                    if only != KEYC_UNKNOWN && (*bd).key != only {
                        bd = key_bindings_next(table, bd);
                        continue;
                    }
                    let key = args_escape(key_string_lookup_key((*bd).key, 0));

                    if (*bd).flags & KEY_BINDING_REPEAT != 0 {
                        repeat = 1;
                    }

                    width = utf8_cstrwidth((*table).name) as _;
                    if width > tablewidth {
                        tablewidth = width;
                    }
                    width = utf8_cstrwidth(key) as _;
                    if width > keywidth {
                        keywidth = width;
                    }

                    free_(key);
                    bd = key_bindings_next(table, bd);
                }
                table = key_bindings_next_table(table);
            }

            let mut tmpsize: usize = 256;
            let mut tmp: NonNull<u8> = xmalloc(tmpsize).cast();

            table = key_bindings_first_table();
            while !table.is_null() {
                if !tablename.is_null() && strcmp((*table).name, tablename) != 0 {
                    table = key_bindings_next_table(table);
                    continue;
                }
                let mut bd = key_bindings_first(table);
                while !bd.is_null() {
                    if only != KEYC_UNKNOWN && (*bd).key != only {
                        bd = key_bindings_next(table, bd);
                        continue;
                    }
                    found = 1;
                    let key = args_escape(key_string_lookup_key((*bd).key, 0));

                    let r = if repeat == 0 {
                        ""
                    } else if (*bd).flags & KEY_BINDING_REPEAT != 0 {
                        "-r "
                    } else {
                        "   "
                    };
                    let mut tmpused: usize =
                        xsnprintf_!(tmp.as_ptr(), tmpsize, "{}-T ", r).unwrap() as _;

                    let mut cp = utf8_padcstr((*table).name, tablewidth as _);
                    let mut cplen = strlen(cp) + 1;
                    while tmpused + cplen + 1 >= tmpsize {
                        tmpsize *= 2;
                        tmp = xrealloc_(tmp.as_ptr(), tmpsize);
                    }
                    strlcat(tmp.as_ptr(), cp, tmpsize);
                    tmpused = strlcat(tmp.as_ptr(), c!(" "), tmpsize as _);
                    free_(cp);

                    cp = utf8_padcstr(key, keywidth as _);
                    cplen = strlen(cp) + 1;
                    while tmpused + cplen + 1 >= tmpsize {
                        tmpsize *= 2;
                        tmp = xrealloc_(tmp.as_ptr(), tmpsize);
                    }
                    strlcat(tmp.as_ptr(), cp, tmpsize);
                    tmpused = strlcat(tmp.as_ptr(), c!(" "), tmpsize);
                    free_(cp);

                    cp = cmd_list_print(&*(*bd).cmdlist, 1);
                    cplen = strlen(cp);
                    while tmpused + cplen + 1 >= tmpsize {
                        tmpsize *= 2;
                        tmp = xrealloc_(tmp.as_ptr(), tmpsize);
                    }
                    strlcat(tmp.as_ptr(), cp, tmpsize);
                    free_(cp);

                    if args_has(args, '1') && tc.is_null() {
                        status_message_set!(tc, -1, 1, false, "bind-key {}", _s(tmp.as_ptr()));
                    } else {
                        cmdq_print!(item, "bind-key {}", _s(tmp.as_ptr()));
                    }
                    free_(key);

                    if args_has(args, '1') {
                        break;
                    }
                    bd = key_bindings_next(table, bd);
                }
                table = key_bindings_next_table(table);
            }

            free_(tmp.as_ptr());
        }

        if only != KEYC_UNKNOWN && found == 0 {
            cmdq_error!(item, "unknown key list: {}", _s(args_string(args, 0)));
            return cmd_retval::CMD_RETURN_ERROR;
        }
        cmd_retval::CMD_RETURN_NORMAL
    }
}

unsafe fn cmd_list_keys_commands(self_: *mut cmd, item: *mut cmdq_item) -> cmd_retval {
    unsafe {
        let args = cmd_get_args(self_);

        let mut template = args_get_(args, 'F');
        if template.is_null() {
            template = cstring_concat!(
                "#{command_list_name}",
                "#{?command_list_alias, (#{command_list_alias}),} ",
                "#{command_list_usage}"
            )
            .as_ptr()
            .cast();
        }

        let ft = format_create(
            cmdq_get_client(item),
            item,
            FORMAT_NONE,
            format_flags::empty(),
        );
        format_defaults(ft, null_mut(), None, None, None);

        let command = args_string(args, 0);

        for entry in CMD_TABLE {
            if !command.is_null()
                && (!streq_(command, entry.name)
                    && entry.alias.is_none_or(|alias| !streq_(command, alias)))
            {
                continue;
            }

            format_add!(ft, "command_list_name", "{}", entry.name);
            format_add!(
                ft,
                "command_list_alias",
                "{}",
                entry.alias.unwrap_or_default()
            );
            format_add!(ft, "command_list_usage", "{}", entry.usage);

            let line = format_expand(ft, template);
            if *line != b'\0' {
                cmdq_print!(item, "{}", _s(line));
            }
            free_(line);
        }

        format_free(ft);
        cmd_retval::CMD_RETURN_NORMAL
    }
}

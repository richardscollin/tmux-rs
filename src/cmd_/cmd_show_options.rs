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

pub static CMD_SHOW_OPTIONS_ENTRY: cmd_entry = cmd_entry {
    name: SyncCharPtr::new(c"show-options"),
    alias: SyncCharPtr::new(c"show"),

    args: args_parse::new(c"AgHpqst:vw", 0, 1, None),
    usage: SyncCharPtr::new(c"[-AgHpqsvw] [-t target-pane] [option]"),

    target: cmd_entry_flag::new(
        b't',
        cmd_find_type::CMD_FIND_PANE,
        cmd_find_flags::CMD_FIND_CANFAIL,
    ),

    flags: cmd_flag::CMD_AFTERHOOK,
    exec: cmd_show_options_exec,
    source: cmd_entry_flag::zeroed(),
};

pub static CMD_SHOW_WINDOW_OPTIONS_ENTRY: cmd_entry = cmd_entry {
    name: SyncCharPtr::new(c"show-window-options"),
    alias: SyncCharPtr::new(c"showw"),

    args: args_parse::new(c"gvt:", 0, 1, None),
    usage: SyncCharPtr::new(c"[-gv] [-t target-window] [option]"),

    target: cmd_entry_flag::new(
        b't',
        cmd_find_type::CMD_FIND_WINDOW,
        cmd_find_flags::CMD_FIND_CANFAIL,
    ),

    flags: cmd_flag::CMD_AFTERHOOK,
    exec: cmd_show_options_exec,

    source: cmd_entry_flag::zeroed(),
};

pub static CMD_SHOW_HOOKS_ENTRY: cmd_entry = cmd_entry {
    name: SyncCharPtr::new(c"show-hooks"),
    alias: SyncCharPtr::null(),

    args: args_parse::new(c"gpt:w", 0, 1, None),
    usage: SyncCharPtr::new(c"[-gpw] [-t target-pane]"),

    target: cmd_entry_flag::new(
        b't',
        cmd_find_type::CMD_FIND_PANE,
        cmd_find_flags::CMD_FIND_CANFAIL,
    ),

    flags: cmd_flag::CMD_AFTERHOOK,
    exec: cmd_show_options_exec,

    source: cmd_entry_flag::zeroed(),
};

unsafe fn cmd_show_options_exec(self_: *mut cmd, item: *mut cmdq_item) -> cmd_retval {
    unsafe {
        let args = cmd_get_args(self_);
        let target = cmdq_get_target(item);
        let mut oo: *mut options = null_mut();
        let argument: *mut u8;
        let name: *mut u8;
        let mut cause: *mut u8 = null_mut();

        let mut idx = 0;
        let mut ambiguous = 0;
        let parent;
        let mut o: *mut options_entry;

        let window = std::ptr::eq(cmd_get_entry(self_), &CMD_SHOW_WINDOW_OPTIONS_ENTRY) as i32;

        'fail: {
            'out: {
                if args_count(args) == 0 {
                    let scope =
                        options_scope_from_flags(args, window, target, &raw mut oo, &raw mut cause);
                    if scope == OPTIONS_TABLE_NONE {
                        if args_has_(args, 'q') {
                            return cmd_retval::CMD_RETURN_NORMAL;
                        }
                        cmdq_error!(item, "{}", _s(cause));
                        free_(cause);
                        return cmd_retval::CMD_RETURN_ERROR;
                    }
                    return cmd_show_options_all(self_, item, scope, oo);
                }
                argument = format_single_from_target(item, args_string(args, 0));

                name = options_match(argument, &raw mut idx, &raw mut ambiguous);
                if name.is_null() {
                    if args_has_(args, 'q') {
                        break 'out;
                    }
                    if ambiguous != 0 {
                        cmdq_error!(item, "ambiguous option: {}", _s(argument));
                    } else {
                        cmdq_error!(item, "invalid option: {}", _s(argument));
                    }
                    break 'fail;
                }
                let scope = options_scope_from_name(
                    args,
                    window,
                    name,
                    target,
                    &raw mut oo,
                    &raw mut cause,
                );
                if scope == OPTIONS_TABLE_NONE {
                    if args_has_(args, 'q') {
                        break 'out;
                    }
                    cmdq_error!(item, "{}", _s(cause));
                    free_(cause);
                    break 'fail;
                }
                o = options_get_only(oo, name);
                if args_has_(args, 'A') && o.is_null() {
                    o = options_get(oo, name);
                    parent = 1;
                } else {
                    parent = 0;
                }
                if !o.is_null() {
                    cmd_show_options_print(self_, item, o, idx, parent);
                } else if *name == b'@' as _ {
                    if args_has_(args, 'q') {
                        break 'out;
                    }
                    cmdq_error!(item, "invalid option: {}", _s(argument));
                    break 'fail;
                }
            }
            // out:
            free_(name);
            free_(argument);
            return cmd_retval::CMD_RETURN_NORMAL;
        }
        // fail:
        free_(name);
        free_(argument);
        cmd_retval::CMD_RETURN_ERROR
    }
}

pub unsafe fn cmd_show_options_print(
    self_: *mut cmd,
    item: *mut cmdq_item,
    o: *mut options_entry,
    mut idx: i32,
    parent: i32,
) {
    unsafe {
        let args = cmd_get_args(self_);
        let mut a: *mut options_array_item;
        let mut name = options_name(o);

        let mut tmp = null_mut();
        let escaped;

        if idx != -1 {
            tmp = format_nul!("{}[{}]", _s(name), idx);
            name = tmp;
        } else if options_is_array(o) != 0 {
            a = options_array_first(o);
            if a.is_null() {
                if !args_has_(args, 'v') {
                    cmdq_print!(item, "{}", _s(name));
                }
                return;
            }
            while !a.is_null() {
                idx = options_array_item_index(a) as i32;
                cmd_show_options_print(self_, item, o, idx, parent);
                a = options_array_next(a);
            }
            return;
        }

        let value = options_to_string(o, idx, 0);
        if args_has_(args, 'v') {
            cmdq_print!(item, "{}", _s(value));
        } else if options_is_string(o) != 0 {
            escaped = args_escape(value);
            if parent != 0 {
                cmdq_print!(item, "{}* {}", _s(name), _s(escaped));
            } else {
                cmdq_print!(item, "{} {}", _s(name), _s(escaped));
            }
            free_(escaped);
        } else if parent != 0 {
            cmdq_print!(item, "{}* {}", _s(name), _s(value));
        } else {
            cmdq_print!(item, "{} {}", _s(name), _s(value));
        }
        free_(value);

        free_(tmp);
    }
}

pub unsafe fn cmd_show_options_all(
    self_: *mut cmd,
    item: *mut cmdq_item,
    scope: i32,
    oo: *mut options,
) -> cmd_retval {
    unsafe {
        let args = cmd_get_args(self_);
        let mut o: *mut options_entry;
        let mut parent;

        if !std::ptr::eq(cmd_get_entry(self_), &CMD_SHOW_HOOKS_ENTRY) {
            o = options_first(oo);
            while !o.is_null() {
                if options_table_entry(o).is_null() {
                    cmd_show_options_print(self_, item, o, -1, 0);
                }
                o = options_next(o);
            }
        }
        let mut oe = &raw const OPTIONS_TABLE as *const options_table_entry;
        while !(*oe).name.is_null() {
            if !(*oe).scope & scope != 0 {
                oe = oe.add(1);
                continue;
            }

            if !std::ptr::eq(cmd_get_entry(self_), &CMD_SHOW_HOOKS_ENTRY)
                && !args_has_(args, 'H')
                && ((*oe).flags & OPTIONS_TABLE_IS_HOOK != 0)
                || (std::ptr::eq(cmd_get_entry(self_), &CMD_SHOW_HOOKS_ENTRY)
                    && (!(*oe).flags & OPTIONS_TABLE_IS_HOOK != 0))
            {
                oe = oe.add(1);
                continue;
            }

            o = options_get_only(oo, (*oe).name);
            if o.is_null() {
                if !args_has_(args, 'A') {
                    oe = oe.add(1);
                    continue;
                }
                o = options_get(oo, (*oe).name);
                if o.is_null() {
                    oe = oe.add(1);
                    continue;
                }
                parent = 1;
            } else {
                parent = 0;
            }

            if options_is_array(o) == 0 {
                cmd_show_options_print(self_, item, o, -1, parent);
            } else if let Some(a) = NonNull::new(options_array_first(o)) {
                let mut a = a.as_ptr();
                while !a.is_null() {
                    let idx = options_array_item_index(a);
                    cmd_show_options_print(self_, item, o, idx as i32, parent);
                    a = options_array_next(a);
                }
            } else if !args_has_(args, 'v') {
                let name = options_name(o);
                if parent != 0 {
                    cmdq_print!(item, "{}*", _s(name));
                } else {
                    cmdq_print!(item, "{}", _s(name));
                }
            }

            oe = oe.add(1);
        }
    }
    cmd_retval::CMD_RETURN_NORMAL
}

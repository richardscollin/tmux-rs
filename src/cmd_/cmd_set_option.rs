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
use crate::compat::queue::tailq_foreach;
use crate::*;

pub static CMD_SET_OPTION_ENTRY: cmd_entry = cmd_entry {
    name: SyncCharPtr::new(c"set-option"),
    alias: SyncCharPtr::new(c"set"),

    args: args_parse::new(c"aFgopqst:uUw", 1, 2, Some(cmd_set_option_args_parse)),
    usage: SyncCharPtr::new(c"[-aFgopqsuUw] [-t target-pane] option [value]"),

    target: cmd_entry_flag::new(
        b't',
        cmd_find_type::CMD_FIND_PANE,
        cmd_find_flags::CMD_FIND_CANFAIL,
    ),

    flags: cmd_flag::CMD_AFTERHOOK,
    exec: cmd_set_option_exec,
    source: cmd_entry_flag::zeroed(),
};

pub static CMD_SET_WINDOW_OPTION_ENTRY: cmd_entry = cmd_entry {
    name: SyncCharPtr::new(c"set-window-option"),
    alias: SyncCharPtr::new(c"setw"),

    args: args_parse::new(c"aFgoqt:u", 1, 2, Some(cmd_set_option_args_parse)),
    usage: SyncCharPtr::new(c"[-aFgoqu] [-t target-window] option [value]"),

    target: cmd_entry_flag::new(
        b't',
        cmd_find_type::CMD_FIND_WINDOW,
        cmd_find_flags::CMD_FIND_CANFAIL,
    ),

    flags: cmd_flag::CMD_AFTERHOOK,
    exec: cmd_set_option_exec,
    source: cmd_entry_flag::zeroed(),
};

pub static CMD_SET_HOOK_ENTRY: cmd_entry = cmd_entry {
    name: SyncCharPtr::new(c"set-hook"),
    alias: SyncCharPtr::null(),

    args: args_parse::new(c"agpRt:uw", 1, 2, Some(cmd_set_option_args_parse)),
    usage: SyncCharPtr::new(c"[-agpRuw] [-t target-pane] hook [command]"),

    target: cmd_entry_flag::new(
        b't',
        cmd_find_type::CMD_FIND_PANE,
        cmd_find_flags::CMD_FIND_CANFAIL,
    ),

    flags: cmd_flag::CMD_AFTERHOOK,
    exec: cmd_set_option_exec,
    source: cmd_entry_flag::zeroed(),
};

pub fn cmd_set_option_args_parse(
    _args: *mut args,
    idx: u32,
    _cause: *mut *mut u8,
) -> args_parse_type {
    match idx {
        1 => args_parse_type::ARGS_PARSE_COMMANDS_OR_STRING,
        _ => args_parse_type::ARGS_PARSE_STRING,
    }
}

pub unsafe fn cmd_set_option_exec(self_: *mut cmd, item: *mut cmdq_item) -> cmd_retval {
    unsafe {
        let args = cmd_get_args(self_);
        let append = args_has(args, 'a');
        let target = cmdq_get_target(item);
        let mut oo: *mut options = null_mut();
        let parent: *mut options_entry;
        let mut o: *mut options_entry;
        let name: *mut u8;
        let argument: *mut u8;
        let mut expanded: *mut u8 = null_mut();
        let mut value: *const u8;
        let mut idx: i32 = 0;
        let already: i32;
        let mut ambiguous: i32 = 0;
        let scope: i32;

        'fail: {
            'out: {
                let window =
                    std::ptr::eq(cmd_get_entry(self_), &CMD_SET_WINDOW_OPTION_ENTRY) as i32;

                // Expand argument.
                argument = format_single_from_target(item, args_string(args, 0));

                // If set-hook -R, fire the hook straight away.
                if std::ptr::eq(cmd_get_entry(self_), &CMD_SET_HOOK_ENTRY) && args_has(args, 'R') {
                    notify_hook(item, argument);
                    free_(argument);
                    return cmd_retval::CMD_RETURN_NORMAL;
                }

                // Parse option name and index.
                name = options_match(argument, &raw mut idx, &raw mut ambiguous);
                if name.is_null() {
                    if args_has(args, 'q') {
                        break 'out;
                    }
                    if ambiguous != 0 {
                        cmdq_error!(item, "ambiguous option: {}", _s(argument));
                    } else {
                        cmdq_error!(item, "invalid option: {}", _s(argument));
                    }
                    break 'fail;
                }
                if args_count(args) < 2 {
                    value = null_mut();
                } else {
                    value = args_string(args, 1);
                }
                if !value.is_null() && args_has(args, 'F') {
                    expanded = format_single_from_target(item, value);
                    value = expanded;
                }

                let mut cause = null_mut();
                // Get the scope and table for the option .
                scope = options_scope_from_name(
                    args,
                    window,
                    name,
                    target,
                    &raw mut oo,
                    &raw mut cause,
                );
                if scope == OPTIONS_TABLE_NONE {
                    if args_has(args, 'q') {
                        break 'out;
                    }
                    cmdq_error!(item, "{}", _s(cause));
                    free_(cause);
                    break 'fail;
                }
                o = options_get_only(oo, name);
                parent = options_get(oo, name);

                // Check that array options and indexes match up.
                if idx != -1 && (*name == b'@' as _ || options_is_array(parent) == 0) {
                    cmdq_error!(item, "not an array: {}", _s(argument));
                    break 'fail;
                }

                // With -o, check this option is not already set.
                if !args_has(args, 'u') && args_has(args, 'o') {
                    if idx == -1 {
                        already = !o.is_null() as i32;
                    } else if o.is_null() {
                        already = 0;
                    } else {
                        already = (!options_array_get(o, idx as u32).is_null()) as i32;
                    }
                    if already != 0 {
                        if args_has(args, 'q') {
                            break 'out;
                        }
                        cmdq_error!(item, "already set: {}", _s(argument));
                        break 'fail;
                    }
                }

                // Change the option.
                if args_has(args, 'U') && scope == OPTIONS_TABLE_WINDOW {
                    for loop_ in tailq_foreach::<_, discr_entry>(&raw mut (*(*target).w).panes)
                        .map(NonNull::as_ptr)
                    {
                        let po = options_get_only((*loop_).options, name);
                        if po.is_null() {
                            continue;
                        }
                        if let Err(cause) = options_remove_or_default(po, idx) {
                            cmdq_error!(item, "{}", cause.to_str().unwrap());
                            break 'fail;
                        }
                    }
                }
                if args_has(args, 'u') || args_has(args, 'U') {
                    if o.is_null() {
                        break 'out;
                    }
                    if let Err(cause) = options_remove_or_default(o, idx) {
                        cmdq_error!(item, "{}", cause.to_str().unwrap());
                        break 'fail;
                    }
                } else if *name == b'@' {
                    if value.is_null() {
                        cmdq_error!(item, "empty value");
                        break 'fail;
                    }
                    options_set_string!(oo, name, append, "{}", _s(value));
                } else if idx == -1 && options_is_array(parent) == 0 {
                    if let Err(cause) = options_from_string(
                        oo,
                        options_table_entry(parent),
                        (*options_table_entry(parent)).name,
                        value,
                        args_has(args, 'a'),
                    ) {
                        cmdq_error!(item, "{}", cause.to_str().unwrap());
                        break 'fail;
                    }
                } else {
                    if value.is_null() {
                        cmdq_error!(item, "empty value");
                        break 'fail;
                    }
                    if o.is_null() {
                        o = options_empty(oo, options_table_entry(parent));
                    }
                    if idx == -1 {
                        if !append {
                            options_array_clear(o);
                        }
                        if let Err(cause) = options_array_assign(o, cstr_to_str(value)) {
                            cmdq_error!(item, "{}", cause.to_str().unwrap());
                            break 'fail;
                        }
                    } else if let Err(cause) =
                        options_array_set(o, idx as u32, Some(cstr_to_str(value)), append)
                    {
                        cmdq_error!(item, "{}", cause.to_str().unwrap());
                        break 'fail;
                    }
                }

                options_push_changes(name);
            }
            // out:
            free_(argument);
            free_(expanded);
            free_(name);
            return cmd_retval::CMD_RETURN_NORMAL;
        }
        // fail:
        free_(argument);
        free_(expanded);
        free_(name);
        cmd_retval::CMD_RETURN_ERROR
    }
}

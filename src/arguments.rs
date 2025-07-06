// Copyright (c) 2010 Nicholas Marriott <nicholas.marriott@gmail.com>
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
use std::cmp::Ordering;

use crate::{xmalloc::xrecallocarray, *};

use crate::compat::{
    queue::{
        tailq_empty, tailq_first, tailq_foreach, tailq_init, tailq_insert_tail, tailq_last,
        tailq_next, tailq_remove,
    },
    strlcat,
    tree::{rb_find, rb_foreach, rb_init, rb_insert, rb_min, rb_next, rb_remove},
};

pub type args_values = tailq_head<args_value>;

const ARGS_ENTRY_OPTIONAL_VALUE: c_int = 1;
#[repr(C)]
pub struct args_entry {
    pub flag: c_uchar,
    pub values: args_values,
    pub count: c_uint,

    pub flags: c_int,

    pub entry: rb_entry<args_entry>,
}

#[repr(C)]
pub struct args {
    pub tree: args_tree,
    pub count: u32,
    pub values: *mut args_value,
}

#[repr(C)]
pub struct args_command_state<'a> {
    pub cmdlist: *mut cmd_list,
    pub cmd: *mut u8,
    pub pi: cmd_parse_input<'a>,
}

crate::compat::RB_GENERATE!(args_tree, args_entry, entry, discr_entry, args_cmp);

fn args_cmp(a1: &args_entry, a2: &args_entry) -> Ordering {
    a1.flag.cmp(&a2.flag)
}

pub unsafe fn args_find(args: *mut args, flag: c_uchar) -> *mut args_entry {
    unsafe {
        let mut entry: args_entry = args_entry { flag, ..zeroed() };

        rb_find(&raw mut (*args).tree, &raw mut entry)
    }
}

pub unsafe fn args_copy_value(to: *mut args_value, from: *const args_value) {
    unsafe {
        (*to).type_ = (*from).type_;
        match (*from).type_ {
            args_type::ARGS_NONE => (),
            args_type::ARGS_COMMANDS => {
                (*to).union_.cmdlist = (*from).union_.cmdlist;
                (*(*to).union_.cmdlist).references += 1;
            }
            args_type::ARGS_STRING => {
                (*to).union_.string = xstrdup((*from).union_.string).cast().as_ptr();
            }
        }
    }
}

pub fn args_type_to_string(type_: args_type) -> &'static str {
    match type_ {
        args_type::ARGS_NONE => "NONE",
        args_type::ARGS_STRING => "STRING",
        args_type::ARGS_COMMANDS => "COMMANDS",
    }
}

pub unsafe fn args_value_as_string(value: *mut args_value) -> *const u8 {
    unsafe {
        match (*value).type_ {
            args_type::ARGS_NONE => c!(""),
            args_type::ARGS_STRING => (*value).union_.string,
            args_type::ARGS_COMMANDS => {
                if (*value).cached.is_null() {
                    (*value).cached = cmd_list_print(&mut *(*value).union_.cmdlist, 0);
                }
                (*value).cached
            }
        }
    }
}

impl args {
    fn create() -> Box<Self> {
        Box::new(Self {
            tree: rb_head::rb_init(),
            count: 0,
            values: null_mut(),
        })
    }
}

pub fn args_create<'a>() -> &'a mut args {
    Box::leak(args::create())
}

pub unsafe fn args_parse_flag_argument(
    values: *mut args_value,
    count: u32,
    cause: *mut *mut u8,
    args: *mut args,
    i: *mut u32,
    string: *mut u8,
    flag: i32,
    optional_argument: i32,
) -> i32 {
    let argument: *mut args_value;
    let new: *mut args_value;
    unsafe {
        'out: {
            new = xcalloc(1, size_of::<args_value>()).cast().as_ptr();

            if *string != b'\0' {
                (*new).type_ = args_type::ARGS_STRING;
                (*new).union_.string = xstrdup(string).cast().as_ptr();
                break 'out;
            }

            if *i == count {
                argument = null_mut();
            } else {
                argument = values.add(*i as usize);
                if (*argument).type_ != args_type::ARGS_STRING {
                    *cause = format_nul!("-{} argument must be a string", flag as u8 as char);
                    args_free_value(new);
                    free(new as _);
                    return -1;
                }
            }

            if argument.is_null() {
                args_free_value(new);
                free(new as _);
                if optional_argument != 0 {
                    log_debug!("{}: -{} (optional)", "args_parse_flag_argument", flag);
                    args_set(args, flag as c_uchar, null_mut(), ARGS_ENTRY_OPTIONAL_VALUE);
                    return 0; /* either - or end */
                }
                *cause = format_nul!("-{} expects an argument", flag as u8 as char);
                return -1;
            }

            args_copy_value(new, argument);
            (*i) += 1;

            break 'out;
        }
        // out:
        let s = args_value_as_string(new);
        log_debug!("{}: -{} = {}", "args_parse_flag_argument", flag, _s(s));
        args_set(args, flag as c_uchar, new, 0);
    }

    0
}

pub unsafe fn args_parse_flags(
    parse: *const args_parse,
    values: *mut args_value,
    count: u32,
    cause: *mut *mut u8,
    args: *mut args,
    i: *mut u32,
) -> i32 {
    let __func__ = "args_parse_flags";
    unsafe {
        let value = values.add(*i as usize);
        if (*value).type_ != args_type::ARGS_STRING {
            return 1;
        }

        let mut string = (*value).union_.string;
        log_debug!("{}: next {}", __func__, _s(string));
        if ({
            let tmp = *string != b'-';
            string = string.add(1);
            tmp
        }) || *string == b'\0' as _
        {
            return 1;
        }
        (*i) += 1;
        if *string == b'-' as _ && *string.add(1) == b'\0' as _ {
            return 1;
        }

        loop {
            let flag = *string as c_uchar;
            string = string.add(1);
            if flag == b'\0' {
                return 0;
            }
            if flag == b'?' {
                return -1;
            }
            if !flag.is_ascii_alphanumeric() {
                *cause = format_nul!("invalid flag -{}", flag as char);
                return -1;
            }

            let found = libc::strchr((*parse).template.as_ptr(), flag as i32);
            if found.is_null() {
                *cause = format_nul!("unknown flag -{}", flag as char);
                return -1;
            }
            if *found.add(1) != b':' {
                log_debug!("{}: -{}", __func__, flag as i32);
                args_set(args, flag, null_mut(), 0);
                continue;
            }
            let optional_argument = (*found.add(2) == b':') as i32;
            return args_parse_flag_argument(
                values,
                count,
                cause,
                args,
                i,
                string,
                flag as i32,
                optional_argument,
            );
        }
    }
}

/// Parse arguments into a new argument set.
pub unsafe fn args_parse(
    parse: *const args_parse,
    values: *mut args_value,
    count: u32,
    cause: *mut *mut u8,
) -> *mut args {
    let __func__ = "args_parse";
    unsafe {
        let mut type_: args_parse_type;

        if count == 0 {
            return args_create();
        }

        let args = args_create();

        let mut i: u32 = 1;
        while i < count {
            let stop = args_parse_flags(parse, values, count, cause, args, &raw mut i);
            if stop == -1 {
                args_free(args);
                return null_mut();
            }
            if stop == 1 {
                break;
            }
        }
        log_debug!("{}: flags end at {} of {}", __func__, i, count);
        if i != count {
            while i < count {
                let value = values.add(i as usize);

                let s = args_value_as_string(value);
                log_debug!(
                    "{}: {} = {} (type {})",
                    __func__,
                    i,
                    _s(s),
                    args_type_to_string((*value).type_),
                );

                if let Some(cb) = (*parse).cb {
                    type_ = cb(args, args.count, cause);
                    if type_ == args_parse_type::ARGS_PARSE_INVALID {
                        args_free(args);
                        return null_mut();
                    }
                } else {
                    type_ = args_parse_type::ARGS_PARSE_STRING;
                }

                args.values = xrecallocarray(
                    args.values.cast(),
                    args.count as usize,
                    args.count as usize + 1,
                    size_of::<args_value>(),
                )
                .cast()
                .as_ptr();
                let new = args.values.add(args.count as usize);
                args.count += 1;

                match type_ {
                    args_parse_type::ARGS_PARSE_INVALID => fatalx("unexpected argument type"),
                    args_parse_type::ARGS_PARSE_STRING => {
                        if (*value).type_ != args_type::ARGS_STRING {
                            *cause = format_nul!("argument {} must be \"string\"", args.count);
                            args_free(args);
                            return null_mut();
                        }
                        args_copy_value(new, value);
                    }
                    args_parse_type::ARGS_PARSE_COMMANDS_OR_STRING => args_copy_value(new, value),
                    args_parse_type::ARGS_PARSE_COMMANDS => {
                        if (*value).type_ != args_type::ARGS_COMMANDS {
                            *cause = format_nul!("argument {} must be {{ commands }}", args.count,);
                            args_free(args);
                            return null_mut();
                        }
                        args_copy_value(new, value);
                    }
                }
                i += 1;
            }
        }

        if (*parse).lower != -1 && args.count < (*parse).lower as u32 {
            *cause = format_nul!("too few arguments (need at least {})", (*parse).lower);
            args_free(args);
            return null_mut();
        }
        if (*parse).upper != -1 && args.count > (*parse).upper as u32 {
            *cause = format_nul!("too many arguments (need at most {})", (*parse).upper);
            args_free(args);
            return null_mut();
        }
        args
    }
}

pub unsafe fn args_copy_copy_value(
    to: *mut args_value,
    from: *mut args_value,
    argc: i32,
    argv: *mut *mut u8,
) {
    unsafe {
        (*to).type_ = (*from).type_;
        match (*from).type_ {
            args_type::ARGS_NONE => (),
            args_type::ARGS_STRING => {
                let mut expanded = xstrdup((*from).union_.string).as_ptr();
                for i in 0..argc {
                    let s = cmd_template_replace(expanded, *argv.add(i as usize), i + 1);
                    free_(expanded);
                    expanded = s;
                }
                (*to).union_.string = expanded;
            }
            args_type::ARGS_COMMANDS => {
                (*to).union_.cmdlist = cmd_list_copy(&mut *(*from).union_.cmdlist, argc, argv)
            }
        }
    }
}

/// Copy an arguments set.
pub unsafe fn args_copy(args: *mut args, argc: i32, argv: *mut *mut u8) -> *mut args {
    let __func__ = "args_copy";
    unsafe {
        cmd_log_argv!(argc, argv, "{__func__}");

        let new_args = args_create();
        for entry in rb_foreach(&raw mut (*args).tree).map(NonNull::as_ptr) {
            if tailq_empty(&raw mut (*entry).values) {
                for _ in 0..(*entry).count {
                    args_set(new_args, (*entry).flag, null_mut(), 0);
                }
                continue;
            }
            for value in tailq_foreach(&raw mut (*entry).values) {
                let new_value = xcalloc1();
                args_copy_copy_value(new_value, value.as_ptr(), argc, argv);
                args_set(new_args, (*entry).flag, new_value, 0);
            }
        }
        if (*args).count == 0 {
            return new_args;
        }
        new_args.count = (*args).count;
        new_args.values = xcalloc_((*args).count as usize).as_ptr();
        for i in 0..(*args).count {
            let new_value = new_args.values.add(i as usize);
            args_copy_copy_value(new_value, (*args).values.add(i as usize), argc, argv);
        }

        new_args
    }
}

pub unsafe fn args_free_value(value: *mut args_value) {
    unsafe {
        match (*value).type_ {
            args_type::ARGS_NONE => (),
            args_type::ARGS_STRING => free_((*value).union_.string),
            args_type::ARGS_COMMANDS => cmd_list_free((*value).union_.cmdlist),
        }
        free_((*value).cached);
    }
}

pub unsafe fn args_free_values(values: *mut args_value, count: u32) {
    unsafe {
        for i in 0..count {
            args_free_value(values.add(i as usize));
        }
    }
}

pub unsafe fn args_free(args: *mut args) {
    unsafe {
        args_free_values((*args).values, (*args).count);
        free_((*args).values);

        for entry in rb_foreach(&raw mut (*args).tree).map(NonNull::as_ptr) {
            rb_remove(&raw mut (*args).tree, entry);
            for value in tailq_foreach(&raw mut (*entry).values).map(NonNull::as_ptr) {
                tailq_remove(&raw mut (*entry).values, value);
                args_free_value(value);
                free_(value);
            }
            free_(entry);
        }

        free_(args);
    }
}

pub unsafe fn args_to_vector(args: *mut args, argc: *mut i32, argv: *mut *mut *mut u8) {
    unsafe {
        *argc = 0;
        *argv = null_mut();

        for i in 0..(*args).count {
            match (*(*args).values.add(i as usize)).type_ {
                args_type::ARGS_NONE => (),
                args_type::ARGS_STRING => {
                    cmd_append_argv(argc, argv, (*(*args).values.add(i as usize)).union_.string)
                }
                args_type::ARGS_COMMANDS => {
                    let s =
                        cmd_list_print(&mut *(*(*args).values.add(i as usize)).union_.cmdlist, 0);
                    cmd_append_argv(argc, argv, s);
                    free_(s);
                }
            }
        }
    }
}

pub unsafe fn args_from_vector(argc: i32, argv: *mut *mut u8) -> *mut args_value {
    unsafe {
        let values: *mut args_value = xcalloc_(argc as usize).as_ptr();
        for i in 0..argc {
            (*values.add(i as usize)).type_ = args_type::ARGS_STRING;
            (*values.add(i as usize)).union_.string = xstrdup(*argv.add(i as usize)).as_ptr();
        }
        values
    }
}

macro_rules! args_print_add {
   ($buf:expr, $len:expr, $fmt:literal $(, $args:expr)* $(,)?) => {
        crate::arguments::args_print_add_($buf, $len, format_args!($fmt $(, $args)*))
    };
}
pub(crate) use args_print_add;
pub unsafe fn args_print_add_(buf: *mut *mut u8, len: *mut usize, fmt: std::fmt::Arguments) {
    unsafe {
        let s = fmt.to_string();

        *len += s.len();
        *buf = xrealloc(*buf as *mut c_void, *len).cast().as_ptr();

        strlcat(*buf, s.as_ptr().cast(), *len);
    }
}

pub unsafe fn args_print_add_value(buf: *mut *mut u8, len: *mut usize, value: *mut args_value) {
    unsafe {
        if **buf != b'\0' {
            args_print_add!(buf, len, " ");
        }

        match (*value).type_ {
            args_type::ARGS_NONE => (),
            args_type::ARGS_COMMANDS => {
                let expanded = cmd_list_print(&mut *(*value).union_.cmdlist, 0);
                args_print_add!(buf, len, "{{ {} }}", _s(expanded));
                free_(expanded);
            }
            args_type::ARGS_STRING => {
                let expanded = args_escape((*value).union_.string);
                args_print_add!(buf, len, "{}", _s(expanded));
                free_(expanded);
            }
        }
    }
}

pub unsafe fn args_print(args: *mut args) -> *mut u8 {
    unsafe {
        let mut last: *mut args_entry = null_mut();

        let mut len: usize = 1;
        let mut buf: *mut u8 = xcalloc(1, len).cast().as_ptr();

        /* Process the flags first. */
        for entry in rb_foreach(&raw mut (*args).tree).map(NonNull::as_ptr) {
            if (*entry).flags & ARGS_ENTRY_OPTIONAL_VALUE != 0 {
                continue;
            }
            if !tailq_empty(&raw mut (*entry).values) {
                continue;
            }

            if *buf == b'\0' {
                args_print_add!(&raw mut buf, &raw mut len, "-");
            }
            for _ in 0..(*entry).count {
                args_print_add!(&raw mut buf, &raw mut len, "{}", (*entry).flag as char);
            }
        }

        /* Then the flags with arguments. */
        for entry in rb_foreach(&raw mut (*args).tree).map(NonNull::as_ptr) {
            if (*entry).flags & ARGS_ENTRY_OPTIONAL_VALUE != 0 {
                if *buf != b'\0' {
                    args_print_add!(&raw mut buf, &raw mut len, " -{}", (*entry).flag as char);
                } else {
                    args_print_add!(&raw mut buf, &raw mut len, "-{}", (*entry).flag as char,);
                }
                last = entry;
                continue;
            }
            if tailq_empty(&raw mut (*entry).values) {
                continue;
            }
            for value in tailq_foreach(&raw mut (*entry).values) {
                {
                    if *buf != b'\0' {
                        args_print_add!(&raw mut buf, &raw mut len, " -{}", (*entry).flag as char,);
                    } else {
                        args_print_add!(&raw mut buf, &raw mut len, "-{}", (*entry).flag as char,);
                    }
                    args_print_add_value(&raw mut buf, &raw mut len, value.as_ptr());
                }
            }
            last = entry;
        }
        if !last.is_null() && ((*last).flags & ARGS_ENTRY_OPTIONAL_VALUE != 0) {
            args_print_add!(&raw mut buf, &raw mut len, " --");
        }

        /* And finally the argument vector. */
        for i in 0..(*args).count {
            args_print_add_value(&raw mut buf, &raw mut len, (*args).values.add(i as usize));
        }

        buf
    }
}

/// Escape an argument.
pub unsafe fn args_escape(s: *const u8) -> *mut u8 {
    unsafe {
        let dquoted: *const u8 = c!(" #';${}%");
        let squoted: *const u8 = c!(" \"");

        let mut escaped: *mut u8 = null_mut();

        let mut quotes: i32 = 0;

        if *s == b'\0' {
            return format_nul!("''");
        }
        if *s.add(libc::strcspn(s, dquoted)) != b'\0' as _ {
            quotes = b'"' as _;
        } else if *s.add(libc::strcspn(s, squoted)) != b'\0' as _ {
            quotes = b'\'' as _;
        }

        if *s != b' ' as _ && *s.add(1) == b'\0' as _ && (quotes != 0 || *s == b'~' as _) {
            escaped = format_nul!("\\{}", *s as char);
            return escaped;
        }

        let mut flags =
            vis_flags::VIS_OCTAL | vis_flags::VIS_CSTYLE | vis_flags::VIS_TAB | vis_flags::VIS_NL;
        if quotes == b'"' as _ {
            flags |= vis_flags::VIS_DQ;
        }
        utf8_stravis(&raw mut escaped, s, flags);

        let result = if quotes == b'\'' as i32 {
            format_nul!("'{}'", _s(escaped))
        } else if quotes == b'"' as i32 {
            if *escaped == b'~' {
                format_nul!("\"\\{}\"", _s(escaped))
            } else {
                format_nul!("\"{}\"", _s(escaped))
            }
        } else if *escaped == b'~' {
            format_nul!("\\{}", _s(escaped))
        } else {
            xstrdup(escaped).as_ptr()
        };
        free_(escaped);

        result
    }
}

pub unsafe fn args_has(args: *mut args, flag: u8) -> i32 {
    unsafe {
        let entry = args_find(args, flag);
        if entry.is_null() {
            return 0;
        }
        (*entry).count as i32
    }
}

pub unsafe fn args_set(args: *mut args, flag: c_uchar, value: *mut args_value, flags: i32) {
    unsafe {
        let mut entry: *mut args_entry = args_find(args, flag);

        if entry.is_null() {
            entry = xcalloc1();
            (*entry).flag = flag;
            (*entry).count = 1;
            (*entry).flags = flags;
            tailq_init(&raw mut (*entry).values);
            rb_insert(&raw mut (*args).tree, entry);
        } else {
            (*entry).count += 1;
        }
        if !value.is_null() && (*value).type_ != args_type::ARGS_NONE {
            tailq_insert_tail(&raw mut (*entry).values, value);
        } else {
            free_(value);
        }
    }
}

pub unsafe fn args_get(args: *mut args, flag: u8) -> *const u8 {
    unsafe {
        let entry = args_find(args, flag);

        if entry.is_null() {
            return null_mut();
        }
        if tailq_empty(&raw mut (*entry).values) {
            return null_mut();
        }
        (*tailq_last(&raw mut (*entry).values)).union_.string
    }
}

pub unsafe fn args_first(args: *mut args, entry: *mut *mut args_entry) -> u8 {
    unsafe {
        *entry = rb_min(&raw mut (*args).tree);
        if (*entry).is_null() {
            return 0;
        }
        (*(*entry)).flag
    }
}

/// Get next argument.
pub unsafe fn args_next(entry: *mut *mut args_entry) -> u8 {
    unsafe {
        *entry = rb_next(*entry);
        if (*entry).is_null() {
            return 0;
        }
        (*(*entry)).flag
    }
}

/// Get argument count.
pub unsafe fn args_count(args: *const args) -> u32 {
    unsafe { (*args).count }
}

/// Get argument values.
pub unsafe fn args_values(args: *mut args) -> *mut args_value {
    unsafe { (*args).values }
}

/// Get argument value.
pub unsafe fn args_value(args: *mut args, idx: u32) -> *mut args_value {
    unsafe {
        if idx >= (*args).count {
            return null_mut();
        }
        (*args).values.add(idx as usize)
    }
}

/// Return argument as string.
pub unsafe fn args_string(args: *mut args, idx: u32) -> *const u8 {
    unsafe {
        if idx >= (*args).count {
            return null();
        }
        args_value_as_string((*args).values.add(idx as usize))
    }
}

/// Make a command now.
pub unsafe fn args_make_commands_now(
    self_: *mut cmd,
    item: *mut cmdq_item,
    idx: u32,
    expand: i32,
) -> *mut cmd_list {
    unsafe {
        let mut error = null_mut();
        let state = args_make_commands_prepare(self_, item, idx, null_mut(), 0, expand);
        let cmdlist = args_make_commands(state, 0, null_mut(), &raw mut error);
        if cmdlist.is_null() {
            cmdq_error!(item, "{}", _s(error));
            free_(error);
        } else {
            (*cmdlist).references += 1;
        }
        args_make_commands_free(state);
        cmdlist
    }
}

/// Save bits to make a command later.
pub unsafe fn args_make_commands_prepare<'a>(
    self_: *mut cmd,
    item: *mut cmdq_item,
    idx: u32,
    default_command: *const u8,
    wait: i32,
    expand: i32,
) -> *mut args_command_state<'a> {
    let __func__ = "args_make_commands_prepare";
    unsafe {
        let args = cmd_get_args(self_);
        let target = cmdq_get_target(item);
        let tc = cmdq_get_target_client(item);

        let state = xcalloc1::<args_command_state>() as *mut args_command_state;

        let cmd = if idx < (*args).count {
            let value = (*args).values.add(idx as usize);
            if (*value).type_ == args_type::ARGS_COMMANDS {
                (*state).cmdlist = (*value).union_.cmdlist;
                (*(*state).cmdlist).references += 1;
                return state;
            }
            (*value).union_.string
        } else {
            if default_command.is_null() {
                fatalx("argument out of range");
            }
            default_command
        };

        if expand != 0 {
            (*state).cmd = format_single_from_target(item, cmd);
        } else {
            (*state).cmd = xstrdup(cmd).as_ptr();
        }
        log_debug!("{}: {}", __func__, _s((*state).cmd));

        if wait != 0 {
            (*state).pi.item = item;
        }
        let mut file = null();
        cmd_get_source(self_, &raw mut file, &(*state).pi.line);
        if !file.is_null() {
            (*state).pi.file = Some(cstr_to_str(xstrdup(file).as_ptr()));
        }
        (*state).pi.c = tc;
        if !(*state).pi.c.is_null() {
            (*(*state).pi.c).references += 1;
        }
        cmd_find_copy_state(&raw mut (*state).pi.fs, target);

        state
    }
}

/// Return argument as command.
pub unsafe fn args_make_commands(
    state: *mut args_command_state,
    argc: i32,
    argv: *mut *mut u8,
    error: *mut *mut u8,
) -> *mut cmd_list {
    let __func__ = "args_make_commands";
    unsafe {
        if !(*state).cmdlist.is_null() {
            if argc == 0 {
                return (*state).cmdlist;
            }
            return cmd_list_copy(&mut *(*state).cmdlist, argc, argv);
        }

        let mut cmd = xstrdup((*state).cmd).as_ptr();
        log_debug!("{}: {}", __func__, _s(cmd));
        cmd_log_argv!(argc, argv, "args_make_commands");
        for i in 0..argc {
            let new_cmd = cmd_template_replace(cmd, *argv.add(i as usize), i + 1);
            log_debug!(
                "{}: %%{} {}: {}",
                __func__,
                i + 1,
                _s(*argv.add(i as usize)),
                _s(new_cmd)
            );
            free_(cmd);
            cmd = new_cmd;
        }
        log_debug!("{}: {}", __func__, _s(cmd));

        let pr = cmd_parse_from_string(cstr_to_str(cmd), Some(&(*state).pi));
        free_(cmd);

        match pr {
            Err(err) => {
                *error = err;
                null_mut()
            }
            Ok(cmdlist) => cmdlist,
        }
    }
}

/// Free commands state.
pub unsafe fn args_make_commands_free(state: *mut args_command_state) {
    unsafe {
        if !(*state).cmdlist.is_null() {
            cmd_list_free((*state).cmdlist);
        }
        if !(*state).pi.c.is_null() {
            server_client_unref((*state).pi.c);
        }
        free_(
            (*state)
                .pi
                .file
                .map(|p| p.as_ptr())
                .unwrap_or_default()
                .cast_mut(),
        ); // TODO casting away const
        free_((*state).cmd);
        free_(state);
    }
}

/// Get prepared command.
pub unsafe fn args_make_commands_get_command(state: *mut args_command_state) -> *mut u8 {
    unsafe {
        if !(*state).cmdlist.is_null() {
            let first = cmd_list_first((*state).cmdlist);
            if first.is_null() {
                return xstrdup_(c"").as_ptr();
            }
            return xstrdup(cmd_get_entry(first).name.as_ptr()).as_ptr();
        }
        let n = libc::strcspn((*state).cmd, c!(" ,"));
        format_nul!("{1:0$}", n, _s((*state).cmd))
    }
}

/// Get first value in argument.
pub unsafe fn args_first_value(args: *mut args, flag: u8) -> *mut args_value {
    unsafe {
        let entry = args_find(args, flag);
        if entry.is_null() {
            return null_mut();
        }
        tailq_first(&raw mut (*entry).values)
    }
}

/// Get next value in argument.
pub unsafe fn args_next_value(value: *mut args_value) -> *mut args_value {
    unsafe { tailq_next(value) }
}

/// Convert an argument value to a number.
pub unsafe fn args_strtonum(
    args: *mut args,
    flag: u8,
    minval: i64,
    maxval: i64,
    cause: *mut *mut u8,
) -> i64 {
    unsafe {
        let entry = args_find(args, flag);
        if entry.is_null() {
            *cause = xstrdup_(c"missing").as_ptr();
            return 0;
        }
        let value = tailq_last(&raw mut (*entry).values);
        if value.is_null()
            || (*value).type_ != args_type::ARGS_STRING
            || (*value).union_.string.is_null()
        {
            *cause = xstrdup_(c"missing").as_ptr();
            return 0;
        }

        match strtonum((*value).union_.string, minval, maxval) {
            Ok(ll) => {
                *cause = null_mut();
                ll
            }
            Err(errstr) => {
                *cause = xstrdup(errstr.as_ptr().cast()).as_ptr();
                0
            }
        }
    }
}

/// Convert an argument value to a number, and expand formats.
pub unsafe fn args_strtonum_and_expand(
    args: *mut args,
    flag: u8,
    minval: c_longlong,
    maxval: c_longlong,
    item: *mut cmdq_item,
    cause: *mut *mut u8,
) -> c_longlong {
    unsafe {
        let entry = args_find(args, flag);
        if entry.is_null() {
            *cause = xstrdup_(c"missing").as_ptr();
            return 0;
        }
        let value = tailq_last(&raw mut (*entry).values);
        if value.is_null()
            || (*value).type_ != args_type::ARGS_STRING
            || (*value).union_.string.is_null()
        {
            *cause = xstrdup_(c"missing").as_ptr();
            return 0;
        }

        let formatted = format_single_from_target(item, (*value).union_.string);
        let tmp = strtonum(formatted, minval, maxval);
        free_(formatted);
        match tmp {
            Ok(ll) => {
                *cause = null_mut();
                ll
            }
            Err(errstr) => {
                *cause = xstrdup_(errstr).as_ptr();
                0
            }
        }
    }
}

/// Convert an argument to a number which may be a percentage.
pub unsafe fn args_percentage(
    args: *mut args,
    flag: u8,
    minval: i64,
    maxval: i64,
    curval: i64,
    cause: *mut *mut u8,
) -> i64 {
    unsafe {
        let entry = args_find(args, flag);
        if entry.is_null() {
            *cause = xstrdup_(c"missing").as_ptr();
            return 0;
        }
        if tailq_empty(&raw mut (*entry).values) {
            *cause = xstrdup_(c"empty").as_ptr();
            return 0;
        }
        let value = (*tailq_last(&raw mut (*entry).values)).union_.string;
        args_string_percentage(value, minval, maxval, curval, cause)
    }
}

/// Convert a string to a number which may be a percentage.
pub unsafe fn args_string_percentage(
    value: *const u8,
    minval: i64,
    maxval: i64,
    curval: i64,
    cause: *mut *mut u8,
) -> i64 {
    unsafe {
        let mut ll: i64;
        let valuelen: usize = strlen(value);
        let copy;

        if valuelen == 0 {
            *cause = xstrdup_(c"empty").as_ptr();
            return 0;
        }
        if *value.add(valuelen - 1) == b'%' as _ {
            copy = xstrdup(value).as_ptr();
            *copy.add(valuelen - 1) = b'\0' as _;

            let tmp = strtonum(copy, 0, 100);
            free_(copy);
            ll = match tmp {
                Ok(n) => n,
                Err(errstr) => {
                    *cause = xstrdup_(errstr).as_ptr();
                    return 0;
                }
            };
            ll = (curval * ll) / 100;
            if ll < minval {
                *cause = xstrdup_(c"too small").as_ptr();
                return 0;
            }
            if ll > maxval {
                *cause = xstrdup_(c"too large").as_ptr();
                return 0;
            }
        } else {
            ll = match strtonum(value, minval, maxval) {
                Ok(n) => n,
                Err(errstr) => {
                    *cause = xstrdup_(errstr).as_ptr();
                    return 0;
                }
            };
        }

        *cause = null_mut();
        ll
    }
}

/// Convert an argument to a number which may be a percentage, and expand formats.
pub unsafe fn args_percentage_and_expand(
    args: *mut args,
    flag: u8,
    minval: i64,
    maxval: i64,
    curval: i64,
    item: *mut cmdq_item,
    cause: *mut *mut u8,
) -> i64 {
    unsafe {
        let entry = args_find(args, flag);
        if entry.is_null() {
            *cause = xstrdup_(c"missing").as_ptr();
            return 0;
        }
        if tailq_empty(&raw mut (*entry).values) {
            *cause = xstrdup_(c"empty").as_ptr();
            return 0;
        }
        let value = (*tailq_last(&raw mut (*entry).values)).union_.string;
        args_string_percentage_and_expand(value, minval, maxval, curval, item, cause)
    }
}

/// Convert a string to a number which may be a percentage, and expand formats.
pub unsafe fn args_string_percentage_and_expand(
    value: *mut u8,
    minval: i64,
    maxval: i64,
    curval: i64,
    item: *mut cmdq_item,
    cause: *mut *mut u8,
) -> i64 {
    unsafe {
        let valuelen = strlen(value);
        let mut ll: i64;
        let f: *mut u8;

        if *value.add(valuelen - 1) == b'%' as _ {
            let copy = xstrdup(value).as_ptr();
            *copy.add(valuelen - 1) = b'\0';

            f = format_single_from_target(item, copy);
            let tmp = strtonum(f, 0, 100);
            free_(f);
            free_(copy);
            ll = match tmp {
                Ok(n) => n,
                Err(errstr) => {
                    *cause = xstrdup_(errstr).as_ptr();
                    return 0;
                }
            };
            ll = (curval * ll) / 100;
            if ll < minval {
                *cause = xstrdup_(c"too small").as_ptr();
                return 0;
            }
            if ll > maxval {
                *cause = xstrdup_(c"too large").as_ptr();
                return 0;
            }
        } else {
            f = format_single_from_target(item, value);
            let tmp = strtonum(f, minval, maxval);
            free_(f);
            ll = match tmp {
                Ok(n) => n,
                Err(errstr) => {
                    *cause = xstrdup_(errstr).as_ptr();
                    return 0;
                }
            };
        }

        *cause = null_mut();
        ll
    }
}

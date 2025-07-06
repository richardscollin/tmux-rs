// Copyright (c) 2013 Nicholas Marriott <nicholas.marriott@gmail.com>
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

use crate::cfg_::cfg_add_cause;
use crate::compat::queue::{
    tailq_empty, tailq_first, tailq_init, tailq_insert_after, tailq_insert_tail, tailq_last,
    tailq_next, tailq_remove,
};
use crate::xmalloc::xcalloc1;

// #define cmdq_get_callback(cb, data) cmdq_get_callback1(#cb, cb, data)
macro_rules! cmdq_get_callback {
    ($cb:ident, $data:expr) => {
        $crate::cmd_::cmd_queue::cmdq_get_callback1(stringify!($cb), Some($cb), $data)
    };
}
pub(crate) use cmdq_get_callback;
use crate::libc::{getpwuid, getuid, toupper};

/* Command queue flags. */
pub const CMDQ_FIRED: i32 = 0x1;
pub const CMDQ_WAITING: i32 = 0x2;

/* Command queue item type. */
#[repr(i32)]
#[derive(Copy, Clone)]
pub enum cmdq_type {
    CMDQ_COMMAND,
    CMDQ_CALLBACK,
}

// #[derive(crate::compat::TailQEntry)]
crate::compat::impl_tailq_entry!(cmdq_item, entry, tailq_entry<cmdq_item>);
#[repr(C)]
pub struct cmdq_item {
    pub name: *mut u8,
    pub queue: *mut cmdq_list,
    pub next: *mut cmdq_item,

    pub client: *mut client,
    pub target_client: *mut client,

    pub type_: cmdq_type,
    pub group: u32,

    pub number: u32,
    pub time: time_t,

    pub flags: i32,

    pub state: *mut cmdq_state,
    pub source: cmd_find_state,
    pub target: cmd_find_state,

    pub cmdlist: *mut cmd_list,
    pub cmd: *mut cmd,

    pub cb: cmdq_cb,
    pub data: *mut c_void,

    // #[entry]
    pub entry: tailq_entry<cmdq_item>,
}

pub type cmdq_item_list = tailq_head<cmdq_item>;

#[repr(C)]
pub struct cmdq_state {
    pub references: i32,
    pub flags: cmdq_state_flags,

    pub formats: *mut format_tree,

    pub event: key_event,
    pub current: cmd_find_state,
}

#[repr(C)]
pub struct cmdq_list {
    pub item: *mut cmdq_item,
    pub list: cmdq_item_list,
}

pub unsafe fn cmdq_name(c: *const client) -> *const u8 {
    static mut buf: [u8; 256] = [0; 256];
    let s = &raw mut buf as *mut u8;

    if c.is_null() {
        return c!("<global>");
    }

    unsafe {
        if !(*c).name.is_null() {
            xsnprintf_!(s, 256, "<{}>", _s((*c).name));
        } else {
            xsnprintf_!(s, 256, "<{:p}>", c);
        }
    }

    s
}

pub unsafe fn cmdq_get(c: *mut client) -> *mut cmdq_list {
    static mut global_queue: *mut cmdq_list = null_mut();

    unsafe {
        if c.is_null() {
            if global_queue.is_null() {
                global_queue = cmdq_new().as_ptr();
            }
            return global_queue;
        }

        (*c).queue
    }
}

pub unsafe fn cmdq_new() -> NonNull<cmdq_list> {
    unsafe {
        let queue = NonNull::from(xcalloc1::<cmdq_list>());
        tailq_init(&raw mut (*queue.as_ptr()).list);
        queue
    }
}

pub unsafe fn cmdq_free(queue: *mut cmdq_list) {
    unsafe {
        if !tailq_empty(&raw mut (*queue).list) {
            fatalx("queue not empty");
        }
        free_(queue);
    }
}

pub unsafe fn cmdq_get_name(item: *mut cmdq_item) -> *mut u8 {
    unsafe { (*item).name }
}

pub unsafe fn cmdq_get_client(item: *mut cmdq_item) -> *mut client {
    unsafe { (*item).client }
}

pub unsafe fn cmdq_get_target_client(item: *mut cmdq_item) -> *mut client {
    unsafe { (*item).target_client }
}

pub unsafe fn cmdq_get_state(item: *mut cmdq_item) -> *mut cmdq_state {
    unsafe { (*item).state }
}

pub unsafe fn cmdq_get_target(item: *mut cmdq_item) -> *mut cmd_find_state {
    unsafe { &raw mut (*item).target }
}

pub unsafe fn cmdq_get_source(item: *mut cmdq_item) -> *mut cmd_find_state {
    unsafe { &raw mut (*item).source }
}

pub unsafe fn cmdq_get_event(item: *mut cmdq_item) -> *mut key_event {
    unsafe { &raw mut (*(*item).state).event }
}

pub unsafe fn cmdq_get_current(item: *mut cmdq_item) -> *mut cmd_find_state {
    unsafe { &raw mut (*(*item).state).current }
}

pub unsafe fn cmdq_get_flags(item: *mut cmdq_item) -> cmdq_state_flags {
    unsafe { (*(*item).state).flags }
}

pub unsafe fn cmdq_new_state(
    current: *mut cmd_find_state,
    event: *mut key_event,
    flags: cmdq_state_flags,
) -> *mut cmdq_state {
    unsafe {
        let state: *mut cmdq_state = xcalloc1::<cmdq_state>();
        (*state).references = 1;
        (*state).flags = flags;

        if !event.is_null() {
            memcpy__(&raw mut (*state).event, event);
        } else {
            (*state).event.key = KEYC_NONE;
        }
        if !current.is_null() && cmd_find_valid_state(current) {
            cmd_find_copy_state(&raw mut (*state).current, current);
        } else {
            cmd_find_clear_state(&raw mut (*state).current, 0);
        }

        state
    }
}

pub unsafe fn cmdq_link_state(state: *mut cmdq_state) -> *mut cmdq_state {
    unsafe {
        (*state).references += 1;
    }
    state
}

pub unsafe fn cmdq_copy_state(
    state: *mut cmdq_state,
    current: *mut cmd_find_state,
) -> *mut cmdq_state {
    unsafe {
        if !current.is_null() {
            return cmdq_new_state(current, &raw mut (*state).event, (*state).flags);
        }

        cmdq_new_state(
            &raw mut (*state).current,
            &raw mut (*state).event,
            (*state).flags,
        )
    }
}

pub unsafe fn cmdq_free_state(state: *mut cmdq_state) {
    unsafe {
        (*state).references -= 1;
        if (*state).references != 0 {
            return;
        }

        if !(*state).formats.is_null() {
            format_free((*state).formats);
        }
        free_(state);
    }
}

macro_rules! cmdq_add_format {
   ($state:expr, $key:expr, $fmt:literal $(, $args:expr)* $(,)?) => {
        crate::cmd_::cmd_queue::cmdq_add_format_($state, $key, format_args!($fmt $(, $args)*))
    };
}
pub(crate) use cmdq_add_format;

pub unsafe fn cmdq_add_format_(state: *mut cmdq_state, key: *const u8, args: std::fmt::Arguments) {
    unsafe {
        let value = args.to_string();

        if (*state).formats.is_null() {
            (*state).formats =
                format_create(null_mut(), null_mut(), FORMAT_NONE, format_flags::empty());
        }
        format_add!((*state).formats, key, "{}", value);
    }
}

pub unsafe fn cmdq_add_formats(state: *mut cmdq_state, ft: *mut format_tree) {
    unsafe {
        if (*state).formats.is_null() {
            (*state).formats =
                format_create(null_mut(), null_mut(), FORMAT_NONE, format_flags::empty());
        }
        format_merge((*state).formats, ft);
    }
}

pub unsafe fn cmdq_merge_formats(item: *mut cmdq_item, ft: *mut format_tree) {
    unsafe {
        if !(*item).cmd.is_null() {
            let entry = cmd_get_entry((*item).cmd);
            format_add!(ft, c!("command"), "{}", _s(entry.name.as_ptr()));
        }

        if !(*(*item).state).formats.is_null() {
            format_merge(ft, (*(*item).state).formats);
        }
    }
}

pub unsafe fn cmdq_append(c: *mut client, mut item: *mut cmdq_item) -> *mut cmdq_item {
    let __func__ = "cmdq_append";

    unsafe {
        let queue = cmdq_get(c);
        let mut next = null_mut();

        loop {
            next = (*item).next;
            (*item).next = null_mut();

            if !c.is_null() {
                (*c).references += 1;
            }
            (*item).client = c;

            (*item).queue = queue;
            tailq_insert_tail::<_, ()>(&raw mut (*queue).list, item);
            log_debug!("{} {}: {}", __func__, _s(cmdq_name(c)), _s((*item).name));

            item = next;
            if item.is_null() {
                break;
            }
        }
        tailq_last(&raw mut (*queue).list)
    }
}

// TODO crashes with this one

pub unsafe fn cmdq_insert_after(
    mut after: *mut cmdq_item,
    mut item: *mut cmdq_item,
) -> *mut cmdq_item {
    unsafe {
        let c = (*after).client;
        let queue = (*after).queue;

        loop {
            let next = (*item).next;
            (*item).next = (*after).next;
            (*after).next = item;

            if !c.is_null() {
                (*c).references += 1;
            }
            (*item).client = c;

            (*item).queue = queue;
            tailq_insert_after(&raw mut (*queue).list, after, item);
            log_debug!(
                "{} {}: {} after {}",
                "cmdq_insert_after",
                _s(cmdq_name(c)),
                _s((*item).name),
                _s((*after).name),
            );

            after = item;
            item = next;
            if item.is_null() {
                break;
            }
        }
        after
    }
}

macro_rules! cmdq_insert_hook {
   ($s:expr,$item:expr,$current:expr, $fmt:literal $(, $args:expr)* $(,)?) => {
        crate::cmd_::cmd_queue::cmdq_insert_hook_($s, $item, $current, format_args!($fmt $(, $args)*))
    };
}
pub(crate) use cmdq_insert_hook;

pub unsafe fn cmdq_insert_hook_(
    s: *mut session,
    mut item: *mut cmdq_item,
    current: *mut cmd_find_state,
    format_args: std::fmt::Arguments,
) {
    unsafe {
        let state = (*item).state;
        let cmd = (*item).cmd;
        let args = cmd_get_args(cmd);
        let mut ae: *mut args_entry = null_mut();
        let mut flag: c_uchar = 0;
        const sizeof_tmp: usize = 32;
        let mut buf: [u8; 32] = zeroed();
        let tmp = &raw mut buf as *mut u8;

        if (*(*item).state)
            .flags
            .intersects(cmdq_state_flags::CMDQ_STATE_NOHOOKS)
        {
            return;
        }
        let oo = if s.is_null() {
            global_s_options
        } else {
            (*s).options
        };

        let mut name: String = format_args.to_string();
        name.push('\0');

        let o = options_get(oo, name.as_ptr().cast());
        if o.is_null() {
            return;
        }
        log_debug!("running hook {} (parent {:p})", name, item);

        /*
         * The hooks get a new state because they should not update the current
         * target or formats for any subsequent commands.
         */
        let new_state = cmdq_new_state(
            current,
            &raw mut (*state).event,
            cmdq_state_flags::CMDQ_STATE_NOHOOKS,
        );
        cmdq_add_format!(new_state, c!("hook"), "{}", name);

        let arguments = args_print(args);
        cmdq_add_format!(new_state, c!("hook_arguments"), "{}", _s(arguments),);
        free_(arguments);

        for i in 0..args_count(args) {
            xsnprintf_!(tmp, sizeof_tmp, "hook_argument_{}", i);
            cmdq_add_format!(new_state, tmp, "{}", _s(args_string(args, i)));
        }
        flag = args_first(args, &raw mut ae);
        while flag != 0 {
            let value = args_get(args, flag);
            if value.is_null() {
                xsnprintf_!(tmp, sizeof_tmp, "hook_flag_{}", flag as char);
                cmdq_add_format!(new_state, tmp, "1");
            } else {
                xsnprintf_!(tmp, sizeof_tmp, "hook_flag_{}", flag as char);
                cmdq_add_format!(new_state, tmp, "{}", _s(value));
            }

            let mut i = 0;
            let mut av = args_first_value(args, flag);
            while !av.is_null() {
                xsnprintf_!(tmp, sizeof_tmp, "hook_flag_{}_{}", flag as char, i);
                cmdq_add_format!(new_state, tmp, "{}", _s((*av).union_.string));
                i += 1;
                av = args_next_value(av);
            }

            flag = args_next(&raw mut ae);
        }

        let mut a = options_array_first(o);
        while !a.is_null() {
            let cmdlist = (*options_array_item_value(a)).cmdlist;
            if !cmdlist.is_null() {
                let new_item = cmdq_get_command(cmdlist, new_state);
                if !item.is_null() {
                    item = cmdq_insert_after(item, new_item);
                } else {
                    item = cmdq_append(null_mut(), new_item);
                }
            }
            a = options_array_next(a);
        }

        cmdq_free_state(new_state);
    }
}

pub unsafe fn cmdq_continue(item: *mut cmdq_item) {
    unsafe {
        (*item).flags &= !CMDQ_WAITING;
    }
}

pub unsafe fn cmdq_remove(item: *mut cmdq_item) {
    unsafe {
        if !(*item).client.is_null() {
            server_client_unref((*item).client);
        }
        if !(*item).cmdlist.is_null() {
            cmd_list_free((*item).cmdlist);
        }
        cmdq_free_state((*item).state);

        tailq_remove(&raw mut (*(*item).queue).list, item);

        free_((*item).name);
        free_(item);
    }
}

pub unsafe fn cmdq_remove_group(item: *mut cmdq_item) {
    unsafe {
        if (*item).group == 0 {
            return;
        }
        let mut this = tailq_next(item);
        while !this.is_null() {
            let next = tailq_next(this);
            if (*this).group == (*item).group {
                cmdq_remove(this);
            }
            this = next;
        }
    }
}

pub unsafe fn cmdq_empty_command(_item: *mut cmdq_item, _data: *mut c_void) -> cmd_retval {
    cmd_retval::CMD_RETURN_NORMAL
}

pub unsafe fn cmdq_get_command(
    cmdlist: *mut cmd_list,
    mut state: *mut cmdq_state,
) -> *mut cmdq_item {
    unsafe {
        let mut first: *mut cmdq_item = null_mut();
        let mut last: *mut cmdq_item = null_mut();
        let mut created = false;

        let mut cmd = cmd_list_first(cmdlist);
        if cmd.is_null() {
            return cmdq_get_callback!(cmdq_empty_command, null_mut()).as_ptr();
        }

        if state.is_null() {
            state = cmdq_new_state(null_mut(), null_mut(), cmdq_state_flags::empty());
            created = true;
        }

        while !cmd.is_null() {
            let entry = cmd_get_entry(cmd);

            let item = xcalloc1::<cmdq_item>() as *mut cmdq_item;
            (*item).name = format_nul!("[{}/{:p}]", _s(entry.name.as_ptr()), item,);
            (*item).type_ = cmdq_type::CMDQ_COMMAND;

            (*item).group = cmd_get_group(cmd);
            (*item).state = cmdq_link_state(state);

            (*item).cmdlist = cmdlist;
            (*item).cmd = cmd;

            (*cmdlist).references += 1;
            // log_debug_!("cmdq_get_command: {} group {}", PercentS((*item).name), (*item).group,);

            if first.is_null() {
                first = item;
            }
            if !last.is_null() {
                (*last).next = item;
            }
            last = item;

            cmd = cmd_list_next(cmd);
        }

        if created {
            cmdq_free_state(state);
        }
        first
    }
}

pub unsafe fn cmdq_find_flag(
    item: *mut cmdq_item,
    fs: *mut cmd_find_state,
    flag: *const cmd_entry_flag,
) -> cmd_retval {
    unsafe {
        if (*flag).flag == 0 {
            cmd_find_from_client(fs, (*item).target_client, 0);
            return cmd_retval::CMD_RETURN_NORMAL;
        }

        let value = args_get(cmd_get_args((*item).cmd), (*flag).flag as u8);
        if cmd_find_target(fs, item, value, (*flag).type_, (*flag).flags) != 0 {
            cmd_find_clear_state(fs, 0);
            return cmd_retval::CMD_RETURN_ERROR;
        }

        cmd_retval::CMD_RETURN_NORMAL
    }
}

pub unsafe fn cmdq_add_message(item: *mut cmdq_item) {
    unsafe {
        let c = (*item).client;
        let state = (*item).state;
        let mut user = null_mut();

        let tmp = cmd_print((*item).cmd);
        if !c.is_null() {
            let uid = proc_get_peer_uid((*c).peer);
            if uid != -1i32 as uid_t && uid != getuid() {
                let pw = getpwuid(uid);
                if !pw.is_null() {
                    user = format_nul!("[{}]", _s((*pw).pw_name));
                } else {
                    user = xstrdup(c!("[unknown]")).as_ptr();
                }
            } else {
                user = xstrdup(c!("")).as_ptr();
            }
            if !(*c).session.is_null() && (*state).event.key != KEYC_NONE {
                let key = key_string_lookup_key((*state).event.key, 0);
                server_add_message!("{}{} key {}: {}", _s((*c).name), _s(user), _s(key), _s(tmp));
            } else {
                server_add_message!("{}{} command: {}", _s((*c).name), _s(user), _s(tmp));
            }
            free_(user);
        } else {
            server_add_message!("command: {}", _s(tmp));
        }
        free_(tmp);
    }
}

pub unsafe fn cmdq_fire_command(item: *mut cmdq_item) -> cmd_retval {
    let __func__ = "cmdq_fire_command";

    unsafe {
        let name = cmdq_name((*item).client);
        let state = (*item).state;
        let cmd = (*item).cmd;
        let args = cmd_get_args(cmd);
        let entry = cmd_get_entry(cmd);
        let mut tc = null_mut();
        let saved = (*item).client;
        let mut retval;
        let mut fs: cmd_find_state = zeroed();
        let mut fsp: *mut cmd_find_state = null_mut();
        let mut quiet = 0;
        let mut flags = false;

        'out: {
            if cfg_finished != 0 {
                cmdq_add_message(item);
            }
            if log_get_level() > 1 {
                let tmp = cmd_print(cmd);
                log_debug!("{} {}: ({}) {}", __func__, _s(name), (*item).group, _s(tmp));
                free_(tmp);
            }

            flags = (*state)
                .flags
                .intersects(cmdq_state_flags::CMDQ_STATE_CONTROL);
            cmdq_guard(item, c!("begin"), flags);

            if (*item).client.is_null() {
                (*item).client = cmd_find_client(item, null_mut(), 1);
            }

            if entry.flags.intersects(cmd_flag::CMD_CLIENT_CANFAIL) {
                quiet = 1;
            }
            if entry.flags.intersects(cmd_flag::CMD_CLIENT_CFLAG) {
                tc = cmd_find_client(item, args_get_(args, 'c'), quiet);
                if tc.is_null() && quiet == 0 {
                    retval = cmd_retval::CMD_RETURN_ERROR;
                    break 'out;
                }
            } else if entry.flags.intersects(cmd_flag::CMD_CLIENT_TFLAG) {
                tc = cmd_find_client(item, args_get_(args, 't'), quiet);
                if tc.is_null() && quiet == 0 {
                    retval = cmd_retval::CMD_RETURN_ERROR;
                    break 'out;
                }
            } else {
                tc = cmd_find_client(item, null_mut(), 1);
            }
            (*item).target_client = tc;

            retval = cmdq_find_flag(item, &raw mut (*item).source, &entry.source);
            if retval == cmd_retval::CMD_RETURN_ERROR {
                break 'out;
            }
            retval = cmdq_find_flag(item, &raw mut (*item).target, &entry.target);
            if retval == cmd_retval::CMD_RETURN_ERROR {
                break 'out;
            }

            // log_debug_!("entry_name: {}", PercentS((*entry).name));

            retval = (entry.exec)(cmd, item);
            if retval == cmd_retval::CMD_RETURN_ERROR {
                break 'out;
            }

            if entry.flags.intersects(cmd_flag::CMD_AFTERHOOK) {
                fsp = if cmd_find_valid_state(&raw mut (*item).target) {
                    &raw mut (*item).target
                } else if cmd_find_valid_state(&raw mut (*(*item).state).current) {
                    &raw mut (*(*item).state).current
                } else if cmd_find_from_client(&raw mut fs, (*item).client, 0) == 0 {
                    &raw mut fs
                } else {
                    break 'out;
                };
                cmdq_insert_hook!((*fsp).s, item, fsp, "after-{}", _s(entry.name.as_ptr()));
            }
        }

        (*item).client = saved;
        if retval == cmd_retval::CMD_RETURN_ERROR {
            fsp = null_mut();
            if cmd_find_valid_state(&raw mut (*item).target) {
                fsp = &raw mut (*item).target;
            } else if cmd_find_valid_state(&raw mut (*(*item).state).current) {
                fsp = &raw mut (*(*item).state).current;
            } else if cmd_find_from_client(&raw mut fs, (*item).client, 0) == 0 {
                fsp = &raw mut fs;
            }
            cmdq_insert_hook!(
                if !fsp.is_null() { (*fsp).s } else { null_mut() },
                item,
                fsp,
                "command-error"
            );
            cmdq_guard(item, c!("error"), flags);
        } else {
            cmdq_guard(item, c!("end"), flags);
        }
        retval
    }
}

pub unsafe fn cmdq_get_callback1(name: &str, cb: cmdq_cb, data: *mut u8) -> NonNull<cmdq_item> {
    let item = xcalloc_::<cmdq_item>(1).as_ptr();

    unsafe {
        (*item).name = format_nul!("[{}/{:p}]", name, item);
        (*item).type_ = cmdq_type::CMDQ_CALLBACK;

        (*item).group = 0;
        (*item).state = cmdq_new_state(null_mut(), null_mut(), cmdq_state_flags::empty());

        (*item).cb = cb;
        (*item).data = data as _;

        NonNull::new_unchecked(item)
    }
}

pub unsafe fn cmdq_error_callback(item: *mut cmdq_item, data: *mut c_void) -> cmd_retval {
    let error = data as *mut u8;

    unsafe {
        cmdq_error!(item, "{}", _s(error));
        free_(error);
    }

    cmd_retval::CMD_RETURN_NORMAL
}

pub unsafe fn cmdq_get_error(error: *const u8) -> NonNull<cmdq_item> {
    unsafe { cmdq_get_callback!(cmdq_error_callback, xstrdup(error).as_ptr()) }
}

pub unsafe fn cmdq_fire_callback(item: *mut cmdq_item) -> cmd_retval {
    unsafe { ((*item).cb.unwrap())(item, (*item).data) }
}

pub unsafe fn cmdq_next(c: *mut client) -> u32 {
    let __func__ = "cmdq_next";
    static mut number: u32 = 0;
    let mut items = 0;
    let mut retval: cmd_retval = cmd_retval::CMD_RETURN_NORMAL;

    unsafe {
        let queue = cmdq_get(c);
        let name = cmdq_name(c);

        'waiting: {
            if tailq_empty(&raw mut (*queue).list) {
                log_debug!("{} {}: empty", __func__, _s(name));
                return 0;
            }
            if (*tailq_first(&raw mut (*queue).list)).flags & CMDQ_WAITING != 0 {
                log_debug!("{} {}: waiting", __func__, _s(name));
                return 0;
            }

            log_debug!("{} {}: enter", __func__, _s(name));
            loop {
                (*queue).item = tailq_first(&raw mut (*queue).list);
                let item = (*queue).item;
                if item.is_null() {
                    break;
                }
                log_debug!(
                    "{} {}: {} ({}), flags {}",
                    __func__,
                    _s(name),
                    _s((*item).name),
                    (*item).type_ as i32,
                    (*item).flags
                );

                if (*item).flags & CMDQ_WAITING != 0 {
                    break 'waiting;
                }

                if !(*item).flags & CMDQ_FIRED != 0 {
                    (*item).time = libc::time(null_mut());
                    number += 1;
                    (*item).number = number;

                    match (*item).type_ {
                        cmdq_type::CMDQ_COMMAND => {
                            retval = cmdq_fire_command(item);

                            if retval == cmd_retval::CMD_RETURN_ERROR {
                                cmdq_remove_group(item);
                            }
                        }
                        cmdq_type::CMDQ_CALLBACK => retval = cmdq_fire_callback(item),
                        _ => retval = cmd_retval::CMD_RETURN_ERROR,
                    }
                    (*item).flags |= CMDQ_FIRED;

                    if retval == cmd_retval::CMD_RETURN_WAIT {
                        (*item).flags |= CMDQ_WAITING;
                        break 'waiting;
                    }
                    items += 1;
                }
                cmdq_remove(item);
            }
            (*queue).item = null_mut();

            log_debug!("{} {}: exit (empty)", __func__, _s(name));
            return items;
        } // 'waiting
        //waiting:
        log_debug!("{} {}: exit (wait)", __func__, _s(name));
        items
    }
}

pub unsafe fn cmdq_running(c: *mut client) -> *mut cmdq_item {
    unsafe {
        let queue = cmdq_get(c);

        if (*queue).item.is_null() {
            return null_mut();
        }
        if (*(*queue).item).flags & CMDQ_WAITING != 0 {
            return null_mut();
        }
        (*queue).item
    }
}

pub unsafe fn cmdq_guard(item: *mut cmdq_item, guard: *const u8, flags: bool) {
    unsafe {
        let c = (*item).client;
        let t = (*item).time;
        let number = (*item).number;

        if !c.is_null() && (*c).flags.intersects(client_flag::CONTROL) {
            control_write!(c, "%{} {} {} {}", _s(guard), t, number, flags as i32);
        }
    }
}

pub unsafe fn cmdq_print_data(item: *mut cmdq_item, parse: i32, evb: *mut evbuffer) {
    unsafe {
        server_client_print((*item).client, parse, evb);
    }
}

macro_rules! cmdq_print {
   ($item:expr, $fmt:literal $(, $args:expr)* $(,)?) => {
        crate::cmd_::cmd_queue::cmdq_print_($item, format_args!($fmt $(, $args)*))
    };
}
pub(crate) use cmdq_print;
pub unsafe fn cmdq_print_(item: *mut cmdq_item, args: std::fmt::Arguments) {
    unsafe {
        let evb = evbuffer_new();
        if evb.is_null() {
            fatalx("out of memory");
        }

        evbuffer_add_vprintf(evb, args);

        cmdq_print_data(item, 0, evb);
        evbuffer_free(evb);
    }
}

macro_rules! cmdq_error {
   ($item:expr, $fmt:literal $(, $args:expr)* $(,)?) => {
        crate::cmd_::cmd_queue::cmdq_error_($item, format_args!($fmt $(, $args)*))
    };
}
pub(crate) use cmdq_error;
pub unsafe fn cmdq_error_(item: *mut cmdq_item, args: std::fmt::Arguments) {
    unsafe {
        let c = (*item).client;
        let cmd = (*item).cmd;
        let mut tmp = null_mut();
        let mut file = null();
        let mut line = AtomicU32::new(0);

        let mut msg = args.to_string();
        msg.push('\0');
        let mut msg = msg.leak().as_mut_ptr().cast();

        log_debug!("cmdq_error: {}", _s(msg));

        if c.is_null() {
            cmd_get_source(cmd, &raw mut file, &line);
            cfg_add_cause!("{}:{}: {}", _s(file), line.into_inner(), _s(msg));
        } else if (*c).session.is_null() || (*c).flags.intersects(client_flag::CONTROL) {
            server_add_message!("{} message: {}", _s((*c).name), _s(msg));
            if !(*c).flags.intersects(client_flag::UTF8) {
                tmp = msg;
                msg = utf8_sanitize(tmp);
                free_(tmp);
            }
            if (*c).flags.intersects(client_flag::CONTROL) {
                control_write!(c, "{}", _s(msg));
            } else {
                file_error!(c, "{}\n", _s(msg));
            }
            (*c).retval = 1;
        } else {
            *msg = toupper((*msg) as i32) as _;
            status_message_set!(c, -1, 1, 0, "{}", _s(msg));
        }

        free_(msg);
    }
}

// Copyright (c) 2013 Nicholas Marriott <nicholas.marriott@gmail.com>
// Copyright (c) 2013 Thiago de Arruda <tpadilha84@gmail.com>
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
use std::cmp::Ordering;

use crate::compat::{
    queue::{tailq_empty, tailq_first, tailq_foreach, tailq_init, tailq_insert_tail, tailq_remove},
    tree::{rb_find, rb_foreach, rb_initializer, rb_insert, rb_remove},
};

pub static mut cmd_wait_for_entry: cmd_entry = cmd_entry {
    name: c"wait-for".as_ptr(),
    alias: c"wait".as_ptr(),

    args: args_parse::new(c"LSU", 1, 1, None),
    usage: c"[-L|-S|-U] channel".as_ptr(),

    flags: cmd_flag::empty(),
    exec: Some(cmd_wait_for_exec),
    ..unsafe { zeroed() }
};

crate::compat::impl_tailq_entry!(wait_item, entry, tailq_entry<wait_item>);
#[repr(C)]
pub struct wait_item {
    item: *mut cmdq_item,
    // #[entry]
    entry: tailq_entry<wait_item>,
}

#[repr(C)]
pub struct wait_channel {
    pub name: *mut c_char,
    pub locked: i32,
    pub woken: i32,

    pub waiters: tailq_head<wait_item>,
    pub lockers: tailq_head<wait_item>,

    pub entry: rb_entry<wait_channel>,
}

pub type wait_channels = rb_head<wait_channel>;

static mut wait_channels: wait_channels = rb_initializer();

RB_GENERATE!(
    wait_channels,
    wait_channel,
    entry,
    discr_entry,
    wait_channel_cmp
);

pub unsafe fn wait_channel_cmp(
    wc1: *const wait_channel,
    wc2: *const wait_channel,
) -> Ordering {
    unsafe { i32_to_ordering(libc::strcmp((*wc1).name, (*wc2).name)) }
}

pub unsafe fn cmd_wait_for_add(name: *const c_char) -> *mut wait_channel {
    let wc: *mut wait_channel = xmalloc_().as_ptr();
    unsafe {
        (*wc).name = xstrdup(name).as_ptr();

        (*wc).locked = 0;
        (*wc).woken = 0;

        tailq_init(&raw mut (*wc).waiters);
        tailq_init(&raw mut (*wc).lockers);

        rb_insert(&raw mut wait_channels, wc);

        log_debug!("add wait channel {}", _s((*wc).name));
    }
    wc
}

pub unsafe fn cmd_wait_for_remove(wc: *mut wait_channel) {
    unsafe {
        if (*wc).locked != 0 {
            return;
        }
        if !tailq_empty(&raw mut (*wc).waiters) || (*wc).woken == 0 {
            return;
        }

        log_debug!("remove wait channel {}", _s((*wc).name));

        rb_remove(&raw mut wait_channels, wc);

        free_((*wc).name);
        free_(wc);
    }
}

pub unsafe fn cmd_wait_for_exec(self_: *mut cmd, item: *mut cmdq_item) -> cmd_retval {
    unsafe {
        let args = cmd_get_args(self_);
        let name = args_string(args, 0);
        // struct wait_channel *wc, find;

        let mut find: wait_channel = zeroed();
        find.name = name as *mut c_char; // TODO casting away const
        let wc = rb_find(&raw mut wait_channels, &raw mut find);

        if args_has_(args, 'S') {
            return cmd_wait_for_signal(item, name, wc);
        }
        if args_has_(args, 'L') {
            return cmd_wait_for_lock(item, name, wc);
        }
        if args_has_(args, 'U') {
            return cmd_wait_for_unlock(item, name, wc);
        }

        cmd_wait_for_wait(item, name, wc)
    }
}

pub unsafe fn cmd_wait_for_signal(
    _item: *mut cmdq_item,
    name: *const c_char,
    mut wc: *mut wait_channel,
) -> cmd_retval {
    unsafe {
        if wc.is_null() {
            wc = cmd_wait_for_add(name);
        }

        if tailq_empty(&raw mut (*wc).waiters) && (*wc).woken == 0 {
            log_debug!("signal wait channel {}, no waiters", _s((*wc).name));
            (*wc).woken = 1;
            return cmd_retval::CMD_RETURN_NORMAL;
        }
        log_debug!("signal wait channel {}, with waiters", _s((*wc).name));

        for wi in tailq_foreach::<_, ()>(&raw mut (*wc).waiters).map(NonNull::as_ptr) {
            cmdq_continue((*wi).item);

            tailq_remove::<_, ()>(&raw mut (*wc).waiters, wi);
            free_(wi);
        }

        cmd_wait_for_remove(wc);

        cmd_retval::CMD_RETURN_NORMAL
    }
}

pub unsafe fn cmd_wait_for_wait(
    item: *mut cmdq_item,
    name: *const c_char,
    mut wc: *mut wait_channel,
) -> cmd_retval {
    unsafe {
        let c = cmdq_get_client(item);

        if c.is_null() {
            cmdq_error!(item, "not able to wait");
            return cmd_retval::CMD_RETURN_ERROR;
        }

        if wc.is_null() {
            wc = cmd_wait_for_add(name);
        }

        if (*wc).woken != 0 {
            log_debug!("wait channel {} already woken ({:p})", _s((*wc).name), c);
            cmd_wait_for_remove(wc);
            return cmd_retval::CMD_RETURN_NORMAL;
        }
        log_debug!("wait channel {} not woken ({:p})", _s((*wc).name), c);

        let wi: *mut wait_item = xcalloc1();
        (*wi).item = item;
        tailq_insert_tail(&raw mut (*wc).waiters, wi);
    }
    cmd_retval::CMD_RETURN_WAIT
}

pub unsafe fn cmd_wait_for_lock(
    item: *mut cmdq_item,
    name: *const c_char,
    mut wc: *mut wait_channel,
) -> cmd_retval {
    unsafe {
        if cmdq_get_client(item).is_null() {
            cmdq_error!(item, "not able to lock");
            return cmd_retval::CMD_RETURN_ERROR;
        }

        if wc.is_null() {
            wc = cmd_wait_for_add(name);
        }

        if (*wc).locked != 0 {
            let wi = xcalloc1::<wait_item>();
            wi.item = item;
            tailq_insert_tail(&raw mut (*wc).lockers, wi);
            return cmd_retval::CMD_RETURN_WAIT;
        }
        (*wc).locked = 1;
    }
    cmd_retval::CMD_RETURN_NORMAL
}

pub unsafe fn cmd_wait_for_unlock(
    item: *mut cmdq_item,
    name: *const c_char,
    wc: *mut wait_channel,
) -> cmd_retval {
    unsafe {
        if wc.is_null() || (*wc).locked == 0 {
            cmdq_error!(item, "channel {} not locked", _s(name));
            return cmd_retval::CMD_RETURN_ERROR;
        }

        let wi = tailq_first(&raw mut (*wc).lockers);
        if !wi.is_null() {
            cmdq_continue((*wi).item);
            tailq_remove(&raw mut (*wc).lockers, wi);
            free_(wi);
        } else {
            (*wc).locked = 0;
            cmd_wait_for_remove(wc);
        }
    }
    cmd_retval::CMD_RETURN_NORMAL
}

pub unsafe fn cmd_wait_for_flush() {
    unsafe {
        for wc in rb_foreach(&raw mut wait_channels).map(NonNull::as_ptr) {
            for wi in tailq_foreach(&raw mut (*wc).waiters).map(NonNull::as_ptr) {
                cmdq_continue((*wi).item);
                tailq_remove(&raw mut (*wc).waiters, wi);
                free_(wi);
            }
            (*wc).woken = 1;
            for wi in tailq_foreach(&raw mut (*wc).lockers).map(NonNull::as_ptr) {
                cmdq_continue((*wi).item);
                tailq_remove(&raw mut (*wc).lockers, wi);
                free_(wi);
            }
            (*wc).locked = 0;
            cmd_wait_for_remove(wc);
        }
    }
}

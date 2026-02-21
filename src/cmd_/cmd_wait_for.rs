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
use std::cell::RefCell;
use std::collections::BTreeMap;

use crate::compat::queue::{
    tailq_empty, tailq_first, tailq_foreach, tailq_init, tailq_insert_tail, tailq_remove,
};
use crate::*;

pub static CMD_WAIT_FOR_ENTRY: cmd_entry = cmd_entry {
    name: "wait-for",
    alias: Some("wait"),

    args: args_parse::new("LSU", 1, 1, None),
    usage: "[-L|-S|-U] channel",

    flags: cmd_flag::empty(),
    exec: cmd_wait_for_exec,
    source: cmd_entry_flag::zeroed(),
    target: cmd_entry_flag::zeroed(),
};

impl_tailq_entry!(wait_item, entry, tailq_entry<wait_item>);
#[repr(C)]
pub struct wait_item {
    item: *mut cmdq_item,
    // #[entry]
    entry: tailq_entry<wait_item>,
}

#[repr(C)]
pub struct wait_channel {
    pub name: *mut u8,
    pub locked: bool,
    pub woken: bool,

    pub waiters: tailq_head<wait_item>,
    pub lockers: tailq_head<wait_item>,
}

thread_local! {
    static WAIT_CHANNELS: RefCell<BTreeMap<Box<str>, Box<wait_channel>>> = const { RefCell::new(BTreeMap::new()) };
}

pub unsafe fn cmd_wait_for_add(name: *const u8) -> *mut wait_channel {
    unsafe {
        let key: Box<str> = cstr_to_str(name).into();
        let mut wc = Box::new(wait_channel {
            name: xstrdup(name).as_ptr(),
            locked: false,
            woken: false,
            waiters: zeroed(),
            lockers: zeroed(),
        });

        tailq_init(&raw mut wc.waiters);
        tailq_init(&raw mut wc.lockers);

        log_debug!("add wait channel {}", _s(wc.name));

        WAIT_CHANNELS.with_borrow_mut(|channels| {
            let entry = channels.entry(key).or_insert(wc);
            &mut **entry as *mut wait_channel
        })
    }
}

pub unsafe fn cmd_wait_for_remove(wc: *mut wait_channel) {
    unsafe {
        if (*wc).locked {
            return;
        }
        if !tailq_empty(&raw mut (*wc).waiters) || !(*wc).woken {
            return;
        }

        log_debug!("remove wait channel {}", _s((*wc).name));

        let key: Box<str> = cstr_to_str((*wc).name).into();
        WAIT_CHANNELS.with_borrow_mut(|channels| {
            if let Some(removed) = channels.remove(key.as_ref()) {
                free_(removed.name);
            }
        });
    }
}

pub unsafe fn cmd_wait_for_exec(self_: *mut cmd, item: *mut cmdq_item) -> cmd_retval {
    unsafe {
        let args = cmd_get_args(self_);
        let name = args_string(args, 0);

        let key = cstr_to_str(name);
        let wc = WAIT_CHANNELS.with_borrow_mut(|channels| match channels.get_mut(key) {
            Some(wc) => &mut **wc as *mut wait_channel,
            None => null_mut(),
        });

        if args_has(args, 'S') {
            return cmd_wait_for_signal(item, name, wc);
        }
        if args_has(args, 'L') {
            return cmd_wait_for_lock(item, name, wc);
        }
        if args_has(args, 'U') {
            return cmd_wait_for_unlock(item, name, wc);
        }

        cmd_wait_for_wait(item, name, wc)
    }
}

pub unsafe fn cmd_wait_for_signal(
    _item: *const cmdq_item,
    name: *const u8,
    mut wc: *mut wait_channel,
) -> cmd_retval {
    unsafe {
        if wc.is_null() {
            wc = cmd_wait_for_add(name);
        }

        if tailq_empty(&raw mut (*wc).waiters) && !(*wc).woken {
            log_debug!("signal wait channel {}, no waiters", _s((*wc).name));
            (*wc).woken = true;
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
    name: *const u8,
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

        if (*wc).woken {
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
    name: *const u8,
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

        if (*wc).locked {
            let wi = xcalloc1::<wait_item>();
            wi.item = item;
            tailq_insert_tail(&raw mut (*wc).lockers, wi);
            return cmd_retval::CMD_RETURN_WAIT;
        }
        (*wc).locked = true;
    }
    cmd_retval::CMD_RETURN_NORMAL
}

pub unsafe fn cmd_wait_for_unlock(
    item: *mut cmdq_item,
    name: *const u8,
    wc: *mut wait_channel,
) -> cmd_retval {
    unsafe {
        if wc.is_null() || !(*wc).locked {
            cmdq_error!(item, "channel {} not locked", _s(name));
            return cmd_retval::CMD_RETURN_ERROR;
        }

        let wi = tailq_first(&raw mut (*wc).lockers);
        if !wi.is_null() {
            cmdq_continue((*wi).item);
            tailq_remove(&raw mut (*wc).lockers, wi);
            free_(wi);
        } else {
            (*wc).locked = false;
            cmd_wait_for_remove(wc);
        }
    }
    cmd_retval::CMD_RETURN_NORMAL
}

pub unsafe fn cmd_wait_for_flush() {
    unsafe {
        let keys: Vec<Box<str>> =
            WAIT_CHANNELS.with_borrow(|channels| channels.keys().cloned().collect());
        for key in keys {
            let wc = WAIT_CHANNELS.with_borrow_mut(|channels| match channels.get_mut(key.as_ref())
            {
                Some(wc) => &mut **wc as *mut wait_channel,
                None => null_mut(),
            });
            if wc.is_null() {
                continue;
            }
            for wi in tailq_foreach(&raw mut (*wc).waiters).map(NonNull::as_ptr) {
                cmdq_continue((*wi).item);
                tailq_remove(&raw mut (*wc).waiters, wi);
                free_(wi);
            }
            (*wc).woken = true;
            for wi in tailq_foreach(&raw mut (*wc).lockers).map(NonNull::as_ptr) {
                cmdq_continue((*wi).item);
                tailq_remove(&raw mut (*wc).lockers, wi);
                free_(wi);
            }
            (*wc).locked = false;
            cmd_wait_for_remove(wc);
        }
    }
}

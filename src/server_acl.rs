// Copyright (c) 2021 Holland Schutte, Jayson Morberg
// Copyright (c) 2021 Dallas Lyons <dallasdlyons@gmail.com>
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

use crate::libc::{getpwuid, getuid};
use crate::*;

bitflags::bitflags! {
    #[repr(transparent)]
    #[derive(Eq, PartialEq)]
    pub struct server_acl_user_flags: i32 {
        const SERVER_ACL_READONLY = 0x1;
    }
}

pub struct server_acl_user {
    pub uid: uid_t,

    pub flags: server_acl_user_flags,
}

thread_local! {
    static SERVER_ACL_ENTRIES: RefCell<BTreeMap<uid_t, Box<server_acl_user>>> = const { RefCell::new(BTreeMap::new()) };
}

pub fn server_acl_init() {
    let uid = unsafe { getuid() };
    if uid != 0 {
        server_acl_user_allow(0);
    }
    server_acl_user_allow(uid);
}

pub fn server_acl_user_exists(uid: uid_t) -> bool {
    SERVER_ACL_ENTRIES.with_borrow(|entries| entries.contains_key(&uid))
}

pub fn server_acl_user_is_readonly(uid: uid_t) -> Option<bool> {
    SERVER_ACL_ENTRIES.with_borrow(|entries| {
        entries
            .get(&uid)
            .map(|user| user.flags.contains(server_acl_user_flags::SERVER_ACL_READONLY))
    })
}

pub unsafe fn server_acl_display(item: *mut cmdq_item) {
    SERVER_ACL_ENTRIES.with_borrow(|entries| {
        for user in entries.values() {
            if user.uid == 0 {
                continue;
            }
            unsafe {
                let pw = getpwuid(user.uid);
                let name: *const u8 = if !pw.is_null() {
                    (*pw).pw_name.cast()
                } else {
                    c!("unknown")
                };
                if user.flags == server_acl_user_flags::SERVER_ACL_READONLY {
                    cmdq_print!(item, "{} (R)", _s(name));
                } else {
                    cmdq_print!(item, "{} (W)", _s(name));
                }
            }
        }
    });
}

pub fn server_acl_user_allow(uid: uid_t) {
    SERVER_ACL_ENTRIES.with_borrow_mut(|entries| {
        entries.entry(uid).or_insert_with(|| {
            Box::new(server_acl_user {
                uid,
                flags: server_acl_user_flags::empty(),
            })
        });
    });
}

pub fn server_acl_user_deny(uid: uid_t) {
    SERVER_ACL_ENTRIES.with_borrow_mut(|entries| {
        entries.remove(&uid);
    });
}

pub unsafe fn server_acl_user_allow_write(uid: uid_t) {
    SERVER_ACL_ENTRIES.with_borrow_mut(|entries| {
        if let Some(user) = entries.get_mut(&uid) {
            user.flags &= !server_acl_user_flags::SERVER_ACL_READONLY;
        }
    });

    unsafe {
        for c in tailq_foreach(&raw mut CLIENTS).map(NonNull::as_ptr) {
            let peer_uid = proc_get_peer_uid((*c).peer);
            if peer_uid != -1i32 as uid_t && peer_uid == uid {
                (*c).flags &= !client_flag::READONLY;
            }
        }
    }
}

pub unsafe fn server_acl_user_deny_write(uid: uid_t) {
    SERVER_ACL_ENTRIES.with_borrow_mut(|entries| {
        if let Some(user) = entries.get_mut(&uid) {
            user.flags |= server_acl_user_flags::SERVER_ACL_READONLY;
        }
    });

    unsafe {
        for c in tailq_foreach(&raw mut CLIENTS).map(NonNull::as_ptr) {
            let peer_uid = proc_get_peer_uid((*c).peer);
            if peer_uid != -1i32 as uid_t && peer_uid == uid {
                (*c).flags &= !client_flag::READONLY;
            }
        }
    }
}

pub unsafe fn server_acl_join(c: *mut client) -> c_int {
    unsafe {
        let uid = proc_get_peer_uid((*c).peer);
        if uid == -1i32 as uid_t {
            return 0;
        }

        match server_acl_user_is_readonly(uid) {
            None => 0,
            Some(true) => {
                (*c).flags |= client_flag::READONLY;
                1
            }
            Some(false) => 1,
        }
    }
}

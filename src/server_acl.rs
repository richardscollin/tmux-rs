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

use std::cmp::Ordering;

use crate::*;

use crate::libc::{getpwuid, getuid};

use crate::compat::{
    queue::tailq_foreach,
    tree::{rb_find, rb_foreach, rb_init, rb_insert, rb_remove},
};

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

    pub entry: rb_entry<server_acl_user>,
}

pub fn server_acl_cmp(user1: &server_acl_user, user2: &server_acl_user) -> Ordering {
    user1.uid.cmp(&user2.uid)
}

pub type server_acl_entries = rb_head<server_acl_user>;
static mut SERVER_ACL_ENTRIES: server_acl_entries = unsafe { zeroed() };

RB_GENERATE!(
    server_acl_entries,
    server_acl_user,
    entry,
    discr_entry,
    server_acl_cmp
);

pub unsafe fn server_acl_init() {
    unsafe {
        rb_init(&raw mut SERVER_ACL_ENTRIES);

        if getuid() != 0 {
            server_acl_user_allow(0);
        }
        server_acl_user_allow(getuid());
    }
}

pub unsafe fn server_acl_user_find(uid: uid_t) -> *mut server_acl_user {
    unsafe {
        let mut find: server_acl_user = server_acl_user { uid, ..zeroed() };

        rb_find::<_, _>(&raw mut SERVER_ACL_ENTRIES, &raw mut find)
    }
}

pub unsafe fn server_acl_display(item: *mut cmdq_item) {
    unsafe {
        // server_acl_entries
        for loop_ in rb_foreach(&raw mut SERVER_ACL_ENTRIES).map(NonNull::as_ptr) {
            if (*loop_).uid == 0 {
                continue;
            }
            let pw = getpwuid((*loop_).uid);
            let name: *const u8 = if !pw.is_null() {
                (*pw).pw_name.cast()
            } else {
                c!("unknown")
            };
            if (*loop_).flags == server_acl_user_flags::SERVER_ACL_READONLY {
                cmdq_print!(item, "{} (R)", _s(name));
            } else {
                cmdq_print!(item, "{} (W)", _s(name));
            }
        }
    }
}

pub unsafe fn server_acl_user_allow(uid: uid_t) {
    unsafe {
        let mut user = server_acl_user_find(uid);
        if user.is_null() {
            user = xcalloc1();
            (*user).uid = uid;
            // server_acl_entries
            rb_insert(&raw mut SERVER_ACL_ENTRIES, user);
        }
    }
}

pub unsafe fn server_acl_user_deny(uid: uid_t) {
    unsafe {
        let user = server_acl_user_find(uid);
        if !user.is_null() {
            // server_acl_entries
            rb_remove(&raw mut SERVER_ACL_ENTRIES, user);
            free_(user);
        }
    }
}

pub unsafe fn server_acl_user_allow_write(mut uid: uid_t) {
    unsafe {
        let user = server_acl_user_find(uid);
        if user.is_null() {
            return;
        }
        (*user).flags &= !server_acl_user_flags::SERVER_ACL_READONLY;

        for c in tailq_foreach(&raw mut CLIENTS).map(NonNull::as_ptr) {
            uid = proc_get_peer_uid((*c).peer);
            if uid != -1i32 as uid_t && uid == (*user).uid {
                (*c).flags &= !client_flag::READONLY;
            }
        }
    }
}

pub unsafe fn server_acl_user_deny_write(mut uid: uid_t) {
    unsafe {
        let user = server_acl_user_find(uid);
        if user.is_null() {
            return;
        }
        (*user).flags |= server_acl_user_flags::SERVER_ACL_READONLY;

        for c in tailq_foreach(&raw mut CLIENTS).map(NonNull::as_ptr) {
            uid = proc_get_peer_uid((*c).peer);
            if uid != -1i32 as uid_t && uid == (*user).uid {
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

        let user = server_acl_user_find(uid);
        if user.is_null() {
            return 0;
        }
        if (*user)
            .flags
            .contains(server_acl_user_flags::SERVER_ACL_READONLY)
        {
            (*c).flags |= client_flag::READONLY;
        }
        1
    }
}

pub unsafe fn server_acl_get_uid(user: *mut server_acl_user) -> uid_t {
    unsafe { (*user).uid }
}

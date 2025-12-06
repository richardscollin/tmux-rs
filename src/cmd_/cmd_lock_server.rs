// Copyright (c) 2008 Nicholas Marriott <nicholas.marriott@gmail.com>
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

pub static CMD_LOCK_SERVER_ENTRY: cmd_entry = cmd_entry {
    name: "lock-server",
    alias: Some("lock"),

    args: args_parse::new("", 0, 0, None),
    usage: "",

    flags: cmd_flag::CMD_AFTERHOOK,
    exec: cmd_lock_server_exec,
    source: cmd_entry_flag::zeroed(),
    target: cmd_entry_flag::zeroed(),
};

pub static CMD_LOCK_SESSION_ENTRY: cmd_entry = cmd_entry {
    name: "lock-session",
    alias: Some("locks"),

    args: args_parse::new("t:", 0, 0, None),
    usage: "[-t target-session]",

    target: cmd_entry_flag::new(
        b't',
        cmd_find_type::CMD_FIND_SESSION,
        cmd_find_flags::empty(),
    ),

    flags: cmd_flag::CMD_AFTERHOOK,
    exec: cmd_lock_server_exec,
    source: cmd_entry_flag::zeroed(),
};

pub static CMD_LOCK_CLIENT_ENTRY: cmd_entry = cmd_entry {
    name: "lock-client",
    alias: Some("lockc"),

    args: args_parse::new("t:", 0, 0, None),
    usage: "[-t target-client]",

    flags: cmd_flag::CMD_AFTERHOOK.union(cmd_flag::CMD_CLIENT_TFLAG),
    exec: cmd_lock_server_exec,
    source: cmd_entry_flag::zeroed(),
    target: cmd_entry_flag::zeroed(),
};

unsafe fn cmd_lock_server_exec(self_: *mut cmd, item: *mut cmdq_item) -> cmd_retval {
    unsafe {
        let target = cmdq_get_target(item);
        let tc = cmdq_get_target_client(item);

        if std::ptr::eq(cmd_get_entry(self_), &CMD_LOCK_SERVER_ENTRY) {
            server_lock();
        } else if std::ptr::eq(cmd_get_entry(self_), &CMD_LOCK_SESSION_ENTRY) {
            server_lock_session((*target).s);
        } else {
            server_lock_client(tc);
        }
        recalculate_sizes();
    }

    cmd_retval::CMD_RETURN_NORMAL
}

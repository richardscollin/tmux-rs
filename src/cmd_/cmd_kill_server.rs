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

use core::mem::zeroed;
use core::ptr::null;

use crate::libc::{SIGTERM, kill, pid_t};

use crate::{args_parse, cmd, cmd_entry, cmd_flag, cmd_get_entry, cmd_retval, cmdq_item, *};

pub static CMD_KILL_SERVER_ENTRY: cmd_entry = cmd_entry {
    name: SyncCharPtr::new(c"kill-server"),
    alias: SyncCharPtr::null(),

    args: args_parse::new(c"", 0, 0, None),
    usage: SyncCharPtr::new(c""),

    flags: cmd_flag::empty(),
    exec: cmd_kill_server_exec,
    source: cmd_entry_flag::zeroed(),
    target: cmd_entry_flag::zeroed(),
};

pub static CMD_START_SERVER_ENTRY: cmd_entry = cmd_entry {
    name: SyncCharPtr::new(c"start-server"),
    alias: SyncCharPtr::new(c"start"),
    args: args_parse::new(c"", 0, 0, None),
    usage: SyncCharPtr::new(c""),
    flags: cmd_flag::CMD_STARTSERVER,
    exec: cmd_kill_server_exec,
    source: cmd_entry_flag::zeroed(),
    target: cmd_entry_flag::zeroed(),
};

unsafe fn cmd_kill_server_exec(self_: *mut cmd, _: *mut cmdq_item) -> cmd_retval {
    unsafe {
        if std::ptr::eq(cmd_get_entry(self_), &CMD_KILL_SERVER_ENTRY) {
            kill(std::process::id() as pid_t, SIGTERM);
        }
    }

    cmd_retval::CMD_RETURN_NORMAL
}

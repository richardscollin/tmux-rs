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
use super::*;

use crate::compat::queue::tailq_foreach;
use crate::compat::tree::rb_empty;

pub static cmd_attach_session_entry: cmd_entry = cmd_entry {
    name: SyncCharPtr::new(c"attach-session"),
    alias: SyncCharPtr::new(c"attach"),

    args: args_parse::new(c"c:dEf:rt:x", 0, 0, None),
    usage: SyncCharPtr::new(c"[-dErx] [-c working-directory] [-f flags] [-t target-session]"),

    flags: cmd_flag::CMD_STARTSERVER.union(cmd_flag::CMD_READONLY),
    exec: cmd_attach_session_exec,
    source: cmd_entry_flag::zeroed(),
    target: cmd_entry_flag::zeroed(),
};

pub unsafe fn cmd_attach_session(
    item: *mut cmdq_item,
    tflag: *const u8,
    dflag: c_int,
    xflag: c_int,
    rflag: c_int,
    cflag: *const u8,
    eflag: c_int,
    fflag: *const u8,
) -> cmd_retval {
    unsafe {
        let current: *mut cmd_find_state = cmdq_get_current(item);
        let mut target: cmd_find_state = zeroed(); // TODO can be uninit
        let c: *mut client = cmdq_get_client(item);

        let cwd: *mut u8;
        let mut cause: *mut u8 = null_mut();

        let msgtype: msgtype;

        if rb_empty(&raw mut sessions) {
            cmdq_error!(item, "no sessions");
            return cmd_retval::CMD_RETURN_ERROR;
        }

        if c.is_null() {
            return cmd_retval::CMD_RETURN_NORMAL;
        }

        if server_client_check_nested(c) != 0 {
            cmdq_error!(
                item,
                "sessions should be nested with care, unset $TMUX to force",
            );
            return cmd_retval::CMD_RETURN_ERROR;
        }

        let (type_, flags) =
            if !tflag.is_null() && *tflag.add(libc::strcspn(tflag, c!(":."))) != b'\0' as u8 {
                (cmd_find_type::CMD_FIND_PANE, 0)
            } else {
                (cmd_find_type::CMD_FIND_SESSION, CMD_FIND_PREFER_UNATTACHED)
            };
        if cmd_find_target(&raw mut target, item, tflag, type_, flags) != 0 {
            return cmd_retval::CMD_RETURN_ERROR;
        }

        let s = target.s;
        let wl = target.wl;
        let wp = target.wp;

        if !wl.is_null() {
            if !wp.is_null() {
                window_set_active_pane((*wp).window, wp, 1);
            }
            session_set_current(s, wl);
            if !wp.is_null() {
                cmd_find_from_winlink_pane(current, wl, wp, 0);
            } else {
                cmd_find_from_winlink(current, wl, 0);
            }
        }

        if !cflag.is_null() {
            cwd = format_single(item, cflag, c, s, wl, wp);
            free_((*s).cwd);
            (*s).cwd = cwd;
        }
        if !fflag.is_null() {
            server_client_set_flags(c, fflag);
        }
        if rflag != 0 {
            (*c).flags |= client_flag::READONLY | client_flag::IGNORESIZE;
        }

        (*c).last_session = (*c).session;
        if !(*c).session.is_null() {
            if dflag != 0 || xflag != 0 {
                if xflag != 0 {
                    msgtype = msgtype::MSG_DETACHKILL;
                } else {
                    msgtype = msgtype::MSG_DETACH;
                }
                for c_loop in tailq_foreach(&raw mut clients).map(NonNull::as_ptr) {
                    {
                        if (*c_loop).session != s || c == c_loop {
                            continue;
                        }
                        server_client_detach(c_loop, msgtype);
                    }
                }
            }
            if eflag == 0 {
                environ_update((*s).options, (*c).environ, (*s).environ);
            }

            server_client_set_session(c, s);
            if !cmdq_get_flags(item).intersects(cmdq_state_flags::CMDQ_STATE_REPEAT) {
                server_client_set_key_table(c, null_mut());
            }
        } else {
            if server_client_open(c, &raw mut cause) != 0 {
                cmdq_error!(item, "open terminal failed: {}", _s(cause));
                free_(cause);
                return cmd_retval::CMD_RETURN_ERROR;
            }

            if dflag != 0 || xflag != 0 {
                msgtype = if xflag != 0 {
                    msgtype::MSG_DETACHKILL
                } else {
                    msgtype::MSG_DETACH
                };
                for c_loop in tailq_foreach(&raw mut clients).map(NonNull::as_ptr) {
                    if (*c_loop).session != s || c == c_loop {
                        continue;
                    }
                    server_client_detach(c_loop, msgtype);
                }
            }
            if eflag == 0 {
                environ_update((*s).options, (*c).environ, (*s).environ);
            }

            server_client_set_session(c, s);
            server_client_set_key_table(c, null_mut());

            if !(*c).flags.intersects(client_flag::CONTROL) {
                proc_send((*c).peer, msgtype::MSG_READY, -1, null_mut(), 0);
            }
            notify_client(c"client-attached", c);
            (*c).flags |= client_flag::ATTACHED;

            if cfg_finished != 0 {
                cfg_show_causes(s);
            }
        }
        cmd_retval::CMD_RETURN_NORMAL
    }
}

unsafe fn cmd_attach_session_exec(self_: *mut cmd, item: *mut cmdq_item) -> cmd_retval {
    unsafe {
        let args = cmd_get_args(self_);

        cmd_attach_session(
            item,
            args_get(args, b't'),
            args_has(args, b'd'),
            args_has(args, b'x'),
            args_has(args, b'r'),
            args_get(args, b'c'),
            args_has(args, b'E'),
            args_get(args, b'f'),
        )
    }
}

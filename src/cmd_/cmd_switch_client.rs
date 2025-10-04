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
use crate::libc::strcspn;
use crate::*;

pub static CMD_SWITCH_CLIENT_ENTRY: cmd_entry = cmd_entry {
    name: "switch-client",
    alias: Some("switchc"),

    args: args_parse::new(c"lc:EFnpt:rT:Z", 0, 0, None),
    usage: SyncCharPtr::new(c"[-ElnprZ] [-c target-client] [-t target-session] [-T key-table]"),

    flags: cmd_flag::CMD_READONLY.union(cmd_flag::CMD_CLIENT_CFLAG),
    exec: cmd_switch_client_exec,
    source: cmd_entry_flag::zeroed(),
    target: cmd_entry_flag::zeroed(),
};

unsafe fn cmd_switch_client_exec(self_: *mut cmd, item: *mut cmdq_item) -> cmd_retval {
    unsafe {
        let args = cmd_get_args(self_);
        let current = cmdq_get_current(item);
        let mut target: cmd_find_state = zeroed(); // TODO use uninit
        let tflag = args_get_(args, 't');
        let tc = cmdq_get_target_client(item);

        let type_: cmd_find_type;
        let flags: cmd_find_flags;
        if !tflag.is_null() && *tflag.add(strcspn(tflag, c!(":.%"))) != b'\0' {
            type_ = cmd_find_type::CMD_FIND_PANE;
            flags = cmd_find_flags::empty();
        } else {
            type_ = cmd_find_type::CMD_FIND_SESSION;
            flags = cmd_find_flags::CMD_FIND_PREFER_UNATTACHED;
        }
        if cmd_find_target(&raw mut target, item, tflag, type_, flags) != 0 {
            return cmd_retval::CMD_RETURN_ERROR;
        }
        let mut s = target.s;
        let wl = target.wl;
        let wp = target.wp;

        if args_has(args, 'r') {
            if (*tc).flags.intersects(client_flag::READONLY) {
                (*tc).flags &= !(client_flag::READONLY | client_flag::IGNORESIZE);
            } else {
                (*tc).flags |= client_flag::READONLY | client_flag::IGNORESIZE;
            }
        }

        let tablename = args_get_(args, 'T');
        if !tablename.is_null() {
            let table = key_bindings_get_table(tablename, false);
            if table.is_null() {
                cmdq_error!(item, "table {} doesn't exist", _s(tablename));
                return cmd_retval::CMD_RETURN_ERROR;
            }
            (*table).references += 1;
            key_bindings_unref_table((*tc).keytable);
            (*tc).keytable = table;
            return cmd_retval::CMD_RETURN_NORMAL;
        }

        if args_has(args, 'n') {
            s = session_next_session((*tc).session);
            if s.is_null() {
                cmdq_error!(item, "can't find next session");
                return cmd_retval::CMD_RETURN_ERROR;
            }
        } else if args_has(args, 'p') {
            s = session_previous_session((*tc).session);
            if s.is_null() {
                cmdq_error!(item, "can't find previous session");
                return cmd_retval::CMD_RETURN_ERROR;
            }
        } else if args_has(args, 'l') {
            if !(*tc).last_session.is_null() && session_alive((*tc).last_session) {
                s = (*tc).last_session;
            } else {
                s = null_mut();
            }
            if s.is_null() {
                cmdq_error!(item, "can't find last session");
                return cmd_retval::CMD_RETURN_ERROR;
            }
        } else {
            if cmdq_get_client(item).is_null() {
                return cmd_retval::CMD_RETURN_NORMAL;
            }
            if !wl.is_null() && !wp.is_null() && wp != (*(*wl).window).active {
                let w = (*wl).window;
                if window_push_zoom(w, false, args_has(args, 'Z')) {
                    server_redraw_window(w);
                }
                window_redraw_active_switch(w, wp);
                window_set_active_pane(w, wp, 1);
                if window_pop_zoom(w) != 0 {
                    server_redraw_window(w);
                }
            }
            if !wl.is_null() {
                session_set_current(s, wl);
                cmd_find_from_session(current, s, cmd_find_flags::empty());
            }
        }

        if !args_has(args, 'E') {
            environ_update((*s).options, (*tc).environ, (*s).environ);
        }

        server_client_set_session(tc, s);
        if !cmdq_get_flags(item).intersects(cmdq_state_flags::CMDQ_STATE_REPEAT) {
            server_client_set_key_table(tc, null_mut());
        }

        cmd_retval::CMD_RETURN_NORMAL
    }
}

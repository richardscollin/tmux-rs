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
use crate::compat::tree::rb_min;
use crate::libc::{sscanf, tcgetattr};
use crate::*;

const NEW_SESSION_TEMPLATE: *const u8 = c!("#{session_name}:");

pub static CMD_NEW_SESSION_ENTRY: cmd_entry = cmd_entry {
    name: SyncCharPtr::new(c"new-session"),
    alias: SyncCharPtr::new(c"new"),

    args: args_parse::new(c"Ac:dDe:EF:f:n:Ps:t:x:Xy:", 0, -1, None),
    usage: SyncCharPtr::new(c"[-AdDEPX] [-c start-directory] [-e environment] [-F format] [-f flags] [-n window-name] [-s session-name] [-t target-session] [-x width] [-y height] [shell-command]"),

    target: cmd_entry_flag::new(b't', cmd_find_type::CMD_FIND_SESSION, cmd_find_flags::CMD_FIND_CANFAIL),

    flags: cmd_flag::CMD_STARTSERVER,
    exec: cmd_new_session_exec,
    source: cmd_entry_flag::zeroed(),
};

pub static CMD_HAS_SESSION_ENTRY: cmd_entry = cmd_entry {
    name: SyncCharPtr::new(c"has-session"),
    alias: SyncCharPtr::new(c"has"),

    args: args_parse::new(c"t:", 0, 0, None),
    usage: SyncCharPtr::new(c"[-t target-session]"),

    target: cmd_entry_flag::new(
        b't',
        cmd_find_type::CMD_FIND_SESSION,
        cmd_find_flags::empty(),
    ),

    flags: cmd_flag::empty(),
    exec: cmd_new_session_exec,

    source: cmd_entry_flag::zeroed(),
};

unsafe fn cmd_new_session_exec(self_: *mut cmd, item: *mut cmdq_item) -> cmd_retval {
    let __func__ = c!("cmd_new_session_exec");

    unsafe {
        let args = cmd_get_args(self_);
        let current = cmdq_get_current(item);
        let target = cmdq_get_target(item);
        let c = cmdq_get_client(item);
        let s;
        let as_;
        let mut groupwith = null_mut();
        let env: *mut environ;
        let oo;
        let mut tio: termios = zeroed();
        let tiop;
        let mut sg: *mut session_group = null_mut();
        let errstr: *const u8 = null();
        let group: *const u8;
        let mut tmp: *const u8;
        let mut cause = null_mut();
        let mut cwd = null_mut();
        let cp;
        let mut newname = null_mut();
        let name;
        let mut prefix = null_mut();
        let mut detached;
        let mut already_attached;
        let mut is_control = false;
        let mut sx = 0u32;
        let mut sy = 0u32;
        let mut dsx;
        let mut dsy;
        let count = args_count(args);
        let mut sc: spawn_context = zeroed();
        let retval;
        let mut av: *mut args_value;

        'fail: {
            if std::ptr::eq(cmd_get_entry(self_), &CMD_HAS_SESSION_ENTRY) {
                // cmd_find_target() will fail if the session cannot be found,
                // so always return success here.
                return cmd_retval::CMD_RETURN_NORMAL;
            }

            if args_has(args, 't') && (count != 0 || args_has(args, 'n')) {
                cmdq_error!(item, "command or window name given with target");
                return cmd_retval::CMD_RETURN_ERROR;
            }

            tmp = args_get_(args, 's');
            if !tmp.is_null() {
                name = format_single(item, tmp, c, null_mut(), null_mut(), null_mut());
                newname = session_check_name(name);
                if newname.is_null() {
                    cmdq_error!(item, "invalid session: {}", _s(name));
                    free_(name);
                    return cmd_retval::CMD_RETURN_ERROR;
                }
                free_(name);
            }
            if args_has(args, 'A') {
                as_ = if !newname.is_null() {
                    session_find(newname)
                } else {
                    (*target).s
                };
                if !as_.is_null() {
                    retval = cmd_attach_session(
                        item,
                        (*as_).name,
                        args_has(args, 'D'),
                        args_has(args, 'X'),
                        false,
                        null(),
                        args_has(args, 'E'),
                        args_get(args, b'f'),
                    );
                    free_(newname);
                    return retval;
                }
            }
            if !newname.is_null() && !session_find(newname).is_null() {
                cmdq_error!(item, "duplicate session: {}", _s(newname));
                break 'fail;
            }

            // Is this going to be part of a session group?
            group = args_get_(args, 't');
            if !group.is_null() {
                groupwith = (*target).s;
                sg = if groupwith.is_null() {
                    session_group_find(group)
                } else {
                    session_group_contains(groupwith)
                };
                if !sg.is_null() {
                    prefix = xstrdup((*sg).name).as_ptr();
                } else if !groupwith.is_null() {
                    prefix = xstrdup((*groupwith).name).as_ptr();
                } else {
                    prefix = session_check_name(group);
                    if prefix.is_null() {
                        cmdq_error!(item, "invalid session group: {}", _s(group));
                        break 'fail;
                    }
                }
            }

            // Set -d if no client.
            detached = args_has(args, 'd');
            if c.is_null() {
                detached = true;
            } else if (*c).flags.intersects(client_flag::CONTROL) {
                is_control = true;
            }

            // Is this client already attached?
            already_attached = false;
            if !c.is_null() && !(*c).session.is_null() {
                already_attached = true;
            }

            // Get the new session working directory.
            tmp = args_get_(args, 'c');
            cwd = if !tmp.is_null() {
                format_single(item, tmp, c, null_mut(), null_mut(), null_mut())
            } else {
                xstrdup(server_client_get_cwd(c, null_mut())).as_ptr()
            };

            // If this is a new client, check for nesting and save the termios
            // settings (part of which is used for new windows in this session).
            //
            // tcgetattr() is used rather than using tty.tio since if the client is
            // detached, tty_open won't be called. It must be done before opening
            // the terminal as that calls tcsetattr() to prepare for tmux taking
            // over.
            if !detached
                && !already_attached
                && (*c).fd != -1
                && !(*c).flags.intersects(client_flag::CONTROL)
            {
                if server_client_check_nested(cmdq_get_client(item)) {
                    cmdq_error!(
                        item,
                        "sessions should be nested with care, unset $TMUX to force"
                    );
                    break 'fail;
                }
                if tcgetattr((*c).fd, &raw mut tio) != 0 {
                    fatal("tcgetattr failed");
                }
                tiop = &raw mut tio;
            } else {
                tiop = null_mut();
            }

            // Open the terminal if necessary.
            if !detached && !already_attached && server_client_open(c, &raw mut cause) != 0 {
                cmdq_error!(item, "open terminal failed: {}", _s(cause));
                free_(cause);
                break 'fail;
            }

            // Get default session size.
            dsx = if args_has(args, 'x') {
                tmp = args_get_(args, 'x');
                if streq_(tmp, "-") {
                    if !c.is_null() { (*c).tty.sx } else { 80 }
                } else {
                    let Ok(dsx_) = strtonum(tmp, 1, u16::MAX) else {
                        cmdq_error!(item, "width {}", _s(errstr));
                        break 'fail;
                    };
                    dsx_ as u32
                }
            } else {
                80
            };

            dsy = if args_has(args, 'y') {
                tmp = args_get_(args, 'y');
                if streq_(tmp, "-") {
                    if !c.is_null() { (*c).tty.sy } else { 24 }
                } else {
                    let Ok(dsy_) = strtonum(tmp, 1, u16::MAX) else {
                        cmdq_error!(item, "height {}", _s(errstr));
                        break 'fail;
                    };
                    dsy_ as u32
                }
            } else {
                24
            };

            // sx = 0;
            // sy = 0;
            // Find new session size.
            if !detached && !is_control {
                sx = (*c).tty.sx;
                sy = (*c).tty.sy;
                if sy > 0 && options_get_number_(GLOBAL_S_OPTIONS, "status") != 0 {
                    sy -= 1;
                }
            } else {
                tmp = options_get_string_(GLOBAL_S_OPTIONS, "default-size");
                if sscanf(tmp.cast(), c"%ux%u".as_ptr(), &raw mut sx, &raw mut sy) != 2 {
                    sx = dsx;
                    sy = dsy;
                } else {
                    if args_has(args, 'x') {
                        sx = dsx;
                    }
                    if args_has(args, 'y') {
                        sy = dsy;
                    }
                }
            }
            if sx == 0 {
                sx = 1;
            }
            if sy == 0 {
                sy = 1;
            }

            // Create the new session.
            oo = options_create(GLOBAL_S_OPTIONS);
            if args_has(args, 'x') || args_has(args, 'y') {
                if !args_has(args, 'x') {
                    dsx = sx;
                }
                if !args_has(args, 'y') {
                    dsy = sy;
                }
                options_set_string!(oo, c!("default-size"), false, "{dsx}x{dsy}");
            }
            env = environ_create().as_ptr();
            if !c.is_null() && !args_has(args, 'E') {
                environ_update(GLOBAL_S_OPTIONS, (*c).environ, env);
            }
            av = args_first_value(args, b'e');
            while !av.is_null() {
                environ_put(env, (*av).union_.string, environ_flags::empty());
                av = args_next_value(av);
            }
            s = session_create(prefix, newname, cwd, env, oo, tiop);

            // Spawn the initial window.
            sc.item = item;
            sc.s = s;
            if !detached {
                sc.tc = c;
            }

            sc.name = args_get_(args, 'n');
            args_to_vector(args, &raw mut sc.argc, &raw mut sc.argv);

            sc.idx = -1;
            sc.cwd = args_get_(args, 'c');

            sc.flags = spawn_flags::empty();

            if spawn_window(&raw mut sc, &raw mut cause).is_null() {
                session_destroy(s, 0, __func__);
                cmdq_error!(item, "create window failed: {}", _s(cause));
                free_(cause);
                break 'fail;
            }

            // If a target session is given, this is to be part of a session group,
            // so add it to the group and synchronize.
            if !group.is_null() {
                if sg.is_null() {
                    if !groupwith.is_null() {
                        sg = session_group_new((*groupwith).name);
                        session_group_add(sg, groupwith);
                    } else {
                        sg = session_group_new(group);
                    }
                }
                session_group_add(sg, s);
                session_group_synchronize_to(s);
                session_select(s, (*rb_min::<winlink, _>(&raw mut (*s).windows)).idx);
            }
            notify_session(c"session-created", s);

            // Set the client to the new session. If a command client exists, it is
            // taking this session and needs to get MSG_READY and stay around.
            if !detached {
                if args_has(args, 'f') {
                    server_client_set_flags(c, args_get_(args, 'f'));
                }
                if !already_attached {
                    if !(*c).flags.intersects(client_flag::CONTROL) {
                        proc_send((*c).peer, msgtype::MSG_READY, -1, null(), 0);
                    }
                } else if !(*c).session.is_null() {
                    (*c).last_session = (*c).session;
                }
                server_client_set_session(c, s);
                if !cmdq_get_flags(item).intersects(cmdq_state_flags::CMDQ_STATE_REPEAT) {
                    server_client_set_key_table(c, null_mut());
                }
            }

            // Print if requested.
            if args_has(args, 'P') {
                let mut template: *const u8 = args_get_(args, 'F');
                if template.is_null() {
                    template = NEW_SESSION_TEMPLATE;
                }
                cp = format_single(item, template, c, s, (*s).curw, null_mut());
                cmdq_print!(item, "{}", _s(cp));
                free_(cp);
            }

            if !detached {
                (*c).flags |= client_flag::ATTACHED;
            }
            if !args_has(args, 'd') {
                cmd_find_from_session(current, s, cmd_find_flags::empty());
            }

            let mut fs: MaybeUninit<cmd_find_state> = MaybeUninit::<cmd_find_state>::uninit(); //TODO use uninit;
            cmd_find_from_session(fs.as_mut_ptr(), s, cmd_find_flags::empty());
            cmdq_insert_hook!(s, item, fs.as_mut_ptr(), "after-new-session");

            if CFG_FINISHED.load(atomic::Ordering::Acquire) {
                cfg_show_causes(s);
            }

            if !sc.argv.is_null() {
                cmd_free_argv(sc.argc, sc.argv);
            }
            free_(cwd);
            free_(newname);
            free_(prefix);
            return cmd_retval::CMD_RETURN_NORMAL;
        }

        // fail:
        if !sc.argv.is_null() {
            cmd_free_argv(sc.argc, sc.argv);
        }

        free_(cwd);
        free_(newname);
        free_(prefix);
        cmd_retval::CMD_RETURN_ERROR
    }
}

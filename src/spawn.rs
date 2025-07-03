// Copyright (c) 2019 Nicholas Marriott <nicholas.marriott@gmail.com>
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

use crate::compat::{
    closefrom,
    fdforkpty::fdforkpty,
    queue::{tailq_first, tailq_foreach, tailq_remove},
    tailq_insert_head,
};
use libc::{
    _exit, SIG_BLOCK, SIG_SETMASK, STDERR_FILENO, STDIN_FILENO, TCSANOW, VERASE, chdir, close,
    execl, execvp, sigfillset, sigprocmask, strrchr, tcgetattr, tcsetattr,
};

#[cfg(feature = "utempter")]
use crate::utempter::utempter_add_record;

pub unsafe fn spawn_log(from: *const c_char, sc: *mut spawn_context) {
    unsafe {
        let s = (*sc).s;
        let wl = (*sc).wl;
        let wp0 = (*sc).wp0;
        let name = cmdq_get_name((*sc).item);
        type tmp_type = [c_char; 128];
        let mut tmp = MaybeUninit::<tmp_type>::uninit();

        log_debug!("{}: {}, flags={:#x}", _s(from), _s(name), (*sc).flags);

        if !wl.is_null() && !wp0.is_null() {
            xsnprintf_!(
                tmp.as_mut_ptr().cast(),
                size_of::<tmp_type>(),
                "wl={} wp0=%{}",
                (*wl).idx,
                (*wp0).id,
            );
        } else if !wl.is_null() {
            xsnprintf_!(
                tmp.as_mut_ptr().cast(),
                size_of::<tmp_type>(),
                "wl={} wp0=none",
                (*wl).idx,
            );
        } else if !wp0.is_null() {
            xsnprintf_!(
                tmp.as_mut_ptr().cast(),
                size_of::<tmp_type>(),
                "wl=none wp0=%{}",
                (*wp0).id,
            );
        } else {
            xsnprintf_!(
                tmp.as_mut_ptr().cast(),
                size_of::<tmp_type>(),
                "wl=none wp0=none",
            );
        }
        log_debug!(
            "{}: s=${} {} idx={}",
            _s(from),
            (*s).id,
            _s(tmp.as_ptr().cast()),
            (*sc).idx
        );
        log_debug!(
            "{}: name={}",
            _s(from),
            _s(if (*sc).name.is_null() {
                c"none".as_ptr()
            } else {
                (*sc).name
            }),
        );
    }
}

pub unsafe fn spawn_window(
    sc: *mut spawn_context,
    cause: *mut *mut c_char,
) -> *mut winlink {
    let __func__ = c"spawn_window".as_ptr();
    unsafe {
        let item = (*sc).item;
        let c = cmdq_get_client(item);
        let s = (*sc).s;
        let mut idx = (*sc).idx;
        let mut w: *mut window = null_mut();
        // struct window *w;
        // struct window_pane *wp;
        let mut wp = null_mut();
        // struct winlink *wl;

        spawn_log(__func__, sc);

        /*
         * If the window already exists, we are respawning, so destroy all the
         * panes except one.
         */
        if (*sc).flags & SPAWN_RESPAWN != 0 {
            w = (*(*sc).wl).window;
            if !(*sc).flags & SPAWN_KILL != 0 {
                for wp_ in tailq_foreach::<_, discr_entry>(&raw mut (*w).panes).map(NonNull::as_ptr)
                {
                    wp = wp_;
                    if (*wp).fd != -1 {
                        break;
                    }
                }
                if !wp.is_null() {
                    *cause =
                        format_nul!("window {}:{} still active", _s((*s).name), (*(*sc).wl).idx,);
                    return null_mut();
                }
            }

            (*sc).wp0 = tailq_first(&raw mut (*w).panes);
            tailq_remove::<_, discr_entry>(&raw mut (*w).panes, (*sc).wp0);

            layout_free(w);
            window_destroy_panes(w);

            tailq_insert_head::<_, discr_entry>(&raw mut (*w).panes, (*sc).wp0);
            window_pane_resize((*sc).wp0, (*w).sx, (*w).sy);

            layout_init(w, (*sc).wp0);
            (*w).active = null_mut();
            window_set_active_pane(w, (*sc).wp0, 0);
        }

        /*
         * Otherwise we have no window so we will need to create one. First
         * check if the given index already exists and destroy it if so.
         */
        if (!(*sc).flags & SPAWN_RESPAWN != 0) && idx != -1 {
            let wl = winlink_find_by_index(&raw mut (*s).windows, idx);
            if !wl.is_null() && (!(*sc).flags & SPAWN_KILL != 0) {
                *cause = format_nul!("index {} in use", idx);
                return null_mut();
            }
            if !wl.is_null() {
                /*
                 * Can't use session_detach as it will destroy session
                 * if this makes it empty.
                 */
                (*wl).flags &= !WINLINK_ALERTFLAGS;
                notify_session_window(c"window-unlinked", s, (*wl).window);
                winlink_stack_remove(&raw mut (*s).lastw, wl);
                winlink_remove(&raw mut (*s).windows, wl);

                if (*s).curw == wl {
                    (*s).curw = null_mut();
                    (*sc).flags &= !SPAWN_DETACHED;
                }
            }
        }

        /* Then create a window if needed. */
        if !(*sc).flags & SPAWN_RESPAWN != 0 {
            if idx == -1 {
                idx = -1 - options_get_number_((*s).options, c"base-index") as i32;
            }
            (*sc).wl = winlink_add(&raw mut (*s).windows, idx);
            if (*sc).wl.is_null() {
                *cause = format_nul!("couldn't add window {}", idx);
                return null_mut();
            }
            let mut sx = 0u32;
            let mut sy = 0u32;
            let mut xpixel = 0u32;
            let mut ypixel = 0u32;
            default_window_size(
                (*sc).tc,
                s,
                null_mut(),
                &raw mut sx,
                &raw mut sy,
                &raw mut xpixel,
                &raw mut ypixel,
                None,
            );
            w = window_create(sx, sy, xpixel, ypixel);
            if w.is_null() {
                winlink_remove(&raw mut (*s).windows, (*sc).wl);
                *cause = format_nul!("couldn't create window {idx}");
                return null_mut();
            }
            if (*s).curw.is_null() {
                (*s).curw = (*sc).wl;
            }
            (*(*sc).wl).session = s;
            (*w).latest = (*sc).tc.cast();
            winlink_set_window((*sc).wl, w);
        } else {
            w = null_mut();
        }
        (*sc).flags |= SPAWN_NONOTIFY;

        /* Spawn the pane. */
        wp = spawn_pane(sc, cause);
        if wp.is_null() {
            if !(*sc).flags & SPAWN_RESPAWN != 0 {
                winlink_remove(&raw mut (*s).windows, (*sc).wl);
            }
            return null_mut();
        }

        /* Set the name of the new window. */
        if !(*sc).flags & SPAWN_RESPAWN != 0 {
            free_((*w).name);
            if !(*sc).name.is_null() {
                (*w).name = format_single(item, (*sc).name, c, s, null_mut(), null_mut());
                options_set_number((*w).options, c"automatic-rename".as_ptr(), 0);
            } else {
                (*w).name = default_window_name(w);
            }
        }

        /* Switch to the new window if required. */
        if !(*sc).flags & SPAWN_DETACHED != 0 {
            session_select(s, (*(*sc).wl).idx);
        }

        /* Fire notification if new window. */
        if !(*sc).flags & SPAWN_RESPAWN != 0 {
            notify_session_window(c"window-linked", s, w);
        }

        session_group_synchronize_from(s);
        (*sc).wl
    }
}

pub unsafe fn spawn_pane(
    sc: *mut spawn_context,
    cause: *mut *mut c_char,
) -> *mut window_pane {
    let __func__ = c"spawn_pane".as_ptr();
    unsafe {
        let item = (*sc).item;
        let target = cmdq_get_target(item);
        let c = cmdq_get_client(item);
        let s = (*sc).s;
        let w = (*(*sc).wl).window;
        let mut new_wp: *mut window_pane = null_mut();
        let mut child: *mut environ = null_mut();
        let mut ee: *mut environ_entry = null_mut();
        let mut argv: *mut *mut c_char = null_mut();
        let mut cp: *mut c_char = null_mut();
        let mut argvp: *mut *mut c_char = null_mut();
        let mut argv0: *mut c_char = null_mut();
        let mut cwd: *mut c_char = null_mut();
        let mut new_cwd: *mut c_char = null_mut();
        let mut cmd: *const c_char = null();
        let mut tmp: *const c_char = null();
        let mut argc = 0;
        let mut idx: u32 = 0;
        let mut now: libc::termios = zeroed();
        let mut hlimit: u32 = 0;
        let mut ws: libc::winsize = zeroed();
        let mut set: libc::sigset_t = zeroed();
        let mut oldset: libc::sigset_t = zeroed();
        let mut key: key_code = 0;

        'complete: {
            spawn_log(__func__, sc);

            if !(*sc).cwd.is_null() {
                cwd = format_single(item, (*sc).cwd, c, (*target).s, null_mut(), null_mut());
                if *cwd != b'/' as _ {
                    new_cwd =
                        format_nul!("{}/{}", _s(server_client_get_cwd(c, (*target).s)), _s(cwd));
                    free_(cwd);
                    cwd = new_cwd;
                }
            } else if !(*sc).flags & SPAWN_RESPAWN != 0 {
                cwd = xstrdup(server_client_get_cwd(c, (*target).s)).as_ptr();
            } else {
                cwd = null_mut();
            }

            /*
             * If we are respawning then get rid of the old process. Otherwise
             * either create a new cell or assign to the one we are given.
             */
            hlimit = options_get_number_((*s).options, c"history-limit") as u32;
            if (*sc).flags & SPAWN_RESPAWN != 0 {
                if (*(*sc).wp0).fd != -1 && (!(*sc).flags & SPAWN_KILL != 0) {
                    window_pane_index((*sc).wp0, &raw mut idx);
                    *cause = format_nul!(
                        "pane {}:{}.{} still active",
                        _s((*s).name),
                        (*(*sc).wl).idx,
                        idx
                    );
                    free_(cwd);
                    return null_mut();
                }
                if (*(*sc).wp0).fd != -1 {
                    bufferevent_free((*(*sc).wp0).event);
                    close((*(*sc).wp0).fd);
                }
                window_pane_reset_mode_all((*sc).wp0);
                screen_reinit(&raw mut (*(*sc).wp0).base);
                input_free((*(*sc).wp0).ictx);
                (*(*sc).wp0).ictx = null_mut();
                new_wp = (*sc).wp0;
                (*new_wp).flags &=
                    !(window_pane_flags::PANE_STATUSREADY | window_pane_flags::PANE_STATUSDRAWN);
            } else if (*sc).lc.is_null() {
                new_wp = window_add_pane(w, null_mut(), hlimit, (*sc).flags);
                layout_init(w, new_wp);
            } else {
                new_wp = window_add_pane(w, (*sc).wp0, hlimit, (*sc).flags);
                if (*sc).flags & SPAWN_ZOOM != 0 {
                    layout_assign_pane((*sc).lc, new_wp, 1);
                } else {
                    layout_assign_pane((*sc).lc, new_wp, 0);
                }
            }

            /*
             * Now we have a pane with nothing running in it ready for the new
             * process. Work out the command and arguments and store the working
             * directory.
             */
            if (*sc).argc == 0 && (!(*sc).flags & SPAWN_RESPAWN != 0) {
                cmd = options_get_string_((*s).options, c"default-command");
                if !cmd.is_null() && *cmd != b'\0' as c_char {
                    argc = 1;
                    argv = &raw mut cmd as *mut *mut c_char;
                } else {
                    argc = 0;
                    argv = null_mut();
                }
            } else {
                argc = (*sc).argc;
                argv = (*sc).argv;
            }
            if !cwd.is_null() {
                free_((*new_wp).cwd);
                (*new_wp).cwd = cwd;
            }

            /*
             * Replace the stored arguments if there are new ones. If not, the
             * existing ones will be used (they will only exist for respawn).
             */
            if argc > 0 {
                cmd_free_argv((*new_wp).argc, (*new_wp).argv);
                (*new_wp).argc = argc;
                (*new_wp).argv = cmd_copy_argv(argc, argv);
            }

            /* Create an environment for this pane. */
            child = environ_for_session(s, 0);
            if !(*sc).environ.is_null() {
                environ_copy((*sc).environ, child);
            }
            environ_set!(child, c"TMUX_PANE".as_ptr(), 0, "%{}", (*new_wp).id,);

            /*
             * Then the PATH environment variable. The session one is replaced from
             * the client if there is one because otherwise running "tmux new
             * myprogram" wouldn't work if myprogram isn't in the session's path.
             */
            if !c.is_null() && (*c).session.is_null() {
                /* only unattached clients */
                ee = environ_find((*c).environ, c"PATH".as_ptr());
                if !ee.is_null() {
                    environ_set!(
                        child,
                        c"PATH".as_ptr(),
                        0,
                        "{}",
                        _s(transmute_ptr((*ee).value))
                    );
                }
            }
            if environ_find(child, c"PATH".as_ptr()).is_null() {
                environ_set!(child, c"PATH".as_ptr(), 0, "{}", _s(_PATH_DEFPATH));
            }

            /* Then the shell. If respawning, use the old one. */
            if !(*sc).flags & SPAWN_RESPAWN != 0 {
                tmp = options_get_string_((*s).options, c"default-shell");
                if !checkshell(tmp) {
                    tmp = _PATH_BSHELL;
                }
                free_((*new_wp).shell);
                (*new_wp).shell = xstrdup(tmp).as_ptr();
            }
            environ_set!(child, c"SHELL".as_ptr(), 0, "{}", _s((*new_wp).shell));

            /* Log the arguments we are going to use. */
            log_debug!("{}: shell={}", _s(__func__), _s((*new_wp).shell));
            if (*new_wp).argc != 0 {
                cp = cmd_stringify_argv((*new_wp).argc, (*new_wp).argv);
                log_debug!("{}: cmd={}", _s(__func__), _s(cp));
                free_(cp);
            }
            log_debug!("{}: cwd={}", _s(__func__), _s((*new_wp).cwd));
            cmd_log_argv!((*new_wp).argc, (*new_wp).argv, "{}", _s(__func__));
            environ_log!(child, "{}: environment ", _s(__func__));

            /* Initialize the window size. */
            memset0(&raw mut ws);
            ws.ws_col = screen_size_x(&raw mut (*new_wp).base) as u16;
            ws.ws_row = screen_size_y(&raw mut (*new_wp).base) as u16;
            ws.ws_xpixel = ((*w).xpixel * ws.ws_col as u32) as u16;
            ws.ws_ypixel = ((*w).ypixel * ws.ws_row as u32) as u16;

            /* Block signals until fork has completed. */
            sigfillset(&raw mut set);
            sigprocmask(SIG_BLOCK, &raw mut set, &raw mut oldset);

            /* If the command is empty, don't fork a child process. */
            if (*sc).flags & SPAWN_EMPTY != 0 {
                (*new_wp).flags |= window_pane_flags::PANE_EMPTY;
                (*new_wp).base.mode &= !mode_flag::MODE_CURSOR;
                (*new_wp).base.mode |= mode_flag::MODE_CRLF;
                break 'complete;
            }

            /* Fork the new process. */
            (*new_wp).pid = fdforkpty(
                ptm_fd,
                &raw mut (*new_wp).fd,
                (*new_wp).tty.as_mut_ptr(),
                null_mut(),
                &raw mut ws,
            );
            if (*new_wp).pid == -1 {
                *cause = format_nul!("fork failed: {}", _s(strerror(errno!())));
                (*new_wp).fd = -1;
                if !(*sc).flags & SPAWN_RESPAWN != 0 {
                    server_client_remove_pane(new_wp);
                    layout_close_pane(new_wp);
                    window_remove_pane(w, new_wp);
                }
                sigprocmask(SIG_SETMASK, &raw mut oldset, null_mut());
                environ_free(child);
                return null_mut();
            }

            /* In the parent process, everything is done now. */
            if (*new_wp).pid != 0 {
                #[cfg(all(feature = "systemd", feature = "cgroups"))]
                {
                    /*
                     * Move the child process into a new cgroup for systemd-oomd
                     * isolation.
                     */
                    if (systemd_move_pid_to_new_cgroup((*new_wp).pid, cause) < 0) {
                        log_debug!(
                            "{}: moving pane to new cgroup failed: {}",
                            _s(__func__),
                            _s(*cause)
                        );
                        free_(*cause);
                    }
                }
                break 'complete;
            }

            /*
             * Child process. Change to the working directory or home if that
             * fails.
             */
            if chdir((*new_wp).cwd) == 0 {
                environ_set!(child, c"PWD".as_ptr(), 0, "{}", _s((*new_wp).cwd));
            } else if ({
                tmp = find_home();
                !tmp.is_null()
            }) && chdir(tmp) == 0
            {
                environ_set!(child, c"PWD".as_ptr(), 0, "{}", _s(tmp));
            } else if chdir(c"/".as_ptr()) == 0 {
                environ_set!(child, c"PWD".as_ptr(), 0, "/");
            } else {
                fatal(c"chdir failed".as_ptr());
            }

            /*
             * Update terminal escape characters from the session if available and
             * force VERASE to tmux's backspace.
             */
            if tcgetattr(STDIN_FILENO, &raw mut now) != 0 {
                _exit(1);
            }
            if !(*s).tio.is_null() {
                memcpy__(now.c_cc.as_mut_ptr(), (*(*s).tio).c_cc.as_ptr());
            }
            key = options_get_number_(global_options, c"backspace") as u64;
            if key >= 0x7f {
                now.c_cc[VERASE] = b'\x7f';
            } else {
                now.c_cc[VERASE] = key as u8;
            }
            #[cfg(feature = "iutf8")]
            {
                now.c_iflag |= IUTF8;
            }
            if tcsetattr(STDIN_FILENO, TCSANOW, &now) != 0 {
                _exit(1);
            }

            /* Clean up file descriptors and signals and update the environment. */
            proc_clear_signals(server_proc, 1);
            closefrom(STDERR_FILENO + 1);
            sigprocmask(SIG_SETMASK, &raw mut oldset, null_mut());
            log_close();
            environ_push(child);

            /*
             * If given multiple arguments, use execvp(). Copy the arguments to
             * ensure they end in a NULL.
             */
            if (*new_wp).argc != 0 && (*new_wp).argc != 1 {
                argvp = cmd_copy_argv((*new_wp).argc, (*new_wp).argv);
                execvp(*argvp, argvp.cast());
                _exit(1);
            }

            /*
             * If one argument, pass it to $SHELL -c. Otherwise create a login
             * shell.
             */
            cp = strrchr((*new_wp).shell, b'/' as i32);
            if (*new_wp).argc == 1 {
                tmp = *(*new_wp).argv;
                argv0 = if !cp.is_null() && *cp.add(1) != b'\0' as c_char {
                    format_nul!("{}", _s(cp.add(1)))
                } else {
                    format_nul!("{}", _s((*new_wp).shell))
                };
                execl(
                    (*new_wp).shell,
                    argv0,
                    c"-c".as_ptr(),
                    tmp,
                    null_mut::<c_char>(),
                );
                _exit(1);
            }
            argv0 = if !cp.is_null() && *cp.add(1) != b'\0' as c_char {
                format_nul!("-{}", _s(cp.add(1)))
            } else {
                format_nul!("-{}", _s((*new_wp).shell))
            };
            execl((*new_wp).shell, argv0, null_mut::<c_char>());
            _exit(1);
        }

        // complete:
        #[cfg(feature = "utempter")]
        {
            if !(*new_wp).flags.intersects(window_pane_flags::PANE_EMPTY) {
                cp = format_nul!("tmux({}).%{}", std::process::id() as c_long, (*new_wp).id);
                utempter_add_record((*new_wp).fd, cp);
                kill(std::process::id() as i32, SIGCHLD);
                free_(cp);
            }
        }

        (*new_wp).flags &= !window_pane_flags::PANE_EXITED;

        sigprocmask(SIG_SETMASK, &raw mut oldset, null_mut());
        window_pane_set_event(new_wp);

        environ_free(child);

        if (*sc).flags & SPAWN_RESPAWN != 0 {
            return new_wp;
        }
        if (!(*sc).flags & SPAWN_DETACHED != 0) || (*w).active.is_null() {
            if (*sc).flags & SPAWN_NONOTIFY != 0 {
                window_set_active_pane(w, new_wp, 0);
            } else {
                window_set_active_pane(w, new_wp, 1);
            }
        }
        if !(*sc).flags & SPAWN_NONOTIFY != 0 {
            notify_window(c"window-layout-changed", w);
        }

        new_wp
    }
}

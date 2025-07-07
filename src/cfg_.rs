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

use crate::libc::{ENOENT, fclose, fopen, strerror};

use crate::cmd_::cmd_queue::cmdq_get_callback;
use crate::compat::{queue::tailq_first, tree::rb_min};

pub static mut CFG_CLIENT: *mut client = null_mut();

pub static mut CFG_FINISHED: c_int = 0;

static mut CFG_CAUSES: *mut *mut u8 = null_mut();
static mut CFG_NCAUSES: c_uint = 0;
static mut CFG_ITEM: *mut cmdq_item = null_mut();

pub static mut CFG_QUIET: c_int = 1;

pub static mut CFG_FILES: *mut *mut u8 = null_mut();

pub static mut CFG_NFILES: c_uint = 0;

unsafe fn cfg_client_done(_item: *mut cmdq_item, _data: *mut c_void) -> cmd_retval {
    if unsafe { CFG_FINISHED } == 0 {
        cmd_retval::CMD_RETURN_WAIT
    } else {
        cmd_retval::CMD_RETURN_NORMAL
    }
}

unsafe fn cfg_done(_item: *mut cmdq_item, _data: *mut c_void) -> cmd_retval {
    unsafe {
        if CFG_FINISHED != 0 {
            return cmd_retval::CMD_RETURN_NORMAL;
        }
        CFG_FINISHED = 1;

        cfg_show_causes(null_mut());

        if !CFG_ITEM.is_null() {
            cmdq_continue(CFG_ITEM);
        }

        status_prompt_load_history();

        cmd_retval::CMD_RETURN_NORMAL
    }
}

pub unsafe fn start_cfg() {
    let c: *mut client;
    let mut i: u32;
    let mut flags: cmd_parse_input_flags = cmd_parse_input_flags::empty();

    //
    // Configuration files are loaded without a client, so commands are run
    // in the global queue with item->client NULL.
    //
    // However, we must block the initial client (but just the initial
    // client) so that its command runs after the configuration is loaded.
    // Because start_cfg() is called so early, we can be sure the client's
    // command queue is currently empty and our callback will be at the
    // front - we need to get in before MSG_COMMAND.

    unsafe {
        c = tailq_first(&raw mut CLIENTS);
        CFG_CLIENT = c;
        if !c.is_null() {
            CFG_ITEM = cmdq_get_callback!(cfg_client_done, null_mut()).as_ptr();
            cmdq_append(c, CFG_ITEM);
        }

        if CFG_QUIET != 0 {
            flags = cmd_parse_input_flags::CMD_PARSE_QUIET;
        }

        i = 0;
        while i < CFG_NFILES {
            load_cfg(
                cstr_to_str(*CFG_FILES.add(i as usize)),
                c,
                null_mut(),
                null_mut(),
                flags,
                null_mut(),
            );
            i += 1;
        }

        cmdq_append(
            null_mut(),
            cmdq_get_callback!(cfg_done, null_mut()).as_ptr(),
        );
    }
}

pub unsafe fn load_cfg(
    path: &str,
    c: *mut client,
    item: *mut cmdq_item,
    current: *mut cmd_find_state,
    flags: cmd_parse_input_flags,
    new_item: *mut *mut cmdq_item,
) -> c_int {
    unsafe {
        if !new_item.is_null() {
            *new_item = null_mut();
        }

        log_debug!("loading {}", path);
        let mut f = match std::fs::OpenOptions::new().read(true).open(path) {
            Ok(f) => std::io::BufReader::new(f),
            Err(err) => {
                let code = err.raw_os_error().unwrap();

                if code == ENOENT && flags.intersects(cmd_parse_input_flags::CMD_PARSE_QUIET) {
                    return 0;
                }
                cfg_add_cause!("{}: {}", path, _s(strerror(code)));
                return -1;
            }
        };

        let mut pi: cmd_parse_input = zeroed();
        pi.flags = flags.into();
        pi.file = Some(path);
        pi.line = AtomicU32::new(1);
        pi.item = item;
        pi.c = c;

        let pr = cmd_parse_from_file(&mut f, Some(&pi));
        drop(f);
        let cmdlist = match pr {
            Err(error) => {
                cfg_add_cause!("{}", _s(error));
                free_(error);
                return -1;
            }
            Ok(cmdlist) => cmdlist,
        };
        if flags.intersects(cmd_parse_input_flags::CMD_PARSE_PARSEONLY) {
            cmd_list_free(cmdlist);
            return 0;
        }

        let state = if !item.is_null() {
            cmdq_copy_state(cmdq_get_state(item), current)
        } else {
            cmdq_new_state(null_mut(), null_mut(), cmdq_state_flags::empty())
        };
        cmdq_add_format!(state, c!("current_file"), "{}", pi.file.as_ref().unwrap());

        let mut new_item0 = cmdq_get_command(cmdlist, state);
        if !item.is_null() {
            new_item0 = cmdq_insert_after(item, new_item0);
        } else {
            new_item0 = cmdq_append(null_mut(), new_item0);
        }
        cmd_list_free(cmdlist);
        cmdq_free_state(state);

        if !new_item.is_null() {
            *new_item = new_item0;
        }

        0
    }
}

pub unsafe fn load_cfg_from_buffer(
    buf: &[u8],
    path: &str,
    c: *mut client,
    item: *mut cmdq_item,
    current: *mut cmd_find_state,
    flags: cmd_parse_input_flags,
    new_item: *mut *mut cmdq_item,
) -> c_int {
    unsafe {
        if !new_item.is_null() {
            *new_item = null_mut();
        }

        log_debug!("loading {}", path);

        let mut pi: cmd_parse_input = zeroed();
        pi.flags = flags.into();
        pi.file = Some(path);
        pi.line = AtomicU32::new(1);
        pi.item = item;
        pi.c = c;

        let cmdlist = match cmd_parse_from_buffer(buf, Some(&pi)) {
            Err(error) => {
                cfg_add_cause!("{}", _s(error));
                free_(error);
                return -1;
            }
            Ok(cmdlist) => cmdlist,
        };

        if flags.intersects(cmd_parse_input_flags::CMD_PARSE_PARSEONLY) {
            cmd_list_free(cmdlist);
            return 0;
        }

        let state = if !item.is_null() {
            cmdq_copy_state(cmdq_get_state(item), current)
        } else {
            cmdq_new_state(null_mut(), null_mut(), cmdq_state_flags::empty())
        };
        cmdq_add_format!(state, c!("current_file"), "{}", pi.file.as_ref().unwrap());

        let mut new_item0 = cmdq_get_command(cmdlist, state);
        if !item.is_null() {
            new_item0 = cmdq_insert_after(item, new_item0);
        } else {
            new_item0 = cmdq_append(null_mut(), new_item0);
        }
        cmd_list_free(cmdlist);
        cmdq_free_state(state);

        if !new_item.is_null() {
            *new_item = new_item0;
        }
        0
    }
}

macro_rules! cfg_add_cause {
   ($fmt:literal $(, $args:expr)* $(,)?) => {
        crate::cfg_::cfg_add_cause_(format_args!($fmt $(, $args)*))
    };
}
pub(crate) use cfg_add_cause;

pub unsafe fn cfg_add_cause_(args: std::fmt::Arguments) {
    unsafe {
        let mut msg = args.to_string();
        msg.push('\0');
        let msg = msg.leak();

        CFG_NCAUSES += 1;
        CFG_CAUSES = xreallocarray_::<*mut u8>(CFG_CAUSES, CFG_NCAUSES as usize).as_ptr();
        *CFG_CAUSES.add(CFG_NCAUSES as usize - 1) = msg.as_mut_ptr().cast();
    }
}

pub unsafe fn cfg_print_causes(item: *mut cmdq_item) {
    unsafe {
        for i in 0..CFG_NCAUSES {
            cmdq_print!(item, "{}", _s(*CFG_CAUSES.add(i as usize)));
            free_(*CFG_CAUSES.add(i as usize));
        }

        free_(CFG_CAUSES);
        CFG_CAUSES = null_mut();
        CFG_NCAUSES = 0;
    }
}

pub unsafe fn cfg_show_causes(mut s: *mut session) {
    unsafe {
        'out: {
            let c = tailq_first(&raw mut CLIENTS);

            if CFG_NCAUSES == 0 {
                return;
            }

            if !c.is_null() && (*c).flags.intersects(client_flag::CONTROL) {
                for i in 0..CFG_NCAUSES {
                    control_write!(c, "%config-error {}", _s(*CFG_CAUSES.add(i as usize)),);
                    free_(*CFG_CAUSES.add(i as usize));
                }
                break 'out;
            }

            if s.is_null() {
                if !c.is_null() && !(*c).session.is_null() {
                    s = (*c).session;
                } else {
                    s = rb_min(&raw mut SESSIONS);
                }
            }
            if s.is_null() || (*s).attached == 0 {
                return;
            }
            let wp = (*(*(*s).curw).window).active;

            let wme: *mut window_mode_entry = tailq_first(&raw mut (*wp).modes);
            if wme.is_null() || (*wme).mode != &raw const WINDOW_VIEW_MODE {
                window_pane_set_mode(
                    wp,
                    null_mut(),
                    &raw const WINDOW_VIEW_MODE,
                    null_mut(),
                    null_mut(),
                );
            }
            for i in 0..CFG_NCAUSES {
                window_copy_add!(wp, 0, "{}", _s(*CFG_CAUSES.add(i as usize)));
                free(*CFG_CAUSES.add(i as usize) as _);
            }
            break 'out;
        }
        // out:
        free_(CFG_CAUSES);
        CFG_CAUSES = null_mut();
        CFG_NCAUSES = 0;
    }
}

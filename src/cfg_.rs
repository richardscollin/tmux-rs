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
use crate::cmd_::cmd_queue::cmdq_get_callback;
use crate::libc::{ENOENT, strerror};
use crate::*;

pub static mut CFG_CLIENT: *mut client = null_mut();

pub static CFG_FINISHED: AtomicBool = AtomicBool::new(false);

static CFG_CAUSES: Mutex<Vec<CString>> = Mutex::new(Vec::new());

static mut CFG_ITEM: *mut cmdq_item = null_mut();

pub static CFG_QUIET: AtomicBool = AtomicBool::new(true);

pub static CFG_FILES: Mutex<Vec<CString>> = Mutex::new(Vec::new());

fn cfg_client_done(_item: *mut cmdq_item, _data: *mut c_void) -> cmd_retval {
    if !CFG_FINISHED.load(atomic::Ordering::Acquire) {
        cmd_retval::CMD_RETURN_WAIT
    } else {
        cmd_retval::CMD_RETURN_NORMAL
    }
}

unsafe fn cfg_done(_item: *mut cmdq_item, _data: *mut c_void) -> cmd_retval {
    unsafe {
        if CFG_FINISHED.load(atomic::Ordering::Acquire) {
            return cmd_retval::CMD_RETURN_NORMAL;
        }
        CFG_FINISHED.store(true, atomic::Ordering::Release);

        cfg_show_causes(null_mut());

        if !CFG_ITEM.is_null() {
            cmdq_continue(CFG_ITEM);
        }

        status_prompt_load_history();

        cmd_retval::CMD_RETURN_NORMAL
    }
}

pub unsafe fn start_cfg() {
    // Configuration files are loaded without a client, so commands are run
    // in the global queue with item->client NULL.
    //
    // However, we must block the initial client (but just the initial
    // client) so that its command runs after the configuration is loaded.
    // Because start_cfg() is called so early, we can be sure the client's
    // command queue is currently empty and our callback will be at the
    // front - we need to get in before MSG_COMMAND.

    unsafe {
        let c = tailq_first(&raw mut CLIENTS);
        CFG_CLIENT = c;
        if !c.is_null() {
            CFG_ITEM = cmdq_get_callback!(cfg_client_done, null_mut()).as_ptr();
            cmdq_append(c, CFG_ITEM);
        }

        let flags: cmd_parse_input_flags = if CFG_QUIET.load(atomic::Ordering::Relaxed) {
            cmd_parse_input_flags::CMD_PARSE_QUIET
        } else {
            cmd_parse_input_flags::empty()
        };

        for file in CFG_FILES.lock().unwrap().iter() {
            load_cfg(
                file.to_str().expect("cfg file isn't valid utf8"),
                c,
                null_mut(),
                null_mut(),
                flags,
                null_mut(),
            );
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

pub fn cfg_add_cause_(args: std::fmt::Arguments) {
    CFG_CAUSES
        .lock()
        .unwrap()
        .push(CString::new(args.to_string()).unwrap());
}

pub unsafe fn cfg_print_causes(item: *mut cmdq_item) {
    for cause in CFG_CAUSES.lock().unwrap().drain(..) {
        unsafe {
            cmdq_print!(item, "{}", cause.to_string_lossy());
        }
    }
}

pub unsafe fn cfg_show_causes(mut s: *mut session) {
    unsafe {
        let c = tailq_first(&raw mut CLIENTS);

        if CFG_CAUSES.lock().unwrap().is_empty() {
            return;
        }

        if !c.is_null() && (*c).flags.intersects(client_flag::CONTROL) {
            for cause in CFG_CAUSES.lock().unwrap().drain(..) {
                control_write!(c, "%config-error {}", cause.to_string_lossy());
            }
            return;
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
        for cause in CFG_CAUSES.lock().unwrap().drain(..) {
            window_copy_add!(wp, 0, "{}", cause.to_string_lossy());
        }
    }
}

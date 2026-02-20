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
use std::io::BufWriter;
use std::{
    fs::File,
    io::{LineWriter, Write},
    sync::atomic::{AtomicI32, Ordering},
};

use crate::compat::{stravis, vis_flags};
use crate::event_::event_set_log_callback;
use crate::*;

macro_rules! log_debug {
    ($($arg:tt)*) => {$crate::log::log_debug_rs(format_args!($($arg)*))};
}
pub(crate) use log_debug;

// can't use File because it's open before fork which causes issues with how file works
static LOG_FILE: Mutex<Option<LineWriter<File>>> = Mutex::new(None);
static LOG_LEVEL: AtomicI32 = AtomicI32::new(0);

const DEFAULT_ORDERING: Ordering = Ordering::SeqCst;

unsafe extern "C-unwind" fn log_event_cb(_severity: c_int, msg: *const u8) {
    unsafe { log_debug!("{}", _s(msg)) }
}

pub fn log_add_level() {
    LOG_LEVEL.fetch_add(1, DEFAULT_ORDERING);
}

pub fn log_get_level() -> i32 {
    LOG_LEVEL.load(DEFAULT_ORDERING)
}

pub fn log_open(name: &CStr) {
    let level = LOG_LEVEL.load(DEFAULT_ORDERING);
    let pid = std::process::id();
    let filename = format!("tmux-{}-{}.log", name.to_str().unwrap(), pid);

    if level == 0 {
        return;
    }

    log_close();
    let Ok(file) = std::fs::File::options()
        .read(false)
        .append(true)
        .create(true)
        .open(&filename)
    else {
        return;
    };

    *LOG_FILE.lock().unwrap() = Some(LineWriter::new(file));
    unsafe { event_set_log_callback(Some(log_event_cb)) };
}

pub fn log_toggle(name: &CStr) {
    if LOG_LEVEL.fetch_xor(1, DEFAULT_ORDERING) == 0 {
        log_open(name);
        log_debug!("log opened");
    } else {
        log_debug!("log closed");
        log_close();
    }
}

pub fn log_close() {
    // If we drop the file when it's already closed it will panic in debug mode.
    // Because of this and our use of fork, extra care has to be made when closing the file.
    // see std::sys::pal::unix::fs::debug_assert_fd_is_open;
    if let Some(mut old_handle) = LOG_FILE.lock().unwrap().take() {
        let _flush_err = old_handle.flush(); // TODO
        match old_handle.into_inner() {
            Ok(file) => unsafe {
                #[cfg(unix)]
                {
                    use std::os::fd::AsRawFd;
                    libc::close(file.as_raw_fd());
                }
                #[cfg(windows)]
                drop(file.sync_all());
                std::mem::forget(file);
            },
            Err(err) => {
                let lw = err.into_inner();
                // TODO this is invalid, and compiler version dependent, but prevents a memory leak
                // need a way to properly get out the file and drop the buffer
                unsafe {
                    let bw = std::mem::transmute::<
                        std::io::LineWriter<std::fs::File>,
                        BufWriter<File>,
                    >(lw);
                    let (file, _) = bw.into_parts();
                    std::mem::forget(file);
                }
            }
        }

        unsafe {
            event_set_log_callback(None);
        }
    }
}

#[track_caller]
pub fn log_debug_rs(args: std::fmt::Arguments) {
    if LOG_FILE.lock().unwrap().is_none() {
        return;
    }
    log_vwrite_rs(args, "");
}

#[track_caller]
fn log_vwrite_rs(args: std::fmt::Arguments, prefix: &str) {
    unsafe {
        if LOG_FILE.lock().unwrap().is_none() {
            return;
        }

        let msg = CString::new(format!("{args}")).unwrap();
        let mut out = null_mut();
        if stravis(
            &mut out,
            msg.as_ptr().cast(),
            vis_flags::VIS_OCTAL | vis_flags::VIS_CSTYLE | vis_flags::VIS_TAB | vis_flags::VIS_NL,
        ) == -1
        {
            return;
        }
        let duration = std::time::SystemTime::now()
            .duration_since(std::time::SystemTime::UNIX_EPOCH)
            .unwrap_or_default();
        let secs = duration.as_secs();
        let micros = duration.subsec_micros();

        let str_out = CStr::from_ptr(out.cast()).to_string_lossy();
        if let Some(f) = LOG_FILE.lock().unwrap().as_mut() {
            let location = std::panic::Location::caller();
            let file = location.file();
            let line = location.line();
            let _ = f.write_fmt(format_args!(
                "{secs}.{micros:06} {file}:{line} {prefix}{str_out}\n"
            ));
        }

        crate::free_(out);
    }
}

pub fn fatal(msg: &str) -> ! {
    let os_error = std::io::Error::last_os_error();
    let error_msg = os_error.to_string();

    let prefix = format!("fatal: {error_msg}: ");

    log_vwrite_rs(format_args!("{msg}"), &prefix);

    std::process::exit(1)
}

macro_rules! fatalx_ {
   ($fmt:literal $(, $args:expr)* $(,)?) => {
        crate::log::fatalx_c(format_args!($fmt $(, $args)*))
    };
}
pub(crate) use fatalx_;
pub fn fatalx_c(args: std::fmt::Arguments) -> ! {
    log_vwrite_rs(args, "fatal: ");
    std::process::exit(1)
}

#[track_caller]
pub fn fatalx(msg: &str) -> ! {
    let location = std::panic::Location::caller();
    let file = location.file();
    let line = location.line();

    log_vwrite_rs(format_args!("{file}:{line} {msg}"), "fatal: ");
    std::process::exit(1)
}

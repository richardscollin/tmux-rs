use core::ffi::c_int;

#[cfg(not(target_os = "windows"))]
use crate::libc;

pub extern "C-unwind" fn getptmfd() -> c_int {
    c_int::MAX
}

#[cfg(not(target_os = "windows"))]
pub unsafe fn fdforkpty(
    _ptmfd: c_int,
    master: *mut c_int,
    name: *mut u8,
    tio: *mut libc::termios,
    ws: *mut libc::winsize,
) -> libc::pid_t {
    unsafe { libc::forkpty(master, name.cast(), tio, ws) }
}


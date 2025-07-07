use core::ffi::c_int;
use libc::{pid_t, termios, winsize};

pub extern "C" fn getptmfd() -> c_int {
    c_int::MAX
}

pub unsafe fn fdforkpty(
    _ptmfd: c_int,
    master: *mut c_int,
    name: *mut u8,
    tio: *mut termios,
    ws: *mut winsize,
) -> pid_t {
    unsafe { ::libc::forkpty(master, name.cast(), tio, ws) }
}

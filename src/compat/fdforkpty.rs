use core::ffi::c_int;

pub extern "C" fn getptmfd() -> c_int {
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
    unsafe { ::libc::forkpty(master, name.cast(), tio, ws) }
}

#[cfg(target_os = "windows")]
pub unsafe fn fdforkpty(
    _ptmfd: c_int,
    _master: *mut c_int,
    _name: *mut u8,
    _tio: *mut crate::libc::termios,
    _ws: *mut crate::libc::winsize,
) -> crate::libc::pid_t {
    todo!("fdforkpty not available on Windows")
}

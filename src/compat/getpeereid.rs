#[cfg(not(target_os = "windows"))]
pub unsafe fn getpeereid(_s: i32, uid: *mut libc::uid_t, gid: *mut libc::gid_t) -> i32 {
    unsafe {
        *uid = libc::geteuid();
        *gid = libc::getegid();
    }
    0
}

#[cfg(target_os = "windows")]
pub unsafe fn getpeereid(
    _s: i32,
    uid: *mut crate::libc::uid_t,
    gid: *mut crate::libc::gid_t,
) -> i32 {
    // On Windows, always local — return current user's uid/gid
    unsafe {
        *uid = crate::libc::geteuid();
        *gid = crate::libc::getegid();
    }
    0
}

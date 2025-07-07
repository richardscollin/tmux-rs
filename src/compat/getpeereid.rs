pub unsafe fn getpeereid(_s: i32, uid: *mut libc::uid_t, gid: *mut libc::gid_t) -> i32 {
    unsafe {
        *uid = libc::geteuid();
        *gid = libc::getegid();
    }
    0
}

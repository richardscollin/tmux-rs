pub fn systemd_create_socket(flags: i32, cause: *mut *mut u8) -> i32 {
    unsafe extern "C" {
        fn systemd_create_socket(flags: i32, cause: *mut *mut u8) -> i32;
    }
    unsafe { systemd_create_socket(flags, cause) }
}

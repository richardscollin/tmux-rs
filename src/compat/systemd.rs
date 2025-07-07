pub fn systemd_create_socket(flags: i32, cause: *mut *mut u8) -> i32 {
    unsafe extern "C" {
        #[link(name = "systemd_create_socket")]
        fn systemd_create_socket_c(flags: i32, cause: *mut *mut core::ffi::c_char) -> i32;
    }
    unsafe { systemd_create_socket_c(flags, cause.cast()) }
}

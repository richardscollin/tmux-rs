pub mod b64;
pub mod closefrom;
pub mod fdforkpty;
pub mod getdtablecount;
pub mod getopt;
pub mod getprogname;
pub mod imsg;
pub mod imsg_buffer;
pub mod queue;
pub mod reallocarray;
pub mod recallocarray;
pub mod tree;

mod freezero;
mod getpeereid;
mod setproctitle;
mod strlcat;
mod strlcpy;
mod strtonum;
mod unvis;
mod vis;

pub use freezero::freezero;
pub use getpeereid::getpeereid;
pub use setproctitle::setproctitle_;
pub use strlcat::{strlcat, strlcat_};
pub use strlcpy::strlcpy;
pub use strtonum::{strtonum, strtonum_};
pub use unvis::strunvis;
pub use vis::*;

// #[rustfmt::skip]
// unsafe extern "C" {
//     pub static mut optreset: c_int;
//     pub static mut optarg: *mut c_char;
//     pub static mut optind: c_int;
//     pub fn getopt(___argc: c_int, ___argv: *const *mut c_char, __shortopts: *const c_char) -> c_int;
//     pub fn bsd_getopt(argc: c_int, argv: *const *mut c_char, shortopts: *const c_char) -> c_int;
// }

/// Portable trait to extract a raw file descriptor (`c_int`) from a `File`.
/// On unix, delegates to `std::os::unix::io::IntoRawFd`.
/// On windows, uses `IntoRawHandle` and casts to `c_int`.
pub trait FileIntoRawFd {
    fn into_fd(self) -> core::ffi::c_int;
}

impl FileIntoRawFd for std::fs::File {
    #[cfg(unix)]
    fn into_fd(self) -> core::ffi::c_int {
        std::os::unix::io::IntoRawFd::into_raw_fd(self)
    }

    #[cfg(windows)]
    fn into_fd(self) -> core::ffi::c_int {
        std::os::windows::io::IntoRawHandle::into_raw_handle(self) as core::ffi::c_int
    }
}

pub const HOST_NAME_MAX: usize = 255;
pub const WAIT_ANY: crate::libc::pid_t = -1;
pub const ACCESSPERMS: crate::libc::mode_t =
    crate::libc::S_IRWXU | crate::libc::S_IRWXG | crate::libc::S_IRWXO;

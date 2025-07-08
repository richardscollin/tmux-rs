use core::ffi::{c_char, c_int};

pub mod b64;
pub mod fdforkpty;
pub mod getdtablecount;
pub mod getopt;
pub mod getprogname;
pub mod imsg;
pub mod imsg_buffer;
pub mod queue;
pub mod systemd;
pub mod tree;

mod closefrom;
mod freezero;
mod getpeereid;
mod reallocarray;
mod recallocarray;
mod setproctitle;
mod strlcat;
mod strlcpy;
mod strtonum;
mod unvis;
mod vis;

pub use closefrom::closefrom;
pub use freezero::freezero;
pub use getpeereid::getpeereid;
pub(crate) use reallocarray::reallocarray;
pub use recallocarray::recallocarray;
pub use setproctitle::setproctitle_;
pub use strlcat::strlcat;
pub use strlcpy::strlcpy;
pub use strtonum::{strtonum, strtonum_};
pub use systemd::systemd_create_socket;
pub use unvis::strunvis;
pub use vis::*;

pub(crate) use queue::{TAILQ_HEAD_INITIALIZER, impl_tailq_entry, tailq_insert_head};
pub(crate) use tree::RB_GENERATE;

// #[rustfmt::skip]
// unsafe extern "C" {
//     pub static mut optreset: c_int;
//     pub static mut optarg: *mut c_char;
//     pub static mut optind: c_int;
//     pub fn getopt(___argc: c_int, ___argv: *const *mut c_char, __shortopts: *const c_char) -> c_int;
//     pub fn bsd_getopt(argc: c_int, argv: *const *mut c_char, shortopts: *const c_char) -> c_int;
// }

pub const HOST_NAME_MAX: usize = 255;

pub const WAIT_ANY: libc::pid_t = -1;

pub const ACCESSPERMS: libc::mode_t = libc::S_IRWXU | libc::S_IRWXG | libc::S_IRWXO;

// #define S_ISDIR(mode)  (((mode) & S_IFMT) == S_IFDIR)
// TODO move this to a better spot
#[allow(non_snake_case)]
#[inline]
pub fn S_ISDIR(mode: libc::mode_t) -> bool {
    mode & libc::S_IFMT == libc::S_IFDIR
}

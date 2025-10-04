pub mod b64;
pub mod fdforkpty;
pub mod getdtablecount;
pub mod getopt;
pub mod getprogname;
pub mod imsg;
pub mod imsg_buffer;
pub mod queue;
pub mod reallocarray;
pub mod recallocarray;
pub mod systemd;
pub mod tree;

mod closefrom;
mod freezero;
mod getpeereid;
mod setproctitle;
mod strlcat;
mod strlcpy;
mod strtonum;
mod unvis;
mod vis;

pub use closefrom::closefrom;
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

pub const HOST_NAME_MAX: usize = 255;

pub const WAIT_ANY: libc::pid_t = -1;

pub const ACCESSPERMS: libc::mode_t = libc::S_IRWXU | libc::S_IRWXG | libc::S_IRWXO;

// #define S_ISDIR(mode)  (((mode) & S_IFMT) == S_IFDIR)
// TODO move this to a better spot
#[expect(non_snake_case)]
#[inline]
pub fn S_ISDIR(mode: libc::mode_t) -> bool {
    mode & libc::S_IFMT == libc::S_IFDIR
}

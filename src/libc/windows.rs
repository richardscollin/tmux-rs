//! Windows shims for Unix types, constants, and functions that don't exist in the libc crate on Windows.
//! These are used to allow the codebase to compile on Windows. Most function implementations
//! use todo!() since they have no Windows equivalent.
#![allow(nonstandard_style, non_camel_case_types, non_upper_case_globals, dead_code, clippy::missing_safety_doc)]

// Re-export everything from ::libc first, then our definitions shadow what we override
pub use ::libc::*;

use core::ffi::{c_char, c_int, c_uint, c_void};

// ============================================================
// Types (not in ::libc on Windows)
// ============================================================

pub type pid_t = i32;
pub type uid_t = u32;
pub type gid_t = u32;
pub type mode_t = u16;
pub type suseconds_t = i32;
pub type cc_t = u8;
pub type speed_t = u32;
pub type tcflag_t = u32;
pub type sigset_t = u64;
pub type socklen_t = i32;
pub type sa_family_t = u16;
pub type clockid_t = i32;
pub type nl_item = i32;
pub type nfds_t = u64;

pub const NCCS: usize = 32;

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct termios {
    pub c_iflag: tcflag_t,
    pub c_oflag: tcflag_t,
    pub c_cflag: tcflag_t,
    pub c_lflag: tcflag_t,
    pub c_line: cc_t,
    pub c_cc: [cc_t; NCCS],
    pub c_ispeed: speed_t,
    pub c_ospeed: speed_t,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct winsize {
    pub ws_row: u16,
    pub ws_col: u16,
    pub ws_xpixel: u16,
    pub ws_ypixel: u16,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct passwd {
    pub pw_name: *mut c_char,
    pub pw_passwd: *mut c_char,
    pub pw_uid: uid_t,
    pub pw_gid: gid_t,
    pub pw_gecos: *mut c_char,
    pub pw_dir: *mut c_char,
    pub pw_shell: *mut c_char,
}

pub const _UTSNAME_LENGTH: usize = 65;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct utsname {
    pub sysname: [c_char; _UTSNAME_LENGTH],
    pub nodename: [c_char; _UTSNAME_LENGTH],
    pub release: [c_char; _UTSNAME_LENGTH],
    pub version: [c_char; _UTSNAME_LENGTH],
    pub machine: [c_char; _UTSNAME_LENGTH],
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct regex_t {
    _opaque: [u8; 64],
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct regmatch_t {
    pub rm_so: i32,
    pub rm_eo: i32,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct glob_t {
    pub gl_pathc: usize,
    pub gl_pathv: *mut *mut c_char,
    pub gl_offs: usize,
}

pub const UNIX_PATH_MAX: usize = 108;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct sockaddr_un {
    pub sun_family: sa_family_t,
    pub sun_path: [c_char; UNIX_PATH_MAX],
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct sockaddr_storage {
    pub ss_family: sa_family_t,
    _padding: [u8; 126],
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct iovec {
    pub iov_base: *mut c_void,
    pub iov_len: usize,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct msghdr {
    pub msg_name: *mut c_void,
    pub msg_namelen: socklen_t,
    pub msg_iov: *mut iovec,
    pub msg_iovlen: usize,
    pub msg_control: *mut c_void,
    pub msg_controllen: usize,
    pub msg_flags: c_int,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct cmsghdr {
    pub cmsg_len: usize,
    pub cmsg_level: c_int,
    pub cmsg_type: c_int,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct sigaction {
    pub sa_sigaction: usize,
    pub sa_mask: sigset_t,
    pub sa_flags: c_int,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct pollfd {
    pub fd: c_int,
    pub events: i16,
    pub revents: i16,
}

// ============================================================
// Signal constants
// ============================================================

pub const SIGHUP: c_int = 1;
pub const SIGQUIT: c_int = 3;
pub const SIGPIPE: c_int = 13;
pub const SIGCHLD: c_int = 17;
pub const SIGCONT: c_int = 18;
pub const SIGTSTP: c_int = 20;
pub const SIGTTIN: c_int = 21;
pub const SIGTTOU: c_int = 22;
pub const SIGUSR1: c_int = 10;
pub const SIGUSR2: c_int = 12;
pub const SIGWINCH: c_int = 28;

pub const SA_RESTART: c_int = 0x10000000;
pub const SIG_BLOCK: c_int = 0;
pub const SIG_SETMASK: c_int = 2;
pub const SIG_DFL: usize = 0;
pub const SIG_IGN: usize = 1;

// ============================================================
// Socket constants
// ============================================================

pub const AF_UNIX: c_int = 1;
pub const PF_UNSPEC: c_int = 0;
pub const SHUT_WR: c_int = 1;
pub const SOCK_STREAM: c_int = 1;
pub const SCM_RIGHTS: c_int = 1;
pub const SOL_SOCKET: c_int = 1;

// ============================================================
// File constants
// ============================================================

pub const O_NONBLOCK: c_int = 0x800;
pub const X_OK: c_int = 1;
pub const F_GETFL: c_int = 3;
pub const F_SETFL: c_int = 4;
pub const F_SETFD: c_int = 2;
pub const FD_CLOEXEC: c_int = 1;

pub const S_IRUSR: mode_t = 0o400;
pub const S_IWUSR: mode_t = 0o200;
pub const S_IXUSR: mode_t = 0o100;
pub const S_IRWXU: mode_t = 0o700;
pub const S_IRGRP: mode_t = 0o040;
pub const S_IWGRP: mode_t = 0o020;
pub const S_IXGRP: mode_t = 0o010;
pub const S_IRWXG: mode_t = 0o070;
pub const S_IROTH: mode_t = 0o004;
pub const S_IWOTH: mode_t = 0o002;
pub const S_IXOTH: mode_t = 0o001;
pub const S_IRWXO: mode_t = 0o007;

// ============================================================
// Terminal/ioctl constants
// ============================================================

pub const TIOCSWINSZ: u64 = 0x5414;
pub const TIOCGWINSZ: u64 = 0x5413;
pub const TIOCGSID: u64 = 0x5429;
pub const TCSANOW: c_int = 0;
pub const TCSAFLUSH: c_int = 2;
pub const TCOFLUSH: c_int = 1;
pub const FIONREAD: u64 = 0x541B;

pub const VERASE: usize = 2;
pub const VMIN: usize = 6;
pub const VTIME: usize = 5;

pub const IXON: tcflag_t = 0o002000;
pub const IXOFF: tcflag_t = 0o010000;
pub const IXANY: tcflag_t = 0o004000;
pub const ICRNL: tcflag_t = 0o000400;
pub const INLCR: tcflag_t = 0o000100;
pub const IGNCR: tcflag_t = 0o000200;
pub const IMAXBEL: tcflag_t = 0o020000;
pub const ISTRIP: tcflag_t = 0o000040;
pub const IGNBRK: tcflag_t = 0o000001;
pub const IUTF8: tcflag_t = 0o040000;

pub const OPOST: tcflag_t = 0o000001;
pub const ONLCR: tcflag_t = 0o000004;
pub const OCRNL: tcflag_t = 0o000010;
pub const ONLRET: tcflag_t = 0o000040;

pub const IEXTEN: tcflag_t = 0o100000;
pub const ICANON: tcflag_t = 0o000002;
pub const ECHO: tcflag_t = 0o000010;
pub const ECHOE: tcflag_t = 0o000020;
pub const ECHONL: tcflag_t = 0o000100;
pub const ECHOCTL: tcflag_t = 0o001000;
pub const ECHOPRT: tcflag_t = 0o002000;
pub const ECHOKE: tcflag_t = 0o004000;
pub const ISIG: tcflag_t = 0o000001;

pub const CREAD: tcflag_t = 0o000200;
pub const CS8: tcflag_t = 0o000060;
pub const HUPCL: tcflag_t = 0o002000;

pub const _POSIX_VDISABLE: cc_t = 0;

// ============================================================
// Wait constants and macros
// ============================================================

pub const WNOHANG: c_int = 1;
pub const WUNTRACED: c_int = 2;

#[inline]
pub fn WIFEXITED(status: c_int) -> bool {
    (status & 0x7f) == 0
}
#[inline]
pub fn WIFSIGNALED(status: c_int) -> bool {
    ((status & 0x7f) + 1) as i8 >= 2
}
#[inline]
pub fn WIFSTOPPED(status: c_int) -> bool {
    (status & 0xff) == 0x7f
}
#[inline]
pub fn WEXITSTATUS(status: c_int) -> c_int {
    (status >> 8) & 0xff
}
#[inline]
pub fn WTERMSIG(status: c_int) -> c_int {
    status & 0x7f
}
#[inline]
pub fn WSTOPSIG(status: c_int) -> c_int {
    (status >> 8) & 0xff
}

// ============================================================
// Regex constants
// ============================================================

pub const REG_EXTENDED: c_int = 1;
pub const REG_ICASE: c_int = 2;
pub const REG_NOSUB: c_int = 8;
pub const REG_NOTBOL: c_int = 1;

// ============================================================
// Glob constants
// ============================================================

pub const GLOB_NOMATCH: c_int = 3;
pub const GLOB_NOSPACE: c_int = 1;

// ============================================================
// Misc constants
// ============================================================

pub const FNM_CASEFOLD: c_int = 16;
pub const CODESET: nl_item = 14;
pub const CLOCK_MONOTONIC: clockid_t = 1;
pub const CLOCK_REALTIME: clockid_t = 0;
pub const _SC_MB_LEN_MAX: c_int = 6;
pub const PATH_DEVNULL: &str = "NUL";

pub const PR_SET_NAME: c_int = 15;

// ============================================================
// CMSG macros (const fn shims)
// ============================================================

pub const fn CMSG_ALIGN(len: usize) -> usize {
    (len + size_of::<usize>() - 1) & !(size_of::<usize>() - 1)
}
pub const fn CMSG_SPACE(len: u32) -> u32 {
    (CMSG_ALIGN(size_of::<cmsghdr>()) + CMSG_ALIGN(len as usize)) as u32
}
pub const fn CMSG_LEN(len: u32) -> u32 {
    (CMSG_ALIGN(size_of::<cmsghdr>()) + len as usize) as u32
}
pub unsafe fn CMSG_DATA(cmsg: *const cmsghdr) -> *mut u8 {
    unsafe { (cmsg as *mut u8).add(CMSG_ALIGN(size_of::<cmsghdr>())) }
}
pub unsafe fn CMSG_FIRSTHDR(msg: *const msghdr) -> *mut cmsghdr {
    unsafe {
        if (*msg).msg_controllen >= size_of::<cmsghdr>() {
            (*msg).msg_control as *mut cmsghdr
        } else {
            core::ptr::null_mut()
        }
    }
}
pub unsafe fn CMSG_NXTHDR(msg: *const msghdr, cmsg: *const cmsghdr) -> *mut cmsghdr {
    unsafe {
        let next = (cmsg as *const u8).add(CMSG_ALIGN((*cmsg).cmsg_len));
        let end = ((*msg).msg_control as *const u8).add((*msg).msg_controllen);
        if next.add(size_of::<cmsghdr>()) > end {
            core::ptr::null_mut()
        } else {
            next as *mut cmsghdr
        }
    }
}

// ============================================================
// Function shims - variadic C functions (extern declarations)
// ============================================================

unsafe extern "C" {
    pub fn sscanf(s: *const c_char, format: *const c_char, ...) -> c_int;
    pub fn snprintf(s: *mut c_char, n: usize, format: *const c_char, ...) -> c_int;
    pub fn fcntl(fd: c_int, cmd: c_int, ...) -> c_int;
    pub fn ioctl(fd: c_int, request: u64, ...) -> c_int;
    pub fn execl(path: *const c_char, arg: *const c_char, ...) -> c_int;
    pub fn prctl(option: c_int, ...) -> c_int;
}

// ============================================================
// Function shims - todo! stubs for Unix-only functions
// ============================================================

pub unsafe fn fnmatch(_pattern: *const u8, _name: *const u8, _flags: c_int) -> c_int {
    todo!("fnmatch not available on Windows")
}

pub unsafe fn gethostname(_name: *mut u8, _len: usize) -> c_int {
    todo!("gethostname not available on Windows")
}

pub unsafe fn strftime(
    _s: *mut u8,
    _max: usize,
    _format: *const u8,
    _tm: *const ::libc::tm,
) -> usize {
    todo!("strftime not available on Windows")
}

pub unsafe fn ttyname(_fd: c_int) -> *mut u8 {
    todo!("ttyname not available on Windows")
}

pub unsafe fn strncasecmp(_s1: *const u8, _s2: *const u8, _n: usize) -> c_int {
    todo!("strncasecmp not available on Windows")
}

pub unsafe fn regcomp(_preg: *mut regex_t, _pattern: *const u8, _cflags: c_int) -> c_int {
    todo!("regcomp not available on Windows")
}

pub unsafe fn regexec(
    _preg: *const regex_t,
    _string: *const u8,
    _nmatch: usize,
    _pmatch: *mut regmatch_t,
    _eflags: c_int,
) -> c_int {
    todo!("regexec not available on Windows")
}

pub unsafe fn regfree(_preg: *mut regex_t) {
    todo!("regfree not available on Windows")
}

pub unsafe fn glob(
    _pattern: *const u8,
    _flags: c_int,
    _errfunc: Option<extern "C" fn(epath: *const c_char, errno: c_int) -> c_int>,
    _pglob: *mut glob_t,
) -> c_int {
    todo!("glob not available on Windows")
}

pub unsafe fn globfree(_pglob: *mut glob_t) {
    todo!("globfree not available on Windows")
}


pub unsafe fn fork() -> pid_t {
    todo!("fork not available on Windows")
}

pub unsafe fn forkpty(
    _master: *mut c_int,
    _name: *mut u8,
    _tio: *mut termios,
    _ws: *mut winsize,
) -> pid_t {
    todo!("forkpty not available on Windows")
}

pub unsafe fn kill(_pid: pid_t, _sig: c_int) -> c_int {
    todo!("kill not available on Windows")
}

pub unsafe fn killpg(_pgrp: pid_t, _sig: c_int) -> c_int {
    todo!("killpg not available on Windows")
}

pub unsafe fn waitpid(_pid: pid_t, _status: *mut c_int, _options: c_int) -> pid_t {
    todo!("waitpid not available on Windows")
}

pub unsafe fn socketpair(
    _domain: c_int,
    _type_: c_int,
    _protocol: c_int,
    _sv: *mut c_int,
) -> c_int {
    todo!("socketpair not available on Windows")
}

pub unsafe fn socket(_domain: c_int, _type_: c_int, _protocol: c_int) -> c_int {
    todo!("socket not available on Windows")
}

pub unsafe fn bind(_sockfd: c_int, _addr: *const c_void, _addrlen: socklen_t) -> c_int {
    todo!("bind not available on Windows")
}

pub unsafe fn listen(_sockfd: c_int, _backlog: c_int) -> c_int {
    todo!("listen not available on Windows")
}

pub unsafe fn accept(_sockfd: c_int, _addr: *mut c_void, _addrlen: *mut socklen_t) -> c_int {
    todo!("accept not available on Windows")
}

pub unsafe fn connect(_sockfd: c_int, _addr: *const c_void, _addrlen: socklen_t) -> c_int {
    todo!("connect not available on Windows")
}

pub unsafe fn shutdown(_sockfd: c_int, _how: c_int) -> c_int {
    todo!("shutdown not available on Windows")
}

pub unsafe fn sendmsg(_sockfd: c_int, _msg: *const msghdr, _flags: c_int) -> isize {
    todo!("sendmsg not available on Windows")
}

pub unsafe fn recvmsg(_sockfd: c_int, _msg: *mut msghdr, _flags: c_int) -> isize {
    todo!("recvmsg not available on Windows")
}

pub unsafe fn writev(_fd: c_int, _iov: *const iovec, _iovcnt: c_int) -> isize {
    todo!("writev not available on Windows")
}

pub unsafe fn getdtablesize() -> c_int {
    todo!("getdtablesize not available on Windows")
}

pub unsafe fn sigfillset(_set: *mut sigset_t) -> c_int {
    todo!("sigfillset not available on Windows")
}

pub unsafe fn sigprocmask(_how: c_int, _set: *const sigset_t, _oldset: *mut sigset_t) -> c_int {
    todo!("sigprocmask not available on Windows")
}

pub unsafe fn sigemptyset(_set: *mut sigset_t) -> c_int {
    todo!("sigemptyset not available on Windows")
}

pub unsafe fn sigaction(
    _signum: c_int,
    _act: *const sigaction,
    _oldact: *mut sigaction,
) -> c_int {
    todo!("sigaction not available on Windows")
}

pub unsafe fn tcgetattr(_fd: c_int, _termios: *mut termios) -> c_int {
    todo!("tcgetattr not available on Windows")
}

pub unsafe fn tcsetattr(_fd: c_int, _optional_actions: c_int, _termios: *const termios) -> c_int {
    todo!("tcsetattr not available on Windows")
}

pub unsafe fn tcflush(_fd: c_int, _queue_selector: c_int) -> c_int {
    todo!("tcflush not available on Windows")
}

pub unsafe fn tcgetpgrp(_fd: c_int) -> pid_t {
    todo!("tcgetpgrp not available on Windows")
}

pub unsafe fn cfsetispeed(_termios: *mut termios, _speed: speed_t) -> c_int {
    todo!("cfsetispeed not available on Windows")
}

pub unsafe fn cfsetospeed(_termios: *mut termios, _speed: speed_t) -> c_int {
    todo!("cfsetospeed not available on Windows")
}

pub unsafe fn cfgetispeed(_termios: *const termios) -> speed_t {
    todo!("cfgetispeed not available on Windows")
}

pub unsafe fn cfgetospeed(_termios: *const termios) -> speed_t {
    todo!("cfgetospeed not available on Windows")
}

pub unsafe fn usleep(_usec: c_uint) -> c_int {
    todo!("usleep not available on Windows")
}

pub unsafe fn getpwuid(_uid: uid_t) -> *mut passwd {
    todo!("getpwuid not available on Windows")
}

pub unsafe fn getpwnam(_name: *const u8) -> *mut passwd {
    todo!("getpwnam not available on Windows")
}

pub unsafe fn getuid() -> uid_t {
    todo!("getuid not available on Windows")
}

pub unsafe fn geteuid() -> uid_t {
    todo!("geteuid not available on Windows")
}

pub unsafe fn getegid() -> gid_t {
    todo!("getegid not available on Windows")
}

pub unsafe fn umask(_mask: mode_t) -> mode_t {
    todo!("umask not available on Windows")
}

pub unsafe fn chmod(_path: *const u8, _mode: mode_t) -> c_int {
    todo!("chmod not available on Windows")
}

pub unsafe fn gettimeofday(_tv: *mut ::libc::timeval, _tz: *mut c_void) -> c_int {
    todo!("gettimeofday not available on Windows")
}

pub unsafe fn clock_gettime(_clockid: clockid_t, _tp: *mut ::libc::timespec) -> c_int {
    todo!("clock_gettime not available on Windows")
}

pub unsafe fn localtime_r(
    _time: *const ::libc::time_t,
    _result: *mut ::libc::tm,
) -> *mut ::libc::tm {
    todo!("localtime_r not available on Windows")
}

pub unsafe fn ctime_r(_time: *const ::libc::time_t, _buf: *mut u8) -> *mut u8 {
    todo!("ctime_r not available on Windows")
}

pub unsafe fn gmtime_r(
    _time: *const ::libc::time_t,
    _result: *mut ::libc::tm,
) -> *mut ::libc::tm {
    todo!("gmtime_r not available on Windows")
}

pub unsafe fn localtime(_time: *const ::libc::time_t) -> *mut ::libc::tm {
    todo!("localtime not available on Windows")
}

pub unsafe fn nl_langinfo(_item: nl_item) -> *mut u8 {
    todo!("nl_langinfo not available on Windows")
}

pub unsafe fn uname(_buf: *mut utsname) -> c_int {
    todo!("uname not available on Windows")
}

pub unsafe fn strsignal(_sig: c_int) -> *mut u8 {
    todo!("strsignal not available on Windows")
}

pub unsafe fn dirname(_path: *mut u8) -> *mut u8 {
    todo!("dirname not available on Windows")
}

pub unsafe fn readlink(_path: *const u8, _buf: *mut u8, _bufsiz: usize) -> isize {
    todo!("readlink not available on Windows")
}

pub unsafe fn daemon(_nochdir: c_int, _noclose: c_int) -> c_int {
    todo!("daemon not available on Windows")
}

pub unsafe fn malloc_trim(_pad: usize) -> c_int {
    todo!("malloc_trim not available on Windows")
}

pub unsafe fn dup2(_oldfd: c_int, _newfd: c_int) -> c_int {
    todo!("dup2 not available on Windows")
}

pub unsafe fn execvp(_file: *const u8, _argv: *const *const u8) -> c_int {
    todo!("execvp not available on Windows")
}

pub unsafe fn setsid() -> pid_t {
    todo!("setsid not available on Windows")
}

pub unsafe fn poll(_fds: *mut pollfd, _nfds: nfds_t, _timeout: c_int) -> c_int {
    todo!("poll not available on Windows")
}

pub unsafe fn fseeko(stream: *mut ::libc::FILE, offset: ::libc::off_t, whence: c_int) -> c_int {
    unsafe { ::libc::fseek(stream, offset as core::ffi::c_long, whence) as c_int }
}

pub unsafe fn ftello(stream: *mut ::libc::FILE) -> ::libc::off_t {
    unsafe { ::libc::ftell(stream) as ::libc::off_t }
}

pub unsafe fn sysconf(_name: c_int) -> i64 {
    todo!("sysconf not available on Windows")
}

pub unsafe fn stat(_path: *const u8, _buf: *mut ::libc::stat) -> c_int {
    todo!("stat not available on Windows")
}

//! Windows shims for Unix types, constants, and functions that don't exist in the libc crate on Windows.
//! These provide real implementations where possible, noops for inapplicable Unix concepts,
//! and todo!() only for functions that need proper Windows subsystem implementations (IPC, PTY, etc.).
#![allow(
    nonstandard_style,
    non_camel_case_types,
    non_upper_case_globals,
    dead_code,
    clippy::missing_safety_doc
)]

// Re-export everything from ::libc first, then our definitions shadow what we override
pub use ::libc::*;

use core::ffi::{c_char, c_int, c_uint, c_void};

// ============================================================
// Types (not in ::libc on Windows)
// ============================================================

/// Equivalent to C's `max_align_t` – guarantees maximum fundamental alignment.
/// On MSVC x64 this is 8 bytes (double); we use `f64` to get the right size + alignment.
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct max_align_t(f64);

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

/// WSAPOLLFD from windows-sys — correct SOCKET-sized fd field for WSAPoll on x64.
pub type pollfd = windows_sys::Win32::Networking::WinSock::WSAPOLLFD;

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

pub const STDIN_FILENO: c_int = 0;
pub const STDOUT_FILENO: c_int = 1;
pub const STDERR_FILENO: c_int = 2;

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
    #[link_name = "_execl"]
    pub fn execl(path: *const c_char, arg: *const c_char, ...) -> c_int;
}

// ioctl/fcntl/prctl don't exist on Windows MSVC.
// These are never actually called on Windows (call sites are #[cfg]'d out or unreachable),
// but they need to exist for the code to compile and link.
pub unsafe fn ioctl<T>(_fd: c_int, _request: u64, _arg: T) -> c_int {
    -1
}

// fcntl and prctl: all call sites are #[cfg(not(target_os = "windows"))],
// so we don't define them here. If a call site needs them, it should be cfg'd.

// MSVC CRT functions not in the libc crate
unsafe extern "C" {
    #[link_name = "strftime"]
    fn msvc_strftime(s: *mut u8, max: usize, format: *const u8, tm: *const ::libc::tm) -> usize;
    #[link_name = "localtime"]
    fn msvc_localtime(time: *const ::libc::time_t) -> *mut ::libc::tm;
    #[link_name = "_ctime64_s"]
    fn ctime_s(buf: *mut c_char, bufsz: usize, time: *const ::libc::time_t) -> c_int;
}

// ============================================================
// Function shims - implemented for Windows
// ============================================================

// -- String / pattern matching --

/// Simple fnmatch implementation supporting * and ? wildcards.
pub unsafe fn fnmatch(pattern: *const u8, name: *const u8, flags: c_int) -> c_int {
    let case_fold = (flags & FNM_CASEFOLD) != 0;
    unsafe { fnmatch_inner(pattern, name, case_fold) }
}

unsafe fn fnmatch_inner(mut p: *const u8, mut n: *const u8, case_fold: bool) -> c_int {
    unsafe {
        loop {
            match *p {
                0 => return if *n == 0 { 0 } else { 1 },
                b'?' => {
                    if *n == 0 {
                        return 1;
                    }
                    p = p.add(1);
                    n = n.add(1);
                }
                b'*' => {
                    p = p.add(1);
                    // skip consecutive stars
                    while *p == b'*' {
                        p = p.add(1);
                    }
                    if *p == 0 {
                        return 0;
                    }
                    while *n != 0 {
                        if fnmatch_inner(p, n, case_fold) == 0 {
                            return 0;
                        }
                        n = n.add(1);
                    }
                    return 1;
                }
                pc => {
                    let nc = *n;
                    if nc == 0 {
                        return 1;
                    }
                    let matches = if case_fold {
                        pc.to_ascii_lowercase() == nc.to_ascii_lowercase()
                    } else {
                        pc == nc
                    };
                    if !matches {
                        return 1;
                    }
                    p = p.add(1);
                    n = n.add(1);
                }
            }
        }
    }
}

pub unsafe fn gethostname(name: *mut u8, len: usize) -> c_int {
    if let Ok(hostname) = std::env::var("COMPUTERNAME") {
        let bytes = hostname.as_bytes();
        let copy_len = bytes.len().min(len.saturating_sub(1));
        unsafe {
            core::ptr::copy_nonoverlapping(bytes.as_ptr(), name, copy_len);
            *name.add(copy_len) = 0;
        }
        0
    } else {
        -1
    }
}

pub unsafe fn strftime(s: *mut u8, max: usize, format: *const u8, tm: *const ::libc::tm) -> usize {
    unsafe { msvc_strftime(s, max, format, tm) }
}

pub unsafe fn ttyname(_fd: c_int) -> *mut u8 {
    // No real tty names on Windows; return null
    core::ptr::null_mut()
}

pub unsafe fn strncasecmp(s1: *const u8, s2: *const u8, n: usize) -> c_int {
    unsafe {
        for i in 0..n {
            let a = *s1.add(i);
            let b = *s2.add(i);
            if a == 0 && b == 0 {
                return 0;
            }
            let diff = a.to_ascii_lowercase() as c_int - b.to_ascii_lowercase() as c_int;
            if diff != 0 {
                return diff;
            }
            if a == 0 {
                return 0;
            }
        }
        0
    }
}

// -- Regex (bridged to regex crate) --

const REG_NOMATCH: c_int = 1;

/// Store a `Box<regex::Regex>` as a raw pointer in the first 8 bytes of regex_t._opaque.
pub unsafe fn regcomp(preg: *mut regex_t, pattern: *const u8, cflags: c_int) -> c_int {
    let pat_cstr = unsafe { core::ffi::CStr::from_ptr(pattern.cast()) };
    let pat_str = match pat_cstr.to_str() {
        Ok(s) => s,
        Err(_) => return REG_NOMATCH,
    };

    let case_insensitive = (cflags & REG_ICASE) != 0;
    let regex_pat = if case_insensitive {
        format!("(?i){pat_str}")
    } else {
        pat_str.to_string()
    };

    match regex::Regex::new(&regex_pat) {
        Ok(re) => {
            let boxed = Box::new(re);
            let ptr = Box::into_raw(boxed);
            unsafe {
                let opaque = &raw mut (*preg)._opaque;
                core::ptr::write(opaque as *mut *mut regex::Regex, ptr);
            }
            0
        }
        Err(_) => REG_NOMATCH,
    }
}

pub unsafe fn regexec(
    preg: *const regex_t,
    string: *const u8,
    nmatch: usize,
    pmatch: *mut regmatch_t,
    _eflags: c_int,
) -> c_int {
    let opaque = unsafe { &(*preg)._opaque };
    let re_ptr: *mut regex::Regex =
        unsafe { core::ptr::read(opaque.as_ptr() as *const *mut regex::Regex) };
    if re_ptr.is_null() {
        return REG_NOMATCH;
    }
    let re = unsafe { &*re_ptr };

    let s_cstr = unsafe { core::ffi::CStr::from_ptr(string.cast()) };
    let s = match s_cstr.to_str() {
        Ok(s) => s,
        Err(_) => return REG_NOMATCH,
    };

    if nmatch == 0 || pmatch.is_null() {
        // Just check if it matches
        return if re.is_match(s) { 0 } else { REG_NOMATCH };
    }

    match re.find(s) {
        Some(m) => {
            unsafe {
                (*pmatch.add(0)).rm_so = m.start() as i32;
                (*pmatch.add(0)).rm_eo = m.end() as i32;
                // Fill remaining match slots with -1
                for i in 1..nmatch {
                    (*pmatch.add(i)).rm_so = -1;
                    (*pmatch.add(i)).rm_eo = -1;
                }
            }
            0
        }
        None => REG_NOMATCH,
    }
}

pub unsafe fn regfree(preg: *mut regex_t) {
    let opaque = unsafe { &mut (*preg)._opaque };
    let re_ptr: *mut regex::Regex =
        unsafe { core::ptr::read(opaque.as_ptr() as *const *mut regex::Regex) };
    if !re_ptr.is_null() {
        drop(unsafe { Box::from_raw(re_ptr) });
        unsafe {
            core::ptr::write(
                opaque.as_mut_ptr() as *mut *mut regex::Regex,
                core::ptr::null_mut(),
            );
        }
    }
}

// -- Glob (minimal implementation) --

pub unsafe fn glob(
    pattern: *const u8,
    _flags: c_int,
    _errfunc: Option<extern "C" fn(epath: *const c_char, errno: c_int) -> c_int>,
    pglob: *mut glob_t,
) -> c_int {
    let pat_cstr = unsafe { core::ffi::CStr::from_ptr(pattern.cast()) };
    let pat_str = match pat_cstr.to_str() {
        Ok(s) => s,
        Err(_) => return GLOB_NOMATCH,
    };

    let entries: Vec<std::path::PathBuf> = match ::glob::glob(pat_str) {
        Ok(paths) => paths.filter_map(|r| r.ok()).collect(),
        Err(_) => return GLOB_NOMATCH,
    };

    if entries.is_empty() {
        unsafe {
            (*pglob).gl_pathc = 0;
            (*pglob).gl_pathv = core::ptr::null_mut();
        }
        return GLOB_NOMATCH;
    }

    // Allocate pathv array (null-terminated)
    let count = entries.len();
    let pathv =
        unsafe { ::libc::malloc((count + 1) * size_of::<*mut c_char>()) } as *mut *mut c_char;
    if pathv.is_null() {
        return GLOB_NOSPACE;
    }

    for (i, entry) in entries.iter().enumerate() {
        let s = entry.to_string_lossy();
        let bytes = s.as_bytes();
        let dup = unsafe { ::libc::malloc(bytes.len() + 1) } as *mut c_char;
        if !dup.is_null() {
            unsafe {
                core::ptr::copy_nonoverlapping(bytes.as_ptr(), dup.cast(), bytes.len());
                *dup.cast::<u8>().add(bytes.len()) = 0;
            }
        }
        unsafe {
            *pathv.add(i) = dup;
        }
    }
    unsafe {
        *pathv.add(count) = core::ptr::null_mut();
    }

    unsafe {
        (*pglob).gl_pathc = count;
        (*pglob).gl_pathv = pathv;
    }
    0
}

pub unsafe fn globfree(pglob: *mut glob_t) {
    unsafe {
        if (*pglob).gl_pathv.is_null() {
            return;
        }
        for i in 0..(*pglob).gl_pathc {
            let p = *(*pglob).gl_pathv.add(i);
            if !p.is_null() {
                ::libc::free(p.cast());
            }
        }
        ::libc::free((*pglob).gl_pathv.cast());
        (*pglob).gl_pathv = core::ptr::null_mut();
        (*pglob).gl_pathc = 0;
    }
}

// -- Process functions (no fork on Windows) --

pub unsafe fn fork() -> pid_t {
    // fork does not exist on Windows; callers must use #[cfg(windows)] paths
    eprintln!("fork() called on Windows - this should never happen");
    -1
}

pub unsafe fn forkpty(
    _master: *mut c_int,
    _name: *mut u8,
    _tio: *mut termios,
    _ws: *mut winsize,
) -> pid_t {
    eprintln!("forkpty() called on Windows - this should never happen");
    -1
}

pub unsafe fn kill(_pid: pid_t, _sig: c_int) -> c_int {
    // TODO: implement via TerminateProcess for SIGKILL, GenerateConsoleCtrlEvent for SIGINT
    0
}

pub unsafe fn killpg(_pgrp: pid_t, _sig: c_int) -> c_int {
    0
}

pub unsafe fn waitpid(_pid: pid_t, status: *mut c_int, _options: c_int) -> pid_t {
    // TODO: implement via WaitForSingleObject
    if !status.is_null() {
        unsafe {
            *status = 0;
        }
    }
    -1
}

// -- Socket functions (AF_UNIX via Winsock) --

// Winsock functions and types (via windows-sys)
use windows_sys::Win32::Networking::WinSock::{
    self as ws, FIONBIO, INVALID_SOCKET, SOCKET, WSADATA,
};

const WSAEWOULDBLOCK: c_int = 10035;

unsafe extern "C" {
    fn _open_osfhandle(osfhandle: isize, flags: c_int) -> c_int;
    pub fn _get_osfhandle(fd: c_int) -> isize;
    fn _errno() -> *mut c_int;
}

// ============================================================
// Win32 Console API (via windows-sys)
// ============================================================

pub use windows_sys::Win32::System::Console::{
    CONSOLE_SCREEN_BUFFER_INFO, DISABLE_NEWLINE_AUTO_RETURN, ENABLE_ECHO_INPUT, ENABLE_LINE_INPUT,
    ENABLE_PROCESSED_INPUT, ENABLE_PROCESSED_OUTPUT, ENABLE_VIRTUAL_TERMINAL_INPUT,
    ENABLE_VIRTUAL_TERMINAL_PROCESSING, ENABLE_WINDOW_INPUT, ENABLE_WRAP_AT_EOL_OUTPUT,
    GetConsoleMode, GetConsoleScreenBufferInfo, GetStdHandle, STD_INPUT_HANDLE, STD_OUTPUT_HANDLE,
    SetConsoleMode,
};

/// Get the console window dimensions (columns, rows).
pub fn get_console_size() -> (u32, u32) {
    use windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE;
    unsafe {
        let mut csbi: CONSOLE_SCREEN_BUFFER_INFO = core::mem::zeroed();
        let handle = GetStdHandle(STD_OUTPUT_HANDLE);
        if handle != INVALID_HANDLE_VALUE
            && !handle.is_null()
            && GetConsoleScreenBufferInfo(handle, &mut csbi) != 0
        {
            let sx = (csbi.srWindow.Right - csbi.srWindow.Left + 1) as u32;
            let sy = (csbi.srWindow.Bottom - csbi.srWindow.Top + 1) as u32;
            (sx.max(1), sy.max(1))
        } else {
            (80, 24)
        }
    }
}

/// Open stdout as a CRT file descriptor for VT output.
pub fn stdout_as_fd() -> c_int {
    use windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE;
    unsafe {
        let handle = GetStdHandle(STD_OUTPUT_HANDLE);
        if handle == INVALID_HANDLE_VALUE || handle.is_null() {
            return -1;
        }
        _open_osfhandle(handle as isize, 0)
    }
}

/// Enable VT sequence processing on the console output.
pub fn enable_vt_processing() {
    use windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE;
    unsafe {
        let handle = GetStdHandle(STD_OUTPUT_HANDLE);
        if handle == INVALID_HANDLE_VALUE || handle.is_null() {
            return;
        }
        let mut mode: u32 = 0;
        if GetConsoleMode(handle, &mut mode) != 0 {
            mode |= ENABLE_VIRTUAL_TERMINAL_PROCESSING | DISABLE_NEWLINE_AUTO_RETURN;
            SetConsoleMode(handle, mode);
        }
    }
}

/// Set console stdin to raw mode (disable line edit, echo, processed input).
/// Returns the original mode for later restoration.
pub fn set_console_raw_mode() -> u32 {
    use windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE;
    unsafe {
        let handle = GetStdHandle(STD_INPUT_HANDLE);
        let mut orig_mode: u32 = 0;
        if handle != INVALID_HANDLE_VALUE
            && !handle.is_null()
            && GetConsoleMode(handle, &mut orig_mode) != 0
        {
            let raw_mode = (orig_mode
                & !(ENABLE_LINE_INPUT | ENABLE_ECHO_INPUT | ENABLE_PROCESSED_INPUT))
                | ENABLE_WINDOW_INPUT
                | ENABLE_VIRTUAL_TERMINAL_INPUT;
            SetConsoleMode(handle, raw_mode);
        }
        orig_mode
    }
}

/// Restore console stdin mode.
pub fn restore_console_mode(mode: u32) {
    use windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE;
    unsafe {
        let handle = GetStdHandle(STD_INPUT_HANDLE);
        if handle != INVALID_HANDLE_VALUE && !handle.is_null() {
            SetConsoleMode(handle, mode);
        }
    }
}

fn ensure_winsock_init() {
    use std::sync::Once;
    static INIT: Once = Once::new();
    INIT.call_once(|| unsafe {
        let mut data: WSADATA = core::mem::zeroed();
        ws::WSAStartup(0x0202, &mut data);
    });
}

/// Convert a Winsock SOCKET to a CRT file descriptor.
unsafe fn socket_to_fd(sock: usize) -> c_int {
    if sock == INVALID_SOCKET {
        return -1;
    }
    unsafe {
        let fd = _open_osfhandle(sock as isize, 0);
        if fd == -1 {
            ws::closesocket(sock);
        }
        fd
    }
}

/// Get the Winsock SOCKET from a CRT file descriptor.
unsafe fn fd_to_socket(fd: c_int) -> usize {
    unsafe { _get_osfhandle(fd) as usize }
}

/// Set a socket to non-blocking mode.
pub unsafe fn socket_set_nonblocking(fd: c_int, nonblocking: bool) {
    unsafe {
        let sock = fd_to_socket(fd);
        let mut mode: u32 = if nonblocking { 1 } else { 0 };
        ws::ioctlsocket(sock, FIONBIO, &mut mode);
    }
}

fn set_winsock_errno() {
    unsafe {
        let wsa_err = ws::WSAGetLastError();
        // Map common Winsock errors to errno values
        let err = match wsa_err {
            10004 => EINTR,        // WSAEINTR
            10013 => EACCES,       // WSAEACCES
            10022 => EINVAL,       // WSAEINVAL
            10024 => EMFILE,       // WSAEMFILE
            10035 => EAGAIN,       // WSAEWOULDBLOCK
            10036 => EINPROGRESS,  // WSAEINPROGRESS
            10048 => EADDRINUSE,   // WSAEADDRINUSE
            10054 => ECONNRESET,   // WSAECONNRESET
            10056 => EISCONN,      // WSAEISCONN
            10057 => ENOTCONN,     // WSAENOTCONN
            10060 => ETIMEDOUT,    // WSAETIMEDOUT
            10061 => ECONNREFUSED, // WSAECONNREFUSED
            _ => wsa_err,
        };
        *_errno() = err;
    }
}

pub unsafe fn socket(domain: c_int, type_: c_int, protocol: c_int) -> c_int {
    ensure_winsock_init();
    unsafe {
        let sock = ws::socket(domain, type_, protocol);
        if sock == INVALID_SOCKET {
            set_winsock_errno();
            return -1;
        }
        let fd = socket_to_fd(sock);
        if fd == -1 {
            *_errno() = EMFILE;
        }
        fd
    }
}

pub unsafe fn bind(sockfd: c_int, addr: *const c_void, addrlen: socklen_t) -> c_int {
    unsafe {
        let sock = fd_to_socket(sockfd);
        let ret = ws::bind(sock, addr.cast(), addrlen as c_int);
        if ret != 0 {
            set_winsock_errno();
            return -1;
        }
        0
    }
}

pub unsafe fn listen(sockfd: c_int, backlog: c_int) -> c_int {
    unsafe {
        let sock = fd_to_socket(sockfd);
        let ret = ws::listen(sock, backlog);
        if ret != 0 {
            set_winsock_errno();
            return -1;
        }
        0
    }
}

pub unsafe fn accept(sockfd: c_int, addr: *mut c_void, addrlen: *mut socklen_t) -> c_int {
    unsafe {
        let sock = fd_to_socket(sockfd);
        let new_sock = ws::accept(sock, addr.cast(), addrlen.cast());
        if new_sock == INVALID_SOCKET {
            set_winsock_errno();
            return -1;
        }
        let fd = socket_to_fd(new_sock);
        if fd == -1 {
            *_errno() = EMFILE;
        }
        fd
    }
}

pub unsafe fn connect(sockfd: c_int, addr: *const c_void, addrlen: socklen_t) -> c_int {
    unsafe {
        let sock = fd_to_socket(sockfd);
        let ret = ws::connect(sock, addr.cast(), addrlen as c_int);
        if ret != 0 {
            set_winsock_errno();
            return -1;
        }
        0
    }
}

pub unsafe fn shutdown(sockfd: c_int, how: c_int) -> c_int {
    unsafe {
        let sock = fd_to_socket(sockfd);
        let ret = ws::shutdown(sock, how);
        if ret != 0 {
            set_winsock_errno();
            return -1;
        }
        0
    }
}

pub unsafe fn socketpair(domain: c_int, type_: c_int, protocol: c_int, sv: *mut c_int) -> c_int {
    // Emulate socketpair using AF_UNIX: create listener, connect, accept
    ensure_winsock_init();
    unsafe {
        let listener = ws::socket(domain, type_, protocol);
        if listener == INVALID_SOCKET {
            set_winsock_errno();
            return -1;
        }

        let mut addr: sockaddr_un = core::mem::zeroed();
        addr.sun_family = domain as u16;
        // Use a temp path for the ephemeral listener
        let tmp = std::env::temp_dir();
        let path = format!(
            "{}/tmux-rs-sockpair-{}",
            tmp.to_string_lossy(),
            std::process::id()
        );
        let path_bytes = path.as_bytes();
        if path_bytes.len() >= UNIX_PATH_MAX {
            ws::closesocket(listener);
            *_errno() = ENAMETOOLONG;
            return -1;
        }
        core::ptr::copy_nonoverlapping(
            path_bytes.as_ptr(),
            addr.sun_path.as_mut_ptr().cast(),
            path_bytes.len(),
        );
        addr.sun_path[path_bytes.len()] = 0;

        // Clean up any stale socket file
        let _ = std::fs::remove_file(&path);

        if ws::bind(
            listener,
            &raw const addr as _,
            core::mem::size_of::<sockaddr_un>() as _,
        ) != 0
            || ws::listen(listener, 1) != 0
        {
            set_winsock_errno();
            ws::closesocket(listener);
            let _ = std::fs::remove_file(&path);
            return -1;
        }

        let connector = ws::socket(domain, type_, protocol);
        if connector == INVALID_SOCKET {
            set_winsock_errno();
            ws::closesocket(listener);
            let _ = std::fs::remove_file(&path);
            return -1;
        }

        if ws::connect(
            connector,
            &raw const addr as _,
            core::mem::size_of::<sockaddr_un>() as _,
        ) != 0
        {
            set_winsock_errno();
            ws::closesocket(connector);
            ws::closesocket(listener);
            let _ = std::fs::remove_file(&path);
            return -1;
        }

        let acceptor = ws::accept(listener, core::ptr::null_mut(), core::ptr::null_mut());
        ws::closesocket(listener);
        let _ = std::fs::remove_file(&path);

        if acceptor == INVALID_SOCKET {
            set_winsock_errno();
            ws::closesocket(connector);
            return -1;
        }

        *sv = socket_to_fd(connector);
        *sv.add(1) = socket_to_fd(acceptor);
        if *sv == -1 || *sv.add(1) == -1 {
            if *sv != -1 {
                ::libc::close(*sv);
            }
            if *sv.add(1) != -1 {
                ::libc::close(*sv.add(1));
            }
            *_errno() = EMFILE;
            return -1;
        }
        0
    }
}

pub unsafe fn sendmsg(sockfd: c_int, msg: *const msghdr, _flags: c_int) -> isize {
    // Send data from iovec array. Ignore control messages (no SCM_RIGHTS on Windows).
    unsafe {
        let sock = fd_to_socket(sockfd);
        let msg = &*msg;
        let mut total: isize = 0;
        for i in 0..msg.msg_iovlen {
            let iov = &*msg.msg_iov.add(i);
            if iov.iov_len == 0 {
                continue;
            }
            let n = ws::send(sock, iov.iov_base.cast(), iov.iov_len as c_int, 0);
            if n < 0 {
                if total > 0 {
                    return total;
                }
                set_winsock_errno();
                return -1;
            }
            total += n as isize;
            if (n as usize) < iov.iov_len {
                break; // partial write
            }
        }
        total
    }
}

pub unsafe fn recvmsg(sockfd: c_int, msg: *mut msghdr, _flags: c_int) -> isize {
    // Receive data into iovec array. Ignore control messages (no SCM_RIGHTS on Windows).
    unsafe {
        let sock = fd_to_socket(sockfd);
        let msg = &mut *msg;
        msg.msg_controllen = 0; // No control data on Windows
        let mut total: isize = 0;
        for i in 0..msg.msg_iovlen {
            let iov = &*msg.msg_iov.add(i);
            if iov.iov_len == 0 {
                continue;
            }
            let n = ws::recv(sock, iov.iov_base.cast(), iov.iov_len as c_int, 0);
            if n < 0 {
                if total > 0 {
                    return total;
                }
                set_winsock_errno();
                return -1;
            }
            if n == 0 {
                return total; // EOF
            }
            total += n as isize;
            if (n as usize) < iov.iov_len {
                break; // partial read
            }
        }
        total
    }
}

pub unsafe fn writev(fd: c_int, iov: *const iovec, iovcnt: c_int) -> isize {
    unsafe {
        let mut total: isize = 0;
        for i in 0..iovcnt as usize {
            let v = &*iov.add(i);
            let n = ::libc::write(fd, v.iov_base, v.iov_len as c_uint);
            if n < 0 {
                return -1;
            }
            total += n as isize;
        }
        total
    }
}

pub unsafe fn send(sockfd: c_int, buf: *const c_void, len: usize, flags: c_int) -> isize {
    unsafe {
        let sock = fd_to_socket(sockfd);
        let n = ws::send(sock, buf.cast(), len as c_int, flags);
        if n < 0 {
            set_winsock_errno();
            return -1;
        }
        n as isize
    }
}

pub unsafe fn recv(sockfd: c_int, buf: *mut c_void, len: usize, flags: c_int) -> isize {
    unsafe {
        let sock = fd_to_socket(sockfd);
        let n = ws::recv(sock, buf.cast(), len as c_int, flags);
        if n < 0 {
            set_winsock_errno();
            return -1;
        }
        n as isize
    }
}

pub unsafe fn getdtablesize() -> c_int {
    // Windows CRT default max open files
    2048
}

// -- Signal functions (noop on Windows) --

pub unsafe fn sigfillset(set: *mut sigset_t) -> c_int {
    if !set.is_null() {
        unsafe {
            *set = !0u64;
        }
    }
    0
}

pub unsafe fn sigprocmask(_how: c_int, _set: *const sigset_t, _oldset: *mut sigset_t) -> c_int {
    0
}

pub unsafe fn sigemptyset(set: *mut sigset_t) -> c_int {
    if !set.is_null() {
        unsafe {
            *set = 0;
        }
    }
    0
}

pub unsafe fn sigaction(_signum: c_int, _act: *const sigaction, _oldact: *mut sigaction) -> c_int {
    0
}

// -- Terminal functions (stubs for now, Phase 4 will implement via Console API) --

pub unsafe fn tcgetattr(_fd: c_int, termios: *mut termios) -> c_int {
    // Return zeroed termios - will be replaced by GetConsoleMode in Phase 4
    if !termios.is_null() {
        unsafe {
            core::ptr::write_bytes(termios, 0, 1);
        }
    }
    0
}

pub unsafe fn tcsetattr(_fd: c_int, _optional_actions: c_int, _termios: *const termios) -> c_int {
    0
}

pub unsafe fn tcflush(_fd: c_int, _queue_selector: c_int) -> c_int {
    0
}

pub unsafe fn tcgetpgrp(_fd: c_int) -> pid_t {
    -1
}

pub unsafe fn cfsetispeed(_termios: *mut termios, _speed: speed_t) -> c_int {
    0
}

pub unsafe fn cfsetospeed(_termios: *mut termios, _speed: speed_t) -> c_int {
    0
}

pub unsafe fn cfgetispeed(_termios: *const termios) -> speed_t {
    38400
}

pub unsafe fn cfgetospeed(_termios: *const termios) -> speed_t {
    38400
}

// -- User / passwd --

pub unsafe fn usleep(usec: c_uint) -> c_int {
    std::thread::sleep(std::time::Duration::from_micros(usec as u64));
    0
}

/// Static passwd storage allocated on the heap via Box::leak.
/// Populated from environment variables on first call.
static PASSWD_PTR: std::sync::OnceLock<usize> = std::sync::OnceLock::new();

#[repr(C)]
struct PasswdStorage {
    pw: passwd,
    name_buf: [u8; 256],
    dir_buf: [u8; 512],
    shell_buf: [u8; 512],
}

fn get_passwd_ptr() -> *mut PasswdStorage {
    *PASSWD_PTR.get_or_init(|| {
        let storage = Box::leak(Box::new(PasswdStorage {
            pw: passwd {
                pw_name: core::ptr::null_mut(),
                pw_passwd: core::ptr::null_mut(),
                pw_uid: 0,
                pw_gid: 0,
                pw_gecos: core::ptr::null_mut(),
                pw_dir: core::ptr::null_mut(),
                pw_shell: core::ptr::null_mut(),
            },
            name_buf: [0; 256],
            dir_buf: [0; 512],
            shell_buf: [0; 512],
        }));
        let ptr: *mut PasswdStorage = storage;

        unsafe {
            // pw_name from USERNAME
            if let Ok(name) = std::env::var("USERNAME") {
                let bytes = name.as_bytes();
                let len = bytes.len().min(255);
                core::ptr::copy_nonoverlapping(bytes.as_ptr(), (*ptr).name_buf.as_mut_ptr(), len);
                (*ptr).name_buf[len] = 0;
            }
            (*ptr).pw.pw_name = (*ptr).name_buf.as_mut_ptr().cast();
            (*ptr).pw.pw_gecos = (*ptr).name_buf.as_mut_ptr().cast();

            // pw_dir from USERPROFILE
            if let Ok(dir) = std::env::var("USERPROFILE") {
                let bytes = dir.as_bytes();
                let len = bytes.len().min(511);
                core::ptr::copy_nonoverlapping(bytes.as_ptr(), (*ptr).dir_buf.as_mut_ptr(), len);
                (*ptr).dir_buf[len] = 0;
            }
            (*ptr).pw.pw_dir = (*ptr).dir_buf.as_mut_ptr().cast();

            // pw_shell: prefer SHELL, then ComSpec, then cmd.exe
            let shell = std::env::var("SHELL")
                .or_else(|_| std::env::var("ComSpec"))
                .unwrap_or_else(|_| "C:\\Windows\\System32\\cmd.exe".to_string());
            let bytes = shell.as_bytes();
            let len = bytes.len().min(511);
            core::ptr::copy_nonoverlapping(bytes.as_ptr(), (*ptr).shell_buf.as_mut_ptr(), len);
            (*ptr).shell_buf[len] = 0;
            (*ptr).pw.pw_shell = (*ptr).shell_buf.as_mut_ptr().cast();

            (*ptr).pw.pw_passwd = b"\0".as_ptr() as *mut c_char;
        }

        ptr as usize
    }) as *mut PasswdStorage
}

pub unsafe fn getpwuid(_uid: uid_t) -> *mut passwd {
    let storage = get_passwd_ptr();
    unsafe { &raw mut (*storage).pw }
}

pub unsafe fn getpwnam(_name: *const u8) -> *mut passwd {
    let storage = get_passwd_ptr();
    unsafe { &raw mut (*storage).pw }
}

pub unsafe fn getuid() -> uid_t {
    0
}

pub unsafe fn geteuid() -> uid_t {
    0
}

pub unsafe fn getegid() -> gid_t {
    0
}

// -- File permission functions (noop on Windows) --

pub unsafe fn umask(_mask: mode_t) -> mode_t {
    0o022
}

pub unsafe fn chmod(_path: *const u8, _mode: mode_t) -> c_int {
    0
}

// -- Time functions --

pub unsafe fn gettimeofday(tv: *mut ::libc::timeval, _tz: *mut c_void) -> c_int {
    if tv.is_null() {
        return -1;
    }
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    unsafe {
        (*tv).tv_sec = now.as_secs() as ::libc::c_long;
        (*tv).tv_usec = now.subsec_micros() as ::libc::c_long;
    }
    0
}

pub unsafe fn clock_gettime(clockid: clockid_t, tp: *mut ::libc::timespec) -> c_int {
    if tp.is_null() {
        return -1;
    }
    if clockid == CLOCK_MONOTONIC {
        // Use QueryPerformanceCounter for monotonic time
        static mut QPC_FREQ: i64 = 0;
        static QPC_INIT: std::sync::Once = std::sync::Once::new();
        unsafe {
            QPC_INIT.call_once(|| {
                let mut freq: i64 = 0;
                QueryPerformanceFrequency(&mut freq);
                QPC_FREQ = freq;
            });
            let mut count: i64 = 0;
            QueryPerformanceCounter(&mut count);
            (*tp).tv_sec = (count / QPC_FREQ) as ::libc::time_t;
            (*tp).tv_nsec = ((count % QPC_FREQ) * 1_000_000_000 / QPC_FREQ) as i32;
        }
    } else {
        // CLOCK_REALTIME
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default();
        unsafe {
            (*tp).tv_sec = now.as_secs() as ::libc::time_t;
            (*tp).tv_nsec = now.subsec_nanos() as i32;
        }
    }
    0
}

use windows_sys::Win32::System::Performance::{QueryPerformanceCounter, QueryPerformanceFrequency};

pub unsafe fn localtime_r(time: *const ::libc::time_t, result: *mut ::libc::tm) -> *mut ::libc::tm {
    unsafe {
        // MSVC localtime_s has reversed args: (result, time)
        if ::libc::localtime_s(result, time) == 0 {
            result
        } else {
            core::ptr::null_mut()
        }
    }
}

pub unsafe fn ctime_r(time: *const ::libc::time_t, buf: *mut u8) -> *mut u8 {
    unsafe {
        if ctime_s(buf.cast(), 26, time) == 0 {
            buf
        } else {
            core::ptr::null_mut()
        }
    }
}

pub unsafe fn gmtime_r(time: *const ::libc::time_t, result: *mut ::libc::tm) -> *mut ::libc::tm {
    unsafe {
        if ::libc::gmtime_s(result, time) == 0 {
            result
        } else {
            core::ptr::null_mut()
        }
    }
}

pub unsafe fn localtime(time: *const ::libc::time_t) -> *mut ::libc::tm {
    unsafe { msvc_localtime(time) }
}

pub unsafe fn nl_langinfo(item: nl_item) -> *mut u8 {
    if item == CODESET {
        // Windows Terminal always uses UTF-8
        static UTF8: &[u8] = b"UTF-8\0";
        UTF8.as_ptr() as *mut u8
    } else {
        static EMPTY: &[u8] = b"\0";
        EMPTY.as_ptr() as *mut u8
    }
}

pub unsafe fn uname(buf: *mut utsname) -> c_int {
    if buf.is_null() {
        return -1;
    }
    unsafe {
        core::ptr::write_bytes(buf, 0, 1);
        let sysname = b"Windows\0";
        core::ptr::copy_nonoverlapping(
            sysname.as_ptr(),
            (*buf).sysname.as_mut_ptr().cast(),
            sysname.len(),
        );

        if let Ok(hostname) = std::env::var("COMPUTERNAME") {
            let bytes = hostname.as_bytes();
            let len = bytes.len().min(_UTSNAME_LENGTH - 1);
            core::ptr::copy_nonoverlapping(
                bytes.as_ptr(),
                (*buf).nodename.as_mut_ptr().cast(),
                len,
            );
        }

        let release = b"10.0\0";
        core::ptr::copy_nonoverlapping(
            release.as_ptr(),
            (*buf).release.as_mut_ptr().cast(),
            release.len(),
        );

        #[cfg(target_arch = "x86_64")]
        {
            let machine = b"x86_64\0";
            core::ptr::copy_nonoverlapping(
                machine.as_ptr(),
                (*buf).machine.as_mut_ptr().cast(),
                machine.len(),
            );
        }
        #[cfg(target_arch = "aarch64")]
        {
            let machine = b"aarch64\0";
            core::ptr::copy_nonoverlapping(
                machine.as_ptr(),
                (*buf).machine.as_mut_ptr().cast(),
                machine.len(),
            );
        }
    }
    0
}

pub unsafe fn strsignal(_sig: c_int) -> *mut u8 {
    static UNKNOWN: &[u8] = b"Unknown signal\0";
    UNKNOWN.as_ptr() as *mut u8
}

/// POSIX dirname: return directory component of path.
/// Modifies the input buffer in place (like the real dirname).
pub unsafe fn dirname(path: *mut u8) -> *mut u8 {
    static DOT: [u8; 2] = [b'.', 0];
    if path.is_null() || unsafe { *path } == 0 {
        return DOT.as_ptr() as *mut u8;
    }
    unsafe {
        let len = ::libc::strlen(path.cast());
        // Strip trailing slashes
        let mut end = len;
        while end > 0 && (*path.add(end - 1) == b'/' || *path.add(end - 1) == b'\\') {
            end -= 1;
        }
        if end == 0 {
            *path = b'/';
            *path.add(1) = 0;
            return path;
        }
        // Find last slash
        while end > 0 && *path.add(end - 1) != b'/' && *path.add(end - 1) != b'\\' {
            end -= 1;
        }
        if end == 0 {
            return DOT.as_ptr() as *mut u8;
        }
        // Strip trailing slashes from result
        while end > 1 && (*path.add(end - 1) == b'/' || *path.add(end - 1) == b'\\') {
            end -= 1;
        }
        *path.add(end) = 0;
        path
    }
}

pub unsafe fn readlink(_path: *const u8, _buf: *mut u8, _bufsiz: usize) -> isize {
    -1 // Not applicable on Windows
}

pub unsafe fn daemon(_nochdir: c_int, _noclose: c_int) -> c_int {
    -1 // Not applicable on Windows; always run in foreground
}

pub unsafe fn malloc_trim(_pad: usize) -> c_int {
    0 // Noop on Windows
}

pub unsafe fn dup2(oldfd: c_int, newfd: c_int) -> c_int {
    unsafe { ::libc::dup2(oldfd, newfd) }
}

pub unsafe fn execvp(_file: *const u8, _argv: *const *const u8) -> c_int {
    eprintln!("execvp() called on Windows - this should never happen");
    -1
}

pub unsafe fn setsid() -> pid_t {
    -1 // Not applicable on Windows
}

pub unsafe fn poll(fds: *mut pollfd, nfds: nfds_t, timeout: c_int) -> c_int {
    unsafe { ws::WSAPoll(fds, nfds as u32, timeout) }
}

pub unsafe fn fseeko(stream: *mut ::libc::FILE, offset: ::libc::off_t, whence: c_int) -> c_int {
    unsafe { ::libc::fseek(stream, offset as core::ffi::c_long, whence) as c_int }
}

pub unsafe fn ftello(stream: *mut ::libc::FILE) -> ::libc::off_t {
    unsafe { ::libc::ftell(stream) as ::libc::off_t }
}

pub unsafe fn sysconf(_name: c_int) -> i64 {
    // _SC_MB_LEN_MAX = max bytes per multibyte character
    16
}

pub unsafe fn stat(path: *const u8, buf: *mut ::libc::stat) -> c_int {
    unsafe { ::libc::stat(path.cast(), buf) }
}

// ============================================================
// POSIX globals / functions not available on Windows
// ============================================================

pub static mut environ: *mut *mut u8 = core::ptr::null_mut();

/// Rust implementation of POSIX strsep for Windows.
pub unsafe fn strsep(stringp: *mut *mut u8, delim: *const u8) -> *mut u8 {
    unsafe {
        let s = *stringp;
        if s.is_null() {
            return core::ptr::null_mut();
        }
        // Find the first occurrence of any delimiter character
        let mut p = s;
        while *p != 0 {
            let mut d = delim;
            while *d != 0 {
                if *p == *d {
                    *p = 0;
                    *stringp = p.add(1);
                    return s;
                }
                d = d.add(1);
            }
            p = p.add(1);
        }
        // No delimiter found
        *stringp = core::ptr::null_mut();
        s
    }
}

pub unsafe fn wcwidth(c: super::wchar_t) -> i32 {
    use unicode_width::UnicodeWidthChar;
    if let Some(ch) = char::from_u32(c as u32) {
        ch.width().unwrap_or(0) as i32
    } else {
        -1
    }
}

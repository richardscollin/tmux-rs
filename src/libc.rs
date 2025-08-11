#![allow(clippy::disallowed_types)]

// reexport everything in libc from this module, then override things we want to change the interface for
pub use ::libc::*;

pub type wchar_t = core::ffi::c_int;

unsafe extern "C" {
    pub static mut environ: *mut *mut u8;
    pub fn strsep(_: *mut *mut u8, _delim: *const u8) -> *mut u8;
}

pub unsafe fn chdir(dir: *const u8) -> i32 {
    unsafe { ::libc::chdir(dir.cast()) }
}

pub unsafe fn free_<T>(p: *mut T) {
    unsafe { ::libc::free(p as *mut c_void) }
}

#[allow(clippy::allow_attributes)]
#[allow(
    clippy::unnecessary_cast,
    reason = "mode_t is u16 on macos so cast is required for some platforms only (should be allow, not expect)"
)]
pub unsafe fn open(path: *const u8, oflag: i32, mode: libc::mode_t) -> i32 {
    unsafe { ::libc::open(path.cast(), oflag, mode as u32) }
}

pub unsafe fn fopen(filename: *const u8, mode: *const u8) -> *mut FILE {
    unsafe { ::libc::fopen(filename.cast(), mode.cast()) }
}

pub unsafe fn fnmatch(pattern: *const u8, name: *const u8, flags: c_int) -> c_int {
    unsafe { ::libc::fnmatch(pattern.cast(), name.cast(), flags) }
}

pub unsafe fn gethostname(name: *mut u8, len: size_t) -> c_int {
    unsafe { ::libc::gethostname(name.cast(), len) }
}

pub unsafe fn memcpy_<T>(dest: *mut T, src: *const T, n: usize) -> *mut T {
    unsafe { ::libc::memcpy(dest as *mut c_void, src as *const c_void, n).cast() }
}

pub unsafe fn memcpy__<T>(dest: *mut T, src: *const T) -> *mut T {
    unsafe { ::libc::memcpy(dest as *mut c_void, src as *const c_void, size_of::<T>()).cast() }
}

pub unsafe fn setlocale(category: i32, locale: *const u8) -> *mut u8 {
    unsafe { ::libc::setlocale(category, locale.cast()).cast() }
}

pub unsafe fn strftime(s: *mut u8, max: usize, format: *const u8, tm: *const tm) -> usize {
    unsafe { ::libc::strftime(s.cast(), max, format.cast(), tm.cast()) }
}

pub unsafe fn strcpy(dst: *mut u8, src: *const u8) -> *mut u8 {
    unsafe { ::libc::strcpy(dst.cast(), src.cast()).cast() }
}

pub unsafe fn strdup(cs: *const u8) -> *mut u8 {
    unsafe { ::libc::strdup(cs.cast()).cast() }
}

pub unsafe fn strndup(cs: *const u8, n: usize) -> *mut u8 {
    unsafe { ::libc::strndup(cs.cast(), n).cast() }
}

pub unsafe fn strlen(cs: *const u8) -> usize {
    unsafe { ::libc::strlen(cs.cast()) }
}

pub unsafe fn strnlen(cs: *const u8, maxlen: usize) -> usize {
    unsafe { ::libc::strnlen(cs.cast(), maxlen) }
}

pub unsafe fn strspn(cs: *const u8, ct: *const u8) -> usize {
    unsafe { ::libc::strspn(cs.cast(), ct.cast()) }
}

pub unsafe fn strpbrk(cs: *const u8, ct: *const u8) -> *mut u8 {
    unsafe { ::libc::strpbrk(cs.cast(), ct.cast()).cast() }
}

pub unsafe fn strcspn(cs: *const u8, ct: *const u8) -> usize {
    unsafe { ::libc::strcspn(cs.cast(), ct.cast()) }
}

pub unsafe fn strrchr(cs: *const u8, c: c_int) -> *mut u8 {
    unsafe { ::libc::strrchr(cs.cast(), c).cast() }
}

pub unsafe fn strchr(cs: *const u8, c: i32) -> *mut u8 {
    unsafe { ::libc::strchr(cs.cast(), c).cast() }
}

pub unsafe fn strchr_(cs: *const u8, c: char) -> *mut u8 {
    unsafe { ::libc::strchr(cs.cast(), c as i32).cast() }
}

pub unsafe fn strstr(cs: *const u8, ct: *const u8) -> *mut u8 {
    unsafe { ::libc::strstr(cs.cast(), ct.cast()).cast() }
}

pub unsafe fn strtol(s: *const u8, endp: *mut *mut u8, base: i32) -> i64 {
    unsafe { ::libc::strtol(s.cast(), endp.cast(), base) }
}

pub unsafe fn strtoul(s: *const u8, endp: *mut *mut u8, base: i32) -> u64 {
    unsafe { ::libc::strtoul(s.cast(), endp.cast(), base) }
}

pub unsafe fn strtod(s: *const u8, endp: *mut *mut u8) -> f64 {
    unsafe { ::libc::strtod(s.cast(), endp.cast()) }
}

pub fn time(t: *mut time_t) -> time_t {
    assert!(t.is_null());

    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as time_t
}

pub unsafe fn tzset() {
    unsafe extern "C" {
        fn tzset();
    }
    unsafe { tzset() }
}

pub unsafe fn unlink(c: *const u8) -> i32 {
    unsafe { ::libc::unlink(c.cast()) }
}

#[inline]
pub unsafe fn bsearch__<T>(
    key: *const T,
    base: *const T,
    num: usize,
    compar: unsafe extern "C" fn(*const c_void, *const c_void) -> i32,
) -> *mut T {
    unsafe { ::libc::bsearch(key.cast(), base.cast(), num, size_of::<T>(), Some(compar)).cast() }
}

#[cfg(target_os = "linux")]
macro_rules! errno {
    () => {
        *::libc::__errno_location()
    };
}
#[cfg(target_os = "macos")]
macro_rules! errno {
    () => {
        *::libc::__error()
    };
}
pub(crate) use errno;

#[cfg(target_os = "linux")]
#[expect(non_snake_case)]
#[inline]
pub fn MB_CUR_MAX() -> usize {
    unsafe extern "C" {
        unsafe fn __ctype_get_mb_cur_max() -> usize;
    }
    unsafe { __ctype_get_mb_cur_max() }
}

#[cfg(target_os = "macos")]
#[expect(non_snake_case)]
#[inline]
pub fn MB_CUR_MAX() -> usize {
    unsafe extern "C" {
        unsafe fn ___mb_cur_max() -> i32;
    }
    unsafe { ___mb_cur_max() as usize }
}

unsafe extern "C" {
    pub fn wcwidth(c: wchar_t) -> i32;
    pub fn mbtowc(pwc: *mut wchar_t, s: *const u8, n: usize) -> i32;
    pub fn wctomb(s: *mut u8, wc: wchar_t) -> i32;
}

#[inline]
pub unsafe fn memset0<T>(dest: *mut T) -> *mut T {
    unsafe { libc::memset(dest.cast(), 0, size_of::<T>()).cast() }
}

pub unsafe fn regcomp(preg: *mut regex_t, pattern: *const u8, cflags: i32) -> i32 {
    unsafe { ::libc::regcomp(preg, pattern.cast(), cflags) }
}

pub unsafe fn glob(
    pattern: *const u8,
    flags: i32,
    errfunc: Option<extern "C" fn(epath: *const c_char, errno: c_int) -> c_int>,
    pglob: *mut glob_t,
) -> i32 {
    unsafe { ::libc::glob(pattern.cast(), flags, errfunc, pglob) }
}

pub unsafe fn regexec(
    preg: *const regex_t,
    input: *const u8,
    nmatch: usize,
    pmatch: *mut regmatch_t,
    eflags: i32,
) -> i32 {
    unsafe { ::libc::regexec(preg, input.cast(), nmatch, pmatch, eflags) }
}

#[inline]
pub unsafe fn timerclear(tv: *mut timeval) {
    // implemented as a macro by most libc's
    unsafe {
        (*tv).tv_sec = 0;
        (*tv).tv_usec = 0;
    }
}

/// result must be initialized after this function
#[inline]
pub unsafe fn timersub(a: *const timeval, b: *const timeval, result: *mut timeval) {
    // implemented as a macro by most libc's
    unsafe {
        (*result).tv_sec = (*a).tv_sec - (*b).tv_sec;
        (*result).tv_usec = (*a).tv_usec - (*b).tv_usec;
        if (*result).tv_usec < 0 {
            (*result).tv_sec -= 1;
            (*result).tv_usec += 1000000;
        }
    }
}

pub struct timer(*const libc::timeval);
impl timer {
    pub unsafe fn new(ptr: *const libc::timeval) -> Self {
        Self(ptr)
    }
}
impl Eq for timer {}
impl PartialEq for timer {
    fn eq(&self, other: &Self) -> bool {
        unsafe { (*self.0).tv_sec == (*other.0).tv_sec && (*self.0).tv_usec == (*other.0).tv_usec }
    }
}
impl Ord for timer {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        unsafe {
            if (*self.0).tv_sec == (*other.0).tv_sec {
                (*self.0).tv_usec.cmp(&(*other.0).tv_usec)
            } else {
                (*self.0).tv_sec.cmp(&(*other.0).tv_sec)
            }
        }
    }
}
impl PartialOrd for timer {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

pub unsafe fn strcmp(cs: *const u8, ct: *const u8) -> i32 {
    unsafe { ::libc::strcmp(cs.cast(), ct.cast()) }
}

pub unsafe fn strncmp(cs: *const u8, ct: *const u8, n: usize) -> i32 {
    unsafe { ::libc::strncmp(cs.cast(), ct.cast(), n) }
}

pub unsafe fn strcmp_(left: *const u8, right: &str) -> std::cmp::Ordering {
    unsafe {
        for (i, r_ch) in right.bytes().enumerate() {
            let l_ch = *left.add(i);

            if l_ch == b'\0' {
                return std::cmp::Ordering::Less;
            }

            match l_ch.cmp(&r_ch) {
                std::cmp::Ordering::Equal => continue,
                value => return value,
            }
        }

        if *left.add(right.len()) == b'\0' {
            std::cmp::Ordering::Equal
        } else {
            std::cmp::Ordering::Greater
        }
    }
}

pub unsafe fn strcasecmp(s1: *const u8, s2: *const u8) -> i32 {
    unsafe { ::libc::strcasecmp(s1.cast(), s2.cast()) }
}

pub unsafe fn strncasecmp(s1: *const u8, s2: *const u8, n: usize) -> i32 {
    unsafe { ::libc::strncasecmp(s1.cast(), s2.cast(), n) }
}

pub unsafe fn strcasecmp_(left: *const u8, right: &'static str) -> std::cmp::Ordering {
    unsafe {
        for (i, r_ch) in right.bytes().enumerate() {
            let l_ch = *left.add(i);

            if l_ch == b'\0' {
                return std::cmp::Ordering::Less;
            }

            match l_ch.to_ascii_lowercase().cmp(&r_ch.to_ascii_lowercase()) {
                std::cmp::Ordering::Equal => continue,
                value => return value,
            }
        }

        if *left.add(right.len()) == b'\0' {
            std::cmp::Ordering::Equal
        } else {
            std::cmp::Ordering::Greater
        }
    }
}

pub unsafe fn strerror(n: c_int) -> *mut u8 {
    unsafe { ::libc::strerror(n).cast() }
}

pub unsafe fn streq_(left: *const u8, right: &'static str) -> bool {
    unsafe { matches!(strcmp_(left, right), std::cmp::Ordering::Equal) }
}

pub unsafe fn strcaseeq_(left: *const u8, right: &'static str) -> bool {
    unsafe { matches!(strcasecmp_(left, right), std::cmp::Ordering::Equal) }
}

pub unsafe fn ttyname(fd: i32) -> *mut u8 {
    unsafe { ::libc::ttyname(fd).cast() }
}

pub(crate) fn basename(path: &str) -> &str {
    std::path::Path::new(path)
        .file_name()
        .expect("TODO properly handle this case with ..")
        .to_str()
        .expect("should always be utf8")
}

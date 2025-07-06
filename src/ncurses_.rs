use ::core::ffi::c_char;

pub const ERR: i32 = -1;
pub const OK: i32 = 0;

#[allow(clippy::upper_case_acronyms)]
#[repr(C)]
#[derive(Copy, Clone)]
pub struct TERMINAL {
    _opaque: [u8; 0],
}

// note I've modified the bindings under the assumption that
// *c_char can be mixed with *u8
#[link(name = "ncurses")]
unsafe extern "C" {
    pub fn setupterm(term: *const u8, filedes: i32, errret: *mut i32) -> i32;

    pub fn tiparm_s(expected: i32, mask: i32, str: *const u8, ...) -> *mut u8;
    pub fn tiparm(str: *const u8, ...) -> *mut u8;
    pub fn tparm(str: *const u8, ...) -> *mut u8;

    pub fn tigetflag(cap_code: *const u8) -> i32;
    pub fn tigetnum(cap_code: *const u8) -> i32;
    pub fn tigetstr(cap_code: *const u8) -> *mut u8;

    pub fn del_curterm(oterm: *mut TERMINAL) -> i32;
    pub static mut cur_term: *mut TERMINAL;
}

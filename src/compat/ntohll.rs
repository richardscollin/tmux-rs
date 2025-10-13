use ::libc;
pub type __uint32_t = libc::c_uint;
pub type __uint64_t = libc::c_ulong;
pub type uint32_t = __uint32_t;
pub type uint64_t = __uint64_t;
#[inline]
unsafe fn __bswap_32(mut __bsx: __uint32_t) -> __uint32_t {
    return (__bsx & 0xff000000 as libc::c_uint) >> 24 as libc::c_int
        | (__bsx & 0xff0000 as libc::c_uint) >> 8 as libc::c_int
        | (__bsx & 0xff00 as libc::c_uint) << 8 as libc::c_int
        | (__bsx & 0xff as libc::c_uint) << 24 as libc::c_int;
}

pub unsafe fn ntohll(mut v: uint64_t) -> uint64_t {
    let mut b: uint32_t = 0;
    let mut t: uint32_t = 0;
    b = __bswap_32((v & 0xffffffff as libc::c_uint as libc::c_ulong) as __uint32_t);
    t = __bswap_32((v >> 32 as libc::c_int) as __uint32_t);
    return (b as uint64_t) << 32 as libc::c_int | t as libc::c_ulong;
}

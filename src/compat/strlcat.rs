use crate::libc;

/// The strlcat() function appends the NUL-terminated string src to the end of dst.
/// It will append at most size - strlen(dst) - 1 bytes, NUL-terminating the result.
pub unsafe fn strlcat(dst: *mut u8, src: *const u8, size: usize) -> usize {
    unsafe {
        let dst_strlen = libc::strnlen(dst, size);
        let src_strlen = libc::strnlen(src, size.saturating_sub(dst_strlen).saturating_sub(1));

        core::ptr::copy_nonoverlapping(src, dst.add(dst_strlen), src_strlen);
        *dst.add(dst_strlen + src_strlen) = b'\0';

        dst_strlen + src_strlen
    }
}

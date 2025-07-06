// Copyright (c) 2016 Nicholas Marriott <nicholas.marriott@gmail.com>
//
// Permission to use, copy, modify, and distribute this software for any
// purpose with or without fee is hereby granted, provided that the above
// copyright notice and this permission notice appear in all copies.
//
// THE SOFTWARE IS PROVIDED "AS IS" AND THE AUTHOR DISCLAIMS ALL WARRANTIES
// WITH REGARD TO THIS SOFTWARE INCLUDING ALL IMPLIED WARRANTIES OF
// MERCHANTABILITY AND FITNESS. IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR
// ANY SPECIAL, DIRECT, INDIRECT, OR CONSEQUENTIAL DAMAGES OR ANY DAMAGES
// WHATSOEVER RESULTING FROM LOSS OF MIND, USE, DATA OR PROFITS, WHETHER
// IN AN ACTION OF CONTRACT, NEGLIGENCE OR OTHER TORTIOUS ACTION, ARISING
// OUT OF OR IN CONNECTION WITH THE USE OR PERFORMANCE OF THIS SOFTWARE.
use core::ffi::c_char;

// a custom version of setproctitle which just supports our usage:
// setproctitle( c!("%s (%s)"), name, socket_path);
#[cfg(target_os = "linux")]
pub unsafe fn setproctitle_(_fmt: *const c_char, name: *const c_char, socket_path: *const u8) {
    unsafe {
        let mut name: [c_char; 16] = [0; 16];

        let used = libc::snprintf(
            &raw mut name as *mut c_char,
            name.len(),
            c"%s: %s (%s)".as_ptr(),
            getprogname(),
            &raw const name as *const c_char,
            socket_path,
        );
        if used >= name.len() as i32 {
            let cp = libc::strrchr(&raw const name as *const c_char, b' ' as i32);
            if !cp.is_null() {
                *cp = b'\0' as c_char;
            }
        }
        libc::prctl(libc::PR_SET_NAME, &raw const name as *const c_char);
    }
}

#[cfg(target_os = "macos")]
pub unsafe fn setproctitle_(_: *const c_char, _: *const c_char, _: *const c_char) {}

fn getprogname() -> *const c_char {
    c"tmux".as_ptr()
}

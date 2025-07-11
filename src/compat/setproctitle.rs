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

// a custom version of setproctitle which just supports our usage:
// setproctitle( c!("%s (%s)"), name, socket_path);
#[cfg(target_os = "linux")]
pub unsafe fn setproctitle_(_fmt: *const u8, name: *const u8, socket_path: *const u8) {
    use crate::libc;

    unsafe {
        let mut title: [u8; 16] = [0; 16];

        let used = libc::snprintf(
            &raw mut title as _,
            title.len(),
            c"%s: %s (%s)".as_ptr(),
            getprogname(),
            name,
            socket_path,
        );
        if used >= title.len() as i32 {
            let cp = libc::strrchr(&raw const title as *const u8, b' ' as i32);
            if !cp.is_null() {
                *cp = b'\0';
            }
        }
        libc::prctl(libc::PR_SET_NAME, &raw const title as *const u8);
    }
}

#[cfg(target_os = "macos")]
pub unsafe fn setproctitle_(_: *const u8, _: *const u8, _: *const u8) {}

fn getprogname() -> *const u8 {
    c"tmux".as_ptr().cast()
}

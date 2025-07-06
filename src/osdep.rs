// Copyright (c) 2009 Nicholas Marriott <nicholas.marriott@gmail.com>
// Copyright (c) 2009 Joshua Elsasser <josh@elsasser.org>
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

use crate::*;

// this is for osdep-linux.c

#[cfg(target_os = "linux")]
pub unsafe fn osdep_get_name(fd: i32, tty: *const u8) -> *mut u8 {
    unsafe {
        let pgrp = libc::tcgetpgrp(fd);
        if pgrp == -1 {
            return null_mut();
        }

        let mut path = format_nul!("/proc/{pgrp}/cmdline");
        let f = fopen(path, c!("r"));
        if f.is_null() {
            free_(path);
            return null_mut();
        }
        free_(path);

        let mut len = 0;
        let mut buf: *mut u8 = null_mut();

        loop {
            let ch = libc::fgetc(f);
            if ch == libc::EOF {
                break;
            }
            if ch == b'\0' as i32 {
                break;
            }
            buf = xrealloc_(buf, len + 2).as_ptr();
            *buf.add(len) = ch as u8;
            len += 1;
        }
        if !buf.is_null() {
            *buf.add(len) = b'\0';
        }

        fclose(f);
        buf
    }
}

#[cfg(target_os = "linux")]
pub unsafe fn osdep_get_cwd(fd: i32) -> *const u8 {
    const MAXPATHLEN: usize = libc::PATH_MAX as usize;
    static mut target_buffer: [u8; MAXPATHLEN + 1] = [0; MAXPATHLEN + 1];
    unsafe {
        let target = &raw mut target_buffer as *mut u8;

        let pgrp = libc::tcgetpgrp(fd);
        if pgrp == -1 {
            return null_mut();
        }

        let mut path = format_nul!("/proc/{pgrp}/cwd");
        let mut n = libc::readlink(path.cast(), target.cast(), MAXPATHLEN);
        free_(path);

        let mut sid: pid_t = 0;
        if n == -1 && libc::ioctl(fd, libc::TIOCGSID, &raw mut sid) != -1 {
            path = format_nul!("/proc/{sid}/cwd");
            n = libc::readlink(path.cast(), target.cast(), MAXPATHLEN);
            free_(path);
        }

        if n > 0 {
            *target.add(n as usize) = b'\0';
            return target;
        }
        null_mut()
    }
}

#[cfg(target_os = "linux")]
pub unsafe fn osdep_event_init() -> *mut event_base {
    unsafe {
        // On Linux, epoll doesn't work on /dev/null (yes, really).
        libc::setenv(c!("EVENT_NOEPOLL"), c!("1"), 1);

        let base = event_init();
        libc::unsetenv(c!("EVENT_NOEPOLL"));
        base
    }
}

// osdep darwin

#[cfg(target_os = "macos")]
pub unsafe fn osdep_get_name(fd: i32, tty: *const u8) -> *mut u8 {
    // note only bothering to port the version for > Mac OS X 10.7 SDK or later
    unsafe {
        use libc::proc_pidinfo;

        let mut bsdinfo: proc_bsdshortinfo = zeroed();
        let mut pgrp: pid_t = libc::tcgetpgrp(fd);
        if pgrp == -1 {
            return null_mut();
        }

        const PROC_PIDT_SHORTBSDINFO: i32 = 13;
        // abi compatible version of struct defined in sys/proc_info.h
        #[repr(C)]
        struct proc_bsdshortinfo {
            padding1: [u32; 4],
            pbsi_comm: [u8; 16],
            padding2: [u32; 8],
        }

        let mut ret = proc_pidinfo(
            pgrp,
            PROC_PIDT_SHORTBSDINFO as _,
            0,
            (&raw mut bsdinfo).cast(),
            size_of::<proc_bsdshortinfo>() as _,
        );
        if (ret == size_of::<proc_bsdshortinfo>() as _ && bsdinfo.pbsi_comm[0] != b'\0') {
            return libc::strdup((&raw const bsdinfo.pbsi_comm).cast());
        }
        null_mut()
    }
}

#[cfg(target_os = "macos")]
pub unsafe fn osdep_get_cwd(fd: i32) -> *const u8 {
    static mut wd: [u8; libc::PATH_MAX as usize] = [0; libc::PATH_MAX as usize];
    unsafe {
        let mut pathinfo: libc::proc_vnodepathinfo = zeroed();

        let mut pgrp: pid_t = libc::tcgetpgrp(fd);
        if pgrp == -1 {
            return null_mut();
        }

        let ret = libc::proc_pidinfo(
            pgrp,
            libc::PROC_PIDVNODEPATHINFO as _,
            0,
            (&raw mut pathinfo).cast(),
            size_of::<libc::proc_vnodepathinfo>() as _,
        );
        if ret == size_of::<libc::proc_vnodepathinfo>() as i32 {
            crate::compat::strlcpy(
                &raw mut wd as *mut u8,
                &raw const pathinfo.pvi_cdir.vip_path as *const u8,
                libc::PATH_MAX as usize,
            );
            return &raw const wd as *const u8;
        }

        null_mut()
    }
}

#[cfg(target_os = "macos")]
pub unsafe fn osdep_event_init() -> *mut event_base {
    unsafe {
        /*
         * On OS X, kqueue and poll are both completely broken and don't
         * work on anything except socket file descriptors (yes, really).
         */
        crate::libc::setenv(c!("EVENT_NOKQUEUE"), c!("1"), 1);
        crate::libc::setenv(c!("EVENT_NOPOLL"), c!("1"), 1);

        let mut base: *mut event_base = event_init();
        crate::libc::unsetenv(c!("EVENT_NOKQUEUE"));
        crate::libc::unsetenv(c!("EVENT_NOPOLL"));

        base
    }
}

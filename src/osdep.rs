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
#[cfg(not(target_os = "windows"))]
use crate::libc;
use crate::*;

#[cfg(target_os = "linux")]
pub unsafe fn osdep_get_name(fd: i32, _tty: *const u8) -> *mut u8 {
    unsafe {
        let pgrp = libc::tcgetpgrp(fd);
        if pgrp == -1 {
            return null_mut();
        }

        let path = format_nul!("/proc/{pgrp}/cmdline");
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
pub unsafe fn osdep_get_cwd(fd: i32, _pid: pid_t) -> Option<String> {
    const MAXPATHLEN: usize = libc::PATH_MAX as usize;
    let mut buf = [0u8; MAXPATHLEN + 1];
    unsafe {
        let pgrp = libc::tcgetpgrp(fd);
        if pgrp == -1 {
            return None;
        }

        let mut path = format_nul!("/proc/{pgrp}/cwd");
        let mut n = libc::readlink(path.cast(), buf.as_mut_ptr().cast(), MAXPATHLEN);
        free_(path);

        let mut sid: pid_t = 0;
        if n == -1 && libc::ioctl(fd, libc::TIOCGSID, &raw mut sid) != -1 {
            path = format_nul!("/proc/{sid}/cwd");
            n = libc::readlink(path.cast(), buf.as_mut_ptr().cast(), MAXPATHLEN);
            free_(path);
        }

        if n > 0 {
            return Some(String::from_utf8_lossy(&buf[..n as usize]).into_owned());
        }
        None
    }
}

#[cfg(target_os = "linux")]
pub unsafe fn osdep_event_init() -> *mut event_base {
    unsafe {
        // On Linux, epoll doesn't work on /dev/null (yes, really).
        std::env::set_var("EVENT_NOEPOLL", "1");

        let base = event_init();

        std::env::remove_var("EVENT_NOEPOLL");

        base
    }
}

#[cfg(target_os = "windows")]
pub unsafe fn osdep_get_name(_fd: i32, _tty: *const u8) -> *mut u8 {
    null_mut()
}

#[cfg(target_os = "macos")]
pub unsafe fn osdep_get_name(fd: i32, _tty: *const u8) -> *mut u8 {
    // note only bothering to port the version for > Mac OS X 10.7 SDK or later
    unsafe {
        use libc::proc_pidinfo;

        let mut bsdinfo: proc_bsdshortinfo = zeroed();
        let pgrp: pid_t = libc::tcgetpgrp(fd);
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

        let ret = proc_pidinfo(
            pgrp,
            PROC_PIDT_SHORTBSDINFO as _,
            0,
            (&raw mut bsdinfo).cast(),
            size_of::<proc_bsdshortinfo>() as _,
        );
        if ret == size_of::<proc_bsdshortinfo>() as _ && bsdinfo.pbsi_comm[0] != b'\0' {
            return libc::strdup((&raw const bsdinfo.pbsi_comm).cast());
        }
        null_mut()
    }
}

#[cfg(target_os = "macos")]
pub unsafe fn osdep_get_cwd(fd: i32, _pid: pid_t) -> Option<String> {
    unsafe {
        let mut pathinfo: libc::proc_vnodepathinfo = zeroed();

        let pgrp: pid_t = libc::tcgetpgrp(fd);
        if pgrp == -1 {
            return None;
        }

        let ret = libc::proc_pidinfo(
            pgrp,
            libc::PROC_PIDVNODEPATHINFO as _,
            0,
            (&raw mut pathinfo).cast(),
            size_of::<libc::proc_vnodepathinfo>() as _,
        );
        if ret == size_of::<libc::proc_vnodepathinfo>() as i32 {
            let path = &pathinfo.pvi_cdir.vip_path;
            let path_bytes: &[u8] = std::slice::from_raw_parts(path.as_ptr().cast(), path.len());
            let len = path_bytes.iter().position(|&b| b == 0).unwrap_or(path_bytes.len());
            return Some(String::from_utf8_lossy(&path_bytes[..len]).into_owned());
        }

        None
    }
}

#[cfg(target_os = "macos")]
pub unsafe fn osdep_event_init() -> *mut event_base {
    unsafe {
        // On OS X, kqueue and poll are both completely broken and don't
        // work on anything except socket file descriptors (yes, really).
        std::env::set_var("EVENT_NOKQUEUE", "1");
        std::env::set_var("EVENT_NOPOLL", "1");

        let base: *mut event_base = event_init();

        std::env::remove_var("EVENT_NOKQUEUE");
        std::env::remove_var("EVENT_NOPOLL");

        base
    }
}


#[cfg(target_os = "windows")]
pub unsafe fn osdep_get_cwd(_fd: i32, pid: pid_t) -> Option<String> {
    use windows_sys::Wdk::System::Threading::{NtQueryInformationProcess, ProcessBasicInformation};
    use windows_sys::Win32::Foundation::{CloseHandle, UNICODE_STRING};
    use windows_sys::Win32::System::Diagnostics::Debug::ReadProcessMemory;
    use windows_sys::Win32::System::Threading::{
        OpenProcess, PROCESS_BASIC_INFORMATION, PROCESS_QUERY_INFORMATION, PROCESS_VM_READ,
    };

    const MAX_PATH_BUF: usize = 1024;

    // CurrentDirectory.DosPath offset within RTL_USER_PROCESS_PARAMETERS.
    // windows-sys hides this field behind Reserved2, so we use the raw offset.
    const CURDIR_DOS_PATH_OFFSET: usize = 0x38;

    unsafe {
        if pid <= 0 {
            return None;
        }

        let handle = OpenProcess(
            PROCESS_QUERY_INFORMATION | PROCESS_VM_READ,
            false.into(),
            pid as u32,
        );
        if handle.is_null() {
            return None;
        }

        let result = (|| -> Option<String> {
            // Step 1: Get PEB address via NtQueryInformationProcess.
            let mut pbi: PROCESS_BASIC_INFORMATION = zeroed();
            let status = NtQueryInformationProcess(
                handle,
                ProcessBasicInformation,
                (&raw mut pbi).cast(),
                size_of::<PROCESS_BASIC_INFORMATION>() as u32,
                null_mut(),
            );
            if status < 0 {
                return None;
            }
            let peb_addr = pbi.PebBaseAddress as usize;

            // Step 2: Read ProcessParameters pointer from PEB.
            let params_ptr_offset =
                peb_addr + core::mem::offset_of!(windows_sys::Win32::System::Threading::PEB, ProcessParameters);
            let mut process_params_addr: usize = 0;
            if ReadProcessMemory(
                handle,
                params_ptr_offset as *const _,
                (&raw mut process_params_addr).cast(),
                size_of::<usize>(),
                null_mut(),
            ) == 0
            {
                return None;
            }

            // Step 3: Read CurrentDirectoryPath UNICODE_STRING from
            // RTL_USER_PROCESS_PARAMETERS.
            let cwd_ustr_offset = process_params_addr + CURDIR_DOS_PATH_OFFSET;
            let mut ustr: UNICODE_STRING = zeroed();
            if ReadProcessMemory(
                handle,
                cwd_ustr_offset as *const _,
                (&raw mut ustr).cast(),
                size_of::<UNICODE_STRING>(),
                null_mut(),
            ) == 0
            {
                return None;
            }

            let wchar_len = ustr.Length as usize / 2;
            if wchar_len == 0 || ustr.Buffer.is_null() {
                return None;
            }

            // Step 4: Read the UTF-16 path buffer.
            let mut wbuf = [0u16; MAX_PATH_BUF];
            let read_len = wchar_len.min(MAX_PATH_BUF - 1);
            if ReadProcessMemory(
                handle,
                ustr.Buffer as *const _,
                wbuf.as_mut_ptr().cast(),
                read_len * 2,
                null_mut(),
            ) == 0
            {
                return None;
            }

            // Strip trailing backslash (unless root like "C:\").
            let mut len = read_len;
            if len > 3 && wbuf[len - 1] == b'\\' as u16 {
                len -= 1;
            }

            Some(String::from_utf16_lossy(&wbuf[..len]))
        })();

        CloseHandle(handle);
        result
    }
}

#[cfg(target_os = "windows")]
pub unsafe fn osdep_event_init() -> *mut event_base {
    unsafe { event_init() }
}

#![allow(unused)]

use ::core::{
    ffi::{c_int, c_void},
    ptr::{NonNull, null_mut},
};
use ::libc::{
    CMSG_DATA, CMSG_FIRSTHDR, CMSG_LEN, CMSG_SPACE, EAGAIN, EBADMSG, EINTR, EINVAL, ENOBUFS,
    ERANGE, SCM_RIGHTS, SOL_SOCKET, abort, c_uchar, calloc, close, cmsghdr, free, iovec, memcpy,
    memset, msghdr, sendmsg, writev,
};

use super::imsg::{ibuf, msgbuf};
use super::queue::{
    tailq_first, tailq_foreach, tailq_init, tailq_insert_tail, tailq_next, tailq_remove,
};
use super::{freezero, recallocarray::recallocarray};
use crate::errno;

const IOV_MAX: usize = 1024; // TODO find where IOV_MAX is defined

pub unsafe fn ibuf_open(len: usize) -> *mut ibuf {
    unsafe {
        if len == 0 {
            errno!() = EINVAL;
            return null_mut();
        }
        let buf: *mut ibuf = calloc(1, size_of::<ibuf>()) as *mut ibuf;
        if buf.is_null() {
            return null_mut();
        }
        (*buf).buf = calloc(len, 1) as *mut c_uchar;
        if (*buf).buf.is_null() {
            free(buf as *mut c_void);
            return null_mut();
        }

        (*buf).max = len;
        (*buf).size = len;
        (*buf).fd = -1;

        buf
    }
}

pub unsafe fn ibuf_dynamic(len: usize, max: usize) -> *mut ibuf {
    unsafe {
        if len == 0 || max < len {
            errno!() = EINVAL;
            return null_mut();
        }
        let buf: *mut ibuf = calloc(1, size_of::<ibuf>()) as *mut ibuf;
        if buf.is_null() {
            return null_mut();
        }
        if len > 0 {
            (*buf).buf = calloc(len, 1) as *mut c_uchar;
            if (*buf).buf.is_null() {
                free(buf as *mut c_void);
                return null_mut();
            }
        }
        (*buf).max = len;
        (*buf).size = len;
        (*buf).fd = -1;

        buf
    }
}

pub unsafe fn ibuf_realloc(buf: *mut ibuf, len: usize) -> i32 {
    unsafe {
        // on static buffers max is eq size and so the following fails
        if len > usize::MAX - (*buf).wpos || (*buf).wpos + len > (*buf).max {
            errno!() = ERANGE;
            return -1;
        }

        let b = recallocarray((*buf).buf as *mut c_void, (*buf).size, (*buf).wpos + len, 1);
        if b.is_null() {
            return -1;
        }
        (*buf).buf = b as *mut u8;
        (*buf).size = (*buf).wpos + len;

        0
    }
}

pub unsafe fn ibuf_reserve(buf: *mut ibuf, len: usize) -> *mut c_void {
    unsafe {
        if len > usize::MAX - (*buf).wpos || (*buf).max == 0 {
            errno!() = ERANGE;
            return null_mut();
        }

        if (*buf).wpos + len > (*buf).size && ibuf_realloc(buf, len) == -1 {
            return null_mut();
        }

        let b = (*buf).buf.add((*buf).wpos);
        (*buf).wpos += len;
        b as *mut c_void
    }
}

pub unsafe fn ibuf_add(buf: *mut ibuf, data: *const c_void, len: usize) -> i32 {
    unsafe {
        let b = ibuf_reserve(buf, len);
        if b.is_null() {
            return -1;
        }

        memcpy(b, data, len);
        0
    }
}

pub unsafe fn ibuf_add_ibuf(buf: *mut ibuf, from: *const ibuf) -> c_int {
    unsafe { ibuf_add(buf, ibuf_data(from), ibuf_size(from)) }
}

pub unsafe fn ibuf_add_buf(buf: *mut ibuf, from: *const ibuf) -> c_int {
    unsafe { ibuf_add_ibuf(buf, from) }
}

pub unsafe fn ibuf_add_n8(buf: *mut ibuf, value: u64) -> c_int {
    unsafe {
        if value > u8::MAX as u64 {
            errno!() = EINVAL;
            return -1;
        }
        let v = value;
        ibuf_add(buf, &raw const v as _, size_of::<u8>())
    }
}

pub unsafe fn ibuf_add_n16(buf: *mut ibuf, value: u64) -> c_int {
    unsafe {
        if value > u16::MAX as u64 {
            errno!() = EINVAL;
            return -1;
        }
        let v = (value as u16).to_be();
        ibuf_add(buf, &raw const v as _, size_of::<u16>())
    }
}

pub unsafe fn ibuf_add_n32(buf: *mut ibuf, value: u64) -> c_int {
    unsafe {
        if value > u32::MAX as u64 {
            errno!() = EINVAL;
            return -1;
        }
        let v = (value as u32).to_be();
        ibuf_add(buf, &raw const v as _, size_of::<u32>())
    }
}

pub unsafe fn ibuf_add_n64(buf: *mut ibuf, value: u64) -> c_int {
    unsafe {
        let v = value.to_be();
        ibuf_add(buf, &raw const v as _, size_of::<u64>())
    }
}

pub unsafe fn ibuf_add_h16(buf: *mut ibuf, value: u64) -> c_int {
    unsafe {
        if value > u16::MAX as u64 {
            errno!() = EINVAL;
            return -1;
        }
        let v = value as u16;
        ibuf_add(buf, &raw const v as _, size_of::<u16>())
    }
}

pub unsafe fn ibuf_add_h32(buf: *mut ibuf, value: u64) -> c_int {
    unsafe {
        if value > u32::MAX as u64 {
            errno!() = EINVAL;
            return -1;
        }
        let v = value as u32;
        ibuf_add(buf, &raw const v as _, size_of::<u32>())
    }
}

pub unsafe fn ibuf_add_h64(buf: *mut ibuf, value: u64) -> c_int {
    unsafe { ibuf_add(buf, &raw const value as *const c_void, size_of::<u64>()) }
}

pub unsafe fn ibuf_add_zero(buf: *mut ibuf, len: usize) -> c_int {
    unsafe {
        let b: *mut c_void = ibuf_reserve(buf, len);
        if b.is_null() {
            return -1;
        }
        memset(b, 0, len);
        0
    }
}

pub unsafe fn ibuf_seek(buf: *mut ibuf, pos: usize, len: usize) -> *mut c_void {
    unsafe {
        // only allow seeking between rpos and wpos
        if ibuf_size(buf) < pos || usize::MAX - pos < len || ibuf_size(buf) < pos + len {
            errno!() = ERANGE;
            return null_mut();
        }

        (*buf).buf.add((*buf).rpos + pos) as _
    }
}

pub unsafe fn ibuf_set(buf: *mut ibuf, pos: usize, data: *const c_void, len: usize) -> c_int {
    unsafe {
        let b = ibuf_seek(buf, pos, len);
        if b.is_null() {
            return -1;
        }

        memcpy(b, data, len);
        0
    }
}

pub unsafe fn ibuf_set_n8(buf: *mut ibuf, pos: usize, value: u64) -> c_int {
    unsafe {
        if value > u8::MAX as u64 {
            errno!() = EINVAL;
            return -1;
        }
        let v = value as u8;
        ibuf_set(buf, pos, &raw const v as *const c_void, size_of::<u8>())
    }
}

pub unsafe fn ibuf_set_n16(buf: *mut ibuf, pos: usize, value: u64) -> c_int {
    unsafe {
        if value > u16::MAX as u64 {
            errno!() = EINVAL;
            return -1;
        }
        let v = u16::to_be(value as u16);
        ibuf_set(buf, pos, &raw const v as *const c_void, size_of::<u16>())
    }
}

pub unsafe fn ibuf_set_n32(buf: *mut ibuf, pos: usize, value: u64) -> c_int {
    unsafe {
        if value > u32::MAX as u64 {
            errno!() = EINVAL;
            return -1;
        }
        let v = u32::to_be(value as u32);
        ibuf_set(buf, pos, &raw const v as *const c_void, size_of::<u32>())
    }
}

pub unsafe fn ibuf_set_n64(buf: *mut ibuf, pos: usize, value: u64) -> c_int {
    unsafe {
        let v = u64::to_be(value);
        ibuf_set(buf, pos, &raw const v as *const c_void, size_of::<u64>())
    }
}

pub unsafe fn ibuf_set_h16(buf: *mut ibuf, pos: usize, value: u64) -> c_int {
    unsafe {
        if value > u16::MAX as u64 {
            errno!() = EINVAL;
            return -1;
        }
        let v = value as u16;
        ibuf_set(buf, pos, &raw const v as *const c_void, size_of::<u16>())
    }
}

pub unsafe fn ibuf_set_h32(buf: *mut ibuf, pos: usize, value: u64) -> c_int {
    unsafe {
        if value > u32::MAX as u64 {
            errno!() = EINVAL;
            return -1;
        }
        let v = value as u32;
        ibuf_set(buf, pos, &raw const v as *const c_void, size_of::<u32>())
    }
}

pub unsafe fn ibuf_set_h64(buf: *mut ibuf, pos: usize, value: u64) -> c_int {
    unsafe {
        ibuf_set(
            buf,
            pos,
            &raw const value as *const c_void,
            size_of::<u64>(),
        )
    }
}

pub unsafe fn ibuf_data(buf: *const ibuf) -> *mut c_void {
    unsafe { (*buf).buf.add((*buf).rpos) as *mut c_void }
}

pub unsafe fn ibuf_size(buf: *const ibuf) -> usize {
    unsafe { (*buf).wpos - (*buf).rpos }
}

pub unsafe fn ibuf_left(buf: *const ibuf) -> usize {
    unsafe {
        if (*buf).max == 0 {
            return 0;
        }
        (*buf).max - (*buf).wpos
    }
}

pub unsafe fn ibuf_truncate(buf: *mut ibuf, len: usize) -> c_int {
    unsafe {
        if ibuf_size(buf) >= len {
            (*buf).wpos = (*buf).rpos + len;
            return 0;
        }
        if (*buf).max == 0 {
            // only allow to truncate down
            errno!() = ERANGE;
            return -1;
        }
        ibuf_add_zero(buf, len - ibuf_size(buf))
    }
}

pub unsafe fn ibuf_rewind(buf: *mut ibuf) {
    unsafe {
        (*buf).rpos = 0;
    }
}

pub unsafe fn ibuf_close(msgbuf: *mut msgbuf, buf: *mut ibuf) {
    unsafe {
        ibuf_enqueue(msgbuf, buf);
    }
}

pub unsafe fn ibuf_from_buffer(buf: *mut ibuf, data: *mut c_void, len: usize) {
    unsafe {
        memset(buf as _, 0, size_of::<ibuf>());
        (*buf).buf = data as _;
        (*buf).wpos = len;
        (*buf).size = len;
        (*buf).fd = -1;
    }
}

pub unsafe fn ibuf_from_ibuf(buf: *mut ibuf, from: *const ibuf) {
    unsafe {
        ibuf_from_buffer(buf, ibuf_data(from), ibuf_size(from));
    }
}

pub unsafe fn ibuf_get(buf: *mut ibuf, data: *mut c_void, len: usize) -> c_int {
    unsafe {
        if ibuf_size(buf) < len {
            errno!() = EBADMSG;
            return -1;
        }

        memcpy(data, ibuf_data(buf), len);
        (*buf).rpos += len;
        0
    }
}

pub unsafe fn ibuf_get_ibuf(buf: *mut ibuf, len: usize, new: *mut ibuf) -> c_int {
    unsafe {
        if ibuf_size(buf) < len {
            errno!() = EBADMSG;
            return -1;
        }

        ibuf_from_buffer(new, ibuf_data(buf), len);
        (*buf).rpos += len;
        0
    }
}

pub unsafe fn ibuf_get_n8(buf: *mut ibuf, value: *mut u8) -> c_int {
    unsafe { ibuf_get(buf, value as _, size_of::<u8>()) }
}

pub unsafe fn ibuf_get_n16(buf: *mut ibuf, value: *mut u16) -> c_int {
    unsafe {
        let rv = ibuf_get(buf, value as _, size_of::<u16>());
        *value = u16::from_be(*value);
        rv
    }
}

pub unsafe fn ibuf_get_n32(buf: *mut ibuf, value: *mut u32) -> c_int {
    unsafe {
        let rv = ibuf_get(buf, value as _, size_of::<u32>());
        *value = u32::from_be(*value);
        rv
    }
}

pub unsafe fn ibuf_get_n64(buf: *mut ibuf, value: *mut u64) -> c_int {
    unsafe {
        let rv = ibuf_get(buf, value as _, size_of::<u64>());
        *value = u64::from_be(*value);
        rv
    }
}

pub unsafe fn ibuf_get_h16(buf: *mut ibuf, value: *mut u16) -> c_int {
    unsafe { ibuf_get(buf, value as _, size_of::<u16>()) }
}

pub unsafe fn ibuf_get_h32(buf: *mut ibuf, value: *mut u32) -> c_int {
    unsafe { ibuf_get(buf, value as _, size_of::<u32>()) }
}

pub unsafe fn ibuf_get_h64(buf: *mut ibuf, value: *mut u64) -> c_int {
    unsafe { ibuf_get(buf, value as _, size_of::<u64>()) }
}

pub unsafe fn ibuf_skip(buf: *mut ibuf, len: usize) -> c_int {
    unsafe {
        if ibuf_size(buf) < len {
            errno!() = EBADMSG;
            return -1;
        }

        (*buf).rpos += len;
        0
    }
}

pub unsafe fn ibuf_free(buf: *mut ibuf) {
    unsafe {
        if buf.is_null() {
            return;
        }
        if (*buf).max == 0 {
            // if buf lives on the stack
            abort(); /* abort before causing more harm */
        }
        if (*buf).fd != -1 {
            close((*buf).fd);
        }
        freezero((*buf).buf.cast(), (*buf).size);
        free(buf as *mut c_void);
    }
}

pub unsafe fn ibuf_fd_avail(buf: *mut ibuf) -> c_int {
    unsafe { ((*buf).fd != -1) as c_int }
}

pub unsafe fn ibuf_fd_get(buf: *mut ibuf) -> c_int {
    unsafe {
        let fd = (*buf).fd;
        (*buf).fd = -1;
        fd
    }
}

pub unsafe fn ibuf_fd_set(buf: *mut ibuf, fd: c_int) {
    unsafe {
        if (*buf).max == 0 {
            // if buf lives on the stack
            abort(); /* abort before causing more harm */
        }
        if (*buf).fd != -1 {
            close((*buf).fd);
        }
        (*buf).fd = fd;
    }
}

pub unsafe fn ibuf_write(msgbuf: *mut msgbuf) -> c_int {
    unsafe {
        let mut i: u32 = 0;

        let mut iov: [iovec; IOV_MAX] = [const {
            iovec {
                iov_base: null_mut(),
                iov_len: 0,
            }
        }; IOV_MAX];
        for buf in tailq_foreach(&raw mut (*msgbuf).bufs).map(NonNull::as_ptr) {
            if i as usize >= IOV_MAX {
                break;
            }
            iov[i as usize].iov_base = ibuf_data(buf);
            iov[i as usize].iov_len = ibuf_size(buf);
            i += 1;
        }
        if i == 0 {
            return 0;
        }

        let mut n: isize;
        'again: loop {
            n = writev((*msgbuf).fd, iov.as_ptr(), i as i32);
            if n == -1 {
                if errno!() == EINTR {
                    continue 'again;
                }
                if errno!() == ENOBUFS {
                    errno!() = EAGAIN;
                }
                return -1;
            }

            break 'again; // need a break here to emulate goto
        }

        if n == 0 {
            // connection closed
            errno!() = 0;
            return 0;
        }

        msgbuf_drain(msgbuf, n as usize);

        1
    }
}

pub unsafe fn msgbuf_init(msgbuf: *mut msgbuf) {
    unsafe {
        (*msgbuf).queued = 0;
        (*msgbuf).fd = -1;
        tailq_init(&raw mut (*msgbuf).bufs);
    }
}

unsafe fn msgbuf_drain(msgbuf: *mut msgbuf, mut n: usize) {
    unsafe {
        let mut buf = tailq_first(&raw mut (*msgbuf).bufs);

        while !buf.is_null() && n > 0 {
            let next = tailq_next(buf);
            if n >= ibuf_size(buf) {
                n -= ibuf_size(buf);
                ibuf_dequeue(msgbuf, buf);
            } else {
                (*buf).rpos += n;
                n = 0;
            }
            buf = next;
        }
    }
}

pub unsafe fn msgbuf_clear(msgbuf: *mut msgbuf) {
    unsafe {
        let mut buf;
        while {
            buf = tailq_first(&raw mut (*msgbuf).bufs);
            !buf.is_null()
        } {
            ibuf_dequeue(msgbuf, buf);
        }
    }
}

pub unsafe fn msgbuf_write(msgbuf: *mut msgbuf) -> c_int {
    unsafe {
        let mut iov: [iovec; IOV_MAX] = std::mem::zeroed();
        let mut buf0: *mut ibuf = null_mut();
        let mut i: u32 = 0;
        let mut msg: msghdr = std::mem::zeroed();
        let mut cmsgbuf: cmsgbuf = std::mem::zeroed();
        union cmsgbuf {
            _hdr: cmsghdr,
            buf: [u8; unsafe { CMSG_SPACE(size_of::<c_int>() as _) as usize }],
        }

        for buf in tailq_foreach(&raw mut (*msgbuf).bufs).map(NonNull::as_ptr) {
            if i as usize >= IOV_MAX {
                break;
            }
            if i > 0 && (*buf).fd != -1 {
                break;
            }
            iov[i as usize].iov_base = ibuf_data(buf);
            iov[i as usize].iov_len = ibuf_size(buf);
            i += 1;
            if (*buf).fd != -1 {
                buf0 = buf;
            }
        }

        msg.msg_iov = iov.as_mut_ptr();
        msg.msg_iovlen = i.try_into().unwrap();

        if !buf0.is_null() {
            msg.msg_control = &raw mut cmsgbuf.buf as _;
            msg.msg_controllen = size_of_val(&cmsgbuf.buf) as _;
            let cmsg = CMSG_FIRSTHDR(&raw const msg);
            (*cmsg).cmsg_len = CMSG_LEN(size_of::<c_int>() as u32) as _;
            (*cmsg).cmsg_level = SOL_SOCKET;
            (*cmsg).cmsg_type = SCM_RIGHTS;
            *CMSG_DATA(cmsg).cast() = (*buf0).fd;
        }

        let mut n: isize;
        'again: loop {
            n = sendmsg((*msgbuf).fd, &raw const msg, 0);
            if n == -1 {
                if errno!() == EINTR {
                    continue 'again;
                }
                if errno!() == ENOBUFS {
                    errno!() = EAGAIN;
                }
                return -1;
            }
            break 'again;
        }

        if n == 0 {
            errno!() = 0;
            return 0;
        }

        if !buf0.is_null() {
            close((*buf0).fd);
            (*buf0).fd = -1;
        }

        msgbuf_drain(msgbuf, n as usize);

        1
    }
}

pub unsafe fn msgbuf_queuelen(msgbuf: *mut msgbuf) -> u32 {
    unsafe { (*msgbuf).queued }
}

unsafe fn ibuf_enqueue(msgbuf: *mut msgbuf, buf: *mut ibuf) {
    unsafe {
        if (*buf).max == 0 {
            // if buf lives on the stack
            abort(); /* abort before causing more harm */
        }
        tailq_insert_tail::<_, _>(&raw mut (*msgbuf).bufs, buf);
        (*msgbuf).queued += 1;
    }
}

unsafe fn ibuf_dequeue(msgbuf: *mut msgbuf, buf: *mut ibuf) {
    unsafe {
        tailq_remove(&raw mut (*msgbuf).bufs, buf);
        (*msgbuf).queued -= 1;
        ibuf_free(buf);
    }
}

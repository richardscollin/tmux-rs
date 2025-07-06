// Copyright (c) 2015 Nicholas Marriott <nicholas.marriott@gmail.com>
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

use crate::libc::{
    AF_UNIX, EAGAIN, PF_UNSPEC, SA_RESTART, SIG_DFL, SIG_IGN, SIGCHLD, SIGCONT, SIGHUP, SIGINT,
    SIGPIPE, SIGQUIT, SIGTERM, SIGTSTP, SIGTTIN, SIGTTOU, SIGUSR1, SIGUSR2, SIGWINCH, close, gid_t,
    sigaction, sigemptyset, socketpair, uname, utsname,
};

use crate::compat::{
    getpeereid,
    imsg::{
        imsg_clear, imsg_compose, imsg_flush, imsg_free, imsg_get, imsg_get_fd, imsg_init,
        imsg_read, imsgbuf,
    },
    imsg_buffer::msgbuf_write,
    queue::{tailq_foreach, tailq_init, tailq_insert_tail, tailq_remove},
    setproctitle_,
};
use crate::event_::{signal_add, signal_set};

#[repr(C)]
pub struct tmuxproc {
    pub name: *const u8,
    pub exit: i32,

    pub signalcb: Option<unsafe fn(i32)>,

    pub ev_sigint: event,
    pub ev_sighup: event,
    pub ev_sigchld: event,
    pub ev_sigcont: event,
    pub ev_sigterm: event,
    pub ev_sigusr1: event,
    pub ev_sigusr2: event,
    pub ev_sigwinch: event,

    pub peers: tailq_head<tmuxpeer>,
}

pub const PEER_BAD: i32 = 0x1;

crate::compat::impl_tailq_entry!(tmuxpeer, entry, tailq_entry<tmuxpeer>);
#[repr(C)]
pub struct tmuxpeer {
    pub parent: *mut tmuxproc,

    pub ibuf: imsgbuf,
    pub event: event,
    pub uid: uid_t,

    pub flags: i32,

    pub dispatchcb: Option<unsafe fn(*mut imsg, *mut c_void)>,
    pub arg: *mut c_void,

    // #[entry]
    pub entry: tailq_entry<tmuxpeer>,
}

pub unsafe extern "C" fn proc_event_cb(_fd: i32, events: i16, arg: *mut c_void) {
    unsafe {
        let peer = arg as *mut tmuxpeer;
        let mut imsg: MaybeUninit<imsg> = MaybeUninit::<imsg>::uninit();
        let imsg = imsg.as_mut_ptr();

        if (*peer).flags & PEER_BAD == 0 && events & EV_READ != 0 {
            let mut n = imsg_read(&raw mut (*peer).ibuf);
            if (n == -1 && errno!() != EAGAIN) || n == 0 {
                ((*peer).dispatchcb.unwrap())(null_mut(), (*peer).arg);
                return;
            }
            loop {
                n = imsg_get(&raw mut (*peer).ibuf, imsg);
                if n == -1 {
                    ((*peer).dispatchcb.unwrap())(null_mut(), (*peer).arg);
                    return;
                }
                if n == 0 {
                    break;
                }
                let msgtype = msgtype::try_from((*imsg).hdr.type_);
                log_debug!("peer {:p} message {:?}", peer, msgtype);

                if peer_check_version(peer, imsg) != 0 {
                    let fd = imsg_get_fd(imsg);
                    if fd != -1 {
                        close(fd);
                    }
                    imsg_free(imsg);
                    break;
                }

                ((*peer).dispatchcb.unwrap())(imsg, (*peer).arg);
                imsg_free(imsg);
            }
        }

        if events & EV_WRITE != 0
            && msgbuf_write((&raw mut (*peer).ibuf.w).cast()) <= 0
            && errno!() != EAGAIN
        {
            ((*peer).dispatchcb.unwrap())(null_mut(), (*peer).arg);
            return;
        }

        if ((*peer).flags & PEER_BAD != 0) && (*peer).ibuf.w.queued == 0 {
            ((*peer).dispatchcb.unwrap())(null_mut(), (*peer).arg);
            return;
        }

        proc_update_event(peer);
    }
}

pub unsafe extern "C" fn proc_signal_cb(signo: i32, _events: i16, arg: *mut c_void) {
    unsafe {
        let tp = arg as *mut tmuxproc;

        ((*tp).signalcb.unwrap())(signo);
    }
}

pub unsafe fn peer_check_version(peer: *mut tmuxpeer, imsg: *mut imsg) -> i32 {
    unsafe {
        let version = (*imsg).hdr.peerid & 0xff;
        if (*imsg).hdr.type_ != msgtype::MSG_VERSION as u32 && version != PROTOCOL_VERSION as u32 {
            log_debug!("peer {:p} bad version {}", peer, version);

            proc_send(peer, msgtype::MSG_VERSION, -1, null_mut(), 0);
            (*peer).flags |= PEER_BAD;

            return -1;
        }
        0
    }
}

pub unsafe fn proc_update_event(peer: *mut tmuxpeer) {
    unsafe {
        event_del(&raw mut (*peer).event);

        let mut events: i16 = EV_READ;
        if (*peer).ibuf.w.queued > 0 {
            events |= EV_WRITE;
        }
        event_set(
            &raw mut (*peer).event,
            (*peer).ibuf.fd,
            events,
            Some(proc_event_cb),
            peer.cast(),
        );

        event_add(&raw mut (*peer).event, null_mut());
    }
}

pub unsafe fn proc_send(
    peer: *mut tmuxpeer,
    type_: msgtype,
    fd: i32,
    buf: *const c_void,
    len: usize,
) -> i32 {
    unsafe {
        let ibuf = &raw mut (*peer).ibuf;
        let vp = buf;

        if (*peer).flags & PEER_BAD != 0 {
            return -1;
        }
        // log_debug_!("sending message {type_:?} to peer {peer:p} ({len} bytes)");

        let retval = imsg_compose(ibuf, type_ as u32, PROTOCOL_VERSION as u32, -1, fd, vp, len);
        if retval != 1 {
            return -1;
        }
        proc_update_event(peer);
        0
    }
}

pub unsafe fn proc_start(name: &CStr) -> *mut tmuxproc {
    unsafe {
        log_open(name);
        let name: *const u8 = name.as_ptr().cast();
        setproctitle_(c"%s (%s)".as_ptr().cast(), name, SOCKET_PATH);

        let mut u = MaybeUninit::<utsname>::uninit();
        if uname(u.as_mut_ptr()) < 0 {
            memset0(u.as_mut_ptr());
        }
        let u = u.as_mut_ptr();

        log_debug!(
            "{} started ({}): version {}, socket {}, protocol {}",
            _s(name),
            std::process::id(),
            getversion(),
            _s(SOCKET_PATH),
            PROTOCOL_VERSION,
        );
        log_debug!(
            "on {} {} {}",
            _s((*u).sysname.as_ptr()),
            _s((*u).release.as_ptr()),
            _s((*u).version.as_ptr()),
        );
        log_debug!(
            "using libevent {} {}",
            _s(event_get_version()),
            _s(event_get_method())
        );
        #[cfg(feature = "utf8proc")]
        {
            log_debug!("using utf8proc {}", _s(utf8proc_version()));
        }
        #[cfg(feature = "ncurses")]
        {
            log_debug!(
                "using ncurses {} {:06}",
                _s(NCURSES_VERSION),
                NCURSES_VERSION_PATCH
            );
        }

        let tp = xcalloc1::<tmuxproc>();
        tp.name = xstrdup(name).as_ptr();
        tailq_init(&raw mut tp.peers);

        tp
    }
}

pub unsafe fn proc_loop(tp: *mut tmuxproc, loopcb: Option<unsafe fn() -> i32>) {
    unsafe {
        log_debug!("{} loop enter", _s((*tp).name));
        match loopcb {
            None => loop {
                event_loop(EVLOOP_ONCE);

                if (*tp).exit != 0 {
                    break;
                }
            },
            Some(loopcb) => loop {
                event_loop(EVLOOP_ONCE);

                if (*tp).exit != 0 {
                    break;
                }

                if loopcb() != 0 {
                    break;
                }
            },
        }
        log_debug!("{} loop exit", _s((*tp).name));
    }
}

pub unsafe fn proc_exit(tp: *mut tmuxproc) {
    unsafe {
        for peer in tailq_foreach(&raw mut (*tp).peers).map(NonNull::as_ptr) {
            imsg_flush(&raw mut (*peer).ibuf);
        }
        (*tp).exit = 1;
    }
}

pub unsafe fn proc_set_signals(tp: *mut tmuxproc, signalcb: Option<unsafe fn(i32)>) {
    unsafe {
        let mut sa: sigaction = zeroed();

        (*tp).signalcb = signalcb;

        sigemptyset(&raw mut sa.sa_mask);
        sa.sa_flags = SA_RESTART;
        sa.sa_sigaction = SIG_IGN;

        sigaction(SIGPIPE, &sa, null_mut());
        sigaction(SIGTSTP, &sa, null_mut());
        sigaction(SIGTTIN, &sa, null_mut());
        sigaction(SIGTTOU, &sa, null_mut());
        sigaction(SIGQUIT, &sa, null_mut());

        signal_set(
            &raw mut (*tp).ev_sigint,
            SIGINT,
            Some(proc_signal_cb),
            tp.cast(),
        );
        signal_add(&raw mut (*tp).ev_sigint, null_mut());
        signal_set(
            &raw mut (*tp).ev_sighup,
            SIGHUP,
            Some(proc_signal_cb),
            tp.cast(),
        );
        signal_add(&raw mut (*tp).ev_sighup, null_mut());
        signal_set(
            &raw mut (*tp).ev_sigchld,
            SIGCHLD,
            Some(proc_signal_cb),
            tp.cast(),
        );
        signal_add(&raw mut (*tp).ev_sigchld, null_mut());
        signal_set(
            &raw mut (*tp).ev_sigcont,
            SIGCONT,
            Some(proc_signal_cb),
            tp.cast(),
        );
        signal_add(&raw mut (*tp).ev_sigcont, null_mut());
        signal_set(
            &raw mut (*tp).ev_sigterm,
            SIGTERM,
            Some(proc_signal_cb),
            tp.cast(),
        );
        signal_add(&raw mut (*tp).ev_sigterm, null_mut());
        signal_set(
            &raw mut (*tp).ev_sigusr1,
            SIGUSR1,
            Some(proc_signal_cb),
            tp.cast(),
        );
        signal_add(&raw mut (*tp).ev_sigusr1, null_mut());
        signal_set(
            &raw mut (*tp).ev_sigusr2,
            SIGUSR2,
            Some(proc_signal_cb),
            tp.cast(),
        );
        signal_add(&raw mut (*tp).ev_sigusr2, null_mut());
        signal_set(
            &raw mut (*tp).ev_sigwinch,
            SIGWINCH,
            Some(proc_signal_cb),
            tp.cast(),
        );
        signal_add(&raw mut (*tp).ev_sigwinch, null_mut());
    }
}

pub unsafe fn proc_clear_signals(tp: *mut tmuxproc, defaults: i32) {
    unsafe {
        let mut sa: sigaction = zeroed();

        sigemptyset(&raw mut sa.sa_mask);
        sa.sa_flags = SA_RESTART;
        sa.sa_sigaction = SIG_DFL;

        sigaction(SIGPIPE, &raw mut sa, null_mut());
        sigaction(SIGTSTP, &raw mut sa, null_mut());

        event_del(&raw mut (*tp).ev_sigint);
        event_del(&raw mut (*tp).ev_sighup);
        event_del(&raw mut (*tp).ev_sigchld);
        event_del(&raw mut (*tp).ev_sigcont);
        event_del(&raw mut (*tp).ev_sigterm);
        event_del(&raw mut (*tp).ev_sigusr1);
        event_del(&raw mut (*tp).ev_sigusr2);
        event_del(&raw mut (*tp).ev_sigwinch);

        if defaults != 0 {
            sigaction(SIGINT, &sa, null_mut());
            sigaction(SIGQUIT, &sa, null_mut());
            sigaction(SIGHUP, &sa, null_mut());
            sigaction(SIGCHLD, &sa, null_mut());
            sigaction(SIGCONT, &sa, null_mut());
            sigaction(SIGTERM, &sa, null_mut());
            sigaction(SIGUSR1, &sa, null_mut());
            sigaction(SIGUSR2, &sa, null_mut());
            sigaction(SIGWINCH, &sa, null_mut());
        }
    }
}

pub unsafe fn proc_add_peer(
    tp: *mut tmuxproc,
    fd: i32,
    dispatchcb: Option<unsafe fn(*mut imsg, *mut c_void)>,
    arg: *mut c_void,
) -> *mut tmuxpeer {
    unsafe {
        let mut gid: gid_t = 0;
        let peer = xcalloc1::<tmuxpeer>() as *mut tmuxpeer;
        (*peer).parent = tp;

        (*peer).dispatchcb = dispatchcb;
        (*peer).arg = arg;

        imsg_init(&raw mut (*peer).ibuf, fd);
        event_set(
            &raw mut (*peer).event,
            fd,
            EV_READ,
            Some(proc_event_cb),
            peer.cast(), // TODO could be ub if this and function below both write
        );

        if getpeereid(fd, &raw mut (*peer).uid, &raw mut gid) != 0 {
            (*peer).uid = -1i32 as uid_t;
        }

        log_debug!("add peer {:p}: {} ({:p})", peer, fd, arg);
        tailq_insert_tail(&raw mut (*tp).peers, peer);

        proc_update_event(peer);
        peer
    }
}

pub unsafe fn proc_remove_peer(peer: *mut tmuxpeer) {
    unsafe {
        tailq_remove(&raw mut (*(*peer).parent).peers, peer);
        log_debug!("remove peer {:p}", peer);

        event_del(&raw mut (*peer).event);
        imsg_clear(&raw mut (*peer).ibuf);

        close((*peer).ibuf.fd);
        free_(peer);
    }
}

pub unsafe fn proc_kill_peer(peer: *mut tmuxpeer) {
    unsafe {
        (*peer).flags |= PEER_BAD;
    }
}

pub unsafe fn proc_flush_peer(peer: *mut tmuxpeer) {
    unsafe {
        imsg_flush(&raw mut (*peer).ibuf);
    }
}

pub unsafe fn proc_toggle_log(tp: *mut tmuxproc) {
    unsafe {
        log_toggle(CStr::from_ptr((*tp).name.cast::<c_char>()));
    }
}

#[cfg_attr(target_os = "macos", expect(deprecated))]
/// On success, the PID of the child process is returned in the parent, and 0 is returned in the child.
pub unsafe fn proc_fork_and_daemon(fd: *mut i32) -> pid_t {
    unsafe {
        let mut pair: [c_int; 2] = [0; 2];

        if socketpair(
            AF_UNIX,
            libc::SOCK_STREAM,
            PF_UNSPEC,
            &raw mut pair as *mut i32,
        ) != 0
        {
            fatal("socketpair failed");
        }

        match libc::fork() {
            -1 => fatal("fork failed"),
            0 => {
                close(pair[0]);
                *fd = pair[1];
                if libc::daemon(1, 0) != 0 {
                    fatal("daemon failed");
                }
                0
            }
            pid => {
                close(pair[1]);
                *fd = pair[0];
                pid
            }
        }
    }
}

pub unsafe fn proc_get_peer_uid(peer: *const tmuxpeer) -> uid_t {
    unsafe { (*peer).uid }
}

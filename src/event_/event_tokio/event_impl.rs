use std::collections::HashMap;
use std::ffi::{c_int, c_short, c_void};
use std::time::Duration;

use ::libc::timeval;
use tokio::runtime::Runtime;
use tokio::sync::mpsc;
use tokio::task::{JoinHandle, LocalSet};

use super::super::{
    EV_PERSIST, EV_READ, EV_SIGNAL, EV_TIMEOUT, EV_WRITE, EVLOOP_NONBLOCK, EVLOOP_ONCE, evbuffer,
    event_base, event_log_cb,
};
use super::event;
use crate::log::log_debug;

// ---------------------------------------------------------------------------
// Signal self-pipe infrastructure
// ---------------------------------------------------------------------------

/// Maximum signal number we support (POSIX guarantees at most 64).
const MAX_SIGNALS: usize = 64;

/// Per-signal pipe write-end. The signal handler writes here.
/// -1 means no pipe installed for that signal.
static mut SIGNAL_WRITE_FDS: [c_int; MAX_SIGNALS] = [-1; MAX_SIGNALS];

/// Async-signal-safe handler: writes the signal number to the self-pipe.
unsafe extern "C" fn signal_handler_trampoline(signum: c_int) {
    unsafe {
        let fd = SIGNAL_WRITE_FDS[signum as usize];
        if fd >= 0 {
            // write() is async-signal-safe per POSIX.
            let b = signum as u8;
            libc::write(fd, &b as *const u8 as *const c_void, 1);
        }
    }
}

/// A ready event notification sent from a watcher task to the event loop.
struct ReadyEvent {
    id: u64,
    fd: c_int,
    events: c_short,
    callback: Option<unsafe extern "C-unwind" fn(arg1: c_int, arg2: c_short, arg3: *mut c_void)>,
    arg: *mut c_void,
}

/// The tokio-backed event base.
///
/// Owns the tokio current_thread runtime, a LocalSet for spawn_local
/// (avoiding Send requirements), and the channel used to deliver
/// ready-event notifications from watcher tasks.
pub(crate) struct EventBase {
    runtime: Runtime,
    local_set: LocalSet,
    ready_tx: mpsc::UnboundedSender<ReadyEvent>,
    ready_rx: mpsc::UnboundedReceiver<ReadyEvent>,
    registrations: HashMap<u64, JoinHandle<()>>,
    next_id: u64,
}

impl EventBase {
    fn new() -> Self {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_io()
            .enable_time()
            .build()
            .expect("failed to create tokio runtime");
        let (ready_tx, ready_rx) = mpsc::unbounded_channel();
        Self {
            runtime,
            local_set: LocalSet::new(),
            ready_tx,
            ready_rx,
            registrations: HashMap::new(),
            next_id: 1,
        }
    }

    fn alloc_id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }
}

/// Global event base pointer. libevent's legacy API uses a single global base.
static mut GLOBAL_BASE: *mut EventBase = std::ptr::null_mut();

unsafe fn get_base() -> &'static mut EventBase {
    unsafe { &mut *GLOBAL_BASE }
}

fn timeval_to_duration(tv: &timeval) -> Duration {
    Duration::new(tv.tv_sec as u64, (tv.tv_usec as u32) * 1000)
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn event_init() -> *mut event_base {
    let base = Box::new(EventBase::new());
    let ptr = Box::into_raw(base);
    unsafe { GLOBAL_BASE = ptr };
    log_debug!("event_init: ptr={ptr:p} pid={}", std::process::id());
    ptr as *mut event_base
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn event_reinit(base: *mut event_base) -> c_int {
    if base.is_null() {
        return -1;
    }

    // After fork: leak the old EventBase (don't drop it -- dropping a
    // tokio runtime in a forked child is unsafe because the epoll fd
    // and internal state are inherited copies). Create a fresh one.
    // The old allocation is intentionally leaked.

    let new_base = Box::new(EventBase::new());
    let new_ptr = Box::into_raw(new_base);
    unsafe { GLOBAL_BASE = new_ptr };

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn event_set(
    ev: *mut event,
    fd: c_int,
    events: c_short,
    cb: Option<unsafe extern "C-unwind" fn(arg1: c_int, arg2: c_short, arg3: *mut c_void)>,
    arg: *mut c_void,
) {
    if ev.is_null() {
        return;
    }
    let base = unsafe { GLOBAL_BASE };
    let ev = unsafe { &mut *ev };
    // Keep the existing id if the event was already initialized (id != 0).
    // This ensures event_del and event_add can track registrations correctly
    // across event_set → event_del → event_set → event_add cycles.
    if ev.id == 0 {
        ev.id = if base.is_null() {
            1
        } else {
            unsafe { (*base).alloc_id() }
        };
    }
    ev.ev_fd = fd;
    ev.ev_events = events;
    ev.ev_callback = cb;
    ev.ev_arg = arg;
    ev.ev_base = base as *mut event_base;
    ev.ev_timeout = timeval {
        tv_sec: 0,
        tv_usec: 0,
    };
    ev.ev_res = 0;
    // Don't reset ev.added -- let event_del/event_add manage it.
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn event_add(ev: *mut event, timeout: *const timeval) -> c_int {
    if ev.is_null() {
        return -1;
    }
    let ev = unsafe { &mut *ev };

    let base_ptr = ev.ev_base as *mut EventBase;
    if base_ptr.is_null() {
        return -1;
    }
    let base = unsafe { &mut *base_ptr };

    // If already added, remove the old watcher first
    if ev.added {
        if let Some(handle) = base.registrations.remove(&ev.id) {
            handle.abort();
        }
    }

    // Store timeout if provided
    if !timeout.is_null() {
        ev.ev_timeout = unsafe { *timeout };
    }

    let id = ev.id;
    let fd = ev.ev_fd;
    log_debug!(
        "event_add: id={id} fd={fd} events=0x{:x} base={base_ptr:p}",
        ev.ev_events
    );
    let events = ev.ev_events;
    let callback = ev.ev_callback;
    let arg = ev.ev_arg;
    let tx = base.ready_tx.clone();

    let has_timeout =
        !timeout.is_null() && unsafe { (*timeout).tv_sec != 0 || (*timeout).tv_usec != 0 };
    let timeout_duration = if has_timeout {
        Some(timeval_to_duration(unsafe { &*timeout }))
    } else {
        None
    };

    let persist = (events & EV_PERSIST) != 0;

    if (events & EV_SIGNAL) != 0 {
        // Signal event: fd is the signal number.
        let signum = fd;
        if signum >= 0 && (signum as usize) < MAX_SIGNALS {
            // Create a pipe for this signal (close old one first if re-adding).
            unsafe {
                let old_w = SIGNAL_WRITE_FDS[signum as usize];
                if old_w >= 0 {
                    libc::close(old_w);
                    SIGNAL_WRITE_FDS[signum as usize] = -1;
                }
            }

            let mut pipe_fds: [c_int; 2] = [0; 2];
            if unsafe { libc::pipe2(pipe_fds.as_mut_ptr(), libc::O_NONBLOCK | libc::O_CLOEXEC) }
                != 0
            {
                log_debug!("event_add: pipe2 failed for signal {signum}");
            } else {
                let pipe_read = pipe_fds[0];
                let pipe_write = pipe_fds[1];

                // Store write-end so the signal handler can reach it.
                unsafe { SIGNAL_WRITE_FDS[signum as usize] = pipe_write };

                // Install the trampoline via sigaction.
                unsafe {
                    let mut sa: libc::sigaction = std::mem::zeroed();
                    sa.sa_sigaction = signal_handler_trampoline as *const () as usize;
                    sa.sa_flags = libc::SA_RESTART;
                    libc::sigemptyset(&raw mut sa.sa_mask);
                    libc::sigaction(signum, &sa, std::ptr::null_mut());
                }

                // Spawn watcher: poll the read-end of the pipe.
                let handle = base.local_set.spawn_local(async move {
                    use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
                    use tokio::io::unix::AsyncFd;

                    let pipe_owned = unsafe { OwnedFd::from_raw_fd(pipe_read) };
                    let Ok(async_fd) =
                        AsyncFd::with_interest(pipe_owned, tokio::io::Interest::READABLE)
                    else {
                        log_debug!("signal watcher: AsyncFd failed signum={signum}");
                        return;
                    };

                    loop {
                        if let Ok(mut guard) = async_fd.readable().await {
                            // Drain all pending bytes (coalesce multiple deliveries).
                            let mut buf = [0u8; 64];
                            let _ = unsafe {
                                libc::read(async_fd.as_raw_fd(), buf.as_mut_ptr().cast(), buf.len())
                            };
                            guard.clear_ready();

                            let _ = tx.send(ReadyEvent {
                                id,
                                fd: signum,
                                events: EV_SIGNAL,
                                callback,
                                arg,
                            });

                            if !persist {
                                break;
                            }
                            tokio::task::yield_now().await;
                        }
                    }
                });
                base.registrations.insert(id, handle);
            }
        }
    } else if fd >= 0 && (events & (EV_READ | EV_WRITE)) != 0 {
        // I/O event on a file descriptor
        let handle = base.local_set.spawn_local(async move {
            loop {
                let result = poll_fd(fd, events, timeout_duration).await;
                match result {
                    Some(fired) => {
                        let _ = tx.send(ReadyEvent {
                            id,
                            fd,
                            events: fired,
                            callback,
                            arg,
                        });
                    }
                    None => {
                        // Timeout
                        let _ = tx.send(ReadyEvent {
                            id,
                            fd,
                            events: EV_TIMEOUT,
                            callback,
                            arg,
                        });
                        break;
                    }
                }
                if !persist {
                    break;
                }
                // Yield so the event loop can dispatch the callback and
                // consume data before we re-check readiness.  Without
                // this, level-triggered epoll may fire immediately
                // (data not yet consumed) producing a spurious event.
                tokio::task::yield_now().await;
            }
        });
        base.registrations.insert(id, handle);
    } else if let Some(dur) = timeout_duration {
        // Pure timer (fd == -1, timeout set)
        let handle = base.local_set.spawn_local(async move {
            tokio::time::sleep(dur).await;
            let _ = tx.send(ReadyEvent {
                id,
                fd,
                events: EV_TIMEOUT,
                callback,
                arg,
            });
        });
        base.registrations.insert(id, handle);
    }

    ev.added = true;
    0
}

/// Poll a file descriptor for readiness using tokio's AsyncFd.
async fn poll_fd(fd: c_int, events: c_short, timeout: Option<Duration>) -> Option<c_short> {
    use std::os::fd::{FromRawFd, OwnedFd};
    use tokio::io::unix::AsyncFd;

    // dup() the fd so each watcher task gets its own epoll registration.
    // This is necessary because multiple events can watch the same fd
    // (e.g., bufferevent's ev_read + ev_write), and tokio/epoll only
    // allows one registration per fd.  The dup'd fd is owned by the
    // AsyncFd and closed automatically when the task completes or is
    // aborted.
    let dup_fd = unsafe { libc::dup(fd) };
    if dup_fd < 0 {
        log_debug!("poll_fd: dup failed fd={fd}");
        return None;
    }
    let owned = unsafe { OwnedFd::from_raw_fd(dup_fd) };

    let interest = if (events & EV_READ) != 0 && (events & EV_WRITE) != 0 {
        tokio::io::Interest::READABLE | tokio::io::Interest::WRITABLE
    } else if (events & EV_READ) != 0 {
        tokio::io::Interest::READABLE
    } else {
        tokio::io::Interest::WRITABLE
    };

    let async_fd = match AsyncFd::with_interest(owned, interest) {
        Ok(afd) => afd,
        Err(e) => {
            log_debug!("poll_fd: AsyncFd::with_interest failed fd={fd} (dup={dup_fd}) err={e}");
            return None;
        }
    };

    let want_read = (events & EV_READ) != 0;
    let want_write = (events & EV_WRITE) != 0;

    let poll_future = async {
        let mut fired: c_short = 0;
        match (want_read, want_write) {
            (true, true) => {
                // Wait for either readable or writable (whichever fires first).
                tokio::select! {
                    guard = async_fd.readable() => {
                        if let Ok(mut g) = guard {
                            fired |= EV_READ;
                            g.clear_ready();
                        }
                    }
                    guard = async_fd.writable() => {
                        if let Ok(mut g) = guard {
                            fired |= EV_WRITE;
                            g.clear_ready();
                        }
                    }
                }
            }
            (true, false) => {
                if let Ok(mut g) = async_fd.readable().await {
                    fired |= EV_READ;
                    g.clear_ready();
                }
            }
            (false, true) => {
                if let Ok(mut g) = async_fd.writable().await {
                    fired |= EV_WRITE;
                    g.clear_ready();
                }
            }
            _ => {}
        }
        fired
    };

    if let Some(dur) = timeout {
        match tokio::time::timeout(dur, poll_future).await {
            Ok(fired) => Some(fired),
            Err(_) => None, // timed out
        }
    } else {
        Some(poll_future.await)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn event_del(ev: *mut event) -> c_int {
    if ev.is_null() {
        return -1;
    }
    let ev = unsafe { &mut *ev };
    if !ev.added {
        return 0;
    }

    log_debug!(
        "event_del: id={} fd={} events=0x{:x}",
        ev.id, ev.ev_fd, ev.ev_events
    );

    let base_ptr = ev.ev_base as *mut EventBase;
    if !base_ptr.is_null() {
        let base = unsafe { &mut *base_ptr };
        if let Some(handle) = base.registrations.remove(&ev.id) {
            handle.abort();
        }
    }

    // Clean up signal pipe if this was a signal event.
    if (ev.ev_events & EV_SIGNAL) != 0 {
        let signum = ev.ev_fd;
        if signum >= 0 && (signum as usize) < MAX_SIGNALS {
            unsafe {
                let w = SIGNAL_WRITE_FDS[signum as usize];
                if w >= 0 {
                    libc::close(w);
                    SIGNAL_WRITE_FDS[signum as usize] = -1;
                }
                // The pipe read-end is closed by the watcher task's OwnedFd
                // being dropped when the task is aborted above.
            }
        }
    }

    ev.added = false;
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn event_active(ev: *mut event, res: c_int, _ncalls: c_short) {
    if ev.is_null() {
        return;
    }
    let ev = unsafe { &*ev };
    let base_ptr = ev.ev_base as *mut EventBase;
    if base_ptr.is_null() {
        return;
    }
    let base = unsafe { &*base_ptr };
    let _ = base.ready_tx.send(ReadyEvent {
        id: ev.id,
        fd: ev.ev_fd,
        events: res as c_short,
        callback: ev.ev_callback,
        arg: ev.ev_arg,
    });
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn event_pending(
    ev: *const event,
    events: c_short,
    tv: *mut timeval,
) -> c_int {
    if ev.is_null() {
        return 0;
    }
    let ev = unsafe { &*ev };
    if !ev.added {
        return 0;
    }
    let mut result: c_int = 0;
    if (events & EV_TIMEOUT) != 0 && (ev.ev_timeout.tv_sec != 0 || ev.ev_timeout.tv_usec != 0) {
        result |= EV_TIMEOUT as c_int;
        if !tv.is_null() {
            unsafe { *tv = ev.ev_timeout };
        }
    }
    if (events & EV_READ) != 0 && (ev.ev_events & EV_READ) != 0 {
        result |= EV_READ as c_int;
    }
    if (events & EV_WRITE) != 0 && (ev.ev_events & EV_WRITE) != 0 {
        result |= EV_WRITE as c_int;
    }
    if (events & EV_SIGNAL) != 0 && (ev.ev_events & EV_SIGNAL) != 0 {
        result |= EV_SIGNAL as c_int;
    }
    result
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn event_initialized(ev: *const event) -> c_int {
    if ev.is_null() {
        return 0;
    }
    let ev = unsafe { &*ev };
    // id is assigned by event_set; 0 means uninitialized (zeroed memory)
    (ev.id != 0) as c_int
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn event_loop(flags: c_int) -> c_int {
    let base_ptr = unsafe { GLOBAL_BASE };
    if base_ptr.is_null() {
        return -1;
    }
    let base = unsafe { &mut *base_ptr };

    if (flags & EVLOOP_NONBLOCK) != 0 {
        // Non-blocking: tick the local_set to let spawned tasks make progress,
        // then drain whatever is ready on the channel.
        base.runtime
            .block_on(base.local_set.run_until(tokio::task::yield_now()));
        let mut events = Vec::new();
        while let Ok(ev) = base.ready_rx.try_recv() {
            events.push(ev);
        }
        for ready in events {
            let base = unsafe { &mut *base_ptr };
            if !base.registrations.contains_key(&ready.id) {
                continue;
            }
            if let Some(cb) = ready.callback {
                unsafe { cb(ready.fd, ready.events, ready.arg) };
            }
        }
        return 0;
    }

    // EVLOOP_ONCE (or default): block until at least one event fires.
    // We run the local_set (which drives all spawn_local tasks) until
    // a ReadyEvent appears on the channel.
    log_debug!(
        "event_loop: EVLOOP_ONCE regs={} base={base_ptr:p}",
        base.registrations.len()
    );
    let first = base
        .runtime
        .block_on(base.local_set.run_until(base.ready_rx.recv()));
    let Some(first) = first else {
        return -1; // channel closed
    };

    let mut events = vec![first];
    // Drain any others that are also ready
    while let Ok(ev) = base.ready_rx.try_recv() {
        events.push(ev);
    }

    log_debug!(
        "event_loop: dispatching {} events",
        events.len()
    );
    for ready in events {
        // A previous callback in this batch may have called event_del,
        // freeing the event's data.  Check that the registration still
        // exists before dispatching to avoid use-after-free.
        let base = unsafe { &mut *base_ptr };
        if !base.registrations.contains_key(&ready.id) {
            log_debug!(
                "event_loop: skipping stale id={} fd={} events=0x{:x}",
                ready.id, ready.fd, ready.events
            );
            continue;
        }
        log_debug!(
            "event_loop: dispatch id={} fd={} events=0x{:x}",
            ready.id, ready.fd, ready.events
        );
        if let Some(cb) = ready.callback {
            unsafe { cb(ready.fd, ready.events, ready.arg) };
        }
    }

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn event_once(
    fd: c_int,
    events: c_short,
    cb: Option<unsafe extern "C-unwind" fn(arg1: c_int, arg2: c_short, arg3: *mut c_void)>,
    arg: *mut c_void,
    tv: *const timeval,
) -> c_int {
    // Allocate a temporary event, set it up, add it.
    // The watcher task is non-persistent by design, so it will fire once
    // and then its JoinHandle will complete.
    let ev = unsafe { libc::calloc(1, std::mem::size_of::<event>()) } as *mut event;
    if ev.is_null() {
        return -1;
    }
    unsafe { event_set(ev, fd, events, cb, arg) };
    // Clear EV_PERSIST to ensure single-shot
    unsafe { (*ev).ev_events &= !EV_PERSIST };
    // Note: we intentionally leak `ev` here -- in a full implementation
    // we'd register a wrapper callback that frees it after firing.
    // For now this matches the libevent behavior where event_once allocates internally.
    unsafe { event_add(ev, tv) }
}

static VERSION_STR: &[u8] = b"tokio-event/1.0\0";
static METHOD_STR: &[u8] = b"tokio\0";

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn event_get_version() -> *const u8 {
    VERSION_STR.as_ptr()
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn event_get_method() -> *const u8 {
    METHOD_STR.as_ptr()
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn event_set_log_callback(_cb: event_log_cb) {
    // TODO: wire up to tracing or a global callback
}

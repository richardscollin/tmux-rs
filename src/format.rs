// Copyright (c) 2011 Nicholas Marriott <nicholas.marriott@gmail.com>
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
use std::borrow::Cow;

use crate::compat::HOST_NAME_MAX;
use crate::libc::{
    FNM_CASEFOLD, REG_NOSUB, ctime_r, getpwuid, getuid, ispunct, localtime_r, memcpy, regcomp,
    regex_t, regexec, regfree, strchr, strcmp, strcspn, strftime, strstr, strtod, tm,
};
use crate::*;
use crate::options_::*;

bitflags::bitflags! {
    #[repr(transparent)]
    #[derive(Copy, Clone)]
    pub struct format_flags: i32 {
        const FORMAT_STATUS  = 1;
        const FORMAT_FORCE   = 2;
        const FORMAT_NOJOBS  = 4;
        const FORMAT_VERBOSE = 8;
    }
}

pub const FORMAT_NONE: i32 = 0;
pub const FORMAT_PANE: u32 = 0x80000000u32;
pub const FORMAT_WINDOW: u32 = 0x40000000u32;

pub type format_cb = unsafe fn(_: *mut format_tree) -> format_table_type;

// Entry in format job tree.
#[repr(C)]
pub struct format_job {
    pub client: *mut client,
    pub tag: u32,
    pub cmd: *mut u8,
    pub expanded: *mut u8,

    pub last: time_t,
    pub out: *mut u8,
    pub updated: i32,

    pub job: *mut job,
    pub status: i32,

    pub entry: rb_entry<format_job>,
}

pub type format_job_tree = rb_head<format_job>;

pub static mut FORMAT_JOBS: format_job_tree = rb_initializer();
RB_GENERATE!(
    format_job_tree,
    format_job,
    entry,
    discr_entry,
    format_job_cmp
);

// Format job tree comparison function.
pub fn format_job_cmp(fj1: &format_job, fj2: &format_job) -> cmp::Ordering {
    unsafe {
        fj1.tag
            .cmp(&fj2.tag)
            .then_with(|| i32_to_ordering(strcmp(fj1.cmd, fj2.cmd)))
    }
}

bitflags::bitflags! {
    #[repr(transparent)]
    #[derive(Copy, Clone)]
    pub struct format_modifiers : i32 {
        const FORMAT_TIMESTRING = 0x1;
        const FORMAT_BASENAME   = 0x2;
        const FORMAT_DIRNAME    = 0x4;
        const FORMAT_QUOTE_SHELL  = 0x8;
        const FORMAT_LITERAL = 0x10;
        const FORMAT_EXPAND = 0x20;
        const FORMAT_EXPANDTIME = 0x40;
        const FORMAT_SESSIONS = 0x80;
        const FORMAT_WINDOWS = 0x100;
        const FORMAT_PANES = 0x200;
        const FORMAT_PRETTY = 0x400;
        const FORMAT_LENGTH = 0x800;
        const FORMAT_WIDTH = 0x1000;
        const FORMAT_QUOTE_STYLE = 0x2000;
        const FORMAT_WINDOW_NAME = 0x4000;
        const FORMAT_SESSION_NAME = 0x8000;
        const FORMAT_CHARACTER = 0x10000;
        const FORMAT_COLOUR = 0x20000;
        const FORMAT_CLIENTS = 0x40000;
    }
}

/// Limit on recursion.
const FORMAT_LOOP_LIMIT: i32 = 100;

bitflags::bitflags! {
    /// Format expand flags.
    #[repr(transparent)]
    #[derive(Copy, Clone)]
    pub struct format_expand_flags: i32 {
        const FORMAT_EXPAND_TIME = 0x1;
        const FORMAT_EXPAND_NOJOBS = 0x2;
    }
}

#[repr(i32)]
#[derive(Copy, Clone, Eq, PartialEq)]
pub enum format_type {
    FORMAT_TYPE_UNKNOWN,
    FORMAT_TYPE_SESSION,
    FORMAT_TYPE_WINDOW,
    FORMAT_TYPE_PANE,
}

// Entry in format tree.
#[repr(C)]
pub struct format_entry {
    pub key: *mut u8,
    pub value: *mut u8,
    pub time: time_t,
    pub cb: Option<format_cb>,
    pub entry: rb_entry<format_entry>,
}

#[repr(C)]
pub struct format_tree {
    pub type_: format_type,

    pub c: *mut client,
    pub s: *mut session,
    pub wl: *mut winlink,
    pub w: *mut window,
    pub wp: *mut window_pane,
    pub pb: *mut paste_buffer,

    pub item: *mut cmdq_item,
    pub client: *mut client,
    pub flags: format_flags,
    pub tag: u32,

    pub m: mouse_event,

    pub tree: format_entry_tree,
}
pub type format_entry_tree = rb_head<format_entry>;
RB_GENERATE!(
    format_entry_tree,
    format_entry,
    entry,
    discr_entry,
    format_entry_cmp
);

/// Format expand state.
#[repr(C)]
pub struct format_expand_state {
    pub ft: *mut format_tree,
    pub loop_: u32,
    pub time: time_t,
    pub tm: tm,
    pub flags: format_expand_flags,
}

/// Format modifier.
#[repr(C)]
pub struct format_modifier {
    pub modifier: [u8; 3],
    pub size: u32,

    pub argv: *mut *mut u8,
    pub argc: i32,
}

/// Format entry tree comparison function.
fn format_entry_cmp(fe1: &format_entry, fe2: &format_entry) -> cmp::Ordering {
    unsafe { i32_to_ordering(strcmp(fe1.key, fe2.key)) }
}

/// Single-character uppercase aliases.
static FORMAT_UPPER: [SyncCharPtr; 26] = const {
    const fn idx(c: char) -> usize {
        (c as u8 - b'A') as usize
    }
    let mut tmp = [SyncCharPtr::null(); 26];

    tmp[idx('D')] = SyncCharPtr::new(c"pane_id");
    tmp[idx('F')] = SyncCharPtr::new(c"window_flags");
    tmp[idx('H')] = SyncCharPtr::new(c"host");
    tmp[idx('I')] = SyncCharPtr::new(c"window_index");
    tmp[idx('P')] = SyncCharPtr::new(c"pane_index");
    tmp[idx('S')] = SyncCharPtr::new(c"session_name");
    tmp[idx('T')] = SyncCharPtr::new(c"pane_title");
    tmp[idx('W')] = SyncCharPtr::new(c"window_name");

    tmp
};

/// Single-character lowercase aliases.
static FORMAT_LOWER: [SyncCharPtr; 26] = const {
    const fn idx(c: char) -> usize {
        (c as u8 - b'a') as usize
    }
    let mut tmp = [SyncCharPtr::null(); 26];
    tmp[idx('h')] = SyncCharPtr::new(c"host_short");
    tmp
};

/// Is logging enabled?
pub fn format_logging(ft: &format_tree) -> bool {
    log_get_level() != 0 || ft.flags.intersects(format_flags::FORMAT_VERBOSE)
}

macro_rules! format_log1 {
   ($es:expr, $from:expr, $fmt:literal $(, $args:expr)* $(,)?) => {
        format_log1_($es, $from, format_args!($fmt $(, $args)*))
    };
}

/// Log a message if verbose.
pub unsafe fn format_log1_(
    es: *mut format_expand_state,
    from: *const u8,
    args: std::fmt::Arguments,
) {
    unsafe {
        let ft: *mut format_tree = (*es).ft;
        let spaces = c"          ";

        if !format_logging(&*ft) {
            return;
        }

        let s = args.to_string();

        log_debug!("{}: {}", _s(from), s);
        if !(*ft).item.is_null() && (*ft).flags.intersects(format_flags::FORMAT_VERBOSE) {
            cmdq_print!(
                (*ft).item,
                "#{1:0$}{2}",
                (*es).loop_ as usize,
                _s(spaces.as_ptr()),
                s
            );
        }
    }
}

/// Copy expand state.
pub unsafe fn format_copy_state(
    to: *mut format_expand_state,
    from: *mut format_expand_state,
    flags: format_expand_flags,
) {
    unsafe {
        (*to).ft = (*from).ft;
        (*to).loop_ = (*from).loop_;
        (*to).time = (*from).time;
        memcpy__(&raw mut (*to).tm, &raw const (*from).tm);
        (*to).flags = (*from).flags | flags;
    }
}

/// Format job update callback.
pub unsafe fn format_job_update(job: *mut job) {
    unsafe {
        let fj = job_get_data(job) as *mut format_job;
        let evb: *mut evbuffer = (*job_get_event(job)).input;
        let mut line: *mut u8 = null_mut();

        while let Some(next) = NonNull::new(evbuffer_readline(evb)) {
            free(line.cast());
            line = next.as_ptr();
        }
        if line.is_null() {
            return;
        }
        (*fj).updated = 1;

        free((*fj).out.cast());
        (*fj).out = line;

        log_debug!(
            "{}: {:p} {}: {}",
            function_name!(),
            fj,
            _s((*fj).cmd),
            _s((*fj).out)
        );

        let t = libc::time(null_mut());
        if (*fj).status != 0 && (*fj).last != t {
            if !(*fj).client.is_null() {
                server_status_client((*fj).client);
            }
            (*fj).last = t;
        }
    }
}

/// Format job complete callback.
pub unsafe fn format_job_complete(job: *mut job) {
    unsafe {
        let fj = job_get_data(job) as *mut format_job;
        let evb: *mut evbuffer = (*job_get_event(job)).input;

        (*fj).job = null_mut();

        let buf: *mut u8;

        let line = evbuffer_readline(evb);
        if line.is_null() {
            let len = EVBUFFER_LENGTH(evb);
            buf = xmalloc(len + 1).as_ptr().cast();
            if len != 0 {
                memcpy(buf.cast(), EVBUFFER_DATA(evb).cast(), len);
            }
            *buf.add(len) = b'\0';
        } else {
            buf = line;
        }

        log_debug!(
            "{}: {:p} {}: {}",
            function_name!(),
            fj,
            _s((*fj).cmd),
            _s(buf)
        );

        if *buf != b'\0' || (*fj).updated == 0 {
            free((*fj).out.cast());
            (*fj).out = buf;
        } else {
            free(buf.cast());
        }

        if (*fj).status != 0 {
            if !(*fj).client.is_null() {
                server_status_client((*fj).client);
            }
            (*fj).status = 0;
        }
    }
}

pub unsafe fn format_job_get(es: *mut format_expand_state, cmd: *mut u8) -> *mut u8 {
    unsafe {
        let ft: *mut format_tree = (*es).ft;
        let mut fj0 = MaybeUninit::<format_job>::uninit();
        let fj0 = fj0.as_mut_ptr();

        let jobs = if (*ft).client.is_null() {
            &raw mut FORMAT_JOBS
        } else if !(*(*ft).client).jobs.is_null() {
            (*(*ft).client).jobs
        } else {
            (*(*ft).client).jobs = Box::leak(Box::new(zeroed())) as *mut format_job_tree;
            rb_init((*(*ft).client).jobs);
            (*(*ft).client).jobs
        };

        (*fj0).tag = (*ft).tag;
        (*fj0).cmd = cmd;
        let mut fj = rb_find(jobs, fj0);
        if fj.is_null() {
            fj = xcalloc1() as *mut format_job;
            (*fj).client = (*ft).client;
            (*fj).tag = (*ft).tag;
            (*fj).cmd = xstrdup(cmd).as_ptr();

            rb_insert(jobs, fj);
        }

        let mut next = MaybeUninit::<format_expand_state>::uninit();
        let next = next.as_mut_ptr();
        format_copy_state(next, es, format_expand_flags::FORMAT_EXPAND_NOJOBS);
        (*next).flags &= !format_expand_flags::FORMAT_EXPAND_TIME;

        let expanded = format_expand1(next, cmd);

        let force = if (*fj).expanded.is_null() || strcmp(expanded, (*fj).expanded) != 0 {
            free((*fj).expanded.cast());
            (*fj).expanded = xstrdup(expanded).as_ptr();
            true
        } else {
            (*ft).flags.intersects(format_flags::FORMAT_FORCE)
        };

        let t = libc::time(null_mut());
        if force && !(*fj).job.is_null() {
            job_free((*fj).job);
        }
        if force || ((*fj).job.is_null() && (*fj).last != t) {
            (*fj).job = job_run(
                expanded,
                0,
                null_mut(),
                null_mut(),
                null_mut(),
                server_client_get_cwd((*ft).client, null_mut()),
                Some(format_job_update),
                Some(format_job_complete),
                None,
                fj.cast(),
                job_flag::JOB_NOWAIT,
                -1,
                -1,
            );
            if (*fj).job.is_null() {
                free((*fj).out.cast());
                (*fj).out = format_nul!("<'{}' didn't start>", _s((*fj).cmd),);
            }
            (*fj).last = t;
            (*fj).updated = 0;
        } else if !(*fj).job.is_null() && (t - (*fj).last) > 1 && (*fj).out.is_null() {
            (*fj).out = format_nul!("<'{}' not ready>", _s((*fj).cmd));
        }
        free(expanded.cast());

        if (*ft).flags.intersects(format_flags::FORMAT_STATUS) {
            (*fj).status = 1;
        }
        if (*fj).out.is_null() {
            return xstrdup_(c"").as_ptr();
        }

        format_expand1(next, (*fj).out)
    }
}

pub unsafe fn format_job_tidy(jobs: *mut format_job_tree, force: i32) {
    unsafe {
        let now = libc::time(null_mut());
        for fj in rb_foreach(jobs) {
            let fj = fj.as_ptr();
            if force == 0 && ((*fj).last > now || now - (*fj).last < 3600) {
                continue;
            }
            rb_remove(jobs, fj);

            log_debug!("{}: {}", "format_job_tidy", _s((*fj).cmd));

            if !(*fj).job.is_null() {
                job_free((*fj).job);
            }

            free_((*fj).expanded);
            free_((*fj).cmd);
            free_((*fj).out);

            free_(fj);
        }
    }
}

pub unsafe fn format_tidy_jobs() {
    unsafe {
        format_job_tidy(&raw mut FORMAT_JOBS, 0);
        for c in tailq_foreach(&raw mut CLIENTS).map(NonNull::as_ptr) {
            if !(*c).jobs.is_null() {
                format_job_tidy((*c).jobs, 0);
            }
        }
    }
}

pub unsafe fn format_lost_client(c: *mut client) {
    unsafe {
        if !(*c).jobs.is_null() {
            format_job_tidy((*c).jobs, 1);
        }
        free_((*c).jobs);
    }
}

pub unsafe fn format_cb_host(_ft: *mut format_tree) -> format_table_type {
    unsafe {
        let mut host = MaybeUninit::<[u8; HOST_NAME_MAX + 1]>::uninit();

        if libc::gethostname(host.as_mut_ptr().cast(), HOST_NAME_MAX + 1) != 0 {
            "".into()
        } else {
            format!("{}", _s(host.as_ptr().cast::<u8>())).into()
        }
    }
}

/// Callback for `host_short`.
pub unsafe fn format_cb_host_short(_ft: *mut format_tree) -> format_table_type {
    unsafe {
        let mut host = MaybeUninit::<[u8; HOST_NAME_MAX + 1]>::uninit();

        if libc::gethostname(host.as_mut_ptr().cast(), HOST_NAME_MAX + 1) != 0 {
            return "".into();
        }

        let cp = strchr(host.as_mut_ptr().cast(), b'.' as i32);
        if !cp.is_null() {
            *cp = b'\0';
        }
        format!("{}", _s(&raw const host as *const u8)).into()
    }
}

/// Callback for pid.
pub unsafe fn format_cb_pid(_ft: *mut format_tree) -> format_table_type {
    unsafe { format!("{}", libc::getpid()).into() }
}

/// Callback for `session_attached_list`.
pub unsafe fn format_cb_session_attached_list(ft: *mut format_tree) -> format_table_type {
    unsafe {
        let s = (*ft).s;

        if s.is_null() {
            return format_table_type::None;
        }

        let buffer = evbuffer_new();
        if buffer.is_null() {
            fatalx("out of memory");
        }

        for loop_ in tailq_foreach(&raw mut CLIENTS).map(NonNull::as_ptr) {
            if (*loop_).session == s {
                if EVBUFFER_LENGTH(buffer) > 0 {
                    evbuffer_add(buffer, c!(",").cast(), 1);
                }
                evbuffer_add_printf!(buffer, "{}", _s((*loop_).name));
            }
        }

        let size = EVBUFFER_LENGTH(buffer);
        let result = if size != 0 {
            format!("{1:0$}", size, _s(EVBUFFER_DATA(buffer).cast::<u8>())).into()
        } else {
            format_table_type::None
        };
        evbuffer_free(buffer);
        result
    }
}

/// Callback for `session_alerts`.
pub unsafe fn format_cb_session_alerts(ft: *mut format_tree) -> format_table_type {
    unsafe {
        let s: *mut session = (*ft).s;
        const SIZEOF_ALERTS: usize = 1024;
        const SIZEOF_TMP: usize = 16;
        let mut alerts = MaybeUninit::<[u8; 1024]>::uninit();
        let alerts: *mut u8 = alerts.as_mut_ptr().cast();
        let mut tmp = MaybeUninit::<[u8; 16]>::uninit();
        let tmp: *mut u8 = tmp.as_mut_ptr().cast();

        if s.is_null() {
            return format_table_type::None;
        }

        *alerts = b'\0';
        for wl in rb_foreach(&raw mut (*s).windows).map(NonNull::as_ptr) {
            if !(*wl).flags.intersects(WINLINK_ALERTFLAGS) {
                continue;
            }
            _ = xsnprintf_!(tmp, SIZEOF_TMP, "{}", (*wl).idx);

            if *alerts != b'\0' {
                strlcat(alerts, c!(","), SIZEOF_ALERTS);
            }
            strlcat(alerts, tmp, SIZEOF_ALERTS);
            if (*wl).flags.intersects(winlink_flags::WINLINK_ACTIVITY) {
                strlcat(alerts, c!("#"), SIZEOF_ALERTS);
            }
            if (*wl).flags.intersects(winlink_flags::WINLINK_BELL) {
                strlcat(alerts, c!("!"), SIZEOF_ALERTS);
            }
            if (*wl).flags.intersects(winlink_flags::WINLINK_SILENCE) {
                strlcat(alerts, c!("~"), SIZEOF_ALERTS);
            }
        }
        format!("{}", _s(alerts)).into()
    }
}

/// Callback for `session_stack`.
pub unsafe fn format_cb_session_stack(ft: *mut format_tree) -> format_table_type {
    unsafe {
        let s = (*ft).s;
        const SIZEOF_RESULT: usize = 1024;
        const SIZEOF_TMP: usize = 16;

        let mut result = MaybeUninit::<[u8; 1024]>::uninit();
        let result: *mut u8 = result.as_mut_ptr().cast();
        let mut tmp = MaybeUninit::<[u8; 16]>::uninit();
        let tmp: *mut u8 = tmp.as_mut_ptr().cast();

        if s.is_null() {
            return format_table_type::None;
        }

        _ = xsnprintf_!(result, SIZEOF_RESULT, "{}", (*(*s).curw).idx);
        for wl in tailq_foreach::<_, discr_sentry>(&raw mut (*s).lastw).map(NonNull::as_ptr) {
            _ = xsnprintf_!(tmp, SIZEOF_TMP, "{}", (*wl).idx);

            if *result != b'\0' {
                strlcat(result, c!(","), SIZEOF_RESULT);
            }
            strlcat(result, tmp, SIZEOF_RESULT);
        }
        format!("{}", _s(result)).into()
    }
}

/// Callback for `window_stack_index`.
pub unsafe fn format_cb_window_stack_index(ft: *mut format_tree) -> format_table_type {
    unsafe {
        if (*ft).wl.is_null() {
            return format_table_type::None;
        }
        let s = (*(*ft).wl).session;

        let mut idx: u32 = 0;
        let mut wl = null_mut();
        for wl_ in tailq_foreach::<_, discr_sentry>(&raw mut (*s).lastw).map(NonNull::as_ptr) {
            wl = wl_;
            idx += 1;
            if wl == (*ft).wl {
                break;
            }
        }
        if wl.is_null() {
            return "0".into();
        }
        format!("{idx}").into()
    }
}

/// Callback for `window_linked_sessions_list`.
pub unsafe fn format_cb_window_linked_sessions_list(ft: *mut format_tree) -> format_table_type {
    unsafe {
        if (*ft).wl.is_null() {
            return format_table_type::None;
        }
        let w = (*(*ft).wl).window;

        let buffer = evbuffer_new();
        if buffer.is_null() {
            fatalx("out of memory");
        }

        for wl in tailq_foreach::<_, discr_wentry>(&raw mut (*w).winlinks).map(NonNull::as_ptr) {
            if EVBUFFER_LENGTH(buffer) > 0 {
                evbuffer_add(buffer, c!(",").cast(), 1);
            }
            evbuffer_add_printf!(buffer, "{}", (*(*wl).session).name);
        }

        let size = EVBUFFER_LENGTH(buffer);
        let mut value = format_table_type::None;
        if size != 0 {
            value = format_table_type::String(
                format!("{1:0$}", size, _s(EVBUFFER_DATA(buffer).cast::<u8>())).into(),
            );
        }
        evbuffer_free(buffer);
        value
    }
}

/// Callback for `window_active_sessions`.
pub unsafe fn format_cb_window_active_sessions(ft: *mut format_tree) -> format_table_type {
    unsafe {
        if (*ft).wl.is_null() {
            return format_table_type::None;
        }
        let w = (*(*ft).wl).window;

        let n = tailq_foreach::<_, discr_wentry>(&raw mut (*w).winlinks)
            .filter(|wl| (*(*wl.as_ptr()).session).curw == wl.as_ptr())
            .count() as u32;

        format!("{n}").into()
    }
}

/// Callback for `window_active_sessions_list`.
pub unsafe fn format_cb_window_active_sessions_list(ft: *mut format_tree) -> format_table_type {
    unsafe {
        if (*ft).wl.is_null() {
            return format_table_type::None;
        }
        let w = (*(*ft).wl).window;

        let buffer = evbuffer_new();
        if buffer.is_null() {
            fatalx("out of memory");
        }

        for wl in tailq_foreach::<_, discr_wentry>(&raw mut (*w).winlinks).map(NonNull::as_ptr) {
            if (*(*wl).session).curw == wl {
                if EVBUFFER_LENGTH(buffer) > 0 {
                    evbuffer_add(buffer, c!(",").cast(), 1);
                }
                evbuffer_add_printf!(buffer, "{}", (*(*wl).session).name);
            }
        }

        let size = EVBUFFER_LENGTH(buffer);
        let mut value = format_table_type::None;
        if size != 0 {
            value = format_table_type::String(
                format!("{1:0$}", size, _s(EVBUFFER_DATA(buffer).cast::<u8>())).into(),
            );
        }
        evbuffer_free(buffer);
        value
    }
}

/// Callback for `window_active_clients`.
pub unsafe fn format_cb_window_active_clients(ft: *mut format_tree) -> format_table_type {
    unsafe {
        if (*ft).wl.is_null() {
            return format_table_type::None;
        }
        let w = (*(*ft).wl).window;

        let mut n = 0u32;
        for loop_ in tailq_foreach(&raw mut CLIENTS).map(NonNull::as_ptr) {
            let client_session = (*loop_).session;
            if client_session.is_null() {
                continue;
            }

            if w == (*(*client_session).curw).window {
                n += 1;
            }
        }

        format!("{n}").into()
    }
}

/// Callback for `window_active_clients_list`.
pub unsafe fn format_cb_window_active_clients_list(ft: *mut format_tree) -> format_table_type {
    unsafe {
        if (*ft).wl.is_null() {
            return format_table_type::None;
        }
        let w = (*(*ft).wl).window;

        let buffer = evbuffer_new();
        if buffer.is_null() {
            fatalx("out of memory");
        }

        for loop_ in tailq_foreach(&raw mut CLIENTS).map(NonNull::as_ptr) {
            let client_session = (*loop_).session;
            if client_session.is_null() {
                continue;
            }

            if w == (*(*client_session).curw).window {
                if EVBUFFER_LENGTH(buffer) > 0 {
                    evbuffer_add(buffer, c!(",").cast(), 1);
                }
                evbuffer_add_printf!(buffer, "{}", _s((*loop_).name));
            }
        }

        let size = EVBUFFER_LENGTH(buffer);
        let mut value = format_table_type::None;
        if size != 0 {
            value = format_table_type::String(
                format!("{1:0$}", size, _s(EVBUFFER_DATA(buffer).cast::<u8>())).into(),
            );
        }
        evbuffer_free(buffer);
        value
    }
}

/// Callback for `window_layout`.
pub unsafe fn format_cb_window_layout(ft: *mut format_tree) -> format_table_type {
    unsafe {
        let w = (*ft).w;

        if w.is_null() {
            return format_table_type::None;
        }

        if !(*w).saved_layout_root.is_null() {
            return layout_dump((*w).saved_layout_root)
                .map(Into::into)
                .unwrap_or_default();
        }
        layout_dump((*w).layout_root)
            .map(Into::into)
            .unwrap_or_default()
    }
}

/// Callback for `window_visible_layout`.
pub unsafe fn format_cb_window_visible_layout(ft: *mut format_tree) -> format_table_type {
    unsafe {
        let w = (*ft).w;

        if w.is_null() {
            return format_table_type::None;
        }

        layout_dump((*w).layout_root)
            .map(Into::into)
            .unwrap_or_default()
    }
}

/// Callback for `pane_start_command`.
pub unsafe fn format_cb_start_command(ft: *mut format_tree) -> format_table_type {
    unsafe {
        let wp = (*ft).wp;

        if wp.is_null() {
            return format_table_type::None;
        }

        cmd_stringify_argv((*wp).argc, (*wp).argv).into()
    }
}

/// Callback for `pane_start_path`.
pub unsafe fn format_cb_start_path(ft: *mut format_tree) -> format_table_type {
    unsafe {
        let wp = (*ft).wp;

        if wp.is_null() {
            return format_table_type::None;
        }

        if (*wp).cwd.is_null() {
            return "".into();
        }
        format!("{}", _s((*wp).cwd)).into()
    }
}

/// Callback for `pane_current_command`.
pub unsafe fn format_cb_current_command(ft: *mut format_tree) -> format_table_type {
    unsafe {
        let wp = (*ft).wp;

        if wp.is_null() || (*wp).shell.is_null() {
            return format_table_type::None;
        }

        let mut cmd = osdep_get_name((*wp).fd, (*wp).tty.as_ptr());
        if cmd.is_null() || *cmd == b'\0' {
            free_(cmd);
            cmd = CString::new(cmd_stringify_argv((*wp).argc, (*wp).argv))
                .unwrap()
                .into_raw()
                .cast();
            if cmd.is_null() || *cmd == b'\0' {
                free_(cmd);
                cmd = xstrdup((*wp).shell).as_ptr().cast();
            }
        }
        let value = parse_window_name(cmd);
        free_(cmd);
        value.into()
    }
}

/// Callback for `pane_current_path`.
pub unsafe fn format_cb_current_path(ft: *mut format_tree) -> format_table_type {
    unsafe {
        let wp = (*ft).wp;

        if wp.is_null() {
            return format_table_type::None;
        }

        let cwd = osdep_get_cwd((*wp).fd);
        if cwd.is_null() {
            return format_table_type::None;
        }
        format!("{}", _s(cwd)).into()
    }
}

/// Callback for `history_bytes`.
pub unsafe fn format_cb_history_bytes(ft: *mut format_tree) -> format_table_type {
    unsafe {
        let wp = (*ft).wp;

        if wp.is_null() {
            return format_table_type::None;
        }

        let gd = (*wp).base.grid;
        let mut size: usize = 0;

        for i in 0..((*gd).hsize + (*gd).sy) {
            let gl = grid_get_line(gd, i);
            size += (*gl).cellsize as usize * std::mem::size_of::<grid_cell>();
            size += (*gl).extdsize as usize * std::mem::size_of::<grid_cell>();
        }
        size += ((*gd).hsize + (*gd).sy) as usize * std::mem::size_of::<grid_line>();

        format!("{size}").into()
    }
}

/// Callback for `history_all_bytes`.
pub unsafe fn format_cb_history_all_bytes(ft: *mut format_tree) -> format_table_type {
    unsafe {
        let wp = (*ft).wp;

        if wp.is_null() {
            return format_table_type::None;
        }

        let gd = (*wp).base.grid;
        let lines = (*gd).hsize + (*gd).sy;
        let mut cells = 0;
        let mut extended_cells = 0;

        for i in 0..lines {
            let gl = grid_get_line(gd, i);
            cells += (*gl).cellsize;
            extended_cells += (*gl).extdsize;
        }

        format!(
            "{},{},{},{},{},{}",
            lines,
            lines as usize * std::mem::size_of::<grid_line>(),
            cells,
            cells as usize * std::mem::size_of::<grid_cell>(),
            extended_cells,
            extended_cells as usize * std::mem::size_of::<grid_cell>(),
        )
        .into()
    }
}

/// Callback for `pane_tabs`.
pub unsafe fn format_cb_pane_tabs(ft: *mut format_tree) -> format_table_type {
    unsafe {
        let wp = (*ft).wp;

        if wp.is_null() {
            return format_table_type::None;
        }

        let buffer = evbuffer_new();
        if buffer.is_null() {
            fatalx("out of memory");
        }

        let mut first = true;
        for i in 0..(*(*wp).base.grid).sx {
            if !(*wp).base.tabs.as_ref().unwrap().bit_test(i) {
                continue;
            }

            if !first {
                evbuffer_add(buffer, c!(",").cast(), 1);
            }
            evbuffer_add_printf!(buffer, "{i}");
            first = false;
        }

        let size = EVBUFFER_LENGTH(buffer);
        let result = if size != 0 {
            format!("{}", _s(EVBUFFER_DATA(buffer).cast::<u8>())).into()
        } else {
            format_table_type::None
        };
        evbuffer_free(buffer);
        result
    }
}

/// Callback for `pane_fg`.
pub unsafe fn format_cb_pane_fg(ft: *mut format_tree) -> format_table_type {
    unsafe {
        let wp = (*ft).wp;
        let mut gc = MaybeUninit::<grid_cell>::uninit();

        if wp.is_null() {
            return format_table_type::None;
        }

        tty_default_colours(gc.as_mut_ptr(), wp);

        colour_tostring((*gc.as_ptr()).fg).into()
    }
}

/// Callback for `pane_bg`.
pub unsafe fn format_cb_pane_bg(ft: *mut format_tree) -> format_table_type {
    unsafe {
        let wp = (*ft).wp;
        let mut gc = MaybeUninit::<grid_cell>::uninit();

        if wp.is_null() {
            return format_table_type::None;
        }

        tty_default_colours(gc.as_mut_ptr(), wp);

        colour_tostring((*gc.as_ptr()).bg).into()
    }
}

/// Callback for `session_group_list`.
pub unsafe fn format_cb_session_group_list(ft: *mut format_tree) -> format_table_type {
    unsafe {
        let s = (*ft).s;
        if s.is_null() {
            return format_table_type::None;
        }

        let sg = session_group_contains(s);
        if sg.is_null() {
            return format_table_type::None;
        }

        let buffer = evbuffer_new();
        if buffer.is_null() {
            fatalx("out of memory");
        }

        for loop_ in tailq_foreach(&raw mut (*sg).sessions).map(NonNull::as_ptr) {
            if EVBUFFER_LENGTH(buffer) > 0 {
                evbuffer_add(buffer, c!(",").cast(), 1);
            }
            evbuffer_add_printf!(buffer, "{}", (*loop_).name);
        }

        let size = EVBUFFER_LENGTH(buffer);
        let result = if size != 0 {
            format!("{1:0$}", size, _s(EVBUFFER_DATA(buffer).cast::<u8>())).into()
        } else {
            format_table_type::None
        };
        evbuffer_free(buffer);
        result
    }
}

/// Callback for `session_group_attached_list`.
pub unsafe fn format_cb_session_group_attached_list(ft: *mut format_tree) -> format_table_type {
    unsafe {
        let s = (*ft).s;
        if s.is_null() {
            return format_table_type::None;
        }

        let sg = session_group_contains(s);
        if sg.is_null() {
            return format_table_type::None;
        }

        let buffer = evbuffer_new();
        if buffer.is_null() {
            fatalx("out of memory");
        }

        for loop_ in tailq_foreach(&raw mut CLIENTS).map(NonNull::as_ptr) {
            let client_session = (*loop_).session;
            if client_session.is_null() {
                continue;
            }

            for session_loop in tailq_foreach(&raw mut (*sg).sessions).map(NonNull::as_ptr) {
                if session_loop == client_session {
                    if EVBUFFER_LENGTH(buffer) > 0 {
                        evbuffer_add(buffer, c!(",").cast(), 1);
                    }
                    evbuffer_add_printf!(buffer, "{}", _s((*loop_).name));
                }
            }
        }

        let size = EVBUFFER_LENGTH(buffer);
        let result = if size != 0 {
            format!("{1:0$}", size, _s(EVBUFFER_DATA(buffer).cast::<u8>())).into()
        } else {
            format_table_type::None
        };
        evbuffer_free(buffer);
        result
    }
}

/// Callback for `pane_in_mode`.
pub unsafe fn format_cb_pane_in_mode(ft: *mut format_tree) -> format_table_type {
    unsafe {
        let wp = (*ft).wp;
        if wp.is_null() {
            return format_table_type::None;
        }

        let n = tailq_foreach(&raw mut (*wp).modes).count() as u32;

        format!("{n}").into()
    }
}

/// Callback for `pane_at_top`.
pub unsafe fn format_cb_pane_at_top(ft: *mut format_tree) -> format_table_type {
    unsafe {
        let wp = (*ft).wp;
        if wp.is_null() {
            return format_table_type::None;
        }

        let w = (*wp).window;
        let status: i64 = options_get_number___(&*(*w).options, "pane-border-status");
        let flag = if status == pane_status::PANE_STATUS_TOP as i64 {
            (*wp).yoff == 1
        } else {
            (*wp).yoff == 0
        };

        format!("{flag}").into()
    }
}

/// Callback for `pane_at_bottom`.
pub unsafe fn format_cb_pane_at_bottom(ft: *mut format_tree) -> format_table_type {
    unsafe {
        let wp = (*ft).wp;
        if wp.is_null() {
            return format_table_type::None;
        }

        let w = (*wp).window;
        let status: i64 = options_get_number___(&*(*w).options, "pane-border-status");
        let flag = if status == pane_status::PANE_STATUS_BOTTOM as i64 {
            (*wp).yoff + (*wp).sy == (*w).sy - 1
        } else {
            (*wp).yoff + (*wp).sy == (*w).sy
        };

        format!("{flag}").into()
    }
}

/// Callback for `cursor_character`.
pub unsafe fn format_cb_cursor_character(ft: *mut format_tree) -> format_table_type {
    unsafe {
        let wp = (*ft).wp;
        if wp.is_null() {
            return format_table_type::None;
        }
        let mut gc = MaybeUninit::<grid_cell>::uninit();
        grid_view_get_cell(
            (*wp).base.grid,
            (*wp).base.cx,
            (*wp).base.cy,
            gc.as_mut_ptr(),
        );
        if !(*gc.as_ptr()).flags.intersects(grid_flag::PADDING) {
            format!(
                "{1:0$}",
                (*gc.as_ptr()).data.size as usize,
                _s((&raw const (*gc.as_ptr()).data.data).cast::<u8>())
            )
            .into()
        } else {
            format_table_type::None
        }
    }
}

/// Callback for `mouse_word`.
pub unsafe fn format_cb_mouse_word(ft: *mut format_tree) -> format_table_type {
    unsafe {
        if !(*ft).m.valid {
            return format_table_type::None;
        }
        let Some(wp) = cmd_mouse_pane(&raw mut (*ft).m, null_mut(), null_mut()) else {
            return format_table_type::None;
        };
        let mut x = 0;
        let mut y = 0;
        if cmd_mouse_at(wp.as_ptr(), &raw mut (*ft).m, &mut x, &mut y, 0) != 0 {
            return format_table_type::None;
        }

        if !tailq_empty(&raw mut (*wp.as_ptr()).modes) {
            if window_pane_mode(wp.as_ptr()) != WINDOW_PANE_NO_MODE {
                return window_copy_get_word(wp.as_ptr(), x, y).into();
            }
            return format_table_type::None;
        }
        let gd = (*wp.as_ptr()).base.grid;
        format_grid_word(gd, x, (*gd).hsize + y).into()
    }
}

/// Callback for `mouse_hyperlink`.
pub unsafe fn format_cb_mouse_hyperlink(ft: *mut format_tree) -> format_table_type {
    unsafe {
        if !(*ft).m.valid {
            return format_table_type::None;
        }
        let Some(wp) = cmd_mouse_pane(&raw mut (*ft).m, null_mut(), null_mut()) else {
            return format_table_type::None;
        };
        let mut x = 0;
        let mut y = 0;
        if cmd_mouse_at(wp.as_ptr(), &raw mut (*ft).m, &mut x, &mut y, 0) != 0 {
            return format_table_type::None;
        }
        let gd = (*wp.as_ptr()).base.grid;
        format_grid_hyperlink(gd, x, (*gd).hsize + y, (*wp.as_ptr()).screen)
            .map(Into::into)
            .unwrap_or(format_table_type::None)
    }
}

/// Callback for `mouse_line`.
pub unsafe fn format_cb_mouse_line(ft: *mut format_tree) -> format_table_type {
    unsafe {
        if !(*ft).m.valid {
            return format_table_type::None;
        }
        let Some(wp) = cmd_mouse_pane(&raw mut (*ft).m, null_mut(), null_mut()) else {
            return format_table_type::None;
        };
        let mut x = 0;
        let mut y = 0;
        if cmd_mouse_at(wp.as_ptr(), &raw mut (*ft).m, &mut x, &mut y, 0) != 0 {
            return format_table_type::None;
        }

        if !tailq_empty(&raw mut (*wp.as_ptr()).modes) {
            if window_pane_mode(wp.as_ptr()) != WINDOW_PANE_NO_MODE {
                return window_copy_get_line(wp.as_ptr(), y).into();
            }
            return format_table_type::None;
        }
        let gd = (*wp.as_ptr()).base.grid;
        format_grid_line(gd, (*gd).hsize + y).into()
    }
}

/// Callback for `mouse_status_line`.
pub unsafe fn format_cb_mouse_status_line(ft: *mut format_tree) -> format_table_type {
    unsafe {
        if !(*ft).m.valid {
            return format_table_type::None;
        }
        if (*ft).c.is_null() || !(*(*ft).c).tty.flags.intersects(tty_flags::TTY_STARTED) {
            return format_table_type::None;
        }

        let y = if (*ft).m.statusat == 0 && (*ft).m.y < (*ft).m.statuslines {
            (*ft).m.y
        } else if (*ft).m.statusat > 0 && (*ft).m.y >= (*ft).m.statusat as u32 {
            (*ft).m.y - (*ft).m.statusat as u32
        } else {
            return format_table_type::None;
        };

        format!("{y}").into()
    }
}

/// Callback for `mouse_status_range`.
pub unsafe fn format_cb_mouse_status_range(ft: *mut format_tree) -> format_table_type {
    unsafe {
        if !(*ft).m.valid {
            return format_table_type::None;
        }
        if (*ft).c.is_null() || !(*(*ft).c).tty.flags.intersects(tty_flags::TTY_STARTED) {
            return format_table_type::None;
        }

        let x;
        let y;
        if (*ft).m.statusat == 0 && (*ft).m.y < (*ft).m.statuslines {
            x = (*ft).m.x;
            y = (*ft).m.y;
        } else if (*ft).m.statusat > 0 && (*ft).m.y >= (*ft).m.statusat as u32 {
            x = (*ft).m.x;
            y = (*ft).m.y - (*ft).m.statusat as u32;
        } else {
            return format_table_type::None;
        }

        let sr = status_get_range((*ft).c, x, y);
        if sr.is_null() {
            return format_table_type::None;
        }

        match (*sr).type_ {
            style_range_type::STYLE_RANGE_NONE => format_table_type::None,
            style_range_type::STYLE_RANGE_LEFT => "left".into(),
            style_range_type::STYLE_RANGE_RIGHT => "right".into(),
            style_range_type::STYLE_RANGE_PANE => "pane".into(),
            style_range_type::STYLE_RANGE_WINDOW => "window".into(),
            style_range_type::STYLE_RANGE_SESSION => "session".into(),
            style_range_type::STYLE_RANGE_USER => format!("{}", _s((*sr).string.as_ptr())).into(),
        }
    }
}

pub unsafe fn format_cb_alternate_on(ft: *mut format_tree) -> format_table_type {
    unsafe {
        if !(*ft).wp.is_null() {
            if !(*(*ft).wp).base.saved_grid.is_null() {
                return "1".into();
            }
            return "0".into();
        }
        format_table_type::None
    }
}

pub unsafe fn format_cb_alternate_saved_x(ft: *mut format_tree) -> format_table_type {
    unsafe {
        if !(*ft).wp.is_null() {
            return format!("{}", (*(*ft).wp).base.saved_cx).into();
        }
        format_table_type::None
    }
}

pub unsafe fn format_cb_alternate_saved_y(ft: *mut format_tree) -> format_table_type {
    unsafe {
        if !(*ft).wp.is_null() {
            return format!("{}", (*(*ft).wp).base.saved_cy).into();
        }
        format_table_type::None
    }
}

pub unsafe fn format_cb_buffer_name(ft: *mut format_tree) -> format_table_type {
    unsafe {
        if let Some(pb) = NonNull::new((*ft).pb) {
            return paste_buffer_name(pb).to_string().into();
        }
        format_table_type::None
    }
}

pub unsafe fn format_cb_buffer_sample(ft: *mut format_tree) -> format_table_type {
    unsafe {
        if !(*ft).pb.is_null() {
            return paste_make_sample((*ft).pb).into();
        }
        format_table_type::None
    }
}

pub unsafe fn format_cb_buffer_size(ft: *mut format_tree) -> format_table_type {
    unsafe {
        if !(*ft).pb.is_null() {
            let mut size = 0usize;
            paste_buffer_data((*ft).pb, &mut size);
            return format!("{size}").into();
        }
        format_table_type::None
    }
}

pub unsafe fn format_cb_client_cell_height(ft: *mut format_tree) -> format_table_type {
    unsafe {
        if !(*ft).c.is_null() && (*(*ft).c).tty.flags.intersects(tty_flags::TTY_STARTED) {
            return format!("{}", (*(*ft).c).tty.ypixel).into();
        }
        format_table_type::None
    }
}

pub unsafe fn format_cb_client_cell_width(ft: *mut format_tree) -> format_table_type {
    unsafe {
        if !(*ft).c.is_null() && (*(*ft).c).tty.flags.intersects(tty_flags::TTY_STARTED) {
            return format!("{}", (*(*ft).c).tty.xpixel).into();
        }
        format_table_type::None
    }
}

pub unsafe fn format_cb_client_control_mode(ft: *mut format_tree) -> format_table_type {
    unsafe {
        if !(*ft).c.is_null() {
            if (*(*ft).c).flags.intersects(client_flag::CONTROL) {
                return "1".into();
            }
            return "0".into();
        }
        format_table_type::None
    }
}

pub unsafe fn format_cb_client_discarded(ft: *mut format_tree) -> format_table_type {
    unsafe {
        if !(*ft).c.is_null() {
            return format!("{}", (*(*ft).c).discarded).into();
        }
        format_table_type::None
    }
}

pub unsafe fn format_cb_client_flags(ft: *mut format_tree) -> format_table_type {
    unsafe {
        if !(*ft).c.is_null() {
            return format!("{}", _s(server_client_get_flags((*ft).c))).into();
        }
        format_table_type::None
    }
}

pub unsafe fn format_cb_client_height(ft: *mut format_tree) -> format_table_type {
    unsafe {
        if !(*ft).c.is_null() && (*(*ft).c).tty.flags.intersects(tty_flags::TTY_STARTED) {
            return format!("{}", (*(*ft).c).tty.sy).into();
        }
        format_table_type::None
    }
}

pub unsafe fn format_cb_client_key_table(ft: *mut format_tree) -> format_table_type {
    unsafe {
        if !(*ft).c.is_null() {
            return format!("{}", _s((*(*(*ft).c).keytable).name)).into();
        }
        format_table_type::None
    }
}

pub unsafe fn format_cb_client_last_session(ft: *mut format_tree) -> format_table_type {
    unsafe {
        if !(*ft).c.is_null()
            && !(*(*ft).c).last_session.is_null()
            && session_alive((*(*ft).c).last_session)
        {
            return format!("{}", (*(*(*ft).c).last_session).name).into();
        }
        format_table_type::None
    }
}

pub unsafe fn format_cb_client_name(ft: *mut format_tree) -> format_table_type {
    unsafe {
        if !(*ft).c.is_null() {
            return format!("{}", _s((*(*ft).c).name)).into();
        }
        format_table_type::None
    }
}

pub unsafe fn format_cb_client_pid(ft: *mut format_tree) -> format_table_type {
    unsafe {
        if !(*ft).c.is_null() {
            return format!("{}", (*(*ft).c).pid as c_long).into();
        }
        format_table_type::None
    }
}

/// Callback for `client_prefix`.
pub unsafe fn format_cb_client_prefix(ft: *mut format_tree) -> format_table_type {
    unsafe {
        if !(*ft).c.is_null() {
            let name = server_client_get_key_table((*ft).c);
            if strcmp((*(*(*ft).c).keytable).name, name) == 0 {
                return "0".into();
            }
            return "1".into();
        }
        format_table_type::None
    }
}

pub unsafe fn format_cb_client_readonly(ft: *mut format_tree) -> format_table_type {
    unsafe {
        if !(*ft).c.is_null() {
            if (*(*ft).c).flags.intersects(client_flag::READONLY) {
                return "1".into();
            }
            return "0".into();
        }
        format_table_type::None
    }
}

pub unsafe fn format_cb_client_session(ft: *mut format_tree) -> format_table_type {
    unsafe {
        if !(*ft).c.is_null() && !(*(*ft).c).session.is_null() {
            return format!("{}", (*(*(*ft).c).session).name).into();
        }
        format_table_type::None
    }
}

pub unsafe fn format_cb_client_termfeatures(ft: *mut format_tree) -> format_table_type {
    unsafe {
        if !(*ft).c.is_null() {
            return format!("{}", _s(tty_get_features((*(*ft).c).term_features))).into();
        }
        format_table_type::None
    }
}

pub unsafe fn format_cb_client_termname(ft: *mut format_tree) -> format_table_type {
    unsafe {
        if !(*ft).c.is_null() {
            return format!("{}", _s((*(*ft).c).term_name)).into();
        }
        format_table_type::None
    }
}

pub unsafe fn format_cb_client_termtype(ft: *mut format_tree) -> format_table_type {
    unsafe {
        if !(*ft).c.is_null() {
            if (*(*ft).c).term_type.is_null() {
                return "".into();
            }
            return format!("{}", _s((*(*ft).c).term_type)).into();
        }
        format_table_type::None
    }
}

pub unsafe fn format_cb_client_tty(ft: *mut format_tree) -> format_table_type {
    unsafe {
        if !(*ft).c.is_null() {
            return format!("{}", _s((*(*ft).c).ttyname)).into();
        }
        format_table_type::None
    }
}

pub unsafe fn format_cb_client_uid(ft: *mut format_tree) -> format_table_type {
    unsafe {
        if !(*ft).c.is_null() {
            let uid = proc_get_peer_uid((*(*ft).c).peer);
            if uid != -1_i32 as uid_t {
                return format!("{}", uid as c_long).into();
            }
        }
        format_table_type::None
    }
}

pub unsafe fn format_cb_client_user(ft: *mut format_tree) -> format_table_type {
    unsafe {
        if !(*ft).c.is_null() {
            let uid = proc_get_peer_uid((*(*ft).c).peer);
            if uid != -1_i32 as uid_t
                && let Some(pw) = NonNull::new(libc::getpwuid(uid))
            {
                return format!("{}", _s((*pw.as_ptr()).pw_name)).into();
            }
        }
        format_table_type::None
    }
}

pub unsafe fn format_cb_client_utf8(ft: *mut format_tree) -> format_table_type {
    unsafe {
        if !(*ft).c.is_null() {
            if (*(*ft).c).flags.intersects(client_flag::UTF8) {
                return "1".into();
            }
            return "0".into();
        }
        format_table_type::None
    }
}

pub unsafe fn format_cb_client_width(ft: *mut format_tree) -> format_table_type {
    unsafe {
        if !(*ft).c.is_null() {
            return format!("{}", (*(*ft).c).tty.sx).into();
        }
        format_table_type::None
    }
}

pub unsafe fn format_cb_client_written(ft: *mut format_tree) -> format_table_type {
    unsafe {
        if !(*ft).c.is_null() {
            return format!("{}", (*(*ft).c).written).into();
        }
        format_table_type::None
    }
}

/// Callback for `config_files`.
pub unsafe fn format_cb_config_files(_ft: *mut format_tree) -> format_table_type {
    let mut s = String::new();

    for file in CFG_FILES.lock().unwrap().iter() {
        s.push_str(file.to_str().expect("cfg_files invalid utf8"));
        s.push(',');
    }

    s.into()
}

/// Callback for `cursor_flag`.
pub unsafe fn format_cb_cursor_flag(ft: *mut format_tree) -> format_table_type {
    unsafe {
        if !(*ft).wp.is_null() {
            if (*(*ft).wp).base.mode.intersects(mode_flag::MODE_CURSOR) {
                return "1".into();
            }
            return "0".into();
        }
        format_table_type::None
    }
}

/// Callback for `cursor_x`.
pub unsafe fn format_cb_cursor_x(ft: *mut format_tree) -> format_table_type {
    unsafe {
        if !(*ft).wp.is_null() {
            return format!("{}", (*(*ft).wp).base.cx).into();
        }
        format_table_type::None
    }
}

/// Callback for `cursor_y`.
pub unsafe fn format_cb_cursor_y(ft: *mut format_tree) -> format_table_type {
    unsafe {
        if !(*ft).wp.is_null() {
            return format!("{}", (*(*ft).wp).base.cy).into();
        }
        format_table_type::None
    }
}

/// Callback for `history_limit`.
pub unsafe fn format_cb_history_limit(ft: *mut format_tree) -> format_table_type {
    unsafe {
        if !(*ft).wp.is_null() {
            return format!("{}", (*(*(*ft).wp).base.grid).hlimit).into();
        }
        format_table_type::None
    }
}

/// Callback for `history_size`.
pub unsafe fn format_cb_history_size(ft: *mut format_tree) -> format_table_type {
    unsafe {
        if !(*ft).wp.is_null() {
            return format!("{}", (*(*(*ft).wp).base.grid).hsize).into();
        }
        format_table_type::None
    }
}

/// Callback for `insert_flag`.
pub unsafe fn format_cb_insert_flag(ft: *mut format_tree) -> format_table_type {
    unsafe {
        if !(*ft).wp.is_null() {
            if (*(*ft).wp).base.mode.intersects(mode_flag::MODE_INSERT) {
                return "1".into();
            }
            return "0".into();
        }
        format_table_type::None
    }
}

/// Callback for `keypad_cursor_flag`.
pub unsafe fn format_cb_keypad_cursor_flag(ft: *mut format_tree) -> format_table_type {
    unsafe {
        if !(*ft).wp.is_null() {
            if (*(*ft).wp).base.mode.intersects(mode_flag::MODE_KCURSOR) {
                return "1".into();
            }
            return "0".into();
        }
        format_table_type::None
    }
}

/// Callback for `keypad_flag`.
pub unsafe fn format_cb_keypad_flag(ft: *mut format_tree) -> format_table_type {
    unsafe {
        if !(*ft).wp.is_null() {
            if (*(*ft).wp).base.mode.intersects(mode_flag::MODE_KKEYPAD) {
                return "1".into();
            }
            return "0".into();
        }
        format_table_type::None
    }
}

/// Callback for `mouse_all_flag`.
pub unsafe fn format_cb_mouse_all_flag(ft: *mut format_tree) -> format_table_type {
    unsafe {
        if !(*ft).wp.is_null() {
            if (*(*ft).wp).base.mode.intersects(mode_flag::MODE_MOUSE_ALL) {
                return "1".into();
            }
            return "0".into();
        }
        format_table_type::None
    }
}

/// Callback for `mouse_any_flag`.
pub unsafe fn format_cb_mouse_any_flag(ft: *mut format_tree) -> format_table_type {
    unsafe {
        if !(*ft).wp.is_null() {
            if (*(*ft).wp).base.mode.intersects(ALL_MOUSE_MODES) {
                return "1".into();
            }
            return "0".into();
        }
        format_table_type::None
    }
}

/// Callback for `mouse_button_flag`.
pub unsafe fn format_cb_mouse_button_flag(ft: *mut format_tree) -> format_table_type {
    unsafe {
        if !(*ft).wp.is_null() {
            if (*(*ft).wp)
                .base
                .mode
                .intersects(mode_flag::MODE_MOUSE_BUTTON)
            {
                return "1".into();
            }
            return "0".into();
        }
        format_table_type::None
    }
}

/// Callback for `mouse_pane`.
pub unsafe fn format_cb_mouse_pane(ft: *mut format_tree) -> format_table_type {
    unsafe {
        if (*ft).m.valid {
            if let Some(wp) = cmd_mouse_pane(&raw mut (*ft).m, null_mut(), null_mut()) {
                return format!("%{}", (*wp.as_ptr()).id).into();
            }
            return format_table_type::None;
        }
        format_table_type::None
    }
}

/// Callback for `mouse_sgr_flag`.
pub unsafe fn format_cb_mouse_sgr_flag(ft: *mut format_tree) -> format_table_type {
    unsafe {
        if !(*ft).wp.is_null() {
            if (*(*ft).wp).base.mode.intersects(mode_flag::MODE_MOUSE_SGR) {
                return "1".into();
            }
            return "0".into();
        }
        format_table_type::None
    }
}

/// Callback for `mouse_standard_flag`.
pub unsafe fn format_cb_mouse_standard_flag(ft: *mut format_tree) -> format_table_type {
    unsafe {
        if !(*ft).wp.is_null() {
            if (*(*ft).wp)
                .base
                .mode
                .intersects(mode_flag::MODE_MOUSE_STANDARD)
            {
                return "1".into();
            }
            return "0".into();
        }
        format_table_type::None
    }
}

/// Callback for `mouse_utf8_flag`.
pub unsafe fn format_cb_mouse_utf8_flag(ft: *mut format_tree) -> format_table_type {
    unsafe {
        if !(*ft).wp.is_null() {
            if (*(*ft).wp).base.mode.intersects(mode_flag::MODE_MOUSE_UTF8) {
                return "1".into();
            }
            return "0".into();
        }
        format_table_type::None
    }
}

/// Callback for `mouse_x`.
pub unsafe fn format_cb_mouse_x(ft: *mut format_tree) -> format_table_type {
    unsafe {
        if !(*ft).m.valid {
            return format_table_type::None;
        }
        let wp = cmd_mouse_pane(&raw mut (*ft).m, null_mut(), null_mut());
        let mut x: u32 = 0;
        let mut y: u32 = 0;
        if let Some(wp) = wp
            && cmd_mouse_at(wp.as_ptr(), &raw mut (*ft).m, &mut x, &mut y, 0) == 0
        {
            return format!("{x}").into();
        }
        if !(*ft).c.is_null() && (*(*ft).c).tty.flags.intersects(tty_flags::TTY_STARTED) {
            if (*ft).m.statusat == 0 && (*ft).m.y < (*ft).m.statuslines {
                return format!("{}", (*ft).m.x).into();
            }
            if (*ft).m.statusat > 0 && (*ft).m.y >= (*ft).m.statusat as u32 {
                return format!("{}", (*ft).m.x).into();
            }
        }
        format_table_type::None
    }
}

/// Callback for `mouse_y`.
pub unsafe fn format_cb_mouse_y(ft: *mut format_tree) -> format_table_type {
    unsafe {
        if !(*ft).m.valid {
            return format_table_type::None;
        }
        let wp = cmd_mouse_pane(&raw mut (*ft).m, null_mut(), null_mut());
        let mut x: u32 = 0;
        let mut y: u32 = 0;
        if let Some(wp) = wp
            && cmd_mouse_at(wp.as_ptr(), &raw mut (*ft).m, &mut x, &mut y, 0) == 0
        {
            return format!("{y}").into();
        }
        if !(*ft).c.is_null() && (*(*ft).c).tty.flags.intersects(tty_flags::TTY_STARTED) {
            if (*ft).m.statusat == 0 && (*ft).m.y < (*ft).m.statuslines {
                return format!("{}", (*ft).m.y).into();
            }
            if (*ft).m.statusat > 0 && (*ft).m.y >= (*ft).m.statusat as u32 {
                return format!("{}", (*ft).m.y - (*ft).m.statusat as u32).into();
            }
        }
        format_table_type::None
    }
}

/// Callback for `next_session_id`.
pub unsafe fn format_cb_next_session_id(_ft: *mut format_tree) -> format_table_type {
    let value = NEXT_SESSION_ID.load(atomic::Ordering::Relaxed);
    format!("${value}").into()
}

/// Callback for `origin_flag`.
pub unsafe fn format_cb_origin_flag(ft: *mut format_tree) -> format_table_type {
    unsafe {
        if !(*ft).wp.is_null() {
            if (*(*ft).wp).base.mode.intersects(mode_flag::MODE_ORIGIN) {
                return "1".into();
            }
            return "0".into();
        }
        format_table_type::None
    }
}

/// Callback for `pane_active`.
pub unsafe fn format_cb_pane_active(ft: *mut format_tree) -> format_table_type {
    unsafe {
        if !(*ft).wp.is_null() {
            if (*ft).wp == (*(*(*ft).wp).window).active {
                return "1".into();
            }
            return "0".into();
        }
        format_table_type::None
    }
}

/// Callback for `pane_at_left`.
pub unsafe fn format_cb_pane_at_left(ft: *mut format_tree) -> format_table_type {
    unsafe {
        if !(*ft).wp.is_null() {
            if (*(*ft).wp).xoff == 0 {
                return "1".into();
            }
            return "0".into();
        }
        format_table_type::None
    }
}

/// Callback for `pane_at_right`.
pub unsafe fn format_cb_pane_at_right(ft: *mut format_tree) -> format_table_type {
    unsafe {
        if !(*ft).wp.is_null() {
            if (*(*ft).wp).xoff + (*(*ft).wp).sx == (*(*(*ft).wp).window).sx {
                return "1".into();
            }
            return "0".into();
        }
        format_table_type::None
    }
}

/// Callback for `pane_bottom`.
pub unsafe fn format_cb_pane_bottom(ft: *mut format_tree) -> format_table_type {
    unsafe {
        if !(*ft).wp.is_null() {
            return format!("{}", (*(*ft).wp).yoff + (*(*ft).wp).sy - 1).into();
        }
        format_table_type::None
    }
}

/// Callback for `pane_dead`.
pub unsafe fn format_cb_pane_dead(ft: *mut format_tree) -> format_table_type {
    unsafe {
        if !(*ft).wp.is_null() {
            if (*(*ft).wp).fd == -1 {
                return "1".into();
            }
            return "0".into();
        }
        format_table_type::None
    }
}

/// Callback for `pane_dead_signal`.
pub unsafe fn format_cb_pane_dead_signal(ft: *mut format_tree) -> format_table_type {
    unsafe {
        let wp = (*ft).wp;
        if !wp.is_null() {
            if (*wp).flags.intersects(window_pane_flags::PANE_STATUSREADY)
                && WIFSIGNALED((*wp).status)
            {
                return format!("{}", WTERMSIG((*wp).status)).into();
            }
            return format_table_type::None;
        }
        format_table_type::None
    }
}

/// Callback for `pane_dead_status`.
pub unsafe fn format_cb_pane_dead_status(ft: *mut format_tree) -> format_table_type {
    unsafe {
        let wp = (*ft).wp;
        if !wp.is_null() {
            if (*wp).flags.intersects(window_pane_flags::PANE_STATUSREADY)
                && WIFEXITED((*wp).status)
            {
                return format!("{}", WEXITSTATUS((*wp).status)).into();
            }
            return format_table_type::None;
        }
        format_table_type::None
    }
}

/// Callback for `pane_dead_time`.
pub unsafe fn format_cb_pane_dead_time(ft: *mut format_tree) -> format_table_type {
    unsafe {
        let wp = (*ft).wp;
        if !wp.is_null() && (*wp).flags.intersects(window_pane_flags::PANE_STATUSDRAWN) {
            return format_table_type::Time((*wp).dead_time);
        }
        format_table_type::None
    }
}

/// Callback for `pane_format`.
pub unsafe fn format_cb_pane_format(ft: *mut format_tree) -> format_table_type {
    unsafe {
        if (*ft).type_ == format_type::FORMAT_TYPE_PANE {
            return "1".into();
        }
        "0".into()
    }
}

/// Callback for `pane_height`.
pub unsafe fn format_cb_pane_height(ft: *mut format_tree) -> format_table_type {
    unsafe {
        if !(*ft).wp.is_null() {
            return format!("{}", (*(*ft).wp).sy).into();
        }
        format_table_type::None
    }
}

/// Callback for `pane_id`.
pub unsafe fn format_cb_pane_id(ft: *mut format_tree) -> format_table_type {
    unsafe {
        if !(*ft).wp.is_null() {
            return format!("%{}", (*(*ft).wp).id).into();
        }
        format_table_type::None
    }
}

/// Callback for `pane_index`.
pub unsafe fn format_cb_pane_index(ft: *mut format_tree) -> format_table_type {
    unsafe {
        let mut idx: u32 = 0;
        if !(*ft).wp.is_null() && window_pane_index((*ft).wp, &mut idx) == 0 {
            return format!("{idx}").into();
        }
        format_table_type::None
    }
}

/// Callback for `pane_input_off`.
pub unsafe fn format_cb_pane_input_off(ft: *mut format_tree) -> format_table_type {
    unsafe {
        if !(*ft).wp.is_null() {
            if (*(*ft).wp)
                .flags
                .intersects(window_pane_flags::PANE_INPUTOFF)
            {
                return "1".into();
            }
            return "0".into();
        }
        format_table_type::None
    }
}

/// Callback for `pane_unseen_changes`.
pub unsafe fn format_cb_pane_unseen_changes(ft: *mut format_tree) -> format_table_type {
    unsafe {
        if !(*ft).wp.is_null() {
            if (*(*ft).wp)
                .flags
                .intersects(window_pane_flags::PANE_UNSEENCHANGES)
            {
                return "1".into();
            }
            return "0".into();
        }
        format_table_type::None
    }
}

/// Callback for `pane_key_mode`.
pub unsafe fn format_cb_pane_key_mode(ft: *mut format_tree) -> format_table_type {
    unsafe {
        if !(*ft).wp.is_null() && !(*(*ft).wp).screen.is_null() {
            match (*(*(*ft).wp).screen).mode & EXTENDED_KEY_MODES {
                mode_flag::MODE_KEYS_EXTENDED => return "Ext 1".into(),
                mode_flag::MODE_KEYS_EXTENDED_2 => {
                    return "Ext 2".into();
                }
                _ => return "VT10x".into(),
            }
        }
        format_table_type::None
    }
}

/// Callback for `pane_last`.
pub unsafe fn format_cb_pane_last(ft: *mut format_tree) -> format_table_type {
    unsafe {
        if !(*ft).wp.is_null() {
            if (*ft).wp == tailq_first(&raw mut (*(*(*ft).wp).window).last_panes) {
                return "1".into();
            }
            return "0".into();
        }
        format_table_type::None
    }
}

/// Callback for `pane_left`.
pub unsafe fn format_cb_pane_left(ft: *mut format_tree) -> format_table_type {
    unsafe {
        if !(*ft).wp.is_null() {
            return format!("{}", (*(*ft).wp).xoff).into();
        }
        format_table_type::None
    }
}

/// Callback for `pane_marked`.
pub unsafe fn format_cb_pane_marked(ft: *mut format_tree) -> format_table_type {
    unsafe {
        if !(*ft).wp.is_null() {
            if server_check_marked() && MARKED_PANE.wp == (*ft).wp {
                return "1".into();
            }
            return "0".into();
        }
        format_table_type::None
    }
}

/// Callback for `pane_marked_set`.
pub unsafe fn format_cb_pane_marked_set(ft: *mut format_tree) -> format_table_type {
    unsafe {
        if !(*ft).wp.is_null() {
            if server_check_marked() {
                return "1".into();
            }
            return "0".into();
        }
        format_table_type::None
    }
}

/// Callback for `pane_mode`.
pub unsafe fn format_cb_pane_mode(ft: *mut format_tree) -> format_table_type {
    unsafe {
        if !(*ft).wp.is_null() {
            let wme = tailq_first(&raw mut (*(*ft).wp).modes);
            if !wme.is_null() {
                return (*(*wme).mode).name.into();
            }
            return format_table_type::None;
        }
        format_table_type::None
    }
}

/// Callback for `pane_path`.
pub unsafe fn format_cb_pane_path(ft: *mut format_tree) -> format_table_type {
    unsafe {
        if !(*ft).wp.is_null() {
            if (*(*ft).wp).base.path.is_null() {
                return "".into();
            }
            return format!("{}", _s((*(*ft).wp).base.path)).into();
        }
        format_table_type::None
    }
}

/// Callback for `pane_pid`.
pub unsafe fn format_cb_pane_pid(ft: *mut format_tree) -> format_table_type {
    unsafe {
        if !(*ft).wp.is_null() {
            return format!("{}", (*(*ft).wp).pid as i64).into();
        }
        format_table_type::None
    }
}

/// Callback for `pane_pipe`.
pub unsafe fn format_cb_pane_pipe(ft: *mut format_tree) -> format_table_type {
    unsafe {
        if !(*ft).wp.is_null() {
            if (*(*ft).wp).pipe_fd != -1 {
                return "1".into();
            }
            return "0".into();
        }
        format_table_type::None
    }
}

/// Callback for `pane_right`.
pub unsafe fn format_cb_pane_right(ft: *mut format_tree) -> format_table_type {
    unsafe {
        if !(*ft).wp.is_null() {
            return format!("{}", (*(*ft).wp).xoff + (*(*ft).wp).sx - 1).into();
        }
        format_table_type::None
    }
}

/// Callback for `pane_search_string`.
pub unsafe fn format_cb_pane_search_string(ft: *mut format_tree) -> format_table_type {
    unsafe {
        if !(*ft).wp.is_null() {
            if (*(*ft).wp).searchstr.is_null() {
                return "".into();
            }
            return format!("{}", _s((*(*ft).wp).searchstr)).into();
        }
        format_table_type::None
    }
}

/// Callback for `pane_synchronized`.
pub unsafe fn format_cb_pane_synchronized(ft: *mut format_tree) -> format_table_type {
    unsafe {
        if !(*ft).wp.is_null() {
            if options_get_number___::<i64>(&*(*(*ft).wp).options, "synchronize-panes") != 0 {
                return "1".into();
            }
            return "0".into();
        }
        format_table_type::None
    }
}

/// Callback for `pane_title`.
pub unsafe fn format_cb_pane_title(ft: *mut format_tree) -> format_table_type {
    unsafe {
        if !(*ft).wp.is_null() {
            return format!("{}", _s((*(*ft).wp).base.title)).into();
        }
        format_table_type::None
    }
}

/// Callback for `pane_top`.
pub unsafe fn format_cb_pane_top(ft: *mut format_tree) -> format_table_type {
    unsafe {
        if !(*ft).wp.is_null() {
            return format!("{}", (*(*ft).wp).yoff).into();
        }
        format_table_type::None
    }
}

/// Callback for `pane_tty`.
pub unsafe fn format_cb_pane_tty(ft: *mut format_tree) -> format_table_type {
    unsafe {
        if !(*ft).wp.is_null() {
            return format!("{}", _s((*(*ft).wp).tty.as_ptr())).into();
        }
        format_table_type::None
    }
}

/// Callback for `pane_width`.
pub unsafe fn format_cb_pane_width(ft: *mut format_tree) -> format_table_type {
    unsafe {
        if !(*ft).wp.is_null() {
            return format!("{}", (*(*ft).wp).sx).into();
        }
        format_table_type::None
    }
}

/// Callback for `scroll_region_lower`.
pub unsafe fn format_cb_scroll_region_lower(ft: *mut format_tree) -> format_table_type {
    unsafe {
        if !(*ft).wp.is_null() {
            return format!("{}", (*(*ft).wp).base.rlower).into();
        }
        format_table_type::None
    }
}

/// Callback for `scroll_region_upper`.
pub unsafe fn format_cb_scroll_region_upper(ft: *mut format_tree) -> format_table_type {
    unsafe {
        if !(*ft).wp.is_null() {
            return format!("{}", (*(*ft).wp).base.rupper).into();
        }
        format_table_type::None
    }
}

/// Callback for `server_sessions`.
pub unsafe fn format_cb_server_sessions(_ft: *mut format_tree) -> format_table_type {
    unsafe {
        let n: u32 = rb_foreach(&raw mut SESSIONS).count() as u32;
        format!("{n}").into()
    }
}

/// Callback for `session_attached`.
pub unsafe fn format_cb_session_attached(ft: *mut format_tree) -> format_table_type {
    unsafe {
        if !(*ft).s.is_null() {
            return format!("{}", (*(*ft).s).attached).into();
        }
        format_table_type::None
    }
}

/// Callback for `session_format`.
pub unsafe fn format_cb_session_format(ft: *mut format_tree) -> format_table_type {
    unsafe {
        if (*ft).type_ == format_type::FORMAT_TYPE_SESSION {
            return "1".into();
        }
        "0".into()
    }
}

/// Callback for `session_group`.
pub unsafe fn format_cb_session_group(ft: *mut format_tree) -> format_table_type {
    unsafe {
        if !(*ft).s.is_null() {
            let sg = session_group_contains((*ft).s);
            if !sg.is_null() {
                return format!("{}", (*sg).name).into();
            }
        }
        format_table_type::None
    }
}

/// Callback for `session_group_attached`.
pub unsafe fn format_cb_session_group_attached(ft: *mut format_tree) -> format_table_type {
    unsafe {
        if !(*ft).s.is_null() {
            let sg = session_group_contains((*ft).s);
            if !sg.is_null() {
                return format!("{}", session_group_attached_count(sg)).into();
            }
        }
        format_table_type::None
    }
}

/// Callback for `session_group_many_attached`.
pub unsafe fn format_cb_session_group_many_attached(ft: *mut format_tree) -> format_table_type {
    unsafe {
        if !(*ft).s.is_null() {
            let sg = session_group_contains((*ft).s);
            if !sg.is_null() {
                if session_group_attached_count(sg) > 1 {
                    return "1".into();
                }
                return "0".into();
            }
        }
        format_table_type::None
    }
}

/// Callback for `session_group_size`.
pub unsafe fn format_cb_session_group_size(ft: *mut format_tree) -> format_table_type {
    unsafe {
        if !(*ft).s.is_null() {
            let sg = session_group_contains((*ft).s);
            if !sg.is_null() {
                return format!("{}", session_group_count(sg)).into();
            }
        }
        format_table_type::None
    }
}

/// Callback for `session_grouped`.
pub unsafe fn format_cb_session_grouped(ft: *mut format_tree) -> format_table_type {
    unsafe {
        if !(*ft).s.is_null() {
            if !session_group_contains((*ft).s).is_null() {
                return "1".into();
            }
            return "0".into();
        }
        format_table_type::None
    }
}

/// Callback for `session_id`.
pub unsafe fn format_cb_session_id(ft: *mut format_tree) -> format_table_type {
    unsafe {
        if !(*ft).s.is_null() {
            return format!("${}", (*(*ft).s).id).into();
        }
        format_table_type::None
    }
}

/// Callback for `session_many_attached`.
pub unsafe fn format_cb_session_many_attached(ft: *mut format_tree) -> format_table_type {
    unsafe {
        if !(*ft).s.is_null() {
            if (*(*ft).s).attached > 1 {
                return "1".into();
            }
            return "0".into();
        }
        format_table_type::None
    }
}

/// Callback for `session_marked`.
pub unsafe fn format_cb_session_marked(ft: *mut format_tree) -> format_table_type {
    unsafe {
        if !(*ft).s.is_null() {
            if server_check_marked() && MARKED_PANE.s == (*ft).s {
                return "1".into();
            }
            return "0".into();
        }
        format_table_type::None
    }
}

/// Callback for `session_name`.
pub unsafe fn format_cb_session_name(ft: *mut format_tree) -> format_table_type {
    unsafe {
        if !(*ft).s.is_null() {
            return format!("{}", (*(*ft).s).name).into();
        }
        format_table_type::None
    }
}

/// Callback for `session_path`.
pub unsafe fn format_cb_session_path(ft: *mut format_tree) -> format_table_type {
    unsafe {
        if !(*ft).s.is_null() {
            return format!("{}", _s((*(*ft).s).cwd)).into();
        }
        format_table_type::None
    }
}

/// Callback for `session_windows`.
pub unsafe fn format_cb_session_windows(ft: *mut format_tree) -> format_table_type {
    unsafe {
        if !(*ft).s.is_null() {
            return format!("{}", winlink_count(&raw mut (*(*ft).s).windows)).into();
        }
        format_table_type::None
    }
}

/// Callback for `socket_path`.
pub unsafe fn format_cb_socket_path(_ft: *mut format_tree) -> format_table_type {
    unsafe { format!("{}", _s(SOCKET_PATH)).into() }
}

/// Callback for version.
pub fn format_cb_version(_ft: *mut format_tree) -> format_table_type {
    getversion().into()
}

/// Callback for `active_window_index`.
pub unsafe fn format_cb_active_window_index(ft: *mut format_tree) -> format_table_type {
    unsafe {
        if !(*ft).s.is_null() {
            return format!("{}", (*(*(*ft).s).curw).idx).into();
        }
        format_table_type::None
    }
}

/// Callback for `last_window_index`.
pub unsafe fn format_cb_last_window_index(ft: *mut format_tree) -> format_table_type {
    unsafe {
        if !(*ft).s.is_null() {
            let wl = rb_max(&raw mut (*(*ft).s).windows);
            return format!("{}", (*wl).idx).into();
        }
        format_table_type::None
    }
}

/// Callback for `window_active`.
pub unsafe fn format_cb_window_active(ft: *mut format_tree) -> format_table_type {
    unsafe {
        if !(*ft).wl.is_null() {
            if (*ft).wl == (*(*(*ft).wl).session).curw {
                return "1".into();
            }
            return "0".into();
        }
        format_table_type::None
    }
}

/// Callback for `window_activity_flag`.
pub unsafe fn format_cb_window_activity_flag(ft: *mut format_tree) -> format_table_type {
    unsafe {
        if !(*ft).wl.is_null() {
            if (*(*ft).wl)
                .flags
                .intersects(winlink_flags::WINLINK_ACTIVITY)
            {
                return "1".into();
            }
            return "0".into();
        }
        format_table_type::None
    }
}

/// Callback for `window_bell_flag`.
pub unsafe fn format_cb_window_bell_flag(ft: *mut format_tree) -> format_table_type {
    unsafe {
        if !(*ft).wl.is_null() {
            if (*(*ft).wl).flags.intersects(winlink_flags::WINLINK_BELL) {
                return "1".into();
            }
            return "0".into();
        }
        format_table_type::None
    }
}

/// Callback for `window_bigger`.
pub unsafe fn format_cb_window_bigger(ft: *mut format_tree) -> format_table_type {
    unsafe {
        if !(*ft).c.is_null() {
            let mut ox: u32 = 0;
            let mut oy: u32 = 0;
            let mut sx: u32 = 0;
            let mut sy: u32 = 0;
            if tty_window_offset(&raw mut (*(*ft).c).tty, &mut ox, &mut oy, &mut sx, &mut sy) != 0 {
                return "1".into();
            }
            return "0".into();
        }
        format_table_type::None
    }
}

/// Callback for `window_cell_height`.
pub unsafe fn format_cb_window_cell_height(ft: *mut format_tree) -> format_table_type {
    unsafe {
        if !(*ft).w.is_null() {
            return format!("{}", (*(*ft).w).ypixel).into();
        }
        format_table_type::None
    }
}

/// Callback for `window_cell_width`.
pub unsafe fn format_cb_window_cell_width(ft: *mut format_tree) -> format_table_type {
    unsafe {
        if !(*ft).w.is_null() {
            return format!("{}", (*(*ft).w).xpixel).into();
        }
        format_table_type::None
    }
}

/// Callback for `window_end_flag`.
pub unsafe fn format_cb_window_end_flag(ft: *mut format_tree) -> format_table_type {
    unsafe {
        if !(*ft).wl.is_null() {
            if (*ft).wl == rb_max(&raw mut (*(*(*ft).wl).session).windows) {
                return "1".into();
            }
            return "0".into();
        }
        format_table_type::None
    }
}

/// Callback for `window_flags`.
pub unsafe fn format_cb_window_flags(ft: *mut format_tree) -> format_table_type {
    unsafe {
        if !(*ft).wl.is_null() {
            return format!("{}", _s(window_printable_flags((*ft).wl, 1))).into();
        }
        format_table_type::None
    }
}

/// Callback for `window_format`.
pub unsafe fn format_cb_window_format(ft: *mut format_tree) -> format_table_type {
    unsafe {
        if (*ft).type_ == format_type::FORMAT_TYPE_WINDOW {
            return "1".into();
        }
        "0".into()
    }
}

/// Callback for `window_height`.
pub unsafe fn format_cb_window_height(ft: *mut format_tree) -> format_table_type {
    unsafe {
        if !(*ft).w.is_null() {
            return format!("{}", (*(*ft).w).sy).into();
        }
        format_table_type::None
    }
}

/// Callback for `window_id`.
pub unsafe fn format_cb_window_id(ft: *mut format_tree) -> format_table_type {
    unsafe {
        if !(*ft).w.is_null() {
            return format!("@{}", (*(*ft).w).id).into();
        }
        format_table_type::None
    }
}

/// Callback for `window_index`.
pub unsafe fn format_cb_window_index(ft: *mut format_tree) -> format_table_type {
    unsafe {
        if !(*ft).wl.is_null() {
            return format!("{}", (*(*ft).wl).idx).into();
        }
        format_table_type::None
    }
}

/// Callback for `window_last_flag`.
pub unsafe fn format_cb_window_last_flag(ft: *mut format_tree) -> format_table_type {
    unsafe {
        if !(*ft).wl.is_null() {
            if (*ft).wl == tailq_first(&raw mut (*(*(*ft).wl).session).lastw) {
                return "1".into();
            }
            return "0".into();
        }
        format_table_type::None
    }
}

/// Callback for `window_linked`.
pub unsafe fn format_cb_window_linked(ft: *mut format_tree) -> format_table_type {
    unsafe {
        if !(*ft).wl.is_null() {
            if session_is_linked((*(*ft).wl).session, (*(*ft).wl).window) {
                return "1".into();
            }
            return "0".into();
        }
        format_table_type::None
    }
}

/// Callback for `window_linked_sessions`.
pub unsafe fn format_cb_window_linked_sessions(ft: *mut format_tree) -> format_table_type {
    unsafe {
        if !(*ft).wl.is_null() {
            return format!("{}", (*(*(*ft).wl).window).references).into();
        }
        format_table_type::None
    }
}

/// Callback for `window_marked_flag`.
pub unsafe fn format_cb_window_marked_flag(ft: *mut format_tree) -> format_table_type {
    unsafe {
        if !(*ft).wl.is_null() {
            if server_check_marked() && MARKED_PANE.wl == (*ft).wl {
                return "1".into();
            }
            return "0".into();
        }
        format_table_type::None
    }
}

/// Callback for `window_name`.
pub unsafe fn format_cb_window_name(ft: *mut format_tree) -> format_table_type {
    unsafe {
        if !(*ft).w.is_null() {
            return format!("{}", _s((*(*ft).w).name)).into();
        }
        format_table_type::None
    }
}

/// Callback for `window_offset_x`.
pub unsafe fn format_cb_window_offset_x(ft: *mut format_tree) -> format_table_type {
    unsafe {
        if !(*ft).c.is_null() {
            let mut ox: u32 = 0;
            let mut oy: u32 = 0;
            let mut sx: u32 = 0;
            let mut sy: u32 = 0;
            if tty_window_offset(&raw mut (*(*ft).c).tty, &mut ox, &mut oy, &mut sx, &mut sy) != 0 {
                return format!("{ox}").into();
            }
        }
        format_table_type::None
    }
}

/// Callback for `window_offset_y`.
pub unsafe fn format_cb_window_offset_y(ft: *mut format_tree) -> format_table_type {
    unsafe {
        if !(*ft).c.is_null() {
            let mut ox: u32 = 0;
            let mut oy: u32 = 0;
            let mut sx: u32 = 0;
            let mut sy: u32 = 0;
            if tty_window_offset(&raw mut (*(*ft).c).tty, &mut ox, &mut oy, &mut sx, &mut sy) != 0 {
                return format!("{oy}").into();
            }
        }
        format_table_type::None
    }
}

/// Callback for `window_panes`.
pub unsafe fn format_cb_window_panes(ft: *mut format_tree) -> format_table_type {
    unsafe {
        if !(*ft).w.is_null() {
            return format!("{}", window_count_panes((*ft).w)).into();
        }
        format_table_type::None
    }
}

/// Callback for `window_raw_flags`.
pub unsafe fn format_cb_window_raw_flags(ft: *mut format_tree) -> format_table_type {
    unsafe {
        if !(*ft).wl.is_null() {
            return format!("{}", _s(window_printable_flags((*ft).wl, 0))).into();
        }
        format_table_type::None
    }
}

/// Callback for `window_silence_flag`.
pub unsafe fn format_cb_window_silence_flag(ft: *mut format_tree) -> format_table_type {
    unsafe {
        if !(*ft).wl.is_null() {
            if (*(*ft).wl).flags.intersects(winlink_flags::WINLINK_SILENCE) {
                return "1".into();
            }
            return "0".into();
        }
        format_table_type::None
    }
}

/// Callback for `window_start_flag`.
pub unsafe fn format_cb_window_start_flag(ft: *mut format_tree) -> format_table_type {
    unsafe {
        if !(*ft).wl.is_null() {
            if (*ft).wl == rb_min(&raw mut (*(*(*ft).wl).session).windows) {
                return "1".into();
            }
            return "0".into();
        }
        format_table_type::None
    }
}

/// Callback for `window_width`.
pub unsafe fn format_cb_window_width(ft: *mut format_tree) -> format_table_type {
    unsafe {
        if !(*ft).w.is_null() {
            return format!("{}", (*(*ft).w).sx).into();
        }
        format_table_type::None
    }
}

/// Callback for `window_zoomed_flag`.
pub unsafe fn format_cb_window_zoomed_flag(ft: *mut format_tree) -> format_table_type {
    unsafe {
        if !(*ft).w.is_null() {
            if (*(*ft).w).flags.intersects(window_flag::ZOOMED) {
                return "1".into();
            }
            return "0".into();
        }
        format_table_type::None
    }
}

/// Callback for `wrap_flag`.
pub unsafe fn format_cb_wrap_flag(ft: *mut format_tree) -> format_table_type {
    unsafe {
        if !(*ft).wp.is_null() {
            if (*(*ft).wp).base.mode.intersects(mode_flag::MODE_WRAP) {
                return "1".into();
            }
            return "0".into();
        }
        format_table_type::None
    }
}

/// Callback for `buffer_created`.
pub unsafe fn format_cb_buffer_created(ft: *mut format_tree) -> format_table_type {
    unsafe {
        if let Some(pb) = NonNull::new((*ft).pb) {
            format_table_type::Time(timeval {
                tv_sec: paste_buffer_created(pb),
                tv_usec: 0,
            })
        } else {
            format_table_type::None
        }
    }
}

/// Callback for `client_activity`.
pub unsafe fn format_cb_client_activity(ft: *mut format_tree) -> format_table_type {
    unsafe {
        if !(*ft).c.is_null() {
            return format_table_type::Time((*(*ft).c).activity_time);
        }
        format_table_type::None
    }
}

/// Callback for `client_created`.
pub unsafe fn format_cb_client_created(ft: *mut format_tree) -> format_table_type {
    unsafe {
        if !(*ft).c.is_null() {
            return format_table_type::Time((*(*ft).c).creation_time);
        }
        format_table_type::None
    }
}

/// Callback for `session_activity`.
pub unsafe fn format_cb_session_activity(ft: *mut format_tree) -> format_table_type {
    unsafe {
        if !(*ft).s.is_null() {
            return format_table_type::Time((*(*ft).s).activity_time);
        }
        format_table_type::None
    }
}

/// Callback for `session_created`.
pub unsafe fn format_cb_session_created(ft: *mut format_tree) -> format_table_type {
    unsafe {
        if !(*ft).s.is_null() {
            return format_table_type::Time((*(*ft).s).creation_time);
        }
        format_table_type::None
    }
}

/// Callback for `session_last_attached`.
pub unsafe fn format_cb_session_last_attached(ft: *mut format_tree) -> format_table_type {
    unsafe {
        if !(*ft).s.is_null() {
            return format_table_type::Time((*(*ft).s).last_attached_time);
        }
        format_table_type::None
    }
}

/// Callback for `start_time`.
pub unsafe fn format_cb_start_time(_ft: *mut format_tree) -> format_table_type {
    format_table_type::Time(unsafe { START_TIME })
}

/// Callback for `window_activity`.
pub unsafe fn format_cb_window_activity(ft: *mut format_tree) -> format_table_type {
    unsafe {
        if !(*ft).w.is_null() {
            return format_table_type::Time((*(*ft).w).activity_time);
        }
        format_table_type::None
    }
}

/// Callback for `buffer_mode_format`.
pub unsafe fn format_cb_buffer_mode_format(_ft: *mut format_tree) -> format_table_type {
    WINDOW_BUFFER_MODE.default_format.unwrap().into()
}

/// Callback for `client_mode_format`.
pub unsafe fn format_cb_client_mode_format(_ft: *mut format_tree) -> format_table_type {
    WINDOW_CLIENT_MODE.default_format.unwrap().into()
}

/// Callback for `tree_mode_format`.
pub unsafe fn format_cb_tree_mode_format(_ft: *mut format_tree) -> format_table_type {
    WINDOW_TREE_MODE.default_format.unwrap().into()
}

/// Callback for uid.
pub unsafe fn format_cb_uid(_ft: *mut format_tree) -> format_table_type {
    unsafe { format!("{}", getuid() as i64).into() }
}

/// Callback for user.
pub unsafe fn format_cb_user(_ft: *mut format_tree) -> format_table_type {
    unsafe {
        if let Some(pw) = NonNull::new(getpwuid(getuid())) {
            cstr_to_str((*pw.as_ptr()).pw_name.cast())
                .to_string()
                .into()
        } else {
            format_table_type::None
        }
    }
}

/// Format table type.
#[derive(Default)]
pub enum format_table_type {
    #[default]
    None,
    String(Cow<'static, str>),
    Time(timeval),
}

impl From<Cow<'static, str>> for format_table_type {
    fn from(value: Cow<'static, str>) -> Self {
        Self::String(value)
    }
}

impl From<String> for format_table_type {
    fn from(value: String) -> Self {
        format_table_type::String(Cow::Owned(value))
    }
}

impl From<&'static str> for format_table_type {
    fn from(value: &'static str) -> Self {
        format_table_type::String(Cow::Borrowed(value))
    }
}

/// Format table entry.
#[repr(C)]
pub struct format_table_entry {
    key: &'static str,
    cb: format_cb,
}

impl format_table_entry {
    pub const fn new(key: &'static str, cb: format_cb) -> Self {
        Self { key, cb }
    }
}

// Format table. Default format variables (that are almost always in the tree
// and where the value is expanded by a callback in this file) are listed
// here. Only variables which are added by the caller go into the tree.
static FORMAT_TABLE: &[format_table_entry] = &[
    format_table_entry::new("active_window_index", format_cb_active_window_index),
    format_table_entry::new("alternate_on", format_cb_alternate_on),
    format_table_entry::new("alternate_saved_x", format_cb_alternate_saved_x),
    format_table_entry::new("alternate_saved_y", format_cb_alternate_saved_y),
    format_table_entry::new("buffer_created", format_cb_buffer_created),
    format_table_entry::new("buffer_mode_format", format_cb_buffer_mode_format),
    format_table_entry::new("buffer_name", format_cb_buffer_name),
    format_table_entry::new("buffer_sample", format_cb_buffer_sample),
    format_table_entry::new("buffer_size", format_cb_buffer_size),
    format_table_entry::new("client_activity", format_cb_client_activity),
    format_table_entry::new("client_cell_height", format_cb_client_cell_height),
    format_table_entry::new("client_cell_width", format_cb_client_cell_width),
    format_table_entry::new("client_control_mode", format_cb_client_control_mode),
    format_table_entry::new("client_created", format_cb_client_created),
    format_table_entry::new("client_discarded", format_cb_client_discarded),
    format_table_entry::new("client_flags", format_cb_client_flags),
    format_table_entry::new("client_height", format_cb_client_height),
    format_table_entry::new("client_key_table", format_cb_client_key_table),
    format_table_entry::new("client_last_session", format_cb_client_last_session),
    format_table_entry::new("client_mode_format", format_cb_client_mode_format),
    format_table_entry::new("client_name", format_cb_client_name),
    format_table_entry::new("client_pid", format_cb_client_pid),
    format_table_entry::new("client_prefix", format_cb_client_prefix),
    format_table_entry::new("client_readonly", format_cb_client_readonly),
    format_table_entry::new("client_session", format_cb_client_session),
    format_table_entry::new("client_termfeatures", format_cb_client_termfeatures),
    format_table_entry::new("client_termname", format_cb_client_termname),
    format_table_entry::new("client_termtype", format_cb_client_termtype),
    format_table_entry::new("client_tty", format_cb_client_tty),
    format_table_entry::new("client_uid", format_cb_client_uid),
    format_table_entry::new("client_user", format_cb_client_user),
    format_table_entry::new("client_utf8", format_cb_client_utf8),
    format_table_entry::new("client_width", format_cb_client_width),
    format_table_entry::new("client_written", format_cb_client_written),
    format_table_entry::new("config_files", format_cb_config_files),
    format_table_entry::new("cursor_character", format_cb_cursor_character),
    format_table_entry::new("cursor_flag", format_cb_cursor_flag),
    format_table_entry::new("cursor_x", format_cb_cursor_x),
    format_table_entry::new("cursor_y", format_cb_cursor_y),
    format_table_entry::new("history_all_bytes", format_cb_history_all_bytes),
    format_table_entry::new("history_bytes", format_cb_history_bytes),
    format_table_entry::new("history_limit", format_cb_history_limit),
    format_table_entry::new("history_size", format_cb_history_size),
    format_table_entry::new("host", format_cb_host),
    format_table_entry::new("host_short", format_cb_host_short),
    format_table_entry::new("insert_flag", format_cb_insert_flag),
    format_table_entry::new("keypad_cursor_flag", format_cb_keypad_cursor_flag),
    format_table_entry::new("keypad_flag", format_cb_keypad_flag),
    format_table_entry::new("last_window_index", format_cb_last_window_index),
    format_table_entry::new("mouse_all_flag", format_cb_mouse_all_flag),
    format_table_entry::new("mouse_any_flag", format_cb_mouse_any_flag),
    format_table_entry::new("mouse_button_flag", format_cb_mouse_button_flag),
    format_table_entry::new("mouse_hyperlink", format_cb_mouse_hyperlink),
    format_table_entry::new("mouse_line", format_cb_mouse_line),
    format_table_entry::new("mouse_pane", format_cb_mouse_pane),
    format_table_entry::new("mouse_sgr_flag", format_cb_mouse_sgr_flag),
    format_table_entry::new("mouse_standard_flag", format_cb_mouse_standard_flag),
    format_table_entry::new("mouse_status_line", format_cb_mouse_status_line),
    format_table_entry::new("mouse_status_range", format_cb_mouse_status_range),
    format_table_entry::new("mouse_utf8_flag", format_cb_mouse_utf8_flag),
    format_table_entry::new("mouse_word", format_cb_mouse_word),
    format_table_entry::new("mouse_x", format_cb_mouse_x),
    format_table_entry::new("mouse_y", format_cb_mouse_y),
    format_table_entry::new("next_session_id", format_cb_next_session_id),
    format_table_entry::new("origin_flag", format_cb_origin_flag),
    format_table_entry::new("pane_active", format_cb_pane_active),
    format_table_entry::new("pane_at_bottom", format_cb_pane_at_bottom),
    format_table_entry::new("pane_at_left", format_cb_pane_at_left),
    format_table_entry::new("pane_at_right", format_cb_pane_at_right),
    format_table_entry::new("pane_at_top", format_cb_pane_at_top),
    format_table_entry::new("pane_bg", format_cb_pane_bg),
    format_table_entry::new("pane_bottom", format_cb_pane_bottom),
    format_table_entry::new("pane_current_command", format_cb_current_command),
    format_table_entry::new("pane_current_path", format_cb_current_path),
    format_table_entry::new("pane_dead", format_cb_pane_dead),
    format_table_entry::new("pane_dead_signal", format_cb_pane_dead_signal),
    format_table_entry::new("pane_dead_status", format_cb_pane_dead_status),
    format_table_entry::new("pane_dead_time", format_cb_pane_dead_time),
    format_table_entry::new("pane_fg", format_cb_pane_fg),
    format_table_entry::new("pane_format", format_cb_pane_format),
    format_table_entry::new("pane_height", format_cb_pane_height),
    format_table_entry::new("pane_id", format_cb_pane_id),
    format_table_entry::new("pane_in_mode", format_cb_pane_in_mode),
    format_table_entry::new("pane_index", format_cb_pane_index),
    format_table_entry::new("pane_input_off", format_cb_pane_input_off),
    format_table_entry::new("pane_key_mode", format_cb_pane_key_mode),
    format_table_entry::new("pane_last", format_cb_pane_last),
    format_table_entry::new("pane_left", format_cb_pane_left),
    format_table_entry::new("pane_marked", format_cb_pane_marked),
    format_table_entry::new("pane_marked_set", format_cb_pane_marked_set),
    format_table_entry::new("pane_mode", format_cb_pane_mode),
    format_table_entry::new("pane_path", format_cb_pane_path),
    format_table_entry::new("pane_pid", format_cb_pane_pid),
    format_table_entry::new("pane_pipe", format_cb_pane_pipe),
    format_table_entry::new("pane_right", format_cb_pane_right),
    format_table_entry::new("pane_search_string", format_cb_pane_search_string),
    format_table_entry::new("pane_start_command", format_cb_start_command),
    format_table_entry::new("pane_start_path", format_cb_start_path),
    format_table_entry::new("pane_synchronized", format_cb_pane_synchronized),
    format_table_entry::new("pane_tabs", format_cb_pane_tabs),
    format_table_entry::new("pane_title", format_cb_pane_title),
    format_table_entry::new("pane_top", format_cb_pane_top),
    format_table_entry::new("pane_tty", format_cb_pane_tty),
    format_table_entry::new("pane_unseen_changes", format_cb_pane_unseen_changes),
    format_table_entry::new("pane_width", format_cb_pane_width),
    format_table_entry::new("pid", format_cb_pid),
    format_table_entry::new("scroll_region_lower", format_cb_scroll_region_lower),
    format_table_entry::new("scroll_region_upper", format_cb_scroll_region_upper),
    format_table_entry::new("server_sessions", format_cb_server_sessions),
    format_table_entry::new("session_activity", format_cb_session_activity),
    format_table_entry::new("session_alerts", format_cb_session_alerts),
    format_table_entry::new("session_attached", format_cb_session_attached),
    format_table_entry::new("session_attached_list", format_cb_session_attached_list),
    format_table_entry::new("session_created", format_cb_session_created),
    format_table_entry::new("session_format", format_cb_session_format),
    format_table_entry::new("session_group", format_cb_session_group),
    format_table_entry::new("session_group_attached", format_cb_session_group_attached),
    format_table_entry::new(
        "session_group_attached_list",
        format_cb_session_group_attached_list,
    ),
    format_table_entry::new("session_group_list", format_cb_session_group_list),
    format_table_entry::new(
        "session_group_many_attached",
        format_cb_session_group_many_attached,
    ),
    format_table_entry::new("session_group_size", format_cb_session_group_size),
    format_table_entry::new("session_grouped", format_cb_session_grouped),
    format_table_entry::new("session_id", format_cb_session_id),
    format_table_entry::new("session_last_attached", format_cb_session_last_attached),
    format_table_entry::new("session_many_attached", format_cb_session_many_attached),
    format_table_entry::new("session_marked", format_cb_session_marked),
    format_table_entry::new("session_name", format_cb_session_name),
    format_table_entry::new("session_path", format_cb_session_path),
    format_table_entry::new("session_stack", format_cb_session_stack),
    format_table_entry::new("session_windows", format_cb_session_windows),
    format_table_entry::new("socket_path", format_cb_socket_path),
    format_table_entry::new("start_time", format_cb_start_time),
    format_table_entry::new("tree_mode_format", format_cb_tree_mode_format),
    format_table_entry::new("uid", format_cb_uid),
    format_table_entry::new("user", format_cb_user),
    format_table_entry::new("version", format_cb_version),
    format_table_entry::new("window_active", format_cb_window_active),
    format_table_entry::new("window_active_clients", format_cb_window_active_clients),
    format_table_entry::new(
        "window_active_clients_list",
        format_cb_window_active_clients_list,
    ),
    format_table_entry::new("window_active_sessions", format_cb_window_active_sessions),
    format_table_entry::new(
        "window_active_sessions_list",
        format_cb_window_active_sessions_list,
    ),
    format_table_entry::new("window_activity", format_cb_window_activity),
    format_table_entry::new("window_activity_flag", format_cb_window_activity_flag),
    format_table_entry::new("window_bell_flag", format_cb_window_bell_flag),
    format_table_entry::new("window_bigger", format_cb_window_bigger),
    format_table_entry::new("window_cell_height", format_cb_window_cell_height),
    format_table_entry::new("window_cell_width", format_cb_window_cell_width),
    format_table_entry::new("window_end_flag", format_cb_window_end_flag),
    format_table_entry::new("window_flags", format_cb_window_flags),
    format_table_entry::new("window_format", format_cb_window_format),
    format_table_entry::new("window_height", format_cb_window_height),
    format_table_entry::new("window_id", format_cb_window_id),
    format_table_entry::new("window_index", format_cb_window_index),
    format_table_entry::new("window_last_flag", format_cb_window_last_flag),
    format_table_entry::new("window_layout", format_cb_window_layout),
    format_table_entry::new("window_linked", format_cb_window_linked),
    format_table_entry::new("window_linked_sessions", format_cb_window_linked_sessions),
    format_table_entry::new(
        "window_linked_sessions_list",
        format_cb_window_linked_sessions_list,
    ),
    format_table_entry::new("window_marked_flag", format_cb_window_marked_flag),
    format_table_entry::new("window_name", format_cb_window_name),
    format_table_entry::new("window_offset_x", format_cb_window_offset_x),
    format_table_entry::new("window_offset_y", format_cb_window_offset_y),
    format_table_entry::new("window_panes", format_cb_window_panes),
    format_table_entry::new("window_raw_flags", format_cb_window_raw_flags),
    format_table_entry::new("window_silence_flag", format_cb_window_silence_flag),
    format_table_entry::new("window_stack_index", format_cb_window_stack_index),
    format_table_entry::new("window_start_flag", format_cb_window_start_flag),
    format_table_entry::new("window_visible_layout", format_cb_window_visible_layout),
    format_table_entry::new("window_width", format_cb_window_width),
    format_table_entry::new("window_zoomed_flag", format_cb_window_zoomed_flag),
    format_table_entry::new("wrap_flag", format_cb_wrap_flag),
];

pub unsafe fn format_table_compare(
    key: *const u8,
    entry: *const format_table_entry,
) -> std::cmp::Ordering {
    unsafe { strcmp_(key, (*entry).key) }
}

pub unsafe fn format_table_get(key: *const u8) -> Option<&'static format_table_entry> {
    unsafe {
        match FORMAT_TABLE.binary_search_by(|e| format_table_compare(key, e).reverse()) {
            Ok(idx) => Some(&FORMAT_TABLE[idx]),
            Err(_) => None,
        }
    }
}

pub unsafe fn format_merge(ft: *mut format_tree, from: *mut format_tree) {
    unsafe {
        for fe in rb_foreach(&raw mut (*from).tree).map(NonNull::as_ptr) {
            if !(*fe).value.is_null() {
                format_add!(ft, cstr_to_str((*fe).key), "{}", _s((*fe).value));
            }
        }
    }
}

pub unsafe fn format_get_pane(ft: *mut format_tree) -> *mut window_pane {
    unsafe { (*ft).wp }
}

pub unsafe fn format_create_add_item(ft: *mut format_tree, item: *mut cmdq_item) {
    unsafe {
        let event = cmdq_get_event(item);
        let m = &(*event).m;

        cmdq_merge_formats(item, ft);
        memcpy__(&raw mut (*ft).m, m);
    }
}

pub unsafe fn format_create(
    c: *mut client,
    item: *mut cmdq_item,
    tag: i32,
    flags: format_flags,
) -> *mut format_tree {
    unsafe {
        let ft = xcalloc1::<format_tree>() as *mut format_tree;
        rb_init(&raw mut (*ft).tree);

        if !c.is_null() {
            (*ft).client = c;
            (*c).references += 1;
        }
        (*ft).item = item;
        (*ft).tag = tag as u32;
        (*ft).flags = flags;

        if !item.is_null() {
            format_create_add_item(ft, item);
        }

        ft
    }
}

pub unsafe fn format_free(ft: *mut format_tree) {
    unsafe {
        for fe in rb_foreach(&raw mut (*ft).tree).map(NonNull::as_ptr) {
            rb_remove(&raw mut (*ft).tree, fe);
            free_((*fe).value);
            free_((*fe).key);
            free_(fe);
        }

        if !(*ft).client.is_null() {
            server_client_unref((*ft).client);
        }
        free(ft as *mut c_void);
    }
}

pub unsafe fn format_log_debug_cb(key: &str, value: &str, prefix: *mut u8) {
    unsafe {
        log_debug!("{}: {}={}", _s(prefix), key, value);
    }
}

pub unsafe fn format_log_debug(ft: *mut format_tree, prefix: *const u8) {
    unsafe {
        format_each(ft, format_log_debug_cb, prefix.cast_mut());
    }
}

pub unsafe fn format_each<T>(ft: *mut format_tree, cb: unsafe fn(&str, &str, *mut T), arg: *mut T) {
    unsafe {
        for fte in FORMAT_TABLE {
            let value = (fte.cb)(ft);
            match value {
                format_table_type::None => continue,
                format_table_type::Time(tv) => {
                    let s = format!("{}", tv.tv_sec);
                    cb(fte.key, &s, arg);
                }
                format_table_type::String(string) => {
                    cb(fte.key, &string, arg);
                }
            }
        }

        for fe in rb_foreach(&raw mut (*ft).tree).map(NonNull::as_ptr) {
            if (*fe).time != 0 {
                let s = format!("{}", (*fe).time);
                cb(cstr_to_str((*fe).key), &s, arg);
            } else {
                if let Some(fe_cb) = (*fe).cb
                    && (*fe).value.is_null()
                {
                    (*fe).value = match fe_cb(ft) {
                        format_table_type::None => CString::default().into_raw().cast(),
                        format_table_type::String(cow) => {
                            CString::new(cow.into_owned()).unwrap().into_raw().cast()
                        }
                        format_table_type::Time(_timeval) => unreachable!("unreachable?"),
                    }
                }
                cb(cstr_to_str((*fe).key), cstr_to_str((*fe).value), arg);
            }
        }
    }
}

macro_rules! format_add {
   ($state:expr, $key:expr, $fmt:literal $(, $args:expr)* $(,)?) => {
        crate::format::format_add_($state, $key, format_args!($fmt $(, $args)*))
    };
}
pub(crate) use format_add;

/// Add a key-value pair.
pub unsafe fn format_add_(ft: *mut format_tree, key: &str, args: std::fmt::Arguments) {
    unsafe {
        let fe = Box::leak(Box::new(format_entry {
            key: xstrdup__(key),
            value: null_mut(),
            time: 0,
            cb: None,
            entry: zeroed(),
        })) as *mut format_entry;

        let fe = match rb_insert(&raw mut (*ft).tree, fe) {
            fe_now if !fe_now.is_null() => {
                free_((*fe).key);
                free_(fe);
                free_((*fe_now).value);
                fe_now
            }
            _ => fe,
        };

        let mut value = args.to_string();
        value.push('\0');
        (*fe).value = value.leak().as_mut_ptr().cast();
    }
}

/// Add a key and time.
pub unsafe fn format_add_tv(ft: *mut format_tree, key: *const u8, tv: *const timeval) {
    unsafe {
        let fe = Box::leak(Box::new(format_entry {
            key: xstrdup(key).as_ptr(),
            value: null_mut(),
            time: (*tv).tv_sec,
            cb: None,
            entry: zeroed(),
        })) as *mut format_entry;

        let fe_now = rb_insert(&raw mut (*ft).tree, fe);
        if !fe_now.is_null() {
            free_((*fe).key);
            free_(fe);
            free_((*fe_now).value);
        }
    }
}

/// Add a key and function.
pub unsafe fn format_add_cb(ft: *mut format_tree, key: *const u8, cb: format_cb) {
    unsafe {
        let fe = Box::leak(Box::new(format_entry {
            key: xstrdup(key).as_ptr(),
            value: null_mut(),
            time: 0,
            cb: Some(cb),
            entry: zeroed(),
        })) as *mut format_entry;

        let fe_now = rb_insert(&raw mut (*ft).tree, fe);
        if !fe_now.is_null() {
            free_((*fe).key);
            free_(fe);
            free_((*fe_now).value);
        }
    }
}

/// Quote shell special characters in string.
pub unsafe fn format_quote_shell(s: *const u8) -> *mut u8 {
    unsafe {
        let out: *mut u8 = xmalloc(strlen(s) * 2 + 1).as_ptr().cast();
        let mut at = out;
        let mut cp = s;
        while *cp != b'\0' {
            if !strchr(c!("|&;<>()$`\\\"'*?[# =%"), *cp as i32).is_null() {
                *at = b'\\';
                at = at.add(1);
            }
            *at = *cp;
            at = at.add(1);
            cp = cp.add(1);
        }
        *at = b'\0';
        out
    }
}

/// Quote #s in string.
pub unsafe fn format_quote_style(s: *const u8) -> *mut u8 {
    unsafe {
        let out: *mut u8 = xmalloc(strlen(s) * 2 + 1).as_ptr().cast();
        let mut at = out;

        let mut cp = s;
        while *cp != b'\0' {
            if *cp == b'#' {
                *at = b'#';
                at = at.add(1);
            }
            *at = *cp;
            at = at.add(1);
            cp = cp.add(1);
        }
        *at = b'\0';
        out
    }
}

/// Make a prettier time.
pub unsafe fn format_pretty_time(t: time_t, seconds: i32) -> *mut u8 {
    unsafe {
        let mut now: time_t = libc::time(null_mut());
        if now < t {
            now = t;
        }
        let age = now - t;

        let mut now_tm = MaybeUninit::<tm>::uninit();
        let now_tm = now_tm.as_mut_ptr();
        let mut tm = MaybeUninit::<tm>::uninit();
        let tm = tm.as_mut_ptr();

        localtime_r(&raw const now, now_tm);
        localtime_r(&raw const t, tm);

        // Last 24 hours.
        const SIZEOF_S: usize = 9;
        let mut s = [0u8; 9];
        if age < 24 * 3600 {
            if seconds != 0 {
                strftime(s.as_mut_ptr(), SIZEOF_S, c!("%H:%M:%S"), tm);
            } else {
                strftime(s.as_mut_ptr(), SIZEOF_S, c!("%H:%M"), tm);
            }
            return xstrdup(s.as_ptr()).as_ptr();
        }

        // This month or last 28 days.
        if ((*tm).tm_year == (*now_tm).tm_year && (*tm).tm_mon == (*now_tm).tm_mon)
            || age < 28 * 24 * 3600
        {
            strftime(s.as_mut_ptr(), SIZEOF_S, c!("%a%d"), tm);
            return xstrdup(s.as_ptr()).as_ptr();
        }

        // Last 12 months.
        if ((*tm).tm_year == (*now_tm).tm_year && (*tm).tm_mon < (*now_tm).tm_mon)
            || ((*tm).tm_year == (*now_tm).tm_year - 1 && (*tm).tm_mon > (*now_tm).tm_mon)
        {
            strftime(s.as_mut_ptr(), SIZEOF_S, c!("%d%b"), tm);
            return xstrdup(s.as_ptr()).as_ptr();
        }

        // Older than that.
        strftime(s.as_mut_ptr(), SIZEOF_S, c!("%h%y"), tm);
        xstrdup(s.as_ptr()).as_ptr()
    }
}

/// Find a format entry.
fn format_find(
    ft: *mut format_tree,
    key: *const u8,
    modifiers: format_modifiers,
    time_format: *const u8,
) -> *mut u8 {
    unsafe {
        let mut s = MaybeUninit::<[u8; 512]>::uninit();
        let s = s.as_mut_ptr() as *mut u8;
        let mut fe_find = MaybeUninit::<format_entry>::uninit();

        const SIZEOF_S: usize = 512;
        let mut t: time_t = 0;
        let mut idx = 0;
        let mut found = null_mut();

        'found: {
            let mut o = options_parse_get(GLOBAL_OPTIONS, cstr_to_str(key), &raw mut idx, 0);
            if o.is_null() && !(*ft).wp.is_null() {
                o = options_parse_get((*(*ft).wp).options, cstr_to_str(key), &raw mut idx, 0);
            }
            if o.is_null() && !(*ft).w.is_null() {
                o = options_parse_get((*(*ft).w).options, cstr_to_str(key), &raw mut idx, 0);
            }
            if o.is_null() {
                o = options_parse_get(GLOBAL_W_OPTIONS, cstr_to_str(key), &raw mut idx, 0);
            }
            if o.is_null() && !(*ft).s.is_null() {
                o = options_parse_get((*(*ft).s).options, cstr_to_str(key), &raw mut idx, 0);
            }
            if o.is_null() {
                o = options_parse_get(GLOBAL_S_OPTIONS, cstr_to_str(key), &raw mut idx, 0);
            }
            if !o.is_null() {
                found = options_to_string(o, idx, 1);
                break 'found;
            }

            if let Some(fte) = format_table_get(key) {
                match (fte.cb)(ft) {
                    format_table_type::Time(tv) => t = tv.tv_sec,
                    format_table_type::String(string) => {
                        found = CString::new(string.into_owned()).unwrap().into_raw().cast();
                    }
                    format_table_type::None => found = null_mut(),
                }
                break 'found;
            }

            (*fe_find.as_mut_ptr()).key = key.cast_mut(); // TODO: check if this is correct casting away const
            let fe = rb_find(&raw mut (*ft).tree, fe_find.as_mut_ptr());
            if !fe.is_null() {
                if (*fe).time != 0 {
                    t = (*fe).time;
                    break 'found;
                }
                if let Some(cb) = (*fe).cb
                    && (*fe).value.is_null()
                {
                    (*fe).value = match cb(ft) {
                        format_table_type::None => CString::default().into_raw().cast(),
                        format_table_type::String(cow) => {
                            CString::new(cow.into_owned()).unwrap().into_raw().cast()
                        }
                        format_table_type::Time(_timeval) => unreachable!("unreachable?"),
                    };
                }
                found = xstrdup((*fe).value).as_ptr();
                break 'found;
            }

            if !modifiers.intersects(format_modifiers::FORMAT_TIMESTRING) {
                let mut envent = null_mut();
                if !(*ft).s.is_null() {
                    envent = environ_find((*(*ft).s).environ, key);
                }
                if envent.is_null() {
                    envent = environ_find(GLOBAL_ENVIRON, key);
                }
                if !envent.is_null() && (*envent).value.is_some() {
                    found = xstrdup((*envent).value.unwrap().as_ptr()).as_ptr();
                    break 'found;
                }
            }

            return null_mut();
        }
        // found
        if modifiers.intersects(format_modifiers::FORMAT_TIMESTRING) {
            if t == 0 && !found.is_null() {
                t = strtonum(found, 0, i64::MAX).unwrap_or_default();
                free_(found);
            }
            if t == 0 {
                return null_mut();
            }
            if modifiers.intersects(format_modifiers::FORMAT_PRETTY) {
                found = format_pretty_time(t, 0);
            } else {
                if !time_format.is_null() {
                    let mut tm = MaybeUninit::<tm>::uninit();
                    let tm = tm.as_mut_ptr();

                    localtime_r(&raw const t, tm);
                    strftime(s, SIZEOF_S, time_format, tm);
                } else {
                    ctime_r(&raw const t, s.cast());
                    *s.add(strcspn(s, c!("\n"))) = b'\0';
                }
                found = xstrdup(s).as_ptr();
            }
            return found;
        }

        if t != 0 {
            found = format_nul!("{t}");
        } else if found.is_null() {
            return null_mut();
        }
        let mut saved: *mut u8;
        if modifiers.intersects(format_modifiers::FORMAT_BASENAME) {
            saved = found;
            found = xstrdup__(basename(cstr_to_str(saved)));
            free_(saved);
        }
        if modifiers.intersects(format_modifiers::FORMAT_DIRNAME) {
            saved = found;
            found = xstrdup(libc::dirname(saved.cast()).cast()).as_ptr();
            free_(saved);
        }
        if modifiers.intersects(format_modifiers::FORMAT_QUOTE_SHELL) {
            saved = found;
            found = format_quote_shell(saved);
            free_(saved);
        }
        if modifiers.intersects(format_modifiers::FORMAT_QUOTE_STYLE) {
            saved = found;
            found = format_quote_style(saved);
            free_(saved);
        }
        found
    }
}

/// Unescape escaped characters.
pub unsafe fn format_unescape(mut s: *const u8) -> *mut u8 {
    unsafe {
        let mut cp = xmalloc(strlen(s) + 1).as_ptr().cast();
        let out = cp;
        let mut brackets = 0;
        while *s != b'\0' {
            if *s == b'#' && *s.add(1) == b'{' {
                brackets += 1;
            }
            if brackets == 0 && *s == b'#' && !strchr(c!(",#{}:"), *s.add(1) as i32).is_null() {
                s = s.add(1);
                *cp = *s;
                cp = cp.add(1);
                continue;
            }
            if *s == b'}' {
                brackets -= 1;
            }
            *cp = *s;
            cp = cp.add(1);
        }
        *cp = b'\0';
        out
    }
}

/// Remove escaped characters.
pub unsafe fn format_strip(mut s: *const u8) -> *mut u8 {
    unsafe {
        let out = xmalloc(strlen(s) + 1).as_ptr().cast();
        let mut cp = out;
        let mut brackets = 0;

        while *s != b'\0' {
            if *s == b'#' && *s.add(1) == b'{' {
                brackets += 1;
            }
            if *s == b'#' && !strchr(c!(",#{}:"), *s.add(1) as i32).is_null() {
                if brackets != 0 {
                    *cp = *s;
                    cp = cp.add(1);
                }
                s = s.add(1);
                continue;
            }
            if *s == b'}' {
                brackets -= 1;
            }
            *cp = *s;
            cp = cp.add(1);
            s = s.add(1);
        }
        *cp = b'\0';
        out
    }
}

/// Skip until end.
pub unsafe fn format_skip(mut s: *const u8, end: *const u8) -> *const u8 {
    unsafe {
        let mut brackets = 0;

        while *s != b'\0' {
            if *s == b'#' && *s.add(1) == b'{' {
                brackets += 1;
            }
            if *s == b'#' && *s.add(1) != b'\0' && !strchr(c!(",#{}:"), *s.add(1) as i32).is_null()
            {
                s = s.add(2);
                continue;
            }
            if *s == b'}' {
                brackets -= 1;
            }
            if !strchr(end, *s as i32).is_null() && brackets == 0 {
                break;
            }
            s = s.add(1);
        }
        if *s == b'\0' {
            return null_mut();
        }
        s
    }
}

/// Return left and right alternatives separated by commas.
pub unsafe fn format_choose(
    es: *mut format_expand_state,
    s: *const u8,
    left: *mut *mut u8,
    right: *mut *mut u8,
    expand: c_int,
) -> c_int {
    unsafe {
        let cp: *const u8 = format_skip(s, c!(","));
        if cp.is_null() {
            return -1;
        }
        let left0 = xstrndup(s, cp.offset_from(s) as usize).as_ptr();
        let right0 = xstrdup(cp.add(1)).as_ptr();

        if expand != 0 {
            *left = format_expand1(es, left0);
            free_(left0);
            *right = format_expand1(es, right0);
            free_(right0);
        } else {
            *left = left0;
            *right = right0;
        }
        0
    }
}

/// Is this true?
pub unsafe fn format_true(s: *const u8) -> bool {
    unsafe { !s.is_null() && *s != b'\0' && (*s != b'0' || *s.add(1) != b'\0') }
}

/// Check if modifier end.
pub fn format_is_end(c: u8) -> bool {
    c == b';' || c == b':'
}

/// Add to modifier list.
pub unsafe fn format_add_modifier(
    list: *mut *mut format_modifier,
    count: *mut u32,
    c: *const u8,
    n: usize,
    argv: *mut *mut u8,
    argc: i32,
) {
    unsafe {
        *list = xreallocarray_(*list, (*count) as usize + 1).as_ptr();
        let fm = (*list).add(*count as usize);
        (*count) += 1;

        memcpy((*fm).modifier.as_mut_ptr().cast(), c.cast(), n);
        (*fm).modifier[n] = b'\0';
        (*fm).size = n as u32;

        (*fm).argv = argv;
        (*fm).argc = argc;
    }
}

/// Free modifier list.
pub unsafe fn format_free_modifiers(list: *mut format_modifier, count: u32) {
    unsafe {
        for i in 0..count as usize {
            cmd_free_argv((*list.add(i)).argc, (*list.add(i)).argv);
        }
        free_(list);
    }
}

/// Build modifier list.
pub unsafe fn format_build_modifiers(
    es: *mut format_expand_state,
    s: *mut *const u8,
    count: *mut u32,
) -> *mut format_modifier {
    unsafe {
        let mut cp = *s;
        let mut end: *const u8;
        let mut list: *mut format_modifier = null_mut();

        let mut last: [u8; 4] = [b'X', b';', b':', b'\0'];
        let last: *mut u8 = last.as_mut_ptr();

        // char c, last[] = "X;:", **argv, *value;
        // int argc;

        // Modifiers are a ; separated list of the forms:
        //      l,m,C,a,b,c,d,n,t,w,q,E,T,S,W,P,<,>
        // 	=a
        // 	=/a
        //      =/a/
        // 	s/a/b/
        // 	s/a/b
        // 	||,&&,!=,==,<=,>=

        *count = 0;

        while *cp != b'\0' && *cp != b':' {
            // Skip any separator character.
            if *cp == b';' {
                cp = cp.add(1);
            }

            // Check single character modifiers with no arguments.
            if !strchr(c!("labcdnwETSWPL<>"), *cp as i32).is_null() && format_is_end(*cp.add(1)) {
                format_add_modifier(&raw mut list, count, cp, 1, null_mut(), 0);
                cp = cp.add(1);
                continue;
            }

            // Then try double character with no arguments.
            if (memcmp(c!("||").cast(), cp.cast(), 2) == 0
                || memcmp(c!("&&").cast(), cp.cast(), 2) == 0
                || memcmp(c!("!=").cast(), cp.cast(), 2) == 0
                || memcmp(c!("==").cast(), cp.cast(), 2) == 0
                || memcmp(c!("<=").cast(), cp.cast(), 2) == 0
                || memcmp(c!(">=").cast(), cp.cast(), 2) == 0)
                && format_is_end(*cp.add(2))
            {
                format_add_modifier(&raw mut list, count, cp, 2, null_mut(), 0);
                cp = cp.add(2);
                continue;
            }

            // Now try single character with arguments.
            if strchr(c!("mCNst=peq"), *cp as i32).is_null() {
                break;
            }
            let mut c = *cp;

            // No arguments provided.
            if format_is_end(*cp.add(1)) {
                format_add_modifier(&raw mut list, count, cp, 1, null_mut(), 0);
                cp = cp.add(1);
                continue;
            }
            let mut argv: *mut *mut u8 = null_mut();
            let mut argc = 0;

            // Single argument with no wrapper character.
            if ispunct(*cp.add(1) as i32) == 0 || *cp.add(1) == b'-' {
                end = format_skip(cp.add(1), c!(":;"));
                if end.is_null() {
                    break;
                }

                argv = xcalloc1();
                let value = xstrndup(cp.add(1), end.offset_from(cp.add(1)) as usize).as_ptr();
                *argv = format_expand1(es, value);
                free_(value);
                argc = 1;

                format_add_modifier(&raw mut list, count, &raw mut c, 1, argv, argc);
                cp = end;
                continue;
            }

            // Multiple arguments with a wrapper character.
            *last = *cp.add(1);
            cp = cp.add(1);
            loop {
                if *cp == *last && format_is_end(*cp.add(1)) {
                    cp = cp.add(1);
                    break;
                }
                end = format_skip(cp.add(1), last);
                if end.is_null() {
                    break;
                }
                cp = cp.add(1);

                argv = xreallocarray_(argv, argc as usize + 1).as_ptr();
                let value = xstrndup(cp, end.offset_from(cp) as usize).as_ptr();
                *argv.add(argc as usize) = format_expand1(es, value);
                argc += 1;
                free_(value);

                cp = end;
                if format_is_end(*cp) {
                    break;
                }
            }
            format_add_modifier(&raw mut list, count, &raw mut c, 1, argv, argc);
        }
        if *cp != b':' {
            format_free_modifiers(list, *count);
            *count = 0;
            return null_mut();
        }
        *s = cp.add(1);
        list
    }
}

pub unsafe fn format_match(
    fm: *mut format_modifier,
    pattern: *const u8,
    text: *const u8,
) -> *mut u8 {
    unsafe {
        let mut s = c!("");
        let mut r = MaybeUninit::<regex_t>::uninit();
        let r = r.as_mut_ptr();
        let mut flags: i32 = 0;

        if (*fm).argc >= 1 {
            s = *(*fm).argv;
        }
        if strchr(s, b'r' as i32).is_null() {
            if !strchr(s, b'i' as i32).is_null() {
                flags |= FNM_CASEFOLD;
            }
            if libc::fnmatch(pattern, text, flags) != 0 {
                return xstrdup(c!("0")).as_ptr();
            }
        } else {
            flags = REG_EXTENDED | REG_NOSUB;
            if !strchr(s, b'i' as i32).is_null() {
                flags |= REG_ICASE;
            }
            if regcomp(r, pattern, flags) != 0 {
                return xstrdup(c!("0")).as_ptr();
            }
            if regexec(r, text, 0, null_mut(), 0) != 0 {
                regfree(r);
                return xstrdup(c!("0")).as_ptr();
            }
            regfree(r);
        }
        xstrdup(c!("1")).as_ptr()
    }
}

pub unsafe fn format_sub(
    fm: *mut format_modifier,
    text: *const u8,
    pattern: *const u8,
    with: *const u8,
) -> *mut u8 {
    unsafe {
        let mut flags: i32 = REG_EXTENDED;

        if (*fm).argc >= 3 && !strchr(*(*fm).argv.add(2), b'i' as i32).is_null() {
            flags |= REG_ICASE;
        }
        let value = regsub(pattern, with, text, flags);
        if value.is_null() {
            xstrdup(text).as_ptr()
        } else {
            value
        }
    }
}

pub unsafe fn format_search(
    fm: *mut format_modifier,
    wp: *mut window_pane,
    s: *const u8,
) -> *mut u8 {
    unsafe {
        let mut ignore = 0;
        let mut regex = 0;

        if (*fm).argc >= 1 {
            if !strchr(*(*fm).argv, b'i' as i32).is_null() {
                ignore = 1;
            }
            if !strchr(*(*fm).argv, b'r' as i32).is_null() {
                regex = 1;
            }
        }
        format_nul!("{}", window_pane_search(wp, s, regex, ignore))
    }
}

pub unsafe fn format_session_name(es: *mut format_expand_state, fmt: *const u8) -> *mut u8 {
    unsafe {
        let name = format_expand1(es, fmt);

        for s in rb_foreach(&raw mut SESSIONS).map(NonNull::as_ptr) {
            if streq_(name, &(*s).name) {
                free_(name);
                return xstrdup(c!("1")).as_ptr();
            }
        }

        free_(name);
        xstrdup(c!("0")).as_ptr()
    }
}

pub unsafe fn format_loop_sessions(es: *mut format_expand_state, fmt: *const u8) -> *mut u8 {
    unsafe {
        let ft = (*es).ft;
        let c = (*ft).client;
        let item = (*ft).item;
        let mut value: *mut u8 = xcalloc(1, 1).as_ptr().cast();
        let mut valuelen = 1;

        for s in rb_foreach(&raw mut SESSIONS).map(NonNull::as_ptr) {
            format_log1!(es, c!("format_loop_sessions"), "session loop: ${}", (*s).id,);
            let nft = format_create(c, item, FORMAT_NONE, (*ft).flags);
            format_defaults(nft, (*ft).c, NonNull::new(s), None, None);
            let mut next = zeroed();
            format_copy_state(&mut next, es, format_expand_flags::empty());
            next.ft = nft;
            let expanded = format_expand1(&mut next, fmt);
            format_free(next.ft);

            valuelen += strlen(expanded);
            value = xrealloc(value.cast(), valuelen).as_ptr().cast();
            strlcat(value, expanded, valuelen);
            free_(expanded);
        }

        value
    }
}

pub unsafe fn format_window_name(es: *mut format_expand_state, fmt: *const u8) -> *mut u8 {
    unsafe {
        let ft = (*es).ft;
        if (*ft).s.is_null() {
            format_log1!(es, c!("format_window_name"), "window name but no session",);
            return null_mut();
        }

        let name = format_expand1(es, fmt);
        for wl in rb_foreach(&raw mut (*(*ft).s).windows).map(NonNull::as_ptr) {
            if strcmp((*(*wl).window).name, name) == 0 {
                free_(name);
                return xstrdup(c!("1")).as_ptr();
            }
        }
        free_(name);
        xstrdup(c!("0")).as_ptr()
    }
}

pub unsafe fn format_loop_windows(es: *mut format_expand_state, fmt: *const u8) -> *mut u8 {
    unsafe {
        let ft = (*es).ft;
        let c = (*ft).client;
        let item = (*ft).item;
        let mut all: *mut u8 = null_mut();
        let mut active: *mut u8 = null_mut();
        let mut value: *mut u8 = xcalloc(1, 1).as_ptr().cast();
        let mut valuelen = 1;

        if (*ft).s.is_null() {
            format_log1!(es, c!("format_loop_windows"), "window loop but no session",);
            return null_mut();
        }

        if format_choose(es, fmt, &mut all, &mut active, 0) != 0 {
            all = xstrdup(fmt).as_ptr();
            active = null_mut();
        }

        for wl in rb_foreach(&raw mut (*(*ft).s).windows).map(NonNull::as_ptr) {
            let w = (*wl).window;
            format_log1!(
                es,
                c!("format_loop_windows"),
                "window loop: {} @{}",
                (*wl).idx,
                (*w).id,
            );
            let use_ = if !active.is_null() && wl == (*(*ft).s).curw {
                active
            } else {
                all
            };

            let nft = format_create(c, item, FORMAT_WINDOW as i32 | (*w).id as i32, (*ft).flags);
            format_defaults(nft, (*ft).c, NonNull::new((*ft).s), NonNull::new(wl), None);
            let mut next = zeroed();
            format_copy_state(&raw mut next, es, format_expand_flags::empty());
            next.ft = nft;
            let expanded = format_expand1(&mut next, use_);
            format_free(nft);

            valuelen += strlen(expanded);
            value = xrealloc(value.cast(), valuelen).as_ptr().cast();
            strlcat(value, expanded, valuelen);
            free_(expanded);
        }

        free_(active);
        free_(all);
        value
    }
}

/// Loop over panes.
pub unsafe fn format_loop_panes(es: *mut format_expand_state, fmt: *const u8) -> *mut u8 {
    unsafe {
        let ft = (*es).ft;
        let c = (*ft).client;
        let item = (*ft).item;

        if (*ft).w.is_null() {
            format_log1!(es, c!("format_loop_panes"), "pane loop but no window");
            return null_mut();
        }

        let mut all: *mut u8 = null_mut();
        let mut active: *mut u8 = null_mut();
        if format_choose(es, fmt, &raw mut all, &raw mut active, 0) != 0 {
            all = xstrdup(fmt).as_ptr();
            active = null_mut();
        }

        let mut value: *mut u8 = xcalloc(1, 1).as_ptr().cast();
        let mut valuelen = 1;

        let mut next = MaybeUninit::<format_expand_state>::uninit();
        let next = next.as_mut_ptr();
        for wp in tailq_foreach::<_, discr_entry>(&raw mut (*(*ft).w).panes).map(NonNull::as_ptr) {
            format_log1!(es, c!("format_loop_panes"), "pane loop: %{}", (*wp).id,);
            let use_ = if !active.is_null() && wp == (*(*ft).w).active {
                active
            } else {
                all
            };
            let nft = format_create(c, item, FORMAT_PANE as i32 | (*wp).id as i32, (*ft).flags);
            format_defaults(
                nft,
                (*ft).c,
                NonNull::new((*ft).s),
                NonNull::new((*ft).wl),
                NonNull::new(wp),
            );
            format_copy_state(next, es, format_expand_flags::empty());
            (*next).ft = nft;
            let expanded = format_expand1(next, use_);
            format_free(nft);

            valuelen += strlen(expanded);
            value = xrealloc(value.cast(), valuelen).as_ptr().cast();

            strlcat(value, expanded, valuelen);
            free_(expanded);
        }

        free_(active);
        free_(all);

        value
    }
}

/// Loop over clients.
pub unsafe fn format_loop_clients(es: *mut format_expand_state, fmt: *const u8) -> *mut u8 {
    unsafe {
        let ft = (*es).ft;
        let item = (*ft).item;
        let mut next = MaybeUninit::<format_expand_state>::uninit();
        let next = next.as_mut_ptr();

        let mut value = xcalloc(1, 1).as_ptr();
        let mut valuelen = 1;

        for c in tailq_foreach(&raw mut CLIENTS).map(NonNull::as_ptr) {
            format_log1!(
                es,
                c!("format_loop_clients"),
                "client loop: {}",
                _s((*c).name),
            );
            let nft = format_create(c, item, 0, (*ft).flags);
            format_defaults(
                nft,
                c,
                NonNull::new((*ft).s),
                NonNull::new((*ft).wl),
                NonNull::new((*ft).wp),
            );
            format_copy_state(next, es, format_expand_flags::empty());
            (*next).ft = nft;
            let expanded = format_expand1(next, fmt);
            format_free(nft);

            valuelen += strlen(expanded);
            value = xrealloc(value.cast(), valuelen).as_ptr().cast();

            strlcat(value.cast(), expanded, valuelen);
            free_(expanded);
        }

        value.cast()
    }
}

pub unsafe fn format_replace_expression(
    mexp: *mut format_modifier,
    es: *mut format_expand_state,
    copy: *const u8,
) -> *mut u8 {
    unsafe {
        let argc = (*mexp).argc;

        let mut endch: *mut u8 = null_mut();
        let value: *mut u8;

        let mut left: *mut u8 = null_mut();
        let mut right: *mut u8 = null_mut();

        'fail: {
            let mut use_fp: i32 = 0;
            let mut prec: u32 = 0;

            let mut mleft: f64;
            let mut mright: f64;

            enum Operator {
                Add,
                Subtract,
                Multiply,
                Divide,
                Modulus,
                Equal,
                NotEqual,
                GreaterThan,
                GreaterThanEqual,
                LessThan,
                LessThanEqual,
            }

            let operator;

            if streq_(*(*mexp).argv, "+") {
                operator = Operator::Add;
            } else if streq_(*(*mexp).argv, "-") {
                operator = Operator::Subtract;
            } else if streq_(*(*mexp).argv, "*") {
                operator = Operator::Multiply;
            } else if streq_(*(*mexp).argv, "/") {
                operator = Operator::Divide;
            } else if streq_(*(*mexp).argv, "%") || streq_(*(*mexp).argv, "m") {
                operator = Operator::Modulus;
            } else if streq_(*(*mexp).argv, "==") {
                operator = Operator::Equal;
            } else if streq_(*(*mexp).argv, "!=") {
                operator = Operator::NotEqual;
            } else if streq_(*(*mexp).argv, ">") {
                operator = Operator::GreaterThan;
            } else if streq_(*(*mexp).argv, "<") {
                operator = Operator::LessThan;
            } else if streq_(*(*mexp).argv, ">=") {
                operator = Operator::GreaterThanEqual;
            } else if streq_(*(*mexp).argv, "<=") {
                operator = Operator::LessThanEqual;
            } else {
                format_log1!(
                    es,
                    c!("format_replace_expression"),
                    "expression has no valid operator: '{}'",
                    _s(*(*mexp).argv),
                );
                break 'fail;
            }

            // The second argument may be flags.
            if argc >= 2 && !strchr(*(*mexp).argv.add(1), b'f' as i32).is_null() {
                use_fp = 1;
                prec = 2;
            }

            // The third argument may be precision.
            if argc >= 3 {
                prec = match strtonum(*(*mexp).argv.add(2), i32::MIN, i32::MAX) {
                    Ok(value) => value as u32,
                    Err(errstr) => {
                        format_log1!(
                            es,
                            c!("format_replace_expression"),
                            "expression precision {}: {}",
                            errstr.to_string_lossy(),
                            _s(*(*mexp).argv.add(2)),
                        );
                        break 'fail;
                    }
                }
            }

            if format_choose(es, copy, &raw mut left, &raw mut right, 1) != 0 {
                format_log1!(
                    es,
                    c!("format_replace_expression"),
                    "expression syntax error"
                );
                break 'fail;
            }

            mleft = strtod(left, &raw mut endch);
            if *endch != b'\0' {
                format_log1!(
                    es,
                    c!("format_replace_expression"),
                    "expression left side is invalid: {}",
                    _s(left),
                );
                break 'fail;
            }

            mright = strtod(right, &raw mut endch);
            if *endch != b'\0' {
                format_log1!(
                    es,
                    c!("format_replace_expression"),
                    "expression right side is invalid: {}",
                    _s(right),
                );
                break 'fail;
            }

            if use_fp == 0 {
                mleft = (mleft as c_longlong) as f64;
                mright = (mright as c_longlong) as f64;
            }
            format_log1!(
                es,
                c!("format_replace_expression"),
                "expression left side is: {1:0$}",
                prec as usize,
                mleft,
            );
            format_log1!(
                es,
                c!("format_replace_expression"),
                "expression right side is: {1:0$}",
                prec as usize,
                mright,
            );

            let result = match operator {
                Operator::Add => mleft + mright,
                Operator::Subtract => mleft - mright,
                Operator::Multiply => mleft * mright,
                Operator::Divide => mleft / mright,
                Operator::Modulus => mleft % mright,
                Operator::Equal => ((mleft - mright).abs() < 1e-9) as i32 as f64,
                Operator::NotEqual => ((mleft - mright).abs() > 1e-9) as i32 as f64,
                Operator::GreaterThan => (mleft > mright) as i32 as f64,
                Operator::GreaterThanEqual => (mleft >= mright) as i32 as f64,
                Operator::LessThan => (mleft < mright) as i32 as f64,
                Operator::LessThanEqual => (mleft <= mright) as i32 as f64,
            };

            value = if use_fp != 0 {
                format_nul!("{:.*}", prec as usize, result)
            } else {
                format_nul!("{:.*}", prec as usize, (result as c_longlong) as f64)
            };
            format_log1!(
                es,
                c!("format_replace_expression"),
                "expression result is {}",
                _s(value),
            );

            free_(right);
            free_(left);
            return value;
        }

        // fail:
        free_(right);
        free_(left);
        null_mut()
    }
}

/// Replace a key.
pub unsafe fn format_replace(
    es: *mut format_expand_state,
    key: *const u8,
    keylen: usize,
    buf: *mut *mut u8,
    len: *mut usize,
    off: *mut usize,
) -> i32 {
    let __func__: *const u8 = c!("format_replace");

    unsafe {
        let ft = (*es).ft;
        let wp = (*ft).wp;
        let mut copy: *const u8;
        let cp: *const u8;
        let mut marker: *const u8 = null();

        let mut time_format: *const u8 = null();

        let copy0: *mut u8;
        let condition: *mut u8;
        let mut found: *mut u8;
        let mut new: *mut u8;
        let mut value: *mut u8 = null_mut();
        let mut left: *mut u8 = null_mut();
        let mut right: *mut u8 = null_mut();

        let valuelen;

        let mut modifiers: format_modifiers = format_modifiers::empty();
        let mut limit: i32 = 0;
        let mut width: i32 = 0;

        let mut c;

        let list: *mut format_modifier;
        let mut cmp: *mut format_modifier = null_mut();
        let mut search: *mut format_modifier = null_mut();

        let mut sub: *mut *mut format_modifier = null_mut();
        let mut mexp: *mut format_modifier = null_mut();

        // let mut i = 0u32;
        let mut count = 0u32;
        let mut nsub = 0u32;

        let mut next = MaybeUninit::<format_expand_state>::uninit();
        let next = next.as_mut_ptr();

        'fail: {
            'done: {
                // Make a copy of the key.
                copy0 = xstrndup(key, keylen).as_ptr();
                copy = copy0;

                // Process modifier list.
                list = format_build_modifiers(es, &raw mut copy, &raw mut count);
                for i in 0..count {
                    let fm = list.add(i as usize);
                    if format_logging(&*ft) {
                        format_log1!(
                            es,
                            __func__,
                            "modifier {} is {}",
                            i,
                            _s((&raw mut (*fm).modifier).cast::<u8>())
                        );
                        for j in 0..(*fm).argc {
                            format_log1!(
                                es,
                                __func__,
                                "modifier {} argument {}: {}",
                                i,
                                j,
                                _s(*(*fm).argv.add(j as usize)),
                            );
                        }
                    }
                    if (*fm).size == 1 {
                        match (*fm).modifier[0] {
                            b'm' | b'<' | b'>' => cmp = fm,
                            b'C' => search = fm,
                            b's' => {
                                if (*fm).argc < 2 {
                                } else {
                                    sub = xreallocarray_(sub, nsub as usize + 1).as_ptr();
                                    *sub.add(nsub as usize) = fm;
                                    nsub += 1;
                                }
                            }
                            b'=' => {
                                if (*fm).argc < 1 {
                                } else {
                                    limit = strtonum(*(*fm).argv, i32::MIN, i32::MAX)
                                        .unwrap_or_default();
                                    if (*fm).argc >= 2 && !(*(*fm).argv.add(1)).is_null() {
                                        marker = *(*fm).argv.add(1);
                                    }
                                }
                            }
                            b'p' => {
                                if (*fm).argc < 1 {
                                    break;
                                } else {
                                    width = strtonum(*(*fm).argv, i32::MIN, i32::MAX)
                                        .unwrap_or_default();
                                }
                            }
                            b'w' => modifiers |= format_modifiers::FORMAT_WIDTH,
                            b'e' => {
                                if (*fm).argc < 1 || (*fm).argc > 3 {
                                } else {
                                    mexp = fm;
                                }
                            }
                            b'l' => modifiers |= format_modifiers::FORMAT_LITERAL,
                            b'a' => modifiers |= format_modifiers::FORMAT_CHARACTER,
                            b'b' => modifiers |= format_modifiers::FORMAT_BASENAME,
                            b'c' => modifiers |= format_modifiers::FORMAT_COLOUR,
                            b'd' => modifiers |= format_modifiers::FORMAT_DIRNAME,
                            b'n' => modifiers |= format_modifiers::FORMAT_LENGTH,
                            b't' => {
                                modifiers |= format_modifiers::FORMAT_TIMESTRING;
                                if (*fm).argc >= 1 {
                                    if !strchr(*(*fm).argv, b'p' as i32).is_null() {
                                        modifiers |= format_modifiers::FORMAT_PRETTY;
                                    } else if (*fm).argc >= 2
                                        && !strchr(*(*fm).argv, b'f' as i32).is_null()
                                    {
                                        time_format = format_strip(*(*fm).argv.add(1));
                                    }
                                }
                            }
                            b'q' => {
                                if (*fm).argc < 1 {
                                    modifiers |= format_modifiers::FORMAT_QUOTE_SHELL;
                                } else if !strchr(*(*fm).argv, b'e' as i32).is_null()
                                    || !strchr(*(*fm).argv, b'h' as i32).is_null()
                                {
                                    modifiers |= format_modifiers::FORMAT_QUOTE_STYLE;
                                }
                            }
                            b'E' => modifiers |= format_modifiers::FORMAT_EXPAND,
                            b'T' => modifiers |= format_modifiers::FORMAT_EXPANDTIME,
                            b'N' => {
                                if (*fm).argc < 1 || !strchr(*(*fm).argv, b'w' as i32).is_null() {
                                    modifiers |= format_modifiers::FORMAT_WINDOW_NAME;
                                } else if !strchr(*(*fm).argv, b's' as i32).is_null() {
                                    modifiers |= format_modifiers::FORMAT_SESSION_NAME;
                                }
                            }
                            b'S' => modifiers |= format_modifiers::FORMAT_SESSIONS,
                            b'W' => modifiers |= format_modifiers::FORMAT_WINDOWS,
                            b'P' => modifiers |= format_modifiers::FORMAT_PANES,
                            b'L' => modifiers |= format_modifiers::FORMAT_CLIENTS,
                            _ => (),
                        }
                    } else if (*fm).size == 2
                        && (streq_((*fm).modifier.as_ptr(), "||")
                            || streq_((*fm).modifier.as_ptr(), "&&")
                            || streq_((*fm).modifier.as_ptr(), "==")
                            || streq_((*fm).modifier.as_ptr(), "!=")
                            || streq_((*fm).modifier.as_ptr(), ">=")
                            || streq_((*fm).modifier.as_ptr(), "<="))
                    {
                        cmp = fm;
                    }
                }

                // Is this a literal string?
                if modifiers.intersects(format_modifiers::FORMAT_LITERAL) {
                    format_log1!(es, __func__, "literal string is '{}'", _s(copy));
                    value = format_unescape(copy);
                    break 'done;
                }

                // Is this a character?
                if modifiers.intersects(format_modifiers::FORMAT_CHARACTER) {
                    new = format_expand1(es, copy);
                    value = match strtonum::<u8>(new, 32, 126) {
                        Ok(n) => format_nul!("{}", n as char),
                        Err(_) => xstrdup(c!("")).as_ptr(),
                    };
                    free_(new);
                    break 'done;
                }

                // Is this a colour?
                if modifiers.intersects(format_modifiers::FORMAT_COLOUR) {
                    new = format_expand1(es, copy);
                    c = colour_fromstring(cstr_to_str(new));
                    value = if c == -1
                        || ({
                            c = colour_force_rgb(c);
                            c == -1
                        }) {
                        xstrdup(c!("")).as_ptr()
                    } else {
                        format_nul!("{:06x}", c & 0xffffff)
                    };
                    free_(new);
                    break 'done;
                }

                // Is this a loop, comparison or condition?
                if modifiers.intersects(format_modifiers::FORMAT_SESSIONS) {
                    value = format_loop_sessions(es, copy);
                    if value.is_null() {
                        break 'fail;
                    }
                } else if modifiers.intersects(format_modifiers::FORMAT_WINDOWS) {
                    value = format_loop_windows(es, copy);
                    if value.is_null() {
                        break 'fail;
                    }
                } else if modifiers.intersects(format_modifiers::FORMAT_PANES) {
                    value = format_loop_panes(es, copy);
                    if value.is_null() {
                        break 'fail;
                    }
                } else if modifiers.intersects(format_modifiers::FORMAT_CLIENTS) {
                    value = format_loop_clients(es, copy);
                    if value.is_null() {
                        break 'fail;
                    }
                } else if modifiers.intersects(format_modifiers::FORMAT_WINDOW_NAME) {
                    value = format_window_name(es, copy);
                    if value.is_null() {
                        break 'fail;
                    }
                } else if modifiers.intersects(format_modifiers::FORMAT_SESSION_NAME) {
                    value = format_session_name(es, copy);
                    if value.is_null() {
                        break 'fail;
                    }
                } else if !search.is_null() {
                    // Search in pane.
                    new = format_expand1(es, copy);
                    if wp.is_null() {
                        format_log1!(es, __func__, "search '{}' but no pane", _s(new));
                        value = xstrdup(c!("0")).as_ptr();
                    } else {
                        format_log1!(es, __func__, "search '{}' pane %{}", _s(new), (*wp).id,);
                        value = format_search(search, wp, new);
                    }
                    free_(new);
                } else if !cmp.is_null() {
                    // Comparison of left and right.
                    if format_choose(es, copy, &raw mut left, &raw mut right, 1) != 0 {
                        format_log1!(
                            es,
                            __func__,
                            "compare {} syntax error: {}",
                            _s((&raw const (*cmp).modifier).cast::<u8>()),
                            _s(copy),
                        );
                        break 'fail;
                    }
                    format_log1!(
                        es,
                        __func__,
                        "compare {} left is: {}",
                        _s((&raw const (*cmp).modifier).cast::<u8>()),
                        _s(left),
                    );
                    format_log1!(
                        es,
                        __func__,
                        "compare {} right is: {}",
                        _s((&raw const (*cmp).modifier).cast::<u8>()),
                        _s(right),
                    );

                    if streq_((*cmp).modifier.as_ptr(), "||") {
                        if format_true(left) || format_true(right) {
                            value = xstrdup(c!("1")).as_ptr();
                        } else {
                            value = xstrdup(c!("0")).as_ptr();
                        }
                    } else if streq_((*cmp).modifier.as_ptr(), "&&") {
                        if format_true(left) && format_true(right) {
                            value = xstrdup(c!("1")).as_ptr();
                        } else {
                            value = xstrdup(c!("0")).as_ptr();
                        }
                    } else if streq_((*cmp).modifier.as_ptr(), "==") {
                        if strcmp(left, right) == 0 {
                            value = xstrdup(c!("1")).as_ptr();
                        } else {
                            value = xstrdup(c!("0")).as_ptr();
                        }
                    } else if streq_((*cmp).modifier.as_ptr(), "!=") {
                        if strcmp(left, right) != 0 {
                            value = xstrdup(c!("1")).as_ptr();
                        } else {
                            value = xstrdup(c!("0")).as_ptr();
                        }
                    } else if streq_((*cmp).modifier.as_ptr(), "<") {
                        if strcmp(left, right) < 0 {
                            value = xstrdup(c!("1")).as_ptr();
                        } else {
                            value = xstrdup(c!("0")).as_ptr();
                        }
                    } else if streq_((*cmp).modifier.as_ptr(), ">") {
                        if strcmp(left, right) > 0 {
                            value = xstrdup(c!("1")).as_ptr();
                        } else {
                            value = xstrdup(c!("0")).as_ptr();
                        }
                    } else if streq_((*cmp).modifier.as_ptr(), "<=") {
                        if strcmp(left, right) <= 0 {
                            value = xstrdup(c!("1")).as_ptr();
                        } else {
                            value = xstrdup(c!("0")).as_ptr();
                        }
                    } else if streq_((*cmp).modifier.as_ptr(), ">=") {
                        if strcmp(left, right) >= 0 {
                            value = xstrdup(c!("1")).as_ptr();
                        } else {
                            value = xstrdup(c!("0")).as_ptr();
                        }
                    } else if streq_((*cmp).modifier.as_ptr(), "m") {
                        value = format_match(cmp, left, right);
                    }

                    free_(right);
                    free_(left);
                } else if *copy == b'?' {
                    // Conditional: check first and choose second or third.
                    cp = format_skip(copy.add(1), c!(","));
                    if cp.is_null() {
                        format_log1!(es, __func__, "condition syntax error: {}", _s(copy.add(1)),);
                        break 'fail;
                    }
                    condition =
                        xstrndup(copy.add(1), cp.offset_from(copy.add(1)) as usize).as_ptr();
                    format_log1!(es, __func__, "condition is: {}", _s(condition));

                    found = format_find(ft, condition, modifiers, time_format);
                    if found.is_null() {
                        // If the condition not found, try to expand it. If
                        // the expansion doesn't have any effect, then assume
                        // false.
                        found = format_expand1(es, condition);
                        if strcmp(found, condition) == 0 {
                            free_(found);
                            found = xstrdup(c!("")).as_ptr();
                            format_log1!(
                                es,
                                __func__,
                                "condition '{}' not found; assuming false",
                                _s(condition),
                            );
                        }
                    } else {
                        format_log1!(
                            es,
                            __func__,
                            "condition '{}' found: {}",
                            _s(condition),
                            _s(found),
                        );
                    }

                    if format_choose(es, cp.add(1), &raw mut left, &raw mut right, 0) != 0 {
                        format_log1!(
                            es,
                            __func__,
                            "condition '{}' syntax error: {}",
                            _s(condition),
                            _s(cp.add(1)),
                        );
                        free_(found);
                        break 'fail;
                    }
                    if format_true(found) {
                        format_log1!(es, __func__, "condition '{}' is true", _s(condition));
                        value = format_expand1(es, left);
                    } else {
                        format_log1!(es, __func__, "condition '{}' is false", _s(condition));
                        value = format_expand1(es, right);
                    }
                    free_(right);
                    free_(left);

                    free_(condition);
                    free_(found);
                } else if !mexp.is_null() {
                    value = format_replace_expression(mexp, es, copy);
                    if value.is_null() {
                        value = xstrdup(c!("")).as_ptr();
                    }
                } else if !strstr(copy, c!("#{")).is_null() {
                    format_log1!(es, __func__, "expanding inner format '{}'", _s(copy));
                    value = format_expand1(es, copy);
                } else {
                    value = format_find(ft, copy, modifiers, time_format);
                    if value.is_null() {
                        format_log1!(es, __func__, "format '{}' not found", _s(copy));
                        value = xstrdup(c!("")).as_ptr();
                    } else {
                        format_log1!(es, __func__, "format '{}' found: {}", _s(copy), _s(value),);
                    }
                }
            }
            // done:

            // Expand again if required.
            if modifiers.intersects(format_modifiers::FORMAT_EXPAND) {
                new = format_expand1(es, value);
                free_(value);
                value = new;
            } else if modifiers.intersects(format_modifiers::FORMAT_EXPANDTIME) {
                format_copy_state(next, es, format_expand_flags::FORMAT_EXPAND_TIME);
                new = format_expand1(next, value);
                free_(value);
                value = new;
            }

            // Perform substitution if any.
            for i in 0..nsub {
                left = format_expand1(es, *(**sub.add(i as usize)).argv);
                right = format_expand1(es, *(**sub.add(i as usize)).argv.add(1));
                new = format_sub(*sub.add(i as usize), value, left, right);
                format_log1!(
                    es,
                    __func__,
                    "substitute '{}' to '{}': {}",
                    _s(left),
                    _s(right),
                    _s(new),
                );
                free_(value);
                value = new;
                free_(right);
                free_(left);
            }

            // Truncate the value if needed.
            if limit > 0 {
                new = format_trim_left(value, limit as u32);
                value = if !marker.is_null() && strcmp(new, value) != 0 {
                    free_(value);
                    format_nul!("{}{}", _s(new), _s(marker))
                } else {
                    free_(value);
                    new
                };
                format_log1!(
                    es,
                    __func__,
                    "applied length limit {}: {}",
                    limit,
                    _s(value),
                );
            } else if limit < 0 {
                new = format_trim_right(value, (-limit) as u32);
                value = if !marker.is_null() && strcmp(new, value) != 0 {
                    free_(value);
                    format_nul!("{}{}", _s(marker), _s(new))
                } else {
                    free_(value);
                    new
                };
                format_log1!(
                    es,
                    __func__,
                    "applied length limit {}: {}",
                    limit,
                    _s(value),
                );
            }

            // Pad the value if needed.
            if width > 0 {
                new = utf8_padcstr(value, width as u32);
                free_(value);
                value = new;
                format_log1!(
                    es,
                    __func__,
                    "applied padding width {}: {}",
                    width,
                    _s(value),
                );
            } else if width < 0 {
                new = utf8_rpadcstr(value, (-width) as u32);
                free_(value);
                value = new;
                format_log1!(
                    es,
                    __func__,
                    "applied padding width {}: {}",
                    width,
                    _s(value),
                );
            }

            // Replace with the length or width if needed.
            if modifiers.intersects(format_modifiers::FORMAT_LENGTH) {
                new = format_nul!("{}", strlen(value));
                free_(value);
                value = new;
                format_log1!(es, __func__, "replacing with length: {}", _s(new));
            }
            if modifiers.intersects(format_modifiers::FORMAT_WIDTH) {
                new = format_nul!("{}", format_width(value));
                free_(value);
                value = new;
                format_log1!(es, __func__, "replacing with width: {}", _s(new));
            }

            // Expand the buffer and copy in the value.
            valuelen = strlen(value);
            while *len - *off < valuelen + 1 {
                *buf = xreallocarray((*buf).cast(), 2, *len).as_ptr().cast();
                *len *= 2;
            }
            memcpy((*buf).add(*off).cast(), value.cast(), valuelen);
            *off += valuelen;

            format_log1!(
                es,
                __func__,
                "replaced '{}' with '{}'",
                _s(copy0),
                _s(value),
            );
            free_(value);

            free_(sub);
            format_free_modifiers(list, count);
            free_(copy0);
            return 0;
        }

        // fail:
        format_log1!(es, __func__, "failed {}", _s(copy0));

        free_(sub);
        format_free_modifiers(list, count);
        free_(copy0);
        -1
    }
}

/// Expand keys in a template.
pub unsafe fn format_expand1(es: *mut format_expand_state, mut fmt: *const u8) -> *mut u8 {
    unsafe {
        let ft = (*es).ft;
        let mut out: *mut u8;

        let mut s: *const u8;
        let mut style_end: *const u8 = null();

        const SIZEOF_EXPANDED: usize = 8192;
        let mut expanded = MaybeUninit::<[u8; SIZEOF_EXPANDED]>::uninit();
        let expanded = expanded.as_mut_ptr() as *mut u8;

        if fmt.is_null() || *fmt == b'\0' {
            return xstrdup(c!("")).as_ptr();
        }

        if (*es).loop_ == FORMAT_LOOP_LIMIT as u32 {
            format_log1!(
                es,
                c!("format_expand1"),
                "reached loop limit ({})",
                FORMAT_LOOP_LIMIT,
            );
            return xstrdup(c!("")).as_ptr();
        }
        (*es).loop_ += 1;

        format_log1!(es, c!("format_expand1"), "expanding format: {}", _s(fmt),);

        if ((*es)
            .flags
            .intersects(format_expand_flags::FORMAT_EXPAND_TIME))
            && !strchr(fmt, b'%' as i32).is_null()
        {
            if (*es).time == 0 {
                (*es).time = libc::time(null_mut());
                localtime_r(&raw mut (*es).time, &raw mut (*es).tm);
            }
            if strftime(expanded, SIZEOF_EXPANDED, fmt, &raw mut (*es).tm) == 0 {
                format_log1!(es, c!("format_expand1"), "format is too long",);
                return xstrdup(c!("")).as_ptr();
            }
            if format_logging(&*ft) && strcmp(expanded, fmt) != 0 {
                format_log1!(
                    es,
                    c!("format_expand1"),
                    "after time expanded: {}",
                    _s(expanded),
                );
            }
            fmt = expanded;
        }

        let mut len = 64;
        let mut buf: *mut u8 = xmalloc(len).as_ptr().cast();
        let mut off = 0;
        let mut n;

        while *fmt != b'\0' {
            if *fmt != b'#' {
                while len - off < 2 {
                    buf = xreallocarray(buf.cast(), 2, len).as_ptr().cast();
                    len *= 2;
                }
                *buf.add(off) = *fmt;
                off += 1;
                fmt = fmt.add(1);
                continue;
            }
            fmt = fmt.add(1);

            let ch: u8 = *fmt;
            fmt = fmt.add(1);
            let mut brackets;

            let mut ptr: *const u8;
            match ch {
                b'(' => {
                    brackets = 1;
                    ptr = fmt;
                    while *ptr != b'\0' {
                        if *ptr == b'(' {
                            brackets += 1;
                        }
                        if *ptr == b')'
                            && ({
                                brackets -= 1;
                                brackets == 0
                            })
                        {
                            break;
                        }
                        ptr = ptr.add(1);
                    }
                    if *ptr != b')' || brackets != 0 {
                        break;
                    }
                    n = ptr.offset_from(fmt) as usize;

                    let name = xstrndup(fmt, n).as_ptr();
                    format_log1!(es, c!("format_expand1"), "found #(): {}", _s(name),);

                    if ((*ft).flags.intersects(format_flags::FORMAT_NOJOBS))
                        || ((*es)
                            .flags
                            .intersects(format_expand_flags::FORMAT_EXPAND_NOJOBS))
                    {
                        out = xstrdup(c!("")).as_ptr();
                        format_log1!(es, c!("format_expand1"), "#() is disabled");
                    } else {
                        out = format_job_get(es, name);
                        format_log1!(es, c!("format_expand1"), "#() result: {}", _s(out),);
                    }
                    free_(name);

                    let outlen = strlen(out);
                    while len - off < outlen + 1 {
                        buf = xreallocarray(buf.cast(), 2, len).as_ptr().cast();
                        len *= 2;
                    }
                    memcpy(buf.add(off).cast(), out.cast(), outlen);
                    off += outlen;

                    free_(out);

                    fmt = fmt.add(n + 1);
                    continue;
                }
                b'{' => {
                    ptr = format_skip(fmt.offset(-2), c!("}"));
                    if ptr.is_null() {
                        break;
                    }
                    n = ptr.offset_from(fmt) as usize;

                    format_log1!(es, c!("format_expand1"), "found #{}: {1:0$}", n, _s(fmt),);
                    if format_replace(es, fmt, n, &raw mut buf, &raw mut len, &raw mut off) != 0 {
                        break;
                    }
                    fmt = fmt.add(n + 1);
                    continue;
                }
                b'[' | b'#' => {
                    // If ##[ (with two or more #s), then it is a style and
                    // can be left for format_draw to handle.
                    ptr = fmt.sub((ch == b'[') as usize);
                    n = 2 - (ch == b'[') as usize;
                    while *ptr == b'#' {
                        ptr = ptr.add(1);
                        n += 1;
                    }
                    if *ptr == b'[' {
                        style_end = format_skip(fmt.offset(-2), c!("]"));
                        format_log1!(es, c!("format_expand1"), "found #*{}[", n);
                        while len - off < n + 2 {
                            buf = xreallocarray(buf.cast(), 2, len).as_ptr().cast();
                            len *= 2;
                        }
                        memcpy(buf.add(off).cast(), fmt.offset(-2).cast(), n + 1);
                        off += n + 1;
                        fmt = ptr.add(1);
                        continue;
                    }
                    // FALLTHROUGH
                    format_log1!(es, c!("format_expand1"), "found #{}", ch as char);
                    while len - off < 2 {
                        buf = xreallocarray(buf.cast(), 2, len).as_ptr().cast();
                        len *= 2;
                    }
                    *buf.add(off) = ch;
                    off += 1;
                    continue;
                }
                // FALLTHROUGH
                b'}' | b',' => {
                    format_log1!(es, c!("format_expand1"), "found #{}", ch as char,);
                    while len - off < 2 {
                        buf = xreallocarray(buf.cast(), 2, len).as_ptr().cast();
                        len *= 2;
                    }
                    *buf.add(off) = ch;
                    off += 1;
                    continue;
                }
                _ => {
                    s = null_mut();
                    if fmt > style_end {
                        if ch.is_ascii_uppercase() {
                            s = FORMAT_UPPER[(ch - b'A') as usize].as_ptr();
                        } else if ch.is_ascii_lowercase() {
                            s = FORMAT_LOWER[(ch - b'a') as usize].as_ptr();
                        }
                    } /* skip inside #[] */
                    if s.is_null() {
                        while len - off < 3 {
                            buf = xreallocarray(buf.cast(), 2, len).as_ptr().cast();
                            len *= 2;
                        }
                        *buf.add(off) = b'#';
                        off += 1;
                        *buf.add(off) = ch;
                        off += 1;

                        continue;
                    }
                    n = strlen(s);
                    format_log1!(es, c!("format_expand1"), "found #{}: {}", ch as char, _s(s),);
                    if format_replace(es, s, n, &raw mut buf, &raw mut len, &raw mut off) != 0 {
                        break;
                    }
                    continue;
                }
            }

            #[expect(unreachable_code)]
            {
                break;
            }
        }
        *buf.add(off) = b'\0';

        format_log1!(es, c!("format_expand1"), "result is: {}", _s(buf),);
        (*es).loop_ -= 1;

        buf
    }
}

/// Expand keys in a template, passing through strftime first.
pub unsafe fn format_expand_time(ft: *mut format_tree, fmt: *const u8) -> *mut u8 {
    unsafe {
        let mut es = MaybeUninit::<format_expand_state>::uninit();
        let es = es.as_mut_ptr();

        memset0(es);
        (*es).ft = ft;
        (*es).flags = format_expand_flags::FORMAT_EXPAND_TIME;
        format_expand1(es, fmt)
    }
}

/// Expand keys in a template.
pub unsafe fn format_expand(ft: *mut format_tree, fmt: *const u8) -> *mut u8 {
    unsafe {
        let mut es = MaybeUninit::<format_expand_state>::uninit();
        let es = es.as_mut_ptr();

        memset0(es);
        (*es).ft = ft;
        (*es).flags = format_expand_flags::empty();
        format_expand1(es, fmt)
    }
}

/// Expand a single string.
pub unsafe fn format_single(
    item: *mut cmdq_item,
    fmt: *const u8,
    c: *mut client,
    s: *mut session,
    wl: *mut winlink,
    wp: *mut window_pane,
) -> *mut u8 {
    unsafe {
        let ft = format_create_defaults(item, c, s, wl, wp);
        let expanded: *mut u8 = format_expand(ft, fmt);
        format_free(ft);
        expanded
    }
}

/// Expand a single string using state.
pub unsafe fn format_single_from_state(
    item: *mut cmdq_item,
    fmt: *const u8,
    c: *mut client,
    fs: *mut cmd_find_state,
) -> *mut u8 {
    unsafe { format_single(item, fmt, c, (*fs).s, (*fs).wl, (*fs).wp) }
}

/// Expand a single string using target.
pub unsafe fn format_single_from_target(item: *mut cmdq_item, fmt: *const u8) -> *mut u8 {
    unsafe {
        let tc = cmdq_get_target_client(item);

        format_single_from_state(item, fmt, tc, cmdq_get_target(item))
    }
}

/// Create and add defaults.
pub unsafe fn format_create_defaults(
    item: *mut cmdq_item,
    c: *mut client,
    s: *mut session,
    wl: *mut winlink,
    wp: *mut window_pane,
) -> *mut format_tree {
    unsafe {
        let ft = if !item.is_null() {
            format_create(
                cmdq_get_client(item),
                item,
                FORMAT_NONE,
                format_flags::empty(),
            )
        } else {
            format_create(null_mut(), item, FORMAT_NONE, format_flags::empty())
        };
        format_defaults(ft, c, NonNull::new(s), NonNull::new(wl), NonNull::new(wp));
        ft
    }
}

/// Create and add defaults using state.
pub unsafe fn format_create_from_state(
    item: *mut cmdq_item,
    c: *mut client,
    fs: *mut cmd_find_state,
) -> *mut format_tree {
    unsafe { format_create_defaults(item, c, (*fs).s, (*fs).wl, (*fs).wp) }
}

/// Create and add defaults using target.
pub unsafe fn format_create_from_target(item: *mut cmdq_item) -> *mut format_tree {
    unsafe {
        let tc = cmdq_get_target_client(item);

        format_create_from_state(item, tc, cmdq_get_target(item))
    }
}

/// Set defaults for any of arguments that are not NULL.
pub unsafe fn format_defaults(
    ft: *mut format_tree,
    c: *mut client,
    s: Option<NonNull<session>>,
    wl: Option<NonNull<winlink>>,
    wp: Option<NonNull<window_pane>>,
) {
    unsafe {
        let mut s = transmute_ptr(s);
        let mut wl = transmute_ptr(wl);
        let mut wp = transmute_ptr(wp);

        if !c.is_null() && !(*c).name.is_null() {
            log_debug!("{}: c={}", function_name!(), _s((*c).name));
        } else {
            log_debug!("{}: c=none", function_name!());
        }
        if !s.is_null() {
            log_debug!("{}: s=${}", function_name!(), (*s).id);
        } else {
            log_debug!("{}: s=none", function_name!());
        }
        if !wl.is_null() {
            log_debug!("{}: wl={}", function_name!(), (*wl).idx);
        } else {
            log_debug!("{}: wl=none", function_name!());
        }
        if !wp.is_null() {
            log_debug!("{}: wp=%%{}", function_name!(), (*wp).id);
        } else {
            log_debug!("{}: wp=none", function_name!());
        }

        if !c.is_null() && !s.is_null() && (*c).session != s {
            log_debug!("{}: session does not match", function_name!());
        }

        (*ft).type_ = if !wp.is_null() {
            format_type::FORMAT_TYPE_PANE
        } else if !wl.is_null() {
            format_type::FORMAT_TYPE_WINDOW
        } else if !s.is_null() {
            format_type::FORMAT_TYPE_SESSION
        } else {
            format_type::FORMAT_TYPE_UNKNOWN
        };

        if s.is_null() && !c.is_null() {
            s = (*c).session;
        }
        if wl.is_null() && !s.is_null() {
            wl = (*s).curw;
        }
        if wp.is_null() && !wl.is_null() {
            wp = (*(*wl).window).active;
        }

        if !c.is_null() {
            format_defaults_client(ft, c);
        }
        if !s.is_null() {
            format_defaults_session(ft, s);
        }
        if !wl.is_null() {
            format_defaults_winlink(ft, wl);
        }
        if !wp.is_null() {
            format_defaults_pane(ft, wp);
        }

        let pb = paste_get_top(null_mut());
        if !pb.is_null() {
            format_defaults_paste_buffer(ft, pb);
        }
    }
}

/// Set default format keys for a session.
pub unsafe fn format_defaults_session(ft: *mut format_tree, s: *mut session) {
    unsafe {
        (*ft).s = s;
    }
}

/// Set default format keys for a client.
pub unsafe fn format_defaults_client(ft: *mut format_tree, c: *mut client) {
    unsafe {
        if (*ft).s.is_null() {
            (*ft).s = (*c).session;
        }
        (*ft).c = c;
    }
}

/// Set default format keys for a window.
pub unsafe fn format_defaults_window(ft: *mut format_tree, w: *mut window) {
    unsafe {
        (*ft).w = w;
    }
}

/// Set default format keys for a winlink.
pub unsafe fn format_defaults_winlink(ft: *mut format_tree, wl: *mut winlink) {
    unsafe {
        if (*ft).w.is_null() {
            format_defaults_window(ft, (*wl).window);
        }
        (*ft).wl = wl;
    }
}

/// Set default format keys for a window pane.
pub unsafe fn format_defaults_pane(ft: *mut format_tree, wp: *mut window_pane) {
    unsafe {
        if (*ft).w.is_null() {
            format_defaults_window(ft, (*wp).window);
        }
        (*ft).wp = wp;

        if let Some(wme) = NonNull::new(tailq_first(&raw mut (*wp).modes))
            && let Some(formats) = (*(*wme.as_ptr()).mode).formats
        {
            formats(wme.as_ptr(), ft);
        }
    }
}

/// Set default format keys for paste buffer.
pub unsafe fn format_defaults_paste_buffer(ft: *mut format_tree, pb: *mut paste_buffer) {
    unsafe {
        (*ft).pb = pb;
    }
}

/// Return word at given coordinates. Caller frees.
pub unsafe fn format_grid_word(gd: *mut grid, mut x: u32, mut y: u32) -> String {
    unsafe {
        let mut ud: Vec<utf8_data> = Vec::new();
        let mut gc = MaybeUninit::<grid_cell>::uninit();
        let gc = gc.as_mut_ptr();
        let mut found = false;

        let ws: *const u8 = options_get_string_(GLOBAL_S_OPTIONS, "word-separators");

        loop {
            grid_get_cell(gd, x, y, gc);
            if (*gc).flags.intersects(grid_flag::PADDING) {
                break;
            }
            if utf8_cstrhas(ws, &raw const (*gc).data)
                || ((*gc).data.size == 1 && (*gc).data.data[0] == b' ')
            {
                found = true;
                break;
            }

            if x == 0 {
                if y == 0 {
                    break;
                }
                let gl = grid_peek_line(gd, y - 1);
                if !(*gl).flags.intersects(grid_line_flag::WRAPPED) {
                    break;
                }
                y -= 1;
                x = grid_line_length(gd, y);
                if x == 0 {
                    break;
                }
            }
            x -= 1;
        }
        loop {
            if found {
                let end = grid_line_length(gd, y);
                if end == 0 || x == end - 1 {
                    if y == (*gd).hsize + (*gd).sy - 1 {
                        break;
                    }
                    let gl = grid_peek_line(gd, y);
                    if !(*gl).flags.intersects(grid_line_flag::WRAPPED) {
                        break;
                    }
                    y += 1;
                    x = 0;
                } else {
                    x += 1;
                }
            }
            found = true;

            grid_get_cell(gd, x, y, gc);
            if (*gc).flags.intersects(grid_flag::PADDING) {
                break;
            }
            if utf8_cstrhas(ws, &raw mut (*gc).data)
                || ((*gc).data.size == 1 && (*gc).data.data[0] == b' ')
            {
                break;
            }

            ud.push((*gc).data);
        }

        utf8_to_string(&ud)
    }
}

/// Return line at given coordinates. Caller frees.
pub unsafe fn format_grid_line(gd: *mut grid, y: u32) -> String {
    unsafe {
        let mut ud: Vec<utf8_data> = Vec::new();
        let mut gc = MaybeUninit::<grid_cell>::uninit();
        let gc = gc.as_mut_ptr();
        for x in 0..grid_line_length(gd, y) {
            grid_get_cell(gd, x, y, gc);
            if (*gc).flags.intersects(grid_flag::PADDING) {
                break;
            }

            ud.push((*gc).data);
        }
        utf8_to_string(&ud)
    }
}

/// Return hyperlink at given coordinates. Caller frees.
pub unsafe fn format_grid_hyperlink(
    gd: *mut grid,
    x: u32,
    y: u32,
    s: *mut screen,
) -> Option<String> {
    unsafe {
        let mut uri: *const u8 = null();
        let mut gc = MaybeUninit::<grid_cell>::uninit();
        let gc = gc.as_mut_ptr();

        grid_get_cell(gd, x, y, gc);
        if (*gc).flags.intersects(grid_flag::PADDING) {
            return None;
        }
        if (*s).hyperlinks.is_null() || (*gc).link == 0 {
            return None;
        }
        if !hyperlinks_get(
            (*s).hyperlinks,
            (*gc).link,
            &mut uri,
            null_mut(),
            null_mut(),
        ) {
            return None;
        }
        Some(cstr_to_str(uri).to_string())
    }
}

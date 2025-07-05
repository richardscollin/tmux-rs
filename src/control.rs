// Copyright (c) 2012 Nicholas Marriott <nicholas.marriott@gmail.com>
// Copyright (c) 2012 George Nachman <tmux@georgester.com>
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
use std::cmp::Ordering;

use crate::compat::{
    queue::{tailq_empty, tailq_first, tailq_foreach, tailq_init, tailq_insert_tail, tailq_remove},
    tree::{rb_empty, rb_find, rb_foreach, rb_init, rb_insert, rb_remove},
};
use crate::log::fatalx_c;

#[repr(C)]
pub struct control_block {
    pub size: usize,
    pub line: *mut c_char,
    pub t: u64,

    pub entry: tailq_entry<control_block>,
    pub all_entry: tailq_entry<control_block>,
}

impl crate::compat::queue::Entry<control_block, discr_entry> for control_block {
    unsafe fn entry(this: *mut Self) -> *mut tailq_entry<control_block> {
        unsafe { &raw mut (*this).entry }
    }
}

impl crate::compat::queue::Entry<control_block, discr_all_entry> for control_block {
    unsafe fn entry(this: *mut Self) -> *mut tailq_entry<control_block> {
        unsafe { &raw mut (*this).all_entry }
    }
}

pub const CONTROL_PANE_OFF: i32 = 1;
pub const CONTROL_PANE_PAUSED: i32 = 2;

#[repr(C)]
pub struct control_pane {
    pub pane: u32,

    pub offset: window_pane_offset,
    pub queued: window_pane_offset,

    pub flags: i32,

    pub pending_flag: i32,
    pub pending_entry: tailq_entry<control_pane>,

    pub blocks: tailq_head<control_block>,

    pub entry: rb_entry<control_pane>,
}
pub type control_panes = rb_head<control_pane>;

impl Entry<control_pane, discr_pending_entry> for control_pane {
    unsafe fn entry(this: *mut Self) -> *mut tailq_entry<control_pane> {
        unsafe { &raw mut (*this).pending_entry }
    }
}

#[repr(C)]
pub struct control_sub_pane {
    pane: u32,
    idx: u32,
    last: *mut c_char,

    entry: rb_entry<control_sub_pane>,
}
pub type control_sub_panes = rb_head<control_sub_pane>;

#[repr(C)]
pub struct control_sub_window {
    window: u32,
    idx: u32,
    last: *mut c_char,

    entry: rb_entry<control_sub_window>,
}
pub type control_sub_windows = rb_head<control_sub_window>;

#[repr(C)]
pub struct control_sub {
    pub name: *mut c_char,
    pub format: *mut c_char,
    pub type_: control_sub_type,
    pub id: u32,

    pub last: *mut c_char,

    pub panes: control_sub_panes,
    pub windows: control_sub_windows,

    pub entry: rb_entry<control_sub>,
}
pub type control_subs = rb_head<control_sub>;

#[repr(C)]
pub struct control_state {
    pub panes: control_panes,

    pub pending_list: tailq_head<control_pane>,

    pub pending_count: u32,

    pub all_blocks: tailq_head<control_block>,

    pub read_event: *mut bufferevent,
    pub write_event: *mut bufferevent,

    pub subs: control_subs,
    pub subs_timer: event,
}

/// Low and high watermarks.
pub const CONTROL_BUFFER_LOW: i32 = 512;
pub const CONTROL_BUFFER_HIGH: i32 = 8192;

/// Minimum to write to each client.
pub const CONTROL_WRITE_MINIMUM: i32 = 32;

/// Maximum age for clients that are not using pause mode.
pub const CONTROL_MAXIMUM_AGE: u64 = 300000;

pub const CONTROL_IGNORE_FLAGS: client_flag =
    client_flag::CONTROL_NOOUTPUT.union(CLIENT_UNATTACHEDFLAGS);

pub fn control_pane_cmp(cp1: &control_pane, cp2: &control_pane) -> Ordering {
    cp1.pane.cmp(&cp2.pane)
}
RB_GENERATE!(
    control_panes,
    control_pane,
    entry,
    discr_entry,
    control_pane_cmp
);

pub fn control_sub_cmp(csub1: &control_sub, csub2: &control_sub) -> std::cmp::Ordering {
    unsafe { i32_to_ordering(libc::strcmp(csub1.name, csub2.name)) }
}
RB_GENERATE!(
    control_subs,
    control_sub,
    entry,
    discr_entry,
    control_sub_cmp
);

pub fn control_sub_pane_cmp(
    csp1: &control_sub_pane,
    csp2: &control_sub_pane,
) -> std::cmp::Ordering {
    csp1.pane
        .cmp(&csp2.pane)
        .then_with(|| csp1.idx.cmp(&csp2.idx))
}
RB_GENERATE!(
    control_sub_panes,
    control_sub_pane,
    entry,
    discr_entry,
    control_sub_pane_cmp
);

pub fn control_sub_window_cmp(csw1: &control_sub_window, csw2: &control_sub_window) -> Ordering {
    csw1.window
        .cmp(&csw2.window)
        .then_with(|| csw1.idx.cmp(&csw2.idx))
}
RB_GENERATE!(
    control_sub_windows,
    control_sub_window,
    entry,
    discr_entry,
    control_sub_window_cmp
);

pub unsafe fn control_free_sub(cs: *mut control_state, csub: *mut control_sub) {
    unsafe {
        for csp in rb_foreach(&raw mut (*csub).panes).map(NonNull::as_ptr) {
            rb_remove(&raw mut (*csub).panes, csp);
            free_(csp);
        }
        for csw in rb_foreach(&raw mut (*csub).windows).map(NonNull::as_ptr) {
            rb_remove(&raw mut (*csub).windows, csw);
            free_(csw);
        }
        free_((*csub).last);

        rb_remove(&raw mut (*cs).subs, csub);
        free_((*csub).name);
        free_((*csub).format);
        free_(csub);
    }
}

pub unsafe fn control_free_block(cs: *mut control_state, cb: *mut control_block) {
    unsafe {
        free_((*cb).line);
        tailq_remove::<_, discr_all_entry>(&raw mut (*cs).all_blocks, cb);
        free_(cb);
    }
}

pub unsafe fn control_get_pane(c: *mut client, wp: *mut window_pane) -> *mut control_pane {
    unsafe {
        let cs = (*c).control_state;
        let mut cp = MaybeUninit::<control_pane>::uninit();
        (*cp.as_mut_ptr()).pane = (*wp).id;
        rb_find(&raw mut (*cs).panes, cp.as_mut_ptr())
    }
}

pub unsafe fn control_add_pane(c: *mut client, wp: *mut window_pane) -> NonNull<control_pane> {
    unsafe {
        let cs = (*c).control_state;

        if let Some(cp) = NonNull::new(control_get_pane(c, wp)) {
            return cp;
        }

        let cp = xcalloc_::<control_pane>(1);
        (*cp.as_ptr()).pane = (*wp).id;
        rb_insert(&raw mut (*cs).panes, cp.as_ptr());

        (*cp.as_ptr()).offset = (*wp).offset;
        (*cp.as_ptr()).queued = (*wp).offset;
        tailq_init(&raw mut (*cp.as_ptr()).blocks);

        cp
    }
}

pub unsafe fn control_discard_pane(c: *mut client, cp: *mut control_pane) {
    unsafe {
        let cs = (*c).control_state;

        for cb in tailq_foreach::<_, discr_entry>(&raw mut (*cp).blocks).map(NonNull::as_ptr) {
            tailq_remove::<_, discr_entry>(&raw mut (*cp).blocks, cb);
            control_free_block(cs, cb);
        }
    }
}

pub unsafe fn control_window_pane(c: *mut client, pane: u32) -> Option<NonNull<window_pane>> {
    unsafe {
        if (*c).session.is_null() {
            return None;
        }
        let wp = NonNull::new(window_pane_find_by_id(pane))?;

        winlink_find_by_window(&raw mut (*(*c).session).windows, (*wp.as_ptr()).window)?;

        Some(wp)
    }
}

pub unsafe fn control_reset_offsets(c: *mut client) {
    unsafe {
        let cs = (*c).control_state;

        for cp in rb_foreach(&raw mut (*cs).panes).map(NonNull::as_ptr) {
            rb_remove(&raw mut (*cs).panes, cp);
            free_(cp);
        }

        tailq_init(&raw mut (*cs).pending_list);
        (*cs).pending_count = 0;
    }
}

pub unsafe fn control_pane_offset(
    c: *mut client,
    wp: *mut window_pane,
    off: *mut i32,
) -> *mut window_pane_offset {
    unsafe {
        let cs = (*c).control_state;

        if (*c).flags.intersects(client_flag::CONTROL_NOOUTPUT) {
            *off = 0;
            return null_mut();
        }

        let cp = control_get_pane(c, wp);
        if cp.is_null() || ((*cp).flags & CONTROL_PANE_PAUSED != 0) {
            *off = 0;
            return null_mut();
        }
        if (*cp).flags & CONTROL_PANE_OFF != 0 {
            *off = 1;
            return null_mut();
        }
        *off = (EVBUFFER_LENGTH((*(*cs).write_event).output) >= CONTROL_BUFFER_LOW as usize) as i32;
        &raw mut (*cp).offset
    }
}

pub unsafe fn control_set_pane_on(c: *mut client, wp: *mut window_pane) {
    unsafe {
        let cp = control_get_pane(c, wp);
        if !cp.is_null() && (*cp).flags & CONTROL_PANE_OFF != 0 {
            (*cp).flags &= !CONTROL_PANE_OFF;
            memcpy__(&raw mut (*cp).offset, &raw mut (*wp).offset);
            memcpy__(&raw mut (*cp).queued, &raw mut (*wp).offset);
        }
    }
}

pub unsafe fn control_set_pane_off(c: *mut client, wp: *mut window_pane) {
    unsafe {
        let cp = control_add_pane(c, wp);
        (*cp.as_ptr()).flags |= CONTROL_PANE_OFF;
    }
}

pub unsafe fn control_continue_pane(c: *mut client, wp: *mut window_pane) {
    unsafe {
        let cp = control_get_pane(c, wp);
        if !cp.is_null() && ((*cp).flags & CONTROL_PANE_PAUSED) != 0 {
            (*cp).flags &= !CONTROL_PANE_PAUSED;
            memcpy__(&raw mut (*cp).offset, &raw const (*wp).offset);
            memcpy__(&raw mut (*cp).queued, &raw const (*wp).offset);
            control_write!(c, "%continue %{}", (*wp).id);
        }
    }
}

pub unsafe fn control_pause_pane(c: *mut client, wp: *mut window_pane) {
    unsafe {
        let cp = control_add_pane(c, wp).as_ptr();
        if !(*cp).flags & CONTROL_PANE_PAUSED != 0 {
            (*cp).flags |= CONTROL_PANE_PAUSED;
            control_discard_pane(c, cp);
            control_write!(c, "%pause %{}", (*wp).id);
        }
    }
}

pub unsafe fn control_vwrite(c: *mut client, args: std::fmt::Arguments) {
    unsafe {
        let cs = (*c).control_state;

        let mut s = args.to_string();
        s.push('\0');
        let s = s.as_mut_ptr().cast();

        log_debug!(
            "{}: {}: writing line: {}",
            "control_vwrite",
            _s((*c).name),
            _s(s)
        );

        bufferevent_write((*cs).write_event, s.cast(), strlen(s));
        bufferevent_write((*cs).write_event, c"\n".as_ptr().cast(), 1);

        bufferevent_enable((*cs).write_event, EV_WRITE);
        free_(s);
    }
}

macro_rules! control_write {
   ($c:expr, $fmt:literal $(, $args:expr)* $(,)?) => {
        crate::control::control_write_($c, format_args!($fmt $(, $args)*))
    };
}
pub(crate) use control_write;

pub unsafe fn control_write_(c: *mut client, args: std::fmt::Arguments) {
    unsafe {
        let cs = (*c).control_state;

        if tailq_empty(&raw mut (*cs).all_blocks) {
            control_vwrite(c, args);
            return;
        }

        let cb = xcalloc_::<control_block>(1).as_ptr();
        let mut value = args.to_string();
        value.push('\0');
        (*cb).line = value.leak().as_mut_ptr().cast();
        tailq_insert_tail::<_, discr_all_entry>(&raw mut (*cs).all_blocks, cb);
        (*cb).t = get_timer();

        log_debug!(
            "{}: {}: storing line: {}",
            "control_write",
            _s((*c).name),
            _s((*cb).line)
        );
        bufferevent_enable((*cs).write_event, EV_WRITE);
    }
}

pub unsafe fn control_check_age(
    c: *mut client,
    wp: *mut window_pane,
    cp: *mut control_pane,
) -> i32 {
    let __func__ = "control_check_age";
    unsafe {
        let cb = tailq_first(&raw mut (*cp).blocks);
        if cb.is_null() {
            return 0;
        }
        let t = get_timer();
        if (*cb).t >= t {
            return 0;
        }

        let age = t - (*cb).t;
        log_debug!(
            "{}: {}: %%{} is {} behind",
            __func__,
            _s((*c).name),
            (*wp).id,
            age as c_ulonglong,
        );

        if (*c).flags.intersects(client_flag::CONTROL_PAUSEAFTER) {
            if age < (*c).pause_age as u64 {
                return 0;
            }
            (*cp).flags |= CONTROL_PANE_PAUSED;
            control_discard_pane(c, cp);
            control_write!(c, "%pause %{}", (*wp).id);
        } else {
            if age < CONTROL_MAXIMUM_AGE {
                return 0;
            }
            (*c).exit_message = xstrdup_(c"too far behind").as_ptr();
            (*c).flags |= client_flag::EXIT;
            control_discard(c);
        }
    }

    1
}

pub unsafe fn control_write_output(c: *mut client, wp: *mut window_pane) {
    let __func__ = "control_write_output";
    unsafe {
        let cs = (*c).control_state;
        let cp: *mut control_pane;
        let mut new_size = 0usize;

        'ignore: {
            if winlink_find_by_window(&raw mut (*(*c).session).windows, (*wp).window).is_none() {
                return;
            }

            if (*c).flags.intersects(CONTROL_IGNORE_FLAGS) {
                cp = control_get_pane(c, wp);
                if !cp.is_null() {
                    break 'ignore;
                }
                return;
            }
            cp = control_add_pane(c, wp).as_ptr();
            if (*cp).flags & (CONTROL_PANE_OFF | CONTROL_PANE_PAUSED) != 0 {
                break 'ignore;
            }
            if control_check_age(c, wp, cp) != 0 {
                return;
            }

            window_pane_get_new_data(wp, &raw mut (*cp).queued, &raw mut new_size);
            if new_size == 0 {
                return;
            }
            window_pane_update_used_data(wp, &raw mut (*cp).queued, new_size);

            let cb = xcalloc_::<control_block>(1).as_ptr();
            (*cb).size = new_size;
            tailq_insert_tail::<_, discr_all_entry>(&raw mut (*cs).all_blocks, cb);
            (*cb).t = get_timer();

            tailq_insert_tail::<_, discr_entry>(&raw mut (*cp).blocks, cb);
            log_debug!(
                "{}: {}: new output block of {} for %%{}",
                __func__,
                _s((*c).name),
                (*cb).size,
                (*wp).id,
            );

            if (*cp).pending_flag == 0 {
                log_debug!(
                    "{}: {}: %%{} now pending",
                    __func__,
                    _s((*c).name),
                    (*wp).id
                );
                tailq_insert_tail::<_, discr_pending_entry>(&raw mut (*cs).pending_list, cp);
                (*cp).pending_flag = 1;
                (*cs).pending_count += 1;
            }
            bufferevent_enable((*cs).write_event, EV_WRITE);
            return;
        }
        //ignore:
        log_debug!(
            "{}: {}: ignoring pane %%{}",
            __func__,
            _s((*c).name),
            (*wp).id
        );
        window_pane_update_used_data(wp, &raw mut (*cp).offset, usize::MAX);
        window_pane_update_used_data(wp, &raw mut (*cp).queued, usize::MAX);
    }
}

pub unsafe fn control_error(item: *mut cmdq_item, data: *mut c_void) -> cmd_retval {
    unsafe {
        let c = cmdq_get_client(item);
        let error = data as *mut c_char;

        cmdq_guard(item, c"begin".as_ptr(), true);
        control_write!(c, "parse error: {}", _s(error));
        cmdq_guard(item, c"error".as_ptr(), true);

        free_(error);
    }
    cmd_retval::CMD_RETURN_NORMAL
}

pub unsafe extern "C" fn control_error_callback(
    _bufev: *mut bufferevent,
    what: i16,
    data: *mut c_void,
) {
    let c: *mut client = data.cast();

    unsafe {
        (*c).flags |= client_flag::EXIT;
    }
}

pub unsafe extern "C" fn control_read_callback(bufev: *mut bufferevent, data: *mut c_void) {
    let __func__ = "control_read_callback";
    let c: *mut client = data.cast();

    unsafe {
        let cs = (*c).control_state;
        let buffer = (*(*cs).read_event).input;
        let mut error = null_mut();

        loop {
            let line = evbuffer_readln(buffer, null_mut(), evbuffer_eol_style_EVBUFFER_EOL_LF);
            if line.is_null() {
                break;
            }
            log_debug!("{}: {}: {}", __func__, _s((*c).name), _s(line));
            if *line == b'\0' as c_char {
                free_(line);
                (*c).flags |= client_flag::EXIT;
                break;
            }

            let state =
                cmdq_new_state(null_mut(), null_mut(), cmdq_state_flags::CMDQ_STATE_CONTROL);
            let status = cmd_parse_and_append(cstr_to_str(line), None, c, state, &raw mut error);
            if status == cmd_parse_status::CMD_PARSE_ERROR {
                cmdq_append(c, cmdq_get_callback!(control_error, error).as_ptr());
            }
            cmdq_free_state(state);

            free_(line);
        }
    }
}

pub unsafe fn control_all_done(c: *mut client) -> i32 {
    unsafe {
        let cs = (*c).control_state;

        if !tailq_empty(&raw mut (*cs).all_blocks) {
            return 0;
        }
        (EVBUFFER_LENGTH((*(*cs).write_event).output) == 0) as i32
    }
}

pub unsafe fn control_flush_all_blocks(c: *mut client) {
    let __func__ = "control_flush_all_blocks";
    unsafe {
        let cs = (*c).control_state;

        for cb in
            tailq_foreach::<_, discr_all_entry>(&raw mut (*cs).all_blocks).map(NonNull::as_ptr)
        {
            if (*cb).size != 0 {
                break;
            }
            log_debug!(
                "{}: {}: flushing line: {}",
                __func__,
                _s((*c).name),
                _s((*cb).line)
            );

            bufferevent_write((*cs).write_event, (*cb).line.cast(), strlen((*cb).line));
            bufferevent_write((*cs).write_event, c"\n".as_ptr().cast(), 1);
            control_free_block(cs, cb);
        }
    }
}

pub unsafe fn control_append_data(
    c: *mut client,
    cp: *mut control_pane,
    age: u64,
    mut message: *mut evbuffer,
    wp: *mut window_pane,
    size: usize,
) -> *mut evbuffer {
    unsafe {
        if message.is_null() {
            message = evbuffer_new();
            if message.is_null() {
                fatalx("out of memory");
            }
            if (*c).flags.intersects(client_flag::CONTROL_PAUSEAFTER) {
                evbuffer_add_printf!(message, "%extended-output %{} {} : ", (*wp).id, age);
            } else {
                evbuffer_add_printf!(message, "%output %{} ", (*wp).id);
            }
        }

        let mut new_size = 0usize;
        let new_data: *mut c_uchar =
            window_pane_get_new_data(wp, &raw mut (*cp).offset, &raw mut new_size).cast();
        if new_size < size {
            fatalx_!("not enough data: {} < {}", new_size, size);
        }
        for i in 0..size {
            if *new_data.add(i) < b' ' || *new_data.add(i) == b'\\' {
                evbuffer_add_printf!(message, "\\{:03o}", *new_data.add(i) as i32);
            } else {
                evbuffer_add_printf!(message, "{}", *new_data.add(i) as char);
            }
        }
        window_pane_update_used_data(wp, &raw mut (*cp).offset, size);
        message
    }
}

pub unsafe fn control_write_data(c: *mut client, message: *mut evbuffer) {
    unsafe {
        let cs = (*c).control_state;

        log_debug!(
            "control_write_data: {0}: {2:1$}",
            _s((*c).name),
            EVBUFFER_LENGTH(message),
            _s(EVBUFFER_DATA(message).cast()),
        );

        evbuffer_add(message, c"\n".as_ptr().cast(), 1);
        bufferevent_write_buffer((*cs).write_event, message);
        evbuffer_free(message);
    }
}

pub unsafe fn control_write_pending(c: *mut client, cp: *mut control_pane, limit: usize) -> i32 {
    unsafe {
        let cs = (*c).control_state;
        let mut message: *mut evbuffer = null_mut();
        let mut used = 0;
        let mut size;
        let mut cb = null_mut();
        let t = get_timer();

        let wp = control_window_pane(c, (*cp).pane);
        if wp.is_none() || (*wp.unwrap().as_ptr()).fd == -1 {
            for cb_ in tailq_foreach::<_, discr_entry>(&raw mut (*cp).blocks).map(NonNull::as_ptr) {
                cb = cb_;
                tailq_remove::<_, discr_entry>(&raw mut (*cp).blocks, cb);
                control_free_block(cs, cb);
            }
            control_flush_all_blocks(c);
            return 0;
        }

        while used != limit && !tailq_empty(&raw mut (*cp).blocks) {
            if control_check_age(c, transmute_ptr(wp), cp) != 0 {
                if !message.is_null() {
                    evbuffer_free(message);
                }
                message = null_mut();
                break;
            }

            cb = tailq_first(&raw mut (*cp).blocks);
            let age = t.saturating_sub((*cb).t);
            log_debug!(
                "{}: {}: output block {} (age {}) for %%{} (used {}/{})",
                "control_write_pending",
                _s((*c).name),
                (*cb).size,
                age,
                (*cp).pane,
                used,
                limit,
            );

            size = (*cb).size;
            if size > limit - used {
                size = limit - used;
            }
            used += size;

            message = control_append_data(c, cp, age, message, transmute_ptr(wp), size);

            (*cb).size -= size;
            if (*cb).size == 0 {
                tailq_remove::<_, discr_entry>(&raw mut (*cp).blocks, cb);
                control_free_block(cs, cb);

                cb = tailq_first(&raw mut (*cs).all_blocks);
                if !cb.is_null() && (*cb).size == 0 {
                    if wp.is_some() && !message.is_null() {
                        control_write_data(c, message);
                        message = null_mut();
                    }
                    control_flush_all_blocks(c);
                }
            }
        }
        if !message.is_null() {
            control_write_data(c, message);
        }
        !tailq_empty(&raw mut (*cp).blocks) as i32
    }
}

pub unsafe extern "C" fn control_write_callback(bufev: *mut bufferevent, data: *mut c_void) {
    unsafe {
        let c: *mut client = data.cast();
        let cs = (*c).control_state;
        let evb = (*(*cs).write_event).output;

        control_flush_all_blocks(c);

        while EVBUFFER_LENGTH(evb) < CONTROL_BUFFER_HIGH as usize {
            if (*cs).pending_count == 0 {
                break;
            }
            let space = CONTROL_BUFFER_HIGH as usize - EVBUFFER_LENGTH(evb);
            log_debug!(
                "{}: {}: {} bytes available, {} panes",
                "control_write_callback",
                _s((*c).name),
                space,
                (*cs).pending_count,
            );

            let mut limit: usize = space / (*cs).pending_count as usize / 3;
            if limit < CONTROL_WRITE_MINIMUM as usize {
                limit = CONTROL_WRITE_MINIMUM as usize;
            }

            for cp in tailq_foreach::<_, discr_pending_entry>(&raw mut (*cs).pending_list)
                .map(NonNull::as_ptr)
            {
                if EVBUFFER_LENGTH(evb) >= CONTROL_BUFFER_HIGH as usize {
                    break;
                }
                if control_write_pending(c, cp, limit) != 0 {
                    continue;
                }
                tailq_remove::<_, discr_pending_entry>(&raw mut (*cs).pending_list, cp);
                (*cp).pending_flag = 0;
                (*cs).pending_count -= 1;
            }
        }
        if EVBUFFER_LENGTH(evb) == 0 {
            bufferevent_disable((*cs).write_event, EV_WRITE);
        }
    }
}

pub unsafe fn control_start(c: *mut client) {
    unsafe {
        if (*c).flags.intersects(client_flag::CONTROLCONTROL) {
            libc::close((*c).out_fd);
            (*c).out_fd = -1;
        } else {
            setblocking((*c).out_fd, 0);
        }
        setblocking((*c).fd, 0);

        (*c).control_state = xcalloc_::<control_state>(1).as_ptr();
        let cs = (*c).control_state;
        rb_init(&raw mut (*cs).panes);
        tailq_init(&raw mut (*cs).pending_list);
        tailq_init(&raw mut (*cs).all_blocks);
        rb_init(&raw mut (*cs).subs);

        (*cs).read_event = bufferevent_new(
            (*c).fd,
            Some(control_read_callback),
            Some(control_write_callback),
            Some(control_error_callback),
            c.cast(),
        );
        if (*cs).read_event.is_null() {
            fatalx("out of memory");
        }

        if (*c).flags.intersects(client_flag::CONTROLCONTROL) {
            (*cs).write_event = (*cs).read_event;
        } else {
            (*cs).write_event = bufferevent_new(
                (*c).out_fd,
                None,
                Some(control_write_callback),
                Some(control_error_callback),
                c.cast(),
            );
            if (*cs).write_event.is_null() {
                fatalx("out of memory");
            }
        }
        bufferevent_setwatermark((*cs).write_event, EV_WRITE, CONTROL_BUFFER_LOW as usize, 0);

        if (*c).flags.intersects(client_flag::CONTROLCONTROL) {
            bufferevent_write((*cs).write_event, c"\x1bP1000p".as_ptr().cast(), 7);
            bufferevent_enable((*cs).write_event, EV_WRITE);
        }
    }
}

pub unsafe fn control_ready(c: *mut client) {
    unsafe {
        bufferevent_enable((*(*c).control_state).read_event, EV_READ);
    }
}

pub unsafe fn control_discard(c: *mut client) {
    unsafe {
        let cs = (*c).control_state;
        for cp in rb_foreach(&raw mut (*cs).panes) {
            control_discard_pane(c, cp.as_ptr());
        }
        bufferevent_disable((*cs).read_event, EV_READ);
    }
}

pub unsafe fn control_stop(c: *mut client) {
    unsafe {
        let cs = (*c).control_state;
        if !(*c).flags.intersects(client_flag::CONTROLCONTROL) {
            bufferevent_free((*cs).write_event);
        }
        bufferevent_free((*cs).read_event);

        for csub in rb_foreach(&raw mut (*cs).subs).map(NonNull::as_ptr) {
            control_free_sub(cs, csub);
        }
        if evtimer_initialized(&raw mut (*cs).subs_timer) {
            evtimer_del(&raw mut (*cs).subs_timer);
        }

        for cb in
            tailq_foreach::<_, discr_all_entry>(&raw mut (*cs).all_blocks).map(NonNull::as_ptr)
        {
            control_free_block(cs, cb);
        }
        control_reset_offsets(c);

        free_(cs);
    }
}

pub unsafe fn control_check_subs_session(c: *mut client, csub: *mut control_sub) {
    unsafe {
        let s = (*c).session;

        let ft = format_create_defaults(null_mut(), c, s, null_mut(), null_mut());
        let value = format_expand(ft, (*csub).format);
        format_free(ft);

        if !(*csub).last.is_null() && libc::strcmp(value, (*csub).last) == 0 {
            free_(value);
            return;
        }
        control_write!(
            c,
            "%subscription-changed {} ${} - - - : {}",
            _s((*csub).name),
            (*s).id,
            _s(value),
        );
        free_((*csub).last);
        (*csub).last = value;
    }
}

pub unsafe fn control_check_subs_pane(c: *mut client, csub: *mut control_sub) {
    unsafe {
        let s = (*c).session;
        let mut find: control_sub_pane = zeroed(); //TODO uninit

        let wp = window_pane_find_by_id((*csub).id);
        if wp.is_null() || (*wp).fd == -1 {
            return;
        }
        let w = (*wp).window;

        for wl in tailq_foreach::<_, discr_wentry>(&raw mut (*w).winlinks).map(NonNull::as_ptr) {
            if (*wl).session != s {
                continue;
            }

            let ft = format_create_defaults(null_mut(), c, s, wl, wp);
            let value = format_expand(ft, (*csub).format);
            format_free(ft);

            find.pane = (*wp).id;
            find.idx = (*wl).idx as u32;

            let mut csp = rb_find(&raw mut (*csub).panes, &raw mut find);
            if csp.is_null() {
                csp = xcalloc_::<control_sub_pane>(1).as_ptr();
                (*csp).pane = (*wp).id;
                (*csp).idx = (*wl).idx as u32;
                rb_insert(&raw mut (*csub).panes, csp);
            }

            if !(*csp).last.is_null() && libc::strcmp(value, (*csp).last) == 0 {
                free_(value);
                continue;
            }
            control_write!(
                c,
                "%subscription-changed {} ${} @{} {} %{} : {}",
                _s((*csub).name),
                (*s).id,
                (*w).id,
                (*wl).idx,
                (*wp).id,
                _s(value),
            );
            free_((*csp).last);
            (*csp).last = value;
        }
    }
}

pub unsafe fn control_check_subs_all_panes(c: *mut client, csub: *mut control_sub) {
    unsafe {
        let s = (*c).session;
        let mut find: control_sub_pane = zeroed();

        for wl in rb_foreach(&raw mut (*s).windows).map(NonNull::as_ptr) {
            let w = (*wl).window;
            for wp in tailq_foreach::<_, discr_entry>(&raw mut (*w).panes).map(NonNull::as_ptr) {
                let ft = format_create_defaults(null_mut(), c, s, wl, wp);
                let value = format_expand(ft, (*csub).format);
                format_free(ft);

                find.pane = (*wp).id;
                find.idx = (*wl).idx as u32;

                let mut csp = rb_find(&raw mut (*csub).panes, &raw mut find);
                if csp.is_null() {
                    csp = xcalloc_::<control_sub_pane>(1).as_ptr();
                    (*csp).pane = (*wp).id;
                    (*csp).idx = (*wl).idx as u32;
                    rb_insert(&raw mut (*csub).panes, csp);
                }

                if !(*csp).last.is_null() && libc::strcmp(value, (*csp).last) == 0 {
                    free_(value);
                    continue;
                }
                control_write!(
                    c,
                    "%subscription-changed {} ${} @{} {} %{} : {}",
                    _s((*csub).name),
                    (*s).id,
                    (*w).id,
                    (*wl).idx,
                    (*wp).id,
                    _s(value),
                );
                free_((*csp).last);
                (*csp).last = value;
            }
        }
    }
}

pub unsafe fn control_check_subs_window(c: *mut client, csub: *mut control_sub) {
    unsafe {
        let s = (*c).session;
        let mut find: control_sub_window = zeroed(); // TODO uninit

        let w = window_find_by_id((*csub).id);
        if w.is_null() {
            return;
        }

        for wl in
            tailq_foreach::<winlink, discr_wentry>(&raw mut (*w).winlinks).map(NonNull::as_ptr)
        {
            if (*wl).session != s {
                continue;
            }

            let ft = format_create_defaults(null_mut(), c, s, wl, null_mut());
            let value = format_expand(ft, (*csub).format);
            format_free(ft);

            find.window = (*w).id;
            find.idx = (*wl).idx as u32;

            let mut csw = rb_find(&raw mut (*csub).windows, &raw mut find);
            if csw.is_null() {
                csw = xcalloc_::<control_sub_window>(1).as_ptr();
                (*csw).window = (*w).id;
                (*csw).idx = (*wl).idx as u32;
                rb_insert(&raw mut (*csub).windows, csw);
            }

            if !(*csw).last.is_null() && libc::strcmp(value, (*csw).last) == 0 {
                free_(value);
                continue;
            }
            control_write!(
                c,
                "%subscription-changed {} ${} @{} {} - : {}",
                _s((*csub).name),
                (*s).id,
                (*w).id,
                (*wl).idx,
                _s(value),
            );
            free_((*csw).last);
            (*csw).last = value;
        }
    }
}

pub unsafe fn control_check_subs_all_windows(c: *mut client, csub: *mut control_sub) {
    unsafe {
        let s = (*c).session;
        let mut find: control_sub_window = zeroed();

        for wl in rb_foreach(&raw mut (*s).windows).map(NonNull::as_ptr) {
            let w = (*wl).window;

            let ft = format_create_defaults(null_mut(), c, s, wl, null_mut());
            let value = format_expand(ft, (*csub).format);
            format_free(ft);

            find.window = (*w).id;
            find.idx = (*wl).idx as u32;

            let mut csw = rb_find(&raw mut (*csub).windows, &raw mut find);
            if csw.is_null() {
                csw = xcalloc_::<control_sub_window>(1).as_ptr();
                (*csw).window = (*w).id;
                (*csw).idx = (*wl).idx as u32;
                rb_insert(&raw mut (*csub).windows, csw);
            }

            if !(*csw).last.is_null() && libc::strcmp(value, (*csw).last) == 0 {
                free_(value);
                continue;
            }
            control_write!(
                c,
                "%subscription-changed {} ${} @{} {} - : {}",
                _s((*csub).name),
                (*s).id,
                (*w).id,
                (*wl).idx,
                _s(value),
            );
            free_((*csw).last);
            (*csw).last = value;
        }
    }
}

pub unsafe extern "C" fn control_check_subs_timer(fd: i32, events: i16, data: *mut c_void) {
    unsafe {
        let c: *mut client = data.cast();
        let cs = (*c).control_state;
        let mut tv = timeval {
            tv_sec: 1,
            tv_usec: 0,
        };

        log_debug!("{}: timer fired", "control_check_subs_timer");
        evtimer_add(&raw mut (*cs).subs_timer, &raw mut tv);

        for csub in rb_foreach(&raw mut (*cs).subs).map(NonNull::as_ptr) {
            match (*csub).type_ {
                control_sub_type::CONTROL_SUB_SESSION => control_check_subs_session(c, csub),
                control_sub_type::CONTROL_SUB_PANE => control_check_subs_pane(c, csub),
                control_sub_type::CONTROL_SUB_ALL_PANES => control_check_subs_all_panes(c, csub),
                control_sub_type::CONTROL_SUB_WINDOW => control_check_subs_window(c, csub),
                control_sub_type::CONTROL_SUB_ALL_WINDOWS => {
                    control_check_subs_all_windows(c, csub)
                }
            }
        }
    }
}

pub unsafe fn control_add_sub(
    c: *mut client,
    name: *mut c_char,
    type_: control_sub_type,
    id: i32,
    format: *const c_char,
) {
    unsafe {
        let cs = (*c).control_state;
        let tv = timeval {
            tv_sec: 1,
            tv_usec: 0,
        };

        let mut find: control_sub = zeroed();

        find.name = name.cast();
        let mut csub = rb_find(&raw mut (*cs).subs, &raw mut find);
        if !csub.is_null() {
            control_free_sub(cs, csub);
        }

        csub = xcalloc_::<control_sub>(1).as_ptr();
        (*csub).name = xstrdup(name).as_ptr();
        (*csub).type_ = type_;
        (*csub).id = id as u32;
        (*csub).format = xstrdup(format).as_ptr();
        rb_insert(&raw mut (*cs).subs, csub);

        rb_init(&raw mut (*csub).panes);
        rb_init(&raw mut (*csub).windows);

        if !evtimer_initialized(&raw mut (*cs).subs_timer) {
            evtimer_set(
                &raw mut (*cs).subs_timer,
                Some(control_check_subs_timer),
                c.cast(),
            );
        }
        if evtimer_pending(&raw mut (*cs).subs_timer, null_mut()) == 0 {
            evtimer_add(&raw mut (*cs).subs_timer, &tv);
        }
    }
}

pub unsafe fn control_remove_sub(c: *mut client, name: *mut c_char) {
    unsafe {
        let cs = (*c).control_state;

        let mut find: control_sub = zeroed();
        find.name = name.cast();
        let csub = rb_find(&raw mut (*cs).subs, &raw mut find);
        if !csub.is_null() {
            control_free_sub(cs, csub);
        }
        if rb_empty(&raw mut (*cs).subs) {
            evtimer_del(&raw mut (*cs).subs_timer);
        }
    }
}

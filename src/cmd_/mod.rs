// Copyright (c) 2007 Nicholas Marriott <nicholas.marriott@gmail.com>
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

use crate::libc::{strchr, strcmp, strlen, strncmp};

use crate::compat::{
    queue::{
        tailq_concat, tailq_first, tailq_foreach, tailq_init, tailq_insert_tail, tailq_next,
        tailq_remove,
    },
    strlcat, strlcpy,
};
use crate::xmalloc::{xrealloc_, xreallocarray_};

pub mod cmd_attach_session;
pub mod cmd_bind_key;
pub mod cmd_break_pane;
pub mod cmd_capture_pane;
pub mod cmd_choose_tree;
pub mod cmd_command_prompt;
pub mod cmd_confirm_before;
pub mod cmd_copy_mode;
pub mod cmd_detach_client;
pub mod cmd_display_menu;
pub mod cmd_display_message;
pub mod cmd_display_panes;
pub mod cmd_find;
pub mod cmd_find_window;
pub mod cmd_if_shell;
pub mod cmd_join_pane;
pub mod cmd_kill_pane;
pub mod cmd_kill_server;
pub mod cmd_kill_session;
pub mod cmd_kill_window;
pub mod cmd_list_buffers;
pub mod cmd_list_clients;
pub mod cmd_list_keys;
pub mod cmd_list_panes;
pub mod cmd_list_sessions;
pub mod cmd_list_windows;
pub mod cmd_load_buffer;
pub mod cmd_lock_server;
pub mod cmd_move_window;
pub mod cmd_new_session;
pub mod cmd_new_window;
pub mod cmd_paste_buffer;
pub mod cmd_pipe_pane;
pub mod cmd_queue;
pub mod cmd_refresh_client;
pub mod cmd_rename_session;
pub mod cmd_rename_window;
pub mod cmd_resize_pane;
pub mod cmd_resize_window;
pub mod cmd_respawn_pane;
pub mod cmd_respawn_window;
pub mod cmd_rotate_window;
pub mod cmd_run_shell;
pub mod cmd_save_buffer;
pub mod cmd_select_layout;
pub mod cmd_select_pane;
pub mod cmd_select_window;
pub mod cmd_send_keys;
pub mod cmd_server_access;
pub mod cmd_set_buffer;
pub mod cmd_set_environment;
pub mod cmd_set_option;
pub mod cmd_show_environment;
pub mod cmd_show_messages;
pub mod cmd_show_options;
pub mod cmd_show_prompt_history;
pub mod cmd_source_file;
pub mod cmd_split_window;
pub mod cmd_swap_pane;
pub mod cmd_swap_window;
pub mod cmd_switch_client;
pub mod cmd_unbind_key;
pub mod cmd_wait_for;

use cmd_attach_session::CMD_ATTACH_SESSION_ENTRY;
use cmd_bind_key::CMD_BIND_KEY_ENTRY;
use cmd_break_pane::CMD_BREAK_PANE_ENTRY;
use cmd_capture_pane::{CMD_CAPTURE_PANE_ENTRY, CMD_CLEAR_HISTORY_ENTRY};
use cmd_choose_tree::{
    CMD_CHOOSE_BUFFER_ENTRY, CMD_CHOOSE_CLIENT_ENTRY, CMD_CHOOSE_TREE_ENTRY,
    CMD_CUSTOMIZE_MODE_ENTRY,
};
use cmd_command_prompt::CMD_COMMAND_PROMPT_ENTRY;
use cmd_confirm_before::CMD_CONFIRM_BEFORE_ENTRY;
use cmd_copy_mode::{CMD_CLOCK_MODE_ENTRY, CMD_COPY_MODE_ENTRY};
use cmd_detach_client::CMD_DETACH_CLIENT_ENTRY;
use cmd_detach_client::CMD_SUSPEND_CLIENT_ENTRY;
use cmd_display_menu::{CMD_DISPLAY_MENU_ENTRY, CMD_DISPLAY_POPUP_ENTRY};
use cmd_display_message::CMD_DISPLAY_MESSAGE_ENTRY;
use cmd_display_panes::CMD_DISPLAY_PANES_ENTRY;
use cmd_find_window::CMD_FIND_WINDOW_ENTRY;
use cmd_if_shell::CMD_IF_SHELL_ENTRY;
use cmd_join_pane::{CMD_JOIN_PANE_ENTRY, CMD_MOVE_PANE_ENTRY};
use cmd_kill_pane::CMD_KILL_PANE_ENTRY;
use cmd_kill_server::CMD_KILL_SERVER_ENTRY;
use cmd_kill_server::CMD_START_SERVER_ENTRY;
use cmd_kill_session::CMD_KILL_SESSION_ENTRY;
use cmd_kill_window::CMD_KILL_WINDOW_ENTRY;
use cmd_kill_window::CMD_UNLINK_WINDOW_ENTRY;
use cmd_list_buffers::CMD_LIST_BUFFERS_ENTRY;
use cmd_list_clients::CMD_LIST_CLIENTS_ENTRY;
use cmd_list_keys::{CMD_LIST_COMMANDS_ENTRY, CMD_LIST_KEYS_ENTRY};
use cmd_list_panes::CMD_LIST_PANES_ENTRY;
use cmd_list_sessions::CMD_LIST_SESSIONS_ENTRY;
use cmd_list_windows::CMD_LIST_WINDOWS_ENTRY;
use cmd_load_buffer::CMD_LOAD_BUFFER_ENTRY;
use cmd_lock_server::{CMD_LOCK_CLIENT_ENTRY, CMD_LOCK_SERVER_ENTRY, CMD_LOCK_SESSION_ENTRY};
use cmd_move_window::CMD_LINK_WINDOW_ENTRY;
use cmd_move_window::CMD_MOVE_WINDOW_ENTRY;
use cmd_new_session::CMD_HAS_SESSION_ENTRY;
use cmd_new_session::CMD_NEW_SESSION_ENTRY;
use cmd_new_window::CMD_NEW_WINDOW_ENTRY;
use cmd_paste_buffer::CMD_PASTE_BUFFER_ENTRY;
use cmd_pipe_pane::CMD_PIPE_PANE_ENTRY;
use cmd_refresh_client::CMD_REFRESH_CLIENT_ENTRY;
use cmd_rename_session::CMD_RENAME_SESSION_ENTRY;
use cmd_rename_window::CMD_RENAME_WINDOW_ENTRY;
use cmd_resize_pane::CMD_RESIZE_PANE_ENTRY;
use cmd_resize_window::CMD_RESIZE_WINDOW_ENTRY;
use cmd_respawn_pane::CMD_RESPAWN_PANE_ENTRY;
use cmd_respawn_window::CMD_RESPAWN_WINDOW_ENTRY;
use cmd_rotate_window::CMD_ROTATE_WINDOW_ENTRY;
use cmd_run_shell::CMD_RUN_SHELL_ENTRY;
use cmd_save_buffer::CMD_SAVE_BUFFER_ENTRY;
use cmd_save_buffer::CMD_SHOW_BUFFER_ENTRY;
use cmd_select_layout::CMD_NEXT_LAYOUT_ENTRY;
use cmd_select_layout::CMD_PREVIOUS_LAYOUT_ENTRY;
use cmd_select_layout::CMD_SELECT_LAYOUT_ENTRY;
use cmd_select_pane::CMD_LAST_PANE_ENTRY;
use cmd_select_pane::CMD_SELECT_PANE_ENTRY;
use cmd_select_window::CMD_LAST_WINDOW_ENTRY;
use cmd_select_window::CMD_NEXT_WINDOW_ENTRY;
use cmd_select_window::CMD_PREVIOUS_WINDOW_ENTRY;
use cmd_select_window::CMD_SELECT_WINDOW_ENTRY;
use cmd_send_keys::CMD_SEND_KEYS_ENTRY;
use cmd_send_keys::CMD_SEND_PREFIX_ENTRY;
use cmd_server_access::CMD_SERVER_ACCESS_ENTRY;
use cmd_set_buffer::CMD_DELETE_BUFFER_ENTRY;
use cmd_set_buffer::CMD_SET_BUFFER_ENTRY;
use cmd_set_environment::CMD_SET_ENVIRONMENT_ENTRY;
use cmd_set_option::CMD_SET_HOOK_ENTRY;
use cmd_set_option::CMD_SET_OPTION_ENTRY;
use cmd_set_option::CMD_SET_WINDOW_OPTION_ENTRY;
use cmd_show_environment::CMD_SHOW_ENVIRONMENT_ENTRY;
use cmd_show_messages::CMD_SHOW_MESSAGES_ENTRY;
use cmd_show_options::CMD_SHOW_HOOKS_ENTRY;
use cmd_show_options::CMD_SHOW_OPTIONS_ENTRY;
use cmd_show_options::CMD_SHOW_WINDOW_OPTIONS_ENTRY;
use cmd_show_prompt_history::{CMD_CLEAR_PROMPT_HISTORY_ENTRY, CMD_SHOW_PROMPT_HISTORY_ENTRY};
use cmd_source_file::CMD_SOURCE_FILE_ENTRY;
use cmd_split_window::CMD_SPLIT_WINDOW_ENTRY;
use cmd_swap_pane::CMD_SWAP_PANE_ENTRY;
use cmd_swap_window::CMD_SWAP_WINDOW_ENTRY;
use cmd_switch_client::CMD_SWITCH_CLIENT_ENTRY;
use cmd_unbind_key::CMD_UNBIND_KEY_ENTRY;
use cmd_wait_for::CMD_WAIT_FOR_ENTRY;

pub static CMD_TABLE: [&cmd_entry; 90] = [
    &CMD_ATTACH_SESSION_ENTRY,
    &CMD_BIND_KEY_ENTRY,
    &CMD_BREAK_PANE_ENTRY,
    &CMD_CAPTURE_PANE_ENTRY,
    &CMD_CHOOSE_BUFFER_ENTRY,
    &CMD_CHOOSE_CLIENT_ENTRY,
    &CMD_CHOOSE_TREE_ENTRY,
    &CMD_CLEAR_HISTORY_ENTRY,
    &CMD_CLEAR_PROMPT_HISTORY_ENTRY,
    &CMD_CLOCK_MODE_ENTRY,
    &CMD_COMMAND_PROMPT_ENTRY,
    &CMD_CONFIRM_BEFORE_ENTRY,
    &CMD_COPY_MODE_ENTRY,
    &CMD_CUSTOMIZE_MODE_ENTRY,
    &CMD_DELETE_BUFFER_ENTRY,
    &CMD_DETACH_CLIENT_ENTRY,
    &CMD_DISPLAY_MENU_ENTRY,
    &CMD_DISPLAY_MESSAGE_ENTRY,
    &CMD_DISPLAY_POPUP_ENTRY,
    &CMD_DISPLAY_PANES_ENTRY,
    &CMD_FIND_WINDOW_ENTRY,
    &CMD_HAS_SESSION_ENTRY,
    &CMD_IF_SHELL_ENTRY,
    &CMD_JOIN_PANE_ENTRY,
    &CMD_KILL_PANE_ENTRY,
    &CMD_KILL_SERVER_ENTRY,
    &CMD_KILL_SESSION_ENTRY,
    &CMD_KILL_WINDOW_ENTRY,
    &CMD_LAST_PANE_ENTRY,
    &CMD_LAST_WINDOW_ENTRY,
    &CMD_LINK_WINDOW_ENTRY,
    &CMD_LIST_BUFFERS_ENTRY,
    &CMD_LIST_CLIENTS_ENTRY,
    &CMD_LIST_COMMANDS_ENTRY,
    &CMD_LIST_KEYS_ENTRY,
    &CMD_LIST_PANES_ENTRY,
    &CMD_LIST_SESSIONS_ENTRY,
    &CMD_LIST_WINDOWS_ENTRY,
    &CMD_LOAD_BUFFER_ENTRY,
    &CMD_LOCK_CLIENT_ENTRY,
    &CMD_LOCK_SERVER_ENTRY,
    &CMD_LOCK_SESSION_ENTRY,
    &CMD_MOVE_PANE_ENTRY,
    &CMD_MOVE_WINDOW_ENTRY,
    &CMD_NEW_SESSION_ENTRY,
    &CMD_NEW_WINDOW_ENTRY,
    &CMD_NEXT_LAYOUT_ENTRY,
    &CMD_NEXT_WINDOW_ENTRY,
    &CMD_PASTE_BUFFER_ENTRY,
    &CMD_PIPE_PANE_ENTRY,
    &CMD_PREVIOUS_LAYOUT_ENTRY,
    &CMD_PREVIOUS_WINDOW_ENTRY,
    &CMD_REFRESH_CLIENT_ENTRY,
    &CMD_RENAME_SESSION_ENTRY,
    &CMD_RENAME_WINDOW_ENTRY,
    &CMD_RESIZE_PANE_ENTRY,
    &CMD_RESIZE_WINDOW_ENTRY,
    &CMD_RESPAWN_PANE_ENTRY,
    &CMD_RESPAWN_WINDOW_ENTRY,
    &CMD_ROTATE_WINDOW_ENTRY,
    &CMD_RUN_SHELL_ENTRY,
    &CMD_SAVE_BUFFER_ENTRY,
    &CMD_SELECT_LAYOUT_ENTRY,
    &CMD_SELECT_PANE_ENTRY,
    &CMD_SELECT_WINDOW_ENTRY,
    &CMD_SEND_KEYS_ENTRY,
    &CMD_SEND_PREFIX_ENTRY,
    &CMD_SERVER_ACCESS_ENTRY,
    &CMD_SET_BUFFER_ENTRY,
    &CMD_SET_ENVIRONMENT_ENTRY,
    &CMD_SET_HOOK_ENTRY,
    &CMD_SET_OPTION_ENTRY,
    &CMD_SET_WINDOW_OPTION_ENTRY,
    &CMD_SHOW_BUFFER_ENTRY,
    &CMD_SHOW_ENVIRONMENT_ENTRY,
    &CMD_SHOW_HOOKS_ENTRY,
    &CMD_SHOW_MESSAGES_ENTRY,
    &CMD_SHOW_OPTIONS_ENTRY,
    &CMD_SHOW_PROMPT_HISTORY_ENTRY,
    &CMD_SHOW_WINDOW_OPTIONS_ENTRY,
    &CMD_SOURCE_FILE_ENTRY,
    &CMD_SPLIT_WINDOW_ENTRY,
    &CMD_START_SERVER_ENTRY,
    &CMD_SUSPEND_CLIENT_ENTRY,
    &CMD_SWAP_PANE_ENTRY,
    &CMD_SWAP_WINDOW_ENTRY,
    &CMD_SWITCH_CLIENT_ENTRY,
    &CMD_UNBIND_KEY_ENTRY,
    &CMD_UNLINK_WINDOW_ENTRY,
    &CMD_WAIT_FOR_ENTRY,
];

// Instance of a command.
#[repr(C)]
pub struct cmd {
    pub entry: &'static cmd_entry,
    pub args: *mut args,
    pub group: u32,
    pub file: *mut u8,
    pub line: u32,

    pub qentry: tailq_entry<cmd>,
}
pub type cmds = tailq_head<cmd>;

pub struct qentry;
impl Entry<cmd, qentry> for cmd {
    unsafe fn entry(this: *mut Self) -> *mut tailq_entry<cmd> {
        unsafe { &raw mut (*this).qentry }
    }
}

/// Next group number for new command list.
static CMD_LIST_NEXT_GROUP: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(1);

macro_rules! cmd_log_argv {
   ($argc:expr, $argv:expr, $fmt:literal $(, $args:expr)* $(,)?) => {
        crate::cmd_::cmd_log_argv_($argc, $argv, format_args!($fmt $(, $args)*))
    };
}
pub(crate) use cmd_log_argv;

// Log an argument vector.
pub unsafe fn cmd_log_argv_(argc: i32, argv: *mut *mut u8, args: std::fmt::Arguments) {
    unsafe {
        let prefix = args.to_string();
        for i in 0..argc {
            log_debug!("{}: argv[{}]{}", prefix, i, _s(*argv.add(i as usize)));
        }
    }
}

pub unsafe fn cmd_append_argv(argc: *mut c_int, argv: *mut *mut *mut u8, arg: *const u8) {
    unsafe {
        *argv = xreallocarray_::<*mut u8>(*argv, (*argc) as usize + 1).as_ptr();
        *(*argv).add((*argc) as usize) = xstrdup(arg).as_ptr();
        *argc += 1;
    }
}

pub unsafe fn cmd_pack_argv(
    argc: c_int,
    argv: *mut *mut u8,
    mut buf: *mut u8,
    mut len: usize,
) -> c_int {
    unsafe {
        //
        if argc == 0 {
            return 0;
        }
        cmd_log_argv!(argc, argv, "cmd_pack_argv");

        *buf = b'\0';
        for i in 0..argc {
            if strlcpy(buf, *argv.add(i as usize), len) >= len {
                return -1;
            }
            let arglen = strlen(*argv.add(i as usize)) + 1;
            buf = buf.add(arglen);
            len -= arglen;
        }

        0
    }
}

pub unsafe fn cmd_unpack_argv(
    mut buf: *mut u8,
    mut len: usize,
    argc: c_int,
    argv: *mut *mut *mut u8,
) -> c_int {
    unsafe {
        if argc == 0 {
            return 0;
        }
        *argv = xcalloc_::<*mut u8>(argc as usize).as_ptr();

        *buf.add(len - 1) = b'\0';
        for i in 0..argc {
            if len == 0 {
                cmd_free_argv(argc, *argv);
                return -1;
            }

            let arglen = strlen(buf) + 1;
            *(*argv).add(i as usize) = xstrdup(buf).as_ptr();

            buf = buf.add(arglen);
            len -= arglen;
        }
        cmd_log_argv!(argc, *argv, "cmd_unpack_argv");

        0
    }
}

pub unsafe fn cmd_copy_argv(argc: c_int, argv: *mut *mut u8) -> *mut *mut u8 {
    unsafe {
        if argc == 0 {
            return null_mut();
        }
        let new_argv: *mut *mut u8 = xcalloc(argc as usize + 1, size_of::<*mut u8>())
            .cast()
            .as_ptr();
        for i in 0..argc {
            if !(*argv.add(i as usize)).is_null() {
                *new_argv.add(i as usize) = xstrdup(*argv.add(i as usize)).as_ptr();
            }
        }
        new_argv
    }
}

pub unsafe fn cmd_free_argv(argc: c_int, argv: *mut *mut u8) {
    unsafe {
        if argc == 0 {
            return;
        }
        for i in 0..argc {
            free(*argv.add(i as usize) as _);
        }
        free(argv as _);
    }
}

pub unsafe fn cmd_stringify_argv(argc: c_int, argv: *mut *mut u8) -> *mut u8 {
    unsafe {
        let mut buf: *mut u8 = null_mut();
        let mut len: usize = 0;

        if argc == 0 {
            return xstrdup(c!("")).as_ptr();
        }

        for i in 0..argc {
            let s = args_escape(*argv.add(i as usize));
            log_debug!(
                "{}: {} {} = {}",
                "cmd_stringify_argv",
                i,
                _s(*argv.add(i as usize)),
                _s(s)
            );

            len += strlen(s) + 1;
            buf = xrealloc_(buf, len).as_ptr();

            if i == 0 {
                *buf = b'\0';
            } else {
                strlcat(buf, c!(" "), len);
            }
            strlcat(buf, s, len);

            free(s as _);
        }
        buf
    }
}

pub unsafe fn cmd_get_entry(cmd: *mut cmd) -> &'static cmd_entry {
    unsafe { (*cmd).entry }
}

pub unsafe fn cmd_get_args(cmd: *mut cmd) -> *mut args {
    unsafe { (*cmd).args }
}

pub unsafe fn cmd_get_group(cmd: *mut cmd) -> c_uint {
    unsafe { (*cmd).group }
}

pub unsafe fn cmd_get_source(cmd: *mut cmd, file: *mut *const u8, line: &AtomicU32) {
    unsafe {
        if !file.is_null() {
            *file = (*cmd).file;
        }
        line.store((*cmd).line, std::sync::atomic::Ordering::SeqCst);
    }
}

pub unsafe fn cmd_get_alias(name: *const u8) -> *mut u8 {
    unsafe {
        let o = options_get_only(GLOBAL_OPTIONS, c!("command-alias"));
        if o.is_null() {
            return null_mut();
        }
        let wanted = strlen(name);

        let mut a = options_array_first(o);
        while !a.is_null() {
            let ov = options_array_item_value(a);

            let equals = strchr((*ov).string, b'=' as i32);
            if !equals.is_null() {
                let n = equals.addr() - (*ov).string.addr();
                if n == wanted && strncmp(name, (*ov).string, n) == 0 {
                    return xstrdup(equals.add(1)).as_ptr();
                }
            }

            a = options_array_next(a);
        }
        null_mut()
    }
}

pub unsafe fn cmd_find(name: *const u8) -> Result<&'static cmd_entry, *mut u8> {
    let mut loop_: *mut *mut cmd_entry;
    let mut entry: *mut cmd_entry;
    let mut found = None;

    let mut ambiguous: i32 = 0;
    type s_buf = [u8; 8192];
    let mut s: s_buf = [0; 8192];

    unsafe {
        'ambiguous: {
            for entry in CMD_TABLE {
                if !entry.alias.is_null() && strcmp(entry.alias.as_ptr(), name) == 0 {
                    ambiguous = 0;
                    found = Some(entry);
                    break;
                }

                if strncmp(entry.name.as_ptr(), name, strlen(name)) != 0 {
                    continue;
                }
                if found.is_some() {
                    ambiguous = 1;
                }
                found = Some(entry);

                if strcmp(entry.name.as_ptr(), name) == 0 {
                    break;
                }
            }
            if ambiguous != 0 {
                break 'ambiguous;
            }

            return match found {
                Some(value) => Ok(value),
                None => Err(format_nul!("unknown command: {}", _s(name))),
            };
        }

        // ambiguous:
        s[0] = b'\0';
        for entry in CMD_TABLE {
            if strncmp(entry.name.as_ptr(), name, strlen(name)) != 0 {
                continue;
            }
            if strlcat(&raw mut s as _, entry.name.as_ptr(), size_of::<s_buf>())
                >= size_of::<s_buf>()
            {
                break;
            }
            if strlcat(&raw mut s as _, c!(", "), size_of::<s_buf>()) >= size_of::<s_buf>() {
                break;
            }
        }
        s[strlen(&raw mut s as _) - 2] = b'\0';

        Err(format_nul!(
            "ambiguous command: {}, could be: {}",
            _s(name),
            _s((&raw const s).cast::<u8>()),
        ))
    }
}

pub unsafe fn cmd_parse(
    values: *mut args_value,
    count: c_uint,
    file: Option<&str>,
    line: c_uint,
) -> Result<*mut cmd, *mut u8> {
    unsafe {
        let mut error: *mut u8 = null_mut();

        if count == 0 || (*values).type_ != args_type::ARGS_STRING {
            return Err(format_nul!("no command"));
        }
        let entry = cmd_find((*values).union_.string)?;

        let args = args_parse(&entry.args, values, count, &raw mut error);
        if args.is_null() && error.is_null() {
            let cause = format_nul!(
                "usage: {} {}",
                _s(entry.name.as_ptr()),
                _s(entry.usage.as_ptr())
            );
            return Err(cause);
        }
        if args.is_null() {
            let cause = format_nul!("command {}: {}", _s(entry.name.as_ptr()), _s(error));
            free(error as _);
            return Err(cause);
        }

        let cmd: *mut cmd = Box::leak(Box::new(cmd {
            entry,
            args,
            group: 0,
            file: null_mut(),
            line: 0,
            qentry: tailq_entry {
                tqe_next: null_mut(),
                tqe_prev: null_mut(),
            },
        }));

        if let Some(file) = file {
            let mut file = file.to_string();
            file.push('\0');
            (*cmd).file = file.leak().as_mut_ptr().cast();
        }
        (*cmd).line = line;

        Ok(cmd)
    }
}

pub unsafe fn cmd_free(cmd: *mut cmd) {
    unsafe {
        free((*cmd).file as _);

        args_free((*cmd).args);
        free(cmd as _);
    }
}

pub unsafe fn cmd_copy(cmd: *mut cmd, argc: c_int, argv: *mut *mut u8) -> *mut cmd {
    unsafe {
        let new_cmd: *mut cmd = Box::leak(Box::new(cmd {
            entry: (*cmd).entry,
            args: args_copy((*cmd).args, argc, argv),
            group: 0,
            file: null_mut(),
            line: 0,
            qentry: tailq_entry {
                tqe_next: null_mut(),
                tqe_prev: null_mut(),
            },
        }));

        if !(*cmd).file.is_null() {
            (*new_cmd).file = xstrdup((*cmd).file).as_ptr();
        }
        (*new_cmd).line = (*cmd).line;

        new_cmd
    }
}

pub unsafe fn cmd_print(cmd: *mut cmd) -> *mut u8 {
    unsafe {
        let s = args_print((*cmd).args);
        let out = if *s != b'\0' {
            format_nul!("{} {}", _s((*cmd).entry.name.as_ptr()), _s(s))
        } else {
            xstrdup((*cmd).entry.name.as_ptr()).as_ptr()
        };
        free(s as _);

        out
    }
}

pub unsafe fn cmd_list_new<'a>() -> &'a mut cmd_list {
    unsafe {
        let group = CMD_LIST_NEXT_GROUP.fetch_add(1, std::sync::atomic::Ordering::SeqCst);

        let cmdlist = Box::leak(Box::new(cmd_list {
            references: 1,
            group,
            list: Box::leak(Box::new(zeroed())),
        }));

        tailq_init(cmdlist.list);
        cmdlist
    }
}

pub unsafe fn cmd_list_append(cmdlist: *mut cmd_list, cmd: *mut cmd) {
    unsafe {
        (*cmd).group = (*cmdlist).group;
        tailq_insert_tail::<_, qentry>((*cmdlist).list, cmd);
    }
}

pub unsafe fn cmd_list_append_all(cmdlist: *mut cmd_list, from: *mut cmd_list) {
    unsafe {
        for cmd in tailq_foreach::<_, qentry>((*from).list).map(NonNull::as_ptr) {
            (*cmd).group = (*cmdlist).group;
        }
        tailq_concat::<_, qentry>((*cmdlist).list, (*from).list);
    }
}

pub unsafe fn cmd_list_move(cmdlist: *mut cmd_list, from: *mut cmd_list) {
    unsafe {
        tailq_concat::<_, qentry>((*cmdlist).list, (*from).list);
        (*cmdlist).group = CMD_LIST_NEXT_GROUP.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    }
}

pub unsafe fn cmd_list_free(cmdlist: *mut cmd_list) {
    unsafe {
        (*cmdlist).references -= 1;
        if (*cmdlist).references != 0 {
            return;
        }

        for cmd in tailq_foreach::<_, qentry>((*cmdlist).list).map(NonNull::as_ptr) {
            tailq_remove::<_, qentry>((*cmdlist).list, cmd);
            cmd_free(cmd);
        }
        free_((*cmdlist).list);
        free_(cmdlist);
    }
}

pub unsafe fn cmd_list_copy(
    cmdlist: &mut cmd_list,
    argc: c_int,
    argv: *mut *mut u8,
) -> *mut cmd_list {
    unsafe {
        let mut group: u32 = cmdlist.group;
        let s = cmd_list_print(cmdlist, 0);
        log_debug!("{}: {}", "cmd_list_copy", _s(s));
        free(s as _);

        let new_cmdlist = cmd_list_new();
        for cmd in tailq_foreach(cmdlist.list).map(NonNull::as_ptr) {
            if (*cmd).group != group {
                new_cmdlist.group =
                    CMD_LIST_NEXT_GROUP.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                group = (*cmd).group;
            }
            let new_cmd = cmd_copy(cmd, argc, argv);
            cmd_list_append(new_cmdlist, new_cmd);
        }

        let s = cmd_list_print(new_cmdlist, 0);
        log_debug!("{}: {}", "cmd_list_copy", _s(s));
        free(s as _);

        new_cmdlist
    }
}

pub fn cmd_list_print(cmdlist: &mut cmd_list, escaped: c_int) -> *mut u8 {
    unsafe {
        let mut len = 1;
        let mut buf: *mut u8 = xcalloc(1, len).cast().as_ptr();

        for cmd in tailq_foreach::<_, qentry>(cmdlist.list).map(NonNull::as_ptr) {
            let this = cmd_print(cmd);

            len += strlen(this) + 6;
            buf = xrealloc_(buf, len).as_ptr();

            strlcat(buf, this, len);

            let next = tailq_next::<_, _, qentry>(cmd);
            if !next.is_null() {
                if (*cmd).group != (*next).group {
                    if escaped != 0 {
                        strlcat(buf, c!(" \\;\\; "), len);
                    } else {
                        strlcat(buf, c!(" ;; "), len);
                    }
                } else {
                    #[allow(clippy::collapsible_else_if)]
                    if escaped != 0 {
                        strlcat(buf, c!(" \\; "), len);
                    } else {
                        strlcat(buf, c!(" ; "), len);
                    }
                }
            }

            free_(this);
        }

        buf
    }
}

pub unsafe fn cmd_list_first(cmdlist: *mut cmd_list) -> *mut cmd {
    unsafe { tailq_first((*cmdlist).list) }
}

pub unsafe fn cmd_list_next(cmd: *mut cmd) -> *mut cmd {
    unsafe { tailq_next::<_, _, qentry>(cmd) }
}

pub unsafe fn cmd_list_all_have(cmdlist: *mut cmd_list, flag: cmd_flag) -> bool {
    unsafe {
        tailq_foreach((*cmdlist).list).all(|cmd| (*cmd.as_ptr()).entry.flags.intersects(flag))
    }
}

pub unsafe fn cmd_list_any_have(cmdlist: *mut cmd_list, flag: cmd_flag) -> bool {
    unsafe {
        tailq_foreach((*cmdlist).list).any(|cmd| (*cmd.as_ptr()).entry.flags.intersects(flag))
    }
}

pub unsafe fn cmd_mouse_at(
    wp: *mut window_pane,
    m: *mut mouse_event,
    xp: *mut c_uint,
    yp: *mut c_uint,
    last: c_int,
) -> c_int {
    unsafe {
        let x: u32;
        let mut y: u32;

        if last != 0 {
            x = (*m).lx + (*m).ox;
            y = (*m).ly + (*m).oy;
        } else {
            x = (*m).x + (*m).ox;
            y = (*m).y + (*m).oy;
        }
        log_debug!(
            "{}: x={}, y={}{}",
            "cmd_mouse_at",
            x,
            y,
            if last != 0 { " (last)" } else { "" }
        );

        if (*m).statusat == 0 && y >= (*m).statuslines {
            y -= (*m).statuslines;
        }

        if x < (*wp).xoff || x >= (*wp).xoff + (*wp).sx {
            return -1;
        }

        if y < (*wp).yoff || y >= (*wp).yoff + (*wp).sy {
            return -1;
        }

        if !xp.is_null() {
            *xp = x - (*wp).xoff;
        }
        if !yp.is_null() {
            *yp = y - (*wp).yoff;
        }
        0
    }
}

pub unsafe fn cmd_mouse_window(
    m: *mut mouse_event,
    sp: *mut *mut session,
) -> Option<NonNull<winlink>> {
    unsafe {
        let mut s: *mut session = null_mut();

        if (*m).valid == 0 {
            return None;
        }
        if (*m).s == -1
            || ({
                s = transmute_ptr(session_find_by_id((*m).s as u32));
                s.is_null()
            })
        {
            return None;
        }
        let wl = if (*m).w == -1 {
            NonNull::new((*s).curw)
        } else {
            let w = window_find_by_id((*m).w as u32);
            if w.is_null() {
                return None;
            }
            winlink_find_by_window(&raw mut (*s).windows, w)
        };
        if !sp.is_null() {
            *sp = s;
        }
        wl
    }
}

pub unsafe fn cmd_mouse_pane(
    m: *mut mouse_event,
    sp: *mut *mut session,
    wlp: *mut *mut winlink,
) -> Option<NonNull<window_pane>> {
    unsafe {
        let wl = cmd_mouse_window(m, sp)?;
        let mut wp = None;

        if (*m).wp == -1 {
            wp = NonNull::new((*(*wl.as_ptr()).window).active);
        } else {
            let wp = NonNull::new(window_pane_find_by_id((*m).wp as u32))?;
            if !window_has_pane((*wl.as_ptr()).window, wp.as_ptr()) {
                return None;
            }
        }

        if !wlp.is_null() {
            *wlp = wl.as_ptr();
        }
        wp
    }
}

pub unsafe fn cmd_template_replace(template: *const u8, s: *const u8, idx: c_int) -> *mut u8 {
    unsafe {
        let quote = c!("\"\\$;~");

        if strchr(template, b'%' as i32).is_null() {
            return xstrdup(template).cast().as_ptr();
        }

        let mut buf: *mut u8 = xmalloc(1).cast().as_ptr();
        *buf = b'\0';
        let mut len = 0;
        let mut replaced = 0;

        let mut ptr = template;
        while *ptr != b'\0' {
            let ch = *ptr;
            ptr = ptr.add(1);
            if matches!(ch as c_uchar, b'%') {
                if *ptr < b'1' || *ptr > b'9' || *ptr as i32 - b'0' as i32 != idx {
                    if *ptr != b'%' || replaced != 0 {
                        break;
                    }
                    replaced = 1;
                }
                ptr = ptr.add(1);

                let quoted = *ptr == b'%';
                if !quoted {
                    ptr = ptr.add(1);
                }

                buf = xrealloc_(buf, len + (strlen(s) * 3) + 1).as_ptr();
                let mut cp = s;
                while *cp != b'\0' {
                    if quoted && !strchr(quote, *cp as i32).is_null() {
                        *buf.add(len) = b'\\';
                        len += 1;
                    }
                    *buf.add(len) = *cp;
                    len += 1;
                    cp = cp.add(1);
                }
                *buf.add(len) = b'\0';
                continue;
            }
            buf = xrealloc_(buf, len + 2).as_ptr();
            *buf.add(len) = ch;
            len += 1;
            *buf.add(len) = b'\0';
        }

        buf
    }
}

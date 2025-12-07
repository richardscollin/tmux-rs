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
use crate::*;

// This file has a tables with all the server, session and window
// options. These tables are the master copy of the options with their real
//(user-visible) types, range limits and default values. At start these are
// copied into the runtime global options trees (which only has number and
// string types). These tables are then used to look up the real type when the
// user sets an option or its value needs to be shown.

// Choice option type lists.
static OPTIONS_TABLE_MODE_KEYS_LIST: [&str; 2] = ["emacs", "vi"];
static OPTIONS_TABLE_CLOCK_MODE_STYLE_LIST: [&str; 2] = ["12", "24"];
static OPTIONS_TABLE_STATUS_LIST: [&str; 6] = ["off", "on", "2", "3", "4", "5"];
static OPTIONS_TABLE_MESSAGE_LINE_LIST: [&str; 5] = ["0", "1", "2", "3", "4"];
static OPTIONS_TABLE_STATUS_KEYS_LIST: [&str; 2] = ["emacs", "vi"];
static OPTIONS_TABLE_STATUS_JUSTIFY_LIST: [&str; 4] =
    ["left", "centre", "right", "absolute-centre"];
static OPTIONS_TABLE_STATUS_POSITION_LIST: [&str; 2] = ["top", "bottom"];
static OPTIONS_TABLE_BELL_ACTION_LIST: [&str; 4] = ["none", "any", "current", "other"];
static OPTIONS_TABLE_VISUAL_BELL_LIST: [&str; 3] = ["off", "on", "both"];
static OPTIONS_TABLE_CURSOR_STYLE_LIST: [&str; 7] = [
    "default",
    "blinking-block",
    "block",
    "blinking-underline",
    "underline",
    "blinking-bar",
    "bar",
];
static OPTIONS_TABLE_PANE_STATUS_LIST: [&str; 3] = ["off", "top", "bottom"];
static OPTIONS_TABLE_PANE_BORDER_INDICATORS_LIST: [&str; 4] = ["off", "colour", "arrows", "both"];
static OPTIONS_TABLE_PANE_BORDER_LINES_LIST: [&str; 5] =
    ["single", "double", "heavy", "simple", "number"];
static OPTIONS_TABLE_POPUP_BORDER_LINES_LIST: [&str; 7] = [
    "single", "double", "heavy", "simple", "rounded", "padded", "none",
];
static OPTIONS_TABLE_SET_CLIPBOARD_LIST: [&str; 3] = ["off", "external", "on"];
static OPTIONS_TABLE_WINDOW_SIZE_LIST: [&str; 4] = ["largest", "smallest", "manual", "latest"];
static OPTIONS_TABLE_REMAIN_ON_EXIT_LIST: [&str; 3] = ["off", "on", "failed"];
static OPTIONS_TABLE_DESTROY_UNATTACHED_LIST: [&str; 4] = ["off", "on", "keep-last", "keep-group"];
static OPTIONS_TABLE_DETACH_ON_DESTROY_LIST: [&str; 5] =
    ["off", "on", "no-detached", "previous", "next"];
static OPTIONS_TABLE_EXTENDED_KEYS_LIST: [&str; 3] = ["off", "on", "always"];
static OPTIONS_TABLE_EXTENDED_KEYS_FORMAT_LIST: [&str; 2] = ["csi-u", "xterm"];
static OPTIONS_TABLE_ALLOW_PASSTHROUGH_LIST: [&str; 3] = ["off", "on", "all"];

#[rustfmt::skip]
/// Map of name conversions.
pub static OPTIONS_OTHER_NAMES: [options_name_map; 5] = [
    options_name_map::new("display-panes-color", "display-panes-colour"),
    options_name_map::new("display-panes-active-color", "display-panes-active-colour"),
    options_name_map::new("clock-mode-color", "clock-mode-colour"),
    options_name_map::new("cursor-color", "cursor-colour"),
    options_name_map::new("pane-colors", "pane-colours"),
];

#[rustfmt::skip]
/// Map of name conversions.
pub static OPTIONS_OTHER_NAMES_STR: [options_name_map_str; 5] = [
    options_name_map_str::new("display-panes-color", "display-panes-colour"),
    options_name_map_str::new("display-panes-active-color", "display-panes-active-colour"),
    options_name_map_str::new("clock-mode-color", "clock-mode-colour"),
    options_name_map_str::new("cursor-color", "cursor-colour"),
    options_name_map_str::new("pane-colors", "pane-colours"),
];

#[expect(clippy::disallowed_methods)]
/// Status line format.
pub const OPTIONS_TABLE_STATUS_FORMAT1: *const u8 = concat!(
    "#[align=left range=left #{E:status-left-style}]",
    "#[push-default]",
    "#{T;=/#{status-left-length}:status-left}",
    "#[pop-default]",
    "#[norange default]",
    "#[list=on align=#{status-justify}]",
    "#[list=left-marker]<#[list=right-marker]>#[list=on]",
    "#{W:",
    "#[range=window|#{window_index} ",
    "#{E:window-status-style}",
    "#{?#{&&:#{window_last_flag},",
    "#{!=:#{E:window-status-last-style},default}}, ",
    "#{E:window-status-last-style},",
    "}",
    "#{?#{&&:#{window_bell_flag},",
    "#{!=:#{E:window-status-bell-style},default}}, ",
    "#{E:window-status-bell-style},",
    "#{?#{&&:#{||:#{window_activity_flag},",
    "#{window_silence_flag}},",
    "#{!=:",
    "#{E:window-status-activity-style},",
    "default}}, ",
    "#{E:window-status-activity-style},",
    "}",
    "}",
    "]",
    "#[push-default]",
    "#{T:window-status-format}",
    "#[pop-default]",
    "#[norange default]",
    "#{?window_end_flag,,#{window-status-separator}}",
    ",",
    "#[range=window|#{window_index} list=focus ",
    "#{?#{!=:#{E:window-status-current-style},default},",
    "#{E:window-status-current-style},",
    "#{E:window-status-style}",
    "}",
    "#{?#{&&:#{window_last_flag},",
    "#{!=:#{E:window-status-last-style},default}}, ",
    "#{E:window-status-last-style},",
    "}",
    "#{?#{&&:#{window_bell_flag},",
    "#{!=:#{E:window-status-bell-style},default}}, ",
    "#{E:window-status-bell-style},",
    "#{?#{&&:#{||:#{window_activity_flag},",
    "#{window_silence_flag}},",
    "#{!=:",
    "#{E:window-status-activity-style},",
    "default}}, ",
    "#{E:window-status-activity-style},",
    "}",
    "}",
    "]",
    "#[push-default]",
    "#{T:window-status-current-format}",
    "#[pop-default]",
    "#[norange list=on default]",
    "#{?window_end_flag,,#{window-status-separator}}",
    "}",
    "#[nolist align=right range=right #{E:status-right-style}]",
    "#[push-default]",
    "#{T;=/#{status-right-length}:status-right}",
    "#[pop-default]",
    "#[norange default]\0"
)
.as_ptr()
.cast();

#[expect(clippy::disallowed_methods)]
pub const OPTIONS_TABLE_STATUS_FORMAT2: *const u8 = concat!(
    "#[align=centre]#{P:#{?pane_active,#[reverse],}",
    "#{pane_index}[#{pane_width}x#{pane_height}]#[default] }\0"
)
.as_ptr()
.cast();

pub static mut OPTIONS_TABLE_STATUS_FORMAT_DEFAULT: [*const u8; 3] = [
    OPTIONS_TABLE_STATUS_FORMAT1,
    OPTIONS_TABLE_STATUS_FORMAT2,
    null(),
];

// Helpers for hook options.
macro_rules! options_table_hook {
    ($hook_name:expr, $default_value:expr) => {
        options_table_entry {
            name: $hook_name,
            type_: options_table_type::OPTIONS_TABLE_COMMAND,
            scope: OPTIONS_TABLE_SESSION,
            flags: OPTIONS_TABLE_IS_ARRAY | OPTIONS_TABLE_IS_HOOK,
            default_str: Some($default_value),
            separator: c!(""),
            ..options_table_entry::const_default()
        }
    };
}

macro_rules! options_table_pane_hook {
    ($hook_name:expr, $default_value:expr) => {
        options_table_entry {
            name: $hook_name,
            type_: options_table_type::OPTIONS_TABLE_COMMAND,
            scope: OPTIONS_TABLE_WINDOW | OPTIONS_TABLE_PANE,
            flags: OPTIONS_TABLE_IS_ARRAY | OPTIONS_TABLE_IS_HOOK,
            default_str: Some($default_value),
            separator: c!(""),
            ..options_table_entry::const_default()
        }
    };
}

macro_rules! options_table_window_hook {
    ($hook_name:expr, $default_value:expr) => {
        options_table_entry {
            name: $hook_name,
            type_: options_table_type::OPTIONS_TABLE_COMMAND,
            scope: OPTIONS_TABLE_WINDOW,
            flags: OPTIONS_TABLE_IS_ARRAY | OPTIONS_TABLE_IS_HOOK,
            default_str: Some($default_value),
            separator: c!(""),
            ..options_table_entry::const_default()
        }
    };
}

pub static OPTIONS_TABLE: [options_table_entry; 190] = [
    options_table_entry {
        name: "backspace",
        type_: options_table_type::OPTIONS_TABLE_KEY,
        scope: OPTIONS_TABLE_SERVER,
        default_num: b'\x7f' as i64,
        text: c!("The key to send for backspace."),
        choices: &[],
        ..options_table_entry::const_default()
    },
    options_table_entry {
        name: "buffer-limit",
        type_: options_table_type::OPTIONS_TABLE_NUMBER,
        scope: OPTIONS_TABLE_SERVER,
        minimum: 1,
        maximum: i32::MAX as u32,
        default_num: 50,
        text: c!(
            "The maximum number of automatic buffers. When this is reached, the oldest buffer is deleted."
        ),
        choices: &[],
        ..options_table_entry::const_default()
    },
    options_table_entry {
        name: "command-alias",
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_SERVER,
        flags: OPTIONS_TABLE_IS_ARRAY,
        default_str: Some(
            "split-pane=split-window,splitp=split-window,server-info=show-messages -JT,info=show-messages -JT,choose-window=choose-tree -w,choose-session=choose-tree -s",
        ),
        separator: c!(","),
        text: c!(
            "Array of command aliases. Each entry is an alias and a command separated by '='."
        ),
        ..options_table_entry::const_default()
    },
    options_table_entry {
        name: "copy-command",
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_SERVER,
        default_str: Some(""),
        text: c!("Shell command run when text is copied. If empty, no command is run."),
        ..options_table_entry::const_default()
    },
    options_table_entry {
        name: "cursor-colour",
        type_: options_table_type::OPTIONS_TABLE_COLOUR,
        scope: OPTIONS_TABLE_WINDOW | OPTIONS_TABLE_PANE,
        default_num: -1,
        text: c!("Colour of the cursor."),
        ..options_table_entry::const_default()
    },
    options_table_entry {
        name: "cursor-style",
        type_: options_table_type::OPTIONS_TABLE_CHOICE,
        scope: OPTIONS_TABLE_WINDOW | OPTIONS_TABLE_PANE,
        choices: &OPTIONS_TABLE_CURSOR_STYLE_LIST,
        default_num: 0,
        text: c!("Style of the cursor."),
        ..options_table_entry::const_default()
    },
    options_table_entry {
        name: "default-terminal",
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_SERVER,
        default_str: Some(TMUX_TERM),
        text: c!("Default for the 'TERM' environment variable."),
        ..options_table_entry::const_default()
    },
    options_table_entry {
        name: "editor",
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_SERVER,
        default_str: Some(_PATH_VI),
        text: c!("Editor run to edit files."),
        ..options_table_entry::const_default()
    },
    options_table_entry {
        name: "escape-time",
        type_: options_table_type::OPTIONS_TABLE_NUMBER,
        scope: OPTIONS_TABLE_SERVER,
        minimum: 0,
        maximum: i32::MAX as u32,
        default_num: 10,
        unit: c!("milliseconds"),
        text: c!("Time to wait before assuming a key is Escape."),
        ..options_table_entry::const_default()
    },
    options_table_entry {
        name: "exit-empty",
        type_: options_table_type::OPTIONS_TABLE_FLAG,
        scope: OPTIONS_TABLE_SERVER,
        default_num: 1,
        text: c!("Whether the server should exit if there are no sessions."),
        ..options_table_entry::const_default()
    },
    options_table_entry {
        name: "exit-unattached",
        type_: options_table_type::OPTIONS_TABLE_FLAG,
        scope: OPTIONS_TABLE_SERVER,
        default_num: 0,
        text: c!("Whether the server should exit if there are no attached clients."),
        ..options_table_entry::const_default()
    },
    options_table_entry {
        name: "extended-keys",
        type_: options_table_type::OPTIONS_TABLE_CHOICE,
        scope: OPTIONS_TABLE_SERVER,
        choices: &OPTIONS_TABLE_EXTENDED_KEYS_LIST,
        default_num: 0,
        text: c!("Whether to request extended key sequences from terminals that support it."),
        ..options_table_entry::const_default()
    },
    options_table_entry {
        name: "extended-keys-format",
        type_: options_table_type::OPTIONS_TABLE_CHOICE,
        scope: OPTIONS_TABLE_SERVER,
        choices: &OPTIONS_TABLE_EXTENDED_KEYS_FORMAT_LIST,
        default_num: 1,
        text: c!("The format of emitted extended key sequences."),
        ..options_table_entry::const_default()
    },
    options_table_entry {
        name: "focus-events",
        type_: options_table_type::OPTIONS_TABLE_FLAG,
        scope: OPTIONS_TABLE_SERVER,
        default_num: 0,
        text: c!("Whether to send focus events to applications."),
        ..options_table_entry::const_default()
    },
    options_table_entry {
        name: "history-file",
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_SERVER,
        default_str: Some(""),
        text: c!(
            "Location of the command prompt history file. Empty does not write a history file."
        ),
        ..options_table_entry::const_default()
    },
    options_table_entry {
        name: "menu-style",
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_WINDOW,
        flags: OPTIONS_TABLE_IS_STYLE,
        default_str: Some("default"),
        separator: c!(","),
        text: c!("Default style of menu."),
        ..options_table_entry::const_default()
    },
    options_table_entry {
        name: "menu-selected-style",
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_WINDOW,
        flags: OPTIONS_TABLE_IS_STYLE,
        default_str: Some("bg=yellow,fg=black"),
        separator: c!(","),
        text: c!("Default style of selected menu item."),
        ..options_table_entry::const_default()
    },
    options_table_entry {
        name: "menu-border-style",
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_WINDOW,
        default_str: Some("default"),
        flags: OPTIONS_TABLE_IS_STYLE,
        separator: c!(","),
        text: c!("Default style of menu borders."),
        ..options_table_entry::const_default()
    },
    options_table_entry {
        name: "menu-border-lines",
        type_: options_table_type::OPTIONS_TABLE_CHOICE,
        scope: OPTIONS_TABLE_WINDOW,
        choices: &OPTIONS_TABLE_POPUP_BORDER_LINES_LIST,
        default_num: box_lines::BOX_LINES_SINGLE as i64,
        text: c!(
            "Type of characters used to draw menu border lines. Some of these are only supported on terminals with UTF-8 support."
        ),
        ..options_table_entry::const_default()
    },
    options_table_entry {
        name: "message-limit",
        type_: options_table_type::OPTIONS_TABLE_NUMBER,
        scope: OPTIONS_TABLE_SERVER,
        minimum: 0,
        maximum: i32::MAX as u32,
        default_num: 1000,
        text: c!("Maximum number of server messages to keep."),
        ..options_table_entry::const_default()
    },
    options_table_entry {
        name: "prefix-timeout",
        type_: options_table_type::OPTIONS_TABLE_NUMBER,
        scope: OPTIONS_TABLE_SERVER,
        minimum: 0,
        maximum: i32::MAX as u32,
        default_num: 0,
        unit: c!("milliseconds"),
        text: c!(
            "The timeout for the prefix key if no subsequent key is pressed. Zero means disabled."
        ),
        ..options_table_entry::const_default()
    },
    options_table_entry {
        name: "prompt-history-limit",
        type_: options_table_type::OPTIONS_TABLE_NUMBER,
        scope: OPTIONS_TABLE_SERVER,
        minimum: 0,
        maximum: i32::MAX as u32,
        default_num: 100,
        text: c!("Maximum number of commands to keep in history."),
        ..options_table_entry::const_default()
    },
    options_table_entry {
        name: "set-clipboard",
        type_: options_table_type::OPTIONS_TABLE_CHOICE,
        scope: OPTIONS_TABLE_SERVER,
        choices: &OPTIONS_TABLE_SET_CLIPBOARD_LIST,
        default_num: 1,
        text: c!(
            "Whether to attempt to set the system clipboard ('on' or 'external') and whether to allow applications to create paste buffers with an escape sequence ('on' only)."
        ),
        ..options_table_entry::const_default()
    },
    options_table_entry {
        name: "terminal-overrides",
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_SERVER,
        flags: OPTIONS_TABLE_IS_ARRAY,
        default_str: Some("linux*:AX@"),
        separator: c!(","),
        text: c!("List of terminal capabilities overrides."),
        ..options_table_entry::const_default()
    },
    options_table_entry {
        name: "terminal-features",
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_SERVER,
        flags: OPTIONS_TABLE_IS_ARRAY,
        default_str: Some(
            "xterm*:clipboard:ccolour:cstyle:focus:title,screen*:title,rxvt*:ignorefkeys",
        ),
        separator: c!(","),
        text: c!("List of terminal features, used if they cannot be automatically detected."),
        ..options_table_entry::const_default()
    },
    options_table_entry {
        name: "user-keys",
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_SERVER,
        flags: OPTIONS_TABLE_IS_ARRAY,
        default_str: Some(""),
        separator: c!(","),
        text: c!(
            "User key assignments. Each sequence in the list is translated into a key: 'User0', 'User1' and so on."
        ),
        ..options_table_entry::const_default()
    },
    // Session options.
    options_table_entry {
        name: "activity-action",
        type_: options_table_type::OPTIONS_TABLE_CHOICE,
        scope: OPTIONS_TABLE_SESSION,
        choices: &OPTIONS_TABLE_BELL_ACTION_LIST,
        default_num: alert_option::ALERT_OTHER as i64,
        text: c!("Action to take on an activity alert."),
        ..options_table_entry::const_default()
    },
    options_table_entry {
        name: "assume-paste-time",
        type_: options_table_type::OPTIONS_TABLE_NUMBER,
        scope: OPTIONS_TABLE_SESSION,
        minimum: 0,
        maximum: i32::MAX as u32,
        default_num: 1,
        unit: c!("milliseconds"),
        text: c!("Maximum time between input to assume it is pasting rather than typing."),
        ..options_table_entry::const_default()
    },
    options_table_entry {
        name: "base-index",
        type_: options_table_type::OPTIONS_TABLE_NUMBER,
        scope: OPTIONS_TABLE_SESSION,
        minimum: 0,
        maximum: i32::MAX as u32,
        default_num: 0,
        text: c!("Default index of the first window in each session."),
        ..options_table_entry::const_default()
    },
    options_table_entry {
        name: "bell-action",
        type_: options_table_type::OPTIONS_TABLE_CHOICE,
        scope: OPTIONS_TABLE_SESSION,
        choices: &OPTIONS_TABLE_BELL_ACTION_LIST,
        default_num: alert_option::ALERT_ANY as i64,
        text: c!("Action to take on a bell alert."),
        ..options_table_entry::const_default()
    },
    options_table_entry {
        name: "default-command",
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_SESSION,
        default_str: Some(""),
        text: c!("Default command to run in new panes. If empty, a shell is started."),
        ..options_table_entry::const_default()
    },
    options_table_entry {
        name: "default-shell",
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_SESSION,
        default_str: Some(_PATH_BSHELL_STR),
        text: c!("Location of default shell."),
        ..options_table_entry::const_default()
    },
    options_table_entry {
        name: "default-size",
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_SESSION,
        pattern: c!("[0-9]*x[0-9]*"),
        default_str: Some("80x24"),
        text: c!("Initial size of new sessions."),
        ..options_table_entry::const_default()
    },
    options_table_entry {
        name: "destroy-unattached",
        type_: options_table_type::OPTIONS_TABLE_CHOICE,
        scope: OPTIONS_TABLE_SESSION,
        choices: &OPTIONS_TABLE_DESTROY_UNATTACHED_LIST,
        default_num: 0,
        text: c!(
            "Whether to destroy sessions when they have no attached clients, or keep the last session whether in the group."
        ),
        ..options_table_entry::const_default()
    },
    options_table_entry {
        name: "detach-on-destroy",
        type_: options_table_type::OPTIONS_TABLE_CHOICE,
        scope: OPTIONS_TABLE_SESSION,
        choices: &OPTIONS_TABLE_DETACH_ON_DESTROY_LIST,
        default_num: 1,
        text: c!(
            "Whether to detach when a session is destroyed, or switch the client to another session if any exist."
        ),
        ..options_table_entry::const_default()
    },
    options_table_entry {
        name: "display-panes-active-colour",
        type_: options_table_type::OPTIONS_TABLE_COLOUR,
        scope: OPTIONS_TABLE_SESSION,
        default_num: 1,
        text: c!("Colour of the active pane for 'display-panes'."),
        ..options_table_entry::const_default()
    },
    options_table_entry {
        name: "display-panes-colour",
        type_: options_table_type::OPTIONS_TABLE_COLOUR,
        scope: OPTIONS_TABLE_SESSION,
        default_num: 4,
        text: c!("Colour of not active panes for 'display-panes'."),
        ..options_table_entry::const_default()
    },
    options_table_entry {
        name: "display-panes-time",
        type_: options_table_type::OPTIONS_TABLE_NUMBER,
        scope: OPTIONS_TABLE_SESSION,
        minimum: 1,
        maximum: i32::MAX as u32,
        default_num: 1000,
        unit: c!("milliseconds"),
        text: c!("Time for which 'display-panes' should show pane numbers."),
        ..options_table_entry::const_default()
    },
    options_table_entry {
        name: "display-time",
        type_: options_table_type::OPTIONS_TABLE_NUMBER,
        scope: OPTIONS_TABLE_SESSION,
        minimum: 0,
        maximum: i32::MAX as u32,
        default_num: 750,
        unit: c!("milliseconds"),
        text: c!("Time for which status line messages should appear."),
        ..options_table_entry::const_default()
    },
    options_table_entry {
        name: "history-limit",
        type_: options_table_type::OPTIONS_TABLE_NUMBER,
        scope: OPTIONS_TABLE_SESSION,
        minimum: 0,
        maximum: i32::MAX as u32,
        default_num: 2000,
        unit: c!("lines"),
        text: c!(
            "Maximum number of lines to keep in the history for each pane. If changed, the new value applies only to new panes."
        ),
        ..options_table_entry::const_default()
    },
    options_table_entry {
        name: "key-table",
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_SESSION,
        default_str: Some("root"),
        text: c!("Default key table. Key presses are first looked up in this table."),
        ..options_table_entry::const_default()
    },
    options_table_entry {
        name: "lock-after-time",
        type_: options_table_type::OPTIONS_TABLE_NUMBER,
        scope: OPTIONS_TABLE_SESSION,
        minimum: 0,
        maximum: i32::MAX as u32,
        default_num: 0,
        unit: c!("seconds"),
        text: c!("Time after which a client is locked if not used."),
        ..options_table_entry::const_default()
    },
    options_table_entry {
        name: "lock-command",
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_SESSION,
        default_str: Some(TMUX_LOCK_CMD),
        text: c!("Shell command to run to lock a client."),
        ..options_table_entry::const_default()
    },
    options_table_entry {
        name: "message-command-style",
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_SESSION,
        default_str: Some("bg=black,fg=yellow"),
        flags: OPTIONS_TABLE_IS_STYLE,
        separator: c!(","),
        text: c!(
            "Style of the command prompt when in command mode, if 'mode-keys' is set to 'vi'."
        ),
        ..options_table_entry::const_default()
    },
    options_table_entry {
        name: "message-line",
        type_: options_table_type::OPTIONS_TABLE_CHOICE,
        scope: OPTIONS_TABLE_SESSION,
        choices: &OPTIONS_TABLE_MESSAGE_LINE_LIST,
        default_num: 0,
        text: c!("Position (line) of messages and the command prompt."),
        ..options_table_entry::const_default()
    },
    options_table_entry {
        name: "message-style",
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_SESSION,
        default_str: Some("bg=yellow,fg=black"),
        flags: OPTIONS_TABLE_IS_STYLE,
        separator: c!(","),
        text: c!("Style of messages and the command prompt."),
        ..options_table_entry::const_default()
    },
    options_table_entry {
        name: "mouse",
        type_: options_table_type::OPTIONS_TABLE_FLAG,
        scope: OPTIONS_TABLE_SESSION,
        default_num: 0,
        text: c!(
            "Whether the mouse is recognised and mouse key bindings are executed. Applications inside panes can use the mouse even when 'off'."
        ),
        ..options_table_entry::const_default()
    },
    options_table_entry {
        name: "prefix",
        type_: options_table_type::OPTIONS_TABLE_KEY,
        scope: OPTIONS_TABLE_SESSION,
        default_num: b'b' as i64 | KEYC_CTRL as i64,
        text: c!("The prefix key."),
        ..options_table_entry::const_default()
    },
    options_table_entry {
        name: "prefix2",
        type_: options_table_type::OPTIONS_TABLE_KEY,
        scope: OPTIONS_TABLE_SESSION,
        default_num: KEYC_NONE as i64,
        text: c!("A second prefix key."),
        ..options_table_entry::const_default()
    },
    options_table_entry {
        name: "renumber-windows",
        type_: options_table_type::OPTIONS_TABLE_FLAG,
        scope: OPTIONS_TABLE_SESSION,
        default_num: 0,
        text: c!("Whether windows are automatically renumbered rather than leaving gaps."),
        ..options_table_entry::const_default()
    },
    options_table_entry {
        name: "repeat-time",
        type_: options_table_type::OPTIONS_TABLE_NUMBER,
        scope: OPTIONS_TABLE_SESSION,
        minimum: 0,
        maximum: i16::MAX as u32,
        default_num: 500,
        unit: c!("milliseconds"),
        text: c!("Time to wait for a key binding to repeat, if it is bound with the '-r' flag."),
        ..options_table_entry::const_default()
    },
    options_table_entry {
        name: "set-titles",
        type_: options_table_type::OPTIONS_TABLE_FLAG,
        scope: OPTIONS_TABLE_SESSION,
        default_num: 0,
        text: c!("Whether to set the terminal title, if supported."),
        ..options_table_entry::const_default()
    },
    options_table_entry {
        name: "set-titles-string",
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_SESSION,
        default_str: Some("#S:#I:#W - \"#T\" #{session_alerts}"),
        text: c!("Format of the terminal title to set."),
        ..options_table_entry::const_default()
    },
    options_table_entry {
        name: "silence-action",
        type_: options_table_type::OPTIONS_TABLE_CHOICE,
        scope: OPTIONS_TABLE_SESSION,
        choices: &OPTIONS_TABLE_BELL_ACTION_LIST,
        default_num: alert_option::ALERT_OTHER as i64,
        text: c!("Action to take on a silence alert."),
        ..options_table_entry::const_default()
    },
    options_table_entry {
        name: "status",
        type_: options_table_type::OPTIONS_TABLE_CHOICE,
        scope: OPTIONS_TABLE_SESSION,
        choices: &OPTIONS_TABLE_STATUS_LIST,
        default_num: 1,
        text: c!("Number of lines in the status line."),
        ..options_table_entry::const_default()
    },
    options_table_entry {
        name: "status-bg",
        type_: options_table_type::OPTIONS_TABLE_COLOUR,
        scope: OPTIONS_TABLE_SESSION,
        default_num: 8,
        text: c!(
            "Background colour of the status line. This option is deprecated, use 'status-style' instead."
        ),
        ..options_table_entry::const_default()
    },
    options_table_entry {
        name: "status-fg",
        type_: options_table_type::OPTIONS_TABLE_COLOUR,
        scope: OPTIONS_TABLE_SESSION,
        default_num: 8,
        text: c!(
            "Foreground colour of the status line. This option is deprecated, use 'status-style' instead."
        ),
        ..options_table_entry::const_default()
    },
    options_table_entry {
        name: "status-format",
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_SESSION,
        flags: OPTIONS_TABLE_IS_ARRAY,
        default_arr: &raw const OPTIONS_TABLE_STATUS_FORMAT_DEFAULT as *const *const u8,
        text: c!(
            "Formats for the status lines. Each array member is the format for one status line. The default status line is made up of several components which may be configured individually with other options such as 'status-left'."
        ),
        ..options_table_entry::const_default()
    },
    options_table_entry {
        name: "status-interval",
        type_: options_table_type::OPTIONS_TABLE_NUMBER,
        scope: OPTIONS_TABLE_SESSION,
        minimum: 0,
        maximum: i32::MAX as u32,
        default_num: 15,
        unit: c!("seconds"),
        text: c!("Number of seconds between status line updates."),
        ..options_table_entry::const_default()
    },
    options_table_entry {
        name: "status-justify",
        type_: options_table_type::OPTIONS_TABLE_CHOICE,
        scope: OPTIONS_TABLE_SESSION,
        choices: &OPTIONS_TABLE_STATUS_JUSTIFY_LIST,
        default_num: 0,
        text: c!("Position of the window list in the status line."),
        ..options_table_entry::const_default()
    },
    options_table_entry {
        name: "status-keys",
        type_: options_table_type::OPTIONS_TABLE_CHOICE,
        scope: OPTIONS_TABLE_SESSION,
        choices: &OPTIONS_TABLE_STATUS_KEYS_LIST,
        default_num: modekey::MODEKEY_EMACS as i64,
        text: c!("Key set to use at the command prompt."),
        ..options_table_entry::const_default()
    },
    options_table_entry {
        name: "status-left",
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_SESSION,
        default_str: Some("[#{session_name}] "),
        text: c!("Contents of the left side of the status line."),
        ..options_table_entry::const_default()
    },
    options_table_entry {
        name: "status-left-length",
        type_: options_table_type::OPTIONS_TABLE_NUMBER,
        scope: OPTIONS_TABLE_SESSION,
        minimum: 0,
        maximum: i16::MAX as u32,
        default_num: 10,
        text: c!("Maximum width of the left side of the status line."),
        ..options_table_entry::const_default()
    },
    options_table_entry {
        name: "status-left-style",
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_SESSION,
        default_str: Some("default"),
        flags: OPTIONS_TABLE_IS_STYLE,
        separator: c!(","),
        text: c!("Style of the left side of the status line."),
        ..options_table_entry::const_default()
    },
    options_table_entry {
        name: "status-position",
        type_: options_table_type::OPTIONS_TABLE_CHOICE,
        scope: OPTIONS_TABLE_SESSION,
        choices: &OPTIONS_TABLE_STATUS_POSITION_LIST,
        default_num: 1,
        text: c!("Position of the status line."),
        ..options_table_entry::const_default()
    },
    options_table_entry {
        name: "status-right",
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_SESSION,
        default_str: Some(
            "#{?window_bigger,[#{window_offset_x}#,#{window_offset_y}] ,}\"#{=21:pane_title}\" %H:%M %d-%b-%y",
        ),
        text: c!("Contents of the right side of the status line."),
        ..options_table_entry::const_default()
    },
    options_table_entry {
        name: "status-right-length",
        type_: options_table_type::OPTIONS_TABLE_NUMBER,
        scope: OPTIONS_TABLE_SESSION,
        minimum: 0,
        maximum: i16::MAX as u32,
        default_num: 40,
        text: c!("Maximum width of the right side of the status line."),
        ..options_table_entry::const_default()
    },
    options_table_entry {
        name: "status-right-style",
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_SESSION,
        default_str: Some("default"),
        flags: OPTIONS_TABLE_IS_STYLE,
        separator: c!(","),
        text: c!("Style of the right side of the status line."),
        ..options_table_entry::const_default()
    },
    options_table_entry {
        name: "status-style",
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_SESSION,
        default_str: Some("bg=green,fg=black"),
        flags: OPTIONS_TABLE_IS_STYLE,
        separator: c!(","),
        text: c!("Style of the status line."),
        ..options_table_entry::const_default()
    },
    options_table_entry {
        name: "update-environment",
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_SESSION,
        flags: OPTIONS_TABLE_IS_ARRAY,
        default_str: Some(
            "DISPLAY KRB5CCNAME SSH_ASKPASS SSH_AUTH_SOCK SSH_AGENT_PID SSH_CONNECTION WINDOWID XAUTHORITY",
        ),
        text: c!(
            "List of environment variables to update in the session environment when a client is attached."
        ),
        ..options_table_entry::const_default()
    },
    options_table_entry {
        name: "visual-activity",
        type_: options_table_type::OPTIONS_TABLE_CHOICE,
        scope: OPTIONS_TABLE_SESSION,
        choices: &OPTIONS_TABLE_VISUAL_BELL_LIST,
        default_num: visual_option::VISUAL_OFF as i64,
        text: c!(
            "How activity alerts should be shown: a message ('on'), a message and a bell ('both') or nothing ('off')."
        ),
        ..options_table_entry::const_default()
    },
    options_table_entry {
        name: "visual-bell",
        type_: options_table_type::OPTIONS_TABLE_CHOICE,
        scope: OPTIONS_TABLE_SESSION,
        choices: &OPTIONS_TABLE_VISUAL_BELL_LIST,
        default_num: visual_option::VISUAL_OFF as i64,
        text: c!(
            "How bell alerts should be shown: a message ('on'), a message and a bell ('both') or nothing ('off')."
        ),
        ..options_table_entry::const_default()
    },
    options_table_entry {
        name: "visual-silence",
        type_: options_table_type::OPTIONS_TABLE_CHOICE,
        scope: OPTIONS_TABLE_SESSION,
        choices: &OPTIONS_TABLE_VISUAL_BELL_LIST,
        default_num: visual_option::VISUAL_OFF as i64,
        text: c!(
            "How silence alerts should be shown: a message ('on'), a message and a bell ('both') or nothing ('off')."
        ),
        ..options_table_entry::const_default()
    },
    options_table_entry {
        name: "word-separators",
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_SESSION,
        default_str: Some("!\"#$%&'()*+,-./:;<=>?@[\\]^`{|}~"),
        text: c!("Characters considered to separate words."),
        ..options_table_entry::const_default()
    },
    // Window options
    options_table_entry {
        name: "aggressive-resize",
        type_: options_table_type::OPTIONS_TABLE_FLAG,
        scope: OPTIONS_TABLE_WINDOW,
        default_num: 0,
        text: c!(
            "When 'window-size' is 'smallest', whether the maximum size of a window is the smallest attached session where it is the current window ('on') or the smallest session it is linked to ('off')."
        ),
        ..options_table_entry::const_default()
    },
    options_table_entry {
        name: "allow-passthrough",
        type_: options_table_type::OPTIONS_TABLE_CHOICE,
        scope: OPTIONS_TABLE_WINDOW | OPTIONS_TABLE_PANE,
        choices: &OPTIONS_TABLE_ALLOW_PASSTHROUGH_LIST,
        default_num: 0,
        text: c!(
            "Whether applications are allowed to use the escape sequence to bypass tmux. Can be 'off' (disallowed), 'on' (allowed if the pane is visible), or 'all' (allowed even if the pane is invisible)."
        ),
        ..options_table_entry::const_default()
    },
    options_table_entry {
        name: "allow-rename",
        type_: options_table_type::OPTIONS_TABLE_FLAG,
        scope: OPTIONS_TABLE_WINDOW | OPTIONS_TABLE_PANE,
        default_num: 0,
        text: c!("Whether applications are allowed to use the escape sequence to rename windows."),
        ..options_table_entry::const_default()
    },
    options_table_entry {
        name: "allow-set-title",
        type_: options_table_type::OPTIONS_TABLE_FLAG,
        scope: OPTIONS_TABLE_WINDOW | OPTIONS_TABLE_PANE,
        default_num: 1,
        text: c!(
            "Whether applications are allowed to use the escape sequence to set the pane title."
        ),
        ..options_table_entry::const_default()
    },
    options_table_entry {
        name: "alternate-screen",
        type_: options_table_type::OPTIONS_TABLE_FLAG,
        scope: OPTIONS_TABLE_WINDOW | OPTIONS_TABLE_PANE,
        default_num: 1,
        text: c!("Whether applications are allowed to use the alternate screen."),
        ..options_table_entry::const_default()
    },
    options_table_entry {
        name: "automatic-rename",
        type_: options_table_type::OPTIONS_TABLE_FLAG,
        scope: OPTIONS_TABLE_WINDOW,
        default_num: 1,
        text: c!("Whether windows are automatically renamed."),
        ..options_table_entry::const_default()
    },
    options_table_entry {
        name: "automatic-rename-format",
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_WINDOW,
        default_str: Some("#{?pane_in_mode,[tmux],#{pane_current_command}}#{?pane_dead,[dead],}"),
        text: c!("Format used to automatically rename windows."),
        ..options_table_entry::const_default()
    },
    options_table_entry {
        name: "clock-mode-colour",
        type_: options_table_type::OPTIONS_TABLE_COLOUR,
        scope: OPTIONS_TABLE_WINDOW,
        default_num: 4,
        text: c!("Colour of the clock in clock mode."),
        ..options_table_entry::const_default()
    },
    options_table_entry {
        name: "clock-mode-style",
        type_: options_table_type::OPTIONS_TABLE_CHOICE,
        scope: OPTIONS_TABLE_WINDOW,
        choices: &OPTIONS_TABLE_CLOCK_MODE_STYLE_LIST,
        default_num: 1,
        text: c!("Time format of the clock in clock mode."),
        ..options_table_entry::const_default()
    },
    options_table_entry {
        name: "copy-mode-match-style",
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_WINDOW,
        default_str: Some("bg=cyan,fg=black"),
        flags: OPTIONS_TABLE_IS_STYLE,
        separator: c!(","),
        text: c!("Style of search matches in copy mode."),
        ..options_table_entry::const_default()
    },
    options_table_entry {
        name: "copy-mode-current-match-style",
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_WINDOW,
        default_str: Some("bg=magenta,fg=black"),
        flags: OPTIONS_TABLE_IS_STYLE,
        separator: c!(","),
        text: c!("Style of the current search match in copy mode."),
        ..options_table_entry::const_default()
    },
    options_table_entry {
        name: "copy-mode-mark-style",
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_WINDOW,
        default_str: Some("bg=red,fg=black"),
        flags: OPTIONS_TABLE_IS_STYLE,
        separator: c!(","),
        text: c!("Style of the marked line in copy mode."),
        ..options_table_entry::const_default()
    },
    options_table_entry {
        name: "fill-character",
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_WINDOW,
        default_str: Some(""),
        text: c!("Character used to fill unused parts of window."),
        ..options_table_entry::const_default()
    },
    options_table_entry {
        name: "main-pane-height",
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_WINDOW,
        default_str: Some("24"),
        text: c!(
            "Height of the main pane in the 'main-horizontal' layout. This may be a percentage, for example '10%'."
        ),
        ..options_table_entry::const_default()
    },
    options_table_entry {
        name: "main-pane-width",
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_WINDOW,
        default_str: Some("80"),
        text: c!(
            "Width of the main pane in the 'main-vertical' layout. This may be a percentage, for example '10%'."
        ),
        ..options_table_entry::const_default()
    },
    options_table_entry {
        name: "mode-keys",
        type_: options_table_type::OPTIONS_TABLE_CHOICE,
        scope: OPTIONS_TABLE_WINDOW,
        choices: &OPTIONS_TABLE_MODE_KEYS_LIST,
        default_num: modekey::MODEKEY_EMACS as i64,
        text: c!("Key set used in copy mode."),
        ..options_table_entry::const_default()
    },
    options_table_entry {
        name: "mode-style",
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_WINDOW,
        flags: OPTIONS_TABLE_IS_STYLE,
        default_str: Some("bg=yellow,fg=black"),
        separator: c!(","),
        text: c!("Style of indicators and highlighting in modes."),
        ..options_table_entry::const_default()
    },
    options_table_entry {
        name: "monitor-activity",
        type_: options_table_type::OPTIONS_TABLE_FLAG,
        scope: OPTIONS_TABLE_WINDOW,
        default_num: 0,
        text: c!("Whether an alert is triggered by activity."),
        ..options_table_entry::const_default()
    },
    options_table_entry {
        name: "monitor-bell",
        type_: options_table_type::OPTIONS_TABLE_FLAG,
        scope: OPTIONS_TABLE_WINDOW,
        default_num: 1,
        text: c!("Whether an alert is triggered by a bell."),
        ..options_table_entry::const_default()
    },
    options_table_entry {
        name: "monitor-silence",
        type_: options_table_type::OPTIONS_TABLE_NUMBER,
        scope: OPTIONS_TABLE_WINDOW,
        minimum: 0,
        maximum: i32::MAX as u32,
        default_num: 0,
        text: c!("Time after which an alert is triggered by silence. Zero means no alert."),
        ..options_table_entry::const_default()
    },
    options_table_entry {
        name: "other-pane-height",
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_WINDOW,
        default_str: Some("0"),
        text: c!(
            "Height of the other panes in the 'main-horizontal' layout. This may be a percentage, for example '10%'."
        ),
        ..options_table_entry::const_default()
    },
    options_table_entry {
        name: "other-pane-width",
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_WINDOW,
        default_str: Some("0"),
        text: c!(
            "Height of the other panes in the 'main-vertical' layout. This may be a percentage, for example '10%'."
        ),
        ..options_table_entry::const_default()
    },
    options_table_entry {
        name: "pane-active-border-style",
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_WINDOW,
        default_str: Some("#{?pane_in_mode,fg=yellow,#{?synchronize-panes,fg=red,fg=green}}"),
        flags: OPTIONS_TABLE_IS_STYLE,
        separator: c!(","),
        text: c!("Style of the active pane border."),
        ..options_table_entry::const_default()
    },
    options_table_entry {
        name: "pane-base-index",
        type_: options_table_type::OPTIONS_TABLE_NUMBER,
        scope: OPTIONS_TABLE_WINDOW,
        minimum: 0,
        maximum: u16::MAX as u32,
        default_num: 0,
        text: c!("Index of the first pane in each window."),
        ..options_table_entry::const_default()
    },
    options_table_entry {
        name: "pane-border-format",
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_WINDOW | OPTIONS_TABLE_PANE,
        default_str: Some("#{?pane_active,#[reverse],}#{pane_index}#[default] \"#{pane_title}\""),
        text: c!("Format of text in the pane status lines."),
        ..options_table_entry::const_default()
    },
    options_table_entry {
        name: "pane-border-indicators",
        type_: options_table_type::OPTIONS_TABLE_CHOICE,
        scope: OPTIONS_TABLE_WINDOW,
        choices: &OPTIONS_TABLE_PANE_BORDER_INDICATORS_LIST,
        default_num: pane_border_indicator::PANE_BORDER_COLOUR as i64,
        text: c!(
            "Whether to indicate the active pane by colouring border or displaying arrow markers."
        ),
        ..options_table_entry::const_default()
    },
    options_table_entry {
        name: "pane-border-lines",
        type_: options_table_type::OPTIONS_TABLE_CHOICE,
        scope: OPTIONS_TABLE_WINDOW,
        choices: &OPTIONS_TABLE_PANE_BORDER_LINES_LIST,
        default_num: pane_lines::PANE_LINES_SINGLE as i64,
        text: c!(
            "Type of characters used to draw pane border lines. Some of these are only supported on terminals with UTF-8 support."
        ),
        ..options_table_entry::const_default()
    },
    options_table_entry {
        name: "pane-border-status",
        type_: options_table_type::OPTIONS_TABLE_CHOICE,
        scope: OPTIONS_TABLE_WINDOW,
        choices: &OPTIONS_TABLE_PANE_STATUS_LIST,
        default_num: pane_status::PANE_STATUS_OFF as i64,
        text: c!("Position of the pane status lines."),
        ..options_table_entry::const_default()
    },
    options_table_entry {
        name: "pane-border-style",
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_WINDOW,
        default_str: Some("default"),
        flags: OPTIONS_TABLE_IS_STYLE,
        separator: c!(","),
        text: c!("Style of the pane status lines."),
        ..options_table_entry::const_default()
    },
    options_table_entry {
        name: "pane-colours",
        type_: options_table_type::OPTIONS_TABLE_COLOUR,
        scope: OPTIONS_TABLE_WINDOW | OPTIONS_TABLE_PANE,
        default_str: Some(""),
        flags: OPTIONS_TABLE_IS_ARRAY,
        text: c!("The default colour palette for colours zero to 255."),
        ..options_table_entry::const_default()
    },
    options_table_entry {
        name: "popup-style",
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_WINDOW,
        default_str: Some("default"),
        flags: OPTIONS_TABLE_IS_STYLE,
        separator: c!(","),
        text: c!("Default style of popups."),
        ..options_table_entry::const_default()
    },
    options_table_entry {
        name: "popup-border-style",
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_WINDOW,
        default_str: Some("default"),
        flags: OPTIONS_TABLE_IS_STYLE,
        separator: c!(","),
        text: c!("Default style of popup borders."),
        ..options_table_entry::const_default()
    },
    options_table_entry {
        name: "popup-border-lines",
        type_: options_table_type::OPTIONS_TABLE_CHOICE,
        scope: OPTIONS_TABLE_WINDOW,
        choices: &OPTIONS_TABLE_POPUP_BORDER_LINES_LIST,
        default_num: box_lines::BOX_LINES_SINGLE as i64,
        text: c!(
            "Type of characters used to draw popup border lines. Some of these are only supported on terminals with UTF-8 support."
        ),
        ..options_table_entry::const_default()
    },
    options_table_entry {
        name: "remain-on-exit",
        type_: options_table_type::OPTIONS_TABLE_CHOICE,
        scope: OPTIONS_TABLE_WINDOW | OPTIONS_TABLE_PANE,
        choices: &OPTIONS_TABLE_REMAIN_ON_EXIT_LIST,
        default_num: 0,
        text: c!(
            "Whether panes should remain ('on') or be automatically killed ('off' or 'failed') when the program inside exits."
        ),
        ..options_table_entry::const_default()
    },
    options_table_entry {
        name: "remain-on-exit-format",
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_WINDOW | OPTIONS_TABLE_PANE,
        default_str: Some(
            "Pane is dead (#{?#{!=:#{pane_dead_status},},status #{pane_dead_status},}#{?#{!=:#{pane_dead_signal},},signal #{pane_dead_signal},}, #{t:pane_dead_time})",
        ),
        text: c!(
            "Message shown after the program in a pane has exited, if remain-on-exit is enabled."
        ),
        ..options_table_entry::const_default()
    },
    options_table_entry {
        name: "scroll-on-clear",
        type_: options_table_type::OPTIONS_TABLE_FLAG,
        scope: OPTIONS_TABLE_WINDOW | OPTIONS_TABLE_PANE,
        default_num: 1,
        text: c!(
            "Whether the contents of the screen should be scrolled into history when clearing the whole screen."
        ),
        ..options_table_entry::const_default()
    },
    options_table_entry {
        name: "synchronize-panes",
        type_: options_table_type::OPTIONS_TABLE_FLAG,
        scope: OPTIONS_TABLE_WINDOW | OPTIONS_TABLE_PANE,
        default_num: 0,
        text: c!("Whether typing should be sent to all panes simultaneously."),
        ..options_table_entry::const_default()
    },
    options_table_entry {
        name: "window-active-style",
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_WINDOW | OPTIONS_TABLE_PANE,
        default_str: Some("default"),
        flags: OPTIONS_TABLE_IS_STYLE,
        separator: c!(","),
        text: c!("Default style of the active pane."),
        ..options_table_entry::const_default()
    },
    options_table_entry {
        name: "window-size",
        type_: options_table_type::OPTIONS_TABLE_CHOICE,
        scope: OPTIONS_TABLE_WINDOW,
        choices: &OPTIONS_TABLE_WINDOW_SIZE_LIST,
        default_num: window_size_option::WINDOW_SIZE_LATEST as i64,
        text: c!(
            "How window size is calculated. 'latest' uses the size of the most recently used client, 'largest' the largest client, 'smallest' the smallest client and 'manual' a size set by the 'resize-window' command."
        ),
        ..options_table_entry::const_default()
    },
    options_table_entry {
        name: "window-style",
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_WINDOW | OPTIONS_TABLE_PANE,
        default_str: Some("default"),
        flags: OPTIONS_TABLE_IS_STYLE,
        separator: c!(","),
        text: c!("Default style of panes that are not the active pane."),
        ..options_table_entry::const_default()
    },
    options_table_entry {
        name: "window-status-activity-style",
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_WINDOW,
        default_str: Some("reverse"),
        flags: OPTIONS_TABLE_IS_STYLE,
        separator: c!(","),
        text: c!("Style of windows in the status line with an activity alert."),
        ..options_table_entry::const_default()
    },
    options_table_entry {
        name: "window-status-bell-style",
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_WINDOW,
        default_str: Some("reverse"),
        flags: OPTIONS_TABLE_IS_STYLE,
        separator: c!(","),
        text: c!("Style of windows in the status line with a bell alert."),
        ..options_table_entry::const_default()
    },
    options_table_entry {
        name: "window-status-current-format",
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_WINDOW,
        default_str: Some("#I:#W#{?window_flags,#{window_flags}, }"),
        text: c!("Format of the current window in the status line."),
        ..options_table_entry::const_default()
    },
    options_table_entry {
        name: "window-status-current-style",
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_WINDOW,
        default_str: Some("default"),
        flags: OPTIONS_TABLE_IS_STYLE,
        separator: c!(","),
        text: c!("Style of the current window in the status line."),
        ..options_table_entry::const_default()
    },
    options_table_entry {
        name: "window-status-format",
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_WINDOW,
        default_str: Some("#I:#W#{?window_flags,#{window_flags}, }"),
        text: c!("Format of windows in the status line, except the current window."),
        ..options_table_entry::const_default()
    },
    options_table_entry {
        name: "window-status-last-style",
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_WINDOW,
        default_str: Some("default"),
        flags: OPTIONS_TABLE_IS_STYLE,
        separator: c!(","),
        text: c!("Style of the last window in the status line."),
        ..options_table_entry::const_default()
    },
    options_table_entry {
        name: "window-status-separator",
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_WINDOW,
        default_str: Some(" "),
        text: c!("Separator between windows in the status line."),
        ..options_table_entry::const_default()
    },
    options_table_entry {
        name: "window-status-style",
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_WINDOW,
        default_str: Some("default"),
        flags: OPTIONS_TABLE_IS_STYLE,
        separator: c!(","),
        text: c!("Style of windows in the status line, except the current and last windows."),
        ..options_table_entry::const_default()
    },
    options_table_entry {
        name: "wrap-search",
        type_: options_table_type::OPTIONS_TABLE_FLAG,
        scope: OPTIONS_TABLE_WINDOW,
        default_num: 1,
        text: c!("Whether searching in copy mode should wrap at the top or bottom."),
        ..options_table_entry::const_default()
    },
    options_table_entry {
        name: "xterm-keys",
        type_: options_table_type::OPTIONS_TABLE_FLAG,
        scope: OPTIONS_TABLE_WINDOW,
        default_num: 1,
        text: c!(
            "Whether xterm-style function key sequences should be sent. This option is no longer used."
        ),
        ..options_table_entry::const_default()
    },
    // Hook options.
    options_table_hook!("after-bind-key", ""),
    options_table_hook!("after-capture-pane", ""),
    options_table_hook!("after-copy-mode", ""),
    options_table_hook!("after-display-message", ""),
    options_table_hook!("after-display-panes", ""),
    options_table_hook!("after-kill-pane", ""),
    options_table_hook!("after-list-buffers", ""),
    options_table_hook!("after-list-clients", ""),
    options_table_hook!("after-list-keys", ""),
    options_table_hook!("after-list-panes", ""),
    options_table_hook!("after-list-sessions", ""),
    options_table_hook!("after-list-windows", ""),
    options_table_hook!("after-load-buffer", ""),
    options_table_hook!("after-lock-server", ""),
    options_table_hook!("after-new-session", ""),
    options_table_hook!("after-new-window", ""),
    options_table_hook!("after-paste-buffer", ""),
    options_table_hook!("after-pipe-pane", ""),
    options_table_hook!("after-queue", ""),
    options_table_hook!("after-refresh-client", ""),
    options_table_hook!("after-rename-session", ""),
    options_table_hook!("after-rename-window", ""),
    options_table_hook!("after-resize-pane", ""),
    options_table_hook!("after-resize-window", ""),
    options_table_hook!("after-save-buffer", ""),
    options_table_hook!("after-select-layout", ""),
    options_table_hook!("after-select-pane", ""),
    options_table_hook!("after-select-window", ""),
    options_table_hook!("after-send-keys", ""),
    options_table_hook!("after-set-buffer", ""),
    options_table_hook!("after-set-environment", ""),
    options_table_hook!("after-set-hook", ""),
    options_table_hook!("after-set-option", ""),
    options_table_hook!("after-show-environment", ""),
    options_table_hook!("after-show-messages", ""),
    options_table_hook!("after-show-options", ""),
    options_table_hook!("after-split-window", ""),
    options_table_hook!("after-unbind-key", ""),
    options_table_hook!("alert-activity", ""),
    options_table_hook!("alert-bell", ""),
    options_table_hook!("alert-silence", ""),
    options_table_hook!("client-active", ""),
    options_table_hook!("client-attached", ""),
    options_table_hook!("client-detached", ""),
    options_table_hook!("client-focus-in", ""),
    options_table_hook!("client-focus-out", ""),
    options_table_hook!("client-resized", ""),
    options_table_hook!("client-session-changed", ""),
    options_table_hook!("command-error", ""),
    options_table_pane_hook!("pane-died", ""),
    options_table_pane_hook!("pane-exited", ""),
    options_table_pane_hook!("pane-fous-in", ""),
    options_table_pane_hook!("pane-fous-out", ""),
    options_table_pane_hook!("pane-mode-hanged", ""),
    options_table_pane_hook!("pane-set-lipboard", ""),
    options_table_pane_hook!("pane-title-hanged", ""),
    options_table_hook!("session-closed", ""),
    options_table_hook!("session-created", ""),
    options_table_hook!("session-renamed", ""),
    options_table_hook!("session-window-changed", ""),
    options_table_window_hook!("window-layout-changed", ""),
    options_table_hook!("window-linked", ""),
    options_table_window_hook!("window-pane-changed", ""),
    options_table_window_hook!("window-renamed", ""),
    options_table_window_hook!("window-resized", ""),
    options_table_hook!("window-unlinked", ""),
];

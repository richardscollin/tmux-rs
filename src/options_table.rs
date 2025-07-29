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
static mut OPTIONS_TABLE_MODE_KEYS_LIST: [*const u8; 3] = [c!("emacs"), c!("vi"), null()];
static mut OPTIONS_TABLE_CLOCK_MODE_STYLE_LIST: [*const u8; 3] = [c!("12"), c!("24"), null()];
static mut OPTIONS_TABLE_STATUS_LIST: [*const u8; 7] = [
    c!("off"),
    c!("on"),
    c!("2"),
    c!("3"),
    c!("4"),
    c!("5"),
    null(),
];
static mut OPTIONS_TABLE_MESSAGE_LINE_LIST: [*const u8; 6] =
    [c!("0"), c!("1"), c!("2"), c!("3"), c!("4"), null()];
static mut OPTIONS_TABLE_STATUS_KEYS_LIST: [*const u8; 3] = [c!("emacs"), c!("vi"), null()];
static mut OPTIONS_TABLE_STATUS_JUSTIFY_LIST: [*const u8; 5] = [
    c!("left"),
    c!("centre"),
    c!("right"),
    c!("absolute-centre"),
    null(),
];
static mut OPTIONS_TABLE_STATUS_POSITION_LIST: [*const u8; 3] = [c!("top"), c!("bottom"), null()];
static mut OPTIONS_TABLE_BELL_ACTION_LIST: [*const u8; 5] =
    [c!("none"), c!("any"), c!("current"), c!("other"), null()];
static mut OPTIONS_TABLE_VISUAL_BELL_LIST: [*const u8; 4] =
    [c!("off"), c!("on"), c!("both"), null()];
static mut OPTIONS_TABLE_CURSOR_STYLE_LIST: [*const u8; 8] = [
    c!("default"),
    c!("blinking-block"),
    c!("block"),
    c!("blinking-underline"),
    c!("underline"),
    c!("blinking-bar"),
    c!("bar"),
    null(),
];
static mut OPTIONS_TABLE_PANE_STATUS_LIST: [*const u8; 4] =
    [c!("off"), c!("top"), c!("bottom"), null()];
static mut OPTIONS_TABLE_PANE_BORDER_INDICATORS_LIST: [*const u8; 5] =
    [c!("off"), c!("colour"), c!("arrows"), c!("both"), null()];
static mut OPTIONS_TABLE_PANE_BORDER_LINES_LIST: [*const u8; 6] = [
    c!("single"),
    c!("double"),
    c!("heavy"),
    c!("simple"),
    c!("number"),
    null(),
];
static mut OPTIONS_TABLE_POPUP_BORDER_LINES_LIST: [*const u8; 8] = [
    c!("single"),
    c!("double"),
    c!("heavy"),
    c!("simple"),
    c!("rounded"),
    c!("padded"),
    c!("none"),
    null(),
];
static mut OPTIONS_TABLE_SET_CLIPBOARD_LIST: [*const u8; 4] =
    [c!("off"), c!("external"), c!("on"), null()];
static mut OPTIONS_TABLE_WINDOW_SIZE_LIST: [*const u8; 5] = [
    c!("largest"),
    c!("smallest"),
    c!("manual"),
    c!("latest"),
    null(),
];
static mut OPTIONS_TABLE_REMAIN_ON_EXIT_LIST: [*const u8; 4] =
    [c!("off"), c!("on"), c!("failed"), null()];
static mut OPTIONS_TABLE_DESTROY_UNATTACHED_LIST: [*const u8; 5] = [
    c!("off"),
    c!("on"),
    c!("keep-last"),
    c!("keep-group"),
    null(),
];
static mut OPTIONS_TABLE_DETACH_ON_DESTROY_LIST: [*const u8; 6] = [
    c!("off"),
    c!("on"),
    c!("no-detached"),
    c!("previous"),
    c!("next"),
    null(),
];
static mut OPTIONS_TABLE_EXTENDED_KEYS_LIST: [*const u8; 4] =
    [c!("off"), c!("on"), c!("always"), null()];
static mut OPTIONS_TABLE_EXTENDED_KEYS_FORMAT_LIST: [*const u8; 3] =
    [c!("csi-u"), c!("xterm"), null()];
static mut OPTIONS_TABLE_ALLOW_PASSTHROUGH_LIST: [*const u8; 4] =
    [c!("off"), c!("on"), c!("all"), null()];

#[rustfmt::skip]
/// Map of name conversions.
pub static mut OPTIONS_OTHER_NAMES: [options_name_map; 6] = [
    options_name_map::new(c!("display-panes-color"), c!("display-panes-colour")),
    options_name_map::new(c!("display-panes-active-color"), c!("display-panes-active-colour")),
    options_name_map::new(c!("clock-mode-color"), c!("clock-mode-colour")),
    options_name_map::new(c!("cursor-color"), c!("cursor-colour")),
    options_name_map::new(c!("pane-colors"), c!("pane-colours")),
    options_name_map::new(null(), null()),
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

#[allow(clippy::disallowed_methods)]
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

#[allow(clippy::disallowed_methods)]
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
            name: $hook_name.as_ptr().cast(),
            type_: options_table_type::OPTIONS_TABLE_COMMAND,
            scope: OPTIONS_TABLE_SESSION,
            flags: OPTIONS_TABLE_IS_ARRAY | OPTIONS_TABLE_IS_HOOK,
            default_str: Some($default_value),
            separator: c!(""),
            ..unsafe { zeroed() }
        }
    };
}

macro_rules! options_table_pane_hook {
    ($hook_name:expr, $default_value:expr) => {
        options_table_entry {
            name: $hook_name.as_ptr().cast(),
            type_: options_table_type::OPTIONS_TABLE_COMMAND,
            scope: OPTIONS_TABLE_WINDOW | OPTIONS_TABLE_PANE,
            flags: OPTIONS_TABLE_IS_ARRAY | OPTIONS_TABLE_IS_HOOK,
            default_str: Some($default_value),
            separator: c!(""),
            ..unsafe { zeroed() }
        }
    };
}

macro_rules! options_table_window_hook {
    ($hook_name:expr, $default_value:expr) => {
        options_table_entry {
            name: $hook_name.as_ptr().cast(),
            type_: options_table_type::OPTIONS_TABLE_COMMAND,
            scope: OPTIONS_TABLE_WINDOW,
            flags: OPTIONS_TABLE_IS_ARRAY | OPTIONS_TABLE_IS_HOOK,
            default_str: Some($default_value),
            separator: c!(""),
            ..unsafe { zeroed() }
        }
    };
}

pub static mut OPTIONS_TABLE: [options_table_entry; 191] = [
    options_table_entry {
        name: c!("backspace"),
        type_: options_table_type::OPTIONS_TABLE_KEY,
        scope: OPTIONS_TABLE_SERVER,
        default_num: b'\x7f' as i64,
        text: c!("The key to send for backspace."),
        ..unsafe { zeroed() }
    },
    options_table_entry {
        name: c!("buffer-limit"),
        type_: options_table_type::OPTIONS_TABLE_NUMBER,
        scope: OPTIONS_TABLE_SERVER,
        minimum: 1,
        maximum: i32::MAX as u32,
        default_num: 50,
        text: c!(
            "The maximum number of automatic buffers. When this is reached, the oldest buffer is deleted."
        ),
        ..unsafe { zeroed() }
    },
    options_table_entry {
        name: c!("command-alias"),
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
        ..unsafe { zeroed() }
    },
    options_table_entry {
        name: c!("copy-command"),
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_SERVER,
        default_str: Some(""),
        text: c!("Shell command run when text is copied. If empty, no command is run."),
        ..unsafe { zeroed() }
    },
    options_table_entry {
        name: c!("cursor-colour"),
        type_: options_table_type::OPTIONS_TABLE_COLOUR,
        scope: OPTIONS_TABLE_WINDOW | OPTIONS_TABLE_PANE,
        default_num: -1,
        text: c!("Colour of the cursor."),
        ..unsafe { zeroed() }
    },
    options_table_entry {
        name: c!("cursor-style"),
        type_: options_table_type::OPTIONS_TABLE_CHOICE,
        scope: OPTIONS_TABLE_WINDOW | OPTIONS_TABLE_PANE,
        choices: &raw const OPTIONS_TABLE_CURSOR_STYLE_LIST as *const *const u8,
        default_num: 0,
        text: c!("Style of the cursor."),
        ..unsafe { zeroed() }
    },
    options_table_entry {
        name: c!("default-terminal"),
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_SERVER,
        default_str: Some(TMUX_TERM),
        text: c!("Default for the 'TERM' environment variable."),
        ..unsafe { zeroed() }
    },
    options_table_entry {
        name: c!("editor"),
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_SERVER,
        default_str: Some(_PATH_VI),
        text: c!("Editor run to edit files."),
        ..unsafe { zeroed() }
    },
    options_table_entry {
        name: c!("escape-time"),
        type_: options_table_type::OPTIONS_TABLE_NUMBER,
        scope: OPTIONS_TABLE_SERVER,
        minimum: 0,
        maximum: i32::MAX as u32,
        default_num: 10,
        unit: c!("milliseconds"),
        text: c!("Time to wait before assuming a key is Escape."),
        ..unsafe { zeroed() }
    },
    options_table_entry {
        name: c!("exit-empty"),
        type_: options_table_type::OPTIONS_TABLE_FLAG,
        scope: OPTIONS_TABLE_SERVER,
        default_num: 1,
        text: c!("Whether the server should exit if there are no sessions."),
        ..unsafe { zeroed() }
    },
    options_table_entry {
        name: c!("exit-unattached"),
        type_: options_table_type::OPTIONS_TABLE_FLAG,
        scope: OPTIONS_TABLE_SERVER,
        default_num: 0,
        text: c!("Whether the server should exit if there are no attached clients."),
        ..unsafe { zeroed() }
    },
    options_table_entry {
        name: c!("extended-keys"),
        type_: options_table_type::OPTIONS_TABLE_CHOICE,
        scope: OPTIONS_TABLE_SERVER,
        choices: &raw const OPTIONS_TABLE_EXTENDED_KEYS_LIST as *const *const u8,
        default_num: 0,
        text: c!("Whether to request extended key sequences from terminals that support it."),
        ..unsafe { zeroed() }
    },
    options_table_entry {
        name: c!("extended-keys-format"),
        type_: options_table_type::OPTIONS_TABLE_CHOICE,
        scope: OPTIONS_TABLE_SERVER,
        choices: &raw const OPTIONS_TABLE_EXTENDED_KEYS_FORMAT_LIST as *const *const u8,
        default_num: 1,
        text: c!("The format of emitted extended key sequences."),
        ..unsafe { zeroed() }
    },
    options_table_entry {
        name: c!("focus-events"),
        type_: options_table_type::OPTIONS_TABLE_FLAG,
        scope: OPTIONS_TABLE_SERVER,
        default_num: 0,
        text: c!("Whether to send focus events to applications."),
        ..unsafe { zeroed() }
    },
    options_table_entry {
        name: c!("history-file"),
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_SERVER,
        default_str: Some(""),
        text: c!(
            "Location of the command prompt history file. Empty does not write a history file."
        ),
        ..unsafe { zeroed() }
    },
    options_table_entry {
        name: c!("menu-style"),
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_WINDOW,
        flags: OPTIONS_TABLE_IS_STYLE,
        default_str: Some("default"),
        separator: c!(","),
        text: c!("Default style of menu."),
        ..unsafe { zeroed() }
    },
    options_table_entry {
        name: c!("menu-selected-style"),
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_WINDOW,
        flags: OPTIONS_TABLE_IS_STYLE,
        default_str: Some("bg=yellow,fg=black"),
        separator: c!(","),
        text: c!("Default style of selected menu item."),
        ..unsafe { zeroed() }
    },
    options_table_entry {
        name: c!("menu-border-style"),
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_WINDOW,
        default_str: Some("default"),
        flags: OPTIONS_TABLE_IS_STYLE,
        separator: c!(","),
        text: c!("Default style of menu borders."),
        ..unsafe { zeroed() }
    },
    options_table_entry {
        name: c!("menu-border-lines"),
        type_: options_table_type::OPTIONS_TABLE_CHOICE,
        scope: OPTIONS_TABLE_WINDOW,
        choices: &raw const OPTIONS_TABLE_POPUP_BORDER_LINES_LIST as *const *const u8,
        default_num: box_lines::BOX_LINES_SINGLE as i64,
        text: c!(
            "Type of characters used to draw menu border lines. Some of these are only supported on terminals with UTF-8 support."
        ),
        ..unsafe { zeroed() }
    },
    options_table_entry {
        name: c!("message-limit"),
        type_: options_table_type::OPTIONS_TABLE_NUMBER,
        scope: OPTIONS_TABLE_SERVER,
        minimum: 0,
        maximum: i32::MAX as u32,
        default_num: 1000,
        text: c!("Maximum number of server messages to keep."),
        ..unsafe { zeroed() }
    },
    options_table_entry {
        name: c!("prefix-timeout"),
        type_: options_table_type::OPTIONS_TABLE_NUMBER,
        scope: OPTIONS_TABLE_SERVER,
        minimum: 0,
        maximum: i32::MAX as u32,
        default_num: 0,
        unit: c!("milliseconds"),
        text: c!(
            "The timeout for the prefix key if no subsequent key is pressed. Zero means disabled."
        ),
        ..unsafe { zeroed() }
    },
    options_table_entry {
        name: c!("prompt-history-limit"),
        type_: options_table_type::OPTIONS_TABLE_NUMBER,
        scope: OPTIONS_TABLE_SERVER,
        minimum: 0,
        maximum: i32::MAX as u32,
        default_num: 100,
        text: c!("Maximum number of commands to keep in history."),
        ..unsafe { zeroed() }
    },
    options_table_entry {
        name: c!("set-clipboard"),
        type_: options_table_type::OPTIONS_TABLE_CHOICE,
        scope: OPTIONS_TABLE_SERVER,
        choices: &raw const OPTIONS_TABLE_SET_CLIPBOARD_LIST as *const *const u8,
        default_num: 1,
        text: c!(
            "Whether to attempt to set the system clipboard ('on' or 'external') and whether to allow applications to create paste buffers with an escape sequence ('on' only)."
        ),
        ..unsafe { zeroed() }
    },
    options_table_entry {
        name: c!("terminal-overrides"),
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_SERVER,
        flags: OPTIONS_TABLE_IS_ARRAY,
        default_str: Some("linux*:AX@"),
        separator: c!(","),
        text: c!("List of terminal capabilities overrides."),
        ..unsafe { zeroed() }
    },
    options_table_entry {
        name: c!("terminal-features"),
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_SERVER,
        flags: OPTIONS_TABLE_IS_ARRAY,
        default_str: Some(
            "xterm*:clipboard:ccolour:cstyle:focus:title,screen*:title,rxvt*:ignorefkeys",
        ),
        separator: c!(","),
        text: c!("List of terminal features, used if they cannot be automatically detected."),
        ..unsafe { zeroed() }
    },
    options_table_entry {
        name: c!("user-keys"),
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_SERVER,
        flags: OPTIONS_TABLE_IS_ARRAY,
        default_str: Some(""),
        separator: c!(","),
        text: c!(
            "User key assignments. Each sequence in the list is translated into a key: 'User0', 'User1' and so on."
        ),
        ..unsafe { zeroed() }
    },
    // Session options.
    options_table_entry {
        name: c!("activity-action"),
        type_: options_table_type::OPTIONS_TABLE_CHOICE,
        scope: OPTIONS_TABLE_SESSION,
        choices: &raw const OPTIONS_TABLE_BELL_ACTION_LIST as *const *const u8,
        default_num: alert_option::ALERT_OTHER as i64,
        text: c!("Action to take on an activity alert."),
        ..unsafe { zeroed() }
    },
    options_table_entry {
        name: c!("assume-paste-time"),
        type_: options_table_type::OPTIONS_TABLE_NUMBER,
        scope: OPTIONS_TABLE_SESSION,
        minimum: 0,
        maximum: i32::MAX as u32,
        default_num: 1,
        unit: c!("milliseconds"),
        text: c!("Maximum time between input to assume it is pasting rather than typing."),
        ..unsafe { zeroed() }
    },
    options_table_entry {
        name: c!("base-index"),
        type_: options_table_type::OPTIONS_TABLE_NUMBER,
        scope: OPTIONS_TABLE_SESSION,
        minimum: 0,
        maximum: i32::MAX as u32,
        default_num: 0,
        text: c!("Default index of the first window in each session."),
        ..unsafe { zeroed() }
    },
    options_table_entry {
        name: c!("bell-action"),
        type_: options_table_type::OPTIONS_TABLE_CHOICE,
        scope: OPTIONS_TABLE_SESSION,
        choices: &raw const OPTIONS_TABLE_BELL_ACTION_LIST as *const *const u8,
        default_num: alert_option::ALERT_ANY as i64,
        text: c!("Action to take on a bell alert."),
        ..unsafe { zeroed() }
    },
    options_table_entry {
        name: c!("default-command"),
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_SESSION,
        default_str: Some(""),
        text: c!("Default command to run in new panes. If empty, a shell is started."),
        ..unsafe { zeroed() }
    },
    options_table_entry {
        name: c!("default-shell"),
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_SESSION,
        default_str: Some(_PATH_BSHELL_STR),
        text: c!("Location of default shell."),
        ..unsafe { zeroed() }
    },
    options_table_entry {
        name: c!("default-size"),
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_SESSION,
        pattern: c!("[0-9]*x[0-9]*"),
        default_str: Some("80x24"),
        text: c!("Initial size of new sessions."),
        ..unsafe { zeroed() }
    },
    options_table_entry {
        name: c!("destroy-unattached"),
        type_: options_table_type::OPTIONS_TABLE_CHOICE,
        scope: OPTIONS_TABLE_SESSION,
        choices: &raw const OPTIONS_TABLE_DESTROY_UNATTACHED_LIST as *const *const u8,
        default_num: 0,
        text: c!(
            "Whether to destroy sessions when they have no attached clients, or keep the last session whether in the group."
        ),
        ..unsafe { zeroed() }
    },
    options_table_entry {
        name: c!("detach-on-destroy"),
        type_: options_table_type::OPTIONS_TABLE_CHOICE,
        scope: OPTIONS_TABLE_SESSION,
        choices: &raw const OPTIONS_TABLE_DETACH_ON_DESTROY_LIST as *const *const u8,
        default_num: 1,
        text: c!(
            "Whether to detach when a session is destroyed, or switch the client to another session if any exist."
        ),
        ..unsafe { zeroed() }
    },
    options_table_entry {
        name: c!("display-panes-active-colour"),
        type_: options_table_type::OPTIONS_TABLE_COLOUR,
        scope: OPTIONS_TABLE_SESSION,
        default_num: 1,
        text: c!("Colour of the active pane for 'display-panes'."),
        ..unsafe { zeroed() }
    },
    options_table_entry {
        name: c!("display-panes-colour"),
        type_: options_table_type::OPTIONS_TABLE_COLOUR,
        scope: OPTIONS_TABLE_SESSION,
        default_num: 4,
        text: c!("Colour of not active panes for 'display-panes'."),
        ..unsafe { zeroed() }
    },
    options_table_entry {
        name: c!("display-panes-time"),
        type_: options_table_type::OPTIONS_TABLE_NUMBER,
        scope: OPTIONS_TABLE_SESSION,
        minimum: 1,
        maximum: i32::MAX as u32,
        default_num: 1000,
        unit: c!("milliseconds"),
        text: c!("Time for which 'display-panes' should show pane numbers."),
        ..unsafe { zeroed() }
    },
    options_table_entry {
        name: c!("display-time"),
        type_: options_table_type::OPTIONS_TABLE_NUMBER,
        scope: OPTIONS_TABLE_SESSION,
        minimum: 0,
        maximum: i32::MAX as u32,
        default_num: 750,
        unit: c!("milliseconds"),
        text: c!("Time for which status line messages should appear."),
        ..unsafe { zeroed() }
    },
    options_table_entry {
        name: c!("history-limit"),
        type_: options_table_type::OPTIONS_TABLE_NUMBER,
        scope: OPTIONS_TABLE_SESSION,
        minimum: 0,
        maximum: i32::MAX as u32,
        default_num: 2000,
        unit: c!("lines"),
        text: c!(
            "Maximum number of lines to keep in the history for each pane. If changed, the new value applies only to new panes."
        ),
        ..unsafe { zeroed() }
    },
    options_table_entry {
        name: c!("key-table"),
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_SESSION,
        default_str: Some("root"),
        text: c!("Default key table. Key presses are first looked up in this table."),
        ..unsafe { zeroed() }
    },
    options_table_entry {
        name: c!("lock-after-time"),
        type_: options_table_type::OPTIONS_TABLE_NUMBER,
        scope: OPTIONS_TABLE_SESSION,
        minimum: 0,
        maximum: i32::MAX as u32,
        default_num: 0,
        unit: c!("seconds"),
        text: c!("Time after which a client is locked if not used."),
        ..unsafe { zeroed() }
    },
    options_table_entry {
        name: c!("lock-command"),
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_SESSION,
        default_str: Some(TMUX_LOCK_CMD),
        text: c!("Shell command to run to lock a client."),
        ..unsafe { zeroed() }
    },
    options_table_entry {
        name: c!("message-command-style"),
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_SESSION,
        default_str: Some("bg=black,fg=yellow"),
        flags: OPTIONS_TABLE_IS_STYLE,
        separator: c!(","),
        text: c!(
            "Style of the command prompt when in command mode, if 'mode-keys' is set to 'vi'."
        ),
        ..unsafe { zeroed() }
    },
    options_table_entry {
        name: c!("message-line"),
        type_: options_table_type::OPTIONS_TABLE_CHOICE,
        scope: OPTIONS_TABLE_SESSION,
        choices: &raw const OPTIONS_TABLE_MESSAGE_LINE_LIST as *const *const u8,
        default_num: 0,
        text: c!("Position (line) of messages and the command prompt."),
        ..unsafe { zeroed() }
    },
    options_table_entry {
        name: c!("message-style"),
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_SESSION,
        default_str: Some("bg=yellow,fg=black"),
        flags: OPTIONS_TABLE_IS_STYLE,
        separator: c!(","),
        text: c!("Style of messages and the command prompt."),
        ..unsafe { zeroed() }
    },
    options_table_entry {
        name: c!("mouse"),
        type_: options_table_type::OPTIONS_TABLE_FLAG,
        scope: OPTIONS_TABLE_SESSION,
        default_num: 0,
        text: c!(
            "Whether the mouse is recognised and mouse key bindings are executed. Applications inside panes can use the mouse even when 'off'."
        ),
        ..unsafe { zeroed() }
    },
    options_table_entry {
        name: c!("prefix"),
        type_: options_table_type::OPTIONS_TABLE_KEY,
        scope: OPTIONS_TABLE_SESSION,
        default_num: b'b' as i64 | KEYC_CTRL as i64,
        text: c!("The prefix key."),
        ..unsafe { zeroed() }
    },
    options_table_entry {
        name: c!("prefix2"),
        type_: options_table_type::OPTIONS_TABLE_KEY,
        scope: OPTIONS_TABLE_SESSION,
        default_num: KEYC_NONE as i64,
        text: c!("A second prefix key."),
        ..unsafe { zeroed() }
    },
    options_table_entry {
        name: c!("renumber-windows"),
        type_: options_table_type::OPTIONS_TABLE_FLAG,
        scope: OPTIONS_TABLE_SESSION,
        default_num: 0,
        text: c!("Whether windows are automatically renumbered rather than leaving gaps."),
        ..unsafe { zeroed() }
    },
    options_table_entry {
        name: c!("repeat-time"),
        type_: options_table_type::OPTIONS_TABLE_NUMBER,
        scope: OPTIONS_TABLE_SESSION,
        minimum: 0,
        maximum: i16::MAX as u32,
        default_num: 500,
        unit: c!("milliseconds"),
        text: c!("Time to wait for a key binding to repeat, if it is bound with the '-r' flag."),
        ..unsafe { zeroed() }
    },
    options_table_entry {
        name: c!("set-titles"),
        type_: options_table_type::OPTIONS_TABLE_FLAG,
        scope: OPTIONS_TABLE_SESSION,
        default_num: 0,
        text: c!("Whether to set the terminal title, if supported."),
        ..unsafe { zeroed() }
    },
    options_table_entry {
        name: c!("set-titles-string"),
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_SESSION,
        default_str: Some("#S:#I:#W - \"#T\" #{session_alerts}"),
        text: c!("Format of the terminal title to set."),
        ..unsafe { zeroed() }
    },
    options_table_entry {
        name: c!("silence-action"),
        type_: options_table_type::OPTIONS_TABLE_CHOICE,
        scope: OPTIONS_TABLE_SESSION,
        choices: &raw const OPTIONS_TABLE_BELL_ACTION_LIST as *const *const u8,
        default_num: alert_option::ALERT_OTHER as i64,
        text: c!("Action to take on a silence alert."),
        ..unsafe { zeroed() }
    },
    options_table_entry {
        name: c!("status"),
        type_: options_table_type::OPTIONS_TABLE_CHOICE,
        scope: OPTIONS_TABLE_SESSION,
        choices: &raw const OPTIONS_TABLE_STATUS_LIST as *const *const u8,
        default_num: 1,
        text: c!("Number of lines in the status line."),
        ..unsafe { zeroed() }
    },
    options_table_entry {
        name: c!("status-bg"),
        type_: options_table_type::OPTIONS_TABLE_COLOUR,
        scope: OPTIONS_TABLE_SESSION,
        default_num: 8,
        text: c!(
            "Background colour of the status line. This option is deprecated, use 'status-style' instead."
        ),
        ..unsafe { zeroed() }
    },
    options_table_entry {
        name: c!("status-fg"),
        type_: options_table_type::OPTIONS_TABLE_COLOUR,
        scope: OPTIONS_TABLE_SESSION,
        default_num: 8,
        text: c!(
            "Foreground colour of the status line. This option is deprecated, use 'status-style' instead."
        ),
        ..unsafe { zeroed() }
    },
    options_table_entry {
        name: c!("status-format"),
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_SESSION,
        flags: OPTIONS_TABLE_IS_ARRAY,
        default_arr: &raw const OPTIONS_TABLE_STATUS_FORMAT_DEFAULT as *const *const u8,
        text: c!(
            "Formats for the status lines. Each array member is the format for one status line. The default status line is made up of several components which may be configured individually with other options such as 'status-left'."
        ),
        ..unsafe { zeroed() }
    },
    options_table_entry {
        name: c!("status-interval"),
        type_: options_table_type::OPTIONS_TABLE_NUMBER,
        scope: OPTIONS_TABLE_SESSION,
        minimum: 0,
        maximum: i32::MAX as u32,
        default_num: 15,
        unit: c!("seconds"),
        text: c!("Number of seconds between status line updates."),
        ..unsafe { zeroed() }
    },
    options_table_entry {
        name: c!("status-justify"),
        type_: options_table_type::OPTIONS_TABLE_CHOICE,
        scope: OPTIONS_TABLE_SESSION,
        choices: &raw const OPTIONS_TABLE_STATUS_JUSTIFY_LIST as *const *const u8,
        default_num: 0,
        text: c!("Position of the window list in the status line."),
        ..unsafe { zeroed() }
    },
    options_table_entry {
        name: c!("status-keys"),
        type_: options_table_type::OPTIONS_TABLE_CHOICE,
        scope: OPTIONS_TABLE_SESSION,
        choices: &raw const OPTIONS_TABLE_STATUS_KEYS_LIST as *const *const u8,
        default_num: modekey::MODEKEY_EMACS as i64,
        text: c!("Key set to use at the command prompt."),
        ..unsafe { zeroed() }
    },
    options_table_entry {
        name: c!("status-left"),
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_SESSION,
        default_str: Some("[#{session_name}] "),
        text: c!("Contents of the left side of the status line."),
        ..unsafe { zeroed() }
    },
    options_table_entry {
        name: c!("status-left-length"),
        type_: options_table_type::OPTIONS_TABLE_NUMBER,
        scope: OPTIONS_TABLE_SESSION,
        minimum: 0,
        maximum: i16::MAX as u32,
        default_num: 10,
        text: c!("Maximum width of the left side of the status line."),
        ..unsafe { zeroed() }
    },
    options_table_entry {
        name: c!("status-left-style"),
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_SESSION,
        default_str: Some("default"),
        flags: OPTIONS_TABLE_IS_STYLE,
        separator: c!(","),
        text: c!("Style of the left side of the status line."),
        ..unsafe { zeroed() }
    },
    options_table_entry {
        name: c!("status-position"),
        type_: options_table_type::OPTIONS_TABLE_CHOICE,
        scope: OPTIONS_TABLE_SESSION,
        choices: &raw const OPTIONS_TABLE_STATUS_POSITION_LIST as *const *const u8,
        default_num: 1,
        text: c!("Position of the status line."),
        ..unsafe { zeroed() }
    },
    options_table_entry {
        name: c!("status-right"),
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_SESSION,
        default_str: Some(
            "#{?window_bigger,[#{window_offset_x}#,#{window_offset_y}] ,}\"#{=21:pane_title}\" %H:%M %d-%b-%y",
        ),
        text: c!("Contents of the right side of the status line."),
        ..unsafe { zeroed() }
    },
    options_table_entry {
        name: c!("status-right-length"),
        type_: options_table_type::OPTIONS_TABLE_NUMBER,
        scope: OPTIONS_TABLE_SESSION,
        minimum: 0,
        maximum: i16::MAX as u32,
        default_num: 40,
        text: c!("Maximum width of the right side of the status line."),
        ..unsafe { zeroed() }
    },
    options_table_entry {
        name: c!("status-right-style"),
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_SESSION,
        default_str: Some("default"),
        flags: OPTIONS_TABLE_IS_STYLE,
        separator: c!(","),
        text: c!("Style of the right side of the status line."),
        ..unsafe { zeroed() }
    },
    options_table_entry {
        name: c!("status-style"),
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_SESSION,
        default_str: Some("bg=green,fg=black"),
        flags: OPTIONS_TABLE_IS_STYLE,
        separator: c!(","),
        text: c!("Style of the status line."),
        ..unsafe { zeroed() }
    },
    options_table_entry {
        name: c!("update-environment"),
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_SESSION,
        flags: OPTIONS_TABLE_IS_ARRAY,
        default_str: Some(
            "DISPLAY KRB5CCNAME SSH_ASKPASS SSH_AUTH_SOCK SSH_AGENT_PID SSH_CONNECTION WINDOWID XAUTHORITY",
        ),
        text: c!(
            "List of environment variables to update in the session environment when a client is attached."
        ),
        ..unsafe { zeroed() }
    },
    options_table_entry {
        name: c!("visual-activity"),
        type_: options_table_type::OPTIONS_TABLE_CHOICE,
        scope: OPTIONS_TABLE_SESSION,
        choices: &raw const OPTIONS_TABLE_VISUAL_BELL_LIST as *const *const u8,
        default_num: visual_option::VISUAL_OFF as i64,
        text: c!(
            "How activity alerts should be shown: a message ('on'), a message and a bell ('both') or nothing ('off')."
        ),
        ..unsafe { zeroed() }
    },
    options_table_entry {
        name: c!("visual-bell"),
        type_: options_table_type::OPTIONS_TABLE_CHOICE,
        scope: OPTIONS_TABLE_SESSION,
        choices: &raw const OPTIONS_TABLE_VISUAL_BELL_LIST as *const *const u8,
        default_num: visual_option::VISUAL_OFF as i64,
        text: c!(
            "How bell alerts should be shown: a message ('on'), a message and a bell ('both') or nothing ('off')."
        ),
        ..unsafe { zeroed() }
    },
    options_table_entry {
        name: c!("visual-silence"),
        type_: options_table_type::OPTIONS_TABLE_CHOICE,
        scope: OPTIONS_TABLE_SESSION,
        choices: &raw const OPTIONS_TABLE_VISUAL_BELL_LIST as *const *const u8,
        default_num: visual_option::VISUAL_OFF as i64,
        text: c!(
            "How silence alerts should be shown: a message ('on'), a message and a bell ('both') or nothing ('off')."
        ),
        ..unsafe { zeroed() }
    },
    options_table_entry {
        name: c!("word-separators"),
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_SESSION,
        default_str: Some("!\"#$%&'()*+,-./:;<=>?@[\\]^`{|}~"),
        text: c!("Characters considered to separate words."),
        ..unsafe { zeroed() }
    },
    // Window options
    options_table_entry {
        name: c!("aggressive-resize"),
        type_: options_table_type::OPTIONS_TABLE_FLAG,
        scope: OPTIONS_TABLE_WINDOW,
        default_num: 0,
        text: c!(
            "When 'window-size' is 'smallest', whether the maximum size of a window is the smallest attached session where it is the current window ('on') or the smallest session it is linked to ('off')."
        ),
        ..unsafe { zeroed() }
    },
    options_table_entry {
        name: c!("allow-passthrough"),
        type_: options_table_type::OPTIONS_TABLE_CHOICE,
        scope: OPTIONS_TABLE_WINDOW | OPTIONS_TABLE_PANE,
        choices: &raw const OPTIONS_TABLE_ALLOW_PASSTHROUGH_LIST as *const *const u8,
        default_num: 0,
        text: c!(
            "Whether applications are allowed to use the escape sequence to bypass tmux. Can be 'off' (disallowed), 'on' (allowed if the pane is visible), or 'all' (allowed even if the pane is invisible)."
        ),
        ..unsafe { zeroed() }
    },
    options_table_entry {
        name: c!("allow-rename"),
        type_: options_table_type::OPTIONS_TABLE_FLAG,
        scope: OPTIONS_TABLE_WINDOW | OPTIONS_TABLE_PANE,
        default_num: 0,
        text: c!("Whether applications are allowed to use the escape sequence to rename windows."),
        ..unsafe { zeroed() }
    },
    options_table_entry {
        name: c!("allow-set-title"),
        type_: options_table_type::OPTIONS_TABLE_FLAG,
        scope: OPTIONS_TABLE_WINDOW | OPTIONS_TABLE_PANE,
        default_num: 1,
        text: c!(
            "Whether applications are allowed to use the escape sequence to set the pane title."
        ),
        ..unsafe { zeroed() }
    },
    options_table_entry {
        name: c!("alternate-screen"),
        type_: options_table_type::OPTIONS_TABLE_FLAG,
        scope: OPTIONS_TABLE_WINDOW | OPTIONS_TABLE_PANE,
        default_num: 1,
        text: c!("Whether applications are allowed to use the alternate screen."),
        ..unsafe { zeroed() }
    },
    options_table_entry {
        name: c!("automatic-rename"),
        type_: options_table_type::OPTIONS_TABLE_FLAG,
        scope: OPTIONS_TABLE_WINDOW,
        default_num: 1,
        text: c!("Whether windows are automatically renamed."),
        ..unsafe { zeroed() }
    },
    options_table_entry {
        name: c!("automatic-rename-format"),
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_WINDOW,
        default_str: Some("#{?pane_in_mode,[tmux],#{pane_current_command}}#{?pane_dead,[dead],}"),
        text: c!("Format used to automatically rename windows."),
        ..unsafe { zeroed() }
    },
    options_table_entry {
        name: c!("clock-mode-colour"),
        type_: options_table_type::OPTIONS_TABLE_COLOUR,
        scope: OPTIONS_TABLE_WINDOW,
        default_num: 4,
        text: c!("Colour of the clock in clock mode."),
        ..unsafe { zeroed() }
    },
    options_table_entry {
        name: c!("clock-mode-style"),
        type_: options_table_type::OPTIONS_TABLE_CHOICE,
        scope: OPTIONS_TABLE_WINDOW,
        choices: &raw const OPTIONS_TABLE_CLOCK_MODE_STYLE_LIST as *const *const u8,
        default_num: 1,
        text: c!("Time format of the clock in clock mode."),
        ..unsafe { zeroed() }
    },
    options_table_entry {
        name: c!("copy-mode-match-style"),
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_WINDOW,
        default_str: Some("bg=cyan,fg=black"),
        flags: OPTIONS_TABLE_IS_STYLE,
        separator: c!(","),
        text: c!("Style of search matches in copy mode."),
        ..unsafe { zeroed() }
    },
    options_table_entry {
        name: c!("copy-mode-current-match-style"),
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_WINDOW,
        default_str: Some("bg=magenta,fg=black"),
        flags: OPTIONS_TABLE_IS_STYLE,
        separator: c!(","),
        text: c!("Style of the current search match in copy mode."),
        ..unsafe { zeroed() }
    },
    options_table_entry {
        name: c!("copy-mode-mark-style"),
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_WINDOW,
        default_str: Some("bg=red,fg=black"),
        flags: OPTIONS_TABLE_IS_STYLE,
        separator: c!(","),
        text: c!("Style of the marked line in copy mode."),
        ..unsafe { zeroed() }
    },
    options_table_entry {
        name: c!("fill-character"),
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_WINDOW,
        default_str: Some(""),
        text: c!("Character used to fill unused parts of window."),
        ..unsafe { zeroed() }
    },
    options_table_entry {
        name: c!("main-pane-height"),
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_WINDOW,
        default_str: Some("24"),
        text: c!(
            "Height of the main pane in the 'main-horizontal' layout. This may be a percentage, for example '10%'."
        ),
        ..unsafe { zeroed() }
    },
    options_table_entry {
        name: c!("main-pane-width"),
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_WINDOW,
        default_str: Some("80"),
        text: c!(
            "Width of the main pane in the 'main-vertical' layout. This may be a percentage, for example '10%'."
        ),
        ..unsafe { zeroed() }
    },
    options_table_entry {
        name: c!("mode-keys"),
        type_: options_table_type::OPTIONS_TABLE_CHOICE,
        scope: OPTIONS_TABLE_WINDOW,
        choices: &raw const OPTIONS_TABLE_MODE_KEYS_LIST as *const *const u8,
        default_num: modekey::MODEKEY_EMACS as i64,
        text: c!("Key set used in copy mode."),
        ..unsafe { zeroed() }
    },
    options_table_entry {
        name: c!("mode-style"),
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_WINDOW,
        flags: OPTIONS_TABLE_IS_STYLE,
        default_str: Some("bg=yellow,fg=black"),
        separator: c!(","),
        text: c!("Style of indicators and highlighting in modes."),
        ..unsafe { zeroed() }
    },
    options_table_entry {
        name: c!("monitor-activity"),
        type_: options_table_type::OPTIONS_TABLE_FLAG,
        scope: OPTIONS_TABLE_WINDOW,
        default_num: 0,
        text: c!("Whether an alert is triggered by activity."),
        ..unsafe { zeroed() }
    },
    options_table_entry {
        name: c!("monitor-bell"),
        type_: options_table_type::OPTIONS_TABLE_FLAG,
        scope: OPTIONS_TABLE_WINDOW,
        default_num: 1,
        text: c!("Whether an alert is triggered by a bell."),
        ..unsafe { zeroed() }
    },
    options_table_entry {
        name: c!("monitor-silence"),
        type_: options_table_type::OPTIONS_TABLE_NUMBER,
        scope: OPTIONS_TABLE_WINDOW,
        minimum: 0,
        maximum: i32::MAX as u32,
        default_num: 0,
        text: c!("Time after which an alert is triggered by silence. Zero means no alert."),
        ..unsafe { zeroed() }
    },
    options_table_entry {
        name: c!("other-pane-height"),
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_WINDOW,
        default_str: Some("0"),
        text: c!(
            "Height of the other panes in the 'main-horizontal' layout. This may be a percentage, for example '10%'."
        ),
        ..unsafe { zeroed() }
    },
    options_table_entry {
        name: c!("other-pane-width"),
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_WINDOW,
        default_str: Some("0"),
        text: c!(
            "Height of the other panes in the 'main-vertical' layout. This may be a percentage, for example '10%'."
        ),
        ..unsafe { zeroed() }
    },
    options_table_entry {
        name: c!("pane-active-border-style"),
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_WINDOW,
        default_str: Some("#{?pane_in_mode,fg=yellow,#{?synchronize-panes,fg=red,fg=green}}"),
        flags: OPTIONS_TABLE_IS_STYLE,
        separator: c!(","),
        text: c!("Style of the active pane border."),
        ..unsafe { zeroed() }
    },
    options_table_entry {
        name: c!("pane-base-index"),
        type_: options_table_type::OPTIONS_TABLE_NUMBER,
        scope: OPTIONS_TABLE_WINDOW,
        minimum: 0,
        maximum: u16::MAX as u32,
        default_num: 0,
        text: c!("Index of the first pane in each window."),
        ..unsafe { zeroed() }
    },
    options_table_entry {
        name: c!("pane-border-format"),
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_WINDOW | OPTIONS_TABLE_PANE,
        default_str: Some("#{?pane_active,#[reverse],}#{pane_index}#[default] \"#{pane_title}\""),
        text: c!("Format of text in the pane status lines."),
        ..unsafe { zeroed() }
    },
    options_table_entry {
        name: c!("pane-border-indicators"),
        type_: options_table_type::OPTIONS_TABLE_CHOICE,
        scope: OPTIONS_TABLE_WINDOW,
        choices: &raw const OPTIONS_TABLE_PANE_BORDER_INDICATORS_LIST as *const *const u8,
        default_num: pane_border_indicator::PANE_BORDER_COLOUR as i64,
        text: c!(
            "Whether to indicate the active pane by colouring border or displaying arrow markers."
        ),
        ..unsafe { zeroed() }
    },
    options_table_entry {
        name: c!("pane-border-lines"),
        type_: options_table_type::OPTIONS_TABLE_CHOICE,
        scope: OPTIONS_TABLE_WINDOW,
        choices: &raw const OPTIONS_TABLE_PANE_BORDER_LINES_LIST as *const *const u8,
        default_num: pane_lines::PANE_LINES_SINGLE as i64,
        text: c!(
            "Type of characters used to draw pane border lines. Some of these are only supported on terminals with UTF-8 support."
        ),
        ..unsafe { zeroed() }
    },
    options_table_entry {
        name: c!("pane-border-status"),
        type_: options_table_type::OPTIONS_TABLE_CHOICE,
        scope: OPTIONS_TABLE_WINDOW,
        choices: &raw const OPTIONS_TABLE_PANE_STATUS_LIST as *const *const u8,
        default_num: pane_status::PANE_STATUS_OFF as i64,
        text: c!("Position of the pane status lines."),
        ..unsafe { zeroed() }
    },
    options_table_entry {
        name: c!("pane-border-style"),
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_WINDOW,
        default_str: Some("default"),
        flags: OPTIONS_TABLE_IS_STYLE,
        separator: c!(","),
        text: c!("Style of the pane status lines."),
        ..unsafe { zeroed() }
    },
    options_table_entry {
        name: c!("pane-colours"),
        type_: options_table_type::OPTIONS_TABLE_COLOUR,
        scope: OPTIONS_TABLE_WINDOW | OPTIONS_TABLE_PANE,
        default_str: Some(""),
        flags: OPTIONS_TABLE_IS_ARRAY,
        text: c!("The default colour palette for colours zero to 255."),
        ..unsafe { zeroed() }
    },
    options_table_entry {
        name: c!("popup-style"),
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_WINDOW,
        default_str: Some("default"),
        flags: OPTIONS_TABLE_IS_STYLE,
        separator: c!(","),
        text: c!("Default style of popups."),
        ..unsafe { zeroed() }
    },
    options_table_entry {
        name: c!("popup-border-style"),
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_WINDOW,
        default_str: Some("default"),
        flags: OPTIONS_TABLE_IS_STYLE,
        separator: c!(","),
        text: c!("Default style of popup borders."),
        ..unsafe { zeroed() }
    },
    options_table_entry {
        name: c!("popup-border-lines"),
        type_: options_table_type::OPTIONS_TABLE_CHOICE,
        scope: OPTIONS_TABLE_WINDOW,
        choices: &raw const OPTIONS_TABLE_POPUP_BORDER_LINES_LIST as *const *const u8,
        default_num: box_lines::BOX_LINES_SINGLE as i64,
        text: c!(
            "Type of characters used to draw popup border lines. Some of these are only supported on terminals with UTF-8 support."
        ),
        ..unsafe { zeroed() }
    },
    options_table_entry {
        name: c!("remain-on-exit"),
        type_: options_table_type::OPTIONS_TABLE_CHOICE,
        scope: OPTIONS_TABLE_WINDOW | OPTIONS_TABLE_PANE,
        choices: &raw const OPTIONS_TABLE_REMAIN_ON_EXIT_LIST as *const *const u8,
        default_num: 0,
        text: c!(
            "Whether panes should remain ('on') or be automatically killed ('off' or 'failed') when the program inside exits."
        ),
        ..unsafe { zeroed() }
    },
    options_table_entry {
        name: c!("remain-on-exit-format"),
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_WINDOW | OPTIONS_TABLE_PANE,
        default_str: Some(
            "Pane is dead (#{?#{!=:#{pane_dead_status},},status #{pane_dead_status},}#{?#{!=:#{pane_dead_signal},},signal #{pane_dead_signal},}, #{t:pane_dead_time})",
        ),
        text: c!(
            "Message shown after the program in a pane has exited, if remain-on-exit is enabled."
        ),
        ..unsafe { zeroed() }
    },
    options_table_entry {
        name: c!("scroll-on-clear"),
        type_: options_table_type::OPTIONS_TABLE_FLAG,
        scope: OPTIONS_TABLE_WINDOW | OPTIONS_TABLE_PANE,
        default_num: 1,
        text: c!(
            "Whether the contents of the screen should be scrolled into history when clearing the whole screen."
        ),
        ..unsafe { zeroed() }
    },
    options_table_entry {
        name: c!("synchronize-panes"),
        type_: options_table_type::OPTIONS_TABLE_FLAG,
        scope: OPTIONS_TABLE_WINDOW | OPTIONS_TABLE_PANE,
        default_num: 0,
        text: c!("Whether typing should be sent to all panes simultaneously."),
        ..unsafe { zeroed() }
    },
    options_table_entry {
        name: c!("window-active-style"),
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_WINDOW | OPTIONS_TABLE_PANE,
        default_str: Some("default"),
        flags: OPTIONS_TABLE_IS_STYLE,
        separator: c!(","),
        text: c!("Default style of the active pane."),
        ..unsafe { zeroed() }
    },
    options_table_entry {
        name: c!("window-size"),
        type_: options_table_type::OPTIONS_TABLE_CHOICE,
        scope: OPTIONS_TABLE_WINDOW,
        choices: &raw const OPTIONS_TABLE_WINDOW_SIZE_LIST as *const *const u8,
        default_num: window_size_option::WINDOW_SIZE_LATEST as i64,
        text: c!(
            "How window size is calculated. 'latest' uses the size of the most recently used client, 'largest' the largest client, 'smallest' the smallest client and 'manual' a size set by the 'resize-window' command."
        ),
        ..unsafe { zeroed() }
    },
    options_table_entry {
        name: c!("window-style"),
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_WINDOW | OPTIONS_TABLE_PANE,
        default_str: Some("default"),
        flags: OPTIONS_TABLE_IS_STYLE,
        separator: c!(","),
        text: c!("Default style of panes that are not the active pane."),
        ..unsafe { zeroed() }
    },
    options_table_entry {
        name: c!("window-status-activity-style"),
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_WINDOW,
        default_str: Some("reverse"),
        flags: OPTIONS_TABLE_IS_STYLE,
        separator: c!(","),
        text: c!("Style of windows in the status line with an activity alert."),
        ..unsafe { zeroed() }
    },
    options_table_entry {
        name: c!("window-status-bell-style"),
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_WINDOW,
        default_str: Some("reverse"),
        flags: OPTIONS_TABLE_IS_STYLE,
        separator: c!(","),
        text: c!("Style of windows in the status line with a bell alert."),
        ..unsafe { zeroed() }
    },
    options_table_entry {
        name: c!("window-status-current-format"),
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_WINDOW,
        default_str: Some("#I:#W#{?window_flags,#{window_flags}, }"),
        text: c!("Format of the current window in the status line."),
        ..unsafe { zeroed() }
    },
    options_table_entry {
        name: c!("window-status-current-style"),
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_WINDOW,
        default_str: Some("default"),
        flags: OPTIONS_TABLE_IS_STYLE,
        separator: c!(","),
        text: c!("Style of the current window in the status line."),
        ..unsafe { zeroed() }
    },
    options_table_entry {
        name: c!("window-status-format"),
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_WINDOW,
        default_str: Some("#I:#W#{?window_flags,#{window_flags}, }"),
        text: c!("Format of windows in the status line, except the current window."),
        ..unsafe { zeroed() }
    },
    options_table_entry {
        name: c!("window-status-last-style"),
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_WINDOW,
        default_str: Some("default"),
        flags: OPTIONS_TABLE_IS_STYLE,
        separator: c!(","),
        text: c!("Style of the last window in the status line."),
        ..unsafe { zeroed() }
    },
    options_table_entry {
        name: c!("window-status-separator"),
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_WINDOW,
        default_str: Some(" "),
        text: c!("Separator between windows in the status line."),
        ..unsafe { zeroed() }
    },
    options_table_entry {
        name: c!("window-status-style"),
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_WINDOW,
        default_str: Some("default"),
        flags: OPTIONS_TABLE_IS_STYLE,
        separator: c!(","),
        text: c!("Style of windows in the status line, except the current and last windows."),
        ..unsafe { zeroed() }
    },
    options_table_entry {
        name: c!("wrap-search"),
        type_: options_table_type::OPTIONS_TABLE_FLAG,
        scope: OPTIONS_TABLE_WINDOW,
        default_num: 1,
        text: c!("Whether searching in copy mode should wrap at the top or bottom."),
        ..unsafe { zeroed() }
    },
    options_table_entry {
        name: c!("xterm-keys"),
        type_: options_table_type::OPTIONS_TABLE_FLAG,
        scope: OPTIONS_TABLE_WINDOW,
        default_num: 1,
        text: c!(
            "Whether xterm-style function key sequences should be sent. This option is no longer used."
        ),
        ..unsafe { zeroed() }
    },
    // Hook options.
    options_table_hook!(c"after-bind-key", ""),
    options_table_hook!(c"after-capture-pane", ""),
    options_table_hook!(c"after-copy-mode", ""),
    options_table_hook!(c"after-display-message", ""),
    options_table_hook!(c"after-display-panes", ""),
    options_table_hook!(c"after-kill-pane", ""),
    options_table_hook!(c"after-list-buffers", ""),
    options_table_hook!(c"after-list-clients", ""),
    options_table_hook!(c"after-list-keys", ""),
    options_table_hook!(c"after-list-panes", ""),
    options_table_hook!(c"after-list-sessions", ""),
    options_table_hook!(c"after-list-windows", ""),
    options_table_hook!(c"after-load-buffer", ""),
    options_table_hook!(c"after-lock-server", ""),
    options_table_hook!(c"after-new-session", ""),
    options_table_hook!(c"after-new-window", ""),
    options_table_hook!(c"after-paste-buffer", ""),
    options_table_hook!(c"after-pipe-pane", ""),
    options_table_hook!(c"after-queue", ""),
    options_table_hook!(c"after-refresh-client", ""),
    options_table_hook!(c"after-rename-session", ""),
    options_table_hook!(c"after-rename-window", ""),
    options_table_hook!(c"after-resize-pane", ""),
    options_table_hook!(c"after-resize-window", ""),
    options_table_hook!(c"after-save-buffer", ""),
    options_table_hook!(c"after-select-layout", ""),
    options_table_hook!(c"after-select-pane", ""),
    options_table_hook!(c"after-select-window", ""),
    options_table_hook!(c"after-send-keys", ""),
    options_table_hook!(c"after-set-buffer", ""),
    options_table_hook!(c"after-set-environment", ""),
    options_table_hook!(c"after-set-hook", ""),
    options_table_hook!(c"after-set-option", ""),
    options_table_hook!(c"after-show-environment", ""),
    options_table_hook!(c"after-show-messages", ""),
    options_table_hook!(c"after-show-options", ""),
    options_table_hook!(c"after-split-window", ""),
    options_table_hook!(c"after-unbind-key", ""),
    options_table_hook!(c"alert-activity", ""),
    options_table_hook!(c"alert-bell", ""),
    options_table_hook!(c"alert-silence", ""),
    options_table_hook!(c"client-active", ""),
    options_table_hook!(c"client-attached", ""),
    options_table_hook!(c"client-detached", ""),
    options_table_hook!(c"client-focus-in", ""),
    options_table_hook!(c"client-focus-out", ""),
    options_table_hook!(c"client-resized", ""),
    options_table_hook!(c"client-session-changed", ""),
    options_table_hook!(c"command-error", ""),
    options_table_pane_hook!(c"pane-died", ""),
    options_table_pane_hook!(c"pane-exited", ""),
    options_table_pane_hook!(c"pane-fous-in", ""),
    options_table_pane_hook!(c"pane-fous-out", ""),
    options_table_pane_hook!(c"pane-mode-hanged", ""),
    options_table_pane_hook!(c"pane-set-lipboard", ""),
    options_table_pane_hook!(c"pane-title-hanged", ""),
    options_table_hook!(c"session-closed", ""),
    options_table_hook!(c"session-created", ""),
    options_table_hook!(c"session-renamed", ""),
    options_table_hook!(c"session-window-changed", ""),
    options_table_window_hook!(c"window-layout-changed", ""),
    options_table_hook!(c"window-linked", ""),
    options_table_window_hook!(c"window-pane-changed", ""),
    options_table_window_hook!(c"window-renamed", ""),
    options_table_window_hook!(c"window-resized", ""),
    options_table_hook!(c"window-unlinked", ""),
    options_table_entry {
        name: null(),
        ..unsafe { zeroed() }
    },
];

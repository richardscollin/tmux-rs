#[cfg(unix)]
mod common;

#[cfg(unix)]
mod regress {
    pub use super::common::*;

    mod am_terminal;
    mod capture_pane;
    mod combine;
    mod command_order;
    mod conf_syntax;
    mod control_client;
    mod copy_mode;
    mod cursor;
    mod format_strings;
    mod has_session;
    mod if_shell;
    mod input_keys;
    mod kill_session;
    mod new_session;
    mod new_window;
    mod osc;
    mod run_shell;
    mod style_trim;
    mod tty_keys;
    mod utf8;
}

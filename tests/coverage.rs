#[cfg(unix)]
mod common;

#[cfg(unix)]
mod coverage {
    pub use super::common::*;

    mod alerts;
    mod bind_key;
    mod break_pane;
    mod cmd_choose_tree;
    mod copy_mode;
    mod cmd_confirm_before;
    mod cmd_display_menu;
    mod cmd_display_message;
    mod cmd_display_panes;
    mod cmd_find_window;
    mod if_shell;
    mod cmd_kill_session;
    mod cmd_kill_window;
    mod cmd_list_keys;
    mod cmd_list_panes;
    mod cmd_refresh_client;
    mod command_prompt;
    mod detach_client;
    mod join_pane;
    mod kill_pane;
    mod list_buffers;
    mod list_clients;
    mod list_sessions;
    mod list_windows;
    mod load_buffer;
    mod lock_server;
    mod move_window;
    mod paste_buffer;
    mod pipe_pane;
    mod rename_window;
    mod resize_pane;
    mod resize_window;
    mod respawn_pane;
    mod respawn_window;
    mod rotate_window;
    mod save_buffer;
    mod select_layout;
    mod select_window;
    mod server_access;
    mod set_buffer;
    mod set_environment;
    mod show_environment;
    mod show_messages;
    mod show_prompt_history;
    mod status;
    mod swap_pane;
    mod swap_window;
    mod unbind_key;
    mod window_clock;
    mod window_copy;
}

use super::*;

/// Test display-message variants and status option manipulation.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn status_display_and_options() {
    let tmux = TmuxServer::new("stat_dispopts");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // display-message -p prints to stdout
    let out = tmux.display("hello world");
    assert_eq!(out, "hello world");

    // display-message with format strings
    let out = tmux.display("#{session_name}");
    assert!(!out.is_empty());

    // display-message with -d (custom delay)
    tmux.run(&["display-message", "-d", "100", "short delay"]);
    tmux.run(&["display-message", "-d", "0", "no delay"]);

    // display-message -N (ignore keys branch)
    tmux.run(&["display-message", "-N", "press any key"]);

    // display-time option (delay=-1 path in status_message_set_)
    tmux.run(&["set", "-g", "display-time", "500"]);
    tmux.run(&["display-message", "using display-time"]);

    // Status on/off (exercises status_update_cache)
    let out = tmux.display("#{status}");
    assert_eq!(out, "on");
    tmux.run(&["set", "-g", "status", "off"]);
    let out = tmux.display("#{status}");
    assert_eq!(out, "off");
    tmux.run(&["set", "-g", "status", "on"]);

    // Status position top vs bottom
    tmux.run(&["set", "-g", "status-position", "top"]);
    assert_eq!(tmux.display("#{status-position}"), "top");
    tmux.run(&["set", "-g", "status-position", "bottom"]);
    assert_eq!(tmux.display("#{status-position}"), "bottom");

    // Status interval (exercises status_timer_start)
    tmux.run(&["set", "-g", "status-interval", "0"]);
    assert_eq!(tmux.display("#{status-interval}"), "0");
    tmux.run(&["set", "-g", "status-interval", "5"]);
    assert_eq!(tmux.display("#{status-interval}"), "5");

    // Message-line option (exercises status_prompt_line_at)
    tmux.run(&["set", "-g", "message-line", "0"]);
    tmux.run(&["display-message", "line 0"]);
    tmux.run(&["set", "-g", "message-line", "1"]);
    tmux.run(&["display-message", "line 1"]);
}

/// Test status-format, multiline, and style options.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn status_format_and_styles() {
    let tmux = TmuxServer::new("stat_fmtstyle");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Custom status-format
    tmux.run(&[
        "set",
        "-g",
        "status-format[0]",
        "#[fg=green]#S #[default]#W",
    ]);
    let out = tmux.run(&["show", "-g", "status-format[0]"]);
    assert!(out.contains("#S"));

    // Multi-line status
    tmux.run(&["set", "-g", "status", "2"]);
    tmux.run(&["set", "-g", "status-format[1]", "second line: #S"]);
    tmux.run(&["set", "-g", "status", "3"]);
    tmux.run(&["set", "-g", "status", "on"]);

    // Status style
    tmux.run(&["set", "-g", "status-style", "bg=red,fg=white"]);
    tmux.run(&["set", "-g", "status-fg", "green"]);
    tmux.run(&["set", "-g", "status-bg", "blue"]);

    // Message style
    tmux.run(&["set", "-g", "message-style", "bg=yellow"]);
    tmux.run(&["display-message", "styled message"]);
}

/// Test prompt history load/save with a config that sets history-file.
/// Exercises status_prompt_find_history_file (absolute path),
/// status_prompt_load_history, status_prompt_add_typed_history,
/// status_prompt_add_history, and status_prompt_save_history on exit.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn status_prompt_history_load_save() {
    let tmux = TmuxServer::new("stat_histload");

    // Create a history file with typed entries, backward-compatible entries, and invalid types
    let histfile = tmux.write_temp(
        "command:ls -la\nsearch:pattern\ntarget:mysess\nwindow-target:mysess:0\nold_entry_no_type\n",
    );

    // Create a config file that sets history-file to our temp file
    let conf = tmux.write_temp(&format!(
        "set -g history-file {}\nset -g prompt-history-limit 100\n",
        histfile.path_str()
    ));

    // Start server with that config - this triggers status_prompt_load_history
    let confpath = format!("-f{}", conf.path_str());
    tmux.run(&[&confpath, "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Verify history was loaded - showphist should show our entries
    let out = tmux.run(&["showphist", "-T", "command"]);
    assert!(
        out.contains("ls -la"),
        "expected 'ls -la' in command history, got: {out}"
    );

    let out = tmux.run(&["showphist", "-T", "search"]);
    assert!(
        out.contains("pattern"),
        "expected 'pattern' in search history, got: {out}"
    );

    // kill-server triggers status_prompt_save_history which writes back to the file
    tmux.kill_server();

    // Verify the history file was written with typed format
    let saved = histfile.read_to_string();
    assert!(
        saved.contains("command:"),
        "expected 'command:' prefix in saved history, got: {saved}"
    );
}

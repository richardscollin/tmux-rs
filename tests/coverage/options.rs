use super::*;

/// Test various option operations to exercise options_.rs paths.

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn options_array_set() {
    let tmux = TmuxServer::new("opts_array");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // terminal-overrides is an array option
    tmux.run(&["set", "-g", "-a", "terminal-overrides", ",xterm:Tc"]);
    let out = tmux.run(&["show", "-gv", "terminal-overrides"]);
    assert!(out.contains("xterm:Tc"), "got: {out}");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn options_array_index() {
    let tmux = TmuxServer::new("opts_arridx");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Set specific array index
    tmux.run(&["set", "-g", "terminal-overrides[0]", "xterm:Tc"]);
    let out = tmux.run(&["show", "-gv", "terminal-overrides[0]"]);
    assert!(out.contains("xterm:Tc"), "got: {out}");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn options_unset_array() {
    let tmux = TmuxServer::new("opts_unsarr");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Unset array option
    tmux.run(&["set", "-g", "terminal-overrides[0]", "test:val"]);
    tmux.run(&["set", "-gu", "terminal-overrides[0]"]);
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn options_number() {
    let tmux = TmuxServer::new("opts_num");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Number option
    tmux.run(&["set", "-g", "history-limit", "5000"]);
    let out = tmux.run(&["show", "-gv", "history-limit"]);
    assert_eq!(out.trim(), "5000");

    // Invalid number
    let out = tmux.try_run(&["set", "-g", "history-limit", "notanumber"]);
    assert!(!out.status.success());
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn options_choice() {
    let tmux = TmuxServer::new("opts_choice");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Choice option (status)
    tmux.run(&["set", "-g", "status", "off"]);
    let out = tmux.run(&["show", "-gv", "status"]);
    assert_eq!(out.trim(), "off");

    tmux.run(&["set", "-g", "status", "on"]);
    let out = tmux.run(&["show", "-gv", "status"]);
    assert_eq!(out.trim(), "on");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn options_colour() {
    let tmux = TmuxServer::new("opts_colour");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Colour option
    tmux.run(&["set", "-g", "display-panes-active-colour", "red"]);
    let out = tmux.run(&["show", "-gv", "display-panes-active-colour"]);
    assert_eq!(out.trim(), "red");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn options_key() {
    let tmux = TmuxServer::new("opts_key");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Key option
    tmux.run(&["set", "-g", "prefix", "C-a"]);
    let out = tmux.run(&["show", "-gv", "prefix"]);
    assert_eq!(out.trim(), "C-a");

    // Set back
    tmux.run(&["set", "-g", "prefix", "C-b"]);
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn options_style() {
    let tmux = TmuxServer::new("opts_style");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Style option
    tmux.run(&["set", "-g", "status-style", "bg=blue,fg=white"]);
    let out = tmux.run(&["show", "-gv", "status-style"]);
    assert!(out.contains("blue"), "got: {out}");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn options_append_string() {
    let tmux = TmuxServer::new("opts_append");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Append to a style option
    tmux.run(&["set", "-g", "status-style", "bg=blue"]);
    tmux.run(&["set", "-ga", "status-style", ",fg=white"]);
    let out = tmux.run(&["show", "-gv", "status-style"]);
    assert!(out.contains("blue") && out.contains("white"), "got: {out}");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn options_hook() {
    let tmux = TmuxServer::new("opts_hook");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Set a hook
    tmux.run(&["set-hook", "-g", "after-new-window", "set -g @hook_fired yes"]);

    // Trigger the hook
    tmux.run(&["new-window"]);
    sleep_ms(100);

    let out = tmux.run(&["show", "-gv", "@hook_fired"]);
    assert_eq!(out.trim(), "yes");

    // Unset the hook
    tmux.run(&["set-hook", "-gu", "after-new-window"]);
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn options_window_scope() {
    let tmux = TmuxServer::new("opts_wscope");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Window option (not global)
    tmux.run(&["setw", "automatic-rename", "off"]);
    let out = tmux.run(&["showw", "-v", "automatic-rename"]);
    assert_eq!(out.trim(), "off");

    // Unset window option (falls back to global)
    tmux.run(&["setw", "-u", "automatic-rename"]);
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn options_pane_scope() {
    let tmux = TmuxServer::new("opts_pscope");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Pane option
    tmux.run(&["set", "-p", "remain-on-exit", "on"]);
    let out = tmux.run(&["show", "-pv", "remain-on-exit"]);
    assert_eq!(out.trim(), "on");

    tmux.run(&["set", "-pu", "remain-on-exit"]);
}

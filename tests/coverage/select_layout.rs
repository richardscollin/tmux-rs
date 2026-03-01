use super::*;

/// Select a named layout (even-horizontal).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn select_layout_named() {
    let tmux = TmuxServer::new("selectl_named");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["splitw", "-d"]);
    tmux.run(&["selectl", "even-horizontal"]);
    let layout = tmux.display("#{window_layout}");
    assert!(!layout.trim().is_empty());
}

/// Select layout with -n (next layout).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn select_layout_next_flag() {
    let tmux = TmuxServer::new("selectl_n");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["splitw", "-d"]);
    let before = tmux.display("#{window_layout}");
    tmux.run(&["selectl", "-n"]);
    let after = tmux.display("#{window_layout}");
    assert_ne!(before.trim(), after.trim(), "layout should change with -n");
}

/// Select layout with -p (previous layout).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn select_layout_previous_flag() {
    let tmux = TmuxServer::new("selectl_p");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["splitw", "-d"]);
    tmux.run(&["selectl", "-p"]);
}

/// next-layout command.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn select_layout_next_command() {
    let tmux = TmuxServer::new("selectl_nextl");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["splitw", "-d"]);
    tmux.run(&["nextl"]);
}

/// previous-layout command.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn select_layout_previous_command() {
    let tmux = TmuxServer::new("selectl_prevl");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["splitw", "-d"]);
    tmux.run(&["prevl"]);
}

/// Select layout with -E (spread out evenly).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn select_layout_spread() {
    let tmux = TmuxServer::new("selectl_e");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["splitw", "-d"]);
    tmux.run(&["splitw", "-d"]);
    tmux.run(&["selectl", "-E"]);
}

/// Select layout with -o (restore old layout).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn select_layout_old() {
    let tmux = TmuxServer::new("selectl_o");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["splitw", "-d"]);
    // Set a layout, then change it, then use -o to restore
    tmux.run(&["selectl", "even-horizontal"]);
    tmux.run(&["selectl", "even-vertical"]);
    tmux.run(&["selectl", "-o"]);
}

/// Select layout with explicit layout string (custom layout).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn select_layout_custom_string() {
    let tmux = TmuxServer::new("selectl_custom");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["splitw", "-d"]);
    // Get current layout string, then re-apply it
    let layout = tmux.display("#{window_layout}");
    tmux.run(&["selectl", layout.trim()]);
}

/// Select layout with invalid layout string (error path).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn select_layout_invalid_string() {
    let tmux = TmuxServer::new("selectl_invalid");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let result = tmux.try_run(&["selectl", "invalid-layout-string"]);
    assert!(!result.status.success());
}

/// Select layout with no args and no previous layout (no-op path).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn select_layout_no_args() {
    let tmux = TmuxServer::new("selectl_noargs");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // No args, no -o: uses lastlayout which may be -1
    tmux.run(&["selectl"]);
}

/// Select layout using lastlayout (set by previous selectl with named layout).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn select_layout_lastlayout() {
    let tmux = TmuxServer::new("selectl_last");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["splitw", "-d"]);
    // Set a named layout so lastlayout gets set
    tmux.run(&["selectl", "even-horizontal"]);
    // Now call with no args to use lastlayout
    tmux.run(&["selectl"]);
}

use super::*;

/// Test new-window: basic, named, detached, -S (select existing),
/// -a/-b (after/before), -k (kill), -P (print), -e (env), -c (cwd).

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn new_window_basic() {
    let tmux = TmuxServer::new("neww_basic");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["new-window"]);
    let count = tmux.display("#{session_windows}");
    assert_eq!(count, "2");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn new_window_named() {
    let tmux = TmuxServer::new("neww_named");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["new-window", "-n", "mywin"]);
    let name = tmux.display("#{window_name}");
    assert_eq!(name, "mywin");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn new_window_detached() {
    let tmux = TmuxServer::new("neww_detach");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let idx_before = tmux.display("#{window_index}");
    tmux.run(&["new-window", "-d"]);
    let idx_after = tmux.display("#{window_index}");
    // Current window should not change with -d
    assert_eq!(idx_before, idx_after);
    let count = tmux.display("#{session_windows}");
    assert_eq!(count, "2");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn new_window_select_existing() {
    let tmux = TmuxServer::new("neww_select");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Create a named window
    tmux.run(&["new-window", "-d", "-n", "target"]);
    let count1 = tmux.display("#{session_windows}");
    assert_eq!(count1, "2");

    // -S -n target: select existing window instead of creating new
    tmux.run(&["new-window", "-S", "-n", "target"]);
    let count2 = tmux.display("#{session_windows}");
    assert_eq!(count2, "2", "should not create a new window with -S");
    let name = tmux.display("#{window_name}");
    assert_eq!(name, "target", "should select existing window");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn new_window_select_existing_detached() {
    let tmux = TmuxServer::new("neww_sel_d");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["new-window", "-d", "-n", "existing"]);

    // -S -d -n existing: find but don't select
    let idx_before = tmux.display("#{window_index}");
    tmux.run(&["new-window", "-S", "-d", "-n", "existing"]);
    let idx_after = tmux.display("#{window_index}");
    assert_eq!(idx_before, idx_after, "should not switch with -S -d");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn new_window_after() {
    let tmux = TmuxServer::new("neww_after");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["new-window"]);
    tmux.run(&["new-window"]);

    // Go to window 0, create window -a (after current)
    tmux.run(&["select-window", "-t", ":0"]);
    tmux.run(&["new-window", "-a", "-n", "after0"]);
    // New window should be at index 1
    let name = tmux.run(&["display-message", "-t", ":1", "-p", "#{window_name}"]);
    assert_eq!(name.trim(), "after0");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn new_window_before() {
    let tmux = TmuxServer::new("neww_before");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["new-window"]);

    // Go to window 1, create window -b (before current)
    tmux.run(&["select-window", "-t", ":1"]);
    tmux.run(&["new-window", "-b", "-n", "before1"]);
    let name = tmux.run(&["display-message", "-t", ":1", "-p", "#{window_name}"]);
    assert_eq!(name.trim(), "before1");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn new_window_kill() {
    let tmux = TmuxServer::new("neww_kill");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["new-window"]);

    // -k: kill existing window at target index
    tmux.run(&["new-window", "-k", "-t", ":1", "-n", "replaced"]);
    let name = tmux.run(&["display-message", "-t", ":1", "-p", "#{window_name}"]);
    assert_eq!(name.trim(), "replaced");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn new_window_print() {
    let tmux = TmuxServer::new("neww_print");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // -P: print window info
    let out = tmux.run(&["new-window", "-P"]);
    // Should contain session:window.pane format
    assert!(out.contains(":"), "expected session:window.pane format, got: {out}");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn new_window_print_format() {
    let tmux = TmuxServer::new("neww_pfmt");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // -P -F: print with custom format
    let out = tmux.run(&["new-window", "-P", "-F", "#{window_index}"]);
    assert_eq!(out.trim(), "1");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn new_window_environment() {
    let tmux = TmuxServer::new("neww_env");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // -e: set environment variable for the new window
    tmux.run(&["new-window", "-e", "MYVAR=hello"]);
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn new_window_cwd() {
    let tmux = TmuxServer::new("neww_cwd");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // -c: set working directory
    tmux.run(&["new-window", "-c", "/tmp"]);
}

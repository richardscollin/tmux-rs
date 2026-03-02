use super::*;

/// Basic new-window creates a second window.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn new_window_basic() {
    let tmux = TmuxServer::new("neww_basic");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["neww"]);
    let count = tmux.display("#{session_windows}");
    assert_eq!(count.trim(), "2");
}

/// New window with -d (detached, don't select).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn new_window_detached() {
    let tmux = TmuxServer::new("neww_detach");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Current window should stay at index 0 after -d
    tmux.run(&["neww", "-d"]);
    let cur = tmux.display("#{window_index}");
    assert_eq!(cur.trim(), "0");
    let count = tmux.display("#{session_windows}");
    assert_eq!(count.trim(), "2");
}

/// New window with -n (name).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn new_window_named() {
    let tmux = TmuxServer::new("neww_named");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["neww", "-n", "mywin"]);
    let name = tmux.display("#{window_name}");
    assert_eq!(name.trim(), "mywin");
}

/// New window with -a (insert after current).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn new_window_after() {
    let tmux = TmuxServer::new("neww_after");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["neww", "-d"]);  // window 1
    tmux.run(&["neww", "-d"]);  // window 2

    // Select window 0, then insert after it with -a
    tmux.run(&["selectw", "-t", ":0"]);
    tmux.run(&["neww", "-a", "-n", "inserted"]);

    // The new window should be at index 1
    let name = tmux.run(&["display", "-t", ":1", "-p", "#{window_name}"]);
    assert_eq!(name.trim(), "inserted");
}

/// New window with -b (insert before current).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn new_window_before() {
    let tmux = TmuxServer::new("neww_before");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Select window 0, insert before it with -b
    tmux.run(&["neww", "-b", "-n", "before_win"]);
    // The new window should now be at index 0
    let name = tmux.run(&["display", "-t", ":0", "-p", "#{window_name}"]);
    assert_eq!(name.trim(), "before_win");
}

/// New window with -k (kill existing window at target index).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn new_window_kill() {
    let tmux = TmuxServer::new("neww_kill");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["neww", "-d", "-n", "victim"]);

    let id_before = tmux.run(&["display", "-t", ":1", "-p", "#{window_id}"]);

    // Kill window at index 1 and replace it
    tmux.run(&["neww", "-k", "-t", ":1", "-n", "replacement"]);
    let name = tmux.run(&["display", "-t", ":1", "-p", "#{window_name}"]);
    let id_after = tmux.run(&["display", "-t", ":1", "-p", "#{window_id}"]);

    assert_eq!(name.trim(), "replacement");
    assert_ne!(id_before.trim(), id_after.trim(), "window should have been replaced");
}

/// New window with -e (environment variable).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn new_window_environment() {
    let tmux = TmuxServer::new("neww_env");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["neww", "-d", "-e", "MYVAR=hello", "-e", "OTHER=world"]);
    let count = tmux.display("#{session_windows}");
    assert_eq!(count.trim(), "2");
}

/// New window with -P (print window info).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn new_window_print() {
    let tmux = TmuxServer::new("neww_print");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let out = tmux.run(&["neww", "-d", "-P"]);
    // Default template: #{session_name}:#{window_index}.#{pane_index}
    assert!(
        out.contains(":"),
        "expected session:window.pane format, got: {out}"
    );
}

/// New window with -P -F (custom format).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn new_window_print_format() {
    let tmux = TmuxServer::new("neww_print_fmt");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let out = tmux.run(&["neww", "-d", "-P", "-F", "#{window_id}"]);
    assert!(
        out.trim().starts_with('@'),
        "expected window id starting with @, got: {out}"
    );
}

/// New window with -S -n (select existing window by name, single match).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn new_window_select_existing() {
    let tmux = TmuxServer::new("neww_S_select");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["neww", "-d", "-n", "target"]);

    let count_before = tmux.display("#{session_windows}");

    // -S with existing name should select it, not create a new one
    tmux.run(&["neww", "-S", "-n", "target"]);

    let count_after = tmux.display("#{session_windows}");
    assert_eq!(count_before.trim(), count_after.trim(), "should not create a new window");

    let cur_name = tmux.display("#{window_name}");
    assert_eq!(cur_name.trim(), "target", "should have selected the existing window");
}

/// New window with -S -n -d (existing window, detached -- noop).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn new_window_select_existing_detached() {
    let tmux = TmuxServer::new("neww_S_d");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["neww", "-d", "-n", "target"]);

    // -S -d with existing name: early return, don't select
    tmux.run(&["neww", "-S", "-n", "target", "-d"]);
    let cur = tmux.display("#{window_index}");
    assert_eq!(cur.trim(), "0", "should not have switched window");
}

/// New window with -S -n where no window matches (creates new).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn new_window_select_no_match() {
    let tmux = TmuxServer::new("neww_S_nomatch");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // -S with a name that doesn't exist creates a new window
    tmux.run(&["neww", "-S", "-n", "brandnew"]);
    let count = tmux.display("#{session_windows}");
    assert_eq!(count.trim(), "2");
    let name = tmux.display("#{window_name}");
    assert_eq!(name.trim(), "brandnew");
}

/// New window with -S -n where multiple windows match (error).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn new_window_select_multiple() {
    let tmux = TmuxServer::new("neww_S_multi");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Create two windows with the same name
    tmux.run(&["neww", "-d", "-n", "dupname"]);
    tmux.run(&["neww", "-d", "-n", "dupname"]);

    let result = tmux.try_run(&["neww", "-S", "-n", "dupname"]);
    assert!(!result.status.success());
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        stderr.contains("multiple windows"),
        "expected 'multiple windows' error, got: {stderr}"
    );
}

/// New window with shell command.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn new_window_with_command() {
    let tmux = TmuxServer::new("neww_cmd");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["neww", "-d", "sleep", "100"]);
    let count = tmux.display("#{session_windows}");
    assert_eq!(count.trim(), "2");
}

/// New window with -c (start directory).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn new_window_start_directory() {
    let tmux = TmuxServer::new("neww_cwd");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["neww", "-c", "/tmp"]);
    let cwd = tmux.display("#{pane_current_path}");
    assert_eq!(cwd.trim(), "/tmp");
}

/// New window using 'neww' alias.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn new_window_alias() {
    let tmux = TmuxServer::new("neww_alias");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["neww", "-d"]);
    let count = tmux.display("#{session_windows}");
    assert_eq!(count.trim(), "2");
}

/// New window with -a: coverage shows args_has('a') is never true despite
/// the test passing and -a being in the args_parse spec. The flag is parsed
/// and the window IS inserted after the current one (verifiable by index),
/// but llvm-cov does not attribute the branch as taken.
#[test]
#[ignore = "coverage bug: -a flag never shows as true in llvm-cov despite correct behavior"]
fn new_window_after_coverage_bug() {
    let tmux = TmuxServer::new("neww_a_bug");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    // Create windows 1 and 2
    tmux.run(&["neww", "-d", "-n", "win1"]);
    tmux.run(&["neww", "-d", "-n", "win2"]);

    // Select window 0, then neww -a should insert at index 1
    tmux.run(&["selectw", "-t", ":0"]);
    tmux.run(&["new-window", "-a", "-n", "after_zero"]);

    // Verify the window was inserted at the right position
    let name_at_1 = tmux.run(&["display", "-t", ":1", "-p", "#{window_name}"]);
    assert_eq!(name_at_1.trim(), "after_zero", "-a should insert after current window");
}

/// Trigger spawn_window failure by setting default-shell to a nonexistent path.
/// spawn_pane falls back to _PATH_BSHELL (/bin/sh) when checkshell_ fails,
/// so the error path in cmd_new_window (lines 117-123) is not reachable via CLI.
#[test]
#[ignore = "untestable: spawn_pane falls back to /bin/sh for invalid default-shell"]
fn new_window_spawn_failure() {
    let tmux = TmuxServer::new("neww_spawn_fail");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["set", "-g", "default-shell", "/nonexistent/shell/xyz"]);
    let result = tmux.try_run(&["neww"]);
    assert!(
        !result.status.success(),
        "expected spawn failure with nonexistent shell"
    );
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        stderr.contains("create window failed"),
        "expected 'create window failed' error, got: {stderr}"
    );
}

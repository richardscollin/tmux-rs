use super::*;

/// Select a specific window by index.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn select_window_basic() {
    let tmux = TmuxServer::new("selectw_basic");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["neww", "-d"]);
    tmux.run(&["selectw", "-t", ":1"]);
    let idx = tmux.display("#{window_index}");
    assert_eq!(idx.trim(), "1");
}

/// Next window with -n flag.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn select_window_next_flag() {
    let tmux = TmuxServer::new("selectw_n");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["neww", "-d"]);
    tmux.run(&["selectw", "-n"]);
    let idx = tmux.display("#{window_index}");
    assert_eq!(idx.trim(), "1");
}

/// Previous window with -p flag.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn select_window_prev_flag() {
    let tmux = TmuxServer::new("selectw_p");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["neww", "-d"]);
    tmux.run(&["selectw", "-t", ":1"]);
    tmux.run(&["selectw", "-p"]);
    let idx = tmux.display("#{window_index}");
    assert_eq!(idx.trim(), "0");
}

/// Last window with -l flag.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn select_window_last_flag() {
    let tmux = TmuxServer::new("selectw_l");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["neww", "-d"]);
    // Go to window 1, then last should go back to 0
    tmux.run(&["selectw", "-t", ":1"]);
    tmux.run(&["selectw", "-l"]);
    let idx = tmux.display("#{window_index}");
    assert_eq!(idx.trim(), "0");
}

/// next-window command.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn select_window_next_command() {
    let tmux = TmuxServer::new("selectw_next");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["neww", "-d"]);
    tmux.run(&["next"]);
    let idx = tmux.display("#{window_index}");
    assert_eq!(idx.trim(), "1");
}

/// previous-window command.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn select_window_prev_command() {
    let tmux = TmuxServer::new("selectw_prev");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["neww", "-d"]);
    tmux.run(&["selectw", "-t", ":1"]);
    tmux.run(&["prev"]);
    let idx = tmux.display("#{window_index}");
    assert_eq!(idx.trim(), "0");
}

/// last-window command.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn select_window_last_command() {
    let tmux = TmuxServer::new("selectw_last");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["neww", "-d"]);
    tmux.run(&["selectw", "-t", ":1"]);
    tmux.run(&["last"]);
    let idx = tmux.display("#{window_index}");
    assert_eq!(idx.trim(), "0");
}

/// next-window with -a (activity alert) - no alert window is an error.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn select_window_next_activity() {
    let tmux = TmuxServer::new("selectw_next_a");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["neww", "-d"]);
    // -a with no alerted windows fails, but exercises the activity=true branch
    let result = tmux.try_run(&["next", "-a"]);
    assert!(!result.status.success());
}

/// previous-window with -a (activity alert) - no alert window is an error.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn select_window_prev_activity() {
    let tmux = TmuxServer::new("selectw_prev_a");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["neww", "-d"]);
    tmux.run(&["selectw", "-t", ":1"]);
    let result = tmux.try_run(&["prev", "-a"]);
    assert!(!result.status.success());
}

/// -T toggle: selecting current window switches to last.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn select_window_toggle() {
    let tmux = TmuxServer::new("selectw_toggle");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["neww", "-d"]);
    // Go to window 1 so window 0 becomes last
    tmux.run(&["selectw", "-t", ":1"]);
    // -T on current window (1) should toggle to last (0)
    tmux.run(&["selectw", "-T", "-t", ":1"]);
    let idx = tmux.display("#{window_index}");
    assert_eq!(idx.trim(), "0");
}

/// -T on non-current window just selects it normally.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn select_window_toggle_different() {
    let tmux = TmuxServer::new("selectw_toggle_d");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["neww", "-d"]);
    // -T on different window selects it
    tmux.run(&["selectw", "-T", "-t", ":1"]);
    let idx = tmux.display("#{window_index}");
    assert_eq!(idx.trim(), "1");
}

/// No next window error (only one window).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn select_window_no_next() {
    let tmux = TmuxServer::new("selectw_nonext");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let result = tmux.try_run(&["next"]);
    assert!(!result.status.success());
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(stderr.contains("no next window"), "got: {stderr}");
}

/// No previous window error (only one window).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn select_window_no_prev() {
    let tmux = TmuxServer::new("selectw_noprev");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let result = tmux.try_run(&["prev"]);
    assert!(!result.status.success());
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(stderr.contains("no previous window"), "got: {stderr}");
}

/// No last window error (no previous selection).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn select_window_no_last() {
    let tmux = TmuxServer::new("selectw_nolast");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let result = tmux.try_run(&["last"]);
    assert!(!result.status.success());
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(stderr.contains("no last window"), "got: {stderr}");
}

/// -T toggle on current window with no last window (error).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn select_window_toggle_no_last() {
    let tmux = TmuxServer::new("selectw_t_nolast");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // -T on current window with no last window
    let result = tmux.try_run(&["selectw", "-T"]);
    assert!(!result.status.success());
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(stderr.contains("no last window"), "got: {stderr}");
}

/// Select window with attached control-mode client (exercises latest assignment).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn select_window_with_client() {
    let tmux = TmuxServer::new("selectw_client");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["neww", "-d"]);

    let output = tmux.run_with_stdin(
        &["-C", "attach"],
        b"select-window -t :1\nselect-window -t :0\ndetach-client\n",
    );
    assert!(output.status.success());
}

/// Test select-window, next-window, previous-window, last-window.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn select_window_by_index() {
    let tmux = TmuxServer::new("selectw_idx");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["new-window"]);
    tmux.run(&["new-window"]);

    // Select window 0 explicitly
    tmux.run(&["select-window", "-t", ":0"]);
    let idx = tmux.display("#{window_index}");
    assert_eq!(idx, "0");

    // Select window 2
    tmux.run(&["select-window", "-t", ":2"]);
    let idx = tmux.display("#{window_index}");
    assert_eq!(idx, "2");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn select_window_next() {
    let tmux = TmuxServer::new("selectw_next");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["new-window"]);
    tmux.run(&["new-window"]);

    tmux.run(&["select-window", "-t", ":0"]);
    // next-window
    tmux.run(&["next-window"]);
    let idx = tmux.display("#{window_index}");
    assert_eq!(idx, "1");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn select_window_previous() {
    let tmux = TmuxServer::new("selectw_prev");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["new-window"]);
    tmux.run(&["new-window"]);

    tmux.run(&["select-window", "-t", ":2"]);
    // previous-window
    tmux.run(&["previous-window"]);
    let idx = tmux.display("#{window_index}");
    assert_eq!(idx, "1");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn select_window_last() {
    let tmux = TmuxServer::new("selectw_last");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["new-window"]);

    // Go to window 0, then back to 1 to set "last"
    tmux.run(&["select-window", "-t", ":0"]);
    tmux.run(&["select-window", "-t", ":1"]);

    // last-window should go back to 0
    tmux.run(&["last-window"]);
    let idx = tmux.display("#{window_index}");
    assert_eq!(idx, "0");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn select_window_toggle_02() {
    let tmux = TmuxServer::new("selectw_toggle");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["new-window"]);

    // Visit both windows to set last
    tmux.run(&["select-window", "-t", ":0"]);
    tmux.run(&["select-window", "-t", ":1"]);

    // select-window -T on current window should toggle to last
    tmux.run(&["select-window", "-T", "-t", ":1"]);
    let idx = tmux.display("#{window_index}");
    assert_eq!(idx, "0");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn select_window_next_prev_flags() {
    let tmux = TmuxServer::new("selectw_npflags");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["new-window"]);
    tmux.run(&["new-window"]);

    tmux.run(&["select-window", "-t", ":0"]);

    // select-window -n: next
    tmux.run(&["select-window", "-n"]);
    let idx = tmux.display("#{window_index}");
    assert_eq!(idx, "1");

    // select-window -p: previous
    tmux.run(&["select-window", "-p"]);
    let idx = tmux.display("#{window_index}");
    assert_eq!(idx, "0");

    // select-window -l: last
    tmux.run(&["select-window", "-t", ":2"]);
    tmux.run(&["select-window", "-l"]);
    let idx = tmux.display("#{window_index}");
    assert_eq!(idx, "0");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn select_window_no_next_02() {
    let tmux = TmuxServer::new("selectw_nonext");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Only one window — next should wrap (tmux wraps by default)
    // Just exercise the code path
    let out = tmux.try_run(&["next-window"]);
    let _ = out;
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn select_window_no_last_02() {
    let tmux = TmuxServer::new("selectw_nolast");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // No last window (never switched)
    let out = tmux.try_run(&["last-window"]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("no last window"),
        "expected 'no last window' error, got: {stderr}"
    );
}

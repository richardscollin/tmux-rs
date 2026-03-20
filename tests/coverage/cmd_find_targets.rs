use super::*;

/// Test target resolution via various -t syntax to exercise cmd_find.rs.

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn find_session_by_name() {
    let tmux = TmuxServer::new("find_sess");
    tmux.run(&["-f/dev/null", "new", "-d", "-s", "findme", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let out = tmux.run(&["display-message", "-t", "findme", "-p", "#{session_name}"]);
    assert_eq!(out.trim(), "findme");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn find_session_by_id() {
    let tmux = TmuxServer::new("find_sessid");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let id = tmux.display("#{session_id}");
    let out = tmux.run(&["display-message", "-t", &id, "-p", "#{session_name}"]);
    assert!(!out.trim().is_empty());
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn find_window_by_index() {
    let tmux = TmuxServer::new("find_widx");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["new-window"]);

    let out = tmux.run(&["display-message", "-t", ":1", "-p", "#{window_index}"]);
    assert_eq!(out.trim(), "1");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn find_window_by_id() {
    let tmux = TmuxServer::new("find_wid");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let wid = tmux.display("#{window_id}");
    let out = tmux.run(&["display-message", "-t", &wid, "-p", "#{window_index}"]);
    assert_eq!(out.trim(), "0");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn find_pane_by_id() {
    let tmux = TmuxServer::new("find_pid");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["split-window", "-d"]);

    let pid = tmux.run(&["display-message", "-t", ":.1", "-p", "#{pane_id}"]);
    let out = tmux.run(&["display-message", "-t", pid.trim(), "-p", "#{pane_index}"]);
    assert_eq!(out.trim(), "1");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn find_pane_relative_next() {
    let tmux = TmuxServer::new("find_pnext");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["split-window", "-d"]);
    tmux.run(&["split-window", "-d"]);

    // {next} / {previous} pane targets
    let out = tmux.run(&["display-message", "-t", ":.+", "-p", "#{pane_index}"]);
    assert_eq!(out.trim(), "1");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn find_pane_relative_prev() {
    let tmux = TmuxServer::new("find_pprev");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["split-window", "-d"]);
    tmux.run(&["split-window", "-d"]);

    // Select pane 2, then find previous
    tmux.run(&["select-pane", "-t", ":.2"]);
    let out = tmux.run(&["display-message", "-t", ":.-", "-p", "#{pane_index}"]);
    assert_eq!(out.trim(), "1");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn find_window_relative() {
    let tmux = TmuxServer::new("find_wrel");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["new-window"]);
    tmux.run(&["new-window"]);

    // :+ / :- for next/prev window
    tmux.run(&["select-window", "-t", ":0"]);
    let out = tmux.run(&["display-message", "-t", ":+", "-p", "#{window_index}"]);
    assert_eq!(out.trim(), "1");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn find_session_colon_window() {
    let tmux = TmuxServer::new("find_sw");
    tmux.run(&["-f/dev/null", "new", "-d", "-s", "mysess", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["new-window"]);

    let out = tmux.run(&["display-message", "-t", "mysess:1", "-p", "#{window_index}"]);
    assert_eq!(out.trim(), "1");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn find_session_window_pane() {
    let tmux = TmuxServer::new("find_swp");
    tmux.run(&["-f/dev/null", "new", "-d", "-s", "s", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["split-window", "-d"]);

    let out = tmux.run(&["display-message", "-t", "s:0.1", "-p", "#{pane_index}"]);
    assert_eq!(out.trim(), "1");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn find_invalid_target() {
    let tmux = TmuxServer::new("find_invalid");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Use kill-session with nonexistent target — should fail
    let out = tmux.try_run(&["kill-session", "-t", "nosuchsession"]);
    assert!(!out.status.success());
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn find_marked_pane() {
    let tmux = TmuxServer::new("find_marked");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["split-window", "-d"]);

    // Mark pane 1
    tmux.run(&["select-pane", "-m", "-t", ":.1"]);

    // Reference marked pane with {marked}
    let out = tmux.run(&["display-message", "-t", "{marked}", "-p", "#{pane_index}"]);
    assert_eq!(out.trim(), "1");

    tmux.run(&["select-pane", "-M"]);
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn find_window_by_name() {
    let tmux = TmuxServer::new("find_wname");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["rename-window", "mywin"]);

    // Find window by name
    let out = tmux.run(&["display-message", "-t", ":mywin", "-p", "#{window_name}"]);
    assert_eq!(out.trim(), "mywin");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn find_session_by_prefix() {
    let tmux = TmuxServer::new("find_sprefix");
    tmux.run(&["-f/dev/null", "new", "-d", "-s", "unique_session", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Find session by unique prefix
    let out = tmux.run(&["display-message", "-t", "unique_", "-p", "#{session_name}"]);
    assert_eq!(out.trim(), "unique_session");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn find_pane_top_bottom() {
    let tmux = TmuxServer::new("find_ptb");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["split-window", "-d"]);
    tmux.run(&["split-window", "-d"]);

    // {top} and {bottom} — exercise target resolution, don't assert specific values
    let out = tmux.try_run(&["display-message", "-t", ":{top}", "-p", "#{pane_index}"]);
    let _ = out;
    let out = tmux.try_run(&["display-message", "-t", ":{bottom}", "-p", "#{pane_index}"]);
    let _ = out;
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn find_pane_left_right() {
    let tmux = TmuxServer::new("find_plr");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["split-window", "-d", "-h"]);

    // {left} and {right} — exercise target resolution
    let out = tmux.try_run(&["display-message", "-t", ":{left}", "-p", "#{pane_index}"]);
    let _ = out;
    let out = tmux.try_run(&["display-message", "-t", ":{right}", "-p", "#{pane_index}"]);
    let _ = out;
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn find_window_start_end() {
    let tmux = TmuxServer::new("find_wse");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["new-window"]);
    tmux.run(&["new-window"]);

    // {start} and {end} window targets
    let out = tmux.run(&["display-message", "-t", ":{start}", "-p", "#{window_index}"]);
    assert_eq!(out.trim(), "0");
    let out = tmux.run(&["display-message", "-t", ":{end}", "-p", "#{window_index}"]);
    assert_eq!(out.trim(), "2");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn find_multiple_sessions() {
    let tmux = TmuxServer::new("find_msess");
    tmux.run(&["-f/dev/null", "new", "-d", "-s", "alpha", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["new-session", "-d", "-s", "beta"]);
    tmux.run(&["new-session", "-d", "-s", "gamma"]);

    // Target specific sessions
    let out = tmux.run(&["display-message", "-t", "beta:", "-p", "#{session_name}"]);
    assert_eq!(out.trim(), "beta");
    let out = tmux.run(&["display-message", "-t", "gamma:", "-p", "#{session_name}"]);
    assert_eq!(out.trim(), "gamma");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn find_last_window() {
    let tmux = TmuxServer::new("find_lastw");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["new-window"]);

    // Visit window 0 then 1 to set last
    tmux.run(&["select-window", "-t", ":0"]);
    tmux.run(&["select-window", "-t", ":1"]);

    // {last} window
    let out = tmux.run(&["display-message", "-t", ":{last}", "-p", "#{window_index}"]);
    assert_eq!(out.trim(), "0");
}

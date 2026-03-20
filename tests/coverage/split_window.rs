use super::*;

/// Test split-window: vertical, horizontal, size, before, fullsize,
/// detached, print, zoom, environment, cwd.

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn split_window_vertical() {
    let tmux = TmuxServer::new("splitw_vert");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["split-window"]);
    let count = tmux.display("#{window_panes}");
    assert_eq!(count, "2");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn split_window_horizontal() {
    let tmux = TmuxServer::new("splitw_horiz");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // -h: horizontal split
    tmux.run(&["split-window", "-h"]);
    let count = tmux.display("#{window_panes}");
    assert_eq!(count, "2");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn split_window_size_lines() {
    let tmux = TmuxServer::new("splitw_size");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // -l: explicit size in lines
    tmux.run(&["split-window", "-l", "5"]);
    let count = tmux.display("#{window_panes}");
    assert_eq!(count, "2");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn split_window_size_percent() {
    let tmux = TmuxServer::new("splitw_pct");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // -p: size as percentage
    tmux.run(&["split-window", "-p", "30"]);
    let count = tmux.display("#{window_panes}");
    assert_eq!(count, "2");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn split_window_size_percentage_syntax() {
    let tmux = TmuxServer::new("splitw_lpct");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // -l with percentage syntax
    tmux.run(&["split-window", "-l", "30%"]);
    let count = tmux.display("#{window_panes}");
    assert_eq!(count, "2");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn split_window_before() {
    let tmux = TmuxServer::new("splitw_before");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // -b: split before current pane
    tmux.run(&["split-window", "-b"]);
    let count = tmux.display("#{window_panes}");
    assert_eq!(count, "2");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn split_window_fullsize() {
    let tmux = TmuxServer::new("splitw_full");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["split-window", "-d"]);

    // BUG-005: split-window -f crashes the server
    let out = tmux.try_run(&["split-window", "-f", "-l", "5"]);
    let _ = out;
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn split_window_fullsize_horizontal() {
    let tmux = TmuxServer::new("splitw_fullh");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["split-window", "-d", "-h"]);

    // BUG-005: split-window -f crashes the server
    let out = tmux.try_run(&["split-window", "-f", "-h", "-l", "10"]);
    let _ = out;
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn split_window_detached() {
    let tmux = TmuxServer::new("splitw_det");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let pane_before = tmux.display("#{pane_index}");
    tmux.run(&["split-window", "-d"]);
    let pane_after = tmux.display("#{pane_index}");
    assert_eq!(
        pane_before, pane_after,
        "active pane should not change with -d"
    );
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn split_window_print() {
    let tmux = TmuxServer::new("splitw_print");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // -P: print info
    let out = tmux.run(&["split-window", "-P"]);
    assert!(
        out.contains(":"),
        "expected session:window.pane, got: {out}"
    );
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn split_window_print_format() {
    let tmux = TmuxServer::new("splitw_pfmt");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // -P -F: custom format
    let out = tmux.run(&["split-window", "-P", "-F", "#{pane_index}"]);
    assert_eq!(out.trim(), "1");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn split_window_zoom() {
    let tmux = TmuxServer::new("splitw_zoom");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // -Z: zoom the new pane
    tmux.run(&["split-window", "-Z"]);
    let count = tmux.display("#{window_panes}");
    assert_eq!(count, "2");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn split_window_environment() {
    let tmux = TmuxServer::new("splitw_env");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // -e: environment variable
    tmux.run(&["split-window", "-d", "-e", "FOO=bar"]);
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn split_window_cwd() {
    let tmux = TmuxServer::new("splitw_cwd");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // -c: working directory
    tmux.run(&["split-window", "-d", "-c", "/tmp"]);
}

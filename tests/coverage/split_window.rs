use super::*;

/// Basic vertical split (default).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn split_window_basic() {
    let tmux = TmuxServer::new("splitw_basic");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["splitw"]);
    let count = tmux.display("#{window_panes}");
    assert_eq!(count.trim(), "2");
}

/// Split with -v (explicit vertical).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn split_window_vertical() {
    let tmux = TmuxServer::new("splitw_v");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["splitw", "-v"]);
    let count = tmux.display("#{window_panes}");
    assert_eq!(count.trim(), "2");
}

/// Split with -h (horizontal, left-right).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn split_window_horizontal() {
    let tmux = TmuxServer::new("splitw_h");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["splitw", "-h"]);
    let count = tmux.display("#{window_panes}");
    assert_eq!(count.trim(), "2");
}

/// Split with -d (detached, don't select new pane).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn split_window_detached() {
    let tmux = TmuxServer::new("splitw_d");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["splitw", "-d"]);
    let active = tmux.display("#{pane_index}");
    assert_eq!(active.trim(), "0", "should stay on original pane");
    let count = tmux.display("#{window_panes}");
    assert_eq!(count.trim(), "2");
}

/// Split with -b (before current pane).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn split_window_before() {
    let tmux = TmuxServer::new("splitw_b");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["splitw", "-b"]);
    let count = tmux.display("#{window_panes}");
    assert_eq!(count.trim(), "2");
}

/// Split with -f (full window size).
#[test]
#[ignore = "server crash: splitw -f causes server exit"]
fn split_window_fullsize() {
    let tmux = TmuxServer::new("splitw_f");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["splitw", "-d"]);
    tmux.run(&["splitw", "-f"]);
    let count = tmux.display("#{window_panes}");
    assert_eq!(count.trim(), "3");
}

/// Split with -f -h (full-width horizontal).
#[test]
#[ignore = "server crash: splitw -f causes server exit"]
fn split_window_fullsize_horizontal() {
    let tmux = TmuxServer::new("splitw_fh");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["splitw", "-d"]);
    tmux.run(&["splitw", "-f", "-h"]);
    let count = tmux.display("#{window_panes}");
    assert_eq!(count.trim(), "3");
}

/// Split with -l (absolute size).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn split_window_size() {
    let tmux = TmuxServer::new("splitw_l");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["splitw", "-l", "5"]);
    let count = tmux.display("#{window_panes}");
    assert_eq!(count.trim(), "2");
}

/// Split with -l percentage.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn split_window_size_percentage() {
    let tmux = TmuxServer::new("splitw_l_pct");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["splitw", "-l", "50%"]);
    let count = tmux.display("#{window_panes}");
    assert_eq!(count.trim(), "2");
}

/// Split with -p (percentage, legacy flag).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn split_window_percentage() {
    let tmux = TmuxServer::new("splitw_p");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["splitw", "-p", "30"]);
    let count = tmux.display("#{window_panes}");
    assert_eq!(count.trim(), "2");
}

/// Split with -P (print pane info).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn split_window_print() {
    let tmux = TmuxServer::new("splitw_P");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let out = tmux.run(&["splitw", "-d", "-P"]);
    assert!(
        out.contains(":"),
        "expected session:window.pane format, got: {out}"
    );
}

/// Split with -P -F (custom format).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn split_window_print_format() {
    let tmux = TmuxServer::new("splitw_PF");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let out = tmux.run(&["splitw", "-d", "-P", "-F", "#{pane_id}"]);
    assert!(
        out.trim().starts_with('%'),
        "expected pane id starting with %, got: {out}"
    );
}

/// Split with -Z (zoom).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn split_window_zoom() {
    let tmux = TmuxServer::new("splitw_Z");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["splitw", "-Z"]);
    let count = tmux.display("#{window_panes}");
    assert_eq!(count.trim(), "2");
}

/// Split with -c (start directory).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn split_window_directory() {
    let tmux = TmuxServer::new("splitw_c");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["splitw", "-c", "/tmp"]);
    let cwd = tmux.display("#{pane_current_path}");
    assert_eq!(cwd.trim(), "/tmp");
}

/// Split with -e (environment variable).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn split_window_environment() {
    let tmux = TmuxServer::new("splitw_e");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["splitw", "-d", "-e", "SPLITVAR=hello"]);
    let count = tmux.display("#{window_panes}");
    assert_eq!(count.trim(), "2");
}

/// Split with invalid -l size.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn split_window_invalid_size() {
    let tmux = TmuxServer::new("splitw_badl");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let result = tmux.try_run(&["splitw", "-l", "abc"]);
    assert!(!result.status.success());
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        stderr.contains("size"),
        "expected size error, got: {stderr}"
    );
}

/// Split with invalid -p percentage.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn split_window_invalid_percentage() {
    let tmux = TmuxServer::new("splitw_badp");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let result = tmux.try_run(&["splitw", "-p", "abc"]);
    assert!(!result.status.success());
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        stderr.contains("size"),
        "expected size error, got: {stderr}"
    );
}

/// Split when there's no space (tiny pane).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn split_window_no_space() {
    let tmux = TmuxServer::new("splitw_nospace");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y4"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Fill up the window with splits until no space
    let _ = tmux.try_run(&["splitw"]);
    let result = tmux.try_run(&["splitw"]);
    assert!(!result.status.success());
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        stderr.contains("no space"),
        "expected 'no space' error, got: {stderr}"
    );
}

/// Split with shell command.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn split_window_with_command() {
    let tmux = TmuxServer::new("splitw_cmd");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["splitw", "-d", "sleep", "100"]);
    let count = tmux.display("#{window_panes}");
    assert_eq!(count.trim(), "2");
}

/// Split with -b -f (before + fullsize).
#[test]
#[ignore = "server crash: splitw -f causes server exit"]
fn split_window_before_fullsize() {
    let tmux = TmuxServer::new("splitw_bf");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["splitw", "-d"]);
    tmux.run(&["splitw", "-b", "-f"]);
    let count = tmux.display("#{window_panes}");
    assert_eq!(count.trim(), "3");
}

/// splitw alias.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn split_window_alias() {
    let tmux = TmuxServer::new("splitw_alias");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["split-window", "-d"]);
    let count = tmux.display("#{window_panes}");
    assert_eq!(count.trim(), "2");
}

/// Split with -l and -f together (fullsize uses window dimension).
#[test]
#[ignore = "server crash: splitw -f causes server exit"]
fn split_window_size_fullsize() {
    let tmux = TmuxServer::new("splitw_lf");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["splitw", "-d"]);
    tmux.run(&["splitw", "-f", "-l", "5"]);
    let count = tmux.display("#{window_panes}");
    assert_eq!(count.trim(), "3");
}

/// Split with -p and -f (percentage + fullsize).
#[test]
#[ignore = "server crash: splitw -f causes server exit"]
fn split_window_percentage_fullsize() {
    let tmux = TmuxServer::new("splitw_pf");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["splitw", "-d"]);
    tmux.run(&["splitw", "-f", "-p", "30"]);
    let count = tmux.display("#{window_panes}");
    assert_eq!(count.trim(), "3");
}

/// Split with -h and -l (horizontal + absolute size).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn split_window_horizontal_size() {
    let tmux = TmuxServer::new("splitw_hl");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["splitw", "-h", "-l", "20"]);
    let count = tmux.display("#{window_panes}");
    assert_eq!(count.trim(), "2");
}

/// Split with -h and -p (horizontal + percentage).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn split_window_horizontal_percentage() {
    let tmux = TmuxServer::new("splitw_hp");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["splitw", "-h", "-p", "25"]);
    let count = tmux.display("#{window_panes}");
    assert_eq!(count.trim(), "2");
}

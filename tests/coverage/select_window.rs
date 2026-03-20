use super::*;

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
fn select_window_toggle() {
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
fn select_window_no_next() {
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
fn select_window_no_last() {
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

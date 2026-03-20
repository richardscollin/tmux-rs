use super::*;

/// Basic swap-window.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn swap_window_basic() {
    let tmux = TmuxServer::new("swapw_basic");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["neww", "-d"]);

    let id0 = tmux.run(&["display", "-t", ":0", "-p", "#{window_id}"]);
    let id1 = tmux.run(&["display", "-t", ":1", "-p", "#{window_id}"]);

    tmux.run(&["swapw", "-s", ":0", "-t", ":1"]);

    let new_id0 = tmux.run(&["display", "-t", ":0", "-p", "#{window_id}"]);
    let new_id1 = tmux.run(&["display", "-t", ":1", "-p", "#{window_id}"]);

    assert_eq!(id0.trim(), new_id1.trim());
    assert_eq!(id1.trim(), new_id0.trim());
}

/// Swap-window with -d (don't select).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn swap_window_no_select() {
    let tmux = TmuxServer::new("swapw_nosel");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["neww", "-d"]);

    let id0 = tmux.run(&["display", "-t", ":0", "-p", "#{window_id}"]);
    let id1 = tmux.run(&["display", "-t", ":1", "-p", "#{window_id}"]);

    tmux.run(&["swapw", "-d", "-s", ":0", "-t", ":1"]);

    let new_id0 = tmux.run(&["display", "-t", ":0", "-p", "#{window_id}"]);
    let new_id1 = tmux.run(&["display", "-t", ":1", "-p", "#{window_id}"]);

    assert_eq!(id0.trim(), new_id1.trim());
    assert_eq!(id1.trim(), new_id0.trim());
}

/// Swap window with itself (same window): should be a no-op.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn swap_window_same() {
    let tmux = TmuxServer::new("swapw_same");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let id = tmux.run(&["display", "-t", ":0", "-p", "#{window_id}"]);
    tmux.run(&["swapw", "-s", ":0", "-t", ":0"]);
    let id_after = tmux.run(&["display", "-t", ":0", "-p", "#{window_id}"]);
    assert_eq!(id.trim(), id_after.trim());
}

/// Swap windows across sessions.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn swap_window_cross_session() {
    let tmux = TmuxServer::new("swapw_cross");
    tmux.run(&["-f/dev/null", "new", "-d", "-s", "s1", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["new-session", "-d", "-s", "s2"]);

    let id_s1 = tmux.run(&["display", "-t", "s1:0", "-p", "#{window_id}"]);
    let id_s2 = tmux.run(&["display", "-t", "s2:0", "-p", "#{window_id}"]);

    tmux.run(&["swapw", "-s", "s1:0", "-t", "s2:0"]);

    let new_s1 = tmux.run(&["display", "-t", "s1:0", "-p", "#{window_id}"]);
    let new_s2 = tmux.run(&["display", "-t", "s2:0", "-p", "#{window_id}"]);

    assert_eq!(id_s1.trim(), new_s2.trim());
    assert_eq!(id_s2.trim(), new_s1.trim());
}

/// Swap-window -d across sessions.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn swap_window_cross_session_d() {
    let tmux = TmuxServer::new("swapw_crossd");
    tmux.run(&["-f/dev/null", "new", "-d", "-s", "a", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["new-session", "-d", "-s", "b"]);

    tmux.run(&["swapw", "-d", "-s", "a:0", "-t", "b:0"]);
}

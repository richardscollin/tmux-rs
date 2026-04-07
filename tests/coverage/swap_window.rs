use super::*;

/// Swap window with itself (noop -- same window).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn swap_window_same_window() {
    let tmux = TmuxServer::new("swapw_same");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let id0 = tmux.run(&["display", "-t", ":0", "-p", "#{window_id}"]);
    // Swapping a window with itself is a noop
    tmux.run(&["swapw", "-s", ":0", "-t", ":0"]);
    let after = tmux.run(&["display", "-t", ":0", "-p", "#{window_id}"]);
    assert_eq!(id0.trim(), after.trim());
}

/// Swap windows across sessions with -d.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn swap_window_cross_session() {
    let tmux = TmuxServer::new("swapw_cross");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24", "-s", "s1"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["new", "-d", "-s", "s2"]);

    let id_s1 = tmux.run(&["display", "-t", "s1:0", "-p", "#{window_id}"]);
    let id_s2 = tmux.run(&["display", "-t", "s2:0", "-p", "#{window_id}"]);

    tmux.run(&["swapw", "-d", "-s", "s1:0", "-t", "s2:0"]);

    let new_s1 = tmux.run(&["display", "-t", "s1:0", "-p", "#{window_id}"]);
    let new_s2 = tmux.run(&["display", "-t", "s2:0", "-p", "#{window_id}"]);

    assert_eq!(id_s1.trim(), new_s2.trim());
    assert_eq!(id_s2.trim(), new_s1.trim());
}

/// Swap windows in grouped sessions (error).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn swap_window_grouped_sessions() {
    let tmux = TmuxServer::new("swapw_group");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24", "-s", "main"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["neww", "-d", "-t", "main"]);

    // Create a grouped session
    tmux.run(&["new", "-d", "-s", "linked", "-t", "main"]);

    let result = tmux.try_run(&["swapw", "-s", "main:0", "-t", "linked:1"]);
    assert!(
        !result.status.success(),
        "expected error for grouped session swap"
    );
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        stderr.contains("grouped"),
        "expected grouped session error, got: {stderr}"
    );
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
fn swap_window_cross_session_02() {
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

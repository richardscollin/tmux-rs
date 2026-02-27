use super::*;

/// Basic swap-window.
#[test]
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
fn swap_window_no_select() {
    let tmux = TmuxServer::new("swapw_nosel");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["neww", "-d"]);

    let id0 = tmux.run(&["display", "-t", ":0", "-p", "#{window_id}"]);
    let id1 = tmux.run(&["display", "-t", ":1", "-p", "#{window_id}"]);

    tmux.run(&["swapw", "-d", "-s", ":0", "-t", ":1"]);

    // Windows should have swapped even with -d
    let new_id0 = tmux.run(&["display", "-t", ":0", "-p", "#{window_id}"]);
    let new_id1 = tmux.run(&["display", "-t", ":1", "-p", "#{window_id}"]);

    assert_eq!(id0.trim(), new_id1.trim());
    assert_eq!(id1.trim(), new_id0.trim());
}

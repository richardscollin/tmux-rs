use super::*;

/// Basic join-pane: move pane from one window into another.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn join_pane_basic() {
    let tmux = TmuxServer::new("joinp_basic");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Create a second window with its own pane
    tmux.run(&["neww", "-d"]);

    let id1 = tmux.run(&["display", "-t", ":1", "-p", "#{pane_id}"]);

    // Join pane from window 1 into window 0
    tmux.run(&["joinp", "-s", ":1.0", "-t", ":0"]);

    // Window 0 should now have 2 panes
    let panes = tmux.run(&["lsp", "-t", ":0", "-F", "#{pane_id}"]);
    assert_eq!(panes.lines().count(), 2);
    assert!(panes.contains(id1.trim()));

    // Window 1 should be gone (had only 1 pane)
    let wins = tmux.run(&["lsw", "-F", "#{window_index}"]);
    assert_eq!(wins.trim(), "0");
}

/// Join pane with -h (horizontal split).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn join_pane_horizontal() {
    let tmux = TmuxServer::new("joinp_horiz");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["neww", "-d"]);

    tmux.run(&["joinp", "-h", "-s", ":1.0", "-t", ":0"]);

    let panes = tmux.run(&["lsp", "-t", ":0", "-F", "#{pane_id}"]);
    assert_eq!(panes.lines().count(), 2);
}

/// Join pane with -d (don't change active pane).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn join_pane_detached() {
    let tmux = TmuxServer::new("joinp_detach");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["neww", "-d"]);

    let active_before = tmux.display("#{pane_id}");
    tmux.run(&["joinp", "-d", "-s", ":1.0", "-t", ":0"]);
    let active_after = tmux.display("#{pane_id}");

    assert_eq!(
        active_before, active_after,
        "active pane should not change with -d"
    );
}

/// Join pane with -b (before target).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn join_pane_before() {
    let tmux = TmuxServer::new("joinp_before");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["neww", "-d"]);

    let src_id = tmux.run(&["display", "-t", ":1", "-p", "#{pane_id}"]);
    tmux.run(&["joinp", "-b", "-s", ":1.0", "-t", ":0"]);

    // The joined pane should be first (before the original)
    let panes = tmux.run(&["lsp", "-t", ":0", "-F", "#{pane_id}"]);
    let first_pane = panes.lines().next().unwrap();
    assert_eq!(first_pane.trim(), src_id.trim());
}

/// Join pane error: source == target.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn join_pane_same_pane_error() {
    let tmux = TmuxServer::new("joinp_same");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let result = tmux.try_run(&["joinp", "-s", ":0.0", "-t", ":0"]);
    assert!(!result.status.success());
}

/// Join pane with -l (size).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn join_pane_with_size() {
    let tmux = TmuxServer::new("joinp_size");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["neww", "-d"]);

    tmux.run(&["joinp", "-l", "10", "-s", ":1.0", "-t", ":0"]);

    let panes = tmux.run(&["lsp", "-t", ":0", "-F", "#{pane_id}"]);
    assert_eq!(panes.lines().count(), 2);
}

/// Join pane with -p (percentage).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn join_pane_percentage() {
    let tmux = TmuxServer::new("joinp_pct");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["neww", "-d"]);

    tmux.run(&["joinp", "-l", "30%", "-s", ":1.0", "-t", ":0"]);

    let panes = tmux.run(&["lsp", "-t", ":0", "-F", "#{pane_id}"]);
    assert_eq!(panes.lines().count(), 2);
}

/// move-pane alias uses same exec function.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn move_pane_alias() {
    let tmux = TmuxServer::new("movep_alias");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["neww", "-d"]);

    tmux.run(&["movep", "-s", ":1.0", "-t", ":0"]);

    let panes = tmux.run(&["lsp", "-t", ":0", "-F", "#{pane_id}"]);
    assert_eq!(panes.lines().count(), 2);
}

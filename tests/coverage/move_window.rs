use super::*;

/// Basic move-window to a new index.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn move_window_basic() {
    let tmux = TmuxServer::new("movew_basic");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["neww", "-d"]);

    tmux.run(&["movew", "-s", ":1", "-t", ":5"]);

    let wins = tmux.run(&["lsw", "-F", "#{window_index}"]);
    assert_eq!(wins.trim(), "0\n5");
}

/// Move-window with -r to renumber.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn move_window_renumber() {
    let tmux = TmuxServer::new("movew_renum");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["neww", "-d"]);
    tmux.run(&["neww", "-d"]);

    tmux.run(&["movew", "-s", ":1", "-t", ":5"]);
    tmux.run(&["movew", "-r"]);

    let wins = tmux.run(&["lsw", "-F", "#{window_index}"]);
    assert_eq!(wins.trim(), "0\n1\n2");
}

/// Move-window with -k (kill existing at target).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn move_window_kill() {
    let tmux = TmuxServer::new("movew_kill");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["neww", "-d"]);

    let id_w0 = tmux.run(&["display", "-t", ":0", "-p", "#{window_id}"]);
    tmux.run(&["movew", "-k", "-s", ":0", "-t", ":1"]);

    let wins = tmux.run(&["lsw", "-F", "#{window_index} #{window_id}"]);
    let lines: Vec<&str> = wins.trim().lines().collect();
    assert_eq!(lines.len(), 1);
    assert!(lines[0].contains(id_w0.trim()));
}

/// Move-window target in use error.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn move_window_target_in_use() {
    let tmux = TmuxServer::new("movew_in_use");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["neww", "-d"]);

    let result = tmux.try_run(&["movew", "-s", ":0", "-t", ":1"]);
    assert!(!result.status.success());
}

/// Move-window with -a (after).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn move_window_after() {
    let tmux = TmuxServer::new("movew_after");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["neww", "-d"]);
    tmux.run(&["neww", "-d"]);

    tmux.run(&["movew", "-a", "-s", ":0", "-t", ":2"]);
    let wins = tmux.run(&["lsw", "-F", "#{window_index}"]);
    assert_eq!(wins.lines().count(), 3);
}

/// Move-window with -b (before).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn move_window_before() {
    let tmux = TmuxServer::new("movew_before");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["neww", "-d"]);
    tmux.run(&["neww", "-d"]);

    tmux.run(&["movew", "-b", "-s", ":2", "-t", ":0"]);
    let wins = tmux.run(&["lsw", "-F", "#{window_index}"]);
    assert_eq!(wins.lines().count(), 3);
}

/// Move-window cross-session.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn move_window_cross_session() {
    let tmux = TmuxServer::new("movew_cross");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24", "-s", "src"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["new", "-d", "-x80", "-y24", "-s", "dst"]);
    tmux.run(&["neww", "-d", "-t", "src"]);

    tmux.run(&["movew", "-s", "src:1", "-t", "dst:5"]);

    let dst_wins = tmux.run(&["lsw", "-t", "dst", "-F", "#{window_index}"]);
    assert!(dst_wins.contains("5"));
}

/// Link-window basic.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn link_window_basic() {
    let tmux = TmuxServer::new("linkw_basic");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24", "-s", "src"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["new", "-d", "-x80", "-y24", "-s", "dst"]);

    let id = tmux.run(&["display", "-t", "src:0", "-p", "#{window_id}"]);
    tmux.run(&["linkw", "-s", "src:0", "-t", "dst:5"]);

    // Both sessions should have the window
    let src_wins = tmux.run(&["lsw", "-t", "src", "-F", "#{window_id}"]);
    let dst_wins = tmux.run(&["lsw", "-t", "dst", "-F", "#{window_id}"]);
    assert!(src_wins.contains(id.trim()));
    assert!(dst_wins.contains(id.trim()));
}

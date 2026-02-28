use super::*;

/// Rotate window down (-D).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn rotate_window_down() {
    let tmux = TmuxServer::new("rotatew_down");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["splitw", "-d"]);
    tmux.run(&["splitw", "-d"]);

    let before = tmux.run(&["lsp", "-F", "#{pane_id}"]);
    tmux.run(&["rotatew", "-D"]);
    let after = tmux.run(&["lsp", "-F", "#{pane_id}"]);

    assert_ne!(before, after, "pane order should change after rotate -D");
}

/// Rotate window up (-U, default).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn rotate_window_up() {
    let tmux = TmuxServer::new("rotatew_up");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["splitw", "-d"]);
    tmux.run(&["splitw", "-d"]);

    let before = tmux.run(&["lsp", "-F", "#{pane_id}"]);
    tmux.run(&["rotatew", "-U"]);
    let after = tmux.run(&["lsp", "-F", "#{pane_id}"]);

    assert_ne!(before, after, "pane order should change after rotate -U");
}

/// Rotate window default (no flag = up).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn rotate_window_default() {
    let tmux = TmuxServer::new("rotatew_default");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["splitw", "-d"]);

    let before = tmux.run(&["lsp", "-F", "#{pane_id}"]);
    tmux.run(&["rotatew"]);
    let after = tmux.run(&["lsp", "-F", "#{pane_id}"]);

    assert_ne!(before, after);
}

/// Rotate preserves zoom with -Z.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn rotate_window_zoom() {
    let tmux = TmuxServer::new("rotatew_zoom");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["splitw", "-d"]);

    // Zoom first pane
    tmux.run(&["resizep", "-Z"]);
    let zoomed = tmux.display("#{window_zoomed_flag}");
    assert_eq!(zoomed, "1");

    // Rotate with -Z should preserve zoom
    tmux.run(&["rotatew", "-Z"]);
    let still_zoomed = tmux.display("#{window_zoomed_flag}");
    assert_eq!(still_zoomed, "1");
}

/// Rotate down with 3 panes verifies pane order changes correctly.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn rotate_window_down_three_panes() {
    let tmux = TmuxServer::new("rotatew_d3");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["splitw", "-d"]);
    tmux.run(&["splitw", "-d"]);

    // Record pane ids in order before rotate
    let before = tmux.run(&["lsp", "-F", "#{pane_id}"]);
    let before_ids: Vec<&str> = before.trim().lines().collect();
    assert_eq!(before_ids.len(), 3, "should have 3 panes");

    // Rotate down: last pane moves to first position
    tmux.run(&["rotatew", "-D"]);
    let after = tmux.run(&["lsp", "-F", "#{pane_id}"]);
    let after_ids: Vec<&str> = after.trim().lines().collect();

    // After -D, the last pane should now be first
    assert_eq!(
        after_ids[0], before_ids[2],
        "last pane should move to first"
    );
    assert_eq!(after_ids[1], before_ids[0], "first pane should shift down");
    assert_eq!(after_ids[2], before_ids[1], "second pane should shift down");
}

/// Rotate up with 3 panes verifies pane order changes correctly.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn rotate_window_up_three_panes() {
    let tmux = TmuxServer::new("rotatew_u3");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["splitw", "-d"]);
    tmux.run(&["splitw", "-d"]);

    let before = tmux.run(&["lsp", "-F", "#{pane_id}"]);
    let before_ids: Vec<&str> = before.trim().lines().collect();
    assert_eq!(before_ids.len(), 3, "should have 3 panes");

    // Rotate up: first pane moves to last position
    tmux.run(&["rotatew", "-U"]);
    let after = tmux.run(&["lsp", "-F", "#{pane_id}"]);
    let after_ids: Vec<&str> = after.trim().lines().collect();

    assert_eq!(
        after_ids[0], before_ids[1],
        "second pane should move to first"
    );
    assert_eq!(after_ids[1], before_ids[2], "third pane should shift up");
    assert_eq!(
        after_ids[2], before_ids[0],
        "first pane should move to last"
    );
}

/// Rotate down twice restores near-original order with 3 panes.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn rotate_window_down_twice() {
    let tmux = TmuxServer::new("rotatew_d2x");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["splitw", "-d"]);
    tmux.run(&["splitw", "-d"]);

    let before = tmux.run(&["lsp", "-F", "#{pane_id}"]);
    tmux.run(&["rotatew", "-D"]);
    tmux.run(&["rotatew", "-D"]);
    tmux.run(&["rotatew", "-D"]);
    let after = tmux.run(&["lsp", "-F", "#{pane_id}"]);

    // Three -D rotations on 3 panes should return to original order
    assert_eq!(
        before.trim(),
        after.trim(),
        "3 rotations of 3 panes should restore original order"
    );
}

/// Rotate down with -Z preserves zoom.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn rotate_window_down_zoom() {
    let tmux = TmuxServer::new("rotatew_dz");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["splitw", "-d"]);

    tmux.run(&["resizep", "-Z"]);
    let zoomed = tmux.display("#{window_zoomed_flag}");
    assert_eq!(zoomed, "1");

    tmux.run(&["rotatew", "-DZ"]);
    let still_zoomed = tmux.display("#{window_zoomed_flag}");
    assert_eq!(still_zoomed, "1");
}

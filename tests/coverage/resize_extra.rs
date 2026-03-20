use super::*;

/// Test resize-related paths to exercise resize.rs.

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn resize_zoom_unzoom_cycle() {
    let tmux = TmuxServer::new("resize_zoom");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["split-window", "-d"]);

    // Zoom, then resize (should unzoom), then zoom again
    tmux.run(&["resize-pane", "-Z"]);
    let zoomed = tmux.display("#{window_zoomed_flag}");
    assert_eq!(zoomed, "1");

    // Resize while zoomed — should trigger unzoom path in resize
    tmux.run(&["resize-window", "-x", "100", "-y", "30"]);

    // Zoom again
    tmux.run(&["resize-pane", "-Z"]);
    tmux.run(&["resize-pane", "-Z"]); // unzoom
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn resize_recalculate_after_kill() {
    let tmux = TmuxServer::new("resize_kill");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["split-window", "-d"]);
    tmux.run(&["split-window", "-d"]);

    // Kill a pane — triggers recalculate_sizes
    tmux.run(&["kill-pane", "-t", ":.2"]);
    let count = tmux.display("#{window_panes}");
    assert_eq!(count, "2");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn resize_multiple_sessions_same_window() {
    let tmux = TmuxServer::new("resize_msess");
    tmux.run(&["-f/dev/null", "new", "-d", "-s", "s1", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Link the window into another session
    tmux.run(&["new-session", "-d", "-s", "s2"]);
    tmux.run(&["link-window", "-s", "s1:0", "-t", "s2:1"]);

    // Resize in one session — should trigger recalculate for shared window
    tmux.run(&["resize-window", "-t", "s1", "-x", "100", "-y", "30"]);
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn resize_manual_window_size() {
    let tmux = TmuxServer::new("resize_manual");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Explicitly set manual window size
    tmux.run(&["resize-window", "-x", "60", "-y", "20"]);
    let w = tmux.display("#{window_width}");
    let h = tmux.display("#{window_height}");
    assert_eq!(w, "60");
    assert_eq!(h, "20");

    // Change again
    tmux.run(&["resize-window", "-x", "120", "-y", "40"]);
    let w = tmux.display("#{window_width}");
    assert_eq!(w, "120");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn resize_after_layout_change() {
    let tmux = TmuxServer::new("resize_layout");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["split-window", "-d"]);
    tmux.run(&["split-window", "-d"]);

    // Apply layout then resize
    tmux.run(&["select-layout", "even-vertical"]);
    tmux.run(&["resize-window", "-x", "60", "-y", "30"]);

    // Apply another layout — triggers recalculate
    tmux.run(&["select-layout", "main-vertical"]);
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn resize_pane_absolute_after_split() {
    let tmux = TmuxServer::new("resize_abs");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["split-window", "-d"]);

    // Absolute resize with percentage
    tmux.run(&["resize-pane", "-y", "80%"]);
    tmux.run(&["resize-pane", "-x", "50%", "-t", ":.0"]);
}

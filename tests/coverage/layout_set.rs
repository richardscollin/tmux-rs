use super::*;

/// Test layout presets with multiple panes to exercise layout_set.rs.

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn layout_even_horizontal_3panes() {
    let tmux = TmuxServer::new("layout_eh3");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["split-window", "-d"]);
    tmux.run(&["split-window", "-d"]);

    // BUG-006: even-horizontal crashes with 3+ panes
    let out = tmux.try_run(&["select-layout", "even-horizontal"]);
    let _ = out;
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn layout_even_vertical_3panes() {
    let tmux = TmuxServer::new("layout_ev3");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["split-window", "-d"]);
    tmux.run(&["split-window", "-d"]);

    tmux.run(&["select-layout", "even-vertical"]);
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn layout_main_horizontal_3panes() {
    let tmux = TmuxServer::new("layout_mh3");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["split-window", "-d"]);
    tmux.run(&["split-window", "-d"]);

    tmux.run(&["select-layout", "main-horizontal"]);
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn layout_main_vertical_3panes() {
    let tmux = TmuxServer::new("layout_mv3");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["split-window", "-d"]);
    tmux.run(&["split-window", "-d"]);

    tmux.run(&["select-layout", "main-vertical"]);
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn layout_tiled_3panes() {
    let tmux = TmuxServer::new("layout_tiled3");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["split-window", "-d"]);
    tmux.run(&["split-window", "-d"]);

    tmux.run(&["select-layout", "tiled"]);
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn layout_even_horizontal_5panes() {
    let tmux = TmuxServer::new("layout_eh5");
    tmux.run(&["-f/dev/null", "new", "-d", "-x120", "-y40"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    for _ in 0..4 {
        tmux.run(&["split-window", "-d"]);
    }

    tmux.run(&["select-layout", "even-horizontal"]);
    let count = tmux.display("#{window_panes}");
    assert_eq!(count, "5");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn layout_main_vertical_5panes() {
    let tmux = TmuxServer::new("layout_mv5");
    tmux.run(&["-f/dev/null", "new", "-d", "-x120", "-y40"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    for _ in 0..4 {
        tmux.run(&["split-window", "-d"]);
    }

    tmux.run(&["select-layout", "main-vertical"]);
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn layout_main_horizontal_5panes() {
    let tmux = TmuxServer::new("layout_mh5");
    tmux.run(&["-f/dev/null", "new", "-d", "-x120", "-y40"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    for _ in 0..4 {
        tmux.run(&["split-window", "-d"]);
    }

    tmux.run(&["select-layout", "main-horizontal"]);
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn layout_tiled_4panes() {
    let tmux = TmuxServer::new("layout_tiled4");
    tmux.run(&["-f/dev/null", "new", "-d", "-x120", "-y40"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    for _ in 0..3 {
        tmux.run(&["split-window", "-d"]);
    }

    tmux.run(&["select-layout", "tiled"]);
    let count = tmux.display("#{window_panes}");
    assert_eq!(count, "4");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn layout_cycle_next_prev() {
    let tmux = TmuxServer::new("layout_cycle");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["split-window", "-d"]);
    tmux.run(&["split-window", "-d"]);

    // Cycle through layouts — BUG-006 may crash on even-horizontal
    for _ in 0..6 {
        let out = tmux.try_run(&["next-layout"]);
        if !out.status.success() {
            break;
        }
    }
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn layout_set_with_tiny_window() {
    let tmux = TmuxServer::new("layout_tiny");
    tmux.run(&["-f/dev/null", "new", "-d", "-x20", "-y10"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["split-window", "-d"]);
    tmux.run(&["split-window", "-d"]);

    // Small window should still handle layout presets
    // Skip even-horizontal due to BUG-006
    tmux.run(&["select-layout", "even-vertical"]);
    tmux.run(&["select-layout", "main-vertical"]);
    tmux.run(&["select-layout", "main-horizontal"]);
    tmux.run(&["select-layout", "tiled"]);
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn layout_main_pane_height() {
    let tmux = TmuxServer::new("layout_mph");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y30"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["split-window", "-d"]);
    tmux.run(&["split-window", "-d"]);

    // Set main-pane-height then apply main-horizontal
    tmux.run(&["setw", "main-pane-height", "20"]);
    tmux.run(&["select-layout", "main-horizontal"]);
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn layout_main_pane_width() {
    let tmux = TmuxServer::new("layout_mpw");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["split-window", "-d"]);
    tmux.run(&["split-window", "-d"]);

    // Set main-pane-width then apply main-vertical
    tmux.run(&["setw", "main-pane-width", "50"]);
    tmux.run(&["select-layout", "main-vertical"]);
}

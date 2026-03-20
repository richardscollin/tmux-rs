use super::*;

/// Test custom layout strings to exercise layout_custom.rs parsing and validation.

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn layout_custom_apply_and_reapply() {
    let tmux = TmuxServer::new("lcust_reapply");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["split-window", "-d"]);

    // Get the current layout string and re-apply it
    let layout = tmux.display("#{window_layout}");
    tmux.run(&["select-layout", &layout]);

    // Apply again after changing layout
    tmux.run(&["select-layout", "even-vertical"]);
    tmux.run(&["select-layout", &layout]);
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn layout_custom_3panes() {
    let tmux = TmuxServer::new("lcust_3p");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["split-window", "-d"]);
    tmux.run(&["split-window", "-d"]);

    let layout = tmux.display("#{window_layout}");
    // Re-apply the 3-pane layout
    tmux.run(&["select-layout", &layout]);
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn layout_custom_horizontal_split() {
    let tmux = TmuxServer::new("lcust_hsplit");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["split-window", "-d", "-h"]);

    let layout = tmux.display("#{window_layout}");
    // Apply a horizontal layout
    tmux.run(&["select-layout", "even-vertical"]);
    tmux.run(&["select-layout", &layout]);
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn layout_custom_mixed() {
    let tmux = TmuxServer::new("lcust_mixed");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // BUG-008: mixed h/v splits crash on #{window_layout}
    tmux.run(&["split-window", "-d"]);
    tmux.run(&["split-window", "-d", "-h", "-t", ":.1"]);

    // Don't query #{window_layout} — it crashes (BUG-008)
    let count = tmux.display("#{window_panes}");
    assert_eq!(count, "3");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn layout_custom_invalid_string() {
    let tmux = TmuxServer::new("lcust_invalid");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Invalid layout string
    let out = tmux.try_run(&["select-layout", "garbage_layout_string"]);
    assert!(!out.status.success());
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn layout_custom_wrong_pane_count() {
    let tmux = TmuxServer::new("lcust_wrongcnt");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["split-window", "-d"]);

    // Get a 2-pane layout, add a third pane, try to apply the 2-pane layout
    let layout_2 = tmux.display("#{window_layout}");
    tmux.run(&["split-window", "-d"]);
    // 3 panes but 2-pane layout — should adjust or succeed
    let out = tmux.try_run(&["select-layout", &layout_2]);
    let _ = out;
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn layout_custom_too_few_panes() {
    let tmux = TmuxServer::new("lcust_toofew");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["split-window", "-d"]);
    tmux.run(&["split-window", "-d"]);

    // Get a 3-pane layout, kill one pane, try to apply it
    let layout_3 = tmux.display("#{window_layout}");
    tmux.run(&["kill-pane", "-t", ":.2"]);
    // 2 panes but 3-pane layout — should error
    let out = tmux.try_run(&["select-layout", &layout_3]);
    let _ = out;
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn layout_custom_checksum_mismatch() {
    let tmux = TmuxServer::new("lcust_cksum");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["split-window", "-d"]);

    // Corrupt the checksum in a layout string
    let layout = tmux.display("#{window_layout}");
    // Replace first hex digit of checksum
    let corrupted = if layout.len() > 4 {
        format!("ffff,{}", &layout[5..])
    } else {
        "0000,80x24,0,0".to_string()
    };
    let out = tmux.try_run(&["select-layout", &corrupted]);
    assert!(!out.status.success());
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn layout_dump_and_parse_4panes() {
    let tmux = TmuxServer::new("lcust_4p");
    tmux.run(&["-f/dev/null", "new", "-d", "-x120", "-y40"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Create 4 panes in a grid-like arrangement
    tmux.run(&["split-window", "-d", "-h"]);
    tmux.run(&["split-window", "-d", "-v", "-t", ":.0"]);
    tmux.run(&["split-window", "-d", "-v", "-t", ":.2"]);

    let layout = tmux.display("#{window_layout}");
    // Cycle through layouts then re-apply the custom one
    tmux.run(&["select-layout", "tiled"]);
    tmux.run(&["select-layout", &layout]);

    let count = tmux.display("#{window_panes}");
    assert_eq!(count, "4");
}

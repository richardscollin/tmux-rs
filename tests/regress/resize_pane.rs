use super::*;

/// Resize pane down with -D (extends pane 0 downward).
#[test]
fn resize_pane_down() {
    let tmux = TmuxServer::new("resizep_down");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["splitw", "-d"]);

    let h_before: u32 = tmux
        .run(&["display", "-t", ":0.0", "-p", "#{pane_height}"])
        .trim()
        .parse()
        .unwrap();
    tmux.run(&["resizep", "-D", "-t", ":0.0", "5"]);
    let h_after: u32 = tmux
        .run(&["display", "-t", ":0.0", "-p", "#{pane_height}"])
        .trim()
        .parse()
        .unwrap();

    assert!(
        h_after > h_before,
        "pane 0 should grow after -D, before={h_before} after={h_after}"
    );
}

/// Resize pane up with -U (shrinks pane 0 from bottom).
#[test]
fn resize_pane_up() {
    let tmux = TmuxServer::new("resizep_up");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["splitw", "-d"]);

    let h_before: u32 = tmux
        .run(&["display", "-t", ":0.0", "-p", "#{pane_height}"])
        .trim()
        .parse()
        .unwrap();
    tmux.run(&["resizep", "-U", "-t", ":0.0", "3"]);
    let h_after: u32 = tmux
        .run(&["display", "-t", ":0.0", "-p", "#{pane_height}"])
        .trim()
        .parse()
        .unwrap();

    assert!(
        h_after < h_before,
        "pane 0 should shrink after -U, before={h_before} after={h_after}"
    );
}

/// Resize pane left with -L.
#[test]
fn resize_pane_left() {
    let tmux = TmuxServer::new("resizep_left");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["splitw", "-h", "-d"]);

    let w_before: u32 = tmux
        .run(&["display", "-t", ":0.0", "-p", "#{pane_width}"])
        .trim()
        .parse()
        .unwrap();
    tmux.run(&["resizep", "-L", "-t", ":0.0", "5"]);
    let w_after: u32 = tmux
        .run(&["display", "-t", ":0.0", "-p", "#{pane_width}"])
        .trim()
        .parse()
        .unwrap();

    assert!(w_after < w_before, "pane should be narrower after -L");
}

/// Resize pane right with -R.
#[test]
fn resize_pane_right() {
    let tmux = TmuxServer::new("resizep_right");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["splitw", "-h", "-d"]);

    let w_before: u32 = tmux
        .run(&["display", "-t", ":0.0", "-p", "#{pane_width}"])
        .trim()
        .parse()
        .unwrap();
    tmux.run(&["resizep", "-R", "-t", ":0.0", "5"]);
    let w_after: u32 = tmux
        .run(&["display", "-t", ":0.0", "-p", "#{pane_width}"])
        .trim()
        .parse()
        .unwrap();

    assert!(w_after > w_before, "pane should be wider after -R");
}

/// Resize pane with -Z (zoom toggle).
#[test]
fn resize_pane_zoom() {
    let tmux = TmuxServer::new("resizep_zoom");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["splitw", "-d"]);

    // Zoom in
    tmux.run(&["resizep", "-Z", "-t", ":0.0"]);
    let zoomed = tmux.display("#{window_zoomed_flag}");
    assert_eq!(zoomed, "1");

    // Zoom out
    tmux.run(&["resizep", "-Z", "-t", ":0.0"]);
    let unzoomed = tmux.display("#{window_zoomed_flag}");
    assert_eq!(unzoomed, "0");
}

/// Resize pane with -x (absolute width).
#[test]
fn resize_pane_absolute_width() {
    let tmux = TmuxServer::new("resizep_abs_w");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["splitw", "-h", "-d"]);

    tmux.run(&["resizep", "-x", "20", "-t", ":0.0"]);
    let w: u32 = tmux
        .run(&["display", "-t", ":0.0", "-p", "#{pane_width}"])
        .trim()
        .parse()
        .unwrap();
    assert_eq!(w, 20);
}

/// Resize pane with -y (absolute height).
#[test]
fn resize_pane_absolute_height() {
    let tmux = TmuxServer::new("resizep_abs_h");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["splitw", "-d"]);

    tmux.run(&["resizep", "-y", "5", "-t", ":0.0"]);
    let h: u32 = tmux
        .run(&["display", "-t", ":0.0", "-p", "#{pane_height}"])
        .trim()
        .parse()
        .unwrap();
    assert_eq!(h, 5);
}

/// Resize pane default adjustment (1 cell).
#[test]
fn resize_pane_default_adjustment() {
    let tmux = TmuxServer::new("resizep_default");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["splitw", "-d"]);

    let h_before: u32 = tmux
        .run(&["display", "-t", ":0.0", "-p", "#{pane_height}"])
        .trim()
        .parse()
        .unwrap();
    tmux.run(&["resizep", "-D", "-t", ":0.0"]);
    let h_after: u32 = tmux
        .run(&["display", "-t", ":0.0", "-p", "#{pane_height}"])
        .trim()
        .parse()
        .unwrap();

    assert_eq!(
        h_after - h_before,
        1,
        "default -D adjustment should grow pane 0 by 1"
    );
}

/// Resize pane with -T (trim).
#[test]
fn resize_pane_trim() {
    let tmux = TmuxServer::new("resizep_trim");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // -T trims lines below cursor from history
    let result = tmux.try_run(&["resizep", "-T"]);
    // Should succeed (or at least not crash)
    assert!(result.status.success());
}

/// -T in copy-mode returns early (modes list is non-empty).
#[test]
fn resize_pane_trim_in_copy_mode() {
    let tmux = TmuxServer::new("resizep_trim_mode");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Enter copy-mode so modes queue is non-empty
    tmux.run(&["copy-mode"]);
    let result = tmux.try_run(&["resizep", "-T"]);
    assert!(
        result.status.success(),
        "-T in copy-mode should return normally"
    );
}

/// Invalid adjustment value triggers strtonum error path.
#[test]
fn resize_pane_invalid_adjustment() {
    let tmux = TmuxServer::new("resizep_bad_adj");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["splitw", "-d"]);

    // adjustment of 0 is below minimum (1)
    let result = tmux.try_run(&["resizep", "-D", "0"]);
    assert!(!result.status.success(), "adjustment 0 should fail");

    // negative adjustment
    let result = tmux.try_run(&["resizep", "-D", "-5"]);
    assert!(!result.status.success(), "negative adjustment should fail");

    // non-numeric adjustment
    let result = tmux.try_run(&["resizep", "-D", "abc"]);
    assert!(
        !result.status.success(),
        "non-numeric adjustment should fail"
    );
}

/// Invalid -x value triggers args_percentage error.
#[test]
fn resize_pane_invalid_x() {
    let tmux = TmuxServer::new("resizep_bad_x");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["splitw", "-h", "-d"]);

    let result = tmux.try_run(&["resizep", "-x", "abc"]);
    assert!(
        !result.status.success(),
        "-x with non-numeric value should fail"
    );
}

/// Invalid -y value triggers args_percentage error.
#[test]
fn resize_pane_invalid_y() {
    let tmux = TmuxServer::new("resizep_bad_y");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["splitw", "-d"]);

    let result = tmux.try_run(&["resizep", "-y", "abc"]);
    assert!(
        !result.status.success(),
        "-y with non-numeric value should fail"
    );
}

/// -x with percentage value exercises args_percentage percent path.
#[test]
fn resize_pane_x_percentage() {
    let tmux = TmuxServer::new("resizep_x_pct");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["splitw", "-h", "-d"]);

    tmux.run(&["resizep", "-x", "50%", "-t", ":0.0"]);
    let w: u32 = tmux
        .run(&["display", "-t", ":0.0", "-p", "#{pane_width}"])
        .trim()
        .parse()
        .unwrap();
    // 50% of 80 = 40; allow some tolerance for borders
    assert!(w >= 38 && w <= 42, "50% of 80 should be ~40, got {w}");
}

/// -y with percentage value exercises args_percentage percent path.
#[test]
fn resize_pane_y_percentage() {
    let tmux = TmuxServer::new("resizep_y_pct");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["splitw", "-d"]);

    tmux.run(&["resizep", "-y", "50%", "-t", ":0.0"]);
    let h: u32 = tmux
        .run(&["display", "-t", ":0.0", "-p", "#{pane_height}"])
        .trim()
        .parse()
        .unwrap();
    // 50% of 24 = 12; allow some tolerance for status bar
    assert!(h >= 10 && h <= 14, "50% of 24 should be ~12, got {h}");
}

/// -y with pane-border-status top exercises the PANE_STATUS_TOP branch.
#[test]
fn resize_pane_y_with_border_status_top() {
    let tmux = TmuxServer::new("resizep_y_bst");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["splitw", "-d"]);
    tmux.run(&["set", "-w", "pane-border-status", "top"]);

    tmux.run(&["resizep", "-y", "5", "-t", ":0.0"]);
    let h: u32 = tmux
        .run(&["display", "-t", ":0.0", "-p", "#{pane_height}"])
        .trim()
        .parse()
        .unwrap();
    // Exact value depends on border adjustment; just verify it succeeded
    assert!(
        h >= 4 && h <= 7,
        "pane height should be around 5 with border-status top, got {h}"
    );
}

/// -y with pane-border-status bottom exercises the PANE_STATUS_BOTTOM branch.
#[test]
fn resize_pane_y_with_border_status_bottom() {
    let tmux = TmuxServer::new("resizep_y_bsb");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["splitw", "-d"]);
    tmux.run(&["set", "-w", "pane-border-status", "bottom"]);

    tmux.run(&["resizep", "-y", "5", "-t", ":0.0"]);
    let h: u32 = tmux
        .run(&["display", "-t", ":0.0", "-p", "#{pane_height}"])
        .trim()
        .parse()
        .unwrap();
    assert!(
        h >= 4 && h <= 7,
        "pane height should be around 5 with border-status bottom, got {h}"
    );
}

/// Combined -x and -y sets both width and height.
#[test]
fn resize_pane_x_and_y_combined() {
    let tmux = TmuxServer::new("resizep_xy");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    // Create a 2x2 grid so both horizontal and vertical resize are possible
    tmux.run(&["splitw", "-h", "-d"]);
    tmux.run(&["splitw", "-d"]);

    tmux.run(&["resizep", "-x", "30", "-y", "8", "-t", ":0.0"]);
    let w: u32 = tmux
        .run(&["display", "-t", ":0.0", "-p", "#{pane_width}"])
        .trim()
        .parse()
        .unwrap();
    let h: u32 = tmux
        .run(&["display", "-t", ":0.0", "-p", "#{pane_height}"])
        .trim()
        .parse()
        .unwrap();
    assert_eq!(w, 30, "width should be 30");
    assert_eq!(h, 8, "height should be 8");
}

/// Resize while zoomed unzooms first (server_unzoom_window path, line 82).
#[test]
fn resize_pane_unzooms_before_resize() {
    let tmux = TmuxServer::new("resizep_unzoom");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["splitw", "-d"]);

    // Zoom the pane first
    tmux.run(&["resizep", "-Z", "-t", ":0.0"]);
    assert_eq!(tmux.display("#{window_zoomed_flag}"), "1");

    // Now do a directional resize (not -Z), which should unzoom
    tmux.run(&["resizep", "-D", "-t", ":0.0", "2"]);
    assert_eq!(
        tmux.display("#{window_zoomed_flag}"),
        "0",
        "directional resize should unzoom the window"
    );
}

/// Resize with no direction flags and no -x/-y/-Z/-T/-M just calls server_redraw_window.
#[test]
fn resize_pane_no_direction() {
    let tmux = TmuxServer::new("resizep_nodir");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["splitw", "-d"]);

    let h_before: u32 = tmux
        .run(&["display", "-t", ":0.0", "-p", "#{pane_height}"])
        .trim()
        .parse()
        .unwrap();
    // No direction flag: falls through all if/else if branches, just redraws
    let result = tmux.try_run(&["resizep", "-t", ":0.0"]);
    assert!(result.status.success());
    let h_after: u32 = tmux
        .run(&["display", "-t", ":0.0", "-p", "#{pane_height}"])
        .trim()
        .parse()
        .unwrap();
    assert_eq!(
        h_before, h_after,
        "no direction flag should not change size"
    );
}

/// -y applied to the second pane (pane 1) exercises different yoff conditions.
#[test]
fn resize_pane_y_second_pane() {
    let tmux = TmuxServer::new("resizep_y_p1");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["splitw", "-d"]);

    tmux.run(&["resizep", "-y", "8", "-t", ":0.1"]);
    let h: u32 = tmux
        .run(&["display", "-t", ":0.1", "-p", "#{pane_height}"])
        .trim()
        .parse()
        .unwrap();
    assert_eq!(h, 8, "second pane height should be 8");
}

/// -y with border-status top on second pane (different yoff value).
#[test]
fn resize_pane_y_border_top_second_pane() {
    let tmux = TmuxServer::new("resizep_y_bt_p1");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["splitw", "-d"]);
    tmux.run(&["set", "-w", "pane-border-status", "top"]);

    tmux.run(&["resizep", "-y", "5", "-t", ":0.1"]);
    let h: u32 = tmux
        .run(&["display", "-t", ":0.1", "-p", "#{pane_height}"])
        .trim()
        .parse()
        .unwrap();
    assert!(
        h >= 4 && h <= 7,
        "second pane with border-status top should resize to ~5, got {h}"
    );
}

/// -y with border-status bottom on second pane (tests bottom border path
/// where pane is at the bottom of the window).
#[test]
fn resize_pane_y_border_bottom_second_pane() {
    let tmux = TmuxServer::new("resizep_y_bb_p1");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["splitw", "-d"]);
    tmux.run(&["set", "-w", "pane-border-status", "bottom"]);

    tmux.run(&["resizep", "-y", "5", "-t", ":0.1"]);
    let h: u32 = tmux
        .run(&["display", "-t", ":0.1", "-p", "#{pane_height}"])
        .trim()
        .parse()
        .unwrap();
    assert!(
        h >= 4 && h <= 7,
        "second pane with border-status bottom should resize to ~5, got {h}"
    );
}

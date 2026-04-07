use super::*;

/// Resize window with explicit -x width.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn resize_window_width() {
    let tmux = TmuxServer::new("resizew_x");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["resizew", "-x", "60"]);
    let w = tmux.display("#{window_width}");
    assert_eq!(w.trim(), "60");
}

/// Resize window with explicit -y height.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn resize_window_height() {
    let tmux = TmuxServer::new("resizew_y");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["resizew", "-y", "16"]);
    let h = tmux.display("#{window_height}");
    assert_eq!(h.trim(), "16");
}

/// Resize window with both -x and -y.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn resize_window_both() {
    let tmux = TmuxServer::new("resizew_xy");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["resizew", "-x", "100", "-y", "30"]);
    let w = tmux.display("#{window_width}");
    let h = tmux.display("#{window_height}");
    assert_eq!(w.trim(), "100");
    assert_eq!(h.trim(), "30");
}

/// Resize window with -L (shrink width).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn resize_window_left() {
    let tmux = TmuxServer::new("resizew_l");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["resizew", "-L"]);
    let w = tmux.display("#{window_width}");
    assert_eq!(w.trim(), "79");
}

/// Resize window with -R (grow width).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn resize_window_right() {
    let tmux = TmuxServer::new("resizew_r");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["resizew", "-R"]);
    let w = tmux.display("#{window_width}");
    assert_eq!(w.trim(), "81");
}

/// Resize window with -U (shrink height).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn resize_window_up() {
    let tmux = TmuxServer::new("resizew_u");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["resizew", "-U"]);
    let h = tmux.display("#{window_height}");
    assert_eq!(h.trim(), "23");
}

/// Resize window with -D (grow height).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn resize_window_down() {
    let tmux = TmuxServer::new("resizew_d");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["resizew", "-D"]);
    let h = tmux.display("#{window_height}");
    assert_eq!(h.trim(), "25");
}

/// Resize window with custom adjustment value.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn resize_window_custom_adjust() {
    let tmux = TmuxServer::new("resizew_adj");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["resizew", "-R", "10"]);
    let w = tmux.display("#{window_width}");
    assert_eq!(w.trim(), "90");
}

/// Resize window with -A (largest size).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn resize_window_largest() {
    let tmux = TmuxServer::new("resizew_big");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["resizew", "-A"]);
}

/// Resize window with -a (smallest size).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn resize_window_smallest() {
    let tmux = TmuxServer::new("resizew_small");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["resizew", "-a"]);
}

/// Resize -L with adjustment larger than width (underflow guard).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn resize_window_left_underflow() {
    let tmux = TmuxServer::new("resizew_l_uf");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Adjustment larger than current width - guard prevents underflow
    tmux.run(&["resizew", "-L", "999"]);
}

/// Resize -U with adjustment larger than height (underflow guard).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn resize_window_up_underflow() {
    let tmux = TmuxServer::new("resizew_u_uf");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Adjustment larger than current height - guard prevents underflow
    tmux.run(&["resizew", "-U", "999"]);
}

/// Invalid adjustment value.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn resize_window_invalid_adjust() {
    let tmux = TmuxServer::new("resizew_badadj");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let result = tmux.try_run(&["resizew", "-R", "0"]);
    assert!(!result.status.success());
}

/// Invalid width value.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn resize_window_invalid_width() {
    let tmux = TmuxServer::new("resizew_badx");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let result = tmux.try_run(&["resizew", "-x", "0"]);
    assert!(!result.status.success());
}

/// Invalid height value.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn resize_window_invalid_height() {
    let tmux = TmuxServer::new("resizew_bady");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let result = tmux.try_run(&["resizew", "-y", "0"]);
    assert!(!result.status.success());
}

/// Test resize-window: explicit size, directional, adjustment, -A/-a, errors.

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn resize_window_explicit() {
    let tmux = TmuxServer::new("resizew_explicit");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // resize-window -x 100 -y 30
    tmux.run(&["resize-window", "-x", "100", "-y", "30"]);
    let w = tmux.display("#{window_width}");
    let h = tmux.display("#{window_height}");
    assert_eq!(w, "100");
    assert_eq!(h, "30");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn resize_window_left_02() {
    let tmux = TmuxServer::new("resizew_left");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // -L: shrink width
    tmux.run(&["resize-window", "-L", "5"]);
    let w = tmux.display("#{window_width}");
    assert_eq!(w, "75");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn resize_window_right_02() {
    let tmux = TmuxServer::new("resizew_right");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // -R: grow width
    tmux.run(&["resize-window", "-R", "10"]);
    let w = tmux.display("#{window_width}");
    assert_eq!(w, "90");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn resize_window_up_02() {
    let tmux = TmuxServer::new("resizew_up");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // -U: shrink height
    tmux.run(&["resize-window", "-U", "4"]);
    let h = tmux.display("#{window_height}");
    assert_eq!(h, "20");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn resize_window_down_02() {
    let tmux = TmuxServer::new("resizew_down");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // -D: grow height
    tmux.run(&["resize-window", "-D", "6"]);
    let h = tmux.display("#{window_height}");
    assert_eq!(h, "30");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn resize_window_default_adjust() {
    let tmux = TmuxServer::new("resizew_defadj");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // -R without adjustment: defaults to 1
    tmux.run(&["resize-window", "-R"]);
    let w = tmux.display("#{window_width}");
    assert_eq!(w, "81");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn resize_window_largest_02() {
    let tmux = TmuxServer::new("resizew_largest");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // -A: adjust to largest client
    tmux.run(&["resize-window", "-A"]);
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn resize_window_smallest_02() {
    let tmux = TmuxServer::new("resizew_smallest");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // -a: adjust to smallest client
    tmux.run(&["resize-window", "-a"]);
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn resize_window_bad_adjustment() {
    let tmux = TmuxServer::new("resizew_badadj");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Invalid adjustment
    let out = tmux.try_run(&["resize-window", "-R", "notanumber"]);
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("adjustment"), "got: {stderr}");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn resize_window_bad_x() {
    let tmux = TmuxServer::new("resizew_badx");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let out = tmux.try_run(&["resize-window", "-x", "0"]);
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("width"), "got: {stderr}");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn resize_window_bad_y() {
    let tmux = TmuxServer::new("resizew_bady");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let out = tmux.try_run(&["resize-window", "-y", "0"]);
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("height"), "got: {stderr}");
}

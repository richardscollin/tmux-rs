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

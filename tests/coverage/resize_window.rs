use super::*;

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
fn resize_window_left() {
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
fn resize_window_right() {
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
fn resize_window_up() {
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
fn resize_window_down() {
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
fn resize_window_largest() {
    let tmux = TmuxServer::new("resizew_largest");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // -A: adjust to largest client
    tmux.run(&["resize-window", "-A"]);
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn resize_window_smallest() {
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

use super::*;

/// Enter copy-mode and exit with -q.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn copy_mode_enter_and_quit() {
    let tmux = TmuxServer::new("copymode_q");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["copy-mode"]);
    let mode = tmux.display("#{pane_mode}");
    assert_eq!(mode.trim(), "copy-mode");

    // -q exits all modes
    tmux.run(&["copy-mode", "-q"]);
    let mode = tmux.display("#{pane_mode}");
    assert!(
        mode.trim().is_empty(),
        "expected no mode after -q, got: {mode}"
    );
}

/// Enter copy-mode with -u (page up).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn copy_mode_page_up() {
    let tmux = TmuxServer::new("copymode_u");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["copy-mode", "-u"]);
    let mode = tmux.display("#{pane_mode}");
    assert_eq!(mode.trim(), "copy-mode");
}

/// Enter copy-mode with -d (page down, only meaningful if already in copy mode).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn copy_mode_page_down() {
    let tmux = TmuxServer::new("copymode_d");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Enter copy mode first, then page down
    tmux.run(&["copy-mode"]);
    tmux.run(&["copy-mode", "-d"]);
    let mode = tmux.display("#{pane_mode}");
    assert_eq!(mode.trim(), "copy-mode");
}

/// Enter copy-mode with -de (page down with scroll-exit).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn copy_mode_page_down_exit() {
    let tmux = TmuxServer::new("copymode_de");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Enter copy mode, scroll up first, then page down with -e
    tmux.run(&["copy-mode", "-u"]);
    tmux.run(&["copy-mode", "-d", "-e"]);
}

/// Enter copy-mode with -s (source pane).
#[test]
#[ignore = "crashes server: copy-mode -s with different source pane causes server exit"]
fn copy_mode_source_pane() {
    let tmux = TmuxServer::new("copymode_s");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Create a second pane, then enter copy mode on pane 0 with source pane 1
    tmux.run(&["splitw", "-d"]);
    tmux.run(&["copy-mode", "-s", ":.1", "-t", ":.0"]);
    let mode = tmux.display("#{pane_mode}");
    assert_eq!(mode.trim(), "copy-mode");
}

/// Enter copy-mode with -H (hides position).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn copy_mode_hide_position() {
    let tmux = TmuxServer::new("copymode_h");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["copy-mode", "-H"]);
    let mode = tmux.display("#{pane_mode}");
    assert_eq!(mode.trim(), "copy-mode");
}

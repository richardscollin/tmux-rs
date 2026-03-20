use super::*;

/// Test copy-mode: enter, quit (-q), page up (-u), page down (-d).

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn copy_mode_enter_quit() {
    let tmux = TmuxServer::new("copymode_eq");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Enter copy mode
    tmux.run(&["copy-mode"]);
    let mode = tmux.display("#{pane_mode}");
    assert_eq!(mode, "copy-mode");

    // Quit copy mode with -q
    tmux.run(&["copy-mode", "-q"]);
    let mode = tmux.display("#{pane_mode}");
    assert_eq!(mode, "", "should exit copy mode with -q");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn copy_mode_page_up() {
    let tmux = TmuxServer::new("copymode_pu");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // copy-mode -u: enter copy mode and page up
    tmux.run(&["copy-mode", "-u"]);
    let mode = tmux.display("#{pane_mode}");
    assert_eq!(mode, "copy-mode");

    tmux.run(&["copy-mode", "-q"]);
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn copy_mode_page_down() {
    let tmux = TmuxServer::new("copymode_pd");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Enter copy mode first, then page down
    tmux.run(&["copy-mode"]);
    tmux.run(&["copy-mode", "-d"]);
    let mode = tmux.display("#{pane_mode}");
    assert_eq!(mode, "copy-mode");

    tmux.run(&["copy-mode", "-q"]);
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn copy_mode_page_down_half() {
    let tmux = TmuxServer::new("copymode_pdh");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Enter copy mode, page down with -e (exit at bottom)
    tmux.run(&["copy-mode"]);
    tmux.run(&["copy-mode", "-de"]);

    // May or may not still be in copy mode depending on scrollback
    tmux.run(&["copy-mode", "-q"]);
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn copy_mode_history() {
    let tmux = TmuxServer::new("copymode_hist");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // copy-mode -H: enter with history (shows scrollback)
    tmux.run(&["copy-mode", "-H"]);
    let mode = tmux.display("#{pane_mode}");
    assert_eq!(mode, "copy-mode");
    tmux.run(&["copy-mode", "-q"]);
}

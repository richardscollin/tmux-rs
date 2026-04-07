use super::*;

/// Extra window copy mode tests to exercise uncovered paths.

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn copy_mode_centre_vertical() {
    let tmux = TmuxServer::new("wc_centre_v");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["copy-mode"]);
    tmux.run(&["send-keys", "-X", "middle-line"]);
    tmux.run(&["copy-mode", "-q"]);
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn copy_mode_search_forward() {
    let tmux = TmuxServer::new("wc_searchfwd");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["send-keys", "-l", "searchable text here"]);
    sleep_ms(100);

    // search-forward opens a prompt — just exercise entering it
    tmux.run(&["copy-mode"]);
    let out = tmux.try_run(&["send-keys", "-X", "search-forward"]);
    let _ = out;
    // Cancel any prompt
    tmux.run(&["send-keys", "Escape"]);
    tmux.run(&["copy-mode", "-q"]);
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn copy_mode_search_backward() {
    let tmux = TmuxServer::new("wc_searchback");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["copy-mode"]);
    let out = tmux.try_run(&["send-keys", "-X", "search-backward"]);
    let _ = out;
    tmux.run(&["send-keys", "Escape"]);
    tmux.run(&["copy-mode", "-q"]);
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn copy_mode_next_prev_search() {
    let tmux = TmuxServer::new("wc_npsearch");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Just exercise search-again/search-reverse (they may fail if no prior search)
    tmux.run(&["copy-mode"]);
    let _ = tmux.try_run(&["send-keys", "-X", "search-again"]);
    let _ = tmux.try_run(&["send-keys", "-X", "search-reverse"]);
    tmux.run(&["copy-mode", "-q"]);
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn copy_mode_goto_line() {
    let tmux = TmuxServer::new("wc_gotoline");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["copy-mode"]);
    let out = tmux.try_run(&["send-keys", "-X", "goto-line"]);
    let _ = out;
    tmux.run(&["send-keys", "Escape"]);
    tmux.run(&["copy-mode", "-q"]);
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn copy_mode_rectangle_toggle() {
    let tmux = TmuxServer::new("wc_rect");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["copy-mode"]);
    tmux.run(&["send-keys", "-X", "begin-selection"]);
    tmux.run(&["send-keys", "-X", "rectangle-toggle"]);
    tmux.run(&["send-keys", "-X", "cancel"]);
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn copy_mode_top_bottom_line() {
    let tmux = TmuxServer::new("wc_topbot");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["copy-mode"]);
    tmux.run(&["send-keys", "-X", "top-line"]);
    tmux.run(&["send-keys", "-X", "bottom-line"]);
    tmux.run(&["send-keys", "-X", "cancel"]);
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn copy_mode_clear_selection() {
    let tmux = TmuxServer::new("wc_clearsel");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["copy-mode"]);
    tmux.run(&["send-keys", "-X", "begin-selection"]);
    tmux.run(&["send-keys", "-X", "clear-selection"]);
    tmux.run(&["send-keys", "-X", "cancel"]);
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn copy_mode_search_incremental() {
    let tmux = TmuxServer::new("wc_incsearch");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["copy-mode"]);
    let out = tmux.try_run(&["send-keys", "-X", "search-forward-incremental"]);
    let _ = out;
    tmux.run(&["send-keys", "Escape"]);
    tmux.run(&["copy-mode", "-q"]);
}

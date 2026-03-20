use super::*;

/// Test capture-pane and clear-history: basic capture, -p (print),
/// -b (buffer), -e (escape), -C (C-style escape), -J (join), -T (trim),
/// -N (no trim), alternate screen, clear-history, -S/-E (range).

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn capture_pane_print() {
    let tmux = TmuxServer::new("cappane_print");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // -p: print to stdout
    let out = tmux.run(&["capture-pane", "-p"]);
    // Should return pane content (may be mostly empty)
    let _ = out;
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn capture_pane_to_buffer() {
    let tmux = TmuxServer::new("cappane_buf");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // -b: save to named buffer
    tmux.run(&["capture-pane", "-b", "capbuf"]);
    let out = tmux.run(&["show-buffer", "-b", "capbuf"]);
    let _ = out;
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn capture_pane_with_escapes() {
    let tmux = TmuxServer::new("cappane_esc");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // -e: include escape sequences
    let out = tmux.run(&["capture-pane", "-p", "-e"]);
    let _ = out;
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn capture_pane_c_escape() {
    let tmux = TmuxServer::new("cappane_cesc");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // -C: C-style escape (octal escapes for non-printable)
    let out = tmux.run(&["capture-pane", "-p", "-C"]);
    let _ = out;
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn capture_pane_join_lines() {
    let tmux = TmuxServer::new("cappane_join");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // -J: join wrapped lines
    let out = tmux.run(&["capture-pane", "-p", "-J"]);
    let _ = out;
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn capture_pane_no_trim() {
    let tmux = TmuxServer::new("cappane_notrim");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // -N: do not trim trailing spaces
    let out = tmux.run(&["capture-pane", "-p", "-N"]);
    let _ = out;
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn capture_pane_no_trailing_empty() {
    let tmux = TmuxServer::new("cappane_notempty");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // -T: do not include trailing empty lines
    let out = tmux.run(&["capture-pane", "-p", "-T"]);
    let _ = out;
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn capture_pane_range() {
    let tmux = TmuxServer::new("cappane_range");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // -S/-E: capture a specific range of lines
    let out = tmux.run(&["capture-pane", "-p", "-S", "0", "-E", "5"]);
    let _ = out;
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn capture_pane_negative_range() {
    let tmux = TmuxServer::new("cappane_negrange");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Negative range: relative to scrollback
    let out = tmux.run(&["capture-pane", "-p", "-S", "-", "-E", "-"]);
    let _ = out;
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn capture_pane_alternate_screen() {
    let tmux = TmuxServer::new("cappane_alt");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // -a: capture alternate screen (no alternate active — should error or be empty)
    let out = tmux.try_run(&["capture-pane", "-p", "-a"]);
    // May fail with "no alternate screen" or succeed with empty
    let _ = out;
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn capture_pane_alternate_quiet() {
    let tmux = TmuxServer::new("cappane_altq");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // -a -q: alternate screen, quiet — should succeed with empty output
    let out = tmux.try_run(&["capture-pane", "-p", "-a", "-q"]);
    assert!(
        out.status.success(),
        "capture-pane -a -q should succeed silently"
    );
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn clear_history() {
    let tmux = TmuxServer::new("clearhist");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // clear-history
    tmux.run(&["clear-history"]);
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn clear_history_with_hyperlinks() {
    let tmux = TmuxServer::new("clearhist_h");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // clear-history -H: also clear hyperlinks
    tmux.run(&["clear-history", "-H"]);
}

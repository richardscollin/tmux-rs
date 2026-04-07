use super::*;

/// Basic capture-pane with -p (print to stdout).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn capture_pane_print() {
    let tmux = TmuxServer::new("capturep_p");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let out = tmux.run(&["capturep", "-p"]);
    // Output should be the pane contents (possibly empty lines)
    let _ = out;
}

/// capture-pane with -p via control mode (exercises control_write path).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn capture_pane_print_control() {
    let tmux = TmuxServer::new("capturep_p_ctrl");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let cmd = "capture-pane -p\ndetach-client\n";
    let output = tmux.run_with_stdin(&["-C", "attach"], cmd.as_bytes());
    assert!(output.status.success());
}

/// capture-pane with -S and -E (line ranges).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn capture_pane_range() {
    let tmux = TmuxServer::new("capturep_SE");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let out = tmux.run(&["capturep", "-p", "-S", "0", "-E", "5"]);
    let _ = out;
}

/// capture-pane with -S - and -E - (full range).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn capture_pane_range_dash() {
    let tmux = TmuxServer::new("capturep_dash");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let out = tmux.run(&["capturep", "-p", "-S", "-", "-E", "-"]);
    let _ = out;
}

/// capture-pane with negative line numbers.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn capture_pane_negative_range() {
    let tmux = TmuxServer::new("capturep_neg");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let out = tmux.run(&["capturep", "-p", "-S", "-5", "-E", "-1"]);
    let _ = out;
}

/// capture-pane with -J (join wrapped lines).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn capture_pane_join() {
    let tmux = TmuxServer::new("capturep_J");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let out = tmux.run(&["capturep", "-p", "-J"]);
    let _ = out;
}

/// capture-pane with -N (no trailing spaces trim, preserve empty cells).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn capture_pane_no_trim() {
    let tmux = TmuxServer::new("capturep_N");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let out = tmux.run(&["capturep", "-p", "-N"]);
    let _ = out;
}

/// capture-pane with -T (include trailing spaces).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn capture_pane_trailing() {
    let tmux = TmuxServer::new("capturep_T");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let out = tmux.run(&["capturep", "-p", "-T"]);
    let _ = out;
}

/// capture-pane with -C (escape non-printable).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn capture_pane_escape() {
    let tmux = TmuxServer::new("capturep_C");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let out = tmux.run(&["capturep", "-p", "-C"]);
    let _ = out;
}

/// capture-pane with -e (include escape sequences).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn capture_pane_sequences() {
    let tmux = TmuxServer::new("capturep_e");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let out = tmux.run(&["capturep", "-p", "-e"]);
    let _ = out;
}

/// capture-pane with -a (alternate screen, error when no alternate).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn capture_pane_alternate_error() {
    let tmux = TmuxServer::new("capturep_a");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let result = tmux.try_run(&["capturep", "-p", "-a"]);
    assert!(!result.status.success());
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        stderr.contains("no alternate screen"),
        "expected 'no alternate screen' error, got: {stderr}"
    );
}

/// capture-pane with -a -q (quiet, suppress alternate screen error).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn capture_pane_alternate_quiet() {
    let tmux = TmuxServer::new("capturep_aq");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let result = tmux.try_run(&["capturep", "-p", "-a", "-q"]);
    assert!(
        result.status.success(),
        "-q should suppress alternate screen error"
    );
}

/// capture-pane with -b (store in named paste buffer).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn capture_pane_buffer() {
    let tmux = TmuxServer::new("capturep_b");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["capturep", "-b", "mybuf"]);
    let out = tmux.run(&["show-buffer", "-b", "mybuf"]);
    // Buffer should exist and have content
    let _ = out;
}

/// capture-pane without -p or -b (default: store in unnamed buffer).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn capture_pane_default_buffer() {
    let tmux = TmuxServer::new("capturep_defbuf");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["capturep"]);
    // Should have a paste buffer now
    let out = tmux.run(&["list-buffers"]);
    assert!(
        !out.trim().is_empty(),
        "expected at least one buffer after capture"
    );
}

/// clear-history command.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn clear_history() {
    let tmux = TmuxServer::new("clearhist");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["clearhist"]);
    let has = tmux.try_run(&["has-session"]);
    assert!(has.status.success());
}

/// clear-history with -H (clear hyperlinks).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn clear_history_hyperlinks() {
    let tmux = TmuxServer::new("clearhist_H");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["clearhist", "-H"]);
    let has = tmux.try_run(&["has-session"]);
    assert!(has.status.success());
}

/// clear-history using alias.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn clear_history_alias() {
    let tmux = TmuxServer::new("clearhist_alias");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["clear-history"]);
    let has = tmux.try_run(&["has-session"]);
    assert!(has.status.success());
}

/// capture-pane with -M (mode screen).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn capture_pane_mode_screen() {
    let tmux = TmuxServer::new("capturep_M");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Enter copy mode to create a mode screen, then capture it
    tmux.run(&["copy-mode"]);
    sleep_ms(100);
    let out = tmux.run(&["capturep", "-p", "-M"]);
    let _ = out;
}

/// capture-pane with -P (pending input).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn capture_pane_pending() {
    let tmux = TmuxServer::new("capturep_pending");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // -P captures pending input (usually empty)
    let out = tmux.run(&["capturep", "-p", "-P"]);
    let _ = out;
}

/// capture-pane with -P -C (pending with escape).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn capture_pane_pending_escape() {
    let tmux = TmuxServer::new("capturep_PC");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let out = tmux.run(&["capturep", "-p", "-P", "-C"]);
    let _ = out;
}

/// capturep alias.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn capture_pane_alias() {
    let tmux = TmuxServer::new("capturep_alias");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["capture-pane", "-p"]);
}

/// capture-pane with multiple flags (-J -N -e combined).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn capture_pane_combined_flags() {
    let tmux = TmuxServer::new("capturep_combo");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let out = tmux.run(&["capturep", "-p", "-J", "-N", "-e"]);
    let _ = out;
}

/// capture-pane where bottom < top (gets swapped).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn capture_pane_swapped_range() {
    let tmux = TmuxServer::new("capturep_swap");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Start > End should get swapped internally
    let out = tmux.run(&["capturep", "-p", "-S", "5", "-E", "0"]);
    let _ = out;
}

/// capture-pane with very large negative start (clamps to 0).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn capture_pane_large_negative() {
    let tmux = TmuxServer::new("capturep_lneg");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let out = tmux.run(&["capturep", "-p", "-S", "-10000"]);
    let _ = out;
}

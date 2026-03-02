use super::*;

/// Send literal keys to a pane.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn send_keys_literal() {
    let tmux = TmuxServer::new("sendkeys_lit");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["send-keys", "-l", "hello"]);
}

/// Send named key (Enter).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn send_keys_named() {
    let tmux = TmuxServer::new("sendkeys_named");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["send-keys", "Enter"]);
}

/// Send keys with -R (reset terminal).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn send_keys_reset() {
    let tmux = TmuxServer::new("sendkeys_reset");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["send-keys", "-R"]);
}

/// Send keys with -R and literal text.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn send_keys_reset_and_text() {
    let tmux = TmuxServer::new("sendkeys_reset_txt");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["send-keys", "-R", "echo", "hello", "Enter"]);
}

/// Send keys with -H (hex).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn send_keys_hex() {
    let tmux = TmuxServer::new("sendkeys_hex");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Send 'A' (0x41) via hex
    tmux.run(&["send-keys", "-H", "41"]);
}

/// Send keys with -H (invalid hex).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn send_keys_hex_invalid() {
    let tmux = TmuxServer::new("sendkeys_hex_inv");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Invalid hex value, should not crash
    tmux.run(&["send-keys", "-H", "gg"]);
}

/// Send keys with -N (repeat count).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn send_keys_repeat() {
    let tmux = TmuxServer::new("sendkeys_repeat");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["send-keys", "-N", "3", "-l", "x"]);
}

/// Send keys with invalid -N (error).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn send_keys_repeat_invalid() {
    let tmux = TmuxServer::new("sendkeys_rep_inv");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let result = tmux.try_run(&["send-keys", "-N", "0", "-l", "x"]);
    assert!(!result.status.success());
}

/// Send prefix key.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn send_prefix() {
    let tmux = TmuxServer::new("sendprefix");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["send-prefix"]);
}

/// Send prefix2 key.
#[test]
#[ignore = "crashes server: send-prefix -2 causes server exit"]
fn send_prefix2() {
    let tmux = TmuxServer::new("sendprefix2");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["send-prefix", "-2"]);
}

/// Send keys using 'send' alias.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn send_keys_alias() {
    let tmux = TmuxServer::new("sendkeys_alias");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["send", "-l", "test"]);
}

/// Send keys -X in copy mode.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn send_keys_copy_mode_command() {
    let tmux = TmuxServer::new("sendkeys_X");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Enter copy mode, then send a command with -X
    tmux.run(&["copy-mode"]);
    tmux.run(&["send-keys", "-X", "cancel"]);

    // Should have exited copy mode
    let mode = tmux.display("#{pane_mode}");
    assert!(mode.trim().is_empty(), "expected no mode after cancel");
}

/// Send keys -X not in a mode (error).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn send_keys_x_not_in_mode() {
    let tmux = TmuxServer::new("sendkeys_X_nomode");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let result = tmux.try_run(&["send-keys", "-X", "cancel"]);
    assert!(!result.status.success());
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        stderr.contains("not in a mode"),
        "expected 'not in a mode' error, got: {stderr}"
    );
}

/// Send keys with -N in copy mode.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn send_keys_copy_mode_repeat() {
    let tmux = TmuxServer::new("sendkeys_N_copy");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["copy-mode"]);
    tmux.run(&["send-keys", "-N", "3", "-X", "cursor-down"]);
}

/// Send keys with -N only (no args, no -X) to repeat event key.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn send_keys_repeat_no_args() {
    let tmux = TmuxServer::new("sendkeys_N_noargs");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // -N with no keys and no -X repeats the event key
    tmux.run(&["send-keys", "-N", "1"]);
}

/// Send multiple key arguments.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn send_keys_multiple() {
    let tmux = TmuxServer::new("sendkeys_multi");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["send-keys", "e", "c", "h", "o", " ", "h", "i", "Enter"]);
}

/// Send unknown key name (treated as literal).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn send_keys_unknown_keyname() {
    let tmux = TmuxServer::new("sendkeys_unk");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Unknown key name falls through to literal
    tmux.run(&["send-keys", "notakey"]);
}

use super::*;

/// Test send-keys: literal, named keys, reset, hex, prefix, repeat, errors.

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn send_keys_literal() {
    let tmux = TmuxServer::new("sendkeys_lit");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // send-keys -l: send literal string
    tmux.run(&["send-keys", "-l", "hello"]);
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn send_keys_named() {
    let tmux = TmuxServer::new("sendkeys_named");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Send a named key
    tmux.run(&["send-keys", "Enter"]);
    tmux.run(&["send-keys", "C-c"]);
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn send_keys_reset() {
    let tmux = TmuxServer::new("sendkeys_reset");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // -R: reset terminal
    tmux.run(&["send-keys", "-R"]);
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn send_keys_reset_with_keys() {
    let tmux = TmuxServer::new("sendkeys_rk");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // -R with keys: reset then send
    tmux.run(&["send-keys", "-R", "Enter"]);
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn send_keys_hex() {
    let tmux = TmuxServer::new("sendkeys_hex");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // -H: send hex byte (0x41 = 'A')
    tmux.run(&["send-keys", "-H", "41"]);
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn send_keys_hex_invalid() {
    let tmux = TmuxServer::new("sendkeys_hexbad");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // -H with invalid hex: should be a no-op (not crash)
    tmux.run(&["send-keys", "-H", "ZZ"]);
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn send_prefix() {
    let tmux = TmuxServer::new("sendprefix");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // send-prefix: send the prefix key
    tmux.run(&["send-prefix"]);
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn send_prefix_2() {
    let tmux = TmuxServer::new("sendprefix2");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // BUG-004: send-prefix -2 crashes when prefix2 is None (default)
    // Set prefix2 to a real key first to avoid the crash
    tmux.run(&["set", "-g", "prefix2", "C-a"]);
    tmux.run(&["send-prefix", "-2"]);
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn send_keys_repeat() {
    let tmux = TmuxServer::new("sendkeys_rep");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // -N: repeat count
    tmux.run(&["send-keys", "-N", "3", "-l", "x"]);
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn send_keys_repeat_bad() {
    let tmux = TmuxServer::new("sendkeys_repbad");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // -N with invalid number
    let out = tmux.try_run(&["send-keys", "-N", "abc", "x"]);
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("repeat count"), "got: {stderr}");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn send_keys_copy_mode_command() {
    let tmux = TmuxServer::new("sendkeys_xcmd");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Enter copy mode, then send -X command
    tmux.run(&["copy-mode"]);
    tmux.run(&["send-keys", "-X", "cancel"]);
    let mode = tmux.display("#{pane_mode}");
    assert_eq!(mode, "", "should exit copy mode after cancel");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn send_keys_x_not_in_mode() {
    let tmux = TmuxServer::new("sendkeys_xnomode");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // -X without being in copy mode: should error
    let out = tmux.try_run(&["send-keys", "-X", "cancel"]);
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("not in a mode"), "got: {stderr}");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn send_keys_utf8() {
    let tmux = TmuxServer::new("sendkeys_utf8");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Send a multi-byte UTF-8 character literally
    tmux.run(&["send-keys", "-l", "é"]);
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn send_keys_repeat_with_n_only() {
    let tmux = TmuxServer::new("sendkeys_nonly");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // -N with no keys and no -R: exercises the "no count" + "N flag" early return
    tmux.run(&["send-keys", "-N", "3"]);
}

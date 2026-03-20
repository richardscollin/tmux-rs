use super::*;

/// Test set-option: global, session, window, pane, user options,
/// -u (unset), -o (only if not set), -q (quiet), -F (format), errors.

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn set_option_global() {
    let tmux = TmuxServer::new("setopt_global");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Set a global option
    tmux.run(&["set", "-g", "base-index", "1"]);
    let out = tmux.run(&["show-options", "-gv", "base-index"]);
    assert_eq!(out.trim(), "1");

    // Set it back
    tmux.run(&["set", "-g", "base-index", "0"]);
    let out = tmux.run(&["show-options", "-gv", "base-index"]);
    assert_eq!(out.trim(), "0");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn set_option_window() {
    let tmux = TmuxServer::new("setopt_window");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Set a window option via setw
    tmux.run(&["setw", "-g", "mode-keys", "vi"]);
    let out = tmux.run(&["showw", "-gv", "mode-keys"]);
    assert_eq!(out.trim(), "vi");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn set_option_user() {
    let tmux = TmuxServer::new("setopt_user");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Set a user option
    tmux.run(&["set", "-g", "@myvar", "myvalue"]);
    let out = tmux.run(&["show-options", "-gv", "@myvar"]);
    assert_eq!(out.trim(), "myvalue");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn set_option_user_append() {
    let tmux = TmuxServer::new("setopt_user_a");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Set then append to a user option
    tmux.run(&["set", "-g", "@appendvar", "hello"]);
    tmux.run(&["set", "-ag", "@appendvar", " world"]);
    let out = tmux.run(&["show-options", "-gv", "@appendvar"]);
    assert_eq!(out.trim(), "hello world");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn set_option_unset() {
    let tmux = TmuxServer::new("setopt_unset");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Set a user option then unset it
    tmux.run(&["set", "-g", "@tmpopt", "tmpval"]);
    tmux.run(&["set", "-gu", "@tmpopt"]);

    // Should not exist anymore
    let out = tmux.try_run(&["show-options", "-gv", "@tmpopt"]);
    assert!(!out.status.success() || String::from_utf8_lossy(&out.stdout).trim().is_empty());
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn set_option_only_if_not_set() {
    let tmux = TmuxServer::new("setopt_o");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Set with -o: only if not already set
    tmux.run(&["set", "-g", "@onceopt", "first"]);
    let out = tmux.try_run(&["set", "-go", "@onceopt", "second"]);
    assert!(!out.status.success(), "set -o should fail if already set");

    // Verify value unchanged
    let val = tmux.run(&["show-options", "-gv", "@onceopt"]);
    assert_eq!(val.trim(), "first");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn set_option_only_if_not_set_quiet() {
    let tmux = TmuxServer::new("setopt_oq");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["set", "-g", "@qopt", "val"]);
    // set -oq: quiet, should succeed silently even if already set
    let out = tmux.try_run(&["set", "-goq", "@qopt", "newval"]);
    assert!(out.status.success(), "set -oq should succeed silently");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn set_option_invalid() {
    let tmux = TmuxServer::new("setopt_invalid");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Invalid option name
    let out = tmux.try_run(&["set", "-g", "not-a-real-option", "value"]);
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("invalid option"), "got: {stderr}");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn set_option_invalid_quiet() {
    let tmux = TmuxServer::new("setopt_inv_q");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // -q: quiet mode
    let out = tmux.try_run(&["set", "-gq", "not-a-real-option", "value"]);
    assert!(out.status.success(), "set -q should succeed silently on invalid option");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn set_option_format_value() {
    let tmux = TmuxServer::new("setopt_fmt");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // set -F: expand format in value
    tmux.run(&["set", "-gF", "@fmtopt", "#{session_name}"]);
    let val = tmux.run(&["show-options", "-gv", "@fmtopt"]);
    // Should be the actual session name, not the literal format string
    assert!(!val.trim().contains("#{"), "format should be expanded, got: {val}");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn set_option_user_empty_value() {
    let tmux = TmuxServer::new("setopt_empty");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // set user option with no value: should error
    let out = tmux.try_run(&["set", "-g", "@novalue"]);
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("empty value"), "got: {stderr}");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn set_option_pane() {
    let tmux = TmuxServer::new("setopt_pane");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // set -p: pane option
    tmux.run(&["set", "-p", "remain-on-exit", "on"]);
    let val = tmux.run(&["show-options", "-pv", "remain-on-exit"]);
    assert_eq!(val.trim(), "on");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn set_hook_run() {
    let tmux = TmuxServer::new("sethook_run");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // set-hook -R: fire the hook immediately
    // Set a hook first
    tmux.run(&["set-hook", "-g", "client-attached", "display-message 'attached'"]);
    // Fire it with -R
    let out = tmux.try_run(&["set-hook", "-gR", "client-attached"]);
    let _ = out;
}

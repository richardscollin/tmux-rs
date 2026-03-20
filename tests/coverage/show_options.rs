use super::*;

/// Test show-options: global options, window options, specific option,
/// -v (value only), -A (include parent), -q (quiet), errors.

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn show_options_global() {
    let tmux = TmuxServer::new("showopts_global");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // show-options -g: show all global session options
    let out = tmux.run(&["show-options", "-g"]);
    assert!(
        out.contains("base-index"),
        "global options should include base-index"
    );
    assert!(
        out.contains("status"),
        "global options should include status"
    );
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn show_options_window() {
    let tmux = TmuxServer::new("showopts_window");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // show-window-options -g: show all global window options
    let out = tmux.run(&["show-window-options", "-g"]);
    assert!(
        out.contains("mode-keys"),
        "window options should include mode-keys, got: {}",
        &out[..out.len().min(200)]
    );
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn show_options_specific() {
    let tmux = TmuxServer::new("showopts_specific");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // show-options -g base-index: show a specific option
    let out = tmux.run(&["show-options", "-g", "base-index"]);
    assert!(out.contains("base-index"), "should show base-index option");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn show_options_value_only() {
    let tmux = TmuxServer::new("showopts_vonly");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // show-options -gv base-index: show value only
    let out = tmux.run(&["show-options", "-gv", "base-index"]);
    // Should just be the value (e.g., "0") without the option name
    assert!(
        !out.contains("base-index"),
        "with -v should not show option name"
    );
    assert!(!out.trim().is_empty(), "should have a value");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn show_options_with_parent() {
    let tmux = TmuxServer::new("showopts_parent");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // show-options -A: include parent (inherited) options
    let out = tmux.run(&["show-options", "-A"]);
    // With -A, inherited options should show with * suffix
    assert!(!out.is_empty(), "should have output with -A");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn show_options_invalid() {
    let tmux = TmuxServer::new("showopts_invalid");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // show-options with invalid option name: should error
    let out = tmux.try_run(&["show-options", "-g", "not-a-real-option"]);
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("invalid option"),
        "expected 'invalid option' error, got: {stderr}"
    );
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn show_options_quiet_invalid() {
    let tmux = TmuxServer::new("showopts_qinv");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // show-options -q with invalid option: should succeed silently
    let out = tmux.try_run(&["show-options", "-gq", "not-a-real-option"]);
    assert!(
        out.status.success(),
        "with -q should succeed even on invalid option"
    );
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn show_options_ambiguous() {
    let tmux = TmuxServer::new("showopts_ambig");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // An ambiguous prefix that matches multiple options
    // "status" might be ambiguous since there's status, status-style, etc.
    // Use a shorter prefix
    let out = tmux.try_run(&["show-options", "-g", "statu"]);
    // Could be ambiguous or match "status" exactly — exercise the code path
    let _ = out;
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn show_options_user_option() {
    let tmux = TmuxServer::new("showopts_user");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Set a user option (@-prefixed)
    tmux.run(&["set", "-g", "@myopt", "myval"]);
    let out = tmux.run(&["show-options", "-g", "@myopt"]);
    assert!(out.contains("myval"), "should show user option value");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn show_options_user_option_invalid() {
    let tmux = TmuxServer::new("showopts_user_inv");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // show-options for nonexistent @-option: should error
    let out = tmux.try_run(&["show-options", "-g", "@nosuchoption"]);
    assert!(!out.status.success());
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn show_options_user_option_quiet() {
    let tmux = TmuxServer::new("showopts_user_q");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // show-options -q for nonexistent @-option: should succeed silently
    let out = tmux.try_run(&["show-options", "-gq", "@nosuchoption"]);
    assert!(
        out.status.success(),
        "with -q should succeed on missing user option"
    );
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn show_options_session() {
    let tmux = TmuxServer::new("showopts_sess");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // show-options without -g: session-level options
    let out = tmux.run(&["show-options"]);
    // Session options may be empty if nothing is overridden, or contain overrides
    let _ = out;
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn show_options_pane() {
    let tmux = TmuxServer::new("showopts_pane");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // show-options -p: pane-level options
    let out = tmux.run(&["show-options", "-p"]);
    let _ = out;
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn show_hooks() {
    let tmux = TmuxServer::new("showopts_hooks");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // show-hooks -g: show global hooks
    let out = tmux.run(&["show-hooks", "-g"]);
    // May be empty if no hooks set
    let _ = out;
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn show_options_string_option() {
    let tmux = TmuxServer::new("showopts_str");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // show a string-type option to exercise the escaped output path
    let out = tmux.run(&["show-options", "-g", "default-command"]);
    assert!(out.contains("default-command"));
}

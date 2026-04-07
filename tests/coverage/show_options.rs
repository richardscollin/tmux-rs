use super::*;

/// Show all global options (default).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn show_options_global() {
    let tmux = TmuxServer::new("show_global");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let out = tmux.run(&["show", "-g"]);
    assert!(
        out.contains("status"),
        "expected global options to include 'status'"
    );
}

/// Show all window options with -w.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn show_options_window() {
    let tmux = TmuxServer::new("show_window");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Set a window option first to ensure there's output
    tmux.run(&["setw", "mode-keys", "vi"]);
    let out = tmux.run(&["show", "-w"]);
    assert!(out.contains("mode-keys"), "expected window options output");
}

/// Show all pane options with -p.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn show_options_pane() {
    let tmux = TmuxServer::new("show_pane");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let out = tmux.run(&["show", "-p"]);
    let _ = out;
}

/// Show specific option by name.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn show_options_specific() {
    let tmux = TmuxServer::new("show_specific");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let out = tmux.run(&["show", "-g", "status"]);
    assert!(out.contains("status"), "expected 'status' option in output");
}

/// Show option with -v (value only).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn show_options_value_only() {
    let tmux = TmuxServer::new("show_value");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let out = tmux.run(&["show", "-gv", "status"]);
    // -v prints just the value, not the option name
    assert!(
        !out.contains("status "),
        "expected just value, not 'name value'"
    );
}

/// Show option with -A (include inherited/parent).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn show_options_inherited() {
    let tmux = TmuxServer::new("show_inherited");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // -A shows inherited options (parent) with * suffix
    let out = tmux.run(&["show", "-A", "status"]);
    assert!(!out.trim().is_empty(), "expected output with -A");
}

/// Show option with -q (quiet, no error on invalid).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn show_options_quiet_invalid() {
    let tmux = TmuxServer::new("show_quiet");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // -q suppresses errors for invalid options
    let result = tmux.try_run(&["show", "-gq", "nonexistent-option-xyz"]);
    assert!(result.status.success(), "expected -q to suppress error");
}

/// Show invalid option without -q (error).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn show_options_invalid() {
    let tmux = TmuxServer::new("show_invalid");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let result = tmux.try_run(&["show", "-g", "nonexistent-option-xyz"]);
    assert!(!result.status.success());
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        stderr.contains("invalid option"),
        "expected 'invalid option' error, got: {stderr}"
    );
}

/// Show ambiguous option.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn show_options_ambiguous() {
    let tmux = TmuxServer::new("show_ambiguous");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // "status-" is ambiguous (status-left, status-right, etc.)
    let result = tmux.try_run(&["show", "-g", "status-"]);
    assert!(!result.status.success());
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        stderr.contains("ambiguous option"),
        "expected 'ambiguous option' error, got: {stderr}"
    );
}

/// Show window options with showw alias.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn show_window_options_alias() {
    let tmux = TmuxServer::new("showw_alias");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let out = tmux.run(&["showw", "-g"]);
    assert!(!out.trim().is_empty(), "expected showw output");
}

/// Show hooks with show-hooks.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn show_hooks_basic() {
    let tmux = TmuxServer::new("show_hooks");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Set a hook first, then show it
    tmux.run(&["set-hook", "-g", "after-new-session", "display 'hello'"]);
    let out = tmux.run(&["show-hooks", "-g"]);
    assert!(
        out.contains("after-new-session"),
        "expected hook in output, got: {out}"
    );
}

/// Show global options with -H (include hooks).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn show_options_with_hooks() {
    let tmux = TmuxServer::new("show_with_hooks");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["set-hook", "-g", "after-new-session", "display 'hello'"]);
    let out = tmux.run(&["show", "-gH"]);
    assert!(
        out.contains("after-new-session"),
        "expected hooks when using -H, got partial: {}",
        &out[..out.len().min(200)]
    );
}

/// Show pane options with -p.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn show_options_pane_specific() {
    let tmux = TmuxServer::new("show_pane_opt");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Set a pane option, then query it
    tmux.run(&["set", "-p", "remain-on-exit", "on"]);
    let out = tmux.run(&["show", "-p", "remain-on-exit"]);
    assert!(out.contains("on"), "expected 'on' in pane option output");
}

/// Show user option (@-prefixed).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn show_options_user_option() {
    let tmux = TmuxServer::new("show_user_opt");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["set", "-g", "@myvar", "hello"]);
    let out = tmux.run(&["show", "-g", "@myvar"]);
    assert!(out.contains("hello"), "expected user option value");
}

/// Show non-existent user option (@-prefixed, error).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn show_options_user_option_missing() {
    let tmux = TmuxServer::new("show_user_miss");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let result = tmux.try_run(&["show", "-g", "@nonexistent"]);
    assert!(!result.status.success());
}

/// Show non-existent user option with -q (quiet).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn show_options_user_option_quiet() {
    let tmux = TmuxServer::new("show_user_q");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let result = tmux.try_run(&["show", "-gq", "@nonexistent"]);
    assert!(
        result.status.success(),
        "-q should suppress missing user option error"
    );
}

/// Show all options with -A (inherited/parent values).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn show_options_all_inherited() {
    let tmux = TmuxServer::new("show_all_A");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // -A with no argument shows all options, with inherited values marked *
    let out = tmux.run(&["show", "-A"]);
    assert!(!out.trim().is_empty(), "expected output with -A");
}

/// Show server options with -s.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn show_options_server() {
    let tmux = TmuxServer::new("show_server");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let out = tmux.run(&["show", "-s"]);
    assert!(!out.trim().is_empty(), "expected server options output");
}

/// Show specific server option.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn show_options_server_specific() {
    let tmux = TmuxServer::new("show_srv_spec");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let out = tmux.run(&["show", "-s", "default-terminal"]);
    assert!(
        out.contains("default-terminal"),
        "expected 'default-terminal' in output"
    );
}

/// Show option with -v for array option (e.g., terminal-features).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn show_options_array_value() {
    let tmux = TmuxServer::new("show_arr_val");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let out = tmux.run(&["show", "-s", "terminal-features"]);
    // terminal-features is an array option
    assert!(!out.trim().is_empty(), "expected array option output");
}

/// Test show-options: global options, window options, specific option,
/// -v (value only), -A (include parent), -q (quiet), errors.

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn show_options_global_02() {
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
fn show_options_window_02() {
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
fn show_options_specific_02() {
    let tmux = TmuxServer::new("showopts_specific");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // show-options -g base-index: show a specific option
    let out = tmux.run(&["show-options", "-g", "base-index"]);
    assert!(out.contains("base-index"), "should show base-index option");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn show_options_value_only_02() {
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
fn show_options_invalid_02() {
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
fn show_options_quiet_invalid_02() {
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
fn show_options_ambiguous_02() {
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
fn show_options_user_option_02() {
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
fn show_options_user_option_quiet_02() {
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
fn show_options_pane_02() {
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

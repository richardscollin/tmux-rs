use super::*;

/// Show all global options (default).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn show_options_global() {
    let tmux = TmuxServer::new("show_global");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let out = tmux.run(&["show", "-g"]);
    assert!(out.contains("status"), "expected global options to include 'status'");
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
    assert!(!out.contains("status "), "expected just value, not 'name value'");
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
    assert!(result.status.success(), "-q should suppress missing user option error");
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

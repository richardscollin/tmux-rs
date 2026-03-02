use super::*;

/// Set a global option.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn set_option_global() {
    let tmux = TmuxServer::new("setopt_global");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["set", "-g", "status-position", "top"]);
    let out = tmux.run(&["show", "-gv", "status-position"]);
    assert_eq!(out.trim(), "top");
}

/// Set a window option with -w.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn set_option_window() {
    let tmux = TmuxServer::new("setopt_window");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["setw", "mode-keys", "vi"]);
    let out = tmux.run(&["showw", "-v", "mode-keys"]);
    assert_eq!(out.trim(), "vi");
}

/// Set with -F (format expansion).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn set_option_format() {
    let tmux = TmuxServer::new("setopt_format");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["set", "-g", "-F", "@myvar", "#{session_name}"]);
    let out = tmux.run(&["show", "-gv", "@myvar"]);
    assert!(!out.trim().is_empty(), "expected expanded format value");
}

/// Set with -u (unset).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn set_option_unset() {
    let tmux = TmuxServer::new("setopt_unset");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["set", "@myvar", "hello"]);
    tmux.run(&["set", "-u", "@myvar"]);
    let result = tmux.try_run(&["show", "@myvar"]);
    // After unset, should either error or show nothing
    assert!(
        !result.status.success() || String::from_utf8_lossy(&result.stdout).trim().is_empty(),
        "expected option to be unset"
    );
}

/// Set with -u on non-existent option (noop).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn set_option_unset_nonexistent() {
    let tmux = TmuxServer::new("setopt_unset_ne");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Unsetting a non-existent option should succeed silently
    tmux.run(&["set", "-u", "@nonexistent"]);
}

/// Set invalid option name.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn set_option_invalid() {
    let tmux = TmuxServer::new("setopt_invalid");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let result = tmux.try_run(&["set", "-g", "nonexistent-option-xyz", "val"]);
    assert!(!result.status.success());
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        stderr.contains("invalid option"),
        "expected 'invalid option' error, got: {stderr}"
    );
}

/// Set ambiguous option name.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn set_option_ambiguous() {
    let tmux = TmuxServer::new("setopt_ambig");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let result = tmux.try_run(&["set", "-g", "status-", "val"]);
    assert!(!result.status.success());
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        stderr.contains("ambiguous option"),
        "expected 'ambiguous option' error, got: {stderr}"
    );
}

/// Set with -q (quiet, no error on invalid).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn set_option_quiet() {
    let tmux = TmuxServer::new("setopt_quiet");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let result = tmux.try_run(&["set", "-gq", "nonexistent-option-xyz", "val"]);
    assert!(result.status.success(), "-q should suppress error");
}

/// Set with -o (only if not already set).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn set_option_only_if_unset() {
    let tmux = TmuxServer::new("setopt_o");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["set", "@myvar", "first"]);
    let result = tmux.try_run(&["set", "-o", "@myvar", "second"]);
    assert!(!result.status.success());
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        stderr.contains("already set"),
        "expected 'already set' error, got: {stderr}"
    );

    // Value should remain "first"
    let out = tmux.run(&["show", "-v", "@myvar"]);
    assert_eq!(out.trim(), "first");
}

/// Set with -oq (quiet, already set).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn set_option_only_if_unset_quiet() {
    let tmux = TmuxServer::new("setopt_oq");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["set", "@myvar", "first"]);
    let result = tmux.try_run(&["set", "-oq", "@myvar", "second"]);
    assert!(result.status.success(), "-oq should suppress already-set error");
}

/// Set a user option (@-prefixed).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn set_option_user() {
    let tmux = TmuxServer::new("setopt_user");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["set", "-g", "@custom-var", "custom-value"]);
    let out = tmux.run(&["show", "-gv", "@custom-var"]);
    assert_eq!(out.trim(), "custom-value");
}

/// Set with -a (append).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn set_option_append() {
    let tmux = TmuxServer::new("setopt_append");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["set", "-g", "@myvar", "hello"]);
    tmux.run(&["set", "-ga", "@myvar", " world"]);
    let out = tmux.run(&["show", "-gv", "@myvar"]);
    assert_eq!(out.trim(), "hello world");
}

/// Set user option without value (error).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn set_option_user_no_value() {
    let tmux = TmuxServer::new("setopt_noval");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let result = tmux.try_run(&["set", "-g", "@myvar"]);
    assert!(!result.status.success());
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        stderr.contains("empty value"),
        "expected 'empty value' error, got: {stderr}"
    );
}

/// Set a hook with set-hook.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn set_hook_basic() {
    let tmux = TmuxServer::new("sethook_basic");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["set-hook", "-g", "after-new-session", "display 'hello'"]);
    let out = tmux.run(&["show-hooks", "-g"]);
    assert!(out.contains("after-new-session"), "expected hook to be set");
}

/// Set hook with -R (run immediately).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn set_hook_run() {
    let tmux = TmuxServer::new("sethook_run");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // -R fires the hook immediately
    tmux.run(&["set-hook", "-R", "after-new-session"]);
}

/// Set window option using setw alias.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn set_window_option_alias() {
    let tmux = TmuxServer::new("setw_alias");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["setw", "-g", "mode-keys", "vi"]);
    let out = tmux.run(&["showw", "-gv", "mode-keys"]);
    assert_eq!(out.trim(), "vi");
}

/// Set pane option with -p.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn set_option_pane() {
    let tmux = TmuxServer::new("setopt_pane");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["set", "-p", "remain-on-exit", "on"]);
    let out = tmux.run(&["show", "-pv", "remain-on-exit"]);
    assert_eq!(out.trim(), "on");
}

/// Unset with -U (unset from all panes in window).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn set_option_unset_panes() {
    let tmux = TmuxServer::new("setopt_U");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Set a pane option, then unset it with -U (which removes from all panes)
    tmux.run(&["set", "-p", "remain-on-exit", "on"]);
    tmux.run(&["set", "-wU", "remain-on-exit"]);
}

/// Set array option with index.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn set_option_array_index() {
    let tmux = TmuxServer::new("setopt_arr_idx");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["set", "-s", "terminal-features[99]", "xterm-256color:clipboard"]);
    let out = tmux.run(&["show", "-s", "terminal-features[99]"]);
    assert!(out.contains("clipboard"), "expected array item in output");
}

/// Set not-an-array option with index (error).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn set_option_not_array_with_index() {
    let tmux = TmuxServer::new("setopt_notarr");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let result = tmux.try_run(&["set", "-g", "status[0]", "on"]);
    assert!(!result.status.success());
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        stderr.contains("not an array"),
        "expected 'not an array' error, got: {stderr}"
    );
}

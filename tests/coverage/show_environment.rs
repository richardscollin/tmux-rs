use super::*;

/// Show global environment.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn show_environment_global() {
    let tmux = TmuxServer::new("showenv_global");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["setenv", "-g", "MY_TEST_VAR", "hello"]);
    let out = tmux.run(&["showenv", "-g", "MY_TEST_VAR"]);
    assert_eq!(out.trim(), "MY_TEST_VAR=hello");
}

/// Show session environment.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn show_environment_session() {
    let tmux = TmuxServer::new("showenv_session");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["setenv", "SESS_VAR", "world"]);
    let out = tmux.run(&["showenv", "SESS_VAR"]);
    assert_eq!(out.trim(), "SESS_VAR=world");
}

/// Show environment with -s (shell format).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn show_environment_shell_format() {
    let tmux = TmuxServer::new("showenv_shell");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["setenv", "-g", "SHELL_VAR", "test_val"]);
    let out = tmux.run(&["showenv", "-g", "-s", "SHELL_VAR"]);
    assert!(out.contains("SHELL_VAR=") || out.contains("export"));
}

/// Show all global environment variables.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn show_environment_all() {
    let tmux = TmuxServer::new("showenv_all");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["setenv", "-g", "VAR_A", "1"]);
    tmux.run(&["setenv", "-g", "VAR_B", "2"]);

    let out = tmux.run(&["showenv", "-g"]);
    assert!(out.contains("VAR_A=1"));
    assert!(out.contains("VAR_B=2"));
}

/// Show unknown variable (error).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn show_environment_unknown() {
    let tmux = TmuxServer::new("showenv_unknown");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let result = tmux.try_run(&["showenv", "-g", "NO_SUCH_VAR_EVER"]);
    assert!(!result.status.success());
}

/// Unset environment variable with -u.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn show_environment_unset() {
    let tmux = TmuxServer::new("showenv_unset");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["setenv", "-g", "RM_VAR", "val"]);
    tmux.run(&["setenv", "-g", "-u", "RM_VAR"]);

    // After unset, the var should be gone or marked for removal
    let result = tmux.try_run(&["showenv", "-g", "RM_VAR"]);
    // Either shows -RM_VAR or returns error for unknown var
    let stdout = String::from_utf8_lossy(&result.stdout);
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        stdout.contains("-RM_VAR") || stderr.contains("unknown"),
        "should show removal or error, got stdout: {stdout}, stderr: {stderr}"
    );
}

/// Show environment with -h (hidden only).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn show_environment_hidden() {
    let tmux = TmuxServer::new("showenv_hidden");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["setenv", "-g", "-h", "HIDDEN_VAR", "secret"]);
    let out = tmux.run(&["showenv", "-g", "-h"]);
    assert!(
        out.contains("HIDDEN_VAR"),
        "hidden var should appear with -h"
    );
}

/// Show a specific hidden variable by name with -h.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn show_environment_hidden_specific_var() {
    let tmux = TmuxServer::new("showenv_hidden_sp");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["setenv", "-g", "-h", "HIDE_ME", "secret_val"]);
    let out = tmux.run(&["showenv", "-g", "-h", "HIDE_ME"]);
    assert!(
        out.contains("HIDE_ME=secret_val"),
        "should show hidden var value, got: {out}"
    );
}

/// Hidden var should NOT appear without -h flag.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn show_environment_hidden_excluded_without_flag() {
    let tmux = TmuxServer::new("showenv_hidden_ex");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["setenv", "-g", "-h", "INVISIBLE", "gone"]);
    tmux.run(&["setenv", "-g", "VISIBLE", "here"]);
    let out = tmux.run(&["showenv", "-g"]);
    assert!(
        !out.contains("INVISIBLE"),
        "hidden var should NOT appear without -h"
    );
    assert!(out.contains("VISIBLE=here"), "normal var should appear");
}

/// Non-hidden var should NOT appear with -h flag.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn show_environment_nonhidden_excluded_with_h() {
    let tmux = TmuxServer::new("showenv_nonhid_ex");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["setenv", "-g", "NORMAL_VAR", "val"]);
    tmux.run(&["setenv", "-g", "-h", "HIDDEN_X", "secret"]);
    let out = tmux.run(&["showenv", "-g", "-h"]);
    assert!(
        !out.contains("NORMAL_VAR"),
        "non-hidden var should NOT appear with -h"
    );
    assert!(out.contains("HIDDEN_X"), "hidden var should appear with -h");
}

/// Show removed variable in shell format should output "unset VAR;".
/// Use setenv -r to mark a variable for removal (sets value to None)
/// rather than -u which completely removes the entry.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn show_environment_shell_format_unset() {
    let tmux = TmuxServer::new("showenv_s_unset");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["setenv", "-g", "GONE_VAR", "val"]);
    tmux.run(&["setenv", "-g", "-r", "GONE_VAR"]);
    let out = tmux.run(&["showenv", "-g", "-s", "GONE_VAR"]);
    assert!(
        out.contains("unset GONE_VAR;"),
        "removed var in shell format should show 'unset VAR;', got: {out}"
    );
}

/// Shell format should escape special characters ($, backtick, ", \).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn show_environment_shell_format_escape() {
    let tmux = TmuxServer::new("showenv_s_esc");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Value contains special chars that should be escaped
    tmux.run(&["setenv", "-g", "ESC_VAR", "a$b"]);
    let out = tmux.run(&["showenv", "-g", "-s", "ESC_VAR"]);
    assert!(
        out.contains("ESC_VAR=\"a\\$b\"; export ESC_VAR;"),
        "dollar sign should be escaped, got: {out}"
    );
}

/// Show all session environment variables (without -g).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn show_environment_all_session() {
    let tmux = TmuxServer::new("showenv_all_sess");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["setenv", "SVAR1", "one"]);
    tmux.run(&["setenv", "SVAR2", "two"]);
    let out = tmux.run(&["showenv"]);
    assert!(out.contains("SVAR1=one"), "should list session var SVAR1");
    assert!(out.contains("SVAR2=two"), "should list session var SVAR2");
}

/// Removed variable without -s shows "-VARNAME" format.
/// Use setenv -r to mark for removal (keeps entry with None value)
/// rather than -u which removes the entry entirely.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn show_environment_removed_var_format() {
    let tmux = TmuxServer::new("showenv_rm_fmt");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["setenv", "-g", "DEL_VAR", "val"]);
    tmux.run(&["setenv", "-g", "-r", "DEL_VAR"]);
    let out = tmux.run(&["showenv", "-g", "DEL_VAR"]);
    assert_eq!(out.trim(), "-DEL_VAR", "removed var should show '-VARNAME'");
}

/// Shell format with valid value should show export syntax.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn show_environment_shell_format_export() {
    let tmux = TmuxServer::new("showenv_s_export");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["setenv", "-g", "EXP_VAR", "myval"]);
    let out = tmux.run(&["showenv", "-g", "-s", "EXP_VAR"]);
    assert_eq!(
        out.trim(),
        "EXP_VAR=\"myval\"; export EXP_VAR;",
        "shell format should export, got: {out}"
    );
}

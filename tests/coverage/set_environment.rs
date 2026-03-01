use super::*;

/// Set a global environment variable.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn set_environment_global() {
    let tmux = TmuxServer::new("setenv_global");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["setenv", "-g", "MY_VAR", "hello"]);
    let out = tmux.run(&["showenv", "-g", "MY_VAR"]);
    assert_eq!(out.trim(), "MY_VAR=hello");
}

/// Set a session environment variable.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn set_environment_session() {
    let tmux = TmuxServer::new("setenv_session");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["setenv", "SESS_VAR", "world"]);
    let out = tmux.run(&["showenv", "SESS_VAR"]);
    assert_eq!(out.trim(), "SESS_VAR=world");
}

/// Set with -h (hidden variable).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn set_environment_hidden() {
    let tmux = TmuxServer::new("setenv_hidden");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["setenv", "-g", "-h", "SECRET", "val"]);
    let out = tmux.run(&["showenv", "-g", "-h", "SECRET"]);
    assert!(out.contains("SECRET=val"));
}

/// Unset with -u.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn set_environment_unset() {
    let tmux = TmuxServer::new("setenv_unset");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["setenv", "-g", "RM_VAR", "val"]);
    tmux.run(&["setenv", "-g", "-u", "RM_VAR"]);
    let result = tmux.try_run(&["showenv", "-g", "RM_VAR"]);
    assert!(!result.status.success(), "var should be gone after -u");
}

/// Unset with value is an error.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn set_environment_unset_with_value() {
    let tmux = TmuxServer::new("setenv_u_val");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let result = tmux.try_run(&["setenv", "-g", "-u", "X", "badval"]);
    assert!(!result.status.success());
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        stderr.contains("can't specify a value with -u"),
        "expected error about value with -u, got: {stderr}"
    );
}

/// Remove with -r (mark for removal, keep entry).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn set_environment_remove() {
    let tmux = TmuxServer::new("setenv_remove");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["setenv", "-g", "CLR_VAR", "val"]);
    tmux.run(&["setenv", "-g", "-r", "CLR_VAR"]);
    let out = tmux.run(&["showenv", "-g", "CLR_VAR"]);
    assert_eq!(out.trim(), "-CLR_VAR");
}

/// Remove with value is an error.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn set_environment_remove_with_value() {
    let tmux = TmuxServer::new("setenv_r_val");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let result = tmux.try_run(&["setenv", "-g", "-r", "X", "badval"]);
    assert!(!result.status.success());
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        stderr.contains("can't specify a value with -r"),
        "expected error about value with -r, got: {stderr}"
    );
}

/// No value specified (set without value or -u/-r).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn set_environment_no_value() {
    let tmux = TmuxServer::new("setenv_noval");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let result = tmux.try_run(&["setenv", "-g", "NOVALUE"]);
    assert!(!result.status.success());
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        stderr.contains("no value specified"),
        "expected 'no value specified' error, got: {stderr}"
    );
}

/// Empty variable name.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn set_environment_empty_name() {
    let tmux = TmuxServer::new("setenv_empty");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let result = tmux.try_run(&["setenv", "-g", "", "val"]);
    assert!(!result.status.success());
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        stderr.contains("empty variable name"),
        "expected 'empty variable name' error, got: {stderr}"
    );
}

/// Variable name with = is an error.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn set_environment_name_with_equals() {
    let tmux = TmuxServer::new("setenv_equals");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let result = tmux.try_run(&["setenv", "-g", "FOO=BAR", "val"]);
    assert!(!result.status.success());
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        stderr.contains("variable name contains ="),
        "expected 'variable name contains =' error, got: {stderr}"
    );
}

/// Set with -F (format expansion).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn set_environment_format() {
    let tmux = TmuxServer::new("setenv_format");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["setenv", "-g", "-F", "FMT_VAR", "#{session_name}"]);
    let out = tmux.run(&["showenv", "-g", "FMT_VAR"]);
    // The format should have been expanded (not literal #{session_name})
    assert!(!out.contains("#{session_name}"), "format should be expanded, got: {out}");
}

/// Target nonexistent session.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn set_environment_bad_session() {
    let tmux = TmuxServer::new("setenv_badsess");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let result = tmux.try_run(&["setenv", "-t", "nosuchsession", "X", "val"]);
    assert!(!result.status.success());
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        stderr.contains("no such session") || stderr.contains("can't find session"),
        "expected session error, got: {stderr}"
    );
}

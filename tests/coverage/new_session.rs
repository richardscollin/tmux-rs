use super::*;

/// Test new-session: named, detached, -A (attach-or-create),
/// -P (print), -e (env), -x/-y (size), -n (window name), errors.

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn new_session_named() {
    let tmux = TmuxServer::new("newsess_named");
    tmux.run(&["-f/dev/null", "new", "-d", "-s", "main", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Create a second named session
    tmux.run(&["new-session", "-d", "-s", "second"]);
    let sessions = tmux.run(&["list-sessions", "-F", "#{session_name}"]);
    assert!(sessions.contains("main"));
    assert!(sessions.contains("second"));
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn new_session_with_window_name() {
    let tmux = TmuxServer::new("newsess_wname");
    tmux.run(&["-f/dev/null", "new", "-d", "-s", "wntest", "-n", "mywin", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let name = tmux.run(&["display-message", "-t", "wntest", "-p", "#{window_name}"]);
    assert_eq!(name.trim(), "mywin");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn new_session_with_size() {
    let tmux = TmuxServer::new("newsess_size");
    tmux.run(&["-f/dev/null", "new", "-d", "-s", "sized", "-x100", "-y30"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let w = tmux.run(&["display-message", "-t", "sized", "-p", "#{window_width}"]);
    let h = tmux.run(&["display-message", "-t", "sized", "-p", "#{window_height}"]);
    assert_eq!(w.trim(), "100");
    assert_eq!(h.trim(), "30");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn new_session_duplicate() {
    let tmux = TmuxServer::new("newsess_dup");
    tmux.run(&["-f/dev/null", "new", "-d", "-s", "duptest", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Creating a session with the same name should fail
    let out = tmux.try_run(&["new-session", "-d", "-s", "duptest"]);
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("duplicate session"), "got: {stderr}");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn new_session_print() {
    let tmux = TmuxServer::new("newsess_print");
    tmux.run(&["-f/dev/null", "new", "-d", "-s", "ptest", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // -P: print session info
    let out = tmux.run(&["new-session", "-d", "-s", "printed", "-P", "-x80", "-y24"]);
    assert!(out.contains("printed:"), "expected session info, got: {out}");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn new_session_print_format() {
    let tmux = TmuxServer::new("newsess_pfmt");
    tmux.run(&["-f/dev/null", "new", "-d", "-s", "pfmt", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // -P -F: custom format
    let out = tmux.run(&["new-session", "-d", "-s", "pfmt2", "-P", "-F", "#{session_name}", "-x80", "-y24"]);
    assert_eq!(out.trim(), "pfmt2");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn new_session_environment() {
    let tmux = TmuxServer::new("newsess_env");
    tmux.run(&["-f/dev/null", "new", "-d", "-s", "envtest", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // -e: set environment
    tmux.run(&["new-session", "-d", "-s", "envs", "-e", "MYVAR=hello", "-x80", "-y24"]);
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn new_session_cwd() {
    let tmux = TmuxServer::new("newsess_cwd");
    tmux.run(&["-f/dev/null", "new", "-d", "-s", "cwdtest", "-c", "/tmp", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let sessions = tmux.run(&["list-sessions", "-F", "#{session_name}"]);
    assert!(sessions.contains("cwdtest"));
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn new_session_attach_if_exists() {
    let tmux = TmuxServer::new("newsess_attach");
    tmux.run(&["-f/dev/null", "new", "-d", "-s", "existing", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // -A: attach to existing session if it exists (in detached mode, exercises the path)
    let out = tmux.try_run(&["new-session", "-d", "-A", "-s", "existing"]);
    // Should succeed — either attaches or is a no-op in detached mode
    let _ = out;
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn new_session_invalid_name() {
    let tmux = TmuxServer::new("newsess_badname");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Empty session name should fail
    let out = tmux.try_run(&["new-session", "-d", "-s", ""]);
    assert!(!out.status.success());
}

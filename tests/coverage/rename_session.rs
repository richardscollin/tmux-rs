use super::*;

/// Test rename-session: basic rename, invalid name, duplicate name.

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn rename_session_basic() {
    let tmux = TmuxServer::new("renamesess_basic");
    tmux.run(&["-f/dev/null", "new", "-d", "-s", "old", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["rename-session", "-t", "old", "new"]);
    let sessions = tmux.run(&["list-sessions", "-F", "#{session_name}"]);
    assert!(sessions.contains("new"));
    assert!(!sessions.contains("old"));
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn rename_session_invalid() {
    let tmux = TmuxServer::new("renamesess_inv");
    tmux.run(&["-f/dev/null", "new", "-d", "-s", "valid", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Rename to empty name — should be invalid
    let out = tmux.try_run(&["rename-session", ""]);
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("invalid session") || stderr.contains("bad session"),
        "got: {stderr}"
    );
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn rename_session_duplicate() {
    let tmux = TmuxServer::new("renamesess_dup");
    tmux.run(&["-f/dev/null", "new", "-d", "-s", "first", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["new-session", "-d", "-s", "second"]);

    // Rename to existing session name
    let out = tmux.try_run(&["rename-session", "-t", "first", "second"]);
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("duplicate session"), "got: {stderr}");
}

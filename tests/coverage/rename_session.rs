use super::*;

/// Basic rename-session.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn rename_session_basic() {
    let tmux = TmuxServer::new("rename_basic");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24", "-s", "orig"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["rename-session", "-t", "orig", "newname"]);

    let name = tmux.display("#{session_name}");
    assert_eq!(name.trim(), "newname");
}

/// Rename to the same name (noop).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn rename_session_same_name() {
    let tmux = TmuxServer::new("rename_same");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24", "-s", "same"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Renaming to the same name should succeed (noop)
    tmux.run(&["rename-session", "same"]);

    let name = tmux.display("#{session_name}");
    assert_eq!(name.trim(), "same");
}

/// Rename to a duplicate session name (error).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn rename_session_duplicate() {
    let tmux = TmuxServer::new("rename_dup");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24", "-s", "sess1"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["new", "-d", "-s", "sess2"]);

    let result = tmux.try_run(&["rename-session", "-t", "sess1", "sess2"]);
    assert!(!result.status.success());
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        stderr.contains("duplicate session"),
        "expected 'duplicate session' error, got: {stderr}"
    );
}

/// Rename with empty name (error).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn rename_session_invalid_name() {
    let tmux = TmuxServer::new("rename_invalid");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Empty name is invalid
    let result = tmux.try_run(&["rename-session", ""]);
    assert!(!result.status.success());
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        stderr.contains("invalid session"),
        "expected 'invalid session' error, got: {stderr}"
    );
}

/// Rename using alias 'rename'.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn rename_session_alias() {
    let tmux = TmuxServer::new("rename_alias");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24", "-s", "before"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["rename", "-t", "before", "after"]);

    let name = tmux.display("#{session_name}");
    assert_eq!(name.trim(), "after");
}

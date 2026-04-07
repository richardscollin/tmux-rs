use super::*;

/// List server access (should work even with no explicit ACL entries).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn server_access_list() {
    let tmux = TmuxServer::new("svraccess_list");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let out = tmux.run(&["server-access", "-l"]);
    // Should list the current user at minimum
    let _ = out;
}

/// Server-access -a and -d conflict.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn server_access_add_deny_conflict() {
    let tmux = TmuxServer::new("svraccess_conflict");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let result = tmux.try_run(&["server-access", "-a", "-d", "nobody"]);
    assert!(!result.status.success());
}

/// Server-access -r and -w conflict.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn server_access_rw_conflict() {
    let tmux = TmuxServer::new("svraccess_rw_conflict");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let result = tmux.try_run(&["server-access", "-a", "-r", "-w", "nobody"]);
    assert!(!result.status.success());
}

/// Server-access add unknown user.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn server_access_unknown_user() {
    let tmux = TmuxServer::new("svraccess_unknown");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let result = tmux.try_run(&["server-access", "-a", "nonexistent_user_xyz123"]);
    assert!(!result.status.success());
}

/// Server-access deny owner (error).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn server_access_deny_owner() {
    let tmux = TmuxServer::new("svraccess_deny_owner");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Try to deny the current user (who is the server owner)
    let result = tmux.try_run(&["server-access", "-d", &whoami()]);
    assert!(!result.status.success());
}

fn whoami() -> String {
    let out = std::process::Command::new("whoami")
        .output()
        .expect("whoami failed");
    String::from_utf8(out.stdout).unwrap().trim().to_string()
}

/// Server-access missing user argument.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn server_access_missing_user() {
    let tmux = TmuxServer::new("svraccess_nouser");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let result = tmux.try_run(&["server-access"]);
    assert!(!result.status.success());
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        stderr.contains("missing user"),
        "expected 'missing user' error, got: {stderr}"
    );
}

/// Server-access add a user with -a.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn server_access_add_user() {
    let tmux = TmuxServer::new("svraccess_add");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["server-access", "-a", "nobody"]);
    let out = tmux.run(&["server-access", "-l"]);
    assert!(
        out.contains("nobody"),
        "nobody should be in ACL list, got: {out}"
    );
}

/// Server-access add duplicate user (error).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn server_access_add_duplicate() {
    let tmux = TmuxServer::new("svraccess_dup");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["server-access", "-a", "nobody"]);
    let result = tmux.try_run(&["server-access", "-a", "nobody"]);
    assert!(!result.status.success());
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        stderr.contains("already added"),
        "expected 'already added' error, got: {stderr}"
    );
}

/// Server-access deny a user with -d.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn server_access_deny_user() {
    let tmux = TmuxServer::new("svraccess_deny");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["server-access", "-a", "nobody"]);
    tmux.run(&["server-access", "-d", "nobody"]);
}

/// Server-access deny non-added user (error).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn server_access_deny_not_found() {
    let tmux = TmuxServer::new("svraccess_deny_nf");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let result = tmux.try_run(&["server-access", "-d", "nobody"]);
    assert!(!result.status.success());
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        stderr.contains("not found"),
        "expected 'not found' error, got: {stderr}"
    );
}

/// Server-access grant write with -w.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn server_access_write() {
    let tmux = TmuxServer::new("svraccess_write");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["server-access", "-a", "nobody"]);
    tmux.run(&["server-access", "-w", "nobody"]);
}

/// Server-access -w implies -a for unknown user.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn server_access_write_implies_add() {
    let tmux = TmuxServer::new("svraccess_w_add");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // -w on non-added user should implicitly add them
    tmux.run(&["server-access", "-w", "nobody"]);
    let out = tmux.run(&["server-access", "-l"]);
    assert!(out.contains("nobody"), "nobody should be added via -w");
}

/// Server-access deny write with -r.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn server_access_read_only() {
    let tmux = TmuxServer::new("svraccess_read");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["server-access", "-a", "nobody"]);
    tmux.run(&["server-access", "-r", "nobody"]);
}

/// Server-access -r implies -a for unknown user.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn server_access_read_implies_add() {
    let tmux = TmuxServer::new("svraccess_r_add");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["server-access", "-r", "nobody"]);
    let out = tmux.run(&["server-access", "-l"]);
    assert!(out.contains("nobody"), "nobody should be added via -r");
}

/// Server-access add with -a and -w together.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn server_access_add_write() {
    let tmux = TmuxServer::new("svraccess_aw");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["server-access", "-a", "-w", "nobody"]);
}

/// Server-access add with -a and -r together.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn server_access_add_read() {
    let tmux = TmuxServer::new("svraccess_ar");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["server-access", "-a", "-r", "nobody"]);
}

/// Server-access with empty username.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn server_access_empty_name() {
    let tmux = TmuxServer::new("svraccess_empty");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let result = tmux.try_run(&["server-access", "-a", ""]);
    assert!(!result.status.success());
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        stderr.contains("unknown user"),
        "expected 'unknown user' error, got: {stderr}"
    );
}

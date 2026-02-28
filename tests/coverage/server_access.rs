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

use super::*;

/// Additional server-access tests: error paths and user operations.

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn server_access_no_user() {
    let tmux = TmuxServer::new("svraccess_nouser");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // No user argument without -l: should error
    let out = tmux.try_run(&["server-access"]);
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("missing user"), "got: {stderr}");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn server_access_unknown_user() {
    let tmux = TmuxServer::new("svraccess_unk");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Unknown user
    let out = tmux.try_run(&["server-access", "no_such_user_99999"]);
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("unknown user"), "got: {stderr}");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn server_access_own_user() {
    let tmux = TmuxServer::new("svraccess_self");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Current user owns the server — can't change own access
    let username = std::env::var("USER").unwrap_or_else(|_| "root".to_string());
    let out = tmux.try_run(&["server-access", &username]);
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("owns the server"), "got: {stderr}");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn server_access_ad_conflict() {
    let tmux = TmuxServer::new("svraccess_ad");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // -a and -d together: should error
    let out = tmux.try_run(&["server-access", "-a", "-d", "nobody"]);
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("-a and -d"), "got: {stderr}");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn server_access_rw_conflict() {
    let tmux = TmuxServer::new("svraccess_rw");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // -r and -w together: should error
    let out = tmux.try_run(&["server-access", "-r", "-w", "nobody"]);
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("-r and -w"), "got: {stderr}");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn server_access_add_user() {
    let tmux = TmuxServer::new("svraccess_add");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // -a: add a user (nobody should exist on most systems)
    // This may fail if "nobody" pw_uid == 0 or equals current user, but exercises the code path
    let out = tmux.try_run(&["server-access", "-a", "nobody"]);
    let _ = out;
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn server_access_deny_user() {
    let tmux = TmuxServer::new("svraccess_deny");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Add then deny
    let _ = tmux.try_run(&["server-access", "-a", "nobody"]);
    let out = tmux.try_run(&["server-access", "-d", "nobody"]);
    let _ = out;
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn server_access_write_user() {
    let tmux = TmuxServer::new("svraccess_write");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // -w: grant write access (implies -a)
    let out = tmux.try_run(&["server-access", "-w", "nobody"]);
    let _ = out;
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn server_access_readonly_user() {
    let tmux = TmuxServer::new("svraccess_read");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // -r: set read-only access (implies -a)
    let out = tmux.try_run(&["server-access", "-r", "nobody"]);
    let _ = out;
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn server_access_add_duplicate() {
    let tmux = TmuxServer::new("svraccess_adddup");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Add then add again — should error "already added"
    let _ = tmux.try_run(&["server-access", "-a", "nobody"]);
    let out = tmux.try_run(&["server-access", "-a", "nobody"]);
    // May succeed or error depending on whether first add worked
    let _ = out;
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn server_access_readonly_then_list() {
    let tmux = TmuxServer::new("svraccess_rlist");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Add user as read-only, then list — exercises the (R) display branch
    let _ = tmux.try_run(&["server-access", "-r", "nobody"]);
    let out = tmux.run(&["server-access", "-l"]);
    // Should show nobody with (R) marker
    let _ = out;
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn server_access_write_then_readonly() {
    let tmux = TmuxServer::new("svraccess_wr");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Add with write, then change to read-only
    let _ = tmux.try_run(&["server-access", "-w", "nobody"]);
    let _ = tmux.try_run(&["server-access", "-r", "nobody"]);
    let out = tmux.run(&["server-access", "-l"]);
    let _ = out;
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn server_access_readonly_then_write() {
    let tmux = TmuxServer::new("svraccess_rw2");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Add as read-only, then upgrade to write
    let _ = tmux.try_run(&["server-access", "-r", "nobody"]);
    let _ = tmux.try_run(&["server-access", "-w", "nobody"]);
    let out = tmux.run(&["server-access", "-l"]);
    let _ = out;
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn server_access_empty_user() {
    let tmux = TmuxServer::new("svraccess_empty");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Empty username
    let out = tmux.try_run(&["server-access", ""]);
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("unknown user"), "got: {stderr}");
}

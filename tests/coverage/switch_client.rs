use super::*;

/// Test switch-client: -n (next session), -p (previous session), -l (last session),
/// -r (toggle readonly), -T (key table), and default (switch to target).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn switch_client_next_prev() {
    let tmux = TmuxServer::new("switchc_np");
    tmux.run(&["-f/dev/null", "new", "-d", "-s", "a", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["new-session", "-d", "-s", "b"]);
    tmux.run(&["new-session", "-d", "-s", "c"]);

    // switch-client -n: next session
    let out = tmux.try_run(&["switch-client", "-n", "-t", "a"]);
    // This may succeed or fail depending on client attachment, but exercises the code
    let _ = out;

    // switch-client -p: previous session
    let out = tmux.try_run(&["switch-client", "-p", "-t", "a"]);
    let _ = out;
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn switch_client_last() {
    let tmux = TmuxServer::new("switchc_last");
    tmux.run(&["-f/dev/null", "new", "-d", "-s", "s1", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["new-session", "-d", "-s", "s2"]);

    // switch-client -l: last session (may fail with no last session)
    let out = tmux.try_run(&["switch-client", "-l", "-t", "s1"]);
    let _ = out;
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn switch_client_key_table() {
    let tmux = TmuxServer::new("switchc_table");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // switch-client -T with valid table (prefix always exists)
    let out = tmux.try_run(&["switch-client", "-T", "prefix"]);
    let _ = out;

    // switch-client -T with nonexistent table: should error
    // Note: may error with "no current client" in detached mode, or "table doesn't exist"
    let out = tmux.try_run(&["switch-client", "-T", "nonexistent_table"]);
    assert!(!out.status.success());
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn switch_client_readonly() {
    let tmux = TmuxServer::new("switchc_ro");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // switch-client -r: toggle readonly (exercises both toggle directions)
    let out = tmux.try_run(&["switch-client", "-r"]);
    let _ = out;
    let out = tmux.try_run(&["switch-client", "-r"]);
    let _ = out;
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn switch_client_target_session() {
    let tmux = TmuxServer::new("switchc_target");
    tmux.run(&["-f/dev/null", "new", "-d", "-s", "orig", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["new-session", "-d", "-s", "dest"]);

    // switch-client -t dest: switch to target session
    let out = tmux.try_run(&["switch-client", "-t", "dest"]);
    let _ = out;
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn switch_client_target_pane() {
    let tmux = TmuxServer::new("switchc_pane");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["split-window", "-d"]);

    // switch-client -t :.1 : target includes pane specifier (exercises CMD_FIND_PANE path)
    let out = tmux.try_run(&["switch-client", "-t", ":.1"]);
    let _ = out;
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn switch_client_no_env_update() {
    let tmux = TmuxServer::new("switchc_noenv");
    tmux.run(&["-f/dev/null", "new", "-d", "-s", "envtest", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // switch-client -E: do not update environment
    let out = tmux.try_run(&["switch-client", "-E", "-t", "envtest"]);
    let _ = out;
}

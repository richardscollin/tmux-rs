use super::*;

/// Basic attach via control mode.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn attach_session_basic() {
    let tmux = TmuxServer::new("attach_basic");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);

    let output = tmux.run_with_stdin(&["-C", "attach"], b"detach-client\n");
    assert!(output.status.success());

    let has = tmux.try_run(&["has-session"]);
    assert!(has.status.success());
}

/// Attach with -d (detach other clients).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn attach_session_detach_others() {
    let tmux = TmuxServer::new("attach_d");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);

    let output = tmux.run_with_stdin(&["-C", "attach", "-d"], b"detach-client\n");
    assert!(output.status.success());
}

/// Attach with -r (read-only).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn attach_session_readonly() {
    let tmux = TmuxServer::new("attach_r");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);

    let output = tmux.run_with_stdin(&["-C", "attach", "-r"], b"detach-client\n");
    assert!(output.status.success());
}

/// Attach with -t (target session).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn attach_session_target() {
    let tmux = TmuxServer::new("attach_t");
    tmux.run(&["-f/dev/null", "new", "-d", "-s", "target1", "-x80", "-y24"]);
    tmux.run(&["new", "-d", "-s", "target2"]);

    let output = tmux.run_with_stdin(&["-C", "attach", "-t", "target2"], b"detach-client\n");
    assert!(output.status.success());
}

/// Attach with -x (detach and kill other clients).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn attach_session_detach_kill() {
    let tmux = TmuxServer::new("attach_x");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);

    let output = tmux.run_with_stdin(&["-C", "attach", "-x"], b"detach-client\n");
    assert!(output.status.success());
}

/// Attach with -c (set session working directory).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn attach_session_directory() {
    let tmux = TmuxServer::new("attach_c");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);

    let output = tmux.run_with_stdin(&["-C", "attach", "-c", "/tmp"], b"detach-client\n");
    assert!(output.status.success());
}

/// Attach with no sessions (error).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn attach_session_no_sessions() {
    let tmux = TmuxServer::new("attach_nosess");
    // Don't create any sessions - just try to attach (non-control mode)
    let result = tmux.try_run(&["attach"]);
    assert!(!result.status.success());
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        stderr.contains("no session") || stderr.contains("can't find"),
        "expected no-sessions error, got: {stderr}"
    );
}

/// Attach with -E (don't update environment).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn attach_session_no_env() {
    let tmux = TmuxServer::new("attach_E");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);

    let output = tmux.run_with_stdin(&["-C", "attach", "-E"], b"detach-client\n");
    assert!(output.status.success());
}

/// Attach with -t targeting a specific pane.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn attach_session_target_pane() {
    let tmux = TmuxServer::new("attach_t_pane");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["split-window", "-d"]);

    let output = tmux.run_with_stdin(&["-C", "attach", "-t", ":.1"], b"detach-client\n");
    assert!(output.status.success());
}

/// Attach using 'attach' alias.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn attach_session_alias() {
    let tmux = TmuxServer::new("attach_alias");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);

    let output = tmux.run_with_stdin(&["-C", "attach-session"], b"detach-client\n");
    assert!(output.status.success());
}

/// Attach with -d -x (both detach and kill).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn attach_session_detach_and_kill() {
    let tmux = TmuxServer::new("attach_dx");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);

    let output = tmux.run_with_stdin(&["-C", "attach", "-d", "-x"], b"detach-client\n");
    assert!(output.status.success());
}

/// Re-attach when already attached (exercises the already-attached path).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn attach_session_already_attached() {
    let tmux = TmuxServer::new("attach_reattach");
    tmux.run(&["-f/dev/null", "new", "-d", "-s", "sess1", "-x80", "-y24"]);
    tmux.run(&["new", "-d", "-s", "sess2"]);

    // Attach to sess1, then re-attach to sess2 from within control mode
    let cmd = "attach-session -t sess2\ndetach-client\n";
    let output = tmux.run_with_stdin(&["-C", "attach", "-t", "sess1"], cmd.as_bytes());
    assert!(output.status.success());
}

/// Re-attach with -d (detach others from target when already attached).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn attach_session_already_attached_detach() {
    let tmux = TmuxServer::new("attach_reatt_d");
    tmux.run(&["-f/dev/null", "new", "-d", "-s", "sess1", "-x80", "-y24"]);
    tmux.run(&["new", "-d", "-s", "sess2"]);

    let cmd = "attach-session -d -t sess2\ndetach-client\n";
    let output = tmux.run_with_stdin(&["-C", "attach", "-t", "sess1"], cmd.as_bytes());
    assert!(output.status.success());
}

/// Re-attach with -x (kill others from target when already attached).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn attach_session_already_attached_kill() {
    let tmux = TmuxServer::new("attach_reatt_x");
    tmux.run(&["-f/dev/null", "new", "-d", "-s", "sess1", "-x80", "-y24"]);
    tmux.run(&["new", "-d", "-s", "sess2"]);

    let cmd = "attach-session -x -t sess2\ndetach-client\n";
    let output = tmux.run_with_stdin(&["-C", "attach", "-t", "sess1"], cmd.as_bytes());
    assert!(output.status.success());
}

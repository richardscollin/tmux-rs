use super::*;

/// Lock server (no-op without terminal clients).
#[test]
fn lock_server_basic() {
    let tmux = TmuxServer::new("lock_basic");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);

    let result = tmux.try_run(&["lock-server"]);
    let _ = result; // May or may not succeed, just no crash

    let has = tmux.try_run(&["has-session"]);
    assert!(has.status.success());
}

/// Lock session.
#[test]
fn lock_session_basic() {
    let tmux = TmuxServer::new("lock_sess");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);

    let result = tmux.try_run(&["lock-session"]);
    let _ = result;

    let has = tmux.try_run(&["has-session"]);
    assert!(has.status.success());
}

/// Lock client via control mode.
#[test]
fn lock_client_control() {
    let tmux = TmuxServer::new("lock_client");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);

    let output = tmux.run_with_stdin(&["-C", "attach"], b"lock-client\ndetach-client\n");
    let _ = output;

    let has = tmux.try_run(&["has-session"]);
    assert!(has.status.success());
}

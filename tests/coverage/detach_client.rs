use super::*;

/// Detach control-mode client.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn detach_client_basic() {
    let tmux = TmuxServer::new("detach_basic");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);

    let output = tmux.run_with_stdin(&["-C", "attach"], b"detach-client\n");
    assert!(output.status.success());

    let has = tmux.try_run(&["has-session"]);
    assert!(has.status.success());
}

/// Detach with -P (kill).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn detach_client_kill() {
    let tmux = TmuxServer::new("detach_kill");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);

    let output = tmux.run_with_stdin(&["-C", "attach"], b"detach-client -P\n");
    assert!(output.status.success());
}

/// Detach with -a (all others).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn detach_client_all_others() {
    let tmux = TmuxServer::new("detach_all");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);

    let output = tmux.run_with_stdin(&["-C", "attach"], b"detach-client -a\ndetach-client\n");
    assert!(output.status.success());
}

/// Detach with -s (target session).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn detach_client_session() {
    let tmux = TmuxServer::new("detach_sess");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24", "-s", "mysess"]);

    let output = tmux.run_with_stdin(&["-C", "attach"], b"detach-client -s mysess\n");
    assert!(output.status.success());
}

/// Detach with -E (exec command on detach).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn detach_client_exec() {
    let tmux = TmuxServer::new("detach_exec");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);

    let output = tmux.run_with_stdin(&["-C", "attach"], b"detach-client -E /bin/true\n");
    assert!(output.status.success());
}

/// Detach with -a -E (all others, exec command).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn detach_client_all_exec() {
    let tmux = TmuxServer::new("detach_all_exec");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);

    let output = tmux.run_with_stdin(
        &["-C", "attach"],
        b"detach-client -a -E /bin/true\ndetach-client\n",
    );
    assert!(output.status.success());
}

/// Detach with -s -E (session, exec command).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn detach_client_session_exec() {
    let tmux = TmuxServer::new("detach_sess_exec");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24", "-s", "sexec"]);

    let output = tmux.run_with_stdin(&["-C", "attach"], b"detach-client -s sexec -E /bin/true\n");
    assert!(output.status.success());
}

/// Suspend-client command (suspendc alias).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn suspend_client() {
    let tmux = TmuxServer::new("suspendc");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);

    // suspend-client via control mode -- the client will get SIGTSTP
    // which in control mode just causes it to exit/stop
    let output = tmux.run_with_stdin(&["-C", "attach"], b"suspend-client\n");
    // The command should not cause a crash; exit status may vary
    let _ = output;

    // Verify server is still alive
    let has = tmux.try_run(&["has-session"]);
    assert!(has.status.success(), "server should survive suspend-client");
}

/// Suspend-client using alias suspendc.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn suspend_client_alias() {
    let tmux = TmuxServer::new("suspendc_alias");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);

    let output = tmux.run_with_stdin(&["-C", "attach"], b"suspendc\n");
    let _ = output;

    let has = tmux.try_run(&["has-session"]);
    assert!(has.status.success(), "server should survive suspendc alias");
}

/// Detach with -P -a (kill + all others).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn detach_client_kill_all() {
    let tmux = TmuxServer::new("detach_kill_all");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);

    let output = tmux.run_with_stdin(&["-C", "attach"], b"detach-client -P -a\ndetach-client\n");
    assert!(output.status.success());
}

/// Detach using alias 'detach'.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn detach_client_alias() {
    let tmux = TmuxServer::new("detach_alias");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);

    let output = tmux.run_with_stdin(&["-C", "attach"], b"detach\n");
    assert!(output.status.success());

    let has = tmux.try_run(&["has-session"]);
    assert!(has.status.success(), "session should remain after detach");
}

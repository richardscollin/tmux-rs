use super::*;

/// Test that killing a session also kills its child processes
/// (translates kill-session-process-exit.sh)
#[test]
fn kill_session_process_exit() {
    let tmux = TmuxServer::new("kill_session_process_exit");

    // Create a session with a long-running process
    tmux.run(&["-f/dev/null", "new", "-d", "sleep 1000"]);

    // Get the pane PID
    let pid_str = tmux.display("#{pane_pid}");
    let pid = pid_str.trim().to_string();
    assert!(!pid.is_empty(), "pane_pid should not be empty");

    // Create another session so killing the first doesn't shut down the server
    tmux.run(&["-f/dev/null", "new", "-d"]);
    sleep_secs(1);

    // Kill the first session
    tmux.run(&["kill-session", "-t0:"]);
    sleep_secs(3);

    // Verify the process is dead: kill -0 returns non-zero if process is gone
    let status = std::process::Command::new("kill")
        .args(["-0", &pid])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .expect("failed to run kill -0");

    assert!(
        !status.success(),
        "process {} should be dead after kill-session",
        pid
    );
}

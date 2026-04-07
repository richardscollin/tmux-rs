use super::*;

/// wait-for -S (signal a channel with no waiters).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn wait_for_signal_no_waiters() {
    let tmux = TmuxServer::new("wait_sig_none");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);

    // Signal a channel with no waiters -- sets woken flag
    tmux.run(&["wait", "-S", "ch1"]);
    let has = tmux.try_run(&["has-session"]);
    assert!(has.status.success());
}

/// wait-for -S then wait (already woken, returns immediately).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn wait_for_signal_then_wait() {
    let tmux = TmuxServer::new("wait_sig_then");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);

    // Signal first, then wait -- channel already woken, immediate return
    tmux.run(&["wait", "-S", "ch2"]);

    let cmd = "wait-for ch2\ndetach-client\n";
    let output = tmux.run_with_stdin(&["-C", "attach"], cmd.as_bytes());
    assert!(output.status.success());
}

/// wait-for with signal waking a blocked waiter via control mode.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn wait_for_wait_then_signal() {
    let tmux = TmuxServer::new("wait_w_then_s");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);

    // Use run-shell -b to signal after a short delay
    let cmd = "run-shell -b -d1 'tmux -L regress_wait_w_then_s wait -S ch3'\nwait-for ch3\ndetach-client\n";
    let output = tmux.run_with_stdin(&["-C", "attach"], cmd.as_bytes());
    assert!(output.status.success());
}

/// wait-for -L (lock a channel).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn wait_for_lock() {
    let tmux = TmuxServer::new("wait_lock");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);

    // Lock and then unlock
    let cmd = "wait-for -L ch4\nwait-for -U ch4\ndetach-client\n";
    let output = tmux.run_with_stdin(&["-C", "attach"], cmd.as_bytes());
    assert!(output.status.success());
}

/// wait-for -U without prior lock (error).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn wait_for_unlock_without_lock() {
    let tmux = TmuxServer::new("wait_unlock_err");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);

    let result = tmux.try_run(&["wait", "-U", "not_locked"]);
    assert!(!result.status.success());
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        stderr.contains("not locked"),
        "expected 'not locked' error, got: {stderr}"
    );
}

/// wait-for without client context (error: not able to wait).
/// CLI always has a client, so the null-client error path is unreachable from CLI.
#[test]
#[ignore = "untestable: CLI always provides a client context, null-client path unreachable"]
fn wait_for_no_client() {
    let tmux = TmuxServer::new("wait_no_client");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);

    let result = tmux.try_run(&["wait", "some_channel"]);
    assert!(!result.status.success());
}

/// wait-for -L without client context (error: not able to lock).
/// CLI always has a client, so the null-client error path is unreachable from CLI.
#[test]
#[ignore = "untestable: CLI always provides a client context, null-client path unreachable"]
fn wait_for_lock_no_client() {
    let tmux = TmuxServer::new("wait_lock_nocl");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);

    let result = tmux.try_run(&["wait", "-L", "some_channel"]);
    assert!(!result.status.success());
}

/// wait-for -L on already-locked channel (queues the locker).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn wait_for_lock_contention() {
    let tmux = TmuxServer::new("wait_lock_cont");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);

    // Lock, then try to lock again from background (should queue), then unlock
    let binary = TmuxServer::binary_path();
    let cmd = format!(
        concat!(
            "wait-for -L ch5\n",
            "run-shell -b '{binary} -L regress_wait_lock_cont wait-for -L ch5 \\; wait-for -U ch5'\n",
            "wait-for -U ch5\n",
            "detach-client\n",
        ),
        binary = binary,
    );
    let output = tmux.run_with_stdin(&["-C", "attach"], cmd.as_bytes());
    assert!(output.status.success());
    sleep_ms(200);
}

/// wait-for using 'wait' alias.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn wait_for_alias() {
    let tmux = TmuxServer::new("wait_alias");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);

    tmux.run(&["wait-for", "-S", "alias_ch"]);
    let has = tmux.try_run(&["has-session"]);
    assert!(has.status.success());
}

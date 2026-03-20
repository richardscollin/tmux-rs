use super::*;

/// Test wait-for: -S (signal), -U (unlock error). Note: wait-for (no flags)
/// and -L block waiting for a client, so they can't be tested in detached mode.

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn wait_for_signal_no_waiters() {
    let tmux = TmuxServer::new("wait_sig_none");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Signal a channel with no waiters — creates the channel and marks it woken
    tmux.run(&["wait-for", "-S", "mychannel"]);
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn wait_for_signal_twice() {
    let tmux = TmuxServer::new("wait_sig2");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Signal twice — second time the channel is already woken
    tmux.run(&["wait-for", "-S", "ch2"]);
    tmux.run(&["wait-for", "-S", "ch2"]);
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn wait_for_unlock_not_locked() {
    let tmux = TmuxServer::new("wait_unlock_err");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Unlock a channel that is not locked: should error
    let out = tmux.try_run(&["wait-for", "-U", "notlocked"]);
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("not locked"), "got: {stderr}");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn wait_for_signal_multiple_channels() {
    let tmux = TmuxServer::new("wait_sigmulti");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Signal multiple different channels
    tmux.run(&["wait-for", "-S", "ch_a"]);
    tmux.run(&["wait-for", "-S", "ch_b"]);
    tmux.run(&["wait-for", "-S", "ch_c"]);
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn wait_for_unlock_signaled_channel() {
    let tmux = TmuxServer::new("wait_unlock_sig");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Signal creates a woken channel, unlock on it should error (not locked)
    tmux.run(&["wait-for", "-S", "sigchan"]);
    let out = tmux.try_run(&["wait-for", "-U", "sigchan"]);
    assert!(!out.status.success());
}

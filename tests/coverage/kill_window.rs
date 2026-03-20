use super::*;

/// Test kill-window: default (kill target window), -a (kill all others),
/// and unlink-window.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn kill_window_default() {
    let tmux = TmuxServer::new("kill_window_default");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["new-window"]);
    tmux.run(&["new-window"]);

    let count = tmux.display("#{session_windows}");
    assert_eq!(count, "3", "should have 3 windows");

    // kill-window -t :2
    tmux.run(&["kill-window", "-t", ":2"]);
    let count = tmux.display("#{session_windows}");
    assert_eq!(count, "2", "should have 2 windows after killing one");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn kill_window_all_others() {
    let tmux = TmuxServer::new("kill_window_all");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["new-window"]);
    tmux.run(&["new-window"]);

    let count = tmux.display("#{session_windows}");
    assert_eq!(count, "3");

    // kill-window -a: kill all windows except the current one
    tmux.run(&["kill-window", "-a"]);
    let count = tmux.display("#{session_windows}");
    assert_eq!(count, "1", "should have 1 window after kill-window -a");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn kill_window_only_window() {
    let tmux = TmuxServer::new("kill_window_only");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // kill-window -a with only one window: should be a no-op
    tmux.run(&["kill-window", "-a"]);
    let count = tmux.display("#{session_windows}");
    assert_eq!(count, "1", "single window should survive kill-window -a");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn unlink_window() {
    let tmux = TmuxServer::new("unlink_window");
    tmux.run(&["-f/dev/null", "new", "-d", "-s", "sess1", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["new-session", "-d", "-s", "sess2"]);

    // Link sess1's window into sess2
    tmux.run(&["link-window", "-s", "sess1:0", "-t", "sess2:1"]);

    let count2 = tmux.run(&["display-message", "-t", "sess2", "-p", "#{session_windows}"]);
    assert_eq!(count2.trim(), "2", "sess2 should have 2 windows after link");

    // unlink-window: remove the linked window from sess2
    tmux.run(&["unlink-window", "-t", "sess2:1"]);

    let count2 = tmux.run(&["display-message", "-t", "sess2", "-p", "#{session_windows}"]);
    assert_eq!(
        count2.trim(),
        "1",
        "sess2 should have 1 window after unlink"
    );

    // sess1 should still have its window
    let count1 = tmux.run(&["display-message", "-t", "sess1", "-p", "#{session_windows}"]);
    assert_eq!(count1.trim(), "1");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn unlink_window_only_link_no_kill() {
    let tmux = TmuxServer::new("unlink_window_nok");
    tmux.run(&["-f/dev/null", "new", "-d", "-s", "only", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // unlink-window without -k on a window linked to only one session: should error
    let out = tmux.try_run(&["unlink-window", "-t", "only:"]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("only linked to one session"),
        "should error when window only linked to one session, got: {stderr}"
    );
}

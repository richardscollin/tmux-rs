use super::*;

/// Test kill-session: default (kill target session), -a (kill all others), -C (clear alerts).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn kill_session_default() {
    let tmux = TmuxServer::new("kill_session_default");
    tmux.run(&["-f/dev/null", "new", "-d", "-s", "main", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["new-session", "-d", "-s", "extra"]);

    // Verify two sessions exist
    let sessions = tmux.run(&["list-sessions", "-F", "#{session_name}"]);
    assert!(sessions.contains("main"));
    assert!(sessions.contains("extra"));

    // kill-session -t extra
    tmux.run(&["kill-session", "-t", "extra"]);

    let sessions = tmux.run(&["list-sessions", "-F", "#{session_name}"]);
    assert!(sessions.contains("main"));
    assert!(
        !sessions.contains("extra"),
        "extra session should be killed"
    );
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn kill_session_all_others() {
    let tmux = TmuxServer::new("kill_session_all");
    tmux.run(&["-f/dev/null", "new", "-d", "-s", "keep", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["new-session", "-d", "-s", "remove1"]);
    tmux.run(&["new-session", "-d", "-s", "remove2"]);

    // kill-session -a -t keep: kill all except "keep"
    tmux.run(&["kill-session", "-a", "-t", "keep"]);

    let sessions = tmux.run(&["list-sessions", "-F", "#{session_name}"]);
    assert!(sessions.contains("keep"));
    assert!(!sessions.contains("remove1"));
    assert!(!sessions.contains("remove2"));
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn kill_session_clear_alerts() {
    let tmux = TmuxServer::new("kill_session_alerts");
    tmux.run(&[
        "-f/dev/null",
        "new",
        "-d",
        "-s",
        "alerttest",
        "-x80",
        "-y24",
    ]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // kill-session -C: clear alert flags without killing the session
    tmux.run(&["kill-session", "-C", "-t", "alerttest"]);

    // Session should still exist
    let sessions = tmux.run(&["list-sessions", "-F", "#{session_name}"]);
    assert!(
        sessions.contains("alerttest"),
        "session should survive -C flag"
    );
}

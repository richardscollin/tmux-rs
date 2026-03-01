use super::*;

/// Test kill-session: default (kill target), -a (kill all others), -C (clear alerts).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn kill_session_all_branches() {
    let tmux = TmuxServer::new("kill_session_cov");

    // Create 3 sessions
    let conf =
        tmux.write_temp("new -sfirst -x80 -y24\nnew -ssecond -x80 -y24\nnew -sthird -x80 -y24\n");
    let f_flag = format!("-f{}", conf.path_str());
    tmux.run(&[&f_flag, "start"]);

    // Verify we have 3 sessions
    let output = tmux.run(&["list-sessions", "-F", "#{session_name}"]);
    let sessions: Vec<&str> = output.lines().collect();
    assert_eq!(sessions.len(), 3);

    // -- kill-session -C: clear alert flags (doesn't destroy session)
    tmux.run(&["kill-session", "-C", "-tsecond"]);
    // Session should still exist
    let output = tmux.run(&["has-session", "-tsecond"]);
    assert!(output.is_empty()); // has-session prints nothing on success

    // -- kill-session -a: kill all except target
    tmux.run(&["kill-session", "-a", "-tsecond"]);
    let output = tmux.run(&["list-sessions", "-F", "#{session_name}"]);
    assert_eq!(output.trim(), "second", "only 'second' should survive -a");

    // -- kill-session (default): kill the target session
    // Create a new session first so the server doesn't exit
    tmux.run(&["new", "-dsfourth", "-x80", "-y24"]);
    tmux.run(&["kill-session", "-tsecond"]);
    let output = tmux.run(&["list-sessions", "-F", "#{session_name}"]);
    assert_eq!(output.trim(), "fourth", "only 'fourth' should remain");
}

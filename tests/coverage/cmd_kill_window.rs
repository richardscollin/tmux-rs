use super::*;

/// Test kill-window default, -a (kill all others), and -a with single window.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn kill_window_branches() {
    let tmux = TmuxServer::new("kill_window_cov");

    let conf = tmux.write_temp("new -smain -x80 -y24\nneww\nneww\n");
    let f_flag = format!("-f{}", conf.path_str());
    tmux.run(&[&f_flag, "start"]);

    // Verify 3 windows
    let count = tmux.run(&["list-windows", "-tmain", "-F", "#{window_index}"]);
    assert_eq!(count.lines().count(), 3);

    // -- kill-window (default): kill a specific window
    tmux.run(&["kill-window", "-tmain:2"]);
    let count = tmux.run(&["list-windows", "-tmain", "-F", "#{window_index}"]);
    assert_eq!(count.lines().count(), 2);

    // -- kill-window -a: kill all except current (select last window so rb_prev is non-null)
    tmux.run(&["neww", "-tmain"]); // back to 3 windows
    let indices = tmux.run(&["list-windows", "-tmain", "-F", "#{window_index}"]);
    let last = indices.lines().last().unwrap();
    tmux.run(&["select-window", &format!("-tmain:{last}")]);
    tmux.run(&["kill-window", "-a", "-tmain"]);
    let count = tmux.run(&["list-windows", "-tmain", "-F", "#{window_index}"]);
    assert_eq!(
        count.lines().count(),
        1,
        "only current window should remain after -a"
    );

    // -- kill-window -a with only one window: should be a no-op
    tmux.run(&["kill-window", "-a", "-tmain"]);
    let count = tmux.run(&["list-windows", "-tmain", "-F", "#{window_index}"]);
    assert_eq!(count.lines().count(), 1, "single window should survive -a");
}

/// Test unlink-window: error on single-linked, success on multi-linked, -k force.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn unlink_window_branches() {
    let tmux = TmuxServer::new("unlink_window_cov");

    let conf = tmux.write_temp("new -sfirst -x80 -y24\nneww\nnew -ssecond -x80 -y24\n");
    let f_flag = format!("-f{}", conf.path_str());
    tmux.run(&[&f_flag, "start"]);

    // -- unlink-window without -k on single-linked window: should error
    let output = tmux.try_run(&["unlink-window", "-tsecond"]);
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("only linked to one session"),
        "expected 'only linked to one session' error, got: {}",
        stderr
    );

    // Link a window from "first" into "second"
    tmux.run(&["link-window", "-sfirst:0", "-tsecond:5"]);
    let count = tmux.run(&["list-windows", "-tsecond", "-F", "#{window_index}"]);
    assert_eq!(count.lines().count(), 2, "second should now have 2 windows");

    // -- unlink-window on multi-linked window: should succeed
    tmux.run(&["unlink-window", "-tsecond:5"]);
    let count = tmux.run(&["list-windows", "-tsecond", "-F", "#{window_index}"]);
    assert_eq!(
        count.lines().count(),
        1,
        "unlink should remove the linked window"
    );

    // -- unlink-window -k: force unlink even if only linked once
    tmux.run(&["neww", "-tsecond"]); // need 2 windows so session survives
    tmux.run(&["unlink-window", "-k", "-tsecond:0"]);
    let count = tmux.run(&["list-windows", "-tsecond", "-F", "#{window_index}"]);
    assert_eq!(count.lines().count(), 1, "-k should force unlink");
}

/// Test kill-window -a with duplicate window links (same window linked twice).
/// The found>1 path kills the current window too, destroying the session.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn kill_window_a_duplicate_links() {
    let tmux = TmuxServer::new("kill_window_dup");

    // Create two sessions so the server survives when "main" is destroyed
    let conf = tmux.write_temp("new -smain -x80 -y24\nneww\nnew -skeeper -x80 -y24\n");
    let f_flag = format!("-f{}", conf.path_str());
    tmux.run(&[&f_flag, "start"]);

    // Link window 0 again at index 5 (same window, two winlinks)
    tmux.run(&["link-window", "-smain:0", "-tmain:5"]);
    let count = tmux.run(&["list-windows", "-tmain", "-F", "#{window_index}"]);
    assert_eq!(count.lines().count(), 3, "should have 3 winlinks (0,1,5)");

    // Select window 0, then kill-window -a: kills window 1, then sees
    // window 0 at both index 0 and 5 (found>1), so kills it too.
    // This destroys the session since all windows are gone.
    tmux.run(&["select-window", "-tmain:0"]);
    tmux.run(&["kill-window", "-a", "-tmain"]);

    // "main" session should be gone, but server still alive via "keeper"
    let output = tmux.try_run(&["has-session", "-tmain"]);
    assert!(!output.status.success(), "main session should be destroyed");
    let output = tmux.try_run(&["has-session", "-tkeeper"]);
    assert!(output.status.success(), "keeper session should still exist");
}

use super::*;

/// Respawn pane with -k (kill existing process).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn respawn_pane_kill() {
    let tmux = TmuxServer::new("respawnp_kill");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let pid_before = tmux.display("#{pane_pid}");
    tmux.run(&["respawnp", "-k"]);
    sleep_ms(200);
    let pid_after = tmux.display("#{pane_pid}");

    assert_ne!(
        pid_before, pid_after,
        "pane pid should change after respawn"
    );
}

/// Respawn pane without -k on a dead pane.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn respawn_pane_dead() {
    let tmux = TmuxServer::new("respawnp_dead");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["set", "remain-on-exit", "on"]);

    // Kill the pane's process to make it dead
    tmux.run(&["send-keys", "exit", "Enter"]);
    sleep_ms(500);

    let dead = tmux.display("#{pane_dead}");
    assert_eq!(dead, "1", "pane should be dead");

    // Respawn without -k should work on dead pane
    tmux.run(&["respawnp"]);
    sleep_ms(200);
    let dead_after = tmux.display("#{pane_dead}");
    assert_eq!(dead_after, "0", "pane should be alive after respawn");
}

/// Respawn pane without -k on alive pane (should fail).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn respawn_pane_alive_no_kill() {
    let tmux = TmuxServer::new("respawnp_alive");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let result = tmux.try_run(&["respawnp"]);
    assert!(
        !result.status.success(),
        "respawn without -k on alive pane should fail"
    );
}

/// Respawn pane with custom command.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn respawn_pane_custom_command() {
    let tmux = TmuxServer::new("respawnp_cmd");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["respawnp", "-k", "cat"]);
    sleep_ms(200);

    let cmd = tmux.display("#{pane_current_command}");
    assert_eq!(cmd, "cat");
}

/// Respawn pane with -e (environment variable).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn respawn_pane_env() {
    let tmux = TmuxServer::new("respawnp_env");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["respawnp", "-k", "-e", "MY_VAR=hello"]);
    sleep_ms(200);

    let dead = tmux.display("#{pane_dead}");
    assert_eq!(dead, "0", "pane should be alive after respawn with -e");
}

/// Respawn pane with -c (start directory).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn respawn_pane_start_directory() {
    let tmux = TmuxServer::new("respawnp_cwd");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["respawnp", "-k", "-c", "/tmp"]);
    sleep_ms(200);

    let dead = tmux.display("#{pane_dead}");
    assert_eq!(dead, "0", "pane should be alive after respawn with -c");
    let cwd = tmux.display("#{pane_current_path}");
    assert!(
        cwd.contains("/tmp"),
        "pane should start in /tmp, got: {cwd}"
    );
}

/// Respawn pane with multiple -e flags.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn respawn_pane_multi_env() {
    let tmux = TmuxServer::new("respawnp_menv");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["respawnp", "-k", "-e", "VAR1=one", "-e", "VAR2=two"]);
    sleep_ms(200);

    let dead = tmux.display("#{pane_dead}");
    assert_eq!(
        dead, "0",
        "pane should be alive after respawn with multiple -e"
    );
}

/// Respawn dead pane with -c and custom command.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn respawn_pane_dead_with_cwd() {
    let tmux = TmuxServer::new("respawnp_deadcwd");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["set", "remain-on-exit", "on"]);

    // Kill the pane's process to make it dead
    tmux.run(&["send-keys", "exit", "Enter"]);
    sleep_ms(500);

    let dead = tmux.display("#{pane_dead}");
    assert_eq!(dead, "1", "pane should be dead");

    // Respawn with -c start directory
    tmux.run(&["respawnp", "-c", "/tmp"]);
    sleep_ms(200);
    let dead_after = tmux.display("#{pane_dead}");
    assert_eq!(
        dead_after, "0",
        "pane should be alive after respawn with -c"
    );
}

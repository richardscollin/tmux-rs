use super::*;

/// Respawn window with -k.
#[test]
fn respawn_window_kill() {
    let tmux = TmuxServer::new("respawnw_kill");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let pid_before = tmux.display("#{pane_pid}");
    tmux.run(&["respawnw", "-k"]);
    sleep_ms(200);
    let pid_after = tmux.display("#{pane_pid}");

    assert_ne!(pid_before, pid_after);
}

/// Respawn window with custom command using -k.
#[test]
fn respawn_window_custom_cmd() {
    let tmux = TmuxServer::new("respawnw_cmd");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["respawnw", "-k", "cat"]);
    sleep_ms(200);
    let cmd = tmux.display("#{pane_current_command}");
    assert_eq!(cmd, "cat");
}

/// Respawn alive window without -k should fail.
#[test]
fn respawn_window_alive_no_kill() {
    let tmux = TmuxServer::new("respawnw_alive");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let result = tmux.try_run(&["respawnw"]);
    assert!(!result.status.success());
}

/// Respawn window with -e (environment).
#[test]
fn respawn_window_env() {
    let tmux = TmuxServer::new("respawnw_env");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["respawnw", "-k", "-e", "FOO=bar"]);
    sleep_ms(200);
    let dead = tmux.display("#{pane_dead}");
    assert_eq!(dead, "0");
}

/// Respawn window with -c (start directory).
#[test]
fn respawn_window_start_directory() {
    let tmux = TmuxServer::new("respawnw_cwd");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["respawnw", "-k", "-c", "/tmp"]);
    sleep_ms(200);

    let dead = tmux.display("#{pane_dead}");
    assert_eq!(dead, "0", "window should be alive after respawn with -c");
    let cwd = tmux.display("#{pane_current_path}");
    assert!(
        cwd.contains("/tmp"),
        "pane should start in /tmp, got: {cwd}"
    );
}

/// Respawn window with multiple -e flags.
#[test]
fn respawn_window_multi_env() {
    let tmux = TmuxServer::new("respawnw_menv");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["respawnw", "-k", "-e", "A=1", "-e", "B=2"]);
    sleep_ms(200);
    let dead = tmux.display("#{pane_dead}");
    assert_eq!(
        dead, "0",
        "window should be alive after respawn with multiple -e"
    );
}

/// Respawn window with -k and -c (start directory) together.
#[test]
fn respawn_window_kill_with_cwd() {
    let tmux = TmuxServer::new("respawnw_kcwd");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["respawnw", "-k", "-c", "/tmp"]);
    sleep_ms(200);
    let dead = tmux.display("#{pane_dead}");
    assert_eq!(dead, "0", "window should be alive after respawn -k -c");
    let cwd = tmux.display("#{pane_current_path}");
    assert!(
        cwd.contains("/tmp"),
        "pane should start in /tmp, got: {cwd}"
    );
}

/// Respawn window with -k, -c, and -e combined.
#[test]
fn respawn_window_kill_cwd_env() {
    let tmux = TmuxServer::new("respawnw_kce");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["respawnw", "-k", "-c", "/tmp", "-e", "FOO=bar"]);
    sleep_ms(200);
    let dead = tmux.display("#{pane_dead}");
    assert_eq!(
        dead, "0",
        "window should be alive after respawn with -k -c -e"
    );
}

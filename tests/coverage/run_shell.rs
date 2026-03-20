use super::*;

/// Test run-shell: basic command, -b (background), -d (delay),
/// -c (cwd), -t (target), -E (show stderr), errors.

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn run_shell_basic() {
    let tmux = TmuxServer::new("runsh_basic");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // run-shell: run a command and capture output
    let out = tmux.run(&["run-shell", "echo hello"]);
    assert!(out.contains("hello"), "got: {out}");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn run_shell_no_command() {
    let tmux = TmuxServer::new("runsh_nocmd");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // run-shell with no args and no -d: should return immediately
    tmux.run(&["run-shell"]);
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn run_shell_background() {
    let tmux = TmuxServer::new("runsh_bg");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // -b: run in background (don't wait)
    tmux.run(&["run-shell", "-b", "true"]);
    sleep_ms(100);
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn run_shell_delay() {
    let tmux = TmuxServer::new("runsh_delay");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // -d: delay before running (run in background to avoid blocking)
    tmux.run(&["run-shell", "-b", "-d", "0.01", "true"]);
    sleep_ms(100);
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn run_shell_delay_invalid() {
    let tmux = TmuxServer::new("runsh_delay_bad");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // -d with invalid value
    let out = tmux.try_run(&["run-shell", "-d", "notanumber", "true"]);
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("invalid delay"), "got: {stderr}");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn run_shell_cwd() {
    let tmux = TmuxServer::new("runsh_cwd");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // -c: set working directory
    let out = tmux.run(&["run-shell", "-c", "/tmp", "pwd"]);
    // On macOS /tmp -> /private/tmp
    assert!(
        out.contains("/tmp") || out.contains("/private/tmp"),
        "expected /tmp, got: {out}"
    );
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn run_shell_target_pane() {
    let tmux = TmuxServer::new("runsh_target");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // -t: target pane (exercises the wp_id path)
    tmux.run(&["run-shell", "-t", ":.0", "echo targeted"]);
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn run_shell_show_stderr() {
    let tmux = TmuxServer::new("runsh_stderr");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // -E: show stderr in output (just exercises the flag path)
    let out = tmux.run(&["run-shell", "-E", "echo ok"]);
    assert!(out.contains("ok"), "got: {out}");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn run_shell_failing_command() {
    let tmux = TmuxServer::new("runsh_fail");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Command that exits non-zero — run-shell reports exit status
    let out = tmux.try_run(&["run-shell", "exit 1"]);
    // The command itself may succeed (run-shell returns normal) but output
    // may or may not include the return code depending on client state
    let _ = out;
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn run_shell_delay_only() {
    let tmux = TmuxServer::new("runsh_donly");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // -d with no command: just delay then return
    tmux.run(&["run-shell", "-b", "-d", "0.01"]);
    sleep_ms(100);
}

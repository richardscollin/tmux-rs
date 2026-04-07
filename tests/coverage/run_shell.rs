use super::*;

/// Basic run-shell via control mode.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn run_shell_basic() {
    let tmux = TmuxServer::new("run_basic");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);

    // run-shell echoes output to the pane in view mode;
    // from CLI it waits and returns.
    let out = tmux.run(&["run", "-b", "echo hello"]);
    // -b doesn't wait, so no output captured; just verify success
    let _ = out;
}

/// run-shell captures output.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn run_shell_output() {
    let tmux = TmuxServer::new("run_output");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);

    let cmd = "run-shell 'echo hello_from_run'\ndetach-client\n";
    let output = tmux.run_with_stdin(&["-C", "attach"], cmd.as_bytes());
    assert!(output.status.success());
}

/// run-shell with -b (background, don't wait).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn run_shell_background() {
    let tmux = TmuxServer::new("run_bg");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);

    // -b returns immediately
    tmux.run(&["run", "-b", "sleep 0"]);
    let has = tmux.try_run(&["has-session"]);
    assert!(has.status.success());
}

/// run-shell with -d (delay).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn run_shell_delay() {
    let tmux = TmuxServer::new("run_delay");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);

    // -d 0 delay (immediate timer)
    tmux.run(&["run", "-b", "-d0", "echo delayed"]);
    sleep_ms(100);
    let has = tmux.try_run(&["has-session"]);
    assert!(has.status.success());
}

/// run-shell with invalid delay.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn run_shell_invalid_delay() {
    let tmux = TmuxServer::new("run_baddelay");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);

    let result = tmux.try_run(&["run", "-d", "abc", "echo test"]);
    assert!(!result.status.success());
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        stderr.contains("invalid delay"),
        "expected 'invalid delay' error, got: {stderr}"
    );
}

/// run-shell with -C (command mode, tmux commands instead of shell).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn run_shell_command_mode() {
    let tmux = TmuxServer::new("run_C");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);

    // -C runs tmux commands, not shell commands
    tmux.run(&["run", "-C", "set -g @runtest runval"]);
    let out = tmux.run(&["show", "-gv", "@runtest"]);
    assert_eq!(out.trim(), "runval");
}

/// run-shell exit code propagation (non-zero exit shows message in pane).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn run_shell_exit_code() {
    let tmux = TmuxServer::new("run_exitcode");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);

    // Non-zero exit code from shell command - exercises WIFEXITED/WEXITSTATUS path
    // The "returned N" message is printed to the pane view, not to control stdout
    tmux.run(&["run", "-b", "exit 42"]);
    sleep_ms(200);
    let has = tmux.try_run(&["has-session"]);
    assert!(has.status.success());
}

/// run-shell signal termination.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn run_shell_signal() {
    let tmux = TmuxServer::new("run_signal");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);

    // Kill self with signal - exercises WIFSIGNALED/WTERMSIG path
    // The "terminated by signal" message is printed to the pane view
    tmux.run(&["run", "-b", "kill -15 $$"]);
    sleep_ms(200);
    let has = tmux.try_run(&["has-session"]);
    assert!(has.status.success());
}

/// run-shell with -c (working directory).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn run_shell_directory() {
    let tmux = TmuxServer::new("run_cwd");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);

    tmux.run(&["run", "-b", "-c", "/tmp", "pwd"]);
    let has = tmux.try_run(&["has-session"]);
    assert!(has.status.success());
}

/// run-shell with no arguments and -d (delay only, no command).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn run_shell_delay_only() {
    let tmux = TmuxServer::new("run_delay_only");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);

    // -d with no command: timer fires and continues
    let cmd = "run-shell -d0\ndetach-client\n";
    let output = tmux.run_with_stdin(&["-C", "attach"], cmd.as_bytes());
    assert!(output.status.success());
}

/// run-shell with no arguments and no -d (early return).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn run_shell_no_args() {
    let tmux = TmuxServer::new("run_noargs");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);

    // No delay and no command: returns immediately
    tmux.run(&["run"]);
}

/// run-shell using 'run' alias.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn run_shell_alias() {
    let tmux = TmuxServer::new("run_alias");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);

    tmux.run(&["run-shell", "-b", "echo alias_test"]);
    let has = tmux.try_run(&["has-session"]);
    assert!(has.status.success());
}

/// run-shell with -t target pane.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn run_shell_target_pane() {
    let tmux = TmuxServer::new("run_target");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["split-window", "-d"]);

    tmux.run(&["run", "-b", "-t", ":.1", "echo pane1"]);
    let has = tmux.try_run(&["has-session"]);
    assert!(has.status.success());
}

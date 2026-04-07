use super::*;

/// Helper: run a command through a control-mode client and return stdout.
fn control_cmd(tmux: &TmuxServer, cmd: &str) -> String {
    let input = format!("{cmd}\ndetach\n");
    let output = tmux.run_with_stdin(&["-C", "attach"], input.as_bytes());
    String::from_utf8_lossy(&output.stdout).into_owned()
}

/// Helper: run multiple commands through a single control-mode session.
fn control_cmds(tmux: &TmuxServer, cmds: &[&str]) -> String {
    let mut input = String::new();
    for cmd in cmds {
        input.push_str(cmd);
        input.push('\n');
    }
    input.push_str("detach\n");
    let output = tmux.run_with_stdin(&["-C", "attach"], input.as_bytes());
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn control_cmd_ok(tmux: &TmuxServer, cmd: &str) {
    let stdout = control_cmd(tmux, cmd);
    assert!(
        !stdout.contains("server exited unexpectedly"),
        "server crashed running '{cmd}': {stdout}"
    );
    let has = tmux.try_run(&["has-session"]);
    assert!(has.status.success(), "server died after '{cmd}'");
}

/// Test basic exec function paths.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn display_panes_exec() {
    let tmux = TmuxServer::new("dp_exec");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // display-panes -b (background mode, CMD_RETURN_NORMAL)
    control_cmd_ok(&tmux, "display-panes -b");

    // display-panes -b -N (no key handler path)
    control_cmd_ok(&tmux, "display-panes -b -N");

    // display-panes -b -d 100 (custom delay via -d flag)
    control_cmd_ok(&tmux, "display-panes -b -d 100");

    // display-panes -d with invalid value (error path)
    let stdout = control_cmd(&tmux, "display-panes -d abc");
    assert!(
        stdout.contains("delay"),
        "expected error for invalid delay: {stdout}"
    );

    // display-panes with custom template
    control_cmd_ok(&tmux, "display-panes -b \"select-pane -t '%%%'\"");

    // display-panes while overlay is already active (early return)
    let stdout = control_cmds(&tmux, &["display-panes -b -d 5000", "display-panes -b"]);
    assert!(
        !stdout.contains("server exited"),
        "double overlay crashed: {stdout}"
    );

    // display-panes wait mode (no -b) - client disconnect triggers free callback
    let stdout = control_cmd(&tmux, "display-panes");
    assert!(
        !stdout.contains("server exited"),
        "wait mode crashed: {stdout}"
    );

    // Add panes
    tmux.run(&["resize-window", "-x", "80", "-y", "200"]);
    for _ in 0..4 {
        tmux.run(&["split-window", "-d", "-v", "-l", "3"]);
    }
}

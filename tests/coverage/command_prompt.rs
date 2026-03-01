use super::*;

/// command-prompt in control mode causes server crash due to
/// subtraction overflow in screen_reinit when status_line_size returns 0.
///
/// Root cause: status_push_screen calls screen_init with height=0 for
/// control mode clients (status_line_size returns 0 for CLIENT_CONTROL).
/// screen_init -> screen_reinit does `screen_size_y(s) - 1` which panics
/// with overflow on u32 when height is 0.
///
/// Upstream C tmux silently wraps (unsigned underflow is defined in C).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn command_prompt_control_mode_crash() {
    let tmux = TmuxServer::new("cmd_prompt_crash");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);

    // This crashes the server: command-prompt in control mode triggers
    // screen_reinit with a 0-height screen.
    let output = tmux.run_with_stdin(&["-C", "attach"], b"command-prompt -b\ndetach\n");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.contains("server exited unexpectedly"),
        "server crashed: {stdout}"
    );

    // Verify server is still alive
    let has = tmux.try_run(&["has-session"]);
    assert!(
        has.status.success(),
        "server died after command-prompt in control mode"
    );
}

/// Test command-prompt exec function code paths with various flags.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn command_prompt_exec_paths() {
    let tmux = TmuxServer::new("cmd_prompt_exec");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Default prompt, no template (count==0, no -p): default ":" prompt, space=0
    tmux.run_with_stdin(&["-C", "attach"], b"command-prompt -b\ndetach\n");

    // Template prompt (count!=0, no -p): formats prompt from template
    tmux.run_with_stdin(
        &["-C", "attach"],
        b"command-prompt -b \"display-message -p '%%'\"\ndetach\n",
    );

    // Custom prompt (-p) with template
    tmux.run_with_stdin(
        &["-C", "attach"],
        b"command-prompt -b -p \"name:\" \"display '%%'\"\ndetach\n",
    );

    // Custom input (-I) with template
    tmux.run_with_stdin(
        &["-C", "attach"],
        b"command-prompt -b -I \"default\" \"display '%%'\"\ndetach\n",
    );

    // Multiple comma-separated prompts with fewer inputs (exercises
    // strsep loop and null next_input fallback for the second prompt)
    tmux.run_with_stdin(
        &["-C", "attach"],
        b"command-prompt -b -p \"first:,second:\" -I \"val1\" \"display '%%'\"\ndetach\n",
    );

    // Multiple prompts with matching inputs
    tmux.run_with_stdin(
        &["-C", "attach"],
        b"command-prompt -b -p \"a:,b:\" -I \"x,y\" \"display '%%'\"\ndetach\n",
    );

    // Multiple prompts without any inputs
    tmux.run_with_stdin(
        &["-C", "attach"],
        b"command-prompt -b -p \"one:,two:,three:\" \"display '%%'\"\ndetach\n",
    );

    // Literal mode (-l): single prompt, no comma splitting
    tmux.run_with_stdin(
        &["-C", "attach"],
        b"command-prompt -b -l -p \"test:\" -I \"lit\" \"display '%%'\"\ndetach\n",
    );

    // Prompt type (-T search)
    tmux.run_with_stdin(
        &["-C", "attach"],
        b"command-prompt -b -T search \"display '%%'\"\ndetach\n",
    );

    // Single char mode (-1)
    tmux.run_with_stdin(
        &["-C", "attach"],
        b"command-prompt -b -1 \"display '%%'\"\ndetach\n",
    );

    // Numeric mode (-N)
    tmux.run_with_stdin(
        &["-C", "attach"],
        b"command-prompt -b -N \"display '%%'\"\ndetach\n",
    );

    // Incremental mode (-i): also sets wait=false
    tmux.run_with_stdin(
        &["-C", "attach"],
        b"command-prompt -i \"display '%%'\"\ndetach\n",
    );

    // Key mode (-k)
    tmux.run_with_stdin(
        &["-C", "attach"],
        b"command-prompt -b -k \"display '%%'\"\ndetach\n",
    );

    // -F flag (format expansion in template)
    tmux.run_with_stdin(
        &["-C", "attach"],
        b"command-prompt -bF \"display '%%'\"\ndetach\n",
    );

    // prompt_string already set: second command-prompt hits early return
    tmux.run_with_stdin(
        &["-C", "attach"],
        b"command-prompt -b\ncommand-prompt -b\ndetach\n",
    );
}

/// Test -T with invalid type triggers error path.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn command_prompt_invalid_type() {
    let tmux = TmuxServer::new("cmd_prompt_badtype");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);

    let output = tmux.run_with_stdin(&["-C", "attach"], b"command-prompt -T bogus\ndetach\n");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("unknown type"));
}

/// Test callback cancel path (s=NULL) when client disconnects.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn command_prompt_blocking_cancel() {
    let tmux = TmuxServer::new("cmd_prompt_cancel");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);

    // Without -b, command-prompt blocks (CMD_RETURN_WAIT). EOF on stdin
    // triggers client disconnect -> prompt cancel -> callback(s=NULL) ->
    // cmdq_continue. This exercises the wait path and callback cancel.
    let output = tmux.run_with_stdin(&["-C", "attach"], b"command-prompt\n");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.contains("server exited unexpectedly"),
        "server crashed: {stdout}"
    );

    sleep_ms(500);

    // Server should still be alive
    let has = tmux.try_run(&["has-session"]);
    assert!(has.status.success());
}

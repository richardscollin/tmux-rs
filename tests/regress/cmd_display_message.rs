use super::*;

/// Test display-message with -p flag (print to stdout) and various format strings.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn display_message_print_and_formats() {
    let tmux = TmuxServer::new("dispmsg_print");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Basic -p with custom message
    let out = tmux.run(&["display", "-p", "hello world"]);
    assert_eq!(out.trim(), "hello world");

    // -p with format string (exercises format_expand_time path)
    let out = tmux.run(&["display", "-p", "#{session_name}"]);
    assert_eq!(out.trim(), "0");

    // -p with -F format flag (uses -F instead of positional arg)
    let out = tmux.run(&["display", "-p", "-F", "#{window_index}"]);
    assert_eq!(out.trim(), "0");

    // Default template (no message, no -F) - exercises DISPLAY_MESSAGE_TEMPLATE
    // Output format: "[<session>] <widx>:<wname>, current pane <pidx> - (HH:MM DD-Mon-YY)"
    let out = tmux.run(&["display", "-p"]);
    assert!(
        out.starts_with("[0] 0:"),
        "default template should start with session/window info, got: {}",
        out.trim()
    );

    // -p with -l (literal, no format expansion)
    let out = tmux.run(&["display", "-p", "-l", "#{session_name}"]);
    assert_eq!(out.trim(), "#{session_name}");
}

/// Test display-message -a (list all format variables).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn display_message_all_variables() {
    let tmux = TmuxServer::new("dispmsg_allvars");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // -a lists all format key=value pairs via cmd_display_message_each
    let out = tmux.run(&["display", "-a"]);
    // Should contain common format variables
    assert!(out.contains("session_name="), "should list session_name");
    assert!(out.contains("window_index="), "should list window_index");
    assert!(out.contains("pane_index="), "should list pane_index");
}

/// Test display-message -v (verbose format expansion).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn display_message_verbose() {
    let tmux = TmuxServer::new("dispmsg_verbose");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // -v flag enables verbose format expansion (FORMAT_VERBOSE)
    // With -p to capture output; verbose mode prints expansion trace lines
    let out = tmux.run(&["display", "-p", "-v", "#{session_name}"]);
    let lines: Vec<&str> = out.lines().collect();
    // Verbose output includes "# ..." trace lines followed by the result
    assert!(
        lines.iter().any(|l| l.starts_with("# ")),
        "verbose mode should output trace lines, got: {}",
        out
    );
    assert_eq!(
        lines.last().unwrap().trim(),
        "0",
        "last line should be the expanded value"
    );
}

/// Test display-message error paths.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn display_message_errors() {
    let tmux = TmuxServer::new("dispmsg_errors");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // -F and positional argument together -> error
    let out = tmux.try_run(&["display", "-F", "#{session_name}", "hello"]);
    assert!(
        !out.status.success(),
        "-F and argument together should fail"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("only one of -F or argument"),
        "should give specific error, got: {}",
        stderr
    );

    // -d with invalid delay value
    let out = tmux.try_run(&["display", "-d", "notanumber"]);
    assert!(!out.status.success(), "-d with bad value should fail");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("delay"),
        "should mention delay in error, got: {}",
        stderr
    );

    // -d with negative (out of range) delay
    let out = tmux.try_run(&["display", "-d", "-5"]);
    assert!(!out.status.success(), "-d with negative value should fail");
}

/// Test display-message -d with valid delay.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn display_message_delay() {
    let tmux = TmuxServer::new("dispmsg_delay");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // -d with valid delay (exercises the Ok(v) => delay = v path)
    // Use -p so we can see the output; the delay affects status bar display time
    // but -p just prints, so this tests the parsing path
    tmux.run(&["display", "-d", "5000", "hello with delay"]);
}

/// Test display-message via control mode client (exercises %message output path).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn display_message_control_mode() {
    let tmux = TmuxServer::new("dispmsg_ctrl");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Attach in control mode and send display-message
    let commands = b"display-message 'hello from control'\n";
    let output = tmux.run_with_stdin(&["-C", "a"], commands);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Control mode output should contain %message prefix
    assert!(
        stdout.contains("%message"),
        "control mode should output %message, got: {}",
        stdout
    );
    assert!(
        stdout.contains("hello from control"),
        "should contain our message text, got: {}",
        stdout
    );
}

/// Test display-message with -N and -C flags (affects status_message_set).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn display_message_status_flags() {
    let tmux = TmuxServer::new("dispmsg_flags");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // -N flag (no wait for status message) - exercises nflag branch
    // -C flag (clear status) - exercises cflag branch
    // These need a client to display to; via control mode we test the control path
    // but we can still exercise the flag parsing
    let commands = b"display-message -N 'no-wait msg'\ndisplay-message -C\n";
    let output = tmux.run_with_stdin(&["-C", "a"], commands);
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Control mode client takes the control branch, but flags are still parsed
    assert!(
        stdout.contains("%message"),
        "control mode should output %message"
    );
}

/// Test display-message with -I flag (pane input mode).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn display_message_input_mode() {
    let tmux = TmuxServer::new("dispmsg_input");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // -I starts pane input mode; via control mode we can trigger it
    let commands = b"display-message -I\n";
    let output = tmux.run_with_stdin(&["-C", "a"], commands);
    // Just verify the server doesn't crash
    let has = tmux.try_run(&["has"]);
    assert!(
        has.status.success(),
        "server should still be running after -I"
    );
    let _ = output;

    // -I with invalid pane target (exercises wp.is_null() -> return NORMAL path)
    // CMD_FIND_CANFAIL means an invalid target produces null wp instead of error
    let out = tmux.try_run(&["display", "-I", "-t", "nosuchpane"]);
    // Should succeed (returns NORMAL when wp is null)
    let _ = out;
}

/// Test display-message targeting a different client session (exercises client matching branches).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn display_message_client_mismatch() {
    let tmux = TmuxServer::new("dispmsg_climatch");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24", "-s", "sess1"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    // Create a second session
    tmux.run(&["new", "-d", "-s", "sess2"]);

    // Display targeting sess2 (when the control client is attached to sess1)
    // This exercises the tc->session != s branch, falling through to cmd_find_best_client
    let commands = b"display-message -t sess2 -p '#{session_name}'\n";
    let output = tmux.run_with_stdin(&["-C", "a", "-t", "sess1"], commands);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("sess2"),
        "should display from sess2 context, got: {}",
        stdout
    );
}

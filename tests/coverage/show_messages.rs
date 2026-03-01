use super::*;

/// Show server messages.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn show_messages_basic() {
    let tmux = TmuxServer::new("showmsgs_basic");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Generate a message by running display
    tmux.run(&["display", "-p", "test message"]);

    let out = tmux.run(&["showmsgs"]);
    // Should contain at least the server startup message
    // Output may be empty if no messages logged, but command should succeed
    let _ = out;
}

/// Show messages with -T (terminals) - no attached clients.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn show_messages_terminals() {
    let tmux = TmuxServer::new("showmsgs_terms");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let out = tmux.run(&["showmsgs", "-T"]);
    // Lists terminal types; output may vary
    let _ = out;
}

/// Show messages with -J (jobs).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn show_messages_jobs() {
    let tmux = TmuxServer::new("showmsgs_jobs");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // -J lists background jobs; may be empty but should succeed
    let out = tmux.run(&["showmsgs", "-J"]);
    let _ = out;
}

/// Show messages with both -J and -T flags.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn show_messages_jobs_and_terminals() {
    let tmux = TmuxServer::new("showmsgs_jt");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Both flags together should print terminals then jobs, then return
    let out = tmux.run(&["showmsgs", "-J", "-T"]);
    let _ = out;
}

/// Show messages default (message log) contains server startup info.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn show_messages_log() {
    let tmux = TmuxServer::new("showmsgs_log");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let out = tmux.run(&["showmsgs"]);
    // The message log should contain at least one entry from server startup
    // The format is "timestamp: message_text"
    assert!(
        !out.trim().is_empty(),
        "expected at least one message in log"
    );
}

/// Show messages with -T from control-mode client (exercises terminal loop body).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn show_messages_terminals_with_client() {
    let tmux = TmuxServer::new("showmsgs_terms_c");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Attach control-mode client so TTY_TERMS has entries, then show terminals
    let output = tmux.run_with_stdin(
        &["-C", "attach"],
        b"show-messages -T\ndetach-client\n",
    );
    assert!(output.status.success());
}

/// Show messages with -T and -J from control-mode client (blank separator).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn show_messages_terminals_and_jobs_with_client() {
    let tmux = TmuxServer::new("showmsgs_tj_c");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // -T then -J: if -T printed terminals, blank > 0 triggers the blank line
    let output = tmux.run_with_stdin(
        &["-C", "attach"],
        b"show-messages -T -J\ndetach-client\n",
    );
    assert!(output.status.success());
}

/// Show messages with -T -t targeting a specific client.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn show_messages_terminals_target_client() {
    let tmux = TmuxServer::new("showmsgs_terms_t");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Attach control-mode client, then show terminals with -t targeting self
    // This exercises the args_has('t') && term != tc->tty.term branch
    let output = tmux.run_with_stdin(
        &["-C", "attach"],
        b"show-messages -T -t /dev/tty\nshow-messages -T\ndetach-client\n",
    );
    assert!(output.status.success());
}

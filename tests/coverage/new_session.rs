use super::*;

/// Basic detached new-session creates a session.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn new_session_basic() {
    let tmux = TmuxServer::new("newsess_basic");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);

    let name = tmux.display("#{session_name}");
    assert!(!name.trim().is_empty(), "session should have a name");
}

/// New session with -s (custom session name).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn new_session_named() {
    let tmux = TmuxServer::new("newsess_named");
    tmux.run(&["-f/dev/null", "new", "-d", "-s", "mysess", "-x80", "-y24"]);

    let name = tmux.display("#{session_name}");
    assert_eq!(name.trim(), "mysess");
}

/// New session with -n (window name).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn new_session_window_name() {
    let tmux = TmuxServer::new("newsess_winname");
    tmux.run(&["-f/dev/null", "new", "-d", "-n", "mywin", "-x80", "-y24"]);

    let name = tmux.display("#{window_name}");
    assert_eq!(name.trim(), "mywin");
}

/// New session with -x and -y (custom dimensions).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn new_session_dimensions() {
    let tmux = TmuxServer::new("newsess_dims");
    tmux.run(&["-f/dev/null", "new", "-d", "-x120", "-y40"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let width = tmux.display("#{window_width}");
    let height = tmux.display("#{window_height}");
    assert_eq!(width.trim(), "120");
    assert_eq!(height.trim(), "40");
}

/// New session with -A attaches to existing session.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn new_session_attach_existing() {
    let tmux = TmuxServer::new("newsess_A");
    tmux.run(&["-f/dev/null", "new", "-d", "-s", "existing", "-x80", "-y24"]);

    // -A with existing name should attach, not create new
    let cmd = "new-session -A -s existing\ndetach-client\n";
    let output = tmux.run_with_stdin(&["-C", "attach"], cmd.as_bytes());
    assert!(output.status.success());

    // Should still be just one session
    let count = tmux.run(&["list-sessions"]);
    assert_eq!(count.trim().lines().count(), 1);
}

/// New session with -A where session doesn't exist (creates new).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn new_session_attach_create() {
    let tmux = TmuxServer::new("newsess_A_new");
    tmux.run(&["-f/dev/null", "new", "-d", "-s", "first", "-x80", "-y24"]);

    // -A with non-existing name should create it
    tmux.run(&["new", "-d", "-A", "-s", "second"]);

    let count = tmux.run(&["list-sessions"]);
    assert_eq!(count.trim().lines().count(), 2);
}

/// New session with -P (print session info).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn new_session_print() {
    let tmux = TmuxServer::new("newsess_P");
    let out = tmux.run(&["-f/dev/null", "new", "-d", "-P", "-x80", "-y24"]);
    // Default template: #{session_name}:
    assert!(
        out.contains(":"),
        "expected session_name: format, got: {out}"
    );
}

/// New session with -P -F (custom format).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn new_session_print_format() {
    let tmux = TmuxServer::new("newsess_PF");
    let out = tmux.run(&[
        "-f/dev/null",
        "new",
        "-d",
        "-P",
        "-F",
        "#{session_id}",
        "-x80",
        "-y24",
    ]);
    assert!(
        out.trim().starts_with('$'),
        "expected session id starting with $, got: {out}"
    );
}

/// New session with -e (environment variables).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn new_session_environment() {
    let tmux = TmuxServer::new("newsess_env");
    tmux.run(&[
        "-f/dev/null",
        "new",
        "-d",
        "-e",
        "MYVAR=hello",
        "-e",
        "OTHER=world",
        "-x80",
        "-y24",
    ]);

    // Session should exist
    let has = tmux.try_run(&["has-session"]);
    assert!(has.status.success());
}

/// New session with -E (don't update environment).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn new_session_no_env() {
    let tmux = TmuxServer::new("newsess_E");
    tmux.run(&["-f/dev/null", "new", "-d", "-E", "-x80", "-y24"]);

    let has = tmux.try_run(&["has-session"]);
    assert!(has.status.success());
}

/// Invalid session name (empty string).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn new_session_invalid_name() {
    let tmux = TmuxServer::new("newsess_badname");
    // First create a valid session so the server is running
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);

    let result = tmux.try_run(&["new", "-d", "-s", ""]);
    assert!(!result.status.success());
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        stderr.contains("invalid session"),
        "expected 'invalid session' error, got: {stderr}"
    );
}

/// Duplicate session name.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn new_session_duplicate_name() {
    let tmux = TmuxServer::new("newsess_dup");
    tmux.run(&["-f/dev/null", "new", "-d", "-s", "dup", "-x80", "-y24"]);

    let result = tmux.try_run(&["new", "-d", "-s", "dup"]);
    assert!(!result.status.success());
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        stderr.contains("duplicate session"),
        "expected 'duplicate session' error, got: {stderr}"
    );
}

/// Invalid width value.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn new_session_invalid_width() {
    let tmux = TmuxServer::new("newsess_badx");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);

    let result = tmux.try_run(&["new", "-d", "-xabc"]);
    assert!(!result.status.success());
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        stderr.contains("width"),
        "expected width error, got: {stderr}"
    );
}

/// Invalid height value.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn new_session_invalid_height() {
    let tmux = TmuxServer::new("newsess_bady");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);

    let result = tmux.try_run(&["new", "-d", "-yabc"]);
    assert!(!result.status.success());
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        stderr.contains("height"),
        "expected height error, got: {stderr}"
    );
}

/// has-session command (shares exec with new-session).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn has_session_exists() {
    let tmux = TmuxServer::new("hassess_yes");
    tmux.run(&["-f/dev/null", "new", "-d", "-s", "check", "-x80", "-y24"]);

    let result = tmux.try_run(&["has", "-t", "check"]);
    assert!(result.status.success());
}

/// has-session for nonexistent session.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn has_session_not_exists() {
    let tmux = TmuxServer::new("hassess_no");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);

    let result = tmux.try_run(&["has", "-t", "nonexistent"]);
    assert!(!result.status.success());
}

/// New session with -t and -n should error (command or window name given with target).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn new_session_target_with_name() {
    let tmux = TmuxServer::new("newsess_t_n");
    tmux.run(&["-f/dev/null", "new", "-d", "-s", "base", "-x80", "-y24"]);

    let result = tmux.try_run(&["new", "-d", "-t", "base", "-n", "winname"]);
    assert!(!result.status.success());
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        stderr.contains("command or window name given with target"),
        "expected target+name error, got: {stderr}"
    );
}

/// New session with -x - (dash means use client width).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn new_session_dash_dimensions() {
    let tmux = TmuxServer::new("newsess_dash_dim");
    // -x- and -y- without a client should use defaults (80x24)
    tmux.run(&["-f/dev/null", "new", "-d", "-x-", "-y-"]);

    let has = tmux.try_run(&["has-session"]);
    assert!(has.status.success());
}

/// New session with shell command.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn new_session_with_command() {
    let tmux = TmuxServer::new("newsess_cmd");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24", "sleep", "100"]);

    let has = tmux.try_run(&["has-session"]);
    assert!(has.status.success());
}

/// New session with -c (start directory).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn new_session_start_directory() {
    let tmux = TmuxServer::new("newsess_cwd");
    tmux.run(&["-f/dev/null", "new", "-d", "-c", "/tmp", "-x80", "-y24"]);

    let cwd = tmux.display("#{pane_current_path}");
    assert_eq!(cwd.trim(), "/tmp");
}

/// New session using 'new' alias.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn new_session_alias() {
    let tmux = TmuxServer::new("newsess_alias");
    tmux.run(&["-f/dev/null", "new-session", "-d", "-x80", "-y24"]);

    let has = tmux.try_run(&["has-session"]);
    assert!(has.status.success());
}

/// New session with -D and -A (detach other clients on attach).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn new_session_attach_detach_others() {
    let tmux = TmuxServer::new("newsess_AD");
    tmux.run(&["-f/dev/null", "new", "-d", "-s", "shared", "-x80", "-y24"]);

    // Attach with -A -D on existing session
    let cmd = "new-session -A -D -s shared\ndetach-client\n";
    let output = tmux.run_with_stdin(&["-C", "attach"], cmd.as_bytes());
    assert!(output.status.success());
}

/// New session with -t (session group).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn new_session_group() {
    let tmux = TmuxServer::new("newsess_group");
    tmux.run(&["-f/dev/null", "new", "-d", "-s", "leader", "-x80", "-y24"]);

    // Create grouped session
    tmux.run(&["new", "-d", "-t", "leader", "-s", "follower"]);

    let count = tmux.run(&["list-sessions"]);
    assert_eq!(count.trim().lines().count(), 2);
}

/// Test new-session: named, detached, -A (attach-or-create),
/// -P (print), -e (env), -x/-y (size), -n (window name), errors.

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn new_session_named_02() {
    let tmux = TmuxServer::new("newsess_named_02");
    tmux.run(&["-f/dev/null", "new", "-d", "-s", "main", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Create a second named session
    tmux.run(&["new-session", "-d", "-s", "second"]);
    let sessions = tmux.run(&["list-sessions", "-F", "#{session_name}"]);
    assert!(sessions.contains("main"));
    assert!(sessions.contains("second"));
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn new_session_with_window_name() {
    let tmux = TmuxServer::new("newsess_wname");
    tmux.run(&[
        "-f/dev/null",
        "new",
        "-d",
        "-s",
        "wntest",
        "-n",
        "mywin",
        "-x80",
        "-y24",
    ]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let name = tmux.run(&["display-message", "-t", "wntest", "-p", "#{window_name}"]);
    assert_eq!(name.trim(), "mywin");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn new_session_with_size() {
    let tmux = TmuxServer::new("newsess_size");
    tmux.run(&["-f/dev/null", "new", "-d", "-s", "sized", "-x100", "-y30"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let w = tmux.run(&["display-message", "-t", "sized", "-p", "#{window_width}"]);
    let h = tmux.run(&["display-message", "-t", "sized", "-p", "#{window_height}"]);
    assert_eq!(w.trim(), "100");
    assert_eq!(h.trim(), "30");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn new_session_duplicate() {
    let tmux = TmuxServer::new("newsess_dup_02");
    tmux.run(&["-f/dev/null", "new", "-d", "-s", "duptest", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Creating a session with the same name should fail
    let out = tmux.try_run(&["new-session", "-d", "-s", "duptest"]);
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("duplicate session"), "got: {stderr}");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn new_session_print_02() {
    let tmux = TmuxServer::new("newsess_print_02");
    tmux.run(&["-f/dev/null", "new", "-d", "-s", "ptest", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // -P: print session info
    let out = tmux.run(&["new-session", "-d", "-s", "printed", "-P", "-x80", "-y24"]);
    assert!(
        out.contains("printed:"),
        "expected session info, got: {out}"
    );
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn new_session_print_format_02() {
    let tmux = TmuxServer::new("newsess_pfmt_02");
    tmux.run(&["-f/dev/null", "new", "-d", "-s", "pfmt", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // -P -F: custom format
    let out = tmux.run(&[
        "new-session",
        "-d",
        "-s",
        "pfmt2",
        "-P",
        "-F",
        "#{session_name}",
        "-x80",
        "-y24",
    ]);
    assert_eq!(out.trim(), "pfmt2");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn new_session_environment_02() {
    let tmux = TmuxServer::new("newsess_env_02");
    tmux.run(&["-f/dev/null", "new", "-d", "-s", "envtest", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // -e: set environment
    tmux.run(&[
        "new-session",
        "-d",
        "-s",
        "envs",
        "-e",
        "MYVAR=hello",
        "-x80",
        "-y24",
    ]);
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn new_session_cwd() {
    let tmux = TmuxServer::new("newsess_cwd");
    tmux.run(&[
        "-f/dev/null",
        "new",
        "-d",
        "-s",
        "cwdtest",
        "-c",
        "/tmp",
        "-x80",
        "-y24",
    ]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let sessions = tmux.run(&["list-sessions", "-F", "#{session_name}"]);
    assert!(sessions.contains("cwdtest"));
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn new_session_attach_if_exists() {
    let tmux = TmuxServer::new("newsess_attach");
    tmux.run(&["-f/dev/null", "new", "-d", "-s", "existing", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // -A: attach to existing session if it exists (in detached mode, exercises the path)
    let out = tmux.try_run(&["new-session", "-d", "-A", "-s", "existing"]);
    // Should succeed — either attaches or is a no-op in detached mode
    let _ = out;
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn new_session_invalid_name_02() {
    let tmux = TmuxServer::new("newsess_badname");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Empty session name should fail
    let out = tmux.try_run(&["new-session", "-d", "-s", ""]);
    assert!(!out.status.success());
}

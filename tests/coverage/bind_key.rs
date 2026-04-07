use super::*;

/// Test bind-key with an unknown key name (coverage: cmd_bind_key.rs lines 42-44)
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn bind_key_unknown_key() {
    let tmux = TmuxServer::new("bind_key_unknown");

    tmux.run(&["-f/dev/null", "new", "-d"]);

    let output = tmux.try_run(&["bind", "INVALIDKEYNAME", "display", "hello"]);
    assert!(
        !output.status.success(),
        "binding an invalid key name should fail"
    );
}

/// Test bind-key with key only, no command (coverage: cmd_bind_key.rs lines 56-58)
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn bind_key_no_command() {
    let tmux = TmuxServer::new("bind_key_no_cmd");

    tmux.run(&["-f/dev/null", "new", "-d"]);

    // bind x with no command should succeed (clears any existing binding)
    tmux.run(&["bind", "x"]);

    // Verify the key is bound (to nothing)
    let output = tmux.run(&["list-keys", "-T", "prefix"]);
    // Should contain a line with "x" that has no command
    assert!(
        output.lines().any(|l| l.contains(" x ")),
        "key 'x' should appear in prefix table bindings"
    );
}

/// Test bind-key with a malformed command string (coverage: cmd_bind_key.rs lines 77-80)
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn bind_key_parse_error() {
    let tmux = TmuxServer::new("bind_key_parse_err");

    tmux.run(&["-f/dev/null", "new", "-d"]);

    // A command with unmatched quotes should fail to parse
    let output = tmux.try_run(&["bind", "y", "if-shell", "'unterminated"]);
    assert!(
        !output.status.success(),
        "bind with a malformed command should fail"
    );
}

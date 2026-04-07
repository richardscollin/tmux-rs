use super::*;

/// Test list-keys basic output (non -N mode) with table iteration,
/// key width calculation, repeat flag logic, and table filtering.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn list_keys_basic() {
    let tmux = TmuxServer::new("list_keys_basic");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);

    // Basic list-keys exercises the non-N path with table iteration,
    // key width calculation, and output formatting.
    let out = tmux.run(&["list-keys"]);
    assert!(out.contains("bind-key"), "should contain bind-key lines");
    assert!(out.contains("prefix"), "should show prefix table");

    // -T filters to a specific table
    let out_prefix = tmux.run(&["list-keys", "-T", "prefix"]);
    assert!(out_prefix.contains("bind-key"));
    for line in out_prefix.lines() {
        if line.contains("bind-key") {
            assert!(line.contains("prefix"), "filtered output: {}", line);
        }
    }

    // Specific key filter in a table
    let out_key = tmux.run(&["list-keys", "-T", "prefix", "d"]);
    assert!(out_key.contains("bind-key"));
    assert!(out_key.contains("detach-client"));
}

/// Test repeat flag rendering (-r) in list-keys output.
/// Default bindings include -r (repeat) keys, so we just verify they appear.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn list_keys_repeat_flag() {
    let tmux = TmuxServer::new("list_keys_repeat");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);

    // Default bindings include repeat keys (e.g. M-Up in root table).
    // Just verify the -r flag appears in the full output.
    let out = tmux.run(&["list-keys"]);
    assert!(
        out.contains("-r "),
        "default bindings should include repeat (-r) keys"
    );
}

/// Test list-keys -N notes mode: without tablename, with -a, with -T,
/// with -P custom prefix, and combined -T -P.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn list_keys_notes_mode() {
    let tmux = TmuxServer::new("list_keys_notes");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);

    // -N without tablename: auto-detects prefix, shows root + prefix tables
    let out_n = tmux.run(&["list-keys", "-N"]);
    assert!(!out_n.is_empty(), "-N should produce output");

    // Bind a key without a note (note will be null) to exercise null-note paths
    tmux.run(&["bind-key", "-T", "prefix", "F1", "display-message", "test"]);

    // -N -a shows all bindings including those without notes (exercises
    // the null-note path in print_notes and the cmd_list_print fallback)
    let out_na = tmux.run(&["list-keys", "-N", "-a"]);
    assert!(!out_na.is_empty());
    assert!(
        out_na.lines().count() >= out_n.lines().count(),
        "-N -a should show at least as many lines as -N"
    );
    // The no-note binding should appear via cmd_list_print fallback
    assert!(
        out_na.contains("display-message"),
        "-a should show the no-note binding"
    );

    // -N -T filters to specific table (tablename branch in notes mode)
    let out_nt = tmux.run(&["list-keys", "-N", "-T", "prefix"]);
    assert!(!out_nt.is_empty());

    // -N -P custom prefix (without tablename)
    let out_np = tmux.run(&["list-keys", "-N", "-P", "MY> "]);
    if !out_np.is_empty() {
        assert!(
            out_np.lines().any(|l| l.contains("MY> ")),
            "custom prefix should appear: {}",
            out_np
        );
    }

    // -N -T -P custom prefix with tablename (exercises the tablename+prefix branch)
    let out_ntp = tmux.run(&["list-keys", "-N", "-T", "prefix", "-P", "PFX "]);
    assert!(!out_ntp.is_empty());
    assert!(
        out_ntp.lines().any(|l| l.starts_with("PFX ")),
        "table prefix should appear: {}",
        out_ntp
    );
}

/// Test list-keys -N with specific key filter.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn list_keys_notes_key_filter() {
    let tmux = TmuxServer::new("list_keys_nkf");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);

    // -N with a key that has a note (e.g. 'd' = detach in prefix table)
    let out = tmux.run(&["list-keys", "-N", "d"]);
    assert!(!out.is_empty(), "-N d should produce output");

    // -N -T with key
    let out = tmux.run(&["list-keys", "-N", "-T", "prefix", "d"]);
    assert!(!out.is_empty());

    // -N with key that has no binding -> error
    let out = tmux.try_run(&["list-keys", "-N", "F12"]);
    assert!(!out.status.success(), "unbound key in -N mode should fail");
}

/// Test error paths: invalid key, non-existent table, unbound key.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn list_keys_errors() {
    let tmux = TmuxServer::new("list_keys_errors");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);

    // Invalid key name
    let out = tmux.try_run(&["list-keys", "NOTAKEY"]);
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("invalid key"),
        "invalid key error: {}",
        stderr
    );

    // Non-existent table
    let out = tmux.try_run(&["list-keys", "-T", "nonexistent_table"]);
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("doesn't exist"), "table error: {}", stderr);

    // Valid key that has no bindings -> "unknown key"
    let out = tmux.try_run(&["list-keys", "F12"]);
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("unknown key"),
        "unknown key error: {}",
        stderr
    );
}

/// Test list-commands: all, single, alias, -F format, and invalid command.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn list_commands() {
    let tmux = TmuxServer::new("list_commands");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);

    // All commands
    let out = tmux.run(&["list-commands"]);
    assert!(out.contains("list-keys"));
    assert!(out.contains("new-session"));

    // Single command
    let out = tmux.run(&["list-commands", "list-keys"]);
    assert!(out.contains("list-keys"));
    assert_eq!(out.lines().count(), 1, "single command = one line");

    // Alias resolution
    let out = tmux.run(&["list-commands", "lsk"]);
    assert!(out.contains("list-keys"));

    // -F custom format
    let out = tmux.run(&["list-commands", "-F", "#{command_list_name}"]);
    assert!(out.contains("list-keys"));
    // Name-only: no spaces in each line
    for line in out.lines() {
        assert!(
            !line.contains(' '),
            "name-only should have no spaces: {}",
            line
        );
    }

    // Invalid command -> error
    let out = tmux.try_run(&["list-commands", "not-a-real-command"]);
    assert!(!out.status.success());
}

/// Test -1 flag: show only first key binding.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn list_keys_first_only() {
    let tmux = TmuxServer::new("list_keys_1");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);

    // -1 in -N notes mode: should output only one line
    let out = tmux.run(&["list-keys", "-1", "-N"]);
    assert_eq!(out.lines().count(), 1, "-1 -N should show one line");

    // -1 in non-N mode without an attached client goes through
    // status_message_set which doesn't produce stdout, but exercises the branch.
    // Just verify it doesn't error out.
    let out = tmux.try_run(&["list-keys", "-1"]);
    assert!(out.status.success(), "-1 should succeed");
}

/// Test list-keys -N with prefix set to None, exercising the KEYC_NONE path.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn list_keys_no_prefix() {
    let tmux = TmuxServer::new("list_keys_noprefix");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);

    // Unset the prefix key so prefix == KEYC_NONE
    tmux.run(&["set", "-g", "prefix", "None"]);

    // -N without tablename: exercises KEYC_NONE path in get_prefix (xstrdup_(""))
    // and the `prefix != KEYC_NONE` false branch. Root table may have no noted
    // bindings, so output can be empty. Just verify it succeeds.
    let out = tmux.try_run(&["list-keys", "-N"]);
    assert!(out.status.success(), "-N with no prefix should succeed");

    // -N -a with no prefix: exercises the -a path in root-only mode.
    // Root table may have only mouse bindings (filtered out), so output can be empty.
    let out = tmux.try_run(&["list-keys", "-N", "-a"]);
    assert!(out.status.success(), "-N -a with no prefix should succeed");
}

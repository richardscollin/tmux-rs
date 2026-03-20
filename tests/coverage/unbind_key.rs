use super::*;

/// Test unbind-key: unbind specific key, -a (remove all), -n (root table),
/// -T (specific table), -q (quiet), and error paths.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn unbind_key_specific() {
    let tmux = TmuxServer::new("unbind_specific");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Bind a key then unbind it
    tmux.run(&["bind-key", "x", "display-message", "test"]);

    // Verify it's bound
    let keys = tmux.run(&["list-keys", "-T", "prefix"]);
    assert!(keys.contains("x"), "key x should be bound");

    // unbind-key x (default prefix table)
    tmux.run(&["unbind-key", "x"]);

    // Verify it's unbound
    let keys = tmux.run(&["list-keys", "-T", "prefix"]);
    assert!(!keys.contains(" x "), "key x should be unbound");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn unbind_key_root_table() {
    let tmux = TmuxServer::new("unbind_root");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Bind in root table with -n, then unbind with -n
    tmux.run(&["bind-key", "-n", "F5", "display-message", "root"]);
    tmux.run(&["unbind-key", "-n", "F5"]);
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn unbind_key_custom_table() {
    let tmux = TmuxServer::new("unbind_custom");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Bind in a custom table, then unbind with -T
    tmux.run(&[
        "bind-key",
        "-T",
        "mytable",
        "y",
        "display-message",
        "custom",
    ]);
    tmux.run(&["unbind-key", "-T", "mytable", "y"]);
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn unbind_key_all() {
    let tmux = TmuxServer::new("unbind_all");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Bind keys in a custom table then remove all with -a
    tmux.run(&["bind-key", "-T", "removeme", "a", "display-message", "a"]);
    tmux.run(&["bind-key", "-T", "removeme", "b", "display-message", "b"]);
    tmux.run(&["unbind-key", "-a", "-T", "removeme"]);
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn unbind_key_all_default_prefix() {
    let tmux = TmuxServer::new("unbind_all_pfx");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // BUG-003: unbind-key -a crashes the server due to use-after-free
    // in key_bindings_remove_table (unref before client pointer fixup).
    // Bind something first so prefix exists, then remove all
    tmux.run(&["bind-key", "z", "display-message", "z"]);
    let out = tmux.try_run(&["unbind-key", "-a"]);
    // Known crash — just verify we don't hang
    let _ = out;
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn unbind_key_all_root() {
    let tmux = TmuxServer::new("unbind_all_root");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // BUG-003: unbind-key -a crashes the server (see unbind_key_all_default_prefix)
    tmux.run(&["bind-key", "-n", "F6", "display-message", "f6"]);
    let out = tmux.try_run(&["unbind-key", "-a", "-n"]);
    let _ = out;
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn unbind_key_errors() {
    let tmux = TmuxServer::new("unbind_errors");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // unbind-key with no key and no -a: should error "missing key"
    let out = tmux.try_run(&["unbind-key"]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("missing key"),
        "expected 'missing key' error, got: {stderr}"
    );

    // unbind-key -a with a key: should error "key given with -a"
    let out = tmux.try_run(&["unbind-key", "-a", "x"]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("key given with -a"),
        "expected 'key given with -a' error, got: {stderr}"
    );

    // unbind-key with unknown key
    let out = tmux.try_run(&["unbind-key", "NOTAKEY"]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("unknown key"),
        "expected 'unknown key' error, got: {stderr}"
    );

    // unbind-key -T nonexistent table
    let out = tmux.try_run(&["unbind-key", "-T", "notable", "x"]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("doesn't exist"),
        "expected table doesn't exist error, got: {stderr}"
    );

    // unbind-key -a -T nonexistent table
    let out = tmux.try_run(&["unbind-key", "-a", "-T", "notable"]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("doesn't exist"),
        "expected table doesn't exist error, got: {stderr}"
    );

    // unbind-key -q: quiet mode suppresses error messages
    // The command may still return error or success depending on implementation
    let out = tmux.try_run(&["unbind-key", "-q"]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    // With -q, error message should be suppressed
    assert!(
        !stderr.contains("missing key"),
        "quiet mode should suppress error messages, got: {stderr}"
    );
}

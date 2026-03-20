use super::*;

/// Test key binding operations to exercise key_bindings_.rs paths.

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn bind_key_with_note() {
    let tmux = TmuxServer::new("kb_note");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // bind-key -N: bind with a note
    tmux.run(&["bind-key", "-N", "My note", "X", "display-message", "test"]);

    let keys = tmux.run(&["list-keys", "-T", "prefix", "-N"]);
    assert!(keys.contains("My note"), "note should appear in list-keys, got: {keys}");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn bind_key_repeat() {
    let tmux = TmuxServer::new("kb_repeat");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // bind-key -r: repeat key
    tmux.run(&["bind-key", "-r", "Y", "display-message", "repeat"]);

    let keys = tmux.run(&["list-keys", "-T", "prefix"]);
    assert!(keys.contains("Y"), "key Y should be bound");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn bind_key_rebind() {
    let tmux = TmuxServer::new("kb_rebind");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Bind, then rebind same key — note should be updated
    tmux.run(&["bind-key", "-N", "first", "Z", "display-message", "one"]);
    tmux.run(&["bind-key", "-N", "second", "Z", "display-message", "two"]);

    let keys = tmux.run(&["list-keys", "-T", "prefix", "-N"]);
    assert!(keys.contains("second"), "note should be updated, got: {keys}");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn bind_key_custom_table() {
    let tmux = TmuxServer::new("kb_custom");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Bind in custom table
    tmux.run(&["bind-key", "-T", "custom", "a", "display-message", "custom-a"]);
    tmux.run(&["bind-key", "-T", "custom", "b", "display-message", "custom-b"]);

    let keys = tmux.run(&["list-keys", "-T", "custom"]);
    assert!(keys.contains("a") && keys.contains("b"), "got: {keys}");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn bind_key_root_table() {
    let tmux = TmuxServer::new("kb_root");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Bind in root table with -n
    tmux.run(&["bind-key", "-n", "F7", "display-message", "f7"]);

    let keys = tmux.run(&["list-keys", "-T", "root"]);
    assert!(keys.contains("F7"), "got: {keys}");

    // Clean up
    tmux.run(&["unbind-key", "-n", "F7"]);
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn unbind_key_from_custom_table() {
    let tmux = TmuxServer::new("kb_unbindcust");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["bind-key", "-T", "temp", "x", "display-message", "x"]);
    tmux.run(&["unbind-key", "-T", "temp", "x"]);

    // Table may no longer exist after removing only binding
    let out = tmux.try_run(&["list-keys", "-T", "temp"]);
    let _ = out;
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn list_keys_all() {
    let tmux = TmuxServer::new("kb_listall");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // list-keys without -T: list all tables
    let out = tmux.run(&["list-keys"]);
    assert!(!out.is_empty());
    // Should include prefix table bindings
    assert!(out.contains("prefix"), "got first 200 chars: {}", &out[..out.len().min(200)]);
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn list_keys_with_notes_only() {
    let tmux = TmuxServer::new("kb_notes");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["bind-key", "-N", "test note", "W", "display-message", "w"]);

    // list-keys -N: show only keys with notes
    let out = tmux.run(&["list-keys", "-N"]);
    assert!(out.contains("test note"), "got: {out}");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn bind_multiple_commands() {
    let tmux = TmuxServer::new("kb_multi");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Bind key to multiple commands (via source-file with braces)
    let tmp = tmux.write_temp("bind-key V { set -g @multi1 yes ; set -g @multi2 yes }\n");
    tmux.run(&["source-file", tmp.path_str()]);

    let keys = tmux.run(&["list-keys", "-T", "prefix"]);
    assert!(keys.contains("V"), "got: {keys}");
}

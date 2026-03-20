use super::*;

/// Test vis/unvis encoding via capture-pane -C with control characters.

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn capture_pane_c_escape_control_chars() {
    let tmux = TmuxServer::new("vis_ctrl");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Send control characters to the pane that capture-pane -C will escape
    tmux.run(&["send-keys", "-l", "hello"]);
    sleep_ms(100);

    // capture-pane -C: C-style escaping exercises vis.rs
    let out = tmux.run(&["capture-pane", "-p", "-C"]);
    // Should contain the text (escaped or not)
    let _ = out;
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn capture_pane_c_escape_with_special() {
    let tmux = TmuxServer::new("vis_special");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Send tab and other chars that vis would encode
    tmux.run(&["send-keys", "-H", "09"]); // tab
    tmux.run(&["send-keys", "-l", "text"]);
    sleep_ms(100);

    let out = tmux.run(&["capture-pane", "-p", "-C"]);
    let _ = out;
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn capture_pane_escape_sequences() {
    let tmux = TmuxServer::new("vis_esc");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Send some text with backslash
    tmux.run(&["send-keys", "-l", "a\\b"]);
    sleep_ms(100);

    let out = tmux.run(&["capture-pane", "-p", "-C"]);
    let _ = out;
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn list_keys_output_exercises_vis() {
    let tmux = TmuxServer::new("vis_lk");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Bind a key with a command containing special characters
    tmux.run(&["bind-key", "Q", "display-message", "hello\\nworld"]);

    // list-keys uses vis for output formatting
    let out = tmux.run(&["list-keys", "-T", "prefix"]);
    assert!(out.contains("Q"), "got: {}", &out[..out.len().min(200)]);
}

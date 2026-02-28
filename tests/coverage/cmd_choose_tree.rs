use std::process::{Command, Stdio};

use super::*;

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn test_choose_tree() {
    let tmux = TmuxServer::new("choose_tree");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // choose-tree enters tree-mode
    tmux.run(&["choose-tree"]);
    assert_eq!(tmux.display("#{pane_mode}"), "tree-mode");

    // exercise cmd_choose_tree_args_parse by passing a template argument
    tmux.run_with_stdin(&["-C", "attach"], b"send-keys q\n");
    tmux.run(&["choose-tree", "kill-session"]);
    assert_eq!(tmux.display("#{pane_mode}"), "tree-mode");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn test_customize_mode() {
    let tmux = TmuxServer::new("customize_mode");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // customize-mode enters options-mode
    tmux.run(&["customize-mode"]);
    assert_eq!(tmux.display("#{pane_mode}"), "options-mode");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn test_choose_buffer() {
    let tmux = TmuxServer::new("choose_buffer");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // choose-buffer with empty paste buffer is a no-op (early return)
    tmux.run(&["choose-buffer"]);
    assert_eq!(tmux.display("#{pane_mode}"), "");

    // Add a paste buffer, then choose-buffer enters buffer-mode
    tmux.run(&["set-buffer", "hello"]);
    tmux.run(&["choose-buffer"]);
    assert_eq!(tmux.display("#{pane_mode}"), "buffer-mode");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn test_choose_client_no_clients() {
    let tmux = TmuxServer::new("choose_client_none");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // choose-client with no attached clients is a no-op (early return)
    tmux.run(&["choose-client"]);
    assert_eq!(tmux.display("#{pane_mode}"), "");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn test_choose_client_with_client() {
    let tmux = TmuxServer::new("choose_client_with");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Spawn a control-mode client that stays alive while we run choose-client.
    // The control client counts as an attached client so server_client_how_many() > 0.
    let binary = TmuxServer::binary_path();
    let socket = tmux.socket().to_string();
    let mut control_client = Command::new(binary)
        .args(["-L", &socket, "-C", "attach"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn control client");

    // Wait for the control client to fully attach
    for _ in 0..50 {
        let n = tmux.display("#{server_clients}");
        if n.trim() != "0" && !n.is_empty() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }

    // Now choose-client should see the attached control client
    tmux.run(&["choose-client"]);
    assert_eq!(tmux.display("#{pane_mode}"), "client-mode");

    // Clean up control client
    let _ = control_client.kill();
    let _ = control_client.wait();
}

use std::io::Write;
use std::process::{Command, Stdio};

use super::*;

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn test_display_panes() {
    let tmux = TmuxServer::new("display_panes");
    tmux.run(&["-f/dev/null", "new", "-d", "-x400", "-y200"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    // Long overlay duration so it doesn't time out during key testing
    tmux.run(&["set", "-g", "display-panes-time", "30000"]);

    // --- exec function paths ---

    // display-panes -b (background mode, no wait) via control client
    let output = tmux.run_with_stdin(&["-C", "attach"], b"display-panes -b\ndetach\n");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.contains("server exited"),
        "display-panes -b crashed: {}",
        stdout
    );

    // display-panes -b -N (no key handler)
    let output = tmux.run_with_stdin(&["-C", "attach"], b"display-panes -b -N\ndetach\n");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.contains("server exited"),
        "display-panes -b -N crashed: {}",
        stdout
    );

    // display-panes -b -d 100 (custom delay)
    let output = tmux.run_with_stdin(&["-C", "attach"], b"display-panes -b -d 100\ndetach\n");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.contains("server exited"),
        "display-panes -b -d 100 crashed: {}",
        stdout
    );

    // display-panes -d with invalid value
    let output = tmux.run_with_stdin(&["-C", "attach"], b"display-panes -d abc\ndetach\n");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("delay"),
        "expected error for invalid delay: {}",
        stdout
    );

    // display-panes with template argument (exercises args_parse)
    let output = tmux.run_with_stdin(
        &["-C", "attach"],
        b"display-panes -b \"select-pane -t '%%%'\"\ndetach\n",
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.contains("server exited"),
        "template test crashed: {}",
        stdout
    );

    // display-panes while overlay is already active (early return)
    let output = tmux.run_with_stdin(
        &["-C", "attach"],
        b"display-panes -b -d 5000\ndisplay-panes -b\ndetach\n",
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.contains("server exited"),
        "double overlay crashed: {}",
        stdout
    );

    // display-panes wait mode (no -b) - client disconnect triggers free callback
    let output = tmux.run_with_stdin(&["-C", "attach"], b"display-panes\n");
    assert!(
        !String::from_utf8_lossy(&output.stdout).contains("server exited"),
        "wait mode crashed"
    );

    // --- key handler paths ---
    // Create 11 panes total (indices 0-10) so alpha key 'a' (index 10) maps to a pane.
    for _ in 0..10 {
        tmux.run(&["split-window", "-d", "-v", "-l", "10"]);
    }

    let binary = TmuxServer::binary_path();
    let socket = tmux.socket().to_string();

    // Helper: start a control client, run display-panes, send a key via -K, clean up.
    let send_display_panes_key = |tmux: &TmuxServer, dp_cmd: &[u8], key: &str| {
        let mut cc = Command::new(binary)
            .args(["-L", &socket, "-C", "attach"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("failed to spawn control client");

        sleep_ms(200);

        let client_name = tmux
            .run(&["list-clients", "-F", "#{client_name}"])
            .trim()
            .to_string();

        let stdin = cc.stdin.as_mut().unwrap();
        stdin.write_all(dp_cmd).unwrap();
        stdin.flush().unwrap();
        sleep_ms(200);

        tmux.run(&["send-keys", "-K", "-c", &client_name, "-t", "0", key]);
        sleep_ms(200);

        let stdin = cc.stdin.as_mut().unwrap();
        let _ = stdin.write_all(b"detach\n");
        let _ = cc.wait();
    };

    // Alpha key 'a' with valid pane (index 10) in wait mode (item != null)
    send_display_panes_key(&tmux, b"display-panes\n", "a");

    // Alpha key 'z' with null pane (index 35 doesn't exist) -> return 1
    send_display_panes_key(&tmux, b"display-panes\n", "z");

    // Non-alpha, non-digit key '/' -> return -1
    send_display_panes_key(&tmux, b"display-panes\n", "/");

    // display-panes -b (item is null) + alpha key 'a' -> null-item path
    send_display_panes_key(&tmux, b"display-panes -b\n", "a");

    // Key with modifiers (C-a) -> return -1
    send_display_panes_key(&tmux, b"display-panes\n", "C-a");

    // Digit key '0' (KEYC_SENT prevents numeric match)
    send_display_panes_key(&tmux, b"display-panes\n", "0");
}

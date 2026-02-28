use std::process::{Command, Stdio};

use super::*;

/// Set up nested tmux (outer + inner) for interactive prompt testing.
/// The outer tmux has a pane running `inner attach`, so send-keys on the
/// outer delivers keystrokes through the inner client's key handler.
fn setup_nested() -> (TmuxServer, TmuxServer) {
    let binary = TmuxServer::binary_path();

    let inner = TmuxServer::new("confirm_inner");
    let outer = TmuxServer::new("confirm_outer");

    // Inner: detached session
    inner.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);

    // Outer: pane runs inner tmux attached
    let attach_cmd = format!("{} -L{} attach", binary, inner.socket());
    outer.run(&["-f/dev/null", "new", "-d", "-x80", "-y24", &attach_cmd]);
    sleep_secs(1);

    (outer, inner)
}

/// Run confirm-before on the inner server in a background thread.
/// The command targets the attached client (the one in the outer pane).
fn spawn_confirm(
    inner_socket: &str,
    extra_args: &[&str],
    command: &str,
) -> std::thread::JoinHandle<std::process::Output> {
    let binary = TmuxServer::binary_path().to_string();
    let socket = inner_socket.to_string();
    let mut args: Vec<String> = vec!["-L".into(), socket, "confirm-before".into()];
    for arg in extra_args {
        args.push(arg.to_string());
    }
    args.push(command.to_string());

    std::thread::spawn(move || {
        Command::new(&binary)
            .args(&args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .env("PATH", "/bin:/usr/bin:/usr/local/bin")
            .env("TERM", "screen")
            .output()
            .expect("failed to run confirm-before")
    })
}

/// Test exec paths via control mode (fast, no sleeps needed).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn confirm_before_exec_paths() {
    let tmux = TmuxServer::new("confirm_exec");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);

    // Basic confirm-before -b (background, default prompt, default 'y' key)
    let output = tmux.run_with_stdin(
        &["-C", "attach"],
        b"confirm-before -b 'set -g @x 1'\ndetach\n",
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.contains("server exited unexpectedly"),
        "server crashed: {stdout}"
    );

    // With -p custom prompt
    tmux.run_with_stdin(
        &["-C", "attach"],
        b"confirm-before -b -p 'Do it? ' 'set -g @x 1'\ndetach\n",
    );

    // With -c valid key
    tmux.run_with_stdin(
        &["-C", "attach"],
        b"confirm-before -b -c k 'set -g @x 1'\ndetach\n",
    );

    // With -y flag
    tmux.run_with_stdin(
        &["-C", "attach"],
        b"confirm-before -b -y 'set -g @x 1'\ndetach\n",
    );

    // Combine flags: -b -p -c -y
    tmux.run_with_stdin(
        &["-C", "attach"],
        b"confirm-before -b -y -c k -p 'Really? ' 'set -g @x 1'\ndetach\n",
    );

    // Invalid command syntax: args_make_commands_now returns null (cmdlist.is_null())
    tmux.run_with_stdin(
        &["-C", "attach"],
        b"confirm-before -b 'set -g @x'\ndetach\n",
    );

    // Server should still be alive
    let has = tmux.try_run(&["has-session"]);
    assert!(has.status.success());
}

/// Test invalid -c key triggers error.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn confirm_before_invalid_key() {
    let tmux = TmuxServer::new("confirm_badkey");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);

    // Multi-character key: should produce "invalid confirm key" error
    let output = tmux.run_with_stdin(
        &["-C", "attach"],
        b"confirm-before -c ab 'set -g @x 1'\ndetach\n",
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("invalid confirm key"),
        "expected error for multi-char key, got: {stdout}"
    );

    // Control character (0x01, below 32): covers *confirm_key > 31 false branch
    let output = tmux.run_with_stdin(
        &["-C", "attach"],
        b"confirm-before -c \x01 'set -g @x 1'\ndetach\n",
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("invalid confirm key"),
        "expected error for control char key, got: {stdout}"
    );

    // DEL character (0x7F, not < 127): covers *confirm_key < 127 false branch
    let output = tmux.run_with_stdin(
        &["-C", "attach"],
        b"confirm-before -c \x7f 'set -g @x 1'\ndetach\n",
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("invalid confirm key"),
        "expected error for DEL key, got: {stdout}"
    );

    // Server should still be alive
    let has = tmux.try_run(&["has-session"]);
    assert!(has.status.success());
}

/// Test blocking cancel path (client disconnect cancels the prompt).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn confirm_before_blocking_cancel() {
    let tmux = TmuxServer::new("confirm_cancel");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);

    // Without -b, confirm-before blocks (CMD_RETURN_WAIT).
    // EOF on stdin disconnects the control client -> prompt cleanup.
    let output = tmux.run_with_stdin(&["-C", "attach"], b"confirm-before 'set -g @x 1'\n");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.contains("server exited unexpectedly"),
        "server crashed: {stdout}"
    );

    sleep_ms(500);

    let has = tmux.try_run(&["has-session"]);
    assert!(has.status.success());
}

/// Test callback paths via nested tmux. The outer tmux sends keystrokes
/// through the pane to the inner attached client, exercising the
/// confirm-before prompt callback.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn confirm_before_callback() {
    let (outer, inner) = setup_nested();
    let inner_socket = inner.socket().to_string();

    // Test 1: Confirm with 'y' (default key, wait mode, item != null)
    {
        let handle = spawn_confirm(&inner_socket, &[], "set -g @test1 yes");
        sleep_ms(500);
        outer.run(&["send-keys", "y"]);
        let _ = handle.join();
        sleep_ms(200);
        assert_eq!(
            inner.display("#{@test1}"),
            "yes",
            "confirm with 'y' should execute command"
        );
    }

    // Test 2: Reject with wrong key (callback breaks out, command not run)
    {
        let handle = spawn_confirm(&inner_socket, &[], "set -g @test2 yes");
        sleep_ms(500);
        outer.run(&["send-keys", "n"]);
        let _ = handle.join();
        sleep_ms(200);
        assert_eq!(
            inner.display("#{@test2}"),
            "",
            "reject should not execute command"
        );
    }

    // Test 3: Default yes (-y) with Enter key
    {
        let handle = spawn_confirm(&inner_socket, &["-y"], "set -g @test3 yes");
        sleep_ms(500);
        outer.run(&["send-keys", "Enter"]);
        let _ = handle.join();
        sleep_ms(200);
        assert_eq!(
            inner.display("#{@test3}"),
            "yes",
            "Enter with -y should execute command"
        );
    }

    // Test 4: Custom confirm key (-c k)
    {
        let handle = spawn_confirm(&inner_socket, &["-c", "k"], "set -g @test4 yes");
        sleep_ms(500);
        outer.run(&["send-keys", "k"]);
        let _ = handle.join();
        sleep_ms(200);
        assert_eq!(
            inner.display("#{@test4}"),
            "yes",
            "custom key 'k' should execute command"
        );
    }

    // Test 5: Custom prompt (-p) with confirm
    {
        let handle = spawn_confirm(&inner_socket, &["-p", "Really? "], "set -g @test5 yes");
        sleep_ms(500);
        outer.run(&["send-keys", "y"]);
        let _ = handle.join();
        sleep_ms(200);
        assert_eq!(
            inner.display("#{@test5}"),
            "yes",
            "custom prompt with 'y' should execute command"
        );
    }

    // Test 6: Background mode (-b): item is null in callback, uses cmdq_append
    {
        inner.run(&["confirm-before", "-b", "set -g @test6 yes"]);
        sleep_ms(500);
        outer.run(&["send-keys", "y"]);
        sleep_ms(500);
        assert_eq!(
            inner.display("#{@test6}"),
            "yes",
            "background confirm should execute command"
        );
    }

    // Test 7: -y flag with wrong key (not Enter, not confirm key) -> reject
    {
        let handle = spawn_confirm(&inner_socket, &["-y"], "set -g @test7 yes");
        sleep_ms(500);
        outer.run(&["send-keys", "x"]);
        let _ = handle.join();
        sleep_ms(200);
        assert_eq!(
            inner.display("#{@test7}"),
            "",
            "-y with wrong key should not execute command"
        );
    }

    // Test 8: Enter without -y flag -> reject (covers !default_yes branch)
    {
        let handle = spawn_confirm(&inner_socket, &[], "set -g @test8 yes");
        sleep_ms(500);
        outer.run(&["send-keys", "Enter"]);
        let _ = handle.join();
        sleep_ms(200);
        assert_eq!(
            inner.display("#{@test8}"),
            "",
            "Enter without -y should not execute command"
        );
    }

    // Test 9: Escape cancels prompt (callback receives s=NULL)
    {
        let handle = spawn_confirm(&inner_socket, &[], "set -g @test9 yes");
        sleep_ms(500);
        outer.run(&["send-keys", "Escape"]);
        let _ = handle.join();
        sleep_ms(200);
        assert_eq!(
            inner.display("#{@test9}"),
            "",
            "Escape should cancel prompt without executing"
        );
    }
}

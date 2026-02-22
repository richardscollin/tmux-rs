use super::*;

/// Test if-shell config error reporting (translates if-shell-error.sh)
#[test]
fn if_shell_error() {
    let tmux = TmuxServer::new("if_shell_error");

    // Test 1: config with "if 'true' 'wibble wobble'" should produce a config error
    {
        let conf = tmux.write_temp("if 'true' 'wibble wobble'\n");
        let f_flag = format!("-f{}", conf.path_str());

        let output = tmux.run_with_stdin(&[&f_flag, "-C", "new"], b"");
        let stdout = String::from_utf8_lossy(&output.stdout);

        // Expected line: %config-error <path>:1: <path>:1: unknown command: wibble
        let expected_prefix = format!(
            "%config-error {}:1: {}:1: unknown command: wibble",
            conf.path_str(),
            conf.path_str()
        );
        assert!(
            stdout.lines().any(|l| l.starts_with(&expected_prefix)),
            "expected config error containing '{}', got:\n{}",
            expected_prefix,
            stdout
        );
    }

    tmux.kill_server();

    // Test 2: config with "wibble wobble", sourced via control client
    {
        let conf = tmux.write_temp("wibble wobble\n");
        let source_cmd = format!("source {}\n", conf.path_str());

        let output = tmux.run_with_stdin(&["-C", "new"], source_cmd.as_bytes());
        let stdout = String::from_utf8_lossy(&output.stdout);

        // Expected line: %config-error <path>:1: unknown command: wibble
        let expected_prefix = format!(
            "%config-error {}:1: unknown command: wibble",
            conf.path_str()
        );
        assert!(
            stdout.lines().any(|l| l.starts_with(&expected_prefix)),
            "expected config error containing '{}', got:\n{}",
            expected_prefix,
            stdout
        );
    }
}

/// Test nested if-shell with tmux run command (translates if-shell-nested.sh)
#[test]
fn if_shell_nested() {
    let binary = TmuxServer::binary_path();

    let tmux = TmuxServer::new("if_shell_nested");
    let tmux_cmd = format!("{} -L{}", binary, tmux.socket());

    // Config: if '$TMUX run "true"' 'set -s @done yes'
    let conf_content = format!("if '{} run \"true\"' 'set -s @done yes'\n", tmux_cmd);
    let conf = tmux.write_temp(&conf_content);

    // Output file for the pane command result
    let out = tmux.write_temp("");

    // The pane command: tmux show -vs @done >> out_path
    let pane_cmd = format!("{} show -vs @done >> {}", tmux_cmd, out.path_str());

    let f_flag = format!("-f{}", conf.path_str());
    tmux.run(&[&f_flag, "new", "-d", &pane_cmd]);
    sleep_secs(2);

    let content = out.read_to_string();
    let last_line = content.lines().last().unwrap_or("");
    assert_eq!(
        last_line, "yes",
        "if-shell nested command should set @done to yes"
    );

    // Server should have exited after the pane command finished
    let output = tmux.try_run(&["has"]);
    assert!(
        !output.status.success(),
        "server should have exited after pane closed"
    );
}

/// Test if-shell with TERM variable (translates if-shell-TERM.sh)
#[test]
fn if_shell_term() {
    let binary = TmuxServer::binary_path();

    let tmux = TmuxServer::new("if_shell_term");
    let socket = tmux.socket();

    // Config: set default-terminal based on $TERM
    let config = concat!(
        "if '[ \"$TERM\" = \"xterm\" ]' ",
        "'set -g default-terminal \"vt220\"' ",
        "'set -g default-terminal \"ansi\"'\n"
    );

    let conf = tmux.write_temp(config);
    let out = tmux.write_temp("");

    let f_flag = format!("-f{}", conf.path_str());

    // Test with TERM=xterm: should set default-terminal to vt220
    let pane_cmd = format!("echo \"#$TERM\" >> {}", out.path_str());
    let output = std::process::Command::new(binary)
        .args(["-L", socket, &f_flag, "new", "-d", &pane_cmd])
        .env("TERM", "xterm")
        .env("PATH", "/bin:/usr/bin:/usr/local/bin")
        .stdin(std::process::Stdio::null())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "new -d with TERM=xterm should succeed"
    );
    sleep_secs(2);

    let content = out.read_to_string();
    let last_line = content.lines().last().unwrap_or("");
    assert_eq!(
        last_line, "#vt220",
        "TERM=xterm should yield default-terminal=vt220"
    );

    // Kill server for next test
    tmux.kill_server();

    // Test with TERM=screen: should set default-terminal to ansi
    let pane_cmd = format!("echo \"#$TERM\" >> {}", out.path_str());
    let output = std::process::Command::new(binary)
        .args(["-L", socket, &f_flag, "new", "-d", &pane_cmd])
        .env("TERM", "screen")
        .env("PATH", "/bin:/usr/bin:/usr/local/bin")
        .stdin(std::process::Stdio::null())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "new -d with TERM=screen should succeed"
    );
    sleep_secs(2);

    let content = out.read_to_string();
    let last_line = content.lines().last().unwrap_or("");
    assert_eq!(
        last_line, "#ansi",
        "TERM=screen should yield default-terminal=ansi"
    );
}

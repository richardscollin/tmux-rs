use super::*;

/// Test base-index with new sessions (translates new-session-base-index.sh)
#[test]
fn new_session_base_index() {
    let tmux = TmuxServer::new("new_session_base_index");

    let conf = tmux.write_temp(
        "\
set -g base-index 100\n\
new\n\
set base-index 200\n\
neww\n",
    );

    let f_flag = format!("-f{}", conf.path_str());
    tmux.run(&[&f_flag, "start"]);

    let output = tmux.run(&["lsw", "-F", "#{window_index}"]);
    let indices: Vec<&str> = output.lines().collect();
    let joined = indices.join(" ");

    assert_eq!(joined, "100 200", "window indices should be 100 200");
}

/// Test new-session with various command forms (translates new-session-command.sh)
#[test]
fn new_session_command() {
    let tmux = TmuxServer::new("new_session_command");

    let conf = tmux.write_temp(
        "\
new sleep 101\n\
new -- sleep 102\n\
new \"sleep 103\"\n",
    );

    let f_flag = format!("-f{}", conf.path_str());
    tmux.run(&[&f_flag, "start"]);

    let output = tmux.run(&["ls"]);
    let session_count = output.lines().count();
    assert_eq!(session_count, 3, "should have 3 sessions");
}

/// Test new-session environment variable propagation (translates new-session-environment.sh)
#[test]
fn new_session_environment() {
    let tmux = TmuxServer::new("new_session_environment");
    let binary = TmuxServer::binary_path();

    // Get the default-terminal value (tmux overrides TERM inside panes).
    let term = tmux.run(&["start", ";", "show", "-gv", "default-terminal"]);
    let term = term.trim();
    tmux.kill_server();

    // Create the output file.
    let out = tmux.write_temp("");

    // Create the script that writes env vars to the output file.
    let script_content = format!(
        "\
(\n\
echo TERM=$TERM\n\
echo PWD=$(pwd)\n\
echo PATH=$PATH\n\
echo SHELL=$SHELL\n\
echo TEST=$TEST\n\
) >{}\n",
        out.path_str()
    );
    let script = tmux.write_temp(&script_content);

    // Create the tmux config that starts a new session running the script.
    let conf_content = format!("new -- /bin/sh {}\n", script.path_str());
    let conf = tmux.write_temp(&conf_content);

    // --- Sub-test 1 ---
    // Start tmux with cleared environment (env -i), from directory /.
    // Config file tells tmux to: new -- /bin/sh $SCRIPT
    let f_flag = format!("-f{}", conf.path_str());
    let status = Command::new(binary)
        .arg("-L")
        .arg(tmux.socket())
        .arg(&f_flag)
        .arg("start")
        .env_clear()
        .env("TERM", "ansi")
        .env("TEST", "test1")
        .env("PATH", "1")
        .env("SHELL", "/bin/sh")
        .current_dir("/")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .expect("failed to run tmux");
    assert!(status.success(), "sub-test 1: tmux start failed");

    sleep_secs(1);

    let expected1 = format!("TERM={term}\nPWD=/\nPATH=1\nSHELL=/bin/sh\nTEST=test1\n");
    assert_eq!(
        out.read_to_string(),
        expected1,
        "sub-test 1: environment mismatch"
    );

    // --- Sub-test 2 ---
    // Create a new detached session with the script as command, on the already-running server.
    let status = Command::new(binary)
        .arg("-L")
        .arg(tmux.socket())
        .arg(&f_flag)
        .arg("new")
        .arg("-d")
        .arg("--")
        .arg("/bin/sh")
        .arg(script.path_str())
        .env_clear()
        .env("TERM", "ansi")
        .env("TEST", "test2")
        .env("PATH", "2")
        .env("SHELL", "/bin/sh")
        .current_dir("/")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .expect("failed to run tmux");
    assert!(status.success(), "sub-test 2: tmux new -d failed");

    sleep_secs(1);

    let expected2 = format!("TERM={term}\nPWD=/\nPATH=2\nSHELL=/bin/sh\nTEST=test2\n");
    assert_eq!(
        out.read_to_string(),
        expected2,
        "sub-test 2: environment mismatch"
    );

    // --- Sub-test 3 ---
    // Start a new session with "source $TMP" as the pane command and -f/dev/null.
    // The pane command "source $TMP" tries to source the tmux config as shell script,
    // which fails because it contains tmux commands. So the output file is unchanged
    // from sub-test 2.
    let status = Command::new(binary)
        .arg("-L")
        .arg(tmux.socket())
        .arg("-f/dev/null")
        .arg("new")
        .arg("-d")
        .arg("source")
        .arg(conf.path_str())
        .env_clear()
        .env("TERM", "ansi")
        .env("TEST", "test3")
        .env("PATH", "3")
        .env("SHELL", "/bin/sh")
        .current_dir("/")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .expect("failed to run tmux");
    assert!(status.success(), "sub-test 3: tmux new -d source failed");

    sleep_secs(1);

    // Expected output is the same as sub-test 2 (source of tmux config as shell fails).
    let expected3 = format!("TERM={term}\nPWD=/\nPATH=2\nSHELL=/bin/sh\nTEST=test2\n");
    assert_eq!(
        out.read_to_string(),
        expected3,
        "sub-test 3: environment mismatch (should match sub-test 2)"
    );
}

/// Test new-session with no client attached (translates new-session-no-client.sh)
#[test]
fn new_session_no_client() {
    let tmux = TmuxServer::new("new_session_no_client");

    let conf = tmux.write_temp("new -stest\n");

    let f_flag = format!("-f{}", conf.path_str());
    tmux.run(&[&f_flag, "start"]);
    sleep_secs(1);

    let output = tmux.try_run(&["has", "-t=test:"]);
    assert!(
        output.status.success(),
        "session 'test' should exist after start"
    );
}

/// Test new-session default and explicit size (translates new-session-size.sh)
#[test]
fn new_session_size() {
    // Test 1: default size should be 80x24
    {
        let tmux = TmuxServer::new("new_session_size_1");

        tmux.run(&["-f/dev/null", "new", "-d"]);
        sleep_secs(1);

        let output = tmux.run(&["ls", "-F", "#{window_width} #{window_height}"]);
        assert_eq!(
            output.trim(),
            "80 24",
            "default session size should be 80x24"
        );
    }

    // Test 2: explicit size -x100 -y50
    {
        let tmux = TmuxServer::new("new_session_size_2");

        tmux.run(&["-f/dev/null", "new", "-d", "-x", "100", "-y", "50"]);
        sleep_secs(1);

        let output = tmux.run(&["ls", "-F", "#{window_width} #{window_height}"]);
        assert_eq!(
            output.trim(),
            "100 50",
            "session with -x100 -y50 should be 100x50"
        );
    }
}

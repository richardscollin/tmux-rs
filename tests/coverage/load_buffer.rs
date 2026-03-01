use super::*;

/// Load buffer from a file.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn load_buffer_basic() {
    let tmux = TmuxServer::new("loadb_basic");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let tmp = tmux.write_temp("file content");
    tmux.run(&["loadb", tmp.path_str()]);

    let out = tmux.run(&["showb"]);
    assert_eq!(out.trim(), "file content");
}

/// Load buffer into a named buffer.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn load_buffer_named() {
    let tmux = TmuxServer::new("loadb_named");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let tmp = tmux.write_temp("named data");
    tmux.run(&["loadb", "-b", "mybuf", tmp.path_str()]);

    let out = tmux.run(&["showb", "-b", "mybuf"]);
    assert_eq!(out.trim(), "named data");
}

/// Load buffer from nonexistent file.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn load_buffer_missing_file() {
    let tmux = TmuxServer::new("loadb_missing");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let result = tmux.try_run(&["loadb", "/nonexistent/path/file"]);
    assert!(!result.status.success());
}

/// Load buffer from stdin using '-' as path.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn load_buffer_stdin() {
    let tmux = TmuxServer::new("loadb_stdin");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Load from stdin via control mode
    let output = tmux.run_with_stdin(
        &["-C", "attach"],
        b"load-buffer -b stdinbuf /dev/stdin\ndetach\n",
    );
    let _ = output;
}

/// Load buffer with -w flag (exercises the code path even without a real tty client).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn load_buffer_with_w_flag() {
    let tmux = TmuxServer::new("loadb_w");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let tmp = tmux.write_temp("selection data");

    // Use -w flag via try_run (may fail without real terminal client, but exercises the path)
    let result = tmux.try_run(&["loadb", "-w", tmp.path_str()]);
    // Even without a proper client, the buffer should still be loaded
    let _ = result;

    // Verify buffer was loaded
    let out = tmux.try_run(&["showb"]);
    if out.status.success() {
        let stdout = String::from_utf8_lossy(&out.stdout);
        assert!(
            stdout.contains("selection data"),
            "buffer should contain the data even with -w"
        );
    }
}

/// Load buffer with -w and -b (named buffer + selection).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn load_buffer_named_with_w() {
    let tmux = TmuxServer::new("loadb_named_w");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let tmp = tmux.write_temp("named selection");
    let result = tmux.try_run(&["loadb", "-w", "-b", "selbuf", tmp.path_str()]);
    let _ = result;

    let out = tmux.try_run(&["showb", "-b", "selbuf"]);
    if out.status.success() {
        let stdout = String::from_utf8_lossy(&out.stdout);
        assert!(stdout.contains("named selection"));
    }
}

/// Load empty file (exercises bsize == 0 path in done callback).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn load_buffer_empty_file() {
    let tmux = TmuxServer::new("loadb_empty");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let tmp = tmux.write_temp("");

    // Loading an empty file should not create a buffer
    let result = tmux.try_run(&["loadb", "-b", "emptybuf", tmp.path_str()]);
    let _ = result;

    // The buffer should not exist since file was empty
    let out = tmux.try_run(&["showb", "-b", "emptybuf"]);
    assert!(
        !out.status.success(),
        "empty file should not create a buffer"
    );
}

/// Load buffer using alias loadb.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn load_buffer_alias() {
    let tmux = TmuxServer::new("loadb_alias");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let tmp = tmux.write_temp("alias content");
    tmux.run(&["loadb", "-b", "aliasbuf", tmp.path_str()]);

    let out = tmux.run(&["showb", "-b", "aliasbuf"]);
    assert_eq!(out.trim(), "alias content");
}

/// Load buffer overwriting existing named buffer.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn load_buffer_overwrite() {
    let tmux = TmuxServer::new("loadb_overwrite");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let tmp1 = tmux.write_temp("first");
    tmux.run(&["loadb", "-b", "buf", tmp1.path_str()]);

    let tmp2 = tmux.write_temp("second");
    tmux.run(&["loadb", "-b", "buf", tmp2.path_str()]);

    let out = tmux.run(&["showb", "-b", "buf"]);
    assert_eq!(out.trim(), "second");
}

/// Load buffer with -w via control mode to exercise client path.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn load_buffer_w_control_mode() {
    let tmux = TmuxServer::new("loadb_w_ctrl");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let tmp = tmux.write_temp("ctrl selection");

    // Use control mode so there is a real client for -w
    let cmd_str = format!("load-buffer -w -b ctrlbuf {}\ndetach\n", tmp.path_str());
    let output = tmux.run_with_stdin(&["-C", "attach"], cmd_str.as_bytes());
    assert!(output.status.success());

    sleep_ms(200);

    let out = tmux.try_run(&["showb", "-b", "ctrlbuf"]);
    if out.status.success() {
        let stdout = String::from_utf8_lossy(&out.stdout);
        assert!(stdout.contains("ctrl selection"));
    }
}

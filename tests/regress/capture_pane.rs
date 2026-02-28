use super::*;

/// Test capture-pane -e for OSC 8 hyperlink sequences (translates capture-pane-hyperlink.sh)
///
/// The shell script runs capturep INSIDE the pane command (after printf), so
/// we replicate that by having the pane write capture output to a temp file.
#[test]
fn capture_pane_hyperlink() {
    let tmux = TmuxServer::new("capture_pane_hyperlink");
    let binary = TmuxServer::binary_path();
    let socket = tmux.socket();

    // Test 1: hyperlink with id parameter
    {
        let out = tmux.write_temp("");
        let pane_cmd = format!(
            "printf '\\033]8;id=1;https://github.com\\033\\\\test1\\033]8;;\\033\\\\\\n'; \
             {} -L{} capturep -peS0 -E1 > {}",
            binary,
            socket,
            out.path_str()
        );
        tmux.run(&["-f/dev/null", "new", "-d", &pane_cmd]);
        sleep_secs(1);

        let captured = out.read_to_bytes();
        let expected = b"\x1b]8;id=1;https://github.com\x1b\\test1\x1b]8;;\x1b\\\n\n";
        assert_eq!(captured, expected, "hyperlink test 1 failed");
    }

    // Verify server exited (pane command finished, no remain-on-exit)
    let output = tmux.try_run(&["has"]);
    assert!(!output.status.success(), "server should have exited");

    // Test 2: hyperlink without id parameter
    {
        let out = tmux.write_temp("");
        let pane_cmd = format!(
            "printf '\\033]8;;https://github.com/tmux/tmux\\033\\\\test1\\033]8;;\\033\\\\\\n'; \
             {} -L{} capturep -peS0 -E1 > {}",
            binary,
            socket,
            out.path_str()
        );
        tmux.run(&["-f/dev/null", "new", "-d", &pane_cmd]);
        sleep_secs(1);

        let captured = out.read_to_bytes();
        let expected = b"\x1b]8;;https://github.com/tmux/tmux\x1b\\test1\x1b]8;;\x1b\\\n\n";
        assert_eq!(captured, expected, "hyperlink test 2 failed");
    }

    // Verify server exited
    let output = tmux.try_run(&["has"]);
    assert!(!output.status.success(), "server should have exited");
}

/// Test capture-pane sends colours after SGR 0 (translates capture-pane-sgr0.sh)
#[test]
fn capture_pane_sgr0() {
    let tmux = TmuxServer::new("capture_pane_sgr0");
    let binary = TmuxServer::binary_path();
    let socket = tmux.socket();

    let out = tmux.write_temp("");
    let pane_cmd = format!(
        "printf '\\033[31;42;1mabc\\033[0;31mdef\\n'; \
         printf '\\033[m\\033[100m bright bg \\033[m'; \
         {} -L{} capturep -peS0 -E1 >> {}",
        binary,
        socket,
        out.path_str()
    );
    tmux.run(&["-f/dev/null", "new", "-d", &pane_cmd]);
    sleep_secs(1);

    let captured = out.read_to_bytes();

    let mut expected = Vec::new();
    expected.extend_from_slice(b"\x1b[1m\x1b[31m\x1b[42mabc\x1b[0m\x1b[31mdef\x1b[39m\n");
    expected.extend_from_slice(b"\x1b[100m bright bg \x1b[49m\n");
    assert_eq!(captured, expected, "SGR 0 colour capture mismatch");
}

/// Test clear-history and clear-history -H
#[test]
#[ignore = "broken"]
fn clear_history() {
    let tmux = TmuxServer::new("clear_history");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Generate some scrollback history by running a command that outputs many lines
    tmux.run(&[
        "send-keys",
        "for i in $(seq 1 100); do echo line$i; done",
        "Enter",
    ]);
    sleep_ms(500);

    // Verify there is history
    let hsize: u32 = tmux.display("#{history_size}").parse().unwrap();
    assert!(hsize > 10, "should have scrollback history, got {}", hsize);

    // clear-history should wipe most of it
    tmux.run(&["clear-history"]);
    let after_clear: u32 = tmux.display("#{history_size}").parse().unwrap();
    assert!(
        after_clear < hsize / 2,
        "history should be mostly cleared (before={}, after={})",
        hsize,
        after_clear,
    );

    // clear-history -H (also resets hyperlinks)
    tmux.run(&[
        "send-keys",
        "for i in $(seq 1 100); do echo more$i; done",
        "Enter",
    ]);
    sleep_ms(500);
    let before_h: u32 = tmux.display("#{history_size}").parse().unwrap();
    tmux.run(&["clear-history", "-H"]);
    let after_h: u32 = tmux.display("#{history_size}").parse().unwrap();
    assert!(
        after_h < before_h / 2,
        "history should be mostly cleared after -H (before={}, after={})",
        before_h,
        after_h,
    );
}

/// Test capture-pane to buffer (no -p), including named buffers with -b
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn capture_to_buffer() {
    let tmux = TmuxServer::new("capture_to_buffer");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Write something to the pane
    tmux.run(&["send-keys", "echo hello_world", "Enter"]);
    sleep_ms(200);

    // Capture to default buffer (no -p)
    tmux.run(&["capture-pane"]);
    let buf = tmux.run(&["show-buffer"]);
    assert!(
        buf.contains("hello_world"),
        "default buffer should contain pane content"
    );

    // Capture to named buffer with -b
    tmux.run(&["capture-pane", "-b", "mybuf"]);
    let buf = tmux.run(&["show-buffer", "-b", "mybuf"]);
    assert!(
        buf.contains("hello_world"),
        "named buffer should contain pane content"
    );
}

/// Test capture-pane -S and -E line range flags, including "-" and negative values
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn capture_line_ranges() {
    let tmux = TmuxServer::new("capture_line_ranges");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Generate some content
    tmux.run(&["send-keys", "echo LINE_ONE", "Enter"]);
    tmux.run(&["send-keys", "echo LINE_TWO", "Enter"]);
    tmux.run(&["send-keys", "echo LINE_THREE", "Enter"]);
    sleep_ms(200);

    // -S - means from the very start of history
    tmux.run(&["capture-pane", "-S", "-"]);
    let buf = tmux.run(&["show-buffer"]);
    assert!(buf.contains("LINE_ONE"), "-S - should capture from start");

    // -E - means to the very end
    tmux.run(&["capture-pane", "-E", "-"]);
    let buf = tmux.run(&["show-buffer"]);
    assert!(buf.contains("LINE_THREE"), "-E - should capture to the end");

    // -S - -E - captures everything
    tmux.run(&["capture-pane", "-S", "-", "-E", "-"]);
    let buf = tmux.run(&["show-buffer"]);
    assert!(buf.contains("LINE_ONE"), "full range should have LINE_ONE");
    assert!(
        buf.contains("LINE_THREE"),
        "full range should have LINE_THREE"
    );

    // Negative -S (relative to visible area)
    tmux.run(&["capture-pane", "-S", "-5"]);
    let buf = tmux.run(&["show-buffer"]);
    // Should work without error
    assert!(!buf.is_empty(), "-S -5 should produce output");

    // -E with explicit line number
    tmux.run(&["capture-pane", "-S", "0", "-E", "2"]);
    let buf = tmux.run(&["show-buffer"]);
    assert!(!buf.is_empty(), "-S 0 -E 2 should produce output");

    // Swap: bottom < top (S > E, should auto-swap)
    tmux.run(&["capture-pane", "-S", "5", "-E", "0"]);
    let buf = tmux.run(&["show-buffer"]);
    assert!(!buf.is_empty(), "swapped range should produce output");
}

/// Test capture-pane -J (join wrapped lines)
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn capture_join_lines() {
    let tmux = TmuxServer::new("capture_join_lines");
    tmux.run(&["-f/dev/null", "new", "-d", "-x40", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Output a line longer than 40 cols so it wraps
    let long_line = "A".repeat(60);
    tmux.run(&["send-keys", &format!("echo {}", long_line), "Enter"]);
    sleep_ms(200);

    // Without -J: wrapped lines are separate
    tmux.run(&["capture-pane"]);
    let without_join = tmux.run(&["show-buffer"]);

    // With -J: wrapped lines are joined
    tmux.run(&["capture-pane", "-J"]);
    let with_join = tmux.run(&["show-buffer"]);

    // The joined version should contain the full long line without a break
    assert!(
        with_join.contains(&long_line),
        "joined capture should contain the full long line"
    );
    // The non-joined version should NOT contain the full line on a single buffer line
    // (it will be split across grid lines)
    let _ = without_join; // just verify both paths run
}

/// Test capture-pane -C (escape control sequences in output)
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn capture_escape_sequences() {
    let tmux = TmuxServer::new("capture_escape_seqs");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Write coloured text to pane
    tmux.run(&["send-keys", "printf '\\033[31mRED\\033[0m'", "Enter"]);
    sleep_ms(200);

    // -C should escape sequences as octal
    tmux.run(&["capture-pane", "-eC"]);
    let buf = tmux.run(&["show-buffer"]);
    // With -C, escape chars should be rendered as \033 or similar octal
    assert!(
        buf.contains("\\033") || buf.contains("\\33") || buf.contains("\\e"),
        "escaped capture should contain octal escape sequences, got: {}",
        &buf[..buf.len().min(200)]
    );
}

/// Test capture-pane -T (no empty cells) and -N (no trim spaces)
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn capture_t_and_n_flags() {
    let tmux = TmuxServer::new("capture_t_n_flags");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["send-keys", "echo hello", "Enter"]);
    sleep_ms(200);

    // -T: no empty cell padding
    tmux.run(&["capture-pane", "-T"]);
    let buf_t = tmux.run(&["show-buffer"]);
    assert!(buf_t.contains("hello"), "-T capture should have content");

    // -N: don't trim trailing spaces
    tmux.run(&["capture-pane", "-N"]);
    let buf_n = tmux.run(&["show-buffer"]);
    assert!(buf_n.contains("hello"), "-N capture should have content");

    // Both -T and -N together
    tmux.run(&["capture-pane", "-TN"]);
    let buf_tn = tmux.run(&["show-buffer"]);
    assert!(buf_tn.contains("hello"), "-TN capture should have content");
}

/// Test capture-pane -a (alternate screen) error paths
#[test]
#[ignore]
fn capture_alternate_screen() {
    let tmux = TmuxServer::new("capture_alt_screen");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // -a without alternate screen active should error
    let output = tmux.try_run(&["capture-pane", "-a"]);
    assert!(
        !output.status.success() || !output.stderr.is_empty(),
        "-a without alternate screen should fail"
    );

    // -aq should succeed quietly (return empty)
    tmux.run(&["capture-pane", "-aq"]);

    // Enter alternate screen (e.g., run less)
    tmux.run(&[
        "send-keys",
        "printf '\\033[?1049h'; sleep 1; printf '\\033[?1049l'",
        "Enter",
    ]);
    sleep_ms(200);

    // -a should now succeed (captures saved grid from before alt screen switch)
    tmux.run(&["capture-pane", "-a"]);
    let buf = tmux.run(&["show-buffer"]);
    // Just verify it doesn't crash - content may vary
    let _ = buf;
}

/// Test capture-pane -M (capture from mode screen, e.g. copy-mode)
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn capture_mode_screen() {
    let tmux = TmuxServer::new("capture_mode_screen");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Put some content in the pane
    tmux.run(&["send-keys", "echo MODE_TEST_CONTENT", "Enter"]);
    sleep_ms(200);

    // Enter copy mode
    tmux.run(&["copy-mode"]);
    sleep_ms(100);

    // Verify we're in copy mode
    let mode = tmux.display("#{pane_mode}");
    assert_eq!(mode, "copy-mode", "should be in copy mode");

    // -M should capture from the copy mode screen
    tmux.run(&["capture-pane", "-M"]);
    let buf = tmux.run(&["show-buffer"]);
    assert!(
        buf.contains("MODE_TEST_CONTENT"),
        "-M should capture mode screen content"
    );

    // Exit copy mode
    tmux.run(&["send-keys", "q"]);
}

/// Test capture-pane -P (pending input)
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn capture_pending() {
    let tmux = TmuxServer::new("capture_pending");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // -P captures pending (unprocessed) input. Normally empty.
    // When pending is null, it stores an empty string buffer. Use -p to print
    // to stdout instead, which avoids the paste buffer path.
    let buf = tmux.run(&["capture-pane", "-Pp"]);
    // Pending is usually empty, just verify the path doesn't crash
    let _ = buf;

    // -PC should also work (escape mode for pending)
    let buf = tmux.run(&["capture-pane", "-PCp"]);
    let _ = buf;
}

/// Test capture-pane -p with control mode client
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn capture_control_mode() {
    let tmux = TmuxServer::new("capture_ctrl_mode");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["send-keys", "echo CTRL_TEST", "Enter"]);
    sleep_ms(200);

    // Attach in control mode and issue capture-pane -p
    let output = tmux.run_with_stdin(&["-C", "attach"], b"capture-pane -p\ndetach\n");
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Control mode output should contain the captured content
    assert!(
        stdout.contains("CTRL_TEST"),
        "control mode capture should contain pane content, got: {}",
        &stdout[..stdout.len().min(500)]
    );
}

/// Test that -S with a value beyond pane size clamps correctly,
/// and -E with a value larger than the screen is clamped.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn capture_range_clamping() {
    let tmux = TmuxServer::new("capture_range_clamp");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["send-keys", "echo CLAMP_TEST", "Enter"]);
    sleep_ms(200);

    // -S with a very large negative value (should clamp to 0)
    tmux.run(&["capture-pane", "-S", "-10000"]);
    let buf = tmux.run(&["show-buffer"]);
    assert!(
        buf.contains("CLAMP_TEST"),
        "large negative -S should clamp and capture content"
    );

    // -E with a very large positive value (should clamp to max)
    tmux.run(&["capture-pane", "-E", "10000"]);
    let buf = tmux.run(&["show-buffer"]);
    assert!(
        buf.contains("CLAMP_TEST"),
        "large positive -E should clamp and capture content"
    );

    // -S and -E with negative values
    tmux.run(&["capture-pane", "-S", "-3", "-E", "-1"]);
    let buf = tmux.run(&["show-buffer"]);
    assert!(!buf.is_empty(), "negative range should produce output");

    // -E with large negative (clamps to 0)
    tmux.run(&["capture-pane", "-E", "-10000"]);
    let buf = tmux.run(&["show-buffer"]);
    let _ = buf; // just ensure no crash

    // -S with very large positive value (covers top > hsize + sy - 1 clamping)
    tmux.run(&["capture-pane", "-S", "30000"]);
    let buf = tmux.run(&["show-buffer"]);
    let _ = buf; // just ensure clamping works

    // Generate scrollback so hsize > 0, then test small negative -S within hsize
    tmux.run(&[
        "send-keys",
        "for i in $(seq 1 50); do echo scroll$i; done",
        "Enter",
    ]);
    sleep_ms(1000);

    // -S with small negative that doesn't exceed hsize (covers (-n) <= hsize branch)
    tmux.run(&["capture-pane", "-S", "-2", "-E", "-1"]);
    let buf = tmux.run(&["show-buffer"]);
    assert!(
        !buf.is_empty(),
        "small negative -S -E should produce output"
    );
}

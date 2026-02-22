use super::*;

/// Ported from utf8-test.sh
///
/// Displays UTF-8-test.txt via cat in a pane, then captures the pane output
/// and compares it byte-for-byte with the expected result file.
#[test]
fn utf8_test() {
    let tmux = TmuxServer::new("utf8_test");

    let test_file = TmuxServer::regress_dir().join("UTF-8-test.txt");
    let expected_file = TmuxServer::regress_dir().join("utf8-test.result");

    tmux.run(&[
        "-f/dev/null",
        "set",
        "-g",
        "remain-on-exit",
        "on",
        ";",
        "set",
        "-g",
        "remain-on-exit-format",
        "",
        ";",
        "new",
        "-d",
        "--",
        "cat",
        test_file.to_str().unwrap(),
    ]);
    sleep_secs(3);

    let captured = tmux.run_bytes(&["capturep", "-pCeJS-"]);
    let expected = std::fs::read(&expected_file)
        .unwrap_or_else(|e| panic!("failed to read {}: {}", expected_file.display(), e));

    assert_eq!(
        captured,
        expected,
        "captured pane output does not match {}",
        expected_file.display()
    );

    tmux.kill_server();
}

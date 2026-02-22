use super::*;

/// Test combining characters and wide emoji rendering (translates combine-test.sh)
///
/// Runs printf commands that position the cursor and write combining characters,
/// emoji with skin tone modifiers, ZWJ sequences, and flag emoji. Then captures
/// the pane with `capturep -pe` and compares against the expected result.
#[test]
fn combine_test() {
    let tmux = TmuxServer::new("combine_test");
    let binary = TmuxServer::binary_path();
    let socket = tmux.socket();

    let out = tmux.write_temp("");

    // Build the pane command: a series of printf calls followed by capturep.
    // The octal escapes are UTF-8 byte sequences for combining/emoji characters.
    // These must be interpreted by the shell's printf, so we use shell escapes.
    let pane_cmd = format!(
        concat!(
            "printf '\\033[H\\033[J'; ",
            "printf '\\033[3;1H\\316\\233\\033[3;1H\\314\\2120\\n'; ",
            "printf '\\033[4;1H\\316\\233\\033[4;2H\\314\\2121\\n'; ",
            "printf '\\033[5;1H\\360\\237\\221\\215\\033[5;1H\\360\\237\\217\\2732\\n'; ",
            "printf '\\033[6;1H\\360\\237\\221\\215\\033[6;3H\\360\\237\\217\\2733\\n'; ",
            "printf '\\033[7;1H\\360\\237\\221\\215\\033[7;10H\\360\\237\\221\\215\\033[7;3H\\360\\237\\217\\273\\033[7;12H\\360\\237\\217\\2734\\n'; ",
            "printf '\\033[8;1H\\360\\237\\244\\267\\342\\200\\215\\342\\231\\202\\357\\270\\2175\\n'; ",
            "printf '\\033[9;1H\\360\\237\\244\\267\\033[9;1H\\342\\200\\215\\342\\231\\202\\357\\270\\2176\\n'; ",
            "printf '\\033[9;1H\\360\\237\\244\\267\\033[9;1H\\342\\200\\215\\342\\231\\202\\357\\270\\2177\\n'; ",
            "printf '\\033[10;1H\\360\\237\\244\\267\\033[10;3H\\342\\200\\215\\342\\231\\202\\357\\270\\2178\\n'; ",
            "printf '\\033[11;1H\\360\\237\\244\\267\\033[11;3H\\342\\200\\215\\033[11;3H\\342\\231\\202\\357\\270\\2179\\n'; ",
            "printf '\\033[12;1H\\360\\237\\244\\267\\033[12;3H\\342\\200\\215\\342\\231\\202\\357\\270\\21710\\n'; ",
            "printf '\\033[13;1H\\360\\237\\207\\25211\\n'; ",
            "printf '\\033[14;1H\\360\\237\\207\\270\\360\\237\\207\\25212\\n'; ",
            "printf '\\033[15;1H\\360\\237\\207\\270  \\010\\010\\360\\237\\207\\25213\\n'; ",
            "{} -L{} capturep -pe >> {}",
        ),
        binary,
        socket,
        out.path_str()
    );
    tmux.run(&["-f/dev/null", "new", "-d", &pane_cmd]);
    sleep_secs(1);

    let captured = out.read_to_bytes();
    let expected = std::fs::read(TmuxServer::regress_dir().join("combine-test.result"))
        .expect("failed to read combine-test.result");

    assert_eq!(captured, expected, "combine test output mismatch");

    // Verify server exited (pane command finished, no remain-on-exit)
    let output = tmux.try_run(&["has"]);
    assert!(!output.status.success(), "server should have exited");
}

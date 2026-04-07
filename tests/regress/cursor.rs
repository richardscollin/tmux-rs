use super::*;

/// Replicate awk '{print NR-1,$0}' on the output of capturep -p.
/// Each line is prefixed with its 0-based line number and a space.
fn awk_number_lines(text: &str) -> String {
    let mut result = String::new();
    for (i, line) in text.lines().enumerate() {
        result.push_str(&format!("{} {}\n", i, line));
    }
    result
}

/// Capture cursor state: display cursor position/character, then capture pane
/// content with numbered lines. Appends both to `output`, matching the shell
/// pattern: display -pF >> TMP ; capturep -p | awk >> TMP
fn capture_cursor_state(tmux: &TmuxServer, output: &mut String) {
    let display = tmux.run(&[
        "display",
        "-pF",
        "#{cursor_x} #{cursor_y} #{cursor_character}",
    ]);
    output.push_str(&display);
    let pane = tmux.run(&["capturep", "-p"]);
    output.push_str(&awk_number_lines(&pane));
}

/// cursor-test1: 40x10 pane, cursor at (14,8), resize to 10 then 50
/// (translates cursor-test1.sh)
#[test]
fn cursor_test1() {
    let tmux = TmuxServer::new("cursor_test1");
    let regress = test_data_dir();
    let txt = regress.join("cursor-test.txt");
    let txt_path = txt.to_str().unwrap();

    let pane_cmd = format!("cat {}; printf '\\e[9;15H'; cat", txt_path);
    tmux.run(&["-f/dev/null", "new", "-d", "-x40", "-y10", &pane_cmd]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    sleep_secs(1);

    let mut output = String::new();

    // Initial state at 40 columns
    capture_cursor_state(&tmux, &mut output);

    // Resize to 10 columns
    tmux.run(&["resizew", "-x10"]);
    capture_cursor_state(&tmux, &mut output);

    // Resize to 50 columns
    tmux.run(&["resizew", "-x50"]);
    capture_cursor_state(&tmux, &mut output);

    let expected = std::fs::read(regress.join("cursor-test1.result")).unwrap();
    assert_eq!(output.as_bytes(), &expected[..], "cursor-test1 mismatch");
}

/// cursor-test2: 10x10 pane, cursor at (9,7), resize to 5 then 50
/// (translates cursor-test2.sh)
#[test]
fn cursor_test2() {
    let tmux = TmuxServer::new("cursor_test2");
    let regress = test_data_dir();
    let txt = regress.join("cursor-test.txt");
    let txt_path = txt.to_str().unwrap();

    let pane_cmd = format!("cat {}; printf '\\e[8;10H'; cat", txt_path);
    tmux.run(&["-f/dev/null", "new", "-d", "-x10", "-y10", &pane_cmd]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    sleep_secs(1);

    let mut output = String::new();

    // Initial state at 10 columns
    capture_cursor_state(&tmux, &mut output);

    // Resize to 5 columns
    tmux.run(&["resizew", "-x5"]);
    capture_cursor_state(&tmux, &mut output);

    // Resize to 50 columns
    tmux.run(&["resizew", "-x50"]);
    capture_cursor_state(&tmux, &mut output);

    let expected = std::fs::read(regress.join("cursor-test2.result")).unwrap();
    assert_eq!(output.as_bytes(), &expected[..], "cursor-test2 mismatch");
}

/// cursor-test3: 7x2 pane with wrapped text, cursor at (6,1), resize to 5 then 7
/// (translates cursor-test3.sh)
#[test]
fn cursor_test3() {
    let tmux = TmuxServer::new("cursor_test3");
    let regress = test_data_dir();

    let pane_cmd = "printf 'abcdefabcdefab'; printf '\\e[2;7H'; cat";
    tmux.run(&["-f/dev/null", "new", "-d", "-x7", "-y2", pane_cmd]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    sleep_secs(1);

    let mut output = String::new();

    // Initial state at 7 columns
    capture_cursor_state(&tmux, &mut output);

    // Resize to 5 columns
    tmux.run(&["resizew", "-x5"]);
    capture_cursor_state(&tmux, &mut output);

    // Resize back to 7 columns
    tmux.run(&["resizew", "-x7"]);
    capture_cursor_state(&tmux, &mut output);

    let expected = std::fs::read(regress.join("cursor-test3.result")).unwrap();
    assert_eq!(output.as_bytes(), &expected[..], "cursor-test3 mismatch");
}

/// cursor-test4: 10x3 pane, resize to 20, 3, then 10
/// (translates cursor-test4.sh)
#[test]
fn cursor_test4() {
    let tmux = TmuxServer::new("cursor_test4");
    let regress = test_data_dir();

    let pane_cmd = "printf 'abcdef\\n'; cat";
    tmux.run(&["-f/dev/null", "new", "-d", "-x10", "-y3", pane_cmd]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    sleep_secs(1);

    let mut output = String::new();

    // Initial state at 10 columns
    capture_cursor_state(&tmux, &mut output);

    // Resize to 20 columns
    tmux.run(&["resizew", "-x20"]);
    capture_cursor_state(&tmux, &mut output);

    // Resize to 3 columns
    tmux.run(&["resizew", "-x3"]);
    capture_cursor_state(&tmux, &mut output);

    // Resize back to 10 columns
    tmux.run(&["resizew", "-x10"]);
    capture_cursor_state(&tmux, &mut output);

    let expected = std::fs::read(regress.join("cursor-test4.result")).unwrap();
    assert_eq!(output.as_bytes(), &expected[..], "cursor-test4 mismatch");
}

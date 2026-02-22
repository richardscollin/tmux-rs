use std::path::PathBuf;
use std::process::{Command, Stdio};

use super::*;

/// Send a raw key sequence through the outer tmux to the inner tmux's
/// command-prompt, which captures the key name and stores it in a server
/// option. Then read the option and compare to expected.
///
/// The shell script does:
///   $TMUX2 command-prompt -k 'display-message -pl "%%"' > $TMP &
///   sleep 0.05
///   $TMUX send-keys $keys
///   wait
///
/// We replicate this by spawning the command-prompt call in a background
/// thread (it may block until a key is pressed), but instead of capturing
/// stdout (which goes to the attached client, not our process), we store
/// the key name in a server option and read it afterwards.
fn assert_key(tmux_outer: &TmuxServer, inner_socket: &str, keys: &str, expected: &str) {
    let binary = PathBuf::from(TmuxServer::binary_path());

    // Use single quotes unless the key name itself contains a single quote
    let callback = if expected.contains('\'') {
        "set -g @lastkey \"%%\"".to_string()
    } else {
        "set -g @lastkey '%%'".to_string()
    };

    // Clear previous key
    let _ = Command::new(&binary)
        .args(["-L", inner_socket, "set", "-g", "@lastkey", ""])
        .stdin(Stdio::null())
        .env("TERM", "screen")
        .output();

    let socket = inner_socket.to_string();
    let binary_clone = binary.clone();
    let callback_owned = callback.clone();
    let handle = std::thread::spawn(move || {
        Command::new(&binary_clone)
            .args(["-L", &socket, "command-prompt", "-k", &callback_owned])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .env("PATH", "/bin:/usr/bin:/usr/local/bin")
            .env("TERM", "screen")
            .output()
    });

    sleep_ms(200);

    // Split keys string into separate arguments for send-keys
    let key_args: Vec<&str> = keys.split_whitespace().collect();
    let mut args = vec!["send-keys"];
    args.extend(&key_args);
    tmux_outer.run(&args);

    sleep_ms(300);

    // Wait for command-prompt to complete
    let _ = handle.join();

    // Read the captured key name from the inner server
    let read_key = || -> String {
        let out = Command::new(&binary)
            .args(["-L", inner_socket, "display-message", "-p", "#{@lastkey}"])
            .stdin(Stdio::null())
            .env("PATH", "/bin:/usr/bin:/usr/local/bin")
            .env("TERM", "screen")
            .output()
            .expect("failed to read @lastkey");
        String::from_utf8_lossy(&out.stdout).trim().to_string()
    };

    let mut actual = read_key();
    // Retry once if empty (timing issue)
    if actual.is_empty() {
        sleep_ms(500);
        actual = read_key();
    }

    assert_eq!(
        actual, expected,
        "tty-keys: keys='{}' expected='{}', got='{}'",
        keys, expected, actual
    );
}

/// Ported from tty-keys.sh
///
/// Uses two tmux servers: outer (tmux) runs inner (tmux2) attached.
/// Raw key sequences are sent through the outer tmux, and the inner
/// tmux's command-prompt captures the interpreted key name.
#[test]
#[cfg_attr(not(feature = "slow-tests"), ignore)]
fn tty_keys() {
    let binary = TmuxServer::binary_path();

    let tmux2 = TmuxServer::new("tty_keys2");
    let tmux = TmuxServer::new("tty_keys");

    // Inner server: detached session
    tmux2.run(&["-f/dev/null", "new", "-d"]);

    // Outer server: runs inner tmux attached
    let attach_cmd = format!("{} -L{} attach", binary, tmux2.socket());
    tmux.run(&["-f/dev/null", "new", "-d", &attach_cmd]);
    sleep_secs(1);

    let inner = tmux2.socket();

    // Basic keys 0x00 - 0x7F
    assert_key(&tmux, inner, "0x00", "C-Space");
    // assert_key(&tmux, inner, "Escape 0x00", "C-M-Space"); // commented in shell

    assert_key(&tmux, inner, "0x01", "C-a");
    assert_key(&tmux, inner, "Escape 0x01", "C-M-a");

    assert_key(&tmux, inner, "0x02", "C-b");
    assert_key(&tmux, inner, "Escape 0x02", "C-M-b");

    assert_key(&tmux, inner, "0x03", "C-c");
    assert_key(&tmux, inner, "Escape 0x03", "C-M-c");

    assert_key(&tmux, inner, "0x04", "C-d");
    assert_key(&tmux, inner, "Escape 0x04", "C-M-d");

    assert_key(&tmux, inner, "0x05", "C-e");
    assert_key(&tmux, inner, "Escape 0x05", "C-M-e");

    assert_key(&tmux, inner, "0x06", "C-f");
    assert_key(&tmux, inner, "Escape 0x06", "C-M-f");

    assert_key(&tmux, inner, "0x07", "C-g");
    assert_key(&tmux, inner, "Escape 0x07", "C-M-g");

    assert_key(&tmux, inner, "0x08", "C-h");
    assert_key(&tmux, inner, "Escape 0x08", "C-M-h");

    assert_key(&tmux, inner, "0x09", "Tab");
    assert_key(&tmux, inner, "Escape 0x09", "M-Tab");

    assert_key(&tmux, inner, "0x0A", "C-j");
    assert_key(&tmux, inner, "Escape 0x0A", "C-M-j");

    assert_key(&tmux, inner, "0x0B", "C-k");
    assert_key(&tmux, inner, "Escape 0x0B", "C-M-k");

    assert_key(&tmux, inner, "0x0C", "C-l");
    assert_key(&tmux, inner, "Escape 0x0C", "C-M-l");

    assert_key(&tmux, inner, "0x0D", "Enter");
    assert_key(&tmux, inner, "Escape 0x0D", "M-Enter");

    assert_key(&tmux, inner, "0x0E", "C-n");
    assert_key(&tmux, inner, "Escape 0x0E", "C-M-n");

    assert_key(&tmux, inner, "0x0F", "C-o");
    assert_key(&tmux, inner, "Escape 0x0F", "C-M-o");

    assert_key(&tmux, inner, "0x10", "C-p");
    assert_key(&tmux, inner, "Escape 0x10", "C-M-p");

    assert_key(&tmux, inner, "0x11", "C-q");
    assert_key(&tmux, inner, "Escape 0x11", "C-M-q");

    assert_key(&tmux, inner, "0x12", "C-r");
    assert_key(&tmux, inner, "Escape 0x12", "C-M-r");

    assert_key(&tmux, inner, "0x13", "C-s");
    assert_key(&tmux, inner, "Escape 0x13", "C-M-s");

    assert_key(&tmux, inner, "0x14", "C-t");
    assert_key(&tmux, inner, "Escape 0x14", "C-M-t");

    assert_key(&tmux, inner, "0x15", "C-u");
    assert_key(&tmux, inner, "Escape 0x15", "C-M-u");

    assert_key(&tmux, inner, "0x16", "C-v");
    assert_key(&tmux, inner, "Escape 0x16", "C-M-v");

    assert_key(&tmux, inner, "0x17", "C-w");
    assert_key(&tmux, inner, "Escape 0x17", "C-M-w");

    assert_key(&tmux, inner, "0x18", "C-x");
    assert_key(&tmux, inner, "Escape 0x18", "C-M-x");

    assert_key(&tmux, inner, "0x19", "C-y");
    assert_key(&tmux, inner, "Escape 0x19", "C-M-y");

    assert_key(&tmux, inner, "0x1A", "C-z");
    assert_key(&tmux, inner, "Escape 0x1A", "C-M-z");

    assert_key(&tmux, inner, "0x1B", "Escape");
    assert_key(&tmux, inner, "Escape 0x1B", "M-Escape");

    assert_key(&tmux, inner, "0x1C", "C-\\");
    assert_key(&tmux, inner, "Escape 0x1C", "C-M-\\");

    assert_key(&tmux, inner, "0x1D", "C-]");
    assert_key(&tmux, inner, "Escape 0x1D", "C-M-]");

    assert_key(&tmux, inner, "0x1E", "C-^");
    assert_key(&tmux, inner, "Escape 0x1E", "C-M-^");

    assert_key(&tmux, inner, "0x1F", "C-_");
    assert_key(&tmux, inner, "Escape 0x1F", "C-M-_");

    assert_key(&tmux, inner, "0x20", "Space");
    assert_key(&tmux, inner, "Escape 0x20", "M-Space");

    assert_key(&tmux, inner, "0x21", "!");
    assert_key(&tmux, inner, "Escape 0x21", "M-!");

    assert_key(&tmux, inner, "0x22", "\"");
    assert_key(&tmux, inner, "Escape 0x22", "M-\"");

    assert_key(&tmux, inner, "0x23", "#");
    assert_key(&tmux, inner, "Escape 0x23", "M-#");

    assert_key(&tmux, inner, "0x24", "$");
    assert_key(&tmux, inner, "Escape 0x24", "M-$");

    assert_key(&tmux, inner, "0x25", "%");
    assert_key(&tmux, inner, "Escape 0x25", "M-%");

    assert_key(&tmux, inner, "0x26", "&");
    assert_key(&tmux, inner, "Escape 0x26", "M-&");

    assert_key(&tmux, inner, "0x27", "'");
    assert_key(&tmux, inner, "Escape 0x27", "M-'");

    assert_key(&tmux, inner, "0x28", "(");
    assert_key(&tmux, inner, "Escape 0x28", "M-(");

    assert_key(&tmux, inner, "0x29", ")");
    assert_key(&tmux, inner, "Escape 0x29", "M-)");

    assert_key(&tmux, inner, "0x2A", "*");
    assert_key(&tmux, inner, "Escape 0x2A", "M-*");

    assert_key(&tmux, inner, "0x2B", "+");
    assert_key(&tmux, inner, "Escape 0x2B", "M-+");

    assert_key(&tmux, inner, "0x2C", ",");
    assert_key(&tmux, inner, "Escape 0x2C", "M-,");

    assert_key(&tmux, inner, "0x2D", "-");
    assert_key(&tmux, inner, "Escape 0x2D", "M--");

    assert_key(&tmux, inner, "0x2E", ".");
    assert_key(&tmux, inner, "Escape 0x2E", "M-.");

    assert_key(&tmux, inner, "0x2F", "/");
    assert_key(&tmux, inner, "Escape 0x2F", "M-/");

    assert_key(&tmux, inner, "0x30", "0");
    assert_key(&tmux, inner, "Escape 0x30", "M-0");

    assert_key(&tmux, inner, "0x31", "1");
    assert_key(&tmux, inner, "Escape 0x31", "M-1");

    assert_key(&tmux, inner, "0x32", "2");
    assert_key(&tmux, inner, "Escape 0x32", "M-2");

    assert_key(&tmux, inner, "0x33", "3");
    assert_key(&tmux, inner, "Escape 0x33", "M-3");

    assert_key(&tmux, inner, "0x34", "4");
    assert_key(&tmux, inner, "Escape 0x34", "M-4");

    assert_key(&tmux, inner, "0x35", "5");
    assert_key(&tmux, inner, "Escape 0x35", "M-5");

    assert_key(&tmux, inner, "0x36", "6");
    assert_key(&tmux, inner, "Escape 0x36", "M-6");

    assert_key(&tmux, inner, "0x37", "7");
    assert_key(&tmux, inner, "Escape 0x37", "M-7");

    assert_key(&tmux, inner, "0x38", "8");
    assert_key(&tmux, inner, "Escape 0x38", "M-8");

    assert_key(&tmux, inner, "0x39", "9");
    assert_key(&tmux, inner, "Escape 0x39", "M-9");

    assert_key(&tmux, inner, "0x3A", ":");
    assert_key(&tmux, inner, "Escape 0x3A", "M-:");

    assert_key(&tmux, inner, "0x3B", ";");
    assert_key(&tmux, inner, "Escape 0x3B", "M-;");

    assert_key(&tmux, inner, "0x3C", "<");
    assert_key(&tmux, inner, "Escape 0x3C", "M-<");

    assert_key(&tmux, inner, "0x3D", "=");
    assert_key(&tmux, inner, "Escape 0x3D", "M-=");

    assert_key(&tmux, inner, "0x3E", ">");
    assert_key(&tmux, inner, "Escape 0x3E", "M->");

    assert_key(&tmux, inner, "0x3F", "?");
    assert_key(&tmux, inner, "Escape 0x3F", "M-?");

    assert_key(&tmux, inner, "0x40", "@");
    assert_key(&tmux, inner, "Escape 0x40", "M-@");

    assert_key(&tmux, inner, "0x41", "A");
    assert_key(&tmux, inner, "Escape 0x41", "M-A");

    assert_key(&tmux, inner, "0x42", "B");
    assert_key(&tmux, inner, "Escape 0x42", "M-B");

    assert_key(&tmux, inner, "0x43", "C");
    assert_key(&tmux, inner, "Escape 0x43", "M-C");

    assert_key(&tmux, inner, "0x44", "D");
    assert_key(&tmux, inner, "Escape 0x44", "M-D");

    assert_key(&tmux, inner, "0x45", "E");
    assert_key(&tmux, inner, "Escape 0x45", "M-E");

    assert_key(&tmux, inner, "0x46", "F");
    assert_key(&tmux, inner, "Escape 0x46", "M-F");

    assert_key(&tmux, inner, "0x47", "G");
    assert_key(&tmux, inner, "Escape 0x47", "M-G");

    assert_key(&tmux, inner, "0x48", "H");
    assert_key(&tmux, inner, "Escape 0x48", "M-H");

    assert_key(&tmux, inner, "0x49", "I");
    assert_key(&tmux, inner, "Escape 0x49", "M-I");

    assert_key(&tmux, inner, "0x4A", "J");
    assert_key(&tmux, inner, "Escape 0x4A", "M-J");

    assert_key(&tmux, inner, "0x4B", "K");
    assert_key(&tmux, inner, "Escape 0x4B", "M-K");

    assert_key(&tmux, inner, "0x4C", "L");
    assert_key(&tmux, inner, "Escape 0x4C", "M-L");

    assert_key(&tmux, inner, "0x4D", "M");
    assert_key(&tmux, inner, "Escape 0x4D", "M-M");

    assert_key(&tmux, inner, "0x4E", "N");
    assert_key(&tmux, inner, "Escape 0x4E", "M-N");

    assert_key(&tmux, inner, "0x4F", "O");
    assert_key(&tmux, inner, "Escape 0x4F", "M-O");

    assert_key(&tmux, inner, "0x50", "P");
    assert_key(&tmux, inner, "Escape 0x50", "M-P");

    assert_key(&tmux, inner, "0x51", "Q");
    assert_key(&tmux, inner, "Escape 0x51", "M-Q");

    assert_key(&tmux, inner, "0x52", "R");
    assert_key(&tmux, inner, "Escape 0x52", "M-R");

    assert_key(&tmux, inner, "0x53", "S");
    assert_key(&tmux, inner, "Escape 0x53", "M-S");

    assert_key(&tmux, inner, "0x54", "T");
    assert_key(&tmux, inner, "Escape 0x54", "M-T");

    assert_key(&tmux, inner, "0x55", "U");
    assert_key(&tmux, inner, "Escape 0x55", "M-U");

    assert_key(&tmux, inner, "0x56", "V");
    assert_key(&tmux, inner, "Escape 0x56", "M-V");

    assert_key(&tmux, inner, "0x57", "W");
    assert_key(&tmux, inner, "Escape 0x57", "M-W");

    assert_key(&tmux, inner, "0x58", "X");
    assert_key(&tmux, inner, "Escape 0x58", "M-X");

    assert_key(&tmux, inner, "0x59", "Y");
    assert_key(&tmux, inner, "Escape 0x59", "M-Y");

    assert_key(&tmux, inner, "0x5A", "Z");
    assert_key(&tmux, inner, "Escape 0x5A", "M-Z");

    assert_key(&tmux, inner, "0x5B", "[");
    assert_key(&tmux, inner, "Escape 0x5B", "M-[");

    assert_key(&tmux, inner, "0x5C", "\\");
    assert_key(&tmux, inner, "Escape 0x5C", "M-\\");

    assert_key(&tmux, inner, "0x5D", "]");
    assert_key(&tmux, inner, "Escape 0x5D", "M-]");

    assert_key(&tmux, inner, "0x5E", "^");
    assert_key(&tmux, inner, "Escape 0x5E", "M-^");

    assert_key(&tmux, inner, "0x5F", "_");
    assert_key(&tmux, inner, "Escape 0x5F", "M-_");

    assert_key(&tmux, inner, "0x60", "`");
    assert_key(&tmux, inner, "Escape 0x60", "M-`");

    assert_key(&tmux, inner, "0x61", "a");
    assert_key(&tmux, inner, "Escape 0x61", "M-a");

    assert_key(&tmux, inner, "0x62", "b");
    assert_key(&tmux, inner, "Escape 0x62", "M-b");

    assert_key(&tmux, inner, "0x63", "c");
    assert_key(&tmux, inner, "Escape 0x63", "M-c");

    assert_key(&tmux, inner, "0x64", "d");
    assert_key(&tmux, inner, "Escape 0x64", "M-d");

    assert_key(&tmux, inner, "0x65", "e");
    assert_key(&tmux, inner, "Escape 0x65", "M-e");

    assert_key(&tmux, inner, "0x66", "f");
    assert_key(&tmux, inner, "Escape 0x66", "M-f");

    assert_key(&tmux, inner, "0x67", "g");
    assert_key(&tmux, inner, "Escape 0x67", "M-g");

    assert_key(&tmux, inner, "0x68", "h");
    assert_key(&tmux, inner, "Escape 0x68", "M-h");

    assert_key(&tmux, inner, "0x69", "i");
    assert_key(&tmux, inner, "Escape 0x69", "M-i");

    assert_key(&tmux, inner, "0x6A", "j");
    assert_key(&tmux, inner, "Escape 0x6A", "M-j");

    assert_key(&tmux, inner, "0x6B", "k");
    assert_key(&tmux, inner, "Escape 0x6B", "M-k");

    assert_key(&tmux, inner, "0x6C", "l");
    assert_key(&tmux, inner, "Escape 0x6C", "M-l");

    assert_key(&tmux, inner, "0x6D", "m");
    assert_key(&tmux, inner, "Escape 0x6D", "M-m");

    assert_key(&tmux, inner, "0x6E", "n");
    assert_key(&tmux, inner, "Escape 0x6E", "M-n");

    assert_key(&tmux, inner, "0x6F", "o");
    assert_key(&tmux, inner, "Escape 0x6F", "M-o");

    assert_key(&tmux, inner, "0x70", "p");
    assert_key(&tmux, inner, "Escape 0x70", "M-p");

    assert_key(&tmux, inner, "0x71", "q");
    assert_key(&tmux, inner, "Escape 0x71", "M-q");

    assert_key(&tmux, inner, "0x72", "r");
    assert_key(&tmux, inner, "Escape 0x72", "M-r");

    assert_key(&tmux, inner, "0x73", "s");
    assert_key(&tmux, inner, "Escape 0x73", "M-s");

    assert_key(&tmux, inner, "0x74", "t");
    assert_key(&tmux, inner, "Escape 0x74", "M-t");

    assert_key(&tmux, inner, "0x75", "u");
    assert_key(&tmux, inner, "Escape 0x75", "M-u");

    assert_key(&tmux, inner, "0x76", "v");
    assert_key(&tmux, inner, "Escape 0x76", "M-v");

    assert_key(&tmux, inner, "0x77", "w");
    assert_key(&tmux, inner, "Escape 0x77", "M-w");

    assert_key(&tmux, inner, "0x78", "x");
    assert_key(&tmux, inner, "Escape 0x78", "M-x");

    assert_key(&tmux, inner, "0x79", "y");
    assert_key(&tmux, inner, "Escape 0x79", "M-y");

    assert_key(&tmux, inner, "0x7A", "z");
    assert_key(&tmux, inner, "Escape 0x7A", "M-z");

    assert_key(&tmux, inner, "0x7B", "{");
    assert_key(&tmux, inner, "Escape 0x7B", "M-{");

    assert_key(&tmux, inner, "0x7C", "|");
    assert_key(&tmux, inner, "Escape 0x7C", "M-|");

    assert_key(&tmux, inner, "0x7D", "}");
    assert_key(&tmux, inner, "Escape 0x7D", "M-}");

    assert_key(&tmux, inner, "0x7E", "~");
    assert_key(&tmux, inner, "Escape 0x7E", "M-~");

    assert_key(&tmux, inner, "0x7F", "BSpace");
    assert_key(&tmux, inner, "Escape 0x7F", "M-BSpace");

    // Numeric keypad
    assert_key(&tmux, inner, "Escape OM", "KPEnter");
    assert_key(&tmux, inner, "Escape Escape OM", "M-KPEnter");

    assert_key(&tmux, inner, "Escape Oj", "KP*");
    assert_key(&tmux, inner, "Escape Escape Oj", "M-KP*");

    assert_key(&tmux, inner, "Escape Ok", "KP+");
    assert_key(&tmux, inner, "Escape Escape Ok", "M-KP+");

    assert_key(&tmux, inner, "Escape Om", "KP-");
    assert_key(&tmux, inner, "Escape Escape Om", "M-KP-");

    assert_key(&tmux, inner, "Escape On", "KP.");
    assert_key(&tmux, inner, "Escape Escape On", "M-KP.");

    assert_key(&tmux, inner, "Escape Oo", "KP/");
    assert_key(&tmux, inner, "Escape Escape Oo", "M-KP/");

    assert_key(&tmux, inner, "Escape Op", "KP0");
    assert_key(&tmux, inner, "Escape Escape Op", "M-KP0");

    assert_key(&tmux, inner, "Escape Oq", "KP1");
    assert_key(&tmux, inner, "Escape Escape Oq", "M-KP1");

    assert_key(&tmux, inner, "Escape Or", "KP2");
    assert_key(&tmux, inner, "Escape Escape Or", "M-KP2");

    assert_key(&tmux, inner, "Escape Os", "KP3");
    assert_key(&tmux, inner, "Escape Escape Os", "M-KP3");

    assert_key(&tmux, inner, "Escape Ot", "KP4");
    assert_key(&tmux, inner, "Escape Escape Ot", "M-KP4");

    assert_key(&tmux, inner, "Escape Ou", "KP5");
    assert_key(&tmux, inner, "Escape Escape Ou", "M-KP5");

    assert_key(&tmux, inner, "Escape Ov", "KP6");
    assert_key(&tmux, inner, "Escape Escape Ov", "M-KP6");

    assert_key(&tmux, inner, "Escape Ow", "KP7");
    assert_key(&tmux, inner, "Escape Escape Ow", "M-KP7");

    assert_key(&tmux, inner, "Escape Ox", "KP8");
    assert_key(&tmux, inner, "Escape Escape Ox", "M-KP8");

    assert_key(&tmux, inner, "Escape Oy", "KP9");
    assert_key(&tmux, inner, "Escape Escape Oy", "M-KP9");

    // Arrow keys (SS3 form)
    assert_key(&tmux, inner, "Escape OA", "Up");
    assert_key(&tmux, inner, "Escape Escape OA", "M-Up");

    assert_key(&tmux, inner, "Escape OB", "Down");
    assert_key(&tmux, inner, "Escape Escape OB", "M-Down");

    assert_key(&tmux, inner, "Escape OC", "Right");
    assert_key(&tmux, inner, "Escape Escape OC", "M-Right");

    assert_key(&tmux, inner, "Escape OD", "Left");
    assert_key(&tmux, inner, "Escape Escape OD", "M-Left");

    // Arrow keys (CSI form)
    assert_key(&tmux, inner, "Escape [A", "Up");
    assert_key(&tmux, inner, "Escape Escape [A", "M-Up");

    assert_key(&tmux, inner, "Escape [B", "Down");
    assert_key(&tmux, inner, "Escape Escape [B", "M-Down");

    assert_key(&tmux, inner, "Escape [C", "Right");
    assert_key(&tmux, inner, "Escape Escape [C", "M-Right");

    assert_key(&tmux, inner, "Escape [D", "Left");
    assert_key(&tmux, inner, "Escape Escape [D", "M-Left");

    // Other xterm keys (SS3 form)
    assert_key(&tmux, inner, "Escape OH", "Home");
    assert_key(&tmux, inner, "Escape Escape OH", "M-Home");

    assert_key(&tmux, inner, "Escape OF", "End");
    assert_key(&tmux, inner, "Escape Escape OF", "M-End");

    // Other xterm keys (CSI form)
    assert_key(&tmux, inner, "Escape [H", "Home");
    assert_key(&tmux, inner, "Escape Escape [H", "M-Home");

    assert_key(&tmux, inner, "Escape [F", "End");
    assert_key(&tmux, inner, "Escape Escape [F", "M-End");

    // rxvt arrow keys
    assert_key(&tmux, inner, "Escape Oa", "C-Up");
    assert_key(&tmux, inner, "Escape Ob", "C-Down");
    assert_key(&tmux, inner, "Escape Oc", "C-Right");
    assert_key(&tmux, inner, "Escape Od", "C-Left");
    assert_key(&tmux, inner, "Escape [a", "S-Up");
    assert_key(&tmux, inner, "Escape [b", "S-Down");
    assert_key(&tmux, inner, "Escape [c", "S-Right");
    assert_key(&tmux, inner, "Escape [d", "S-Left");

    // rxvt function keys
    assert_key(&tmux, inner, "Escape [11~", "F1");
    assert_key(&tmux, inner, "Escape [12~", "F2");
    assert_key(&tmux, inner, "Escape [13~", "F3");
    assert_key(&tmux, inner, "Escape [14~", "F4");
    assert_key(&tmux, inner, "Escape [15~", "F5");
    assert_key(&tmux, inner, "Escape [17~", "F6");
    assert_key(&tmux, inner, "Escape [18~", "F7");
    assert_key(&tmux, inner, "Escape [19~", "F8");
    assert_key(&tmux, inner, "Escape [20~", "F9");
    assert_key(&tmux, inner, "Escape [21~", "F10");
    assert_key(&tmux, inner, "Escape [23~", "F11");
    assert_key(&tmux, inner, "Escape [24~", "F12");

    // Shifted function keys
    // With TERM=screen, F11/F12 are [23~/[24~ so S-F1/S-F2 are not testable
    assert_key(&tmux, inner, "Escape [25~", "S-F3");
    assert_key(&tmux, inner, "Escape [26~", "S-F4");
    assert_key(&tmux, inner, "Escape [28~", "S-F5");
    assert_key(&tmux, inner, "Escape [29~", "S-F6");
    assert_key(&tmux, inner, "Escape [31~", "S-F7");
    assert_key(&tmux, inner, "Escape [32~", "S-F8");
    assert_key(&tmux, inner, "Escape [33~", "S-F9");
    assert_key(&tmux, inner, "Escape [34~", "S-F10");
    assert_key(&tmux, inner, "Escape [23$", "S-F11");
    assert_key(&tmux, inner, "Escape [24$", "S-F12");

    // Ctrl function keys
    assert_key(&tmux, inner, "Escape [11^", "C-F1");
    assert_key(&tmux, inner, "Escape [12^", "C-F2");
    assert_key(&tmux, inner, "Escape [13^", "C-F3");
    assert_key(&tmux, inner, "Escape [14^", "C-F4");
    assert_key(&tmux, inner, "Escape [15^", "C-F5");
    assert_key(&tmux, inner, "Escape [17^", "C-F6");
    assert_key(&tmux, inner, "Escape [18^", "C-F7");
    assert_key(&tmux, inner, "Escape [19^", "C-F8");
    assert_key(&tmux, inner, "Escape [20^", "C-F9");
    assert_key(&tmux, inner, "Escape [21^", "C-F10");
    assert_key(&tmux, inner, "Escape [23^", "C-F11");
    assert_key(&tmux, inner, "Escape [24^", "C-F12");

    // Ctrl+Shift function keys
    assert_key(&tmux, inner, "Escape [11@", "C-S-F1");
    assert_key(&tmux, inner, "Escape [12@", "C-S-F2");
    assert_key(&tmux, inner, "Escape [13@", "C-S-F3");
    assert_key(&tmux, inner, "Escape [14@", "C-S-F4");
    assert_key(&tmux, inner, "Escape [15@", "C-S-F5");
    assert_key(&tmux, inner, "Escape [17@", "C-S-F6");
    assert_key(&tmux, inner, "Escape [18@", "C-S-F7");
    assert_key(&tmux, inner, "Escape [19@", "C-S-F8");
    assert_key(&tmux, inner, "Escape [20@", "C-S-F9");
    assert_key(&tmux, inner, "Escape [21@", "C-S-F10");
    assert_key(&tmux, inner, "Escape [23@", "C-S-F11");
    assert_key(&tmux, inner, "Escape [24@", "C-S-F12");

    // Focus tracking
    assert_key(&tmux, inner, "Escape [I", "FocusIn");
    assert_key(&tmux, inner, "Escape [O", "FocusOut");

    // Paste keys
    assert_key(&tmux, inner, "Escape [200~", "PasteStart");
    assert_key(&tmux, inner, "Escape [201~", "PasteEnd");

    // Back-tab
    assert_key(&tmux, inner, "Escape [Z", "BTab");

    // Extended keys (CSI u format)
    assert_key(&tmux, inner, "Escape [123;5u", "C-{");
    assert_key(&tmux, inner, "Escape [32;2u", "S-Space");
    assert_key(&tmux, inner, "Escape [9;5u", "C-Tab");
    assert_key(&tmux, inner, "Escape [1;5Z", "C-S-Tab");

    tmux.kill_server();
    tmux2.kill_server();
}

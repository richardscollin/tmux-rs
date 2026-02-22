use super::*;

/// Send a named key via send-keys and verify the raw bytes that arrive in a
/// `cat -tv` pane.
///
/// Mirrors the shell assert_key function:
///   1. new-window -P -- sh -c 'stty raw -echo && cat -tv'
///   2. send-keys -t$W "$key" 'EOL'
///   3. sleep 0.2
///   4. capturep -pt$W | head -1 | sed 's/EOL.*$//'
///   5. kill-window -t$W
fn assert_key(tmux: &TmuxServer, key: &str, expected: &str) {
    let window = tmux
        .run(&[
            "new-window",
            "-P",
            "--",
            "sh",
            "-c",
            "stty raw -echo && cat -tv",
        ])
        .trim()
        .to_string();
    tmux.run(&["send-keys", &format!("-t{}", window), key, "EOL"]);
    sleep_ms(200);
    let captured = tmux.run(&["capturep", "-p", &format!("-t{}", window)]);
    let first_line = captured.lines().next().unwrap_or("");
    let actual = match first_line.find("EOL") {
        Some(pos) => &first_line[..pos],
        None => first_line,
    };
    tmux.run(&["kill-window", &format!("-t{}", window)]);
    assert_eq!(
        actual, expected,
        "input-keys: key='{}' expected='{}', got='{}'",
        key, expected, actual
    );
}

/// Ported from regress/input-keys.sh
///
/// Tests that tmux send-keys correctly translates key names into the expected
/// raw byte sequences, as observed through `cat -tv` in a raw tty.
///
/// Roughly 250 assertions covering control keys, printable ASCII, meta
/// combinations, function keys, navigation keys, keypad keys, back-tab, and
/// extended keys with modifier combinations.
#[test]
#[cfg_attr(not(feature = "slow-tests"), ignore)]
fn input_keys() {
    let tmux = TmuxServer::new("input_keys");

    tmux.run(&["-f/dev/null", "new", "-x20", "-y2", "-d"]);
    sleep_secs(1);
    tmux.run(&["set", "-g", "escape-time", "0"]);

    // -- Control keys --
    assert_key(&tmux, "C-Space", "^@");
    assert_key(&tmux, "C-a", "^A");
    assert_key(&tmux, "M-C-a", "^[^A");
    assert_key(&tmux, "C-b", "^B");
    assert_key(&tmux, "M-C-b", "^[^B");
    assert_key(&tmux, "C-c", "^C");
    assert_key(&tmux, "M-C-c", "^[^C");
    assert_key(&tmux, "C-d", "^D");
    assert_key(&tmux, "M-C-d", "^[^D");
    assert_key(&tmux, "C-e", "^E");
    assert_key(&tmux, "M-C-e", "^[^E");
    assert_key(&tmux, "C-f", "^F");
    assert_key(&tmux, "M-C-f", "^[^F");
    assert_key(&tmux, "C-g", "^G");
    assert_key(&tmux, "M-C-g", "^[^G");
    assert_key(&tmux, "C-h", "^H");
    assert_key(&tmux, "M-C-h", "^[^H");
    assert_key(&tmux, "C-i", "^I");
    assert_key(&tmux, "M-C-i", "^[^I");
    assert_key(&tmux, "C-j", ""); // NL
    assert_key(&tmux, "M-C-j", "^["); // NL
    assert_key(&tmux, "C-k", "^K");
    assert_key(&tmux, "M-C-k", "^[^K");
    assert_key(&tmux, "C-l", "^L");
    assert_key(&tmux, "M-C-l", "^[^L");
    assert_key(&tmux, "C-m", "^M");
    assert_key(&tmux, "M-C-m", "^[^M");
    assert_key(&tmux, "C-n", "^N");
    assert_key(&tmux, "M-C-n", "^[^N");
    assert_key(&tmux, "C-o", "^O");
    assert_key(&tmux, "M-C-o", "^[^O");
    assert_key(&tmux, "C-p", "^P");
    assert_key(&tmux, "M-C-p", "^[^P");
    assert_key(&tmux, "C-q", "^Q");
    assert_key(&tmux, "M-C-q", "^[^Q");
    assert_key(&tmux, "C-r", "^R");
    assert_key(&tmux, "M-C-r", "^[^R");
    assert_key(&tmux, "C-s", "^S");
    assert_key(&tmux, "M-C-s", "^[^S");
    assert_key(&tmux, "C-t", "^T");
    assert_key(&tmux, "M-C-t", "^[^T");
    assert_key(&tmux, "C-u", "^U");
    assert_key(&tmux, "M-C-u", "^[^U");
    assert_key(&tmux, "C-v", "^V");
    assert_key(&tmux, "M-C-v", "^[^V");
    assert_key(&tmux, "C-w", "^W");
    assert_key(&tmux, "M-C-w", "^[^W");
    assert_key(&tmux, "C-x", "^X");
    assert_key(&tmux, "M-C-x", "^[^X");
    assert_key(&tmux, "C-y", "^Y");
    assert_key(&tmux, "M-C-y", "^[^Y");
    assert_key(&tmux, "C-z", "^Z");
    assert_key(&tmux, "M-C-z", "^[^Z");
    assert_key(&tmux, "Escape", "^[");
    assert_key(&tmux, "M-Escape", "^[^[");
    assert_key(&tmux, "C-\\", "^\\");
    assert_key(&tmux, "M-C-\\", "^[^\\");
    assert_key(&tmux, "C-]", "^]");
    assert_key(&tmux, "M-C-]", "^[^]");
    assert_key(&tmux, "C-^", "^^");
    assert_key(&tmux, "M-C-^", "^[^^");
    assert_key(&tmux, "C-_", "^_");
    assert_key(&tmux, "M-C-_", "^[^_");

    // -- Printable ASCII (Space through ~) --
    assert_key(&tmux, "Space", " ");
    assert_key(&tmux, "M-Space", "^[ ");
    assert_key(&tmux, "!", "!");
    assert_key(&tmux, "M-!", "^[!");
    assert_key(&tmux, "\"", "\"");
    assert_key(&tmux, "M-\"", "^[\"");
    assert_key(&tmux, "#", "#");
    assert_key(&tmux, "M-#", "^[#");
    assert_key(&tmux, "$", "$");
    assert_key(&tmux, "M-$", "^[$");
    assert_key(&tmux, "%", "%");
    assert_key(&tmux, "M-%", "^[%");
    assert_key(&tmux, "&", "&");
    assert_key(&tmux, "M-&", "^[&");
    assert_key(&tmux, "'", "'");
    assert_key(&tmux, "M-'", "^['");
    assert_key(&tmux, "(", "(");
    assert_key(&tmux, "M-(", "^[(");
    assert_key(&tmux, ")", ")");
    assert_key(&tmux, "M-)", "^[)");
    assert_key(&tmux, "*", "*");
    assert_key(&tmux, "M-*", "^[*");
    assert_key(&tmux, "+", "+");
    assert_key(&tmux, "M-+", "^[+");
    assert_key(&tmux, ",", ",");
    assert_key(&tmux, "M-,", "^[,");
    assert_key(&tmux, "-", "-");
    assert_key(&tmux, "M--", "^[-");
    assert_key(&tmux, ".", ".");
    assert_key(&tmux, "M-.", "^[.");
    assert_key(&tmux, "/", "/");
    assert_key(&tmux, "M-/", "^[/");
    assert_key(&tmux, "0", "0");
    assert_key(&tmux, "M-0", "^[0");
    assert_key(&tmux, "1", "1");
    assert_key(&tmux, "M-1", "^[1");
    assert_key(&tmux, "2", "2");
    assert_key(&tmux, "M-2", "^[2");
    assert_key(&tmux, "3", "3");
    assert_key(&tmux, "M-3", "^[3");
    assert_key(&tmux, "4", "4");
    assert_key(&tmux, "M-4", "^[4");
    assert_key(&tmux, "5", "5");
    assert_key(&tmux, "M-5", "^[5");
    assert_key(&tmux, "6", "6");
    assert_key(&tmux, "M-6", "^[6");
    assert_key(&tmux, "7", "7");
    assert_key(&tmux, "M-7", "^[7");
    assert_key(&tmux, "8", "8");
    assert_key(&tmux, "M-8", "^[8");
    assert_key(&tmux, "9", "9");
    assert_key(&tmux, "M-9", "^[9");
    assert_key(&tmux, ":", ":");
    assert_key(&tmux, "M-:", "^[:");
    assert_key(&tmux, "\\;", ";");
    assert_key(&tmux, "M-\\;", "^[;");
    assert_key(&tmux, "<", "<");
    assert_key(&tmux, "M-<", "^[<");
    assert_key(&tmux, "=", "=");
    assert_key(&tmux, "M-=", "^[=");
    assert_key(&tmux, ">", ">");
    assert_key(&tmux, "M->", "^[>");
    assert_key(&tmux, "?", "?");
    assert_key(&tmux, "M-?", "^[?");
    assert_key(&tmux, "@", "@");
    assert_key(&tmux, "M-@", "^[@");
    assert_key(&tmux, "A", "A");
    assert_key(&tmux, "M-A", "^[A");
    assert_key(&tmux, "B", "B");
    assert_key(&tmux, "M-B", "^[B");
    assert_key(&tmux, "C", "C");
    assert_key(&tmux, "M-C", "^[C");
    assert_key(&tmux, "D", "D");
    assert_key(&tmux, "M-D", "^[D");
    assert_key(&tmux, "E", "E");
    assert_key(&tmux, "M-E", "^[E");
    assert_key(&tmux, "F", "F");
    assert_key(&tmux, "M-F", "^[F");
    assert_key(&tmux, "G", "G");
    assert_key(&tmux, "M-G", "^[G");
    assert_key(&tmux, "H", "H");
    assert_key(&tmux, "M-H", "^[H");
    assert_key(&tmux, "I", "I");
    assert_key(&tmux, "M-I", "^[I");
    assert_key(&tmux, "J", "J");
    assert_key(&tmux, "M-J", "^[J");
    assert_key(&tmux, "K", "K");
    assert_key(&tmux, "M-K", "^[K");
    assert_key(&tmux, "L", "L");
    assert_key(&tmux, "M-L", "^[L");
    assert_key(&tmux, "M", "M");
    assert_key(&tmux, "M-M", "^[M");
    assert_key(&tmux, "N", "N");
    assert_key(&tmux, "M-N", "^[N");
    assert_key(&tmux, "O", "O");
    assert_key(&tmux, "M-O", "^[O");
    assert_key(&tmux, "P", "P");
    assert_key(&tmux, "M-P", "^[P");
    assert_key(&tmux, "Q", "Q");
    assert_key(&tmux, "M-Q", "^[Q");
    assert_key(&tmux, "R", "R");
    assert_key(&tmux, "M-R", "^[R");
    assert_key(&tmux, "S", "S");
    assert_key(&tmux, "M-S", "^[S");
    assert_key(&tmux, "T", "T");
    assert_key(&tmux, "M-T", "^[T");
    assert_key(&tmux, "U", "U");
    assert_key(&tmux, "M-U", "^[U");
    assert_key(&tmux, "V", "V");
    assert_key(&tmux, "M-V", "^[V");
    assert_key(&tmux, "W", "W");
    assert_key(&tmux, "M-W", "^[W");
    assert_key(&tmux, "X", "X");
    assert_key(&tmux, "M-X", "^[X");
    assert_key(&tmux, "Y", "Y");
    assert_key(&tmux, "M-Y", "^[Y");
    assert_key(&tmux, "Z", "Z");
    assert_key(&tmux, "M-Z", "^[Z");
    assert_key(&tmux, "[", "[");
    assert_key(&tmux, "M-[", "^[[");
    assert_key(&tmux, "\\", "\\");
    assert_key(&tmux, "M-\\", "^[\\");
    assert_key(&tmux, "]", "]");
    assert_key(&tmux, "M-]", "^[]");
    assert_key(&tmux, "^", "^");
    assert_key(&tmux, "M-^", "^[^");
    assert_key(&tmux, "_", "_");
    assert_key(&tmux, "M-_", "^[_");
    assert_key(&tmux, "`", "`");
    assert_key(&tmux, "M-`", "^[`");
    assert_key(&tmux, "a", "a");
    assert_key(&tmux, "M-a", "^[a");
    assert_key(&tmux, "b", "b");
    assert_key(&tmux, "M-b", "^[b");
    assert_key(&tmux, "c", "c");
    assert_key(&tmux, "M-c", "^[c");
    assert_key(&tmux, "d", "d");
    assert_key(&tmux, "M-d", "^[d");
    assert_key(&tmux, "e", "e");
    assert_key(&tmux, "M-e", "^[e");
    assert_key(&tmux, "f", "f");
    assert_key(&tmux, "M-f", "^[f");
    assert_key(&tmux, "g", "g");
    assert_key(&tmux, "M-g", "^[g");
    assert_key(&tmux, "h", "h");
    assert_key(&tmux, "M-h", "^[h");
    assert_key(&tmux, "i", "i");
    assert_key(&tmux, "M-i", "^[i");
    assert_key(&tmux, "j", "j");
    assert_key(&tmux, "M-j", "^[j");
    assert_key(&tmux, "k", "k");
    assert_key(&tmux, "M-k", "^[k");
    assert_key(&tmux, "l", "l");
    assert_key(&tmux, "M-l", "^[l");
    assert_key(&tmux, "m", "m");
    assert_key(&tmux, "M-m", "^[m");
    assert_key(&tmux, "n", "n");
    assert_key(&tmux, "M-n", "^[n");
    assert_key(&tmux, "o", "o");
    assert_key(&tmux, "M-o", "^[o");
    assert_key(&tmux, "p", "p");
    assert_key(&tmux, "M-p", "^[p");
    assert_key(&tmux, "q", "q");
    assert_key(&tmux, "M-q", "^[q");
    assert_key(&tmux, "r", "r");
    assert_key(&tmux, "M-r", "^[r");
    assert_key(&tmux, "s", "s");
    assert_key(&tmux, "M-s", "^[s");
    assert_key(&tmux, "t", "t");
    assert_key(&tmux, "M-t", "^[t");
    assert_key(&tmux, "u", "u");
    assert_key(&tmux, "M-u", "^[u");
    assert_key(&tmux, "v", "v");
    assert_key(&tmux, "M-v", "^[v");
    assert_key(&tmux, "w", "w");
    assert_key(&tmux, "M-w", "^[w");
    assert_key(&tmux, "x", "x");
    assert_key(&tmux, "M-x", "^[x");
    assert_key(&tmux, "y", "y");
    assert_key(&tmux, "M-y", "^[y");
    assert_key(&tmux, "z", "z");
    assert_key(&tmux, "M-z", "^[z");
    assert_key(&tmux, "{", "{");
    assert_key(&tmux, "M-{", "^[{");
    assert_key(&tmux, "|", "|");
    assert_key(&tmux, "M-|", "^[|");
    assert_key(&tmux, "}", "}");
    assert_key(&tmux, "M-}", "^[}");
    assert_key(&tmux, "~", "~");
    assert_key(&tmux, "M-~", "^[~");

    // -- Tab and BSpace --
    assert_key(&tmux, "Tab", "^I");
    assert_key(&tmux, "M-Tab", "^[^I");
    assert_key(&tmux, "BSpace", "^?");
    assert_key(&tmux, "M-BSpace", "^[^?");

    // PasteStart and PasteEnd cannot be sent (commented out in upstream)

    // -- Function keys --
    assert_key(&tmux, "F1", "^[OP");
    assert_key(&tmux, "F2", "^[OQ");
    assert_key(&tmux, "F3", "^[OR");
    assert_key(&tmux, "F4", "^[OS");
    assert_key(&tmux, "F5", "^[[15~");
    assert_key(&tmux, "F6", "^[[17~");
    assert_key(&tmux, "F8", "^[[19~");
    assert_key(&tmux, "F9", "^[[20~");
    assert_key(&tmux, "F10", "^[[21~");
    assert_key(&tmux, "F11", "^[[23~");
    assert_key(&tmux, "F12", "^[[24~");

    // -- Insert / Delete --
    assert_key(&tmux, "IC", "^[[2~");
    assert_key(&tmux, "Insert", "^[[2~");
    assert_key(&tmux, "DC", "^[[3~");
    assert_key(&tmux, "Delete", "^[[3~");

    // -- Home / End --
    assert_key(&tmux, "Home", "^[[1~");
    assert_key(&tmux, "End", "^[[4~");

    // -- Page Up / Page Down --
    assert_key(&tmux, "NPage", "^[[6~");
    assert_key(&tmux, "PageDown", "^[[6~");
    assert_key(&tmux, "PgDn", "^[[6~");
    assert_key(&tmux, "PPage", "^[[5~");
    assert_key(&tmux, "PageUp", "^[[5~");
    assert_key(&tmux, "PgUp", "^[[5~");

    // -- Back-tab --
    assert_key(&tmux, "BTab", "^[[Z");
    assert_key(&tmux, "C-S-Tab", "^I");

    // -- Arrow keys --
    assert_key(&tmux, "Up", "^[[A");
    assert_key(&tmux, "Down", "^[[B");
    assert_key(&tmux, "Right", "^[[C");
    assert_key(&tmux, "Left", "^[[D");

    // -- Keypad keys --
    // KPEnter cannot be tested in the shell script either
    assert_key(&tmux, "KP*", "*");
    assert_key(&tmux, "M-KP*", "^[*");
    assert_key(&tmux, "KP+", "+");
    assert_key(&tmux, "M-KP+", "^[+");
    assert_key(&tmux, "KP-", "-");
    assert_key(&tmux, "M-KP-", "^[-");
    assert_key(&tmux, "KP.", ".");
    assert_key(&tmux, "M-KP.", "^[.");
    assert_key(&tmux, "KP/", "/");
    assert_key(&tmux, "M-KP/", "^[/");
    assert_key(&tmux, "KP0", "0");
    assert_key(&tmux, "M-KP0", "^[0");
    assert_key(&tmux, "KP1", "1");
    assert_key(&tmux, "M-KP1", "^[1");
    assert_key(&tmux, "KP2", "2");
    assert_key(&tmux, "M-KP2", "^[2");
    assert_key(&tmux, "KP3", "3");
    assert_key(&tmux, "M-KP3", "^[3");
    assert_key(&tmux, "KP4", "4");
    assert_key(&tmux, "M-KP4", "^[4");
    assert_key(&tmux, "KP5", "5");
    assert_key(&tmux, "M-KP5", "^[5");
    assert_key(&tmux, "KP6", "6");
    assert_key(&tmux, "M-KP6", "^[6");
    assert_key(&tmux, "KP7", "7");
    assert_key(&tmux, "M-KP7", "^[7");
    assert_key(&tmux, "KP8", "8");
    assert_key(&tmux, "M-KP8", "^[8");
    assert_key(&tmux, "KP9", "9");
    assert_key(&tmux, "M-KP9", "^[9");

    // -- Extended keys --
    // Enable extended-keys mode for modifier combinations on special keys.
    tmux.run(&["set", "-g", "extended-keys", "always"]);

    // The shell script uses assert_extended_key which tests all 7 modifier
    // combinations (S-, M-, S-M-, C-, S-C-, C-M-, S-C-M-) for a given key
    // pattern. The pattern uses ";_" as a placeholder replaced by the modifier
    // code (2-8).
    //
    // Modifier codes: 2=Shift, 3=Meta, 4=Shift+Meta, 5=Ctrl, 6=Shift+Ctrl,
    //                 7=Ctrl+Meta, 8=Shift+Ctrl+Meta
    let modifier_prefixes = [
        (2, "S-"),
        (3, "M-"),
        (4, "S-M-"),
        (5, "C-"),
        (6, "S-C-"),
        (7, "C-M-"),
        (8, "S-C-M-"),
    ];

    let extended_keys: &[(&str, &str)] = &[
        // Function keys
        ("F1", "^[[1;_P"),
        ("F2", "^[[1;_Q"),
        ("F3", "^[[1;_R"),
        ("F4", "^[[1;_S"),
        ("F5", "^[[15;_~"),
        ("F6", "^[[17;_~"),
        ("F8", "^[[19;_~"),
        ("F9", "^[[20;_~"),
        ("F10", "^[[21;_~"),
        ("F11", "^[[23;_~"),
        ("F12", "^[[24;_~"),
        // Arrow keys
        ("Up", "^[[1;_A"),
        ("Down", "^[[1;_B"),
        ("Right", "^[[1;_C"),
        ("Left", "^[[1;_D"),
        // Home / End
        ("Home", "^[[1;_H"),
        ("End", "^[[1;_F"),
        // Page Up / Page Down (all aliases)
        ("PPage", "^[[5;_~"),
        ("PageUp", "^[[5;_~"),
        ("PgUp", "^[[5;_~"),
        ("NPage", "^[[6;_~"),
        ("PageDown", "^[[6;_~"),
        ("PgDn", "^[[6;_~"),
        // Insert / Delete (all aliases)
        ("IC", "^[[2;_~"),
        ("Insert", "^[[2;_~"),
        ("DC", "^[[3;_~"),
        ("Delete", "^[[3;_~"),
    ];

    for &(key_name, pattern) in extended_keys {
        for &(mod_code, prefix) in &modifier_prefixes {
            let full_key = format!("{}{}", prefix, key_name);
            let expected = pattern.replace(";_", &format!(";{}", mod_code));
            assert_key(&tmux, &full_key, &expected);
        }
    }

    // Extended key: C-Tab and C-S-Tab with extended-keys enabled
    assert_key(&tmux, "C-Tab", "^[[27;5;9~");
    assert_key(&tmux, "C-S-Tab", "^[[27;6;9~");

    tmux.kill_server();
}

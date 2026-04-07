use super::*;

fn osc_bg_test(tmux: &TmuxServer, escape: &str, expected: &str) {
    let cmd = format!("printf '{}'", escape);
    tmux.run(&["splitw", &cmd]);
    sleep_ms(250);
    let bg = tmux.display("#{pane_bg}");
    tmux.run(&["kill-pane"]);
    assert_eq!(bg, expected, "OSC 11 test failed for escape '{}'", escape);
}

/// Ported from osc-11colours.sh
#[test]
#[cfg_attr(not(feature = "slow-tests"), ignore)]
fn osc_11_colours() {
    let tmux = TmuxServer::new("osc_11_colours");

    tmux.run(&["new", "-d"]);
    tmux.run(&["set", "-g", "remain-on-exit", "on"]);

    // Basic colour formats
    osc_bg_test(&tmux, "\\033]11;rgb:ff/ff/ff\\007", "#ffffff");
    osc_bg_test(&tmux, "\\033]11;rgb:ff/ff/ff\\007\\033]111\\007", "default");

    osc_bg_test(&tmux, "\\033]11;cmy:0.9373/0.6941/0.4549\\007", "#0f4e8b");
    osc_bg_test(&tmux, "\\033]11;cmyk:0.88/0.44/0.00/0.45\\007", "#104e8c");

    osc_bg_test(&tmux, "\\033]11;16,78,139\\007", "#104e8b");
    osc_bg_test(&tmux, "\\033]11;#104E8B\\007", "#104e8b");
    osc_bg_test(&tmux, "\\033]11;#10004E008B00\\007", "#104e8b");
    osc_bg_test(&tmux, "\\033]11;DodgerBlue4\\007", "#104e8b");
    osc_bg_test(&tmux, "\\033]11;DodgerBlue4    \\007", "#104e8b");
    osc_bg_test(&tmux, "\\033]11;    DodgerBlue4\\007", "#104e8b");
    osc_bg_test(&tmux, "\\033]11;rgb:10/4E/8B\\007", "#104e8b");
    osc_bg_test(&tmux, "\\033]11;rgb:1000/4E00/8B00\\007", "#104e8b");

    // grey/gray without number
    osc_bg_test(&tmux, "\\033]11;grey\\007", "#bebebe");
    osc_bg_test(&tmux, "\\033]11;gray\\007", "#bebebe");

    // Expected hex values for grey0 through grey100
    let grey_expected: &[&str] = &[
        "#000000", "#030303", "#050505", "#080808", "#0a0a0a", "#0d0d0d", "#0f0f0f", "#121212",
        "#141414", "#171717", "#1a1a1a", "#1c1c1c", "#1f1f1f", "#212121", "#242424", "#262626",
        "#292929", "#2b2b2b", "#2e2e2e", "#303030", "#333333", "#363636", "#383838", "#3b3b3b",
        "#3d3d3d", "#404040", "#424242", "#454545", "#474747", "#4a4a4a", "#4d4d4d", "#4f4f4f",
        "#525252", "#545454", "#575757", "#595959", "#5c5c5c", "#5e5e5e", "#616161", "#636363",
        "#666666", "#696969", "#6b6b6b", "#6e6e6e", "#707070", "#737373", "#757575", "#787878",
        "#7a7a7a", "#7d7d7d", "#7f7f7f", "#828282", "#858585", "#878787", "#8a8a8a", "#8c8c8c",
        "#8f8f8f", "#919191", "#949494", "#969696", "#999999", "#9c9c9c", "#9e9e9e", "#a1a1a1",
        "#a3a3a3", "#a6a6a6", "#a8a8a8", "#ababab", "#adadad", "#b0b0b0", "#b3b3b3", "#b5b5b5",
        "#b8b8b8", "#bababa", "#bdbdbd", "#bfbfbf", "#c2c2c2", "#c4c4c4", "#c7c7c7", "#c9c9c9",
        "#cccccc", "#cfcfcf", "#d1d1d1", "#d4d4d4", "#d6d6d6", "#d9d9d9", "#dbdbdb", "#dedede",
        "#e0e0e0", "#e3e3e3", "#e5e5e5", "#e8e8e8", "#ebebeb", "#ededed", "#f0f0f0", "#f2f2f2",
        "#f5f5f5", "#f7f7f7", "#fafafa", "#fcfcfc", "#ffffff",
    ];

    // Test grey0 through grey100
    for (i, expected) in grey_expected.iter().enumerate() {
        let escape = format!("\\033]11;grey{}\\007", i);
        osc_bg_test(&tmux, &escape, expected);
    }

    // Test gray0 through gray100 (same values as grey)
    for (i, expected) in grey_expected.iter().enumerate() {
        let escape = format!("\\033]11;gray{}\\007", i);
        osc_bg_test(&tmux, &escape, expected);
    }

    tmux.kill_server();
}

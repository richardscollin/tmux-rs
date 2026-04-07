use super::*;

/// Test colour parsing branches to exercise colour.rs branch coverage.

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn colour_named_colours() {
    let tmux = TmuxServer::new("clr_named");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Named colours
    for colour in &[
        "red", "green", "blue", "yellow", "cyan", "magenta", "white", "black",
    ] {
        tmux.run(&["set", "-g", "status-style", &format!("fg={colour}")]);
    }
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn colour_bright_names() {
    let tmux = TmuxServer::new("clr_bright");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Bright colour names
    for colour in &[
        "brightred",
        "brightgreen",
        "brightblue",
        "brightyellow",
        "brightcyan",
        "brightmagenta",
        "brightwhite",
        "brightblack",
    ] {
        tmux.run(&["set", "-g", "status-style", &format!("fg={colour}")]);
    }
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn colour_256_number() {
    let tmux = TmuxServer::new("clr_256num");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // colour0 through colour255
    tmux.run(&["set", "-g", "status-style", "fg=colour0"]);
    tmux.run(&["set", "-g", "status-style", "fg=colour15"]);
    tmux.run(&["set", "-g", "status-style", "fg=colour196"]);
    tmux.run(&["set", "-g", "status-style", "fg=colour255"]);
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn colour_256_invalid() {
    let tmux = TmuxServer::new("clr_256inv");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Invalid colour number
    let out = tmux.try_run(&["set", "-g", "status-style", "fg=colour256"]);
    assert!(!out.status.success());
    let out = tmux.try_run(&["set", "-g", "status-style", "fg=colour-1"]);
    assert!(!out.status.success());
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn colour_rgb_hex_formats() {
    let tmux = TmuxServer::new("clr_hex");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Various hex formats
    tmux.run(&["set", "-g", "status-style", "fg=#ff0000"]);
    tmux.run(&["set", "-g", "status-style", "fg=#00ff00"]);
    tmux.run(&["set", "-g", "status-style", "fg=#0000ff"]);
    tmux.run(&["set", "-g", "status-style", "fg=#ffffff"]);
    tmux.run(&["set", "-g", "status-style", "fg=#000000"]);
    // Short hex (3 digits)
    let out = tmux.try_run(&["set", "-g", "status-style", "fg=#f00"]);
    let _ = out;
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn colour_invalid_hex() {
    let tmux = TmuxServer::new("clr_badhex");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Invalid hex
    let out = tmux.try_run(&["set", "-g", "status-style", "fg=#gggggg"]);
    assert!(!out.status.success());
    let out = tmux.try_run(&["set", "-g", "status-style", "fg=#"]);
    assert!(!out.status.success());
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn colour_default_terminal() {
    let tmux = TmuxServer::new("clr_default");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // "default" and "terminal" special colours
    tmux.run(&["set", "-g", "status-style", "fg=default"]);
    tmux.run(&["set", "-g", "status-style", "fg=terminal"]);
    tmux.run(&["set", "-g", "status-style", "bg=default"]);
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn colour_x11_grey() {
    let tmux = TmuxServer::new("clr_grey");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // X11 grey/gray colours
    tmux.run(&["set", "-g", "status-style", "fg=grey"]);
    tmux.run(&["set", "-g", "status-style", "fg=gray"]);
    tmux.run(&["set", "-g", "status-style", "fg=grey50"]);
    tmux.run(&["set", "-g", "status-style", "fg=gray0"]);
    tmux.run(&["set", "-g", "status-style", "fg=grey100"]);
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn colour_x11_names() {
    let tmux = TmuxServer::new("clr_x11");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // X11 colour names
    for name in &[
        "coral",
        "gold",
        "khaki",
        "orchid",
        "plum",
        "salmon",
        "sienna",
        "tan",
        "tomato",
        "violet",
        "wheat",
        "SteelBlue",
        "DarkOrange",
        "LightGreen",
    ] {
        let out = tmux.try_run(&["set", "-g", "status-style", &format!("fg={name}")]);
        let _ = out;
    }
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn colour_tostring_roundtrip() {
    let tmux = TmuxServer::new("clr_round");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Set then read back — exercises colour_tostring
    tmux.run(&["set", "-g", "status-style", "fg=colour100,bg=#abcdef"]);
    let out = tmux.run(&["show", "-gv", "status-style"]);
    assert!(out.contains("100") || out.contains("abc"), "got: {out}");
}

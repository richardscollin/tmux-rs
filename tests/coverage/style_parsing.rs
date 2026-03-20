use super::*;

/// Test style parsing paths to exercise style_.rs.

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn style_fg_bg() {
    let tmux = TmuxServer::new("style_fgbg");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["set", "-g", "status-style", "fg=red,bg=blue"]);
    let out = tmux.run(&["show", "-gv", "status-style"]);
    assert!(out.contains("red") && out.contains("blue"), "got: {out}");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn style_attributes() {
    let tmux = TmuxServer::new("style_attr");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Various attributes
    tmux.run(&["set", "-g", "status-style", "bold"]);
    tmux.run(&["set", "-g", "status-style", "underscore"]);
    tmux.run(&["set", "-g", "status-style", "italics"]);
    tmux.run(&["set", "-g", "status-style", "reverse"]);
    tmux.run(&["set", "-g", "status-style", "dim"]);
    tmux.run(&["set", "-g", "status-style", "strikethrough"]);
    tmux.run(&["set", "-g", "status-style", "none"]);
    tmux.run(&["set", "-g", "status-style", "default"]);
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn style_256_colour() {
    let tmux = TmuxServer::new("style_256");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // 256-colour
    tmux.run(&["set", "-g", "status-style", "fg=colour196,bg=colour234"]);
    let out = tmux.run(&["show", "-gv", "status-style"]);
    assert!(out.contains("196"), "got: {out}");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn style_rgb_hex() {
    let tmux = TmuxServer::new("style_rgb");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // RGB hex colour
    tmux.run(&["set", "-g", "status-style", "fg=#ff0000,bg=#0000ff"]);
    let out = tmux.run(&["show", "-gv", "status-style"]);
    assert!(out.contains("#ff0000"), "got: {out}");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn style_multiple_attrs() {
    let tmux = TmuxServer::new("style_multi");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Multiple attributes combined
    tmux.run(&["set", "-g", "status-style", "bold,fg=green,bg=black"]);
    let out = tmux.run(&["show", "-gv", "status-style"]);
    assert!(out.contains("bold"), "got: {out}");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn style_invalid() {
    let tmux = TmuxServer::new("style_invalid");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Invalid style string
    let out = tmux.try_run(&["set", "-g", "status-style", "not_a_valid_style_xyz"]);
    assert!(!out.status.success());
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn style_underscore_colour() {
    let tmux = TmuxServer::new("style_us");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Underscore colour (us= style)
    tmux.run(&["set", "-g", "status-style", "underscore,us=red"]);
    let out = tmux.run(&["show", "-gv", "status-style"]);
    assert!(out.contains("underscore"), "got: {out}");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn style_push_pop() {
    let tmux = TmuxServer::new("style_pushpop");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Push/pop styles in status format
    tmux.run(&["set", "-g", "status-right", "#[push-default]#[fg=red]RED#[pop-default]normal"]);
    let out = tmux.run(&["show", "-gv", "status-right"]);
    assert!(out.contains("push-default"), "got: {out}");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn style_nodefault() {
    let tmux = TmuxServer::new("style_nodef");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // nodefault style attribute
    tmux.run(&["set", "-g", "status-right", "#[nodefault]test"]);
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn style_range() {
    let tmux = TmuxServer::new("style_range");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Range styles in status format
    tmux.run(&["set", "-g", "status-right", "#[range=left]left#[norange]"]);
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn style_align() {
    let tmux = TmuxServer::new("style_align");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Alignment in status
    tmux.run(&["set", "-g", "status-right", "#[align=centre]centered"]);
}

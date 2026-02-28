use super::*;

/// Basic rename-window.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn rename_window_basic() {
    let tmux = TmuxServer::new("renamew_basic");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["renamew", "myname"]);
    let name = tmux.display("#{window_name}");
    assert_eq!(name, "myname");
}

/// Rename specific window with -t.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn rename_window_target() {
    let tmux = TmuxServer::new("renamew_target");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["neww", "-d"]);

    tmux.run(&["renamew", "-t", ":1", "win_one"]);
    let name = tmux.run(&["display", "-t", ":1", "-p", "#{window_name}"]);
    assert_eq!(name.trim(), "win_one");
}

/// Rename window disables automatic-rename.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn rename_window_disables_auto_rename() {
    let tmux = TmuxServer::new("renamew_noauto");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["renamew", "fixed_name"]);
    let auto = tmux.run(&["showw", "-v", "automatic-rename"]);
    assert_eq!(auto.trim(), "off");
}

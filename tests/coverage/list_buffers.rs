use super::*;

/// List buffers when empty.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn list_buffers_empty() {
    let tmux = TmuxServer::new("lsb_empty");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let out = tmux.run(&["lsb"]);
    assert!(out.is_empty() || out.trim().is_empty());
}

/// List buffers after setting one.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn list_buffers_with_data() {
    let tmux = TmuxServer::new("lsb_data");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["setb", "-b", "mybuf", "hello world"]);
    let out = tmux.run(&["lsb"]);
    assert!(out.contains("mybuf"));
}

/// List buffers with custom format.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn list_buffers_format() {
    let tmux = TmuxServer::new("lsb_fmt");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["setb", "-b", "buf1", "data1"]);
    tmux.run(&["setb", "-b", "buf2", "data2"]);

    let out = tmux.run(&["lsb", "-F", "#{buffer_name}"]);
    assert!(out.contains("buf1"));
    assert!(out.contains("buf2"));
}

/// List buffers with filter.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn list_buffers_filter() {
    let tmux = TmuxServer::new("lsb_filter");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["setb", "-b", "keep", "data"]);
    tmux.run(&["setb", "-b", "drop", "data"]);

    let out = tmux.run(&[
        "lsb",
        "-f",
        "#{==:#{buffer_name},keep}",
        "-F",
        "#{buffer_name}",
    ]);
    assert!(out.contains("keep"));
    assert!(!out.contains("drop"));
}

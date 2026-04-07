use super::*;

/// Test new-window with various command forms (translates new-window-command.sh)
#[test]
fn new_window_command() {
    let tmux = TmuxServer::new("new_window_command");

    let conf = tmux.write_temp(
        "\
new\n\
neww sleep 101\n\
neww -- sleep 102\n\
neww \"sleep 103\"\n",
    );

    let f_flag = format!("-f{}", conf.path_str());
    tmux.run(&[&f_flag, "start"]);

    let output = tmux.run(&["lsw"]);
    let window_count = output.lines().count();
    assert_eq!(window_count, 4, "should have 4 windows");
}

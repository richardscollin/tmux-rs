use super::*;

const EXPECTED: &str = "\
bar,bar0
bar,bar1
bar,bar2
foo,foo0
foo,foo1
foo,foo2";

/// Test commands on one line separated by ; (translates command-order.sh test 1)
#[test]
fn command_order_semicolon() {
    let tmux = TmuxServer::new("command_order_semicolon");

    let conf = tmux.write_temp(
        "new -sfoo -nfoo0; neww -nfoo1; neww -nfoo2\nnew -sbar -nbar0; neww -nbar1; neww -nbar2\n",
    );

    let flag = format!("-f{}", conf.path_str());
    tmux.run(&[&flag, "start"]);
    sleep_secs(1);

    let output = tmux.run(&["lsw", "-aF", "#{session_name},#{window_name}"]);
    let mut lines: Vec<&str> = output.lines().collect();
    lines.sort();
    let sorted = lines.join("\n");

    assert_eq!(sorted, EXPECTED);
}

/// Test same commands, one per line (translates command-order.sh test 2)
#[test]
fn command_order_newline() {
    let tmux = TmuxServer::new("command_order_newline");

    let conf = tmux.write_temp(
        "new -sfoo -nfoo0\nneww -nfoo1\nneww -nfoo2\nnew -sbar -nbar0\nneww -nbar1\nneww -nbar2\n",
    );

    let flag = format!("-f{}", conf.path_str());
    tmux.run(&[&flag, "start"]);
    sleep_secs(1);

    let output = tmux.run(&["lsw", "-aF", "#{session_name},#{window_name}"]);
    let mut lines: Vec<&str> = output.lines().collect();
    lines.sort();
    let sorted = lines.join("\n");

    assert_eq!(sorted, EXPECTED);
}

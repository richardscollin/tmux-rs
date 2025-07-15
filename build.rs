fn main() {
    #[cfg(feature = "lalrpop")]
    {
        println!("cargo::rerun-if-changed=src/cmd_parse.lalrpop");
        lalrpop::process_root().unwrap();
    }

    // ncurses and event_core referenced through #[link] on extern block
}

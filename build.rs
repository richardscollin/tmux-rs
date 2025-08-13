fn main() {
    println!("cargo::rerun-if-changed=src/cmd_parse.lalrpop");
    lalrpop::process_root().unwrap();

    // ncurses and event_core referenced through #[link] on extern block

    // Look for libevent_core using pkg-config
    #[cfg(target_os = "macos")]
    if pkg_config::probe_library("libevent_core").is_err() {
        println!("cargo::warning=Could not find libevent_core using pkg-config");
    }
}

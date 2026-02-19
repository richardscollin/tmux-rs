fn main() {
    println!("cargo::rerun-if-changed=src/cmd_parse.lalrpop");
    lalrpop::process_root().unwrap();

    #[cfg(all(target_os = "macos", not(feature = "event-tokio")))]
    {
        use std::path::PathBuf;
        use std::process::Command;

        fn brew_link_prefix(target: &str) -> PathBuf {
            let output = Command::new("brew")
                .arg("--prefix")
                .arg(target)
                .output()
                .expect("homebrew is not installed");

            assert!(output.status.success(), "`brew --prefix {target}` failed");
            let path = String::from_utf8(output.stdout).unwrap();
            PathBuf::from(path.trim()).join("lib")
        }

        println!("cargo::rerun-if-env-changed=TMUX_RS_DISABLE_HOMEBREW_LIBS");
        if matches!(
            std::env::var("TMUX_RS_DISABLE_HOMEBREW_LIBS"),
            Err(std::env::VarError::NotPresent)
        ) {
            println!(
                "cargo::rustc-link-search={}",
                brew_link_prefix("libevent").display()
            );
        }
    }

    if !cfg!(feature = "event-tokio") {
        if is_static_linking() {
            println!("cargo::rustc-link-lib=static=event_core");
        } else {
            println!("cargo::rustc-link-lib=event_core");
        }
    }
}

/// determine how external c libraries should be linked
///
/// default to static linking on mac and dynamic linking on linux
/// this can be configured with the static or dynamic feature flags
///
/// because feature flags are additive, both can be set at the same time
/// if this is the case, follow the platform default rules
fn is_static_linking() -> bool {
    let mut static_linking;
    if cfg!(target_os = "macos") {
        static_linking = true;
        if cfg!(feature = "dynamic") {
            static_linking = false;
        }
        if cfg!(feature = "static") {
            static_linking = true;
        }
    } else {
        static_linking = false;
        if cfg!(feature = "static") {
            static_linking = true;
        }
        if cfg!(feature = "dynamic") {
            static_linking = false;
        }
    }
    static_linking
}

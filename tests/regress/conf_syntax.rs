use super::*;

/// Test that all conf files in the regress/conf/ directory parse without error
/// (translates conf-syntax.sh)
#[test]
fn conf_syntax() {
    let conf_dir = TmuxServer::regress_dir().join("conf");
    let mut entries: Vec<_> = std::fs::read_dir(&conf_dir)
        .unwrap_or_else(|e| panic!("failed to read conf dir {:?}: {}", conf_dir, e))
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map_or(false, |ext| ext == "conf"))
        .collect();
    entries.sort_by_key(|e| e.file_name());

    assert!(
        !entries.is_empty(),
        "no .conf files found in {:?}",
        conf_dir
    );

    for entry in &entries {
        let path = entry.path();
        let path_str = path.to_str().expect("non-utf8 conf path");

        let tmux = TmuxServer::new(&format!(
            "conf_syntax_{}",
            path.file_stem().unwrap().to_str().unwrap()
        ));

        let output = tmux.try_run(&["-f/dev/null", "start", ";", "source", "-n", path_str]);
        assert!(
            output.status.success(),
            "source -n failed for {}: {}",
            path_str,
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

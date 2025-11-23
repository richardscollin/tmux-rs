use std::env;
use std::path::Path;
use std::path::PathBuf;

const TERMINFO_DIRS: &[&str] = &[
    "/etc/terminfo",
    "/lib/terminfo",
    "/usr/share/terminfo",
    "/usr/lib/terminfo",
    "/boot/system/data/terminfo", // haiku
];

fn find_in_directory(term_name: &str, dir: &Path) -> Option<PathBuf> {
    // Standard layout - leaf directories use the first character of the terminal name.
    let first_char = term_name.chars().next()?;
    let filename = dir.join(first_char.to_string()).join(term_name);
    if filename.exists() {
        return Some(filename);
    }

    // Layout for systems with non-case-sensitive filesystems (MacOS, Windows) - leaf
    // directories use the first byte of the terminal name in hexadecimal form.
    let first_byte = term_name.as_bytes()[0];
    let first_byte_hex = format!("{first_byte:02x}");
    let filename = dir.join(first_byte_hex).join(term_name);
    if filename.exists() {
        return Some(filename);
    }

    None
}

fn search_directories() -> Vec<PathBuf> {
    let mut search_dirs = vec![];

    // Lazily evaluated iterator, consumed at most once.
    let mut default_dirs = TERMINFO_DIRS.iter().map(PathBuf::from);

    // Search the directory from the `TERMINFO` environment variable.
    if let Ok(dir) = env::var("TERMINFO") {
        search_dirs.push(PathBuf::from(&dir));
    }

    // Search `.terminfo` in the home directory.
    if let Some(home_dir) = env::home_dir() {
        let dir = home_dir.join(".terminfo");
        search_dirs.push(dir);
    }

    // Search colon separated directories from the `TERMINFO_DIRS`
    // environment variable.
    if let Ok(dirs) = env::var("TERMINFO_DIRS") {
        for dir in dirs.split(":") {
            if dir.is_empty() {
                // Empty directory means search the default locations.
                search_dirs.extend(&mut default_dirs);
            } else {
                search_dirs.push(PathBuf::from(dir));
            }
        }
    }

    // Search default terminfo locations (nothing is added if used already).
    search_dirs.extend(&mut default_dirs);

    search_dirs
}

/// Find terminfo database file for the terminal name
///
/// # Arguments
///
/// * `term_name` - terminal name.
///
/// Returns the file path is found, None if not found.
pub fn locate(term_name: &str) -> Option<PathBuf> {
    for dir in search_directories() {
        if let Some(file) = find_in_directory(term_name, &dir) {
            return Some(file);
        }
    }

    None
}

#[cfg(test)]
mod test {
    use std::fs::File;
    use std::fs::create_dir;
    use std::fs::exists;

    use tempdir::TempDir;

    use super::*;

    const TERM_NAME: &str = "no-such-terminal-123";

    #[test]
    fn empty_name() {
        assert_eq!(locate(""), None);
    }

    #[test]
    fn missing_file() {
        // Not using TERM_NAME to avoid race conditions - `temp_env::with_vars`
        // is serialized, but we are not using that function here.
        assert_eq!(locate("no-such-terminal-1"), None);
    }

    #[test]
    fn found_xterm() {
        let found_file = locate("xterm");
        assert!(found_file.is_some());
        assert!(exists(found_file.unwrap()).unwrap());
    }

    #[test]
    fn found_standard_layout_terminfo_dirs() {
        let tempdir = TempDir::new("terminfo-test").unwrap();
        let tempdir = tempdir.path();
        let leaf_dir = tempdir.join("n");
        let terminfo_file = leaf_dir.join(TERM_NAME);
        create_dir(leaf_dir).unwrap();
        File::create(&terminfo_file).unwrap();
        let terminfo_dirs = format!("foo:{}:bar", tempdir.display());

        temp_env::with_vars(
            [("TERMINFO_DIRS", Some(terminfo_dirs)), ("TERMINFO", None)],
            || {
                assert_eq!(locate(TERM_NAME), Some(terminfo_file));
            },
        );
    }

    #[test]
    fn found_hex_layout_terminfo_dirs() {
        let tempdir = TempDir::new("terminfo-test").unwrap();
        let tempdir = tempdir.path();
        let leaf_dir = tempdir.join("6e");
        let terminfo_file = leaf_dir.join(TERM_NAME);
        create_dir(leaf_dir).unwrap();
        File::create(&terminfo_file).unwrap();
        let terminfo_dirs = format!("foo:{}:bar", tempdir.display());

        temp_env::with_vars(
            [("TERMINFO_DIRS", Some(terminfo_dirs)), ("TERMINFO", None)],
            || {
                assert_eq!(locate(TERM_NAME), Some(terminfo_file));
            },
        );
    }

    #[test]
    fn found_standard_layout_terminfo_variable() {
        let tempdir = TempDir::new("terminfo-test").unwrap();
        let tempdir = tempdir.path();
        let leaf_dir = tempdir.join("n");
        let terminfo_file = leaf_dir.join(TERM_NAME);
        create_dir(leaf_dir).unwrap();
        File::create(&terminfo_file).unwrap();

        temp_env::with_vars(
            [("TERMINFO_DIRS", None), ("TERMINFO", Some(tempdir))],
            || {
                assert_eq!(locate(TERM_NAME), Some(terminfo_file));
            },
        );
    }

    #[test]
    fn dot_terminfo_standard_layout() {
        let tempdir = TempDir::new("terminfo-test").unwrap();
        let tempdir = tempdir.path();
        let dot_terminfo = tempdir.join(".terminfo");
        let leaf_dir = dot_terminfo.join("n");
        let terminfo_file = leaf_dir.join(TERM_NAME);
        create_dir(dot_terminfo).unwrap();
        create_dir(leaf_dir).unwrap();
        File::create(&terminfo_file).unwrap();

        temp_env::with_vars(
            [
                ("TERMINFO_DIRS", None),
                ("TERMINFO", None),
                ("HOME", Some(tempdir)),
            ],
            || {
                assert_eq!(locate(TERM_NAME), Some(terminfo_file));
            },
        );
    }

    #[test]
    fn search_order() {
        let expected_dirs: Vec<PathBuf> = [
            "/my/terminfo",
            "/home/user/.terminfo",
            "/my/terminfo1",
            "/my/terminfo2",
            "/etc/terminfo",
            "/lib/terminfo",
            "/usr/share/terminfo",
            "/usr/lib/terminfo",
            "/boot/system/data/terminfo",
        ]
        .iter()
        .map(PathBuf::from)
        .collect();

        temp_env::with_vars(
            [
                ("TERMINFO_DIRS", Some("/my/terminfo1:/my/terminfo2")),
                ("TERMINFO", Some("/my/terminfo")),
                ("HOME", Some("/home/user")),
            ],
            || {
                assert_eq!(search_directories(), expected_dirs);
            },
        );
    }

    #[test]
    fn search_order_with_empty_element() {
        let expected_dirs: Vec<PathBuf> = [
            "/my/terminfo",
            "/home/user/.terminfo",
            "/my/terminfo1",
            "/etc/terminfo",
            "/lib/terminfo",
            "/usr/share/terminfo",
            "/usr/lib/terminfo",
            "/boot/system/data/terminfo",
            "/my/terminfo2",
        ]
        .iter()
        .map(PathBuf::from)
        .collect();

        temp_env::with_vars(
            [
                ("TERMINFO_DIRS", Some("/my/terminfo1::/my/terminfo2")),
                ("TERMINFO", Some("/my/terminfo")),
                ("HOME", Some("/home/user")),
            ],
            || {
                assert_eq!(search_directories(), expected_dirs);
            },
        );
    }
}

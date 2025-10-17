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
    // Standard layout - leaf directories are the first character of the terminal name.
    let first_char = &term_name[0..1];
    let filename = dir.join(first_char).join(term_name);
    if Path::exists(&filename) {
        return Some(filename);
    }

    // Systems with non-case-sensitive filesystems (MacOS, Windows) - leaf directories use
    // hexadecimal representation of the first character of the terminal name.
    let first_char_hex = format!("{:2x}", term_name.as_bytes().first()?);
    let filename = dir.join(first_char_hex).join(term_name);
    if Path::exists(&filename) {
        return Some(filename);
    }

    None
}

fn search_directories() -> Vec<PathBuf> {
    let mut search_dirs = vec![];

    // Lazily evaluated iterator, consumed at most once
    let mut default_dirs = TERMINFO_DIRS.iter().map(PathBuf::from);

    // Search the directory from the `TERMINFO` environment variable
    if let Ok(dir) = env::var("TERMINFO") {
        search_dirs.push(PathBuf::from(&dir));
    }

    // Search `.terminfo` in the home directory
    if let Some(home_dir) = env::home_dir() {
        let dir = home_dir.join(".terminfo");
        search_dirs.push(dir);
    }

    // Search colon separated directories from the `TERMINFO_DIRS`
    // environment variable
    if let Ok(dirs) = env::var("TERMINFO_DIRS") {
        for dir in dirs.split(":") {
            if dir.is_empty() {
                // Empty directory means search the default locations
                search_dirs.extend(&mut default_dirs);
            } else {
                search_dirs.push(PathBuf::from(dir));
            }
        }
    }

    // Search hardcoded terminfo locations
    search_dirs.extend(&mut default_dirs);

    search_dirs
}

/// Find terminfo database file for the terminal name
pub fn find_database(term_name: &str) -> Option<PathBuf> {
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

    use tempdir::TempDir;

    use super::*;

    #[test]
    fn find_terminfo_file_posix() {
        let term_name = "no-such-terminal-123";
        let tempdir = TempDir::new("terminfo-test").unwrap();
        let tempdir = tempdir.path();
        let leaf_dir = tempdir.join("n");
        let terminfo_file = leaf_dir.join(term_name);
        create_dir(leaf_dir).unwrap();
        File::create(&terminfo_file).unwrap();

        let terminfo_dirs = format!("foo:{}:bar", tempdir.display());
        temp_env::with_var("TERMINFO_DIRS", Some(terminfo_dirs), || {
            let found_file = find_database(term_name);
            assert_eq!(found_file, Some(terminfo_file));
        });
    }

    #[test]
    fn find_terminfo_file_macos() {
        let term_name = "no-such-terminal-123";
        let tempdir = TempDir::new("terminfo-test").unwrap();
        let tempdir = tempdir.path();
        let leaf_dir = tempdir.join("6e");
        let terminfo_file = leaf_dir.join(term_name);
        create_dir(leaf_dir).unwrap();
        File::create(&terminfo_file).unwrap();

        let terminfo_dirs = format!("foo:{}:bar", tempdir.display());
        temp_env::with_var("TERMINFO_DIRS", Some(terminfo_dirs), || {
            let found_file = find_database(term_name);
            assert_eq!(found_file, Some(terminfo_file));
        });
    }

    #[test]
    fn find_terminfo_file_unsuccessful() {
        let term_name = "no-such-terminal-123";
        let tempdir = TempDir::new("terminfo-test").unwrap();
        let tempdir = tempdir.path();
        let leaf_dir = tempdir.join("x"); // not "n"
        let terminfo_file = leaf_dir.join(term_name);
        create_dir(leaf_dir).unwrap();
        File::create(&terminfo_file).unwrap();

        let terminfo_dirs = format!("foo:{}:bar", tempdir.display());
        temp_env::with_var("TERMINFO_DIRS", Some(terminfo_dirs), || {
            let found_file = find_database(term_name);
            assert_eq!(found_file, None);
        });
    }

    #[test]
    fn find_terminfo_file_terminfo_variable() {
        let term_name = "no-such-terminal-123";
        let tempdir = TempDir::new("terminfo-test").unwrap();
        let tempdir = tempdir.path();
        let leaf_dir = tempdir.join("n");
        let terminfo_file = leaf_dir.join(term_name);
        create_dir(leaf_dir).unwrap();
        File::create(&terminfo_file).unwrap();

        temp_env::with_var("TERMINFO", Some(tempdir), || {
            let found_file = find_database(term_name);
            assert_eq!(found_file, Some(terminfo_file));
        });
    }

    #[test]
    fn find_terminfo_file_dot_terminfo() {
        let term_name = "no-such-terminal-123";
        let tempdir = TempDir::new("terminfo-test").unwrap();
        let tempdir = tempdir.path();
        let dot_terminfo = tempdir.join(".terminfo");
        let leaf_dir = dot_terminfo.join("n");
        let terminfo_file = leaf_dir.join(term_name);
        create_dir(dot_terminfo).unwrap();
        create_dir(leaf_dir).unwrap();
        File::create(&terminfo_file).unwrap();

        temp_env::with_var("HOME", Some(tempdir), || {
            let found_file = find_database(term_name);
            assert_eq!(found_file, Some(terminfo_file));
        });
    }
}

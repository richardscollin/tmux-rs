#[cfg(unix)]
mod regress {
    use std::io::Write;
    use std::path::PathBuf;
    use std::process::{Command, Output, Stdio};

    pub struct TmuxServer {
        binary: PathBuf,
        socket: String,
    }

    impl TmuxServer {
        pub fn new(test_name: &str) -> Self {
            let binary = PathBuf::from(Self::binary_path());
            let socket = format!("regress_{}", test_name);
            let server = Self { binary, socket };
            server.kill_server();
            server
        }

        /// Path to the tmux-rs binary built by cargo.
        pub fn binary_path() -> &'static str {
            env!("CARGO_BIN_EXE_tmux-rs")
        }

        /// The -L socket name for this server instance.
        pub fn socket(&self) -> &str {
            &self.socket
        }

        pub fn regress_dir() -> PathBuf {
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("tests")
                .join("regress")
                .join("data")
        }

        fn build_cmd(&self, args: &[&str]) -> Command {
            let mut cmd = Command::new(&self.binary);
            cmd.arg("-L").arg(&self.socket);
            cmd.args(args);
            cmd.env("PATH", "/bin:/usr/bin:/usr/local/bin");
            cmd.env("TERM", "screen");
            cmd.stdin(Stdio::null());
            cmd
        }

        /// Run tmux command. Prepends -L<socket>. Returns stdout as String.
        /// Panics on non-zero exit.
        pub fn run(&self, args: &[&str]) -> String {
            let output = self.build_cmd(args).output().expect("failed to run tmux");
            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                let stdout = String::from_utf8_lossy(&output.stdout);
                panic!(
                    "tmux {:?} failed with {}\nstdout: {}\nstderr: {}",
                    args, output.status, stdout, stderr
                );
            }
            String::from_utf8(output.stdout).expect("non-utf8 stdout")
        }

        /// Like run() but returns stdout as raw bytes (for binary comparison).
        pub fn run_bytes(&self, args: &[&str]) -> Vec<u8> {
            let output = self.build_cmd(args).output().expect("failed to run tmux");
            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                panic!(
                    "tmux {:?} failed with {}\nstderr: {}",
                    args, output.status, stderr
                );
            }
            output.stdout
        }

        /// Run tmux command, return full Output without panicking.
        pub fn try_run(&self, args: &[&str]) -> Output {
            self.build_cmd(args).output().expect("failed to run tmux")
        }

        /// Run tmux command with data piped to stdin. Returns full Output.
        pub fn run_with_stdin(&self, args: &[&str], stdin_data: &[u8]) -> Output {
            let mut cmd = self.build_cmd(args);
            cmd.stdin(Stdio::piped());
            cmd.stdout(Stdio::piped());
            cmd.stderr(Stdio::piped());
            let mut child = cmd.spawn().expect("failed to spawn tmux");
            child
                .stdin
                .as_mut()
                .expect("failed to open stdin")
                .write_all(stdin_data)
                .expect("failed to write to stdin");
            child.wait_with_output().expect("failed to wait for tmux")
        }

        /// Convenience: display-message -p <format>, returns trimmed stdout.
        pub fn display(&self, format: &str) -> String {
            let out = self.run(&["display-message", "-p", format]);
            out.trim_end_matches('\n').to_string()
        }

        /// Kill the server (also called on Drop).
        pub fn kill_server(&self) {
            let _ = self.build_cmd(&["kill-server"]).output();
            // Wait for the server to fully exit. After kill-server returns,
            // the server process may still be shutting down with its listening
            // socket open. A new client connecting during this window would get
            // "server exited unexpectedly".
            for _ in 0..200 {
                let out = self.build_cmd(&["has-session"]).output();
                match out {
                    Ok(o) if o.status.success() => {}
                    _ => break,
                }
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
        }

        /// Write content to a temp file and return its path.
        pub fn write_temp(&self, content: &str) -> TempFile {
            let mut f = tempfile::NamedTempFile::new().expect("failed to create tempfile");
            f.write_all(content.as_bytes())
                .expect("failed to write tempfile");
            f.flush().expect("failed to flush tempfile");
            TempFile(f)
        }
    }

    impl Drop for TmuxServer {
        fn drop(&mut self) {
            self.kill_server();
        }
    }

    pub struct TempFile(tempfile::NamedTempFile);

    impl TempFile {
        pub fn path(&self) -> &std::path::Path {
            self.0.path()
        }

        pub fn path_str(&self) -> &str {
            self.0.path().to_str().expect("non-utf8 path")
        }

        pub fn read_to_string(&self) -> String {
            std::fs::read_to_string(self.path()).unwrap_or_default()
        }

        pub fn read_to_bytes(&self) -> Vec<u8> {
            std::fs::read(self.path()).unwrap_or_default()
        }
    }

    pub fn sleep_ms(ms: u64) {
        std::thread::sleep(std::time::Duration::from_millis(ms));
    }

    pub fn sleep_secs(secs: u64) {
        std::thread::sleep(std::time::Duration::from_secs(secs));
    }

    mod am_terminal;
    mod bind_key;
    mod break_pane;
    mod capture_pane;
    mod combine;
    mod command_order;
    mod conf_syntax;
    mod control_client;
    mod copy_mode;
    mod cursor;
    mod detach_client;
    mod format_strings;
    mod has_session;
    mod if_shell;
    mod input_keys;
    mod join_pane;
    mod kill_session;
    mod list_buffers;
    mod list_clients;
    mod list_sessions;
    mod list_windows;
    mod load_buffer;
    mod lock_server;
    mod move_window;
    mod new_session;
    mod new_window;
    mod osc;
    mod pipe_pane;
    mod rename_window;
    mod resize_pane;
    mod respawn_pane;
    mod respawn_window;
    mod rotate_window;
    mod run_shell;
    mod server_access;
    mod set_buffer;
    mod show_environment;
    mod show_messages;
    mod show_prompt_history;
    mod style_trim;
    mod swap_pane;
    mod swap_window;
    mod tty_keys;
    mod utf8;
    mod window_clock;
}

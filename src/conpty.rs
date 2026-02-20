//! ConPTY (Windows Pseudo Console) wrapper for spawning shell processes.
//!
//! On Unix, tmux uses forkpty() to create a PTY + fork. On Windows, we use
//! ConPTY (CreatePseudoConsole) + CreateProcessW. Since ConPTY gives us pipe
//! HANDLEs (not sockets), and the event loop uses WSAPoll (which only works
//! with Winsock sockets), we bridge them with a socketpair relay:
//!
//! ```text
//! ConPTY output_read ──[reader thread]──→ socketpair[1]
//!                                          socketpair[0] = wp->fd (bufferevent polls this)
//! ConPTY input_write ←──[writer thread]── socketpair[1]
//! ```
#![cfg(target_os = "windows")]

use std::collections::HashMap;
use std::ffi::c_int;
use std::mem::{size_of, zeroed};
use std::sync::Mutex;
use std::thread::JoinHandle;

use windows_sys::Win32::Foundation::{
    CloseHandle, GetLastError, HANDLE, INVALID_HANDLE_VALUE, WAIT_OBJECT_0,
};
use windows_sys::Win32::Storage::FileSystem::{ReadFile, WriteFile};
use windows_sys::Win32::System::Console::{
    COORD, ClosePseudoConsole, CreatePseudoConsole, HPCON, ResizePseudoConsole,
};
use windows_sys::Win32::System::Pipes::CreatePipe;
use windows_sys::Win32::System::Threading::{
    CREATE_UNICODE_ENVIRONMENT, CreateProcessW, EXTENDED_STARTUPINFO_PRESENT, GetExitCodeProcess,
    InitializeProcThreadAttributeList, PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE, PROCESS_INFORMATION,
    STARTF_USESTDHANDLES, STARTUPINFOEXW, STARTUPINFOW, TerminateProcess,
    UpdateProcThreadAttribute, WaitForSingleObject,
};

/// Encode a Rust string as a null-terminated UTF-16 vector.
fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Build a UTF-16 environment block by merging tmux environ vars on top of
/// the current process's environment. This ensures critical system variables
/// (SystemRoot, PATH, etc.) are preserved while tmux-specific overrides apply.
///
/// Uses case-insensitive key matching since Windows env vars are case-insensitive.
fn build_env_block(vars: &[(String, String)]) -> Vec<u16> {
    use std::collections::HashMap;
    const STRIP_PREFIXES: &[&str] = &["WT_", "TERM_PROGRAM", "TERM_SESSION"];

    // Map from UPPERCASE key → (original_key, value) for case-insensitive dedup
    let mut merged: HashMap<String, (String, String)> = HashMap::new();

    // Start from the current process environment
    for (key, value) in std::env::vars() {
        if STRIP_PREFIXES.iter().any(|p| key.starts_with(p)) {
            continue;
        }
        merged.insert(key.to_uppercase(), (key, value));
    }

    // Overlay tmux environ variables (case-insensitive replace)
    for (key, value) in vars {
        merged.insert(key.to_uppercase(), (key.clone(), value.clone()));
    }

    // Windows requires env block to be sorted by variable name (case-insensitive)
    let mut entries: Vec<_> = merged.into_values().collect();
    entries.sort_by(|(a, _), (b, _)| a.to_uppercase().cmp(&b.to_uppercase()));

    // Build the block
    let mut block: Vec<u16> = Vec::new();
    for (key, value) in &entries {
        let entry = format!("{key}={value}");
        block.extend(entry.encode_utf16());
        block.push(0);
    }
    block.push(0); // double-null terminator
    block
}

/// Build environment block from current process environment,
/// filtering out parent-terminal variables.
fn build_clean_env_block() -> Vec<u16> {
    const STRIP_PREFIXES: &[&str] = &["WT_", "TERM_PROGRAM", "TERM_SESSION"];

    let mut block: Vec<u16> = Vec::new();
    for (key, value) in std::env::vars() {
        if STRIP_PREFIXES.iter().any(|p| key.starts_with(p)) {
            continue;
        }
        let entry = format!("{key}={value}");
        block.extend(entry.encode_utf16());
        block.push(0);
    }
    block.push(0);
    block
}

struct ConPtyInner {
    hpc: HPCON,
    input_write: HANDLE,
    output_read: HANDLE,
    process_handle: HANDLE,
    process_id: u32,
    relay_socket: c_int, // socketpair[1] — owned by relay threads
    reader_thread: Option<JoinHandle<()>>,
    writer_thread: Option<JoinHandle<()>>,
}

// HANDLE and socket are Send-safe for our dedicated thread usage.
unsafe impl Send for ConPtyInner {}

static CONPTY_TABLE: Mutex<Option<HashMap<u32, ConPtyInner>>> = Mutex::new(None);

fn conpty_table() -> std::sync::MutexGuard<'static, Option<HashMap<u32, ConPtyInner>>> {
    let mut guard = CONPTY_TABLE.lock().unwrap();
    if guard.is_none() {
        *guard = Some(HashMap::new());
    }
    guard
}

/// Spawn a process under ConPTY and return (fd, pid).
///
/// `fd` is a socket that can be polled with WSAPoll (via bufferevent).
/// `pid` is the Windows process ID.
///
/// `cmd` is the command line (e.g. "pwsh.exe" or "cmd.exe").
/// `cwd` is the working directory (or None for default).
/// `env_vars` if Some, provides environment variables; otherwise uses current env.
pub fn conpty_spawn(
    cmd: &str,
    cols: u16,
    rows: u16,
    cwd: Option<&str>,
    env_vars: Option<&[(String, String)]>,
) -> Result<(c_int, u32), String> {
    unsafe {
        // 1. Create two anonymous pipes
        let mut input_read: HANDLE = std::ptr::null_mut();
        let mut input_write: HANDLE = std::ptr::null_mut();
        let mut output_read: HANDLE = std::ptr::null_mut();
        let mut output_write: HANDLE = std::ptr::null_mut();

        if CreatePipe(&mut input_read, &mut input_write, std::ptr::null(), 0) == 0 {
            return Err(format!("CreatePipe (input) failed: {}", GetLastError()));
        }
        if CreatePipe(&mut output_read, &mut output_write, std::ptr::null(), 0) == 0 {
            CloseHandle(input_read);
            CloseHandle(input_write);
            return Err(format!("CreatePipe (output) failed: {}", GetLastError()));
        }

        // 2. Create pseudo console
        let size = COORD {
            X: cols as i16,
            Y: rows as i16,
        };
        let mut hpc: HPCON = 0;
        let hr = CreatePseudoConsole(size, input_read, output_write, 0, &mut hpc);
        if hr < 0 {
            CloseHandle(input_read);
            CloseHandle(input_write);
            CloseHandle(output_read);
            CloseHandle(output_write);
            return Err(format!("CreatePseudoConsole failed: HRESULT {:#x}", hr));
        }

        // 3. Close ConPTY-side pipe ends — ConPTY owns them now
        CloseHandle(input_read);
        CloseHandle(output_write);

        // 4. Set up STARTUPINFOEXW with proc thread attribute list
        let mut attr_size: usize = 0;
        InitializeProcThreadAttributeList(std::ptr::null_mut(), 1, 0, &mut attr_size);
        let mut attr_buf: Vec<u8> = vec![0u8; attr_size];
        let attr_list_ptr = attr_buf.as_mut_ptr().cast();

        if InitializeProcThreadAttributeList(attr_list_ptr, 1, 0, &mut attr_size) == 0 {
            ClosePseudoConsole(hpc);
            CloseHandle(input_write);
            CloseHandle(output_read);
            return Err(format!(
                "InitializeProcThreadAttributeList failed: {}",
                GetLastError()
            ));
        }

        if UpdateProcThreadAttribute(
            attr_list_ptr,
            0,
            PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE as usize,
            hpc as *const _,
            size_of::<HPCON>(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        ) == 0
        {
            ClosePseudoConsole(hpc);
            CloseHandle(input_write);
            CloseHandle(output_read);
            return Err(format!(
                "UpdateProcThreadAttribute failed: {}",
                GetLastError()
            ));
        }

        // Build STARTUPINFOEXW with typed struct — compiler-verified layout
        // Don't set STARTF_USESTDHANDLES — ConPTY provides stdio via the pseudo console.
        let mut si: STARTUPINFOEXW = zeroed();
        si.StartupInfo.cb = size_of::<STARTUPINFOEXW>() as u32;
        si.lpAttributeList = attr_list_ptr;

        // 5. Create the process
        let mut cmd_wide = to_wide(cmd);
        let cwd_wide = cwd.map(to_wide);

        let env_block = if let Some(vars) = env_vars {
            build_env_block(vars)
        } else {
            build_clean_env_block()
        };

        let mut pi: PROCESS_INFORMATION = zeroed();

        let result = CreateProcessW(
            std::ptr::null(),
            cmd_wide.as_mut_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            0, // don't inherit handles
            EXTENDED_STARTUPINFO_PRESENT | CREATE_UNICODE_ENVIRONMENT,
            env_block.as_ptr().cast(),
            cwd_wide
                .as_ref()
                .map(|w| w.as_ptr())
                .unwrap_or(std::ptr::null()),
            &si as *const STARTUPINFOEXW as *const STARTUPINFOW,
            &mut pi,
        );

        if result == 0 {
            let err = GetLastError();
            ClosePseudoConsole(hpc);
            CloseHandle(input_write);
            CloseHandle(output_read);
            return Err(format!("CreateProcessW failed: {err}"));
        }

        // Close the thread handle — we only need the process handle
        CloseHandle(pi.hThread);

        // 6. Create a socketpair for the relay
        let mut sv: [c_int; 2] = [-1, -1];
        if crate::libc::socketpair(
            crate::libc::AF_UNIX,
            crate::libc::SOCK_STREAM,
            crate::libc::PF_UNSPEC,
            sv.as_mut_ptr(),
        ) != 0
        {
            ClosePseudoConsole(hpc);
            CloseHandle(input_write);
            CloseHandle(output_read);
            CloseHandle(pi.hProcess);
            return Err("socketpair failed for ConPTY relay".into());
        }

        let pane_fd = sv[0]; // goes to wp->fd
        let relay_fd = sv[1]; // used by relay threads

        // 7. Spawn reader thread: ConPTY output_read → send(relay_fd)
        let reader_output_read = output_read as usize;
        let reader_relay_fd = relay_fd;
        let reader_thread = std::thread::Builder::new()
            .name("conpty-reader".into())
            .spawn(move || {
                conpty_reader_thread(reader_output_read, reader_relay_fd);
            })
            .map_err(|e| format!("Failed to spawn reader thread: {e}"))?;

        // 8. Spawn writer thread: recv(relay_fd) → ConPTY input_write
        let writer_input_write = input_write as usize;
        let writer_relay_fd = relay_fd;
        let writer_thread = std::thread::Builder::new()
            .name("conpty-writer".into())
            .spawn(move || {
                conpty_writer_thread(writer_relay_fd, writer_input_write);
            })
            .map_err(|e| format!("Failed to spawn writer thread: {e}"))?;

        // 9. Store in global table
        let inner = ConPtyInner {
            hpc,
            input_write,
            output_read,
            process_handle: pi.hProcess,
            process_id: pi.dwProcessId,
            relay_socket: relay_fd,
            reader_thread: Some(reader_thread),
            writer_thread: Some(writer_thread),
        };

        conpty_table()
            .as_mut()
            .unwrap()
            .insert(pi.dwProcessId, inner);

        Ok((pane_fd, pi.dwProcessId))
    }
}

/// Reader thread: loops ReadFile(output_read) → send(relay_fd).
/// Exits when ReadFile fails (ConPTY closed / process exited).
///
/// Takes `output_read` as `usize` (cast from HANDLE) because HANDLE is
/// `*mut c_void` which isn't `Send`. The raw value is safe to use across
/// threads — Windows HANDLEs are process-global.
fn conpty_reader_thread(output_read: usize, relay_fd: c_int) {
    let output_read = output_read as HANDLE;
    let mut buf = [0u8; 8192];
    loop {
        let mut bytes_read: u32 = 0;
        let ok = unsafe {
            ReadFile(
                output_read,
                buf.as_mut_ptr(),
                buf.len() as u32,
                &mut bytes_read,
                std::ptr::null_mut(),
            )
        };
        if ok == 0 || bytes_read == 0 {
            break;
        }
        // Send to the socket
        let mut offset = 0usize;
        while offset < bytes_read as usize {
            let sent = unsafe {
                crate::libc::send(
                    relay_fd,
                    buf[offset..].as_ptr().cast(),
                    (bytes_read as usize - offset) as _,
                    0,
                )
            };
            if sent <= 0 {
                return; // socket closed
            }
            offset += sent as usize;
        }
    }
    // Signal EOF to the pane by shutting down our end
    unsafe {
        crate::libc::shutdown(relay_fd, crate::libc::SHUT_WR);
    }
}

/// Writer thread: loops recv(relay_fd) → WriteFile(input_write).
/// Exits when recv returns 0 or error (pane fd closed).
fn conpty_writer_thread(relay_fd: c_int, input_write: usize) {
    let input_write = input_write as HANDLE;
    let mut buf = [0u8; 8192];
    loop {
        let received =
            unsafe { crate::libc::recv(relay_fd, buf.as_mut_ptr().cast(), buf.len() as _, 0) };
        if received <= 0 {
            break;
        }
        let mut offset = 0usize;
        while offset < received as usize {
            let mut bytes_written: u32 = 0;
            let ok = unsafe {
                WriteFile(
                    input_write,
                    buf[offset..].as_ptr(),
                    (received as usize - offset) as u32,
                    &mut bytes_written,
                    std::ptr::null_mut(),
                )
            };
            if ok == 0 || bytes_written == 0 {
                return;
            }
            offset += bytes_written as usize;
        }
    }
}

/// Resize the ConPTY for a given process.
pub fn conpty_resize(pid: u32, cols: u16, rows: u16) -> Result<(), String> {
    let table = conpty_table();
    let entry = table
        .as_ref()
        .unwrap()
        .get(&pid)
        .ok_or_else(|| format!("conpty_resize: pid {pid} not found"))?;
    let size = COORD {
        X: cols as i16,
        Y: rows as i16,
    };
    let hr = unsafe { ResizePseudoConsole(entry.hpc, size) };
    if hr < 0 {
        Err(format!("ResizePseudoConsole failed: HRESULT {hr:#x}"))
    } else {
        Ok(())
    }
}

/// Check if a ConPTY process has exited. Returns Some(exit_code) if dead, None if still running.
pub fn conpty_check_exit(pid: u32) -> Option<u32> {
    let table = conpty_table();
    let entry = table.as_ref().unwrap().get(&pid)?;
    unsafe {
        let result = WaitForSingleObject(entry.process_handle, 0);
        if result == WAIT_OBJECT_0 {
            let mut exit_code: u32 = 0;
            GetExitCodeProcess(entry.process_handle, &mut exit_code);
            Some(exit_code)
        } else {
            None
        }
    }
}

/// Clean up a ConPTY entry: close handles, signal threads to stop.
pub fn conpty_cleanup(pid: u32) {
    let mut table = conpty_table();
    if let Some(mut inner) = table.as_mut().unwrap().remove(&pid) {
        unsafe {
            // Close the pseudo console first (if present) — this will cause
            // ReadFile in the reader thread to fail, breaking its loop.
            if inner.hpc != 0 {
                ClosePseudoConsole(inner.hpc);
            }

            // Close the relay socket to unblock the writer thread's recv().
            crate::libc::close(inner.relay_socket);

            // Close pipe handles
            if !inner.input_write.is_null() {
                CloseHandle(inner.input_write);
            }
            CloseHandle(inner.output_read);

            // Close process handle
            CloseHandle(inner.process_handle);
        }

        // Join threads (they should exit quickly now)
        if let Some(t) = inner.reader_thread.take() {
            let _ = t.join();
        }
        if let Some(t) = inner.writer_thread.take() {
            let _ = t.join();
        }
    }
}

/// Spawn a process with piped stdin/stdout (no PTY). Returns (fd, pid).
/// Used for non-PTY jobs (background commands, status-line commands, etc).
/// The returned fd is a socket connected to the child's stdout via a relay thread.
pub fn process_spawn_piped(cmd: &str, cwd: Option<&str>) -> Result<(c_int, u32), String> {
    unsafe {
        // Create pipes for stdout capture
        let mut stdout_read: HANDLE = std::ptr::null_mut();
        let mut stdout_write: HANDLE = std::ptr::null_mut();
        if CreatePipe(&mut stdout_read, &mut stdout_write, std::ptr::null(), 0) == 0 {
            return Err(format!("CreatePipe failed: {}", GetLastError()));
        }

        // Build STARTUPINFOW (not EX — no ConPTY needed)
        let mut si: STARTUPINFOW = zeroed();
        si.cb = size_of::<STARTUPINFOW>() as u32;
        si.dwFlags = STARTF_USESTDHANDLES;
        si.hStdInput = std::ptr::null_mut();
        si.hStdOutput = stdout_write;
        si.hStdError = stdout_write;

        let mut cmd_wide = to_wide(cmd);
        let cwd_wide = cwd.map(to_wide);
        let env_block = build_clean_env_block();
        let mut pi: PROCESS_INFORMATION = zeroed();

        // Note: bInheritHandles must be TRUE (1) for piped handles to work
        let result = CreateProcessW(
            std::ptr::null(),
            cmd_wide.as_mut_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            1, // inherit handles
            CREATE_UNICODE_ENVIRONMENT,
            env_block.as_ptr().cast(),
            cwd_wide
                .as_ref()
                .map(|w| w.as_ptr())
                .unwrap_or(std::ptr::null()),
            &si,
            &mut pi,
        );

        // Close the write end of the pipe — child owns it now
        CloseHandle(stdout_write);

        if result == 0 {
            let err = GetLastError();
            CloseHandle(stdout_read);
            return Err(format!("CreateProcessW failed: {err}"));
        }

        CloseHandle(pi.hThread);

        // Create socketpair relay for stdout_read → socket
        let mut sv: [c_int; 2] = [-1, -1];
        if crate::libc::socketpair(
            crate::libc::AF_UNIX,
            crate::libc::SOCK_STREAM,
            crate::libc::PF_UNSPEC,
            sv.as_mut_ptr(),
        ) != 0
        {
            CloseHandle(stdout_read);
            CloseHandle(pi.hProcess);
            return Err("socketpair failed for piped relay".into());
        }

        let pane_fd = sv[0];
        let relay_fd = sv[1];

        // Spawn reader thread only (no writer — child stdin isn't connected)
        let reader_output_read = stdout_read as usize;
        let reader_relay_fd = relay_fd;
        let reader_thread = std::thread::Builder::new()
            .name("job-reader".into())
            .spawn(move || {
                conpty_reader_thread(reader_output_read, reader_relay_fd);
            })
            .map_err(|e| format!("Failed to spawn reader thread: {e}"))?;

        // Store in table for cleanup
        let inner = ConPtyInner {
            hpc: 0, // no pseudo console for piped jobs
            input_write: std::ptr::null_mut(),
            output_read: stdout_read,
            process_handle: pi.hProcess,
            process_id: pi.dwProcessId,
            relay_socket: relay_fd,
            reader_thread: Some(reader_thread),
            writer_thread: None,
        };

        conpty_table()
            .as_mut()
            .unwrap()
            .insert(pi.dwProcessId, inner);

        Ok((pane_fd, pi.dwProcessId))
    }
}

/// Terminate a ConPTY process.
pub fn conpty_kill(pid: u32) {
    let table = conpty_table();
    if let Some(entry) = table.as_ref().unwrap().get(&pid) {
        unsafe {
            TerminateProcess(entry.process_handle, 1);
        }
    }
}

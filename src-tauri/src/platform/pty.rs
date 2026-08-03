use std::collections::HashMap;
use std::io::{Read, Write};
use std::sync::Mutex;

use portable_pty::{
    Child, ChildKiller, CommandBuilder, MasterPty, NativePtySystem, PtySize, PtySystem,
};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use uuid::Uuid;

const DEFAULT_SSH_PORT: u16 = 22;
const INITIAL_COLS: u16 = 80;
const INITIAL_ROWS: u16 = 24;

#[derive(Debug)]
pub struct PtyError(pub String);

impl std::fmt::Display for PtyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

struct ActiveSession {
    writer: Box<dyn Write + Send>,
    master: Box<dyn MasterPty + Send>,
    killer: Box<dyn ChildKiller + Send + Sync>,
}

/// Tracks live in-app terminal sessions (Tauri-managed state). Each session
/// is one `ssh.exe` child process attached to a Windows ConPTY pseudo-
/// console via `portable-pty`, the same trusted local SSH client every
/// other SSH-backed feature in this app uses -- there is no embedded SSH
/// library, and no arbitrary command ever reaches the child process beyond
/// the fixed `ssh -tt [-p <port>] <user>@<host>` invocation built here.
#[derive(Default)]
pub struct PtySessionManager {
    sessions: Mutex<HashMap<String, ActiveSession>>,
}

#[derive(Clone, Serialize)]
struct TerminalExitPayload {
    #[serde(rename = "exitCode")]
    exit_code: Option<u32>,
}

impl PtySessionManager {
    /// Spawns a new `ssh.exe` session in a pseudo-console and starts a
    /// background reader thread that streams its output to the frontend
    /// via a per-session `terminal://output:<sessionId>` event, and emits
    /// `terminal://exit:<sessionId>` once the process ends (from either
    /// side: the user closing the session or the remote side hanging up).
    pub fn open(
        &self,
        app: &AppHandle,
        host: &str,
        port: u16,
        username: &str,
    ) -> Result<String, PtyError> {
        let pty_system = NativePtySystem::default();
        let pair = pty_system
            .openpty(PtySize {
                rows: INITIAL_ROWS,
                cols: INITIAL_COLS,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|err| PtyError(err.to_string()))?;

        let mut cmd = CommandBuilder::new("ssh.exe");
        cmd.arg("-tt");
        // Matches the same 5s connect timeout every other SSH-backed
        // feature in this app uses (infrastructure/ssh/openssh.rs); without
        // it, an unreachable device leaves the session sitting on the OS's
        // own (much longer, sometimes multi-minute) TCP connect timeout
        // with the terminal showing nothing but "Connecting...".
        cmd.arg("-o");
        cmd.arg("ConnectTimeout=5");
        if port != DEFAULT_SSH_PORT {
            cmd.arg("-p");
            cmd.arg(port.to_string());
        }
        cmd.arg(format!("{username}@{host}"));

        let child = pair
            .slave
            .spawn_command(cmd)
            .map_err(|err| PtyError(err.to_string()))?;
        drop(pair.slave);

        let killer = child.clone_killer();
        let reader = pair
            .master
            .try_clone_reader()
            .map_err(|err| PtyError(err.to_string()))?;
        let writer = pair
            .master
            .take_writer()
            .map_err(|err| PtyError(err.to_string()))?;

        let session_id = Uuid::new_v4().to_string();

        self.sessions.lock().unwrap().insert(
            session_id.clone(),
            ActiveSession {
                writer,
                master: pair.master,
                killer,
            },
        );

        spawn_reader_thread(app.clone(), session_id.clone(), reader);
        spawn_waiter_thread(app.clone(), session_id.clone(), child);

        Ok(session_id)
    }

    pub fn write(&self, session_id: &str, data: &str) -> Result<(), PtyError> {
        let mut sessions = self.sessions.lock().unwrap();
        let session = sessions
            .get_mut(session_id)
            .ok_or_else(|| PtyError("terminal session not found".into()))?;
        session
            .writer
            .write_all(data.as_bytes())
            .map_err(|err| PtyError(err.to_string()))
    }

    pub fn resize(&self, session_id: &str, cols: u16, rows: u16) -> Result<(), PtyError> {
        let sessions = self.sessions.lock().unwrap();
        let session = sessions
            .get(session_id)
            .ok_or_else(|| PtyError("terminal session not found".into()))?;
        session
            .master
            .resize(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|err| PtyError(err.to_string()))
    }

    /// Terminates the session's ssh.exe process. The reader thread's own
    /// EOF/exit handling removes the session from the map and emits the
    /// exit event -- this only needs to trigger that, not duplicate it.
    pub fn close(&self, session_id: &str) -> Result<(), PtyError> {
        let mut sessions = self.sessions.lock().unwrap();
        let session = sessions
            .get_mut(session_id)
            .ok_or_else(|| PtyError("terminal session not found".into()))?;
        session.killer.kill().map_err(|err| PtyError(err.to_string()))
    }
}

/// Streams pty output to the frontend until the pty closes. On Windows,
/// ConPTY only delivers EOF on this read once its underlying console
/// object is actually torn down -- which happens when the session's
/// `ActiveSession` (holding `master`) is dropped, not merely when the
/// child process exits. That drop is `spawn_waiter_thread`'s job, so this
/// loop's `Ok(0)`/`Err` exit is really "the waiter thread has finished
/// cleaning up the session," not "the child exited" (those two things can
/// be milliseconds apart, but they are not the same event).
fn spawn_reader_thread(app: AppHandle, session_id: String, mut reader: Box<dyn Read + Send>) {
    std::thread::spawn(move || {
        let output_event = format!("terminal://output:{session_id}");
        let mut buffer = [0u8; 4096];
        loop {
            match reader.read(&mut buffer) {
                Ok(0) => break,
                Ok(n) => {
                    let text = String::from_utf8_lossy(&buffer[..n]).into_owned();
                    let _ = app.emit(&output_event, text);
                }
                Err(_) => break,
            }
        }
    });
}

/// Blocks until the child process exits (for any reason: the remote side
/// hanging up, the user running `exit`, or `PtySessionManager::close`
/// killing it), then emits `terminal://exit:<sessionId>` and removes the
/// session from the map. Removing it drops the session's `master`, which
/// closes the ConPTY and is what unblocks the reader thread's now-stale
/// blocking read -- see `spawn_reader_thread`'s doc comment.
fn spawn_waiter_thread(app: AppHandle, session_id: String, mut child: Box<dyn Child + Send + Sync>) {
    std::thread::spawn(move || {
        let exit_code = child.wait().ok().map(|status| status.exit_code());
        let _ = app.emit(
            &format!("terminal://exit:{session_id}"),
            TerminalExitPayload { exit_code },
        );

        if let Some(manager) = app.try_state::<PtySessionManager>() {
            manager.sessions.lock().unwrap().remove(&session_id);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Sanity-checks that ConPTY + portable-pty actually spawns a process
    /// and delivers its output back through the pseudo-console on this
    /// machine. The rest of this module's API is shaped around a live
    /// Tauri `AppHandle` (for emitting events) and isn't practical to unit
    /// test directly, so this exercises the exact same underlying
    /// mechanism (NativePtySystem::openpty + CommandBuilder +
    /// SlavePty::spawn_command + MasterPty::try_clone_reader) against a
    /// trivial command instead of ssh.exe.
    ///
    /// Ignored by default: spawning through a real ConPTY console takes on
    /// the order of 30+ seconds in this environment (most likely Windows
    /// scanning the freshly-built test binary before letting it spawn a
    /// child process, not this test's own logic -- a plain `cargo build`
    /// of the same binary is fast), which is much too slow for the
    /// otherwise sub-second default suite. Run explicitly with
    /// `cargo test -- --ignored` when touching this module.
    #[test]
    #[ignore]
    fn native_pty_spawns_a_process_and_captures_its_output() {
        let pty_system = NativePtySystem::default();
        let pair = pty_system
            .openpty(PtySize {
                rows: 24,
                cols: 80,
                pixel_width: 0,
                pixel_height: 0,
            })
            .expect("openpty should succeed");

        // whoami.exe (not e.g. cmd.exe /c echo, tried first) matches
        // portable-pty's own maintained example for this exact scenario:
        // cmd.exe/conhost's console-capability negotiation on attach
        // emits a device-status-report escape query (`ESC[6n`) that nothing
        // in this test responds to, and the output never advances past it.
        // A real terminal emulator (xterm.js, in this app's actual
        // frontend) answers that query automatically; a bare test harness
        // doesn't, so whoami.exe -- which doesn't trigger it -- is the
        // reliable choice here.
        let cmd = CommandBuilder::new("whoami.exe");
        let mut child = pair
            .slave
            .spawn_command(cmd)
            .expect("spawning whoami.exe in the pty should succeed");
        drop(pair.slave);

        let mut reader = pair
            .master
            .try_clone_reader()
            .expect("cloning the pty reader should succeed");

        // Reading and waiting must happen concurrently, on separate
        // threads -- this is not optional. `cmd.exe`'s write of its
        // output can block on ConPTY's pipe buffer until something reads
        // it, so waiting for the child to exit *before* reading deadlocks:
        // the child can't finish writing and exit until it's read, but
        // nothing reads until the child has already exited. This mirrors
        // PtySessionManager::open() running its own reader and waiter as
        // two independent threads rather than sequential steps.
        let reader_handle = std::thread::spawn(move || {
            let mut output = String::new();
            let _ = reader.read_to_string(&mut output);
            output
        });

        // Taking (and here, immediately dropping) the writer matters even
        // though nothing is written to it: portable-pty's own examples
        // warn that not doing so risks a deadlock ("It is important to
        // take the writer even if you don't send anything to its stdin
        // so that EOF can be generated, otherwise you risk deadlocking
        // yourself") -- confirmed the hard way, this test hung
        // indefinitely without this line. The real PtySessionManager
        // never hits this: it always takes the writer (to forward the
        // user's keystrokes) and keeps it alive for the session's
        // lifetime rather than a one-shot command with no input.
        drop(
            pair.master
                .take_writer()
                .expect("taking the pty writer should succeed"),
        );

        child.wait().expect("waiting for the child should succeed");
        // Dropping `master` (as the real waiter thread does by removing
        // the session from the map) lets ConPTY deliver EOF, so the
        // reader thread above can return.
        drop(pair.master);

        let output = reader_handle.join().expect("reader thread should not panic");

        // Not asserting the exact username (environment-dependent, and
        // not the point of this test): the point is that ConPTY delivered
        // *some* real output back through the pty at all, proving the
        // whole spawn/read/wait/teardown mechanism works on this machine.
        assert!(
            !output.trim().is_empty(),
            "expected whoami.exe's output to come back through the pty, got: {output:?}"
        );
    }
}

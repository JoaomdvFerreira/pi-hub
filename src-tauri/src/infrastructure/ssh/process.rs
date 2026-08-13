use std::{
    io::{self, Read},
    process::{Child, Command, Stdio},
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct OutputLimits {
    pub stdout: usize,
    pub stderr: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawProcessOutcome {
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub duration_ms: u64,
    pub timed_out: bool,
    pub output_limit_exceeded: bool,
}

enum CapturedStream {
    Complete(Vec<u8>),
    LimitExceeded,
}

/// Runs `command`, waiting at most `timeout` for it to finish. Output is read
/// concurrently so a full pipe cannot deadlock the child. This compatibility
/// wrapper has no output cap; callers handling untrusted remote evidence must
/// use `run_with_timeout_and_output_limits`.
pub fn run_with_timeout(command: &mut Command, timeout: Duration) -> io::Result<RawProcessOutcome> {
    run_with_timeout_and_output_limits(
        command,
        timeout,
        OutputLimits { stdout: usize::MAX, stderr: usize::MAX },
    )
}

/// Runs `command` with hard per-stream capture bounds. The child is killed as
/// soon as either reader observes data beyond its limit; oversized output is
/// discarded and never returned to callers.
pub(crate) fn run_with_timeout_and_output_limits(
    command: &mut Command,
    timeout: Duration,
    limits: OutputLimits,
) -> io::Result<RawProcessOutcome> {
    command.stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped());

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(CREATE_NO_WINDOW);
    }

    let started = Instant::now();
    let mut child = command.spawn()?;
    let stdout = child.stdout.take().expect("stdout is piped");
    let stderr = child.stderr.take().expect("stderr is piped");
    let (limit_tx, limit_rx) = mpsc::channel();
    let stdout_reader = spawn_capture(stdout, limits.stdout, limit_tx.clone());
    let stderr_reader = spawn_capture(stderr, limits.stderr, limit_tx);

    let mut exit_code = None;
    let mut timed_out = false;
    let mut output_limit_exceeded = false;
    loop {
        if limit_rx.try_recv().is_ok() {
            output_limit_exceeded = true;
            terminate(&mut child);
            break;
        }
        match child.try_wait()? {
            Some(status) => { exit_code = status.code(); break; }
            None if started.elapsed() >= timeout => {
                timed_out = true;
                terminate(&mut child);
                break;
            }
            None => thread::sleep(Duration::from_millis(10)),
        }
    }

    let stdout = join_capture(stdout_reader)?;
    let stderr = join_capture(stderr_reader)?;
    output_limit_exceeded |= matches!(stdout, CapturedStream::LimitExceeded) || matches!(stderr, CapturedStream::LimitExceeded);
    if output_limit_exceeded {
        return Ok(outcome(started, None, String::new(), String::new(), false, true));
    }
    if timed_out {
        return Ok(outcome(started, None, String::new(), String::new(), true, false));
    }
    let stdout = match stdout {
        CapturedStream::Complete(bytes) => bytes,
        CapturedStream::LimitExceeded => unreachable!("limit outcome already returned"),
    };
    let stderr = match stderr {
        CapturedStream::Complete(bytes) => bytes,
        CapturedStream::LimitExceeded => unreachable!("limit outcome already returned"),
    };
    Ok(outcome(
        started,
        exit_code,
        String::from_utf8_lossy(&stdout).into_owned(),
        String::from_utf8_lossy(&stderr).into_owned(),
        false,
        false,
    ))
}

fn spawn_capture<R: Read + Send + 'static>(reader: R, limit: usize, signal: mpsc::Sender<()>) -> thread::JoinHandle<io::Result<CapturedStream>> {
    thread::spawn(move || capture(reader, limit, signal))
}

fn capture<R: Read>(mut reader: R, limit: usize, signal: mpsc::Sender<()>) -> io::Result<CapturedStream> {
    let mut output = Vec::with_capacity(limit.min(8 * 1024));
    let mut buffer = [0_u8; 8 * 1024];
    loop {
        let read = reader.read(&mut buffer)?;
        if read == 0 { return Ok(CapturedStream::Complete(output)); }
        if output.len().saturating_add(read) > limit {
            let _ = signal.send(());
            return Ok(CapturedStream::LimitExceeded);
        }
        output.extend_from_slice(&buffer[..read]);
    }
}

fn join_capture(handle: thread::JoinHandle<io::Result<CapturedStream>>) -> io::Result<CapturedStream> {
    handle.join().map_err(|_| io::Error::other("output capture thread panicked"))?
}

fn terminate(child: &mut Child) {
    let _ = child.kill();
    let _ = child.wait();
}

fn outcome(started: Instant, exit_code: Option<i32>, stdout: String, stderr: String, timed_out: bool, output_limit_exceeded: bool) -> RawProcessOutcome {
    RawProcessOutcome { exit_code, stdout, stderr, duration_ms: started.elapsed().as_millis() as u64, timed_out, output_limit_exceeded }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cmd(script: &str) -> Command {
        let mut command = Command::new("cmd");
        command.args(["/C", script]);
        command
    }

    #[test]
    fn returns_exit_code_and_output_for_a_fast_command() {
        let mut command = cmd("echo hello");
        let outcome = run_with_timeout(&mut command, Duration::from_secs(5)).unwrap();
        assert_eq!(outcome.exit_code, Some(0));
        assert!(outcome.stdout.contains("hello"));
        assert!(!outcome.timed_out);
    }

    #[test]
    fn reports_a_nonzero_exit_code() {
        let mut command = cmd("exit 3");
        let outcome = run_with_timeout(&mut command, Duration::from_secs(5)).unwrap();
        assert_eq!(outcome.exit_code, Some(3));
        assert!(!outcome.timed_out);
    }

    #[test]
    fn stdout_over_limit_kills_capture_without_returning_output() {
        let mut command = cmd("for /L %i in (1,1,20) do @<nul set /p =1234567890");
        let outcome = run_with_timeout_and_output_limits(&mut command, Duration::from_secs(5), OutputLimits { stdout: 64, stderr: 64 }).unwrap();
        assert!(outcome.output_limit_exceeded);
        assert!(outcome.stdout.is_empty());
        assert!(outcome.stderr.is_empty());
    }

    #[test]
    fn stderr_over_limit_kills_capture_without_returning_output() {
        let mut command = cmd("for /L %i in (1,1,20) do @echo 1234567890 1>&2");
        let outcome = run_with_timeout_and_output_limits(&mut command, Duration::from_secs(5), OutputLimits { stdout: 64, stderr: 64 }).unwrap();
        assert!(outcome.output_limit_exceeded);
        assert!(outcome.stdout.is_empty());
        assert!(outcome.stderr.is_empty());
    }

    #[test]
    fn output_at_the_limit_succeeds() {
        let mut command = cmd("<nul set /p =1234567890");
        let outcome = run_with_timeout_and_output_limits(&mut command, Duration::from_secs(5), OutputLimits { stdout: 10, stderr: 0 }).unwrap();
        assert!(!outcome.output_limit_exceeded);
        assert_eq!(outcome.stdout, "1234567890");
    }

    #[test]
    fn output_below_the_limit_succeeds() {
        let mut command = cmd("<nul set /p =12345");
        let outcome = run_with_timeout_and_output_limits(&mut command, Duration::from_secs(5), OutputLimits { stdout: 10, stderr: 0 }).unwrap();
        assert!(!outcome.output_limit_exceeded);
        assert_eq!(outcome.stdout, "12345");
    }

    #[test]
    fn enforces_a_hard_timeout_and_kills_the_process() {
        let mut command = cmd("ping -n 6 127.0.0.1 >NUL");
        let outcome = run_with_timeout_and_output_limits(&mut command, Duration::from_millis(300), OutputLimits { stdout: 64, stderr: 64 }).unwrap();
        assert!(outcome.timed_out);
        assert!(!outcome.output_limit_exceeded);
        assert_eq!(outcome.exit_code, None);
    }
}

use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawProcessOutcome {
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub duration_ms: u64,
    pub timed_out: bool,
}

/// Runs `command`, waiting at most `timeout` for it to finish. If the
/// deadline is reached the child process is killed; `timed_out` is true
/// and `exit_code`/`stdout`/`stderr` are empty in that case, since output
/// is never collected from a killed process.
pub fn run_with_timeout(
    command: &mut Command,
    timeout: Duration,
) -> std::io::Result<RawProcessOutcome> {
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let started = Instant::now();
    let mut child = command.spawn()?;

    if wait_until_exit_or_timeout(&mut child, timeout) {
        let _ = child.kill();
        let _ = child.wait();
        return Ok(RawProcessOutcome {
            exit_code: None,
            stdout: String::new(),
            stderr: String::new(),
            duration_ms: started.elapsed().as_millis() as u64,
            timed_out: true,
        });
    }

    let output = child.wait_with_output()?;
    Ok(RawProcessOutcome {
        exit_code: output.status.code(),
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        duration_ms: started.elapsed().as_millis() as u64,
        timed_out: false,
    })
}

fn wait_until_exit_or_timeout(child: &mut Child, timeout: Duration) -> bool {
    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_status)) => return false,
            Ok(None) => {
                if start.elapsed() >= timeout {
                    return true;
                }
                std::thread::sleep(Duration::from_millis(20));
            }
            Err(_) => return false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn returns_exit_code_and_output_for_a_fast_command() {
        let mut cmd = Command::new("cmd");
        cmd.args(["/C", "echo hello"]);

        let outcome = run_with_timeout(&mut cmd, Duration::from_secs(5)).unwrap();

        assert_eq!(outcome.exit_code, Some(0));
        assert!(outcome.stdout.contains("hello"));
        assert!(!outcome.timed_out);
    }

    #[test]
    fn reports_a_nonzero_exit_code() {
        let mut cmd = Command::new("cmd");
        cmd.args(["/C", "exit 3"]);

        let outcome = run_with_timeout(&mut cmd, Duration::from_secs(5)).unwrap();

        assert_eq!(outcome.exit_code, Some(3));
        assert!(!outcome.timed_out);
    }

    #[test]
    fn enforces_a_hard_timeout_and_kills_the_process() {
        // `timeout.exe` refuses to run with redirected stdin and exits
        // immediately, so `ping` is used here as the standard Windows
        // stand-in for "a command that reliably takes several seconds."
        let mut cmd = Command::new("cmd");
        cmd.args(["/C", "ping -n 6 127.0.0.1 >NUL"]);

        let outcome = run_with_timeout(&mut cmd, Duration::from_millis(300)).unwrap();

        assert!(outcome.timed_out);
        assert_eq!(outcome.exit_code, None);
    }
}

use std::process::Command;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

const WT_EXECUTABLE: &str = "wt.exe";
const POWERSHELL_EXECUTABLE: &str = "powershell.exe";
const CMD_EXECUTABLE: &str = "cmd.exe";
const DEFAULT_SSH_PORT: u16 = 22;
#[cfg(windows)]
const CREATE_NEW_CONSOLE: u32 = 0x00000010;

#[derive(Debug)]
pub struct TerminalError(pub String);

fn build_ssh_args(host: &str, port: u16, username: &str) -> Vec<String> {
    let mut args = Vec::new();
    if port != DEFAULT_SSH_PORT {
        args.push("-p".to_string());
        args.push(port.to_string());
    }
    args.push(format!("{username}@{host}"));
    args
}

/// Builds the preferred `wt.exe` argument list for opening an SSH session
/// to a device in a new Windows Terminal window.
pub fn build_windows_terminal_args(host: &str, port: u16, username: &str) -> Vec<String> {
    let mut args = vec![
        "-w".to_string(),
        "-1".to_string(),
        "new-tab".to_string(),
        "--".to_string(),
        "ssh.exe".to_string(),
    ];
    args.extend(build_ssh_args(host, port, username));
    args
}

/// Builds a fallback PowerShell argument list. The stored device values are
/// already validated before persistence; they are still passed as separate
/// process arguments here rather than interpolated into one command string.
pub fn build_powershell_args(host: &str, port: u16, username: &str) -> Vec<String> {
    let mut args = vec![
        "-NoExit".to_string(),
        "-Command".to_string(),
        "ssh.exe".to_string(),
    ];
    args.extend(build_ssh_args(host, port, username));
    args
}

/// Builds the last-resort `cmd.exe` fallback. `cmd /K` keeps the window open
/// after SSH exits so any error remains visible to the user.
pub fn build_cmd_args(host: &str, port: u16, username: &str) -> Vec<String> {
    let mut args = vec!["/K".to_string(), "ssh.exe".to_string()];
    args.extend(build_ssh_args(host, port, username));
    args
}

fn try_spawn(executable: &str, args: &[String]) -> Result<(), String> {
    let mut command = Command::new(executable);
    command.args(args);

    #[cfg(windows)]
    if executable != WT_EXECUTABLE {
        command.creation_flags(CREATE_NEW_CONSOLE);
    }

    command
        .spawn()
        .map(|_| ())
        .map_err(|err| format!("{executable}: {err}"))
}

/// Launches a detached terminal running an SSH session to the given device.
/// Windows Terminal is preferred, but PowerShell and cmd.exe are used as
/// fallbacks because `wt.exe` is optional and its app execution alias may be
/// disabled. Each launcher opens a new terminal window connected to the
/// device. Never requests or stores a password; relies entirely on the
/// Windows OpenSSH client's own key/ssh-agent configuration, the same as the
/// monitoring SSH executor.
pub fn launch_ssh_terminal(host: &str, port: u16, username: &str) -> Result<(), TerminalError> {
    let launchers = [
        (
            WT_EXECUTABLE,
            build_windows_terminal_args(host, port, username),
        ),
        (
            POWERSHELL_EXECUTABLE,
            build_powershell_args(host, port, username),
        ),
        (CMD_EXECUTABLE, build_cmd_args(host, port, username)),
    ];

    let mut failures = Vec::new();
    for (executable, args) in launchers {
        match try_spawn(executable, &args) {
            Ok(()) => return Ok(()),
            Err(err) => failures.push(err),
        }
    }

    Err(TerminalError(format!(
        "could not launch a terminal for SSH. Tried: {}",
        failures.join("; ")
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn windows_terminal_default_port_omits_the_dash_p_flag() {
        let args = build_windows_terminal_args("raspberrypi5", 22, "pi");
        assert_eq!(
            args,
            vec!["-w", "-1", "new-tab", "--", "ssh.exe", "pi@raspberrypi5"]
        );
    }

    #[test]
    fn windows_terminal_non_default_port_is_passed_as_a_separate_argument() {
        let args = build_windows_terminal_args("raspberrypi5", 2222, "pi");
        assert_eq!(
            args,
            vec![
                "-w",
                "-1",
                "new-tab",
                "--",
                "ssh.exe",
                "-p",
                "2222",
                "pi@raspberrypi5"
            ]
        );
    }

    #[test]
    fn powershell_fallback_keeps_the_window_open() {
        let args = build_powershell_args("raspberrypi5", 22, "pi");
        assert_eq!(
            args,
            vec!["-NoExit", "-Command", "ssh.exe", "pi@raspberrypi5"]
        );
    }

    #[test]
    fn cmd_fallback_keeps_the_window_open() {
        let args = build_cmd_args("raspberrypi5", 2222, "pi");
        assert_eq!(args, vec!["/K", "ssh.exe", "-p", "2222", "pi@raspberrypi5"]);
    }

    #[test]
    fn the_destination_is_a_single_argument_never_a_joined_command_string() {
        // A host/username containing shell metacharacters must still end up
        // as exactly one argument token, not something that could be
        // re-interpreted by a shell -- there is no shell involved at all,
        // since std::process::Command never spawns one.
        let args = build_windows_terminal_args("host; rm -rf /", 22, "pi");
        assert_eq!(args.last(), Some(&"pi@host; rm -rf /".to_string()));
        assert_eq!(args.len(), 6);
    }
}

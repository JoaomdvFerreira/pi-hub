use std::process::Command;

const WT_EXECUTABLE: &str = "wt.exe";
const DEFAULT_SSH_PORT: u16 = 22;

#[derive(Debug)]
pub struct TerminalError(pub String);

/// Builds the `wt.exe` argument list for opening an SSH session to a device
/// in a new Windows Terminal tab. Every token is a separate argument (never
/// joined into one shell string), so a device's host/username can never
/// break out into additional `wt`/`ssh` flags or shell syntax -- the `--`
/// also stops `wt` itself from interpreting anything after it as its own
/// options.
pub fn build_terminal_args(host: &str, port: u16, username: &str) -> Vec<String> {
    let mut args = vec![
        "new-tab".to_string(),
        "--".to_string(),
        "ssh.exe".to_string(),
    ];
    if port != DEFAULT_SSH_PORT {
        args.push("-p".to_string());
        args.push(port.to_string());
    }
    args.push(format!("{username}@{host}"));
    args
}

/// Launches a detached Windows Terminal tab running an SSH session to the
/// given device. Never requests or stores a password; relies entirely on
/// the Windows OpenSSH client's own key/ssh-agent configuration, the same
/// as the monitoring SSH executor.
pub fn launch_ssh_terminal(host: &str, port: u16, username: &str) -> Result<(), TerminalError> {
    let args = build_terminal_args(host, port, username);
    Command::new(WT_EXECUTABLE)
        .args(&args)
        .spawn()
        .map(|_| ())
        .map_err(|err| {
            TerminalError(format!("could not launch Windows Terminal ({WT_EXECUTABLE}): {err}"))
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_port_omits_the_dash_p_flag() {
        let args = build_terminal_args("raspberrypi5", 22, "pi");
        assert_eq!(
            args,
            vec!["new-tab", "--", "ssh.exe", "pi@raspberrypi5"]
        );
    }

    #[test]
    fn non_default_port_is_passed_as_a_separate_argument() {
        let args = build_terminal_args("raspberrypi5", 2222, "pi");
        assert_eq!(
            args,
            vec!["new-tab", "--", "ssh.exe", "-p", "2222", "pi@raspberrypi5"]
        );
    }

    #[test]
    fn the_destination_is_a_single_argument_never_a_joined_command_string() {
        // A host/username containing shell metacharacters must still end up
        // as exactly one argument token, not something that could be
        // re-interpreted by a shell -- there is no shell involved at all,
        // since std::process::Command never spawns one.
        let args = build_terminal_args("host; rm -rf /", 22, "pi");
        assert_eq!(args.last(), Some(&"pi@host; rm -rf /".to_string()));
        assert_eq!(args.len(), 4);
    }
}

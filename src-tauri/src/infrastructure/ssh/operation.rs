use crate::domain::docker_container::ContainerAction;

#[derive(Debug)]
pub struct InvalidContainerIdError(pub String);

impl std::fmt::Display for InvalidContainerIdError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Docker restricts container ids/names to this character set (hex ids,
/// or `[a-zA-Z0-9][a-zA-Z0-9_.-]*` names). Re-validated here rather than
/// trusted, since the id passed in ultimately came from a previous
/// `docker ps` response and is about to be embedded into a remote shell
/// command -- unlike the local `ssh.exe` invocation (where host/user/etc
/// are always passed as separate process arguments), a single SSH command
/// is always sent as one string, so this is the one place in the app that
/// legitimately builds a command by embedding a value into it.
fn validate_container_id(id: &str) -> Result<(), InvalidContainerIdError> {
    let valid = !id.is_empty()
        && id.len() <= 128
        && id.chars().next().is_some_and(|c| c.is_ascii_alphanumeric())
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.' || c == '-');
    if valid {
        Ok(())
    } else {
        Err(InvalidContainerIdError(format!(
            "'{id}' is not a valid Docker container id/name"
        )))
    }
}

/// Builds the fixed `docker <verb> -- '<id>'` remote command for a
/// container lifecycle action. The validated character set can never
/// contain a shell metacharacter or an unescaped single quote, and `--`
/// stops the id from ever being interpreted as a docker flag even if a
/// future change to Docker's own naming rules allowed a leading `-`; the
/// surrounding quotes are still applied as defense in depth on top of
/// the validation, not in place of it.
pub fn docker_container_action_command(
    action: ContainerAction,
    container_id: &str,
) -> Result<String, InvalidContainerIdError> {
    validate_container_id(container_id)?;
    Ok(format!(
        "docker {} -- '{container_id}'",
        action.docker_verb()
    ))
}

/// The fixed set of remote operations Pi-Hub is ever allowed to run. The
/// frontend cannot supply arbitrary commands: every command string
/// executed over SSH is one of these predefined variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemoteOperation {
    Probe,
    /// Identity fields (hostname/model/OS/kernel/arch) are collected as
    /// part of the single `SystemMetrics` session (spec section 12.1:
    /// "collect system data in one SSH session where practical") rather
    /// than a separate round trip, so this variant has no command of its
    /// own. Kept in the catalogue for documentation/future use.
    #[allow(dead_code)]
    SystemIdentity,
    // Only constructed by infrastructure::parsers::metrics, which nothing
    // calls yet -- the scheduler is a later M3 work unit.
    #[allow(dead_code)]
    SystemMetrics,
    // Only constructed by infrastructure::parsers::docker, which nothing
    // calls yet -- the scheduler is a later M3 work unit.
    #[allow(dead_code)]
    DockerContainers,
}

pub const PROBE_COMMAND: &str = "printf 'PIHUB_OK'";

/// Collects CPU usage (via a two-sample `/proc/stat` delta over a bounded
/// interval, per spec section 12.2), memory (`/proc/meminfo`), disk usage
/// of the root filesystem (`df`), temperature (the primary thermal zone,
/// optional), load average, uptime, and identity fields, all as a single
/// `PIHUB_KEY=value` payload in one SSH round trip. Every reading is best-
/// effort: a missing or unreadable source simply omits that line rather
/// than failing the whole script.
pub const SYSTEM_METRICS_COMMAND: &str = r#"
read -r cpu1_line < /proc/stat
set -- $cpu1_line
shift
u1=$1; n1=$2; s1=$3; i1=$4; w1=$5
sleep 0.3
read -r cpu2_line < /proc/stat
set -- $cpu2_line
shift
u2=$1; n2=$2; s2=$3; i2=$4; w2=$5
idle1=$((i1+w1)); idle2=$((i2+w2))
total1=$((u1+n1+s1+i1+w1)); total2=$((u2+n2+s2+i2+w2))
dtotal=$((total2-total1)); didle=$((idle2-idle1))
if [ "$dtotal" -gt 0 ]; then
  cpu_pct=$(( (100 * (dtotal-didle)) / dtotal ))
else
  cpu_pct=0
fi
printf 'PIHUB_CPU_USAGE_PERCENT=%s\n' "$cpu_pct"

hostname_val=$(hostname 2>/dev/null)
printf 'PIHUB_HOSTNAME=%s\n' "$hostname_val"

model_val=$(tr -d '\0' < /proc/device-tree/model 2>/dev/null)
[ -n "$model_val" ] && printf 'PIHUB_MODEL=%s\n' "$model_val"

if [ -r /etc/os-release ]; then
  os_val=$(sh -c '. /etc/os-release 2>/dev/null; echo "$PRETTY_NAME"')
  [ -n "$os_val" ] && printf 'PIHUB_OS=%s\n' "$os_val"
fi

printf 'PIHUB_KERNEL=%s\n' "$(uname -r)"
printf 'PIHUB_ARCH=%s\n' "$(uname -m)"

read -r uptime_val _ < /proc/uptime
printf 'PIHUB_UPTIME_SECONDS=%s\n' "${uptime_val%%.*}"

if [ -r /proc/loadavg ]; then
  read -r l1 l5 l15 _ < /proc/loadavg
  printf 'PIHUB_LOAD_1M=%s\n' "$l1"
  printf 'PIHUB_LOAD_5M=%s\n' "$l5"
  printf 'PIHUB_LOAD_15M=%s\n' "$l15"
fi

if [ -r /proc/meminfo ]; then
  mem_total_kb=$(awk '/^MemTotal:/ {print $2}' /proc/meminfo)
  mem_avail_kb=$(awk '/^MemAvailable:/ {print $2}' /proc/meminfo)
  [ -n "$mem_total_kb" ] && printf 'PIHUB_MEMORY_TOTAL_BYTES=%s\n' "$((mem_total_kb*1024))"
  [ -n "$mem_avail_kb" ] && printf 'PIHUB_MEMORY_AVAILABLE_BYTES=%s\n' "$((mem_avail_kb*1024))"
fi

df_line=$(df -Pk / 2>/dev/null | awk 'NR==2')
if [ -n "$df_line" ]; then
  set -- $df_line
  disk_total_kb=$2
  disk_avail_kb=$4
  printf 'PIHUB_DISK_TOTAL_BYTES=%s\n' "$((disk_total_kb*1024))"
  printf 'PIHUB_DISK_AVAILABLE_BYTES=%s\n' "$((disk_avail_kb*1024))"
fi

if [ -r /sys/class/thermal/thermal_zone0/temp ]; then
  printf 'PIHUB_TEMP_MILLIC=%s\n' "$(cat /sys/class/thermal/thermal_zone0/temp)"
fi
"#;

/// Checks Docker availability, then lists containers read-only. Always
/// exits 0 when Docker itself is unavailable (reported via
/// `PIHUB_DOCKER_AVAILABLE=0`, never as a command failure); when Docker is
/// available, the script's exit status is `docker ps`'s own exit status,
/// so a permission failure surfaces as a normal remote-command error whose
/// stderr can be inspected -- never a second, separate probe. Docker is
/// never contacted over TCP; this only ever runs `docker` as the SSH
/// user's own CLI.
pub const DOCKER_CONTAINERS_COMMAND: &str = r#"
if ! command -v docker >/dev/null 2>&1; then
  printf 'PIHUB_DOCKER_AVAILABLE=0\n'
  exit 0
fi
printf 'PIHUB_DOCKER_AVAILABLE=1\n'
docker ps -a --no-trunc --format '{{json .}}'
"#;

impl RemoteOperation {
    /// The fixed remote shell command for this operation, if defined yet.
    pub fn command(&self) -> Option<&'static str> {
        match self {
            RemoteOperation::Probe => Some(PROBE_COMMAND),
            RemoteOperation::SystemMetrics => Some(SYSTEM_METRICS_COMMAND),
            RemoteOperation::DockerContainers => Some(DOCKER_CONTAINERS_COMMAND),
            RemoteOperation::SystemIdentity => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn probe_has_a_fixed_command() {
        assert_eq!(RemoteOperation::Probe.command(), Some(PROBE_COMMAND));
    }

    #[test]
    fn system_metrics_has_a_fixed_command() {
        assert_eq!(
            RemoteOperation::SystemMetrics.command(),
            Some(SYSTEM_METRICS_COMMAND)
        );
    }

    #[test]
    fn docker_containers_has_a_fixed_command() {
        assert_eq!(
            RemoteOperation::DockerContainers.command(),
            Some(DOCKER_CONTAINERS_COMMAND)
        );
    }

    #[test]
    fn system_identity_is_reserved_and_undefined_for_now() {
        assert_eq!(RemoteOperation::SystemIdentity.command(), None);
    }

    #[test]
    fn docker_action_command_uses_the_right_verb_per_action() {
        assert_eq!(
            docker_container_action_command(ContainerAction::Start, "homeassistant").unwrap(),
            "docker start -- 'homeassistant'"
        );
        assert_eq!(
            docker_container_action_command(ContainerAction::Stop, "homeassistant").unwrap(),
            "docker stop -- 'homeassistant'"
        );
        assert_eq!(
            docker_container_action_command(ContainerAction::Restart, "homeassistant").unwrap(),
            "docker restart -- 'homeassistant'"
        );
    }

    #[test]
    fn docker_action_command_accepts_a_full_length_hex_container_id() {
        let id = "a".repeat(64);
        assert!(docker_container_action_command(ContainerAction::Stop, &id).is_ok());
    }

    #[test]
    fn docker_action_command_rejects_shell_metacharacters() {
        let attempt = "homeassistant; rm -rf /";
        assert!(docker_container_action_command(ContainerAction::Stop, attempt).is_err());
    }

    #[test]
    fn docker_action_command_rejects_a_quote_breakout_attempt() {
        let attempt = "x' ; docker rm -f other-container #";
        assert!(docker_container_action_command(ContainerAction::Restart, attempt).is_err());
    }

    #[test]
    fn docker_action_command_rejects_an_empty_id() {
        assert!(docker_container_action_command(ContainerAction::Start, "").is_err());
    }

    #[test]
    fn docker_action_command_rejects_an_id_not_starting_alphanumeric() {
        assert!(docker_container_action_command(ContainerAction::Start, "-x").is_err());
        assert!(docker_container_action_command(ContainerAction::Start, "_x").is_err());
    }

    #[test]
    fn docker_action_command_rejects_an_overly_long_id() {
        let too_long = "a".repeat(129);
        assert!(docker_container_action_command(ContainerAction::Start, &too_long).is_err());
    }
}

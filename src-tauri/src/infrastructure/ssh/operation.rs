use crate::domain::docker_container::{ContainerAction, ContainerLogMode};

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

pub fn docker_container_logs_command(
    mode: ContainerLogMode,
    container_id: &str,
) -> Result<String, InvalidContainerIdError> {
    validate_container_id(container_id)?;
    Ok(format!(
        "docker logs --timestamps {} -- '{container_id}'",
        mode.docker_arguments()
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
    NetworkVisibility,
    StorageVisibility,
    SystemVisibility,
    UpdateDetect,
    UpdatePackages,
    UpdateHolds,
    UpdateMetadataAge,
    UpdateRebootState,
    Diagnostics,
    RestartDevice,
    ShutdownDevice,
    RestartDocker,
    RestartTailscale,
}

pub const PROBE_COMMAND: &str = "printf 'PIHUB_OK'";
pub const DIAGNOSTICS_COMMAND: &str = "printf 'PIHUB_DIAG_DOCKER='; command -v docker >/dev/null 2>&1 && printf 1 || printf 0; printf '\\nPIHUB_DIAG_TAILSCALE='; command -v tailscale >/dev/null 2>&1 && printf 1 || printf 0";
/// M10's complete privileged command catalogue. These are deliberately
/// fixed literals using `sudo -n`: no frontend value can become a command,
/// unit name, password prompt, or shell fragment.
pub const RESTART_DEVICE_COMMAND: &str = "sudo -n systemctl reboot";
pub const SHUTDOWN_DEVICE_COMMAND: &str = "sudo -n systemctl poweroff";
pub const RESTART_DOCKER_COMMAND: &str = "sudo -n systemctl restart docker";
pub const RESTART_TAILSCALE_COMMAND: &str = "sudo -n systemctl restart tailscaled";

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

if [ -r /proc/meminfo ]; then
  swap_total_kb=$(awk '/^SwapTotal:/ {print $2}' /proc/meminfo)
  swap_free_kb=$(awk '/^SwapFree:/ {print $2}' /proc/meminfo)
  [ -n "$swap_total_kb" ] && printf 'PIHUB_SWAP_TOTAL_BYTES=%s\n' "$((swap_total_kb*1024))"
  [ -n "$swap_free_kb" ] && printf 'PIHUB_SWAP_FREE_BYTES=%s\n' "$((swap_free_kb*1024))"
fi

freq_khz=$(cat /sys/devices/system/cpu/cpu0/cpufreq/scaling_cur_freq 2>/dev/null)
[ -n "$freq_khz" ] && printf 'PIHUB_CPU_FREQUENCY_MHZ=%s\n' "$((freq_khz/1000))"
boot_time=$(awk '/^btime / {print $2}' /proc/stat 2>/dev/null)
[ -n "$boot_time" ] && printf 'PIHUB_BOOT_TIMESTAMP=%s\n' "$boot_time"
[ -e /var/run/reboot-required ] && printf 'PIHUB_REBOOT_REQUIRED=1\n' || printf 'PIHUB_REBOOT_REQUIRED=0\n'
root_opts=$(findmnt -n -o OPTIONS / 2>/dev/null)
[ -n "$root_opts" ] && case ",$root_opts," in *,ro,*) printf 'PIHUB_ROOT_FS_READ_ONLY=1\n' ;; *) printf 'PIHUB_ROOT_FS_READ_ONLY=0\n' ;; esac
if command -v vcgencmd >/dev/null 2>&1; then
  throttled=$(vcgencmd get_throttled 2>/dev/null)
  [ -n "$throttled" ] && printf 'PIHUB_THROTTLED_RAW=%s\n' "$throttled"
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
ids=$(docker ps -aq)
[ -z "$ids" ] || docker inspect --format 'PIHUB_DOCKER_INSPECT={{printf "{\\"id\\":%s,\\"imageId\\":%s,\\"createdAt\\":%s,\\"startedAt\\":%s,\\"restartCount\\":%d,\\"restartPolicy\\":%s,\\"restartMaximumRetryCount\\":%d,\\"mounts\\":%s,\\"networks\\":%s,\\"labels\\":%s}" (json .Id) (json .Image) (json .Created) (json .State.StartedAt) .RestartCount (json .HostConfig.RestartPolicy.Name) .HostConfig.RestartPolicy.MaximumRetryCount (json .Mounts) (json .NetworkSettings.Networks) (json .Config.Labels)}}' $ids
docker stats --no-stream --format 'PIHUB_DOCKER_STATS={{json .}}' 2>/dev/null || true
"#;

/// Fixed, read-only network collection. It emits a compact protocol rather
/// than exposing raw shell output to the frontend; unavailable sources simply
/// omit records and never grant configuration capability.
pub const NETWORK_VISIBILITY_COMMAND: &str = r#"
ip -o link show 2>/dev/null | while IFS= read -r line; do
  set -- $line; name=${2%:}; name=${name%@*}; state=unknown; mac=
  case " $line " in *" state UP "*) state=up ;; *" state DOWN "*) state=down ;; esac
  for word in $line; do case "$word" in [0-9A-Fa-f][0-9A-Fa-f]:[0-9A-Fa-f][0-9A-Fa-f]:[0-9A-Fa-f][0-9A-Fa-f]:[0-9A-Fa-f][0-9A-Fa-f]:[0-9A-Fa-f][0-9A-Fa-f]) mac=$word ;; esac; done
  [ -n "$name" ] && printf 'PIHUB_NET_LINK=%s|%s|%s\n' "$name" "$state" "$mac"
done
ip -o -4 addr show 2>/dev/null | awk '{sub(/:$/, "", $2); print "PIHUB_NET_ADDR=" $2 "|4|" $4}'
ip -o -6 addr show 2>/dev/null | awk '{sub(/:$/, "", $2); print "PIHUB_NET_ADDR=" $2 "|6|" $4}'
ip route show default 2>/dev/null | while read -r _ via gateway dev iface _; do [ -n "$iface" ] && printf 'PIHUB_NET_ROUTE=%s|%s\n' "$iface" "$gateway"; done
awk '/^[[:space:]]*nameserver[[:space:]]+/ {print "PIHUB_NET_DNS=" $2}' /etc/resolv.conf 2>/dev/null
"#;

pub const STORAGE_VISIBILITY_COMMAND: &str = r#"
findmnt -n -P -o SOURCE,TARGET,FSTYPE,OPTIONS 2>/dev/null | awk '
function value(name, prefix) {
  prefix = name "=\""
  if (match($0, prefix "[^\"]*\"")) return substr($0, RSTART + length(prefix), RLENGTH - length(prefix) - 1)
  return ""
}
{
  source = value("SOURCE"); target = value("TARGET"); fstype = value("FSTYPE"); options = value("OPTIONS")
  if (source != "" && target != "" && fstype != "") printf "%s|%s|%s|%s\n", source, target, fstype, options
}
' | while IFS='|' read -r source target fstype options; do
  [ -n "$target" ] || continue
  df -P -B1 "$target" 2>/dev/null | awk -v source="$source" -v target="$target" -v fstype="$fstype" -v options="$options" 'NR==2 {ro=(options ~ /(^|,)ro(,|$)/ ? "ro" : (options == "" ? "-" : "rw")); gsub(/%/, "", $5); printf "PIHUB_STORAGE=%s|%s|%s|%s|%s|%s|%s|%s|x\n", source, target, fstype, $2, $3, $4, $5, ro}'
done
"#;

pub const SYSTEM_VISIBILITY_COMMAND: &str = r#"
printf 'PIHUB_SYS_HOSTNAME=%s\n' "$(hostname 2>/dev/null)"
[ -r /etc/os-release ] && os=$(sh -c '. /etc/os-release 2>/dev/null; printf "%s" "$PRETTY_NAME"') && [ -n "$os" ] && printf 'PIHUB_SYS_OS=%s\n' "$os"
printf 'PIHUB_SYS_KERNEL=%s\n' "$(uname -r 2>/dev/null)"; printf 'PIHUB_SYS_ARCH=%s\n' "$(uname -m 2>/dev/null)"
model=$(tr -d '\0' < /proc/device-tree/model 2>/dev/null); [ -n "$model" ] && printf 'PIHUB_SYS_MODEL=%s\n' "$model"
cpu=$(awk -F: '/model name|Hardware/ {gsub(/^[ \t]+/, "", $2); print $2; exit}' /proc/cpuinfo 2>/dev/null); [ -n "$cpu" ] && printf 'PIHUB_SYS_CPU=%s\n' "$cpu"
cores=$(getconf _NPROCESSORS_ONLN 2>/dev/null); [ -n "$cores" ] && printf 'PIHUB_SYS_CORES=%s\n' "$cores"
mem=$(awk '/^MemTotal:/ {print $2*1024}' /proc/meminfo 2>/dev/null); [ -n "$mem" ] && printf 'PIHUB_SYS_MEMORY_BYTES=%.0f\n' "$mem"
boot=$(awk '/^btime / {print $2}' /proc/stat 2>/dev/null); [ -n "$boot" ] && printf 'PIHUB_SYS_BOOT_TIMESTAMP=%s\n' "$boot"
read -r uptime _ < /proc/uptime; [ -n "$uptime" ] && printf 'PIHUB_SYS_UPTIME_SECONDS=%s\n' "${uptime%%.*}"
"#;
pub const UPDATE_DETECT_COMMAND: &str = r#"LC_ALL=C LANG=C; if [ -r /etc/os-release ]; then . /etc/os-release; printf 'PIHUB_UPDATE_OS_ID=%s\nPIHUB_UPDATE_OS_VERSION_ID=%s\nPIHUB_UPDATE_OS_LIKE=%s\n' "$ID" "$VERSION_ID" "$ID_LIKE"; else printf 'PIHUB_UPDATE_OS_ID=\n'; fi; for x in apt-get dpkg-query dpkg; do command -v "$x" >/dev/null 2>&1 && printf 'PIHUB_UPDATE_%s=1\n' "$(printf %s "$x" | tr a-z- A-Z_)" || printf 'PIHUB_UPDATE_%s=0\n' "$(printf %s "$x" | tr a-z- A-Z_)"; done"#;
pub const UPDATE_PACKAGES_COMMAND: &str = r#"LC_ALL=C LANG=C; output=$(apt-get -s upgrade) || exit $?; printf '%s\n' "$output" | awk '/^Inst / { print "PIHUB_UPDATE_PACKAGE=" $0 } /^The following packages have been kept back:$/ { kept=1; next } kept && /^The following packages / { kept=0 } kept && /^[0-9]+ upgraded,/ { kept=0 } kept && NF { for (i=1;i<=NF;i++) print "PIHUB_UPDATE_KEPT_BACK=" $i } /^[0-9]+ upgraded, [0-9]+ newly installed, [0-9]+ to remove and [0-9]+ not upgraded\.$/ { count=$0; sub(/^.* and /, "", count); sub(/ not upgraded\.$/, "", count); print "PIHUB_UPDATE_KEPT_BACK_COUNT=" count }'; printf 'PIHUB_UPDATE_PACKAGES_DONE=1\n'"#;
pub const UPDATE_HOLDS_COMMAND: &str = r#"LC_ALL=C LANG=C; dpkg --get-selections | while IFS=' ' read -r package selection; do [ "$selection" = hold ] && printf 'PIHUB_UPDATE_HOLD=%s\n' "$package"; done; printf 'PIHUB_UPDATE_HOLDS_DONE=1\n'"#;
pub const UPDATE_METADATA_AGE_COMMAND: &str = r#"LC_ALL=C LANG=C; newest=$(find /var/lib/apt/lists -type f ! -name lock -printf '%T@\n' 2>/dev/null | sort -nr | head -n 1); [ -n "$newest" ] && printf 'PIHUB_UPDATE_METADATA_MTIME=%s\n' "$newest" || printf 'PIHUB_UPDATE_METADATA_MTIME=none\n'"#;
pub const UPDATE_REBOOT_STATE_COMMAND: &str = r#"[ -r /var/run/reboot-required ] && printf 'PIHUB_UPDATE_REBOOT=required\n' || printf 'PIHUB_UPDATE_REBOOT=unknown\n'"#;

impl RemoteOperation {
    /// The fixed remote shell command for this operation, if defined yet.
    pub fn command(&self) -> Option<&'static str> {
        match self {
            RemoteOperation::Probe => Some(PROBE_COMMAND),
            RemoteOperation::SystemMetrics => Some(SYSTEM_METRICS_COMMAND),
            RemoteOperation::DockerContainers => Some(DOCKER_CONTAINERS_COMMAND),
            RemoteOperation::NetworkVisibility => Some(NETWORK_VISIBILITY_COMMAND),
            RemoteOperation::StorageVisibility => Some(STORAGE_VISIBILITY_COMMAND),
            RemoteOperation::SystemVisibility => Some(SYSTEM_VISIBILITY_COMMAND),
            RemoteOperation::UpdateDetect => Some(UPDATE_DETECT_COMMAND),
            RemoteOperation::UpdatePackages => Some(UPDATE_PACKAGES_COMMAND),
            RemoteOperation::UpdateHolds => Some(UPDATE_HOLDS_COMMAND),
            RemoteOperation::UpdateMetadataAge => Some(UPDATE_METADATA_AGE_COMMAND),
            RemoteOperation::UpdateRebootState => Some(UPDATE_REBOOT_STATE_COMMAND),
            RemoteOperation::Diagnostics => Some(DIAGNOSTICS_COMMAND),
            RemoteOperation::RestartDevice => Some(RESTART_DEVICE_COMMAND),
            RemoteOperation::ShutdownDevice => Some(SHUTDOWN_DEVICE_COMMAND),
            RemoteOperation::RestartDocker => Some(RESTART_DOCKER_COMMAND),
            RemoteOperation::RestartTailscale => Some(RESTART_TAILSCALE_COMMAND),
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
    fn storage_visibility_uses_machine_readable_findmnt_records() {
        let command = RemoteOperation::StorageVisibility.command().unwrap();
        assert!(command.contains("findmnt -n -P -o SOURCE,TARGET,FSTYPE,OPTIONS"));
        assert!(!command.contains("findmnt -rn -P"));
        assert!(command.contains(r#"printf "%s|%s|%s|%s\n"#));
        assert!(command.contains(r#"PIHUB_STORAGE=%s|%s|%s|%s|%s|%s|%s|%s|x\n"#));
        assert!(!command.contains(r#"PIHUB_STORAGE=%s|%s|%s|%s|%s|%s|%s|%s|x\\n"#));
        assert!(command.contains("while IFS='|' read -r source target fstype options"));
    }

    #[test]
    fn system_identity_is_reserved_and_undefined_for_now() {
        assert_eq!(RemoteOperation::SystemIdentity.command(), None);
    }
    #[test]
    fn update_operations_are_fixed_read_only_commands() {
        for operation in [
            RemoteOperation::UpdateDetect,
            RemoteOperation::UpdatePackages,
            RemoteOperation::UpdateHolds,
            RemoteOperation::UpdateMetadataAge,
            RemoteOperation::UpdateRebootState,
        ] {
            let command = operation.command().unwrap();
            assert!(!command.contains("apt update"));
            assert!(!command.contains("apt-get update"));
            assert!(!command.contains(" install "));
            assert!(!command.contains(" upgrade ") || command.contains("apt-get -s upgrade"));
            assert!(!command.contains("--no-download"));
        }
        let packages = RemoteOperation::UpdatePackages.command().unwrap();
        assert!(packages.contains("PIHUB_UPDATE_KEPT_BACK_COUNT="));
        assert!(packages.contains("kept && /^The following packages /"));
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
    fn docker_log_command_has_only_the_four_bounded_modes() {
        assert_eq!(
            docker_container_logs_command(ContainerLogMode::Last100, "homeassistant").unwrap(),
            "docker logs --timestamps --tail 100 -- 'homeassistant'"
        );
        assert_eq!(
            docker_container_logs_command(ContainerLogMode::Last500, "homeassistant").unwrap(),
            "docker logs --timestamps --tail 500 -- 'homeassistant'"
        );
        assert_eq!(
            docker_container_logs_command(ContainerLogMode::Last15Minutes, "homeassistant")
                .unwrap(),
            "docker logs --timestamps --since 15m -- 'homeassistant'"
        );
        assert!(
            docker_container_logs_command(ContainerLogMode::Last1Hour, "bad; rm -rf /").is_err()
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

    #[test]
    fn administration_commands_are_closed_and_noninteractive() {
        assert_eq!(
            RemoteOperation::RestartDevice.command(),
            Some(RESTART_DEVICE_COMMAND)
        );
        assert_eq!(
            RemoteOperation::ShutdownDevice.command(),
            Some(SHUTDOWN_DEVICE_COMMAND)
        );
        assert_eq!(
            RemoteOperation::RestartDocker.command(),
            Some(RESTART_DOCKER_COMMAND)
        );
        assert_eq!(
            RemoteOperation::RestartTailscale.command(),
            Some(RESTART_TAILSCALE_COMMAND)
        );
        for command in [
            RESTART_DEVICE_COMMAND,
            SHUTDOWN_DEVICE_COMMAND,
            RESTART_DOCKER_COMMAND,
            RESTART_TAILSCALE_COMMAND,
        ] {
            assert!(command.starts_with("sudo -n systemctl "));
        }
    }
}

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
    // Reserved for the M3 Docker monitoring work unit, which pairs the
    // command body with the parser that consumes its output; not
    // constructed yet.
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

impl RemoteOperation {
    /// The fixed remote shell command for this operation, if defined yet.
    pub fn command(&self) -> Option<&'static str> {
        match self {
            RemoteOperation::Probe => Some(PROBE_COMMAND),
            RemoteOperation::SystemMetrics => Some(SYSTEM_METRICS_COMMAND),
            RemoteOperation::SystemIdentity | RemoteOperation::DockerContainers => None,
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
    fn docker_containers_is_reserved_and_undefined_for_now() {
        assert_eq!(RemoteOperation::SystemIdentity.command(), None);
        assert_eq!(RemoteOperation::DockerContainers.command(), None);
    }
}

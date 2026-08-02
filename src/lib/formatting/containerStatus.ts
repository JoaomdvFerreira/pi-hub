import type { DockerContainerState, DockerHealthStatus } from "@/types/snapshot";

const STATE_LABELS: Record<DockerContainerState, string> = {
  running: "Running",
  stopped: "Stopped",
  exited: "Exited",
  restarting: "Restarting",
  paused: "Paused",
  dead: "Dead",
  unknown: "Unknown",
};

/** Tailwind text-color classes matching the status tokens in src/globals.css. */
const STATE_COLOR_CLASSES: Record<DockerContainerState, string> = {
  running: "text-status-healthy",
  stopped: "text-status-offline",
  exited: "text-status-offline",
  dead: "text-status-offline",
  restarting: "text-status-warning",
  paused: "text-status-warning",
  unknown: "text-muted-foreground",
};

export function containerStateLabel(state: DockerContainerState): string {
  return STATE_LABELS[state];
}

export function containerStateColorClass(state: DockerContainerState): string {
  return STATE_COLOR_CLASSES[state];
}

const HEALTH_LABELS: Record<DockerHealthStatus, string> = {
  healthy: "Healthy",
  unhealthy: "Unhealthy",
  starting: "Starting",
  none: "—",
  unknown: "Unknown",
};

const HEALTH_COLOR_CLASSES: Record<DockerHealthStatus, string> = {
  healthy: "text-status-healthy",
  unhealthy: "text-status-offline",
  starting: "text-status-warning",
  none: "text-muted-foreground",
  unknown: "text-muted-foreground",
};

export function containerHealthLabel(health: DockerHealthStatus): string {
  return HEALTH_LABELS[health];
}

export function containerHealthColorClass(health: DockerHealthStatus): string {
  return HEALTH_COLOR_CLASSES[health];
}

export function formatPorts(
  ports: { hostPort?: number; containerPort: number; protocol: string }[],
): string {
  if (ports.length === 0) {
    return "—";
  }
  return ports
    .map((p) => (p.hostPort !== undefined ? `${p.hostPort}:${p.containerPort}` : `${p.containerPort}`))
    .join(", ");
}

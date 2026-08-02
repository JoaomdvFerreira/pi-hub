import type { SystemMetrics } from "@/types/snapshot";

export function memoryPercent(metrics: SystemMetrics): number | undefined {
  if (
    metrics.memoryUsedBytes === undefined ||
    metrics.memoryTotalBytes === undefined ||
    metrics.memoryTotalBytes === 0
  ) {
    return undefined;
  }
  return (metrics.memoryUsedBytes / metrics.memoryTotalBytes) * 100;
}

export function diskPercent(metrics: SystemMetrics): number | undefined {
  if (
    metrics.diskUsedBytes === undefined ||
    metrics.diskTotalBytes === undefined ||
    metrics.diskTotalBytes === 0
  ) {
    return undefined;
  }
  return (metrics.diskUsedBytes / metrics.diskTotalBytes) * 100;
}

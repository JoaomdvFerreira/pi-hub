/** Formats a duration in seconds as a compact "14d 6h" / "6h 12m" / "42s" style string. */
export function formatUptime(totalSeconds: number): string {
  if (totalSeconds < 60) {
    return `${Math.floor(totalSeconds)}s`;
  }

  const days = Math.floor(totalSeconds / 86400);
  const hours = Math.floor((totalSeconds % 86400) / 3600);
  const minutes = Math.floor((totalSeconds % 3600) / 60);

  if (days > 0) {
    return `${days}d ${hours}h`;
  }
  if (hours > 0) {
    return `${hours}h ${minutes}m`;
  }
  return `${minutes}m`;
}

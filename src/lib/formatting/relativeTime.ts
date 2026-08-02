/** Formats an ISO timestamp as a short relative string: "Just now", "5 min ago", "3h ago", "2d ago". */
export function formatRelativeTime(isoTimestamp: string | undefined): string {
  if (!isoTimestamp) {
    return "Never";
  }

  const then = new Date(isoTimestamp).getTime();
  if (Number.isNaN(then)) {
    return "Never";
  }

  const diffSeconds = Math.max(0, Math.floor((Date.now() - then) / 1000));

  if (diffSeconds < 30) {
    return "Just now";
  }
  if (diffSeconds < 60) {
    return `${diffSeconds}s ago`;
  }
  const minutes = Math.floor(diffSeconds / 60);
  if (minutes < 60) {
    return `${minutes} min ago`;
  }
  const hours = Math.floor(minutes / 60);
  if (hours < 24) {
    return `${hours}h ago`;
  }
  const days = Math.floor(hours / 24);
  return `${days}d ago`;
}

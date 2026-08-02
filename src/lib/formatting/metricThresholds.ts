/** Tailwind text-color classes matching the status tokens in src/globals.css. */
export function levelColorClass(percent: number): string {
  if (percent >= 90) return "text-status-offline";
  if (percent >= 75) return "text-status-warning";
  return "text-muted-foreground";
}

export function temperatureColorClass(celsius: number): string {
  if (celsius >= 80) return "text-status-offline";
  if (celsius >= 68) return "text-status-warning";
  return "text-muted-foreground";
}

/** Same thresholds as levelColorClass, but as a background-color class for progress bars. */
export function levelBarClass(percent: number): string {
  if (percent >= 90) return "bg-status-offline";
  if (percent >= 75) return "bg-status-warning";
  return "bg-status-neutral";
}

export function temperatureBarClass(celsius: number): string {
  if (celsius >= 80) return "bg-status-offline";
  if (celsius >= 68) return "bg-status-warning";
  return "bg-status-neutral";
}

export function clampPercent(value: number): number {
  return Math.max(2, Math.min(100, value));
}

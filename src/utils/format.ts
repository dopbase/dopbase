/** Formats an RFC 3339 timestamp as a compact relative age, e.g. `2h ago`. */
export function formatRelativeTime(
  iso: string,
  now: Date = new Date(),
): string {
  const then = new Date(iso).getTime();
  if (Number.isNaN(then)) return "—";
  const seconds = Math.max(0, Math.floor((now.getTime() - then) / 1000));
  if (seconds < 45) return "just now";
  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) return `${minutes}m ago`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `${hours}h ago`;
  const days = Math.floor(hours / 24);
  if (days < 30) return `${days}d ago`;
  const months = Math.floor(days / 30);
  if (months < 12) return `${months}mo ago`;
  return `${Math.floor(months / 12)}y ago`;
}

/** Formats an RFC 3339 timestamp in the browser locale, e.g. audit details. */
export function formatDateTime(iso: string): string {
  const date = new Date(iso);
  if (Number.isNaN(date.getTime())) return iso;
  return date.toLocaleString(undefined, {
    year: "numeric",
    month: "short",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
  });
}

/** Zero-pads seconds for the reveal countdown, e.g. `28s`. */
export function formatCountdown(seconds: number): string {
  return `${seconds}s`;
}

/** Formats seconds into a human-readable duration, e.g. `2h 15m`, `45s`, `3d 4h`. */
export function formatDuration(seconds: number): string {
  if (seconds <= 0 || Number.isNaN(seconds)) return "0s";
  const d = Math.floor(seconds / 86400);
  const h = Math.floor((seconds % 86400) / 3600);
  const m = Math.floor((seconds % 3600) / 60);
  const s = Math.floor(seconds % 60);
  if (d > 0) return h > 0 ? `${d}d ${h}h` : `${d}d`;
  if (h > 0) return m > 0 ? `${h}h ${m}m` : `${h}h`;
  if (m > 0) return s > 0 ? `${m}m ${s}s` : `${m}m`;
  return `${s}s`;
}

/** Formats byte counts into human-readable strings, e.g. `128 KB`, `1.4 MB`. */
export function formatBytes(bytes: number): string {
  if (bytes <= 0 || Number.isNaN(bytes)) return "0 B";
  if (bytes < 1024) return `${bytes} B`;
  const units = ["KB", "MB", "GB", "TB"];
  let unitIndex = -1;
  let val = bytes;
  do {
    val /= 1024;
    unitIndex++;
  } while (val >= 1024 && unitIndex < units.length - 1);
  return `${val >= 10 ? Math.round(val) : val.toFixed(1)} ${units[unitIndex]}`;
}

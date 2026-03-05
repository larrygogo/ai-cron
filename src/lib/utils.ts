export function formatDuration(ms?: number | null): string {
  if (ms == null) return "\u2014";
  if (ms === 0) return "0s";
  const s = Math.floor(ms / 1000);
  if (s >= 60) return `${Math.floor(s / 60)}m${s % 60}s`;
  return `${s}s`;
}

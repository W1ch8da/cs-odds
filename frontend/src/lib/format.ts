const dateFormat = new Intl.DateTimeFormat(undefined, { day: "numeric", month: "short", year: "numeric" });
const dateTimeFormat = new Intl.DateTimeFormat(undefined, { day: "numeric", month: "short", hour: "numeric", minute: "2-digit" });
const relative = new Intl.RelativeTimeFormat(undefined, { numeric: "auto", style: "long" });

/** "26 Sep 2026" in the viewer's locale. */
export function formatDate(iso: string): string {
  return dateFormat.format(new Date(iso));
}

/** "26 Sep, 14:05" in the viewer's locale. */
export function formatDateTime(iso: string): string {
  return dateTimeFormat.format(new Date(iso));
}

const UNITS: [Intl.RelativeTimeFormatUnit, number][] = [
  ["year", 365 * 24 * 3600],
  ["month", 30 * 24 * 3600],
  ["week", 7 * 24 * 3600],
  ["day", 24 * 3600],
  ["hour", 3600],
  ["minute", 60],
];

/** "5 minutes ago", "yesterday", "just now". */
export function timeAgo(iso: string, now: number = Date.now()): string {
  const seconds = Math.round((new Date(iso).getTime() - now) / 1000);
  for (const [unit, size] of UNITS) {
    if (Math.abs(seconds) >= size) return relative.format(Math.round(seconds / size), unit);
  }
  return "just now";
}

export function firstName(name: string): string {
  return name.split(/\s+/)[0] ?? name;
}

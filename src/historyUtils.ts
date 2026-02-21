export type HistoryEntry = {
  id: string;
  text: string;
  timestamp: string;
  durationSecs?: number | null;
  language?: string | null;
  provider: string;
};

const MINUTE_SECONDS = 60;
const HOUR_SECONDS = 60 * MINUTE_SECONDS;
const DAY_SECONDS = 24 * HOUR_SECONDS;

export function formatHistoryTimestamp(
  timestamp: string,
  nowMs = Date.now(),
  locale?: string,
): string {
  const parsedTimestamp = Date.parse(timestamp);
  if (!Number.isFinite(parsedTimestamp)) {
    return "Unknown time";
  }

  const deltaSeconds = Math.round((nowMs - parsedTimestamp) / 1_000);
  const absSeconds = Math.abs(deltaSeconds);
  const isPast = deltaSeconds >= 0;

  if (absSeconds < 45) {
    return isPast ? "just now" : "in a few seconds";
  }

  if (absSeconds < 90) {
    return isPast ? "1 min ago" : "in 1 min";
  }

  if (absSeconds < 45 * MINUTE_SECONDS) {
    const minutes = Math.round(absSeconds / MINUTE_SECONDS);
    return isPast ? `${minutes} min ago` : `in ${minutes} min`;
  }

  if (absSeconds < 90 * MINUTE_SECONDS) {
    return isPast ? "1 hr ago" : "in 1 hr";
  }

  if (absSeconds < 22 * HOUR_SECONDS) {
    const hours = Math.round(absSeconds / HOUR_SECONDS);
    return isPast ? `${hours} hr ago` : `in ${hours} hr`;
  }

  if (absSeconds < 36 * HOUR_SECONDS) {
    return isPast ? "1 day ago" : "in 1 day";
  }

  if (absSeconds < 7 * DAY_SECONDS) {
    const days = Math.round(absSeconds / DAY_SECONDS);
    return isPast ? `${days} days ago` : `in ${days} days`;
  }

  const date = new Date(parsedTimestamp);
  const now = new Date(nowMs);
  const includeYear = date.getUTCFullYear() !== now.getUTCFullYear();
  const dateFormat: Intl.DateTimeFormatOptions = includeYear
    ? {
        month: "short",
        day: "numeric",
        year: "numeric",
        hour: "numeric",
        minute: "2-digit",
      }
    : { month: "short", day: "numeric", hour: "numeric", minute: "2-digit" };

  return new Intl.DateTimeFormat(locale, dateFormat).format(date);
}

export function formatDuration(durationSecs?: number | null): string {
  if (durationSecs === undefined || durationSecs === null || !Number.isFinite(durationSecs)) {
    return "Duration unknown";
  }

  const wholeSeconds = Math.max(0, Math.round(durationSecs));
  const hours = Math.floor(wholeSeconds / HOUR_SECONDS);
  const minutes = Math.floor((wholeSeconds % HOUR_SECONDS) / MINUTE_SECONDS);
  const seconds = wholeSeconds % MINUTE_SECONDS;

  if (hours > 0) {
    return `${hours}h ${String(minutes).padStart(2, "0")}m`;
  }

  if (minutes > 0) {
    return `${minutes}m ${String(seconds).padStart(2, "0")}s`;
  }

  return `${seconds}s`;
}

export function formatLanguageCode(language?: string | null): string {
  if (!language) {
    return "Unknown language";
  }

  const trimmed = language.trim();
  if (!trimmed) {
    return "Unknown language";
  }

  const [baseCode, regionCode] = trimmed.split(/[-_]/, 2);
  if (!baseCode) {
    return "Unknown language";
  }

  if (regionCode) {
    return `${baseCode.toUpperCase()}-${regionCode.toUpperCase()}`;
  }

  return baseCode.toUpperCase();
}

export function formatProvider(provider: string): string {
  const normalized = provider.trim();
  return normalized ? normalized.toUpperCase() : "UNKNOWN";
}

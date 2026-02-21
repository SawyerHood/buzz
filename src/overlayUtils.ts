export function clampAudioLevel(value: number): number {
  if (!Number.isFinite(value)) {
    return 0;
  }

  // Raw backend levels are usually 0.0-0.2 for normal speech.
  // Apply a gentler gain/curve so quiet speech stays small and loud peaks can still max out.
  const clamped = Math.max(0, Math.min(1, value));
  const gained = Math.min(1, clamped * 2.5);
  return Math.pow(gained, 0.7);
}

export function pushAudioLevelHistory(
  history: number[],
  value: number,
  maxLength: number,
): number[] {
  const boundedLength = Math.max(1, Math.floor(maxLength));
  const safeValue = Number.isFinite(value) ? Math.max(0, Math.min(1, value)) : 0;
  const next = history.slice(-(boundedLength - 1));
  next.push(safeValue);

  while (next.length < boundedLength) {
    next.unshift(0);
  }

  return next;
}

export function formatElapsedLabel(elapsedMs: number): string {
  const safeMs = Number.isFinite(elapsedMs) ? Math.max(0, Math.floor(elapsedMs)) : 0;
  const totalSeconds = Math.floor(safeMs / 1000);
  const minutes = Math.floor(totalSeconds / 60);
  const seconds = totalSeconds % 60;

  return `${minutes.toString().padStart(2, "0")}:${seconds.toString().padStart(2, "0")}`;
}

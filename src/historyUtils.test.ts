import { describe, expect, it } from "vitest";
import {
  formatDuration,
  formatHistoryTimestamp,
  formatLanguageCode,
  formatProvider,
} from "./historyUtils";

describe("formatHistoryTimestamp", () => {
  const now = Date.UTC(2026, 1, 21, 15, 0, 0);

  it("formats recent entries as relative time", () => {
    const timestamp = new Date(now - 2 * 60 * 1_000).toISOString();
    expect(formatHistoryTimestamp(timestamp, now, "en-US")).toBe("2 min ago");
  });

  it("formats older entries with a calendar timestamp", () => {
    const timestamp = new Date(Date.UTC(2025, 11, 1, 8, 30, 0)).toISOString();
    const label = formatHistoryTimestamp(timestamp, now, "en-US");
    expect(label).toContain("Dec");
    expect(label).toContain("2025");
  });

  it("handles invalid timestamps", () => {
    expect(formatHistoryTimestamp("not-a-date", now, "en-US")).toBe("Unknown time");
  });
});

describe("formatDuration", () => {
  it("formats short durations in seconds", () => {
    expect(formatDuration(9.2)).toBe("9s");
  });

  it("formats minute durations in minutes and seconds", () => {
    expect(formatDuration(125)).toBe("2m 05s");
  });

  it("handles missing duration values", () => {
    expect(formatDuration(null)).toBe("Duration unknown");
  });
});

describe("formatLanguageCode", () => {
  it("normalizes bare language codes", () => {
    expect(formatLanguageCode("en")).toBe("EN");
  });

  it("normalizes language and region codes", () => {
    expect(formatLanguageCode("pt-br")).toBe("PT-BR");
  });

  it("falls back for missing language values", () => {
    expect(formatLanguageCode("")).toBe("Unknown language");
  });
});

describe("formatProvider", () => {
  it("uppercases provider names", () => {
    expect(formatProvider("openai")).toBe("OPENAI");
  });
});

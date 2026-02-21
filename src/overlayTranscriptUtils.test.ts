import { describe, expect, it } from "vitest";

import {
  overlayPlaceholder,
  shouldAppendTranscriptionDelta,
  type OverlayStatus,
} from "./overlayTranscriptUtils";

describe("shouldAppendTranscriptionDelta", () => {
  it("accepts deltas while recording or transcribing", () => {
    expect(shouldAppendTranscriptionDelta("listening")).toBe(true);
    expect(shouldAppendTranscriptionDelta("transcribing")).toBe(true);
  });

  it("ignores deltas when overlay should not be active", () => {
    expect(shouldAppendTranscriptionDelta("idle")).toBe(false);
    expect(shouldAppendTranscriptionDelta("error")).toBe(false);
  });
});

describe("overlayPlaceholder", () => {
  it("returns status-specific placeholder text", () => {
    expect(overlayPlaceholder("listening")).toBe("Listening...");
    expect(overlayPlaceholder("transcribing")).toBe("Transcribing...");
  });

  it("returns empty text for hidden statuses", () => {
    const hiddenStatuses: OverlayStatus[] = ["idle", "error"];
    expect(hiddenStatuses.map((status) => overlayPlaceholder(status))).toEqual(["", ""]);
  });
});

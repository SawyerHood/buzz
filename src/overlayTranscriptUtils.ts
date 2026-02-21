export type OverlayStatus = "idle" | "listening" | "transcribing" | "error";

export function shouldAppendTranscriptionDelta(status: OverlayStatus): boolean {
  return status === "listening" || status === "transcribing";
}

export function overlayPlaceholder(status: OverlayStatus): string {
  if (status === "transcribing") {
    return "Transcribing...";
  }

  if (status === "listening") {
    return "Listening...";
  }

  return "";
}

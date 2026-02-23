export type OnboardingPracticeStatus = "idle" | "listening" | "transcribing" | "error";
export type OnboardingAuthMethod = "oauth" | "api_key";
export type OnboardingAuthStatus = { accountId: string } | null;

function normalizeTranscriptText(value: unknown): string {
  if (typeof value !== "string") {
    return "";
  }
  return value.trim();
}

export function extractTranscriptText(payload: unknown): string {
  if (typeof payload === "string") {
    return payload.trim();
  }

  if (!payload || typeof payload !== "object") {
    return "";
  }

  const eventPayload = payload as Record<string, unknown>;

  const fromText = normalizeTranscriptText(eventPayload.text);
  if (fromText.length > 0) {
    return fromText;
  }

  return normalizeTranscriptText(eventPayload.transcript);
}

export function practiceStatusLabel(status: OnboardingPracticeStatus): string {
  switch (status) {
    case "listening":
      return "Recording";
    case "transcribing":
      return "Transcribing";
    case "error":
      return "Error";
    default:
      return "Ready";
  }
}

export function shouldShowOnboardingApiKeyInput(selectedAuthMethod: OnboardingAuthMethod): boolean {
  return selectedAuthMethod === "api_key";
}

export function onboardingAuthSuccessMessage({
  chatgptAuthStatus,
  hasApiKey,
  authActionCompleted,
}: {
  chatgptAuthStatus: OnboardingAuthStatus;
  hasApiKey: boolean;
  authActionCompleted: boolean;
}): string {
  if (!authActionCompleted) {
    return "";
  }

  if (chatgptAuthStatus) {
    return `ChatGPT connected (${chatgptAuthStatus.accountId}).`;
  }

  if (hasApiKey) {
    return "OpenAI API key saved.";
  }

  return "";
}

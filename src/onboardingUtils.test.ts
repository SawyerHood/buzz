import { describe, expect, it } from "vitest";

import {
  extractTranscriptText,
  onboardingAuthSuccessMessage,
  practiceStatusLabel,
  shouldShowOnboardingApiKeyInput,
} from "./onboardingUtils";

describe("onboardingUtils", () => {
  it("extracts transcript text from known payload shapes", () => {
    expect(extractTranscriptText("  hello world  ")).toBe("hello world");
    expect(extractTranscriptText({ text: "  from-text-field  " })).toBe("from-text-field");
    expect(extractTranscriptText({ transcript: "  from-transcript-field  " })).toBe(
      "from-transcript-field"
    );
  });

  it("returns an empty string when transcript payload is missing", () => {
    expect(extractTranscriptText({})).toBe("");
    expect(extractTranscriptText({ text: "   " })).toBe("");
    expect(extractTranscriptText(null)).toBe("");
  });

  it("maps onboarding practice statuses to display labels", () => {
    expect(practiceStatusLabel("idle")).toBe("Ready");
    expect(practiceStatusLabel("listening")).toBe("Recording");
    expect(practiceStatusLabel("transcribing")).toBe("Transcribing");
    expect(practiceStatusLabel("error")).toBe("Error");
  });

  it("always shows the API key input when API key auth is selected", () => {
    expect(shouldShowOnboardingApiKeyInput("api_key")).toBe(true);
    expect(shouldShowOnboardingApiKeyInput("oauth")).toBe(false);
  });

  it("only returns auth success messaging after an explicit auth action", () => {
    expect(
      onboardingAuthSuccessMessage({
        chatgptAuthStatus: null,
        hasApiKey: true,
        authActionCompleted: false,
      }),
    ).toBe("");

    expect(
      onboardingAuthSuccessMessage({
        chatgptAuthStatus: null,
        hasApiKey: true,
        authActionCompleted: true,
      }),
    ).toBe("OpenAI API key saved.");

    expect(
      onboardingAuthSuccessMessage({
        chatgptAuthStatus: { accountId: "acct_123" },
        hasApiKey: false,
        authActionCompleted: true,
      }),
    ).toBe("ChatGPT connected (acct_123).");
  });
});

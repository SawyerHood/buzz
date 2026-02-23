import { useCallback, useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { CheckCircle2, Circle, Loader2, Mic, Shield, Sparkles } from "lucide-react";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { cn } from "@/lib/utils";

type PermissionState = "not_determined" | "granted" | "denied";
type PermissionSnapshot = {
  microphone: PermissionState;
  accessibility: PermissionState;
  allGranted: boolean;
};
type ChatGptAuthStatus = {
  accountId: string;
  expiresAt: number;
};
type VoiceSettings = {
  hotkey_shortcut: string;
};
type AuthMethod = "oauth" | "api_key";

type OnboardingProps = {
  onComplete: () => void;
};

const OPENAI_PROVIDER = "openai";
const TOTAL_STEPS = 5;
const STEP_TITLES = ["Welcome", "Microphone", "Accessibility", "Authentication", "Done"] as const;

function toErrorMessage(error: unknown, fallback: string): string {
  if (typeof error === "string" && error.trim().length > 0) return error;
  if (error instanceof Error && error.message.trim().length > 0) return error.message;
  return fallback;
}

export default function Onboarding({ onComplete }: OnboardingProps) {
  const [step, setStep] = useState(0);
  const [permissions, setPermissions] = useState<PermissionSnapshot | null>(null);
  const [hasApiKey, setHasApiKey] = useState(false);
  const [chatgptAuthStatus, setChatgptAuthStatus] = useState<ChatGptAuthStatus | null>(null);
  const [selectedAuthMethod, setSelectedAuthMethod] = useState<AuthMethod>("oauth");
  const [apiKeyDraft, setApiKeyDraft] = useState("");
  const [hotkeyShortcut, setHotkeyShortcut] = useState("Alt+Space");
  const [errorMessage, setErrorMessage] = useState("");
  const [isLoadingInitialState, setIsLoadingInitialState] = useState(true);
  const [isRequestingMic, setIsRequestingMic] = useState(false);
  const [isOpeningAccessibilitySettings, setIsOpeningAccessibilitySettings] = useState(false);
  const [isStartingOauth, setIsStartingOauth] = useState(false);
  const [isSavingApiKey, setIsSavingApiKey] = useState(false);
  const [isCompleting, setIsCompleting] = useState(false);

  const micGranted = permissions?.microphone === "granted";
  const accessibilityGranted = permissions?.accessibility === "granted";
  const authConfigured = hasApiKey || chatgptAuthStatus !== null;

  const authSuccessMessage = useMemo(() => {
    if (chatgptAuthStatus) {
      return `ChatGPT connected (${chatgptAuthStatus.accountId}).`;
    }
    if (hasApiKey) {
      return "OpenAI API key saved.";
    }
    return "";
  }, [chatgptAuthStatus, hasApiKey]);

  const refreshPermissionStatus = useCallback(async () => {
    try {
      const snapshot = await invoke<PermissionSnapshot>("check_permissions");
      setPermissions(snapshot);
    } catch {
      // Ignore transient polling errors
    }
  }, []);

  const loadInitialState = useCallback(async () => {
    setIsLoadingInitialState(true);
    try {
      const [snapshot, apiKeyPresent, authStatus, settings] = await Promise.all([
        invoke<PermissionSnapshot>("check_permissions"),
        invoke<boolean>("has_api_key", { provider: OPENAI_PROVIDER }),
        invoke<ChatGptAuthStatus | null>("get_auth_status"),
        invoke<VoiceSettings>("get_settings"),
      ]);

      setPermissions(snapshot);
      setHasApiKey(apiKeyPresent);
      setChatgptAuthStatus(authStatus);
      setHotkeyShortcut(settings.hotkey_shortcut || "Alt+Space");
      setErrorMessage("");
    } catch (error) {
      setErrorMessage(toErrorMessage(error, "Unable to load onboarding state."));
    } finally {
      setIsLoadingInitialState(false);
    }
  }, []);

  useEffect(() => {
    void loadInitialState();
  }, [loadInitialState]);

  useEffect(() => {
    if (step !== 1 && step !== 2) return undefined;
    void refreshPermissionStatus();
    const interval = window.setInterval(() => {
      void refreshPermissionStatus();
    }, 1200);
    return () => window.clearInterval(interval);
  }, [refreshPermissionStatus, step]);

  useEffect(() => {
    const handleFocus = () => {
      void refreshPermissionStatus();
    };
    window.addEventListener("focus", handleFocus);
    return () => window.removeEventListener("focus", handleFocus);
  }, [refreshPermissionStatus]);

  const handleRequestMic = useCallback(async () => {
    setIsRequestingMic(true);
    setErrorMessage("");
    try {
      const snapshot = await invoke<PermissionSnapshot>("request_mic_permission");
      setPermissions(snapshot);
    } catch (error) {
      setErrorMessage(toErrorMessage(error, "Unable to request microphone access."));
    } finally {
      setIsRequestingMic(false);
    }
  }, []);

  const handleOpenAccessibilitySettings = useCallback(async () => {
    setIsOpeningAccessibilitySettings(true);
    setErrorMessage("");
    try {
      await invoke("open_accessibility_settings");
      void refreshPermissionStatus();
    } catch (error) {
      setErrorMessage(toErrorMessage(error, "Unable to open Accessibility settings."));
    } finally {
      setIsOpeningAccessibilitySettings(false);
    }
  }, [refreshPermissionStatus]);

  const handleStartOauth = useCallback(async () => {
    setSelectedAuthMethod("oauth");
    setIsStartingOauth(true);
    setErrorMessage("");
    try {
      const status = await invoke<ChatGptAuthStatus>("start_oauth_login");
      setChatgptAuthStatus(status);
    } catch (error) {
      setErrorMessage(toErrorMessage(error, "ChatGPT login failed."));
    } finally {
      setIsStartingOauth(false);
    }
  }, []);

  const handleSaveApiKey = useCallback(async () => {
    const key = apiKeyDraft.trim();
    if (!key) {
      setErrorMessage("Paste an API key before saving.");
      return;
    }

    setSelectedAuthMethod("api_key");
    setIsSavingApiKey(true);
    setErrorMessage("");
    try {
      await invoke("save_api_key", { provider: OPENAI_PROVIDER, key });
      setHasApiKey(true);
      setApiKeyDraft("");
    } catch (error) {
      setErrorMessage(toErrorMessage(error, "Unable to save API key."));
    } finally {
      setIsSavingApiKey(false);
    }
  }, [apiKeyDraft]);

  const handleCompleteOnboarding = useCallback(async () => {
    setIsCompleting(true);
    setErrorMessage("");
    try {
      await invoke<boolean>("complete_onboarding");
      onComplete();
    } catch (error) {
      setErrorMessage(toErrorMessage(error, "Unable to complete onboarding."));
    } finally {
      setIsCompleting(false);
    }
  }, [onComplete]);

  const renderStep = () => {
    if (step === 0) {
      return (
        <div className="space-y-6 text-center">
          <div className="space-y-2">
            <h2 className="text-2xl font-semibold tracking-tight">Welcome to Buzz 🐝</h2>
            <p className="text-sm text-muted-foreground">
              Voice-to-text with a quick buzz. Let&apos;s get you set up in 3 quick steps.
            </p>
          </div>
          <Button onClick={() => setStep(1)} size="sm">
            Get Started
          </Button>
        </div>
      );
    }

    if (step === 1) {
      return (
        <div className="space-y-5">
          <div className="space-y-2">
            <h2 className="text-xl font-semibold tracking-tight">Allow microphone access</h2>
            <p className="text-sm text-muted-foreground">
              Buzz needs microphone access to capture your voice and transcribe it to text.
            </p>
          </div>

          <div className="flex items-center gap-2 rounded-lg border bg-muted/30 px-3 py-2 text-sm">
            {micGranted ? (
              <CheckCircle2 className="size-4 text-emerald-500" />
            ) : (
              <Circle className="size-4 text-muted-foreground" />
            )}
            <span>{micGranted ? "Microphone permission granted" : "Waiting for microphone access"}</span>
          </div>

          <div className="flex items-center justify-between">
            <Button variant="outline" onClick={handleRequestMic} disabled={isRequestingMic}>
              {isRequestingMic ? (
                <>
                  <Loader2 className="size-4 animate-spin" />
                  Requesting...
                </>
              ) : (
                <>
                  <Mic className="size-4" />
                  Grant Microphone Access
                </>
              )}
            </Button>
            <Button onClick={() => setStep(2)} disabled={!micGranted}>
              Continue
            </Button>
          </div>
        </div>
      );
    }

    if (step === 2) {
      return (
        <div className="space-y-5">
          <div className="space-y-2">
            <h2 className="text-xl font-semibold tracking-tight">Allow accessibility access</h2>
            <p className="text-sm text-muted-foreground">
              Buzz uses Accessibility to insert transcribed text at your cursor in other apps.
            </p>
          </div>

          <div className="flex items-center gap-2 rounded-lg border bg-muted/30 px-3 py-2 text-sm">
            {accessibilityGranted ? (
              <CheckCircle2 className="size-4 text-emerald-500" />
            ) : (
              <Circle className="size-4 text-muted-foreground" />
            )}
            <span>
              {accessibilityGranted
                ? "Accessibility permission granted"
                : "Waiting for accessibility access"}
            </span>
          </div>

          <p className="text-xs text-muted-foreground">
            macOS requires you to manually enable Buzz in System Settings after opening the panel.
          </p>

          <div className="flex items-center justify-between">
            <Button
              variant="outline"
              onClick={handleOpenAccessibilitySettings}
              disabled={isOpeningAccessibilitySettings}
            >
              {isOpeningAccessibilitySettings ? (
                <>
                  <Loader2 className="size-4 animate-spin" />
                  Opening...
                </>
              ) : (
                <>
                  <Shield className="size-4" />
                  Open System Settings
                </>
              )}
            </Button>
            <Button onClick={() => setStep(3)} disabled={!accessibilityGranted}>
              Continue
            </Button>
          </div>
        </div>
      );
    }

    if (step === 3) {
      return (
        <div className="space-y-5">
          <div className="space-y-2">
            <h2 className="text-xl font-semibold tracking-tight">Choose authentication</h2>
            <p className="text-sm text-muted-foreground">
              Connect ChatGPT or save an OpenAI API key to start transcribing.
            </p>
          </div>

          <div className="grid gap-3 md:grid-cols-2">
            <Card
              className={cn(
                "border transition-colors",
                selectedAuthMethod === "oauth" ? "border-primary" : "border-border"
              )}
            >
              <CardHeader className="pb-2">
                <CardTitle className="text-sm">Login with ChatGPT</CardTitle>
                <CardDescription>Use your ChatGPT account with OAuth.</CardDescription>
              </CardHeader>
              <CardContent>
                <Button
                  size="sm"
                  className="w-full"
                  onClick={handleStartOauth}
                  disabled={isStartingOauth}
                >
                  {isStartingOauth ? (
                    <>
                      <Loader2 className="size-4 animate-spin" />
                      Opening browser...
                    </>
                  ) : (
                    "Login with ChatGPT"
                  )}
                </Button>
                <Button
                  size="xs"
                  variant="ghost"
                  className="mt-2 w-full"
                  onClick={() => setSelectedAuthMethod("oauth")}
                >
                  Select this option
                </Button>
              </CardContent>
            </Card>

            <Card
              className={cn(
                "border transition-colors",
                selectedAuthMethod === "api_key" ? "border-primary" : "border-border"
              )}
            >
              <CardHeader className="pb-2">
                <CardTitle className="text-sm">Use API Key</CardTitle>
                <CardDescription>Paste an OpenAI key to authenticate directly.</CardDescription>
              </CardHeader>
              <CardContent className="space-y-2">
                <Input
                  value={apiKeyDraft}
                  onChange={(event) => setApiKeyDraft(event.currentTarget.value)}
                  placeholder="sk-..."
                  className="h-8 text-xs font-mono"
                  autoComplete="off"
                />
                <Button
                  size="sm"
                  className="w-full"
                  onClick={handleSaveApiKey}
                  disabled={isSavingApiKey}
                >
                  {isSavingApiKey ? (
                    <>
                      <Loader2 className="size-4 animate-spin" />
                      Saving...
                    </>
                  ) : (
                    "Save API Key"
                  )}
                </Button>
                <Button
                  size="xs"
                  variant="ghost"
                  className="w-full"
                  onClick={() => setSelectedAuthMethod("api_key")}
                >
                  Select this option
                </Button>
              </CardContent>
            </Card>
          </div>

          {authConfigured && (
            <Alert className="border-emerald-500/30 bg-emerald-50/60 dark:bg-emerald-950/20">
              <AlertDescription className="flex items-center gap-2 text-emerald-700 dark:text-emerald-300">
                <CheckCircle2 className="size-4" />
                {authSuccessMessage}
              </AlertDescription>
            </Alert>
          )}

          <div className="flex justify-end">
            <Button onClick={() => setStep(4)} disabled={!authConfigured}>
              Finish Setup
            </Button>
          </div>
        </div>
      );
    }

    return (
      <div className="space-y-6 text-center">
        <div className="space-y-2">
          <h2 className="text-2xl font-semibold tracking-tight">You&apos;re all set! 🐝</h2>
          <p className="text-sm text-muted-foreground">
            Press <span className="font-medium text-foreground">{hotkeyShortcut}</span> to record,
            then Buzz will transcribe and insert your text where your cursor is.
          </p>
        </div>
        <Button onClick={handleCompleteOnboarding} disabled={isCompleting}>
          {isCompleting ? (
            <>
              <Loader2 className="size-4 animate-spin" />
              Finalizing...
            </>
          ) : (
            "Start Using Buzz"
          )}
        </Button>
      </div>
    );
  };

  return (
    <main className="flex min-h-screen items-center justify-center bg-gradient-to-b from-background to-muted/30 px-4 py-8">
      <Card className="w-full max-w-2xl border-border/70 shadow-sm">
        <CardHeader className="space-y-4 pb-1">
          <div className="flex items-center justify-center gap-2 text-xs text-muted-foreground">
            <Sparkles className="size-3.5" />
            First-run setup
          </div>

          <div className="space-y-2">
            <div className="flex items-center justify-center gap-2">
              {STEP_TITLES.map((title, index) => (
                <div key={title} className="flex items-center gap-2">
                  <div
                    className={cn(
                      "size-2.5 rounded-full transition-colors",
                      index <= step ? "bg-primary" : "bg-muted"
                    )}
                  />
                  {index < STEP_TITLES.length - 1 && (
                    <div
                      className={cn(
                        "h-px w-5 transition-colors",
                        index < step ? "bg-primary/70" : "bg-border"
                      )}
                    />
                  )}
                </div>
              ))}
            </div>
            <p className="text-center text-xs text-muted-foreground">
              Step {step + 1} of {TOTAL_STEPS}: {STEP_TITLES[step]}
            </p>
          </div>
        </CardHeader>

        <CardContent className="py-6">
          {isLoadingInitialState ? (
            <div className="flex items-center justify-center gap-2 py-12 text-sm text-muted-foreground">
              <Loader2 className="size-4 animate-spin" />
              Loading onboarding...
            </div>
          ) : (
            <div key={step} className="animate-in fade-in-0 slide-in-from-bottom-1 duration-300">
              {renderStep()}
            </div>
          )}

          {errorMessage && (
            <Alert variant="destructive" className="mt-5 py-2">
              <AlertDescription className="text-xs">{errorMessage}</AlertDescription>
            </Alert>
          )}
        </CardContent>
      </Card>
    </main>
  );
}

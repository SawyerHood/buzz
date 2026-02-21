# Personal Voice-to-Text App Plan

## Project Vision
Build a fast, reliable macOS desktop voice-to-text tool for personal daily use, similar to WhisperFlow / SuperWhisper / Monologue.

The product should feel instant: trigger anywhere, speak naturally, and get high-quality text inserted into the active app with minimal friction.

This is a personal utility, not a startup SaaS. Prioritize reliability, speed of iteration, and low maintenance overhead.

## Locked Decisions
1. Platform: macOS-only for MVP.
2. Transcription strategy: cloud-first for MVP.
3. Insertion UX: auto-insert on completion (no preview step).
4. Recording interaction: support both hold-to-talk and toggle recording.
5. Transcript retention: store history locally by default.
6. Primary use cases: mixed (short chat replies + long-form dictation).
7. Streaming partial transcripts: post-MVP (not required for v1).
8. Day-one integrations: none.

## Goals
- Trigger recording instantly with a global hotkey.
- Support both quick utterances and longer dictation sessions.
- Produce high-quality transcription with punctuation and readable formatting.
- Insert text into the currently focused app automatically.
- Keep settings and operations simple for one-person maintenance.

## Non-Goals (MVP)
- Cross-platform support (Windows/Linux).
- Team collaboration, multi-user accounts, or cloud sync.
- Enterprise security/compliance features.
- Complex audio editing workflows.
- Third-party integrations at launch.

## Core Features
1. Global activation
- Configurable system-wide hotkey.
- Two recording modes in v1:
  - hold-to-talk (press/hold to record, release to transcribe)
  - toggle recording (press once to start, press again to stop)

2. Audio capture
- Microphone selection.
- Basic input level feedback.
- Stable buffering/start-stop behavior for short and long recordings.

3. Transcription
- Cloud transcription provider as default in MVP.
- Provider abstraction kept from day 1 so local/offline providers can be added later.
- Language setting with `auto` and manual override.
- Lightweight text cleanup (punctuation and capitalization normalization).

4. Text insertion
- Auto-insert transcript immediately on completion.
- Reliable insertion strategy with fallback order:
  - direct input simulation
  - clipboard paste fallback
- Optional copy-only mode for manual paste (non-default).

5. UX shell
- Menubar app with status indicator (`idle`, `listening`, `transcribing`, `error`).
- Lightweight settings window.
- Local transcript history view with copy/reinsert actions.

6. Reliability and diagnostics
- Clear permission guidance (microphone, accessibility/input monitoring as needed).
- User-visible errors for provider/network/permission failures.
- Retry policy for transient API/network errors.
- Local logs for debugging.

## Tech Stack Recommendation

### Desktop framework
**Tauri v2 + React + TypeScript**
- Lightweight runtime and packaging.
- Good fit for menubar-style utility apps.
- Native integrations handled in Rust where required (global hotkeys, accessibility-driven insertion, OS hooks).

### Transcription
- MVP provider: **OpenAI transcription API** (cloud-first path to fast delivery).
- Internal interface: `TranscriptionProvider` with methods like `transcribe(audio, options)`.
- Post-MVP provider: local/offline engine (e.g., `faster-whisper` or `whisper.cpp`).

### Storage and secrets
- Local SQLite (or simple JSON in earliest prototype) for settings/history.
- API keys stored in macOS keychain via secure credential APIs.

### Packaging
- Tauri bundle for macOS distribution.
- Auto-update deferred until after v1 proves stable.

## Architecture Overview
Use a small modular architecture with clear boundaries.

### Modules
- `HotkeyService`
- `AudioCaptureService`
- `TranscriptionOrchestrator`
- `Providers/OpenAIProvider`
- `TextInsertionService`
- `SettingsStore`
- `HistoryStore`
- `PermissionService`
- `StatusNotifier`

### Primary runtime flow
1. User triggers recording (hold or toggle).
2. `AudioCaptureService` streams and buffers microphone audio.
3. Recording stops.
4. `TranscriptionOrchestrator` finalizes audio and calls the cloud provider.
5. Transcript is normalized.
6. `TextInsertionService` auto-inserts into focused app.
7. Transcript is saved locally and UI status updates.

### Boundaries
- UI/renderer does not contain provider-specific logic.
- OS-level behavior stays in native/backend services.
- Transcription providers remain swappable through one interface.

## MVP Scope

### Phase 1: End-to-end daily-driver path
- Menubar shell with status states.
- Global hotkey registration.
- Both recording modes (hold and toggle).
- Microphone capture and cloud transcription.
- Auto-insert with clipboard fallback.

### Phase 2: Practical usability
- Settings (hotkey, mode, mic, language, provider/API key).
- Local transcript history with copy/reinsert.
- Startup at login.
- Basic retry/error handling and permission onboarding.

### Phase 3: Hardening
- Better insertion reliability across app contexts.
- Diagnostic logging and export.
- Performance tuning for latency and long dictation stability.

## Post-MVP Features
- Streaming partial transcripts during recording.
- Local offline transcription provider.
- Voice commands (example: “new line”, “send”, “undo”).
- Per-app behavior profiles.
- Custom vocabulary/phrase boosting.
- Optional text transforms (rewrite/summarize/translate).

## Risks and Mitigations
- Hotkey conflicts with other apps.
  - Mitigation: conflict detection and one-click remapping.

- Insertion inconsistency across target apps.
  - Mitigation: ordered fallback strategy plus per-app overrides later.

- Network/API latency spikes from cloud transcription.
  - Mitigation: clear transcribing state, retry policy, and audio chunk handling.

- macOS permission friction.
  - Mitigation: guided setup checklist and diagnostics page.

## Suggested Milestones
1. Bootstrap app skeleton (Tauri + menubar + state model).
2. Deliver full happy path (hotkey -> record -> transcribe -> auto-insert).
3. Add settings, API key handling, and local history.
4. Hardening pass (permissions, fallbacks, retries, logs).
5. Use as daily driver and iterate on pain points.

## Open Questions
No blocking open questions for v1 planning at this time.

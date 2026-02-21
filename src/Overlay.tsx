import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { useEffect, useRef, useState } from "react";
import { formatElapsedLabel } from "./overlayUtils";
import {
  overlayPlaceholder,
  shouldAppendTranscriptionDelta,
  type OverlayStatus,
} from "./overlayTranscriptUtils";
import "./Overlay.css";

type AppStatus = OverlayStatus;

const EVENT_STATUS_CHANGED = "voice://status-changed";
const EVENT_TRANSCRIPTION_DELTA = "voice://transcription-delta";

function Overlay() {
  const [status, setStatus] = useState<AppStatus>("idle");
  const [elapsedMs, setElapsedMs] = useState(0);
  const [transcriptionPreview, setTranscriptionPreview] = useState("");
  const statusRef = useRef<AppStatus>("idle");
  const startedAtRef = useRef<number | null>(null);
  const transcriptScrollRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    let isMounted = true;
    let unlistenFns: UnlistenFn[] = [];

    const applyStatus = (nextStatus: AppStatus) => {
      const previousStatus = statusRef.current;
      statusRef.current = nextStatus;
      setStatus(nextStatus);

      if (nextStatus === "listening") {
        if (previousStatus !== "listening") {
          setTranscriptionPreview("");
          startedAtRef.current = Date.now();
          setElapsedMs(0);
        } else if (startedAtRef.current === null) {
          startedAtRef.current = Date.now();
          setElapsedMs(0);
        }

        return;
      }

      if (nextStatus === "transcribing") {
        if (startedAtRef.current !== null) {
          setElapsedMs(Date.now() - startedAtRef.current);
          startedAtRef.current = null;
        }

        return;
      }

      setTranscriptionPreview("");
      startedAtRef.current = null;
      setElapsedMs(0);
    };

    async function bindOverlayEvents() {
      try {
        const initialStatus = await invoke<AppStatus>("get_status");

        if (!isMounted) {
          return;
        }

        applyStatus(initialStatus);
      } catch {
        // Overlay remains passive if backend sync is unavailable.
      }

      try {
        const listeners = await Promise.all([
          listen<AppStatus>(EVENT_STATUS_CHANGED, ({ payload }) => {
            applyStatus(payload);
          }),
          listen<string>(EVENT_TRANSCRIPTION_DELTA, ({ payload }) => {
            if (!shouldAppendTranscriptionDelta(statusRef.current)) {
              return;
            }

            if (!payload) {
              return;
            }

            setTranscriptionPreview((current) => current + payload);
          }),
        ]);

        if (!isMounted) {
          listeners.forEach((dispose) => dispose());
          return;
        }

        unlistenFns = listeners;
      } catch {
        // Overlay remains passive if backend listeners are unavailable.
      }
    }

    void bindOverlayEvents();

    return () => {
      isMounted = false;
      unlistenFns.forEach((dispose) => dispose());
    };
  }, []);

  useEffect(() => {
    if (status !== "listening") {
      return;
    }

    const interval = window.setInterval(() => {
      if (startedAtRef.current === null) {
        return;
      }

      setElapsedMs(Date.now() - startedAtRef.current);
    }, 100);

    return () => {
      window.clearInterval(interval);
    };
  }, [status]);

  const isListening = status === "listening";
  const isTranscribing = status === "transcribing";
  const placeholder = overlayPlaceholder(status) || "Listening...";

  useEffect(() => {
    if (!transcriptScrollRef.current) {
      return;
    }

    transcriptScrollRef.current.scrollLeft = transcriptScrollRef.current.scrollWidth;
  }, [status, transcriptionPreview]);

  return (
    <main className="overlay-root">
      <section
        className={`overlay-pill ${isListening ? "active" : ""} ${
          isTranscribing ? "transcribing" : ""
        }`}
      >
        <span className="recording-indicator" aria-hidden="true">
          <span className="recording-dot" />
        </span>
        <div className="overlay-transcript-scroll" ref={transcriptScrollRef} aria-live="polite">
          <p className={`overlay-transcript-text ${transcriptionPreview ? "" : "placeholder"}`}>
            {transcriptionPreview || placeholder}
          </p>
        </div>
        <p className="overlay-elapsed">{isListening ? formatElapsedLabel(elapsedMs) : "..."}</p>
      </section>
    </main>
  );
}

export default Overlay;

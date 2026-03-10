import { invoke } from "@tauri-apps/api/core";

type RendererMemorySource = "main" | "overlay";
type RendererMemoryDetails = Record<string, string | number | boolean | null>;

type PerformanceMemory = {
  usedJSHeapSize: number;
  totalJSHeapSize: number;
  jsHeapSizeLimit: number;
};

type PerformanceWithMemory = Performance & {
  memory?: PerformanceMemory;
};

export async function reportRendererMemory(
  source: RendererMemorySource,
  reason: string,
  details: RendererMemoryDetails,
): Promise<void> {
  if (!import.meta.env.DEV) return;

  const performanceWithMemory = window.performance as PerformanceWithMemory;
  const heap = performanceWithMemory.memory;

  try {
    await invoke("debug_report_renderer_memory", {
      snapshot: {
        source,
        reason,
        jsHeapBytes: heap?.usedJSHeapSize ?? null,
        totalJsHeapBytes: heap?.totalJSHeapSize ?? null,
        jsHeapLimitBytes: heap?.jsHeapSizeLimit ?? null,
        domNodeCount: document.getElementsByTagName("*").length,
        url: window.location.href,
        visibilityState: document.visibilityState,
        details,
      },
    });
  } catch {
    // Keep dev instrumentation non-blocking.
  }
}

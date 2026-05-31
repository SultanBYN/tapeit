import { createSignal } from "solid-js";
import { invoke } from "@tauri-apps/api/core";

export interface CaptureSource {
  id: string;
  name: string;
  source_type: "Screen" | "Window";
  width: number;
  height: number;
}

export interface RecordingConfig {
  source_id: string;
  fps: number;
  output_dir: string;
  record_audio: boolean;
  record_microphone: boolean;
}

export type RecordingStatus = "idle" | "recording" | "paused" | "encoding";

export interface RecorderState {
  status: RecordingStatus;
  sources: CaptureSource[];
  selectedSourceId: string | null;
  fps: number;
  recordAudio: boolean;
  recordMicrophone: boolean;
  outputPath: string | null;
  duration: number;
}

const DEFAULT_STATE: RecorderState = {
  status: "idle",
  sources: [],
  selectedSourceId: null,
  fps: 30,
  recordAudio: true,
  recordMicrophone: true,
  outputPath: null,
  duration: 0,
};

export function useRecorderStore() {
  const [state, setState] = createSignal<RecorderState>({ ...DEFAULT_STATE });
  let timerInterval: ReturnType<typeof setInterval> | null = null;

  async function loadSources() {
    try {
      const sources = await invoke<CaptureSource[]>("get_capture_sources");
      setState((prev) => ({
        ...prev,
        sources,
        selectedSourceId: sources.length > 0 ? sources[0].id : null,
      }));
    } catch (err) {
      console.error("Failed to load capture sources:", err);
    }
  }

  function setSelectedSource(id: string) {
    setState((prev) => ({ ...prev, selectedSourceId: id }));
  }

  function setFps(fps: number) {
    setState((prev) => ({ ...prev, fps }));
  }

  function setRecordAudio(value: boolean) {
    setState((prev) => ({ ...prev, recordAudio: value }));
  }

  function setRecordMicrophone(value: boolean) {
    setState((prev) => ({ ...prev, recordMicrophone: value }));
  }

  function startTimer() {
    setState((prev) => ({ ...prev, duration: 0 }));
    timerInterval = setInterval(() => {
      setState((prev) => ({ ...prev, duration: prev.duration + 1 }));
    }, 1000);
  }

  function stopTimer() {
    if (timerInterval) {
      clearInterval(timerInterval);
      timerInterval = null;
    }
  }

  async function startRecording() {
    const current = state();
    if (!current.selectedSourceId) {
      console.error("No source selected");
      return;
    }

    try {
      const outputDir =
        (await getDefaultOutputDir()) || "C:\\Users\\Public\\Videos\\Tapeit";

      const config: RecordingConfig = {
        source_id: current.selectedSourceId,
        fps: current.fps,
        output_dir: outputDir,
        record_audio: current.recordAudio,
        record_microphone: current.recordMicrophone,
      };

      const outputPath = await invoke<string>("start_recording", { config });
      setState((prev) => ({
        ...prev,
        status: "recording",
        outputPath,
      }));
      startTimer();

      // Show overlay and minimize main window
      await invoke("show_overlay");
      await invoke("minimize_main");
    } catch (err) {
      console.error("Failed to start recording:", err);
    }
  }

  async function stopRecording() {
    try {
      await invoke("stop_recording");
      stopTimer();
      setState((prev) => ({ ...prev, status: "idle" }));

      // Clean up overlay and restore main window
      await invoke("hide_overlay");
      await invoke("restore_main");
    } catch (err) {
      console.error("Failed to stop recording:", err);
    }
  }

  async function pauseRecording() {
    try {
      await invoke("pause_recording");
      stopTimer();
      setState((prev) => ({ ...prev, status: "paused" }));
    } catch (err) {
      console.error("Failed to pause recording:", err);
    }
  }

  async function resumeRecording() {
    try {
      await invoke("resume_recording");
      startTimer();
      setState((prev) => ({ ...prev, status: "recording" }));
    } catch (err) {
      console.error("Failed to resume recording:", err);
    }
  }

  async function toggleRecording() {
    const current = state();
    if (current.status === "idle") {
      await startRecording();
    } else if (current.status === "recording" || current.status === "paused") {
      await stopRecording();
    }
  }

  return {
    state,
    loadSources,
    setSelectedSource,
    setFps,
    setRecordAudio,
    setRecordMicrophone,
    startRecording,
    stopRecording,
    pauseRecording,
    resumeRecording,
    toggleRecording,
  };
}

async function getDefaultOutputDir(): Promise<string | null> {
  try {
    const { homeDir } = await import("@tauri-apps/api/path");
    const home = await homeDir();
    return `${home}Videos/Tapeit`;
  } catch {
    return null;
  }
}

import { Component, Show, createSignal, onCleanup, onMount } from "solid-js";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";

export const OverlayPanel: Component = () => {
  const [seconds, setSeconds] = createSignal(0);
  const [isPaused, setIsPaused] = createSignal(false);
  let timerInterval: ReturnType<typeof setInterval> | null = null;
  let dragStartX = 0;
  let dragStartY = 0;

  onMount(() => {
    startTimer();
  });

  onCleanup(() => {
    stopTimer();
  });

  function startTimer() {
    timerInterval = setInterval(() => {
      if (!isPaused()) {
        setSeconds((s) => s + 1);
      }
    }, 1000);
  }

  function stopTimer() {
    if (timerInterval) {
      clearInterval(timerInterval);
      timerInterval = null;
    }
  }

  function formatTime(totalSeconds: number): string {
    const hrs = Math.floor(totalSeconds / 3600);
    const mins = Math.floor((totalSeconds % 3600) / 60);
    const secs = totalSeconds % 60;
    const pad = (n: number) => n.toString().padStart(2, "0");
    if (hrs > 0) return `${pad(hrs)}:${pad(mins)}:${pad(secs)}`;
    return `${pad(mins)}:${pad(secs)}`;
  }

  async function handlePauseResume() {
    if (isPaused()) {
      await invoke("resume_recording");
      setIsPaused(false);
    } else {
      await invoke("pause_recording");
      setIsPaused(true);
    }
  }

  async function handleStop() {
    stopTimer();
    await invoke("stop_recording");
    await invoke("hide_overlay");
    await invoke("restore_main");
  }

  async function handleDragStart(e: MouseEvent) {
    // Use Tauri's built-in window drag
    await getCurrentWindow().startDragging();
  }

  return (
    <div class="overlay" onMouseDown={handleDragStart}>
      {/* Recording indicator dot */}
      <div class={`overlay-dot ${isPaused() ? "paused" : ""}`} />

      {/* Timer */}
      <span class="overlay-timer">{formatTime(seconds())}</span>

      {/* Controls */}
      <div
        class="overlay-controls"
        onMouseDown={(e) => e.stopPropagation()}
      >
        <button
          class="overlay-btn overlay-btn-pause"
          onClick={handlePauseResume}
          title={isPaused() ? "Resume" : "Pause"}
        >
          <Show when={isPaused()} fallback={<PauseIcon />}>
            <PlayIcon />
          </Show>
        </button>

        <button
          class="overlay-btn overlay-btn-stop"
          onClick={handleStop}
          title="Stop Recording"
        >
          <StopIcon />
        </button>
      </div>
    </div>
  );
};

const PauseIcon = () => (
  <svg width="14" height="14" viewBox="0 0 14 14" fill="currentColor">
    <rect x="2" y="1" width="3.5" height="12" rx="1" />
    <rect x="8.5" y="1" width="3.5" height="12" rx="1" />
  </svg>
);

const PlayIcon = () => (
  <svg width="14" height="14" viewBox="0 0 14 14" fill="currentColor">
    <path d="M3 1.5v11l9-5.5z" />
  </svg>
);

const StopIcon = () => (
  <svg width="12" height="12" viewBox="0 0 12 12" fill="currentColor">
    <rect x="1" y="1" width="10" height="10" rx="2" />
  </svg>
);

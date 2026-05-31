import { Component, For, Show, onMount } from "solid-js";
import { useRecorderStore } from "../../stores/recorder";
import { Timer } from "./Timer";
import { SourcePicker } from "./SourcePicker";
import { AudioControls } from "./AudioControls";

export const RecorderPanel: Component = () => {
  const store = useRecorderStore();

  onMount(() => {
    store.loadSources();
  });

  const isIdle = () => store.state().status === "idle";
  const isRecording = () => store.state().status === "recording";
  const isPaused = () => store.state().status === "paused";

  return (
    <div class="recorder-panel">
      {/* Source Selection */}
      <Show when={isIdle()}>
        <SourcePicker
          sources={store.state().sources}
          selectedId={store.state().selectedSourceId}
          onSelect={store.setSelectedSource}
          onRefresh={store.loadSources}
        />

        <AudioControls
          recordAudio={store.state().recordAudio}
          recordMicrophone={store.state().recordMicrophone}
          onToggleAudio={store.setRecordAudio}
          onToggleMicrophone={store.setRecordMicrophone}
        />

        {/* FPS Selector */}
        <div class="card fps-card">
          <div class="label">Frame Rate</div>
          <div class="fps-options">
            <For each={[15, 24, 30, 60]}>
              {(fps) => (
                <button
                  class={`fps-btn ${store.state().fps === fps ? "active" : ""}`}
                  onClick={() => store.setFps(fps)}
                >
                  {fps}
                </button>
              )}
            </For>
          </div>
        </div>
      </Show>

      {/* Recording Timer */}
      <Show when={!isIdle()}>
        <Timer seconds={store.state().duration} />
      </Show>

      {/* Controls */}
      <div class="controls">
        <Show when={isIdle()}>
          <button
            class="btn btn-record"
            onClick={store.startRecording}
            disabled={!store.state().selectedSourceId}
            title="Start Recording"
          >
            &#9679;
          </button>
        </Show>

        <Show when={isRecording()}>
          <button
            class="btn btn-icon btn-secondary"
            onClick={store.pauseRecording}
            title="Pause"
          >
            &#10074;&#10074;
          </button>
          <button
            class="btn btn-record recording"
            onClick={store.stopRecording}
            title="Stop Recording"
          >
            &#9632;
          </button>
        </Show>

        <Show when={isPaused()}>
          <button
            class="btn btn-icon btn-primary"
            onClick={store.resumeRecording}
            title="Resume"
          >
            &#9654;
          </button>
          <button
            class="btn btn-record"
            onClick={store.stopRecording}
            title="Stop Recording"
          >
            &#9632;
          </button>
        </Show>
      </div>
    </div>
  );
};

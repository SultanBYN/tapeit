import { Component } from "solid-js";

interface AudioControlsProps {
  recordAudio: boolean;
  recordMicrophone: boolean;
  onToggleAudio: (value: boolean) => void;
  onToggleMicrophone: (value: boolean) => void;
}

export const AudioControls: Component<AudioControlsProps> = (props) => {
  return (
    <div class="card audio-controls">
      <div class="label">Audio</div>

      <div class="audio-row">
        <span class="audio-label">System Audio</span>
        <button
          class={`toggle ${props.recordAudio ? "active" : ""}`}
          onClick={() => props.onToggleAudio(!props.recordAudio)}
          role="switch"
          aria-checked={props.recordAudio}
        />
      </div>

      <div class="audio-row">
        <span class="audio-label">Microphone</span>
        <button
          class={`toggle ${props.recordMicrophone ? "active" : ""}`}
          onClick={() => props.onToggleMicrophone(!props.recordMicrophone)}
          role="switch"
          aria-checked={props.recordMicrophone}
        />
      </div>
    </div>
  );
};

import { Component, For } from "solid-js";
import type { CaptureSource } from "../../stores/recorder";

interface SourcePickerProps {
  sources: CaptureSource[];
  selectedId: string | null;
  onSelect: (id: string) => void;
  onRefresh: () => void;
}

export const SourcePicker: Component<SourcePickerProps> = (props) => {
  return (
    <div class="card source-picker">
      <div class="source-header">
        <div class="label">Capture Source</div>
        <button class="btn-refresh" onClick={props.onRefresh} title="Refresh sources">
          &#8635;
        </button>
      </div>

      <select
        class="select"
        value={props.selectedId || ""}
        onChange={(e) => props.onSelect(e.currentTarget.value)}
      >
        <For each={props.sources}>
          {(source) => (
            <option value={source.id}>
              {source.source_type === "Screen" ? "🖥" : "🪟"}{" "}
              {source.name} ({source.width}x{source.height})
            </option>
          )}
        </For>
        {props.sources.length === 0 && (
          <option disabled>No sources found — grant permission</option>
        )}
      </select>
    </div>
  );
};

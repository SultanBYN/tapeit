import { Component, Show } from "solid-js";
import { RecorderPanel } from "./components/recorder/RecorderPanel";
import { useRecorderStore } from "./stores/recorder";

const App: Component = () => {
  const { state } = useRecorderStore();

  return (
    <div class="app">
      <header class="app-header">
        <h1 class="app-title">Tapeit</h1>
        <Show when={state().status !== "idle"}>
          <span class="recording-badge">{state().status}</span>
        </Show>
      </header>

      <main class="app-main">
        <RecorderPanel />
      </main>

      <footer class="app-footer">
        <span class="shortcut-hint">Ctrl+Shift+R to toggle recording</span>
      </footer>
    </div>
  );
};

export default App;

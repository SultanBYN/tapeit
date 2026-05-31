import { render } from "solid-js/web";
import { OverlayPanel } from "./components/overlay/OverlayPanel";

render(() => <OverlayPanel />, document.getElementById("overlay-root")!);

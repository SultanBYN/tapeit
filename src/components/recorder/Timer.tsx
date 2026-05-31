import { Component } from "solid-js";

interface TimerProps {
  seconds: number;
}

export const Timer: Component<TimerProps> = (props) => {
  const formatted = () => {
    const hrs = Math.floor(props.seconds / 3600);
    const mins = Math.floor((props.seconds % 3600) / 60);
    const secs = props.seconds % 60;

    const pad = (n: number) => n.toString().padStart(2, "0");

    if (hrs > 0) {
      return `${pad(hrs)}:${pad(mins)}:${pad(secs)}`;
    }
    return `${pad(mins)}:${pad(secs)}`;
  };

  return (
    <div class="timer">
      <div class="timer-dot" />
      <span class="timer-text">{formatted()}</span>
    </div>
  );
};

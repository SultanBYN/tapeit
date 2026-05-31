use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use scap::{
    capturer::{Capturer, Options, Resolution},
    frame::Frame,
    Target,
};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

use crate::encoder::VideoEncoder;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RecordingState {
    Idle,
    Recording,
    Paused,
    Encoding,
}

pub struct ScreenRecorder {
    state: Arc<std::sync::Mutex<RecordingState>>,
    is_paused: Arc<AtomicBool>,
    stop_signal: Option<mpsc::Sender<()>>,
}

impl ScreenRecorder {
    pub fn new() -> Self {
        Self {
            state: Arc::new(std::sync::Mutex::new(RecordingState::Idle)),
            is_paused: Arc::new(AtomicBool::new(false)),
            stop_signal: None,
        }
    }

    pub fn state(&self) -> RecordingState {
        self.state.lock().unwrap().clone()
    }

    /// Start capturing frames from the given target.
    /// Frames are sent over a channel to the encoder.
    pub fn start(
        &mut self,
        target: Target,
        output_path: PathBuf,
        fps: u32,
    ) -> Result<(), String> {
        let current_state = self.state();
        if current_state != RecordingState::Idle {
            return Err(format!("Cannot start: recorder is in {:?} state", current_state));
        }

        let options = Options {
            fps,
            show_cursor: true,
            show_highlight: false,
            target: Some(target),
            output_type: scap::frame::FrameType::BGRAFrame,
            output_resolution: Resolution::_1080p,
            ..Default::default()
        };

        let mut capturer = Capturer::build(options);
        capturer.start_capture();

        let (stop_tx, mut stop_rx) = mpsc::channel::<()>(1);
        self.stop_signal = Some(stop_tx);

        let state = Arc::clone(&self.state);
        let is_paused = Arc::clone(&self.is_paused);

        // Set state to recording
        *state.lock().unwrap() = RecordingState::Recording;

        // Spawn capture loop in a background thread
        std::thread::spawn(move || {
            let mut frames: Vec<Frame> = Vec::new();

            loop {
                // Check for stop signal (non-blocking)
                if stop_rx.try_recv().is_ok() {
                    break;
                }

                // Skip frames while paused
                if is_paused.load(Ordering::Relaxed) {
                    std::thread::sleep(std::time::Duration::from_millis(50));
                    continue;
                }

                // Capture next frame
                match capturer.get_next_frame() {
                    Ok(frame) => {
                        frames.push(frame);
                    }
                    Err(e) => {
                        log::error!("Frame capture error: {:?}", e);
                        break;
                    }
                }
            }

            capturer.stop_capture();

            // Encode captured frames
            *state.lock().unwrap() = RecordingState::Encoding;
            log::info!("Encoding {} frames to {:?}", frames.len(), output_path);

            match VideoEncoder::encode_frames(&frames, &output_path, fps) {
                Ok(_) => log::info!("Recording saved to {:?}", output_path),
                Err(e) => log::error!("Encoding failed: {:?}", e),
            }

            *state.lock().unwrap() = RecordingState::Idle;
        });

        Ok(())
    }

    pub fn stop(&mut self) -> Result<(), String> {
        if let Some(tx) = self.stop_signal.take() {
            let _ = tx.blocking_send(());
            Ok(())
        } else {
            Err("No active recording to stop".into())
        }
    }

    pub fn pause(&self) -> Result<(), String> {
        if self.state() != RecordingState::Recording {
            return Err("Not recording".into());
        }
        self.is_paused.store(true, Ordering::Relaxed);
        *self.state.lock().unwrap() = RecordingState::Paused;
        Ok(())
    }

    pub fn resume(&self) -> Result<(), String> {
        if self.state() != RecordingState::Paused {
            return Err("Not paused".into());
        }
        self.is_paused.store(false, Ordering::Relaxed);
        *self.state.lock().unwrap() = RecordingState::Recording;
        Ok(())
    }
}

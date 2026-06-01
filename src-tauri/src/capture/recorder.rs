use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use scap::{
    capturer::{Capturer, Options, Resolution},
    frame::Frame,
    Target,
};
use serde::{Deserialize, Serialize};
use std::sync::mpsc;
use tauri::{AppHandle, Emitter};

use crate::audio::{AudioData, AudioHandle, start_audio_capture};
use crate::encoder::{StreamingEncoder, VideoEncoder};

/// Wrapper to send scap Options across threads.
/// SAFETY: Options contains Target which holds HWND/HMONITOR raw pointers.
/// These are process-wide handles on Windows, valid from any thread.
struct SendOptions(Options);
unsafe impl Send for SendOptions {}
impl SendOptions {
    fn into_inner(self) -> Options { self.0 }
}

/// Wrapper to send AppHandle across threads (it's Send but we need to be explicit).
struct SendAppHandle(AppHandle);
unsafe impl Send for SendAppHandle {}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RecordingState {
    Idle,
    Recording,
    Paused,
    Encoding,
}

pub struct ScreenRecorder {
    state: Arc<Mutex<RecordingState>>,
    is_paused: Arc<AtomicBool>,
    stop_signal: Option<mpsc::Sender<()>>,
    audio_handle: Option<AudioHandle>,
    last_error: Arc<Mutex<Option<String>>>,
}

impl ScreenRecorder {
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(RecordingState::Idle)),
            is_paused: Arc::new(AtomicBool::new(false)),
            stop_signal: None,
            audio_handle: None,
            last_error: Arc::new(Mutex::new(None)),
        }
    }

    pub fn state(&self) -> RecordingState {
        self.state.lock().unwrap().clone()
    }

    /// Take the last error (clears it after reading).
    pub fn take_error(&self) -> Option<String> {
        self.last_error.lock().unwrap().take()
    }

    /// Start capturing frames (and optionally audio) from the given target.
    pub fn start(
        &mut self,
        target: Target,
        output_path: PathBuf,
        fps: u32,
        record_mic: bool,
        record_system_audio: bool,
        app_handle: AppHandle,
    ) -> Result<(), String> {
        let current_state = self.state();
        if current_state != RecordingState::Idle {
            return Err(format!("Cannot start: recorder is in {:?} state", current_state));
        }

        // Reset pause flag
        self.is_paused.store(false, Ordering::Relaxed);

        // Start audio capture on a dedicated thread if requested
        let wants_audio = record_mic || record_system_audio;
        let mut audio_samples_for_thread = None;
        let mut audio_sample_rate = 0u32;
        let mut audio_channels = 0u16;

        if wants_audio {
            match start_audio_capture(record_mic, record_system_audio, Arc::clone(&self.is_paused)) {
                Ok(handle) => {
                    audio_samples_for_thread = Some(handle.shared_samples());
                    audio_sample_rate = handle.sample_rate();
                    audio_channels = handle.channels();
                    self.audio_handle = Some(handle);
                }
                Err(e) => {
                    log::warn!("Audio capture failed to start, continuing without audio: {}", e);
                }
            }
        }

        let (stop_tx, stop_rx) = mpsc::channel::<()>();
        self.stop_signal = Some(stop_tx);

        let state = Arc::clone(&self.state);
        let is_paused = Arc::clone(&self.is_paused);
        let last_error = Arc::clone(&self.last_error);
        let send_app = SendAppHandle(app_handle);

        // Set state to recording
        *state.lock().unwrap() = RecordingState::Recording;

        let options = Options {
            fps,
            show_cursor: true,
            show_highlight: false,
            target: Some(target),
            output_type: scap::frame::FrameType::BGRAFrame,
            output_resolution: Resolution::_1080p,
            ..Default::default()
        };

        let send_options = SendOptions(options);

        // Spawn capture loop in a background thread
        std::thread::Builder::new()
            .name("tapeit-capture".into())
            .spawn(move || {
                let app_handle = send_app.0;

                // Helper to set error and reset state
                let set_error = |msg: String| {
                    log::error!("{}", msg);
                    *last_error.lock().unwrap() = Some(msg);
                    *state.lock().unwrap() = RecordingState::Idle;
                };

                let mut capturer = match Capturer::build(send_options.into_inner()) {
                    Ok(c) => c,
                    Err(e) => {
                        set_error(format!("Failed to build capturer: {:?}", e));
                        return;
                    }
                };
                capturer.start_capture();

                // Capture the first frame to get dimensions, then start FFmpeg
                let first_frame = match capturer.get_next_frame() {
                    Ok(f) => f,
                    Err(e) => {
                        set_error(format!("Failed to capture first frame: {:?}", e));
                        capturer.stop_capture();
                        return;
                    }
                };

                let (width, height) = match &first_frame {
                    Frame::BGRA(f) => (f.width as u32, f.height as u32),
                    _ => {
                        set_error("Unexpected frame format".into());
                        capturer.stop_capture();
                        return;
                    }
                };

                // Start FFmpeg process for real-time encoding
                let video_output = if audio_samples_for_thread.is_some() {
                    // If audio will be muxed later, write video to a temp file
                    output_path.with_extension("tmp.mp4")
                } else {
                    output_path.clone()
                };

                let mut encoder = match StreamingEncoder::start(&video_output, width, height, fps) {
                    Ok(e) => e,
                    Err(e) => {
                        set_error(format!("Failed to start encoder: {}", e));
                        capturer.stop_capture();
                        return;
                    }
                };

                // Write the first frame
                if let Err(e) = encoder.write_frame(&first_frame) {
                    log::error!("Failed to write first frame: {}", e);
                }
                drop(first_frame); // Free memory immediately

                // Main capture loop — stream frames directly to FFmpeg
                loop {
                    if stop_rx.try_recv().is_ok() {
                        break;
                    }

                    if is_paused.load(Ordering::Relaxed) {
                        std::thread::sleep(std::time::Duration::from_millis(50));
                        continue;
                    }

                    match capturer.get_next_frame() {
                        Ok(frame) => {
                            if let Err(e) = encoder.write_frame(&frame) {
                                log::error!("Frame write error: {}", e);
                                break;
                            }
                            // Frame is dropped here — no memory accumulation
                        }
                        Err(e) => {
                            log::error!("Frame capture error: {:?}", e);
                            break;
                        }
                    }
                }

                capturer.stop_capture();

                // Finalize video encoding (closes FFmpeg stdin, waits for exit)
                *state.lock().unwrap() = RecordingState::Encoding;
                log::info!("Finalizing encoding to {:?}", output_path);

                match encoder.finish() {
                    Ok(_) => log::info!("Video encoded successfully"),
                    Err(e) => {
                        log::error!("Encoding failed: {:?}", e);
                        *state.lock().unwrap() = RecordingState::Idle;
                        return;
                    }
                }

                // Mux audio if we have audio data
                if let Some(audio_buf) = audio_samples_for_thread {
                    std::thread::sleep(std::time::Duration::from_millis(100));
                    let samples = std::mem::take(&mut *audio_buf.lock().unwrap());
                    if !samples.is_empty() {
                        log::info!("Muxing {} audio samples", samples.len());
                        let audio_data = AudioData {
                            samples,
                            sample_rate: audio_sample_rate,
                            channels: audio_channels,
                        };
                        match VideoEncoder::mux_audio(&video_output, &output_path, &audio_data) {
                            Ok(_) => {
                                let _ = std::fs::remove_file(&video_output);
                                log::info!("Recording saved to {:?}", output_path);
                            }
                            Err(e) => {
                                // Fall back to video-only: rename temp to final
                                log::warn!("Audio mux failed ({}), saving video-only", e);
                                let _ = std::fs::rename(&video_output, &output_path);
                            }
                        }
                    } else {
                        // No audio samples collected, rename temp to final
                        let _ = std::fs::rename(&video_output, &output_path);
                        log::info!("Recording saved to {:?} (no audio)", output_path);
                    }
                } else {
                    log::info!("Recording saved to {:?}", output_path);
                }

                // Emit event to frontend with saved file path
                let path_str = output_path.to_string_lossy().to_string();
                let _ = app_handle.emit("recording-saved", path_str);

                *state.lock().unwrap() = RecordingState::Idle;
            })
            .map_err(|e| format!("Failed to spawn capture thread: {}", e))?;

        Ok(())
    }

    pub fn stop(&mut self) -> Result<(), String> {
        // Stop audio capture first (signals audio thread to drop streams)
        if let Some(handle) = self.audio_handle.take() {
            handle.stop();
        }

        if let Some(tx) = self.stop_signal.take() {
            let _ = tx.send(());
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

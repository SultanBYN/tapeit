use std::path::PathBuf;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::capture::{CaptureSource, ScreenRecorder, get_available_sources};

/// Shared recorder state managed by Tauri.
pub struct RecorderState(pub Mutex<ScreenRecorder>);

#[derive(Debug, Serialize, Deserialize)]
pub struct RecordingConfig {
    pub source_id: String,
    pub fps: u32,
    pub output_dir: String,
    pub record_audio: bool,
    pub record_microphone: bool,
}

#[tauri::command]
pub fn get_capture_sources() -> Vec<CaptureSource> {
    get_available_sources()
}

#[tauri::command]
pub fn start_recording(
    config: RecordingConfig,
    state: State<'_, RecorderState>,
) -> Result<String, String> {
    let mut recorder = state.0.lock().map_err(|e| e.to_string())?;

    // Build output file path
    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
    let filename = format!("tapeit_{}.mp4", timestamp);
    let output_path = PathBuf::from(&config.output_dir).join(&filename);

    // Find the matching capture target
    let targets = scap::get_all_targets();
    let target = targets
        .into_iter()
        .find(|t| match t {
            scap::Target::Display(d) => format!("display-{}", d.id) == config.source_id,
            scap::Target::Window(w) => format!("window-{}", w.id) == config.source_id,
        })
        .ok_or("Capture source not found")?;

    recorder.start(
        target,
        output_path.clone(),
        config.fps,
        config.record_microphone,
        config.record_audio,
    )?;

    Ok(output_path.to_string_lossy().to_string())
}

#[tauri::command]
pub fn stop_recording(state: State<'_, RecorderState>) -> Result<(), String> {
    let mut recorder = state.0.lock().map_err(|e| e.to_string())?;
    recorder.stop()
}

#[tauri::command]
pub fn pause_recording(state: State<'_, RecorderState>) -> Result<(), String> {
    let recorder = state.0.lock().map_err(|e| e.to_string())?;
    recorder.pause()
}

#[tauri::command]
pub fn resume_recording(state: State<'_, RecorderState>) -> Result<(), String> {
    let recorder = state.0.lock().map_err(|e| e.to_string())?;
    recorder.resume()
}

#[tauri::command]
pub fn get_recording_state(state: State<'_, RecorderState>) -> String {
    let recorder = state.0.lock().unwrap();
    let state = recorder.state();
    serde_json::to_string(&state).unwrap_or_else(|_| "\"Idle\"".to_string())
}

#[tauri::command]
pub fn get_last_error(state: State<'_, RecorderState>) -> Option<String> {
    let recorder = state.0.lock().unwrap();
    recorder.take_error()
}

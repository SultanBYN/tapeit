// Prevents an additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod audio;
mod capture;
mod commands;
mod encoder;
mod utils;

use capture::ScreenRecorder;
use commands::{
    get_capture_sources, get_last_error, get_recording_state, pause_recording, resume_recording,
    start_recording, stop_recording, RecorderState,
};
use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};

#[tauri::command]
fn show_overlay(app: tauri::AppHandle) -> Result<(), String> {
    // Close existing overlay if any
    if let Some(existing) = app.get_webview_window("overlay") {
        let _ = existing.close();
    }

    let overlay_url = if cfg!(debug_assertions) {
        WebviewUrl::External("http://localhost:1420/overlay.html".parse().unwrap())
    } else {
        WebviewUrl::App("overlay.html".into())
    };

    WebviewWindowBuilder::new(&app, "overlay", overlay_url)
        .title("Tapeit - Recording")
        .inner_size(280.0, 72.0)
        .resizable(false)
        .decorations(false)
        .transparent(true)
        .always_on_top(true)
        .skip_taskbar(true)
        .shadow(false)
        .position(20.0, 20.0)
        .build()
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
fn hide_overlay(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(overlay) = app.get_webview_window("overlay") {
        overlay.close().map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
fn minimize_main(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(main_win) = app.get_webview_window("main") {
        main_win.minimize().map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
fn restore_main(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(main_win) = app.get_webview_window("main") {
        main_win.unminimize().map_err(|e| e.to_string())?;
        main_win.set_focus().map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn main() {
    env_logger::init();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .manage(RecorderState(std::sync::Mutex::new(ScreenRecorder::new())))
        .setup(|_app| {
            log::info!("Tapeit started successfully");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            start_recording,
            stop_recording,
            pause_recording,
            resume_recording,
            get_recording_state,
            get_last_error,
            get_capture_sources,
            show_overlay,
            hide_overlay,
            minimize_main,
            restore_main,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Tapeit");
}

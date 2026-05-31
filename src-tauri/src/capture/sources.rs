use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SourceType {
    Screen,
    Window,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptureSource {
    pub id: String,
    pub name: String,
    pub source_type: SourceType,
    pub width: u32,
    pub height: u32,
}

/// Returns all available capture sources (screens + windows) using the `scap` crate.
///
/// Uses platform-specific APIs under the hood:
/// - Windows: Windows Graphics Capture API
/// - macOS: ScreenCaptureKit
/// - Linux: PipeWire / X11
pub fn get_available_sources() -> Vec<CaptureSource> {
    let mut sources = Vec::new();

    // Check if we have permission to capture
    if !scap::has_permission() {
        log::warn!("Screen capture permission not granted — requesting...");
        scap::request_permission();
        return sources;
    }

    // Get available capture targets
    let targets = scap::get_all_targets();

    for target in targets {
        match target {
            scap::Target::Display(display) => {
                sources.push(CaptureSource {
                    id: format!("display-{}", display.id),
                    name: format!("Display {}", display.id),
                    source_type: SourceType::Screen,
                    width: display.width,
                    height: display.height,
                });
            }
            scap::Target::Window(window) => {
                sources.push(CaptureSource {
                    id: format!("window-{}", window.id),
                    name: window.title,
                    source_type: SourceType::Window,
                    width: window.width,
                    height: window.height,
                });
            }
        }
    }

    sources
}

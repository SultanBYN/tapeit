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
}

/// Returns all available capture sources (screens + windows) using the `scap` crate.
pub fn get_available_sources() -> Vec<CaptureSource> {
    let mut sources = Vec::new();

    // Check if we have permission to capture
    if !scap::has_permission() {
        log::warn!("Screen capture permission not granted — requesting...");
        scap::request_permission();
        return sources;
    }

    let targets = scap::get_all_targets();

    for target in targets {
        match target {
            scap::Target::Display(display) => {
                sources.push(CaptureSource {
                    id: format!("display-{}", display.id),
                    name: display.title,
                    source_type: SourceType::Screen,
                });
            }
            scap::Target::Window(window) => {
                sources.push(CaptureSource {
                    id: format!("window-{}", window.id),
                    name: window.title,
                    source_type: SourceType::Window,
                });
            }
        }
    }

    sources
}

mod recorder;
mod sources;

pub use recorder::ScreenRecorder;
pub use sources::{CaptureSource, SourceType, get_available_sources};

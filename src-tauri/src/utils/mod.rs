use std::path::PathBuf;

/// Returns the default output directory for recordings.
/// Creates it if it doesn't exist.
pub fn default_output_dir() -> PathBuf {
    let dir = dirs_path().join("Tapeit").join("recordings");
    std::fs::create_dir_all(&dir).ok();
    dir
}

fn dirs_path() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        std::env::var("USERPROFILE")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("C:\\Users\\Default"))
            .join("Videos")
    }

    #[cfg(target_os = "macos")]
    {
        std::env::var("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("/tmp"))
            .join("Movies")
    }

    #[cfg(target_os = "linux")]
    {
        std::env::var("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("/tmp"))
            .join("Videos")
    }
}

use scap::frame::Frame;
use std::path::Path;

/// Encodes captured frames into an MP4 file using FFmpeg.
///
/// Supports hardware-accelerated encoding when available:
/// - NVIDIA: NVENC (h264_nvenc)
/// - Intel: QuickSync (h264_qsv)
/// - AMD: AMF (h264_amf)
/// - Fallback: libx264 (software)
pub struct VideoEncoder;

impl VideoEncoder {
    /// Encode a sequence of captured frames to an MP4 file.
    ///
    /// This is a placeholder implementation. The full version will use
    /// `ffmpeg-next` bindings for hardware-accelerated encoding.
    pub fn encode_frames(frames: &[Frame], output_path: &Path, fps: u32) -> Result<(), String> {
        if frames.is_empty() {
            return Err("No frames to encode".into());
        }

        log::info!(
            "Encoding {} frames at {} fps to {:?}",
            frames.len(),
            fps,
            output_path
        );

        // TODO: Phase 1 implementation
        // For MVP, we'll use ffmpeg CLI as a subprocess.
        // Phase 2 will switch to ffmpeg-next bindings for zero-copy encoding.
        //
        // The encoding pipeline:
        // 1. Take raw BGRA frames from scap
        // 2. Convert to YUV420P (required by H.264)
        // 3. Encode using hardware encoder (with software fallback)
        // 4. Mux into MP4 container
        //
        // Hardware encoder detection order:
        //   Windows: h264_nvenc -> h264_qsv -> h264_amf -> libx264
        //   macOS:   h264_videotoolbox -> libx264
        //   Linux:   h264_nvenc -> h264_vaapi -> libx264

        Self::encode_with_ffmpeg_cli(frames, output_path, fps)
    }

    /// MVP encoding: pipe raw frames to ffmpeg CLI.
    fn encode_with_ffmpeg_cli(frames: &[Frame], output_path: &Path, fps: u32) -> Result<(), String> {
        use std::io::Write;
        use std::process::{Command, Stdio};

        // Get dimensions from first frame
        let (width, height) = Self::get_frame_dimensions(&frames[0])?;

        let mut child = Command::new("ffmpeg")
            .args([
                "-y",
                "-f", "rawvideo",
                "-pixel_format", "bgra",
                "-video_size", &format!("{}x{}", width, height),
                "-framerate", &fps.to_string(),
                "-i", "pipe:0",
                "-c:v", "libx264",
                "-preset", "ultrafast",
                "-crf", "23",
                "-pix_fmt", "yuv420p",
                output_path.to_str().unwrap_or("output.mp4"),
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("Failed to spawn ffmpeg: {}. Is ffmpeg installed?", e))?;

        let stdin = child.stdin.as_mut().ok_or("Failed to open ffmpeg stdin")?;

        for frame in frames {
            if let Some(data) = Self::get_frame_data(frame) {
                stdin.write_all(data).map_err(|e| format!("Write error: {}", e))?;
            }
        }

        drop(child.stdin.take());

        let output = child.wait_with_output().map_err(|e| format!("ffmpeg error: {}", e))?;

        if output.status.success() {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(format!("ffmpeg failed: {}", stderr))
        }
    }

    fn get_frame_dimensions(frame: &Frame) -> Result<(u32, u32), String> {
        match frame {
            Frame::BGRA(bgra_frame) => Ok((bgra_frame.width as u32, bgra_frame.height as u32)),
            _ => Err("Unexpected frame format (expected BGRA)".into()),
        }
    }

    fn get_frame_data(frame: &Frame) -> Option<&[u8]> {
        match frame {
            Frame::BGRA(bgra_frame) => Some(&bgra_frame.data),
            _ => None,
        }
    }
}

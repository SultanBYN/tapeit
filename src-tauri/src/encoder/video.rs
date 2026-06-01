use scap::frame::Frame;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

use crate::audio::AudioData;

/// A streaming encoder that pipes frames to FFmpeg in real-time.
pub struct StreamingEncoder {
    child: Child,
    output_path: PathBuf,
}

impl StreamingEncoder {
    /// Start an FFmpeg process ready to receive raw BGRA frames on stdin.
    pub fn start(output_path: &Path, width: u32, height: u32, fps: u32) -> Result<Self, String> {
        // Ensure output directory exists
        if let Some(parent) = output_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create output dir: {}", e))?;
        }

        let child = Command::new("ffmpeg")
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

        Ok(Self {
            child,
            output_path: output_path.to_path_buf(),
        })
    }

    /// Write a single frame to the FFmpeg process.
    pub fn write_frame(&mut self, frame: &Frame) -> Result<(), String> {
        let data = match frame {
            Frame::BGRA(f) => &f.data,
            _ => return Err("Unexpected frame format (expected BGRA)".into()),
        };
        if let Some(stdin) = self.child.stdin.as_mut() {
            stdin.write_all(data).map_err(|e| format!("FFmpeg write error: {}", e))?;
        }
        Ok(())
    }

    /// Close stdin and wait for FFmpeg to finish. Returns the output path on success.
    pub fn finish(mut self) -> Result<PathBuf, String> {
        // Close stdin to signal end of input
        drop(self.child.stdin.take());

        let output = self.child
            .wait_with_output()
            .map_err(|e| format!("FFmpeg error: {}", e))?;

        if output.status.success() {
            Ok(self.output_path)
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(format!("FFmpeg failed: {}", stderr))
        }
    }
}

/// Encodes captured frames (and optional audio) into an MP4 file using FFmpeg CLI.
pub struct VideoEncoder;

impl VideoEncoder {
    /// Mux a video file with audio data into a final MP4.
    pub fn mux_audio(
        video_path: &Path,
        output_path: &Path,
        audio: &AudioData,
    ) -> Result<(), String> {
        // Write audio to temp WAV
        let wav_path = output_path.with_extension("tmp.wav");
        Self::write_wav(&wav_path, &audio.samples, audio.sample_rate, audio.channels)?;

        let output = Command::new("ffmpeg")
            .args([
                "-y",
                "-i", video_path.to_str().unwrap_or(""),
                "-i", wav_path.to_str().unwrap_or(""),
                "-c:v", "copy",
                "-c:a", "aac",
                "-b:a", "128k",
                "-shortest",
                output_path.to_str().unwrap_or("output.mp4"),
            ])
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .output()
            .map_err(|e| format!("Failed to run ffmpeg mux: {}", e))?;

        let _ = std::fs::remove_file(&wav_path);

        if output.status.success() {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(format!("FFmpeg mux failed: {}", stderr))
        }
    }

    /// Encode a sequence of captured frames (with optional audio) to an MP4 file.
    pub fn encode_frames(
        frames: &[Frame],
        output_path: &Path,
        fps: u32,
        audio: Option<&AudioData>,
    ) -> Result<(), String> {
        if frames.is_empty() {
            return Err("No frames to encode".into());
        }

        log::info!(
            "Encoding {} frames at {} fps to {:?} (audio: {})",
            frames.len(),
            fps,
            output_path,
            audio.map_or(false, |a| !a.is_empty())
        );

        Self::encode_with_ffmpeg_cli(frames, output_path, fps, audio)
    }

    /// MVP encoding: pipe raw frames to ffmpeg CLI, with optional audio from a temp WAV file.
    fn encode_with_ffmpeg_cli(
        frames: &[Frame],
        output_path: &Path,
        fps: u32,
        audio: Option<&AudioData>,
    ) -> Result<(), String> {
        use std::io::Write;
        use std::process::{Command, Stdio};

        let (width, height) = Self::get_frame_dimensions(&frames[0])?;

        // Write audio to a temp WAV file if we have audio data
        let audio_wav_path = if let Some(audio) = audio {
            if !audio.is_empty() {
                let wav_path = output_path.with_extension("tmp.wav");
                Self::write_wav(&wav_path, &audio.samples, audio.sample_rate, audio.channels)?;
                Some(wav_path)
            } else {
                None
            }
        } else {
            None
        };

        // Build ffmpeg arguments
        let mut args: Vec<String> = vec![
            "-y".into(),
            "-f".into(), "rawvideo".into(),
            "-pixel_format".into(), "bgra".into(),
            "-video_size".into(), format!("{}x{}", width, height),
            "-framerate".into(), fps.to_string(),
            "-i".into(), "pipe:0".into(),
        ];

        // Add audio input if available
        if let Some(ref wav_path) = audio_wav_path {
            args.extend([
                "-i".into(),
                wav_path.to_string_lossy().to_string(),
            ]);
        }

        // Video codec
        args.extend([
            "-c:v".into(), "libx264".into(),
            "-preset".into(), "ultrafast".into(),
            "-crf".into(), "23".into(),
            "-pix_fmt".into(), "yuv420p".into(),
        ]);

        // Audio codec (if audio input present)
        if audio_wav_path.is_some() {
            args.extend([
                "-c:a".into(), "aac".into(),
                "-b:a".into(), "128k".into(),
                // Use shortest stream to avoid audio/video length mismatch
                "-shortest".into(),
            ]);
        }

        args.push(output_path.to_str().unwrap_or("output.mp4").into());

        let args_ref: Vec<&str> = args.iter().map(|s| s.as_str()).collect();

        let mut child = Command::new("ffmpeg")
            .args(&args_ref)
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

        // Clean up temp WAV file
        if let Some(ref wav_path) = audio_wav_path {
            let _ = std::fs::remove_file(wav_path);
        }

        if output.status.success() {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(format!("ffmpeg failed: {}", stderr))
        }
    }

    /// Write f32 audio samples to a WAV file.
    fn write_wav(
        path: &Path,
        samples: &[f32],
        sample_rate: u32,
        channels: u16,
    ) -> Result<(), String> {
        use std::io::Write;

        let bits_per_sample: u16 = 16;
        let byte_rate = sample_rate * channels as u32 * (bits_per_sample as u32 / 8);
        let block_align = channels * (bits_per_sample / 8);
        let data_size = samples.len() as u32 * (bits_per_sample as u32 / 8);
        let file_size = 36 + data_size;

        let mut file = std::fs::File::create(path)
            .map_err(|e| format!("Failed to create WAV file: {}", e))?;

        // RIFF header
        file.write_all(b"RIFF").map_err(|e| e.to_string())?;
        file.write_all(&file_size.to_le_bytes()).map_err(|e| e.to_string())?;
        file.write_all(b"WAVE").map_err(|e| e.to_string())?;

        // fmt chunk
        file.write_all(b"fmt ").map_err(|e| e.to_string())?;
        file.write_all(&16u32.to_le_bytes()).map_err(|e| e.to_string())?; // chunk size
        file.write_all(&1u16.to_le_bytes()).map_err(|e| e.to_string())?;  // PCM format
        file.write_all(&channels.to_le_bytes()).map_err(|e| e.to_string())?;
        file.write_all(&sample_rate.to_le_bytes()).map_err(|e| e.to_string())?;
        file.write_all(&byte_rate.to_le_bytes()).map_err(|e| e.to_string())?;
        file.write_all(&block_align.to_le_bytes()).map_err(|e| e.to_string())?;
        file.write_all(&bits_per_sample.to_le_bytes()).map_err(|e| e.to_string())?;

        // data chunk
        file.write_all(b"data").map_err(|e| e.to_string())?;
        file.write_all(&data_size.to_le_bytes()).map_err(|e| e.to_string())?;

        // Convert f32 samples to i16 and write
        for &sample in samples {
            let clamped = sample.clamp(-1.0, 1.0);
            let i16_sample = (clamped * i16::MAX as f32) as i16;
            file.write_all(&i16_sample.to_le_bytes()).map_err(|e| e.to_string())?;
        }

        log::info!("Wrote temp WAV: {} samples, {}Hz, {}ch", samples.len(), sample_rate, channels);
        Ok(())
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

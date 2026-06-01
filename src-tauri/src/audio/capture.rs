use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, Stream, StreamConfig};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

/// Collected audio data ready for encoding.
pub struct AudioData {
    pub samples: Vec<f32>,
    pub sample_rate: u32,
    pub channels: u16,
}

impl AudioData {
    pub fn is_empty(&self) -> bool {
        self.samples.is_empty()
    }
}

/// Handle to a running audio capture session.
/// Audio streams run on a dedicated thread (since cpal Stream is !Send).
/// Samples accumulate in a shared buffer accessible via `shared_samples()`.
pub struct AudioHandle {
    samples: Arc<Mutex<Vec<f32>>>,
    sample_rate: u32,
    channels: u16,
    /// Signals the audio thread to stop
    stop_flag: Arc<AtomicBool>,
}

impl AudioHandle {
    /// Returns a clone of the shared samples buffer for the capture thread.
    pub fn shared_samples(&self) -> Arc<Mutex<Vec<f32>>> {
        Arc::clone(&self.samples)
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    pub fn channels(&self) -> u16 {
        self.channels
    }

    /// Signal the audio thread to stop. Streams will be dropped on their thread.
    pub fn stop(&self) {
        self.stop_flag.store(true, Ordering::Relaxed);
    }
}

// AudioHandle only contains Arc types and atomics — all Send+Sync
// (the actual Stream lives on the audio thread, not here)

/// Start audio capture on a dedicated thread.
/// Returns an AudioHandle that can be stored in Send+Sync contexts.
pub fn start_audio_capture(
    record_mic: bool,
    record_system: bool,
    is_paused: Arc<AtomicBool>,
) -> Result<AudioHandle, String> {
    let samples: Arc<Mutex<Vec<f32>>> = Arc::new(Mutex::new(Vec::new()));
    let stop_flag = Arc::new(AtomicBool::new(false));

    // We need to probe the audio config before spawning the thread
    // so we can return it in the handle. Probe on the current thread.
    let host = cpal::default_host();
    let (sample_rate, channels) = probe_audio_config(&host, record_system)?;

    let samples_clone = Arc::clone(&samples);
    let stop_clone = Arc::clone(&stop_flag);
    let is_paused_clone = Arc::clone(&is_paused);

    std::thread::Builder::new()
        .name("tapeit-audio".into())
        .spawn(move || {
            let host = cpal::default_host();
            let mut streams: Vec<Stream> = Vec::new();

            if record_system {
                match build_system_audio_stream(&host, &samples_clone, &is_paused_clone) {
                    Ok(stream) => {
                        streams.push(stream);
                        log::info!("System audio capture started");
                    }
                    Err(e) => log::warn!("System audio failed: {}", e),
                }
            }

            if record_mic {
                match build_mic_stream(&host, &samples_clone, &is_paused_clone) {
                    Ok(stream) => {
                        streams.push(stream);
                        log::info!("Microphone capture started");
                    }
                    Err(e) => log::warn!("Microphone failed: {}", e),
                }
            }

            // Wait for stop signal
            while !stop_clone.load(Ordering::Relaxed) {
                std::thread::sleep(std::time::Duration::from_millis(50));
            }

            // Streams dropped here, stopping capture
            drop(streams);
            log::info!("Audio thread stopped");
        })
        .map_err(|e| format!("Failed to spawn audio thread: {}", e))?;

    Ok(AudioHandle {
        samples,
        sample_rate,
        channels,
        stop_flag,
    })
}

/// Probe audio config to get sample rate and channels without creating streams.
fn probe_audio_config(host: &cpal::Host, use_output: bool) -> Result<(u32, u16), String> {
    #[cfg(target_os = "windows")]
    if use_output {
        let device = host
            .default_output_device()
            .ok_or("No output device for loopback")?;
        let config = device
            .default_output_config()
            .map_err(|e| format!("Failed to get output config: {}", e))?;
        let sc: StreamConfig = config.into();
        return Ok((sc.sample_rate.0, sc.channels));
    }

    let device = host
        .default_input_device()
        .ok_or("No input device available")?;
    let config = device
        .default_input_config()
        .map_err(|e| format!("Failed to get input config: {}", e))?;
    let sc: StreamConfig = config.into();
    Ok((sc.sample_rate.0, sc.channels))
}

fn build_mic_stream(
    host: &cpal::Host,
    samples: &Arc<Mutex<Vec<f32>>>,
    is_paused: &Arc<AtomicBool>,
) -> Result<Stream, String> {
    let device = host
        .default_input_device()
        .ok_or("No input device available")?;
    let supported = device
        .default_input_config()
        .map_err(|e| format!("Failed to get mic config: {}", e))?;
    let config: StreamConfig = supported.clone().into();
    build_stream(&device, &config, supported.sample_format(), samples, is_paused)
}

fn build_system_audio_stream(
    host: &cpal::Host,
    samples: &Arc<Mutex<Vec<f32>>>,
    is_paused: &Arc<AtomicBool>,
) -> Result<Stream, String> {
    #[cfg(target_os = "windows")]
    {
        // WASAPI loopback: use output device with its output config
        let device = host
            .default_output_device()
            .ok_or("No output device for loopback")?;
        let supported = device
            .default_output_config()
            .map_err(|e| format!("Failed to get loopback config: {}", e))?;
        let config: StreamConfig = supported.clone().into();
        return build_stream(&device, &config, supported.sample_format(), samples, is_paused);
    }

    #[cfg(not(target_os = "windows"))]
    {
        // On macOS/Linux, fall back to default input (user must configure virtual audio routing)
        build_mic_stream(host, samples, is_paused)
    }
}

fn build_stream(
    device: &cpal::Device,
    config: &StreamConfig,
    sample_format: SampleFormat,
    samples: &Arc<Mutex<Vec<f32>>>,
    is_paused: &Arc<AtomicBool>,
) -> Result<Stream, String> {
    let samples = Arc::clone(samples);
    let is_paused = Arc::clone(is_paused);

    let stream = match sample_format {
        SampleFormat::F32 => device
            .build_input_stream(
                config,
                move |data: &[f32], _| {
                    if !is_paused.load(Ordering::Relaxed) {
                        samples.lock().unwrap().extend_from_slice(data);
                    }
                },
                |err| log::error!("Audio stream error: {}", err),
                None,
            )
            .map_err(|e| format!("Failed to build stream: {}", e))?,
        SampleFormat::I16 => {
            let samples = Arc::clone(&samples);
            let is_paused = Arc::clone(&is_paused);
            device
                .build_input_stream(
                    config,
                    move |data: &[i16], _| {
                        if !is_paused.load(Ordering::Relaxed) {
                            let float_samples: Vec<f32> =
                                data.iter().map(|&s| s as f32 / i16::MAX as f32).collect();
                            samples.lock().unwrap().extend_from_slice(&float_samples);
                        }
                    },
                    |err| log::error!("Audio stream error: {}", err),
                    None,
                )
                .map_err(|e| format!("Failed to build stream: {}", e))?
        }
        _ => return Err(format!("Unsupported sample format: {:?}", sample_format)),
    };

    stream
        .play()
        .map_err(|e| format!("Failed to start audio stream: {}", e))?;

    Ok(stream)
}

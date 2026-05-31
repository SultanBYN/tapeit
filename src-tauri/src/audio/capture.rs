use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, SampleFormat, Stream, StreamConfig};
use std::sync::{Arc, Mutex};

/// Captures audio from microphone and/or system audio (loopback).
pub struct AudioCapturer {
    samples: Arc<Mutex<Vec<f32>>>,
    stream: Option<Stream>,
    config: Option<StreamConfig>,
}

impl AudioCapturer {
    pub fn new() -> Self {
        Self {
            samples: Arc::new(Mutex::new(Vec::new())),
            stream: None,
            config: None,
        }
    }

    /// Lists available audio input devices.
    pub fn list_devices() -> Vec<String> {
        let host = cpal::default_host();
        let mut devices = Vec::new();

        if let Ok(input_devices) = host.input_devices() {
            for device in input_devices {
                if let Ok(name) = device.name() {
                    devices.push(name);
                }
            }
        }

        devices
    }

    /// Start capturing audio from the default input device (microphone).
    pub fn start_microphone(&mut self) -> Result<(), String> {
        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .ok_or("No input device available")?;

        self.start_device(&device)
    }

    /// Start capturing system audio (loopback).
    /// On Windows, this uses WASAPI loopback.
    /// On macOS, this requires a virtual audio device (e.g., BlackHole).
    /// On Linux, this uses PulseAudio/PipeWire monitor.
    pub fn start_system_audio(&mut self) -> Result<(), String> {
        let host = cpal::default_host();

        // On Windows, try to get the default output device for loopback
        #[cfg(target_os = "windows")]
        {
            let device = host
                .default_output_device()
                .ok_or("No output device available for loopback")?;
            return self.start_device(&device);
        }

        // On other platforms, use default input (user needs virtual audio routing)
        #[cfg(not(target_os = "windows"))]
        {
            let device = host
                .default_input_device()
                .ok_or("No input device available")?;
            self.start_device(&device)
        }
    }

    fn start_device(&mut self, device: &Device) -> Result<(), String> {
        let supported_config = device
            .default_input_config()
            .map_err(|e| format!("Failed to get default config: {}", e))?;

        let config: StreamConfig = supported_config.clone().into();
        let sample_format = supported_config.sample_format();

        let samples = Arc::clone(&self.samples);

        let stream = match sample_format {
            SampleFormat::F32 => device
                .build_input_stream(
                    &config,
                    move |data: &[f32], _| {
                        samples.lock().unwrap().extend_from_slice(data);
                    },
                    |err| log::error!("Audio stream error: {}", err),
                    None,
                )
                .map_err(|e| format!("Failed to build stream: {}", e))?,
            SampleFormat::I16 => {
                let samples = Arc::clone(&self.samples);
                device
                    .build_input_stream(
                        &config,
                        move |data: &[i16], _| {
                            let float_samples: Vec<f32> =
                                data.iter().map(|&s| s as f32 / i16::MAX as f32).collect();
                            samples.lock().unwrap().extend_from_slice(&float_samples);
                        },
                        |err| log::error!("Audio stream error: {}", err),
                        None,
                    )
                    .map_err(|e| format!("Failed to build stream: {}", e))?
            }
            _ => return Err(format!("Unsupported sample format: {:?}", sample_format)),
        };

        stream.play().map_err(|e| format!("Failed to play stream: {}", e))?;

        self.config = Some(config);
        self.stream = Some(stream);

        log::info!("Audio capture started");
        Ok(())
    }

    /// Stop capturing and return the collected samples.
    pub fn stop(&mut self) -> (Vec<f32>, Option<StreamConfig>) {
        self.stream = None; // Dropping the stream stops it
        let samples = std::mem::take(&mut *self.samples.lock().unwrap());
        let config = self.config.take();
        log::info!("Audio capture stopped, {} samples collected", samples.len());
        (samples, config)
    }
}

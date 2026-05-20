use anyhow::{anyhow, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

const TARGET_SAMPLE_RATE: u32 = 16_000; // whisper.cpp expects 16 kHz

pub fn list_input_devices() -> Vec<String> {
    let host = cpal::default_host();
    host.input_devices()
        .map(|iter| {
            iter.filter_map(|d| d.name().ok())
                .collect()
        })
        .unwrap_or_default()
}

/// Start recording from the default (or named) input device.
/// Returns (stop_flag, wav_path). The recording runs on a background thread
/// until stop_flag is set to true.
pub fn start_recording(
    device_name: &str,
    wav_path: PathBuf,
) -> Result<Arc<AtomicBool>> {
    let host = cpal::default_host();

    let device = if device_name.is_empty() || device_name == "default" {
        host.default_input_device()
            .ok_or_else(|| anyhow!("No default audio input device found"))?
    } else {
        host.input_devices()?
            .find(|d| d.name().map(|n| n == device_name).unwrap_or(false))
            .ok_or_else(|| anyhow!("Audio device '{}' not found", device_name))?
    };

    let config = find_config(&device)?;
    let channels = config.channels() as usize;
    let sample_rate = config.sample_rate().0;

    tracing::info!(
        "Recording: device='{}' rate={}Hz ch={}",
        device.name().unwrap_or_default(),
        sample_rate,
        channels
    );

    let stop_flag = Arc::new(AtomicBool::new(false));
    let stop_clone = Arc::clone(&stop_flag);

    // Shared sample buffer
    let samples: Arc<Mutex<Vec<f32>>> = Arc::new(Mutex::new(Vec::new()));
    let samples_clone = Arc::clone(&samples);

    std::thread::spawn(move || {
        let stream = device
            .build_input_stream(
                &config.into(),
                move |data: &[f32], _| {
                    // Downmix to mono by averaging channels
                    let mut buf = samples_clone.lock().unwrap();
                    for chunk in data.chunks(channels) {
                        let mono = chunk.iter().sum::<f32>() / channels as f32;
                        buf.push(mono);
                    }
                },
                |err| tracing::error!("Audio stream error: {}", err),
                None,
            )
            .expect("Failed to build input stream");

        stream.play().expect("Failed to start audio stream");

        // Spin until stop flag is set
        while !stop_clone.load(Ordering::Relaxed) {
            std::thread::sleep(Duration::from_millis(50));
        }

        drop(stream); // Stops recording

        // Write captured samples to WAV file
        let buf = samples.lock().unwrap();
        if let Err(e) = write_wav(&wav_path, &buf, sample_rate) {
            tracing::error!("Failed to write WAV: {}", e);
        }
    });

    Ok(stop_flag)
}

fn find_config(device: &cpal::Device) -> Result<cpal::SupportedStreamConfig> {
    // Prefer 16 kHz mono f32 (ideal for whisper)
    let supported = device.supported_input_configs()?;
    for cfg in supported {
        if cfg.channels() == 1
            && cfg.sample_format() == cpal::SampleFormat::F32
            && cfg.min_sample_rate().0 <= TARGET_SAMPLE_RATE
            && cfg.max_sample_rate().0 >= TARGET_SAMPLE_RATE
        {
            return Ok(cfg.with_sample_rate(cpal::SampleRate(TARGET_SAMPLE_RATE)));
        }
    }
    // Fall back to default config
    device
        .default_input_config()
        .map_err(|e| anyhow!("No suitable input config: {}", e))
}

fn write_wav(path: &PathBuf, samples: &[f32], sample_rate: u32) -> Result<()> {
    // Resample to 16 kHz if needed (linear interpolation)
    let resampled = if sample_rate != TARGET_SAMPLE_RATE {
        resample(samples, sample_rate, TARGET_SAMPLE_RATE)
    } else {
        samples.to_vec()
    };

    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: TARGET_SAMPLE_RATE,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    let mut writer = hound::WavWriter::create(path, spec)?;
    for &s in &resampled {
        let clamped = s.clamp(-1.0, 1.0);
        writer.write_sample((clamped * i16::MAX as f32) as i16)?;
    }
    writer.finalize()?;
    Ok(())
}

fn resample(samples: &[f32], from_rate: u32, to_rate: u32) -> Vec<f32> {
    if samples.is_empty() {
        return vec![];
    }
    let ratio = from_rate as f64 / to_rate as f64;
    let out_len = (samples.len() as f64 / ratio) as usize;
    let mut out = Vec::with_capacity(out_len);
    for i in 0..out_len {
        let src_pos = i as f64 * ratio;
        let idx = src_pos as usize;
        let frac = (src_pos - idx as f64) as f32;
        let a = samples.get(idx).copied().unwrap_or(0.0);
        let b = samples.get(idx + 1).copied().unwrap_or(a);
        out.push(a + frac * (b - a));
    }
    out
}

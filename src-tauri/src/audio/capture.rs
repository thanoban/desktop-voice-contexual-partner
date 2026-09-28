use anyhow::{anyhow, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::oneshot;

const TARGET_SAMPLE_RATE: u32 = 16_000; // whisper.cpp expects 16 kHz
const MAX_RECORDING_SECONDS: usize = 5 * 60;

pub struct RecordingHandle {
    pub stop_flag: Arc<AtomicBool>,
    pub completed: oneshot::Receiver<Result<(), String>>,
}

pub fn list_input_devices() -> Vec<String> {
    let host = cpal::default_host();
    host.input_devices()
        .map(|iter| iter.filter_map(|d| d.name().ok()).collect())
        .unwrap_or_default()
}

/// Start recording from the default (or named) input device.
/// Returns a handle that can stop recording and await final WAV completion.
pub fn start_recording(device_name: &str, wav_path: PathBuf) -> Result<RecordingHandle> {
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
    let (completed_tx, completed_rx) = oneshot::channel();

    // Shared sample buffer
    let samples: Arc<Mutex<Vec<f32>>> = Arc::new(Mutex::new(Vec::new()));
    let samples_clone = Arc::clone(&samples);

    let max_samples = sample_rate as usize * MAX_RECORDING_SECONDS;
    let limit_stop = Arc::clone(&stop_flag);

    std::thread::spawn(move || {
        let result = (|| -> Result<()> {
            let stream_config: cpal::StreamConfig = config.clone().into();
            let stream = match config.sample_format() {
                cpal::SampleFormat::F32 => build_stream::<f32>(
                    &device,
                    &stream_config,
                    channels,
                    Arc::clone(&samples_clone),
                    max_samples,
                    Arc::clone(&limit_stop),
                )?,
                cpal::SampleFormat::I16 => build_stream::<i16>(
                    &device,
                    &stream_config,
                    channels,
                    Arc::clone(&samples_clone),
                    max_samples,
                    Arc::clone(&limit_stop),
                )?,
                cpal::SampleFormat::U16 => build_stream::<u16>(
                    &device,
                    &stream_config,
                    channels,
                    Arc::clone(&samples_clone),
                    max_samples,
                    Arc::clone(&limit_stop),
                )?,
                format => return Err(anyhow!("Unsupported input sample format: {format:?}")),
            };

            stream.play()?;
            while !stop_clone.load(Ordering::Acquire) {
                std::thread::sleep(Duration::from_millis(20));
            }
            drop(stream);

            let buf = samples
                .lock()
                .map_err(|_| anyhow!("Audio buffer lock failed"))?;
            write_wav(&wav_path, &buf, sample_rate)
        })();

        if let Err(error) = &result {
            tracing::error!("Recording failed: {}", error);
        }
        let _ = completed_tx.send(result.map_err(|error| error.to_string()));
    });

    Ok(RecordingHandle {
        stop_flag,
        completed: completed_rx,
    })
}

fn build_stream<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    channels: usize,
    samples: Arc<Mutex<Vec<f32>>>,
    max_samples: usize,
    stop_flag: Arc<AtomicBool>,
) -> Result<cpal::Stream>
where
    T: cpal::Sample + cpal::SizedSample,
    f32: cpal::FromSample<T>,
{
    device
        .build_input_stream(
            config,
            move |data: &[T], _| {
                let Ok(mut buffer) = samples.lock() else {
                    stop_flag.store(true, Ordering::Release);
                    return;
                };
                for frame in data.chunks(channels) {
                    if buffer.len() >= max_samples {
                        stop_flag.store(true, Ordering::Release);
                        break;
                    }
                    let mono = frame
                        .iter()
                        .map(|sample| sample.to_sample::<f32>())
                        .sum::<f32>()
                        / channels as f32;
                    buffer.push(mono);
                }
            },
            |error| tracing::error!("Audio stream error: {}", error),
            None,
        )
        .map_err(Into::into)
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

fn write_wav(path: &Path, samples: &[f32], sample_rate: u32) -> Result<()> {
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

#[cfg(test)]
mod tests {
    use super::resample;

    #[test]
    fn resample_empty_input_is_empty() {
        assert!(resample(&[], 48_000, 16_000).is_empty());
    }

    #[test]
    fn resample_preserves_expected_duration() {
        let input = vec![0.25; 48_000];
        let output = resample(&input, 48_000, 16_000);
        assert_eq!(output.len(), 16_000);
        assert!(output
            .iter()
            .all(|sample| (*sample - 0.25).abs() < f32::EPSILON));
    }
}

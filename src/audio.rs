use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

/// Maximum simultaneously sounding key presses. Excess presses steal the oldest voice.
pub const MAX_VOICES: usize = 16;

/// Output buffer size in frames. 256 @ 44.1 kHz ≈ 5.8 ms of latency.
pub const BUFFER_FRAMES: u32 = 256;

/// Messages sent from any thread into the audio callback.
#[derive(Debug, Clone)]
pub enum AudioCommand {
    /// Start one new voice playing the current pack from position 0.
    Play,
    /// Replace the sample buffer used by future voices. Already-playing voices keep their old buffer.
    SetPack(Arc<Vec<f32>>),
    /// Set master gain. Value is clamped to 0.0..=1.0 by the receiver.
    SetVolume(f32),
}

/// Errors that can occur while building or running the audio engine.
#[derive(Debug)]
pub enum AudioError {
    /// cpal reported no default output device.
    NoOutputDevice,
    /// The device's default output config could not be queried.
    NoOutputConfig(String),
    /// Building or starting the stream failed.
    StreamFailed(String),
}

impl std::fmt::Display for AudioError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AudioError::NoOutputDevice => write!(f, "no default audio output device"),
            AudioError::NoOutputConfig(s) => write!(f, "could not query output config: {s}"),
            AudioError::StreamFailed(s) => write!(f, "failed to build/start audio stream: {s}"),
        }
    }
}

impl std::error::Error for AudioError {}

/// Owns the cpal output stream and the command channel into it.
pub struct AudioEngine {
    tx: crossbeam_channel::Sender<AudioCommand>,
    // Never read directly; held only so the stream keeps playing until AudioEngine drops.
    #[allow(dead_code)]
    stream: cpal::Stream,
    sample_rate: u32,
    /// Set to true by the stream error callback when the device disappears.
    device_lost: Arc<AtomicBool>,
}

impl AudioEngine {
    /// Build and start the output stream on the system default device.
    pub fn new() -> Result<Self, AudioError> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or(AudioError::NoOutputDevice)?;
        let default_config = device
            .default_output_config()
            .map_err(|e| AudioError::NoOutputConfig(e.to_string()))?;

        let sample_rate = default_config.sample_rate().0;
        let channels = default_config.channels() as usize;

        let mut stream_config: cpal::StreamConfig = default_config.into();
        stream_config.buffer_size = cpal::BufferSize::Fixed(BUFFER_FRAMES);

        let (tx, rx) = crossbeam_channel::bounded::<AudioCommand>(128);
        let device_lost = Arc::new(AtomicBool::new(false));
        let device_lost_cb = device_lost.clone();

        let mut mixer = Mixer {
            voices: std::array::from_fn(|_| None),
            next_slot: 0,
            current: Arc::new(Vec::new()),
            volume: 1.0,
            rx,
        };

        let data_callback = move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
            for _ in 0..64 {
                match mixer.rx.try_recv() {
                    Ok(AudioCommand::SetPack(s)) => mixer.current = s,
                    Ok(AudioCommand::SetVolume(v)) => mixer.volume = v.clamp(0.0, 1.0),
                    Ok(AudioCommand::Play) => {
                        if !mixer.current.is_empty() {
                            let slot = mixer.voices.iter().position(|v| v.is_none());
                            let new_voice = Voice {
                                samples: mixer.current.clone(),
                                pos: 0,
                            };
                            match slot {
                                Some(i) => mixer.voices[i] = Some(new_voice),
                                None => {
                                    mixer.voices[mixer.next_slot] = Some(new_voice);
                                    mixer.next_slot = (mixer.next_slot + 1) % MAX_VOICES;
                                }
                            }
                        }
                    }
                    Err(_) => break,
                }
            }

            mix_into(data, channels, &mut mixer.voices, mixer.volume);
        };

        let error_callback = move |err: cpal::StreamError| {
            device_lost_cb.store(true, Ordering::Relaxed);
            log::warn!("audio output stream error: {err}");
        };

        let stream = device
            .build_output_stream(&stream_config, data_callback, error_callback, None)
            .map_err(|e| AudioError::StreamFailed(e.to_string()))?;
        stream
            .play()
            .map_err(|e| AudioError::StreamFailed(e.to_string()))?;

        Ok(Self {
            tx,
            stream,
            sample_rate,
            device_lost,
        })
    }

    /// The sample rate the stream is running at. Packs must be resampled to this.
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// Non-blocking. Drops the command if the channel is full.
    pub fn send(&self, cmd: AudioCommand) {
        let _ = self.tx.try_send(cmd);
    }

    // SPEC-QUESTION: section 4.4 didn't list a sender accessor, but 5.1 step 11 requires the
    // dispatch thread to hold its own `Sender<AudioCommand>` clone (since `cpal::Stream`, and so
    // `AudioEngine`, is not `Send` and must stay on the main thread). Exposing a clone of the
    // internal sender is the natural way to satisfy that.
    /// Clone of the command sender, for threads that cannot hold the engine itself.
    pub fn sender_clone(&self) -> crossbeam_channel::Sender<AudioCommand> {
        self.tx.clone()
    }

    /// True if the output device was lost and the engine needs rebuilding.
    pub fn is_device_lost(&self) -> bool {
        self.device_lost.load(Ordering::Relaxed)
    }
}

/// One sounding key press.
struct Voice {
    samples: Arc<Vec<f32>>,
    /// Index into `samples` (interleaved), always even.
    pos: usize,
}

/// Lives inside the cpal callback closure. Never allocates during playback.
struct Mixer {
    voices: [Option<Voice>; MAX_VOICES],
    /// Round-robin index used when every slot is occupied.
    next_slot: usize,
    current: Arc<Vec<f32>>,
    volume: f32,
    rx: crossbeam_channel::Receiver<AudioCommand>,
}

/// Mixes all active voices into `data`, scaling by `volume` and clamping to avoid clipping.
/// Contains no allocation and is safe to call from the real-time audio callback.
fn mix_into(data: &mut [f32], channels: usize, voices: &mut [Option<Voice>], volume: f32) {
    data.fill(0.0);

    let frames = data.len() / channels;
    for slot in voices.iter_mut() {
        let Some(voice) = slot else { continue };
        for i in 0..frames {
            if voice.pos + 1 >= voice.samples.len() {
                *slot = None;
                break;
            }
            let l = voice.samples[voice.pos];
            let r = voice.samples[voice.pos + 1];
            voice.pos += 2;

            data[i * channels] += l * volume;
            if channels >= 2 {
                data[i * channels + 1] += r * volume;
            }
        }
    }

    for sample in data.iter_mut() {
        *sample = sample.clamp(-1.0, 1.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn voice_from(samples: Vec<f32>) -> Voice {
        Voice {
            samples: Arc::new(samples),
            pos: 0,
        }
    }

    #[test]
    fn silence_with_no_voices() {
        let mut data = vec![1.0; 8];
        let mut voices: [Option<Voice>; MAX_VOICES] = std::array::from_fn(|_| None);
        mix_into(&mut data, 2, &mut voices, 1.0);
        assert!(data.iter().all(|&s| s == 0.0));
    }

    #[test]
    fn single_voice_copies_samples() {
        let mut data = vec![0.0; 2];
        let mut voices: [Option<Voice>; MAX_VOICES] = std::array::from_fn(|_| None);
        voices[0] = Some(voice_from(vec![1.0, 1.0]));
        mix_into(&mut data, 2, &mut voices, 1.0);
        assert_eq!(data, vec![1.0, 1.0]);
    }

    #[test]
    fn volume_scales_output() {
        let mut data = vec![0.0; 2];
        let mut voices: [Option<Voice>; MAX_VOICES] = std::array::from_fn(|_| None);
        voices[0] = Some(voice_from(vec![1.0, 1.0]));
        mix_into(&mut data, 2, &mut voices, 0.5);
        assert_eq!(data, vec![0.5, 0.5]);
    }

    #[test]
    fn two_voices_sum() {
        let mut data = vec![0.0; 2];
        let mut voices: [Option<Voice>; MAX_VOICES] = std::array::from_fn(|_| None);
        voices[0] = Some(voice_from(vec![0.3, 0.3]));
        voices[1] = Some(voice_from(vec![0.3, 0.3]));
        mix_into(&mut data, 2, &mut voices, 1.0);
        assert!((data[0] - 0.6).abs() < 1e-6);
        assert!((data[1] - 0.6).abs() < 1e-6);
    }

    #[test]
    fn output_is_clamped() {
        let mut data = vec![0.0; 2];
        let mut voices: [Option<Voice>; MAX_VOICES] = std::array::from_fn(|_| None);
        for i in 0..5 {
            voices[i] = Some(voice_from(vec![0.5, 0.5]));
        }
        mix_into(&mut data, 2, &mut voices, 1.0);
        assert_eq!(data, vec![1.0, 1.0]);
    }

    #[test]
    fn finished_voice_is_cleared() {
        let mut data = vec![0.0; 8];
        let mut voices: [Option<Voice>; MAX_VOICES] = std::array::from_fn(|_| None);
        voices[0] = Some(voice_from(vec![1.0, 1.0]));
        mix_into(&mut data, 2, &mut voices, 1.0);
        assert!(voices[0].is_none());
        assert_eq!(data, vec![1.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]);
    }
}

use std::sync::Arc;

/// On-disk manifest, deserialized directly from pack.json.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct PackManifest {
    pub id: String,
    pub name: String,
    pub file: String,
}

/// A pack whose audio has been decoded, resampled, and made ready for the mixer.
#[derive(Debug, Clone)]
pub struct Pack {
    pub id: String,
    pub name: String,
    /// Interleaved stereo f32 in -1.0..=1.0, already at the output device's sample rate.
    pub samples: Arc<Vec<f32>>,
}

/// Errors that can occur while loading sound packs.
#[derive(Debug)]
pub enum PackError {
    /// assets/packs/ is missing or unreadable.
    PacksDirMissing(std::path::PathBuf),
    /// pack.json missing, unreadable, or malformed.
    BadManifest { dir: std::path::PathBuf, reason: String },
    /// The WAV referenced by the manifest is missing or not 16-bit PCM.
    BadWav { path: std::path::PathBuf, reason: String },
    /// Zero packs were found.
    NoPacksFound,
}

impl std::fmt::Display for PackError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PackError::PacksDirMissing(p) => write!(f, "packs directory missing: {}", p.display()),
            PackError::BadManifest { dir, reason } => {
                write!(f, "bad manifest in {}: {reason}", dir.display())
            }
            PackError::BadWav { path, reason } => {
                write!(f, "bad wav file {}: {reason}", path.display())
            }
            PackError::NoPacksFound => write!(f, "no sound packs found"),
        }
    }
}

impl std::error::Error for PackError {}

/// Absolute path to the bundled packs directory, resolved at compile time from CARGO_MANIFEST_DIR.
pub fn packs_dir() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/packs")
}

/// Scan assets/packs/, load every valid pack, resample to `output_sample_rate`.
/// Directories that fail to load are logged at warn level and skipped, not fatal.
/// Returns packs sorted by `name`. Errors only if zero packs load.
pub fn load_all(output_sample_rate: u32) -> Result<Vec<Pack>, PackError> {
    let dir = packs_dir();
    let entries = std::fs::read_dir(&dir).map_err(|_| PackError::PacksDirMissing(dir.clone()))?;

    let mut packs = Vec::new();
    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                log::warn!("failed to read pack entry: {e}");
                continue;
            }
        };
        let entry_path = entry.path();
        if !entry_path.is_dir() {
            continue;
        }

        let manifest_path = entry_path.join("pack.json");
        let manifest_str = match std::fs::read_to_string(&manifest_path) {
            Ok(s) => s,
            Err(e) => {
                log::warn!("skipping pack {}: {e}", entry_path.display());
                continue;
            }
        };
        let manifest: PackManifest = match serde_json::from_str(&manifest_str) {
            Ok(m) => m,
            Err(e) => {
                let err = PackError::BadManifest {
                    dir: entry_path.clone(),
                    reason: e.to_string(),
                };
                log::warn!("skipping pack: {err}");
                continue;
            }
        };

        let wav_path = entry_path.join(&manifest.file);
        let (mono, wav_rate) = match decode_wav_mono(&wav_path) {
            Ok(v) => v,
            Err(e) => {
                log::warn!("skipping pack {}: {e}", entry_path.display());
                continue;
            }
        };

        let resampled = resample_linear(&mono, wav_rate, output_sample_rate);
        let samples = Arc::new(mono_to_interleaved_stereo(&resampled));
        packs.push(Pack {
            id: manifest.id,
            name: manifest.name,
            samples,
        });
    }

    if packs.is_empty() {
        return Err(PackError::NoPacksFound);
    }

    packs.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(packs)
}

/// Decode one 16-bit PCM WAV to mono f32 in -1.0..=1.0, averaging channels if multi-channel.
fn decode_wav_mono(path: &std::path::Path) -> Result<(Vec<f32>, u32), PackError> {
    let mut reader = hound::WavReader::open(path).map_err(|e| PackError::BadWav {
        path: path.to_path_buf(),
        reason: e.to_string(),
    })?;

    let spec = reader.spec();
    if spec.sample_format != hound::SampleFormat::Int || spec.bits_per_sample != 16 {
        return Err(PackError::BadWav {
            path: path.to_path_buf(),
            reason: "expected 16-bit integer PCM".to_string(),
        });
    }

    let raw: Result<Vec<i16>, _> = reader.samples::<i16>().collect();
    let raw = raw.map_err(|e| PackError::BadWav {
        path: path.to_path_buf(),
        reason: e.to_string(),
    })?;
    let floats: Vec<f32> = raw.iter().map(|&s| s as f32 / 32768.0).collect();

    let channels = spec.channels as usize;
    let mono = if channels > 1 {
        floats
            .chunks_exact(channels)
            .map(|chunk| chunk.iter().sum::<f32>() / channels as f32)
            .collect()
    } else {
        floats
    };

    Ok((mono, spec.sample_rate))
}

/// Linearly resample mono f32 from `from_rate` to `to_rate`. Returns input unchanged if rates match.
fn resample_linear(input: &[f32], from_rate: u32, to_rate: u32) -> Vec<f32> {
    if from_rate == to_rate || input.is_empty() {
        return input.to_vec();
    }

    let ratio = from_rate as f64 / to_rate as f64;
    let out_len = ((input.len() as f64) / ratio).floor() as usize;
    let mut out = Vec::with_capacity(out_len);
    for i in 0..out_len {
        let src = i as f64 * ratio;
        let idx = src.floor() as usize;
        let frac = (src - src.floor()) as f32;
        let a = input[idx];
        let b = *input.get(idx + 1).unwrap_or(&a);
        out.push(a + (b - a) * frac);
    }
    out
}

/// Duplicate each mono sample into an interleaved L,R pair.
fn mono_to_interleaved_stereo(mono: &[f32]) -> Vec<f32> {
    let mut out = Vec::with_capacity(mono.len() * 2);
    for &s in mono {
        out.push(s);
        out.push(s);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resample_identity() {
        let out = resample_linear(&[1.0, 2.0, 3.0], 44100, 44100);
        assert_eq!(out, vec![1.0, 2.0, 3.0]);
    }

    #[test]
    fn resample_halves_length() {
        let out = resample_linear(&[0.0, 1.0, 2.0, 3.0], 44100, 22050);
        assert_eq!(out.len(), 2);
        assert_eq!(out[0], 0.0);
    }

    #[test]
    fn resample_empty() {
        let out = resample_linear(&[], 44100, 48000);
        assert!(out.is_empty());
    }

    #[test]
    fn resample_interpolates() {
        let out = resample_linear(&[0.0, 1.0], 2, 4);
        assert_eq!(out.len(), 4);
        assert!((out[1] - 0.5).abs() < 1e-6);
    }

    #[test]
    fn mono_to_stereo_duplicates() {
        let out = mono_to_interleaved_stereo(&[0.5, -0.5]);
        assert_eq!(out, vec![0.5, 0.5, -0.5, -0.5]);
    }

    #[test]
    fn mono_to_stereo_empty() {
        let out = mono_to_interleaved_stereo(&[]);
        assert!(out.is_empty());
    }
}

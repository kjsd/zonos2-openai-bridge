use byteorder::{LittleEndian, WriteBytesExt};

/// Converts raw 32-bit float PCM (44.1kHz, Mono, Little-Endian)
/// into standard 16-bit signed integer RIFF WAVE (Format 1).
pub fn float32_to_pcm16_wav(raw_float_pcm: &[u8], sample_rate: u32, channels: u16) -> Result<Vec<u8>, String> {
    if raw_float_pcm.len() % 4 != 0 {
        return Err(format!(
            "Input raw PCM length must be multiple of 4 bytes (Float32), got {}",
            raw_float_pcm.len()
        ));
    }

    let num_samples = raw_float_pcm.len() / 4;
    let data_len = (num_samples * 2) as u32;
    let riff_chunk_size = 36 + data_len;

    // Pre-allocate full buffer: 44 bytes header + PCM 16 data
    let mut out = Vec::with_capacity(44 + (num_samples * 2));

    // 1. RIFF header
    out.extend_from_slice(b"RIFF");
    out.write_u32::<LittleEndian>(riff_chunk_size).map_err(|e| e.to_string())?;
    out.extend_from_slice(b"WAVE");

    // 2. fmt subchunk
    out.extend_from_slice(b"fmt ");
    out.write_u32::<LittleEndian>(16).map_err(|e| e.to_string())?; // Subchunk1Size for PCM
    out.write_u16::<LittleEndian>(1).map_err(|e| e.to_string())?;  // AudioFormat: 1 = PCM (Integer)
    out.write_u16::<LittleEndian>(channels).map_err(|e| e.to_string())?;
    out.write_u32::<LittleEndian>(sample_rate).map_err(|e| e.to_string())?;

    let byte_rate = sample_rate * (channels as u32) * 2;
    let block_align = channels * 2;
    out.write_u32::<LittleEndian>(byte_rate).map_err(|e| e.to_string())?;
    out.write_u16::<LittleEndian>(block_align).map_err(|e| e.to_string())?;
    out.write_u16::<LittleEndian>(16).map_err(|e| e.to_string())?; // BitsPerSample: 16

    // 3. data subchunk
    out.extend_from_slice(b"data");
    out.write_u32::<LittleEndian>(data_len).map_err(|e| e.to_string())?;

    // 4. Convert float32 samples to int16 samples
    for chunk in raw_float_pcm.chunks_exact(4) {
        let f = f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
        
        let sample: i16 = if f.is_nan() || f.is_infinite() {
            0
        } else if f >= 1.0 {
            32767
        } else if f <= -1.0 {
            -32768
        } else if f >= 0.0 {
            (f * 32767.0).round() as i16
        } else {
            (f * 32768.0).round() as i16
        };

        out.write_i16::<LittleEndian>(sample).map_err(|e| e.to_string())?;
    }

    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_float32_to_pcm16_wav_header() {
        // Create 2 float32 samples: 0.0, 0.5
        let mut input = Vec::new();
        input.write_f32::<LittleEndian>(0.0).unwrap();
        input.write_f32::<LittleEndian>(0.5).unwrap();

        let wav = float32_to_pcm16_wav(&input, 44100, 1).expect("Conversion failed");
        
        // 44 bytes header + 2 * 2 bytes = 48 bytes
        assert_eq!(wav.len(), 48);

        // Header magic checks
        assert_eq!(&wav[0..4], b"RIFF");
        assert_eq!(&wav[8..12], b"WAVE");
        assert_eq!(&wav[12..16], b"fmt ");
        assert_eq!(&wav[36..40], b"data");

        // Format code = 1 (PCM Integer)
        let format_code = u16::from_le_bytes([wav[20], wav[21]]);
        assert_eq!(format_code, 1);

        // Channels = 1
        let channels = u16::from_le_bytes([wav[22], wav[23]]);
        assert_eq!(channels, 1);

        // Sample rate = 44100
        let sample_rate = u32::from_le_bytes([wav[24], wav[25], wav[26], wav[27]]);
        assert_eq!(sample_rate, 44100);

        // Bits per sample = 16
        let bits_per_sample = u16::from_le_bytes([wav[34], wav[35]]);
        assert_eq!(bits_per_sample, 16);

        // Check sample values: 0.0 -> 0, 0.5 -> 16384 (approx 32767 * 0.5)
        let s0 = i16::from_le_bytes([wav[44], wav[45]]);
        let s1 = i16::from_le_bytes([wav[46], wav[47]]);
        assert_eq!(s0, 0);
        assert!((s1 - 16384).abs() <= 1);
    }

    #[test]
    fn test_clipping_protection() {
        let mut input = Vec::new();
        input.write_f32::<LittleEndian>(1.5).unwrap();  // Overflow
        input.write_f32::<LittleEndian>(-2.0).unwrap(); // Underflow
        input.write_f32::<LittleEndian>(f32::NAN).unwrap(); // NaN

        let wav = float32_to_pcm16_wav(&input, 44100, 1).expect("Conversion failed");
        assert_eq!(wav.len(), 44 + 6);

        let s0 = i16::from_le_bytes([wav[44], wav[45]]);
        let s1 = i16::from_le_bytes([wav[46], wav[47]]);
        let s2 = i16::from_le_bytes([wav[48], wav[49]]);

        assert_eq!(s0, 32767);
        assert_eq!(s1, -32768);
        assert_eq!(s2, 0);
    }
}

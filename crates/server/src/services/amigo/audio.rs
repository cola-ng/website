use std::io::{Cursor, ErrorKind};

use amigo::protocol::MAX_REPLY_BYTES;
use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::errors::Error;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

pub fn wav(pcm: &[u8]) -> Result<Vec<u8>, String> {
    if pcm.is_empty() || !pcm.len().is_multiple_of(2) || pcm.len() > MAX_REPLY_BYTES {
        return Err("invalid PCM length".into());
    }
    let size = pcm.len() as u32;
    let mut out = Vec::with_capacity(44 + pcm.len());
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + size).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&16_000u32.to_le_bytes());
    out.extend_from_slice(&32_000u32.to_le_bytes());
    out.extend_from_slice(&2u16.to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&size.to_le_bytes());
    out.extend_from_slice(pcm);
    Ok(out)
}

/// Decode bounded provider WAV/MP3 output, downmix and resample to PCM16/16k.
/// A windowed-sinc low-pass filter prevents naive decimation aliasing.
pub fn decode(bytes: Vec<u8>, format: &str) -> Result<Vec<u8>, String> {
    if bytes.is_empty() || bytes.len() > 4 * 1024 * 1024 {
        return Err("TTS media exceeds limit".into());
    }
    let mut hint = Hint::new();
    hint.with_extension(format);
    let source = MediaSourceStream::new(Box::new(Cursor::new(bytes)), Default::default());
    let probed = symphonia::default::get_probe()
        .format(
            &hint,
            source,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        )
        .map_err(|_| "unsupported TTS media")?;
    let mut reader = probed.format;
    let track = reader
        .default_track()
        .ok_or("TTS media has no audio track")?;
    let track_id = track.id;
    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
        .map_err(|_| "unsupported TTS codec")?;
    let mut mono = Vec::new();
    let mut rate = 0;
    loop {
        let packet = match reader.next_packet() {
            Ok(packet) => packet,
            Err(Error::IoError(error)) if error.kind() == ErrorKind::UnexpectedEof => break,
            Err(_) => return Err("invalid TTS container".into()),
        };
        if packet.track_id() != track_id {
            continue;
        }
        let decoded = decoder.decode(&packet).map_err(|_| "invalid TTS audio")?;
        let spec = *decoded.spec();
        let channels = spec.channels.count();
        if channels == 0
            || channels > 8
            || !(8000..=96000).contains(&spec.rate)
            || (rate != 0 && rate != spec.rate)
        {
            return Err("unsupported TTS sample format".into());
        }
        rate = spec.rate;
        if decoded.capacity() > rate as usize * 2 {
            return Err("TTS packet exceeds limit".into());
        }
        let mut buffer = SampleBuffer::<f32>::new(decoded.capacity() as u64, spec);
        buffer.copy_interleaved_ref(decoded);
        for frame in buffer.samples().chunks_exact(channels) {
            mono.push(frame.iter().sum::<f32>() / channels as f32);
        }
        if mono.len() > rate as usize * 30 {
            return Err("TTS exceeds 30 seconds".into());
        }
    }
    if rate == 0 || mono.is_empty() {
        return Err("empty TTS audio".into());
    }
    Ok(resample(&mono, rate))
}

fn resample(input: &[f32], rate: u32) -> Vec<u8> {
    let length = input.len() * 16_000 / rate as usize;
    let mut pcm = Vec::with_capacity(length * 2);
    let cutoff = (16_000.0 / f64::from(rate)).min(1.0) * 0.94;
    for index in 0..length {
        let position = index as f64 * f64::from(rate) / 16_000.0;
        let (mut value, mut weights) = (0.0, 0.0);
        for offset in -24..=24 {
            let sample_index = position.floor() as isize + offset;
            if sample_index < 0 || sample_index >= input.len() as isize {
                continue;
            }
            let delta = position - sample_index as f64;
            let x = std::f64::consts::PI * delta * cutoff;
            let sinc = if x.abs() < 1e-9 { 1.0 } else { x.sin() / x };
            let window = 0.5 + 0.5 * (std::f64::consts::PI * delta / 25.0).cos();
            let weight = cutoff * sinc * window;
            value += f64::from(input[sample_index as usize]) * weight;
            weights += weight;
        }
        let sample = if weights.abs() > 1e-9 {
            value / weights
        } else {
            0.0
        };
        pcm.extend_from_slice(&((sample.clamp(-1.0, 1.0) * 32767.0).round() as i16).to_le_bytes());
    }
    pcm
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn wav_is_decodable_and_invalid_media_is_rejected() {
        let pcm: Vec<_> = (0..1600)
            .flat_map(|i| (((i as f32 * 0.2).sin() * 12000.0) as i16).to_le_bytes())
            .collect();
        let encoded = wav(&pcm).unwrap();
        assert_eq!(&encoded[..4], b"RIFF");
        let decoded = decode(encoded, "wav").unwrap();
        assert_eq!(decoded.len(), pcm.len());
        assert!(decoded.iter().any(|&b| b != 0));
        assert!(decode(vec![0; 100], "mp3").is_err());
        assert!(wav(&[1]).is_err());
    }
    #[test]
    fn resampling_preserves_duration_and_suppresses_aliasing() {
        let tone = |hz: f64| {
            (0..4800)
                .map(|i| (2.0 * std::f64::consts::PI * hz * i as f64 / 48000.0).sin() as f32)
                .collect::<Vec<_>>()
        };
        let low = resample(&tone(1000.0), 48000);
        let high = resample(&tone(14000.0), 48000);
        assert_eq!(low.len(), 3200);
        let energy = |bytes: &[u8]| {
            bytes[100..bytes.len() - 100]
                .as_chunks::<2>()
                .0
                .iter()
                .map(|b| f64::from(i16::from_le_bytes(*b)).powi(2))
                .sum::<f64>()
        };
        assert!(energy(&high) < energy(&low) * 0.01);
    }
}

//! Fixed-size, mono PCM16 frames for the proposed device transport.
//!
//! I2S adapters must convert their hardware sample format to 16 kHz signed
//! 16-bit samples first. The wire format is little-endian PCM, without a WAV
//! header. A transport adapter must handle a final partial frame explicitly.

pub const SAMPLE_RATE_HZ: usize = 16_000;
pub const FRAME_DURATION_MS: usize = 20;
pub const FRAME_SAMPLES: usize = SAMPLE_RATE_HZ * FRAME_DURATION_MS / 1_000;
pub const FRAME_BYTES: usize = FRAME_SAMPLES * size_of::<i16>();

/// One 20 ms frame. No heap allocation or implicit native-endian conversion.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PcmFrame {
    pub samples: [i16; FRAME_SAMPLES],
}

impl PcmFrame {
    pub const fn silence() -> Self {
        Self {
            samples: [0; FRAME_SAMPLES],
        }
    }

    pub fn encode_le(&self) -> [u8; FRAME_BYTES] {
        let mut bytes = [0; FRAME_BYTES];
        for (sample, output) in self.samples.iter().zip(bytes.as_chunks_mut::<2>().0) {
            *output = sample.to_le_bytes();
        }
        bytes
    }

    pub fn decode_le(bytes: &[u8; FRAME_BYTES]) -> Self {
        let mut frame = Self::silence();
        for (sample, input) in frame.samples.iter_mut().zip(bytes.as_chunks::<2>().0) {
            *sample = i16::from_le_bytes(*input);
        }
        frame
    }
}

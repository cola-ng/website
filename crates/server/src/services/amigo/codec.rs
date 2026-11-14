//! Raw RFC6716 packets; PCM remains the model pipeline's internal representation.
use amigo::protocol::{MAX_OPUS_PACKET_BYTES, OPUS_FRAME_SAMPLES, opus_payload};

#[derive(Default)]
pub struct OpusWire {
    encoder: Option<(u64, opus::Encoder)>,
    decoder: Option<(u64, opus::Decoder)>,
}
impl OpusWire {
    pub fn clear_encoder(&mut self) {
        self.encoder = None;
    }
    pub fn clear_decoder(&mut self) {
        self.decoder = None;
    }
    pub fn encode(&mut self, turn: u64, pcm: &[u8]) -> Result<Vec<u8>, String> {
        if pcm.is_empty() || pcm.len() > 640 || !pcm.len().is_multiple_of(2) {
            return Err("invalid encoder PCM".into());
        }
        if self.encoder.as_ref().is_none_or(|(id, _)| *id != turn) {
            let mut encoder =
                opus::Encoder::new(16000, opus::Channels::Mono, opus::Application::Voip)
                    .map_err(|e| e.to_string())?;
            encoder
                .set_bitrate(opus::Bitrate::Bits(16000))
                .map_err(|e| e.to_string())?;
            encoder.set_complexity(1).map_err(|e| e.to_string())?;
            self.encoder = Some((turn, encoder));
        }
        let mut input = [0i16; OPUS_FRAME_SAMPLES];
        for (out, bytes) in input.iter_mut().zip(pcm.as_chunks::<2>().0) {
            *out = i16::from_le_bytes([bytes[0], bytes[1]]);
        }
        let mut output = vec![0u8; MAX_OPUS_PACKET_BYTES + 2];
        output[..2].copy_from_slice(&((pcm.len() / 2) as u16).to_le_bytes());
        let size = self
            .encoder
            .as_mut()
            .unwrap()
            .1
            .encode(&input, &mut output[2..])
            .map_err(|e| e.to_string())?;
        output.truncate(size + 2);
        Ok(output)
    }
    pub fn decode(&mut self, turn: u64, payload: &[u8]) -> Result<Vec<u8>, String> {
        let (valid, packet) = opus_payload(payload).map_err(str::to_owned)?;
        if opus::packet::get_nb_samples(packet, 16000).map_err(|e| e.to_string())?
            != OPUS_FRAME_SAMPLES
        {
            return Err("Opus frame must be 20 ms".into());
        }
        if self.decoder.as_ref().is_none_or(|(id, _)| *id != turn) {
            self.decoder = Some((
                turn,
                opus::Decoder::new(16000, opus::Channels::Mono).map_err(|e| e.to_string())?,
            ));
        }
        let mut output = [0i16; OPUS_FRAME_SAMPLES];
        let size = self
            .decoder
            .as_mut()
            .unwrap()
            .1
            .decode(packet, &mut output, false)
            .map_err(|e| e.to_string())?;
        if size != OPUS_FRAME_SAMPLES {
            return Err("invalid decoded duration".into());
        }
        Ok(output[..valid]
            .iter()
            .flat_map(|sample| sample.to_le_bytes())
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn opus_stream_preserves_duration_tail_and_resets_between_turns() {
        let mut sender = OpusWire::default();
        let mut receiver = OpusWire::default();
        let pcm = (0..320)
            .flat_map(|i| (((i as f32 * 0.12).sin() * 8000.) as i16).to_le_bytes())
            .collect::<Vec<_>>();
        let mut wire_bytes = 0;
        let mut energy = 0i64;
        for _ in 0..10 {
            let packet = sender.encode(1, &pcm).unwrap();
            wire_bytes += packet.len();
            let decoded = receiver.decode(1, &packet).unwrap();
            assert_eq!(decoded.len(), 640);
            energy += decoded
                .as_chunks::<2>()
                .0
                .iter()
                .map(|s| i64::from(i16::from_le_bytes([s[0], s[1]])).abs())
                .sum::<i64>();
        }
        assert!(wire_bytes < 2000 && energy > 10000);
        let tail = sender.encode(1, &pcm[..138]).unwrap();
        assert_eq!(receiver.decode(1, &tail).unwrap().len(), 138);
        let fresh = OpusWire::default().encode(2, &pcm).unwrap();
        assert_eq!(sender.encode(2, &pcm).unwrap(), fresh);
        assert!(receiver.decode(3, &[64, 1, 255, 255]).is_err());
    }
}

//! Version 1 device protocol. Enable only for the ESP-IDF/host adapters.

use std::string::String;
use std::vec::Vec;

use serde::{Deserialize, Serialize};

pub const VERSION: u16 = 1;
pub const LICHUANG_BOARD: &str = "lckfb-szpi-esp32s3";
pub fn supported_board(board: &str) -> bool {
    matches!(board, "esp32-s3-korvo-2" | "simulator" | LICHUANG_BOARD)
}
fn is_false(value: &bool) -> bool {
    !value
}
pub const HEADER_BYTES: usize = 20;
pub const MAX_CONTROL_BYTES: usize = 16 * 1024;
pub const MAX_IMAGE_BYTES: usize = 256 * 1024;
pub const MAX_AUDIO_BYTES: usize = 16_000 * 2 * 15;
pub const MAX_REPLY_BYTES: usize = 16_000 * 2 * 30;
pub const SERVER_TURN_START: u64 = 1 << 63;
pub const OPUS_FRAME_SAMPLES: usize = 320;
pub const MAX_OPUS_PACKET_BYTES: usize = 256;
pub const CODEC_TAIL_BYTES: usize = 640; // One 20 ms flush frame for Opus lookahead.

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum AudioCodec {
    #[default]
    Pcm16,
    Opus,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AudioOffer {
    pub codecs: Vec<AudioCodec>,
    pub frame_ms: u16,
}
pub fn negotiate_audio(
    offer: Option<&AudioOffer>,
    opus_enabled: bool,
) -> Result<AudioCodec, &'static str> {
    let Some(offer) = offer else {
        return Ok(AudioCodec::Pcm16);
    };
    if offer.frame_ms != 20 || offer.codecs.is_empty() || offer.codecs.len() > 2 {
        return Err("expected one or two audio codecs and 20 ms frames");
    }
    if opus_enabled && offer.codecs.contains(&AudioCodec::Opus) {
        return Ok(AudioCodec::Opus);
    }
    if offer.codecs.contains(&AudioCodec::Pcm16) {
        return Ok(AudioCodec::Pcm16);
    }
    Err("no common audio codec")
}

/// Each Opus packet decodes to 320 samples; only the last downlink packet may
/// carry fewer valid samples. The count preserves exact PCM audio_end lengths.
pub fn opus_payload(payload: &[u8]) -> Result<(usize, &[u8]), &'static str> {
    if !(3..=MAX_OPUS_PACKET_BYTES + 2).contains(&payload.len()) {
        return Err("invalid Opus packet size");
    }
    let valid = u16::from_le_bytes([payload[0], payload[1]]) as usize;
    if !(1..=OPUS_FRAME_SAMPLES).contains(&valid) {
        return Err("invalid Opus sample count");
    }
    Ok((valid, &payload[2..]))
}

// Browser JavaScript numbers cannot represent the server's high-bit IDs.
// Serialize decimal strings; accept older numeric inputs for Rust adapters.
mod wire_id {
    use serde::{Deserialize, Deserializer, Serializer};
    use std::string::{String, ToString};
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Id {
        Number(u64),
        Text(String),
    }
    pub fn serialize<S: Serializer>(value: &u64, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&value.to_string())
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<u64, D::Error> {
        match Id::deserialize(d)? {
            Id::Number(n) => Ok(n),
            Id::Text(s) => s.parse().map_err(serde::de::Error::custom),
        }
    }
    pub mod optional {
        use super::*;
        use serde::Serialize;
        pub fn serialize<S: Serializer>(
            value: &Option<u64>,
            serializer: S,
        ) -> Result<S::Ok, S::Error> {
            value.map(|id| id.to_string()).serialize(serializer)
        }
        pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Option<u64>, D::Error> {
            Option::<Id>::deserialize(d)?
                .map(|id| match id {
                    Id::Number(n) => Ok(n),
                    Id::Text(s) => s.parse().map_err(serde::de::Error::custom),
                })
                .transpose()
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Source {
    Screen,
    Book,
    Speech,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Trigger {
    Button,
    WakeWord,
    Voice,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum ClientMessage {
    ConfigurationApplied {
        revision: i64,
    },
    CameraState {
        enabled: bool,
    },
    ObserveNow {
        source: Source,
    },
    Hello {
        version: u16,
        board: String,
        sample_rate_hz: u32,
        channels: u8,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        audio: Option<AudioOffer>,
        #[serde(default, skip_serializing_if = "is_false")]
        streaming: bool,
        #[serde(default, skip_serializing_if = "is_false")]
        service_status: bool,
    },
    Wake {
        #[serde(with = "wire_id")]
        turn_id: u64,
        trigger: Trigger,
    },
    EndTurn {
        #[serde(with = "wire_id")]
        turn_id: u64,
    },
    /// Also useful for text-capable adapters; does not bypass model analysis.
    Text {
        #[serde(with = "wire_id")]
        turn_id: u64,
        text: String,
    },
    Cancel {
        #[serde(with = "wire_id")]
        turn_id: u64,
    },
    PlaybackDone {
        #[serde(with = "wire_id")]
        turn_id: u64,
    },
    Presence {
        busy: bool,
        muted: bool,
    },
    ObservationText {
        #[serde(with = "wire_id")]
        request_id: u64,
        source: Source,
        text: String,
    },
    ObservationStart {
        #[serde(with = "wire_id")]
        request_id: u64,
        source: Source,
        bytes: usize,
    },
    CaptureUnavailable {
        #[serde(with = "wire_id")]
        request_id: u64,
    },
    Feedback {
        #[serde(with = "wire_id")]
        turn_id: u64,
        accepted: bool,
    },
    Ping,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage {
    ServiceState { ready: bool },
    Configuration {
        revision: i64,
        volume: u8,
        continuous: bool,
        voice_interrupt: bool,
        playback_wake: bool,
        camera_enabled: bool,
        ambient_enabled: bool,
    },
    LearningState {
        #[serde(with = "wire_id")]
        turn_id: u64,
        mode: String,
        stage: String,
        goal: String,
        strategy: String,
        explanation: String,
        revision: u64,
        status: String,
    },
    TranscriptPartial {
        #[serde(with = "wire_id")]
        turn_id: u64,
        text: String,
        #[serde(default)]
        utterance_complete: bool,
    },
    ReplyUpdate {
        #[serde(with = "wire_id")]
        turn_id: u64,
        text: String,
        hint_zh: String,
    },
    Welcome {
        session_id: String,
        version: u16,
        sample_rate_hz: u32,
        frame_ms: u16,
        test_mode: bool,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        audio: Option<AudioCodec>,
    },
    Ack {
        #[serde(with = "wire_id")]
        turn_id: u64,
        state: String,
    },
    Transcript {
        #[serde(with = "wire_id")]
        turn_id: u64,
        text: String,
    },
    Reply {
        #[serde(with = "wire_id")]
        turn_id: u64,
        proactive: bool,
        text: String,
        hint_zh: String,
        targets: Vec<String>,
        #[serde(default)]
        agent_id: Option<String>,
        #[serde(default)]
        agent_name: Option<String>,
        #[serde(default)]
        response_act: Option<String>,
    },
    AudioEnd {
        #[serde(with = "wire_id")]
        turn_id: u64,
        bytes: usize,
    },
    Cancelled {
        #[serde(with = "wire_id")]
        turn_id: u64,
    },
    Capture {
        #[serde(with = "wire_id")]
        request_id: u64,
        source: Source,
        max_bytes: usize,
        max_audio_ms: u32,
    },
    ObservationStored {
        #[serde(with = "wire_id")]
        request_id: u64,
    },
    Error {
        #[serde(default, with = "wire_id::optional")]
        turn_id: Option<u64>,
        code: String,
        message: String,
    },
    Pong,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum MediaKind {
    AudioIn = 1,
    AudioOut = 2,
    Observation = 3,
    OpusIn = 4,
    OpusOut = 5,
}

/// LBT1 | kind:u8 | reserved:0 | turn:u64le | sequence:u32le | length:u16le | payload.
pub struct MediaPacket<'a> {
    pub kind: MediaKind,
    pub turn_id: u64,
    pub sequence: u32,
    pub payload: &'a [u8],
}

impl<'a> MediaPacket<'a> {
    pub fn decode(bytes: &'a [u8]) -> Result<Self, &'static str> {
        if bytes.len() < HEADER_BYTES || &bytes[..4] != b"LBT1" || bytes[5] != 0 {
            return Err("invalid media header");
        }
        let kind = match bytes[4] {
            1 => MediaKind::AudioIn,
            2 => MediaKind::AudioOut,
            3 => MediaKind::Observation,
            4 => MediaKind::OpusIn,
            5 => MediaKind::OpusOut,
            _ => return Err("unknown media kind"),
        };
        let turn_id = u64::from_le_bytes(bytes[6..14].try_into().unwrap());
        let sequence = u32::from_le_bytes(bytes[14..18].try_into().unwrap());
        let len = u16::from_le_bytes(bytes[18..20].try_into().unwrap()) as usize;
        if len == 0 || bytes.len() != HEADER_BYTES + len {
            return Err("invalid media length");
        }
        if matches!(kind, MediaKind::AudioIn | MediaKind::AudioOut)
            && (len > 640 || !len.is_multiple_of(2))
        {
            return Err("audio must contain 1-320 PCM16 samples");
        }
        if matches!(kind, MediaKind::OpusIn | MediaKind::OpusOut) {
            opus_payload(&bytes[HEADER_BYTES..])?;
        }
        Ok(Self {
            kind,
            turn_id,
            sequence,
            payload: &bytes[HEADER_BYTES..],
        })
    }

    pub fn encode(&self) -> Result<Vec<u8>, &'static str> {
        let len = u16::try_from(self.payload.len()).map_err(|_| "media packet too large")?;
        let mut bytes = Vec::with_capacity(HEADER_BYTES + self.payload.len());
        bytes.extend_from_slice(b"LBT1");
        bytes.extend_from_slice(&[self.kind as u8, 0]);
        bytes.extend_from_slice(&self.turn_id.to_le_bytes());
        bytes.extend_from_slice(&self.sequence.to_le_bytes());
        bytes.extend_from_slice(&len.to_le_bytes());
        bytes.extend_from_slice(self.payload);
        MediaPacket::decode(&bytes)?;
        Ok(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::vec;
    #[test]
    fn board_handshake_accepts_lichuang_and_preserves_existing_devices() {
        for name in [LICHUANG_BOARD, "esp32-s3-korvo-2", "simulator"] {
            assert!(supported_board(name));
        }
        assert!(!supported_board(""));
        assert!(!supported_board("lichuang-c3"));
        assert!(!supported_board("unknown"));
    }
    #[test]
    fn negotiation_preserves_pcm_legacy_and_requires_a_common_format() {
        assert_eq!(negotiate_audio(None, true).unwrap(), AudioCodec::Pcm16);
        let offer = AudioOffer {
            codecs: vec![AudioCodec::Opus, AudioCodec::Pcm16],
            frame_ms: 20,
        };
        assert_eq!(
            negotiate_audio(Some(&offer), true).unwrap(),
            AudioCodec::Opus
        );
        assert_eq!(
            negotiate_audio(Some(&offer), false).unwrap(),
            AudioCodec::Pcm16
        );
        assert!(
            negotiate_audio(
                Some(&AudioOffer {
                    codecs: vec![AudioCodec::Opus],
                    frame_ms: 20
                }),
                false
            )
            .is_err()
        );
        assert!(
            negotiate_audio(
                Some(&AudioOffer {
                    frame_ms: 60,
                    ..offer
                }),
                true
            )
            .is_err()
        );
    }
    #[test]
    fn opus_packets_allow_odd_lengths_but_bound_duration_and_size() {
        let packet = MediaPacket {
            kind: MediaKind::OpusOut,
            turn_id: 7,
            sequence: 2,
            payload: &[64, 1, 0xf8],
        };
        let encoded = packet.encode().unwrap();
        assert_eq!(
            MediaPacket::decode(&encoded).unwrap().kind,
            MediaKind::OpusOut
        );
        assert!(opus_payload(&[0, 0, 0xf8]).is_err());
        assert!(opus_payload(&[65, 1, 0xf8]).is_err());
        assert!(
            MediaPacket {
                payload: &[1; 260],
                ..packet
            }
            .encode()
            .is_err()
        );
    }

    #[test]
    #[cfg(feature = "simulator")]
    fn browser_ids_preserve_all_u64_bits_and_accept_legacy_numbers() {
        let id = (1u64 << 63) + 123;
        let json =
            serde_json::to_value(ServerMessage::ObservationStored { request_id: id }).unwrap();
        assert_eq!(json["request_id"], "9223372036854775931");
        let event: ServerMessage = serde_json::from_value(json).unwrap();
        assert!(
            matches!(event, ServerMessage::ObservationStored { request_id } if request_id == id)
        );
        for value in [serde_json::json!("42"), serde_json::json!(42)] {
            let event: ClientMessage =
                serde_json::from_value(serde_json::json!({"type":"cancel","turn_id":value}))
                    .unwrap();
            assert!(matches!(event, ClientMessage::Cancel { turn_id: 42 }));
        }
        assert!(
            serde_json::from_value::<ClientMessage>(
                serde_json::json!({"type":"cancel","turn_id":"18446744073709551616"})
            )
            .is_err()
        );
    }

    #[test]
    fn media_header_roundtrip_and_malformed_frames() {
        let packet = MediaPacket {
            kind: MediaKind::AudioIn,
            turn_id: 42,
            sequence: 7,
            payload: &[0, 128],
        };
        let mut bytes = packet.encode().unwrap();
        let parsed = MediaPacket::decode(&bytes).unwrap();
        assert_eq!(
            (parsed.turn_id, parsed.sequence, parsed.payload),
            (42, 7, &[0, 128][..])
        );
        bytes.push(0);
        assert!(MediaPacket::decode(&bytes).is_err());
        assert!(MediaPacket::decode(b"LBT1").is_err());
        assert!(
            MediaPacket {
                payload: &[1],
                ..packet
            }
            .encode()
            .is_err()
        );
    }
}

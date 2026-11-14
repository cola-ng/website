//! Networked Korvo-2 simulator using the production device protocol.
//! Supply real PCM16/16k mono WAV and JPEG fixtures for cloud ASR/vision.

use futures_util::{SinkExt, StreamExt};
use amigo::protocol::*;
use std::error::Error;
use std::time::{Duration, Instant};
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::{Message, client::IntoClientRequest};
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async};

type Socket = WebSocketStream<MaybeTlsStream<TcpStream>>;
type Result<T> = std::result::Result<T, Box<dyn Error>>;

fn option(name: &str) -> Option<String> {
    let args: Vec<_> = std::env::args().collect();
    args.windows(2)
        .find(|pair| pair[0] == name)
        .map(|pair| pair[1].clone())
}

async fn control(ws: &mut Socket, message: ClientMessage) -> Result<()> {
    ws.send(Message::Text(serde_json::to_string(&message)?.into()))
        .await?;
    Ok(())
}

async fn upload(ws: &mut Socket, kind: MediaKind, id: u64, data: &[u8]) -> Result<()> {
    let chunk_size = if kind == MediaKind::AudioIn {
        640
    } else {
        16 * 1024
    };
    for (sequence, payload) in data.chunks(chunk_size).enumerate() {
        let bytes = MediaPacket {
            kind,
            turn_id: id,
            sequence: sequence as u32,
            payload,
        }
        .encode()?;
        ws.send(Message::Binary(bytes.into())).await?;
        if kind == MediaKind::AudioIn {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let base = option("--server").unwrap_or_else(|| "http://127.0.0.1:8119".into());
    let token = match std::env::var("AMIGO_DEVICE_TOKEN") {
        Ok(token) => token,
        Err(_) => {
            let user_token = std::env::var("AMIGO_USER_TOKEN")?;
            let response: serde_json::Value = reqwest::Client::new()
                .post(format!(
                    "{}/api/amigo",
                    base.trim_end_matches('/')
                ))
                .bearer_auth(user_token)
                .json(&serde_json::json!({"name":"Korvo-2 simulator"}))
                .send()
                .await?
                .error_for_status()?
                .json()
                .await?;
            println!(
                "Provisioned device {} (credential kept in this process)",
                response["device"]["id"]
            );
            response["device_token"]
                .as_str()
                .ok_or("missing device credential")?
                .to_owned()
        }
    };
    let pcm = match option("--wav") {
        Some(path) => Some(read_wav(&tokio::fs::read(path).await?)?),
        None => None,
    };
    let jpeg = match option("--jpeg") {
        Some(path) => Some(tokio::fs::read(path).await?),
        None => None,
    };
    if pcm
        .as_ref()
        .is_some_and(|data| data.len() > MAX_AUDIO_BYTES)
    {
        return Err("WAV exceeds 15 seconds".into());
    }
    if jpeg
        .as_ref()
        .is_some_and(|data| data.len() > MAX_IMAGE_BYTES)
    {
        return Err("image exceeds 256 KiB".into());
    }
    let output_dir = option("--output-dir")
        .unwrap_or_else(|| concat!(env!("CARGO_MANIFEST_DIR"), "/../../target/verification/amigo-amigo/simulator-audio").into());
    tokio::fs::create_dir_all(&output_dir).await?;
    let duration: u64 = option("--duration-secs")
        .unwrap_or_else(|| "90".into())
        .parse()?;
    let deadline = Instant::now() + Duration::from_secs(duration.min(3600));
    let url = format!("{}/api/amigo/session", base.trim_end_matches('/'))
        .replacen("https://", "wss://", 1)
        .replacen("http://", "ws://", 1);
    let mut attempts = 0;
    while Instant::now() < deadline {
        let mut request = url.clone().into_client_request()?;
        request
            .headers_mut()
            .insert("Authorization", format!("Bearer {token}").parse()?);
        match connect_async(request).await {
            Ok((mut ws, _)) => {
                attempts = 0;
                let result = run(
                    &mut ws,
                    deadline,
                    pcm.as_deref(),
                    jpeg.as_deref(),
                    &output_dir,
                )
                .await;
                let _ = ws.close(None).await;
                if let Err(error) = result {
                    eprintln!("Connection ended: {error}");
                }
            }
            Err(error) => {
                attempts += 1;
                eprintln!("Connection attempt {attempts} failed: {error}");
                if attempts >= 30 {
                    return Err(error.into());
                }
            }
        }
        if Instant::now() < deadline {
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    }
    Ok(())
}

async fn run(
    ws: &mut Socket,
    deadline: Instant,
    pcm: Option<&[u8]>,
    jpeg: Option<&[u8]>,
    output_dir: &str,
) -> Result<()> {
    control(
        ws,
        ClientMessage::Hello {
            version: VERSION,
            board: "simulator".into(),
            sample_rate_hz: 16000,
            channels: 1,
            audio: None,
            streaming: false,
            service_status: false,
        },
    )
    .await?;
    let mut heartbeat = tokio::time::interval(Duration::from_secs(10));
    let (mut active, mut output, mut sequence) = (None, Vec::new(), 0);
    let mut last_reply_proactive = false;
    loop {
        tokio::select! {
            _=tokio::time::sleep_until(tokio::time::Instant::from_std(deadline))=>return Ok(()),
            _=heartbeat.tick()=>control(ws,ClientMessage::Ping).await?,
            message=ws.next()=>{
                let message=message.ok_or("server closed")??;
                if message.is_close() {return Ok(());}
                if message.is_binary() {
                    let bytes=message.into_data();
                    let packet=MediaPacket::decode(&bytes)?;
                    if packet.kind!=MediaKind::AudioOut || active!=Some(packet.turn_id)
                        || packet.sequence!=sequence || output.len()+packet.payload.len()>MAX_REPLY_BYTES {
                        return Err("invalid speaker media sequence or size".into());
                    }
                    output.extend_from_slice(packet.payload);sequence+=1;
                } else if message.is_text() {
                    let event:ServerMessage=serde_json::from_str(message.to_text()?)?;
                    match event {
                        ServerMessage::Welcome {test_mode,..}=>{
                            println!("Connected. test_mode={test_mode}; true means test fixtures, not real recognition/speech.");
                            if let Some(pcm)=pcm {
                                let trigger=if option("--trigger").as_deref()==Some("wake_word") {Trigger::WakeWord} else {Trigger::Button};
                                control(ws,ClientMessage::Wake {turn_id:1,trigger}).await?;
                                upload(ws,MediaKind::AudioIn,1,pcm).await?;
                                control(ws,ClientMessage::EndTurn {turn_id:1}).await?;
                            } else if let Some(text)=option("--text") {
                                control(ws,ClientMessage::Text {turn_id:1,text}).await?;
                            }
                        }
                        ServerMessage::Reply {turn_id,proactive,text,hint_zh,targets,..}=>{
                            println!("{}: {text}\nHint: {hint_zh}; targets: {targets:?}",if proactive {"Robot starts a conversation"} else {"Robot"});
                            active=Some(turn_id);output.clear();sequence=0;last_reply_proactive=proactive;
                        }
                        ServerMessage::AudioEnd {turn_id,bytes}=>{
                            if active!=Some(turn_id)||bytes!=output.len() {return Err("reply length mismatch".into());}
                            let path=format!("{output_dir}/reply-{turn_id}.wav");
                            tokio::fs::write(&path,make_wav(&output)).await?;
                            println!("Received {} bytes PCM; saved {path}",output.len());
                            control(ws,ClientMessage::PlaybackDone {turn_id}).await?;
                            if last_reply_proactive {control(ws,ClientMessage::Feedback {turn_id,accepted:true}).await?;}
                            active=None;output.clear();
                        }
                        ServerMessage::Capture {request_id,source,..}=>{
                            println!("Capture requested: {source:?}. Real device should show its capture indicator.");
                            let data=if source==Source::Speech {pcm.map(|pcm| &pcm[..pcm.len().min(96000)])} else {jpeg};
                            if let Some(data)=data {
                                control(ws,ClientMessage::ObservationStart {request_id,source,bytes:data.len()}).await?;
                                upload(ws,MediaKind::Observation,request_id,data).await?;
                            } else if let Some(text)=option("--observation-text") {
                                println!("Using explicit text fixture instead of camera/microphone capture.");
                                control(ws,ClientMessage::ObservationText {request_id,source,text}).await?;
                            } else {control(ws,ClientMessage::CaptureUnavailable {request_id}).await?;}
                        }
                        ServerMessage::Transcript {text,..}=>println!("You: {text}"),
                        ServerMessage::Error {code,message,..}=>eprintln!("Gateway {code}: {message}"),
                        ServerMessage::Cancelled {turn_id}=>{if active==Some(turn_id){active=None;output.clear();}},
                        ServerMessage::ObservationStored {request_id}=>println!("Observation {request_id} stored"),
                        _=>{}
                    }
                }
            }
        }
    }
}

fn read_wav(bytes: &[u8]) -> Result<Vec<u8>> {
    if bytes.len() < 12 || &bytes[..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return Err("expected WAV".into());
    }
    let (mut offset, mut valid) = (12, false);
    while offset + 8 <= bytes.len() {
        let size = u32::from_le_bytes(bytes[offset + 4..offset + 8].try_into()?) as usize;
        let end = offset + 8 + size;
        if end > bytes.len() {
            return Err("truncated WAV".into());
        }
        let chunk = &bytes[offset + 8..end];
        if &bytes[offset..offset + 4] == b"fmt " && size >= 16 {
            valid = u16::from_le_bytes(chunk[..2].try_into()?) == 1
                && u16::from_le_bytes(chunk[2..4].try_into()?) == 1
                && u32::from_le_bytes(chunk[4..8].try_into()?) == 16000
                && u16::from_le_bytes(chunk[14..16].try_into()?) == 16;
        }
        if &bytes[offset..offset + 4] == b"data" {
            if !valid || chunk.is_empty() || !chunk.len().is_multiple_of(2) {
                return Err("WAV must be PCM16 mono 16000Hz".into());
            }
            return Ok(chunk.to_vec());
        }
        offset = end + (size % 2);
    }
    Err("WAV has no data".into())
}

fn make_wav(pcm: &[u8]) -> Vec<u8> {
    let mut output = b"RIFF".to_vec();
    output.extend_from_slice(&(36 + pcm.len() as u32).to_le_bytes());
    output
        .extend_from_slice(b"WAVEfmt \x10\0\0\0\x01\0\x01\0\x80\x3e\0\0\0\x7d\0\0\x02\0\x10\0data");
    output.extend_from_slice(&(pcm.len() as u32).to_le_bytes());
    output.extend_from_slice(pcm);
    output
}

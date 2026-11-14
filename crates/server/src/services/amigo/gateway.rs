use std::sync::Arc;
use std::time::{Duration, Instant};

use amigo::protocol::*;
use chrono::Utc;
use salvo::prelude::*;
use salvo::websocket::{Message, WebSocket, WebSocketUpgrade};
use tokio::sync::{mpsc, oneshot};
use tokio::task::JoinHandle;
use uuid::Uuid;
use zeroize::Zeroizing;

use super::agent::{Agent, Route};
use super::model::{Analysis, Context, Dialogue, Plan, VisualContext, Weakness, clean};
use super::store::{self, Device};
use super::tutor::{Backend, ProductionBackend};
use crate::AppResult;

#[derive(Clone, Default)]
pub struct DeviceGateway {
    pub backend: Option<Arc<dyn Backend>>,
}

#[handler]
impl DeviceGateway {
    async fn handle(&self, req: &mut Request, res: &mut Response) -> AppResult<()> {
        let token = req
            .headers()
            .get("authorization")
            .and_then(|h| h.to_str().ok())
            .and_then(|h| h.strip_prefix("Bearer "))
            .filter(|s| (s.starts_with("am_") || s.starts_with("lb_")) && s.len() < 128);
        let hardware = token.is_some();
        let device = if let Some(token) = token {
            store::authenticate(token).await
        } else {
            let ticket = req
                .headers()
                .get("sec-websocket-protocol")
                .and_then(|h| h.to_str().ok())
                .and_then(|h| {
                    h.split(',').map(str::trim).find(|s| {
                        (s.starts_with("amws_") || s.starts_with("lbws_")) && s.len() < 128
                    })
                });
            match ticket {
                Some(ticket) => store::consume_ticket(ticket).await,
                None => Ok(None),
            }
        }
        .map_err(|_| StatusError::service_unavailable())?
        .ok_or_else(StatusError::unauthorized)?;
        let backend = match &self.backend {
            Some(backend) => backend.clone(),
            None => ProductionBackend::for_device(&device)
                .await
                .unwrap_or_else(|_| Arc::new(super::tutor::UnavailableBackend)),
        };
        let lease = Uuid::new_v4();
        let claimed = if hardware {
            store::reconnect(device.id, lease).await
        } else {
            store::claim(device.id, lease).await
        };
        if !claimed.map_err(|_| StatusError::service_unavailable())? {
            return Err(StatusError::conflict()
                .brief("device already connected")
                .into());
        }
        let device_id = device.id;
        if req
            .headers()
            .get("sec-websocket-protocol")
            .and_then(|h| h.to_str().ok())
            .is_some_and(|h| h.split(',').any(|p| p.trim() == "amigo.v1"))
        {
            res.add_header("Sec-WebSocket-Protocol", "amigo.v1", true)?;
        }
        let result = WebSocketUpgrade::new().write_buffer_size(0)
            .max_message_size(65535 + HEADER_BYTES).max_frame_size(65535 + HEADER_BYTES)
            .max_write_buffer_size(128 * 1024)
            .upgrade(req,res,move |ws| async move {
                tracing::info!(%device_id,"Amigo device connected");
                let mut connection = Connection::new(device,lease,backend);
                if let Err(error) = connection.run(ws).await {
                    tracing::info!(device_id = %device_id, reason = %error, "Amigo connection closed");
                }
                connection.abort();
                let _ = store::release(device_id,lease).await;
            }).await;
        if result.is_err() {
            let _ = store::release(device_id, lease).await;
        }
        result.map_err(Into::into)
    }
}

struct Recording {
    turn: u64,
    sequence: u32,
    pcm: Zeroizing<Vec<u8>>,
    started: Instant,
    asr: Option<crate::services::asr_stream::PcmAsrStream>,
    partial: mpsc::Receiver<crate::services::asr_stream::AsrPartial>,
    live: Option<mpsc::Sender<Option<Zeroizing<Vec<u8>>>>>,
}
struct Capture {
    id: u64,
    source: Source,
    total: Option<usize>,
    sequence: u32,
    data: Zeroizing<Vec<u8>>,
    started: Instant,
    response: Option<oneshot::Sender<Option<Zeroizing<Vec<u8>>>>>,
}
struct Playback {
    turn: u64,
    pcm: Vec<u8>,
    sequence: u32,
    offset: usize,
    ended: bool,
    proactive: bool,
    started: Instant,
    complete: bool,
    pending_record: Option<Box<ReplyRecord>>,
}

struct ReplyRecord {
    event: Uuid,
    transcript: String,
    analysis: Analysis,
    plan: Plan,
    assigned: Vec<Weakness>,
    new_agent: Option<Agent>,
    visual: Option<Context>,
    after_playback: bool,
    origin_agent: String,
}

enum JobOutput {
    StreamFinished,
    NoSpeech {
        turn: u64,
    },
    Reply {
        turn: u64,
        record: Box<ReplyRecord>,
        pcm: Vec<u8>,
        streamed: bool,
    },
    Observation {
        request: u64,
        event: Uuid,
        context: Context,
        analysis: Box<Analysis>,
    },
}
enum StreamEvent {
    Learning {
        turn: u64,
        message: ServerMessage,
    },
    Capture {
        turn: u64,
        response: oneshot::Sender<Option<Zeroizing<Vec<u8>>>>,
    },
    VisionStatus {
        turn: u64,
        state: &'static str,
    },
    Transcript {
        turn: u64,
        text: String,
    },
    Start {
        turn: u64,
        reply: ServerMessage,
    },
    Audio {
        turn: u64,
        pcm: Vec<u8>,
    },
    Final {
        turn: u64,
        output: Box<JobOutput>,
    },
}
impl StreamEvent {
    fn turn(&self) -> u64 {
        match self {
            Self::Learning { turn, .. }
            | Self::Capture { turn, .. }
            | Self::VisionStatus { turn, .. }
            | Self::Transcript { turn, .. }
            | Self::Start { turn, .. }
            | Self::Audio { turn, .. }
            | Self::Final { turn, .. } => *turn,
        }
    }
}

struct Connection {
    streaming: bool,
    service_status: bool,
    events_tx: mpsc::Sender<StreamEvent>,
    events_rx: mpsc::Receiver<StreamEvent>,
    audio_codec: AudioCodec,
    opus: super::codec::OpusWire,
    device: Device,
    lease: Uuid,
    backend: Arc<dyn Backend>,
    realtime_mode: bool,
    realtime_config: Option<super::live_voice::Config>,
    realtime_channel: Arc<super::live_voice::Channel>,
    recording: Option<Recording>,
    capture: Option<Capture>,
    playback: Option<Playback>,
    job: Option<JoinHandle<Result<JobOutput, String>>>,
    job_turn: Option<u64>,
    job_proactive: bool,
    last_client_turn: u64,
    next_server_id: u64,
    last_activity: Instant,
    last_heartbeat: Instant,
    last_lease_refresh: Instant,
    last_capture_source: Source,
    busy: bool,
    muted: bool,
    camera_available: Option<bool>,
    feedback: Option<(u64, Uuid)>,
    control_window: Instant,
    control_count: u32,
}

impl Drop for Connection {
    fn drop(&mut self) {
        self.abort();
    }
}

impl Connection {
    fn new(device: Device, lease: Uuid, backend: Arc<dyn Backend>) -> Self {
        let (events_tx, events_rx) = mpsc::channel(1);
        Self {
            streaming: false,
            service_status: false,
            events_tx,
            events_rx,
            device,
            audio_codec: AudioCodec::Pcm16,
            opus: super::codec::OpusWire::default(),
            lease,
            backend,
            realtime_mode: false,
            realtime_config: None,
            realtime_channel: Arc::new(super::live_voice::Channel::default()),
            recording: None,
            capture: None,
            playback: None,
            job: None,
            job_turn: None,
            job_proactive: false,
            last_client_turn: 0,
            next_server_id: SERVER_TURN_START,
            last_activity: Instant::now(),
            last_heartbeat: Instant::now(),
            last_lease_refresh: Instant::now(),
            last_capture_source: Source::Speech,
            busy: false,
            muted: false,
            camera_available: None,
            feedback: None,
            control_window: Instant::now(),
            control_count: 0,
        }
    }

    fn abort(&mut self) {
        self.opus.clear_encoder();
        self.opus.clear_decoder();
        if let Some(job) = self.job.take() {
            job.abort();
        }
        self.job_turn = None;
        self.job_proactive = false;
        self.recording = None;
        self.capture = None;
        self.playback = None;
        while self.events_rx.try_recv().is_ok() {}
    }

    fn idle(&self) -> bool {
        self.recording.is_none()
            && self.job.is_none()
            && self.playback.is_none()
            && self.capture.is_none()
    }

    fn ready(&self) -> bool {
        if self.realtime_mode { self.realtime_config.is_some() } else { self.backend.ready() }
    }

    async fn refresh_voice(&mut self) -> Result<(), String> {
        let settings = super::devices::load(self.device.clone()).await?.settings;
        self.realtime_mode = settings.conversation_mode == super::model::ConversationMode::Realtime;
        self.realtime_config = if self.realtime_mode {
            super::live_voice::Config::for_device(&self.device, &settings).await.ok()
        } else { None };
        Ok(())
    }

    fn server_id(&mut self) -> u64 {
        self.next_server_id += 1;
        self.next_server_id
    }

    async fn sync_configuration(
        &mut self,
        ws: &mut WebSocket,
        sent: &mut Option<i64>,
    ) -> Result<(), String> {
        if let Some(c) = super::devices::configuration(self.device.id).await? {
            if *sent != Some(c.revision) {
                let p = super::devices::load(self.device.clone()).await?;
                send(
                    ws,
                    &ServerMessage::Configuration {
                        revision: c.revision,
                        volume: c.volume,
                        continuous: c.continuous,
                        voice_interrupt: c.voice_interrupt,
                        playback_wake: c.playback_wake,
                        camera_enabled: p.settings.camera_enabled,
                        ambient_enabled: p.settings.ambient_audio_enabled,
                    },
                )
                .await?;
                *sent = Some(c.revision);
            }
        }
        Ok(())
    }

    async fn run(&mut self, mut ws: WebSocket) -> Result<(), String> {
        let first = tokio::time::timeout(Duration::from_secs(10), ws.recv())
            .await
            .map_err(|_| "hello timeout")?
            .ok_or("disconnected")?
            .map_err(|_| "invalid websocket")?;
        if !first.is_text() || first.as_bytes().len() > MAX_CONTROL_BYTES {
            return Err("expected hello".into());
        }
        let hello: ClientMessage =
            serde_json::from_slice(first.as_bytes()).map_err(|_| "invalid hello")?;
        let offer = match hello {
            ClientMessage::Hello {
                version: VERSION,
                board,
                sample_rate_hz: 16000,
                channels: 1,
                audio,
                streaming,
                service_status,
            } if supported_board(&board) => {
                self.service_status = service_status;
                super::devices::board(self.device.id, board).await?;
                self.streaming = streaming
                    && !matches!(
                        crate::services::minimax::setting("AMIGO_STREAMING_ENABLED").as_deref(),
                        Some("false" | "0")
                    );
                audio
            }
            _ => {
                send(
                    &mut ws,
                    &error(
                        None,
                        "incompatible",
                        "expected protocol 1, supported Amigo board, mono 16kHz",
                    ),
                )
                .await?;
                return Ok(());
            }
        };
        let opus_enabled = !matches!(
            std::env::var("AMIGO_OPUS_ENABLED").as_deref(),
            Ok("false" | "0")
        );
        self.audio_codec = match negotiate_audio(offer.as_ref(), opus_enabled) {
            Ok(codec) => codec,
            Err(problem) => {
                send(&mut ws, &error(None, "incompatible_audio", problem)).await?;
                return Ok(());
            }
        };
        send(
            &mut ws,
            &ServerMessage::Welcome {
                session_id: self.lease.to_string(),
                version: VERSION,
                sample_rate_hz: 16000,
                frame_ms: 20,
                test_mode: self.backend.test_mode(),
                audio: offer.as_ref().map(|_| self.audio_codec),
            },
        )
        .await?;
        self.refresh_voice().await?;
        if self.service_status {
            send(
                &mut ws,
                &ServerMessage::ServiceState {
                    ready: self.ready(),
                },
            )
            .await?;
        }
        let mut sent_revision = None;
        self.sync_configuration(&mut ws, &mut sent_revision).await?;
        let mut tick = tokio::time::interval(Duration::from_secs(1));
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        let mut audio_tick = tokio::time::interval(Duration::from_millis(20));
        audio_tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            tokio::select! {
                message = ws.recv() => {
                    let Some(Ok(message)) = message else { break; };
                    if message.is_close() { break; }
                    self.last_heartbeat = Instant::now();
                    if message.is_ping() { continue; }
                    if message.is_binary() {
                        if let Err(problem) = self.media(message.as_bytes()).await {
                            send(&mut ws,&error(None,"invalid_media",&problem)).await?;
                            return Ok(());
                        }
                    } else if message.is_text() {
                        if self.control_window.elapsed() > Duration::from_secs(60) {
                            self.control_count = 0; self.control_window = Instant::now();
                        }
                        self.control_count += 1;
                        if self.control_count > 120 { return Err("control rate limit".into()); }
                        if message.as_bytes().len() > MAX_CONTROL_BYTES { return Err("control size limit".into()); }
                        match serde_json::from_slice::<ClientMessage>(message.as_bytes()) {
                            Ok(control) => if let Err(problem) = self.control(control,&mut ws).await {
                                send(&mut ws,&error(None,"invalid_state",&problem)).await?;
                            },
                            Err(_) => { send(&mut ws,&error(None,"invalid_json","invalid device control message")).await?; }
                        }
                    }
                },
                output = async { self.job.as_mut().unwrap().await }, if self.job.is_some() => {
                    self.job = None;
                    let turn = self.job_turn.take(); self.job_proactive = false;
                    match output {
                        Ok(Ok(output)) => self.complete(output,&mut ws).await?,
                        Ok(Err(problem)) => {
                            self.abort();
                            tracing::warn!(%problem, "Amigo pipeline failed");
                            send(&mut ws,&error(turn,"pipeline_failed","ASR, vision, tutor or TTS failed or timed out")).await?;
                        }
                        Err(problem) => {
                            self.abort();
                            tracing::warn!(%problem, "Amigo pipeline task failed");
                            send(&mut ws,&error(turn,"pipeline_failed","ASR, vision, tutor or TTS failed or timed out")).await?;
                        }
                    }
                },
                partial = async { self.recording.as_mut().unwrap().partial.recv().await }, if self.recording.as_ref().is_some_and(|r|r.asr.is_some()) => {
                    if let Some(update)=partial {
                        if self.streaming {let turn=self.recording.as_ref().unwrap().turn;send(&mut ws,&ServerMessage::TranscriptPartial{turn_id:turn,text:clean(&update.text,2000),utterance_complete:update.utterance_complete}).await?;}
                    } else if let Some(recording)=&mut self.recording {recording.asr=None;}
                },
                event = self.events_rx.recv(), if self.playback.as_ref().is_none_or(|p|p.pcm.len()-p.offset<64000) => {
                    if let Some(event)=event {
                        let turn=event.turn();
                        if self.job_turn==Some(turn) || self.playback.as_ref().is_some_and(|p|p.turn==turn) {
                            self.stream_event(event,&mut ws).await?;
                        }
                    }
                },
                _ = audio_tick.tick(), if self.playback.as_ref().is_some_and(|p| !p.ended) => self.send_audio(&mut ws).await?,
                _ = tick.tick() => {
                    if self.last_heartbeat.elapsed() > Duration::from_secs(60) { return Err("heartbeat timeout".into()); }
                    if self.last_lease_refresh.elapsed() >= Duration::from_secs(5) {
                        if !store::refresh(self.device.id,self.lease).await? { return Err("device revoked or lease lost".into()); }
                        // A long-lived hardware connection must not outlive AI access
                        // or keep using a provider that the owner has deactivated.
                        if !self.backend.test_mode() {
                            let next = ProductionBackend::for_device(&self.device).await
                                .unwrap_or_else(|_| Arc::new(super::tutor::UnavailableBackend));
                            let was_ready = self.ready();
                            self.backend = next;
                            self.refresh_voice().await?;
                            if self.ready() != was_ready {
                                if !self.ready() {
                                    self.abort();
                                    if !self.service_status {send(&mut ws,&error(None,"ai_setup_required","Configure Chat, ASR and TTS in your website account")).await?;}
                                }
                                if self.service_status {send(&mut ws,&ServerMessage::ServiceState{ready:self.ready()}).await?;}
                            }
                        }
                        self.last_lease_refresh = Instant::now();
                        self.sync_configuration(&mut ws,&mut sent_revision).await?;
                    }
                    if self.recording.as_ref().is_some_and(|r| r.started.elapsed() > Duration::from_secs(20)) {
                        let turn = self.recording.take().unwrap().turn;
                        send(&mut ws,&error(Some(turn),"recording_timeout","recording exceeded deadline")).await?;
                    }
                    if self.capture.as_ref().is_some_and(|c| c.started.elapsed() > Duration::from_secs(if c.response.is_some() { 8 } else { 15 })) { self.capture = None; }
                    if self.playback.as_ref().is_some_and(|p| p.started.elapsed() > Duration::from_secs(45)) {
                        let turn = self.playback.take().unwrap().turn;
                        send(&mut ws,&ServerMessage::Cancelled { turn_id:turn }).await?;
                    }
                    self.schedule(&mut ws).await?;
                }
            }
        }
        Ok(())
    }

    async fn control(&mut self, message: ClientMessage, ws: &mut WebSocket) -> Result<(), String> {
        if !self.ready()
            && matches!(
                message,
                ClientMessage::Wake { .. }
                    | ClientMessage::Text { .. }
                    | ClientMessage::ObserveNow { .. }
            )
        {
            let turn = match &message {
                ClientMessage::Wake { turn_id, .. } | ClientMessage::Text { turn_id, .. } => {
                    Some(*turn_id)
                }
                _ => None,
            };
            send(
                ws,
                &error(
                    turn,
                    "ai_setup_required",
                    "Configure Chat, ASR and TTS in your website account",
                ),
            )
            .await?;
            return Ok(());
        }
        match message {
            ClientMessage::CameraState { enabled } => {
                self.camera_available = Some(enabled);
                if !enabled
                    && self
                        .capture
                        .as_ref()
                        .is_some_and(|capture| capture.response.is_some())
                {
                    self.capture = None;
                }
            }
            ClientMessage::ConfigurationApplied { revision } => {
                super::devices::applied(self.device.id, revision).await?;
            }
            ClientMessage::Ping => send(ws, &ServerMessage::Pong).await?,
            ClientMessage::ObserveNow { source } => {
                if !self.idle() || self.busy || self.muted {
                    return Err("请先结束当前对话或取消暂停".into());
                }
                let settings = super::devices::load(self.device.clone()).await?.settings;
                if (source == Source::Speech && !settings.ambient_audio_enabled)
                    || (source != Source::Speech && !settings.camera_enabled)
                {
                    return Err("请先在观察设置中允许这项观察".into());
                }
                super::devices::update(self.device.clone(), |profile| {
                    profile.last_observation_at = Some(Utc::now())
                })
                .await?;
                self.begin_capture(source, ws).await?;
            }
            ClientMessage::Hello { .. } => return Err("hello already completed".into()),
            ClientMessage::Presence { busy, muted } => {
                self.busy = busy;
                self.muted = muted;
                if busy || muted {
                    if self.job_proactive
                        || self.playback.as_ref().is_some_and(|p| p.proactive)
                        || muted
                    {
                        let turn = self
                            .job_turn
                            .or_else(|| self.playback.as_ref().map(|p| p.turn))
                            .or_else(|| self.recording.as_ref().map(|r| r.turn));
                        self.abort();
                        if let Some(turn) = turn {
                            send(ws, &ServerMessage::Cancelled { turn_id: turn }).await?;
                        }
                    }
                    self.capture = None;
                }
            }
            ClientMessage::Wake { turn_id, .. } => {
                self.prepare_turn(turn_id, ws).await?;
                let (tx, partial) = mpsc::channel(8);
                let (asr, live) = if self.realtime_mode {
                    let (input, receive) = mpsc::channel(800);
                    self.start_live(turn_id, receive).await?;
                    (None, Some(input))
                } else { (self.backend.start_asr(tx), None) };
                self.recording = Some(Recording {
                    turn: turn_id,
                    sequence: 0,
                    pcm: Zeroizing::new(Vec::new()),
                    started: Instant::now(),
                    asr,
                    partial,
                    live,
                });
                send(
                    ws,
                    &ServerMessage::Ack {
                        turn_id,
                        state: "recording".into(),
                    },
                )
                .await?;
            }
            ClientMessage::Text { turn_id, text } => {
                if text.trim().is_empty() || text.len() > 8000 {
                    return Err("text must be 1-8000 bytes".into());
                }
                self.prepare_turn(turn_id, ws).await?;
                self.start_reply(turn_id, Some(clean(&text, 2000)), None, false, None)
                    .await?;
                send(
                    ws,
                    &ServerMessage::Ack {
                        turn_id,
                        state: "processing".into(),
                    },
                )
                .await?;
            }
            ClientMessage::EndTurn { turn_id } => {
                let recording = self.recording.as_ref().ok_or("not recording")?;
                if recording.turn != turn_id {
                    return Err("wrong recording turn".into());
                }
                if recording.pcm.len() < 3200 {
                    return Err("record at least 100ms".into());
                }
                let recording = self.recording.take().unwrap();
                self.opus.clear_decoder();
                if let Some(live) = recording.live {
                    live.try_send(None).map_err(|_| "Realtime 输入队列已关闭或已满")?;
                } else {
                    self.start_reply(turn_id, None, Some(recording.pcm), false, recording.asr).await?;
                }
                send(
                    ws,
                    &ServerMessage::Ack {
                        turn_id,
                        state: "processing".into(),
                    },
                )
                .await?;
            }
            ClientMessage::Cancel { turn_id } => {
                let matches = self.recording.as_ref().is_some_and(|r| r.turn == turn_id)
                    || self.job_turn == Some(turn_id)
                    || self.playback.as_ref().is_some_and(|p| p.turn == turn_id)
                    || self.capture.as_ref().is_some_and(|c| c.id == turn_id);
                if matches {
                    self.abort();
                    self.last_activity = Instant::now();
                }
                send(ws, &ServerMessage::Cancelled { turn_id }).await?;
            }
            ClientMessage::PlaybackDone { turn_id } => {
                let Some(playback) = self.playback.as_ref().filter(|p| p.turn == turn_id) else {
                    return Ok(()); // A late drain of an interrupted epoch owns no state.
                };
                if !playback.ended {
                    return Err("playback is not complete".into());
                }
                let playback = self.playback.take().unwrap();
                self.last_activity = Instant::now();
                if let Some(record) = playback.pending_record {
                    self.record_reply(turn_id, &record, ws).await?;
                }
                send(
                    ws,
                    &ServerMessage::Ack {
                        turn_id,
                        state: "ready".into(),
                    },
                )
                .await?;
            }
            ClientMessage::ObservationStart {
                request_id,
                source,
                bytes,
            } => {
                if request_id <= self.last_client_turn
                    && self.capture.as_ref().is_none_or(|c| c.id != request_id)
                {
                    return Ok(()); // A cancelled dialogue's late photo cannot affect a new turn.
                }
                let capture = self.capture.as_mut().ok_or("no capture requested")?;
                let limit = if source == Source::Speech {
                    16000 * 2 * 3
                } else {
                    MAX_IMAGE_BYTES
                };
                if capture.id != request_id
                    || capture.source != source
                    || capture.total.is_some()
                    || bytes == 0
                    || bytes > limit
                    || (source == Source::Speech && !bytes.is_multiple_of(2))
                {
                    return Err("capture does not match request or media limit".into());
                }
                capture.total = Some(bytes);
            }
            ClientMessage::ObservationText {
                request_id,
                source,
                text,
            } => {
                if text.trim().is_empty() || text.len() > 8000 {
                    return Err("invalid observation text".into());
                }
                let capture = self.capture.as_ref().ok_or("no capture requested")?;
                if capture.id != request_id || capture.source != source {
                    return Err("wrong capture request".into());
                }
                // Text-capable adapters can report already extracted text. This
                // is context, never learner recall or evidence of an error.
                self.capture = None;
                let context = Context {
                    source: source_name(source).into(),
                    text: clean(&text, 2000),
                    topics: vec![],
                    observed_at: Utc::now(),
                };
                self.commit_observation(
                    request_id,
                    Uuid::new_v4(),
                    context,
                    Analysis::default(),
                    ws,
                )
                .await?;
            }
            ClientMessage::CaptureUnavailable { request_id } => {
                if self.capture.as_ref().is_some_and(|c| c.id == request_id) {
                    self.capture = None;
                }
            }
            ClientMessage::Feedback { turn_id, accepted } => {
                let (id, event) = self.feedback.ok_or("no proactive feedback pending")?;
                if id != turn_id {
                    return Err("feedback belongs to another turn".into());
                }
                let key = Uuid::new_v5(&event, b"feedback");
                store::record(
                    self.device.clone(),
                    self.lease,
                    key,
                    "feedback",
                    serde_json::json!({"accepted":accepted}),
                    move |profile| {
                        profile.dismissals = if accepted {
                            profile.dismissals.saturating_sub(1)
                        } else {
                            profile.dismissals.saturating_add(1).min(3)
                        };
                        if !accepted
                            && let Some(state) =
                                profile.agent_states.get_mut(&profile.active_agent_id)
                            && state.learning.last_event == Some(event)
                            && state.learning.mode != "conversation"
                        {
                            let learning = &mut state.learning;
                            learning.id = Uuid::new_v4().to_string();
                            learning.mode = "conversation".into();
                            learning.stage = "converse".into();
                            learning.goal.clear();
                            learning.scene.clear();
                            learning.strategy.clear();
                            learning.targets.clear();
                            learning.task = None;
                            learning.practice_attempts = 0;
                            learning.awaiting_choice = false;
                            learning.practice_ended_at = Some(Utc::now());
                            learning.revision += 1;
                            learning.last_event = Some(key);
                        }
                    },
                )
                .await?;
                self.feedback = None;
                if !accepted {
                    self.abort();
                    send(ws, &ServerMessage::Cancelled { turn_id }).await?;
                }
            }
        }
        Ok(())
    }

    async fn prepare_turn(&mut self, turn: u64, ws: &mut WebSocket) -> Result<(), String> {
        if turn == 0 || turn >= SERVER_TURN_START || turn <= self.last_client_turn {
            return Err("turn IDs must increase within the connection".into());
        }
        if self.muted {
            return Err("microphone and playback are muted".into());
        }
        if self.job_turn.is_some_and(|turn| turn >= SERVER_TURN_START)
            || self.playback.as_ref().is_some_and(|p| p.proactive)
        {
            let old = self
                .job_turn
                .or_else(|| self.playback.as_ref().map(|p| p.turn));
            self.abort();
            if let Some(turn_id) = old {
                send(ws, &ServerMessage::Cancelled { turn_id }).await?;
            }
        }
        self.capture = None;
        if !self.idle() {
            return Err("device is busy; cancel current turn first".into());
        }
        self.last_client_turn = turn;
        self.last_activity = Instant::now();
        Ok(())
    }

    async fn media(&mut self, bytes: &[u8]) -> Result<(), String> {
        let packet = MediaPacket::decode(bytes).map_err(str::to_owned)?;
        match packet.kind {
            MediaKind::AudioIn | MediaKind::OpusIn => {
                if (packet.kind == MediaKind::OpusIn) != (self.audio_codec == AudioCodec::Opus) {
                    return Err("audio codec does not match negotiation".into());
                }
                let recording = self.recording.as_mut().ok_or("audio without wake")?;
                if packet.turn_id != recording.turn || packet.sequence != recording.sequence {
                    return Err("invalid audio sequence or recording too long".into());
                }
                let decoded;
                let pcm = if self.audio_codec == AudioCodec::Opus {
                    if opus_payload(packet.payload).map_err(str::to_owned)?.0 != 320 {
                        return Err("uplink Opus must contain 320 valid samples".into());
                    }
                    decoded = self.opus.decode(recording.turn, packet.payload)?;
                    &decoded
                } else {
                    packet.payload
                };
                let limit = MAX_AUDIO_BYTES
                    + if self.audio_codec == AudioCodec::Opus {
                        CODEC_TAIL_BYTES
                    } else {
                        0
                    };
                if recording.pcm.len() + pcm.len() > limit {
                    return Err("decoded recording exceeds limit".into());
                }
                recording.pcm.extend_from_slice(pcm);
                if let Some(live) = &recording.live {
                    live.try_send(Some(Zeroizing::new(pcm.to_vec())))
                        .map_err(|_| "Realtime 输入队列已关闭或已满")?;
                }
                if let Some(asr) = &recording.asr
                    && asr.audio(pcm).is_err()
                {
                    recording.asr = None;
                    tracing::warn!(
                        turn_id = recording.turn,
                        "Amigo streaming ASR unavailable; using buffered fallback"
                    );
                }
                recording.sequence += 1;
            }
            MediaKind::Observation => {
                if packet.turn_id <= self.last_client_turn
                    && self.capture.as_ref().is_none_or(|c| c.id != packet.turn_id)
                {
                    return Ok(());
                }
                let capture = self
                    .capture
                    .as_mut()
                    .ok_or("unsolicited observation media")?;
                let total = capture.total.ok_or("send observation_start first")?;
                if packet.turn_id != capture.id
                    || packet.sequence != capture.sequence
                    || capture.data.len() + packet.payload.len() > total
                {
                    return Err("invalid observation sequence or length".into());
                }
                capture.data.extend_from_slice(packet.payload);
                capture.sequence += 1;
                if capture.data.len() == total {
                    let mut capture = self.capture.take().unwrap();
                    if let Some(response) = capture.response.take() {
                        let _ = response.send(Some(capture.data));
                        return Ok(());
                    }
                    if self.job.is_some() {
                        return Err("model pipeline busy".into());
                    }
                    let backend = super::calls::traced(self.backend.clone(),self.device.clone(),capture.id);
                    self.job_turn = Some(capture.id);
                    self.job_proactive = false;
                    self.job = Some(tokio::spawn(async move {
                        tokio::time::timeout(Duration::from_secs(90), async move {
                            let (text, analysis) =
                                backend.observe(capture.source, &capture.data).await?;
                            let context = Context {
                                source: source_name(capture.source).into(),
                                text: clean(&text, 2000),
                                topics: analysis.topics.clone(),
                                observed_at: Utc::now(),
                            };
                            Ok(JobOutput::Observation {
                                request: capture.id,
                                event: Uuid::new_v4(),
                                context,
                                analysis: Box::new(analysis),
                            })
                        })
                        .await
                        .map_err(|_| "observation timeout".to_owned())?
                    }));
                }
            }
            MediaKind::AudioOut | MediaKind::OpusOut => {
                return Err("device cannot upload speaker audio".into());
            }
        }
        Ok(())
    }

    async fn start_live(
        &mut self,
        turn: u64,
        input: mpsc::Receiver<Option<Zeroizing<Vec<u8>>>>,
    ) -> Result<(), String> {
        let config = self.realtime_config.clone().ok_or("请配置 OpenAI 实时语音服务")?;
        let profile = super::devices::load(self.device.clone()).await?;
        let plan = profile.plan(Utc::now(), false);
        let channel = self.realtime_channel.clone();
        let events = self.events_tx.clone();
        let device = self.device.clone();
        self.job_turn = Some(turn);
        self.job_proactive = false;
        self.job = Some(tokio::spawn(async move {
            let (tx, mut rx) = mpsc::channel(4);
            let generate = super::calls::live(device, turn, &config.model,
                channel.turn(&config, &profile, &plan, input, tx));
            let deliver = async {
                let mut started = false;
                let mut text = String::new();
                while let Some(event) = rx.recv().await {
                    match event {
                        super::live_voice::Event::Transcript(text) => {
                            events.send(StreamEvent::Transcript { turn, text }).await.map_err(|_| "stream cancelled")?;
                        }
                        super::live_voice::Event::Text(reply) => { text = reply; }
                        super::live_voice::Event::Audio(pcm) => {
                            if !started {
                                events.send(StreamEvent::Start { turn, reply: ServerMessage::Reply {
                                    turn_id: turn, proactive: false, text: text.clone(), hint_zh: String::new(),
                                    targets: Vec::new(), agent_id: Some(plan.agent.id.clone()), agent_name: Some(plan.agent.name.clone()),
                                    response_act: Some("respond".into()),
                                }}).await.map_err(|_| "stream cancelled")?;
                                started = true;
                            }
                            events.send(StreamEvent::Audio { turn, pcm }).await.map_err(|_| "stream cancelled")?;
                        }
                    }
                }
                Ok::<_, String>(())
            };
            let (output, _) = tokio::try_join!(generate, deliver)?;
            if output.no_speech { return Ok(JobOutput::NoSpeech { turn }); }
            let origin_agent = profile.active_agent_id.clone();
            let record = Box::new(ReplyRecord {
                event: Uuid::new_v4(), transcript: output.transcript,
                analysis: Analysis { reply_en: output.reply, ..Default::default() },
                plan, assigned: Vec::new(), new_agent: None, visual: None,
                after_playback: true, origin_agent,
            });
            events.send(StreamEvent::Final { turn, output: Box::new(JobOutput::Reply {
                turn, record, pcm: Vec::new(), streamed: true,
            })}).await.map_err(|_| "stream cancelled")?;
            Ok(JobOutput::StreamFinished)
        }));
        Ok(())
    }

    async fn start_reply(
        &mut self,
        turn: u64,
        text: Option<String>,
        pcm: Option<Zeroizing<Vec<u8>>>,
        proactive: bool,
        asr: Option<crate::services::asr_stream::PcmAsrStream>,
    ) -> Result<(), String> {
        let device = self.device.clone();
        let backend = super::calls::traced(self.backend.clone(),device.clone(),turn);
        let streaming = self.streaming;
        let camera_available = self.camera_available;
        let events = self.events_tx.clone();
        self.job_turn = Some(turn);
        self.job_proactive = proactive;
        self.job = Some(tokio::spawn(async move {
            tokio::time::timeout(Duration::from_secs(90), async move {
                let started = Instant::now();
                let from_audio = text.is_none();
                let transcript = match text {
                    Some(text) => text,
                    None if proactive => String::new(),
                    None => {
                        let result = if let Some(asr) = asr {
                            Some(asr.finish().await)
                        } else {
                            None
                        };
                        match result {
                            Some(Ok(text)) => text,
                            fallback => {
                                if fallback.is_some() {
                                    tracing::warn!(
                                        turn_id = turn,
                                        "Amigo ASR final failed; buffered fallback"
                                    );
                                }
                                match backend
                                    .transcribe(
                                        pcm.as_deref()
                                            .map(|p| p.as_slice())
                                            .ok_or("missing audio")?,
                                    )
                                    .await
                                {
                                    Ok(text) => text,
                                    Err(error) if error == "no speech recognized" => String::new(),
                                    Err(error) => return Err(format!("ASR: {error}")),
                                }
                            }
                        }
                    }
                };
                tracing::info!(
                    turn_id = turn,
                    stage = "asr_final",
                    elapsed_ms = started.elapsed().as_millis(),
                    "Amigo stream timing"
                );
                let transcript = clean(&transcript, 2000);
                if !proactive && (transcript.is_empty()
                    || (from_audio && !transcript.chars().any(char::is_alphanumeric))) {
                    return Ok(JobOutput::NoSpeech { turn });
                }
                if streaming && !transcript.is_empty() {
                    events
                        .send(StreamEvent::Transcript {
                            turn,
                            text: transcript.clone(),
                        })
                        .await
                        .map_err(|_| "stream cancelled")?;
                }
                let mut profile = super::devices::load(device.clone()).await?;
                let origin_agent = profile.active_agent_id.clone();
                let new_agent = match profile.route(&transcript) {
                    Route::Talk(id) => {
                        profile.active_agent_id = id;
                        None
                    }
                    Route::Create(agent) => {
                        profile.activate(agent.clone())?;
                        Some(agent)
                    }
                };
                let current = super::workflow::session(&profile, &profile.active_agent_id);
                let routing = async {
                    if streaming {
                        match backend.prepare_turn(&profile, &transcript, proactive, camera_available).await {
                            Ok(Some(prepared)) => {
                                if let Some(decision) = prepared.decision {
                                    return (Ok(Some(decision)), prepared.continuation, true);
                                }
                            }
                            Ok(None) => {}
                            Err(problem) => tracing::warn!(reason=%clean(&problem,240),
                                "Fast routing unavailable; using full workflow planner"),
                        }
                    }
                    let decision = backend.plan_turn(&profile, &transcript, proactive, camera_available).await;
                    (decision, None, false)
                };
                let acoustic = async {
                    let pcm = pcm.as_ref()?;
                    let target = match current.mode.as_str() {
                        "pronunciation" => Some(current.task.as_ref()
                            .filter(|task| task.kind == "pronunciation_repeat")?
                            .target_keys.first()?.as_str()),
                        "conversation" if profile.settings.proactive_enabled => None,
                        _ => return None,
                    };
                    backend.assess_pronunciation(pcm.as_slice(), &transcript, target).await
                };
                let ((decision, continuation, combined), assessment) = tokio::join!(routing, acoustic);
                tracing::info!(turn_id=turn,stage="decision_ready",combined,elapsed_ms=started.elapsed().as_millis(),"Amigo stream timing");
                let mut execution = match decision {
                    Ok(Some(decision)) => {
                        match super::workflow::validate(decision, &current, &transcript, proactive).and_then(|decision| {
                            if proactive { super::workflow::authorize_outreach(&decision, &current, &profile)?; }
                            Ok(decision)
                        })
                        {
                            Ok(mut decision) => {
                                super::workflow::prepare_practice(&mut decision, &current, &transcript, pcm.is_some(), assessment);
                                Some(super::workflow::Execution {
                                    base_revision: current.revision,
                                    decision: Some(decision),
                                    status: "planned".into(),
                                })
                            }
                            Err(problem) => {
                                tracing::warn!(turn_id=turn, reason=%clean(&problem,240), "Learning decision rejected");
                                Some(super::workflow::fallback(&current))
                            }
                        }
                    }
                    Ok(None) if backend.test_mode() => None,
                    Err(problem) => {
                        tracing::warn!(
                            turn_id = turn,
                            reason = %clean(&problem,240),
                            "Learning planner unavailable; preserving current activity"
                        );
                        Some(super::workflow::fallback(&current))
                    }
                    Ok(None) => Some(super::workflow::fallback(&current)),
                };
                let request = execution
                    .as_ref()
                    .and_then(|e| e.decision.as_ref())
                    .map(|d| d.context)
                    .unwrap_or_default();
                let mut visual = None;
                let mut view = None;
                if request == super::workflow::ContextRequest::FreshCamera {
                    let mut status = "camera_disabled";
                    if profile.settings.camera_enabled && camera_available != Some(false) {
                        let (response, receive) = oneshot::channel();
                        events
                            .send(StreamEvent::Capture { turn, response })
                            .await
                            .map_err(|_| "stream cancelled")?;
                        status = "capture_unavailable";
                        if let Ok(Ok(Some(image))) =
                            tokio::time::timeout(Duration::from_secs(12), receive).await
                        {
                            events
                                .send(StreamEvent::VisionStatus {
                                    turn,
                                    state: "analyzing_image",
                                })
                                .await
                                .map_err(|_| "stream cancelled")?;
                            status = "analysis_failed";
                            match tokio::time::timeout(
                                Duration::from_secs(30),
                                backend.observe(Source::Book, &image),
                            )
                            .await
                            {
                                Ok(Ok((text, analysis))) if !text.trim().is_empty() => {
                                    let context = Context {
                                        source: "book".into(),
                                        text: clean(&text, 2000),
                                        topics: analysis.topics,
                                        observed_at: Utc::now(),
                                    };
                                    profile.observations.push(context.clone());
                                    visual = Some(context);
                                    status = "captured";
                                }
                                Ok(Ok(_)) => status = "no_readable_text",
                                _ => tracing::warn!(
                                    turn_id = turn,
                                    "Learning visual tool unavailable"
                                ),
                            }
                        }
                    } else if profile.settings.camera_enabled {
                        status = "capture_unavailable";
                    }
                    tracing::info!(turn_id = turn, status, "Learning visual tool result");
                    events
                        .send(StreamEvent::VisionStatus {
                            turn,
                            state: status,
                        })
                        .await
                        .map_err(|_| "stream cancelled")?;
                    view = Some(VisualContext {
                        status: status.into(),
                        observation: visual.clone(),
                    });
                    if status != "captured"
                        && let Some(decision) = execution.as_mut().and_then(|e| e.decision.as_mut())
                    {
                        decision.assess_user = false;
                        decision.task_policy = super::workflow::TaskPolicy::Keep;
                        if let Some(stage) = super::workflow::playbook(&decision.mode).and_then(|book| book.tool_failure_stage.as_ref()) {
                            decision.stage = stage.clone();
                        }
                    }
                } else if request == super::workflow::ContextRequest::CurrentMaterial
                    && let Some(material) = &current.material
                {
                    events.send(StreamEvent::VisionStatus { turn, state: "material" }).await.map_err(|_| "stream cancelled")?;
                    view = Some(VisualContext {
                        status: "material".into(),
                        observation: Some(material.context.clone()),
                    });
                }
                let after_playback = execution.as_ref().and_then(|flow| flow.decision.as_ref()).is_some_and(|decision| {
                    decision.mode != current.mode || decision.scene != current.scene || decision.practice_attempt
                        || decision.check_in || decision.resume_practice || decision.task_policy == super::workflow::TaskPolicy::Replace
                });
                let introducing = new_agent.is_some();
                let mut plan = profile.plan_for(
                    &profile.active_agent_id.clone(),
                    Utc::now(),
                    proactive,
                    &transcript,
                    introducing,
                );
                if let Some(flow) = &execution {
                    plan.response_act = flow
                        .decision
                        .as_ref()
                        .map(|d| d.action)
                        .unwrap_or(super::agent::ResponseAct::Respond);
                    plan.targets.retain(|target| {
                        flow.decision
                            .as_ref()
                            .is_some_and(|d| d.targets.iter().any(|t| t.key == target.key))
                    });
                    if plan.history.len() > 12 {
                        plan.history.drain(..plan.history.len() - 12);
                    }
                    if !after_playback {
                        events.send(StreamEvent::Learning { turn, message: learning_message(turn, flow, &current) })
                            .await.map_err(|_| "stream cancelled")?;
                    }
                }
                plan.workflow = execution;
                plan.visual = view;
                let assigned = profile
                    .weaknesses
                    .iter()
                    .filter(|w| profile.assigned_targets.contains(&w.key))
                    .cloned()
                    .collect::<Vec<_>>();
                if streaming {
                    let (sentences, receive) = mpsc::channel(2);
                    let generate = backend.reply_prepared(&profile, &plan, &transcript, continuation, sentences);
                    let speech = async {
                        let (segment_sender,mut segments)=mpsc::channel(1);
                        let synthesize=super::speech::synthesize(backend.clone(),plan.agent.clone(),receive,segment_sender);
                        let deliver=async {
                        let mut first = true;
                        let mut total = 0usize;
                        let mut count = 0usize;
                        while let Some(mut segment) = segments.recv().await {
                            let mut received=false;
                            while let Some(audio)=segment.audio.recv().await {
                            received=true;
                            total += audio.len();
                            if audio.is_empty() || !audio.len().is_multiple_of(2) {return Err("segmented TTS returned invalid PCM".to_owned());}
                            if total > MAX_REPLY_BYTES {return Err("streamed reply exceeded device audio budget".to_owned());}
                            if first {
                                let reply = ServerMessage::Reply {
                                    turn_id: turn,
                                    proactive,
                                    text: segment.text.clone(),
                                    hint_zh: String::new(),
                                    targets: plan.targets.iter().map(|t| t.key.clone()).collect(),
                                    agent_id: Some(plan.agent.id.clone()),
                                    agent_name: Some(plan.agent.name.clone()),
                                    response_act: Some(plan.response_act.as_str().into()),
                                };
                                events
                                    .send(StreamEvent::Start { turn, reply })
                                    .await
                                    .map_err(|_| "stream cancelled")?;
                                tracing::info!(
                                    turn_id = turn,
                                    stage = "first_audio",
                                    elapsed_ms = started.elapsed().as_millis(),
                                    "Amigo stream timing"
                                );
                                first = false;
                            }
                            events
                                .send(StreamEvent::Audio { turn, pcm: audio })
                                .await
                                .map_err(|_| "stream cancelled")?;
                            }
                            segment.complete.await.map_err(|_|"TTS worker cancelled")??;
                            if !received {return Err("streaming TTS produced no audio".to_owned())}
                            count += 1;
                        }
                        if first {
                            return Err("stream produced no speech".into());
                        }
                        Ok::<_, String>(count)
                        };
                        let (_,count)=tokio::try_join!(synthesize,deliver)?;
                        Ok::<_,String>(count)
                    };
                    let (analysis, segments) = tokio::try_join!(generate, speech)?;
                    let analysis = plan.workflow.as_ref().map_or_else(
                        || analysis.clone(),
                        |flow| super::workflow::filter_analysis(&analysis, flow, &transcript),
                    );
                    tracing::info!(
                        turn_id = turn,
                        stage = "production_complete",
                        elapsed_ms = started.elapsed().as_millis(),
                        segments,
                        "Amigo stream timing"
                    );
                    let output = JobOutput::Reply {
                        turn,
                        record: Box::new(ReplyRecord { event: Uuid::new_v4(), transcript, analysis, plan, assigned, new_agent, visual, after_playback, origin_agent }),
                        pcm: Vec::new(),
                        streamed: true,
                    };
                    events
                        .send(StreamEvent::Final {
                            turn,
                            output: Box::new(output),
                        })
                        .await
                        .map_err(|_| "stream cancelled")?;
                    return Ok(JobOutput::StreamFinished);
                }
                let analysis = backend
                    .reply(&profile, &plan, &transcript)
                    .await
                    .map_err(|error| format!("tutor: {error}"))?;
                let analysis = plan.workflow.as_ref().map_or_else(
                    || analysis.clone(),
                    |flow| super::workflow::filter_analysis(&analysis, flow, &transcript),
                );
                if analysis.reply_en.trim().is_empty() || analysis.reply_en.len() > 8000 {
                    return Err("invalid reply length".into());
                }
                let pcm = backend
                    .synthesize(&analysis.reply_en, &plan.agent)
                    .await
                    .map_err(|error| format!("TTS: {error}"))?;
                if pcm.is_empty() || !pcm.len().is_multiple_of(2) || pcm.len() > MAX_REPLY_BYTES {
                    return Err("invalid reply audio".into());
                }
                Ok(JobOutput::Reply {
                    turn,
                        record: Box::new(ReplyRecord { event: Uuid::new_v4(), transcript, analysis, plan, assigned, new_agent, visual, after_playback, origin_agent }),
                        pcm,
                        streamed: false,
                })
            })
            .await
            .map_err(|_| "conversation timeout".to_owned())?
        }));
        Ok(())
    }

    async fn stream_event(&mut self, event: StreamEvent, ws: &mut WebSocket) -> Result<(), String> {
        match event {
            StreamEvent::Learning { message, .. } => send(ws, &message).await?,
            StreamEvent::Capture { turn, response } => {
                let settings = super::devices::load(self.device.clone()).await?.settings;
                if self.busy
                    || self.muted
                    || !settings.camera_enabled
                    || self.camera_available == Some(false)
                {
                    let _ = response.send(None);
                    return Ok(());
                }
                self.capture = Some(Capture {
                    id: turn,
                    source: Source::Book,
                    total: None,
                    sequence: 0,
                    data: Zeroizing::new(Vec::new()),
                    started: Instant::now(),
                    response: Some(response),
                });
                send(
                    ws,
                    &ServerMessage::Ack {
                        turn_id: turn,
                        state: "capturing_image".into(),
                    },
                )
                .await?;
                send(
                    ws,
                    &ServerMessage::Capture {
                        request_id: turn,
                        source: Source::Book,
                        max_bytes: MAX_IMAGE_BYTES,
                        max_audio_ms: 0,
                    },
                )
                .await?;
            }
            StreamEvent::VisionStatus { turn, state } => {
                send(
                    ws,
                    &ServerMessage::Ack {
                        turn_id: turn,
                        state: state.into(),
                    },
                )
                .await?
            }
            StreamEvent::Transcript { turn, text } => {
                send(
                    ws,
                    &ServerMessage::Transcript {
                        turn_id: turn,
                        text,
                    },
                )
                .await?
            }
            StreamEvent::Start { turn, reply } => {
                let proactive = matches!(
                    &reply,
                    ServerMessage::Reply {
                        proactive: true,
                        ..
                    }
                );
                if proactive {
                    let profile = super::devices::load(self.device.clone()).await?;
                    if self.busy
                        || self.muted
                        || !profile.settings.proactive_enabled
                        || profile.settings.quiet(Utc::now())
                        || super::workflow::session(&profile, &profile.active_agent_id).mode
                            != "conversation"
                    {
                        self.abort();
                        return send(ws, &ServerMessage::Cancelled { turn_id: turn }).await;
                    }
                }
                send(ws, &reply).await?;
                self.playback = Some(Playback {
                    turn,
                    pcm: Vec::new(),
                    sequence: 0,
                    offset: 0,
                    ended: false,
                    proactive,
                    started: Instant::now(),
                    complete: false,
                    pending_record: None,
                });
            }
            StreamEvent::Audio { turn, pcm } => {
                let playback = self.playback.as_mut().ok_or("stream audio before reply")?;
                if playback.turn != turn || playback.pcm.len() + pcm.len() > MAX_REPLY_BYTES {
                    return Err("invalid stream audio size or turn".into());
                }
                playback.pcm.extend(pcm);
            }
            StreamEvent::Final { output, .. } => self.complete(*output, ws).await?,
        }
        Ok(())
    }
    async fn complete(&mut self, output: JobOutput, ws: &mut WebSocket) -> Result<(), String> {
        match output {
            JobOutput::StreamFinished => {}
            JobOutput::NoSpeech { turn } => {
                self.last_activity = Instant::now();
                send(
                    ws,
                    &ServerMessage::Error {
                        turn_id: Some(turn),
                        code: "no_speech".into(),
                        message: "还没识别到有效语音，请接着说一句话。".into(),
                    },
                )
                .await?;
            }
            JobOutput::Observation {
                request,
                event,
                context,
                analysis,
            } => {
                self.commit_observation(request, event, context, *analysis, ws)
                    .await?
            }
            JobOutput::Reply {
                turn,
                record,
                mut pcm,
                streamed,
            } => {
                let ReplyRecord {
                    transcript,
                    analysis,
                    plan,
                    after_playback,
                    ..
                } = &*record;
                let proactive = plan.proactive;
                let act = plan.response_act;
                let current = super::devices::load(self.device.clone()).await?;
                if plan.proactive
                    && (self.busy
                        || self.muted
                        || !current.settings.proactive_enabled
                        || current.settings.quiet(Utc::now()))
                {
                    self.abort();
                    send(ws, &ServerMessage::Cancelled { turn_id: turn }).await?;
                    return Ok(());
                }
                if !after_playback {
                    self.record_reply(turn, &record, ws).await?;
                }
                if streamed {
                    send(
                        ws,
                        &ServerMessage::ReplyUpdate {
                            turn_id: turn,
                            text: analysis.reply_en.clone(),
                            hint_zh: analysis.hint_zh.clone(),
                        },
                    )
                    .await?;
                    let output = self.playback.as_mut().ok_or("stream playback missing")?;
                    if self.audio_codec == AudioCodec::Opus {
                        output.pcm.resize(output.pcm.len() + CODEC_TAIL_BYTES, 0);
                    }
                    output.complete = true;
                    if *after_playback {
                        output.pending_record = Some(record);
                    }
                    return Ok(());
                }
                if !transcript.is_empty() {
                    send(
                        ws,
                        &ServerMessage::Transcript {
                            turn_id: turn,
                            text: transcript.clone(),
                        },
                    )
                    .await?;
                }
                send(
                    ws,
                    &ServerMessage::Reply {
                        turn_id: turn,
                        proactive,
                        text: analysis.reply_en.clone(),
                        hint_zh: analysis.hint_zh.clone(),
                        targets: plan.targets.iter().map(|t| t.key.clone()).collect(),
                        agent_id: Some(plan.agent.id.clone()),
                        agent_name: Some(plan.agent.name.clone()),
                        response_act: Some(act.as_str().into()),
                    },
                )
                .await?;
                if self.audio_codec == AudioCodec::Opus {
                    pcm.resize(pcm.len() + CODEC_TAIL_BYTES, 0);
                }
                self.playback = Some(Playback {
                    turn,
                    pcm,
                    sequence: 0,
                    offset: 0,
                    ended: false,
                    proactive,
                    started: Instant::now(),
                    complete: true,
                    pending_record: (*after_playback).then_some(record),
                });
            }
        }
        Ok(())
    }

    async fn record_reply(
        &mut self,
        turn: u64,
        record: &ReplyRecord,
        ws: &mut WebSocket,
    ) -> Result<(), String> {
        let ReplyRecord {
            event,
            transcript,
            analysis,
            plan,
            assigned,
            new_agent,
            visual,
            ..
        } = record;
        let event = *event;
        let assigned = assigned.clone();
        let visual = visual.clone();
        let new_agent = new_agent.clone();
        let origin_agent = record.after_playback.then(|| record.origin_agent.clone());
        let learned = analysis.clone();
        let input = transcript.clone();
        let proactive = plan.proactive;
        let agent_id = plan.agent.id.clone();
        let act = plan.response_act;
        let execution = plan.workflow.clone();
        let commit_execution = execution.clone();
        let planned = plan
            .targets
            .iter()
            .map(|w| w.key.clone())
            .collect::<Vec<_>>();
        let committed = store::record(
            self.device.clone(),
            self.lease,
            event,
            if proactive { "outreach" } else { "dialogue" },
            serde_json::json!({"transcript":transcript,"analysis":analysis,"plan":plan}),
            move |profile| {
                if origin_agent
                    .as_ref()
                    .is_some_and(|origin| *origin != profile.active_agent_id)
                {
                    return;
                }
                if proactive
                    && (!profile.settings.proactive_enabled
                        || profile.settings.quiet(Utc::now())
                        || profile.active_agent_id != agent_id
                        || commit_execution
                            .as_ref()
                            .and_then(|flow| flow.decision.as_ref())
                            .is_some_and(|decision| {
                                let current = super::workflow::session(profile, &agent_id);
                                super::workflow::authorize_outreach(decision, &current, profile)
                                    .is_err()
                            }))
                {
                    return;
                }
                let learning_mode = commit_execution
                    .as_ref()
                    .and_then(|flow| flow.decision.as_ref())
                    .map(|decision| decision.mode.clone())
                    .unwrap_or_else(|| super::workflow::session(profile, &agent_id).mode);
                let fictional =
                    super::workflow::playbook(&learning_mode).is_some_and(|book| book.fictional);
                if let Some(flow) = &commit_execution
                    && !super::workflow::commit(
                        profile,
                        super::workflow::Commit {
                            agent_id: &agent_id,
                            execution: flow,
                            analysis: &learned,
                            text: &input,
                            event,
                            visual: visual.as_ref(),
                            now: Utc::now(),
                        },
                    )
                {
                    // This was spoken, but a newer user/device decision owns learning.
                    if !proactive {
                        profile.history.push(Dialogue {
                            learning_mode: learning_mode.clone(),
                            fictional,
                            role: "user".into(),
                            text: input.clone(),
                            agent_id: agent_id.clone(),
                        });
                    }
                    profile.history.push(Dialogue {
                        learning_mode: learning_mode.clone(),
                        fictional,
                        role: "assistant".into(),
                        text: learned.reply_en.clone(),
                        agent_id: agent_id.clone(),
                    });
                    if profile.history.len() > 64 {
                        profile.history.drain(..profile.history.len() - 64);
                    }
                    return;
                }
                if let Some(context) = visual
                    && profile.settings.camera_enabled
                {
                    profile.observe(context, &[]);
                }
                if let Some(agent) = new_agent {
                    let _ = profile.activate(agent);
                }
                profile.active_agent_id = agent_id.clone();
                if proactive {
                    profile.history.push(Dialogue {
                        learning_mode: learning_mode.clone(),
                        fictional,
                        role: "assistant".into(),
                        text: learned.reply_en.clone(),
                        agent_id: agent_id.clone(),
                    });
                    if profile.history.len() > 64 {
                        profile.history.drain(..profile.history.len() - 64);
                    }
                    profile.record_behavior(&agent_id, act);
                } else {
                    profile.learn_for_agent(
                        &input,
                        &learned,
                        &assigned,
                        Utc::now(),
                        &agent_id,
                        act,
                    );
                }
                profile.assigned_targets = planned;
                for issue in &learned.issues {
                    if learned
                        .reply_en
                        .to_lowercase()
                        .contains(&issue.corrected.to_lowercase())
                        && profile.weaknesses.iter().any(|w| w.key == issue.key)
                        && !profile.assigned_targets.contains(&issue.key)
                    {
                        profile.assigned_targets.push(issue.key.clone());
                    }
                }
                profile.assigned_targets.truncate(2);
            },
        )
        .await?;
        if let Some(mut flow) = execution {
            let current = super::workflow::session(&committed, &plan.agent.id);
            flow.status = if flow.decision.is_none() {
                "clarification"
            } else if current.last_event == Some(event) {
                "committed"
            } else if current.revision == flow.base_revision
                && committed.active_agent_id == plan.agent.id
            {
                "clarification"
            } else {
                "state_changed"
            }
            .into();
            send(ws, &learning_message(turn, &flow, &current)).await?;
        }
        if proactive {
            self.feedback = Some((turn, event));
        }
        Ok(())
    }

    async fn commit_observation(
        &mut self,
        request: u64,
        event: Uuid,
        context: Context,
        analysis: Analysis,
        ws: &mut WebSocket,
    ) -> Result<(), String> {
        let settings = super::devices::load(self.device.clone()).await?.settings;
        let allowed = if context.source == "speech" {
            settings.ambient_audio_enabled
        } else {
            settings.camera_enabled
        };
        if !allowed || self.busy || self.muted {
            return Ok(());
        }
        let content = serde_json::json!({"context":context,"memories":analysis.memories});
        store::record(
            self.device.clone(),
            self.lease,
            event,
            "observation",
            content,
            move |profile| {
                profile.observe(context, &analysis.memories);
            },
        )
        .await?;
        send(
            ws,
            &ServerMessage::ObservationStored {
                request_id: request,
            },
        )
        .await
    }

    async fn send_audio(&mut self, ws: &mut WebSocket) -> Result<(), String> {
        let output = self.playback.as_mut().unwrap();
        if !output.complete && output.pcm.len() - output.offset < 640 {
            return Ok(());
        }
        if output.offset == output.pcm.len() {
            self.opus.clear_encoder();
            send(
                ws,
                &ServerMessage::AudioEnd {
                    turn_id: output.turn,
                    bytes: output.pcm.len(),
                },
            )
            .await?;
            output.ended = true;
        } else {
            let end = (output.offset + 640).min(output.pcm.len());
            let encoded;
            let payload = if self.audio_codec == AudioCodec::Opus {
                encoded = self
                    .opus
                    .encode(output.turn, &output.pcm[output.offset..end])?;
                &encoded
            } else {
                &output.pcm[output.offset..end]
            };
            let packet = MediaPacket {
                kind: if self.audio_codec == AudioCodec::Opus {
                    MediaKind::OpusOut
                } else {
                    MediaKind::AudioOut
                },
                turn_id: output.turn,
                sequence: output.sequence,
                payload,
            }
            .encode()
            .map_err(str::to_owned)?;
            tokio::time::timeout(Duration::from_secs(2), ws.send(Message::binary(packet)))
                .await
                .map_err(|_| "speaker backpressure timeout")?
                .map_err(|_| "disconnected")?;
            output.offset = end;
            output.sequence += 1;
        }
        Ok(())
    }

    async fn schedule(&mut self, ws: &mut WebSocket) -> Result<(), String> {
        if self.realtime_mode || !self.backend.ready() || !self.idle() || self.busy || self.muted {
            return Ok(());
        }
        let profile = super::devices::load(self.device.clone()).await?;
        let now = Utc::now();
        if profile.settings.quiet(now) {
            return Ok(());
        }
        let idle = self.last_activity.elapsed().as_secs() >= profile.settings.idle_secs;
        let activity = super::workflow::session(&profile, &profile.active_agent_id);
        if activity.mode == "conversation"
            && activity.task.is_none()
            && profile.may_outreach(now, idle)
            && store::reserve_outreach_device(self.device.clone(), now).await?
        {
            let turn = self.server_id();
            self.start_reply(turn, None, None, true, None).await?;
            return Ok(());
        }
        if !idle || (!profile.settings.camera_enabled && !profile.settings.ambient_audio_enabled) {
            return Ok(());
        }
        let due = profile.last_observation_at.is_none_or(|last| {
            now.signed_duration_since(last).num_seconds()
                >= profile.settings.observation_interval_secs as i64
        });
        if !due {
            return Ok(());
        }
        // Select the source and reserve the interval under the profile lock,
        // rechecking current settings if another device/user changed them.
        let Some(source) =
            store::reserve_observation_device(self.device.clone(), now, self.last_capture_source)
                .await?
        else {
            return Ok(());
        };
        self.begin_capture(source, ws).await
    }

    async fn begin_capture(&mut self, source: Source, ws: &mut WebSocket) -> Result<(), String> {
        self.last_capture_source = source;
        let id = self.server_id();
        self.capture = Some(Capture {
            id,
            source,
            total: None,
            sequence: 0,
            data: Zeroizing::new(Vec::new()),
            started: Instant::now(),
            response: None,
        });
        send(
            ws,
            &ServerMessage::Capture {
                request_id: id,
                source,
                max_bytes: if source == Source::Speech {
                    96000
                } else {
                    MAX_IMAGE_BYTES
                },
                max_audio_ms: 3000,
            },
        )
        .await
    }
}

fn source_name(source: Source) -> &'static str {
    match source {
        Source::Screen => "screen",
        Source::Book => "book",
        Source::Speech => "speech",
    }
}
fn learning_message(
    turn_id: u64,
    execution: &super::workflow::Execution,
    current: &super::workflow::Session,
) -> ServerMessage {
    let decision = if matches!(execution.status.as_str(), "planned" | "committed") {
        execution.decision.as_ref()
    } else {
        None
    };
    ServerMessage::LearningState {
        turn_id,
        mode: decision
            .map(|d| d.mode.clone())
            .unwrap_or_else(|| current.mode.clone()),
        stage: decision
            .map(|d| d.stage.clone())
            .unwrap_or_else(|| current.stage.clone()),
        goal: decision
            .map(|d| d.goal.clone())
            .unwrap_or_else(|| current.goal.clone()),
        strategy: decision
            .map(|d| d.strategy.clone())
            .unwrap_or_else(|| current.strategy.clone()),
        explanation: if execution.status == "state_changed" {
            "学习方式已在其他操作中更新，这轮未计入学习进度".into()
        } else {
            decision
                .map(|d| d.explanation.clone())
                .unwrap_or_else(|| "先确认当前需求，保留原来的学习任务".into())
        },
        revision: current.revision,
        status: execution.status.clone(),
    }
}

fn error(turn_id: Option<u64>, code: &str, message: &str) -> ServerMessage {
    ServerMessage::Error {
        turn_id,
        code: code.into(),
        message: message.into(),
    }
}
async fn send(ws: &mut WebSocket, message: &ServerMessage) -> Result<(), String> {
    let json = serde_json::to_string(message).map_err(|_| "serialization failed")?;
    tokio::time::timeout(Duration::from_secs(2), ws.send(Message::text(json)))
        .await
        .map_err(|_| "device backpressure timeout")?
        .map_err(|_| "disconnected".into())
}

//! Push-to-talk, half-duplex conversation control.
//!
//! Feed events serially and execute each returned action before feeding the
//! next event. A failed action must be followed by `Cancel` or `Disconnected`.
//! The adapter owns deadlines and reports `TimedOut` with the original turn ID.
//! Playback completion means both the network stream and speaker DMA drained.

/// Identifies a turn for the lifetime of a `Session`, including reconnects.
pub type TurnId = u64;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    Offline,
    Ready,
    Recording { turn: TurnId },
    Waiting { turn: TurnId },
    Playing { turn: TurnId },
}

impl State {
    pub const fn turn(self) -> Option<TurnId> {
        match self {
            Self::Recording { turn } | Self::Waiting { turn } | Self::Playing { turn } => {
                Some(turn)
            }
            Self::Offline | Self::Ready => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    /// An authenticated server session is ready, not merely Wi-Fi connected.
    Connected,
    Disconnected,
    RecordPressed,
    RecordReleased,
    /// The first playable reply audio is buffered for this turn.
    ReplyReady {
        turn: TurnId,
    },
    PlaybackFinished {
        turn: TurnId,
    },
    /// User cancellation or a local I/O failure.
    Cancel,
    TimedOut {
        turn: TurnId,
    },
}

#[must_use = "the board adapter must execute session actions"]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    None,
    /// Begin capture and stream frames tagged with this turn ID.
    StartRecording {
        turn: TurnId,
    },
    /// Stop capture, flush the final frame and send the end-of-turn marker.
    FinishRecording {
        turn: TurnId,
    },
    StartPlayback {
        turn: TurnId,
    },
    /// Stop capture/playback, discard queued media and cancel the remote turn.
    StopTurn {
        turn: TurnId,
    },
}

#[derive(Debug)]
pub struct Session {
    state: State,
    last_turn: TurnId,
}

impl Default for Session {
    fn default() -> Self {
        Self::new()
    }
}

impl Session {
    pub const fn new() -> Self {
        Self {
            state: State::Offline,
            last_turn: 0,
        }
    }

    pub const fn state(&self) -> State {
        self.state
    }

    /// Invalid, duplicate or stale events have no effect. IDs never wrap;
    /// after `u64::MAX` turns, new recordings are refused.
    pub fn handle(&mut self, event: Event) -> Action {
        match (self.state, event) {
            (State::Offline, Event::Connected) => {
                self.state = State::Ready;
                Action::None
            }
            (_, Event::Disconnected) => {
                let turn = self.state.turn();
                self.state = State::Offline;
                turn.map_or(Action::None, |turn| Action::StopTurn { turn })
            }
            (State::Ready, Event::RecordPressed) => {
                let Some(turn) = self.last_turn.checked_add(1) else {
                    return Action::None;
                };
                self.last_turn = turn;
                self.state = State::Recording { turn };
                Action::StartRecording { turn }
            }
            (State::Recording { turn }, Event::RecordReleased) => {
                self.state = State::Waiting { turn };
                Action::FinishRecording { turn }
            }
            (State::Waiting { turn }, Event::ReplyReady { turn: reply }) if turn == reply => {
                self.state = State::Playing { turn };
                Action::StartPlayback { turn }
            }
            (State::Playing { turn }, Event::PlaybackFinished { turn: finished })
                if turn == finished =>
            {
                self.state = State::Ready;
                Action::None
            }
            (_, Event::Cancel) => self.stop_turn(),
            (_, Event::TimedOut { turn }) if self.state.turn() == Some(turn) => self.stop_turn(),
            _ => Action::None,
        }
    }

    fn stop_turn(&mut self) -> Action {
        match self.state.turn() {
            Some(turn) => {
                self.state = State::Ready;
                Action::StopTurn { turn }
            }
            None => Action::None,
        }
    }
}

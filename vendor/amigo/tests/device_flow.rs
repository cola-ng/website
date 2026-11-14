use amigo::audio::{FRAME_BYTES, FRAME_SAMPLES, PcmFrame};
use amigo::{Action, Event, Session, State, TurnId};

fn connected() -> Session {
    let mut session = Session::new();
    assert_eq!(session.handle(Event::Connected), Action::None);
    session
}

fn record(session: &mut Session) -> TurnId {
    let Action::StartRecording { turn } = session.handle(Event::RecordPressed) else {
        panic!("expected capture to start");
    };
    turn
}

#[test]
fn push_to_talk_waits_for_matching_audio_and_speaker_completion() {
    let mut session = connected();
    let turn = record(&mut session);
    assert_eq!(session.handle(Event::ReplyReady { turn }), Action::None);
    assert_eq!(session.state(), State::Recording { turn });
    assert_eq!(
        session.handle(Event::RecordReleased),
        Action::FinishRecording { turn }
    );
    assert_eq!(
        session.handle(Event::PlaybackFinished { turn }),
        Action::None
    );
    assert_eq!(session.state(), State::Waiting { turn });
    assert_eq!(
        session.handle(Event::ReplyReady { turn }),
        Action::StartPlayback { turn }
    );
    assert_eq!(session.handle(Event::RecordPressed), Action::None);
    assert_eq!(
        session.handle(Event::PlaybackFinished { turn }),
        Action::None
    );
    assert_eq!(session.state(), State::Ready);
}

#[test]
fn offline_device_cannot_capture_and_duplicate_connect_does_not_reset_capture() {
    let mut session = Session::new();
    assert_eq!(session.handle(Event::RecordPressed), Action::None);
    assert_eq!(session.handle(Event::Cancel), Action::None);
    assert_eq!(session.state(), State::Offline);
    assert_eq!(session.handle(Event::Connected), Action::None);
    let turn = record(&mut session);
    assert_eq!(session.handle(Event::Connected), Action::None);
    assert_eq!(session.handle(Event::RecordPressed), Action::None);
    assert_eq!(session.state(), State::Recording { turn });
}

#[test]
fn disconnect_stops_io_at_every_active_stage_and_reconnect_preserves_turn_ids() {
    for stage in 0..3 {
        let mut session = connected();
        let turn = record(&mut session);
        if stage >= 1 {
            assert_eq!(
                session.handle(Event::RecordReleased),
                Action::FinishRecording { turn }
            );
        }
        if stage == 2 {
            assert_eq!(
                session.handle(Event::ReplyReady { turn }),
                Action::StartPlayback { turn }
            );
        }
        assert_eq!(
            session.handle(Event::Disconnected),
            Action::StopTurn { turn }
        );
        assert_eq!(session.state(), State::Offline);
        assert_eq!(session.handle(Event::Connected), Action::None);
        let next = record(&mut session);
        assert!(next > turn);
        assert_eq!(
            session.handle(Event::RecordReleased),
            Action::FinishRecording { turn: next }
        );
        assert_eq!(session.handle(Event::ReplyReady { turn }), Action::None);
        assert_eq!(session.state(), State::Waiting { turn: next });
    }
}

#[test]
fn cancellation_and_timeouts_stop_each_stage_without_affecting_the_next_turn() {
    for stage in 0..3 {
        for timeout in [false, true] {
            let mut session = connected();
            let turn = record(&mut session);
            if stage >= 1 {
                assert_eq!(
                    session.handle(Event::RecordReleased),
                    Action::FinishRecording { turn }
                );
            }
            if stage == 2 {
                assert_eq!(
                    session.handle(Event::ReplyReady { turn }),
                    Action::StartPlayback { turn }
                );
            }
            let event = if timeout {
                Event::TimedOut { turn }
            } else {
                Event::Cancel
            };
            assert_eq!(session.handle(event), Action::StopTurn { turn });
            assert_eq!(session.state(), State::Ready);
            let next = record(&mut session);
            assert_eq!(session.handle(Event::TimedOut { turn }), Action::None);
            assert_eq!(session.state(), State::Recording { turn: next });
            assert_eq!(
                session.handle(Event::RecordReleased),
                Action::FinishRecording { turn: next }
            );
            assert_eq!(
                session.handle(Event::ReplyReady { turn: next }),
                Action::StartPlayback { turn: next }
            );
            assert_eq!(
                session.handle(Event::PlaybackFinished { turn }),
                Action::None
            );
            assert_eq!(session.state(), State::Playing { turn: next });
        }
    }
}

#[test]
fn pcm_frames_use_signed_little_endian_samples_with_a_fixed_memory_budget() {
    assert_eq!(FRAME_SAMPLES, 320);
    assert_eq!(FRAME_BYTES, 640);
    let mut frame = PcmFrame::silence();
    frame.samples[..5].copy_from_slice(&[i16::MIN, -1, 0, 1, i16::MAX]);
    frame.samples[FRAME_SAMPLES - 1] = -1234;
    let bytes = frame.encode_le();
    assert_eq!(&bytes[..10], &[0, 128, 255, 255, 0, 0, 1, 0, 255, 127]);
    assert_eq!(&bytes[FRAME_BYTES - 2..], &(-1234_i16).to_le_bytes());
    assert_eq!(PcmFrame::decode_le(&bytes), frame);
}

//! A scripted simulation of device logic, without real audio or networking.

use amigo::audio::{FRAME_BYTES, FRAME_DURATION_MS, PcmFrame};
use amigo::{Action, Event, Session, State};

fn dispatch(session: &mut Session, event: Event) -> Action {
    let action = session.handle(event);
    println!("{event:?} -> {:?}; action: {action:?}", session.state());
    action
}

fn main() {
    let mut session = Session::new();
    let _ = dispatch(&mut session, Event::Connected);
    let Action::StartRecording { turn } = dispatch(&mut session, Event::RecordPressed) else {
        panic!("the authenticated device should start recording");
    };

    // A board adapter would fill this from I2S and send it over a real transport.
    let bytes = PcmFrame::silence().encode_le();
    println!("Mock uplink: turn={turn}, {FRAME_DURATION_MS} ms, {FRAME_BYTES} bytes");
    assert_eq!(PcmFrame::decode_le(&bytes), PcmFrame::silence());

    let _ = dispatch(&mut session, Event::RecordReleased);
    println!("Mock server: ASR -> English tutor -> TTS (no model call)");
    let _ = dispatch(&mut session, Event::ReplyReady { turn });
    let _ = dispatch(&mut session, Event::PlaybackFinished { turn });
    assert_eq!(session.state(), State::Ready);

    // Demonstrate cleanup on network loss and rejection of a late server reply.
    let Action::StartRecording { turn } = dispatch(&mut session, Event::RecordPressed) else {
        panic!("the next turn should start recording");
    };
    let _ = dispatch(&mut session, Event::RecordReleased);
    let _ = dispatch(&mut session, Event::Disconnected);
    let _ = dispatch(&mut session, Event::Connected);
    let _ = dispatch(&mut session, Event::ReplyReady { turn });
    assert_eq!(session.state(), State::Ready);
}

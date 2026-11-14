//! Allocation-free device logic for a cloud-connected English-learning robot.
//!
//! Board firmware owns the camera, I2S, Wi-Fi/TLS, timers and effect execution.
//! This library can be shared by ESP-IDF and bare-metal firmware, and exercised
//! on a development PC with `cargo run -p amigo --example simulator`.

#![no_std]
#![forbid(unsafe_code)]

#[cfg(feature = "protocol")]
extern crate std;

#[cfg(feature = "protocol")]
pub mod protocol;

pub mod audio;
pub mod session;

pub use session::{Action, Event, Session, State, TurnId};

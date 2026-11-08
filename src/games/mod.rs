//! One module per supported game. F1 25 has its own engine (`f1_25`); every other game is a
//! parser feeding the shared `sim` engine. Dropping a session stops its listener thread and
//! UI refresh timer.

pub mod ams2;
pub mod bridge_rx;
pub mod bytes;
pub mod dr2;
pub mod f1_25;
pub mod forza;
pub mod map;
pub mod sim;
pub mod wrc;

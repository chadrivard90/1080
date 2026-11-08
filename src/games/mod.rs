//! One module per supported game. Each exposes a session type that owns the telemetry
//! listener thread and the UI refresh timer; dropping the session stops both.

pub mod f1_25;

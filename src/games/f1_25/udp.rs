use std::net::UdpSocket;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use super::packets::F1State;

/// Receive F1 25 packets until `stop` is set. Works with both unicast and broadcast mode.
pub fn run(shared: Arc<Mutex<F1State>>, stop: Arc<AtomicBool>, port: u16) {
    let socket = match UdpSocket::bind(("0.0.0.0", port)) {
        Ok(s) => s,
        Err(e) => {
            if let Ok(mut st) = shared.lock() {
                st.error = Some(format!("Cannot listen on UDP port {port}: {e}"));
            }
            return;
        }
    };
    // Short timeout so we notice `stop` quickly when leaving the dash.
    let _ = socket.set_read_timeout(Some(Duration::from_millis(200)));
    let mut buf = [0u8; 2048];
    while !stop.load(Ordering::Relaxed) {
        if let Ok(n) = socket.recv(&mut buf) {
            if let Ok(mut st) = shared.lock() {
                st.handle_packet(&buf[..n]);
            }
        }
    }
}

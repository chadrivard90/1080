use std::net::UdpSocket;

/// Best-effort LAN IP of this device, shown on screen so you know where to point the game's
/// UDP output. Connecting a UDP socket sends no packets; it only selects the outgoing interface.
pub fn local_ip() -> String {
    UdpSocket::bind("0.0.0.0:0")
        .and_then(|s| {
            s.connect("8.8.8.8:80")?;
            s.local_addr()
        })
        .map(|addr| addr.ip().to_string())
        .unwrap_or_else(|_| "no network".to_string())
}

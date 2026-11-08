//! race-dash-bridge: run this on the gaming PC for games that only expose telemetry through
//! Windows shared memory (Assetto Corsa EVO, iRacing-engine NASCAR). It finds whichever game is
//! running and broadcasts its data to Race Dash on the Pi (UDP port 20800).
//!
//! Usage:  race-dash-bridge.exe [PI_IP] [PORT]
//!         With no IP it broadcasts to the whole local network.

#[cfg(windows)]
mod acevo;
#[cfg(windows)]
mod iracing;
#[cfg(windows)]
mod shm;

pub mod packet;

#[cfg(not(windows))]
fn main() {
    eprintln!("race-dash-bridge reads Windows shared memory and only runs on Windows.");
}

#[cfg(windows)]
fn main() {
    use std::net::UdpSocket;
    use std::time::{Duration, Instant};

    let args: Vec<String> = std::env::args().collect();
    let target_ip = args.get(1).cloned().unwrap_or_else(|| "255.255.255.255".to_string());
    let port: u16 = args.get(2).and_then(|p| p.parse().ok()).unwrap_or(20800);
    let target = format!("{target_ip}:{port}");

    let socket = UdpSocket::bind("0.0.0.0:0").expect("could not open a UDP socket");
    let _ = socket.set_broadcast(true);

    println!("Race Dash bridge");
    println!("Sending to {target}. Leave this window open while you race; close it when done.");
    println!("Waiting for Assetto Corsa EVO or NASCAR (iRacing engine)...");

    let mut ace = acevo::AcEvo::default();
    let mut ir = iracing::IRacing::default();
    let mut last_source = "";
    let mut last_report = Instant::now();
    let mut sent = 0u64;

    loop {
        let packet = if let Some(p) = ace.poll() {
            Some(("Assetto Corsa EVO", p))
        } else if let Some(p) = ir.poll() {
            Some(("NASCAR / iRacing engine", p))
        } else {
            None
        };

        match packet {
            Some((source, text)) => {
                if source != last_source {
                    println!("Connected to {source}.");
                    last_source = source;
                }
                if socket.send_to(text.as_bytes(), &target).is_ok() {
                    sent += 1;
                }
                if last_report.elapsed() > Duration::from_secs(10) {
                    println!("  {source}: {sent} packets sent");
                    last_report = Instant::now();
                }
                std::thread::sleep(Duration::from_millis(33));
            }
            None => {
                if !last_source.is_empty() {
                    println!("Game closed - waiting...");
                    last_source = "";
                }
                std::thread::sleep(Duration::from_millis(1000));
            }
        }
    }
}

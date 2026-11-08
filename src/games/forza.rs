//! Forza "Data Out" parser.
//!
//! - Forza Motorsport (2023) "Car Dash" format: 331 bytes (Sled 232 + Dash + TireWear + TrackOrdinal)
//! - Forza Motorsport 7 Dash: 311 bytes
//! - Forza Horizon 6 (and 4/5): 324 bytes. FH6 documents CarGroup, SmashableVelDiff and
//!   SmashableMass at 232..244 (FH4/5 left those 12 bytes unnamed), so the dash block starts at 244.
//!
//! Wheel order in the packet is FL, FR, RL, RR (same as SimState).

use super::bytes::*;
use super::sim::{Game, SimState};

const G: f32 = 9.81;

pub fn handle(st: &mut SimState, b: &[u8]) {
    let (dash, has_wear) = match b.len() {
        331 => (232usize, true),
        311 => (232, false),
        324 => (244, false),
        232 => {
            st.warning = Some("Forza is sending the 'Sled' format. Set Data Out Packet Format to 'Car Dash'.".into());
            return;
        }
        n => {
            st.warning = Some(format!("Unexpected Forza packet size ({n} bytes)"));
            return;
        }
    };
    if st.game == Game::ForzaHorizon && dash == 232 {
        st.warning = Some("Receiving Forza Motorsport data - pick Forza Motorsport on the title screen.".into());
    } else if st.game == Game::ForzaMotorsport && dash == 244 {
        st.warning = Some("Receiving Forza Horizon data - pick Forza Horizon 6 on the title screen.".into());
    } else {
        st.warning = None;
    }

    // IsRaceOn = 0 in menus / paused: keep the last values on screen.
    if i32_at(b, 0) == 0 {
        return;
    }

    st.max_rpm = f32_at(b, 8);
    st.rpm = f32_at(b, 16);

    // Local frame: X = right, Y = up, Z = forward.
    let (ax, az) = (f32_at(b, 20), f32_at(b, 28));
    st.g_lat = ax / G;
    st.g_long = az / G;
    let (vx, vz) = (f32_at(b, 32), f32_at(b, 40));
    st.yaw_rate = f32_at(b, 48);
    st.yaw = f32_at(b, 56);
    let ground_speed = (vx * vx + vz * vz).sqrt();
    st.drift_deg = if ground_speed > 4.0 && vz > 0.0 { vx.atan2(vz).to_degrees() } else { 0.0 };

    for w in 0..4 {
        let c = &mut st.corners[w];
        c.susp = Some(f32_at(b, 68 + w * 4));
        c.slip = Some(f32_at(b, 180 + w * 4).abs()); // combined slip, >1 = sliding
    }

    let class = i32_at(b, 216);
    let pi = i32_at(b, 220);
    let drive = match i32_at(b, 224) {
        0 => "FWD",
        1 => "RWD",
        _ => "AWD",
    };
    let class_name = if st.game == Game::ForzaHorizon {
        ["D", "C", "B", "A", "S1", "S2", "R", "X"].get(class as usize).copied().unwrap_or("?")
    } else {
        ["E", "D", "C", "B", "A", "S", "R", "P", "X"].get(class as usize).copied().unwrap_or("?")
    };
    st.car_info = format!("{class_name} {pi}  {drive}");

    let p = |o: usize| dash + o;
    let (x, y, z) = (f32_at(b, p(0)), f32_at(b, p(4)), f32_at(b, p(8)));
    st.world = Some((x, y, z));
    st.speed_ms = f32_at(b, p(12));
    st.power_kw = Some(f32_at(b, p(16)) / 1000.0);
    st.torque_nm = Some(f32_at(b, p(20)));
    for w in 0..4 {
        let f = f32_at(b, p(24 + w * 4));
        st.corners[w].surface_c = Some((f - 32.0) * 5.0 / 9.0);
    }
    let boost = f32_at(b, p(40));
    st.boost_psi = Some(boost.max(0.0));
    st.fuel_frac = Some(f32_at(b, p(44)).clamp(0.0, 1.0));
    let distance = f32_at(b, p(48));
    st.distance_m = Some(distance.max(0.0));
    let best = f32_at(b, p(52));
    let last = f32_at(b, p(56));
    let current = f32_at(b, p(60));
    let race_time = f32_at(b, p(64));
    let laps_done = u16_at(b, p(68)) as i32;
    st.best_lap_s = (best > 0.0).then_some(best);
    st.last_lap_s = (last > 0.0).then_some(last);
    st.current_lap_s = (current > 0.0).then_some(current);
    st.position = u8_at(b, p(70)) as u32;
    st.throttle = u8_at(b, p(71)) as f32 / 255.0;
    st.brake = u8_at(b, p(72)) as f32 / 255.0;
    st.clutch = u8_at(b, p(73)) as f32 / 255.0;
    st.handbrake = u8_at(b, p(74)) as f32 / 255.0;
    st.gear = match u8_at(b, p(75)) {
        0 => -1, // reverse
        11 => 0, // neutral
        g => g as i32,
    };
    st.steer = i8_at(b, p(76)) as f32 / 127.0;

    if has_wear {
        for w in 0..4 {
            st.corners[w].wear_pct = Some(f32_at(b, 311 + w * 4).clamp(0.0, 1.0) * 100.0);
        }
        let track = i32_at(b, 327);
        st.track_name = if track > 0 { format!("TRACK {track}") } else { String::new() };
    }

    // New event / restart: race time went backwards.
    if race_time + 1.0 < st.aux_time as f32 {
        st.new_session();
        st.aux_lap = laps_done;
        st.aux_dist = distance;
    }
    st.aux_time = race_time as f64;

    // Our own lap distance for the live delta (Forza has no delta of its own).
    if laps_done != st.aux_lap {
        st.aux_lap = laps_done;
        st.aux_dist = distance;
    }
    st.lap = laps_done + 1;
    st.lap_dist_m = (st.game == Game::ForzaMotorsport).then(|| (distance - st.aux_dist).max(0.0));
    st.session_name = if st.position > 0 { "RACE".into() } else { "FREE ROAM".into() };
}

#[cfg(test)]
mod tests {
    use super::*;

    fn put_f32(b: &mut [u8], off: usize, v: f32) {
        b[off..off + 4].copy_from_slice(&v.to_le_bytes());
    }

    #[test]
    fn parses_fh6_and_fm() {
        let mut b = vec![0u8; 324];
        b[0] = 1;
        put_f32(&mut b, 16, 6500.0);
        put_f32(&mut b, 244 + 12, 50.0); // speed m/s
        b[244 + 75] = 3; // gear
        b[244 + 70] = 2; // position
        let mut st = SimState::new(Game::ForzaHorizon);
        handle(&mut st, &b);
        assert_eq!(st.gear, 3);
        assert!((st.speed_ms - 50.0).abs() < 1e-4);
        assert_eq!(st.position, 2);
        assert!((st.rpm - 6500.0).abs() < 1e-3);

        let mut m = vec![0u8; 331];
        m[0] = 1;
        put_f32(&mut m, 232 + 12, 70.0);
        put_f32(&mut m, 311, 0.25); // FL wear
        let mut st = SimState::new(Game::ForzaMotorsport);
        handle(&mut st, &m);
        assert!((st.speed_ms - 70.0).abs() < 1e-4);
        assert!((st.corners[0].wear_pct.unwrap() - 25.0).abs() < 1e-3);
    }
}

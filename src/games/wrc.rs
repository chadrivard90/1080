//! EA SPORTS WRC UDP, default "wrc" structure, "session_update" packet (237 bytes).
//!
//! Channel order (little-endian, packed): packet_4cc, packet_uid (u64), shiftlights fraction /
//! rpm start / rpm end (f32), shiftlights valid (bool), gear index / neutral index / reverse index /
//! max gear (u8), speed, transmission speed, position xyz, velocity xyz, acceleration xyz,
//! left / forward / up direction xyz, hub position (bl, br, fl, fr), hub velocity, contact-patch
//! forward speed, brake temperature, engine rpm max / idle / current, throttle, brake, clutch,
//! steering, handbrake, game total time, game delta time (f32), frame count (u64),
//! stage current time (f32), stage current distance (f64), stage length (f32).

use super::bytes::*;
use super::sim::SimState;

pub const UPDATE_SIZE: usize = 237;

/// bl, br, fl, fr packet order -> FL, FR, RL, RR state order.
const WHEEL: [usize; 4] = [2, 3, 0, 1];

pub fn handle(st: &mut SimState, b: &[u8]) {
    let fourcc = b.get(0..4).unwrap_or(&[]);
    if fourcc != b"sesu" && b.len() != UPDATE_SIZE {
        return; // start / end / pause packets: nothing to show
    }
    if b.len() < UPDATE_SIZE {
        st.warning = Some(format!("Short WRC packet ({} bytes). Use the default \"wrc\" structure.", b.len()));
        return;
    }
    st.warning = None;

    st.shift_frac = (u8_at(b, 24) != 0).then(|| f32_at(b, 12).clamp(0.0, 1.0));
    let gear = u8_at(b, 25);
    let neutral = u8_at(b, 26);
    let reverse = u8_at(b, 27);
    st.gear = if gear == neutral {
        0
    } else if gear == reverse {
        -1
    } else {
        gear as i32
    };
    st.speed_ms = f32_at(b, 29);
    let (x, y, z) = (f32_at(b, 37), f32_at(b, 41), f32_at(b, 45));
    st.world = Some((x, y, z));
    let (vx, vz) = (f32_at(b, 49), f32_at(b, 57));
    let acc = (f32_at(b, 61), f32_at(b, 65), f32_at(b, 69));
    let left = (f32_at(b, 73), f32_at(b, 77), f32_at(b, 81));
    let fwd = (f32_at(b, 85), f32_at(b, 89), f32_at(b, 93));
    let dot = |a: (f32, f32, f32), b: (f32, f32, f32)| a.0 * b.0 + a.1 * b.1 + a.2 * b.2;
    st.g_lat = -dot(acc, left) / 9.81;
    st.g_long = dot(acc, fwd) / 9.81;
    let yaw = fwd.0.atan2(fwd.2);
    st.yaw_rate = (yaw - st.yaw) * 60.0;
    st.yaw = yaw;
    let mut drift = (vx.atan2(vz) - yaw).to_degrees();
    if drift > 180.0 {
        drift -= 360.0;
    }
    if drift < -180.0 {
        drift += 360.0;
    }
    st.drift_deg = if st.speed_ms > 4.0 { drift } else { 0.0 };

    for (pkt, &w) in WHEEL.iter().enumerate() {
        let hub = f32_at(b, 109 + pkt * 4); // metres, relative to rest
        st.corners[w].susp = Some((0.5 + hub * 5.0).clamp(0.0, 1.0));
        let cp_speed = f32_at(b, 141 + pkt * 4);
        if st.speed_ms > 3.0 {
            st.corners[w].slip = Some(((cp_speed - st.speed_ms).abs() / st.speed_ms * 4.0).min(2.0));
        }
        let brake = f32_at(b, 157 + pkt * 4);
        st.corners[w].brake_c = (brake > 0.0).then_some(brake);
    }

    let max_rpm = f32_at(b, 173);
    if max_rpm > 1000.0 {
        st.max_rpm = max_rpm;
    }
    st.rpm = f32_at(b, 181);
    st.throttle = f32_at(b, 185);
    st.brake = f32_at(b, 189);
    st.clutch = f32_at(b, 193);
    st.steer = f32_at(b, 197);
    st.handbrake = f32_at(b, 201);

    let stage_time = f32_at(b, 221);
    if stage_time + 0.5 < st.aux_time as f32 {
        st.new_session();
    }
    st.aux_time = stage_time as f64;
    st.stage_time_s = Some(stage_time.max(0.0));
    st.current_lap_s = Some(stage_time.max(0.0));
    let dist = f64_at(b, 225) as f32;
    st.stage_dist_m = Some(dist.max(0.0));
    st.distance_m = Some(dist.max(0.0));
    let len = f32_at(b, 233);
    st.stage_len_m = (len > 10.0).then_some(len);
    st.session_name = "STAGE".into();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::games::sim::Game;

    #[test]
    fn parses_session_update() {
        let mut b = vec![0u8; UPDATE_SIZE];
        b[0..4].copy_from_slice(b"sesu");
        b[25] = 3;
        b[26] = 0;
        b[27] = 9;
        b[29..33].copy_from_slice(&25.0f32.to_le_bytes());
        b[181..185].copy_from_slice(&5400.0f32.to_le_bytes());
        b[201..205].copy_from_slice(&1.0f32.to_le_bytes());
        b[225..233].copy_from_slice(&1234.5f64.to_le_bytes());
        b[233..237].copy_from_slice(&10500.0f32.to_le_bytes());
        let mut st = SimState::new(Game::Wrc);
        handle(&mut st, &b);
        assert_eq!(st.gear, 3);
        assert!((st.rpm - 5400.0).abs() < 1e-3);
        assert_eq!(st.handbrake, 1.0);
        assert!((st.stage_dist_m.unwrap() - 1234.5).abs() < 1e-3);
        assert_eq!(st.stage_len_m, Some(10500.0));
    }
}

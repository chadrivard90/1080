//! DiRT Rally 2.0 UDP ("extradata=3"): 66 little-endian f32 values, 264 bytes.
//! Wheel arrays in this format are ordered RL, RR, FL, FR.
//! DR2 does not send water temperature, oil pressure, fuel or tyre data.

use super::bytes::f32_at;
use super::sim::SimState;

fn f(b: &[u8], idx: usize) -> f32 {
    f32_at(b, idx * 4)
}

/// RL, RR, FL, FR packet order -> FL, FR, RL, RR state order.
const WHEEL: [usize; 4] = [2, 3, 0, 1];

pub fn handle(st: &mut SimState, b: &[u8]) {
    if b.len() < 256 {
        st.warning = Some(format!("Packet is {} bytes; set extradata=\"3\" in the DR2 UDP config.", b.len()));
        return;
    }
    st.warning = None;

    let stage_time = f(b, 1);
    let distance = f(b, 2);
    // Restarted / new stage: stage clock went backwards.
    if stage_time + 0.5 < st.aux_time as f32 {
        st.new_session();
    }
    st.aux_time = stage_time as f64;

    let (x, y, z) = (f(b, 4), f(b, 5), f(b, 6));
    st.world = Some((x, y, z));
    st.speed_ms = f(b, 7);
    let (vx, vz) = (f(b, 8), f(b, 10));
    // "Pitch" vector (14..16) is the car's forward direction.
    let (fx, fz) = (f(b, 14), f(b, 16));
    let yaw = fx.atan2(fz);
    st.yaw_rate = (yaw - st.yaw) * 60.0;
    st.yaw = yaw;
    let vel_heading = vx.atan2(vz);
    let mut drift = (vel_heading - yaw).to_degrees();
    if drift > 180.0 {
        drift -= 360.0;
    }
    if drift < -180.0 {
        drift += 360.0;
    }
    st.drift_deg = if st.speed_ms > 4.0 { drift } else { 0.0 };

    for (pkt, &w) in WHEEL.iter().enumerate() {
        let travel = f(b, 17 + pkt); // mm
        st.corners[w].susp = Some(((travel + 60.0) / 160.0).clamp(0.0, 1.0));
        let brake = f(b, 51 + pkt);
        st.corners[w].brake_c = (brake > 0.0).then_some(brake);
        // Wheel slip from wheel speed vs car speed.
        let wheel_speed = f(b, 25 + pkt);
        if st.speed_ms > 3.0 {
            st.corners[w].slip = Some(((wheel_speed - st.speed_ms).abs() / st.speed_ms * 4.0).min(2.0));
        }
    }

    st.throttle = f(b, 29);
    st.steer = f(b, 30);
    st.brake = f(b, 31);
    st.clutch = f(b, 32);
    let gear = f(b, 33).round() as i32;
    st.gear = if gear >= 10 { -1 } else { gear };
    st.g_lat = f(b, 34);
    st.g_long = f(b, 35);
    st.rpm = f(b, 37) * 10.0;
    st.position = f(b, 39).max(0.0) as u32;
    let max_rpm = f(b, 63) * 10.0;
    if max_rpm > 1000.0 {
        st.max_rpm = max_rpm;
    }

    st.stage_time_s = Some(stage_time.max(0.0));
    st.stage_dist_m = Some(distance.max(0.0));
    let length = f(b, 61);
    st.stage_len_m = (length > 10.0).then_some(length);
    st.current_lap_s = Some(stage_time.max(0.0));
    let last = f(b, 62);
    st.last_lap_s = (last > 0.0).then_some(last);
    st.session_name = "STAGE".into();
    st.distance_m = Some(distance.max(0.0));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::games::sim::Game;

    #[test]
    fn parses_core_fields() {
        let mut b = vec![0u8; 264];
        let mut set = |i: usize, v: f32| b[i * 4..i * 4 + 4].copy_from_slice(&v.to_le_bytes());
        set(1, 12.5);
        set(2, 800.0);
        set(7, 30.0);
        set(33, 4.0);
        set(37, 650.0);
        set(53, 410.0); // FL brake temp (packet index 2)
        set(61, 9000.0);
        let mut st = SimState::new(Game::Dr2);
        handle(&mut st, &b);
        assert_eq!(st.gear, 4);
        assert!((st.rpm - 6500.0).abs() < 1e-3);
        assert_eq!(st.corners[0].brake_c, Some(410.0));
        assert_eq!(st.stage_len_m, Some(9000.0));
    }
}

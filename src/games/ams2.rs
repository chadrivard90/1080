//! Automobilista 2 UDP in "Project CARS 2" format (SMS_UDP_Definitions.hpp, port 5606).
//!
//! Packets share a 12-byte header; byte 10 is the packet type:
//!   0 telemetry (559), 1 race definition (308), 2 participant names (1136),
//!   3 timings (1063), 4 game state (24), 7 time stats (1040).
//! Wheel arrays are FL, FR, RL, RR.

use super::bytes::*;
use super::sim::{Car, Flag, SimState};

const PARTICIPANTS: usize = 32;

/// Temperatures sent as u16 are Kelvin in the PC2 data set; small values are already Celsius.
fn kelvin_or_c(v: u16) -> Option<f32> {
    match v {
        0 => None,
        v if v > 200 => Some(v as f32 - 273.15),
        v => Some(v as f32),
    }
}

pub fn handle(st: &mut SimState, b: &[u8]) {
    if b.len() < 12 {
        return;
    }
    match u8_at(b, 10) {
        0 if b.len() >= 559 => telemetry(st, b),
        1 if b.len() >= 308 => race_definition(st, b),
        2 if b.len() >= 1136 => participant_names(st, b),
        3 if b.len() >= 1063 => timings(st, b),
        4 if b.len() >= 23 => game_state(st, b),
        7 if b.len() >= 1040 => time_stats(st, b),
        _ => {}
    }
}

fn telemetry(st: &mut SimState, b: &[u8]) {
    let viewed = i8_at(b, 12);
    st.aux_lap = viewed as i32; // remember which participant is us
    let car_flags = u8_at(b, 17);
    st.pit_limiter = car_flags & 0x08 != 0;
    st.abs_active = car_flags & 0x10 != 0;
    st.oil_c = Some(i16_at(b, 18) as f32).filter(|v| *v > 0.0);
    st.oil_bar = Some(u16_at(b, 20) as f32 / 100.0).filter(|v| *v > 0.0);
    st.water_c = Some(i16_at(b, 22) as f32).filter(|v| *v > 0.0);
    let capacity = u8_at(b, 28) as f32;
    st.brake = u8_at(b, 29) as f32 / 255.0;
    st.throttle = u8_at(b, 30) as f32 / 255.0;
    st.clutch = u8_at(b, 31) as f32 / 255.0;
    let fuel_frac = f32_at(b, 32).clamp(0.0, 1.0);
    st.fuel_frac = Some(fuel_frac);
    st.fuel_l = (capacity > 0.0).then_some(fuel_frac * capacity);
    st.speed_ms = f32_at(b, 36);
    st.rpm = u16_at(b, 40) as f32;
    st.max_rpm = u16_at(b, 42) as f32;
    st.steer = i8_at(b, 44) as f32 / 127.0;
    let gear = (u8_at(b, 45) & 0x0F) as i32;
    st.gear = if gear == 15 { -1 } else { gear };
    st.distance_m = Some(f32_at(b, 48) * 1000.0);
    // Orientation: x = pitch, y = heading, z = roll (radians).
    let heading = f32_at(b, 56);
    st.yaw_rate = f32_at(b, 92);
    st.yaw = heading;
    let (lvx, lvz) = (f32_at(b, 64), f32_at(b, 72));
    let ground = (lvx * lvx + lvz * lvz).sqrt();
    st.drift_deg = if ground > 4.0 { lvx.atan2(lvz.abs()).to_degrees() } else { 0.0 };
    st.g_lat = f32_at(b, 100) / 9.81;
    st.g_long = -f32_at(b, 108) / 9.81;

    for w in 0..4 {
        let c = &mut st.corners[w];
        let surface = u8_at(b, 176 + w) as f32;
        c.surface_c = (surface > 0.0).then_some(surface);
        c.wear_pct = Some(u8_at(b, 196 + w) as f32 / 255.0 * 100.0);
        c.brake_c = Some(i16_at(b, 208 + w * 2) as f32);
        c.inner_c = kelvin_or_c(u16_at(b, 232 + w * 2)); // carcass
        let l = kelvin_or_c(u16_at(b, 256 + w * 2));
        let m = kelvin_or_c(u16_at(b, 264 + w * 2));
        let r = kelvin_or_c(u16_at(b, 272 + w * 2));
        c.imo = match (l, m, r) {
            (Some(l), Some(m), Some(r)) => Some([l, m, r]),
            _ => None,
        };
        let travel = f32_at(b, 312 + w * 4); // metres
        c.susp = Some((travel / 0.12).clamp(0.0, 1.0));
        let air = u16_at(b, 352 + w * 2) as f32; // kPa
        let kpa = if air > 1000.0 { air / 10.0 } else { air };
        c.pressure_psi = (air > 0.0).then_some(kpa * 0.145_038);
    }
    st.handbrake = u8_at(b, 370) as f32 / 255.0;
    let boost = f32_at(b, 538);
    st.boost_psi = (boost > 0.0).then_some(boost * 0.145_038);
    let (x, y, z) = (f32_at(b, 542), f32_at(b, 546), f32_at(b, 550));
    if x != 0.0 || z != 0.0 {
        st.world = Some((x, y, z));
    }
    let bias = u8_at(b, 554) as f32;
    st.brake_bias = (bias > 0.0).then(|| if bias <= 100.0 { bias } else { bias / 255.0 * 100.0 });
}

fn race_definition(st: &mut SimState, b: &[u8]) {
    let length = f32_at(b, 44);
    if length > 0.0 {
        st.aux_dist = length;
    }
    let location = str_at(b, 48, 64);
    let variation = str_at(b, 112, 64);
    let name = if variation.is_empty() { location } else { format!("{location} {variation}") };
    st.track_name = name.to_uppercase();
    let laps_time = u16_at(b, 304);
    st.total_laps = (laps_time & 0x8000 == 0 && laps_time > 0).then_some(laps_time as i32);
}

fn participant_names(st: &mut SimState, b: &[u8]) {
    for k in 0..16 {
        let name = str_at(b, 16 + k * 64, 64);
        let idx = u16_at(b, 1104 + k * 2) as usize;
        if name.is_empty() || idx >= 64 {
            continue;
        }
        if st.names.len() <= idx {
            st.names.resize(idx + 1, String::new());
        }
        st.names[idx] = name.to_uppercase();
    }
}

fn timings(st: &mut SimState, b: &[u8]) {
    let num = (i8_at(b, 12).max(0) as usize).min(PARTICIPANTS);
    let remaining = f32_at(b, 17);
    st.time_left_s = (remaining > 0.0).then_some(remaining);
    let ahead = f32_at(b, 21);
    let behind = f32_at(b, 25);
    st.gap_ahead_s = (ahead > 0.0 && ahead < 600.0).then_some(ahead);
    st.gap_behind_s = (behind > 0.0 && behind < 600.0).then_some(behind);
    let me = st.aux_lap.max(0) as usize;
    let track_len = st.aux_dist;

    let mut cars = Vec::with_capacity(num);
    let mut active = 0u32;
    for i in 0..num {
        let o = 33 + i * 32;
        let race_pos_raw = u8_at(b, o + 14);
        if race_pos_raw & 0x80 == 0 {
            continue; // inactive slot
        }
        active += 1;
        let race_pos = (race_pos_raw & 0x7F) as u32;
        let wx = i16_at(b, o) as f32;
        let wz = i16_at(b, o + 4) as f32;
        let lap_dist = u16_at(b, o + 12) as f32;
        let flag = u8_at(b, o + 16);
        let pit_mode = u8_at(b, o + 17) & 0x07;
        let race_state = u8_at(b, o + 20);
        let lap = u8_at(b, o + 21) as i32;
        let current_time = f32_at(b, o + 22);
        let mp_index = u16_at(b, o + 30) as usize;
        let player = i == me;
        let name = st
            .names
            .get(mp_index)
            .filter(|n| !n.is_empty())
            .cloned()
            .unwrap_or_else(|| format!("CAR {}", i + 1));
        let prev = st.cars.iter().find(|c| c.name == name);
        let (best_s, last_s) = prev.map_or((None, None), |c| (c.best_s, c.last_s));
        if player {
            st.position = race_pos;
            st.lap = lap;
            st.current_lap_s = (current_time > 0.0).then_some(current_time);
            st.lap_invalid = race_state & 0x08 != 0;
            st.in_pit = (1..=4).contains(&pit_mode);
            st.pit_text = match pit_mode {
                1 => "PIT ENTRY",
                2 => "IN PIT BOX",
                3 => "PIT EXIT",
                4 => "IN GARAGE",
                _ => "",
            }
            .into();
            st.flag = match flag & 0x0F {
                2 => Flag::Blue,
                3 | 4 => Flag::White,
                5 => Flag::Red,
                6 | 7 => Flag::Yellow,
                8 | 10 => Flag::Black,
                11 => Flag::Checkered,
                _ => Flag::None,
            };
            if track_len > 0.0 {
                st.lap_pct = Some((lap_dist / track_len).clamp(0.0, 1.0));
            }
        }
        cars.push(Car {
            pos: Some((wx, wz)),
            pct: (track_len > 0.0).then(|| (lap_dist / track_len).clamp(0.0, 1.0)),
            race_pos,
            name,
            number: String::new(),
            lap,
            player,
            in_pit: (1..=4).contains(&pit_mode),
            gap_leader_s: None,
            interval_s: None,
            best_s,
            last_s,
        });
    }
    st.num_cars = active;
    st.cars = cars;
}

fn game_state(st: &mut SimState, b: &[u8]) {
    let session = (u8_at(b, 15) >> 3) & 0x07;
    st.session_name = match session {
        1 => "PRACTICE",
        2 => "TEST",
        3 => "QUALIFYING",
        4 => "FORMATION LAP",
        5 => "RACE",
        6 => "TIME ATTACK",
        _ => "SESSION",
    }
    .into();
    st.air_c = Some(i8_at(b, 16) as f32);
    st.track_c = Some(i8_at(b, 17) as f32);
    let rain = u8_at(b, 18) as f32 / 255.0 * 100.0;
    st.rain_pct = Some(rain);
    st.wind_ms = Some(i8_at(b, 20).max(0) as f32);
    let (wx, wy) = (i8_at(b, 21) as f32, i8_at(b, 22) as f32);
    st.wind_deg = (wx != 0.0 || wy != 0.0).then(|| wx.atan2(wy).to_degrees());
    let weather = if u8_at(b, 19) > 0 {
        "SNOW"
    } else if rain > 60.0 {
        "HEAVY RAIN"
    } else if rain > 20.0 {
        "RAIN"
    } else if rain > 2.0 {
        "DRIZZLE"
    } else {
        "DRY"
    };
    st.weather = Some(weather.to_string());
}

fn time_stats(st: &mut SimState, b: &[u8]) {
    let me = st.aux_lap.max(0) as usize;
    for i in 0..PARTICIPANTS {
        let o = 16 + i * 32;
        let best = f32_at(b, o);
        let last = f32_at(b, o + 4);
        if i == me {
            st.best_lap_s = (best > 0.0).then_some(best);
            st.last_lap_s = (last > 0.0).then_some(last);
        }
        let mp_index = u16_at(b, o + 28) as usize;
        if let Some(name) = st.names.get(mp_index).cloned() {
            if let Some(car) = st.cars.iter_mut().find(|c| c.name == name) {
                car.best_s = (best > 0.0).then_some(best);
                car.last_s = (last > 0.0).then_some(last);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::games::sim::Game;

    #[test]
    fn parses_telemetry() {
        let mut b = vec![0u8; 559];
        b[10] = 0;
        b[36..40].copy_from_slice(&40.0f32.to_le_bytes());
        b[40..42].copy_from_slice(&7200u16.to_le_bytes());
        b[42..44].copy_from_slice(&8000u16.to_le_bytes());
        b[45] = 0x63; // 6 gears, 3rd
        b[232..234].copy_from_slice(&(273u16 + 90).to_le_bytes()); // FL carcass 90 C
        let mut st = SimState::new(Game::Ams2);
        handle(&mut st, &b);
        assert_eq!(st.gear, 3);
        assert!((st.rpm - 7200.0).abs() < 1e-3);
        assert!((st.corners[0].inner_c.unwrap() - 89.85).abs() < 0.01);
    }

    #[test]
    fn parses_timings_positions() {
        let mut b = vec![0u8; 1063];
        b[10] = 3;
        b[12] = 2;
        // participant 0 active, P1
        b[33 + 14] = 0x81;
        b[33..35].copy_from_slice(&100i16.to_le_bytes());
        // participant 1 active, P2
        b[65 + 14] = 0x82;
        let mut st = SimState::new(Game::Ams2);
        st.aux_lap = 1;
        handle(&mut st, &b);
        assert_eq!(st.num_cars, 2);
        assert_eq!(st.position, 2);
        assert_eq!(st.cars[0].pos, Some((100.0, 0.0)));
    }
}

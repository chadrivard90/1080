//! Receives packets from `race-dash-bridge.exe` (see /bridge), which reads shared-memory games
//! (Assetto Corsa EVO, iRacing-engine NASCAR) on the gaming PC and broadcasts them to the Pi.
//!
//! Protocol: UTF-8 text, first line "RDB1", then one `key=value` per line. Every key is optional.
//! Per-wheel values are 4 comma-separated numbers in FL,FR,RL,RR order; per-car lists are
//! comma-separated (names use '|'). See bridge/src/main.rs for the sender.

use std::collections::HashMap;

use super::sim::{Car, Flag, Game, SimState};

fn num(m: &HashMap<&str, &str>, k: &str) -> Option<f32> {
    m.get(k).and_then(|v| v.trim().parse::<f32>().ok()).filter(|v| v.is_finite())
}

fn list(m: &HashMap<&str, &str>, k: &str) -> Vec<f32> {
    m.get(k)
        .map(|v| v.split(',').map(|s| s.trim().parse::<f32>().unwrap_or(f32::NAN)).collect())
        .unwrap_or_default()
}

fn four(m: &HashMap<&str, &str>, k: &str) -> [Option<f32>; 4] {
    let v = list(m, k);
    let get = |i: usize| v.get(i).copied().filter(|x| x.is_finite());
    [get(0), get(1), get(2), get(3)]
}

pub fn handle(st: &mut SimState, b: &[u8]) {
    let Ok(text) = std::str::from_utf8(b) else { return };
    let mut lines = text.lines();
    if lines.next().map(str::trim) != Some("RDB1") {
        return;
    }
    let m: HashMap<&str, &str> = lines.filter_map(|l| l.split_once('=')).collect();

    let source = m.get("game").copied().unwrap_or("");
    let expected = if st.game == Game::AcEvo { "acevo" } else { "nascar" };
    if source != expected {
        st.warning = Some(format!(
            "The bridge is sending '{source}' data. Pick that game on the title screen."
        ));
        return;
    }
    st.warning = None;

    // Session time drives dead reckoning for games without world coordinates.
    let t = num(&m, "t").map(|v| v as f64);
    let dt = t.map_or(0.0, |t| (t - st.aux_time) as f32);
    if let Some(t) = t {
        if t + 1.0 < st.aux_time {
            st.new_session();
            st.aux_pos = (0.0, 0.0);
        }
        st.aux_time = t;
    }

    if let Some(v) = num(&m, "speed") {
        st.speed_ms = v;
    }
    if let Some(v) = num(&m, "gear") {
        st.gear = v as i32;
    }
    if let Some(v) = num(&m, "rpm") {
        st.rpm = v;
    }
    if let Some(v) = num(&m, "maxrpm") {
        st.max_rpm = v;
    }
    st.shift_frac = num(&m, "shift");
    st.throttle = num(&m, "thr").unwrap_or(0.0);
    st.brake = num(&m, "brk").unwrap_or(0.0);
    st.clutch = num(&m, "clu").unwrap_or(0.0);
    st.steer = num(&m, "steer").unwrap_or(0.0);
    st.fuel_l = num(&m, "fuel");
    st.fuel_frac = num(&m, "fuelpct");
    st.fuel_laps = num(&m, "fuellaps");
    st.water_c = num(&m, "water");
    st.oil_c = num(&m, "oilt");
    st.oil_bar = num(&m, "oilp");
    st.boost_psi = num(&m, "boost");
    st.tc_active = num(&m, "tc_act").unwrap_or(0.0) > 0.5;
    st.abs_active = num(&m, "abs_act").unwrap_or(0.0) > 0.5;
    st.tc_level = num(&m, "tc").map(|v| v as i32);
    st.abs_level = num(&m, "abs").map(|v| v as i32);
    st.brake_bias = num(&m, "bias");
    st.pit_limiter = num(&m, "limiter").unwrap_or(0.0) > 0.5;
    st.in_pit = num(&m, "pit").unwrap_or(0.0) > 0.5;
    st.pit_text = m.get("pittext").copied().unwrap_or("").to_string();
    st.incidents = num(&m, "incidents").map(|v| v as i32);

    let surface = four(&m, "tyre_s");
    let core = four(&m, "tyre_c");
    let (ti, tm, to) = (four(&m, "tyre_i"), four(&m, "tyre_m"), four(&m, "tyre_o"));
    let press = four(&m, "tyre_p");
    let wear = four(&m, "tyre_w");
    let brakes = four(&m, "brake_t");
    for w in 0..4 {
        let c = &mut st.corners[w];
        c.surface_c = surface[w];
        c.inner_c = core[w];
        c.imo = match (ti[w], tm[w], to[w]) {
            (Some(a), Some(b), Some(c)) => Some([a, b, c]),
            _ => None,
        };
        c.pressure_psi = press[w];
        c.wear_pct = wear[w];
        c.brake_c = brakes[w];
    }

    st.position = num(&m, "pos").map_or(0, |v| v.max(0.0) as u32);
    st.num_cars = num(&m, "ncars").map_or(0, |v| v.max(0.0) as u32);
    st.lap = num(&m, "lap").map_or(0, |v| v as i32);
    st.total_laps = num(&m, "laps").map(|v| v as i32).filter(|v| *v > 0);
    st.current_lap_s = num(&m, "cur").filter(|v| *v > 0.0);
    st.last_lap_s = num(&m, "last").filter(|v| *v > 0.0);
    st.best_lap_s = num(&m, "best").filter(|v| *v > 0.0);
    st.native_delta_s = num(&m, "delta");
    st.lap_invalid = num(&m, "invalid").unwrap_or(0.0) > 0.5;
    st.session_name = m.get("session").copied().unwrap_or("").to_uppercase();
    st.track_name = m.get("track").copied().unwrap_or("").to_uppercase();
    st.time_left_s = num(&m, "timeleft").filter(|v| *v > 0.0);
    st.gap_ahead_s = num(&m, "gapa").filter(|v| *v > 0.0);
    st.gap_behind_s = num(&m, "gapb").filter(|v| *v > 0.0);
    st.flag = match m.get("flag").copied().unwrap_or("") {
        "green" => Flag::Green,
        "yellow" => Flag::Yellow,
        "blue" => Flag::Blue,
        "white" => Flag::White,
        "red" => Flag::Red,
        "checkered" => Flag::Checkered,
        "black" => Flag::Black,
        "caution" => Flag::Caution,
        _ => Flag::None,
    };
    st.spotter_left = num(&m, "left").map_or(0, |v| v.clamp(0.0, 2.0) as u8);
    st.spotter_right = num(&m, "right").map_or(0, |v| v.clamp(0.0, 2.0) as u8);
    st.air_c = num(&m, "air");
    st.track_c = num(&m, "road");
    st.rain_pct = num(&m, "rain");
    st.g_lat = num(&m, "glat").unwrap_or(0.0);
    st.g_long = num(&m, "glong").unwrap_or(0.0);
    if let Some(y) = num(&m, "yaw") {
        st.yaw = y;
    }
    st.yaw_rate = num(&m, "yawrate").unwrap_or(0.0);
    st.lap_pct = num(&m, "pct").filter(|p| (0.0..=1.0).contains(p));

    // Player position: real coordinates if sent, else dead-reckoned from speed + heading.
    match (num(&m, "x"), num(&m, "z")) {
        (Some(x), Some(z)) => st.world = Some((x, num(&m, "y").unwrap_or(0.0), z)),
        _ => {
            if dt > 0.0 && dt < 0.5 {
                st.aux_pos.0 += st.speed_ms * st.yaw.sin() * dt;
                st.aux_pos.1 += st.speed_ms * st.yaw.cos() * dt;
            }
            st.world = Some((st.aux_pos.0, 0.0, st.aux_pos.1));
        }
    }

    // Field.
    let player_idx = num(&m, "player").map(|v| v as i64).unwrap_or(-1);
    let (cx, cz, cpct) = (list(&m, "cx"), list(&m, "cz"), list(&m, "cpct"));
    let (cpos, clap, cpit) = (list(&m, "cpos"), list(&m, "clap"), list(&m, "cpit"));
    let names: Vec<&str> = m.get("cname").map(|v| v.split('|').collect()).unwrap_or_default();
    let nums: Vec<&str> = m.get("cnum").map(|v| v.split('|').collect()).unwrap_or_default();
    let count = cx.len().max(cpct.len()).max(cpos.len());
    st.cars = (0..count)
        .filter_map(|i| {
            let pos = match (cx.get(i), cz.get(i)) {
                (Some(&x), Some(&z)) if x.is_finite() && z.is_finite() && (x != 0.0 || z != 0.0) => Some((x, z)),
                _ => None,
            };
            let pct = cpct.get(i).copied().filter(|p| (0.0..=1.0).contains(p));
            let race_pos = cpos.get(i).copied().filter(|v| v.is_finite()).map_or(0, |v| v.max(0.0) as u32);
            if pos.is_none() && pct.is_none() {
                return None;
            }
            Some(Car {
                pos,
                pct,
                race_pos,
                name: names.get(i).map_or_else(|| format!("CAR {}", i + 1), |n| n.to_uppercase()),
                number: nums.get(i).map_or_else(String::new, |n| n.trim().to_string()),
                lap: clap.get(i).copied().filter(|v| v.is_finite()).map_or(0, |v| v as i32),
                player: i as i64 == player_idx,
                in_pit: cpit.get(i).copied().unwrap_or(0.0) > 0.5,
                gap_leader_s: None,
                interval_s: None,
                best_s: None,
                last_s: None,
            })
        })
        .collect();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_bridge_text() {
        let pkt = "RDB1\ngame=nascar\nt=10\nspeed=80\ngear=4\nrpm=9000\nleft=1\nflag=caution\ntyre_s=90,95,88,93\ncpct=0.1,0.5\ncpos=2,1\ncname=A|B\nplayer=0\npct=0.1\n";
        let mut st = SimState::new(Game::Nascar);
        handle(&mut st, pkt.as_bytes());
        assert_eq!(st.gear, 4);
        assert_eq!(st.spotter_left, 1);
        assert_eq!(st.flag, Flag::Caution);
        assert_eq!(st.corners[1].surface_c, Some(95.0));
        assert_eq!(st.cars.len(), 2);
        assert!(st.cars[0].player);
        assert_eq!(st.cars[1].race_pos, 1);

        let mut ace = SimState::new(Game::AcEvo);
        handle(&mut ace, pkt.as_bytes());
        assert!(ace.warning.is_some());
    }
}

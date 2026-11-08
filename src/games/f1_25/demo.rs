//! Demo mode: simulates a race at Silverstone so the dash can be tried without the game.
//! Toggle it with the DEMO button on the title screen.

use std::f32::consts::TAU;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use super::packets::*;

const TRACK_LEN: f32 = 5891.0;
const CARS: usize = 20;
const PLAYER: usize = 3;

const NAMES: [&str; CARS] = [
    "NOVAK", "OKAFOR", "LINDQVIST", "MORETTI", "TANAKA", "DUBOIS", "KOWALSKI", "SILVA", "HARTMANN", "REYES",
    "BRENNAN", "IVANOV", "CHEN", "PETERSEN", "ALVAREZ", "MEYER", "SANTOS", "PARK", "BERNARD", "WALSH",
];
const TEAMS: [u8; CARS] = [8, 2, 1, 8, 0, 1, 2, 0, 4, 3, 5, 6, 7, 9, 4, 3, 5, 6, 7, 9];

fn event(st: &mut F1State, text: &str, kind: EventKind) {
    st.event = Some(EventMsg { text: text.to_string(), kind, at: Instant::now() });
}

pub fn run(shared: Arc<Mutex<F1State>>, stop: Arc<AtomicBool>) {
    {
        let Ok(mut st) = shared.lock() else { return };
        st.session_uid = 1;
        st.player = PLAYER;
        st.num_active = CARS as u8;
        st.session = Session {
            received: true,
            weather: 1,
            track_temp: 34,
            air_temp: 22,
            total_laps: 30,
            track_length: TRACK_LEN as u16,
            session_type: 15,
            track_id: 7,
            time_left_s: 5400,
            duration_s: 7200,
            pit_speed_limit: 80,
            pit_ideal_lap: 14,
            pit_latest_lap: 19,
            pit_rejoin_pos: 9,
            speed_units: 1,
            temp_units: 0,
            forecast: [(0, 1, 0), (5, 1, 5), (10, 2, 12), (15, 2, 24), (30, 3, 55), (45, 3, 60)]
                .iter()
                .map(|&(t, w, r)| WeatherSample {
                    session_type: 15,
                    time_offset_min: t,
                    weather: w,
                    track_temp: 34 - (t / 10) as i8,
                    air_temp: 22,
                    rain_pct: r,
                })
                .collect(),
            ..Default::default()
        };
        st.setup = CarSetup { diff_on: 50, engine_braking: 60, brake_bias: 56, ..Default::default() };
        for i in 0..CARS {
            st.participants[i] =
                Participant { ai: i != PLAYER, team_id: TEAMS[i], name: NAMES[i].to_string(), ..Default::default() };
            let best = 87_400 + i as u32 * 140 + (i as u32 % 3) * 37;
            st.history[i] = CarHistory {
                num_laps: 2,
                best_lap_num: 1,
                best_sector_lap: [1, 1, 1],
                laps: vec![LapHistory {
                    time_ms: best,
                    sectors_ms: [best * 30 / 100, best * 37 / 100, best - best * 30 / 100 - best * 37 / 100],
                    flags: 0x0F,
                }],
                stints: Vec::new(),
            };
            st.status[i].visual_compound = [16, 17, 18][i % 3];
            st.status[i].tyre_age_laps = (i % 7) as u8 + 2;
        }
        // The player starts with no laps; their history fills in as demo laps complete.
        st.history[PLAYER] = CarHistory::default();
    }

    let start = Instant::now();
    let mut lap_num: u8 = 1;
    let mut lap_start = 0.0f32;
    let mut lap_len = 88.6f32;
    let mut player_laps: Vec<LapHistory> = Vec::new();
    let mut last_lap_ms = 0u32;
    let mut s1_ms = 0u32;
    let mut s2_ms = 0u32;
    let mut last_event_slot = u64::MAX;

    while !stop.load(Ordering::Relaxed) {
        std::thread::sleep(Duration::from_millis(16));
        let t = start.elapsed().as_secs_f32();
        let Ok(mut st) = shared.lock() else { return };
        st.last_packet = Some(Instant::now());

        // --- lap progression -------------------------------------------------------------
        if t - lap_start >= lap_len {
            last_lap_ms = (lap_len * 1000.0) as u32;
            let invalid = lap_num % 5 == 0;
            let s3 = last_lap_ms.saturating_sub(s1_ms + s2_ms);
            player_laps.push(LapHistory {
                time_ms: last_lap_ms,
                sectors_ms: [s1_ms, s2_ms, s3],
                flags: if invalid { 0x00 } else { 0x0F },
            });
            lap_start += lap_len;
            lap_num = lap_num.saturating_add(1);
            lap_len = 88.2 + 0.7 * (lap_num as f32 * 1.7).sin();
            s1_ms = 0;
            s2_ms = 0;
        }
        let in_lap = t - lap_start;
        let frac = (in_lap / lap_len).clamp(0.0, 1.0);
        let wobble = 0.004 * (frac * TAU * 3.0 + lap_num as f32).sin();
        let dist = ((frac + wobble).clamp(0.0, 1.0)) * TRACK_LEN;
        let sector = if frac < 0.30 { 0 } else if frac < 0.67 { 1 } else { 2 };
        if sector >= 1 && s1_ms == 0 {
            s1_ms = (in_lap * 1000.0) as u32;
        }
        if sector >= 2 && s2_ms == 0 {
            s2_ms = ((in_lap * 1000.0) as u32).saturating_sub(s1_ms);
        }

        // Player history (for best lap / sectors / lap list).
        {
            let mut laps = player_laps.clone();
            laps.push(LapHistory::default()); // current partial lap
            let valid: Vec<(usize, &LapHistory)> =
                player_laps.iter().enumerate().filter(|(_, l)| l.valid()).collect();
            let best_of = |f: &dyn Fn(&LapHistory) -> u32| -> u8 {
                valid.iter().min_by_key(|(_, l)| f(l)).map_or(0, |(i, _)| *i as u8 + 1)
            };
            let best_lap_num = best_of(&|l| l.time_ms);
            let best_sector_lap = [best_of(&|l| l.sectors_ms[0]), best_of(&|l| l.sectors_ms[1]), best_of(&|l| l.sectors_ms[2])];
            st.history[PLAYER] = CarHistory {
                num_laps: laps.len() as u8,
                best_lap_num,
                best_sector_lap,
                laps,
                stints: Vec::new(),
            };
        }

        // --- car physics-ish -------------------------------------------------------------
        let phase = frac * TAU * 6.0;
        let speed = (205.0 + 120.0 * phase.sin() + 25.0 * (phase * 2.3).sin()).clamp(70.0, 335.0);
        let accel = 120.0 * phase.cos() + 57.5 * (phase * 2.3).cos();
        let throttle = if accel > -20.0 { (0.7 + accel / 400.0).clamp(0.4, 1.0) } else { 0.0 };
        let brake = if accel < -60.0 { ((-accel - 60.0) / 100.0).clamp(0.0, 1.0) } else { 0.0 };
        let gear = (1.0 + (speed - 60.0) / 36.0).floor().clamp(1.0, 8.0) as i8;
        let gear_floor = 60.0 + (gear as f32 - 1.0) * 36.0;
        let rpm = (9_800.0 + (speed - gear_floor) / 36.0 * 3_000.0).clamp(4_000.0, 12_900.0);
        let lit = (((rpm - 10_600.0) / 2_000.0) * 15.0).clamp(0.0, 15.0) as u32;
        let rev_bits = ((1u32 << lit) - 1) as u16;
        let drs_zone = (0.05..0.16).contains(&frac) || (0.55..0.63).contains(&frac);
        let lateral = 3.8 * (phase * 0.5).sin();

        let wear_base = 3.0 + (lap_num as f32 - 1.0 + frac) * 2.1;
        let heat = |offset: f32| 96.0 + 7.0 * (t * 0.4 + offset).sin() + 6.0 * brake;
        st.telemetry[PLAYER] = CarTelemetry {
            speed_kph: speed as u16,
            throttle,
            steer: (phase * 0.5).sin() * 0.6,
            brake,
            clutch: 0,
            gear,
            rpm: rpm as u16,
            drs: (drs_zone && t % 60.0 > 8.0) as u8,
            rev_pct: (lit * 100 / 15) as u8,
            rev_bits,
            brake_temp: [0.0f32, 0.3, 0.6, 0.9].map(|o| (380.0 + 600.0 * brake + 40.0 * (t + o).sin()) as u16),
            tyre_surface_temp: [0.0f32, 1.0, 2.0, 3.0].map(|o| (heat(o) + 8.0 * (t * 3.0 + o).sin()) as u8),
            tyre_inner_temp: [0.0f32, 1.0, 2.0, 3.0].map(|o| heat(o) as u8),
            engine_temp: 108,
            tyre_pressure: [22.6, 22.7, 24.1, 24.0],
        };
        st.suggested_gear = 0;

        st.status[PLAYER] = CarStatus {
            traction_control: 0,
            abs: 0,
            fuel_mix: 1,
            brake_bias: 56,
            pit_limiter: 0,
            fuel_in_tank: (100.0 - (lap_num as f32 - 1.0 + frac) * 1.6).max(0.0),
            fuel_capacity: 110.0,
            fuel_remaining_laps: 0.35 + 0.25 * (t * 0.05).sin(),
            max_rpm: 13_000,
            idle_rpm: 4_000,
            max_gears: 8,
            drs_allowed: drs_zone as u8,
            drs_activation_distance: 0,
            actual_compound: 18,
            visual_compound: 17,
            tyre_age_laps: lap_num + 3,
            fia_flag: if (20.0..26.0).contains(&(t % 60.0)) { 3 } else { 0 },
            ers_store: 2_100_000.0 + 1_700_000.0 * (t * 0.15).sin(),
            ers_deploy_mode: if throttle > 0.95 { 3 } else { 1 },
            ers_harvested_mguk: frac * 1_400_000.0,
            ers_harvested_mguh: frac * 500_000.0,
            ers_deployed: frac * 3_300_000.0,
        };

        st.damage[PLAYER] = CarDamage {
            tyre_wear: [wear_base + 1.5, wear_base + 1.2, wear_base, wear_base + 0.4],
            fl_wing: 12,
            floor: 4,
            sidepod: 0,
            gearbox: 9,
            engine: 6,
            wear_ice: 14,
            wear_mguh: 11,
            wear_mguk: 17,
            wear_es: 9,
            wear_ce: 8,
            wear_tc: 12,
            ..Default::default()
        };
        st.motion[PLAYER] = CarMotion { g_lat: lateral, g_long: -accel / 45.0, g_vert: 1.0 };

        // --- field / timing --------------------------------------------------------------
        let mut leader_gap = 0u32;
        for i in 0..CARS {
            let front = if i == 0 { 0 } else { (700.0 + 450.0 * (t * 0.25 + i as f32).sin()) as u32 };
            leader_gap += front;
            let other_frac = (frac - leader_gap as f32 / 1000.0 / lap_len).rem_euclid(1.0);
            let l = &mut st.laps[i];
            l.position = i as u8 + 1;
            l.result_status = 2;
            l.driver_status = 4;
            l.delta_front_ms = front;
            l.delta_leader_ms = leader_gap;
            l.lap_num = lap_num;
            l.lap_distance = other_frac * TRACK_LEN;
            l.num_pit_stops = if i % 4 == 0 { 1 } else { 0 };
            l.pit_status = if i == 12 && (40.0..52.0).contains(&(t % 90.0)) { 1 } else { 0 };
        }
        {
            let l = &mut st.laps[PLAYER];
            l.lap_num = lap_num;
            l.lap_distance = dist;
            l.current_lap_ms = (in_lap * 1000.0) as u32;
            l.last_lap_ms = last_lap_ms;
            l.sector = sector;
            l.sector1_ms = if sector >= 1 { s1_ms } else { 0 };
            l.sector2_ms = if sector >= 2 { s2_ms } else { 0 };
            l.lap_invalid = lap_num % 5 == 0 && frac > 0.4;
            l.warnings = 1;
            l.cc_warnings = 1;
            l.speed_trap_kph = 331.4;
        }
        // World positions on a made-up circuit shape so the MAP tab has something to draw.
        for i in 0..CARS {
            let a = st.laps[i].lap_distance / TRACK_LEN * TAU;
            st.motion[i].x = 900.0 * a.cos() + 250.0 * (2.0 * a).sin();
            st.motion[i].z = 500.0 * a.sin() + 150.0 * (3.0 * a).cos();
        }
        let me = st.motion[PLAYER];
        st.map.add(me.x, me.z, Some(frac));
        st.session.time_left_s = (5400.0 - t).max(0.0) as u16;
        st.session.safety_car = if (40.0..47.0).contains(&(t % 60.0)) { 2 } else { 0 };
        st.update_delta();

        // --- events ----------------------------------------------------------------------
        let slot = (t / 15.0) as u64;
        if slot != last_event_slot && t > 3.0 {
            last_event_slot = slot;
            match slot % 4 {
                0 => event(&mut st, "DRS ENABLED", EventKind::Good),
                1 => {
                    let text = format!("FASTEST LAP  {}  1:27.512", NAMES[(slot as usize * 7) % CARS]);
                    event(&mut st, &text, EventKind::Purple)
                }
                2 => event(&mut st, "OVERTOOK  TANAKA", EventKind::Good),
                _ => event(&mut st, "PENALTY  +5s", EventKind::Danger),
            }
        }
    }
}

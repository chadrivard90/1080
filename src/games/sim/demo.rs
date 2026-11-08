//! Demo data for every shared-engine game, so each dash can be previewed without the game.
//! Circuits drive an oval-ish loop with a field of cars; rally runs a winding stage; Forza
//! Horizon wanders the open world.

use std::f32::consts::TAU;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use super::{Car, Corner, Flag, Game, SimState, Tone};

const NAMES: [&str; 16] = [
    "NOVAK", "OKAFOR", "LINDQVIST", "MORETTI", "TANAKA", "DUBOIS", "KOWALSKI", "SILVA", "HARTMANN", "REYES",
    "BRENNAN", "IVANOV", "CHEN", "PETERSEN", "ALVAREZ", "MEYER",
];

pub fn run(shared: Arc<Mutex<SimState>>, stop: Arc<AtomicBool>) {
    let start = Instant::now();
    let game = match shared.lock() {
        Ok(st) => st.game,
        Err(_) => return,
    };
    let rally = matches!(game, Game::Dr2 | Game::Wrc);
    let open_world = game == Game::ForzaHorizon;
    let lap_len_s = if game == Game::Nascar { 31.0 } else { 92.0 };
    let stage_len_m = 9_800.0;
    let mut best: Option<f32> = None;
    let mut last: Option<f32> = None;
    let mut lap_no = 1;
    let mut lap_start = 0.0f32;
    let mut last_event = 0u64;
    let mut yaw_prev = 0.0f32;
    let mut pos_prev: Option<(f32, f32)> = None;
    let mut wander = (0.0f32, 0.0f32);

    while !stop.load(Ordering::Relaxed) {
        std::thread::sleep(Duration::from_millis(16));
        let t = start.elapsed().as_secs_f32();
        let Ok(mut st) = shared.lock() else { return };
        st.last_packet = Some(Instant::now());

        // --- where are we --------------------------------------------------------------
        let this_len = lap_len_s + 0.6 * (lap_no as f32 * 1.3).sin();
        if !rally && t - lap_start >= this_len {
            last = Some(this_len);
            if best.map_or(true, |b| this_len < b) {
                best = Some(this_len);
            }
            lap_no += 1;
            lap_start = t;
        }
        let frac = if rally { (t / 240.0).fract() } else { ((t - lap_start) / this_len).clamp(0.0, 1.0) };
        let phase = frac * TAU;
        let (x, z) = if rally {
            // Winding stage: a long S-curve path.
            let s = frac * stage_len_m;
            (s * 0.35 + 600.0 * (frac * 9.0).sin(), 400.0 * (frac * 5.0).cos() + s * 0.2)
        } else if open_world {
            wander.0 += 30.0 * 0.016 * (t * 0.05).cos();
            wander.1 += 30.0 * 0.016 * (t * 0.05).sin() + 10.0 * 0.016;
            (wander.0 + 200.0 * (t * 0.11).sin(), wander.1)
        } else if game == Game::Nascar {
            (420.0 * phase.cos(), 180.0 * phase.sin())
        } else {
            (500.0 * phase.cos() + 120.0 * (2.0 * phase).sin(), 300.0 * phase.sin() + 80.0 * (3.0 * phase).cos())
        };
        let corner_wave = (phase * if game == Game::Nascar { 2.0 } else { 6.0 }).sin();
        let speed = if game == Game::Nascar { 78.0 + 6.0 * corner_wave } else { 48.0 + 22.0 * corner_wave };
        let accel = (phase * 6.0).cos();
        let gear = if game == Game::Nascar { 4 } else { (1.0 + speed / 12.0).clamp(1.0, 7.0) as i32 };
        let gear_floor = (gear - 1) as f32 * 12.0;
        let max_rpm = if game == Game::Nascar { 9_400.0 } else { 8_500.0 };
        let rpm = (5_200.0 + (speed - gear_floor) / 12.0 * 3_300.0).clamp(2_000.0, max_rpm);

        st.speed_ms = speed;
        st.gear = gear;
        st.rpm = rpm;
        st.max_rpm = max_rpm;
        st.throttle = if accel > -0.3 { 1.0 } else { 0.1 };
        st.brake = if accel < -0.5 { (-accel - 0.5) * 2.0 } else { 0.0 };
        st.clutch = 0.0;
        st.handbrake = if rally && (t % 9.0) < 0.6 { 1.0 } else { 0.0 };
        st.steer = (phase * 3.0).sin() * 0.5;
        st.g_lat = 2.2 * (phase * 3.0).sin();
        st.g_long = -accel * 1.2;

        let yaw = if let Some((px, pz)) = pos_prev { (x - px).atan2(z - pz) } else { 0.0 };
        st.yaw = yaw;
        st.yaw_rate = (yaw - yaw_prev) / 0.016;
        yaw_prev = yaw;
        pos_prev = Some((x, z));
        st.drift_deg = if open_world { 35.0 * (t * 0.4).sin().powi(3) } else { 6.0 * (phase * 3.0).sin() };
        st.world = Some((x, 40.0 + 30.0 * (t * 0.02).sin(), z));

        // --- car systems ---------------------------------------------------------------
        let heat = |o: f32| 88.0 + 9.0 * (t * 0.3 + o).sin();
        for (i, c) in st.corners.iter_mut().enumerate() {
            let o = i as f32;
            *c = Corner {
                surface_c: Some(heat(o) + 6.0 * (t * 2.0 + o).sin()),
                inner_c: Some(heat(o)),
                imo: Some([heat(o) + 4.0, heat(o), heat(o) - 3.0]),
                pressure_psi: Some(26.4 + 0.8 * (t * 0.2 + o).sin()),
                wear_pct: Some(4.0 + lap_no as f32 * 1.6 + o),
                brake_c: Some(320.0 + 380.0 * (if accel < -0.5 { 1.0 } else { 0.0 }) + 40.0 * (t + o).sin()),
                susp: Some(0.5 + 0.3 * (t * 3.0 + o).sin()),
                slip: Some((0.4 + 0.5 * (phase * 6.0 + o).sin()).abs()),
            };
        }
        if game == Game::Nascar {
            // Right side tyres work hardest on an oval.
            st.corners[1].wear_pct = Some(8.0 + lap_no as f32 * 2.8);
            st.corners[3].wear_pct = Some(6.0 + lap_no as f32 * 2.1);
        }
        if rally {
            for c in st.corners.iter_mut() {
                c.surface_c = None;
                c.inner_c = None;
                c.imo = None;
                c.pressure_psi = None;
                c.wear_pct = None;
            }
        }
        st.fuel_l = Some((70.0 - (t / 60.0) * 2.5).max(0.0));
        st.fuel_frac = Some(st.fuel_l.unwrap_or(0.0) / 80.0);
        st.fuel_laps = Some(st.fuel_l.unwrap_or(0.0) / 2.6);
        st.water_c = Some(92.0 + 4.0 * (t * 0.05).sin());
        st.oil_c = Some(104.0 + 3.0 * (t * 0.04).sin());
        st.oil_bar = Some(4.2 + 0.8 * (rpm / max_rpm));
        let throttle = st.throttle;
        st.boost_psi = matches!(game, Game::ForzaHorizon | Game::ForzaMotorsport | Game::Wrc).then(|| 14.0 * throttle);
        st.boost_max_psi = Some(20.0);
        st.tc_active = game == Game::AcEvo && accel > 0.6 && (t * 8.0).sin() > 0.3;
        st.abs_active = game == Game::AcEvo && accel < -0.6;
        st.tc_level = Some(3);
        st.abs_level = Some(2);
        st.brake_bias = Some(56.5);
        st.power_kw = Some(380.0 * st.throttle);
        st.torque_nm = Some(520.0 * st.throttle);
        st.car_info = match game {
            Game::ForzaHorizon => "S1 900  AWD".into(),
            Game::ForzaMotorsport => "GT 800  RWD".into(),
            _ => String::new(),
        };

        // --- timing / session ----------------------------------------------------------
        st.track_name = match game {
            Game::Nascar => "DAYTONA".into(),
            Game::AcEvo => "MONZA".into(),
            Game::Ams2 => "INTERLAGOS".into(),
            Game::Dr2 => "WALES - PANT MAWR".into(),
            Game::Wrc => "RALLY JAPAN".into(),
            Game::ForzaMotorsport => "SPA".into(),
            Game::ForzaHorizon => "OPEN WORLD".into(),
        };
        st.session_name = if rally { "STAGE".into() } else { "RACE".into() };
        st.lap = lap_no;
        st.total_laps = (!rally && !open_world).then_some(30);
        st.current_lap_s = Some(if rally { t % 240.0 } else { t - lap_start });
        st.last_lap_s = last;
        st.best_lap_s = best;
        st.lap_dist_m = Some(frac * 4000.0);
        st.lap_pct = (!rally && !open_world).then_some(frac);
        st.time_left_s = Some((3600.0 - t).max(0.0));
        st.lap_invalid = lap_no % 4 == 0 && frac > 0.5;
        if rally {
            st.stage_len_m = Some(stage_len_m);
            st.stage_dist_m = Some(frac * stage_len_m);
            st.stage_time_s = Some(t % 240.0);
        }
        st.distance_m = Some(t * 45.0);
        st.weather = Some(if (t % 120.0) < 60.0 { "LIGHT CLOUD".into() } else { "LIGHT RAIN".into() });
        st.air_c = Some(21.0);
        st.track_c = Some(29.0 + 3.0 * (t * 0.01).sin());
        st.rain_pct = Some(if (t % 120.0) < 60.0 { 5.0 } else { 45.0 + 20.0 * (t * 0.1).sin() });
        st.wind_ms = Some(4.0);
        st.incidents = Some(2);
        st.flag = if (t % 60.0) > 40.0 && (t % 60.0) < 46.0 {
            if game == Game::Nascar { Flag::Caution } else { Flag::Yellow }
        } else {
            Flag::None
        };
        st.spotter_left = if game == Game::Nascar && (t % 14.0) < 3.0 { 1 } else { 0 };
        st.spotter_right = if game == Game::Nascar && (t % 14.0) > 6.0 && (t % 14.0) < 8.5 { 1 } else { 0 };

        // Field of cars (circuit games).
        if !rally && !open_world {
            let n = 16usize;
            st.num_cars = n as u32;
            st.position = 4;
            let my = frac;
            st.cars = (0..n)
                .map(|i| {
                    let player = i == 3;
                    let offset = (i as f32 - 3.0) * -0.03 + 0.004 * (t * 0.3 + i as f32).sin();
                    let pct = if player { my } else { (my + offset).rem_euclid(1.0) };
                    let a = pct * TAU;
                    let pos = if game == Game::Nascar {
                        (420.0 * a.cos(), 180.0 * a.sin())
                    } else {
                        (500.0 * a.cos() + 120.0 * (2.0 * a).sin(), 300.0 * a.sin() + 80.0 * (3.0 * a).cos())
                    };
                    Car {
                        pos: if game == Game::Nascar { None } else { Some(pos) },
                        pct: Some(pct),
                        race_pos: i as u32 + 1,
                        name: NAMES[i].into(),
                        number: if game == Game::Nascar { format!("{}", [24, 5, 9, 48, 11, 22, 12, 19, 20, 45, 1, 8, 3, 6, 17, 99][i]) } else { String::new() },
                        lap: lap_no,
                        player,
                        in_pit: i == 11 && (t % 50.0) < 10.0,
                        gap_leader_s: Some(i as f32 * 0.9),
                        interval_s: Some(if i == 0 { 0.0 } else { 0.6 + 0.4 * (t * 0.25 + i as f32).sin() }),
                        best_s: Some(lap_len_s - 0.6 + i as f32 * 0.08),
                        last_s: Some(lap_len_s + i as f32 * 0.1),
                    }
                })
                .collect();
            st.gap_ahead_s = st.cars[3].interval_s;
            st.gap_behind_s = st.cars[4].interval_s;
        } else {
            st.cars.clear();
            st.position = 0;
        }

        // Pop-ups.
        let slot = (t / 12.0) as u64;
        if slot != last_event && t > 3.0 {
            last_event = slot;
            let (text, tone) = match (game, slot % 3) {
                (Game::Nascar, 0) => ("CAR LOW - LEFT SIDE", Tone::Warn),
                (Game::Nascar, 1) => ("FIVE TO GO", Tone::Info),
                (Game::Dr2 | Game::Wrc, 0) => ("SPLIT 1  +0.84", Tone::Warn),
                (Game::Dr2 | Game::Wrc, 1) => ("SPLIT 2  -1.20", Tone::Good),
                (Game::ForzaHorizon, 0) => ("DRIFT  12,450 PTS", Tone::Purple),
                (_, 0) => ("TRACK LIMITS WARNING", Tone::Warn),
                (_, 1) => ("FASTEST LAP  1:31.204", Tone::Purple),
                _ => ("PIT WINDOW OPEN", Tone::Info),
            };
            st.event(text, tone);
        }
        st.derive();
    }
}

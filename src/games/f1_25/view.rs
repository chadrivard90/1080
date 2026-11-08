//! Converts the parsed `F1State` into the Slint `F1Data` / model rows shown on screen.
//! All text formatting and colour decisions live here so the .slint files stay declarative.

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::{Duration, Instant};

use slint::{ComponentHandle as _, Model as _, VecModel};

use super::lookup::{self, hex};
use super::packets::{EventKind, F1State, FL, FR, NUM_CARS, RL, RR};
use crate::games::map::MapCar;
use crate::laplog::{self, LapInput, LapLog};
use crate::{F1Data, ForecastRow, LapRow, MapDot, Part, SectorInfo, StandingRow, Tyre};

const ERS_MAX_J: f32 = 4_000_000.0;

pub struct Models {
    pub standings: Rc<VecModel<StandingRow>>,
    pub laps: Rc<VecModel<LapRow>>,
    pub forecast: Rc<VecModel<ForecastRow>>,
    pub map_dots: Rc<VecModel<MapDot>>,
    pub relative: Rc<VecModel<StandingRow>>,
    pub map_orient: Rc<Cell<u8>>,
    map_version: Cell<u64>,
    map_orient_built: Cell<u8>,
    /// Extra laps of fuel to keep in hand (settings page).
    pub fuel_margin: f32,
    /// Write finished laps to the lap log (off in demo mode).
    pub log_laps: bool,
    laplog: RefCell<LapLog>,
    last_vs_pb: Cell<Option<f32>>,
    pb_event: RefCell<Option<(String, Instant)>>,
}

impl Models {
    pub fn new(map_orient: Rc<Cell<u8>>, fuel_margin: f32, log_laps: bool) -> Self {
        Self {
            standings: Rc::new(VecModel::default()),
            laps: Rc::new(VecModel::default()),
            forecast: Rc::new(VecModel::default()),
            map_dots: Rc::new(VecModel::default()),
            relative: Rc::new(VecModel::default()),
            map_orient,
            map_version: Cell::new(u64::MAX),
            map_orient_built: Cell::new(255),
            fuel_margin: fuel_margin.clamp(0.0, 5.0),
            log_laps,
            laplog: RefCell::new(LapLog::open()),
            last_vs_pb: Cell::new(None),
            pb_event: RefCell::new(None),
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Formatting helpers

pub fn fmt_lap(ms: u32) -> String {
    if ms == 0 {
        return "NO TIME".into();
    }
    format!("{}:{:02}.{:03}", ms / 60_000, (ms % 60_000) / 1000, ms % 1000)
}

pub fn fmt_lap_secs(secs: f32) -> String {
    fmt_lap((secs.max(0.0) * 1000.0).round() as u32)
}

fn fmt_sector(ms: u32) -> String {
    if ms == 0 {
        "NO TIME".into()
    } else if ms >= 60_000 {
        fmt_lap(ms)
    } else {
        format!("{}.{:03}", ms / 1000, ms % 1000)
    }
}

fn fmt_gap(ms: u32) -> String {
    if ms >= 60_000 {
        format!("+{}:{:02}.{}", ms / 60_000, (ms % 60_000) / 1000, (ms % 1000) / 100)
    } else {
        format!("+{}.{:03}", ms / 1000, ms % 1000)
    }
}

fn fmt_clock(secs: u32) -> String {
    let (h, m, s) = (secs / 3600, (secs % 3600) / 60, secs % 60);
    if h > 0 {
        format!("{h}:{m:02}:{s:02}")
    } else {
        format!("{m}:{s:02}")
    }
}

fn gear_text(gear: i8) -> String {
    match gear {
        g if g < 0 => "R".into(),
        0 => "N".into(),
        g => g.to_string(),
    }
}

fn unit(v: f32) -> f32 {
    if v.is_finite() {
        v.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

/// RPM sweep arc in a 1000 x 600 view box: centre (500, 560), radius 440, 190..350 degrees.
fn rpm_sweep(frac: f32) -> String {
    let frac = unit(frac);
    if frac < 0.005 {
        return String::new();
    }
    let pt = |deg: f32| {
        let a = deg.to_radians();
        (500.0 + 440.0 * a.cos(), 560.0 + 440.0 * a.sin())
    };
    let (x0, y0) = pt(190.0);
    let (x1, y1) = pt(190.0 + 160.0 * frac);
    format!("M {x0:.1} {y0:.1} A 440 440 0 0 1 {x1:.1} {y1:.1}")
}

/// Green -> red -> purple through the rev range; blue flash at the limiter.
fn rpm_color(frac: f32, limiter: bool, blink: bool) -> slint::Color {
    if limiter {
        return if blink { hex(0x2F7BFF) } else { hex(0x0C1430) };
    }
    let mix = |a: u32, b: u32, t: f32| {
        let t = t.clamp(0.0, 1.0);
        let ch = |s: u32| -> u8 {
            let (ca, cb) = (((a >> s) & 0xFF) as f32, ((b >> s) & 0xFF) as f32);
            (ca + (cb - ca) * t) as u8
        };
        slint::Color::from_rgb_u8(ch(16), ch(8), ch(0))
    };
    if frac < 0.7 {
        hex(0x00D26A)
    } else if frac < 0.9 {
        mix(0x00D26A, 0xFF1E1E, (frac - 0.7) / 0.2)
    } else {
        mix(0xFF1E1E, 0xB545FF, (frac - 0.9) / 0.1)
    }
}

fn part(pct: u8) -> Part {
    Part { pct: pct as i32, color: lookup::damage_color(pct) }
}

// ---------------------------------------------------------------------------------------------

pub fn apply(ui: &crate::AppWindow, st: &F1State, models: &Models, blink: bool) {
    let f1 = ui.global::<crate::F1>();
    let mut data = build_data(st, blink, models.fuel_margin);
    log_laps(st, models, &mut data);
    let is_race = data.is_race;
    f1.set_data(data);
    let standings = build_standings(st, is_race);
    // Relative: two cars ahead, you, two behind.
    let me = standings.iter().position(|r| r.player).unwrap_or(0);
    let from = me.saturating_sub(2);
    let to = (me + 3).min(standings.len());
    sync_model(&models.relative, standings[from..to].to_vec());
    sync_model(&models.standings, standings);
    sync_model(&models.laps, build_laps(st));
    sync_model(&models.forecast, build_forecast(st));
    apply_map(&f1, st, models);
}

/// Append finished laps to the CSV log and show the all-time personal best for the track.
fn log_laps(st: &F1State, models: &Models, d: &mut F1Data) {
    let now = Instant::now();
    let p = st.player;
    let lap = &st.laps[p];
    let track = if st.session.received { lookup::track_name(st.session.track_id) } else { "" };
    let session = if st.session.received { lookup::session_name(st.session.session_type) } else { "" };
    let mut log = models.laplog.borrow_mut();
    if d.connected && st.session.received && models.log_laps {
        let input = LapInput {
            game: "f1",
            track,
            car: "",
            session,
            lap: lap.lap_num as i32,
            last_lap_s: (lap.last_lap_ms > 0).then(|| lap.last_lap_ms as f32 / 1000.0),
            invalid_now: lap.lap_invalid,
            position: lap.position as u32,
            fuel: Some(st.status[p].fuel_in_tank),
        };
        if let Some(done) = log.observe(&input) {
            if done.new_best {
                *models.pb_event.borrow_mut() = Some((format!("ALL-TIME BEST  {}", fmt_lap_secs(done.time_s)), now));
            }
            models.last_vs_pb.set(done.previous_best.map(|b| done.time_s - b));
        }
    }
    let pb = log.best("f1", track, "");
    d.has_pb = pb.is_some();
    d.pb_lap = pb.map_or_else(|| "NO TIME".to_string(), fmt_lap_secs).into();
    let gap = models.last_vs_pb.get();
    d.pb_gap = gap.map_or_else(String::new, laplog::fmt_diff).into();
    d.pb_gap_good = gap.is_some_and(|g| g <= 0.0);
    if let Some((text, at)) = models.pb_event.borrow().as_ref() {
        if now.duration_since(*at) < Duration::from_secs(6) {
            d.event_text = text.clone().into();
            d.event_color = hex(0xB545FF);
            d.event_text_color = hex(0xFFFFFF);
        }
    }
}

/// Track map: outline from the player's line, every active car as a team-coloured dot.
fn apply_map(f1: &crate::F1<'_>, st: &F1State, models: &Models) {
    let cars: Vec<MapCar> = (0..NUM_CARS)
        .filter(|&i| st.laps[i].position > 0 && st.laps[i].result_status >= 2)
        .map(|i| {
            let team = st.participants[i].team_id;
            let c = lookup::team_color(team);
            MapCar {
                pos: Some((st.motion[i].x, st.motion[i].z)),
                pct: None,
                player: i == st.player,
                label: st.laps[i].position.to_string(),
                color: Some(((c.red() as u32) << 16) | ((c.green() as u32) << 8) | c.blue() as u32),
            }
        })
        .collect();
    let orient = models.map_orient.get();
    let (path, dots) = st.map.render(&cars, orient, None);
    if st.map.version != models.map_version.get() || orient != models.map_orient_built.get() {
        models.map_version.set(st.map.version);
        models.map_orient_built.set(orient);
        f1.set_map_path(path.into());
    }
    f1.set_map_note(
        if !st.map.has_outline() {
            "Start driving to draw the track"
        } else if !st.map.is_complete() {
            "Drawing the track. The outline completes after one lap"
        } else {
            ""
        }
        .into(),
    );
    let ui_dots: Vec<MapDot> = dots
        .into_iter()
        .map(|d| MapDot {
            x: d.x,
            y: d.y,
            player: d.player,
            label: d.label.into(),
            color: d.color.map_or(hex(0xFFFFFF), hex),
            kind: d.kind as i32,
            alpha: d.alpha,
        })
        .collect();
    sync_model(&models.map_dots, ui_dots);
}

/// Update a VecModel in place so Slint only re-renders rows that actually changed.
fn sync_model<T: Clone + PartialEq + 'static>(model: &VecModel<T>, rows: Vec<T>) {
    let existing = model.row_count();
    for (i, row) in rows.iter().enumerate() {
        if i < existing {
            if model.row_data(i).as_ref() != Some(row) {
                model.set_row_data(i, row.clone());
            }
        } else {
            model.push(row.clone());
        }
    }
    for i in (rows.len()..existing).rev() {
        model.remove(i);
    }
}

fn build_data(st: &F1State, blink: bool, fuel_margin: f32) -> F1Data {
    let now = Instant::now();
    let p = st.player;
    let tel = &st.telemetry[p];
    let sta = &st.status[p];
    let lap = &st.laps[p];
    let dmg = &st.damage[p];
    let ses = &st.session;
    let hist = &st.history[p];

    let mut d = F1Data::default();

    // --- connection ------------------------------------------------------------------------
    let fresh = st.last_packet.is_some_and(|t| now.duration_since(t) < Duration::from_secs(2));
    d.connected = fresh && st.wrong_format.is_none() && st.error.is_none();
    d.status_text = if let Some(e) = &st.error {
        e.clone()
    } else if let Some(f) = st.wrong_format {
        format!("Game is sending UDP format {f}. Set UDP Format to 2025.")
    } else if fresh {
        String::new()
    } else {
        "No telemetry received yet".into()
    }
    .into();
    d.blink = blink;

    // --- driving ---------------------------------------------------------------------------
    let mph = ses.received && ses.speed_units == 0;
    let fahrenheit = ses.received && ses.temp_units == 1;
    let to_temp = |c: f32| -> i32 {
        if fahrenheit {
            (c * 9.0 / 5.0 + 32.0).round() as i32
        } else {
            c.round() as i32
        }
    };
    let speed = tel.speed_kph as f32 * if mph { 0.621_371 } else { 1.0 };
    d.speed = speed.round() as i32;
    d.speed_unit = if mph { "MPH" } else { "KPH" }.into();
    d.gear = gear_text(tel.gear).into();
    d.suggested_gear = if st.suggested_gear > 0 { st.suggested_gear.to_string() } else { String::new() }.into();
    d.shift_hint = st.suggested_gear > 0 && st.suggested_gear != tel.gear;
    d.rpm = tel.rpm as i32;
    let max_rpm = if sta.max_rpm > 0 { sta.max_rpm } else { 13_000 };
    d.rpm_pct = unit(tel.rpm as f32 / max_rpm as f32);
    let bits = tel.rev_bits & 0x7FFF;
    d.rev_count = bits.count_ones() as i32;
    d.rev_max = bits == 0x7FFF || tel.rev_pct >= 100;
    d.rpm_arc = rpm_sweep(d.rpm_pct).into();
    d.rpm_track = rpm_sweep(1.0).into();
    d.rpm_color = rpm_color(d.rpm_pct, d.rev_max, blink);
    d.throttle = unit(tel.throttle);
    d.brake = unit(tel.brake);
    d.steer = if tel.steer.is_finite() { tel.steer.clamp(-1.0, 1.0) } else { 0.0 };

    d.drs_open = tel.drs == 1;
    d.drs_allowed = sta.drs_allowed == 1;
    d.drs_distance = sta.drs_activation_distance as i32;

    d.ers_pct = unit(sta.ers_store / ERS_MAX_J);
    d.ers_text = format!("{:.0}%", d.ers_pct * 100.0).into();
    let (mode, mode_color) = lookup::ers_mode(sta.ers_deploy_mode);
    d.ers_mode = mode.into();
    d.ers_mode_color = mode_color;
    d.ers_deployed_pct = unit(sta.ers_deployed / ERS_MAX_J);
    d.ers_harvested_pct = unit((sta.ers_harvested_mguk + sta.ers_harvested_mguh) / ERS_MAX_J);

    d.fuel_kg = format!("{:.1} kg", sta.fuel_in_tank).into();
    d.fuel_laps = format!("{:+.2}", sta.fuel_remaining_laps).into();
    d.fuel_ok = sta.fuel_remaining_laps >= fuel_margin;
    d.fuel_target = format!("TARGET {:+.1}", fuel_margin).into();
    d.fuel_mix = lookup::fuel_mix(sta.fuel_mix).into();

    d.brake_bias = sta.brake_bias as i32;
    d.diff = st.setup.diff_on as i32;
    d.engine_braking = st.setup.engine_braking as i32;
    d.tc = lookup::traction_control(sta.traction_control).into();
    d.abs_on = sta.abs == 1;
    d.pit_limiter = sta.pit_limiter == 1;
    d.engine_temp = format!("{}°", to_temp(tel.engine_temp as f32)).into();

    // --- tyres -----------------------------------------------------------------------------
    let (letter, compound_color) = lookup::compound(sta.visual_compound);
    d.tyre_compound = letter.into();
    d.tyre_color = compound_color;
    d.tyre_age = sta.tyre_age_laps as i32;
    d.tyre_wear_avg = (dmg.tyre_wear.iter().sum::<f32>() / 4.0).round() as i32;
    let tyre = |w: usize| -> Tyre {
        let surface = tel.tyre_surface_temp[w] as f32;
        let inner = tel.tyre_inner_temp[w] as f32;
        let brake = tel.brake_temp[w] as f32;
        Tyre {
            surface: to_temp(surface),
            inner: to_temp(inner),
            brake: to_temp(brake),
            pressure: format!("{:.1}", tel.tyre_pressure[w]).into(),
            wear: dmg.tyre_wear[w].round() as i32,
            damage: dmg.tyre_damage[w] as i32,
            blister: dmg.tyre_blisters[w] as i32,
            surface_color: lookup::tyre_surface_color(surface),
            inner_color: lookup::tyre_inner_color(inner),
            brake_color: lookup::brake_temp_color(brake),
            brake_pct: unit(brake / 1100.0),
            ..Default::default()
        }
    };
    d.fl = tyre(FL);
    d.fr = tyre(FR);
    d.rl = tyre(RL);
    d.rr = tyre(RR);
    d.temp_unit = if fahrenheit { "°F" } else { "°C" }.into();
    // Flag tyres whose surface and core temperatures disagree (e.g. surface cooked, carcass cold).
    let notes: Vec<String> = [("FL", FL), ("FR", FR), ("RL", RL), ("RR", RR)]
        .iter()
        .filter_map(|&(name, w)| {
            let surface = tel.tyre_surface_temp[w] as f32;
            let core = tel.tyre_inner_temp[w] as f32;
            if surface > 110.0 && core < 85.0 {
                Some(format!("{name} SURFACE HOT / CORE COLD"))
            } else if surface < 80.0 && core > 105.0 {
                Some(format!("{name} SURFACE COLD / CORE HOT"))
            } else {
                None
            }
        })
        .collect();
    d.tyre_note = notes.join("  ·  ").into();

    // --- damage ----------------------------------------------------------------------------
    d.wing_fl = part(dmg.fl_wing);
    d.wing_fr = part(dmg.fr_wing);
    d.wing_rear = part(dmg.rear_wing);
    d.floor_dmg = part(dmg.floor);
    d.diffuser = part(dmg.diffuser);
    d.sidepod = part(dmg.sidepod);
    d.gearbox = part(dmg.gearbox);
    d.engine = part(dmg.engine);
    d.drs_fault = dmg.drs_fault != 0;
    d.ers_fault = dmg.ers_fault != 0;
    d.engine_failed = dmg.engine_blown != 0 || dmg.engine_seized != 0;
    d.wear_ice = part(dmg.wear_ice);
    d.wear_mguh = part(dmg.wear_mguh);
    d.wear_mguk = part(dmg.wear_mguk);
    d.wear_es = part(dmg.wear_es);
    d.wear_ce = part(dmg.wear_ce);
    d.wear_tc = part(dmg.wear_tc);

    // --- timing ----------------------------------------------------------------------------
    let is_race = ses.is_race();
    d.is_race = is_race;
    d.position = lap.position as i32;
    let active = (0..NUM_CARS).filter(|&i| st.laps[i].result_status >= 2).count() as i32;
    d.num_cars = if st.num_active > 0 { st.num_active as i32 } else { active };
    d.lap = lap.lap_num as i32;
    d.total_laps = ses.total_laps as i32;
    d.time_left = fmt_clock(ses.time_left_s as u32).into();
    d.lap_text = if is_race {
        format!("LAP {}/{}", lap.lap_num, ses.total_laps)
    } else {
        format!("{} LEFT", fmt_clock(ses.time_left_s as u32))
    }
    .into();
    d.current_lap = fmt_lap(lap.current_lap_ms).into();
    d.last_lap = fmt_lap(lap.last_lap_ms).into();
    let best = hist.best_lap_ms();
    d.best_lap = fmt_lap(best).into();
    let pb_sectors = [hist.best_sector_ms(0), hist.best_sector_ms(1), hist.best_sector_ms(2)];
    let ideal = if pb_sectors.iter().all(|&s| s > 0) { pb_sectors.iter().sum() } else { 0 };
    d.ideal_lap = fmt_lap(ideal).into();
    d.session_best = fmt_lap(st.session_best().map_or(0, |(_, t)| t)).into();

    match st.delta.delta {
        Some(delta) if st.delta.has_reference() => {
            d.has_delta = true;
            d.delta = delta.clamp(-99.0, 99.0);
            d.delta_text = format!("{:+.3}", d.delta).into();
        }
        _ => {
            d.has_delta = false;
            d.delta_text = "NO REF".into();
        }
    }

    // Sectors: completed sectors of this lap are coloured; upcoming ones show last lap dimmed.
    let overall = [st.overall_best_sector(0), st.overall_best_sector(1), st.overall_best_sector(2)];
    let prev_lap = if hist.num_laps >= 2 { hist.laps.get(hist.num_laps as usize - 2).copied() } else { None };
    let current = [lap.sector1_ms, lap.sector2_ms, 0];
    let sector_info = |k: usize| -> SectorInfo {
        let done = (lap.sector as usize) > k && current[k] > 0;
        if done {
            let t = current[k];
            let (bg, fg) = if overall[k] > 0 && t <= overall[k] {
                (hex(0xC04BFF), hex(0xFFFFFF))
            } else if pb_sectors[k] > 0 && t <= pb_sectors[k] {
                (hex(0x2BFF88), hex(0x000000))
            } else {
                (hex(0xFFD400), hex(0x000000))
            };
            SectorInfo { time: fmt_sector(t).into(), color: bg, text_color: fg, ..Default::default() }
        } else {
            let prev = prev_lap.map_or(0, |l| l.sectors_ms[k]);
            let active = lap.sector as usize == k;
            SectorInfo {
                time: fmt_sector(prev).into(),
                color: if active { hex(0x3A3C43) } else { hex(0x25272C) },
                text_color: hex(0x8D8DA3),
                ..Default::default()
            }
        }
    };
    d.s1 = sector_info(0);
    d.s2 = sector_info(1);
    d.s3 = sector_info(2);
    d.sector = lap.sector as i32;
    d.lap_invalid = lap.lap_invalid;

    // Gaps to the cars directly ahead / behind (race only).
    let by_position = |pos: u8| (0..NUM_CARS).find(|&i| st.laps[i].position == pos && st.laps[i].result_status >= 2);
    d.gap_ahead_s = -1.0;
    d.gap_behind_s = -1.0;
    d.gap_ahead = "--".into();
    d.gap_behind = "--".into();
    if is_race && lap.position > 0 {
        if lap.position > 1 {
            if let Some(a) = by_position(lap.position - 1) {
                d.ahead_name = st.name(a).into();
                d.gap_ahead = fmt_gap(lap.delta_front_ms).into();
                d.gap_ahead_s = lap.delta_front_ms as f32 / 1000.0;
            }
        }
        if let Some(b) = by_position(lap.position + 1) {
            let ms = st.laps[b].delta_front_ms;
            d.behind_name = st.name(b).into();
            d.gap_behind = fmt_gap(ms).into();
            d.gap_behind_s = ms as f32 / 1000.0;
        }
        d.gap_leader = if lap.position == 1 { "LEADER".to_string() } else { fmt_gap(lap.delta_leader_ms) }.into();
    } else {
        d.gap_leader = "--".into();
    }

    d.in_pit = lap.pit_status > 0;
    d.pit_stops = lap.num_pit_stops as i32;
    d.penalties = lap.penalties_s as i32;
    d.warnings = lap.warnings as i32;
    d.cc_warnings = lap.cc_warnings as i32;
    d.unserved = match (lap.unserved_dt, lap.unserved_sg) {
        (0, 0) => String::new(),
        (dt, 0) => format!("{dt} DRIVE THROUGH"),
        (0, sg) => format!("{sg} STOP GO"),
        (dt, sg) => format!("{dt} DT + {sg} SG"),
    }
    .into();

    // --- session ---------------------------------------------------------------------------
    d.session_name = if ses.received { lookup::session_name(ses.session_type) } else { "F1 25" }.into();
    d.track_name = lookup::track_name(ses.track_id).into();
    let (wname, wcolor) = lookup::weather(ses.weather);
    d.weather = if ses.received { wname } else { "--" }.into();
    d.weather_color = wcolor;
    d.track_temp = format!("{}°", to_temp(ses.track_temp as f32)).into();
    d.air_temp = format!("{}°", to_temp(ses.air_temp as f32)).into();
    d.rain_next = ses
        .forecast
        .iter()
        .filter(|f| f.session_type == ses.session_type && f.time_offset_min > 0)
        .map(|f| f.rain_pct as i32)
        .next()
        .unwrap_or(0);
    d.pit_window = if ses.pit_ideal_lap > 0 {
        format!("L{} - L{}", ses.pit_ideal_lap, ses.pit_latest_lap.max(ses.pit_ideal_lap))
    } else {
        "--".into()
    }
    .into();
    d.pit_rejoin = ses.pit_rejoin_pos as i32;
    d.pit_speed = ses.pit_speed_limit as i32;

    // --- alerts ----------------------------------------------------------------------------
    let red_flag = st.red_flag_at.is_some_and(|t| now.duration_since(t) < Duration::from_secs(20));
    let (banner, bg, fg): (&str, u32, u32) = if red_flag {
        ("RED FLAG", 0xE10600, 0xFFFFFF)
    } else if ses.safety_car == 1 {
        ("SAFETY CAR", 0xFFD400, 0x000000)
    } else if ses.safety_car == 2 {
        ("VIRTUAL SAFETY CAR", 0xFFD400, 0x000000)
    } else if ses.safety_car == 3 {
        ("FORMATION LAP", 0x2F7BFF, 0xFFFFFF)
    } else if sta.fia_flag == 3 {
        ("YELLOW FLAG", 0xFFD400, 0x000000)
    } else if sta.fia_flag == 2 {
        ("BLUE FLAG", 0x2F7BFF, 0xFFFFFF)
    } else if sta.pit_limiter == 1 {
        ("PIT LIMITER", 0x7A2CFF, 0xFFFFFF)
    } else if dmg.engine_blown != 0 || dmg.engine_seized != 0 {
        ("ENGINE FAILURE", 0xE10600, 0xFFFFFF)
    } else {
        ("", 0x000000, 0xFFFFFF)
    };
    d.banner_text = banner.into();
    d.banner_color = hex(bg);
    d.banner_text_color = hex(fg);

    if let Some(ev) = &st.event {
        let ttl = if ev.kind == EventKind::Danger { 8 } else { 5 };
        if now.duration_since(ev.at) < Duration::from_secs(ttl) {
            let (bg, fg) = match ev.kind {
                EventKind::Info => (0xF0F0F0, 0x000000),
                EventKind::Purple => (0xB545FF, 0xFFFFFF),
                EventKind::Warning => (0xFFD400, 0x000000),
                EventKind::Danger => (0xE10600, 0xFFFFFF),
                EventKind::Good => (0x00D26A, 0x000000),
            };
            d.event_text = ev.text.clone().into();
            d.event_color = hex(bg);
            d.event_text_color = hex(fg);
        }
    }

    // --- motion ----------------------------------------------------------------------------
    let m = &st.motion[p];
    d.g_lat = m.g_lat.clamp(-5.0, 5.0);
    d.g_long = m.g_long.clamp(-5.0, 5.0);
    let trap = lap.speed_trap_kph * if mph { 0.621_371 } else { 1.0 };
    d.speed_trap = if trap > 1.0 { format!("{:.0}", trap) } else { "--".into() }.into();

    d
}

fn build_standings(st: &F1State, is_race: bool) -> Vec<StandingRow> {
    let mut idxs: Vec<usize> =
        (0..NUM_CARS).filter(|&i| st.laps[i].position > 0 && st.laps[i].result_status >= 2).collect();
    idxs.sort_by_key(|&i| st.laps[i].position);
    let fastest = st.session_best().map(|(i, _)| i);
    let leader = idxs.first().copied();
    let leader_best = leader.map_or(0, |i| st.history[i].best_lap_ms());

    idxs.iter()
        .map(|&i| {
            let l = &st.laps[i];
            let s = &st.status[i];
            let (tyre, tyre_color) = lookup::compound(s.visual_compound);
            let best = st.history[i].best_lap_ms();
            let retired = l.result_status >= 4;
            let status = match l.result_status {
                4 => "DNF",
                5 => "DSQ",
                6 => "NC",
                7 => "RET",
                _ if l.pit_status > 0 => "PIT",
                _ if !is_race && l.driver_status == 0 => "GAR",
                _ => "",
            };
            let (interval, gap) = if is_race {
                if Some(i) == leader {
                    ("LEADER".to_string(), format!("L{}", l.lap_num))
                } else {
                    // Whole laps behind the leader (accounting for position within the lap).
                    let lapped = leader.map_or(0, |ld| {
                        let lead = &st.laps[ld];
                        let diff = lead.lap_num as i32 - l.lap_num as i32;
                        if diff <= 0 {
                            0
                        } else if l.lap_distance < lead.lap_distance {
                            diff
                        } else {
                            diff - 1
                        }
                    });
                    let gap = if lapped > 0 {
                        format!("+{lapped} LAP{}", if lapped > 1 { "S" } else { "" })
                    } else {
                        fmt_gap(l.delta_leader_ms)
                    };
                    (fmt_gap(l.delta_front_ms), gap)
                }
            } else {
                let gap = if Some(i) == leader || best == 0 || leader_best == 0 {
                    String::new()
                } else {
                    fmt_gap(best.saturating_sub(leader_best))
                };
                (fmt_lap(best), gap)
            };
            StandingRow {
                pos: l.position as i32,
                name: st.name(i).into(),
                team_color: lookup::team_color(st.participants[i].team_id),
                gap: gap.into(),
                interval: interval.into(),
                tyre: tyre.into(),
                tyre_color,
                tyre_age: s.tyre_age_laps as i32,
                pits: l.num_pit_stops as i32,
                player: i == st.player,
                retired,
                status: status.into(),
                fastest: Some(i) == fastest,
                ..Default::default()
            }
        })
        .collect()
}

fn build_laps(st: &F1State) -> Vec<LapRow> {
    let hist = &st.history[st.player];
    let best = hist.best_lap_ms();
    let completed: Vec<(usize, _)> =
        hist.laps.iter().enumerate().filter(|(_, l)| l.time_ms > 0).map(|(i, l)| (i, *l)).collect();
    completed
        .iter()
        .rev()
        .take(9)
        .map(|&(i, l)| LapRow {
            lap: i as i32 + 1,
            time: fmt_lap(l.time_ms).into(),
            s1: fmt_sector(l.sectors_ms[0]).into(),
            s2: fmt_sector(l.sectors_ms[1]).into(),
            s3: fmt_sector(l.sectors_ms[2]).into(),
            valid: l.valid(),
            best: best > 0 && l.time_ms == best,
            delta: if best > 0 && l.time_ms != best { fmt_gap(l.time_ms.saturating_sub(best)) } else { String::new() }
                .into(),
            ..Default::default()
        })
        .collect()
}

fn build_forecast(st: &F1State) -> Vec<ForecastRow> {
    let ses = &st.session;
    let fahrenheit = ses.temp_units == 1;
    ses.forecast
        .iter()
        .filter(|f| f.session_type == ses.session_type)
        .take(6)
        .map(|f| {
            let (name, color) = lookup::weather(f.weather);
            let t = f.track_temp as f32;
            let t = if fahrenheit { t * 9.0 / 5.0 + 32.0 } else { t };
            ForecastRow {
                minutes: if f.time_offset_min == 0 { "NOW".to_string() } else { format!("+{}m", f.time_offset_min) }
                    .into(),
                weather: name.into(),
                weather_color: color,
                rain: f.rain_pct as i32,
                track_temp: format!("{:.0}°", t).into(),
                ..Default::default()
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_times() {
        assert_eq!(fmt_lap(0), "NO TIME");
        assert_eq!(fmt_lap(83_456), "1:23.456");
        assert_eq!(fmt_sector(28_001), "28.001");
        assert_eq!(fmt_gap(512), "+0.512");
        assert_eq!(fmt_gap(61_234), "+1:01.2");
        assert_eq!(fmt_clock(754), "12:34");
        assert_eq!(gear_text(-1), "R");
        assert_eq!(gear_text(0), "N");
        assert_eq!(gear_text(7), "7");
    }
}

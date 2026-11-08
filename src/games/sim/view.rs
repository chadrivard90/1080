//! `SimState` -> Slint `SimData` + models. All formatting and colour choices live here.

use std::cell::Cell;
use std::collections::VecDeque;
use std::fmt::Write as _;
use std::rc::Rc;
use std::time::{Duration, Instant};

use slint::{Color, ComponentHandle as _, Model as _, ModelRc, SharedString, VecModel};

use super::{dials, Flag, Game, SimState, Tone};
use crate::{CompassMark, DialLabel, DynoBar, MapDot, SimData, SimRow, TrailDot};
use crate::games::map::MapCar;
use crate::laplog::{self, LapInput, LapLog};

type UiCorner = crate::Corner;

pub fn hex(rgb: u32) -> Color {
    Color::from_rgb_u8((rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8)
}

pub fn fmt_lap(secs: f32) -> String {
    if !secs.is_finite() || secs <= 0.0 {
        return "NO TIME".into();
    }
    let ms = (secs * 1000.0).round() as u64;
    format!("{}:{:02}.{:03}", ms / 60_000, (ms % 60_000) / 1000, ms % 1000)
}

fn fmt_opt_lap(s: Option<f32>) -> String {
    s.map_or_else(|| "NO TIME".to_string(), fmt_lap)
}

fn fmt_gap(secs: f32) -> String {
    if secs >= 60.0 {
        let ms = (secs * 1000.0) as u64;
        format!("+{}:{:02}.{}", ms / 60_000, (ms % 60_000) / 1000, (ms % 1000) / 100)
    } else {
        format!("+{:.3}", secs.max(0.0))
    }
}

fn fmt_clock(secs: f32) -> String {
    let s = secs.max(0.0) as u32;
    let (h, m, sec) = (s / 3600, (s % 3600) / 60, s % 60);
    if h > 0 {
        format!("{h}:{m:02}:{sec:02}")
    } else {
        format!("{m}:{sec:02}")
    }
}

fn unit(v: f32) -> f32 {
    if v.is_finite() {
        v.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

fn tyre_color(c: f32) -> Color {
    hex(if c < 60.0 {
        0x2F6BFF
    } else if c < 75.0 {
        0x36C5F0
    } else if c <= 100.0 {
        0x22C55E
    } else if c <= 110.0 {
        0xFFA000
    } else {
        0xFF2A2A
    })
}

fn brake_color(c: f32) -> Color {
    hex(if c < 150.0 {
        0x2F6BFF
    } else if c < 300.0 {
        0x36C5F0
    } else if c <= 700.0 {
        0x22C55E
    } else if c <= 850.0 {
        0xFFA000
    } else {
        0xFF2A2A
    })
}

/// Dark grey at full grip blending to cyan as the tyre slides.
fn slip_color(slip: f32) -> Color {
    let t = unit(slip);
    let lerp = |a: u8, b: u8| (a as f32 + (b as f32 - a as f32) * t) as u8;
    Color::from_rgb_u8(lerp(0x2B, 0x00), lerp(0x2E, 0xE5), lerp(0x33, 0xFF))
}

fn pressure_color(psi: f32, window: Option<(f32, f32)>) -> Color {
    match window {
        None => hex(0xFFFFFF),
        Some((lo, hi)) => hex(if psi < lo - 1.0 {
            0x2F6BFF
        } else if psi < lo {
            0x36C5F0
        } else if psi <= hi {
            0x22C55E
        } else if psi <= hi + 1.0 {
            0xFFA000
        } else {
            0xFF2A2A
        }),
    }
}

/// Keeps the per-session models and caches, and refreshes the UI every tick.
/// Per-game options from the settings page.
#[derive(Clone, Copy, Debug)]
pub struct Tuning {
    pub imperial: bool,
    /// Fraction of max RPM where the shift lights are full (0.80..1.00).
    pub shift_frac: f32,
    /// Extra laps of fuel to keep in hand.
    pub fuel_margin: f32,
    /// Write finished laps to the lap log (off in demo mode).
    pub log_laps: bool,
}

pub struct Renderer {
    imperial: bool,
    shift_full: f32,
    fuel_margin: f32,
    laplog: LapLog,
    log_enabled: bool,
    last_vs_pb: Option<f32>,
    pb_event: Option<(String, Instant)>,
    orient: Rc<Cell<u8>>,
    standings: Rc<VecModel<SimRow>>,
    relative: Rc<VecModel<SimRow>>,
    dots: Rc<VecModel<MapDot>>,
    compass: Rc<VecModel<CompassMark>>,
    tach_labels: Rc<VecModel<DialLabel>>,
    speed_labels: Rc<VecModel<DialLabel>>,
    map_version: u64,
    map_orient: u8,
    map_built: Instant,
    map_path: SharedString,
    tach_max_k: u32,
    started: Instant,

    // History for traces / trails / records (UI side, sampled on a clock).
    trail: Rc<VecModel<TrailDot>>,
    eq: Rc<VecModel<f32>>,
    dyno: Rc<VecModel<DynoBar>>,
    laps: Rc<VecModel<SimRow>>,
    rpm_hist: VecDeque<f32>,
    g_hist: VecDeque<(f32, f32)>,
    last_sample: Instant,
    last_tick: Instant,
    dyno_bins: [f32; 30],
    lap_times: Vec<(i32, f32)>,
    last_lap_seen: i32,
    drift_score: f32,
    drift_idle: f32,
    peak_lat: f32,
    stint_start: Instant,
    was_in_pit: bool,
}

impl Renderer {
    pub fn new(ui: &crate::AppWindow, tuning: Tuning, orient: Rc<Cell<u8>>) -> Self {
        let imperial = tuning.imperial;
        let r = Self {
            imperial,
            shift_full: tuning.shift_frac.clamp(0.8, 1.0),
            fuel_margin: tuning.fuel_margin.clamp(0.0, 5.0),
            laplog: LapLog::open(),
            log_enabled: tuning.log_laps,
            last_vs_pb: None,
            pb_event: None,
            orient,
            standings: Rc::new(VecModel::default()),
            relative: Rc::new(VecModel::default()),
            dots: Rc::new(VecModel::default()),
            compass: Rc::new(VecModel::default()),
            tach_labels: Rc::new(VecModel::default()),
            speed_labels: Rc::new(VecModel::default()),
            map_version: u64::MAX,
            map_orient: 255,
            map_built: Instant::now(),
            map_path: SharedString::default(),
            tach_max_k: 0,
            started: Instant::now(),
            trail: Rc::new(VecModel::default()),
            eq: Rc::new(VecModel::from(vec![0.0f32; 20])),
            dyno: Rc::new(VecModel::default()),
            laps: Rc::new(VecModel::default()),
            rpm_hist: VecDeque::with_capacity(60),
            g_hist: VecDeque::with_capacity(8),
            last_sample: Instant::now(),
            last_tick: Instant::now(),
            dyno_bins: [0.0; 30],
            lap_times: Vec::new(),
            last_lap_seen: 0,
            drift_score: 0.0,
            drift_idle: 0.0,
            peak_lat: 0.0,
            stint_start: Instant::now(),
            was_in_pit: false,
        };
        let sim = ui.global::<crate::Sim>();
        sim.set_standings(ModelRc::from(r.standings.clone()));
        sim.set_relative(ModelRc::from(r.relative.clone()));
        sim.set_map_dots(ModelRc::from(r.dots.clone()));
        sim.set_compass(ModelRc::from(r.compass.clone()));
        sim.set_tach_labels(ModelRc::from(r.tach_labels.clone()));
        sim.set_speed_labels(ModelRc::from(r.speed_labels.clone()));
        sim.set_g_trail(ModelRc::from(r.trail.clone()));
        sim.set_eq_bars(ModelRc::from(r.eq.clone()));
        sim.set_dyno(ModelRc::from(r.dyno.clone()));
        sim.set_lap_list(ModelRc::from(r.laps.clone()));
        // Speedometer: 0..300 km/h or 0..200 mph in 10 steps.
        let top = if imperial { 200 } else { 300 };
        sim.set_speed_ticks(dials::ticks(10).into());
        let labels: Vec<DialLabel> = (0..=10)
            .step_by(2)
            .map(|i| {
                let (x, y) = dials::label_pos(i as f32 / 10.0);
                DialLabel { x, y, text: (top * i / 10).to_string().into() }
            })
            .collect();
        r.speed_labels.set_vec(labels);
        r
    }

    pub fn apply(&mut self, ui: &crate::AppWindow, st: &SimState) {
        let sim = ui.global::<crate::Sim>();
        let now = Instant::now();

        // Tachometer scale follows the car's max RPM.
        let max_rpm = if st.max_rpm > 1000.0 { st.max_rpm } else { 8000.0 };
        let max_k = (max_rpm / 1000.0).ceil().clamp(2.0, 20.0) as u32;
        if max_k != self.tach_max_k {
            self.tach_max_k = max_k;
            sim.set_tach_ticks(dials::ticks(max_k).into());
            let labels: Vec<DialLabel> = (0..=max_k)
                .map(|k| {
                    let (x, y) = dials::label_pos(k as f32 / max_k as f32);
                    DialLabel { x, y, text: k.to_string().into() }
                })
                .collect();
            self.tach_labels.set_vec(labels);
        }

        let mut data = self.build_data(st, now, max_k);
        self.update_history(st, now, &mut data);
        self.log_laps(st, now, &mut data);
        sim.set_data(data);

        sync(&self.standings, self.build_standings(st));
        sync(&self.relative, self.build_relative(st));
        sync(&self.compass, build_compass(st.yaw));
        self.update_map(&sim, st, now);
    }

    /// Append finished laps to the CSV log and show the all-time personal best.
    fn log_laps(&mut self, st: &SimState, now: Instant, d: &mut SimData) {
        let key = st.game.key();
        if d.connected && self.log_enabled {
            let input = LapInput {
                game: key,
                track: &st.track_name,
                car: &st.car_info,
                session: &st.session_name,
                lap: st.lap,
                last_lap_s: st.last_lap_s,
                invalid_now: st.lap_invalid,
                position: st.position,
                fuel: st.fuel_l.or(st.fuel_frac.map(|f| f * 100.0)),
            };
            if let Some(done) = self.laplog.observe(&input) {
                if done.new_best {
                    self.pb_event = Some((format!("ALL-TIME BEST  {}", fmt_lap(done.time_s)), now));
                }
                self.last_vs_pb = done.previous_best.map(|b| done.time_s - b);
            }
        }
        let pb = self.laplog.best(key, &st.track_name, &st.car_info);
        d.pb_lap = pb.map_or_else(|| "NO TIME".to_string(), fmt_lap).into();
        d.has_pb = pb.is_some();
        d.pb_gap = self.last_vs_pb.map_or_else(String::new, laplog::fmt_diff).into();
        d.pb_gap_good = self.last_vs_pb.is_some_and(|g| g <= 0.0);
        if let Some((text, at)) = &self.pb_event {
            if now.duration_since(*at) < Duration::from_secs(6) {
                d.event_text = text.clone().into();
                d.event_color = hex(0xB545FF);
                d.event_text_color = hex(0xFFFFFF);
            }
        }
    }

    /// Rolling histories and records: RPM trace, G trail, equalizer, dyno, laps, drift, stint.
    fn update_history(&mut self, st: &SimState, now: Instant, d: &mut SimData) {
        let dt = now.duration_since(self.last_tick).as_secs_f32().min(0.2);
        self.last_tick = now;
        let fresh = st.last_packet.is_some_and(|t| now.duration_since(t) < Duration::from_secs(2));

        // Sample traces every 100 ms.
        if now.duration_since(self.last_sample) >= Duration::from_millis(100) && fresh {
            self.last_sample = now;
            self.rpm_hist.push_back(d.rpm_pct);
            if self.rpm_hist.len() > 50 {
                self.rpm_hist.pop_front();
            }
            self.g_hist.push_front((d.g_lat, d.g_long));
            self.g_hist.truncate(6);
            // Dyno: best power seen in each RPM bin.
            if let Some(kw) = st.power_kw {
                let bin = ((d.rpm_pct * 29.0).round() as usize).min(29);
                if kw > self.dyno_bins[bin] {
                    self.dyno_bins[bin] = kw;
                }
            }
        }
        // RPM trace path (view box 1000 x 300).
        let mut trace = String::with_capacity(600);
        let n = self.rpm_hist.len();
        for (i, v) in self.rpm_hist.iter().enumerate() {
            let x = if n > 1 { i as f32 * 1000.0 / 49.0 } else { 0.0 };
            let y = 290.0 - v.clamp(0.0, 1.0) * 280.0;
            let _ = write!(trace, "{}{:.0} {:.0} ", if i == 0 { "M " } else { "L " }, x, y);
        }
        d.rpm_trace = trace.into();

        // G trail (oldest first so the newest draws on top).
        let max_g = 3.0;
        let trail: Vec<TrailDot> = self
            .g_hist
            .iter()
            .enumerate()
            .skip(1)
            .rev()
            .map(|(i, (x, y))| TrailDot {
                x: (x / max_g).clamp(-1.0, 1.0),
                y: (y / max_g).clamp(-1.0, 1.0),
                alpha: 0.7 - i as f32 * 0.12,
            })
            .collect();
        sync(&self.trail, trail);

        // RPM equalizer: 20 bars, lit up to the RPM, with a little bounce.
        let t = now.duration_since(self.started).as_secs_f32();
        let lit = d.rpm_pct * 20.0;
        let bars: Vec<f32> = (0..20)
            .map(|i| {
                let i = i as f32;
                if i < lit {
                    (0.35 + 0.65 * (i + 1.0) / 20.0 + 0.08 * (t * 9.0 + i).sin()).clamp(0.1, 1.0)
                } else {
                    0.06
                }
            })
            .collect();
        sync(&self.eq, bars);

        // Dyno bars normalised to the best power seen.
        let peak = self.dyno_bins.iter().cloned().fold(1.0f32, f32::max);
        let active = ((d.rpm_pct * 29.0).round() as usize).min(29);
        let dyno: Vec<DynoBar> =
            self.dyno_bins.iter().enumerate().map(|(i, p)| DynoBar { h: p / peak, active: i == active }).collect();
        sync(&self.dyno, dyno);

        // Lap history (newest first).
        if st.lap != self.last_lap_seen {
            if st.lap > self.last_lap_seen && self.last_lap_seen > 0 {
                if let Some(last) = st.last_lap_s {
                    self.lap_times.push((st.lap - 1, last));
                }
            }
            if st.lap < self.last_lap_seen {
                self.lap_times.clear();
            }
            self.last_lap_seen = st.lap;
        }
        let best = self.lap_times.iter().map(|l| l.1).fold(f32::MAX, f32::min);
        let rows: Vec<SimRow> = self
            .lap_times
            .iter()
            .rev()
            .take(8)
            .map(|&(lap, time)| SimRow {
                pos: lap,
                name: fmt_lap(time).into(),
                gap: if time <= best + 0.0005 { "BEST".to_string() } else { fmt_gap(time - best) }.into(),
                player: time <= best + 0.0005,
                ..Default::default()
            })
            .collect();
        sync(&self.laps, rows);

        // Drift score: builds while sliding fast, banks after 2 s of grip.
        let sliding = st.drift_deg.abs() > 10.0 && st.speed_ms > 8.0;
        if sliding {
            self.drift_score += st.drift_deg.abs() * st.speed_ms * dt * 0.8;
            self.drift_idle = 0.0;
        } else {
            self.drift_idle += dt;
            if self.drift_idle > 2.0 {
                self.drift_score = 0.0;
            }
        }
        d.drift_score = format!("{}", self.drift_score as u32).into();
        d.drifting = sliding;
        self.peak_lat = self.peak_lat.max(st.g_lat.abs());
        d.peak_lat_g = format!("{:.2} G", self.peak_lat).into();

        // Stint timer restarts when leaving the pits.
        if self.was_in_pit && !st.in_pit {
            self.stint_start = now;
        }
        self.was_in_pit = st.in_pit;
        let stint = now.duration_since(self.stint_start).as_secs();
        d.stint_time = format!("{}:{:02}:{:02}", stint / 3600, (stint % 3600) / 60, stint % 60).into();
        let session_left = st.time_left_s.unwrap_or(0.0);
        d.stint_pct = if session_left > 0.0 {
            (session_left / (session_left + stint as f32)).clamp(0.0, 1.0)
        } else {
            1.0
        };
    }

    fn update_map(&mut self, sim: &crate::Sim<'_>, st: &SimState, now: Instant) {
        let orient = self.orient.get();
        let player_pos = st.world.map(|(x, _, z)| (x, z));
        let mut cars: Vec<MapCar> = st
            .cars
            .iter()
            .filter(|c| c.pos.is_some() || c.pct.is_some())
            .map(|c| MapCar {
                pos: c.pos,
                pct: c.pct,
                player: c.player,
                label: if c.race_pos > 0 { c.race_pos.to_string() } else { String::new() },
                color: None,
            })
            .collect();
        if !cars.iter().any(|c| c.player) {
            if let Some(p) = player_pos {
                cars.push(MapCar { pos: Some(p), player: true, ..Default::default() });
            }
        }
        let open_world = st.game == Game::ForzaHorizon;
        let rebuild = st.map.version != self.map_version
            || orient != self.map_orient
            || (open_world && now.duration_since(self.map_built) > Duration::from_millis(300));
        let (path, dots) = st.map.render(&cars, orient, player_pos);
        if rebuild {
            self.map_version = st.map.version;
            self.map_orient = orient;
            self.map_built = now;
            self.map_path = path.into();
        }
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
        sync(&self.dots, ui_dots);
        sim.set_map_path(self.map_path.clone());
    }

    fn build_data(&self, st: &SimState, now: Instant, max_k: u32) -> SimData {
        let mut d = SimData::default();
        let imp = self.imperial;
        let temp = |c: f32| -> String {
            if imp {
                format!("{:.0}°", c * 9.0 / 5.0 + 32.0)
            } else {
                format!("{:.0}°", c)
            }
        };

        let fresh = st.last_packet.is_some_and(|t| now.duration_since(t) < Duration::from_secs(2));
        d.connected = fresh && st.error.is_none();
        d.status_text = if let Some(e) = &st.error {
            e.clone()
        } else if let Some(w) = &st.warning {
            w.clone()
        } else if fresh {
            String::new()
        } else {
            "No telemetry received yet".into()
        }
        .into();
        d.blink = (now.duration_since(self.started).as_millis() / 160) % 2 == 0;

        // --- driving ---------------------------------------------------------------------
        let speed = st.speed_ms.max(0.0) * if imp { 2.236_94 } else { 3.6 };
        d.speed = speed.round() as i32;
        d.speed_unit = if imp { "MPH" } else { "KPH" }.into();
        d.gear = match st.gear {
            g if g < 0 => "R".to_string(),
            0 => "N".to_string(),
            g => g.to_string(),
        }
        .into();
        d.rpm = st.rpm.max(0.0) as i32;
        let max_rpm = if st.max_rpm > 1000.0 { st.max_rpm } else { 8000.0 };
        d.rpm_pct = unit(st.rpm / max_rpm);
        // Shift lights: game value if provided, else fill over the 22% of the rev range below the
        // shift point chosen on the settings page.
        let full = self.shift_full;
        let shift = st.shift_frac.unwrap_or_else(|| unit((d.rpm_pct - (full - 0.22)) / 0.22));
        d.rev_count = (unit(shift) * 15.0).round() as i32;
        d.shift = shift >= 0.99 || d.rpm_pct >= full;
        d.throttle = unit(st.throttle);
        d.brake = unit(st.brake);
        d.clutch = unit(st.clutch);
        d.handbrake = unit(st.handbrake);
        d.steer = if st.steer.is_finite() { st.steer.clamp(-1.0, 1.0) } else { 0.0 };

        if let Some(b) = st.boost_psi {
            d.has_boost = true;
            d.boost = if imp { format!("{:.1} psi", b) } else { format!("{:.2} bar", b * 0.068_947_6) }.into();
            let max = st.boost_max_psi.unwrap_or(30.0).max(5.0);
            d.boost_pct = unit(b / max);
        }
        d.has_fuel = st.fuel_l.is_some() || st.fuel_frac.is_some();
        d.fuel = match (st.fuel_l, st.fuel_frac) {
            (Some(l), _) if imp => format!("{:.1} gal", l * 0.264_172),
            (Some(l), _) => format!("{:.1} L", l),
            (None, Some(f)) => format!("{:.0}%", f * 100.0),
            _ => "--".into(),
        }
        .into();
        d.fuel_pct = unit(st.fuel_frac.unwrap_or(0.0));
        d.fuel_laps = st.fuel_laps.map_or_else(|| "--".to_string(), |l| format!("{:.1}", l)).into();

        if let Some(w) = st.water_c {
            d.has_water = true;
            d.water = temp(w).into();
            d.water_pct = unit((w - 50.0) / 80.0);
        }
        if st.oil_c.is_some() || st.oil_bar.is_some() {
            d.has_oil = true;
            d.oil_temp = st.oil_c.map_or_else(|| "--".to_string(), temp).into();
            d.oil_press = st
                .oil_bar
                .map_or_else(|| "--".to_string(), |b| if imp { format!("{:.0} psi", b * 14.503_8) } else { format!("{:.1} bar", b) })
                .into();
            d.oil_press_pct = unit(st.oil_bar.unwrap_or(0.0) / 8.0);
            d.oil_temp_pct = unit((st.oil_c.unwrap_or(60.0) - 60.0) / 90.0);
        }
        d.tc_active = st.tc_active;
        d.abs_active = st.abs_active;
        d.tc_level = st.tc_level.map_or_else(|| "--".to_string(), |v| v.to_string()).into();
        d.abs_level = st.abs_level.map_or_else(|| "--".to_string(), |v| v.to_string()).into();
        d.brake_bias = st.brake_bias.map_or_else(|| "--".to_string(), |b| format!("{:.1}%", b)).into();
        d.pit_limiter = st.pit_limiter;

        // --- tyres -----------------------------------------------------------------------
        let window = st.game.pressure_window();
        let corner = |i: usize| -> UiCorner {
            let c = &st.corners[i];
            let main = c.surface_c.or(c.inner_c);
            UiCorner {
                temp: main.map_or_else(String::new, temp).into(),
                temp_color: main.map_or(hex(0x5A5A66), tyre_color),
                inner: c.inner_c.map_or_else(String::new, temp).into(),
                inner_color: c.inner_c.map_or(hex(0x5A5A66), tyre_color),
                detail: c
                    .imo
                    .map_or_else(String::new, |t| format!("{} {} {}", temp(t[0]), temp(t[1]), temp(t[2])))
                    .into(),
                pressure: c.pressure_psi.map_or_else(String::new, |p| format!("{:.1}", p)).into(),
                pressure_color: c.pressure_psi.map_or(hex(0x5A5A66), |p| pressure_color(p, window)),
                wear: c.wear_pct.map_or_else(String::new, |w| format!("{:.0}%", w)).into(),
                wear_pct: unit(c.wear_pct.unwrap_or(0.0) / 100.0),
                brake: c.brake_c.map_or_else(String::new, temp).into(),
                brake_color: c.brake_c.map_or(hex(0x5A5A66), brake_color),
                susp: unit(c.susp.unwrap_or(0.0)),
                slip: c.slip.unwrap_or(0.0).clamp(0.0, 2.0),
                slip_color: slip_color(c.slip.unwrap_or(0.0)),
                brake_pct: unit(c.brake_c.unwrap_or(0.0) / 1000.0),
                has_temp: main.is_some(),
                has_brake: c.brake_c.is_some(),
                ..Default::default()
            }
        };
        d.fl = corner(0);
        d.fr = corner(1);
        d.rl = corner(2);
        d.rr = corner(3);
        d.temp_unit = if imp { "°F" } else { "°C" }.into();

        // --- timing ----------------------------------------------------------------------
        d.position = st.position as i32;
        d.num_cars = st.num_cars as i32;
        d.pos_text = if st.position > 0 && st.num_cars > 1 {
            format!("P{}/{}", st.position, st.num_cars)
        } else if st.position > 0 {
            format!("P{}", st.position)
        } else {
            "--".into()
        }
        .into();
        d.lap_text = match st.total_laps {
            Some(t) if t > 0 => format!("LAP {}/{}", st.lap.max(0), t),
            _ if st.lap > 0 => format!("LAP {}", st.lap),
            _ => String::new(),
        }
        .into();
        d.current_lap = fmt_opt_lap(st.current_lap_s).into();
        d.last_lap = fmt_opt_lap(st.last_lap_s).into();
        d.best_lap = fmt_opt_lap(st.best_lap_s).into();
        match st.delta_s() {
            Some(dl) if dl.is_finite() => {
                d.has_delta = true;
                d.delta = dl.clamp(-99.0, 99.0);
                d.delta_text = format!("{:+.3}", d.delta).into();
            }
            _ => d.delta_text = "NO REF".into(),
        }
        d.lap_invalid = st.lap_invalid;
        d.session_name = st.session_name.clone().into();
        d.track_name = st.track_name.clone().into();
        d.time_left = st.time_left_s.map_or_else(String::new, fmt_clock).into();

        if let (Some(dist), Some(len)) = (st.stage_dist_m, st.stage_len_m) {
            if len > 10.0 {
                d.has_stage = true;
                d.stage_progress = unit(dist / len);
                d.stage_pct = format!("{:.0}%", d.stage_progress * 100.0).into();
                d.stage_text = if imp {
                    format!("{:.2} / {:.2} mi", dist.max(0.0) / 1609.34, len / 1609.34)
                } else {
                    format!("{:.2} / {:.2} km", dist.max(0.0) / 1000.0, len / 1000.0)
                }
                .into();
            }
        }
        if let Some(t) = st.stage_time_s {
            let cs = (t.max(0.0) * 100.0) as u64;
            d.stage_time = format!("{:02}:{:02}.{:02}", cs / 6000, (cs / 100) % 60, cs % 100).into();
        } else {
            d.stage_time = "00:00.00".into();
        }

        d.gap_ahead = st.gap_ahead_s.map_or_else(|| "--".to_string(), fmt_gap).into();
        d.gap_behind = st.gap_behind_s.map_or_else(|| "--".to_string(), fmt_gap).into();
        d.gap_ahead_s = st.gap_ahead_s.unwrap_or(-1.0);
        d.gap_behind_s = st.gap_behind_s.unwrap_or(-1.0);
        d.spotter_left = st.spotter_left as i32;
        d.spotter_right = st.spotter_right as i32;

        // --- environment -----------------------------------------------------------------
        d.weather = st.weather.clone().unwrap_or_else(|| "--".into()).into();
        d.air_temp = st.air_c.map_or_else(|| "--".to_string(), temp).into();
        d.track_temp = st.track_c.map_or_else(|| "--".to_string(), temp).into();
        d.rain_pct = st.rain_pct.map_or(0, |r| r.round() as i32);
        d.wind = st
            .wind_ms
            .map_or_else(|| "--".to_string(), |w| if imp { format!("{:.0} mph", w * 2.236_94) } else { format!("{:.0} km/h", w * 3.6) })
            .into();

        // --- motion ----------------------------------------------------------------------
        d.g_lat = st.g_lat.clamp(-4.0, 4.0);
        d.g_long = st.g_long.clamp(-4.0, 4.0);
        d.g_text = format!("{:.2} G", st.g_lat.abs()).into();
        d.drift_deg = st.drift_deg.clamp(-90.0, 90.0);
        d.drift_text = format!("{:.0}°", st.drift_deg.abs()).into();
        d.yaw_rate = format!("{:.0}°/s", st.yaw_rate.to_degrees()).into();
        let heading = st.yaw.to_degrees().rem_euclid(360.0);
        const POINTS: [&str; 8] = ["N", "NE", "E", "SE", "S", "SW", "W", "NW"];
        d.heading = format!("{} {:03.0}°", POINTS[((heading + 22.5) / 45.0) as usize % 8], heading).into();
        d.elevation = st
            .world
            .map_or_else(|| "--".to_string(), |(_, y, _)| if imp { format!("{:.0} ft", y * 3.280_84) } else { format!("{:.0} m", y) })
            .into();
        let vmax = st.max_speed_ms * if imp { 2.236_94 } else { 3.6 };
        d.max_speed = format!("{:.0}", vmax).into();
        d.distance = st
            .distance_m
            .map_or_else(|| "--".to_string(), |m| if imp { format!("{:.1} mi", m / 1609.34) } else { format!("{:.1} km", m / 1000.0) })
            .into();
        d.power = st
            .power_kw
            .map_or_else(|| "--".to_string(), |kw| if imp { format!("{:.0} hp", kw * 1.341) } else { format!("{:.0} kW", kw) })
            .into();
        d.torque = st
            .torque_nm
            .map_or_else(|| "--".to_string(), |nm| if imp { format!("{:.0} lb-ft", nm * 0.737_56) } else { format!("{:.0} Nm", nm) })
            .into();
        d.car_info = st.car_info.clone().into();
        d.in_pit = st.in_pit;
        d.pit_text = st.pit_text.clone().into();
        d.incidents = st.incidents.map_or_else(String::new, |i| format!("{i}x")).into();

        // --- dials -----------------------------------------------------------------------
        let mut rpm_frac = unit(st.rpm / (max_k as f32 * 1000.0));
        if st.game == Game::Dr2 {
            // Analog tach shake, scaled by engine load.
            let t = now.duration_since(self.started).as_secs_f32();
            rpm_frac += (t * 90.0).sin() * 0.004 * (0.3 + st.throttle);
        }
        d.rpm_needle = dials::needle(rpm_frac).into();
        d.rpm_arc = dials::arc(rpm_frac).into();
        let speed_top = if imp { 200.0 } else { 300.0 };
        d.speed_needle = dials::needle(speed / speed_top).into();
        d.speed_arc = dials::arc(speed / speed_top).into();
        d.drift_needle = dials::needle(0.5 + st.drift_deg.clamp(-60.0, 60.0) / 120.0).into();
        d.boost_needle = dials::needle(d.boost_pct).into();
        // Brake bias dial spans 40%..70% front.
        let bias_frac = st.brake_bias.map_or(0.5, |b| unit((b - 40.0) / 30.0));
        d.bias_pct = bias_frac;
        d.bias_needle = dials::needle(bias_frac).into();
        // Image needles rotate about their centre: -135 deg (min) .. +135 deg (max).
        d.rpm_angle = -135.0 + 270.0 * rpm_frac.clamp(0.0, 1.0);
        d.speed_angle = -135.0 + 270.0 * unit(speed / speed_top);
        // Fixed 0..9000 rpm scale for the printed analog dial face, with engine shake.
        let shake = if st.game == Game::Dr2 {
            let t = now.duration_since(self.started).as_secs_f32();
            (t * 90.0).sin() * 1.2 * (0.3 + st.throttle)
        } else {
            0.0
        };
        d.tach9_angle = -135.0 + 270.0 * unit(st.rpm / 9000.0) + shake;
        // Front / rear brake gauges (0..1000 C).
        let avg = |a: usize, b: usize| match (st.corners[a].brake_c, st.corners[b].brake_c) {
            (Some(x), Some(y)) => Some((x + y) / 2.0),
            (Some(x), None) | (None, Some(x)) => Some(x),
            _ => None,
        };
        let bf = avg(0, 1);
        let br = avg(2, 3);
        d.brake_front = bf.map_or_else(|| "--".to_string(), temp).into();
        d.brake_rear = br.map_or_else(|| "--".to_string(), temp).into();
        d.brake_front_needle = dials::needle(unit(bf.unwrap_or(0.0) / 1000.0)).into();
        d.brake_rear_needle = dials::needle(unit(br.unwrap_or(0.0) / 1000.0)).into();
        d.boost_arc = dials::arc(d.boost_pct).into();
        d.heading_deg = st.yaw.to_degrees().rem_euclid(360.0);
        d.has_wind_dir = st.wind_deg.is_some();
        d.wind_deg = st.wind_deg.unwrap_or(0.0);
        // Fuel: per-lap use and what to add to finish.
        if let Some(per_lap) = st.fuel_per_lap {
            d.fuel_per_lap = if imp { format!("{:.2} gal", per_lap * 0.264_172) } else { format!("{:.2} L", per_lap) }.into();
            let laps_left = match (st.total_laps, st.lap) {
                (Some(total), lap) if total > 0 => (total - lap + 1).max(0) as f32,
                _ => 0.0,
            };
            let need = (laps_left + self.fuel_margin) * per_lap - st.fuel_l.unwrap_or(0.0);
            d.fuel_to_add = if laps_left <= 0.0 {
                "--".to_string()
            } else if need <= 0.0 {
                "NONE".to_string()
            } else if imp {
                format!("+{:.1} gal", need * 0.264_172)
            } else {
                format!("+{:.1} L", need)
            }
            .into();
        } else {
            d.fuel_per_lap = "--".into();
            d.fuel_to_add = "--".into();
        }
        d.fuel_laps_num = st.fuel_laps.unwrap_or(-1.0);

        // --- alerts ----------------------------------------------------------------------
        let (banner, bg, fg): (&str, u32, u32) = match st.flag {
            Flag::Red => ("RED FLAG", 0xE10600, 0xFFFFFF),
            Flag::Caution => ("CAUTION", 0xFFD400, 0x000000),
            Flag::Yellow => ("YELLOW FLAG", 0xFFD400, 0x000000),
            Flag::Blue => ("BLUE FLAG", 0x2F7BFF, 0xFFFFFF),
            Flag::White => ("WHITE FLAG", 0xF2F2F2, 0x000000),
            Flag::Black => ("BLACK FLAG", 0x000000, 0xFFFFFF),
            Flag::Checkered => ("CHEQUERED FLAG", 0xF2F2F2, 0x000000),
            _ if st.pit_limiter => ("PIT LIMITER", 0x7A2CFF, 0xFFFFFF),
            _ => ("", 0, 0xFFFFFF),
        };
        d.banner_text = banner.into();
        d.banner_color = hex(bg);
        d.banner_text_color = hex(fg);
        if let Some(ev) = &st.event {
            if now.duration_since(ev.at) < Duration::from_secs(5) {
                let (bg, fg) = match ev.tone {
                    Tone::Info => (0xF0F0F0, 0x000000),
                    Tone::Good => (0x22C55E, 0x000000),
                    Tone::Warn => (0xFFD400, 0x000000),
                    Tone::Bad => (0xE10600, 0xFFFFFF),
                    Tone::Purple => (0xB545FF, 0xFFFFFF),
                };
                d.event_text = ev.text.clone().into();
                d.event_color = hex(bg);
                d.event_text_color = hex(fg);
            }
        }

        // --- map -------------------------------------------------------------------------
        d.map_note = if st.map.has_outline() {
            if st.map.is_complete() || st.game == Game::ForzaHorizon || st.game == Game::Dr2 || st.game == Game::Wrc {
                String::new()
            } else {
                "Drawing the track. The outline completes after one lap".into()
            }
        } else {
            "Start driving to draw the map".into()
        }
        .into();
        d
    }

    fn build_standings(&self, st: &SimState) -> Vec<SimRow> {
        let mut cars: Vec<&super::Car> = st.cars.iter().filter(|c| c.race_pos > 0).collect();
        cars.sort_by_key(|c| c.race_pos);
        cars.iter()
            .take(40)
            .map(|c| {
                let gap = if c.race_pos == 1 {
                    "LEADER".to_string()
                } else if let Some(i) = c.interval_s {
                    fmt_gap(i)
                } else if let Some(g) = c.gap_leader_s {
                    fmt_gap(g)
                } else {
                    String::new()
                };
                let detail = match (c.best_s, c.last_s) {
                    (Some(b), _) if b > 0.0 => fmt_lap(b),
                    (_, Some(l)) if l > 0.0 => fmt_lap(l),
                    _ if c.lap > 0 => format!("L{}", c.lap),
                    _ => String::new(),
                };
                let name = if c.number.is_empty() { c.name.clone() } else { format!("#{} {}", c.number, c.name) };
                SimRow {
                    pos: c.race_pos as i32,
                    name: name.into(),
                    gap: gap.into(),
                    detail: detail.into(),
                    player: c.player,
                    in_pit: c.in_pit,
                    ..Default::default()
                }
            })
            .collect()
    }

    /// Cars physically around the player on track (3 ahead, 3 behind) with time gaps.
    fn build_relative(&self, st: &SimState) -> Vec<SimRow> {
        let Some(me) = st.cars.iter().find(|c| c.player) else { return Vec::new() };
        let Some(my_pct) = me.pct else { return Vec::new() };
        let lap_time = st.best_lap_s.or(st.last_lap_s).filter(|t| *t > 5.0).unwrap_or(60.0);
        let mut rel: Vec<(f32, &super::Car)> = st
            .cars
            .iter()
            .filter_map(|c| {
                let p = c.pct?;
                let mut d = p - my_pct;
                if d > 0.5 {
                    d -= 1.0;
                }
                if d < -0.5 {
                    d += 1.0;
                }
                Some((d, c))
            })
            .collect();
        rel.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        let idx = rel.iter().position(|(_, c)| c.player).unwrap_or(0);
        let from = idx.saturating_sub(3);
        let to = (idx + 4).min(rel.len());
        rel[from..to]
            .iter()
            .map(|(d, c)| {
                let secs = d * lap_time;
                let lapdiff = c.lap - me.lap;
                let detail = if c.player || lapdiff == 0 {
                    String::new()
                } else {
                    format!("{:+}L", lapdiff)
                };
                let name = if c.number.is_empty() { c.name.clone() } else { format!("#{} {}", c.number, c.name) };
                SimRow {
                    pos: c.race_pos as i32,
                    name: name.into(),
                    gap: if c.player { String::new() } else { format!("{:+.2}", -secs) }.into(),
                    detail: detail.into(),
                    player: c.player,
                    in_pit: c.in_pit,
                    ..Default::default()
                }
            })
            .collect()
    }
}

/// Compass strip marks within +/-90 degrees of the heading; x is 0..1 across the strip.
fn build_compass(yaw: f32) -> Vec<CompassMark> {
    let heading = yaw.to_degrees().rem_euclid(360.0);
    let names = ["N", "NE", "E", "SE", "S", "SW", "W", "NW"];
    (0..24)
        .filter_map(|i| {
            let deg = i as f32 * 15.0;
            let mut diff = deg - heading;
            if diff > 180.0 {
                diff -= 360.0;
            }
            if diff < -180.0 {
                diff += 360.0;
            }
            if diff.abs() > 88.0 {
                return None;
            }
            let major = i % 3 == 0;
            let label = if major { names[(i / 3) as usize].to_string() } else { format!("{}", deg as i32) };
            Some(CompassMark { x: 0.5 + diff / 180.0, label: label.into(), major })
        })
        .collect()
}

fn sync<T: Clone + PartialEq + 'static>(model: &VecModel<T>, rows: Vec<T>) {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats() {
        assert_eq!(fmt_lap(83.456), "1:23.456");
        assert_eq!(fmt_lap(0.0), "NO TIME");
        assert_eq!(fmt_gap(0.5), "+0.500");
        assert_eq!(fmt_clock(754.0), "12:34");
        let c = build_compass(0.0);
        assert!(c.iter().any(|m| m.label == "N" && (m.x - 0.5).abs() < 1e-3));
    }
}

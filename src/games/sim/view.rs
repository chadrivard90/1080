//! `SimState` -> Slint `SimData` + models. All formatting and colour choices live here.

use std::cell::Cell;
use std::rc::Rc;
use std::time::{Duration, Instant};

use slint::{Color, ComponentHandle as _, Model as _, ModelRc, SharedString, VecModel};

use super::{dials, Flag, Game, SimState, Tone};
use crate::{CompassMark, DialLabel, MapDot, SimData, SimRow};
use crate::games::map::MapCar;

type UiCorner = crate::Corner;

pub fn hex(rgb: u32) -> Color {
    Color::from_rgb_u8((rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8)
}

pub fn fmt_lap(secs: f32) -> String {
    if !secs.is_finite() || secs <= 0.0 {
        return "-:--.---".into();
    }
    let ms = (secs * 1000.0).round() as u64;
    format!("{}:{:02}.{:03}", ms / 60_000, (ms % 60_000) / 1000, ms % 1000)
}

fn fmt_opt_lap(s: Option<f32>) -> String {
    s.map_or_else(|| "-:--.---".to_string(), fmt_lap)
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
pub struct Renderer {
    imperial: bool,
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
}

impl Renderer {
    pub fn new(ui: &crate::AppWindow, imperial: bool, orient: Rc<Cell<u8>>) -> Self {
        let r = Self {
            imperial,
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
        };
        let sim = ui.global::<crate::Sim>();
        sim.set_standings(ModelRc::from(r.standings.clone()));
        sim.set_relative(ModelRc::from(r.relative.clone()));
        sim.set_map_dots(ModelRc::from(r.dots.clone()));
        sim.set_compass(ModelRc::from(r.compass.clone()));
        sim.set_tach_labels(ModelRc::from(r.tach_labels.clone()));
        sim.set_speed_labels(ModelRc::from(r.speed_labels.clone()));
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

        let data = self.build_data(st, now, max_k);
        sim.set_data(data);

        sync(&self.standings, self.build_standings(st));
        sync(&self.relative, self.build_relative(st));
        sync(&self.compass, build_compass(st.yaw));
        self.update_map(&sim, st, now);
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
        // Shift lights: game value if provided, else light up over the top 25% of the rev range.
        let shift = st.shift_frac.unwrap_or_else(|| unit((d.rpm_pct - 0.75) / 0.22));
        d.rev_count = (unit(shift) * 15.0).round() as i32;
        d.shift = shift >= 0.99 || d.rpm_pct > 0.97;
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
            _ => d.delta_text = "--.---".into(),
        }
        d.lap_invalid = st.lap_invalid;
        d.session_name = st.session_name.clone().into();
        d.track_name = st.track_name.clone().into();
        d.time_left = st.time_left_s.map_or_else(String::new, fmt_clock).into();

        if let (Some(dist), Some(len)) = (st.stage_dist_m, st.stage_len_m) {
            if len > 10.0 {
                d.has_stage = true;
                d.stage_progress = unit(dist / len);
                d.stage_pct = format!("{:.0}% COMPLETE", d.stage_progress * 100.0).into();
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
        assert_eq!(fmt_lap(0.0), "-:--.---");
        assert_eq!(fmt_gap(0.5), "+0.500");
        assert_eq!(fmt_clock(754.0), "12:34");
        let c = build_compass(0.0);
        assert!(c.iter().any(|m| m.label == "N" && (m.x - 0.5).abs() < 1e-3));
    }
}

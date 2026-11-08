//! Shared engine for every game except F1 25 (which has its own richer module).
//!
//! Each game's parser fills a game-agnostic `SimState`; `view.rs` turns that into the Slint
//! `SimData` that all the themed dashes read. Values a game doesn't send stay `None`, and the
//! dashes hide or grey out those panels.

pub mod demo;
pub mod dials;
pub mod view;

use std::cell::Cell;
use std::net::UdpSocket;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use slint::ComponentHandle as _;

use super::f1_25::packets::DeltaTracker;
use super::map::{MapMode, MapRecorder};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Game {
    Nascar,
    AcEvo,
    Ams2,
    Dr2,
    Wrc,
    ForzaMotorsport,
    ForzaHorizon,
}

impl Game {
    /// Title-screen tile index (0 is F1 25).
    pub fn from_index(i: i32) -> Option<Game> {
        Some(match i {
            1 => Game::Nascar,
            2 => Game::AcEvo,
            3 => Game::Ams2,
            4 => Game::Dr2,
            5 => Game::Wrc,
            6 => Game::ForzaMotorsport,
            7 => Game::ForzaHorizon,
            _ => return None,
        })
    }

    pub fn key(self) -> &'static str {
        match self {
            Game::Nascar => "nascar",
            Game::AcEvo => "acevo",
            Game::Ams2 => "ams2",
            Game::Dr2 => "dr2",
            Game::Wrc => "wrc",
            Game::ForzaMotorsport => "fm",
            Game::ForzaHorizon => "fh6",
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Game::Nascar => "NASCAR 26",
            Game::AcEvo => "ASSETTO CORSA EVO",
            Game::Ams2 => "AUTOMOBILISTA 2",
            Game::Dr2 => "DIRT RALLY 2.0",
            Game::Wrc => "EA SPORTS WRC",
            Game::ForzaMotorsport => "FORZA MOTORSPORT",
            Game::ForzaHorizon => "FORZA HORIZON 6",
        }
    }

    pub fn default_port(self) -> u16 {
        match self {
            Game::Nascar | Game::AcEvo => 20800, // PC bridge
            Game::Ams2 => 5606,
            Game::Dr2 | Game::Wrc => 20777,
            Game::ForzaMotorsport | Game::ForzaHorizon => 20440,
        }
    }

    pub fn map_mode(self) -> MapMode {
        match self {
            Game::Dr2 | Game::Wrc => MapMode::Stage,
            Game::ForzaHorizon => MapMode::OpenWorld,
            _ => MapMode::Circuit,
        }
    }

    pub fn imperial_default(self) -> bool {
        matches!(self, Game::Nascar)
    }

    /// Ideal hot tyre pressure window in psi, where the game's cars have a known target.
    pub fn pressure_window(self) -> Option<(f32, f32)> {
        match self {
            Game::AcEvo => Some((26.0, 27.5)),
            Game::Ams2 => Some((25.5, 28.0)),
            _ => None,
        }
    }

    /// Lines shown on the "waiting for data" overlay.
    pub fn setup_lines(self, ip: &str, port: u16) -> Vec<String> {
        match self {
            Game::Nascar | Game::AcEvo => vec![
                format!("{} has no network telemetry; it uses PC shared memory.", self.name()),
                "On the gaming PC run race-dash-bridge.exe (from the GitHub release).".into(),
                format!("It broadcasts to this Pi automatically (or run: race-dash-bridge.exe {ip})."),
                format!("Bridge port: {port}"),
            ],
            Game::Ams2 => vec![
                "In AMS2: Options > System".into(),
                "Shared Memory: Project CARS 2".into(),
                "UDP Protocol Version: Project CARS 2".into(),
                "UDP Frequency: 1 (fastest)".into(),
                format!("AMS2 broadcasts on port {port}; Pi IP {ip} must be on the same network."),
            ],
            Game::Dr2 => vec![
                "Edit Documents\\My Games\\DiRT Rally 2.0\\hardwaresettings\\hardware_settings_config.xml".into(),
                format!("<udp enabled=\"true\" extradata=\"3\" ip=\"{ip}\" port=\"{port}\" delay=\"1\" />"),
                "Save, then start the game.".into(),
            ],
            Game::Wrc => vec![
                "Edit Documents\\My Games\\WRC\\telemetry\\config.json".into(),
                "In \"udp\" > \"packets\": enable the \"wrc\" structure / \"session_update\" packet".into(),
                format!("Set \"ip\": \"{ip}\" and \"port\": {port}"),
                "Restart the game after saving.".into(),
            ],
            Game::ForzaMotorsport => vec![
                "In Forza Motorsport: Settings > Gameplay & HUD".into(),
                "Data Out: On".into(),
                format!("Data Out IP Address: {ip}"),
                format!("Data Out IP Port: {port}"),
                "Data Out Packet Format: Car Dash".into(),
            ],
            Game::ForzaHorizon => vec![
                "In Forza Horizon 6: Settings > HUD and Gameplay".into(),
                "Data Out: On".into(),
                format!("Data Out IP Address: {ip}"),
                format!("Data Out IP Port: {port}  (avoid 5200-5300)"),
            ],
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Flag {
    #[default]
    None,
    Green,
    Yellow,
    Blue,
    White,
    Red,
    Checkered,
    Black,
    Caution,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tone {
    Info,
    Good,
    Warn,
    Bad,
    Purple,
}

#[derive(Clone, Debug)]
pub struct Event {
    pub text: String,
    pub tone: Tone,
    pub at: Instant,
}

/// One wheel. Index order everywhere: FL, FR, RL, RR. Temperatures in °C.
#[derive(Clone, Copy, Debug, Default)]
pub struct Corner {
    pub surface_c: Option<f32>,
    pub inner_c: Option<f32>,
    /// Inner / middle / outer (or left / centre / right) tread temps.
    pub imo: Option<[f32; 3]>,
    pub pressure_psi: Option<f32>,
    /// Percent worn (0 = new).
    pub wear_pct: Option<f32>,
    pub brake_c: Option<f32>,
    /// Suspension compression 0..1.
    pub susp: Option<f32>,
    /// Grip use: 0 = full grip, >= 1 = sliding.
    pub slip: Option<f32>,
}

#[derive(Clone, Debug, Default)]
pub struct Car {
    pub pos: Option<(f32, f32)>,
    pub pct: Option<f32>,
    pub race_pos: u32,
    pub name: String,
    pub number: String,
    pub lap: i32,
    pub player: bool,
    pub in_pit: bool,
    pub gap_leader_s: Option<f32>,
    pub interval_s: Option<f32>,
    pub best_s: Option<f32>,
    pub last_s: Option<f32>,
}

pub struct SimState {
    pub game: Game,
    pub last_packet: Option<Instant>,
    pub error: Option<String>,
    pub warning: Option<String>,

    pub speed_ms: f32,
    pub gear: i32,
    pub rpm: f32,
    pub max_rpm: f32,
    /// Game-provided shift light fraction, if any.
    pub shift_frac: Option<f32>,
    pub throttle: f32,
    pub brake: f32,
    pub clutch: f32,
    pub handbrake: f32,
    pub steer: f32,

    pub boost_psi: Option<f32>,
    pub boost_max_psi: Option<f32>,
    pub fuel_l: Option<f32>,
    pub fuel_frac: Option<f32>,
    pub fuel_laps: Option<f32>,
    pub water_c: Option<f32>,
    pub oil_c: Option<f32>,
    pub oil_bar: Option<f32>,
    pub tc_active: bool,
    pub abs_active: bool,
    pub tc_level: Option<i32>,
    pub abs_level: Option<i32>,
    /// Front brake bias percent.
    pub brake_bias: Option<f32>,
    pub pit_limiter: bool,

    pub corners: [Corner; 4],

    pub position: u32,
    pub num_cars: u32,
    pub lap: i32,
    pub total_laps: Option<i32>,
    pub current_lap_s: Option<f32>,
    pub last_lap_s: Option<f32>,
    pub best_lap_s: Option<f32>,
    /// Delta to best lap from the game itself (seconds, + = slower).
    pub native_delta_s: Option<f32>,
    pub lap_invalid: bool,
    /// Distance into the current lap, for our own delta when the game has none.
    pub lap_dist_m: Option<f32>,

    pub session_name: String,
    pub track_name: String,
    pub time_left_s: Option<f32>,

    pub stage_dist_m: Option<f32>,
    pub stage_len_m: Option<f32>,
    pub stage_time_s: Option<f32>,

    pub gap_ahead_s: Option<f32>,
    pub gap_behind_s: Option<f32>,
    pub flag: Flag,
    pub spotter_left: u8,
    pub spotter_right: u8,

    pub weather: Option<String>,
    pub air_c: Option<f32>,
    pub track_c: Option<f32>,
    pub rain_pct: Option<f32>,
    pub wind_ms: Option<f32>,

    pub g_lat: f32,
    pub g_long: f32,
    /// Heading in radians (0 = +Z / north), for compass and map.
    pub yaw: f32,
    pub yaw_rate: f32,
    pub drift_deg: f32,
    /// Player world position (x, y up, z).
    pub world: Option<(f32, f32, f32)>,
    pub lap_pct: Option<f32>,

    pub power_kw: Option<f32>,
    pub torque_nm: Option<f32>,
    pub car_info: String,
    pub in_pit: bool,
    pub pit_text: String,
    pub incidents: Option<i32>,

    pub cars: Vec<Car>,
    pub event: Option<Event>,

    // Parser scratch space (meaning depends on the game's parser).
    pub aux_dist: f32,
    pub aux_lap: i32,
    pub aux_time: f64,
    pub aux_pos: (f32, f32),
    pub names: Vec<String>,

    // Derived by `derive()`.
    pub max_speed_ms: f32,
    pub distance_m: Option<f32>,
    pub map: MapRecorder,
    pub delta: DeltaTracker,
    prev_best: Option<f32>,
    prev_position: u32,
    fuel_lap: i32,
    fuel_at_lap_start: Option<f32>,
    fuel_per_lap: Option<f32>,
}

impl SimState {
    pub fn new(game: Game) -> Self {
        Self {
            game,
            last_packet: None,
            error: None,
            warning: None,
            speed_ms: 0.0,
            gear: 0,
            rpm: 0.0,
            max_rpm: 0.0,
            shift_frac: None,
            throttle: 0.0,
            brake: 0.0,
            clutch: 0.0,
            handbrake: 0.0,
            steer: 0.0,
            boost_psi: None,
            boost_max_psi: None,
            fuel_l: None,
            fuel_frac: None,
            fuel_laps: None,
            water_c: None,
            oil_c: None,
            oil_bar: None,
            tc_active: false,
            abs_active: false,
            tc_level: None,
            abs_level: None,
            brake_bias: None,
            pit_limiter: false,
            corners: [Corner::default(); 4],
            position: 0,
            num_cars: 0,
            lap: 0,
            total_laps: None,
            current_lap_s: None,
            last_lap_s: None,
            best_lap_s: None,
            native_delta_s: None,
            lap_invalid: false,
            lap_dist_m: None,
            session_name: String::new(),
            track_name: String::new(),
            time_left_s: None,
            stage_dist_m: None,
            stage_len_m: None,
            stage_time_s: None,
            gap_ahead_s: None,
            gap_behind_s: None,
            flag: Flag::None,
            spotter_left: 0,
            spotter_right: 0,
            weather: None,
            air_c: None,
            track_c: None,
            rain_pct: None,
            wind_ms: None,
            g_lat: 0.0,
            g_long: 0.0,
            yaw: 0.0,
            yaw_rate: 0.0,
            drift_deg: 0.0,
            world: None,
            lap_pct: None,
            power_kw: None,
            torque_nm: None,
            car_info: String::new(),
            in_pit: false,
            pit_text: String::new(),
            incidents: None,
            cars: Vec::new(),
            event: None,
            aux_dist: 0.0,
            aux_lap: 0,
            aux_time: 0.0,
            aux_pos: (0.0, 0.0),
            names: Vec::new(),
            max_speed_ms: 0.0,
            distance_m: None,
            map: MapRecorder::new(game.map_mode()),
            delta: DeltaTracker::default(),
            prev_best: None,
            prev_position: 0,
            fuel_lap: 0,
            fuel_at_lap_start: None,
            fuel_per_lap: None,
        }
    }

    pub fn event(&mut self, text: impl Into<String>, tone: Tone) {
        self.event = Some(Event { text: text.into(), tone, at: Instant::now() });
    }

    /// Wipe per-session data (track map, records) when a new session/stage starts.
    pub fn new_session(&mut self) {
        self.map.reset();
        self.delta = DeltaTracker::default();
        self.max_speed_ms = 0.0;
        self.best_lap_s = None;
        self.prev_best = None;
        self.prev_position = 0;
        self.cars.clear();
    }

    /// Run after every packet: track map, delta, records and automatic pop-ups.
    pub fn derive(&mut self) {
        if self.speed_ms.is_finite() && self.speed_ms > self.max_speed_ms && self.speed_ms < 200.0 {
            self.max_speed_ms = self.speed_ms;
        }
        if let Some((x, _, z)) = self.world {
            self.map.add(x, z, self.lap_pct);
        }
        if self.native_delta_s.is_none() {
            if let (Some(dist), Some(cur)) = (self.lap_dist_m, self.current_lap_s) {
                let last_ms = self.last_lap_s.map_or(0, |s| (s * 1000.0) as u32);
                self.delta.update(
                    self.lap.clamp(0, 255) as u8,
                    dist,
                    (cur.max(0.0) * 1000.0) as u32,
                    self.lap_invalid,
                    last_ms,
                );
            }
        }
        // Fuel per lap (rolling average) -> laps of fuel left, when the game doesn't say.
        let fuel_now = self.fuel_l.or(self.fuel_frac.map(|f| f * 100.0));
        if self.lap > 0 && self.lap != self.fuel_lap {
            if let (Some(start), Some(now)) = (self.fuel_at_lap_start, fuel_now) {
                let used = start - now;
                if used > 0.0 && self.lap == self.fuel_lap + 1 {
                    self.fuel_per_lap = Some(self.fuel_per_lap.map_or(used, |avg| avg * 0.6 + used * 0.4));
                }
            }
            self.fuel_lap = self.lap;
            self.fuel_at_lap_start = fuel_now;
        }
        if let (Some(per_lap), Some(now)) = (self.fuel_per_lap, fuel_now) {
            if per_lap > 0.0 && self.game != Game::Nascar && self.game != Game::AcEvo {
                self.fuel_laps = Some(now / per_lap);
            }
        }
        // Personal best pop-up.
        if let Some(best) = self.best_lap_s {
            let improved = self.prev_best.map_or(false, |pb| best < pb - 0.0005);
            if improved {
                self.event(format!("PERSONAL BEST  {}", view::fmt_lap(best)), Tone::Purple);
            }
            self.prev_best = Some(best);
        }
        // Position change pop-up (races with a field only).
        if self.position > 0 && self.num_cars > 1 {
            if self.prev_position > 0 && self.position != self.prev_position {
                let tone = if self.position < self.prev_position { Tone::Good } else { Tone::Warn };
                self.event(format!("P{}  >  P{}", self.prev_position, self.position), tone);
            }
            self.prev_position = self.position;
        }
    }

    pub fn delta_s(&self) -> Option<f32> {
        self.native_delta_s.or_else(|| if self.delta.has_reference() { self.delta.delta } else { None })
    }
}

/// Parse one received datagram for `game`.
fn handle_packet(st: &mut SimState, buf: &[u8]) {
    match st.game {
        Game::ForzaMotorsport | Game::ForzaHorizon => super::forza::handle(st, buf),
        Game::Dr2 => super::dr2::handle(st, buf),
        Game::Wrc => super::wrc::handle(st, buf),
        Game::Ams2 => super::ams2::handle(st, buf),
        Game::Nascar | Game::AcEvo => super::bridge_rx::handle(st, buf),
    }
}

fn run_udp(shared: Arc<Mutex<SimState>>, stop: Arc<AtomicBool>, port: u16) {
    let socket = match UdpSocket::bind(("0.0.0.0", port)) {
        Ok(s) => s,
        Err(e) => {
            if let Ok(mut st) = shared.lock() {
                st.error = Some(format!("Cannot listen on UDP port {port}: {e}"));
            }
            return;
        }
    };
    let _ = socket.set_broadcast(true);
    let _ = socket.set_read_timeout(Some(Duration::from_millis(200)));
    let mut buf = [0u8; 4096];
    while !stop.load(Ordering::Relaxed) {
        if let Ok(n) = socket.recv(&mut buf) {
            if let Ok(mut st) = shared.lock() {
                st.last_packet = Some(Instant::now());
                handle_packet(&mut st, &buf[..n]);
                st.derive();
            }
        }
    }
}

/// A running session for one of the shared-engine games.
pub struct SimSession {
    stop: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
    _timer: slint::Timer,
}

pub struct SessionOptions {
    pub game: Game,
    pub demo: bool,
    pub port: u16,
    pub imperial: bool,
    pub ip: String,
    /// Track map orientation, shared with the "rotate map" button.
    pub map_orient: Rc<Cell<u8>>,
}

impl SimSession {
    pub fn start(ui: &crate::AppWindow, opts: SessionOptions) -> Self {
        let game = opts.game;
        let shared = Arc::new(Mutex::new(SimState::new(game)));
        let stop = Arc::new(AtomicBool::new(false));

        let worker = {
            let shared = shared.clone();
            let stop = stop.clone();
            let (demo, port) = (opts.demo, opts.port);
            std::thread::Builder::new()
                .name(format!("{}-telemetry", game.key()))
                .spawn(move || {
                    if demo {
                        demo::run(shared, stop)
                    } else {
                        run_udp(shared, stop, port)
                    }
                })
                .ok()
        };

        let sim = ui.global::<crate::Sim>();
        let lines: Vec<slint::SharedString> =
            game.setup_lines(&opts.ip, opts.port).into_iter().map(Into::into).collect();
        sim.set_setup_lines(slint::ModelRc::new(slint::VecModel::from(lines)));
        sim.set_game_name(game.name().into());
        sim.set_dial_track(dials::arc(1.0).into());

        let mut renderer = view::Renderer::new(ui, opts.imperial, opts.map_orient.clone());
        let timer = slint::Timer::default();
        let weak = ui.as_weak();
        timer.start(slint::TimerMode::Repeated, Duration::from_millis(33), move || {
            let Some(ui) = weak.upgrade() else { return };
            let state = match shared.lock() {
                Ok(g) => g,
                Err(p) => p.into_inner(),
            };
            renderer.apply(&ui, &state);
        });

        Self { stop, worker, _timer: timer }
    }
}

impl Drop for SimSession {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(h) = self.worker.take() {
            let _ = h.join();
        }
    }
}

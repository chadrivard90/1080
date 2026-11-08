//! Parser for the F1 25 UDP telemetry format (`m_packetFormat` = 2025).
//!
//! Layouts follow EA's "Data Output from F1 25 v3" spec: all values little-endian and packed,
//! 22 cars per array, wheel arrays ordered RL, RR, FL, FR.

use std::time::Instant;

use crate::games::map::MapRecorder;

pub const NUM_CARS: usize = 22;
pub const PACKET_FORMAT: u16 = 2025;
pub const HEADER_SIZE: usize = 29;

/// Wheel array indexes.
pub const RL: usize = 0;
pub const RR: usize = 1;
pub const FL: usize = 2;
pub const FR: usize = 3;

// Per-car struct sizes in bytes.
const CAR_MOTION_SIZE: usize = 60;
const LAP_DATA_SIZE: usize = 57;
const PARTICIPANT_SIZE: usize = 57;
const CAR_SETUP_SIZE: usize = 50;
const CAR_TELEMETRY_SIZE: usize = 60;
const CAR_STATUS_SIZE: usize = 55;
const CAR_DAMAGE_SIZE: usize = 46;
const LAP_HISTORY_SIZE: usize = 14;

/// Full packet sizes for the packets we decode. Shorter packets are rejected.
pub fn expected_size(packet_id: u8) -> Option<usize> {
    Some(match packet_id {
        0 => 1349,  // Motion
        1 => 753,   // Session
        2 => 1285,  // Lap data
        3 => 45,    // Event
        4 => 1284,  // Participants
        5 => 1133,  // Car setups
        6 => 1352,  // Car telemetry
        7 => 1239,  // Car status
        10 => 1041, // Car damage
        11 => 1460, // Session history
        _ => return None,
    })
}

/// Bounds-safe little-endian reader. Reads past the end yield zeros instead of panicking,
/// so a malformed packet can never take down the listener thread.
pub struct Reader<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    pub fn new(buf: &'a [u8], pos: usize) -> Self {
        Self { buf, pos }
    }
    fn take<const N: usize>(&mut self) -> [u8; N] {
        let mut out = [0u8; N];
        if let Some(src) = self.buf.get(self.pos..self.pos + N) {
            out.copy_from_slice(src);
        }
        self.pos += N;
        out
    }
    pub fn u8(&mut self) -> u8 {
        self.take::<1>()[0]
    }
    pub fn i8(&mut self) -> i8 {
        self.u8() as i8
    }
    pub fn u16(&mut self) -> u16 {
        u16::from_le_bytes(self.take())
    }
    pub fn u32(&mut self) -> u32 {
        u32::from_le_bytes(self.take())
    }
    pub fn u64(&mut self) -> u64 {
        u64::from_le_bytes(self.take())
    }
    pub fn f32(&mut self) -> f32 {
        let v = f32::from_le_bytes(self.take());
        if v.is_finite() {
            v
        } else {
            0.0
        }
    }
    pub fn skip(&mut self, n: usize) {
        self.pos += n;
    }
    pub fn bytes(&mut self, n: usize) -> &'a [u8] {
        let s = self.buf.get(self.pos..self.pos + n).unwrap_or(&[]);
        self.pos += n;
        s
    }
    fn u8x4(&mut self) -> [u8; 4] {
        [self.u8(), self.u8(), self.u8(), self.u8()]
    }
    fn f32x4(&mut self) -> [f32; 4] {
        [self.f32(), self.f32(), self.f32(), self.f32()]
    }
    fn u16x4(&mut self) -> [u16; 4] {
        [self.u16(), self.u16(), self.u16(), self.u16()]
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Header {
    pub packet_format: u16,
    pub packet_id: u8,
    pub session_uid: u64,
    pub session_time: f32,
    pub player_car_index: u8,
}

pub fn parse_header(buf: &[u8]) -> Option<Header> {
    if buf.len() < HEADER_SIZE {
        return None;
    }
    let mut r = Reader::new(buf, 0);
    let packet_format = r.u16();
    r.skip(4); // game year, major, minor, packet version
    let packet_id = r.u8();
    let session_uid = r.u64();
    let session_time = r.f32();
    r.skip(8); // frame identifier, overall frame identifier
    let player_car_index = r.u8();
    Some(Header { packet_format, packet_id, session_uid, session_time, player_car_index })
}

#[derive(Clone, Copy, Debug, Default)]
pub struct CarMotion {
    /// World position (metres) on the ground plane.
    pub x: f32,
    pub z: f32,
    pub g_lat: f32,
    pub g_long: f32,
    pub g_vert: f32,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct WeatherSample {
    pub session_type: u8,
    pub time_offset_min: u8,
    pub weather: u8,
    pub track_temp: i8,
    pub air_temp: i8,
    pub rain_pct: u8,
}

#[derive(Clone, Debug, Default)]
pub struct Session {
    pub received: bool,
    pub weather: u8,
    pub track_temp: i8,
    pub air_temp: i8,
    pub total_laps: u8,
    pub track_length: u16,
    pub session_type: u8,
    pub track_id: i8,
    pub time_left_s: u16,
    pub duration_s: u16,
    pub pit_speed_limit: u8,
    pub paused: bool,
    pub safety_car: u8,
    pub forecast: Vec<WeatherSample>,
    pub pit_ideal_lap: u8,
    pub pit_latest_lap: u8,
    pub pit_rejoin_pos: u8,
    /// 0 = MPH, 1 = KPH
    pub speed_units: u8,
    /// 0 = Celsius, 1 = Fahrenheit
    pub temp_units: u8,
}

impl Session {
    pub fn is_race(&self) -> bool {
        (15..=17).contains(&self.session_type)
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct LapData {
    pub last_lap_ms: u32,
    pub current_lap_ms: u32,
    pub sector1_ms: u32,
    pub sector2_ms: u32,
    pub delta_front_ms: u32,
    pub delta_leader_ms: u32,
    pub lap_distance: f32,
    pub position: u8,
    pub lap_num: u8,
    /// 0 = none, 1 = pitting, 2 = in pit area
    pub pit_status: u8,
    pub num_pit_stops: u8,
    /// 0 = sector 1, 1 = sector 2, 2 = sector 3
    pub sector: u8,
    pub lap_invalid: bool,
    pub penalties_s: u8,
    pub warnings: u8,
    pub cc_warnings: u8,
    pub unserved_dt: u8,
    pub unserved_sg: u8,
    /// 0 = garage, 1 = flying lap, 2 = in lap, 3 = out lap, 4 = on track
    pub driver_status: u8,
    /// 0 invalid, 1 inactive, 2 active, 3 finished, 4 DNF, 5 DSQ, 6 not classified, 7 retired
    pub result_status: u8,
    pub speed_trap_kph: f32,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct CarTelemetry {
    pub speed_kph: u16,
    pub throttle: f32,
    pub steer: f32,
    pub brake: f32,
    pub clutch: u8,
    pub gear: i8,
    pub rpm: u16,
    pub drs: u8,
    pub rev_pct: u8,
    pub rev_bits: u16,
    pub brake_temp: [u16; 4],
    pub tyre_surface_temp: [u8; 4],
    pub tyre_inner_temp: [u8; 4],
    pub engine_temp: u16,
    pub tyre_pressure: [f32; 4],
}

#[derive(Clone, Copy, Debug, Default)]
pub struct CarStatus {
    pub traction_control: u8,
    pub abs: u8,
    pub fuel_mix: u8,
    pub brake_bias: u8,
    pub pit_limiter: u8,
    pub fuel_in_tank: f32,
    pub fuel_capacity: f32,
    pub fuel_remaining_laps: f32,
    pub max_rpm: u16,
    pub idle_rpm: u16,
    pub max_gears: u8,
    pub drs_allowed: u8,
    pub drs_activation_distance: u16,
    pub actual_compound: u8,
    pub visual_compound: u8,
    pub tyre_age_laps: u8,
    pub fia_flag: i8,
    pub ers_store: f32,
    pub ers_deploy_mode: u8,
    pub ers_harvested_mguk: f32,
    pub ers_harvested_mguh: f32,
    pub ers_deployed: f32,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct CarDamage {
    pub tyre_wear: [f32; 4],
    pub tyre_damage: [u8; 4],
    pub brake_damage: [u8; 4],
    pub tyre_blisters: [u8; 4],
    pub fl_wing: u8,
    pub fr_wing: u8,
    pub rear_wing: u8,
    pub floor: u8,
    pub diffuser: u8,
    pub sidepod: u8,
    pub drs_fault: u8,
    pub ers_fault: u8,
    pub gearbox: u8,
    pub engine: u8,
    pub wear_mguh: u8,
    pub wear_es: u8,
    pub wear_ce: u8,
    pub wear_ice: u8,
    pub wear_mguk: u8,
    pub wear_tc: u8,
    pub engine_blown: u8,
    pub engine_seized: u8,
}

#[derive(Clone, Debug, Default)]
pub struct Participant {
    pub ai: bool,
    pub driver_id: u8,
    pub team_id: u8,
    pub race_number: u8,
    pub name: String,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct CarSetup {
    pub front_wing: u8,
    pub rear_wing: u8,
    pub diff_on: u8,
    pub diff_off: u8,
    pub brake_pressure: u8,
    pub brake_bias: u8,
    pub engine_braking: u8,
    pub fuel_load: f32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct LapHistory {
    pub time_ms: u32,
    pub sectors_ms: [u32; 3],
    /// 0x01 lap valid, 0x02/0x04/0x08 sector 1/2/3 valid
    pub flags: u8,
}

impl LapHistory {
    pub fn valid(&self) -> bool {
        self.flags & 0x01 != 0
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct TyreStint {
    pub end_lap: u8,
    pub actual: u8,
    pub visual: u8,
}

#[derive(Clone, Debug, Default)]
pub struct CarHistory {
    /// Number of laps in the data, including the current partial lap.
    pub num_laps: u8,
    pub best_lap_num: u8,
    pub best_sector_lap: [u8; 3],
    pub laps: Vec<LapHistory>,
    pub stints: Vec<TyreStint>,
}

impl CarHistory {
    fn lap(&self, lap_num: u8) -> Option<&LapHistory> {
        if lap_num == 0 {
            return None;
        }
        self.laps.get(lap_num as usize - 1)
    }
    pub fn best_lap_ms(&self) -> u32 {
        self.lap(self.best_lap_num).map_or(0, |l| l.time_ms)
    }
    pub fn best_sector_ms(&self, sector: usize) -> u32 {
        self.lap(self.best_sector_lap[sector]).map_or(0, |l| l.sectors_ms[sector])
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EventKind {
    Info,
    Purple,
    Warning,
    Danger,
    Good,
}

#[derive(Clone, Debug)]
pub struct EventMsg {
    pub text: String,
    pub kind: EventKind,
    pub at: Instant,
}

/// Live "delta to best lap": records (distance, time) samples of the current lap and compares
/// against the fastest valid lap seen this session.
#[derive(Clone, Debug, Default)]
pub struct DeltaTracker {
    current: Vec<(f32, u32)>,
    reference: Vec<(f32, u32)>,
    reference_time_ms: u32,
    lap_num: u8,
    current_invalid: bool,
    /// Seconds versus the reference lap (negative = faster). None when no reference yet.
    pub delta: Option<f32>,
}

impl DeltaTracker {
    pub fn update(&mut self, lap_num: u8, distance: f32, time_ms: u32, invalid: bool, last_lap_ms: u32) {
        if lap_num != self.lap_num {
            let completed_one_lap = self.lap_num != 0 && lap_num == self.lap_num.wrapping_add(1);
            if completed_one_lap
                && !self.current_invalid
                && last_lap_ms > 0
                && self.current.len() > 50
                && (self.reference_time_ms == 0 || last_lap_ms < self.reference_time_ms)
            {
                let mut lap = std::mem::take(&mut self.current);
                // Close the reference at the finish line so the end of the lap interpolates.
                if let Some(&(last_d, _)) = lap.last() {
                    lap.push((last_d + 50.0, last_lap_ms));
                }
                self.reference = lap;
                self.reference_time_ms = last_lap_ms;
            }
            self.current.clear();
            self.current_invalid = false;
            self.lap_num = lap_num;
        }
        if invalid {
            self.current_invalid = true;
        }
        if distance < 0.0 {
            // Before the start line on the first lap.
            self.delta = None;
            return;
        }
        // Flashback / reset: drop samples that are now ahead of the car.
        while matches!(self.current.last(), Some(&(d, _)) if d > distance + 1.0) {
            self.current.pop();
        }
        if self.current.last().map_or(true, |&(d, _)| distance - d >= 2.0) {
            self.current.push((distance, time_ms));
        }
        self.delta = self.reference_time_at(distance).map(|ref_ms| (time_ms as f32 - ref_ms) / 1000.0);
    }

    pub fn has_reference(&self) -> bool {
        self.reference_time_ms > 0
    }

    fn reference_time_at(&self, distance: f32) -> Option<f32> {
        let r = &self.reference;
        if r.len() < 2 {
            return None;
        }
        let idx = r.partition_point(|&(d, _)| d < distance);
        if idx == 0 {
            let (d0, t0) = r[0];
            return Some(if d0 > 0.0 { t0 as f32 * (distance / d0) } else { t0 as f32 });
        }
        if idx >= r.len() {
            return None;
        }
        let (d0, t0) = r[idx - 1];
        let (d1, t1) = r[idx];
        let span = (d1 - d0).max(0.001);
        let f = ((distance - d0) / span).clamp(0.0, 1.0);
        Some(t0 as f32 + (t1 as f32 - t0 as f32) * f)
    }
}

/// Everything we know about the current session, updated as packets arrive.
#[derive(Default)]
pub struct F1State {
    pub last_packet: Option<Instant>,
    pub wrong_format: Option<u16>,
    pub error: Option<String>,
    pub session_uid: u64,
    pub player: usize,
    pub num_active: u8,
    pub session: Session,
    pub laps: [LapData; NUM_CARS],
    pub telemetry: [CarTelemetry; NUM_CARS],
    pub status: [CarStatus; NUM_CARS],
    pub damage: [CarDamage; NUM_CARS],
    pub participants: [Participant; NUM_CARS],
    pub motion: [CarMotion; NUM_CARS],
    pub history: [CarHistory; NUM_CARS],
    pub setup: CarSetup,
    pub suggested_gear: i8,
    pub event: Option<EventMsg>,
    pub red_flag_at: Option<Instant>,
    pub delta: DeltaTracker,
    /// Track outline recorded from the player's driving line.
    pub map: MapRecorder,
}

impl F1State {
    pub fn handle_packet(&mut self, buf: &[u8]) {
        let Some(h) = parse_header(buf) else { return };
        self.last_packet = Some(Instant::now());
        if h.packet_format != PACKET_FORMAT {
            self.wrong_format = Some(h.packet_format);
            return;
        }
        self.wrong_format = None;
        if let Some(size) = expected_size(h.packet_id) {
            if buf.len() < size {
                return;
            }
        }
        if h.session_uid != 0 && h.session_uid != self.session_uid {
            self.reset(h.session_uid);
        }
        if (h.player_car_index as usize) < NUM_CARS {
            self.player = h.player_car_index as usize;
        }
        match h.packet_id {
            0 => self.parse_motion(buf),
            1 => self.parse_session(buf),
            2 => self.parse_lap_data(buf),
            3 => self.parse_event(buf),
            4 => self.parse_participants(buf),
            5 => self.parse_setups(buf),
            6 => self.parse_telemetry(buf),
            7 => self.parse_status(buf),
            10 => self.parse_damage(buf),
            11 => self.parse_history(buf),
            _ => {}
        }
    }

    fn reset(&mut self, session_uid: u64) {
        let last_packet = self.last_packet;
        *self = F1State::default();
        self.session_uid = session_uid;
        self.last_packet = last_packet;
    }

    /// Display name for a car index.
    pub fn name(&self, idx: usize) -> String {
        match self.participants.get(idx) {
            Some(p) if !p.name.is_empty() => p.name.to_uppercase(),
            _ => format!("CAR {}", idx + 1),
        }
    }

    /// Fastest lap of the session: (car index, lap time ms).
    pub fn session_best(&self) -> Option<(usize, u32)> {
        (0..NUM_CARS)
            .map(|i| (i, self.history[i].best_lap_ms()))
            .filter(|&(_, t)| t > 0)
            .min_by_key(|&(_, t)| t)
    }

    /// Fastest time for a sector across all cars.
    pub fn overall_best_sector(&self, sector: usize) -> u32 {
        (0..NUM_CARS)
            .map(|i| self.history[i].best_sector_ms(sector))
            .filter(|&t| t > 0)
            .min()
            .unwrap_or(0)
    }

    pub fn update_delta(&mut self) {
        let l = self.laps[self.player];
        self.delta.update(l.lap_num, l.lap_distance, l.current_lap_ms, l.lap_invalid, l.last_lap_ms);
    }

    fn parse_motion(&mut self, b: &[u8]) {
        for i in 0..NUM_CARS {
            let base = HEADER_SIZE + i * CAR_MOTION_SIZE;
            let mut p = Reader::new(b, base);
            let x = p.f32();
            p.skip(4); // y
            let z = p.f32();
            // Skip position (12), velocity (12), forward dir (6), right dir (6) = 36 bytes.
            let mut r = Reader::new(b, base + 36);
            self.motion[i] = CarMotion { x, z, g_lat: r.f32(), g_long: r.f32(), g_vert: r.f32() };
        }
        let me = self.motion[self.player];
        let len = self.session.track_length as f32;
        let dist = self.laps[self.player].lap_distance;
        let pct = (len > 0.0 && dist >= 0.0).then(|| (dist / len).clamp(0.0, 1.0));
        self.map.add(me.x, me.z, pct);
    }

    fn parse_session(&mut self, b: &[u8]) {
        let mut r = Reader::new(b, HEADER_SIZE);
        let s = &mut self.session;
        s.received = true;
        s.weather = r.u8();
        s.track_temp = r.i8();
        s.air_temp = r.i8();
        s.total_laps = r.u8();
        s.track_length = r.u16();
        s.session_type = r.u8();
        s.track_id = r.i8();
        r.skip(1); // formula
        s.time_left_s = r.u16();
        s.duration_s = r.u16();
        s.pit_speed_limit = r.u8();
        s.paused = r.u8() != 0;
        r.skip(3); // is spectating, spectator car index, SLI Pro support
        r.skip(1 + 21 * 5); // num marshal zones + 21 marshal zones
        s.safety_car = r.u8();
        r.skip(1); // network game
        let num_forecast = (r.u8() as usize).min(64);
        s.forecast.clear();
        for k in 0..64 {
            let sample = WeatherSample {
                session_type: r.u8(),
                time_offset_min: r.u8(),
                weather: r.u8(),
                track_temp: r.i8(),
                air_temp: {
                    r.skip(1); // track temp change
                    r.i8()
                },
                rain_pct: {
                    r.skip(1); // air temp change
                    r.u8()
                },
            };
            if k < num_forecast {
                s.forecast.push(sample);
            }
        }
        r.skip(2); // forecast accuracy, AI difficulty
        r.skip(12); // season / weekend / session link identifiers
        s.pit_ideal_lap = r.u8();
        s.pit_latest_lap = r.u8();
        s.pit_rejoin_pos = r.u8();
        r.skip(9); // assists
        r.skip(2); // game mode, ruleset
        r.skip(4); // time of day
        r.skip(1); // session length
        s.speed_units = r.u8();
        s.temp_units = r.u8();
    }

    fn parse_lap_data(&mut self, b: &[u8]) {
        for i in 0..NUM_CARS {
            let mut r = Reader::new(b, HEADER_SIZE + i * LAP_DATA_SIZE);
            let last_lap_ms = r.u32();
            let current_lap_ms = r.u32();
            let s1_ms = r.u16() as u32;
            let s1_min = r.u8() as u32;
            let s2_ms = r.u16() as u32;
            let s2_min = r.u8() as u32;
            let front_ms = r.u16() as u32;
            let front_min = r.u8() as u32;
            let leader_ms = r.u16() as u32;
            let leader_min = r.u8() as u32;
            let lap_distance = r.f32();
            r.skip(4); // total distance
            r.skip(4); // safety car delta
            let position = r.u8();
            let lap_num = r.u8();
            let pit_status = r.u8();
            let num_pit_stops = r.u8();
            let sector = r.u8();
            let lap_invalid = r.u8() != 0;
            let penalties_s = r.u8();
            let warnings = r.u8();
            let cc_warnings = r.u8();
            let unserved_dt = r.u8();
            let unserved_sg = r.u8();
            r.skip(1); // grid position
            let driver_status = r.u8();
            let result_status = r.u8();
            r.skip(1 + 2 + 2 + 1); // pit lane timer active, time in lane, pit stop timer, serve pen
            let speed_trap_kph = r.f32();
            self.laps[i] = LapData {
                last_lap_ms,
                current_lap_ms,
                sector1_ms: s1_min * 60_000 + s1_ms,
                sector2_ms: s2_min * 60_000 + s2_ms,
                delta_front_ms: front_min * 60_000 + front_ms,
                delta_leader_ms: leader_min * 60_000 + leader_ms,
                lap_distance,
                position,
                lap_num,
                pit_status,
                num_pit_stops,
                sector,
                lap_invalid,
                penalties_s,
                warnings,
                cc_warnings,
                unserved_dt,
                unserved_sg,
                driver_status,
                result_status,
                speed_trap_kph,
            };
        }
        self.update_delta();
    }

    fn parse_event(&mut self, b: &[u8]) {
        let code = [b[29], b[30], b[31], b[32]];
        let mut r = Reader::new(b, 33);
        let player = self.player;
        let msg: Option<(String, EventKind)> = match &code {
            b"SSTA" => Some(("SESSION STARTED".into(), EventKind::Info)),
            b"SEND" => Some(("SESSION ENDED".into(), EventKind::Info)),
            b"FTLP" => {
                let idx = r.u8() as usize;
                let time = r.f32();
                Some((
                    format!("FASTEST LAP  {}  {}", self.name(idx), super::view::fmt_lap_secs(time)),
                    EventKind::Purple,
                ))
            }
            b"RTMT" => {
                let idx = r.u8() as usize;
                Some((format!("RETIRED  {}", self.name(idx)), EventKind::Warning))
            }
            b"DRSE" => Some(("DRS ENABLED".into(), EventKind::Good)),
            b"DRSD" => {
                let reason = match r.u8() {
                    0 => "  WET TRACK",
                    1 => "  SAFETY CAR",
                    2 => "  RED FLAG",
                    _ => "",
                };
                Some((format!("DRS DISABLED{reason}"), EventKind::Warning))
            }
            b"TMPT" => Some(("TEAMMATE IN THE PITS".into(), EventKind::Info)),
            b"CHQF" => Some(("CHEQUERED FLAG".into(), EventKind::Info)),
            b"RCWN" => {
                let idx = r.u8() as usize;
                Some((format!("WINNER  {}", self.name(idx)), EventKind::Purple))
            }
            b"PENA" => {
                let penalty_type = r.u8();
                let _infringement = r.u8();
                let idx = r.u8() as usize;
                let _other = r.u8();
                let time = r.u8();
                if idx == player {
                    Some((super::lookup::penalty_text(penalty_type, time), EventKind::Danger))
                } else {
                    None
                }
            }
            b"STLG" => Some((format!("START LIGHTS  {}", r.u8()), EventKind::Danger)),
            b"LGOT" => Some(("LIGHTS OUT".into(), EventKind::Good)),
            b"DTSV" => {
                let idx = r.u8() as usize;
                (idx == player).then(|| ("DRIVE THROUGH SERVED".to_string(), EventKind::Info))
            }
            b"SGSV" => {
                let idx = r.u8() as usize;
                (idx == player).then(|| ("STOP GO SERVED".to_string(), EventKind::Info))
            }
            b"RDFL" => {
                self.red_flag_at = Some(Instant::now());
                Some(("RED FLAG".into(), EventKind::Danger))
            }
            b"OVTK" => {
                let overtaker = r.u8() as usize;
                let overtaken = r.u8() as usize;
                if overtaker == player {
                    Some((format!("OVERTOOK  {}", self.name(overtaken)), EventKind::Good))
                } else if overtaken == player {
                    Some((format!("OVERTAKEN BY  {}", self.name(overtaker)), EventKind::Warning))
                } else {
                    None
                }
            }
            b"SCAR" => {
                let sc_type = r.u8();
                let event = r.u8();
                let kind = match sc_type {
                    2 => "VIRTUAL SAFETY CAR",
                    3 => "FORMATION LAP",
                    _ => "SAFETY CAR",
                };
                let what = match event {
                    0 => "DEPLOYED",
                    1 => "IN THIS LAP",
                    2 => "RETURNED",
                    _ => "ENDING",
                };
                Some((format!("{kind} {what}"), EventKind::Warning))
            }
            b"COLL" => {
                let a = r.u8() as usize;
                let c = r.u8() as usize;
                if a == player {
                    Some((format!("CONTACT  {}", self.name(c)), EventKind::Warning))
                } else if c == player {
                    Some((format!("CONTACT  {}", self.name(a)), EventKind::Warning))
                } else {
                    None
                }
            }
            _ => None,
        };
        if let Some((text, kind)) = msg {
            self.event = Some(EventMsg { text, kind, at: Instant::now() });
        }
    }

    fn parse_participants(&mut self, b: &[u8]) {
        self.num_active = b[HEADER_SIZE];
        for i in 0..NUM_CARS {
            let mut r = Reader::new(b, HEADER_SIZE + 1 + i * PARTICIPANT_SIZE);
            let ai = r.u8() != 0;
            let driver_id = r.u8();
            r.skip(1); // network id
            let team_id = r.u8();
            r.skip(1); // my team
            let race_number = r.u8();
            r.skip(1); // nationality
            let raw = r.bytes(32);
            let end = raw.iter().position(|&c| c == 0).unwrap_or(raw.len());
            let name = String::from_utf8_lossy(&raw[..end]).trim().to_string();
            self.participants[i] = Participant { ai, driver_id, team_id, race_number, name };
        }
    }

    fn parse_setups(&mut self, b: &[u8]) {
        let mut r = Reader::new(b, HEADER_SIZE + self.player * CAR_SETUP_SIZE);
        let front_wing = r.u8();
        let rear_wing = r.u8();
        let diff_on = r.u8();
        let diff_off = r.u8();
        r.skip(16); // camber / toe
        r.skip(6); // suspension, anti-roll bars, ride heights
        let brake_pressure = r.u8();
        let brake_bias = r.u8();
        let engine_braking = r.u8();
        r.skip(16); // tyre pressures
        r.skip(1); // ballast
        let fuel_load = r.f32();
        self.setup = CarSetup {
            front_wing,
            rear_wing,
            diff_on,
            diff_off,
            brake_pressure,
            brake_bias,
            engine_braking,
            fuel_load,
        };
    }

    fn parse_telemetry(&mut self, b: &[u8]) {
        for i in 0..NUM_CARS {
            let mut r = Reader::new(b, HEADER_SIZE + i * CAR_TELEMETRY_SIZE);
            self.telemetry[i] = CarTelemetry {
                speed_kph: r.u16(),
                throttle: r.f32(),
                steer: r.f32(),
                brake: r.f32(),
                clutch: r.u8(),
                gear: r.i8(),
                rpm: r.u16(),
                drs: r.u8(),
                rev_pct: r.u8(),
                rev_bits: r.u16(),
                brake_temp: r.u16x4(),
                tyre_surface_temp: r.u8x4(),
                tyre_inner_temp: r.u8x4(),
                engine_temp: r.u16(),
                tyre_pressure: r.f32x4(),
            };
            // + 4 bytes surface type (unused)
        }
        let mut r = Reader::new(b, HEADER_SIZE + NUM_CARS * CAR_TELEMETRY_SIZE);
        r.skip(2); // MFD panel indexes
        self.suggested_gear = r.i8();
    }

    fn parse_status(&mut self, b: &[u8]) {
        for i in 0..NUM_CARS {
            let mut r = Reader::new(b, HEADER_SIZE + i * CAR_STATUS_SIZE);
            let traction_control = r.u8();
            let abs = r.u8();
            let fuel_mix = r.u8();
            let brake_bias = r.u8();
            let pit_limiter = r.u8();
            let fuel_in_tank = r.f32();
            let fuel_capacity = r.f32();
            let fuel_remaining_laps = r.f32();
            let max_rpm = r.u16();
            let idle_rpm = r.u16();
            let max_gears = r.u8();
            let drs_allowed = r.u8();
            let drs_activation_distance = r.u16();
            let actual_compound = r.u8();
            let visual_compound = r.u8();
            let tyre_age_laps = r.u8();
            let fia_flag = r.i8();
            r.skip(8); // engine power ICE, MGU-K
            let ers_store = r.f32();
            let ers_deploy_mode = r.u8();
            let ers_harvested_mguk = r.f32();
            let ers_harvested_mguh = r.f32();
            let ers_deployed = r.f32();
            self.status[i] = CarStatus {
                traction_control,
                abs,
                fuel_mix,
                brake_bias,
                pit_limiter,
                fuel_in_tank,
                fuel_capacity,
                fuel_remaining_laps,
                max_rpm,
                idle_rpm,
                max_gears,
                drs_allowed,
                drs_activation_distance,
                actual_compound,
                visual_compound,
                tyre_age_laps,
                fia_flag,
                ers_store,
                ers_deploy_mode,
                ers_harvested_mguk,
                ers_harvested_mguh,
                ers_deployed,
            };
        }
    }

    fn parse_damage(&mut self, b: &[u8]) {
        for i in 0..NUM_CARS {
            let mut r = Reader::new(b, HEADER_SIZE + i * CAR_DAMAGE_SIZE);
            self.damage[i] = CarDamage {
                tyre_wear: r.f32x4(),
                tyre_damage: r.u8x4(),
                brake_damage: r.u8x4(),
                tyre_blisters: r.u8x4(),
                fl_wing: r.u8(),
                fr_wing: r.u8(),
                rear_wing: r.u8(),
                floor: r.u8(),
                diffuser: r.u8(),
                sidepod: r.u8(),
                drs_fault: r.u8(),
                ers_fault: r.u8(),
                gearbox: r.u8(),
                engine: r.u8(),
                wear_mguh: r.u8(),
                wear_es: r.u8(),
                wear_ce: r.u8(),
                wear_ice: r.u8(),
                wear_mguk: r.u8(),
                wear_tc: r.u8(),
                engine_blown: r.u8(),
                engine_seized: r.u8(),
            };
        }
    }

    fn parse_history(&mut self, b: &[u8]) {
        let mut r = Reader::new(b, HEADER_SIZE);
        let car = r.u8() as usize;
        if car >= NUM_CARS {
            return;
        }
        let num_laps = r.u8().min(100);
        let num_stints = r.u8().min(8);
        let best_lap_num = r.u8();
        let best_sector_lap = [r.u8(), r.u8(), r.u8()];
        let mut laps = Vec::with_capacity(num_laps as usize);
        for k in 0..100usize {
            let mut lr = Reader::new(b, HEADER_SIZE + 7 + k * LAP_HISTORY_SIZE);
            if k >= num_laps as usize {
                break;
            }
            let time_ms = lr.u32();
            let s1 = lr.u16() as u32 + lr.u8() as u32 * 60_000;
            let s2 = lr.u16() as u32 + lr.u8() as u32 * 60_000;
            let s3 = lr.u16() as u32 + lr.u8() as u32 * 60_000;
            let flags = lr.u8();
            laps.push(LapHistory { time_ms, sectors_ms: [s1, s2, s3], flags });
        }
        let mut stints = Vec::with_capacity(num_stints as usize);
        let mut sr = Reader::new(b, HEADER_SIZE + 7 + 100 * LAP_HISTORY_SIZE);
        for k in 0..8 {
            let stint = TyreStint { end_lap: sr.u8(), actual: sr.u8(), visual: sr.u8() };
            if k < num_stints as usize {
                stints.push(stint);
            }
        }
        self.history[car] = CarHistory { num_laps, best_lap_num, best_sector_lap, laps, stints };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn packet(id: u8, size: usize, player: u8) -> Vec<u8> {
        let mut b = vec![0u8; size];
        b[0..2].copy_from_slice(&PACKET_FORMAT.to_le_bytes());
        b[2] = 25;
        b[6] = id;
        b[7..15].copy_from_slice(&42u64.to_le_bytes());
        b[27] = player;
        b[28] = 255;
        b
    }

    #[test]
    fn struct_sizes_add_up_to_packet_sizes() {
        assert_eq!(HEADER_SIZE + NUM_CARS * CAR_MOTION_SIZE, 1349);
        assert_eq!(HEADER_SIZE + NUM_CARS * LAP_DATA_SIZE + 2, 1285);
        assert_eq!(HEADER_SIZE + 1 + NUM_CARS * PARTICIPANT_SIZE, 1284);
        assert_eq!(HEADER_SIZE + NUM_CARS * CAR_SETUP_SIZE + 4, 1133);
        assert_eq!(HEADER_SIZE + NUM_CARS * CAR_TELEMETRY_SIZE + 3, 1352);
        assert_eq!(HEADER_SIZE + NUM_CARS * CAR_STATUS_SIZE, 1239);
        assert_eq!(HEADER_SIZE + NUM_CARS * CAR_DAMAGE_SIZE, 1041);
        assert_eq!(HEADER_SIZE + 7 + 100 * LAP_HISTORY_SIZE + 8 * 3, 1460);
    }

    #[test]
    fn parses_header() {
        let b = packet(6, 1352, 3);
        let h = parse_header(&b).unwrap();
        assert_eq!(h.packet_format, 2025);
        assert_eq!(h.packet_id, 6);
        assert_eq!(h.session_uid, 42);
        assert_eq!(h.player_car_index, 3);
    }

    #[test]
    fn parses_player_telemetry() {
        let mut b = packet(6, 1352, 3);
        let base = HEADER_SIZE + 3 * CAR_TELEMETRY_SIZE;
        b[base..base + 2].copy_from_slice(&312u16.to_le_bytes());
        b[base + 2..base + 6].copy_from_slice(&0.75f32.to_le_bytes());
        b[base + 15] = 7; // gear
        b[base + 16..base + 18].copy_from_slice(&11800u16.to_le_bytes());
        b[base + 18] = 1; // DRS
        b[base + 20..base + 22].copy_from_slice(&0x7FFFu16.to_le_bytes());
        b[base + 22..base + 24].copy_from_slice(&850u16.to_le_bytes()); // RL brake temp
        b[base + 36] = 101; // FL inner temp (inner temps start at +34)
        b[base + 38..base + 40].copy_from_slice(&105u16.to_le_bytes());
        b[base + 52..base + 56].copy_from_slice(&23.5f32.to_le_bytes()); // FR pressure
        b[1351] = 8; // suggested gear

        let mut st = F1State::default();
        st.handle_packet(&b);
        let t = st.telemetry[3];
        assert_eq!(st.player, 3);
        assert_eq!(t.speed_kph, 312);
        assert!((t.throttle - 0.75).abs() < 1e-6);
        assert_eq!(t.gear, 7);
        assert_eq!(t.rpm, 11800);
        assert_eq!(t.drs, 1);
        assert_eq!(t.rev_bits, 0x7FFF);
        assert_eq!(t.brake_temp[RL], 850);
        assert_eq!(t.tyre_inner_temp[FL], 101);
        assert_eq!(t.engine_temp, 105);
        assert!((t.tyre_pressure[FR] - 23.5).abs() < 1e-6);
        assert_eq!(st.suggested_gear, 8);
    }

    #[test]
    fn parses_lap_data_and_gaps() {
        let mut b = packet(2, 1285, 0);
        let base = HEADER_SIZE;
        b[base..base + 4].copy_from_slice(&91_234u32.to_le_bytes());
        b[base + 4..base + 8].copy_from_slice(&45_000u32.to_le_bytes());
        b[base + 14..base + 16].copy_from_slice(&512u16.to_le_bytes()); // delta front ms
        b[base + 16] = 1; // delta front minutes
        b[base + 20..base + 24].copy_from_slice(&1234.5f32.to_le_bytes());
        b[base + 32] = 4; // position
        b[base + 33] = 12; // lap number
        b[base + 36] = 1; // sector
        b[base + 37] = 1; // invalid
        b[base + 45] = 2; // result status (active)
        let mut st = F1State::default();
        st.handle_packet(&b);
        let l = st.laps[0];
        assert_eq!(l.last_lap_ms, 91_234);
        assert_eq!(l.current_lap_ms, 45_000);
        assert_eq!(l.delta_front_ms, 60_512);
        assert!((l.lap_distance - 1234.5).abs() < 1e-3);
        assert_eq!(l.position, 4);
        assert_eq!(l.lap_num, 12);
        assert_eq!(l.sector, 1);
        assert!(l.lap_invalid);
        assert_eq!(l.result_status, 2);
    }

    #[test]
    fn parses_status_and_damage() {
        let mut b = packet(7, 1239, 1);
        let base = HEADER_SIZE + CAR_STATUS_SIZE;
        b[base + 3] = 56; // brake bias
        b[base + 5..base + 9].copy_from_slice(&42.0f32.to_le_bytes());
        b[base + 13..base + 17].copy_from_slice(&0.4f32.to_le_bytes());
        b[base + 26] = 17; // visual compound
        b[base + 27] = 9; // tyre age
        b[base + 28] = 3; // yellow flag
        b[base + 37..base + 41].copy_from_slice(&3_000_000f32.to_le_bytes());
        b[base + 41] = 2; // hotlap
        let mut st = F1State::default();
        st.handle_packet(&b);
        let s = st.status[1];
        assert_eq!(s.brake_bias, 56);
        assert!((s.fuel_in_tank - 42.0).abs() < 1e-6);
        assert!((s.fuel_remaining_laps - 0.4).abs() < 1e-6);
        assert_eq!(s.visual_compound, 17);
        assert_eq!(s.tyre_age_laps, 9);
        assert_eq!(s.fia_flag, 3);
        assert!((s.ers_store - 3_000_000.0).abs() < 1.0);
        assert_eq!(s.ers_deploy_mode, 2);

        let mut d = packet(10, 1041, 1);
        let base = HEADER_SIZE + CAR_DAMAGE_SIZE;
        d[base + 8..base + 12].copy_from_slice(&33.0f32.to_le_bytes()); // FL wear
        d[base + 28] = 15; // FL wing
        d[base + 31] = 4; // floor
        d[base + 45] = 1; // engine seized
        st.handle_packet(&d);
        let dm = st.damage[1];
        assert!((dm.tyre_wear[FL] - 33.0).abs() < 1e-6);
        assert_eq!(dm.fl_wing, 15);
        assert_eq!(dm.floor, 4);
        assert_eq!(dm.engine_seized, 1);
    }

    #[test]
    fn parses_session_units_and_pit_window() {
        let mut b = packet(1, 753, 0);
        b[HEADER_SIZE + 6] = 15; // session type: race
        b[HEADER_SIZE + 7] = 7; // Silverstone
        b[682] = 18; // pit ideal lap
        b[683] = 22; // pit latest lap
        b[684] = 9; // rejoin position
        b[701] = 1; // KPH
        b[702] = 0; // Celsius
        let mut st = F1State::default();
        st.handle_packet(&b);
        assert!(st.session.is_race());
        assert_eq!(st.session.track_id, 7);
        assert_eq!(st.session.pit_ideal_lap, 18);
        assert_eq!(st.session.pit_latest_lap, 22);
        assert_eq!(st.session.pit_rejoin_pos, 9);
        assert_eq!(st.session.speed_units, 1);
    }

    #[test]
    fn parses_participant_name() {
        let mut b = packet(4, 1284, 0);
        b[HEADER_SIZE] = 20;
        let base = HEADER_SIZE + 1 + 2 * PARTICIPANT_SIZE;
        b[base + 3] = 8; // McLaren
        b[base + 7..base + 13].copy_from_slice(b"NORRIS");
        let mut st = F1State::default();
        st.handle_packet(&b);
        assert_eq!(st.num_active, 20);
        assert_eq!(st.participants[2].team_id, 8);
        assert_eq!(st.name(2), "NORRIS");
    }

    #[test]
    fn rejects_other_formats() {
        let mut b = packet(6, 1352, 0);
        b[0..2].copy_from_slice(&2024u16.to_le_bytes());
        let mut st = F1State::default();
        st.handle_packet(&b);
        assert_eq!(st.wrong_format, Some(2024));
    }

    #[test]
    fn delta_tracker_compares_against_best_lap() {
        let mut d = DeltaTracker::default();
        // Lap 1: 100 s over 1000 m.
        for step in 0..=500 {
            let dist = step as f32 * 2.0;
            d.update(1, dist, step * 200, false, 0);
        }
        // Cross the line: lap 2 starts, last lap = 100 s.
        d.update(2, 0.0, 0, false, 100_000);
        assert!(d.has_reference());
        // Halfway through lap 2, 1 s slower than lap 1 was at the same distance.
        d.update(2, 500.0, 51_000, false, 100_000);
        let delta = d.delta.unwrap();
        assert!((delta - 1.0).abs() < 0.05, "delta was {delta}");
    }
}

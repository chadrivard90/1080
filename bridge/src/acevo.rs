//! Assetto Corsa EVO shared memory: Local\acevo_pmf_physics / graphics / static.
//!
//! Layouts mirror the community reference (kerb crate `ac_evo/structs.rs`, cross-checked with
//! live-telemetry-evo's SHARED_MEMORY.md). The structs below are only used to compute field
//! offsets with `offset_of!`; values are read from a bounds-checked byte snapshot. `bool` fields
//! are mirrored as `u8`, which has the same size and alignment.

#![allow(dead_code)]

use std::mem::{offset_of, size_of};

use crate::packet::Packet;
use crate::shm::{self, Mapping};

#[repr(C)]
struct Physics {
    packet_id: i32,
    gas: f32,
    brake: f32,
    fuel: f32,
    gear: i32,
    rpms: i32,
    steer_angle: f32,
    speed_kmh: f32,
    velocity: [f32; 3],
    acc_g: [f32; 3],
    wheel_slip: [f32; 4],
    wheel_load: [f32; 4],
    wheels_pressure: [f32; 4],
    wheel_angular_speed: [f32; 4],
    tyre_wear: [f32; 4],
    tyre_dirty_level: [f32; 4],
    tyre_core_temperature: [f32; 4],
    camber_rad: [f32; 4],
    suspension_travel: [f32; 4],
    drs: f32,
    tc: f32,
    heading: f32,
    pitch: f32,
    roll: f32,
    cg_height: f32,
    car_damage: [f32; 5],
    number_of_tyres_out: i32,
    pit_limiter_on: i32,
    abs: f32,
    kers_charge: f32,
    kers_input: f32,
    auto_shifter_on: i32,
    ride_height: [f32; 2],
    turbo_boost: f32,
    ballast: f32,
    air_density: f32,
    air_temp: f32,
    road_temp: f32,
    local_angular_vel: [f32; 3],
    final_ff: f32,
    performance_meter: f32,
    engine_brake: i32,
    ers_recovery_level: i32,
    ers_power_level: i32,
    ers_heat_charging: i32,
    ers_is_charging: i32,
    kers_current_kj: f32,
    drs_available: i32,
    drs_enabled: i32,
    brake_temp: [f32; 4],
    clutch: f32,
    tyre_temp_i: [f32; 4],
    tyre_temp_m: [f32; 4],
    tyre_temp_o: [f32; 4],
    is_ai_controlled: i32,
    tyre_contact_point: [[f32; 3]; 4],
    tyre_contact_normal: [[f32; 3]; 4],
    tyre_contact_heading: [[f32; 3]; 4],
    brake_bias: f32,
    local_velocity: [f32; 3],
    p2p_activations: i32,
    p2p_status: i32,
    current_max_rpm: i32,
    mz: [f32; 4],
    fx: [f32; 4],
    fy: [f32; 4],
    slip_ratio: [f32; 4],
    slip_angle: [f32; 4],
    tcin_action: i32,
    abs_in_action: i32,
    suspension_damage: [f32; 4],
    tyre_temp: [f32; 4],
    water_temp: f32,
    brake_torque: [f32; 4],
    front_brake_compound: i32,
    rear_brake_compound: i32,
    pad_life: [f32; 4],
    disc_life: [f32; 4],
    ignition_on: i32,
    starter_engine_on: i32,
    is_engine_running: i32,
    kerb_vibration: f32,
    slip_vibrations: f32,
    road_vibrations: f32,
    abs_vibrations: f32,
}

#[repr(C)]
struct TyreState {
    slip: f32,
    lock: u8,
    tyre_pressure: f32,
    tyre_temperature_c: f32,
    brake_temperature_c: f32,
    brake_pressure: f32,
    tyre_temperature_left: f32,
    tyre_temperature_center: f32,
    tyre_temperature_right: f32,
    tyre_compound_front: [u8; 33],
    tyre_compound_rear: [u8; 33],
    tyre_normalized_pressure: f32,
    tyre_normalized_temperature_left: f32,
    tyre_normalized_temperature_center: f32,
    tyre_normalized_temperature_right: f32,
    brake_normalized_temperature: f32,
    tyre_normalized_temperature_core: f32,
    _pad: [u8; 128],
}

#[repr(C)]
struct DamageState {
    v: [f32; 9],
    _pad: [u8; 92],
}

#[repr(C)]
struct PitInfo {
    v: [i8; 6],
    _pad: [u8; 58],
}

#[repr(C)]
struct Electronics {
    tc_level: i8,
    tc_cut_level: i8,
    abs_level: i8,
    esc_level: i8,
    ebb_level: i8,
    brake_bias: f32,
    engine_map_level: i8,
    turbo_level: f32,
    ers_deployment_map: i8,
    ers_recharge_map: f32,
    is_ers_heat_charging_on: u8,
    is_ers_overtake_mode_on: u8,
    is_drs_open: u8,
    diff_power_level: i8,
    diff_coast_level: i8,
    front_bump_damper_level: i8,
    front_rebound_damper_level: i8,
    rear_bump_damper_level: i8,
    rear_rebound_damper_level: i8,
    is_ignition_on: u8,
    is_pitlimiter_on: u8,
    active_performance_mode: i8,
    _pad: [u8; 88],
}

#[repr(C)]
struct Instrumentation {
    v: [u8; 27],
    _pad: [u8; 101],
}

#[repr(C)]
struct SessionState {
    phase_name: [u8; 33],
    time_left: [u8; 15],
    time_left_ms: i32,
    wait_time: [u8; 15],
    total_lap: i32,
    current_lap: i32,
    lights_on: i32,
    lights_mode: i32,
    lap_length_km: f32,
    end_session_flag: i32,
    time_to_next_session: [u8; 15],
    flags: [u8; 6],
    _pad: [u8; 143],
}

#[repr(C)]
struct TimingState {
    current_laptime: [u8; 15],
    delta_current: [u8; 15],
    delta_current_p: i32,
    last_laptime: [u8; 15],
    delta_last: [u8; 15],
    delta_last_p: i32,
    best_laptime: [u8; 15],
    ideal_laptime: [u8; 15],
    total_time: [u8; 15],
    is_invalid: u8,
    _pad: [u8; 138],
}

#[repr(C)]
struct AssistsState {
    v: [u8; 7],
    auto_steer: f32,
    arcade_stability_control: f32,
    _pad: [u8; 48],
}

#[repr(C)]
struct Graphics {
    packet_id: i32,
    status: i32,
    focused_car_id_a: u64,
    focused_car_id_b: u64,
    player_car_id_a: u64,
    player_car_id_b: u64,
    rpm: u16,
    flags1: [u8; 15],
    display_speed_kmh: i16,
    display_speed_mph: i16,
    display_speed_ms: i16,
    pitspeeding_delta: f32,
    gear_int: i16,
    rpm_percent: f32,
    gas_percent: f32,
    brake_percent: f32,
    handbrake_percent: f32,
    clutch_percent: f32,
    steering_percent: f32,
    ffb_strength: f32,
    car_ffb_multiplier: f32,
    water_temperature_percent: f32,
    water_pressure_bar: f32,
    fuel_pressure_bar: f32,
    water_temperature_c: i8,
    air_temperature_c: i8,
    oil_temperature_c: f32,
    oil_pressure_bar: f32,
    exhaust_temperature_c: f32,
    g_forces_x: f32,
    g_forces_y: f32,
    g_forces_z: f32,
    turbo_boost: f32,
    turbo_boost_level: f32,
    turbo_boost_perc: f32,
    steer_degrees: i32,
    current_km: f32,
    total_km: u32,
    total_driving_time_s: u32,
    time_of_day_hours: i32,
    time_of_day_minutes: i32,
    time_of_day_seconds: i32,
    delta_time_ms: i32,
    current_lap_time_ms: i32,
    predicted_lap_time_ms: i32,
    fuel_liter_current_quantity: f32,
    fuel_liter_current_quantity_percent: f32,
    fuel_liter_per_km: f32,
    km_per_fuel_liter: f32,
    current_torque: f32,
    current_bhp: i32,
    tyre_lf: TyreState,
    tyre_rf: TyreState,
    tyre_lr: TyreState,
    tyre_rr: TyreState,
    npos: f32,
    kers_charge_perc: f32,
    kers_current_perc: f32,
    control_lock_time: f32,
    car_damage: DamageState,
    car_location: i32,
    pit_info: PitInfo,
    fuel_liter_used: f32,
    fuel_liter_per_lap: f32,
    laps_possible_with_fuel: f32,
    battery_temperature: f32,
    battery_voltage: f32,
    instantaneous_fuel_liter_per_km: f32,
    instantaneous_km_per_fuel_liter: f32,
    gear_rpm_window: f32,
    instrumentation: Instrumentation,
    instrumentation_min_limit: Instrumentation,
    instrumentation_max_limit: Instrumentation,
    electronics: Electronics,
    electronics_min_limit: Electronics,
    electronics_max_limit: Electronics,
    electronics_is_modifiable: Electronics,
    total_lap_count: i32,
    current_pos: u32,
    total_drivers: u32,
    last_laptime_ms: i32,
    best_laptime_ms: i32,
    flag: i32,
    global_flag: i32,
    max_gears: u32,
    engine_type: i32,
    has_kers: u8,
    is_last_lap: u8,
    performance_mode_name: [u8; 33],
    diff_coast_raw_value: f32,
    diff_power_raw_value: f32,
    race_cut_gained_time_ms: i32,
    distance_to_deadline: i32,
    race_cut_current_delta: f32,
    session_state: SessionState,
    timing_state: TimingState,
    player_ping: i32,
    player_latency: i32,
    player_cpu_usage: i32,
    player_cpu_usage_avg: i32,
    player_qos: i32,
    player_qos_avg: i32,
    player_fps: i32,
    player_fps_avg: i32,
    driver_name: [u8; 33],
    driver_surname: [u8; 33],
    car_model_name: [u8; 33],
    is_in_pit_box: u8,
    is_in_pit_lane: u8,
    is_valid_lap: u8,
    car_coordinates: [[f32; 3]; 60],
    gap_ahead: f32,
    gap_behind: f32,
    active_cars: u8,
    fuel_per_lap: f32,
    fuel_estimated_laps: f32,
    assists_state: AssistsState,
    max_fuel: f32,
    max_turbo_boost: f32,
    use_single_compound: u8,
    car_ids: [[u64; 2]; 60],
}

#[repr(C)]
struct Static {
    sm_version: [u8; 15],
    ac_evo_version: [u8; 15],
    session: i32,
    session_name: [u8; 33],
    event_id: u8,
    session_id: u8,
    starting_grip: i32,
    starting_ambient_temperature_c: f32,
    starting_ground_temperature_c: f32,
    is_static_weather: u8,
    is_timed_race: u8,
    is_online: u8,
    number_of_sessions: i32,
    nation: [u8; 33],
    longitude: f32,
    latitude: f32,
    track: [u8; 33],
    track_configuration: [u8; 33],
    track_length_m: f32,
}

// Graphics flag bytes inside `flags1` (in order): is_rpm_limiter_on, is_change_up_rpm,
// is_change_down_rpm, tc_active, abs_active, ...
const TC_ACTIVE: usize = 3;
const ABS_ACTIVE: usize = 4;

const STATUS_LIVE: i32 = 2;

const FLAGS: [&str; 11] = [
    "", "white", "green", "red", "blue", "yellow", "black", "black", "checkered", "black", "yellow",
];

#[derive(Default)]
pub struct AcEvo {
    physics: Option<Mapping>,
    graphics: Option<Mapping>,
    statics: Option<Mapping>,
}

impl AcEvo {
    pub fn poll(&mut self) -> Option<String> {
        if self.physics.is_none() {
            self.physics = Mapping::open("Local\\acevo_pmf_physics");
            self.graphics = Mapping::open("Local\\acevo_pmf_graphics");
            self.statics = Mapping::open("Local\\acevo_pmf_static");
            if self.physics.is_none() || self.graphics.is_none() {
                self.physics = None;
                self.graphics = None;
                return None;
            }
        }
        let p = self.physics.as_ref()?.read(0, size_of::<Physics>());
        let g = self.graphics.as_ref()?.read(0, size_of::<Graphics>() + 64);
        let s = self.statics.as_ref().map(|m| m.read(0, size_of::<Static>())).unwrap_or_default();
        if g.len() < offset_of!(Graphics, car_coordinates) {
            self.physics = None;
            return None;
        }
        let status = shm::i32_at(&g, offset_of!(Graphics, status));
        if status != STATUS_LIVE {
            // Menus / replay: if the game has exited, drop the mappings and re-probe.
            if status == 0 {
                self.physics = None;
                self.graphics = None;
                self.statics = None;
            }
            return None;
        }
        Some(build(&p, &g, &s))
    }
}

macro_rules! pf {
    ($b:expr, $field:ident) => {
        shm::f32_at($b, offset_of!(Physics, $field))
    };
    ($b:expr, $field:ident, $i:expr) => {
        shm::f32_at($b, offset_of!(Physics, $field) + 4 * $i)
    };
}
macro_rules! gf {
    ($b:expr, $field:ident) => {
        shm::f32_at($b, offset_of!(Graphics, $field))
    };
}
macro_rules! gi {
    ($b:expr, $field:ident) => {
        shm::i32_at($b, offset_of!(Graphics, $field)) as f64
    };
}

/// Gaps may be reported in ms or seconds depending on build; normalise to seconds.
fn gap_secs(v: f64) -> f64 {
    if v.abs() > 120.0 {
        v / 1000.0
    } else {
        v
    }
}

fn four(b: &[u8], base: usize) -> [f64; 4] {
    [0, 1, 2, 3].map(|i| shm::f32_at(b, base + 4 * i))
}

fn build(p: &[u8], g: &[u8], s: &[u8]) -> String {
    let mut out = Packet::new("acevo");
    // Physics gear: 0 = R, 1 = N, 2 = 1st.
    let gear = shm::i32_at(p, offset_of!(Physics, gear)) as f64 - 1.0;
    out.num("speed", pf!(p, speed_kmh) / 3.6)
        .num("gear", gear)
        .num("rpm", shm::i32_at(p, offset_of!(Physics, rpms)) as f64)
        .num("maxrpm", shm::i32_at(p, offset_of!(Physics, current_max_rpm)) as f64)
        .num("thr", pf!(p, gas))
        .num("brk", pf!(p, brake))
        .num("clu", pf!(p, clutch))
        .num("steer", gf!(g, steering_percent).clamp(-1.0, 1.0))
        .num("water", pf!(p, water_temp))
        .num("oilt", gf!(g, oil_temperature_c))
        .num("oilp", gf!(g, oil_pressure_bar))
        .num("boost", gf!(g, turbo_boost) * 14.5038)
        .num("tc_act", shm::i32_at(p, offset_of!(Physics, tcin_action)) as f64)
        .num("abs_act", shm::i32_at(p, offset_of!(Physics, abs_in_action)) as f64)
        .num("bias", pf!(p, brake_bias) * 100.0)
        .num("limiter", shm::i32_at(p, offset_of!(Physics, pit_limiter_on)) as f64)
        .num("glat", pf!(p, acc_g, 0))
        .num("glong", pf!(p, acc_g, 2))
        .num("yaw", pf!(p, heading))
        .num("yawrate", pf!(p, local_angular_vel, 1))
        .num("air", pf!(p, air_temp))
        .num("road", pf!(p, road_temp));
    // Graphics also reports TC/ABS activity as flags; use either source.
    let flags_base = offset_of!(Graphics, flags1);
    if shm::u8_at(g, flags_base + TC_ACTIVE) != 0 {
        out.num("tc_act", 1.0);
    }
    if shm::u8_at(g, flags_base + ABS_ACTIVE) != 0 {
        out.num("abs_act", 1.0);
    }
    let elec = offset_of!(Graphics, electronics);
    out.num("tc", shm::i8_at(g, elec + offset_of!(Electronics, tc_level)) as f64)
        .num("abs", shm::i8_at(g, elec + offset_of!(Electronics, abs_level)) as f64);

    // Tyres: physics arrays are FL, FR, RL, RR.
    out.list("tyre_s", &four(p, offset_of!(Physics, tyre_temp)))
        .list("tyre_c", &four(p, offset_of!(Physics, tyre_core_temperature)))
        .list("tyre_i", &four(p, offset_of!(Physics, tyre_temp_i)))
        .list("tyre_m", &four(p, offset_of!(Physics, tyre_temp_m)))
        .list("tyre_o", &four(p, offset_of!(Physics, tyre_temp_o)))
        .list("tyre_p", &four(p, offset_of!(Physics, wheels_pressure)))
        .list("brake_t", &four(p, offset_of!(Physics, brake_temp)));

    // Fuel / timing.
    out.num("fuel", gf!(g, fuel_liter_current_quantity))
        .num("fuelpct", gf!(g, fuel_liter_current_quantity_percent).clamp(0.0, 1.0))
        .num("fuellaps", gf!(g, fuel_estimated_laps))
        .num("pos", shm::u32_at(g, offset_of!(Graphics, current_pos)) as f64)
        .num("ncars", shm::u32_at(g, offset_of!(Graphics, total_drivers)) as f64)
        .num("cur", gi!(g, current_lap_time_ms) / 1000.0)
        .num("last", gi!(g, last_laptime_ms) / 1000.0)
        .num("best", gi!(g, best_laptime_ms) / 1000.0)
        .num("delta", gi!(g, delta_time_ms) / 1000.0)
        .num("invalid", if shm::u8_at(g, offset_of!(Graphics, is_valid_lap)) == 0 { 1.0 } else { 0.0 })
        .num("pit", shm::u8_at(g, offset_of!(Graphics, is_in_pit_lane)) as f64)
        .num("pct", gf!(g, npos))
        .num("gapa", gap_secs(gf!(g, gap_ahead)))
        .num("gapb", gap_secs(gf!(g, gap_behind)));
    let sess = offset_of!(Graphics, session_state);
    out.num("lap", shm::i32_at(g, sess + offset_of!(SessionState, current_lap)) as f64)
        .num("laps", shm::i32_at(g, sess + offset_of!(SessionState, total_lap)) as f64)
        .num("timeleft", shm::i32_at(g, sess + offset_of!(SessionState, time_left_ms)) as f64 / 1000.0);
    let flag = shm::i32_at(g, offset_of!(Graphics, flag));
    out.str("flag", FLAGS.get(flag.max(0) as usize).copied().unwrap_or(""));
    if !s.is_empty() {
        out.str("track", &shm::str_at(s, offset_of!(Static, track), 33))
            .str("session", &shm::str_at(s, offset_of!(Static, session_name), 33));
    }

    // Cars: world XZ for every active car; find the player by car id.
    let active = (shm::u8_at(g, offset_of!(Graphics, active_cars)) as usize).min(60);
    let coords = offset_of!(Graphics, car_coordinates);
    let (mut cx, mut cz) = (Vec::with_capacity(active), Vec::with_capacity(active));
    for i in 0..active {
        cx.push(shm::f32_at(g, coords + i * 12));
        cz.push(shm::f32_at(g, coords + i * 12 + 8));
    }
    let player = find_player(g, active);
    if let Some(i) = player {
        out.num("x", cx[i]).num("z", cz[i]);
    }
    out.list("cx", &cx).list("cz", &cz).num("player", player.map_or(-1.0, |i| i as f64));
    out.finish()
}

/// The player's index in `car_coordinates`, by matching the player car id against `car_ids`.
/// Tries the natural-alignment layout first and a 4-byte-packed variant second, since the two
/// community references disagree on the alignment of `car_ids`.
fn find_player(g: &[u8], active: usize) -> Option<usize> {
    let a = shm::u64_at(g, offset_of!(Graphics, player_car_id_a));
    let b = shm::u64_at(g, offset_of!(Graphics, player_car_id_b));
    if a == 0 && b == 0 {
        return None;
    }
    let natural = offset_of!(Graphics, car_ids);
    let packed4 = (offset_of!(Graphics, use_single_compound) + 1 + 3) & !3;
    for base in [natural, packed4] {
        for i in 0..active {
            let o = base + i * 16;
            if shm::u64_at(g, o) == a && shm::u64_at(g, o + 8) == b {
                return Some(i);
            }
        }
    }
    None
}

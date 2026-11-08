//! iRacing SDK shared memory (Local\IRSDKMemMapFileName), used by iRacing-engine titles
//! (NASCAR 25 / 26). Variables are looked up by name from the SDK's own variable table, so
//! anything a title doesn't publish is simply skipped.
//!
//! Header (112 bytes): ver, status, tickRate, sessionInfoUpdate, sessionInfoLen,
//! sessionInfoOffset, numVars, varHeaderOffset, numBuf, bufLen, pad[2], varBuf[4] {tickCount,
//! bufOffset, pad[2]}. Var header (144 bytes): type, offset, count, countAsTime(+pad),
//! name[32], desc[64], unit[32]. Types: 0 char, 1 bool, 2 int, 3 bitfield, 4 float, 5 double.

use std::collections::HashMap;

use crate::packet::Packet;
use crate::shm::{self, Mapping};

const STATUS_CONNECTED: i32 = 1;

#[derive(Clone, Copy)]
struct Var {
    kind: i32,
    offset: usize,
    count: usize,
}

#[derive(Default)]
pub struct IRacing {
    map: Option<Mapping>,
    vars: HashMap<String, Var>,
    vars_for: (i32, i32),
    session_update: i32,
    names: Vec<String>,
    numbers: Vec<String>,
    track: String,
    redline: f64,
}

impl IRacing {
    pub fn poll(&mut self) -> Option<String> {
        if self.map.is_none() {
            self.map = Mapping::open("Local\\IRSDKMemMapFileName");
        }
        let header = self.map.as_ref()?.read(0, 112);
        if header.len() < 112 {
            self.map = None;
            return None;
        }
        let status = shm::i32_at(&header, 4);
        if status & STATUS_CONNECTED == 0 {
            return None;
        }
        let num_vars = shm::i32_at(&header, 24);
        let var_header_offset = shm::i32_at(&header, 28);
        let num_buf = shm::i32_at(&header, 32).clamp(0, 4) as usize;
        let buf_len = shm::i32_at(&header, 36).max(0) as usize;

        if (num_vars, var_header_offset) != self.vars_for || self.vars.is_empty() {
            self.load_vars(num_vars, var_header_offset);
        }
        let session_update = shm::i32_at(&header, 12);
        if session_update != self.session_update {
            self.session_update = session_update;
            let len = shm::i32_at(&header, 16).max(0) as usize;
            let off = shm::i32_at(&header, 20).max(0) as usize;
            let yaml = self.map.as_ref()?.read(off, len);
            self.parse_session(&String::from_utf8_lossy(&yaml));
        }

        // Newest data buffer.
        let mut best: Option<(i32, usize)> = None;
        for i in 0..num_buf {
            let tick = shm::i32_at(&header, 48 + i * 16);
            let off = shm::i32_at(&header, 48 + i * 16 + 4).max(0) as usize;
            if best.map_or(true, |(t, _)| tick > t) {
                best = Some((tick, off));
            }
        }
        let (_, off) = best?;
        let buf = self.map.as_ref()?.read(off, buf_len);
        Some(self.build(&buf))
    }

    fn load_vars(&mut self, num_vars: i32, offset: i32) {
        self.vars.clear();
        self.vars_for = (num_vars, offset);
        let Some(map) = self.map.as_ref() else { return };
        let n = num_vars.clamp(0, 4096) as usize;
        let table = map.read(offset.max(0) as usize, n * 144);
        for i in 0..n {
            let o = i * 144;
            if o + 144 > table.len() {
                break;
            }
            let name = shm::str_at(&table, o + 16, 32);
            let var = Var {
                kind: shm::i32_at(&table, o),
                offset: shm::i32_at(&table, o + 4).max(0) as usize,
                count: shm::i32_at(&table, o + 8).max(1) as usize,
            };
            self.vars.insert(name, var);
        }
    }

    /// Minimal YAML scan for driver names / numbers and track name.
    fn parse_session(&mut self, yaml: &str) {
        self.names = vec![String::new(); 64];
        self.numbers = vec![String::new(); 64];
        let mut idx: Option<usize> = None;
        let mut in_drivers = false;
        for line in yaml.lines() {
            let t = line.trim();
            let value = |key: &str| t.strip_prefix(key).map(|v| v.trim().trim_matches('"').to_string());
            if let Some(v) = value("TrackDisplayName:") {
                self.track = v;
            }
            if let Some(v) = value("DriverCarRedLine:") {
                self.redline = v.parse().unwrap_or(0.0);
            }
            if t.starts_with("Drivers:") {
                in_drivers = true;
                continue;
            }
            if !in_drivers {
                continue;
            }
            if let Some(v) = t.strip_prefix("- CarIdx:").or_else(|| t.strip_prefix("CarIdx:")) {
                idx = v.trim().parse::<usize>().ok().filter(|i| *i < 64);
            } else if let (Some(i), Some(v)) = (idx, value("UserName:")) {
                self.names[i] = v;
            } else if let (Some(i), Some(v)) = (idx, value("CarNumber:")) {
                self.numbers[i] = v;
            } else if !line.starts_with(' ') && !t.is_empty() && !t.starts_with('-') {
                in_drivers = false; // next top-level section
            }
        }
    }

    fn get(&self, buf: &[u8], name: &str, i: usize) -> Option<f64> {
        let v = self.vars.get(name)?;
        if i >= v.count {
            return None;
        }
        let size = match v.kind {
            0 | 1 => 1,
            5 => 8,
            _ => 4,
        };
        let o = v.offset + i * size;
        Some(match v.kind {
            0 | 1 => shm::u8_at(buf, o) as f64,
            2 => shm::i32_at(buf, o) as f64,
            3 => shm::u32_at(buf, o) as f64,
            4 => shm::f32_at(buf, o),
            5 => shm::f64_at(buf, o),
            _ => return None,
        })
    }

    fn build(&self, b: &[u8]) -> String {
        let g = |n: &str| self.get(b, n, 0).unwrap_or(f64::NAN);
        let mut out = Packet::new("nascar");
        out.num("t", g("SessionTime"))
            .num("speed", g("Speed"))
            .num("gear", g("Gear"))
            .num("rpm", g("RPM"))
            .num("maxrpm", if self.redline > 0.0 { self.redline } else { f64::NAN })
            .num("shift", g("ShiftIndicatorPct"))
            .num("thr", g("Throttle"))
            .num("brk", g("Brake"))
            .num("clu", 1.0 - g("Clutch"))
            .num("steer", (g("SteeringWheelAngle") / g("SteeringWheelAngleMax").max(0.1)).clamp(-1.0, 1.0))
            .num("fuel", g("FuelLevel"))
            .num("fuelpct", g("FuelLevelPct"))
            .num("water", g("WaterTemp"))
            .num("oilt", g("OilTemp"))
            .num("oilp", g("OilPress"))
            .num("bias", g("dcBrakeBias"))
            .num("pit", g("OnPitRoad"))
            .num("incidents", g("PlayerCarMyIncidentCount"))
            .num("pos", g("PlayerCarPosition"))
            .num("lap", g("Lap"))
            .num("cur", g("LapCurrentLapTime"))
            .num("last", g("LapLastLapTime"))
            .num("best", g("LapBestLapTime"))
            .num("delta", if g("LapDeltaToBestLap_OK") > 0.5 { g("LapDeltaToBestLap") } else { f64::NAN })
            .num("timeleft", g("SessionTimeRemain"))
            .num("pct", g("LapDistPct"))
            .num("glat", g("LatAccel") / 9.81)
            .num("glong", g("LongAccel") / 9.81)
            .num("yaw", if g("YawNorth").is_finite() { g("YawNorth") } else { g("Yaw") })
            .num("yawrate", g("YawRate"))
            .num("air", g("AirTemp"))
            .num("road", g("TrackTempCrew"));
        let laps_left = g("SessionLapsRemainEx");
        let lap = g("Lap");
        if laps_left.is_finite() && laps_left > 0.0 && laps_left < 5000.0 && lap.is_finite() {
            out.num("laps", lap + laps_left);
        }
        if !self.track.is_empty() {
            out.str("track", &self.track);
        }
        out.str("session", "RACE");

        // Spotter.
        let (left, right) = match g("CarLeftRight") as i32 {
            2 => (1.0, 0.0),
            3 => (0.0, 1.0),
            4 => (1.0, 1.0),
            5 => (2.0, 0.0),
            6 => (0.0, 2.0),
            _ => (0.0, 0.0),
        };
        out.num("left", left).num("right", right);

        // Flags.
        let flags = g("SessionFlags");
        if flags.is_finite() {
            let f = flags as u64 as u32;
            let flag = if f & 0x0010 != 0 {
                "red"
            } else if f & (0x4000 | 0x8000) != 0 {
                "caution"
            } else if f & (0x0008 | 0x0100) != 0 {
                "yellow"
            } else if f & 0x0001_0000 != 0 {
                "black"
            } else if f & 0x0020 != 0 {
                "blue"
            } else if f & 0x0001 != 0 {
                "checkered"
            } else if f & 0x0002 != 0 {
                "white"
            } else {
                ""
            };
            out.str("flag", flag);
        }

        // Tyres: iRacing wheels are LF, RF, LR, RR = FL, FR, RL, RR. Temps are carcass L/M/R.
        let corners = ["LF", "RF", "LR", "RR"];
        let tyre = |c: &str, s: &str| g(&format!("{c}{s}"));
        let mid: Vec<f64> = corners.iter().map(|c| tyre(c, "tempCM")).collect();
        out.list("tyre_s", &mid)
            .list("tyre_i", &corners.iter().map(|c| tyre(c, "tempCL")).collect::<Vec<_>>())
            .list("tyre_m", &mid)
            .list("tyre_o", &corners.iter().map(|c| tyre(c, "tempCR")).collect::<Vec<_>>())
            .list(
                "tyre_w",
                &corners
                    .iter()
                    .map(|c| {
                        let avg = (tyre(c, "wearL") + tyre(c, "wearM") + tyre(c, "wearR")) / 3.0;
                        (1.0 - avg) * 100.0
                    })
                    .collect::<Vec<_>>(),
            )
            .list("tyre_p", &corners.iter().map(|c| tyre(c, "coldPressure") * 0.145_038).collect::<Vec<_>>());

        // Field: every car index with its lap progress.
        let player = g("PlayerCarIdx");
        let mut pct = Vec::with_capacity(64);
        let mut pos = Vec::with_capacity(64);
        let mut lap = Vec::with_capacity(64);
        let mut pit = Vec::with_capacity(64);
        let mut cars = 0;
        for i in 0..64 {
            let p = self.get(b, "CarIdxLapDistPct", i).unwrap_or(-1.0);
            if p >= 0.0 {
                cars += 1;
                pct.push(p);
            } else {
                pct.push(f64::NAN);
            }
            pos.push(self.get(b, "CarIdxPosition", i).unwrap_or(f64::NAN));
            lap.push(self.get(b, "CarIdxLap", i).unwrap_or(f64::NAN));
            pit.push(self.get(b, "CarIdxOnPitRoad", i).unwrap_or(0.0));
        }
        out.num("ncars", cars as f64)
            .list("cpct", &pct)
            .list("cpos", &pos)
            .list("clap", &lap)
            .list("cpit", &pit)
            .names("cname", &self.names)
            .names("cnum", &self.numbers)
            .num("player", player);
        out.finish()
    }
}

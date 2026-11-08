//! Lap log: every completed lap is appended to a CSV file, and the fastest valid lap per
//! game + track + car is kept as an all-time personal best (read back from the CSV at start-up).
//!
//! File: $RACE_DASH_LOGS/laps.csv, default ~/race-dash-logs/laps.csv (outside the install
//! folder, so updates never touch it). Columns:
//!   utc_time,game,track,car,session,lap,lap_time,seconds,valid,position,fuel_used,personal_best

use std::collections::HashMap;
use std::fmt::Write as _;
use std::fs::OpenOptions;
use std::io::Write as _;
use std::path::PathBuf;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const HEADER: &str = "utc_time,game,track,car,session,lap,lap_time,seconds,valid,position,fuel_used,personal_best";

pub fn log_dir() -> Option<PathBuf> {
    if let Some(p) = std::env::var_os("RACE_DASH_LOGS") {
        return Some(PathBuf::from(p));
    }
    let home = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE"))?;
    Some(PathBuf::from(home).join("race-dash-logs"))
}

pub fn log_path() -> Option<PathBuf> {
    log_dir().map(|d| d.join("laps.csv"))
}

/// One game + track + car combination seen in the log.
#[derive(Clone, Debug, Default)]
pub struct Best {
    pub game: String,
    pub track: String,
    pub car: String,
    /// Fastest valid lap, seconds.
    pub best_s: Option<f32>,
    pub laps: u32,
}

/// What the dash reports each tick.
pub struct LapInput<'a> {
    pub game: &'a str,
    pub track: &'a str,
    pub car: &'a str,
    pub session: &'a str,
    pub lap: i32,
    pub last_lap_s: Option<f32>,
    pub invalid_now: bool,
    pub position: u32,
    pub fuel: Option<f32>,
}

/// A lap that has just been written to the log.
pub struct LapDone {
    pub time_s: f32,
    pub valid: bool,
    /// Previous all-time best for this combination, if any.
    pub previous_best: Option<f32>,
    pub new_best: bool,
}

struct Pending {
    lap: i32,
    last_before: Option<f32>,
    invalid: bool,
    fuel_start: Option<f32>,
    fuel_end: Option<f32>,
    since: Instant,
}

pub struct LapLog {
    bests: HashMap<String, Best>,
    cur_lap: Option<i32>,
    invalid: bool,
    fuel_at_start: Option<f32>,
    last_seen_last_lap: Option<f32>,
    pending: Option<Pending>,
}

fn clean(s: &str) -> String {
    let t: String = s.chars().map(|c| if c == ',' || c == '\n' || c == '\r' || c == '"' { ' ' } else { c }).collect();
    t.trim().to_uppercase()
}

fn key(game: &str, track: &str, car: &str) -> String {
    format!("{}|{}|{}", clean(game), clean(track), clean(car))
}

fn track_or_unknown(track: &str) -> String {
    let t = clean(track);
    if t.is_empty() { "UNKNOWN TRACK".into() } else { t }
}

pub fn fmt_lap(secs: f32) -> String {
    if !secs.is_finite() || secs <= 0.0 {
        return "NO TIME".into();
    }
    let ms = (secs * 1000.0).round() as u64;
    format!("{}:{:02}.{:03}", ms / 60_000, (ms % 60_000) / 1000, ms % 1000)
}

/// "+0.412" / "-0.120": a lap time compared with a reference.
pub fn fmt_diff(secs: f32) -> String {
    format!("{:+.3}", secs.clamp(-999.0, 999.0))
}

/// UTC timestamp like 2026-11-08T14:03:22Z (no date crate needed).
fn utc_now() -> String {
    let secs = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0) as i64;
    let days = secs.div_euclid(86_400);
    let rem = secs.rem_euclid(86_400);
    // Civil-from-days (Howard Hinnant).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + if m <= 2 { 1 } else { 0 };
    format!("{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z", rem / 3600, (rem % 3600) / 60, rem % 60)
}

impl LapLog {
    /// Read the existing log (if any) to learn the all-time bests.
    pub fn open() -> Self {
        let mut log = LapLog {
            bests: HashMap::new(),
            cur_lap: None,
            invalid: false,
            fuel_at_start: None,
            last_seen_last_lap: None,
            pending: None,
        };
        if let Some(path) = log_path() {
            if let Ok(text) = std::fs::read_to_string(path) {
                for line in text.lines().skip(1) {
                    let f: Vec<&str> = line.split(',').collect();
                    if f.len() < 9 {
                        continue;
                    }
                    let secs = f[7].trim().parse::<f32>().ok().filter(|s| s.is_finite() && *s > 0.0);
                    let valid = f[8].trim() == "yes";
                    log.note(f[1], f[2], f[3], secs, valid);
                }
            }
        }
        log
    }

    fn note(&mut self, game: &str, track: &str, car: &str, secs: Option<f32>, valid: bool) {
        let entry = self.bests.entry(key(game, track, car)).or_insert_with(|| Best {
            game: clean(game),
            track: clean(track),
            car: clean(car),
            best_s: None,
            laps: 0,
        });
        entry.laps += 1;
        if let (Some(s), true) = (secs, valid) {
            if entry.best_s.map_or(true, |b| s < b) {
                entry.best_s = Some(s);
            }
        }
    }

    /// All-time best for this combination.
    pub fn best(&self, game: &str, track: &str, car: &str) -> Option<f32> {
        self.bests.get(&key(game, &track_or_unknown(track), car)).and_then(|b| b.best_s)
    }

    /// Every combination logged for one game, fastest-known first.
    pub fn bests_for(&self, game: &str) -> Vec<Best> {
        let g = clean(game);
        let mut v: Vec<Best> = self.bests.values().filter(|b| b.game == g).cloned().collect();
        v.sort_by(|a, b| a.track.cmp(&b.track).then(a.car.cmp(&b.car)));
        v
    }

    pub fn total_laps(&self) -> u32 {
        self.bests.values().map(|b| b.laps).sum()
    }

    /// Call every UI tick. Returns the lap that was just logged, if one completed.
    pub fn observe(&mut self, i: &LapInput) -> Option<LapDone> {
        let mut done = None;
        match self.cur_lap {
            None => {
                self.cur_lap = Some(i.lap);
                self.fuel_at_start = i.fuel;
            }
            Some(cur) if i.lap == cur + 1 => {
                // Lap completed. Some games update the last-lap time a packet later, so wait
                // (up to 2 s) for it to change.
                self.pending = Some(Pending {
                    lap: cur,
                    last_before: self.last_seen_last_lap,
                    invalid: self.invalid,
                    fuel_start: self.fuel_at_start,
                    fuel_end: i.fuel,
                    since: Instant::now(),
                });
                self.cur_lap = Some(i.lap);
                self.invalid = false;
                self.fuel_at_start = i.fuel;
            }
            Some(cur) if i.lap != cur => {
                // Restart / new session / jumped laps: start over without logging.
                self.cur_lap = Some(i.lap);
                self.invalid = false;
                self.fuel_at_start = i.fuel;
                self.pending = None;
            }
            _ => {
                if i.invalid_now {
                    self.invalid = true;
                }
            }
        }

        if let Some(p) = &self.pending {
            let changed = i.last_lap_s.is_some() && i.last_lap_s != p.last_before;
            let timed_out = p.since.elapsed() > Duration::from_secs(2);
            if changed || timed_out {
                let p = self.pending.take().expect("pending lap");
                if let Some(t) = i.last_lap_s.filter(|t| t.is_finite() && *t > 5.0 && *t < 3600.0) {
                    done = Some(self.write(i, p.lap, t, !p.invalid, p.fuel_start, p.fuel_end));
                }
            }
        }
        self.last_seen_last_lap = i.last_lap_s;
        done
    }

    fn write(&mut self, i: &LapInput, lap: i32, t: f32, valid: bool, fuel_start: Option<f32>, fuel_end: Option<f32>) -> LapDone {
        let track = track_or_unknown(i.track);
        let previous_best = self.best(i.game, &track, i.car);
        let new_best = valid && previous_best.map_or(true, |b| t < b - 0.0005);
        self.note(i.game, &track, i.car, Some(t), valid);
        let fuel_used = match (fuel_start, fuel_end) {
            (Some(a), Some(b)) if a > b => format!("{:.2}", a - b),
            _ => String::new(),
        };
        let row = format!(
            "{},{},{},{},{},{},{},{:.3},{},{},{},{}\n",
            utc_now(),
            clean(i.game),
            track,
            clean(i.car),
            clean(i.session),
            lap,
            fmt_lap(t),
            t,
            if valid { "yes" } else { "no" },
            if i.position > 0 { i.position.to_string() } else { String::new() },
            fuel_used,
            if new_best { "yes" } else { "" },
        );
        if let Some(path) = log_path() {
            if let Some(dir) = path.parent() {
                let _ = std::fs::create_dir_all(dir);
            }
            let is_new = !path.exists();
            if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(&path) {
                if is_new {
                    let _ = writeln!(f, "{HEADER}");
                }
                if let Err(e) = f.write_all(row.as_bytes()) {
                    eprintln!("race-dash: could not write lap log: {e}");
                }
            }
        }
        LapDone { time_s: t, valid, previous_best, new_best }
    }
}

/// One lap read back from the CSV.
pub struct LoggedLap {
    pub utc: String,
    pub session: String,
    pub secs: f32,
    pub valid: bool,
}

/// Every logged lap for one game + track + car, oldest first.
pub fn laps_for(game: &str, track: &str, car: &str) -> Vec<LoggedLap> {
    let want = key(game, track, car);
    let Some(path) = log_path() else { return Vec::new() };
    let Ok(text) = std::fs::read_to_string(path) else { return Vec::new() };
    text.lines()
        .skip(1)
        .filter_map(|line| {
            let f: Vec<&str> = line.split(',').collect();
            if f.len() < 9 || key(f[1], f[2], f[3]) != want {
                return None;
            }
            let secs = f[7].trim().parse::<f32>().ok().filter(|s| s.is_finite() && *s > 0.0)?;
            Some(LoggedLap {
                utc: f[0].trim().to_string(),
                session: f[4].trim().to_string(),
                secs,
                valid: f[8].trim() == "yes",
            })
        })
        .collect()
}

/// "2026-11-08T14:03:22Z" -> "2026-11-08 14:03 UTC".
pub fn fmt_utc(utc: &str) -> String {
    if utc.len() >= 16 {
        format!("{} {} UTC", &utc[..10], &utc[11..16])
    } else {
        utc.to_string()
    }
}

/// A lap-time line chart in a 1000 x 300 view box: the fastest lap at the top, slow laps
/// lower down (pit / outlier laps are clamped to the bottom). Uses the last 20 laps.
pub struct Chart {
    pub path: String,
    pub best: f32,
    pub worst: f32,
    pub count: usize,
}

pub fn chart(times: &[f32]) -> Chart {
    let all: Vec<f32> = times.iter().copied().filter(|t| t.is_finite() && *t > 5.0).collect();
    let v: Vec<f32> = if all.len() > 20 { all[all.len() - 20..].to_vec() } else { all };
    let best = v.iter().copied().fold(f32::MAX, f32::min);
    let max = v.iter().copied().fold(f32::MIN, f32::max);
    if v.len() < 2 {
        let only = v.first().copied().unwrap_or(0.0);
        return Chart { path: String::new(), best: only, worst: only, count: v.len() };
    }
    let worst = max.min(best * 1.15).max(best + 0.5);
    let range = worst - best;
    let mut path = String::with_capacity(v.len() * 12);
    for (i, t) in v.iter().enumerate() {
        let x = i as f32 * 1000.0 / (v.len() - 1) as f32;
        let y = 20.0 + 260.0 * ((t.min(worst) - best) / range);
        let _ = write!(path, "{}{:.0} {:.0} ", if i == 0 { "M " } else { "L " }, x, y);
    }
    Chart { path, best, worst, count: v.len() }
}

/// Small trend line in a 300 x 60 view box (oldest left). The vertical scale spans at least
/// `min_span` so tiny wobbles stay flat.
pub fn spark<'a>(values: impl Iterator<Item = &'a f32> + Clone, min_span: f32) -> String {
    let v: Vec<f32> = values.filter(|x| x.is_finite()).copied().collect();
    if v.len() < 2 {
        return String::new();
    }
    let lo = v.iter().copied().fold(f32::MAX, f32::min);
    let hi = v.iter().copied().fold(f32::MIN, f32::max);
    let span = (hi - lo).max(min_span);
    let mid = (hi + lo) / 2.0;
    let bottom = mid - span / 2.0;
    let mut path = String::with_capacity(v.len() * 10);
    for (i, x) in v.iter().enumerate() {
        let px = i as f32 * 300.0 / (v.len() - 1) as f32;
        let py = 55.0 - 50.0 * ((x - bottom) / span);
        let _ = write!(path, "{}{:.0} {:.0} ", if i == 0 { "M " } else { "L " }, px, py);
    }
    path
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats() {
        assert_eq!(fmt_lap(83.456), "1:23.456");
        assert_eq!(fmt_diff(-0.12), "-0.120");
        assert_eq!(clean("Spa, Belgium"), "SPA  BELGIUM");
        assert_eq!(utc_now().len(), 20);
    }

    #[test]
    fn chart_puts_best_lap_on_top() {
        let c = chart(&[90.0, 88.0, 89.0]);
        assert_eq!(c.count, 3);
        assert!(c.path.starts_with("M 0 "));
        assert!(c.path.contains("500 20"));
        assert!(chart(&[90.0]).path.is_empty());
        assert_eq!(fmt_utc("2026-11-08T14:03:22Z"), "2026-11-08 14:03 UTC");
        let s = spark([80.0f32, 85.0, 90.0].iter(), 6.0);
        assert!(s.starts_with("M 0 55") && s.contains("300 5"));
    }

    #[test]
    fn bests_track_valid_laps_only() {
        let mut log = LapLog {
            bests: HashMap::new(),
            cur_lap: None,
            invalid: false,
            fuel_at_start: None,
            last_seen_last_lap: None,
            pending: None,
        };
        log.note("fm", "TRACK 3", "A 700", Some(90.0), true);
        log.note("fm", "TRACK 3", "A 700", Some(85.0), false);
        log.note("fm", "TRACK 3", "A 700", Some(88.0), true);
        assert_eq!(log.best("fm", "track 3", "a 700"), Some(88.0));
        assert_eq!(log.bests_for("fm")[0].laps, 3);
    }
}

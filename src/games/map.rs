//! Track maps shared by every game.
//!
//! No game sends a track outline, so we draw it ourselves from the player's own driving line.
//! - Circuit mode records about one lap, then freezes the outline.
//! - Stage mode (rally) keeps recording the whole stage.
//! - Open-world mode (Forza Horizon) keeps a rolling trail and a window that follows the car.
//!
//! Games that only report lap progress (0..1) for other cars, not coordinates (iRacing-based
//! NASCAR), get their cars placed by looking up that progress along the recorded line.

use std::collections::VecDeque;
use std::fmt::Write as _;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MapMode {
    Circuit,
    Stage,
    OpenWorld,
}

/// A car to draw. `x`/`z` are world metres, or None when only `pct` (lap fraction) is known.
#[derive(Clone, Debug, Default)]
pub struct MapCar {
    pub pos: Option<(f32, f32)>,
    pub pct: Option<f32>,
    pub player: bool,
    pub label: String,
    pub color: Option<u32>,
}

/// A dot in the 0..1000 map view box.
#[derive(Clone, Debug, PartialEq)]
pub struct Dot {
    pub x: f32,
    pub y: f32,
    pub player: bool,
    pub label: String,
    pub color: Option<u32>,
    /// 0 = car, 1 = trail point, 2 = start / finish marker, 3 = trail point coloured by delta
    /// (green where time was gained against the best lap, red where it was lost).
    pub kind: u8,
    /// Fade for trail points (1 = newest).
    pub alpha: f32,
}

/// Trail points kept behind the player, and their minimum spacing in metres.
const TRAIL_LEN: usize = 60;
const TRAIL_SPACING: f32 = 8.0;

impl Default for MapRecorder {
    fn default() -> Self {
        MapRecorder::new(MapMode::Circuit)
    }
}

pub struct MapRecorder {
    mode: MapMode,
    pts: Vec<(f32, f32)>,
    pcts: Vec<f32>,
    frozen: bool,
    pct_covered: f32,
    last_pct: Option<f32>,
    path_len: f32,
    /// The player's most recent positions (kept even after the outline is frozen).
    recent: VecDeque<(f32, f32, Option<f32>)>,
    /// Current delta to the best lap (seconds), set by the caller before each dd, so the
    /// trail can show where time is gained or lost.
    pub trail_delta: Option<f32>,
    /// Bumped whenever the outline changes, so the SVG path is only rebuilt when needed.
    pub version: u64,
}

const VIEW: f32 = 1000.0;
const MARGIN: f32 = 60.0;

impl MapRecorder {
    pub fn new(mode: MapMode) -> Self {
        Self {
            mode,
            pts: Vec::new(),
            pcts: Vec::new(),
            frozen: false,
            pct_covered: 0.0,
            last_pct: None,
            path_len: 0.0,
            recent: VecDeque::with_capacity(TRAIL_LEN + 1),
            trail_delta: None,
            version: 0,
        }
    }

    pub fn reset(&mut self) {
        *self = MapRecorder::new(self.mode);
        self.version = self.version.wrapping_add(1);
    }

    pub fn is_complete(&self) -> bool {
        self.frozen
    }

    pub fn has_outline(&self) -> bool {
        self.pts.len() > 10
    }

    /// Feed the player's position (world metres, x and z on the ground plane) and, if the game
    /// reports it, lap progress 0..1.
    pub fn add(&mut self, x: f32, z: f32, pct: Option<f32>) {
        if !x.is_finite() || !z.is_finite() || (x == 0.0 && z == 0.0) {
            return;
        }
        let far_enough = self
            .recent
            .back()
            .map_or(true, |&(lx, lz, _)| ((x - lx).powi(2) + (z - lz).powi(2)).sqrt() >= TRAIL_SPACING);
        if far_enough {
            self.recent.push_back((x, z, self.trail_delta));
            while self.recent.len() > TRAIL_LEN {
                self.recent.pop_front();
            }
        }
        if self.frozen {
            return;
        }
        let spacing = match self.mode {
            MapMode::Circuit => 5.0,
            MapMode::Stage => 8.0,
            MapMode::OpenWorld => 10.0,
        };
        if let Some(&(lx, lz)) = self.pts.last() {
            let d = ((x - lx).powi(2) + (z - lz).powi(2)).sqrt();
            if d < spacing {
                return;
            }
            // A big jump means a teleport / restart / rewind: start again.
            if d > 300.0 && self.mode != MapMode::OpenWorld {
                self.reset();
            } else if d > 1500.0 {
                self.reset();
            } else {
                self.path_len += d;
            }
        }
        self.pts.push((x, z));
        if let Some(p) = pct {
            self.pcts.push(p);
            if let Some(lp) = self.last_pct {
                let mut dp = p - lp;
                if dp < -0.5 {
                    dp += 1.0; // crossed the line
                }
                if dp > 0.0 && dp < 0.2 {
                    self.pct_covered += dp;
                }
            }
            self.last_pct = Some(p);
        }
        self.version = self.version.wrapping_add(1);

        match self.mode {
            MapMode::Circuit => {
                let closed_by_pct = self.pcts.len() == self.pts.len() && self.pct_covered >= 1.0;
                let (fx, fz) = self.pts[0];
                let closed_by_distance =
                    self.path_len > 600.0 && ((x - fx).powi(2) + (z - fz).powi(2)).sqrt() < 20.0;
                if closed_by_pct || closed_by_distance || self.pts.len() >= 4000 {
                    self.close_loop();
                    self.frozen = true;
                }
            }
            MapMode::Stage => {
                if self.pts.len() > 8000 {
                    self.frozen = true;
                }
            }
            MapMode::OpenWorld => {
                if self.pts.len() > 2000 {
                    self.pts.drain(0..200);
                    self.pcts.clear();
                }
            }
        }
    }

    /// Spread any end-to-start mismatch (dead-reckoning drift) along the lap so it closes neatly.
    fn close_loop(&mut self) {
        let n = self.pts.len();
        if n < 3 {
            return;
        }
        let (sx, sz) = self.pts[0];
        let (ex, ez) = self.pts[n - 1];
        let (dx, dz) = (ex - sx, ez - sz);
        if (dx * dx + dz * dz).sqrt() > 200.0 {
            return; // Too far off to be a closing error; leave it.
        }
        for (i, p) in self.pts.iter_mut().enumerate() {
            let f = i as f32 / (n - 1) as f32;
            p.0 -= dx * f;
            p.1 -= dz * f;
        }
    }

    /// World position on the recorded line for a lap fraction (needs recorded pcts).
    fn at_pct(&self, pct: f32) -> Option<(f32, f32)> {
        if self.pcts.len() != self.pts.len() || self.pcts.len() < 10 {
            return None;
        }
        // Pick the sample with the nearest pct (wrap aware). Linear scan is fine (<4000 points).
        let mut best = 0usize;
        let mut best_d = f32::MAX;
        for (i, &p) in self.pcts.iter().enumerate() {
            let mut d = (p - pct).abs();
            if d > 0.5 {
                d = 1.0 - d;
            }
            if d < best_d {
                best_d = d;
                best = i;
            }
        }
        if best_d > 0.05 {
            return None;
        }
        Some(self.pts[best])
    }

    /// Build the SVG path (view box 0..1000) and the car dots.
    /// `orient` 0..7: bits 0-1 rotate by 90 degrees, bit 2 mirrors.
    pub fn render(&self, cars: &[MapCar], orient: u8, player_pos: Option<(f32, f32)>) -> (String, Vec<Dot>) {
        let tf = |x: f32, z: f32| -> (f32, f32) {
            let (mut a, b) = (x, -z);
            if orient & 4 != 0 {
                a = -a;
            }
            match orient & 3 {
                0 => (a, b),
                1 => (-b, a),
                2 => (-a, -b),
                _ => (b, -a),
            }
        };

        // Bounds of the area to show.
        let (min_x, min_y, max_x, max_y) = if self.mode == MapMode::OpenWorld {
            let (px, pz) = player_pos.or_else(|| self.pts.last().copied()).unwrap_or((0.0, 0.0));
            let (cx, cy) = tf(px, pz);
            let half = 700.0;
            (cx - half, cy - half, cx + half, cy + half)
        } else {
            let mut b = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
            let mut grow = |p: (f32, f32)| {
                b.0 = b.0.min(p.0);
                b.1 = b.1.min(p.1);
                b.2 = b.2.max(p.0);
                b.3 = b.3.max(p.1);
            };
            for &(x, z) in &self.pts {
                grow(tf(x, z));
            }
            for c in cars {
                if let Some((x, z)) = c.pos {
                    grow(tf(x, z));
                }
            }
            if b.0 > b.2 {
                return (String::new(), Vec::new());
            }
            b
        };
        let span = (max_x - min_x).max(max_y - min_y).max(50.0);
        let scale = (VIEW - 2.0 * MARGIN) / span;
        let ox = MARGIN + ((VIEW - 2.0 * MARGIN) - (max_x - min_x) * scale) / 2.0;
        let oy = MARGIN + ((VIEW - 2.0 * MARGIN) - (max_y - min_y) * scale) / 2.0;
        let to_view = |x: f32, z: f32| -> (f32, f32) {
            let (a, b) = tf(x, z);
            (ox + (a - min_x) * scale, oy + (b - min_y) * scale)
        };

        let mut path = String::with_capacity(self.pts.len() * 10);
        let mut pen_down = false;
        for &(x, z) in &self.pts {
            let (vx, vy) = to_view(x, z);
            let inside = (-200.0..=1200.0).contains(&vx) && (-200.0..=1200.0).contains(&vy);
            if !inside {
                pen_down = false;
                continue;
            }
            let _ = write!(path, "{}{:.0} {:.0} ", if pen_down { "L " } else { "M " }, vx, vy);
            pen_down = true;
        }
        if self.frozen && self.mode == MapMode::Circuit && !path.is_empty() {
            path.push('Z');
        }

        let mut dots: Vec<Dot> = cars
            .iter()
            .filter_map(|c| {
                let world = c.pos.or_else(|| c.pct.and_then(|p| self.at_pct(p)))?;
                let (x, y) = to_view(world.0, world.1);
                if !(0.0..=VIEW).contains(&x) || !(0.0..=VIEW).contains(&y) {
                    return None;
                }
                Some(Dot { x, y, player: c.player, label: c.label.clone(), color: c.color, kind: 0, alpha: 1.0 })
            })
            .collect();
        // Start / finish marker at the first recorded point.
        if self.mode != MapMode::OpenWorld && self.has_outline() {
            let (x, y) = to_view(self.pts[0].0, self.pts[0].1);
            if (0.0..=VIEW).contains(&x) && (0.0..=VIEW).contains(&y) {
                dots.push(Dot { x, y, player: false, label: String::new(), color: None, kind: 2, alpha: 1.0 });
            }
        }
        // Fading trail behind the player (newest last; the newest sits under the player dot).
        let n = self.recent.len();
        let mut prev_delta: Option<f32> = None;
        for (i, &(wx, wz, delta)) in self.recent.iter().enumerate().take(n.saturating_sub(1)) {
            // Green where the delta fell (time gained) since the previous point, red where it rose.
            let color = match (prev_delta, delta) {
                (Some(p), Some(d)) if (d - p).abs() > 0.0005 => Some(if d < p { 0x22C55E } else { 0xFF3B3B }),
                _ => None,
            };
            prev_delta = delta;
            let (x, y) = to_view(wx, wz);
            if (0.0..=VIEW).contains(&x) && (0.0..=VIEW).contains(&y) {
                let alpha = 0.25 + 0.75 * (i + 1) as f32 / n as f32;
                let kind = if color.is_some() { 3 } else { 1 };
                dots.push(Dot { x, y, player: false, label: String::new(), color, kind, alpha });
            }
        }
        // Markers and trail first, then cars, with the player on top.
        dots.sort_by_key(|d| (d.kind == 0, d.player));
        (path, dots)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn circuit_closes_after_a_lap() {
        let mut m = MapRecorder::new(MapMode::Circuit);
        for i in 0..=400 {
            let a = i as f32 / 400.0 * std::f32::consts::TAU;
            m.add(300.0 * a.cos(), 200.0 * a.sin(), None);
        }
        assert!(m.is_complete());
        let (path, _) = m.render(&[], 0, None);
        assert!(path.starts_with("M "));
        assert!(path.ends_with('Z'));
    }

    #[test]
    fn places_cars_by_lap_fraction() {
        let mut m = MapRecorder::new(MapMode::Circuit);
        for i in 0..=400 {
            let f = i as f32 / 400.0;
            let a = f * std::f32::consts::TAU;
            m.add(300.0 * a.cos(), 300.0 * a.sin(), Some(f % 1.0));
        }
        let cars = vec![MapCar { pct: Some(0.25), label: "1".into(), ..Default::default() }];
        let (_, dots) = m.render(&cars, 0, None);
        assert_eq!(dots.iter().filter(|d| d.kind == 0).count(), 1);
        assert!(dots.iter().any(|d| d.kind == 2));
    }
}

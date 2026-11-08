//! SVG path strings for analog dials, all in a 0..200 view box centred on (100, 100).
//! Sweep: 270 degrees, starting bottom-left (135 deg, clockwise on screen) through the top.

use std::fmt::Write as _;

const CX: f32 = 100.0;
const CY: f32 = 100.0;
const R: f32 = 88.0;
const START_DEG: f32 = 135.0;
const SWEEP_DEG: f32 = 270.0;

fn point(frac: f32, r: f32) -> (f32, f32) {
    let a = (START_DEG + SWEEP_DEG * frac.clamp(0.0, 1.0)).to_radians();
    (CX + r * a.cos(), CY + r * a.sin())
}

/// Coloured value arc from 0 to `frac`.
pub fn arc(frac: f32) -> String {
    let frac = if frac.is_finite() { frac.clamp(0.0, 1.0) } else { 0.0 };
    if frac < 0.002 {
        return String::new();
    }
    let (x0, y0) = point(0.0, R);
    let (x1, y1) = point(frac, R);
    let large = if SWEEP_DEG * frac > 180.0 { 1 } else { 0 };
    format!("M {x0:.1} {y0:.1} A {R} {R} 0 {large} 1 {x1:.1} {y1:.1}")
}

/// Needle line from a short tail through the centre to near the rim.
pub fn needle(frac: f32) -> String {
    let frac = if frac.is_finite() { frac } else { 0.0 };
    let (tx, ty) = point(frac, R - 8.0);
    let a = (START_DEG + SWEEP_DEG * frac.clamp(0.0, 1.0)).to_radians();
    let (bx, by) = (CX - 14.0 * a.cos(), CY - 14.0 * a.sin());
    format!("M {bx:.1} {by:.1} L {tx:.1} {ty:.1}")
}

/// Tick marks: `majors` major divisions with 4 minor ticks between each.
pub fn ticks(majors: u32) -> String {
    let majors = majors.max(1);
    let steps = majors * 5;
    let mut s = String::new();
    for i in 0..=steps {
        let f = i as f32 / steps as f32;
        let inner = if i % 5 == 0 { R - 16.0 } else { R - 8.0 };
        let (x0, y0) = point(f, inner);
        let (x1, y1) = point(f, R);
        let _ = write!(s, "M {x0:.1} {y0:.1} L {x1:.1} {y1:.1} ");
    }
    s
}

/// Position (view box units) for a dial number at `frac` of the sweep.
pub fn label_pos(frac: f32) -> (f32, f32) {
    point(frac, R - 32.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arc_and_needle_are_well_formed() {
        assert!(arc(0.0).is_empty());
        assert!(arc(0.5).starts_with("M "));
        assert!(arc(0.9).contains(" 0 1 1 "));
        assert!(needle(0.3).contains(" L "));
        assert_eq!(ticks(8).matches('M').count(), 41);
        let top = point(0.5, R);
        assert!((top.0 - CX).abs() < 0.01 && (top.1 - (CY - R)).abs() < 0.01);
    }
}

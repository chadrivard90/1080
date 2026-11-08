//! Bounds-safe little-endian readers at fixed byte offsets. Out-of-range reads return 0, and
//! non-finite floats read as 0, so a malformed packet can never panic the listener.

fn arr<const N: usize>(b: &[u8], off: usize) -> [u8; N] {
    let mut out = [0u8; N];
    if let Some(s) = b.get(off..off + N) {
        out.copy_from_slice(s);
    }
    out
}

pub fn u8_at(b: &[u8], off: usize) -> u8 {
    b.get(off).copied().unwrap_or(0)
}
pub fn i8_at(b: &[u8], off: usize) -> i8 {
    u8_at(b, off) as i8
}
pub fn u16_at(b: &[u8], off: usize) -> u16 {
    u16::from_le_bytes(arr(b, off))
}
pub fn i16_at(b: &[u8], off: usize) -> i16 {
    i16::from_le_bytes(arr(b, off))
}
pub fn u32_at(b: &[u8], off: usize) -> u32 {
    u32::from_le_bytes(arr(b, off))
}
pub fn i32_at(b: &[u8], off: usize) -> i32 {
    i32::from_le_bytes(arr(b, off))
}
pub fn f32_at(b: &[u8], off: usize) -> f32 {
    let v = f32::from_le_bytes(arr(b, off));
    if v.is_finite() {
        v
    } else {
        0.0
    }
}
pub fn f64_at(b: &[u8], off: usize) -> f64 {
    let v = f64::from_le_bytes(arr(b, off));
    if v.is_finite() {
        v
    } else {
        0.0
    }
}
/// Null-terminated string of at most `len` bytes.
pub fn str_at(b: &[u8], off: usize, len: usize) -> String {
    let raw = b.get(off..(off + len).min(b.len())).unwrap_or(&[]);
    let end = raw.iter().position(|&c| c == 0).unwrap_or(raw.len());
    String::from_utf8_lossy(&raw[..end]).trim().to_string()
}

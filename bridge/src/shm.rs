//! Read-only access to a named Windows shared-memory mapping, without external crates.
//! Every read is bounds-checked against the mapped region size, so a layout mismatch can never
//! read outside the mapping.

use std::ffi::c_void;

const FILE_MAP_READ: u32 = 0x0004;

#[repr(C)]
struct MemoryBasicInformation {
    base_address: *mut c_void,
    allocation_base: *mut c_void,
    allocation_protect: u32,
    partition_id: u16,
    region_size: usize,
    state: u32,
    protect: u32,
    kind: u32,
}

#[link(name = "kernel32")]
extern "system" {
    fn OpenFileMappingW(desired_access: u32, inherit_handle: i32, name: *const u16) -> *mut c_void;
    fn MapViewOfFile(mapping: *mut c_void, access: u32, offset_high: u32, offset_low: u32, bytes: usize) -> *mut c_void;
    fn UnmapViewOfFile(base: *const c_void) -> i32;
    fn CloseHandle(handle: *mut c_void) -> i32;
    fn VirtualQuery(address: *const c_void, info: *mut MemoryBasicInformation, length: usize) -> usize;
}

pub struct Mapping {
    handle: *mut c_void,
    view: *const u8,
    size: usize,
}

impl Mapping {
    /// Open an existing mapping (never creates one, so a missing game is detected correctly).
    pub fn open(name: &str) -> Option<Mapping> {
        let wide: Vec<u16> = name.encode_utf16().chain(std::iter::once(0)).collect();
        // SAFETY: plain Win32 calls with a valid, null-terminated UTF-16 name.
        unsafe {
            let handle = OpenFileMappingW(FILE_MAP_READ, 0, wide.as_ptr());
            if handle.is_null() {
                return None;
            }
            let view = MapViewOfFile(handle, FILE_MAP_READ, 0, 0, 0) as *const u8;
            if view.is_null() {
                CloseHandle(handle);
                return None;
            }
            let mut info: MemoryBasicInformation = std::mem::zeroed();
            let ok = VirtualQuery(view as *const c_void, &mut info, std::mem::size_of::<MemoryBasicInformation>());
            let size = if ok == 0 { 0 } else { info.region_size };
            Some(Mapping { handle, view, size })
        }
    }

    /// Copy `len` bytes from `offset` (clamped to the mapped size).
    pub fn read(&self, offset: usize, len: usize) -> Vec<u8> {
        if offset >= self.size {
            return Vec::new();
        }
        let len = len.min(self.size - offset);
        // SAFETY: the range is inside the mapped view (checked against VirtualQuery's size).
        unsafe { std::slice::from_raw_parts(self.view.add(offset), len).to_vec() }
    }
}

impl Drop for Mapping {
    fn drop(&mut self) {
        // SAFETY: view and handle came from MapViewOfFile / OpenFileMappingW.
        unsafe {
            UnmapViewOfFile(self.view as *const c_void);
            CloseHandle(self.handle);
        }
    }
}

// Little-endian readers over a snapshot; out-of-range reads give 0.
pub fn u8_at(b: &[u8], o: usize) -> u8 {
    b.get(o).copied().unwrap_or(0)
}
fn arr<const N: usize>(b: &[u8], o: usize) -> [u8; N] {
    let mut out = [0u8; N];
    if let Some(s) = b.get(o..o + N) {
        out.copy_from_slice(s);
    }
    out
}
pub fn i8_at(b: &[u8], o: usize) -> i8 {
    u8_at(b, o) as i8
}
pub fn i16_at(b: &[u8], o: usize) -> i16 {
    i16::from_le_bytes(arr(b, o))
}
pub fn i32_at(b: &[u8], o: usize) -> i32 {
    i32::from_le_bytes(arr(b, o))
}
pub fn u32_at(b: &[u8], o: usize) -> u32 {
    u32::from_le_bytes(arr(b, o))
}
pub fn u64_at(b: &[u8], o: usize) -> u64 {
    u64::from_le_bytes(arr(b, o))
}
pub fn f32_at(b: &[u8], o: usize) -> f64 {
    f32::from_le_bytes(arr(b, o)) as f64
}
pub fn f64_at(b: &[u8], o: usize) -> f64 {
    f64::from_le_bytes(arr(b, o))
}
pub fn str_at(b: &[u8], o: usize, len: usize) -> String {
    let raw = b.get(o..(o + len).min(b.len())).unwrap_or(&[]);
    let end = raw.iter().position(|&c| c == 0).unwrap_or(raw.len());
    String::from_utf8_lossy(&raw[..end]).trim().to_string()
}

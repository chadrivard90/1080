//! Persisted settings: ~/.config/race-dash/config.txt, one `key=value` per line.
//!
//! Keys:
//!   demo=0|1
//!   layout_<game>=N      last selected tab per game (f1, nascar, acevo, ams2, dr2, wrc, fm, fh6)
//!   map_<game>=0..7      track map rotation / mirror per game
//!   port_<game>=N        UDP port override
//!   units_<game>=metric|imperial

use std::collections::BTreeMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Default)]
pub struct Config {
    values: BTreeMap<String, String>,
}

impl Config {
    fn path() -> Option<PathBuf> {
        let home = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE"))?;
        Some(PathBuf::from(home).join(".config").join("race-dash").join("config.txt"))
    }

    pub fn load() -> Self {
        let mut cfg = Config::default();
        let Some(path) = Self::path() else { return cfg };
        let Ok(text) = std::fs::read_to_string(path) else { return cfg };
        for line in text.lines() {
            if let Some((k, v)) = line.split_once('=') {
                cfg.values.insert(k.trim().to_string(), v.trim().to_string());
            }
        }
        // Migrate the old single-game key.
        if let Some(v) = cfg.values.remove("f1_layout") {
            cfg.values.entry("layout_f1".into()).or_insert(v);
        }
        if let Some(v) = cfg.values.remove("f1_port") {
            cfg.values.entry("port_f1".into()).or_insert(v);
        }
        cfg
    }

    pub fn save(&self) {
        let Some(path) = Self::path() else { return };
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let text: String = self.values.iter().map(|(k, v)| format!("{k}={v}\n")).collect();
        let _ = std::fs::write(path, text);
    }

    fn get_i(&self, key: &str, default: i64) -> i64 {
        self.values.get(key).and_then(|v| v.parse().ok()).unwrap_or(default)
    }

    fn set(&mut self, key: String, value: String) {
        self.values.insert(key, value);
    }

    pub fn demo(&self) -> bool {
        self.get_i("demo", 0) != 0
    }
    pub fn set_demo(&mut self, on: bool) {
        self.set("demo".into(), if on { "1" } else { "0" }.into());
    }

    pub fn layout(&self, game: &str) -> i32 {
        self.get_i(&format!("layout_{game}"), 0) as i32
    }
    pub fn set_layout(&mut self, game: &str, layout: i32) {
        self.set(format!("layout_{game}"), layout.to_string());
    }

    pub fn map_orient(&self, game: &str) -> u8 {
        (self.get_i(&format!("map_{game}"), 0) & 7) as u8
    }
    pub fn set_map_orient(&mut self, game: &str, orient: u8) {
        self.set(format!("map_{game}"), (orient & 7).to_string());
    }

    pub fn port(&self, game: &str, default: u16) -> u16 {
        let p = self.get_i(&format!("port_{game}"), default as i64);
        u16::try_from(p).unwrap_or(default)
    }

    pub fn imperial(&self, game: &str, default: bool) -> bool {
        match self.values.get(&format!("units_{game}")).map(String::as_str) {
            Some("imperial") => true,
            Some("metric") => false,
            _ => default,
        }
    }
}

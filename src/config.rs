//! Tiny persisted settings file: ~/.config/race-dash/config.txt (key=value lines).

use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Config {
    /// Feed the dashes with simulated data instead of listening for the game.
    pub demo: bool,
    /// Last selected F1 25 layout tab (0 = MFD).
    pub f1_layout: i32,
    /// UDP port F1 25 sends to (game default 20777).
    pub f1_port: u16,
}

impl Default for Config {
    fn default() -> Self {
        Self { demo: false, f1_layout: 0, f1_port: 20777 }
    }
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
            let Some((key, value)) = line.split_once('=') else { continue };
            let value = value.trim();
            match key.trim() {
                "demo" => cfg.demo = value == "1" || value == "true",
                "f1_layout" => cfg.f1_layout = value.parse().unwrap_or(0),
                "f1_port" => cfg.f1_port = value.parse().unwrap_or(20777),
                _ => {}
            }
        }
        cfg
    }

    pub fn save(&self) {
        let Some(path) = Self::path() else { return };
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let text = format!(
            "demo={}\nf1_layout={}\nf1_port={}\n",
            if self.demo { 1 } else { 0 },
            self.f1_layout,
            self.f1_port
        );
        let _ = std::fs::write(path, text);
    }
}

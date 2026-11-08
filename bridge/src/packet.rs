//! Builder for the Race Dash bridge text protocol ("RDB1" + key=value lines).
//! The receiver is src/games/bridge_rx.rs in the Race Dash app.

use std::fmt::Write as _;

pub struct Packet {
    text: String,
}

impl Packet {
    pub fn new(game: &str) -> Self {
        let mut text = String::with_capacity(2048);
        text.push_str("RDB1\n");
        let _ = writeln!(text, "game={game}");
        Self { text }
    }

    pub fn num(&mut self, key: &str, v: f64) -> &mut Self {
        if v.is_finite() {
            let _ = writeln!(self.text, "{key}={}", trim(v));
        }
        self
    }

    pub fn str(&mut self, key: &str, v: &str) -> &mut Self {
        let clean: String = v.chars().filter(|c| *c != '\n' && *c != '\r').collect();
        let _ = writeln!(self.text, "{key}={clean}");
        self
    }

    /// Comma-separated numbers; non-finite values are sent as empty fields.
    pub fn list(&mut self, key: &str, values: &[f64]) -> &mut Self {
        let _ = write!(self.text, "{key}=");
        for (i, v) in values.iter().enumerate() {
            if i > 0 {
                self.text.push(',');
            }
            if v.is_finite() {
                self.text.push_str(&trim(*v));
            }
        }
        self.text.push('\n');
        self
    }

    /// '|'-separated strings.
    pub fn names(&mut self, key: &str, values: &[String]) -> &mut Self {
        let joined: Vec<String> = values.iter().map(|s| s.replace(['|', '\n', '\r'], " ")).collect();
        let _ = writeln!(self.text, "{key}={}", joined.join("|"));
        self
    }

    pub fn finish(self) -> String {
        self.text
    }
}

fn trim(v: f64) -> String {
    if v.fract() == 0.0 && v.abs() < 1e12 {
        format!("{}", v as i64)
    } else {
        format!("{:.4}", v)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_text() {
        let mut p = Packet::new("acevo");
        p.num("speed", 42.5).num("gear", 3.0).list("tyre_s", &[1.0, f64::NAN, 2.5, 3.0]);
        p.names("cname", &["A|B".to_string(), "C".to_string()]);
        let t = p.finish();
        assert!(t.starts_with("RDB1\ngame=acevo\n"));
        assert!(t.contains("speed=42.5000\n"));
        assert!(t.contains("gear=3\n"));
        assert!(t.contains("tyre_s=1,,2.5000,3\n"));
        assert!(t.contains("cname=A B|C\n"));
    }
}

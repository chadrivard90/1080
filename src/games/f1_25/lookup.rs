//! Lookup tables from the F1 25 spec appendices, plus the colours we use for them.

use slint::Color;

pub fn hex(rgb: u32) -> Color {
    Color::from_rgb_u8((rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8)
}

pub fn track_name(id: i8) -> &'static str {
    match id {
        0 => "MELBOURNE",
        2 => "SHANGHAI",
        3 => "SAKHIR",
        4 => "CATALUNYA",
        5 => "MONACO",
        6 => "MONTREAL",
        7 => "SILVERSTONE",
        9 => "HUNGARORING",
        10 => "SPA",
        11 => "MONZA",
        12 => "SINGAPORE",
        13 => "SUZUKA",
        14 => "ABU DHABI",
        15 => "AUSTIN",
        16 => "INTERLAGOS",
        17 => "RED BULL RING",
        19 => "MEXICO CITY",
        20 => "BAKU",
        26 => "ZANDVOORT",
        27 => "IMOLA",
        29 => "JEDDAH",
        30 => "MIAMI",
        31 => "LAS VEGAS",
        32 => "LOSAIL",
        39 => "SILVERSTONE REV",
        40 => "AUSTRIA REV",
        41 => "ZANDVOORT REV",
        _ => "",
    }
}

pub fn session_name(session_type: u8) -> &'static str {
    match session_type {
        1 => "FP1",
        2 => "FP2",
        3 => "FP3",
        4 => "PRACTICE",
        5 => "Q1",
        6 => "Q2",
        7 => "Q3",
        8 => "QUALIFYING",
        9 => "ONE-SHOT Q",
        10 => "SQ1",
        11 => "SQ2",
        12 => "SQ3",
        13 => "SPRINT SHOOTOUT",
        14 => "ONE-SHOT SQ",
        15 => "RACE",
        16 => "RACE 2",
        17 => "RACE 3",
        18 => "TIME TRIAL",
        _ => "SESSION",
    }
}

/// Team livery colours (2025 grid). Unknown / custom teams get grey.
pub fn team_color(team_id: u8) -> Color {
    hex(match team_id {
        0 | 185 => 0x27F4D2, // Mercedes
        1 | 186 => 0xE8002D, // Ferrari
        2 | 187 => 0x3671C6, // Red Bull Racing
        3 | 188 => 0x64C4FF, // Williams
        4 | 189 => 0x229971, // Aston Martin
        5 | 190 => 0x0093CC, // Alpine
        6 | 191 => 0x6692FF, // RB
        7 | 192 => 0xB6BABD, // Haas
        8 | 193 => 0xFF8000, // McLaren
        9 | 194 => 0x52E252, // Sauber
        _ => 0x8D8DA3,
    })
}

/// (letter, colour) for a *visual* tyre compound.
pub fn compound(visual: u8) -> (&'static str, Color) {
    match visual {
        16 => ("S", hex(0xFF2A2A)),
        17 => ("M", hex(0xFFD400)),
        18 => ("H", hex(0xF0F0F0)),
        7 => ("I", hex(0x2ECC40)),
        8 => ("W", hex(0x2F7BFF)),
        9 => ("D", hex(0xF0F0F0)),
        10 | 15 => ("W", hex(0x2F7BFF)),
        19 => ("SS", hex(0xB545FF)),
        20 => ("S", hex(0xFF2A2A)),
        21 => ("M", hex(0xFFD400)),
        22 => ("H", hex(0xF0F0F0)),
        _ => ("-", hex(0x5A5A6E)),
    }
}

pub fn weather(w: u8) -> (&'static str, Color) {
    match w {
        0 => ("CLEAR", hex(0xFFD23F)),
        1 => ("LIGHT CLOUD", hex(0xC9D6E3)),
        2 => ("OVERCAST", hex(0x8F9BAB)),
        3 => ("LIGHT RAIN", hex(0x4EA8FF)),
        4 => ("HEAVY RAIN", hex(0x2F6BFF)),
        5 => ("STORM", hex(0x8A5CFF)),
        _ => ("--", hex(0x5A5A6E)),
    }
}

pub fn ers_mode(mode: u8) -> (&'static str, Color) {
    match mode {
        1 => ("MEDIUM", hex(0x00D26A)),
        2 => ("HOTLAP", hex(0xFFD400)),
        3 => ("OVERTAKE", hex(0xFF3B3B)),
        _ => ("NONE", hex(0x8D8DA3)),
    }
}

pub fn fuel_mix(mix: u8) -> &'static str {
    match mix {
        0 => "LEAN",
        1 => "STANDARD",
        2 => "RICH",
        3 => "MAX",
        _ => "--",
    }
}

pub fn traction_control(tc: u8) -> &'static str {
    match tc {
        0 => "OFF",
        1 => "MED",
        2 => "FULL",
        _ => "--",
    }
}

pub fn penalty_text(penalty_type: u8, time_s: u8) -> String {
    match penalty_type {
        0 => "PENALTY  DRIVE THROUGH".into(),
        1 => "PENALTY  STOP GO".into(),
        2 => "GRID PENALTY".into(),
        4 => format!("PENALTY  +{time_s}s"),
        5 => "WARNING".into(),
        6 => "DISQUALIFIED".into(),
        10..=15 => "LAP INVALIDATED".into(),
        17 => "BLACK FLAG".into(),
        _ => "PENALTY".into(),
    }
}

/// Tyre carcass (inner) temperature colour, °C. F1 working window ~90-105 °C.
pub fn tyre_inner_color(c: f32) -> Color {
    hex(if c < 70.0 {
        0x2F6BFF
    } else if c < 85.0 {
        0x36C5F0
    } else if c <= 105.0 {
        0x00C060
    } else if c <= 115.0 {
        0xFFA000
    } else {
        0xFF2A2A
    })
}

/// Tyre surface temperature colour, °C (surface swings more, so a wider window).
pub fn tyre_surface_color(c: f32) -> Color {
    hex(if c < 70.0 {
        0x2F6BFF
    } else if c < 85.0 {
        0x36C5F0
    } else if c <= 110.0 {
        0x00C060
    } else if c <= 120.0 {
        0xFFA000
    } else {
        0xFF2A2A
    })
}

pub fn brake_temp_color(c: f32) -> Color {
    hex(if c < 200.0 {
        0x2F6BFF
    } else if c < 400.0 {
        0x36C5F0
    } else if c <= 900.0 {
        0x00C060
    } else if c <= 1050.0 {
        0xFFA000
    } else {
        0xFF2A2A
    })
}

pub fn damage_color(pct: u8) -> Color {
    hex(if pct < 5 {
        0x1E7A46
    } else if pct < 25 {
        0xC9B400
    } else if pct < 50 {
        0xFF8A00
    } else {
        0xFF2020
    })
}

//! Loads custom artwork (e.g. AI-generated images) from the `art` folder at start-up.
//!
//! Each slot is an image file named `<slot>.png` or `<slot>.jpg`. Missing slots keep the
//! built-in vector art. Folder search order:
//!   1. $RACE_DASH_ART
//!   2. <folder containing the app>/art          (release installs: ~/race-dash/art)
//!   3. <folder containing the app>/../../art    (source builds: target/release -> repo/art)

use std::path::{Path, PathBuf};

use slint::ComponentHandle as _;

fn art_dir() -> Option<PathBuf> {
    if let Some(p) = std::env::var_os("RACE_DASH_ART") {
        return Some(PathBuf::from(p));
    }
    let exe = std::env::current_exe().ok()?;
    let dir = exe.parent()?;
    [dir.join("art"), dir.join("..").join("..").join("art")].into_iter().find(|p| p.is_dir())
}

fn find(dir: &Path, slot: &str) -> Option<slint::Image> {
    for ext in ["png", "jpg", "jpeg", "PNG", "JPG"] {
        let path = dir.join(format!("{slot}.{ext}"));
        if path.is_file() {
            match slint::Image::load_from_path(&path) {
                Ok(img) => return Some(img),
                Err(e) => eprintln!("race-dash: could not load {}: {e:?}", path.display()),
            }
        }
    }
    None
}

/// Apply every custom image found. Returns a short status line for the title screen.
pub fn load(ui: &crate::AppWindow) -> String {
    let Some(dir) = art_dir() else {
        return "built-in art".into();
    };
    let art = ui.global::<crate::Art>();
    let mut count = 0;

    macro_rules! img {
        ($slot:literal, $set:ident) => {
            if let Some(i) = find(&dir, $slot) {
                art.$set(i);
                count += 1;
            }
        };
        ($slot:literal, $set:ident, $flag:ident) => {
            if let Some(i) = find(&dir, $slot) {
                art.$set(i);
                art.$flag(true);
                count += 1;
            }
        };
    }

    img!("title-bg", set_title_bg, set_title_bg_custom);
    img!("title-hero", set_title_hero);

    img!("tile-f1", set_tile_f1, set_tile_f1_custom);
    img!("tile-nascar", set_tile_nascar, set_tile_nascar_custom);
    img!("tile-acevo", set_tile_acevo, set_tile_acevo_custom);
    img!("tile-ams2", set_tile_ams2, set_tile_ams2_custom);
    img!("tile-dr2", set_tile_dr2, set_tile_dr2_custom);
    img!("tile-wrc", set_tile_wrc, set_tile_wrc_custom);
    img!("tile-fm", set_tile_fm, set_tile_fm_custom);
    img!("tile-fh6", set_tile_fh6, set_tile_fh6_custom);

    img!("weather-clear", set_weather_clear, set_weather_clear_custom);
    img!("weather-cloud", set_weather_cloud, set_weather_cloud_custom);
    img!("weather-rain", set_weather_rain, set_weather_rain_custom);
    img!("weather-storm", set_weather_storm, set_weather_storm_custom);
    img!("weather-snow", set_weather_snow, set_weather_snow_custom);

    img!("emblem-f1", set_emblem_f1);
    img!("emblem-nascar", set_emblem_nascar);
    img!("emblem-acevo", set_emblem_acevo);
    img!("emblem-ams2", set_emblem_ams2);
    img!("emblem-dr2", set_emblem_dr2);
    img!("emblem-wrc", set_emblem_wrc);
    img!("emblem-fm", set_emblem_fm);
    img!("emblem-fh6", set_emblem_fh6);

    img!("bg-f1", set_bg_f1, set_bg_f1_custom);
    img!("bg-nascar", set_bg_nascar, set_bg_nascar_custom);
    img!("bg-acevo", set_bg_acevo, set_bg_acevo_custom);
    img!("bg-ams2", set_bg_ams2, set_bg_ams2_custom);
    img!("bg-dr2", set_bg_dr2, set_bg_dr2_custom);
    img!("bg-wrc", set_bg_wrc, set_bg_wrc_custom);
    img!("bg-fm", set_bg_fm, set_bg_fm_custom);
    img!("bg-fh6", set_bg_fh6, set_bg_fh6_custom);

    img!("dr2-tach-face", set_dr2_tach_face);
    img!("dr2-needle", set_dr2_needle);
    img!("dr2-gauge-face", set_dr2_gauge_face, set_dr2_gauge_face_custom);
    img!("car-gt", set_car_gt);
    img!("car-drift", set_car_drift);
    img!("car-rally", set_car_rally);
    img!("car-f1", set_car_f1, set_car_f1_custom);
    img!("car-stock", set_car_stock, set_car_stock_custom);

    img!("panel-f1", set_panel_f1, set_panel_f1_custom);
    img!("panel-nascar", set_panel_nascar, set_panel_nascar_custom);
    img!("panel-acevo", set_panel_acevo, set_panel_acevo_custom);
    img!("panel-ams2", set_panel_ams2, set_panel_ams2_custom);
    img!("panel-dr2", set_panel_dr2, set_panel_dr2_custom);
    img!("panel-wrc", set_panel_wrc, set_panel_wrc_custom);
    img!("panel-fm", set_panel_fm, set_panel_fm_custom);
    img!("panel-fh6", set_panel_fh6, set_panel_fh6_custom);

    img!("flag-yellow", set_flag_yellow, set_flag_yellow_custom);
    img!("flag-blue", set_flag_blue, set_flag_blue_custom);
    img!("flag-green", set_flag_green, set_flag_green_custom);
    img!("flag-white", set_flag_white, set_flag_white_custom);
    img!("flag-red", set_flag_red, set_flag_red_custom);
    img!("flag-black", set_flag_black, set_flag_black_custom);
    img!("flag-chequered", set_flag_chequered, set_flag_chequered_custom);
    img!("tyre-soft", set_tyre_soft, set_tyre_soft_custom);
    img!("tyre-medium", set_tyre_medium, set_tyre_medium_custom);
    img!("tyre-hard", set_tyre_hard, set_tyre_hard_custom);
    img!("tyre-inter", set_tyre_inter, set_tyre_inter_custom);
    img!("tyre-wet", set_tyre_wet, set_tyre_wet_custom);
    img!("pb-medal", set_pb_medal, set_pb_medal_custom);
    img!("dock-f1", set_dock_f1, set_dock_f1_custom);
    img!("dock-nascar", set_dock_nascar, set_dock_nascar_custom);
    img!("dock-acevo", set_dock_acevo, set_dock_acevo_custom);
    img!("dock-ams2", set_dock_ams2, set_dock_ams2_custom);
    img!("dock-dr2", set_dock_dr2, set_dock_dr2_custom);
    img!("dock-wrc", set_dock_wrc, set_dock_wrc_custom);
    img!("dock-fm", set_dock_fm, set_dock_fm_custom);
    img!("dock-fh6", set_dock_fh6, set_dock_fh6_custom);
    img!("logo", set_logo, set_logo_custom);
    img!("wrc-gauge-face", set_wrc_gauge_face, set_wrc_gauge_face_custom);
    img!("fm-gauge-face", set_fm_gauge_face, set_fm_gauge_face_custom);
    img!("weather-puddle", set_weather_puddle, set_weather_puddle_custom);
    img!("weather-wetline", set_weather_wetline, set_weather_wetline_custom);

    if count == 0 {
        "built-in art".into()
    } else {
        format!("{count} custom image{}", if count == 1 { "" } else { "s" })
    }
}

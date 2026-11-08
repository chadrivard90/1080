//! Race Dash - sim racing telemetry screen for a 1080x1920 portrait touch display.
//!
//! Page 0 is the title / game picker. Page N (1..=8) is the dash for title tile N-1:
//! 0 F1 25, 1 NASCAR, 2 AC EVO, 3 AMS2, 4 DiRT Rally 2.0, 5 EA WRC, 6 Forza Motorsport, 7 FH6.

slint::include_modules!();

mod art;
mod config;
mod games;
mod laplog;
mod net;

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

use slint::{ComponentHandle as _, ModelRc, VecModel};

use games::sim::{Game, SessionOptions, SimSession};

const GAME_F1_25: i32 = 0;

/// Whatever game is currently running. Dropping it stops telemetry for that game.
enum Active {
    F1(games::f1_25::F1Session),
    Sim(SimSession),
}

/// Config key for a title-screen game index.
fn game_key(index: i32) -> &'static str {
    match Game::from_index(index) {
        Some(g) => g.key(),
        None => "f1",
    }
}

fn default_port(index: i32) -> u16 {
    Game::from_index(index).map_or(20777, |g| g.default_port())
}

fn imperial_default(index: i32) -> bool {
    Game::from_index(index).is_some_and(|g| g.imperial_default())
}

fn fuel_text(tenths: i32) -> String {
    format!("+{}.{} LAPS", tenths / 10, tenths % 10)
}

/// Fill the settings page for one game (title-screen index).
fn load_settings_page(ui: &AppWindow, cfg: &config::Config, game: i32) {
    let app = ui.global::<AppState>();
    let key = game_key(game);
    app.set_set_game(game);
    app.set_set_imperial(cfg.imperial(key, imperial_default(game)));
    app.set_set_units_from_game(game == GAME_F1_25);
    app.set_set_port(cfg.port(key, default_port(game)) as i32);
    app.set_set_default_port(default_port(game) as i32);
    app.set_set_shift(cfg.shift_pct(key));
    app.set_set_shift_from_game(game == GAME_F1_25);
    let tenths = (cfg.fuel_margin(key) * 10.0).round() as i32;
    app.set_set_fuel_tenths(tenths);
    app.set_set_fuel_text(fuel_text(tenths).into());

    let log = laplog::LapLog::open();
    let rows: Vec<BestRow> = log
        .bests_for(key)
        .into_iter()
        .map(|b| BestRow {
            track: b.track.into(),
            car: if b.car.is_empty() { "-".to_string() } else { b.car }.into(),
            time: b.best_s.map_or_else(|| "NO VALID LAP".to_string(), laplog::fmt_lap).into(),
            laps: format!("{} LAPS", b.laps).into(),
        })
        .collect();
    app.set_bests(ModelRc::new(VecModel::from(rows)));
    app.set_log_path(laplog::log_path().map_or_else(String::new, |p| p.display().to_string()).into());
    app.set_log_summary(format!("{} LAPS LOGGED ACROSS ALL GAMES", log.total_laps()).into());
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ui = AppWindow::new()?;
    let config = Rc::new(RefCell::new(config::Config::load()));
    let current_game = Rc::new(Cell::new(-1i32));
    let map_orient = Rc::new(Cell::new(0u8));

    {
        let app = ui.global::<AppState>();
        let cfg = config.borrow();
        app.set_version(env!("CARGO_PKG_VERSION").into());
        app.set_ip_address(net::local_ip().into());
        app.set_demo_mode(cfg.demo());
        app.set_glass(cfg.glass());
        app.set_f1_port(cfg.port("f1", 20777) as i32);
    }

    // Custom artwork from the art folder (falls back to the built-in vector art).
    let art_status = art::load(&ui);
    ui.global::<AppState>().set_art_status(art_status.into());

    let active: Rc<RefCell<Option<Active>>> = Rc::new(RefCell::new(None));

    ui.global::<AppState>().on_select_game({
        let weak = ui.as_weak();
        let active = active.clone();
        let config = config.clone();
        let current_game = current_game.clone();
        let map_orient = map_orient.clone();
        move |game| {
            let Some(ui) = weak.upgrade() else { return };
            active.borrow_mut().take();
            let app = ui.global::<AppState>();
            let demo = app.get_demo_mode();
            let ip = net::local_ip();
            app.set_ip_address(ip.clone().into());
            let key = game_key(game);
            let (layout, orient) = {
                let cfg = config.borrow();
                // Every dash has 3 pages now; older configs may hold a higher tab index.
                (cfg.layout(key).clamp(0, 2), cfg.map_orient(key))
            };
            map_orient.set(orient);
            current_game.set(game);

            let session = if game == GAME_F1_25 {
                let (port, margin) = {
                    let cfg = config.borrow();
                    (cfg.port("f1", 20777), cfg.fuel_margin("f1"))
                };
                ui.global::<F1>().set_layout(layout);
                Active::F1(games::f1_25::F1Session::start(&ui, demo, port, margin, map_orient.clone()))
            } else if let Some(g) = Game::from_index(game) {
                let (port, imperial, shift_frac, fuel_margin) = {
                    let cfg = config.borrow();
                    (
                        cfg.port(g.key(), g.default_port()),
                        cfg.imperial(g.key(), g.imperial_default()),
                        cfg.shift_pct(g.key()) as f32 / 100.0,
                        cfg.fuel_margin(g.key()),
                    )
                };
                ui.global::<Sim>().set_layout(layout);
                Active::Sim(SimSession::start(
                    &ui,
                    SessionOptions { game: g, demo, port, imperial, shift_frac, fuel_margin, ip, map_orient: map_orient.clone() },
                ))
            } else {
                return;
            };
            *active.borrow_mut() = Some(session);
            ui.set_page(game + 1);
        }
    });

    ui.global::<AppState>().on_go_home({
        let weak = ui.as_weak();
        let active = active.clone();
        let current_game = current_game.clone();
        move || {
            active.borrow_mut().take();
            current_game.set(-1);
            if let Some(ui) = weak.upgrade() {
                ui.global::<AppState>().set_ip_address(net::local_ip().into());
                ui.set_page(0);
            }
        }
    });

    ui.global::<AppState>().on_exit_app(|| {
        let _ = slint::quit_event_loop();
    });

    ui.global::<AppState>().on_settings_changed({
        let weak = ui.as_weak();
        let config = config.clone();
        let current_game = current_game.clone();
        move || {
            let Some(ui) = weak.upgrade() else { return };
            let mut cfg = config.borrow_mut();
            cfg.set_demo(ui.global::<AppState>().get_demo_mode());
            cfg.set_glass(ui.global::<AppState>().get_glass());
            let game = current_game.get();
            if game == GAME_F1_25 {
                cfg.set_layout("f1", ui.global::<F1>().get_layout());
            } else if game > 0 {
                cfg.set_layout(game_key(game), ui.global::<Sim>().get_layout());
            }
            cfg.save();
        }
    });

    // Settings page (page 9).
    ui.global::<AppState>().on_open_settings({
        let weak = ui.as_weak();
        let config = config.clone();
        move || {
            let Some(ui) = weak.upgrade() else { return };
            let game = ui.global::<AppState>().get_set_game().clamp(0, 7);
            load_settings_page(&ui, &config.borrow(), game);
            ui.set_page(9);
        }
    });
    ui.global::<AppState>().on_settings_pick_game({
        let weak = ui.as_weak();
        let config = config.clone();
        move |game| {
            let Some(ui) = weak.upgrade() else { return };
            load_settings_page(&ui, &config.borrow(), game.clamp(0, 7));
        }
    });
    ui.global::<AppState>().on_game_settings_changed({
        let weak = ui.as_weak();
        let config = config.clone();
        move || {
            let Some(ui) = weak.upgrade() else { return };
            let app = ui.global::<AppState>();
            let game = app.get_set_game().clamp(0, 7);
            let key = game_key(game);
            let port = u16::try_from(app.get_set_port().clamp(1024, 65535)).unwrap_or(default_port(game));
            let shift = app.get_set_shift().clamp(80, 100);
            let tenths = app.get_set_fuel_tenths().clamp(0, 50);
            // Write the clamped values back so the page shows what was saved.
            app.set_set_port(port as i32);
            app.set_set_shift(shift);
            app.set_set_fuel_tenths(tenths);
            app.set_set_fuel_text(fuel_text(tenths).into());
            let mut cfg = config.borrow_mut();
            cfg.set_port(key, port);
            if game != GAME_F1_25 {
                cfg.set_imperial(key, app.get_set_imperial());
                cfg.set_shift_pct(key, shift);
            }
            cfg.set_fuel_margin(key, tenths as f32 / 10.0);
            cfg.save();
            if game == GAME_F1_25 {
                app.set_f1_port(port as i32);
            }
        }
    });

    // Tapping a track map rotates it (4 rotations x mirror); remembered per game.
    ui.global::<AppState>().on_rotate_map({
        let config = config.clone();
        let current_game = current_game.clone();
        let map_orient = map_orient.clone();
        move || {
            let next = (map_orient.get() + 1) & 7;
            map_orient.set(next);
            let game = current_game.get();
            if game >= 0 {
                let mut cfg = config.borrow_mut();
                cfg.set_map_orient(game_key(game), next);
                cfg.save();
            }
        }
    });

    // Keep the IP shown on the title screen fresh (Wi-Fi may connect after launch).
    let ip_timer = slint::Timer::default();
    ip_timer.start(slint::TimerMode::Repeated, Duration::from_secs(5), {
        let weak = ui.as_weak();
        move || {
            if let Some(ui) = weak.upgrade() {
                if ui.get_page() == 0 {
                    ui.global::<AppState>().set_ip_address(net::local_ip().into());
                }
            }
        }
    });

    // Animation clock: a gentle pulse for highlights everywhere, plus the title screen's slow
    // hero pan and tile light sweep (only while the title screen is showing).
    let anim_timer = slint::Timer::default();
    anim_timer.start(slint::TimerMode::Repeated, Duration::from_millis(40), {
        let weak = ui.as_weak();
        let started = std::time::Instant::now();
        move || {
            let Some(ui) = weak.upgrade() else { return };
            let t = started.elapsed().as_secs_f32();
            let app = ui.global::<AppState>();
            app.set_pulse(0.5 + 0.5 * (t * std::f32::consts::TAU / 1.2).sin());
            if ui.get_page() == 0 {
                app.set_hero_shift((t * std::f32::consts::TAU / 24.0).sin());
                app.set_sweep((t / 5.0).fract());
            }
        }
    });

    ui.show()?;
    // Fullscreen on the Pi. Set RACE_DASH_WINDOWED=1 to run in a normal window for development.
    if std::env::var_os("RACE_DASH_WINDOWED").is_none() {
        ui.window().set_fullscreen(true);
    }
    slint::run_event_loop()?;

    active.borrow_mut().take();
    Ok(())
}

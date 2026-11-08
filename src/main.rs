//! Race Dash - sim racing telemetry screen for a 1080x1920 portrait touch display.
//!
//! Page 0 is the title / game picker. Page N (1..=8) is the dash for title tile N-1:
//! 0 F1 25, 1 NASCAR, 2 AC EVO, 3 AMS2, 4 DiRT Rally 2.0, 5 EA WRC, 6 Forza Motorsport, 7 FH6.

slint::include_modules!();

mod art;
mod config;
mod games;
mod net;

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

use slint::ComponentHandle as _;

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
                let port = config.borrow().port("f1", 20777);
                ui.global::<F1>().set_layout(layout);
                Active::F1(games::f1_25::F1Session::start(&ui, demo, port, map_orient.clone()))
            } else if let Some(g) = Game::from_index(game) {
                let (port, imperial) = {
                    let cfg = config.borrow();
                    (cfg.port(g.key(), g.default_port()), cfg.imperial(g.key(), g.imperial_default()))
                };
                ui.global::<Sim>().set_layout(layout);
                Active::Sim(SimSession::start(
                    &ui,
                    SessionOptions { game: g, demo, port, imperial, ip, map_orient: map_orient.clone() },
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
            let game = current_game.get();
            if game == GAME_F1_25 {
                cfg.set_layout("f1", ui.global::<F1>().get_layout());
            } else if game > 0 {
                cfg.set_layout(game_key(game), ui.global::<Sim>().get_layout());
            }
            cfg.save();
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

    ui.show()?;
    // Fullscreen on the Pi. Set RACE_DASH_WINDOWED=1 to run in a normal window for development.
    if std::env::var_os("RACE_DASH_WINDOWED").is_none() {
        ui.window().set_fullscreen(true);
    }
    slint::run_event_loop()?;

    active.borrow_mut().take();
    Ok(())
}

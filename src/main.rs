//! Race Dash — sim racing telemetry screen for a 1080x1920 portrait touch display.
//!
//! Page 0 is the title / game picker. Each game gets its own page (page = game index + 1)
//! with its own themed dash and a bottom nav bar to switch layouts.

slint::include_modules!();

mod config;
mod games;
mod net;

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use slint::ComponentHandle as _;

/// Game indexes as used by the title screen tiles (see ui/title.slint).
const GAME_F1_25: i32 = 0;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ui = AppWindow::new()?;
    let config = Rc::new(RefCell::new(config::Config::load()));

    {
        let app = ui.global::<AppState>();
        let cfg = config.borrow();
        app.set_version(env!("CARGO_PKG_VERSION").into());
        app.set_ip_address(net::local_ip().into());
        app.set_demo_mode(cfg.demo);
        app.set_f1_port(cfg.f1_port as i32);
        ui.global::<F1>().set_layout(cfg.f1_layout);
    }

    // The currently running game session (telemetry listener + UI refresh timer).
    let active: Rc<RefCell<Option<games::f1_25::F1Session>>> = Rc::new(RefCell::new(None));

    ui.global::<AppState>().on_select_game({
        let weak = ui.as_weak();
        let active = active.clone();
        let config = config.clone();
        move |game| {
            let Some(ui) = weak.upgrade() else { return };
            // Stop whatever was running before starting something new.
            active.borrow_mut().take();
            match game {
                GAME_F1_25 => {
                    let demo = ui.global::<AppState>().get_demo_mode();
                    let port = config.borrow().f1_port;
                    let session = games::f1_25::F1Session::start(&ui, demo, port);
                    *active.borrow_mut() = Some(session);
                    ui.set_page(game + 1);
                }
                _ => {} // Other games are not implemented yet (tiles are disabled).
            }
        }
    });

    ui.global::<AppState>().on_go_home({
        let weak = ui.as_weak();
        let active = active.clone();
        move || {
            active.borrow_mut().take();
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
        move || {
            let Some(ui) = weak.upgrade() else { return };
            let mut cfg = config.borrow_mut();
            cfg.demo = ui.global::<AppState>().get_demo_mode();
            cfg.f1_layout = ui.global::<F1>().get_layout();
            cfg.save();
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

//! F1 25: UDP telemetry listener + UI refresh.

mod demo;
pub mod lookup;
pub mod packets;
mod udp;
pub mod view;

use std::cell::Cell;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use slint::{ComponentHandle as _, ModelRc};

use packets::F1State;

/// UI refresh rate. 30 Hz is smooth on a Pi while leaving CPU headroom.
const UI_REFRESH: Duration = Duration::from_millis(33);

/// A running F1 25 session. Dropping it stops the listener thread and the refresh timer.
pub struct F1Session {
    stop: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
    _timer: slint::Timer,
}

impl F1Session {
    pub fn start(ui: &crate::AppWindow, demo: bool, port: u16, map_orient: Rc<Cell<u8>>) -> Self {
        let shared = Arc::new(Mutex::new(F1State::default()));
        let stop = Arc::new(AtomicBool::new(false));

        let worker = {
            let shared = shared.clone();
            let stop = stop.clone();
            std::thread::Builder::new()
                .name("f1-25-telemetry".into())
                .spawn(move || {
                    if demo {
                        demo::run(shared, stop)
                    } else {
                        udp::run(shared, stop, port)
                    }
                })
                .ok()
        };

        let models = view::Models::new(map_orient);
        let f1 = ui.global::<crate::F1>();
        f1.set_map_dots(ModelRc::from(models.map_dots.clone()));
        f1.set_relative(ModelRc::from(models.relative.clone()));
        f1.set_standings(ModelRc::from(models.standings.clone()));
        f1.set_laps(ModelRc::from(models.laps.clone()));
        f1.set_forecast(ModelRc::from(models.forecast.clone()));

        let timer = slint::Timer::default();
        let weak = ui.as_weak();
        let started = Instant::now();
        timer.start(slint::TimerMode::Repeated, UI_REFRESH, move || {
            let Some(ui) = weak.upgrade() else { return };
            let blink = (started.elapsed().as_millis() / 160) % 2 == 0;
            let state = match shared.lock() {
                Ok(guard) => guard,
                Err(poisoned) => poisoned.into_inner(),
            };
            view::apply(&ui, &state, &models, blink);
        });

        Self { stop, worker, _timer: timer }
    }
}

impl Drop for F1Session {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(handle) = self.worker.take() {
            let _ = handle.join();
        }
    }
}

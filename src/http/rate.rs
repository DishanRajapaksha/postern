//! A global public budget bounds work without trusting caller-controlled proxy IPs.
use std::{sync::Mutex, time::Instant};

pub(super) struct Rate(Mutex<(Instant, f64)>);
impl Default for Rate {
    fn default() -> Self {
        Self(Mutex::new((Instant::now(), 120.0)))
    }
}
impl Rate {
    pub fn admit(&self) -> bool {
        let Ok(mut state) = self.0.lock() else {
            return false;
        };
        let now = Instant::now();
        state.1 = (state.1 + now.duration_since(state.0).as_secs_f64() * 30.0).min(120.0);
        state.0 = now;
        if state.1 < 1.0 {
            return false;
        }
        state.1 -= 1.0;
        true
    }
}

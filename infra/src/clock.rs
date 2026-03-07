//! Clock adapter: system time for the composition root.
//!
//! Implements the `Clock` port so use cases stay testable (inject a mock in tests).

use chrono::{DateTime, Utc};

use portablenote_core::application::ports::Clock;

/// System clock: delegates to `Utc::now()`. Use at the composition root;
/// use a mock in unit tests.
#[derive(Clone, Copy, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> DateTime<Utc> {
        Utc::now()
    }
}

use chrono::{Datelike, Local, Timelike};

use crate::application::ports::MonotonicClock;
use crate::application::upload_service::Clock;
use crate::domain::path_template::DateParts;

/// Reads the machine's local time once per call, so one expansion stays consistent.
pub struct ChronoClock;

impl Clock for ChronoClock {
    fn now(&self) -> DateParts {
        let now = Local::now();
        DateParts {
            year: now.year(),
            month: now.month(),
            day: now.day(),
            hour: now.hour(),
            minute: now.minute(),
            second: now.second(),
        }
    }
}

/// Milliseconds elapsed since construction; immune to wall-clock changes.
pub struct InstantClock(std::time::Instant);

impl InstantClock {
    pub fn new() -> Self {
        Self(std::time::Instant::now())
    }
}

impl Default for InstantClock {
    fn default() -> Self {
        Self::new()
    }
}

impl MonotonicClock for InstantClock {
    fn now_ms(&self) -> u64 {
        u64::try_from(self.0.elapsed().as_millis()).unwrap_or(u64::MAX)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chrono_clock_returns_plausible_local_time() {
        let now = ChronoClock.now();
        assert!(now.year >= 2024);
        assert!((1..=12).contains(&now.month));
        assert!((1..=31).contains(&now.day));
        assert!(now.hour < 24);
        assert!(now.minute < 60);
        assert!(now.second < 60);
    }

    #[test]
    fn instant_clock_never_goes_backwards() {
        let clock = InstantClock::new();
        let a = clock.now_ms();
        std::thread::sleep(std::time::Duration::from_millis(5));
        assert!(clock.now_ms() >= a + 5);
    }
}

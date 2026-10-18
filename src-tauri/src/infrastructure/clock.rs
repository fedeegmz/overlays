use chrono::{Datelike, Local, Timelike};

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
}

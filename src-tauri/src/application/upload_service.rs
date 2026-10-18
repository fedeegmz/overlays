use crate::domain::path_template::DateParts;

/// Source of the machine's local date and time, injectable for deterministic tests.
pub trait Clock: Send + Sync {
    fn now(&self) -> DateParts;
}

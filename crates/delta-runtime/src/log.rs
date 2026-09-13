use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum LogLevel {
    Disabled,
    #[default]
    Normal,
    Verbose,
    Debug,
}

impl LogLevel {
    pub fn allows(self, needed: LogLevel) -> bool {
        rank(self) >= rank(needed)
    }
}

fn rank(level: LogLevel) -> u8 {
    match level {
        LogLevel::Disabled => 0,
        LogLevel::Normal => 1,
        LogLevel::Verbose => 2,
        LogLevel::Debug => 3,
    }
}

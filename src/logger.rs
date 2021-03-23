use lazy_static::lazy_static;
use std::sync::Mutex;
use serde::{Serialize, Deserialize, Serializer, Deserializer};

lazy_static! {
    pub static ref LOGGER: Mutex<Logger> = Mutex::new(Logger::new(LogLevel::Warn));
}

#[derive(Copy, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Error = 0,
    Warn = 1,
    Info = 2,
    Debug = 3
}

// impl Serialize for LogLevel {
//     fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
//     where S: Serializer,
//     {
//         let log_level_string = match self {
//             Error => "error",
//             Warn => "warn",
//             Info => "info",
//             Debug => "debug"
//         };

//         serializer.serialize_str(log_level_string)
//     }
// }

// impl<'de> Deserialize<'de> for LogLevel {
//     fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
//     where D: Deserializer<'de>,
//     {
//         let level = deserializer.deserialize_str()
//     }
// }

pub struct Logger {
    log_level: LogLevel
}

impl Logger {
    pub fn new(log_level: LogLevel) -> Logger {
        Logger {
            log_level,
        }
    }

    pub fn set_log_level(&mut self, log_level: LogLevel) {
        self.log_level = log_level;
    }

    pub fn log(&self, msg: &str, log_level: LogLevel) {
        let log_level_num = log_level as u8;
        if log_level_num > self.log_level as u8 {
            return;
        }

        let level: &str = if log_level_num == 0 {
            "ERROR"
        } else if log_level_num == 1 {
            "WARN"
        } else if log_level_num == 2 {
            "INFO"
        } else if log_level_num == 3 {
            "DEBUG"
        } else {
            "OTHER"
        };

        println!("[{}] {}", level, msg);
    }
}

#[macro_export]
macro_rules! error {
    ($msg:expr) => {{
        use $crate::logger::{LOGGER, LogLevel};
        LOGGER.lock().unwrap().log(&$msg, LogLevel::Error);
    }}
}

#[macro_export]
macro_rules! warn {
    ($msg:expr) => {{
        use $crate::logger::{LOGGER, LogLevel};
        LOGGER.lock().unwrap().log(&$msg, LogLevel::Warn);
    }}
}

#[macro_export]
macro_rules! info {
    ($msg:expr) => {{
        use $crate::logger::{LOGGER, LogLevel};
        LOGGER.lock().unwrap().log(&$msg, LogLevel::Info);
    }}
}

#[macro_export]
macro_rules! debug {
    ($msg:expr) => {{
        use $crate::logger::{LOGGER, LogLevel};
        LOGGER.lock().unwrap().log(&$msg, LogLevel::Debug);
    }}
}
use lazy_static::lazy_static;
use std::sync::Mutex;

lazy_static! {
    pub static ref LOGGER: Mutex<Logger> = Mutex::new(Logger::new(LogLevel::Warn));
}

#[derive(Copy, Clone)]
pub enum LogLevel {
    Unknown = 0,
    Error = 1,
    Warn = 2,
    Info = 3,
    Debug = 4,
}

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
            "UNKNOWN"
        } else if log_level_num == 1 {
            "ERROR"
        } else if log_level_num == 2 {
            "WARN"
        } else if log_level_num == 3 {
            "INFO"
        } else if log_level_num == 4 {
            "DEBUG"
        } else {
            "OTHER"
        };

        println!("[{}] {}", level, msg);
    }

    pub fn string_to_log_level(str_level: &str) -> LogLevel {
        if str_level == "error" {
            LogLevel::Error
        } else if str_level == "warn" {
            LogLevel::Warn
        } else if str_level == "info" {
            LogLevel::Info
        } else if str_level == "debug" {
            LogLevel::Debug
        } else {
            LogLevel::Unknown
        }
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
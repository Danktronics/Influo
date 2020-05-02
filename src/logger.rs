static logger: Logger = Logger::new(LogLevel::Warn);

enum LogLevel {
    Error = 0,
    Warn = 1,
    Info = 2,
}

struct Logger {
    log_level: LogLevel
}

impl Logger {
    pub fn new(log_level: LogLevel) -> Logger {
        Logger {
            log_level: log_level,
        }
    }

    pub fn set_log_level(&mut self, log_level: LogLevel) {
        self.log_level = log_level;
    }

    pub fn log(&self, msg: &str, log_level: LogLevel) {
        if log_level as u8 > self.log_level as u8 {
            return;
        }

        let level: &str = if log_level == 0 {
            "ERROR"
        } else if log_level == 1 {
            "WARN"
        } else if log_level == 2 {
            "INFO"
        } else {
            "OTHER"
        };

        println!("[{}] {}", level, msg);
    }
}

macro_rules! error {
    ($msg:expr) => {{
        logger.log($msg, LogLevel::Error);
    }}
}

macro_rules! warn {
    ($msg:expr) => {{
        logger.log($msg, LogLevel::Warn);
    }}
}

macro_rules! info {
    ($msg:expr) => {{
        logger.log($msg, LogLevel::Info);
    }}
}
use anyhow::Result;
use log::{LevelFilter, Log, Metadata, Record};
use std::fs::{create_dir_all, File, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::sync::Mutex;

/// Logger implementation for the Breez SDK
pub struct SdkLogger {
    app_logger: Option<Box<dyn Log>>,
    log_file: Mutex<Option<File>>,
}

impl SdkLogger {
    /// Creates a new SDK logger
    ///
    /// # Arguments
    ///
    /// * `log_dir` - Directory where log files will be stored
    /// * `app_logger` - Optional application logger to forward logs to
    ///
    /// # Returns
    ///
    /// A new `SdkLogger` instance
    pub fn new(log_dir: &str, app_logger: Option<Box<dyn Log>>) -> Result<Self> {
        let log_path = Path::new(log_dir);
        if !log_path.exists() {
            create_dir_all(log_path)?;
        }

        let log_file_path = log_path.join("sdk.log");
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_file_path)?;

        Ok(Self {
            app_logger,
            log_file: Mutex::new(Some(file)),
        })
    }

    /// Initializes the logger as the global logger
    ///
    /// # Arguments
    ///
    /// * `log_dir` - Directory where log files will be stored
    /// * `app_logger` - Optional application logger to forward logs to
    ///
    /// # Returns
    ///
    /// Result indicating success or failure
    pub fn init(log_dir: &str, app_logger: Option<Box<dyn Log>>) -> Result<()> {
        let logger = Self::new(log_dir, app_logger)?;
        log::set_boxed_logger(Box::new(logger))?;
        log::set_max_level(LevelFilter::Debug);
        Ok(())
    }
}

impl Log for SdkLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= log::max_level()
    }

    fn log(&self, record: &Record) {
        if !self.enabled(record.metadata()) {
            return;
        }

        // Format the log message
        let message = format!(
            "{} [{}] {}: {}\n",
            chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
            record.level(),
            record.target(),
            record.args()
        );

        // Write to log file
        if let Ok(mut file_guard) = self.log_file.lock() {
            if let Some(file) = file_guard.as_mut() {
                let _ = file.write_all(message.as_bytes());
                let _ = file.flush();
            }
        }

        // Forward to app logger if provided
        if let Some(app_logger) = &self.app_logger {
            app_logger.log(record);
        }
    }

    fn flush(&self) {
        if let Ok(mut file_guard) = self.log_file.lock() {
            if let Some(file) = file_guard.as_mut() {
                let _ = file.flush();
            }
        }

        if let Some(app_logger) = &self.app_logger {
            app_logger.flush();
        }
    }
}

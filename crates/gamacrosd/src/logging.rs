// Colorized wrappers for logging

use fern::Dispatch;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{OnceLock, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};

use chrono::{Local, TimeZone};

#[inline(always)]
pub(crate) fn format_log(message: &str) -> String {
    let now = cached_now_string();
    format!("[{now}] {message}")
}

#[inline]
fn cached_now_string() -> String {
    static LAST_SECOND: AtomicU64 = AtomicU64::new(0);
    static CACHED: OnceLock<RwLock<String>> = OnceLock::new();

    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or_else(|_| 0);

    let last = LAST_SECOND.load(Ordering::Acquire);
    if last == secs {
        // Fast path: reuse cached formatted timestamp
        return CACHED
            .get_or_init(|| RwLock::new(String::new()))
            .read()
            .expect("timestamp cache poisoned")
            .clone();
    }

    // Slow path: format a new timestamp and update cache
    let formatted = Local
        .timestamp_opt(secs as i64, 0)
        .single()
        .map(|dt| dt.format("%Y.%m.%d %H:%M:%S").to_string())
        .unwrap_or_else(|| String::from("0000.00.00 00:00:00"));

    let lock = CACHED.get_or_init(|| RwLock::new(String::new()));
    *lock.write().expect("timestamp cache poisoned") = formatted.clone();
    LAST_SECOND.store(secs, Ordering::Release);
    formatted
}

#[macro_export]
macro_rules! print_error {
    ($($arg:tt)*) => {
        if log::log_enabled!(log::Level::Error) {
            let __message = $crate::logging::format_log(&format!($($arg)*));
            log::error!("{}", __message.bright_red());
        }
    }
}

#[macro_export]
macro_rules! print_info {
    ($($arg:tt)*) => {
        if log::log_enabled!(log::Level::Info) {
            let __message = $crate::logging::format_log(&format!($($arg)*));
            log::info!("{__message}");
        }
    }
}

#[macro_export]
macro_rules! print_debug {
    ($($arg:tt)*) => {
        if log::log_enabled!(log::Level::Debug) {
            let __message = $crate::logging::format_log(&format!($($arg)*));
            log::debug!("{}", __message.dimmed());
        }
    }
}

#[macro_export]
macro_rules! print_warning {
    ($($arg:tt)*) => {
        if log::log_enabled!(log::Level::Info) {
            let __message = $crate::logging::format_log(&format!($($arg)*));
            log::info!("{}", __message.bright_yellow());
        }
    }
}

/// Setup the logger.
pub fn setup(verbose: bool, no_color: bool) {
    let log_level = if verbose {
        log::LevelFilter::Debug
    } else {
        log::LevelFilter::Info
    };
    Dispatch::new()
        .level(log::LevelFilter::Error) // Hide enigo logs
        .level_for("gamacrosd", log_level)
        .chain(std::io::stdout())
        .apply()
        .expect("Unable to set up logger");

    if no_color {
        colored::control::set_override(false);
    }
}

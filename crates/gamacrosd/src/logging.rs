// Colorized wrappers for logging

use fern::Dispatch;

#[inline(always)]
pub(crate) fn format_log(message: &str) -> String {
    let now = chrono::Local::now().format("%Y.%m.%d %H:%M:%S").to_string();
    format!("[{now}] {message}")
}

#[macro_export]
macro_rules! print_error {
    ($($arg:tt)*) => {
        let message = $crate::logging::format_log(&format!($($arg)*));
        log::error!("{}", message.bright_red());
    }
}

#[macro_export]
macro_rules! print_info {
    ($($arg:tt)*) => {
        let message = $crate::logging::format_log(&format!($($arg)*));
        log::info!("{message}");
    }
}

#[macro_export]
macro_rules! print_debug {
    ($($arg:tt)*) => {
        let message = $crate::logging::format_log(&format!($($arg)*));
        log::debug!("{}", message.dimmed());
    }
}

#[macro_export]
macro_rules! print_warning {
    ($($arg:tt)*) => {
        let message = $crate::logging::format_log(&format!($($arg)*));
        log::info!("{}", message.bright_yellow());
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

// Colorized wrappers for logging

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

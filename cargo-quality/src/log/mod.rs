// Log module

use log::LevelFilter;
use simple_logger::SimpleLogger;
//use light_log;
// ///
// #[macro_export]
// macro_rules! log_info {
//     // info!(target: "my_target", key1 = 42, key2 = true; "a {} event", "log")
//     // info!(target: "my_target", "a {} event", "log")
//     (target: $target:expr, $($arg:tt)+) => (light_log::info!(target: $target, $($arg)+));

//     // info!("a {} event", "log")
//     ($($arg:tt)+) => (light_log::info!($($arg)+))
// }

// #[macro_export]
// macro_rules! log_warn {
//     // warn!(target: "my_target", key1 = 42, key2 = true; "a {} event", "log")
//     // warn!(target: "my_target", "a {} event", "log")
//     (target: $target:expr, $($arg:tt)+) => (light_log::warn!(target: $target, $($arg)+));

//     // warn!("a {} event", "log")
//     ($($arg:tt)+) => (light_log::warn!($($arg)+))
// }

// pub(crate) use log_info;
// pub(crate) use log_warn;

pub fn simple_logger_init() {
    SimpleLogger::new()
        .with_level(LevelFilter::Off)
        .with_module_level("cargo_quality", LevelFilter::Info)
        .init()
        .unwrap();
}

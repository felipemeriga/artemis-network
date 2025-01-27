use env_logger::Builder;
use log::LevelFilter;
use std::io::Write;

#[macro_export]
macro_rules! server_info {
    ($($arg:tt)*) => {
        info!(target: "server", "{}", format_args!($($arg)*));
    };
}

#[macro_export]
macro_rules! server_error {
    ($($arg:tt)*) => {
        error!(target: "server", "{}", format_args!($($arg)*));
    };
}

#[macro_export]
macro_rules! server_warn {
    ($($arg:tt)*) => {
        warn!(target: "server", "{}", format_args!($($arg)*));
    };
}

#[macro_export]
macro_rules! miner_info {
    ($($arg:tt)*) => {
        info!(target: "miner", "{}", format_args!($($arg)*));
    };
}

pub fn init_logger() {
    Builder::new()
        .format(|buf, record| {
            // Prepend prefix based on the log target
            let prefix = match record.target() {
                "server" => "[SERVER]",
                "miner" => "[MINER]",
                _ => "[GENERAL]", // Default prefix
            };
            writeln!(
                buf,
                "{} [{}] {}",
                prefix,
                chrono::Utc::now().format("%Y-%m-%d %H:%M:%S"),
                record.args()
            )
        })
        .filter(None, LevelFilter::Info) // Default log level
        .init();
}

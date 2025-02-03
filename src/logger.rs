use env_logger::Builder;
use log::LevelFilter;
use std::io::Write;

#[macro_export]
macro_rules! node_info {
    ($($arg:tt)*) => {
        info!(target: "node", "{}", format_args!($($arg)*))
    };
}

#[macro_export]
macro_rules! server_info {
    ($($arg:tt)*) => {
        info!(target: "server", "{}", format_args!($($arg)*));
    };
}

#[macro_export]
macro_rules! server_error {
    ($($arg:tt)*) => {
        error!(target: "server", "{}", format_args!($($arg)*))
    };
}

#[macro_export]
macro_rules! server_warn {
    ($($arg:tt)*) => {
        warn!(target: "server", "{}", format_args!($($arg)*))
    };
}

#[macro_export]
macro_rules! miner_info {
    ($($arg:tt)*) => {
        info!(target: "miner", "{}", format_args!($($arg)*))
    };
}

#[macro_export]
macro_rules! miner_warn {
    ($($arg:tt)*) => {
        warn!(target: "miner", "{}", format_args!($($arg)*))
    };
}

#[macro_export]
macro_rules! miner_error {
    ($($arg:tt)*) => {
        error!(target: "miner", "{}", format_args!($($arg)*))
    };
}

#[macro_export]
macro_rules! broadcaster_info {
    ($($arg:tt)*) => {
        info!(target: "broadcaster", "{}", format_args!($($arg)*))
    };
}

#[macro_export]
macro_rules! broadcaster_error {
    ($($arg:tt)*) => {
        error!(target: "broadcaster", "{}", format_args!($($arg)*))
    };
}

#[macro_export]
macro_rules! broadcaster_warn {
    ($($arg:tt)*) => {
        warn!(target: "broadcaster", "{}", format_args!($($arg)*))
    };
}

#[macro_export]
macro_rules! sync_info {
    ($($arg:tt)*) => {
        info!(target: "sync", "{}", format_args!($($arg)*))
    };
}

#[macro_export]
macro_rules! sync_error {
    ($($arg:tt)*) => {
        error!(target: "sync", "{}", format_args!($($arg)*))
    };
}

#[macro_export]
macro_rules! sync_warn {
    ($($arg:tt)*) => {
        warn!(target: "sync", "{}", format_args!($($arg)*))
    };
}

pub fn init_logger() {
    Builder::new()
        .filter(None, LevelFilter::Debug) // Keep all debug logs
        .filter_module("actix_web", LevelFilter::Off) // Suppress Actix logs
        .filter_module("actix_server", LevelFilter::Off) // Suppress Actix server logs
        .format(|buf, record| {
            // Prepend prefix based on the log target
            let prefix = match record.target() {
                "node" => "[NODE]",
                "server" => "[SERVER]",
                "miner" => "[MINER]",
                "broadcaster" => "[BROADCASTER]",
                "sync" => "[SYNC]",
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

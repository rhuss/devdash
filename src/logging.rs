use std::path::{Path, PathBuf};

use tracing_appender::non_blocking::WorkerGuard;
use tracing_appender::rolling;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::fmt;
use tracing_subscriber::prelude::*;

pub fn init(log_path: &Path, level: &str) -> WorkerGuard {
    let parent = log_path.parent().unwrap_or_else(|| Path::new("."));
    let filename = log_path
        .file_name()
        .unwrap_or_else(|| std::ffi::OsStr::new("devdash.log"));

    std::fs::create_dir_all(parent).ok();

    let file_appender = rolling::daily(parent, filename);
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    let filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new(level))
        .unwrap_or_else(|_| EnvFilter::new("info"));

    tracing_subscriber::registry()
        .with(fmt::layer().with_writer(non_blocking).with_ansi(false))
        .with(filter)
        .init();

    guard
}

pub fn log_path() -> PathBuf {
    directories::ProjectDirs::from("", "", "devdash").map_or_else(
        || PathBuf::from("devdash.log"),
        |dirs| {
            dirs.state_dir().map_or_else(
                || dirs.data_dir().join("devdash.log"),
                |s| s.join("devdash.log"),
            )
        },
    )
}

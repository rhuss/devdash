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

    cleanup_old_logs(parent, 7);

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

fn cleanup_old_logs(dir: &Path, max_files: usize) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let mut log_files: Vec<_> = entries
        .filter_map(std::result::Result::ok)
        .filter(|e| {
            e.file_name()
                .to_str()
                .is_some_and(|n| n.starts_with("devdash.log"))
        })
        .collect();
    if log_files.len() <= max_files {
        return;
    }
    log_files.sort_by_key(|e| std::cmp::Reverse(e.metadata().and_then(|m| m.modified()).ok()));
    for old in &log_files[max_files..] {
        let _ = std::fs::remove_file(old.path());
    }
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

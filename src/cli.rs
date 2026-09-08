use std::path::PathBuf;

use clap::Parser;

#[derive(Parser)]
#[command(name = "devdash", version, about = "GitHub pull request dashboard")]
pub struct Cli {
    #[arg(
        long,
        value_name = "PATH",
        num_args = 0..=1,
        default_missing_value = "fixtures/snapshot.json",
        help = "Use fixture data source instead of live API"
    )]
    pub fixtures: Option<PathBuf>,

    #[arg(long, value_name = "PATH", help = "Alternate configuration file")]
    pub config: Option<PathBuf>,

    #[arg(
        long,
        value_name = "LEVEL",
        default_value = "info",
        help = "Log file verbosity"
    )]
    pub log_level: String,

    #[arg(
        long,
        value_name = "SECS",
        help = "Override refresh interval for this run"
    )]
    pub refresh_interval: Option<u64>,

    #[arg(long, help = "Print log file path and exit")]
    pub print_log_path: bool,
}

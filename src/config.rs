use std::collections::HashSet;
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::domain::repository::{RepoId, TrackedRepo};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_refresh_interval")]
    pub refresh_interval_secs: u64,
    #[serde(default)]
    pub tracked: Vec<TrackedRepo>,
}

const fn default_refresh_interval() -> u64 {
    300
}

impl Default for Config {
    fn default() -> Self {
        Self {
            refresh_interval_secs: 300,
            tracked: Vec::new(),
        }
    }
}

pub fn config_path(override_path: Option<&Path>) -> PathBuf {
    override_path.map_or_else(
        || {
            directories::ProjectDirs::from("", "", "devdash").map_or_else(
                || PathBuf::from("devdash.toml"),
                |dirs| dirs.config_dir().join("config.toml"),
            )
        },
        Path::to_path_buf,
    )
}

pub fn load(path: &Path) -> Config {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Config::default(),
        Err(e) => {
            tracing::warn!("Could not read config at {}: {e}", path.display());
            return Config::default();
        }
    };

    match toml::from_str::<Config>(&content) {
        Ok(mut config) => {
            validate(&mut config);
            config
        }
        Err(e) => {
            tracing::error!(
                "Config at {} is not valid TOML: {e}. Starting with empty tracked set. \
                 The file will NOT be overwritten.",
                path.display()
            );
            Config::default()
        }
    }
}

fn validate(config: &mut Config) {
    if config.refresh_interval_secs < 30 {
        tracing::warn!(
            "refresh_interval_secs {} is below minimum 30, clamping",
            config.refresh_interval_secs
        );
        config.refresh_interval_secs = 30;
    }

    let mut seen = HashSet::new();
    config.tracked.retain(|t| {
        if seen.contains(&t.id) {
            tracing::warn!("Duplicate tracked repo id {}, discarding", t.id.0);
            false
        } else {
            seen.insert(t.id);
            true
        }
    });
}

#[allow(clippy::implicit_hasher)]
pub fn save(config: &Config, path: &Path, untracked_ids: &HashSet<RepoId>) -> anyhow::Result<()> {
    let merged = if path.exists() {
        let on_disk = load(path);
        merge(config, &on_disk, untracked_ids)
    } else {
        config.clone()
    };

    let serialized = toml::to_string_pretty(&merged)?;

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let mut tmp = tempfile::NamedTempFile::new_in(path.parent().unwrap_or_else(|| Path::new(".")))?;
    tmp.write_all(serialized.as_bytes())?;
    tmp.flush()?;
    tmp.persist(path)?;

    Ok(())
}

fn merge(in_memory: &Config, on_disk: &Config, untracked_ids: &HashSet<RepoId>) -> Config {
    let mut result_tracked: Vec<TrackedRepo> = Vec::new();
    let mut seen_ids: HashSet<RepoId> = HashSet::new();

    for repo in &in_memory.tracked {
        if !untracked_ids.contains(&repo.id) && seen_ids.insert(repo.id) {
            result_tracked.push(repo.clone());
        }
    }

    for repo in &on_disk.tracked {
        if !untracked_ids.contains(&repo.id) && seen_ids.insert(repo.id) {
            result_tracked.push(repo.clone());
        }
    }

    Config {
        refresh_interval_secs: in_memory.refresh_interval_secs,
        tracked: result_tracked,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_has_300s_interval() {
        let config = Config::default();
        assert_eq!(config.refresh_interval_secs, 300);
        assert!(config.tracked.is_empty());
    }

    #[test]
    fn validation_clamps_low_interval() {
        let mut config = Config {
            refresh_interval_secs: 10,
            tracked: Vec::new(),
        };
        validate(&mut config);
        assert_eq!(config.refresh_interval_secs, 30);
    }

    #[test]
    fn validation_deduplicates_tracked() {
        let mut config = Config {
            refresh_interval_secs: 300,
            tracked: vec![
                TrackedRepo {
                    id: RepoId(1),
                    owner: "a".into(),
                    name: "b".into(),
                },
                TrackedRepo {
                    id: RepoId(1),
                    owner: "a".into(),
                    name: "b-dup".into(),
                },
                TrackedRepo {
                    id: RepoId(2),
                    owner: "c".into(),
                    name: "d".into(),
                },
            ],
        };
        validate(&mut config);
        assert_eq!(config.tracked.len(), 2);
    }

    #[test]
    fn merge_unions_by_id() {
        let memory = Config {
            refresh_interval_secs: 300,
            tracked: vec![TrackedRepo {
                id: RepoId(1),
                owner: "a".into(),
                name: "one".into(),
            }],
        };
        let disk = Config {
            refresh_interval_secs: 300,
            tracked: vec![TrackedRepo {
                id: RepoId(2),
                owner: "b".into(),
                name: "two".into(),
            }],
        };
        let result = merge(&memory, &disk, &HashSet::new());
        assert_eq!(result.tracked.len(), 2);
    }

    #[test]
    fn merge_respects_untracked() {
        let memory = Config {
            refresh_interval_secs: 300,
            tracked: vec![TrackedRepo {
                id: RepoId(1),
                owner: "a".into(),
                name: "one".into(),
            }],
        };
        let disk = Config {
            refresh_interval_secs: 300,
            tracked: vec![TrackedRepo {
                id: RepoId(2),
                owner: "b".into(),
                name: "two".into(),
            }],
        };
        let mut untracked = HashSet::new();
        untracked.insert(RepoId(2));
        let result = merge(&memory, &disk, &untracked);
        assert_eq!(result.tracked.len(), 1);
        assert_eq!(result.tracked[0].id, RepoId(1));
    }

    #[test]
    fn merge_memory_wins_for_same_id() {
        let memory = Config {
            refresh_interval_secs: 300,
            tracked: vec![TrackedRepo {
                id: RepoId(1),
                owner: "new-owner".into(),
                name: "new-name".into(),
            }],
        };
        let disk = Config {
            refresh_interval_secs: 300,
            tracked: vec![TrackedRepo {
                id: RepoId(1),
                owner: "old-owner".into(),
                name: "old-name".into(),
            }],
        };
        let result = merge(&memory, &disk, &HashSet::new());
        assert_eq!(result.tracked.len(), 1);
        assert_eq!(result.tracked[0].owner, "new-owner");
    }
}

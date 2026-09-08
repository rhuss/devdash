use std::collections::HashSet;

use devdash::config;
use devdash::domain::repository::{RepoId, TrackedRepo};

#[test]
fn load_missing_file_returns_default() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("nonexistent.toml");
    let config = config::load(&path);
    assert_eq!(config.refresh_interval_secs, 300);
    assert!(config.tracked.is_empty());
}

#[test]
fn load_valid_file_parses_tracked() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    std::fs::write(
        &path,
        r#"
refresh_interval_secs = 120

[[tracked]]
id = 42
owner = "acme"
name = "api"
"#,
    )
    .unwrap();

    let config = config::load(&path);
    assert_eq!(config.refresh_interval_secs, 120);
    assert_eq!(config.tracked.len(), 1);
    assert_eq!(config.tracked[0].id, RepoId(42));
}

#[test]
fn load_unparseable_file_returns_default_and_preserves_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("bad.toml");
    let bad_content = "this is [[[ not valid toml";
    std::fs::write(&path, bad_content).unwrap();

    let config = config::load(&path);
    assert_eq!(config.refresh_interval_secs, 300);
    assert!(config.tracked.is_empty());

    let preserved = std::fs::read_to_string(&path).unwrap();
    assert_eq!(
        preserved, bad_content,
        "FR-025: file must not be overwritten"
    );
}

#[test]
fn save_creates_file_atomically() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");

    let config = config::Config {
        refresh_interval_secs: 300,
        tracked: vec![TrackedRepo {
            id: RepoId(1),
            owner: "acme".into(),
            name: "api".into(),
        }],
    };

    config::save(&config, &path, &HashSet::new()).unwrap();
    assert!(path.exists());

    let reloaded = config::load(&path);
    assert_eq!(reloaded.tracked.len(), 1);
    assert_eq!(reloaded.tracked[0].id, RepoId(1));
}

#[test]
fn save_merges_with_existing() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");

    std::fs::write(
        &path,
        r#"
refresh_interval_secs = 300

[[tracked]]
id = 1
owner = "acme"
name = "api"
"#,
    )
    .unwrap();

    let in_memory = config::Config {
        refresh_interval_secs: 300,
        tracked: vec![TrackedRepo {
            id: RepoId(2),
            owner: "acme".into(),
            name: "web".into(),
        }],
    };

    config::save(&in_memory, &path, &HashSet::new()).unwrap();

    let reloaded = config::load(&path);
    assert_eq!(reloaded.tracked.len(), 2, "both repos should survive merge");
}

#[test]
fn save_respects_untracked_ids() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");

    std::fs::write(
        &path,
        r#"
[[tracked]]
id = 1
owner = "acme"
name = "api"

[[tracked]]
id = 2
owner = "acme"
name = "web"
"#,
    )
    .unwrap();

    let in_memory = config::Config {
        refresh_interval_secs: 300,
        tracked: vec![TrackedRepo {
            id: RepoId(1),
            owner: "acme".into(),
            name: "api".into(),
        }],
    };

    let mut untracked = HashSet::new();
    untracked.insert(RepoId(2));

    config::save(&in_memory, &path, &untracked).unwrap();

    let reloaded = config::load(&path);
    assert_eq!(reloaded.tracked.len(), 1);
    assert_eq!(reloaded.tracked[0].id, RepoId(1));
}

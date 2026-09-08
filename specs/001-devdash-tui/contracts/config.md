# Contract: Configuration file

**Feature**: [../spec.md](../spec.md) | **Data model**: [../data-model.md](../data-model.md)

## Location

Resolved by the `directories` crate's project config directory (R7):

| Platform | Path |
|---|---|
| Linux | `$XDG_CONFIG_HOME/devdash/config.toml`, else `~/.config/devdash/config.toml` |
| macOS | `~/Library/Application Support/devdash/config.toml` |

Overridable with `--config PATH` (see [cli.md](./cli.md)).

## Format

TOML, chosen so the file is readable and hand-editable, which FR-021 requires.

```toml
# devdash configuration
# Managed by the settings screen. Safe to edit by hand while devdash is not running.

refresh_interval_secs = 300

[[tracked]]
id = 600886023
owner = "ratatui"
name = "ratatui"

[[tracked]]
id = 17420913
owner = "rust-lang"
name = "cargo"
```

## Fields

| Field | Type | Required | Default | Requirement |
|---|---|---|---|---|
| `refresh_interval_secs` | integer | no | 300 | FR-046 |
| `tracked` | array of tables | no | empty | FR-021 |
| `tracked[].id` | integer | yes | | FR-021, merge key. Deserializes into `RepoId`, a transparent newtype, so the file holds a plain integer. |
| `tracked[].owner` | string | yes | | FR-022, display and query |
| `tracked[].name` | string | yes | | FR-022, display and query |

`id` is GitHub's `databaseId` (R3). It is the identity; `owner` and `name` are
display attributes that devdash rewrites when a repository is renamed (FR-022).

An empty or absent `tracked` array is the valid first-run state, and produces the
empty dashboard that names the settings key (FR-021, US2 scenario 7). It is not an
error.

## Validation

| Rule | On violation | Requirement |
|---|---|---|
| File parses as TOML | Report path and the parse error, start with an empty tracked set, do **not** overwrite the file | FR-025 |
| `tracked[].id` unique | Discard the later duplicate, log a warning | |
| `refresh_interval_secs` >= 30 | Clamp to 30, log a warning | FR-046 |
| Unknown keys present | Preserved on rewrite where possible, ignored otherwise | |

FR-025's "do not overwrite" is the important one. A user who has hand-edited the
file into an invalid state must get it back, so devdash refuses to write until the
user makes a deliberate change in the settings screen.

## Write protocol

Every write follows this sequence, implementing FR-023 and FR-024.

1. **Stat**: read the target's modification time and content hash, recorded when
   the file was last loaded.
2. **Detect**: if either differs, another instance has written since. Re-read the
   file and merge (below). Otherwise use the in-memory set as-is.
3. **Serialize** the merged configuration.
4. **Write** to a temporary file in the *same directory* as the target. Same
   directory matters: rename is only atomic within one filesystem.
5. **Rename** the temporary file over the target. This is atomic, so a crash leaves
   either the complete old file or the complete new one, never a truncated one
   (FR-024).
6. **Re-record** the new modification time and hash.

### Merge rule

Keyed on `id`, so a rename on either side cannot cause a duplicate.

| Present on disk | Present in memory | Untracked this session | Result |
|---|---|---|---|
| yes | yes | no | Keep, with this instance's owner/name |
| yes | no | no | Keep. Another instance added it. |
| yes | any | yes | Remove. This instance untracked it deliberately. |
| no | yes | no | Keep. This instance added it. |

The "untracked this session" column is why removal cannot be inferred from absence.
Without it, an entry another instance added would be indistinguishable from one this
instance removed, and FR-023 would be unsatisfiable.

`refresh_interval_secs` is not merged. The on-disk value wins unless this instance
changed it, since devdash offers no way to change it in the interface today.

## Concurrency guarantees

| Scenario | Outcome | Requirement |
|---|---|---|
| Two instances each track a different repository | Both survive | FR-023 |
| Two instances write simultaneously | Both files are complete; the later write includes the earlier | FR-023, FR-024 |
| Instance killed mid-write | Target file is the complete previous version | FR-024 |
| User hand-edits while devdash runs | Edit is merged on the next write, not clobbered | FR-023 |

There is no lock file. A stale lock after a crash would block the user, and the
merge is cheap enough to make locking unnecessary.

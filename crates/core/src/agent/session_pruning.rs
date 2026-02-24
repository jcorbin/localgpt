//! Session pruning - automatic cleanup of old session files
//!
//! Sessions accumulate over time in `~/.localgpt/agents/{id}/sessions/`.
//! This module provides automatic cleanup based on age and count limits.

use anyhow::Result;
use std::fs;
use std::path::Path;
use std::time::{Duration, SystemTime};
use tracing::{debug, info};

/// Result of a pruning operation
#[derive(Debug, Clone, Default)]
pub struct PruneResult {
    /// Number of sessions deleted
    pub deleted: usize,
    /// Total bytes freed
    pub freed_bytes: u64,
}

/// Session file info for pruning decisions
#[derive(PartialEq)]
struct SessionFileInfo {
    path: std::path::PathBuf,
    modified: SystemTime,
    size: u64,
}

/// Prune old sessions for a given agent.
///
/// Deletes sessions exceeding max_age or max_count (oldest first).
/// Returns count of deleted sessions and bytes freed.
pub fn prune_sessions(
    state_dir: &Path,
    agent_id: &str,
    max_age: Option<Duration>,
    max_count: Option<usize>,
) -> Result<PruneResult> {
    let sessions_dir = state_dir.join("agents").join(agent_id).join("sessions");

    if !sessions_dir.exists() {
        return Ok(PruneResult::default());
    }

    // Collect all session files with metadata
    let mut sessions: Vec<SessionFileInfo> = Vec::new();

    for entry in fs::read_dir(&sessions_dir)? {
        let entry = entry?;
        let path = entry.path();

        // Only process .jsonl files
        if path.extension().map(|e| e == "jsonl").unwrap_or(false) {
            let metadata = entry.metadata()?;
            let modified = metadata.modified()?;
            let size = metadata.len();

            sessions.push(SessionFileInfo {
                path,
                modified,
                size,
            });
        }
    }

    if sessions.is_empty() {
        return Ok(PruneResult::default());
    }

    // Sort by modification time (oldest first)
    sessions.sort_by_key(|s| s.modified);

    let now = SystemTime::now();
    let mut to_delete: Vec<&SessionFileInfo> = Vec::new();

    // Mark sessions for deletion based on age
    if let Some(age) = max_age {
        for session in &sessions {
            if let Ok(elapsed) = now.duration_since(session.modified)
                && elapsed > age
            {
                to_delete.push(session);
            }
        }
    }

    // Mark additional sessions for deletion based on count
    if let Some(max) = max_count {
        let remaining_count = sessions.len() - to_delete.len();
        if remaining_count > max {
            // Delete oldest sessions until we're under the limit
            let excess = remaining_count - max;
            let mut deleted_from_remaining = 0;

            for session in &sessions {
                if !to_delete.contains(&session) {
                    to_delete.push(session);
                    deleted_from_remaining += 1;
                    if deleted_from_remaining >= excess {
                        break;
                    }
                }
            }
        }
    }

    // Delete marked sessions
    let mut result = PruneResult::default();

    for session in to_delete {
        match fs::remove_file(&session.path) {
            Ok(()) => {
                result.deleted += 1;
                result.freed_bytes += session.size;
                debug!(
                    "Deleted session: {} ({} bytes)",
                    session.path.display(),
                    session.size
                );
            }
            Err(e) => {
                debug!("Failed to delete session {}: {}", session.path.display(), e);
            }
        }
    }

    if result.deleted > 0 {
        info!(
            "Pruned {} sessions for agent '{}', freed {} bytes",
            result.deleted, agent_id, result.freed_bytes
        );
    }

    Ok(result)
}

/// Prune sessions for all agents in the state directory.
pub fn prune_all_agents(
    state_dir: &Path,
    max_age: Option<Duration>,
    max_count: Option<usize>,
) -> Result<PruneResult> {
    let agents_dir = state_dir.join("agents");

    if !agents_dir.exists() {
        return Ok(PruneResult::default());
    }

    let mut total_result = PruneResult::default();

    for entry in fs::read_dir(&agents_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir()
            && let Some(agent_id) = path.file_name().and_then(|n| n.to_str())
        {
            let result = prune_sessions(state_dir, agent_id, max_age, max_count)?;
            total_result.deleted += result.deleted;
            total_result.freed_bytes += result.freed_bytes;
        }
    }

    Ok(total_result)
}

/// Preview what would be deleted without actually deleting.
pub fn preview_prune(
    state_dir: &Path,
    agent_id: &str,
    max_age: Option<Duration>,
    max_count: Option<usize>,
) -> Result<Vec<(std::path::PathBuf, u64)>> {
    let sessions_dir = state_dir.join("agents").join(agent_id).join("sessions");

    if !sessions_dir.exists() {
        return Ok(Vec::new());
    }

    // Collect all session files with metadata
    let mut sessions: Vec<SessionFileInfo> = Vec::new();

    for entry in fs::read_dir(&sessions_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.extension().map(|e| e == "jsonl").unwrap_or(false) {
            let metadata = entry.metadata()?;
            let modified = metadata.modified()?;
            let size = metadata.len();

            sessions.push(SessionFileInfo {
                path,
                modified,
                size,
            });
        }
    }

    if sessions.is_empty() {
        return Ok(Vec::new());
    }

    sessions.sort_by_key(|s| s.modified);

    let now = SystemTime::now();
    let mut to_delete: Vec<(std::path::PathBuf, u64)> = Vec::new();
    let mut deleted_set: Vec<usize> = Vec::new();

    // Mark sessions for deletion based on age
    if let Some(age) = max_age {
        for (idx, session) in sessions.iter().enumerate() {
            if let Ok(elapsed) = now.duration_since(session.modified)
                && elapsed > age
            {
                to_delete.push((session.path.clone(), session.size));
                deleted_set.push(idx);
            }
        }
    }

    // Mark additional sessions for deletion based on count
    if let Some(max) = max_count {
        let remaining_count = sessions.len() - deleted_set.len();
        if remaining_count > max {
            let excess = remaining_count - max;
            let mut deleted_from_remaining = 0;

            for (idx, session) in sessions.iter().enumerate() {
                if !deleted_set.contains(&idx) {
                    to_delete.push((session.path.clone(), session.size));
                    deleted_from_remaining += 1;
                    if deleted_from_remaining >= excess {
                        break;
                    }
                }
            }
        }
    }

    Ok(to_delete)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn test_prune_by_age() {
        let temp_dir = TempDir::new().unwrap();
        let state_dir = temp_dir.path();
        let sessions_dir = state_dir.join("agents/test-agent/sessions");
        fs::create_dir_all(&sessions_dir).unwrap();

        // Create a session file
        let session_path = sessions_dir.join("old-session.jsonl");
        let mut file = File::create(&session_path).unwrap();
        file.write_all(b"test content").unwrap();
        drop(file);

        // Set modification time to 31 days ago
        let old_time = SystemTime::now() - Duration::from_secs(31 * 24 * 60 * 60);
        filetime::set_file_mtime(
            &session_path,
            filetime::FileTime::from_system_time(old_time),
        )
        .unwrap();

        // Prune with 30-day max age
        let result = prune_sessions(
            state_dir,
            "test-agent",
            Some(Duration::from_secs(30 * 24 * 60 * 60)),
            None,
        )
        .unwrap();

        assert_eq!(result.deleted, 1);
        assert!(!session_path.exists());
    }

    #[test]
    fn test_prune_by_count() {
        let temp_dir = TempDir::new().unwrap();
        let state_dir = temp_dir.path();
        let sessions_dir = state_dir.join("agents/test-agent/sessions");
        fs::create_dir_all(&sessions_dir).unwrap();

        // Create 5 session files
        for i in 0..5 {
            let session_path = sessions_dir.join(format!("session-{}.jsonl", i));
            let mut file = File::create(&session_path).unwrap();
            file.write_all(b"test").unwrap();
            drop(file);
            // Small delay to ensure different mtimes
            std::thread::sleep(std::time::Duration::from_millis(10));
        }

        // Prune to keep only 3
        let result = prune_sessions(state_dir, "test-agent", None, Some(3)).unwrap();

        assert_eq!(result.deleted, 2);

        // Verify newest 3 remain
        assert!(!sessions_dir.join("session-0.jsonl").exists());
        assert!(!sessions_dir.join("session-1.jsonl").exists());
        assert!(sessions_dir.join("session-2.jsonl").exists());
        assert!(sessions_dir.join("session-3.jsonl").exists());
        assert!(sessions_dir.join("session-4.jsonl").exists());
    }
}

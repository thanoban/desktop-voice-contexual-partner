//! Retention limits for conversation memories and transcript history, plus
//! explicit per-session history deletion.
//!
//! Purges bump the affected projects' write revisions inside the same
//! transaction, so summaries already in flight cannot recreate purged rows.
//! Document chunks are user-managed and are never expired here.

use rusqlite::Connection;
use serde::{Deserialize, Serialize};

pub const MEMORY_RETENTION_KEY: &str = "memory_retention_days";
pub const HISTORY_RETENTION_KEY: &str = "history_retention_days";
/// Upper bound accepted for either limit (~100 years).
pub const MAX_RETENTION_DAYS: u32 = 36_500;
const DAY_MS: i64 = 86_400_000;
/// Mirrors `db::ensure_session`: an open session started this recently is
/// still the reusable conversation and cannot be deleted from history.
const ACTIVE_SESSION_WINDOW_MS: i64 = 4 * 60 * 60 * 1000;

/// Day limits; 0 keeps rows forever.
#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Default)]
pub struct RetentionSettings {
    pub memory_days: u32,
    pub history_days: u32,
}

#[derive(Debug, Serialize, Clone, Copy, PartialEq, Eq, Default)]
pub struct PurgeReport {
    pub memories_deleted: usize,
    pub sessions_deleted: usize,
    pub turns_deleted: usize,
}

fn read_days(conn: &Connection, key: &str) -> u32 {
    // Unreadable values fail safe to "keep forever" rather than deleting.
    crate::db::get_setting(conn, key)
        .and_then(|value| value.trim().parse::<u32>().ok())
        .filter(|days| *days <= MAX_RETENTION_DAYS)
        .unwrap_or(0)
}

pub fn settings(conn: &Connection) -> RetentionSettings {
    RetentionSettings {
        memory_days: read_days(conn, MEMORY_RETENTION_KEY),
        history_days: read_days(conn, HISTORY_RETENTION_KEY),
    }
}

pub fn save_settings(conn: &Connection, settings: &RetentionSettings) -> Result<(), String> {
    if settings.memory_days > MAX_RETENTION_DAYS || settings.history_days > MAX_RETENTION_DAYS {
        return Err(format!(
            "Retention must be between 0 and {MAX_RETENTION_DAYS} days"
        ));
    }
    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
    crate::db::set_setting(&tx, MEMORY_RETENTION_KEY, &settings.memory_days.to_string())
        .map_err(|e| e.to_string())?;
    crate::db::set_setting(
        &tx,
        HISTORY_RETENTION_KEY,
        &settings.history_days.to_string(),
    )
    .map_err(|e| e.to_string())?;
    tx.commit().map_err(|e| e.to_string())
}

fn cutoff(now_ms: i64, days: u32) -> i64 {
    now_ms.saturating_sub(i64::from(days).saturating_mul(DAY_MS))
}

/// Deletes conversation memories and transcript sessions older than the
/// configured limits, across all projects. Lexical index rows are removed by
/// the `memories` delete trigger; vectors live on the deleted rows themselves.
pub fn purge_expired(conn: &Connection, now_ms: i64) -> rusqlite::Result<PurgeReport> {
    let limits = settings(conn);
    let mut report = PurgeReport::default();
    if limits.memory_days == 0 && limits.history_days == 0 {
        return Ok(report);
    }
    let tx = conn.unchecked_transaction()?;
    let mut projects: Vec<String> = Vec::new();

    if limits.memory_days > 0 {
        let before = cutoff(now_ms, limits.memory_days);
        let mut stmt = tx.prepare(
            "SELECT DISTINCT project_id FROM memories WHERE memory_type != 'document' AND created_at < ?1",
        )?;
        let affected = stmt
            .query_map([before], |row| row.get::<_, String>(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        drop(stmt);
        projects.extend(affected);
        report.memories_deleted = tx.execute(
            "DELETE FROM memories WHERE memory_type != 'document' AND created_at < ?1",
            [before],
        )?;
    }

    if limits.history_days > 0 {
        let before = cutoff(now_ms, limits.history_days);
        // A session expires once its latest activity is older than the cutoff.
        let mut stmt = tx.prepare(
            "SELECT s.id, s.project_id FROM sessions s
             WHERE MAX(s.started_at, COALESCE(s.ended_at, s.started_at),
                       COALESCE((SELECT MAX(t.created_at) FROM turns t WHERE t.session_id = s.id), s.started_at)) < ?1",
        )?;
        let expired = stmt
            .query_map([before], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        drop(stmt);
        for (session_id, project_id) in expired {
            report.turns_deleted +=
                tx.execute("DELETE FROM turns WHERE session_id = ?1", [&session_id])?;
            report.sessions_deleted +=
                tx.execute("DELETE FROM sessions WHERE id = ?1", [&session_id])?;
            projects.push(project_id);
        }
    }

    projects.sort();
    projects.dedup();
    for project in &projects {
        super::invalidate_project(&tx, project)?;
    }
    tx.commit()?;
    Ok(report)
}

/// Deletes one session's transcript in the given project. Memories derived from
/// it are a separate explicit deletion and are kept.
pub fn delete_session_history(
    conn: &Connection,
    project_id: &str,
    session_id: &str,
    now_ms: i64,
) -> Result<usize, String> {
    use rusqlite::OptionalExtension;
    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
    let session: Option<(i64, Option<i64>)> = tx
        .query_row(
            "SELECT started_at, ended_at FROM sessions WHERE id = ?1 AND project_id = ?2",
            rusqlite::params![session_id, project_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .map_err(|e| e.to_string())?;
    let Some((started_at, ended_at)) = session else {
        return Err("Conversation not found in this workspace".into());
    };
    if ended_at.is_none() && started_at > now_ms - ACTIVE_SESSION_WINDOW_MS {
        return Err("Start a new conversation before deleting the current one".into());
    }
    let turns = tx
        .execute("DELETE FROM turns WHERE session_id = ?1", [session_id])
        .map_err(|e| e.to_string())?;
    tx.execute("DELETE FROM sessions WHERE id = ?1", [session_id])
        .map_err(|e| e.to_string())?;
    // A summary job admitted for this session must not write after deletion.
    super::invalidate_project(&tx, project_id).map_err(|e| e.to_string())?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(turns)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::{self, retrieval};

    fn database() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        crate::db::migrations::run(&conn).unwrap();
        conn
    }

    fn fts_hits(conn: &Connection, term: &str) -> i64 {
        conn.query_row(
            "SELECT COUNT(*) FROM memory_fts WHERE memory_fts MATCH ?1",
            [format!("\"{term}\"")],
            |row| row.get(0),
        )
        .unwrap()
    }

    #[test]
    fn settings_default_to_keep_forever_and_reject_out_of_range() {
        let conn = database();
        assert_eq!(settings(&conn), RetentionSettings::default());
        crate::db::set_setting(&conn, MEMORY_RETENTION_KEY, "not a number").unwrap();
        assert_eq!(settings(&conn).memory_days, 0);
        let limits = RetentionSettings {
            memory_days: 30,
            history_days: 7,
        };
        save_settings(&conn, &limits).unwrap();
        assert_eq!(settings(&conn), limits);
        assert!(save_settings(
            &conn,
            &RetentionSettings {
                memory_days: MAX_RETENTION_DAYS + 1,
                history_days: 0
            }
        )
        .is_err());
        assert_eq!(purge_expired(&conn, 0).unwrap(), PurgeReport::default());
    }

    #[test]
    fn purge_removes_expired_rows_from_all_indexes_and_blocks_pending_writes() {
        let conn = database();
        let now = crate::domain::now_ms();
        let session = crate::db::ensure_session(&conn).unwrap();
        let guard = memory::summary_guard(&conn, "personal").unwrap();
        assert!(memory::store_summary_checked(
            &conn,
            &guard,
            &session,
            "expired walrus summary",
            &[1.0, 0.0]
        )
        .unwrap());
        conn.execute("UPDATE memories SET created_at = ?1", [now - 40 * DAY_MS])
            .unwrap();
        memory::store_document_chunk(&conn, "walrus manual", &[1.0, 0.0], "doc.txt", "personal")
            .unwrap();
        let pending = memory::summary_guard(&conn, "personal").unwrap();

        save_settings(
            &conn,
            &RetentionSettings {
                memory_days: 30,
                history_days: 0,
            },
        )
        .unwrap();
        let report = purge_expired(&conn, now).unwrap();
        assert_eq!(report.memories_deleted, 1);
        assert_eq!(fts_hits(&conn, "expired"), 0);
        // Documents are not subject to memory retention.
        assert_eq!(fts_hits(&conn, "manual"), 1);
        let index_rows: i64 = conn
            .query_row("SELECT COUNT(*) FROM memory_fts_rows", [], |row| row.get(0))
            .unwrap();
        assert_eq!(index_rows, 1);
        let vectors = retrieval::vector_candidates(
            &conn,
            "personal",
            &memory::current_embedding_model(&conn),
            &[1.0, 0.0],
            10,
            -1.0,
        );
        assert_eq!(vectors.len(), 1);

        assert!(!memory::store_summary_checked(
            &conn,
            &pending,
            &session,
            "expired walrus summary",
            &[1.0, 0.0]
        )
        .unwrap());
        let fresh = memory::summary_guard(&conn, "personal").unwrap();
        assert!(
            memory::store_summary_checked(&conn, &fresh, &session, "new", &[1.0, 0.0]).unwrap()
        );
    }

    #[test]
    fn history_purge_and_session_delete_respect_activity_and_scope() {
        let conn = database();
        let now = crate::domain::now_ms();
        let old = crate::db::create_session(&conn).unwrap();
        crate::db::save_turn(&conn, &old, "user", "old words").unwrap();
        conn.execute(
            "UPDATE sessions SET started_at = ?1, ended_at = ?1 WHERE id = ?2",
            rusqlite::params![now - 20 * DAY_MS, old],
        )
        .unwrap();
        conn.execute(
            "UPDATE turns SET created_at = ?1 WHERE session_id = ?2",
            rusqlite::params![now - 20 * DAY_MS, old],
        )
        .unwrap();
        // Started long ago but still active recently: kept.
        let long_running = crate::db::create_session(&conn).unwrap();
        crate::db::save_turn(&conn, &long_running, "user", "recent words").unwrap();
        conn.execute(
            "UPDATE sessions SET started_at = ?1 WHERE id = ?2",
            rusqlite::params![now - 20 * DAY_MS, long_running],
        )
        .unwrap();

        save_settings(
            &conn,
            &RetentionSettings {
                memory_days: 0,
                history_days: 7,
            },
        )
        .unwrap();
        let report = purge_expired(&conn, now).unwrap();
        assert_eq!(report.sessions_deleted, 1);
        assert_eq!(report.turns_deleted, 1);
        assert!(crate::db::get_all_turns(&conn, &old).is_empty());
        assert_eq!(crate::db::get_all_turns(&conn, &long_running).len(), 1);

        // The current reusable conversation is protected; ended ones are not.
        let current = crate::db::ensure_session(&conn).unwrap();
        assert!(delete_session_history(&conn, "personal", &current, now).is_err());
        assert!(delete_session_history(&conn, "other", &long_running, now).is_err());
        let pending = memory::summary_guard(&conn, "personal").unwrap();
        assert_eq!(
            delete_session_history(&conn, "personal", &long_running, now).unwrap(),
            1
        );
        assert!(!memory::store_summary_checked(&conn, &pending, &current, "late", &[1.0]).unwrap());
    }
}

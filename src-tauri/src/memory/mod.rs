use rusqlite::Connection;
use serde::Serialize;
use std::cmp::{Ordering, Reverse};
use std::collections::BinaryHeap;

#[derive(Debug, Serialize, Clone)]
pub struct Memory {
    pub id: String,
    pub session_id: Option<String>,
    pub content: String,
    pub memory_type: String,
    pub created_at: i64,
    pub source_file: Option<String>,
    pub project_id: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct DocumentInfo {
    pub source_file: String,
    pub chunk_count: i64,
    pub ingested_at: i64,
}

pub struct MemoryResult {
    pub memory: Memory,
    pub score: f32,
}

// The smallest candidate is evicted first. Ties prefer recent memories, then a
// stable ID, independent of SQLite's scan order.
struct Candidate(MemoryResult);

impl PartialEq for Candidate {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}
impl Eq for Candidate {}
impl PartialOrd for Candidate {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for Candidate {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0
            .score
            .total_cmp(&other.0.score)
            .then_with(|| self.0.memory.created_at.cmp(&other.0.memory.created_at))
            .then_with(|| self.0.memory.id.cmp(&other.0.memory.id))
    }
}

// ── Embedding serialization ──────────────────────────────────────────────────

pub fn encode_embedding(v: &[f32]) -> Vec<u8> {
    v.iter().flat_map(|f| f.to_le_bytes()).collect()
}

pub fn decode_embedding(bytes: &[u8]) -> Vec<f32> {
    bytes
        .chunks_exact(4)
        .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect()
}

// ── Similarity ───────────────────────────────────────────────────────────────

pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    if a.iter().chain(b).any(|value| !value.is_finite()) {
        return 0.0;
    }
    // Accumulate in f64 so finite f32 components cannot overflow the norms.
    let dot: f64 = a
        .iter()
        .zip(b)
        .map(|(&x, &y)| f64::from(x) * f64::from(y))
        .sum();
    let na: f64 = a.iter().map(|&x| f64::from(x).powi(2)).sum::<f64>().sqrt();
    let nb: f64 = b.iter().map(|&x| f64::from(x).powi(2)).sum::<f64>().sqrt();
    if na == 0.0 || nb == 0.0 {
        0.0
    } else {
        (dot / (na * nb)).clamp(-1.0, 1.0) as f32
    }
}

// ── Storage ──────────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct WriteGuard {
    project_id: String,
    project_revision: Option<String>,
    source_revision: Option<(String, Option<String>)>,
}
fn revision_key(project: &str, source: Option<&str>) -> String {
    format!(
        "_internal.memory_revision.{}",
        serde_json::to_string(&(project, source)).expect("string serialization")
    )
}
fn revision(conn: &Connection, key: &str) -> rusqlite::Result<Option<String>> {
    use rusqlite::OptionalExtension;
    conn.query_row("SELECT value FROM settings WHERE key = ?1", [key], |row| {
        row.get(0)
    })
    .optional()
}
fn invalidate(conn: &Connection, key: &str) -> rusqlite::Result<()> {
    conn.execute("INSERT INTO settings(key,value) VALUES(?1,?2) ON CONFLICT(key) DO UPDATE SET value=excluded.value", rusqlite::params![key, uuid::Uuid::new_v4().to_string()])?;
    Ok(())
}
pub fn summary_guard(conn: &Connection, project: &str) -> rusqlite::Result<WriteGuard> {
    Ok(WriteGuard {
        project_id: project.into(),
        project_revision: revision(conn, &revision_key(project, None))?,
        source_revision: None,
    })
}
pub fn begin_document_import(
    conn: &Connection,
    project: &str,
    source: &str,
) -> rusqlite::Result<WriteGuard> {
    let tx = conn.unchecked_transaction()?;
    let key = revision_key(project, Some(source));
    invalidate(&tx, &key)?;
    let mut guard = summary_guard(&tx, project)?;
    guard.source_revision = Some((source.into(), revision(&tx, &key)?));
    tx.commit()?;
    Ok(guard)
}
fn valid_guard(conn: &Connection, guard: &WriteGuard) -> rusqlite::Result<bool> {
    if revision(conn, &revision_key(&guard.project_id, None))? != guard.project_revision {
        return Ok(false);
    }
    if let Some((source, expected)) = &guard.source_revision {
        if revision(conn, &revision_key(&guard.project_id, Some(source)))? != *expected {
            return Ok(false);
        }
    }
    Ok(true)
}
pub fn store_summary_checked(
    conn: &Connection,
    guard: &WriteGuard,
    session: &str,
    content: &str,
    embedding: &[f32],
) -> rusqlite::Result<bool> {
    let tx = conn.unchecked_transaction()?;
    let scope: String = tx.query_row(
        "SELECT project_id FROM sessions WHERE id=?1",
        [session],
        |row| row.get(0),
    )?;
    if scope != guard.project_id || !valid_guard(&tx, guard)? {
        return Ok(false);
    }
    store_memory(&tx, session, content, embedding, "session_summary")?;
    tx.commit()?;
    Ok(true)
}
pub fn store_document_checked(
    conn: &Connection,
    guard: &WriteGuard,
    content: &str,
    embedding: &[f32],
    replace: bool,
) -> rusqlite::Result<bool> {
    let tx = conn.unchecked_transaction()?;
    if !valid_guard(&tx, guard)? {
        return Ok(false);
    }
    let Some((source, _)) = &guard.source_revision else {
        return Ok(false);
    };
    if replace {
        tx.execute(
            "DELETE FROM memories WHERE project_id=?1 AND source_file=?2",
            rusqlite::params![guard.project_id, source],
        )?;
    }
    store_document_chunk(&tx, content, embedding, source, &guard.project_id)?;
    tx.commit()?;
    Ok(true)
}

/// Store a conversation summary (session-scoped, no source file).
pub fn store_memory(
    conn: &Connection,
    session_id: &str,
    content: &str,
    embedding: &[f32],
    mem_type: &str,
) -> rusqlite::Result<()> {
    let id = uuid::Uuid::new_v4().to_string();
    let blob = encode_embedding(embedding);
    let project_id: String = conn.query_row(
        "SELECT project_id FROM sessions WHERE id = ?1",
        rusqlite::params![session_id],
        |row| row.get(0),
    )?;
    conn.execute(
        "INSERT INTO memories (id, session_id, content, embedding, memory_type, created_at, project_id)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        rusqlite::params![id, session_id, content, blob, mem_type, now_ms(), project_id],
    )?;
    Ok(())
}

/// Store a document chunk (source_file is the document's display name).
pub fn store_document_chunk(
    conn: &Connection,
    content: &str,
    embedding: &[f32],
    source_file: &str,
    project_id: &str,
) -> rusqlite::Result<()> {
    let id = uuid::Uuid::new_v4().to_string();
    let blob = encode_embedding(embedding);
    conn.execute(
        "INSERT INTO memories (id, session_id, content, embedding, memory_type, created_at, source_file, project_id)
         VALUES (?1, NULL, ?2, ?3, 'document', ?4, ?5, ?6)",
        rusqlite::params![id, content, blob, now_ms(), source_file, project_id],
    )?;
    Ok(())
}

// ── Retrieval ────────────────────────────────────────────────────────────────

/// Returns top-k memories (both conversation and document) by cosine similarity.
/// Exact project-scoped scan with O(top_k * record size) retained candidates,
/// rather than discarding older knowledge. Runtime remains linear in project
/// size until a qualified vector index and embedding provenance are available.
pub fn search_memories(conn: &Connection, query: &[f32], top_k: usize) -> Vec<MemoryResult> {
    if top_k == 0
        || query.is_empty()
        || query.iter().any(|v| !v.is_finite())
        || query.iter().all(|v| *v == 0.0)
    {
        return vec![];
    }
    let Some(byte_length) = query
        .len()
        .checked_mul(4)
        .and_then(|len| i64::try_from(len).ok())
    else {
        return vec![];
    };
    let project_id = crate::db::selected_project_id(conn);
    let mut stmt = match conn.prepare(
        "SELECT id, session_id, content, embedding, memory_type, created_at, source_file, project_id
         FROM memories WHERE project_id = ?1 AND length(embedding) = ?2",
    ) {
        Ok(s) => s,
        Err(_) => return vec![],
    };

    let rows = match stmt.query_map(rusqlite::params![project_id, byte_length], |row| {
        let mem = Memory {
            id: row.get(0)?,
            session_id: row.get(1)?,
            content: row.get(2)?,
            memory_type: row.get(4)?,
            created_at: row.get(5)?,
            source_file: row.get(6)?,
            project_id: row.get(7)?,
        };
        let blob: Vec<u8> = row.get(3)?;
        Ok((mem, blob))
    }) {
        Ok(rows) => rows,
        Err(_) => return vec![],
    };
    let mut best = BinaryHeap::new();
    for (memory, blob) in rows.filter_map(Result::ok) {
        let embedding = decode_embedding(&blob);
        if embedding.iter().any(|v| !v.is_finite()) || embedding.iter().all(|v| *v == 0.0) {
            continue;
        }
        let candidate = Candidate(MemoryResult {
            score: cosine_similarity(query, &embedding),
            memory,
        });
        if best.len() < top_k {
            best.push(Reverse(candidate));
        } else if best.peek().is_some_and(|worst| candidate > worst.0) {
            best.pop();
            best.push(Reverse(candidate));
        }
    }
    best.into_sorted_vec()
        .into_iter()
        .map(|entry| entry.0 .0)
        .collect()
}

/// Returns conversation memories (excludes document chunks).
pub fn list_memories(conn: &Connection) -> Vec<Memory> {
    let project_id = crate::db::selected_project_id(conn);
    let mut stmt = match conn.prepare(
        "SELECT id, session_id, content, memory_type, created_at, source_file, project_id
         FROM memories WHERE memory_type != 'document' AND project_id = ?1 ORDER BY created_at DESC",
    ) {
        Ok(s) => s,
        Err(_) => return vec![],
    };
    stmt.query_map(rusqlite::params![project_id], |row| {
        Ok(Memory {
            id: row.get(0)?,
            session_id: row.get(1)?,
            content: row.get(2)?,
            memory_type: row.get(3)?,
            created_at: row.get(4)?,
            source_file: row.get(5)?,
            project_id: row.get(6)?,
        })
    })
    .unwrap()
    .filter_map(|r| r.ok())
    .collect()
}

/// Returns distinct documents with chunk counts.
pub fn list_documents(conn: &Connection) -> Vec<DocumentInfo> {
    let project_id = crate::db::selected_project_id(conn);
    let mut stmt = match conn.prepare(
        "SELECT source_file, COUNT(*) as chunk_count, MAX(created_at) as ingested_at
         FROM memories WHERE memory_type = 'document' AND source_file IS NOT NULL AND project_id = ?1
         GROUP BY source_file ORDER BY ingested_at DESC",
    ) {
        Ok(s) => s,
        Err(_) => return vec![],
    };
    stmt.query_map(rusqlite::params![project_id], |row| {
        Ok(DocumentInfo {
            source_file: row.get(0)?,
            chunk_count: row.get(1)?,
            ingested_at: row.get(2)?,
        })
    })
    .unwrap()
    .filter_map(|r| r.ok())
    .collect()
}

pub fn delete_memory(conn: &Connection, id: &str) -> rusqlite::Result<()> {
    let project_id = crate::db::selected_project_id(conn);
    let tx = conn.unchecked_transaction()?;
    invalidate(&tx, &revision_key(&project_id, None))?;
    tx.execute(
        "DELETE FROM memories WHERE id = ?1 AND project_id = ?2",
        rusqlite::params![id, project_id],
    )?;
    tx.commit()?;
    Ok(())
}

pub fn delete_document(conn: &Connection, source_file: &str) -> rusqlite::Result<()> {
    let project_id = crate::db::selected_project_id(conn);
    let tx = conn.unchecked_transaction()?;
    invalidate(&tx, &revision_key(&project_id, Some(source_file)))?;
    tx.execute(
        "DELETE FROM memories WHERE source_file = ?1 AND project_id = ?2",
        rusqlite::params![source_file, project_id],
    )?;
    tx.commit()?;
    Ok(())
}

pub fn forget_all(conn: &Connection) -> rusqlite::Result<()> {
    let project_id = crate::db::selected_project_id(conn);
    let tx = conn.unchecked_transaction()?;
    invalidate(&tx, &revision_key(&project_id, None))?;
    tx.execute(
        "DELETE FROM memories WHERE project_id = ?1",
        rusqlite::params![project_id],
    )?;
    tx.commit()?;
    Ok(())
}

pub fn count_memories(conn: &Connection) -> i64 {
    let project_id = crate::db::selected_project_id(conn);
    conn.query_row(
        "SELECT COUNT(*) FROM memories WHERE memory_type != 'document' AND project_id = ?1",
        rusqlite::params![project_id],
        |r| r.get(0),
    )
    .unwrap_or(0)
}

fn now_ms() -> i64 {
    crate::domain::now_ms()
}

#[cfg(test)]
mod tests {
    use super::{
        delete_document, delete_memory, forget_all, list_documents, search_memories,
        store_document_chunk,
    };

    #[test]
    fn older_relevant_memories_survive_large_project_and_invalid_embeddings() {
        let connection = rusqlite::Connection::open_in_memory().unwrap();
        crate::db::migrations::run(&connection).unwrap();
        // The strongest match predates more than 200 distractors.
        store_document_chunk(&connection, "old match", &[1.0, 0.0], "old.txt", "personal").unwrap();
        connection
            .execute("UPDATE memories SET created_at = 1", [])
            .unwrap();
        for _ in 0..250 {
            store_document_chunk(
                &connection,
                "distractor",
                &[0.0, 1.0],
                "new.txt",
                "personal",
            )
            .unwrap();
        }
        for embedding in [
            vec![1.0],
            vec![f32::NAN, 0.0],
            vec![f32::INFINITY, 1.0],
            vec![0.0, 0.0],
        ] {
            store_document_chunk(
                &connection,
                "invalid",
                &embedding,
                "invalid.txt",
                "personal",
            )
            .unwrap();
        }
        connection.execute("INSERT INTO projects (id, name, kind, created_at, updated_at) VALUES ('other', 'Other', 'project', 1, 1)", []).unwrap();
        store_document_chunk(
            &connection,
            "private other project",
            &[1.0, 0.0],
            "other.txt",
            "other",
        )
        .unwrap();
        // A trailing partial component must not decode as a valid 2D vector.
        let mut malformed = super::encode_embedding(&[1.0, 0.0]);
        malformed.push(1);
        connection.execute("INSERT INTO memories (id, content, embedding, memory_type, created_at, project_id) VALUES ('malformed', 'invalid', ?1, 'document', 2, 'personal')", [malformed]).unwrap();

        let results = search_memories(&connection, &[1.0, 0.0], 3);
        assert_eq!(results.len(), 3);
        assert_eq!(results[0].memory.content, "old match");
        assert_eq!(results[0].score, 1.0);
        assert!(results
            .iter()
            .all(|result| result.memory.project_id == "personal"
                && result.memory.content != "invalid"));
        assert_eq!(search_memories(&connection, &[1.0, 0.0], 1000).len(), 251);
        assert!(search_memories(&connection, &[1.0, 0.0], 0).is_empty());
        for query in [vec![], vec![0.0, 0.0], vec![f32::NAN, 1.0]] {
            assert!(search_memories(&connection, &query, 3).is_empty());
        }
        crate::db::set_setting(&connection, "selected_project_id", "other").unwrap();
        let other = search_memories(&connection, &[1.0, 0.0], 3);
        assert_eq!(other.len(), 1);
        assert_eq!(other[0].memory.content, "private other project");
    }

    #[test]
    fn similarity_handles_large_finite_components_and_invalid_inputs() {
        assert_eq!(
            super::cosine_similarity(&[f32::MAX, f32::MAX], &[f32::MAX, f32::MAX]),
            1.0
        );
        assert_eq!(super::cosine_similarity(&[f32::NAN], &[1.0]), 0.0);
        assert_eq!(super::cosine_similarity(&[1.0], &[1.0, 0.0]), 0.0);
    }

    #[test]
    fn retrieval_and_documents_are_project_scoped() {
        let connection = rusqlite::Connection::open_in_memory().expect("database");
        crate::db::migrations::run(&connection).expect("migrations");

        store_document_chunk(
            &connection,
            "personal",
            &[1.0, 0.0],
            "shared.txt",
            "personal",
        )
        .expect("personal chunk");
        connection
            .execute(
                "INSERT INTO projects (id, name, kind, created_at, updated_at) VALUES ('client-a', 'Client A', 'client', 1, 1)",
                [],
            )
            .expect("project");
        crate::db::set_setting(&connection, "selected_project_id", "client-a")
            .expect("select project");
        store_document_chunk(&connection, "client", &[0.0, 1.0], "shared.txt", "client-a")
            .expect("client chunk");

        let results = search_memories(&connection, &[0.0, 1.0], 10);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].memory.content, "client");
        assert_eq!(list_documents(&connection)[0].chunk_count, 1);
        // Delayed import work must stay in its admitted scope after a switch.
        store_document_chunk(
            &connection,
            "late personal chunk",
            &[1.0, 0.0],
            "shared.txt",
            "personal",
        )
        .unwrap();
        let personal_id: String = connection
            .query_row(
                "SELECT id FROM memories WHERE content = 'personal'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        delete_memory(&connection, &personal_id).unwrap();
        delete_document(&connection, "shared.txt").unwrap();
        forget_all(&connection).unwrap();
        crate::db::set_setting(&connection, "selected_project_id", "personal").unwrap();
        assert_eq!(list_documents(&connection)[0].chunk_count, 2);
        assert_eq!(search_memories(&connection, &[1.0, 0.0], 10).len(), 2);
    }
}
#[cfg(test)]
mod deletion_guard_tests {
    use super::*;
    fn database() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        crate::db::migrations::run(&conn).unwrap();
        conn
    }
    #[test]
    fn deletion_invalidates_pending_summary_but_allows_fresh_work() {
        let conn = database();
        let session = crate::db::ensure_session(&conn).unwrap();
        let old = summary_guard(&conn, "personal").unwrap();
        forget_all(&conn).unwrap();
        assert!(!store_summary_checked(&conn, &old, &session, "forgotten", &[1.0]).unwrap());
        let fresh = summary_guard(&conn, "personal").unwrap();
        assert!(store_summary_checked(&conn, &fresh, &session, "new", &[1.0]).unwrap());
    }
    #[test]
    fn source_delete_and_replacement_invalidate_inflight_chunks() {
        let conn = database();
        let first = begin_document_import(&conn, "personal", "brief.txt").unwrap();
        assert!(store_document_checked(&conn, &first, "first", &[1.0], true).unwrap());
        delete_document(&conn, "brief.txt").unwrap();
        assert!(!store_document_checked(&conn, &first, "late", &[1.0], true).unwrap());
        let second = begin_document_import(&conn, "personal", "brief.txt").unwrap();
        assert!(store_document_checked(&conn, &second, "second", &[1.0], true).unwrap());
        let failed_before_insert = begin_document_import(&conn, "personal", "brief.txt").unwrap();
        assert_eq!(list_documents(&conn)[0].chunk_count, 1);
        drop(failed_before_insert);
        let third = begin_document_import(&conn, "personal", "brief.txt").unwrap();
        assert!(!store_document_checked(&conn, &second, "superseded", &[1.0], true).unwrap());
        assert!(store_document_checked(&conn, &third, "third", &[1.0], true).unwrap());
        assert_eq!(list_documents(&conn)[0].chunk_count, 1);
    }
    #[test]
    fn deleting_personal_does_not_invalidate_other_projects_or_other_sources() {
        let conn = database();
        let other = begin_document_import(&conn, "other", "brief.txt").unwrap();
        let retained = begin_document_import(&conn, "personal", "keep.txt").unwrap();
        delete_document(&conn, "brief.txt").unwrap();
        assert!(store_document_checked(&conn, &retained, "kept", &[1.0], true).unwrap());
        forget_all(&conn).unwrap();
        assert!(store_document_checked(&conn, &other, "other", &[1.0], true).unwrap());
    }
}

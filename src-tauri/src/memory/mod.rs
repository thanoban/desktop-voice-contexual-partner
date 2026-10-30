pub mod retention;
pub mod retrieval;

use rusqlite::Connection;
use serde::Serialize;

#[derive(Debug, Serialize, Clone)]
pub struct Memory {
    pub id: String,
    pub session_id: Option<String>,
    pub content: String,
    pub memory_type: String,
    pub created_at: i64,
    pub source_file: Option<String>,
    pub project_id: String,
    /// Embedding provenance. Vectors are compared only within the same model
    /// and dimension; `None` means the row is lexical-only.
    pub embedding_model: Option<String>,
    pub embedding_dim: Option<i64>,
    pub document_id: Option<String>,
    pub document_revision: Option<i64>,
    pub chunk_index: Option<i64>,
}

#[derive(Debug, Serialize, Clone)]
pub struct DocumentInfo {
    pub source_file: String,
    pub chunk_count: i64,
    pub ingested_at: i64,
    pub document_id: String,
    pub revision: i64,
    pub content_sha256: Option<String>,
    pub embedding_model: Option<String>,
    /// Active chunks whose vectors came from a model other than the current
    /// setting; they stay lexically searchable until the document is re-imported.
    pub incompatible_chunks: i64,
}

#[cfg(test)]
pub struct MemoryResult {
    pub memory: Memory,
    pub score: f32,
}

pub(crate) const MEMORY_COLUMNS: &str =
    "m.id, m.session_id, m.content, m.memory_type, m.created_at, m.source_file, m.project_id, \
     m.embedding_model, m.embedding_dim, m.document_id, m.document_revision, m.chunk_index";

pub(crate) fn memory_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Memory> {
    Ok(Memory {
        id: row.get(0)?,
        session_id: row.get(1)?,
        content: row.get(2)?,
        memory_type: row.get(3)?,
        created_at: row.get(4)?,
        source_file: row.get(5)?,
        project_id: row.get(6)?,
        embedding_model: row.get(7)?,
        embedding_dim: row.get(8)?,
        document_id: row.get(9)?,
        document_revision: row.get(10)?,
        chunk_index: row.get(11)?,
    })
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

/// The configured embedding model, recorded as provenance on stored vectors.
pub fn current_embedding_model(conn: &Connection) -> String {
    crate::db::get_setting(conn, "embedding_model")
        .unwrap_or_else(|| crate::embed::DEFAULT_MODEL.into())
}

fn provenance<'a>(model: &'a str, embedding: &[f32]) -> (Option<&'a str>, Option<i64>) {
    if model.trim().is_empty() || embedding.is_empty() {
        return (None, None);
    }
    (Some(model), i64::try_from(embedding.len()).ok())
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
    embedding_model: String,
    /// Stable document ID and the staged revision this import writes.
    document: Option<(String, i64)>,
}
impl WriteGuard {
    /// Model the guarded work must embed with so stored provenance matches.
    pub fn embedding_model(&self) -> &str {
        &self.embedding_model
    }
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
/// Invalidates every pending memory/document write for a project.
pub(crate) fn invalidate_project(conn: &Connection, project: &str) -> rusqlite::Result<()> {
    invalidate(conn, &revision_key(project, None))
}
pub fn summary_guard(conn: &Connection, project: &str) -> rusqlite::Result<WriteGuard> {
    Ok(WriteGuard {
        project_id: project.into(),
        project_revision: revision(conn, &revision_key(project, None))?,
        source_revision: None,
        embedding_model: current_embedding_model(conn),
        document: None,
    })
}
/// Starts a new staged revision of a document. Its chunks stay invisible to
/// retrieval until `finish_document_import` activates the revision, so a
/// failed re-import leaves the previous revision intact.
pub fn begin_document_import(
    conn: &Connection,
    project: &str,
    source: &str,
) -> rusqlite::Result<WriteGuard> {
    let tx = conn.unchecked_transaction()?;
    let key = revision_key(project, Some(source));
    invalidate(&tx, &key)?;
    let now = now_ms();
    tx.execute(
        "INSERT INTO documents (id, project_id, source_name, active_revision, latest_revision, created_at, updated_at)
         VALUES (?1, ?2, ?3, 0, 0, ?4, ?4) ON CONFLICT(project_id, source_name) DO NOTHING",
        rusqlite::params![uuid::Uuid::new_v4().to_string(), project, source, now],
    )?;
    // Revision numbers are never reused, so a staged revision belongs to one import.
    let (document_id, target, active): (String, i64, i64) = tx.query_row(
        "UPDATE documents SET latest_revision = latest_revision + 1
         WHERE project_id = ?1 AND source_name = ?2
         RETURNING id, latest_revision, active_revision",
        rusqlite::params![project, source],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    )?;
    // Interrupted or superseded imports leave inactive chunks behind.
    tx.execute(
        "DELETE FROM memories WHERE document_id = ?1 AND document_revision != ?2",
        rusqlite::params![document_id, active],
    )?;
    let mut guard = summary_guard(&tx, project)?;
    guard.source_revision = Some((source.into(), revision(&tx, &key)?));
    guard.document = Some((document_id, target));
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
    store_memory(
        &tx,
        session,
        content,
        embedding,
        "session_summary",
        &guard.embedding_model,
    )?;
    tx.commit()?;
    Ok(true)
}

/// Stores one chunk of the guard's staged document revision.
pub fn store_document_revision_chunk(
    conn: &Connection,
    guard: &WriteGuard,
    chunk_index: usize,
    content: &str,
    embedding: &[f32],
) -> rusqlite::Result<bool> {
    let tx = conn.unchecked_transaction()?;
    if !valid_guard(&tx, guard)? {
        return Ok(false);
    }
    let (Some((source, _)), Some((document_id, revision))) =
        (&guard.source_revision, &guard.document)
    else {
        return Ok(false);
    };
    let exists: i64 = tx.query_row(
        "SELECT COUNT(*) FROM documents WHERE id = ?1 AND project_id = ?2",
        rusqlite::params![document_id, guard.project_id],
        |row| row.get(0),
    )?;
    if exists == 0 {
        return Ok(false);
    }
    insert_chunk(
        &tx,
        &NewChunk {
            project_id: &guard.project_id,
            source_name: source,
            document_id,
            revision: *revision,
            chunk_index: i64::try_from(chunk_index).unwrap_or(i64::MAX),
            content,
            embedding,
            embedding_model: &guard.embedding_model,
        },
    )?;
    tx.commit()?;
    Ok(true)
}

/// Atomically makes the staged revision active and removes every other
/// revision's chunks (and, through triggers, their lexical index rows).
pub fn finish_document_import(
    conn: &Connection,
    guard: &WriteGuard,
    content_sha256: Option<&str>,
) -> rusqlite::Result<bool> {
    let tx = conn.unchecked_transaction()?;
    if !valid_guard(&tx, guard)? {
        return Ok(false);
    }
    let Some((document_id, revision)) = &guard.document else {
        return Ok(false);
    };
    let changed = tx.execute(
        "UPDATE documents SET active_revision = ?2, content_sha256 = ?3, updated_at = ?4
         WHERE id = ?1 AND project_id = ?5",
        rusqlite::params![
            document_id,
            revision,
            content_sha256,
            now_ms(),
            guard.project_id
        ],
    )?;
    if changed == 0 {
        return Ok(false);
    }
    tx.execute(
        "DELETE FROM memories WHERE document_id = ?1 AND document_revision != ?2",
        rusqlite::params![document_id, revision],
    )?;
    tx.commit()?;
    Ok(true)
}

/// Discards chunks staged by a failed import. Revisions are unique per import,
/// so this never touches the active or a newer revision.
pub fn abandon_document_import(conn: &Connection, guard: &WriteGuard) -> rusqlite::Result<()> {
    if let Some((document_id, revision)) = &guard.document {
        conn.execute(
            "DELETE FROM memories WHERE document_id = ?1 AND document_revision = ?2
             AND document_revision != COALESCE((SELECT active_revision FROM documents WHERE id = ?1), 0)",
            rusqlite::params![document_id, revision],
        )?;
    }
    Ok(())
}

/// Legacy single-call import used by guard regression tests: stage one chunk
/// and immediately activate the guard's revision.
#[cfg(test)]
pub fn store_document_checked(
    conn: &Connection,
    guard: &WriteGuard,
    content: &str,
    embedding: &[f32],
    _replace: bool,
) -> rusqlite::Result<bool> {
    let Some((document_id, revision)) = &guard.document else {
        return Ok(false);
    };
    let index: i64 = conn.query_row(
        "SELECT COUNT(*) FROM memories WHERE document_id = ?1 AND document_revision = ?2",
        rusqlite::params![document_id, revision],
        |row| row.get(0),
    )?;
    if !store_document_revision_chunk(conn, guard, index as usize, content, embedding)? {
        return Ok(false);
    }
    finish_document_import(conn, guard, None)
}

/// Store a conversation summary (session-scoped, no source file).
pub fn store_memory(
    conn: &Connection,
    session_id: &str,
    content: &str,
    embedding: &[f32],
    mem_type: &str,
    embedding_model: &str,
) -> rusqlite::Result<()> {
    let id = uuid::Uuid::new_v4().to_string();
    let blob = encode_embedding(embedding);
    let (model, dim) = provenance(embedding_model, embedding);
    let project_id: String = conn.query_row(
        "SELECT project_id FROM sessions WHERE id = ?1",
        rusqlite::params![session_id],
        |row| row.get(0),
    )?;
    conn.execute(
        "INSERT INTO memories (id, session_id, content, embedding, memory_type, created_at, project_id, embedding_model, embedding_dim)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        rusqlite::params![id, session_id, content, blob, mem_type, now_ms(), project_id, model, dim],
    )?;
    Ok(())
}

struct NewChunk<'a> {
    project_id: &'a str,
    source_name: &'a str,
    document_id: &'a str,
    revision: i64,
    chunk_index: i64,
    content: &'a str,
    embedding: &'a [f32],
    embedding_model: &'a str,
}

fn insert_chunk(conn: &Connection, chunk: &NewChunk<'_>) -> rusqlite::Result<()> {
    let (model, dim) = provenance(chunk.embedding_model, chunk.embedding);
    conn.execute(
        "INSERT INTO memories (id, session_id, content, embedding, memory_type, created_at, source_file, project_id,
                               embedding_model, embedding_dim, document_id, document_revision, chunk_index)
         VALUES (?1, NULL, ?2, ?3, 'document', ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        rusqlite::params![
            uuid::Uuid::new_v4().to_string(),
            chunk.content,
            encode_embedding(chunk.embedding),
            now_ms(),
            chunk.source_name,
            chunk.project_id,
            model,
            dim,
            chunk.document_id,
            chunk.revision,
            chunk.chunk_index
        ],
    )?;
    Ok(())
}

/// Appends an unguarded chunk to a document's active revision (test fixture).
#[cfg(test)]
pub fn store_document_chunk(
    conn: &Connection,
    content: &str,
    embedding: &[f32],
    source_file: &str,
    project_id: &str,
) -> rusqlite::Result<()> {
    let now = now_ms();
    conn.execute(
        "INSERT INTO documents (id, project_id, source_name, active_revision, latest_revision, created_at, updated_at)
         VALUES (?1, ?2, ?3, 1, 1, ?4, ?4) ON CONFLICT(project_id, source_name) DO NOTHING",
        rusqlite::params![uuid::Uuid::new_v4().to_string(), project_id, source_file, now],
    )?;
    let (document_id, revision): (String, i64) = conn.query_row(
        "UPDATE documents SET active_revision = MAX(active_revision, 1), latest_revision = MAX(latest_revision, 1)
         WHERE project_id = ?1 AND source_name = ?2 RETURNING id, active_revision",
        rusqlite::params![project_id, source_file],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;
    let chunk_index: i64 = conn.query_row(
        "SELECT COUNT(*) FROM memories WHERE document_id = ?1 AND document_revision = ?2",
        rusqlite::params![document_id, revision],
        |row| row.get(0),
    )?;
    insert_chunk(
        conn,
        &NewChunk {
            project_id,
            source_name: source_file,
            document_id: &document_id,
            revision,
            chunk_index,
            content,
            embedding,
            embedding_model: &current_embedding_model(conn),
        },
    )
}

// ── Retrieval ────────────────────────────────────────────────────────────────

/// Vector-only exact top-k over the selected project and current embedding
/// model. Production prompt assembly uses `retrieval::retrieve`.
#[cfg(test)]
pub fn search_memories(conn: &Connection, query: &[f32], top_k: usize) -> Vec<MemoryResult> {
    let project_id = crate::db::selected_project_id(conn);
    let model = current_embedding_model(conn);
    let scored = retrieval::vector_candidates(conn, &project_id, &model, query, top_k, -1.0);
    let ids: Vec<String> = scored.iter().map(|(id, _)| id.clone()).collect();
    let mut rows = retrieval::fetch_active(conn, &project_id, &ids).unwrap_or_default();
    scored
        .into_iter()
        .filter_map(|(id, score)| {
            rows.remove(&id)
                .map(|(memory, _)| MemoryResult { memory, score })
        })
        .collect()
}

/// Returns conversation memories (excludes document chunks).
pub fn list_memories(conn: &Connection) -> Vec<Memory> {
    let project_id = crate::db::selected_project_id(conn);
    let mut stmt = match conn.prepare(&format!(
        "SELECT {MEMORY_COLUMNS} FROM memories m
         WHERE m.memory_type != 'document' AND m.project_id = ?1 ORDER BY m.created_at DESC"
    )) {
        Ok(s) => s,
        Err(_) => return vec![],
    };
    stmt.query_map(rusqlite::params![project_id], memory_from_row)
        .map(|rows| rows.filter_map(|r| r.ok()).collect())
        .unwrap_or_default()
}

/// Returns active document revisions with chunk counts and provenance.
pub fn list_documents(conn: &Connection) -> Vec<DocumentInfo> {
    let project_id = crate::db::selected_project_id(conn);
    let model = current_embedding_model(conn);
    let mut stmt = match conn.prepare(
        "SELECT d.source_name, COUNT(m.id), MAX(m.created_at), d.id, d.active_revision, d.content_sha256,
                MAX(m.embedding_model),
                SUM(CASE WHEN m.embedding_model IS ?2 THEN 0 ELSE 1 END)
         FROM documents d
         JOIN memories m ON m.document_id = d.id AND m.document_revision = d.active_revision
         WHERE d.project_id = ?1 AND d.active_revision > 0
         GROUP BY d.id ORDER BY MAX(m.created_at) DESC, d.id",
    ) {
        Ok(s) => s,
        Err(_) => return vec![],
    };
    stmt.query_map(rusqlite::params![project_id, model], |row| {
        Ok(DocumentInfo {
            source_file: row.get(0)?,
            chunk_count: row.get(1)?,
            ingested_at: row.get(2)?,
            document_id: row.get(3)?,
            revision: row.get(4)?,
            content_sha256: row.get(5)?,
            embedding_model: row.get(6)?,
            incompatible_chunks: row.get(7)?,
        })
    })
    .map(|rows| rows.filter_map(|r| r.ok()).collect())
    .unwrap_or_default()
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
    tx.execute(
        "DELETE FROM documents WHERE source_name = ?1 AND project_id = ?2",
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
    tx.execute(
        "DELETE FROM documents WHERE project_id = ?1",
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

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
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let na: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let nb: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if na < f32::EPSILON || nb < f32::EPSILON {
        0.0
    } else {
        (dot / (na * nb)).clamp(-1.0, 1.0)
    }
}

// ── Storage ──────────────────────────────────────────────────────────────────

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
/// Scans last 200 entries (most recent) for performance.
pub fn search_memories(conn: &Connection, query: &[f32], top_k: usize) -> Vec<MemoryResult> {
    let project_id = crate::db::selected_project_id(conn);
    let mut stmt = match conn.prepare(
        "SELECT id, session_id, content, embedding, memory_type, created_at, source_file, project_id
         FROM memories WHERE project_id = ?1 ORDER BY created_at DESC LIMIT 200",
    ) {
        Ok(s) => s,
        Err(_) => return vec![],
    };

    let mut scored: Vec<MemoryResult> = stmt
        .query_map(rusqlite::params![project_id], |row| {
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
        })
        .unwrap()
        .filter_map(|r| r.ok())
        .map(|(mem, blob)| {
            let emb = decode_embedding(&blob);
            let score = cosine_similarity(query, &emb);
            MemoryResult { memory: mem, score }
        })
        .collect();

    scored.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    scored.truncate(top_k);
    scored
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
    conn.execute(
        "DELETE FROM memories WHERE id = ?1 AND project_id = ?2",
        rusqlite::params![id, project_id],
    )?;
    Ok(())
}

pub fn delete_document(conn: &Connection, source_file: &str) -> rusqlite::Result<()> {
    let project_id = crate::db::selected_project_id(conn);
    conn.execute(
        "DELETE FROM memories WHERE source_file = ?1 AND project_id = ?2",
        rusqlite::params![source_file, project_id],
    )?;
    Ok(())
}

pub fn forget_all(conn: &Connection) -> rusqlite::Result<()> {
    let project_id = crate::db::selected_project_id(conn);
    conn.execute(
        "DELETE FROM memories WHERE project_id = ?1",
        rusqlite::params![project_id],
    )?;
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

use rusqlite::Connection;
use serde::Serialize;

#[derive(Debug, Serialize, Clone)]
pub struct Memory {
    pub id: String,
    pub session_id: Option<String>,
    pub content: String,
    pub memory_type: String,
    pub created_at: i64,
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

pub fn store_memory(
    conn: &Connection,
    session_id: &str,
    content: &str,
    embedding: &[f32],
    mem_type: &str,
) -> rusqlite::Result<()> {
    let id = uuid::Uuid::new_v4().to_string();
    let blob = encode_embedding(embedding);
    let now_ms = now_ms();
    conn.execute(
        "INSERT INTO memories (id, session_id, content, embedding, memory_type, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        rusqlite::params![id, session_id, content, blob, mem_type, now_ms],
    )?;
    Ok(())
}

// ── Retrieval ────────────────────────────────────────────────────────────────

/// Returns top-k memories sorted by cosine similarity. Scans last 200 entries.
pub fn search_memories(
    conn: &Connection,
    query: &[f32],
    top_k: usize,
) -> Vec<MemoryResult> {
    let mut stmt = match conn.prepare(
        "SELECT id, session_id, content, embedding, memory_type, created_at
         FROM memories ORDER BY created_at DESC LIMIT 200",
    ) {
        Ok(s) => s,
        Err(_) => return vec![],
    };

    let mut scored: Vec<MemoryResult> = stmt
        .query_map([], |row| {
            let mem = Memory {
                id:          row.get(0)?,
                session_id:  row.get(1)?,
                content:     row.get(2)?,
                memory_type: row.get(4)?,
                created_at:  row.get(5)?,
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

    scored.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    scored.truncate(top_k);
    scored
}

pub fn list_memories(conn: &Connection) -> Vec<Memory> {
    let mut stmt = match conn.prepare(
        "SELECT id, session_id, content, memory_type, created_at
         FROM memories ORDER BY created_at DESC",
    ) {
        Ok(s) => s,
        Err(_) => return vec![],
    };
    stmt.query_map([], |row| {
        Ok(Memory {
            id:          row.get(0)?,
            session_id:  row.get(1)?,
            content:     row.get(2)?,
            memory_type: row.get(3)?,
            created_at:  row.get(4)?,
        })
    })
    .unwrap()
    .filter_map(|r| r.ok())
    .collect()
}

pub fn delete_memory(conn: &Connection, id: &str) -> rusqlite::Result<()> {
    conn.execute("DELETE FROM memories WHERE id = ?1", rusqlite::params![id])?;
    Ok(())
}

pub fn forget_all(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute("DELETE FROM memories", [])?;
    Ok(())
}

pub fn count_memories(conn: &Connection) -> i64 {
    conn.query_row("SELECT COUNT(*) FROM memories", [], |r| r.get(0))
        .unwrap_or(0)
}

// ── Helpers ──────────────────────────────────────────────────────────────────

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64
}

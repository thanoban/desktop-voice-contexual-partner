//! Hybrid, source-grounded retrieval over one project's memories and active
//! document revisions.
//!
//! Lexical candidates come from the FTS5 index (BM25). Vector candidates come
//! from an exact cosine scan restricted to rows with the query's embedding
//! model and dimension; the scan is paged so the connection lock is held for
//! one bounded page at a time and scoring happens outside it. Both ranked lists
//! are fused with reciprocal rank fusion, then the winners are re-read under
//! the lock so rows deleted mid-search are never returned.

use super::{cosine_similarity, decode_embedding, memory_from_row, Memory, MEMORY_COLUMNS};
use rusqlite::Connection;
use serde::Serialize;
use std::cmp::{Ordering, Reverse};
use std::collections::{BinaryHeap, HashMap};
use std::sync::Mutex;

/// Rows decoded per lock acquisition during the vector scan.
pub const VECTOR_PAGE_SIZE: usize = 256;
const LEXICAL_CANDIDATES: usize = 50;
const VECTOR_CANDIDATES: usize = 50;
const RRF_K: f64 = 60.0;
const MAX_QUERY_TERMS: usize = 12;

/// Gives retrieval short-lived access to the database. The mutex
/// implementation releases the lock as soon as each closure returns.
pub trait ConnectionSource {
    fn with_connection<R>(&self, f: impl FnOnce(&Connection) -> R) -> Option<R>;
}

impl ConnectionSource for Connection {
    fn with_connection<R>(&self, f: impl FnOnce(&Connection) -> R) -> Option<R> {
        Some(f(self))
    }
}

impl ConnectionSource for Mutex<Connection> {
    fn with_connection<R>(&self, f: impl FnOnce(&Connection) -> R) -> Option<R> {
        let guard = self.lock().ok()?;
        Some(f(&guard))
    }
}

#[derive(Debug, Serialize, Clone, PartialEq, Eq)]
pub struct Citation {
    pub source_name: String,
    pub document_id: String,
    pub revision: i64,
    pub chunk_index: i64,
}

#[derive(Debug, Serialize, Clone)]
pub struct RetrievalHit {
    pub memory: Memory,
    /// Present for document chunks of the active revision.
    pub citation: Option<Citation>,
    pub vector_score: Option<f32>,
    /// 1-based BM25 rank when the hit matched lexically.
    pub lexical_rank: Option<usize>,
    pub fused_score: f64,
}

pub struct RetrievalQuery<'a> {
    pub project_id: &'a str,
    pub text: &'a str,
    pub embedding: Option<&'a [f32]>,
    /// Model that produced `embedding`; other models' vectors are excluded.
    pub embedding_model: &'a str,
    /// Vector candidates below this cosine similarity are not considered.
    pub min_similarity: f32,
    pub top_k: usize,
}

pub fn retrieve<S: ConnectionSource>(db: &S, query: &RetrievalQuery<'_>) -> Vec<RetrievalHit> {
    if query.top_k == 0 {
        return vec![];
    }
    let lexical = fts_query(query.text)
        .and_then(|expression| {
            db.with_connection(|conn| {
                lexical_candidates(conn, query.project_id, &expression, LEXICAL_CANDIDATES)
            })
        })
        .and_then(Result::ok)
        .unwrap_or_default();
    let vector = query
        .embedding
        .map(|embedding| {
            vector_candidates(
                db,
                query.project_id,
                query.embedding_model,
                embedding,
                VECTOR_CANDIDATES.max(query.top_k),
                query.min_similarity,
            )
        })
        .unwrap_or_default();

    let mut fused: HashMap<String, (f64, Option<usize>, Option<f32>)> = HashMap::new();
    for (index, id) in lexical.into_iter().enumerate() {
        let entry = fused.entry(id).or_default();
        entry.0 += 1.0 / (RRF_K + (index + 1) as f64);
        entry.1 = Some(index + 1);
    }
    for (index, (id, score)) in vector.into_iter().enumerate() {
        let entry = fused.entry(id).or_default();
        entry.0 += 1.0 / (RRF_K + (index + 1) as f64);
        entry.2 = Some(score);
    }
    if fused.is_empty() {
        return vec![];
    }
    let mut ids: Vec<String> = fused.keys().cloned().collect();
    ids.sort();
    let Some(Ok(rows)) = db.with_connection(|conn| fetch_active(conn, query.project_id, &ids))
    else {
        return vec![];
    };
    let mut hits: Vec<RetrievalHit> = rows
        .into_values()
        .filter_map(|(memory, citation)| {
            let (fused_score, lexical_rank, vector_score) = *fused.get(&memory.id)?;
            Some(RetrievalHit {
                memory,
                citation,
                vector_score,
                lexical_rank,
                fused_score,
            })
        })
        .collect();
    hits.sort_by(compare_hits);
    hits.truncate(query.top_k);
    hits
}

// Higher fused score first; ties prefer the stronger vector match, then recent
// rows, then a stable ID, independent of SQLite's scan order.
fn compare_hits(a: &RetrievalHit, b: &RetrievalHit) -> Ordering {
    b.fused_score
        .total_cmp(&a.fused_score)
        .then_with(|| {
            b.vector_score
                .unwrap_or(f32::NEG_INFINITY)
                .total_cmp(&a.vector_score.unwrap_or(f32::NEG_INFINITY))
        })
        .then_with(|| b.memory.created_at.cmp(&a.memory.created_at))
        .then_with(|| a.memory.id.cmp(&b.memory.id))
}

/// Formats hits for the system prompt. Document excerpts carry a bracketed
/// citation the model is asked to reuse.
pub fn prompt_block(hits: &[RetrievalHit], age: impl Fn(i64) -> String) -> Option<String> {
    if hits.is_empty() {
        return None;
    }
    let lines: Vec<String> = hits
        .iter()
        .map(|hit| match &hit.citation {
            Some(citation) => format!(
                "• [{} · rev {} · part {}]: {}",
                citation.source_name,
                citation.revision,
                citation.chunk_index + 1,
                hit.memory.content
            ),
            None => format!("• {}: {}", age(hit.memory.created_at), hit.memory.content),
        })
        .collect();
    let cite = if hits.iter().any(|hit| hit.citation.is_some()) {
        "\nWhen you use a document excerpt, name its bracketed source."
    } else {
        ""
    };
    Some(format!(
        "[MEMORIES — relevant things from past conversations and project documents]\n{}{}\n[/MEMORIES]",
        lines.join("\n"),
        cite
    ))
}

// ── Lexical ──────────────────────────────────────────────────────────────────

const STOPWORDS: &[&str] = &[
    "a", "about", "am", "an", "and", "are", "as", "at", "be", "but", "by", "can", "could", "did",
    "do", "does", "for", "from", "had", "has", "have", "he", "her", "hey", "hi", "his", "how", "i",
    "if", "im", "in", "into", "is", "it", "its", "just", "me", "my", "no", "not", "of", "ok",
    "okay", "on", "or", "our", "she", "so", "that", "the", "their", "them", "then", "there",
    "they", "this", "to", "us", "was", "we", "were", "what", "when", "where", "which", "who",
    "why", "will", "with", "would", "yes", "you", "your",
];

/// Builds an FTS5 OR-query from free text. Terms are reduced to alphanumeric
/// words and quoted, so user text cannot inject FTS5 query syntax.
pub fn fts_query(text: &str) -> Option<String> {
    let mut terms: Vec<String> = Vec::new();
    for word in text.split(|c: char| !c.is_alphanumeric()) {
        let word = word.to_lowercase();
        if word.chars().count() < 2 || STOPWORDS.contains(&word.as_str()) || terms.contains(&word) {
            continue;
        }
        terms.push(word);
        if terms.len() == MAX_QUERY_TERMS {
            break;
        }
    }
    if terms.is_empty() {
        return None;
    }
    Some(
        terms
            .iter()
            .map(|term| format!("\"{term}\""))
            .collect::<Vec<_>>()
            .join(" OR "),
    )
}

fn lexical_candidates(
    conn: &Connection,
    project_id: &str,
    expression: &str,
    limit: usize,
) -> rusqlite::Result<Vec<String>> {
    let mut stmt = conn.prepare_cached(
        "SELECT m.id FROM memory_fts
         JOIN memory_fts_rows r ON r.fts_rowid = memory_fts.rowid
         JOIN memories m ON m.id = r.memory_id
         LEFT JOIN documents d ON d.id = m.document_id
         WHERE memory_fts MATCH ?1 AND m.project_id = ?2
           AND (m.document_id IS NULL OR m.document_revision = d.active_revision)
         ORDER BY bm25(memory_fts), m.created_at DESC, m.id
         LIMIT ?3",
    )?;
    let rows = stmt.query_map(
        rusqlite::params![expression, project_id, limit as i64],
        |row| row.get(0),
    )?;
    rows.collect()
}

// ── Vector ───────────────────────────────────────────────────────────────────

struct Scored {
    score: f32,
    created_at: i64,
    id: String,
}
impl PartialEq for Scored {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}
impl Eq for Scored {}
impl PartialOrd for Scored {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
// The smallest candidate is evicted first. Ties prefer recent memories, then a
// stable ID.
impl Ord for Scored {
    fn cmp(&self, other: &Self) -> Ordering {
        self.score
            .total_cmp(&other.score)
            .then_with(|| self.created_at.cmp(&other.created_at))
            .then_with(|| other.id.cmp(&self.id))
    }
}

type PageRow = (i64, String, Vec<u8>, i64);

/// Exact top-k cosine candidates `(id, score)`, best first. Only rows recorded
/// with `model` and the query's dimension in `project_id` are compared.
pub fn vector_candidates<S: ConnectionSource>(
    db: &S,
    project_id: &str,
    model: &str,
    query: &[f32],
    limit: usize,
    min_similarity: f32,
) -> Vec<(String, f32)> {
    if limit == 0
        || model.trim().is_empty()
        || query.is_empty()
        || query.iter().any(|v| !v.is_finite())
        || query.iter().all(|v| *v == 0.0)
    {
        return vec![];
    }
    let (Ok(dimension), Some(byte_length)) = (
        i64::try_from(query.len()),
        query
            .len()
            .checked_mul(4)
            .and_then(|len| i64::try_from(len).ok()),
    ) else {
        return vec![];
    };
    let mut best: BinaryHeap<Reverse<Scored>> = BinaryHeap::new();
    let mut after = i64::MIN;
    loop {
        let page = db.with_connection(|conn| {
            vector_page(conn, project_id, model, dimension, byte_length, after)
        });
        let Some(Ok(page)) = page else {
            break;
        };
        // Scoring happens here, after the page's lock has been released.
        let full = page.len() == VECTOR_PAGE_SIZE;
        for (rowid, id, blob, created_at) in page {
            after = rowid;
            let embedding = decode_embedding(&blob);
            if embedding.iter().any(|v| !v.is_finite()) || embedding.iter().all(|v| *v == 0.0) {
                continue;
            }
            let score = cosine_similarity(query, &embedding);
            if score < min_similarity {
                continue;
            }
            let candidate = Scored {
                score,
                created_at,
                id,
            };
            if best.len() < limit {
                best.push(Reverse(candidate));
            } else if best.peek().is_some_and(|worst| candidate > worst.0) {
                best.pop();
                best.push(Reverse(candidate));
            }
        }
        if !full {
            break;
        }
    }
    best.into_sorted_vec()
        .into_iter()
        .map(|entry| (entry.0.id, entry.0.score))
        .collect()
}

fn vector_page(
    conn: &Connection,
    project_id: &str,
    model: &str,
    dimension: i64,
    byte_length: i64,
    after: i64,
) -> rusqlite::Result<Vec<PageRow>> {
    let mut stmt = conn.prepare_cached(
        "SELECT m.rowid, m.id, m.embedding, m.created_at FROM memories m
         LEFT JOIN documents d ON d.id = m.document_id
         WHERE m.project_id = ?1 AND m.embedding_model = ?2 AND m.embedding_dim = ?3
           AND length(m.embedding) = ?4 AND m.rowid > ?5
           AND (m.document_id IS NULL OR m.document_revision = d.active_revision)
         ORDER BY m.rowid LIMIT ?6",
    )?;
    let rows = stmt.query_map(
        rusqlite::params![
            project_id,
            model,
            dimension,
            byte_length,
            after,
            VECTOR_PAGE_SIZE as i64
        ],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
    )?;
    rows.collect()
}

// ── Hydration ────────────────────────────────────────────────────────────────

/// Loads rows that still exist in `project_id` and belong to an active
/// document revision (or to no document), with their citations.
pub(crate) fn fetch_active(
    conn: &Connection,
    project_id: &str,
    ids: &[String],
) -> rusqlite::Result<HashMap<String, (Memory, Option<Citation>)>> {
    let mut found = HashMap::new();
    for batch in ids.chunks(500) {
        let placeholders = vec!["?"; batch.len()].join(", ");
        let mut stmt = conn.prepare(&format!(
            "SELECT {MEMORY_COLUMNS}, d.source_name FROM memories m
             LEFT JOIN documents d ON d.id = m.document_id
             WHERE m.project_id = ? AND m.id IN ({placeholders})
               AND (m.document_id IS NULL OR m.document_revision = d.active_revision)"
        ))?;
        let params = std::iter::once(project_id).chain(batch.iter().map(String::as_str));
        let rows = stmt.query_map(rusqlite::params_from_iter(params), |row| {
            let memory = memory_from_row(row)?;
            let source_name: Option<String> = row.get(12)?;
            let citation = match (&memory.document_id, source_name) {
                (Some(document_id), Some(source_name)) => Some(Citation {
                    source_name,
                    document_id: document_id.clone(),
                    revision: memory.document_revision.unwrap_or_default(),
                    chunk_index: memory.chunk_index.unwrap_or_default(),
                }),
                _ => None,
            };
            Ok((memory, citation))
        })?;
        for row in rows {
            let (memory, citation) = row?;
            found.insert(memory.id.clone(), (memory, citation));
        }
    }
    Ok(found)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory;
    use std::cell::Cell;

    fn database() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        crate::db::migrations::run(&conn).unwrap();
        conn.execute(
            "INSERT INTO projects (id, name, kind, created_at, updated_at) VALUES ('client', 'Client', 'client', 1, 1)",
            [],
        )
        .unwrap();
        conn
    }

    fn summary(conn: &Connection, content: &str, embedding: &[f32]) {
        let session = crate::db::ensure_session(conn).unwrap();
        let guard = memory::summary_guard(conn, &crate::db::selected_project_id(conn)).unwrap();
        assert!(memory::store_summary_checked(conn, &guard, &session, content, embedding).unwrap());
    }

    fn query<'a>(text: &'a str, embedding: Option<&'a [f32]>) -> RetrievalQuery<'a> {
        RetrievalQuery {
            project_id: "personal",
            text,
            embedding,
            embedding_model: crate::embed::DEFAULT_MODEL,
            min_similarity: 0.5,
            top_k: 5,
        }
    }

    fn import(conn: &Connection, source: &str, chunks: &[(&str, [f32; 2])]) -> memory::WriteGuard {
        let guard = memory::begin_document_import(conn, "personal", source).unwrap();
        for (index, (content, embedding)) in chunks.iter().enumerate() {
            assert!(
                memory::store_document_revision_chunk(conn, &guard, index, content, embedding)
                    .unwrap()
            );
        }
        guard
    }

    #[test]
    fn query_terms_are_quoted_and_stopwords_dropped() {
        assert_eq!(
            fts_query("What is \"the\" Zanzibar-ferry NEAR(x) time*?").as_deref(),
            Some("\"zanzibar\" OR \"ferry\" OR \"near\" OR \"time\"")
        );
        assert_eq!(fts_query("is it the?"), None);
        assert_eq!(fts_query(""), None);
    }

    #[test]
    fn keyword_recalls_memory_without_embedding_similarity() {
        let conn = database();
        summary(&conn, "The Zanzibar ferry leaves at noon", &[0.0, 1.0]);
        summary(&conn, "Unrelated chat about lunch", &[0.0, 1.0]);
        let embedding = [1.0, 0.0];
        let hits = retrieve(
            &conn,
            &query("When does the zanzibar ferry go?", Some(&embedding)),
        );
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].memory.content, "The Zanzibar ferry leaves at noon");
        assert_eq!(hits[0].lexical_rank, Some(1));
        assert_eq!(hits[0].vector_score, None);
        assert!(hits[0].citation.is_none());
        // Lexical recall also works when no query embedding is available.
        assert_eq!(retrieve(&conn, &query("zanzibar", None)).len(), 1);
        let block = prompt_block(&hits, |_| "today".into()).unwrap();
        assert!(block.contains("• today: The Zanzibar ferry leaves at noon"));
    }

    #[test]
    fn hybrid_fusion_is_deterministic_and_rewards_agreement() {
        let conn = database();
        summary(&conn, "budget review for the harbour project", &[1.0, 0.1]);
        summary(&conn, "harbour notes", &[0.0, 1.0]);
        summary(&conn, "quarterly figures", &[1.0, 0.0]);
        for _ in 0..3 {
            summary(&conn, "tied vector twin", &[0.6, 0.6]);
        }
        let embedding = [1.0, 0.0];
        let first = retrieve(&conn, &query("harbour budget", Some(&embedding)));
        // Lexical and vector agreement outranks a single-list match.
        assert_eq!(
            first[0].memory.content,
            "budget review for the harbour project"
        );
        assert!(first[0].lexical_rank.is_some() && first[0].vector_score.is_some());
        let ids = |hits: &[RetrievalHit]| -> Vec<String> {
            hits.iter().map(|hit| hit.memory.id.clone()).collect()
        };
        for _ in 0..5 {
            let again = retrieve(&conn, &query("harbour budget", Some(&embedding)));
            assert_eq!(ids(&again), ids(&first));
        }
        let mut sorted = first.clone();
        sorted.sort_by(compare_hits);
        assert_eq!(ids(&sorted), ids(&first));
        assert!(retrieve(
            &conn,
            &RetrievalQuery {
                top_k: 0,
                ..query("harbour", None)
            }
        )
        .is_empty());
    }

    #[test]
    fn retrieval_never_crosses_projects() {
        let conn = database();
        summary(&conn, "personal walrus diary", &[1.0, 0.0]);
        crate::db::set_setting(&conn, "selected_project_id", "client").unwrap();
        summary(&conn, "client walrus contract", &[1.0, 0.0]);
        memory::store_document_chunk(&conn, "client walrus annex", &[1.0, 0.0], "a.txt", "client")
            .unwrap();
        crate::db::set_setting(&conn, "selected_project_id", "personal").unwrap();

        let embedding = [1.0, 0.0];
        let personal = retrieve(&conn, &query("walrus", Some(&embedding)));
        assert_eq!(personal.len(), 1);
        assert_eq!(personal[0].memory.project_id, "personal");
        let client = retrieve(
            &conn,
            &RetrievalQuery {
                project_id: "client",
                ..query("walrus", Some(&embedding))
            },
        );
        assert_eq!(client.len(), 2);
        assert!(client.iter().all(|hit| hit.memory.project_id == "client"));
    }

    #[test]
    fn vectors_from_other_models_or_dimensions_are_excluded() {
        let conn = database();
        crate::db::set_setting(&conn, "embedding_model", "other-model").unwrap();
        summary(&conn, "made by another model", &[1.0, 0.0]);
        crate::db::set_setting(&conn, "embedding_model", crate::embed::DEFAULT_MODEL).unwrap();
        summary(&conn, "three dimensional", &[1.0, 0.0, 0.0]);
        summary(&conn, "current model", &[1.0, 0.0]);

        let embedding = [1.0, 0.0];
        let hits = retrieve(&conn, &query("", Some(&embedding)));
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].memory.content, "current model");
        assert_eq!(
            hits[0].memory.embedding_model.as_deref(),
            Some(crate::embed::DEFAULT_MODEL)
        );
        assert_eq!(hits[0].memory.embedding_dim, Some(2));
        // Incompatible rows remain reachable lexically.
        let lexical = retrieve(&conn, &query("another model", Some(&embedding)));
        assert!(lexical
            .iter()
            .any(|hit| hit.memory.content == "made by another model"));
        assert!(retrieve(
            &conn,
            &RetrievalQuery {
                embedding_model: "",
                ..query("", Some(&embedding))
            }
        )
        .is_empty());
    }

    #[test]
    fn reimport_creates_new_revision_and_removes_old_chunks_from_index() {
        let conn = database();
        let embedding = [1.0, 0.0];
        let first = import(
            &conn,
            "brief.txt",
            &[("alpha intro", [1.0, 0.0]), ("alpha body", [1.0, 0.0])],
        );
        // Staged chunks are invisible until the revision is activated.
        assert!(retrieve(&conn, &query("alpha", Some(&embedding))).is_empty());
        assert!(memory::finish_document_import(&conn, &first, Some("hash-1")).unwrap());
        let documents = memory::list_documents(&conn);
        assert_eq!((documents[0].revision, documents[0].chunk_count), (1, 2));
        let document_id = documents[0].document_id.clone();

        // A failed re-import keeps revision 1 searchable.
        let failed = import(&conn, "brief.txt", &[("gamma draft", [1.0, 0.0])]);
        memory::abandon_document_import(&conn, &failed).unwrap();
        assert_eq!(retrieve(&conn, &query("alpha", None)).len(), 2);

        let second = import(&conn, "brief.txt", &[("bravo replacement", [1.0, 0.0])]);
        assert!(!memory::store_document_revision_chunk(
            &conn,
            &first,
            2,
            "late alpha",
            &[1.0, 0.0]
        )
        .unwrap());
        assert!(memory::finish_document_import(&conn, &second, Some("hash-2")).unwrap());

        let documents = memory::list_documents(&conn);
        assert_eq!(documents.len(), 1);
        assert_eq!(documents[0].document_id, document_id);
        assert_eq!(documents[0].revision, 3);
        assert_eq!(documents[0].chunk_count, 1);
        assert_eq!(documents[0].content_sha256.as_deref(), Some("hash-2"));
        assert_eq!(documents[0].incompatible_chunks, 0);
        assert!(retrieve(&conn, &query("alpha gamma", Some(&embedding)))
            .iter()
            .all(|hit| hit.memory.content == "bravo replacement"));
        let indexed: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM memory_fts WHERE memory_fts MATCH 'alpha OR gamma'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(indexed, 0);
        let rows: i64 = conn
            .query_row("SELECT COUNT(*) FROM memory_fts_rows", [], |row| row.get(0))
            .unwrap();
        assert_eq!(rows, 1);

        let hits = retrieve(&conn, &query("bravo", Some(&embedding)));
        assert_eq!(
            hits[0].citation,
            Some(Citation {
                source_name: "brief.txt".into(),
                document_id,
                revision: 3,
                chunk_index: 0,
            })
        );
        let block = prompt_block(&hits, |_| String::new()).unwrap();
        assert!(block.contains("[brief.txt · rev 3 · part 1]: bravo replacement"));
        assert!(block.contains("name its bracketed source"));
    }

    struct CountingSource {
        conn: Connection,
        acquisitions: Cell<usize>,
    }
    impl ConnectionSource for CountingSource {
        fn with_connection<R>(&self, f: impl FnOnce(&Connection) -> R) -> Option<R> {
            self.acquisitions.set(self.acquisitions.get() + 1);
            Some(f(&self.conn))
        }
    }

    #[test]
    fn large_project_scan_is_paged_and_exact() {
        const ROWS: usize = 5_000;
        let conn = database();
        let session = crate::db::ensure_session(&conn).unwrap();
        let tx = conn.unchecked_transaction().unwrap();
        for index in 0..ROWS {
            let needle = index == 1_234;
            let content = if needle {
                "needle".to_string()
            } else {
                format!("filler {index}")
            };
            let embedding = if needle {
                [1.0, 0.0]
            } else {
                [index as f32 / ROWS as f32, 1.0]
            };
            memory::store_memory(
                &tx,
                &session,
                &content,
                &embedding,
                "session_summary",
                crate::embed::DEFAULT_MODEL,
            )
            .unwrap();
        }
        tx.commit().unwrap();
        let source = CountingSource {
            conn,
            acquisitions: Cell::new(0),
        };
        let embedding = [1.0, 0.0];
        let started = std::time::Instant::now();
        let hits = retrieve(
            &source,
            &RetrievalQuery {
                min_similarity: -1.0,
                ..query("needle", Some(&embedding))
            },
        );
        let elapsed = started.elapsed();
        assert_eq!(hits[0].memory.content, "needle");
        assert_eq!(hits[0].vector_score, Some(1.0));
        // Lock scope is one page at a time: lexical + every page + hydration.
        let pages = ROWS.div_ceil(VECTOR_PAGE_SIZE);
        assert!(
            source.acquisitions.get() >= pages + 2,
            "{}",
            source.acquisitions.get()
        );
        // Generous bound for debug builds; catches accidental quadratic work.
        assert!(elapsed < std::time::Duration::from_secs(20), "{elapsed:?}");
    }
}

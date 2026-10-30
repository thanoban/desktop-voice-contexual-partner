use rusqlite::Connection;

pub fn run(conn: &Connection) -> anyhow::Result<()> {
    conn.execute_batch("PRAGMA journal_mode=WAL;")?;
    conn.execute_batch("PRAGMA foreign_keys=ON;")?;
    conn.execute_batch("PRAGMA synchronous=NORMAL;")?;

    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS schema_version (
            version INTEGER PRIMARY KEY
        );
    ",
    )?;

    let version: i64 = conn
        .query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_version",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);

    if version < 1 {
        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS settings (
                key   TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS sessions (
                id         TEXT PRIMARY KEY,
                started_at INTEGER NOT NULL,
                ended_at   INTEGER
            );

            CREATE TABLE IF NOT EXISTS turns (
                id          TEXT PRIMARY KEY,
                session_id  TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
                role        TEXT NOT NULL CHECK(role IN ('user', 'assistant')),
                content     TEXT NOT NULL,
                created_at  INTEGER NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_turns_session ON turns(session_id, created_at);

            INSERT OR IGNORE INTO settings VALUES ('endpoint',        'http://localhost:11434');
            INSERT OR IGNORE INTO settings VALUES ('model',           'llama3.2:8b');
            INSERT OR IGNORE INTO settings VALUES ('companion_name',  'Amy');
            INSERT OR IGNORE INTO settings VALUES ('personality',     'gentle');
            INSERT OR IGNORE INTO settings VALUES ('piper_binary',    '');
            INSERT OR IGNORE INTO settings VALUES ('piper_voice',     'en_US-amy-medium');
            INSERT OR IGNORE INTO settings VALUES ('onboarding_done', 'false');

            INSERT INTO schema_version VALUES (1);
        ",
        )?;
    }

    if version < 2 {
        conn.execute_batch(
            "
            INSERT OR IGNORE INTO settings VALUES ('audio_input_device',  'default');
            INSERT OR IGNORE INTO settings VALUES ('whisper_binary',       '');
            INSERT OR IGNORE INTO settings VALUES ('whisper_model',        '');
            INSERT OR IGNORE INTO settings VALUES ('window_context_auto',  'false');
            INSERT OR IGNORE INTO settings VALUES ('voice_threshold_db',   '-30');

            INSERT INTO schema_version VALUES (2);
        ",
        )?;
    }

    if version < 3 {
        conn.execute_batch(
            "
            INSERT OR IGNORE INTO settings VALUES ('voice_speed',           '1.0');
            INSERT OR IGNORE INTO settings VALUES ('voice_expressiveness',   '0.667');

            INSERT INTO schema_version VALUES (3);
        ",
        )?;
    }

    if version < 4 {
        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS memories (
                id          TEXT PRIMARY KEY,
                session_id  TEXT,
                content     TEXT NOT NULL,
                embedding   BLOB NOT NULL,
                memory_type TEXT NOT NULL DEFAULT 'session_summary',
                created_at  INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_memories_created ON memories(created_at DESC);

            INSERT OR IGNORE INTO settings VALUES ('embedding_model', 'nomic-embed-text');

            INSERT INTO schema_version VALUES (4);
        ",
        )?;
    }

    if version < 5 {
        conn.execute_batch(
            "
            ALTER TABLE memories ADD COLUMN source_file TEXT;
            CREATE INDEX IF NOT EXISTS idx_memories_source ON memories(source_file);

            INSERT OR IGNORE INTO settings VALUES ('custom_system_prompt', '');

            INSERT INTO schema_version VALUES (5);
        ",
        )?;
    }

    if version < 6 {
        conn.execute_batch(
            "
            INSERT OR IGNORE INTO settings VALUES ('window_context_allowed', 'unset');

            INSERT INTO schema_version VALUES (6);
        ",
        )?;
    }

    if version < 7 {
        let transaction = conn.unchecked_transaction()?;
        transaction.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS projects (
                id         TEXT PRIMARY KEY,
                name       TEXT NOT NULL,
                kind       TEXT NOT NULL CHECK(kind IN ('personal', 'client', 'project')),
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            );

            INSERT OR IGNORE INTO projects (id, name, kind, created_at, updated_at)
            VALUES ('personal', 'Personal', 'personal', 0, 0);

            INSERT OR IGNORE INTO settings VALUES ('selected_project_id', 'personal');
            INSERT OR IGNORE INTO settings VALUES ('partner_mode', 'company');

            ALTER TABLE sessions ADD COLUMN project_id TEXT NOT NULL DEFAULT 'personal';
            ALTER TABLE sessions ADD COLUMN partner_mode TEXT NOT NULL DEFAULT 'company';

            ALTER TABLE turns ADD COLUMN input_kind TEXT NOT NULL DEFAULT 'text';
            ALTER TABLE turns ADD COLUMN state TEXT NOT NULL DEFAULT 'completed';
            ALTER TABLE turns ADD COLUMN provider_ref TEXT;
            ALTER TABLE turns ADD COLUMN completed_at INTEGER;
            ALTER TABLE turns ADD COLUMN spoken_characters INTEGER;

            ALTER TABLE memories ADD COLUMN project_id TEXT NOT NULL DEFAULT 'personal';
            ALTER TABLE memories ADD COLUMN provenance_json TEXT;

            CREATE INDEX IF NOT EXISTS idx_sessions_project_started
                ON sessions(project_id, started_at DESC);
            CREATE INDEX IF NOT EXISTS idx_memories_project_created
                ON memories(project_id, created_at DESC);

            INSERT INTO schema_version VALUES (7);
            ",
        )?;
        transaction.commit()?;
    }

    // v9: document identity/revisions, embedding provenance, FTS5 lexical
    // index and retention settings. Independent of the v8 integration tables.
    if version < 9 {
        let transaction = conn.unchecked_transaction()?;
        transaction.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS documents (
                id              TEXT PRIMARY KEY,
                project_id      TEXT NOT NULL,
                source_name     TEXT NOT NULL,
                active_revision INTEGER NOT NULL DEFAULT 0,
                latest_revision INTEGER NOT NULL DEFAULT 0,
                content_sha256  TEXT,
                created_at      INTEGER NOT NULL,
                updated_at      INTEGER NOT NULL,
                UNIQUE(project_id, source_name)
            );

            ALTER TABLE memories ADD COLUMN embedding_model TEXT;
            ALTER TABLE memories ADD COLUMN embedding_dim INTEGER;
            ALTER TABLE memories ADD COLUMN document_id TEXT;
            ALTER TABLE memories ADD COLUMN document_revision INTEGER;
            ALTER TABLE memories ADD COLUMN chunk_index INTEGER;

            INSERT INTO documents (id, project_id, source_name, active_revision, latest_revision, created_at, updated_at)
            SELECT lower(hex(randomblob(16))), project_id, source_file, 1, 1, MIN(created_at), MAX(created_at)
              FROM memories
             WHERE memory_type = 'document' AND source_file IS NOT NULL
             GROUP BY project_id, source_file;

            UPDATE memories
               SET document_id = (SELECT d.id FROM documents d
                                   WHERE d.project_id = memories.project_id
                                     AND d.source_name = memories.source_file),
                   document_revision = 1
             WHERE memory_type = 'document' AND source_file IS NOT NULL;

            UPDATE memories SET chunk_index = ordered.position
              FROM (SELECT id, ROW_NUMBER() OVER (PARTITION BY document_id ORDER BY created_at, rowid) - 1 AS position
                      FROM memories WHERE document_id IS NOT NULL) AS ordered
             WHERE memories.id = ordered.id;

            CREATE INDEX IF NOT EXISTS idx_memories_vector_scope
                ON memories(project_id, embedding_model, embedding_dim);
            CREATE INDEX IF NOT EXISTS idx_memories_document
                ON memories(document_id, document_revision, chunk_index);

            -- Stable integer keys for the FTS index: implicit memories rowids
            -- can change on VACUUM because the table has a TEXT primary key.
            CREATE TABLE IF NOT EXISTS memory_fts_rows (
                fts_rowid INTEGER PRIMARY KEY,
                memory_id TEXT NOT NULL UNIQUE
            );
            -- Contentless: the index does not keep a second copy of the text.
            CREATE VIRTUAL TABLE IF NOT EXISTS memory_fts USING fts5(
                body,
                content = '',
                contentless_delete = 1,
                tokenize = 'porter unicode61 remove_diacritics 2'
            );

            INSERT INTO memory_fts_rows (memory_id)
            SELECT id FROM memories ORDER BY created_at, rowid;
            INSERT INTO memory_fts (rowid, body)
            SELECT r.fts_rowid, m.content FROM memories m JOIN memory_fts_rows r ON r.memory_id = m.id;

            CREATE TRIGGER IF NOT EXISTS memories_fts_insert AFTER INSERT ON memories BEGIN
                INSERT INTO memory_fts_rows (memory_id) VALUES (new.id);
                INSERT INTO memory_fts (rowid, body)
                VALUES ((SELECT fts_rowid FROM memory_fts_rows WHERE memory_id = new.id), new.content);
            END;
            CREATE TRIGGER IF NOT EXISTS memories_fts_delete AFTER DELETE ON memories BEGIN
                DELETE FROM memory_fts
                 WHERE rowid = (SELECT fts_rowid FROM memory_fts_rows WHERE memory_id = old.id);
                DELETE FROM memory_fts_rows WHERE memory_id = old.id;
            END;
            CREATE TRIGGER IF NOT EXISTS memories_fts_update AFTER UPDATE OF content ON memories BEGIN
                DELETE FROM memory_fts
                 WHERE rowid = (SELECT fts_rowid FROM memory_fts_rows WHERE memory_id = old.id);
                INSERT INTO memory_fts (rowid, body)
                VALUES ((SELECT fts_rowid FROM memory_fts_rows WHERE memory_id = new.id), new.content);
            END;

            INSERT OR IGNORE INTO settings VALUES ('memory_retention_days', '0');
            INSERT OR IGNORE INTO settings VALUES ('history_retention_days', '0');

            INSERT INTO schema_version VALUES (9);
            ",
        )?;
        // Legacy vectors were produced by the configured model. A blank setting
        // leaves provenance unknown, which keeps those rows lexical-only.
        transaction.execute(
            "UPDATE memories
                SET embedding_model = NULLIF(TRIM(COALESCE(
                        (SELECT value FROM settings WHERE key = 'embedding_model'), ?1)), ''),
                    embedding_dim = length(embedding) / 4
              WHERE length(embedding) > 0 AND length(embedding) % 4 = 0",
            [crate::embed::DEFAULT_MODEL],
        )?;
        transaction.execute(
            "UPDATE memories SET embedding_dim = NULL WHERE embedding_model IS NULL",
            [],
        )?;
        transaction.commit()?;
    }

    // Future migrations go here as `if version < N { ... }` blocks.
    // Never edit a migration that has already been applied.

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::run;

    #[test]
    fn migration_seven_assigns_legacy_rows_to_personal_scope() {
        let connection = rusqlite::Connection::open_in_memory().expect("database");
        connection
            .execute_batch(
                "
                CREATE TABLE schema_version (version INTEGER PRIMARY KEY);
                INSERT INTO schema_version VALUES (6);
                CREATE TABLE settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);
                CREATE TABLE sessions (id TEXT PRIMARY KEY, started_at INTEGER NOT NULL, ended_at INTEGER);
                CREATE TABLE turns (
                    id TEXT PRIMARY KEY,
                    session_id TEXT NOT NULL,
                    role TEXT NOT NULL,
                    content TEXT NOT NULL,
                    created_at INTEGER NOT NULL
                );
                CREATE TABLE memories (
                    id TEXT PRIMARY KEY,
                    session_id TEXT,
                    content TEXT NOT NULL,
                    embedding BLOB NOT NULL,
                    memory_type TEXT NOT NULL DEFAULT 'session_summary',
                    created_at INTEGER NOT NULL,
                    source_file TEXT
                );
                INSERT INTO sessions VALUES ('legacy-session', 1, NULL);
                INSERT INTO turns VALUES ('legacy-turn', 'legacy-session', 'user', 'hello', 1);
                INSERT INTO memories VALUES ('legacy-memory', 'legacy-session', 'summary', X'', 'session_summary', 1, NULL);
                ",
            )
            .expect("legacy schema");

        run(&connection).expect("migration");

        let version: i64 = connection
            .query_row("SELECT MAX(version) FROM schema_version", [], |row| {
                row.get(0)
            })
            .expect("version");
        let session_project: String = connection
            .query_row(
                "SELECT project_id FROM sessions WHERE id = 'legacy-session'",
                [],
                |row| row.get(0),
            )
            .expect("session project");
        let memory_project: String = connection
            .query_row(
                "SELECT project_id FROM memories WHERE id = 'legacy-memory'",
                [],
                |row| row.get(0),
            )
            .expect("memory project");

        // Later migrations also run; this test covers the v7 scope backfill.
        assert!(version >= 7);
        assert_eq!(session_project, "personal");
        assert_eq!(memory_project, "personal");
    }

    #[test]
    fn migration_nine_backfills_documents_provenance_and_lexical_index() {
        let connection = rusqlite::Connection::open_in_memory().expect("database");
        // Schema exactly as v7 left it, populated like a real user database.
        connection
            .execute_batch(
                "
                CREATE TABLE schema_version (version INTEGER PRIMARY KEY);
                INSERT INTO schema_version VALUES (1), (2), (3), (4), (5), (6), (7);
                CREATE TABLE settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);
                INSERT INTO settings VALUES ('embedding_model', 'legacy-embed');
                INSERT INTO settings VALUES ('selected_project_id', 'personal');
                CREATE TABLE sessions (
                    id TEXT PRIMARY KEY, started_at INTEGER NOT NULL, ended_at INTEGER,
                    project_id TEXT NOT NULL DEFAULT 'personal',
                    partner_mode TEXT NOT NULL DEFAULT 'company'
                );
                CREATE TABLE turns (
                    id TEXT PRIMARY KEY,
                    session_id TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
                    role TEXT NOT NULL, content TEXT NOT NULL, created_at INTEGER NOT NULL,
                    input_kind TEXT NOT NULL DEFAULT 'text', state TEXT NOT NULL DEFAULT 'completed',
                    provider_ref TEXT, completed_at INTEGER, spoken_characters INTEGER
                );
                CREATE TABLE memories (
                    id TEXT PRIMARY KEY, session_id TEXT, content TEXT NOT NULL,
                    embedding BLOB NOT NULL, memory_type TEXT NOT NULL DEFAULT 'session_summary',
                    created_at INTEGER NOT NULL, source_file TEXT,
                    project_id TEXT NOT NULL DEFAULT 'personal', provenance_json TEXT
                );
                CREATE TABLE projects (
                    id TEXT PRIMARY KEY, name TEXT NOT NULL, kind TEXT NOT NULL,
                    created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL
                );
                INSERT INTO projects VALUES ('personal', 'Personal', 'personal', 0, 0);
                INSERT INTO projects VALUES ('client', 'Client', 'client', 0, 0);
                INSERT INTO sessions VALUES ('s1', 1, NULL, 'personal', 'company');
                INSERT INTO turns (id, session_id, role, content, created_at) VALUES ('t1', 's1', 'user', 'hello', 1);
                INSERT INTO memories VALUES ('summary', 's1', 'We planned the harbour trip', X'0000803F00000000', 'session_summary', 5, NULL, 'personal', NULL);
                INSERT INTO memories VALUES ('chunk-b', NULL, 'second part about tides', X'000000000000803F', 'document', 20, 'brief.txt', 'personal', NULL);
                INSERT INTO memories VALUES ('chunk-a', NULL, 'first part about harbour', X'0000803F00000000', 'document', 10, 'brief.txt', 'personal', NULL);
                INSERT INTO memories VALUES ('client-chunk', NULL, 'client harbour notes', X'0000803F00000000', 'document', 10, 'brief.txt', 'client', NULL);
                INSERT INTO memories VALUES ('empty', 's1', 'no vector', X'', 'session_summary', 6, NULL, 'personal', NULL);
                INSERT INTO memories VALUES ('ragged', 's1', 'ragged vector', X'0000803F00', 'session_summary', 7, NULL, 'personal', NULL);
                ",
            )
            .expect("v7 schema");

        run(&connection).expect("migration");

        let version: i64 = connection
            .query_row("SELECT MAX(version) FROM schema_version", [], |row| {
                row.get(0)
            })
            .expect("version");
        assert_eq!(version, 9);

        let provenance = |id: &str| -> (Option<String>, Option<i64>) {
            connection
                .query_row(
                    "SELECT embedding_model, embedding_dim FROM memories WHERE id = ?1",
                    [id],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .expect("memory")
        };
        assert_eq!(
            provenance("summary"),
            (Some("legacy-embed".into()), Some(2))
        );
        assert_eq!(provenance("empty"), (None, None));
        assert_eq!(provenance("ragged"), (None, None));

        let documents: Vec<(String, String, i64)> = connection
            .prepare("SELECT id, project_id, active_revision FROM documents ORDER BY project_id")
            .unwrap()
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
            .unwrap()
            .map(Result::unwrap)
            .collect();
        assert_eq!(documents.len(), 2);
        assert_ne!(documents[0].0, documents[1].0);
        assert!(documents.iter().all(|document| document.2 == 1));
        let chunk = |id: &str| -> (String, i64, i64) {
            connection
                .query_row(
                    "SELECT document_id, document_revision, chunk_index FROM memories WHERE id = ?1",
                    [id],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
                )
                .expect("chunk")
        };
        let personal = &documents[1].0;
        assert_eq!(chunk("chunk-a"), (personal.clone(), 1, 0));
        assert_eq!(chunk("chunk-b"), (personal.clone(), 1, 1));

        let lexical: Vec<String> = connection
            .prepare(
                "SELECT r.memory_id FROM memory_fts JOIN memory_fts_rows r ON r.fts_rowid = memory_fts.rowid
                 WHERE memory_fts MATCH 'harbour' ORDER BY r.memory_id",
            )
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .map(Result::unwrap)
            .collect();
        assert_eq!(lexical, ["chunk-a", "client-chunk", "summary"]);
        let retention: Option<String> =
            crate::db::get_setting(&connection, "memory_retention_days");
        assert_eq!(retention.as_deref(), Some("0"));
        // Re-running is a no-op.
        run(&connection).expect("idempotent migration");
    }

    #[test]
    fn bundled_sqlite_supports_contentless_delete_fts5() {
        let connection = rusqlite::Connection::open_in_memory().expect("database");
        connection
            .execute_batch(
                "CREATE VIRTUAL TABLE probe USING fts5(body, content='', contentless_delete=1);
                 INSERT INTO probe (rowid, body) VALUES (1, 'lexical recall works');
                 DELETE FROM probe WHERE rowid = 1;",
            )
            .expect("FTS5 with contentless_delete is compiled in");
        let remaining: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM probe WHERE probe MATCH 'recall'",
                [],
                |row| row.get(0),
            )
            .expect("match");
        assert_eq!(remaining, 0);
    }
}

use rusqlite::Connection;

pub fn run(conn: &Connection) -> anyhow::Result<()> {
    conn.execute_batch("PRAGMA journal_mode=WAL;")?;
    conn.execute_batch("PRAGMA foreign_keys=ON;")?;
    conn.execute_batch("PRAGMA synchronous=NORMAL;")?;

    conn.execute_batch("
        CREATE TABLE IF NOT EXISTS schema_version (
            version INTEGER PRIMARY KEY
        );
    ")?;

    let version: i64 = conn
        .query_row("SELECT COALESCE(MAX(version), 0) FROM schema_version", [], |r| r.get(0))
        .unwrap_or(0);

    if version < 1 {
        conn.execute_batch("
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
        ")?;
    }

    if version < 2 {
        conn.execute_batch("
            INSERT OR IGNORE INTO settings VALUES ('audio_input_device',  'default');
            INSERT OR IGNORE INTO settings VALUES ('whisper_binary',       '');
            INSERT OR IGNORE INTO settings VALUES ('whisper_model',        '');
            INSERT OR IGNORE INTO settings VALUES ('window_context_auto',  'false');
            INSERT OR IGNORE INTO settings VALUES ('voice_threshold_db',   '-30');

            INSERT INTO schema_version VALUES (2);
        ")?;
    }

    if version < 3 {
        conn.execute_batch("
            INSERT OR IGNORE INTO settings VALUES ('voice_speed',           '1.0');
            INSERT OR IGNORE INTO settings VALUES ('voice_expressiveness',   '0.667');

            INSERT INTO schema_version VALUES (3);
        ")?;
    }

    if version < 4 {
        conn.execute_batch("
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
        ")?;
    }

    if version < 5 {
        conn.execute_batch("
            ALTER TABLE memories ADD COLUMN source_file TEXT;
            CREATE INDEX IF NOT EXISTS idx_memories_source ON memories(source_file);

            INSERT OR IGNORE INTO settings VALUES ('custom_system_prompt', '');

            INSERT INTO schema_version VALUES (5);
        ")?;
    }

    if version < 6 {
        conn.execute_batch("
            INSERT OR IGNORE INTO settings VALUES ('window_context_allowed', 'unset');

            INSERT INTO schema_version VALUES (6);
        ")?;
    }

    // Future migrations go here as `if version < N { ... }` blocks.
    // Never edit a migration that has already been applied.

    Ok(())
}

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

    // Future migrations go here as `if version < N { ... }` blocks.
    // Never edit a migration that has already been applied.

    Ok(())
}

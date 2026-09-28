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

        assert_eq!(version, 7);
        assert_eq!(session_project, "personal");
        assert_eq!(memory_project, "personal");
    }
}

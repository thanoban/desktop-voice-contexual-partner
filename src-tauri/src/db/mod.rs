pub mod migrations;
pub mod projects;

use rusqlite::{Connection, Result};

pub const DEFAULT_PROJECT_ID: &str = "personal";

pub fn get_setting(conn: &Connection, key: &str) -> Option<String> {
    conn.query_row(
        "SELECT value FROM settings WHERE key = ?1",
        rusqlite::params![key],
        |row| row.get(0),
    )
    .ok()
}

pub fn set_setting(conn: &Connection, key: &str, value: &str) -> Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
        rusqlite::params![key, value],
    )?;
    Ok(())
}

pub fn get_all_settings(conn: &Connection) -> Vec<(String, String)> {
    let mut stmt = conn.prepare("SELECT key, value FROM settings").unwrap();
    stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })
    .unwrap()
    .filter_map(|r| r.ok())
    .collect()
}

pub fn ensure_session(conn: &Connection) -> Result<String> {
    let cutoff = now_ms() - (4 * 60 * 60 * 1000); // reuse session within 4h
    let (project_id, partner_mode) = current_scope(conn);
    let existing: Result<String> = conn.query_row(
        "SELECT id FROM sessions WHERE started_at > ?1 AND ended_at IS NULL AND project_id = ?2 \
         ORDER BY started_at DESC LIMIT 1",
        rusqlite::params![cutoff, project_id],
        |row| row.get(0),
    );
    match existing {
        Ok(id) => Ok(id),
        Err(_) => {
            let id = uuid::Uuid::new_v4().to_string();
            conn.execute(
                "INSERT INTO sessions (id, started_at, project_id, partner_mode) VALUES (?1, ?2, ?3, ?4)",
                rusqlite::params![id, now_ms(), project_id, partner_mode],
            )?;
            Ok(id)
        }
    }
}

pub fn create_session(conn: &Connection) -> Result<String> {
    let timestamp = now_ms();
    let id = uuid::Uuid::new_v4().to_string();
    let (project_id, partner_mode) = current_scope(conn);
    let transaction = conn.unchecked_transaction()?;
    transaction.execute(
        "UPDATE sessions SET ended_at = ?1 WHERE ended_at IS NULL AND project_id = ?2",
        rusqlite::params![timestamp, project_id],
    )?;
    transaction.execute(
        "INSERT INTO sessions (id, started_at, project_id, partner_mode) VALUES (?1, ?2, ?3, ?4)",
        rusqlite::params![id, timestamp, project_id, partner_mode],
    )?;
    transaction.commit()?;
    Ok(id)
}

#[allow(dead_code)]
pub fn close_session(conn: &Connection, session_id: &str) -> Result<()> {
    conn.execute(
        "UPDATE sessions SET ended_at = ?1 WHERE id = ?2",
        rusqlite::params![now_ms(), session_id],
    )?;
    Ok(())
}

pub fn save_turn(conn: &Connection, session_id: &str, role: &str, content: &str) -> Result<()> {
    save_turn_with_state(conn, session_id, role, content, "text", "completed")
}

pub fn save_turn_with_state(
    conn: &Connection,
    session_id: &str,
    role: &str,
    content: &str,
    input_kind: &str,
    state: &str,
) -> Result<()> {
    let id = uuid::Uuid::new_v4().to_string();
    let timestamp = now_ms();
    conn.execute(
        "INSERT INTO turns (id, session_id, role, content, created_at, input_kind, state, completed_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        rusqlite::params![id, session_id, role, content, timestamp, input_kind, state, timestamp],
    )?;
    Ok(())
}

pub fn selected_project_id(conn: &Connection) -> String {
    get_setting(conn, "selected_project_id").unwrap_or_else(|| DEFAULT_PROJECT_ID.into())
}

pub fn require_selected_project(
    conn: &Connection,
    expected: &str,
) -> std::result::Result<(), String> {
    if selected_project_id(conn) != expected {
        return Err("Workspace changed. Please retry the action in the selected workspace.".into());
    }
    Ok(())
}

fn current_scope(conn: &Connection) -> (String, String) {
    (
        selected_project_id(conn),
        get_setting(conn, "partner_mode").unwrap_or_else(|| "company".into()),
    )
}

pub fn get_turn_count(conn: &Connection, session_id: &str) -> rusqlite::Result<i64> {
    conn.query_row(
        "SELECT COUNT(*) FROM turns WHERE session_id = ?1",
        rusqlite::params![session_id],
        |row| row.get(0),
    )
}

pub fn get_recent_turns(
    conn: &Connection,
    session_id: &str,
    limit: usize,
) -> Vec<(String, String)> {
    let mut stmt = conn
        .prepare(
            "SELECT role, content FROM turns WHERE session_id = ?1 \
             ORDER BY created_at DESC LIMIT ?2",
        )
        .unwrap();
    let rows: Vec<(String, String)> = stmt
        .query_map(rusqlite::params![session_id, limit as i64], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .unwrap()
        .filter_map(|r| r.ok())
        .collect();
    rows.into_iter().rev().collect()
}

pub struct SessionSummary {
    pub id: String,
    pub started_at: i64,
    pub ended_at: Option<i64>,
    pub turn_count: i64,
    pub first_user_message: Option<String>,
    pub project_id: String,
    pub partner_mode: String,
}

pub fn list_sessions(conn: &Connection) -> Vec<SessionSummary> {
    let project_id = selected_project_id(conn);
    let mut stmt = conn
        .prepare(
            "SELECT s.id, s.started_at, s.ended_at, COUNT(t.id) as turn_count, \
             MIN(CASE WHEN t.role='user' THEN t.content END) as first_msg \
             , s.project_id, s.partner_mode \
             FROM sessions s \
             LEFT JOIN turns t ON t.session_id = s.id \
             WHERE s.project_id = ?1 \
             GROUP BY s.id \
             ORDER BY s.started_at DESC \
             LIMIT 100",
        )
        .unwrap();
    stmt.query_map(rusqlite::params![project_id], |row| {
        Ok(SessionSummary {
            id: row.get(0)?,
            started_at: row.get(1)?,
            ended_at: row.get(2)?,
            turn_count: row.get(3)?,
            first_user_message: row.get(4)?,
            project_id: row.get(5)?,
            partner_mode: row.get(6)?,
        })
    })
    .unwrap()
    .filter_map(|r| r.ok())
    .collect()
}

pub fn get_all_turns(conn: &Connection, session_id: &str) -> Vec<(String, String, i64)> {
    let project_id = selected_project_id(conn);
    let mut stmt = conn
        .prepare(
            "SELECT t.role, t.content, t.created_at FROM turns t \
             JOIN sessions s ON s.id = t.session_id \
             WHERE t.session_id = ?1 AND s.project_id = ?2 ORDER BY t.created_at ASC, t.rowid ASC",
        )
        .unwrap();
    stmt.query_map(rusqlite::params![session_id, project_id], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, i64>(2)?,
        ))
    })
    .unwrap()
    .filter_map(|r| r.ok())
    .collect()
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64
}

#[cfg(test)]
mod tests {
    use super::{create_session, ensure_session};

    fn database() -> rusqlite::Connection {
        let connection = rusqlite::Connection::open_in_memory().expect("in-memory database");
        super::migrations::run(&connection).expect("migrations");
        connection
    }

    #[test]
    fn stale_workspace_request_is_rejected() {
        let connection = database();
        super::require_selected_project(&connection, "personal").expect("current scope");
        let project = super::projects::create(&connection, "Client", "client").expect("project");
        super::projects::select(&connection, &project.id).expect("select project");
        assert!(super::require_selected_project(&connection, "personal").is_err());
        super::require_selected_project(&connection, &project.id).expect("current scope");
    }

    #[test]
    fn ensure_session_reuses_recent_open_session() {
        let connection = database();
        let first = ensure_session(&connection).expect("first session");
        let second = ensure_session(&connection).expect("second session");
        assert_eq!(first, second);
    }

    #[test]
    fn create_session_closes_previous_session() {
        let connection = database();
        let first = ensure_session(&connection).expect("first session");
        let second = create_session(&connection).expect("new session");
        assert_ne!(first, second);

        let ended_at: Option<i64> = connection
            .query_row(
                "SELECT ended_at FROM sessions WHERE id = ?1",
                rusqlite::params![first],
                |row| row.get(0),
            )
            .expect("old session exists");
        assert!(ended_at.is_some());
    }
}

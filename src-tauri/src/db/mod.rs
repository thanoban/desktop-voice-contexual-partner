pub mod migrations;

use rusqlite::{Connection, Result};

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
    let mut stmt = conn
        .prepare("SELECT key, value FROM settings")
        .unwrap();
    stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })
    .unwrap()
    .filter_map(|r| r.ok())
    .collect()
}

pub fn ensure_session(conn: &Connection) -> Result<String> {
    let cutoff = now_ms() - (4 * 60 * 60 * 1000); // reuse session within 4h
    let existing: Result<String> = conn.query_row(
        "SELECT id FROM sessions WHERE started_at > ?1 AND ended_at IS NULL \
         ORDER BY started_at DESC LIMIT 1",
        rusqlite::params![cutoff],
        |row| row.get(0),
    );
    match existing {
        Ok(id) => Ok(id),
        Err(_) => {
            let id = uuid::Uuid::new_v4().to_string();
            conn.execute(
                "INSERT INTO sessions (id, started_at) VALUES (?1, ?2)",
                rusqlite::params![id, now_ms()],
            )?;
            Ok(id)
        }
    }
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
    let id = uuid::Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO turns (id, session_id, role, content, created_at) VALUES (?1,?2,?3,?4,?5)",
        rusqlite::params![id, session_id, role, content, now_ms()],
    )?;
    Ok(())
}

pub fn get_turn_count(conn: &Connection, session_id: &str) -> rusqlite::Result<i64> {
    conn.query_row(
        "SELECT COUNT(*) FROM turns WHERE session_id = ?1",
        rusqlite::params![session_id],
        |row| row.get(0),
    )
}

pub fn get_recent_turns(conn: &Connection, session_id: &str, limit: usize) -> Vec<(String, String)> {
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
}

pub fn list_sessions(conn: &Connection) -> Vec<SessionSummary> {
    let mut stmt = conn
        .prepare(
            "SELECT s.id, s.started_at, s.ended_at, COUNT(t.id) as turn_count, \
             MIN(CASE WHEN t.role='user' THEN t.content END) as first_msg \
             FROM sessions s \
             LEFT JOIN turns t ON t.session_id = s.id \
             GROUP BY s.id \
             ORDER BY s.started_at DESC \
             LIMIT 100",
        )
        .unwrap();
    stmt.query_map([], |row| {
        Ok(SessionSummary {
            id: row.get(0)?,
            started_at: row.get(1)?,
            ended_at: row.get(2)?,
            turn_count: row.get(3)?,
            first_user_message: row.get(4)?,
        })
    })
    .unwrap()
    .filter_map(|r| r.ok())
    .collect()
}

pub fn get_all_turns(conn: &Connection, session_id: &str) -> Vec<(String, String, i64)> {
    let mut stmt = conn
        .prepare(
            "SELECT role, content, created_at FROM turns \
             WHERE session_id = ?1 ORDER BY created_at ASC",
        )
        .unwrap();
    stmt.query_map(rusqlite::params![session_id], |row| {
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

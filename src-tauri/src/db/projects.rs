use rusqlite::{params, Connection};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub kind: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct ProjectSelection {
    pub project_id: String,
    pub session_id: String,
}

pub fn list(conn: &Connection) -> rusqlite::Result<Vec<Project>> {
    let mut statement = conn.prepare(
        "SELECT id, name, kind FROM projects ORDER BY id = 'personal' DESC, name COLLATE NOCASE, id",
    )?;
    let rows = statement.query_map([], |row| {
        Ok(Project {
            id: row.get(0)?,
            name: row.get(1)?,
            kind: row.get(2)?,
        })
    })?;
    rows.collect()
}

pub fn create(conn: &Connection, name: &str, kind: &str) -> Result<Project, String> {
    let name = name.trim();
    if name.is_empty() || name.chars().count() > 100 || name.chars().any(char::is_control) {
        return Err("Use a project name of 1–100 characters without control characters".into());
    }
    if !matches!(kind, "client" | "project") {
        return Err("Project kind must be client or project".into());
    }
    let id = uuid::Uuid::new_v4().to_string();
    let timestamp = crate::domain::now_ms();
    conn.execute(
        "INSERT INTO projects (id, name, kind, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?4)",
        params![id, name, kind, timestamp],
    )
    .map_err(|error| error.to_string())?;
    Ok(Project {
        id,
        name: name.into(),
        kind: kind.into(),
    })
}

pub fn select(conn: &Connection, project_id: &str) -> Result<ProjectSelection, String> {
    let transaction = conn
        .unchecked_transaction()
        .map_err(|error| error.to_string())?;
    let exists: bool = transaction
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM projects WHERE id = ?1)",
            params![project_id],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())?;
    if !exists {
        return Err("Project does not exist".into());
    }
    let previous = super::selected_project_id(&transaction);
    if previous == project_id {
        let session_id = super::ensure_session(&transaction).map_err(|error| error.to_string())?;
        transaction.commit().map_err(|error| error.to_string())?;
        return Ok(ProjectSelection {
            project_id: project_id.into(),
            session_id,
        });
    }
    let timestamp = crate::domain::now_ms();
    let session_id = uuid::Uuid::new_v4().to_string();
    let mode = super::get_setting(&transaction, "partner_mode").unwrap_or_else(|| "company".into());
    transaction
        .execute(
            "UPDATE sessions SET ended_at = ?1 WHERE ended_at IS NULL AND project_id IN (?2, ?3)",
            params![timestamp, previous, project_id],
        )
        .map_err(|error| error.to_string())?;
    super::set_setting(&transaction, "selected_project_id", project_id)
        .map_err(|error| error.to_string())?;
    transaction.execute(
        "INSERT INTO sessions (id, started_at, project_id, partner_mode) VALUES (?1, ?2, ?3, ?4)",
        params![session_id, timestamp, project_id, mode],
    ).map_err(|error| error.to_string())?;
    transaction.commit().map_err(|error| error.to_string())?;
    Ok(ProjectSelection {
        project_id: project_id.into(),
        session_id,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn database() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        crate::db::migrations::run(&conn).unwrap();
        conn
    }

    #[test]
    fn unknown_project_keeps_previous_session_and_selection() {
        let conn = database();
        let original = crate::db::ensure_session(&conn).unwrap();
        assert!(select(&conn, "missing").is_err());
        assert_eq!(crate::db::selected_project_id(&conn), "personal");
        assert_eq!(crate::db::ensure_session(&conn).unwrap(), original);
    }

    #[test]
    fn switching_creates_fresh_session_and_scopes_history() {
        let conn = database();
        let original = crate::db::ensure_session(&conn).unwrap();
        crate::db::save_turn(&conn, &original, "user", "private companion chat").unwrap();
        let client = create(&conn, " Client A ", "client").unwrap();
        assert_eq!(client.name, "Client A");
        let selection = select(&conn, &client.id).unwrap();
        assert_ne!(selection.session_id, original);
        assert!(crate::db::get_all_turns(&conn, &original).is_empty());
        assert!(crate::db::list_sessions(&conn)
            .iter()
            .all(|session| session.project_id == client.id));
        assert_eq!(
            select(&conn, &client.id).unwrap().session_id,
            selection.session_id
        );
        let personal = select(&conn, "personal").unwrap();
        assert_ne!(personal.session_id, original);
        assert_eq!(crate::db::get_all_turns(&conn, &original).len(), 1);
    }

    #[test]
    fn creation_rejects_invalid_names_and_kinds() {
        let conn = database();
        assert!(create(&conn, " ", "client").is_err());
        assert!(create(&conn, "a\nb", "project").is_err());
        assert!(create(&conn, &"a".repeat(101), "project").is_err());
        assert!(create(&conn, "Other personal", "personal").is_err());
        assert_eq!(list(&conn).unwrap().len(), 1);
    }
}

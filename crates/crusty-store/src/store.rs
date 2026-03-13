//! Main storage interface.

use crate::error::{Result, StoreError};
use crate::history::HistoryEntry;
use crate::migrations;
use crusty_core::collection::Collection;
use crusty_core::environment::Environment;
use rusqlite::Connection;
use uuid::Uuid;

/// The main storage interface backed by SQLite.
pub struct Store {
    conn: Connection,
}

impl Store {
    /// Open a store at the given file path, running migrations if needed.
    pub fn open(path: &str) -> Result<Self> {
        let conn = Connection::open(path)?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;
        migrations::run_migrations(&conn)?;
        Ok(Self { conn })
    }

    /// Open an in-memory store (useful for testing).
    pub fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        migrations::run_migrations(&conn)?;
        Ok(Self { conn })
    }

    // --- Collections ---

    /// Save a collection (insert or update).
    pub fn save_collection(&self, collection: &Collection) -> Result<()> {
        let data = serde_json::to_string(collection)?;
        self.conn.execute(
            "INSERT INTO collections (id, name, data, updated_at)
             VALUES (?1, ?2, ?3, datetime('now'))
             ON CONFLICT(id) DO UPDATE SET name = ?2, data = ?3, updated_at = datetime('now')",
            rusqlite::params![collection.id.to_string(), collection.name, data],
        )?;
        Ok(())
    }

    /// Get a collection by ID.
    pub fn get_collection(&self, id: &Uuid) -> Result<Collection> {
        let data: String = self
            .conn
            .query_row(
                "SELECT data FROM collections WHERE id = ?1",
                [id.to_string()],
                |row| row.get(0),
            )
            .map_err(|_| StoreError::NotFound {
                kind: "collection".into(),
                id: id.to_string(),
            })?;
        Ok(serde_json::from_str(&data)?)
    }

    /// List all collections (id and name only for sidebar).
    pub fn list_collections(&self) -> Result<Vec<(Uuid, String)>> {
        let mut stmt = self.conn.prepare("SELECT id, name FROM collections ORDER BY name")?;
        let rows = stmt.query_map([], |row| {
            let id_str: String = row.get(0)?;
            let name: String = row.get(1)?;
            Ok((id_str, name))
        })?;

        let mut collections = Vec::new();
        for row in rows {
            let (id_str, name) = row?;
            if let Ok(id) = Uuid::parse_str(&id_str) {
                collections.push((id, name));
            }
        }
        Ok(collections)
    }

    /// Delete a collection by ID.
    pub fn delete_collection(&self, id: &Uuid) -> Result<()> {
        self.conn.execute(
            "DELETE FROM collections WHERE id = ?1",
            [id.to_string()],
        )?;
        Ok(())
    }

    // --- Environments ---

    /// Save an environment (insert or update).
    pub fn save_environment(&self, env: &Environment) -> Result<()> {
        let data = serde_json::to_string(env)?;
        self.conn.execute(
            "INSERT INTO environments (id, name, data, updated_at)
             VALUES (?1, ?2, ?3, datetime('now'))
             ON CONFLICT(id) DO UPDATE SET name = ?2, data = ?3, updated_at = datetime('now')",
            rusqlite::params![env.id.to_string(), env.name, data],
        )?;
        Ok(())
    }

    /// Get an environment by ID.
    pub fn get_environment(&self, id: &Uuid) -> Result<Environment> {
        let data: String = self
            .conn
            .query_row(
                "SELECT data FROM environments WHERE id = ?1",
                [id.to_string()],
                |row| row.get(0),
            )
            .map_err(|_| StoreError::NotFound {
                kind: "environment".into(),
                id: id.to_string(),
            })?;
        Ok(serde_json::from_str(&data)?)
    }

    /// List all environments.
    pub fn list_environments(&self) -> Result<Vec<(Uuid, String)>> {
        let mut stmt = self.conn.prepare("SELECT id, name FROM environments ORDER BY name")?;
        let rows = stmt.query_map([], |row| {
            let id_str: String = row.get(0)?;
            let name: String = row.get(1)?;
            Ok((id_str, name))
        })?;

        let mut envs = Vec::new();
        for row in rows {
            let (id_str, name) = row?;
            if let Ok(id) = Uuid::parse_str(&id_str) {
                envs.push((id, name));
            }
        }
        Ok(envs)
    }

    /// Delete an environment by ID.
    pub fn delete_environment(&self, id: &Uuid) -> Result<()> {
        self.conn.execute(
            "DELETE FROM environments WHERE id = ?1",
            [id.to_string()],
        )?;
        Ok(())
    }

    // --- History ---

    /// Record a request in history.
    pub fn add_history(&self, entry: &HistoryEntry) -> Result<()> {
        self.conn.execute(
            "INSERT INTO history (id, method, url, status, duration_ms, request_data, response_data, timestamp)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            rusqlite::params![
                entry.id,
                entry.method,
                entry.url,
                entry.status,
                entry.duration_ms,
                entry.request_data,
                entry.response_data,
                entry.timestamp,
            ],
        )?;
        Ok(())
    }

    /// Get recent history entries, most recent first.
    pub fn list_history(&self, limit: u32) -> Result<Vec<HistoryEntry>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, method, url, status, duration_ms, request_data, response_data, timestamp
             FROM history ORDER BY timestamp DESC LIMIT ?1",
        )?;

        let rows = stmt.query_map([limit], |row| {
            Ok(HistoryEntry {
                id: row.get(0)?,
                method: row.get(1)?,
                url: row.get(2)?,
                status: row.get(3)?,
                duration_ms: row.get(4)?,
                request_data: row.get(5)?,
                response_data: row.get(6)?,
                timestamp: row.get(7)?,
            })
        })?;

        let mut entries = Vec::new();
        for row in rows {
            entries.push(row?);
        }
        Ok(entries)
    }

    /// Clear all history.
    pub fn clear_history(&self) -> Result<()> {
        self.conn.execute("DELETE FROM history", [])?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crusty_core::collection::Collection;
    use crusty_core::environment::Environment;
    use crusty_core::request::RequestDefinition;

    #[test]
    fn test_collection_crud() {
        let store = Store::open_in_memory().unwrap();

        let mut col = Collection::new("Test API");
        col.add_request(RequestDefinition::new("Get Users", "https://api.example.com/users"));

        // Save
        store.save_collection(&col).unwrap();

        // Read back
        let loaded = store.get_collection(&col.id).unwrap();
        assert_eq!(loaded.name, "Test API");
        assert_eq!(loaded.request_count(), 1);

        // List
        let list = store.list_collections().unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].1, "Test API");

        // Delete
        store.delete_collection(&col.id).unwrap();
        let list = store.list_collections().unwrap();
        assert!(list.is_empty());
    }

    #[test]
    fn test_environment_crud() {
        let store = Store::open_in_memory().unwrap();

        let mut env = Environment::new("Production");
        env.add_variable("host", "api.example.com");
        env.add_variable("api_key", "secret123");

        store.save_environment(&env).unwrap();

        let loaded = store.get_environment(&env.id).unwrap();
        assert_eq!(loaded.name, "Production");
        assert_eq!(loaded.variables.len(), 2);

        let list = store.list_environments().unwrap();
        assert_eq!(list.len(), 1);

        store.delete_environment(&env.id).unwrap();
        assert!(store.list_environments().unwrap().is_empty());
    }

    #[test]
    fn test_history() {
        let store = Store::open_in_memory().unwrap();

        let entry = HistoryEntry {
            id: Uuid::new_v4().to_string(),
            method: "GET".into(),
            url: "https://api.example.com/users".into(),
            status: Some(200),
            duration_ms: Some(150),
            request_data: "{}".into(),
            response_data: Some("{\"users\":[]}".into()),
            timestamp: "2025-01-01T00:00:00Z".into(),
        };

        store.add_history(&entry).unwrap();

        let history = store.list_history(10).unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].method, "GET");
        assert_eq!(history[0].status, Some(200));

        store.clear_history().unwrap();
        assert!(store.list_history(10).unwrap().is_empty());
    }

    #[test]
    fn test_collection_update() {
        let store = Store::open_in_memory().unwrap();

        let mut col = Collection::new("Original");
        store.save_collection(&col).unwrap();

        col.name = "Updated".into();
        col.add_request(RequestDefinition::new("New", "https://example.com"));
        store.save_collection(&col).unwrap();

        let loaded = store.get_collection(&col.id).unwrap();
        assert_eq!(loaded.name, "Updated");
        assert_eq!(loaded.request_count(), 1);

        // Should still be just one collection
        assert_eq!(store.list_collections().unwrap().len(), 1);
    }
}

use rusqlite::{Connection, Result};

use log::debug;
use log::{error, info};

use std::fs;

use std::rc::Rc;
use std::time::{SystemTime, UNIX_EPOCH};

/// Represents a path entry in the database
/// id: auto increment primary key
/// path: the file path
/// date: the timestamp when the path was added (in seconds since EPOCH)
#[derive(Debug, Clone)]
pub(crate) struct Path {
    pub(crate) id: i64,
    pub(crate) date: i64,
    pub(crate) path: String,
}

/// Represents a shortcut entry in the database
/// id: auto increment primary key
/// name: the name of the shortcut
/// path: the file path associated with the shortcut
#[derive(Debug, Clone)]
pub(crate) struct Shortcut {
    pub(crate) id: i64,
    pub(crate) name: String,
    pub(crate) path: String,
}

/// Store struct to manage database connection and operations
/// db_conn: the SQLite database connection
#[derive(Debug)]
pub(crate) struct Store {
    db_conn: Rc<Connection>,
}

impl Store {
    /// Creates a new Store instance and initializes the database if it doesn't exist.
    ///
    /// ### Parameters
    /// dir_path: the path to the SQLite database file
    ///
    /// ### Returns
    /// a new Store instance
    pub(crate) fn new(dir_path: &std::path::Path) -> Store {
        info!("db file={}", dir_path.display());

        if !dir_path.exists() {
            if let Some(parent) = dir_path.parent() {
                if let Err(e) = fs::create_dir_all(parent) {
                    error!("Failed to create directory '{}': {}", parent.display(), e);
                    panic!("Directory creation failed");
                }
            }
        }
        let db_exists = dir_path.exists();

        let store = Store {
            db_conn: match Connection::open(dir_path) {
                Ok(conn) => Rc::new(conn),
                Err(err) => {
                    error!(
                        "Failed to open connection to database '{}': {}",
                        dir_path.display(),
                        err
                    );
                    panic!("Failed to open connection to database")
                }
            },
        };

        if !db_exists {
            store.init_schema();
        }

        store
    }

    /// Initializes the database schema by creating necessary tables and indexes.
    /// If the tables already exist, this function does nothing.
    fn init_schema(&self) {
        info!("Initializing the database schema");

        let script = "CREATE TABLE IF NOT EXISTS paths (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            path TEXT NOT NULL,
            date INTEGER NOT NULL
        );
        CREATE INDEX IF NOT EXISTS paths_date ON paths (date);
        CREATE TABLE IF NOT EXISTS shortcuts (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            path TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS shortcuts_name ON shortcuts (name);
        ";
        debug!("Schema initialization");
        if let Err(err) = self.db_conn.execute_batch(script) {
            error!("init_schema: {}", err);
            panic!("init_schema")
        }
    }

    /// Adds a new path to the database with the current timestamp.
    /// If the path already exists, it is updated with the new timestamp.
    //
    /// ### Parameters
    /// path: the file path to add
    ///
    /// ### Returns
    /// Ok(()) if the operation was successful, otherwise an error
    pub(crate) fn add_path(&self, path: &String) -> Result<(), rusqlite::Error> {
        debug!("add_path path={}", path);
        self.add_path_with_time(
            path,
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        )
    }

    /// Adds a new path to the database with a specified timestamp.
    /// If the path already exists, it is updated with the new timestamp.
    ///
    /// ### Parameters
    /// path: the file path to add
    /// epoc: the timestamp to associate with the path (in seconds since EPOCH)
    ///
    /// ### Returns
    /// Ok(()) if the operation was successful, otherwise an error
    pub(crate) fn add_path_with_time(
        &self,
        path: &String,
        epoc: u64,
    ) -> Result<(), rusqlite::Error> {
        debug!("add_path_with_time path={} epoch={}", path, epoc);
        {
            let mut stmt = self.db_conn.prepare("DELETE FROM paths WHERE path=(?1)")?;
            if let Err(err) = stmt.execute([path]) {
                error!("Failed to delete path '{}': {}", path, err);
                return Err(err);
            }
        }
        {
            let mut stmt = self
                .db_conn
                .prepare("INSERT INTO paths (path, date) VALUES ((?1),(?2))")?;
            stmt.execute([path, &format!("{}", epoc)])
                .map_err(|e| {
                    error!("Failed to insert path '{}' time'{}: {}", path, epoc, e);
                    e
                })
                .map(|_l| ())
        }
    }

    /// Deletes a path from the database by its ID.
    ///
    /// ### Parameters
    /// id: the ID of the path to delete
    ///
    /// ### Returns
    /// Ok(()) if the operation was successful, otherwise an error
    pub(crate) fn delete_path_by_id(&self, id: i64) -> Result<(), rusqlite::Error> {
        let mut stmt = self.db_conn.prepare("DELETE FROM paths WHERE id=(?1)")?;
        stmt.execute([id])
            .map_err(|e| {
                error!("Failed to delete path by id '{}',{}", id, e);
                e
            })
            .map(|_l: usize| ())
    }

    /// Lists paths from the database with pagination and optional filtering.
    /// The results are ordered by date (descending) and ID (descending).
    /// If `like_text` is provided, only paths containing the text are returned.
    ///
    /// ### Parameters
    /// pos: the starting position (offset) for pagination
    /// len: the number of paths to return
    /// like_text: optional text to filter paths (if empty, no filtering is applied)
    ///
    /// ### Returns
    /// A vector of Path entries if the operation was successful, otherwise an error.
    pub(crate) fn list_paths(
        &self,
        pos: usize,
        len: usize,
        like_text: &str,
    ) -> Result<Vec<Path>, rusqlite::Error> {
        debug!("list_paths pos={} len={} like_text={}", pos, len, like_text);

        let mut params: Vec<String> = vec![];
        let mut sql = String::from("SELECT id, path, date FROM paths");

        if !like_text.is_empty() {
            sql.push_str(" WHERE path like '%' || (?1) || '%'");
            sql.push_str(" ORDER BY date desc, id desc LIMIT (?2) OFFSET (?3)");
            params.push(like_text.to_string());
        } else {
            sql.push_str(" ORDER BY date desc, id desc LIMIT (?1) OFFSET (?2)");
        }
        params.push(format!("{}", len));
        params.push(format!("{}", pos));

        let mut stmt = match self.db_conn.prepare(sql.as_str()) {
            Ok(stmt) => stmt,
            Err(e) => {
                error!("list_paths failed in prepare {}: {}", sql, e);
                return Err(e);
            }
        };

        let rows = match stmt.query_map(rusqlite::params_from_iter(params), |row| {
            Ok(Path {
                id: row.get(0)?,
                path: row.get(1)?,
                date: row.get(2)?,
            })
        }) {
            Ok(rows) => rows,
            Err(e) => {
                error!("list_paths failed in query_map: {}", e);
                return Err(e);
            }
        };

        let mut paths = Vec::new();
        for path in rows {
            paths.push(path?);
        }
        Ok(paths)
    }

    /// Adds a new shortcut to the database.
    /// If a shortcut with the same name already exists, it is deleted before adding the new one.
    ///
    /// ### Parameters
    /// name: the name of the shortcut
    /// path: the file path associated with the shortcut
    ///
    /// ### Returns
    /// Ok(()) if the operation was successful, otherwise an error.
    pub(crate) fn add_shortcut(&self, name: &String, path: &String) -> Result<(), rusqlite::Error> {
        debug!("add_shortcut: {} {}", name, path);
        self.delete_shortcut(name)?;
        let mut stmt = self
            .db_conn
            .prepare("INSERT INTO shortcuts (name, path) VALUES ((?1),(?2))")?;
        stmt.execute([name, path])
            .map_err(|e| {
                error!(
                    "Failed to insert shortcuts name='{}' time='{}': {}",
                    name, path, e
                );
                e
            })
            .map(|_l| ())
    }

    /// Deletes a shortcut from the database by its name.
    /// If the shortcut does not exist, no action is taken.
    ///
    /// ### Parameters
    /// name: the name of the shortcut to delete
    ///
    /// ### Returns
    /// Ok(()) if the operation was successful, otherwise an error.
    pub(crate) fn delete_shortcut(&self, name: &String) -> Result<(), rusqlite::Error> {
        let mut stmt = self
            .db_conn
            .prepare("DELETE FROM shortcuts WHERE name=(?1)")?;
        if let Err(err) = stmt.execute([name]) {
            error!("Failed to delete shortcut '{}': {}", name, err);
            return Err(err);
        }
        Ok(())
    }

    /// Deletes a shortcut from the database by its ID.
    /// If the shortcut does not exist, no action is taken.
    ///
    /// ### Parameters
    /// id: the ID of the shortcut to delete
    ///
    /// ### Returns
    /// Ok(()) if the operation was successful, otherwise an error.
    pub(crate) fn delete_shortcut_by_id(&self, id: i64) -> Result<(), rusqlite::Error> {
        let mut stmt = self
            .db_conn
            .prepare("DELETE FROM shortcuts WHERE id=(?1)")?;
        stmt.execute([id])
            .map_err(|e| {
                error!("Failed to delete shortcuts by id '{}',{}", id, e);
                e
            })
            .map(|_l: usize| ())
    }

    /// Finds a shortcut in the database by its name.
    ///
    //// ### Parameters
    /// name: the name of the shortcut to find
    ///
    /// ### Returns
    /// Some(path) if the shortcut is found, otherwise None.
    pub(crate) fn find_shortcut(&self, name: &String) -> Option<String> {
        debug!("find_shortcut {}", name);

        let mut stmt = match self
            .db_conn
            .prepare("SELECT path FROM shortcuts WHERE name=(?1)")
        {
            Ok(stmt) => stmt,
            Err(e) => {
                error!("find_shortcut failed in prepare: {}", e);
                return None;
            }
        };
        let rrows = stmt.query_map([name], |row| row.get::<_, String>(0));
        match rrows {
            Ok(mut rows) => rows.next().and_then(|row| row.ok()),
            Err(e) => {
                error!("find_shortcut: {}", e);
                None
            }
        }
    }

    /// Lists shortcuts from the database with pagination and optional filtering.
    /// The results are ordered by name (ascending) and ID (descending).
    /// If `like_text` is provided, only shortcuts with names or paths containing the text are returned.
    ///
    /// ### Parameters
    /// pos: the starting position (offset) for pagination
    /// len: the number of shortcuts to return
    /// like_text: optional text to filter shortcuts (if empty, no filtering is applied)
    ///
    /// ### Returns
    /// A vector of Shortcut entries if the operation was successful, otherwise an error.
    pub(crate) fn list_shortcuts(
        &self,
        pos: usize,
        len: usize,
        like_text: &str,
    ) -> Result<Vec<Shortcut>, rusqlite::Error> {
        debug!("list_shortcuts pos={} len={} text={}", pos, len, like_text);

        let mut sql = String::from("SELECT id, name, path FROM shortcuts");
        let mut params: Vec<String> = vec![];
        if !like_text.is_empty() {
            sql.push_str(" WHERE path like '%' || (?1) || '%' OR name like '%' || (?1) || '%'");
            sql.push_str(" ORDER BY name asc, id desc LIMIT (?2) OFFSET (?3)");
            params.push(like_text.to_string());
        } else {
            sql.push_str(" ORDER BY name asc, id desc LIMIT (?1) OFFSET (?2)");
        }
        params.push(format!("{}", len));
        params.push(format!("{}", pos));

        let mut stmt = match self.db_conn.prepare(sql.as_str()) {
            Ok(stmt) => stmt,
            Err(e) => {
                error!("list_shortcuts failed in prepare {}: {}", sql, e);
                return Err(e);
            }
        };

        let rows = match stmt.query_map(rusqlite::params_from_iter(params), |row| {
            Ok(Shortcut {
                id: row.get(0)?,
                name: row.get(1)?,
                path: row.get(2)?,
            })
        }) {
            Ok(rows) => rows,
            Err(e) => {
                error!("list_shortcuts failed in query_map: {}", e);
                return Err(e);
            }
        };

        let mut shortcuts = Vec::new();
        for shortcut in rows {
            shortcuts.push(shortcut?);
        }
        Ok(shortcuts)
    }

    /// Lists all shortcuts from the database.
    /// The results are ordered by name (ascending) and ID (descending).
    ///
    /// ### Returns
    /// A vector of all Shortcut entries if the operation was successful, otherwise an error.
    pub(crate) fn list_all_shortcuts(&self) -> Result<Vec<Shortcut>, rusqlite::Error> {
        let sql = String::from("SELECT id, name, path FROM shortcuts ORDER BY name asc, id desc");

        let mut stmt = match self.db_conn.prepare(sql.as_str()) {
            Ok(stmt) => stmt,
            Err(e) => {
                error!("list_paths failed in prepare {}: {}", sql, e);
                return Err(e);
            }
        };

        let rows = match stmt.query_map([], |row| {
            Ok(Shortcut {
                id: row.get(0)?,
                name: row.get(1)?,
                path: row.get(2)?,
            })
        }) {
            Ok(rows) => rows,
            Err(e) => {
                error!("list_all_shortcuts failed in query_map: {}", e);
                return Err(e);
            }
        };

        let mut shortcuts = Vec::new();
        for shortcut in rows {
            shortcuts.push(shortcut?);
        }
        Ok(shortcuts)
    }

    /// Creates an in-memory store for testing purposes.
    #[allow(dead_code)]
    pub(crate) fn setup_test_store() -> Store {
        let store = Store {
            db_conn: Rc::from(Connection::open_in_memory().unwrap()),
        };
        store.init_schema();
        store
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path() {
        let store = Store::setup_test_store();

        let paths = store.list_paths(0, 10, "").unwrap();
        assert_eq!(paths.len(), 0);

        // A single entry
        store.add_path(&"test_path1".to_string()).unwrap();
        let paths = store.list_paths(0, 10, "").unwrap();
        assert_eq!(paths.len(), 1);
        assert_eq!(paths[0].path, "test_path1");

        // Two entries
        store.add_path(&"test_path2".to_string()).unwrap();
        let paths = store.list_paths(0, 10, "").unwrap();
        assert_eq!(paths.len(), 2);
        assert_eq!(paths[0].path, "test_path2");
        assert_eq!(paths[1].path, "test_path1");

        // A third entry with a specified time
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        store
            .add_path_with_time(&"test_path3".to_string(), now + 7)
            .unwrap();
        let paths = store.list_paths(0, 10, "").unwrap();
        assert_eq!(paths.len(), 3);
        assert_eq!(paths[0].path, "test_path3");
        assert_eq!(paths[0].date, now as i64 + 7);
        assert_eq!(paths[1].path, "test_path2");
        assert_eq!(paths[2].path, "test_path1");

        // Delete the one in the middle
        store.delete_path_by_id(paths[1].id).unwrap();
        let paths = store.list_paths(0, 10, "").unwrap();
        assert_eq!(paths.len(), 2);
        assert_eq!(paths[0].path, "test_path3");
        assert_eq!(paths[1].path, "test_path1");

        // Perform a search
        let paths = store.list_paths(0, 10, "3").unwrap();
        assert_eq!(paths.len(), 1);
        assert_eq!(paths[0].path, "test_path3");
    }

    #[test]
    fn test_shortcut() {
        let store = Store::setup_test_store();

        let paths = store.list_shortcuts(0, 10, "").unwrap();
        assert_eq!(paths.len(), 0);

        // A single entry
        store
            .add_shortcut(&"shortcut_1".to_string(), &"/1".to_string())
            .unwrap();
        let shortcuts = store.list_shortcuts(0, 10, "").unwrap();
        assert_eq!(shortcuts.len(), 1);
        assert_eq!(shortcuts[0].name, "shortcut_1");
        assert_eq!(shortcuts[0].path, "/1");

        // Two entries
        store
            .add_shortcut(&"shortcut_2".to_string(), &"/2".to_string())
            .unwrap();
        let shortcuts = store.list_shortcuts(0, 10, "").unwrap();
        assert_eq!(shortcuts.len(), 2);
        assert_eq!(shortcuts[0].name, "shortcut_1");
        assert_eq!(shortcuts[0].path, "/1");
        assert_eq!(shortcuts[1].name, "shortcut_2");
        assert_eq!(shortcuts[1].path, "/2");

        // Perform a search
        let shortcuts = store.list_shortcuts(0, 10, "2").unwrap();
        assert_eq!(shortcuts.len(), 1);
        assert_eq!(shortcuts[0].name, "shortcut_2");
        assert_eq!(shortcuts[0].path, "/2");

        // Delete the one
        let shortcuts = store.list_shortcuts(0, 10, "").unwrap();
        store.delete_shortcut_by_id(shortcuts[1].id).unwrap();
        let shortcuts = store.list_shortcuts(0, 10, "").unwrap();
        assert_eq!(shortcuts.len(), 1);
        assert_eq!(shortcuts[0].name, "shortcut_1");
    }
}

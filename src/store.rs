use rusqlite::{params, Connection, Result};

use log::debug;
use log::{error, info};

use std::fs;

use std::fmt;
use std::rc::Rc;
use std::time::{SystemTime, UNIX_EPOCH};

// Update this when the database schema changes with the max value value of the sql
// files in ../dbschema (e.g. if 1.sql is the latest, this should be 1)
const CURRENT_SCHEMA_VERSION: i64 = 2;

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
    pub(crate) description: Option<String>,
}

impl fmt::Display for Shortcut {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Shortcut {{ id: {}, name: '{}', path: '{}', description: {:?} }}",
            self.id, self.name, self.path, self.description
        )
    }
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
        } else {
            store.upgrade_schema();
        }

        store
    }

    fn set_schema_version(&self, version: i64) {
        match self.db_conn.execute("DELETE FROM version", params![]) {
            Ok(_) => {}
            Err(e) => {
                error!("upgrade_schema failed to clear version table: {}", e);
                panic!("upgrade_schema failed to clear version table")
            }
        }
        self.db_conn
            .execute(
                "INSERT INTO version (version) VALUES (?1)",
                params![version],
            )
            .unwrap();

        info!("Database schema is now at version {}", version);
    }

    /// Initializes the database schema by creating necessary tables and indexes.
    /// If the tables already exist, this function does nothing.
    fn init_schema(&self) {
        info!("Initializing the database schema");

        let script = include_str!("../dbschema/current.sql");
        debug!("Schema initialization");
        if let Err(err) = self.db_conn.execute_batch(script) {
            error!("init_schema: {}", err);
            panic!("init_schema")
        }
        self.set_schema_version(CURRENT_SCHEMA_VERSION);
    }

    fn upgrade_schema(&self) {
        info!("Upgrading the database schema if necessary");

        let mut version: i64 = 0;

        // Find the current version of the schema
        let version = self.find_schema_version();
        info!(
            "db schema version={}, application schema version={}",
            version, CURRENT_SCHEMA_VERSION
        );

        if version >= CURRENT_SCHEMA_VERSION {
            info!("Database schema is up to date");
            return;
        }

        // embed the sql upgrade scripts
        let u = [
            include_str!("../dbschema/1.sql"),
            include_str!("../dbschema/2.sql"),
            // add other upgrade scripts here
        ];

        for v in version..CURRENT_SCHEMA_VERSION {
            let script = u[v as usize];
            info!("Upgrading schema from version {} to {}", v, v + 1);
            debug!("Upgrade script:\n{}", script);
            if let Err(err) = self.db_conn.execute_batch(script) {
                error!("upgrade_schema from {} to {}: {}", v, v + 1, err);
                panic!("upgrade_schema")
            } else {
                info!("Successfully upgraded schema to version {}", v + 1);
            }
        }

        self.set_schema_version(CURRENT_SCHEMA_VERSION);
    }

    fn find_schema_version(&self) -> i64 {
        let version: i64;

        let mut stmt = match self.db_conn.prepare("SELECT version FROM version") {
            Ok(stmt) => stmt,
            Err(e) => {
                error!("find_schema_version failed in prepare: {}", e);
                return 0;
            }
        };

        let rrows = stmt.query_map([], |row| row.get::<_, i64>(0));
        match rrows {
            Err(err) => {
                error!("find_schema_version query_map error: {}", err);
                return 0;
            }
            Ok(mut rows) => {
                if let Some(row) = rows.next() {
                    match row {
                        Ok(v) => {
                            debug!("found version={}", v);
                            version = v;
                        }
                        Err(err) => {
                            error!("find_schema_version row.next error: {}", err);
                            return 0;
                        }
                    }
                } else {
                    error!("find_schema_version returned no rows");
                    return 0;
                }
            }
        }
        version
    }

    /// Adds a new path to the database with the current timestamp.
    /// If the path already exists, it is updated with the new timestamp.
    //
    /// ### Parameters
    /// path: the file path to add
    ///
    /// ### Returns
    /// Ok(()) if the operation was successful, otherwise an error
    pub(crate) fn add_path(&self, path: &str) -> Result<(), rusqlite::Error> {
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
    pub(crate) fn add_path_with_time(&self, path: &str, epoc: u64) -> Result<(), rusqlite::Error> {
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
                    error!("Failed to insert path '{}' time' {}: {}", path, epoc, e);
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
    pub(crate) fn add_shortcut(
        &self,
        name: &str,
        path: &str,
        description: Option<&str>,
    ) -> Result<(), rusqlite::Error> {
        debug!("add_shortcut: {} {}", name, path);
        self.delete_shortcut(name)?;
        self.db_conn
            .execute(
                "INSERT INTO shortcuts (name, path, description) VALUES ((?1),(?2),(?3))",
                (name, path, description),
            )
            .map_err(|e| {
                error!(
                    "Failed to insert shortcuts name='{}' time='{}': {}",
                    name, path, e
                );
                e
            })
            .map(|_l| ())
    }

    /// Updates an existing shortcut in the database by its id.
    /// If the shortcut does not exist, no action is taken.
    ///
    /// ### Parameters
    /// id: the ID of the shortcut to update
    /// name: the new name of the shortcut
    /// path: the new file path associated with the shortcut
    /// description: the new description of the shortcut (optional)
    ///
    /// ### Returns
    /// Ok(()) if the operation was successful, otherwise an error.
    pub(crate) fn update_shortcut(
        &self,
        id: i64,
        name: &str,
        path: &str,
        description: Option<&str>,
    ) -> Result<(), rusqlite::Error> {
        debug!("update_shortcut: id={} name={} path={}", id, name, path);
        self.db_conn
            .execute(
                "UPDATE shortcuts SET name = (?1), path = (?2), description = (?3) WHERE id = (?4)",
                (name, path, description, id),
            )
            .map_err(|e| {
                error!(
                    "Failed to update shortcut id='{}' name='{}' path='{}': {}",
                    id, name, path, e
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
    pub(crate) fn delete_shortcut(&self, name: &str) -> Result<(), rusqlite::Error> {
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
    pub(crate) fn find_shortcut(&self, name: &str) -> Option<Shortcut> {
        debug!("find_shortcut {}", name);

        let mut stmt = match self
            .db_conn
            .prepare("SELECT id, path, description FROM shortcuts WHERE name=(?1)")
        {
            Ok(stmt) => stmt,
            Err(e) => {
                error!("find_shortcut failed in prepare: {}", e);
                return None;
            }
        };

        let oshort = match stmt.query_map([name], |row| {
            Ok(Shortcut {
                id: row.get(0)?,
                name: name.to_string(),
                path: row.get(1)?,
                description: row.get(2)?,
            })
        }) {
            Ok(mut rows) => rows.next().and_then(|row| row.ok()),
            Err(e) => None,
        };
        debug!("find_shortcut {:?}", oshort);
        oshort
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

        let mut sql = String::from("SELECT id, name, path, description FROM shortcuts");
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
            debug!("list_shortcuts row={:?}", row);
            Ok(Shortcut {
                id: row.get(0)?,
                name: row.get(1)?,
                path: row.get(2)?,
                description: row.get(3)?,
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
        debug!("list_all_shortcuts");
        let sql = String::from(
            "SELECT id, name, path, description FROM shortcuts ORDER BY name asc, id desc",
        );

        let mut stmt = match self.db_conn.prepare(sql.as_str()) {
            Ok(stmt) => stmt,
            Err(e) => {
                error!("list_paths failed in prepare {}: {}", sql, e);
                return Err(e);
            }
        };

        let rows = match stmt.query_map([], |row| {
            debug!("list_shortcuts row={:?}", row);
            Ok(Shortcut {
                id: row.get(0)?,
                name: row.get(1)?,
                path: row.get(2)?,
                description: row.get(3)?,
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

impl Clone for Store {
    fn clone(&self) -> Self {
        Store {
            db_conn: Rc::clone(&self.db_conn),
        }
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
        store.add_path("test_path1").unwrap();
        let paths = store.list_paths(0, 10, "").unwrap();
        assert_eq!(paths.len(), 1);
        assert_eq!(paths[0].path, "test_path1");

        // Two entries
        store.add_path("test_path2").unwrap();
        let paths = store.list_paths(0, 10, "").unwrap();
        assert_eq!(paths.len(), 2);
        assert_eq!(paths[0].path, "test_path2");
        assert_eq!(paths[1].path, "test_path1");

        // A third entry with a specified time
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        store.add_path_with_time("test_path3", now + 7).unwrap();
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
            .add_shortcut("shortcut_1", "/1", Some("desc1"))
            .unwrap();
        let shortcuts = store.list_shortcuts(0, 10, "").unwrap();
        assert_eq!(shortcuts.len(), 1);
        assert_eq!(shortcuts[0].name, "shortcut_1");
        assert_eq!(shortcuts[0].path, "/1");
        assert_eq!(shortcuts[0].description, Some("desc1".to_string()));

        // Two entries
        store
            .add_shortcut("shortcut_2", "/2", Some("desc2"))
            .unwrap();
        let shortcuts = store.list_shortcuts(0, 10, "").unwrap();
        assert_eq!(shortcuts.len(), 2);
        assert_eq!(shortcuts[0].name, "shortcut_1");
        assert_eq!(shortcuts[0].path, "/1");
        assert_eq!(shortcuts[0].description, Some("desc1".to_string()));
        assert_eq!(shortcuts[1].name, "shortcut_2");
        assert_eq!(shortcuts[1].path, "/2");
        assert_eq!(shortcuts[1].description, Some("desc2".to_string()));

        // Perform a search
        let shortcuts = store.list_shortcuts(0, 10, "2").unwrap();
        assert_eq!(shortcuts.len(), 1);
        assert_eq!(shortcuts[0].name, "shortcut_2");
        assert_eq!(shortcuts[0].path, "/2");
        assert_eq!(shortcuts[0].description, Some("desc2".to_string()));

        // Delete the one
        let shortcuts = store.list_shortcuts(0, 10, "").unwrap();
        store.delete_shortcut_by_id(shortcuts[1].id).unwrap();
        let shortcuts = store.list_shortcuts(0, 10, "").unwrap();
        assert_eq!(shortcuts.len(), 1);
        assert_eq!(shortcuts[0].name, "shortcut_1");

        // Test empty description
        store.add_shortcut("shortcut_nodesc", "/1", None).unwrap();
        let shortcuts = store.list_shortcuts(0, 10, "").unwrap();
        assert_eq!(shortcuts.len(), 2);
        assert_eq!(shortcuts[0].name, "shortcut_1");
        assert_eq!(shortcuts[1].name, "shortcut_nodesc");
        assert_eq!(shortcuts[1].description, None);

        let shortcuts = store.list_all_shortcuts().unwrap();
        assert_eq!(shortcuts.len(), 2);
        assert_eq!(shortcuts[0].name, "shortcut_1");
        assert_eq!(shortcuts[1].name, "shortcut_nodesc");
        assert_eq!(shortcuts[1].description, None);
    }
}

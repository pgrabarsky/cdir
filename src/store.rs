use rusqlite::{Connection, Result};

use log::debug;
use log::{error, info};

use std::fs;

use std::rc::Rc;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone)]
pub(crate) struct Path {
    pub(crate) id: i64,
    pub(crate) path: String,
    pub(crate) date: i64, // seconds since EPOCH
}

#[derive(Debug, Clone)]
pub(crate) struct Shortcut {
    pub(crate) id: i64,
    pub(crate) name: String,
    pub(crate) path: String,
}

#[derive(Debug)]
pub(crate) struct Store {
    db_conn: Rc<Connection>,
}

impl Store {
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

    pub(crate) fn init_schema(&self) {
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

    pub(crate) fn delete_path_by_id(&self, id: i64) -> Result<(), rusqlite::Error> {
        let mut stmt = self.db_conn.prepare("DELETE FROM paths WHERE id=(?1)")?;
        stmt.execute([id])
            .map_err(|e| {
                error!("Failed to delete path by id '{}',{}", id, e);
                e
            })
            .map(|_l: usize| ())
    }

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

    // fn list_all_paths(&self) -> Result<Vec<Path>, rusqlite::Error> {
    //     let sql = String::from("SELECT id, path, date FROM paths ORDER BY date desc, id desc");
    //
    //     let mut stmt = match self.db_conn.prepare(sql.as_str()) {
    //         Ok(stmt) => stmt,
    //         Err(e) => {
    //             error!("list_paths failed in prepare {}: {}", sql, e);
    //             return Err(e);
    //         }
    //     };
    //
    //     let rows = match stmt.query_map([], |row| {
    //         Ok(Path {
    //             id: row.get(0)?,
    //             path: row.get(1)?,
    //             date: row.get(2)?,
    //         })
    //     }) {
    //         Ok(rows) => rows,
    //         Err(e) => {
    //             error!("list_paths failed in query_map: {}", e);
    //             return Err(e);
    //         }
    //     };
    //
    //     let mut paths = Vec::new();
    //     for path in rows {
    //         paths.push(path?);
    //     }
    //     Ok(paths)
    // }

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
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn setup_test_db() -> Store {
        let store = Store {
            db_conn: Rc::from(Connection::open_in_memory().unwrap()),
        };
        store.init_schema();
        store
    }

    // #[test]
    // fn test_save_and_delete() {
    //     let store = setup_test_db();
    //
    //     let result = store.add_path(&"test_path".to_string());
    //     assert!(result.is_ok());
    //
    //     let paths = store.list_all_paths().unwrap();
    //     assert_eq!(paths.len(), 1);
    //     assert_eq!(paths[0].path, "test_path");
    //
    //     store
    //         .delete_path_by_id(paths[0].id)
    //         .expect("Failed to delete path by id");
    //     let paths = store.list_all_paths().unwrap();
    //     assert_eq!(paths.len(), 0);
    // }

    #[test]
    fn test_list() {
        let store = setup_test_db();

        store.add_path(&"test_path1".to_string()).unwrap();
        store.add_path(&"test_path2".to_string()).unwrap();

        let paths = store.list_paths(0, 1, "").unwrap();
        assert_eq!(paths.len(), 1);
        assert_eq!(paths[0].path, "test_path2");

        let paths = store.list_paths(1, 1, "").unwrap();
        assert_eq!(paths.len(), 1);
        assert_eq!(paths[0].path, "test_path1");
    }

    // #[test]
    // fn test_list_all() {
    //     let store = setup_test_db();
    //
    //     store.add_path(&"test_path1".to_string()).unwrap();
    //     store.add_path(&"test_path2".to_string()).unwrap();
    //
    //     let paths = store.list_all_paths().unwrap();
    //     assert_eq!(paths.len(), 2);
    //     assert_eq!(paths[0].path, "test_path1");
    //     assert_eq!(paths[1].path, "test_path2");
    // }
}

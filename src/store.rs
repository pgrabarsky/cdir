use std::{
    fmt, fs,
    rc::Rc,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use log::{debug, error, info, trace};
use nucleo_matcher::{
    Matcher, Utf32Str,
    pattern::{CaseMatching, Normalization, Pattern},
};
use rusqlite::{Connection, Result, params};

use crate::config::Config;

// Update this when the database schema changes with the max value value of the sql
// files in ../dbschema (e.g. if 1.sql is the latest, this should be 1)
const CURRENT_SCHEMA_VERSION: i64 = 2;

/// Represents a path entry in the database
/// id: auto increment primary key
/// path: the file path
/// date: the timestamp when the path was added (in seconds since EPOCH)
/// shortcut: the optional shortcut associated with this path
#[derive(Debug, Clone)]
pub(crate) struct Path {
    pub(crate) id: i64,
    pub(crate) date: i64,
    pub(crate) path: String,
    pub(crate) shortcut: Option<Shortcut>,
}

impl Path {
    pub fn new(id: i64, path: String, date: i64, shortcuts: &[Shortcut]) -> Self {
        let mut path = Path {
            id,
            path,
            date,
            shortcut: None,
        };
        path.assign_shortcut(shortcuts);
        path
    }

    pub fn assign_shortcut(&mut self, shortcuts: &[Shortcut]) {
        for shortcut in shortcuts {
            if !Self::is_subpath(&shortcut.path, &self.path) {
                continue;
            }
            if let Some(existing_shortcut) = self.shortcut.as_ref()
                && existing_shortcut.path.len() >= shortcut.path.len()
            {
                // existing shortcut is more specific, keep it
                continue;
            }
            self.shortcut = Some(shortcut.clone());
        }
    }

    pub fn is_subpath(base_path: &str, sub_path: &str) -> bool {
        if !sub_path.starts_with(base_path) {
            return false;
        }
        if sub_path.len() == base_path.len() {
            return true;
        }
        return sub_path.as_bytes()[base_path.len()] == std::path::MAIN_SEPARATOR as u8;
    }
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
pub(crate) struct Store {
    db_conn: Rc<Connection>,
    config: Arc<Config>,
}

impl Store {
    /// Creates a new Store instance and initializes the database if it doesn't exist.
    ///
    /// ### Parameters
    /// dir_path: the path to the SQLite database file
    ///
    /// ### Returns
    /// a new Store instance
    pub(crate) fn new(dir_path: &std::path::Path, config: Arc<Config>) -> Store {
        info!("db file={}", dir_path.display());

        if !dir_path.exists()
            && let Some(parent) = dir_path.parent()
            && let Err(e) = fs::create_dir_all(parent)
        {
            error!("Failed to create directory '{}': {}", parent.display(), e);
            panic!("Directory creation failed");
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
            config,
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
        fuzzy: bool,
    ) -> Result<Vec<Path>, rusqlite::Error> {
        debug!(
            "list_paths pos={} len={} like_text={} fuzzy={}",
            pos, len, like_text, fuzzy
        );
        // Retrieve all shortcuts to associate with paths
        let shortcuts = self.list_all_shortcuts().unwrap_or_default();
        if like_text.is_empty() || !fuzzy {
            self.list_path_exact(pos, len, like_text, &shortcuts)
        } else {
            self.list_path_fuzzy(pos, len, like_text, &shortcuts)
        }
    }

    /// Scores a path for fuzzy search based on the provided pattern and matcher.
    /// The score is calculated as the maximum of:
    /// - The path itself
    /// - For each parent shortcut: a concatenation of the path, shortcut name, and description
    ///
    /// ### Parameters
    /// path: the path to score
    /// matches: the pattern to match against
    /// matcher: the matcher to use for matching
    /// buf: a buffer to use for UTF-32 conversion
    /// shortcuts: all available shortcuts for scoring parent shortcuts
    ///
    /// ### Returns
    /// Some(score) if there is a match, otherwise None.
    fn score_path_for_fuzzy_search(
        &self,
        path: &Path,
        matches: &Pattern,
        matcher: &mut Matcher,
        buf: &mut Vec<char>,
        shortcuts: &[Shortcut],
    ) -> Option<u32> {
        // Score the path itself
        let path_str = Utf32Str::new(path.path.as_str(), buf);
        let mut max_score = matches.score(path_str, matcher);

        trace!("Scoring path '{}' initial score={:?}", path.path, max_score);

        if self.config.path_search_include_shortcuts == false {
            return max_score;
        }

        // Score all shortcuts that are parents (prefixes) of this path
        // by combining the path with the shortcut name and description
        for shortcut in shortcuts {
            if !Path::is_subpath(&shortcut.path, &path.path) {
                continue;
            }

            // Build a combined string of path, shortcut name, and description
            let mut combined = path.path.clone();
            combined.push(' ');
            combined.push_str(&shortcut.name);
            if let Some(desc) = &shortcut.description {
                combined.push(' ');
                combined.push_str(desc);
            }

            // Score the combined string
            let combined_str = Utf32Str::new(&combined, buf);
            if let Some(score) = matches.score(combined_str, matcher) {
                trace!("Scoring combined '{}' score={:?}", combined, score);
                max_score = Some(max_score.map_or(score, |m| m.max(score)));
            }
        }

        max_score
    }

    fn list_path_fuzzy(
        &self,
        pos: usize,
        len: usize,
        like_text: &str,
        shortcuts: &[Shortcut],
    ) -> Result<Vec<Path>, rusqlite::Error> {
        debug!(
            "list_path_fuzzy pos={} len={} like_text={}",
            pos, len, like_text
        );

        let sql = String::from("SELECT id, path, date FROM paths ORDER BY date desc, id desc");
        let mut stmt = match self.db_conn.prepare(sql.as_str()) {
            Ok(stmt) => stmt,
            Err(e) => {
                error!("list_paths failed in prepare {}: {}", sql, e);
                return Err(e);
            }
        };
        let params: Vec<String> = vec![];
        let mut matcher = Matcher::new(nucleo_matcher::Config::DEFAULT.match_paths());
        let matches = Pattern::parse(like_text, CaseMatching::Ignore, Normalization::Smart);
        let mut buf = Vec::new();

        let rows = stmt.query_map(rusqlite::params_from_iter(params), |row| {
            let path_str: String = row.get(1)?;
            Ok(Path::new(row.get(0)?, path_str, row.get(2)?, shortcuts))
        });

        // Build the (Path, score) pairs
        let mut scored_paths: Vec<(Path, u32)> = match rows {
            Ok(rows) => rows.filter_map(|row| {
                if let Ok(path) = row {
                    self.score_path_for_fuzzy_search(
                        &path,
                        &matches,
                        &mut matcher,
                        &mut buf,
                        shortcuts,
                    )
                    .map(|score| (path, score))
                } else {
                    None
                }
            }),
            Err(e) => {
                error!("list_paths failed in query_map: {}", e);
                return Err(e);
            }
        }
        .collect();

        // Sort by descending score
        scored_paths.sort_by(|a, b| b.1.cmp(&a.1));

        // Paginate: skip `pos`, take `len`
        let paginated = scored_paths
            .into_iter()
            .skip(pos)
            .take(len)
            .map(|(path, _)| path)
            .collect();

        Ok(paginated)
    }

    fn build_list_path_exact_sql_statement(
        &self,
        pos: usize,
        len: usize,
        like_text: &str,
        shortcuts: &[Shortcut],
    ) -> (String, Vec<String>) {
        let mut params: Vec<String> = vec![];
        let mut sql = String::from("SELECT id, path, date FROM paths");
        if !like_text.is_empty() {
            // Find shortcuts where name or description matches the like_text
            let like_lower = like_text.to_lowercase();

            if self.config.path_search_include_shortcuts {
                let matching_shortcut_paths: Vec<&str> = shortcuts
                    .iter()
                    .filter(|s| {
                        s.name.to_lowercase().contains(&like_lower)
                            || s.description
                                .as_ref()
                                .is_some_and(|d| d.to_lowercase().contains(&like_lower))
                    })
                    .map(|s| s.path.as_str())
                    .collect();

                // Build WHERE clause: path matches like_text OR path starts with any matching shortcut path
                sql.push_str(" WHERE (path LIKE '%' || (?1) || '%'");
                params.push(like_text.to_string());

                // Add OR conditions for each matching shortcut's path
                for (i, shortcut_path) in matching_shortcut_paths.iter().enumerate() {
                    let param_idx = i + 2; // +2 because ?1 is like_text
                    sql.push_str(&format!(" OR path == (?{})", param_idx));
                    sql.push_str(&format!(" OR path LIKE (?{}) || '/' || '%'", param_idx));
                    params.push(shortcut_path.to_string());
                }
                sql.push(')');
            } else {
                sql.push_str(" WHERE path LIKE '%' || (?1) || '%'");
                params.push(like_text.to_string());
            }

            let limit_idx = params.len() + 1;
            let offset_idx = params.len() + 2;
            sql.push_str(&format!(
                " ORDER BY date desc, id desc LIMIT (?{}) OFFSET (?{})",
                limit_idx, offset_idx
            ));
        } else {
            sql.push_str(" ORDER BY date desc, id desc LIMIT (?1) OFFSET (?2)");
        }
        params.push(format!("{}", len));
        params.push(format!("{}", pos));

        debug!("list_path_exact sql={} params={:?}", sql, params);
        (sql, params)
    }

    fn list_path_exact(
        &self,
        pos: usize,
        len: usize,
        like_text: &str,
        shortcuts: &[Shortcut],
    ) -> Result<Vec<Path>, rusqlite::Error> {
        debug!(
            "list_path_exact pos={} len={} like_text={}",
            pos, len, like_text
        );
        let (sql, params) =
            self.build_list_path_exact_sql_statement(pos, len, like_text, shortcuts);

        let mut stmt = match self.db_conn.prepare(sql.as_str()) {
            Ok(stmt) => stmt,
            Err(e) => {
                error!("list_paths failed in prepare {}: {}", sql, e);
                return Err(e);
            }
        };

        let rows = match stmt.query_map(rusqlite::params_from_iter(params), |row| {
            let path_str: String = row.get(1)?;
            Ok(Path::new(row.get(0)?, path_str, row.get(2)?, shortcuts))
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
            Err(_) => None,
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
        fuzzy: bool,
    ) -> Result<Vec<Shortcut>, rusqlite::Error> {
        debug!(
            "list_shortcuts pos={} len={} text={} fuzzy={}",
            pos, len, like_text, fuzzy
        );

        if like_text.is_empty() || !fuzzy {
            self.list_shortcuts_exact(pos, len, like_text)
        } else {
            self.list_shortcuts_fuzzy(pos, len, like_text)
        }
    }

    fn list_shortcuts_fuzzy(
        &self,
        pos: usize,
        len: usize,
        like_text: &str,
    ) -> Result<Vec<Shortcut>, rusqlite::Error> {
        debug!(
            "list_shortcuts_fuzzy pos={} len={} like_text={}",
            pos, len, like_text
        );

        let sql = String::from(
            "SELECT id, name, path, description FROM shortcuts ORDER BY name asc, id desc",
        );
        let mut stmt = match self.db_conn.prepare(sql.as_str()) {
            Ok(stmt) => stmt,
            Err(e) => {
                error!("list_shortcuts failed in prepare {}: {}", sql, e);
                return Err(e);
            }
        };
        let params: Vec<String> = vec![];
        let mut matcher = Matcher::new(nucleo_matcher::Config::DEFAULT.match_paths());
        let matches = Pattern::parse(like_text, CaseMatching::Ignore, Normalization::Smart);
        let mut buf = Vec::new();
        let rows = stmt.query_map(rusqlite::params_from_iter(params), |row| {
            Ok(Shortcut {
                id: row.get(0)?,
                name: row.get(1)?,
                path: row.get(2)?,
                description: row.get(3)?,
            })
        });

        // Build the (Shortcut, score) pairs
        let mut scored_shortcuts: Vec<(Shortcut, u32)> = match rows {
            Ok(rows) => rows.filter_map(|row| {
                if let Ok(shortcut) = row {
                    // Combine name, path, and description
                    let mut s = shortcut.name.clone();
                    s.push(' ');
                    s.push_str(&shortcut.path);
                    if let Some(desc) = &shortcut.description {
                        s.push(' ');
                        s.push_str(desc);
                    }
                    let s = Utf32Str::new(&s, &mut buf);
                    matches.score(s, &mut matcher).map(|score| {
                        trace!("Scoring shortcut '{}' score={:?}", shortcut.name, score);
                        (shortcut, score)
                    })
                } else {
                    None
                }
            }),
            Err(e) => {
                error!("list_shortcuts failed in query_map: {}", e);
                return Err(e);
            }
        }
        .collect();

        // Sort by descending score
        scored_shortcuts.sort_by(|a, b| b.1.cmp(&a.1));

        // Paginate: skip `pos`, take `len`
        let paginated = scored_shortcuts
            .into_iter()
            .skip(pos)
            .take(len)
            .map(|(shortcut, _)| shortcut)
            .collect();
        Ok(paginated)
    }

    fn list_shortcuts_exact(
        &self,
        pos: usize,
        len: usize,
        like_text: &str,
    ) -> Result<Vec<Shortcut>, rusqlite::Error> {
        debug!("list_shortcuts pos={} len={} text={}", pos, len, like_text);

        let mut sql = String::from("SELECT id, name, path, description FROM shortcuts");
        let mut params: Vec<String> = vec![];
        if !like_text.is_empty() {
            sql.push_str(" WHERE path like '%' || (?1) || '%' OR name like '%' || (?1) || '%' OR description like '%' || (?1) || '%'");
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
            trace!("list_shortcuts row={:?}", row);
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
            config: Arc::new(Config::default()),
        };
        store.init_schema();
        store
    }
}

impl Clone for Store {
    fn clone(&self) -> Self {
        Store {
            db_conn: Rc::clone(&self.db_conn),
            config: self.config.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_assign_shortcut() {
        // Test 1: No shortcuts available
        let mut path = Path {
            id: 1,
            path: "/home/user/documents".to_string(),
            date: 0,
            shortcut: None,
        };
        let shortcuts = [];
        path.assign_shortcut(&shortcuts);
        assert!(path.shortcut.is_none());

        // Test 2: Single matching shortcut
        let mut path = Path {
            id: 1,
            path: "/home/user/documents".to_string(),
            date: 0,
            shortcut: None,
        };
        let shortcuts = vec![Shortcut {
            id: 1,
            name: "docs".to_string(),
            path: "/home/user/documents".to_string(),
            description: None,
        }];
        path.assign_shortcut(&shortcuts);
        assert!(path.shortcut.is_some());
        assert_eq!(path.shortcut.as_ref().unwrap().name, "docs");
        assert_eq!(path.shortcut.as_ref().unwrap().path, "/home/user/documents");

        // Test 3: No matching shortcut (path doesn't start with shortcut path)
        let mut path = Path {
            id: 1,
            path: "/var/log/app".to_string(),
            date: 0,
            shortcut: None,
        };
        let shortcuts = vec![Shortcut {
            id: 1,
            name: "docs".to_string(),
            path: "/home/user/documents".to_string(),
            description: None,
        }];
        path.assign_shortcut(&shortcuts);
        assert!(path.shortcut.is_none());

        // Test 4: Multiple matching shortcuts - should prefer the most specific (longest path)
        let mut path = Path {
            id: 1,
            path: "/home/user/documents/projects/rust".to_string(),
            date: 0,
            shortcut: None,
        };
        let shortcuts = vec![
            Shortcut {
                id: 1,
                name: "home".to_string(),
                path: "/home".to_string(),
                description: None,
            },
            Shortcut {
                id: 2,
                name: "docs".to_string(),
                path: "/home/user/documents".to_string(),
                description: None,
            },
            Shortcut {
                id: 3,
                name: "rust".to_string(),
                path: "/home/user/documents/projects/rust".to_string(),
                description: None,
            },
        ];
        path.assign_shortcut(&shortcuts);
        assert!(path.shortcut.is_some());
        assert_eq!(path.shortcut.as_ref().unwrap().name, "rust");
        assert_eq!(
            path.shortcut.as_ref().unwrap().path,
            "/home/user/documents/projects/rust"
        );

        // Test 5: Existing shortcut is more specific - should keep existing
        let mut path = Path {
            id: 1,
            path: "/home/user/documents/projects".to_string(),
            date: 0,
            shortcut: Some(Shortcut {
                id: 5,
                name: "projects".to_string(),
                path: "/home/user/documents/projects".to_string(),
                description: None,
            }),
        };
        let shortcuts = vec![Shortcut {
            id: 1,
            name: "home".to_string(),
            path: "/home".to_string(),
            description: None,
        }];
        path.assign_shortcut(&shortcuts);
        assert_eq!(path.shortcut.as_ref().unwrap().name, "projects");

        // Test 6: Existing shortcut is less specific - should replace
        let mut path = Path {
            id: 1,
            path: "/home/user/documents/projects".to_string(),
            date: 0,
            shortcut: Some(Shortcut {
                id: 1,
                name: "home".to_string(),
                path: "/home".to_string(),
                description: None,
            }),
        };
        let shortcuts = vec![Shortcut {
            id: 2,
            name: "docs".to_string(),
            path: "/home/user/documents".to_string(),
            description: None,
        }];
        path.assign_shortcut(&shortcuts);
        assert_eq!(path.shortcut.as_ref().unwrap().name, "docs");
        assert_eq!(path.shortcut.as_ref().unwrap().path, "/home/user/documents");

        // Test7: Shortcut should not be assigned it if it not an actual full path match
        let mut path = Path {
            id: 1,
            path: "/home/abcd".to_string(),
            date: 0,
            shortcut: None,
        };
        let shortcuts = vec![Shortcut {
            id: 1,
            name: "home".to_string(),
            path: "/home/abc".to_string(),
            description: None,
        }];
        path.assign_shortcut(&shortcuts);
        assert!(path.shortcut.is_none());
    }

    #[test]
    fn test_path() {
        let store = Store::setup_test_store();

        let paths = store.list_paths(0, 10, "", false).unwrap();
        assert_eq!(paths.len(), 0);

        // A single entry
        store.add_path("test_path1").unwrap();
        let paths = store.list_paths(0, 10, "", false).unwrap();
        assert_eq!(paths.len(), 1);
        assert_eq!(paths[0].path, "test_path1");

        // Two entries
        store.add_path("test_path2").unwrap();
        let paths = store.list_paths(0, 10, "", false).unwrap();
        assert_eq!(paths.len(), 2);
        assert_eq!(paths[0].path, "test_path2");
        assert_eq!(paths[1].path, "test_path1");

        // A third entry with a specified time
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        store.add_path_with_time("test_path3", now + 7).unwrap();
        let paths = store.list_paths(0, 10, "", false).unwrap();
        assert_eq!(paths.len(), 3);
        assert_eq!(paths[0].path, "test_path3");
        assert_eq!(paths[0].date, now as i64 + 7);
        assert_eq!(paths[1].path, "test_path2");
        assert_eq!(paths[2].path, "test_path1");

        // Delete the one in the middle
        store.delete_path_by_id(paths[1].id).unwrap();
        let paths = store.list_paths(0, 10, "", false).unwrap();
        assert_eq!(paths.len(), 2);
        assert_eq!(paths[0].path, "test_path3");
        assert_eq!(paths[1].path, "test_path1");

        // Perform a search
        let paths = store.list_paths(0, 10, "3", false).unwrap();
        assert_eq!(paths.len(), 1);
        assert_eq!(paths[0].path, "test_path3");
    }

    #[test]
    fn test_shortcut() {
        let store = Store::setup_test_store();

        let paths = store.list_shortcuts(0, 10, "", false).unwrap();
        assert_eq!(paths.len(), 0);

        // A single entry
        store
            .add_shortcut("shortcut_1", "/1", Some("desc1"))
            .unwrap();
        let shortcuts = store.list_shortcuts(0, 10, "", false).unwrap();
        assert_eq!(shortcuts.len(), 1);
        assert_eq!(shortcuts[0].name, "shortcut_1");
        assert_eq!(shortcuts[0].path, "/1");
        assert_eq!(shortcuts[0].description, Some("desc1".to_string()));

        // Two entries
        store
            .add_shortcut("shortcut_2", "/2", Some("desc2"))
            .unwrap();
        let shortcuts = store.list_shortcuts(0, 10, "", false).unwrap();
        assert_eq!(shortcuts.len(), 2);
        assert_eq!(shortcuts[0].name, "shortcut_1");
        assert_eq!(shortcuts[0].path, "/1");
        assert_eq!(shortcuts[0].description, Some("desc1".to_string()));
        assert_eq!(shortcuts[1].name, "shortcut_2");
        assert_eq!(shortcuts[1].path, "/2");
        assert_eq!(shortcuts[1].description, Some("desc2".to_string()));

        // Perform a search
        let shortcuts = store.list_shortcuts(0, 10, "2", false).unwrap();
        assert_eq!(shortcuts.len(), 1);
        assert_eq!(shortcuts[0].name, "shortcut_2");
        assert_eq!(shortcuts[0].path, "/2");
        assert_eq!(shortcuts[0].description, Some("desc2".to_string()));

        // Delete the one
        let shortcuts = store.list_shortcuts(0, 10, "", false).unwrap();
        store.delete_shortcut_by_id(shortcuts[1].id).unwrap();
        let shortcuts = store.list_shortcuts(0, 10, "", false).unwrap();
        assert_eq!(shortcuts.len(), 1);
        assert_eq!(shortcuts[0].name, "shortcut_1");

        // Test empty description
        store.add_shortcut("shortcut_nodesc", "/1", None).unwrap();
        let shortcuts = store.list_shortcuts(0, 10, "", false).unwrap();
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

    #[test]
    fn test_list_path_exact_empty_database() {
        let store = Store::setup_test_store();
        let paths = store.list_paths(0, 10, "", false).unwrap();
        assert_eq!(paths.len(), 0);
    }

    #[test]
    fn test_list_path_exact_no_filter() {
        let store = Store::setup_test_store();

        // Add some paths
        store.add_path("/home/user/documents").unwrap();
        store.add_path("/var/log/app").unwrap();
        store.add_path("/usr/local/bin").unwrap();

        // List all paths without filter
        let paths = store.list_paths(0, 10, "", false).unwrap();
        assert_eq!(paths.len(), 3);
        // Paths should be ordered by date desc, id desc (most recent first)
        assert_eq!(paths[0].path, "/usr/local/bin");
        assert_eq!(paths[1].path, "/var/log/app");
        assert_eq!(paths[2].path, "/home/user/documents");
    }

    #[test]
    fn test_list_path_exact_with_pagination() {
        let store = Store::setup_test_store();

        // Add 5 paths
        store.add_path("/path1").unwrap();
        store.add_path("/path2").unwrap();
        store.add_path("/path3").unwrap();
        store.add_path("/path4").unwrap();
        store.add_path("/path5").unwrap();

        // Get first 2 paths
        let paths = store.list_paths(0, 2, "", false).unwrap();
        assert_eq!(paths.len(), 2);
        assert_eq!(paths[0].path, "/path5");
        assert_eq!(paths[1].path, "/path4");

        // Get next 2 paths (offset 2)
        let paths = store.list_paths(2, 2, "", false).unwrap();
        assert_eq!(paths.len(), 2);
        assert_eq!(paths[0].path, "/path3");
        assert_eq!(paths[1].path, "/path2");

        // Get remaining paths (offset 4)
        let paths = store.list_paths(4, 2, "", false).unwrap();
        assert_eq!(paths.len(), 1);
        assert_eq!(paths[0].path, "/path1");

        // Get with offset beyond data
        let paths = store.list_paths(10, 10, "", false).unwrap();
        assert_eq!(paths.len(), 0);
    }

    #[test]
    fn test_list_path_exact_filter_by_path_text() {
        let store = Store::setup_test_store();

        // Add paths with different patterns
        store.add_path("/home/user/documents").unwrap();
        store.add_path("/home/user/downloads").unwrap();
        store.add_path("/var/log/documents").unwrap();
        store.add_path("/var/log/app").unwrap();

        // Filter by text "documents"
        let paths = store.list_paths(0, 10, "documents", false).unwrap();
        assert_eq!(paths.len(), 2);
        assert!(paths.iter().all(|p| p.path.contains("documents")));

        // Filter by text "home"
        let paths = store.list_paths(0, 10, "home", false).unwrap();
        assert_eq!(paths.len(), 2);
        assert!(paths.iter().all(|p| p.path.contains("home")));

        // Filter by text that doesn't match
        let paths = store.list_paths(0, 10, "nonexistent", false).unwrap();
        assert_eq!(paths.len(), 0);
    }

    #[test]
    fn test_list_path_exact_filter_by_shortcut_name() {
        let store = Store::setup_test_store();

        // Add shortcuts
        store
            .add_shortcut("mydocs", "/home/user/documents", None)
            .unwrap();
        store.add_shortcut("logs", "/var/log", None).unwrap();

        // Add paths that match shortcut prefixes
        store.add_path("/home/user/documents/file1.txt").unwrap();
        store
            .add_path("/home/user/documents/subdir/file2.txt")
            .unwrap();
        store.add_path("/var/log/app.log").unwrap();
        store.add_path("/var/log/system.log").unwrap();
        store.add_path("/home/user/downloads/file3.txt").unwrap();

        // Filter by shortcut name "mydocs"
        let paths = store.list_paths(0, 10, "mydocs", false).unwrap();
        assert_eq!(paths.len(), 2);
        assert!(
            paths
                .iter()
                .all(|p| p.path.starts_with("/home/user/documents"))
        );

        // Filter by shortcut name "logs"
        let paths = store.list_paths(0, 10, "logs", false).unwrap();
        assert_eq!(paths.len(), 2);
        assert!(paths.iter().all(|p| p.path.starts_with("/var/log")));
    }

    #[test]
    fn test_list_path_exact_filter_by_shortcut_description() {
        let store = Store::setup_test_store();

        // Add shortcuts with descriptions
        store
            .add_shortcut("proj", "/home/user/projects", Some("my projects"))
            .unwrap();
        store
            .add_shortcut("work", "/home/user/work", Some("work files"))
            .unwrap();

        // Add paths
        store.add_path("/home/user/projects/project1").unwrap();
        store.add_path("/home/user/work/task1").unwrap();
        store.add_path("/home/user/other").unwrap();

        // Filter by description text "my projects"
        let paths = store.list_paths(0, 10, "my projects", false).unwrap();
        assert_eq!(paths.len(), 1);
        assert_eq!(paths[0].path, "/home/user/projects/project1");

        // Filter by description text "work"
        let paths = store.list_paths(0, 10, "work files", false).unwrap();
        assert_eq!(paths.len(), 1);
        assert_eq!(paths[0].path, "/home/user/work/task1");
    }

    #[test]
    fn test_list_path_exact_combined_path_and_shortcut_filter() {
        let store = Store::setup_test_store();

        // Add a shortcut
        store.add_shortcut("home", "/etc/hostname", None).unwrap();

        // Add paths - some matching the text, some matching the shortcut
        store.add_path("/home/user/documents").unwrap();
        store.add_path("/home/user/downloads").unwrap();
        store.add_path("/home/user/pictures").unwrap();
        store.add_path("/var/log/home.log").unwrap();
        store.add_path("/etc/hostname").unwrap();

        // Filter by "home" - should match paths containing "home" OR paths starting with shortcut "home"
        let paths = store.list_paths(0, 10, "home", false).unwrap();
        assert_eq!(paths.len(), 5);
    }

    #[test]
    fn test_list_path_exact_case_insensitive() {
        let store = Store::setup_test_store();

        // Add paths
        store.add_path("/Home/User/Documents").unwrap();
        store.add_path("/home/user/downloads").unwrap();

        // Filter by lowercase "home" should match "/Home/User/Documents"
        let paths = store.list_paths(0, 10, "home", false).unwrap();
        assert_eq!(paths.len(), 2);

        // Filter by uppercase "HOME" should also work (case-insensitive)
        let paths = store.list_paths(0, 10, "HOME", false).unwrap();
        assert_eq!(paths.len(), 2);
    }

    #[test]
    fn test_list_path_exact_with_shortcut_assignment() {
        let store = Store::setup_test_store();

        // Add a shortcut
        store
            .add_shortcut("docs", "/home/user/documents", None)
            .unwrap();

        // Add paths that should be assigned the shortcut
        store.add_path("/home/user/documents/files1").unwrap();
        store.add_path("/home/user/documents/files2").unwrap();

        // List all paths - they should have the shortcut assigned
        let paths = store.list_paths(0, 10, "", false).unwrap();
        assert_eq!(paths.len(), 2);
        for path in &paths {
            assert!(path.shortcut.is_some());
            assert_eq!(path.shortcut.as_ref().unwrap().name, "docs");
        }
    }

    #[test]
    fn test_list_path_exact_multiple_shortcuts_picks_most_specific() {
        let store = Store::setup_test_store();

        // Add multiple overlapping shortcuts
        store.add_shortcut("home", "/home", None).unwrap();
        store.add_shortcut("user", "/home/user", None).unwrap();
        store
            .add_shortcut("docs", "/home/user/documents", None)
            .unwrap();

        // Add a path
        store.add_path("/home/user/documents/file.txt").unwrap();

        // List paths - should assign the most specific shortcut
        let paths = store.list_paths(0, 10, "", false).unwrap();
        assert_eq!(paths.len(), 1);
        assert_eq!(paths[0].shortcut.as_ref().unwrap().name, "docs");
    }

    #[test]
    fn test_list_path_exact_no_shortcut_when_no_prefix_match() {
        let store = Store::setup_test_store();

        // Add a shortcut
        store
            .add_shortcut("docs", "/home/user/documents", None)
            .unwrap();

        // Add a path that doesn't start with the shortcut path
        store.add_path("/var/log/app.log").unwrap();

        // List paths - shortcut should not be assigned
        let paths = store.list_paths(0, 10, "", false).unwrap();
        assert_eq!(paths.len(), 1);
        assert!(paths[0].shortcut.is_none());
    }

    #[test]
    fn test_list_path_exact_empty_filter_text() {
        let store = Store::setup_test_store();

        // Add multiple paths
        store.add_path("/home/user/documents").unwrap();
        store.add_path("/var/log/app").unwrap();
        store.add_path("/usr/bin/executable").unwrap();

        // Filter with empty string should return all paths
        let paths = store.list_paths(0, 10, "", false).unwrap();
        assert_eq!(paths.len(), 3);

        // Even with whitespace, empty-ish filter
        let paths = store.list_paths(0, 10, "", false).unwrap();
        assert_eq!(paths.len(), 3);
    }

    #[test]
    fn test_list_path_exact_pagination_with_filter() {
        let store = Store::setup_test_store();

        // Add paths
        store.add_path("/home/user1").unwrap();
        store.add_path("/home/user2").unwrap();
        store.add_path("/home/user3").unwrap();
        store.add_path("/var/home_backup").unwrap();

        // Filter by "home" with pagination
        let paths = store.list_paths(0, 2, "home", false).unwrap();
        assert_eq!(paths.len(), 2);

        let paths = store.list_paths(2, 2, "home", false).unwrap();
        assert_eq!(paths.len(), 2);

        let paths = store.list_paths(4, 2, "home", false).unwrap();
        assert_eq!(paths.len(), 0);
    }

    #[test]
    fn test_list_path_exact_special_characters_in_path() {
        let store = Store::setup_test_store();

        // Add paths with special characters
        store.add_path("/home/user/documents%20space").unwrap();
        store.add_path("/home/user/file's.txt").unwrap();
        store.add_path("/home/user/[brackets]").unwrap();

        // Filter by path with special character
        let paths = store.list_paths(0, 10, "space", false).unwrap();
        assert_eq!(paths.len(), 1);
        assert!(paths[0].path.contains("space"));

        // List all
        let paths = store.list_paths(0, 10, "", false).unwrap();
        assert_eq!(paths.len(), 3);
    }

    #[test]
    fn test_list_path_exact_filter_with_shortcut_name_and_description() {
        let store = Store::setup_test_store();

        // Add shortcut with both name and description matching different text
        store
            .add_shortcut(
                "myshortcut",
                "/home/user/mydir",
                Some("this is my special directory"),
            )
            .unwrap();

        // Add a path under that shortcut
        store.add_path("/home/user/mydir/files").unwrap();
        store.add_path("/var/log/other.log").unwrap();

        // Filter by shortcut name
        let paths = store.list_paths(0, 10, "myshortcut", false).unwrap();
        assert_eq!(paths.len(), 1);
        assert_eq!(paths[0].path, "/home/user/mydir/files");

        // Filter by shortcut description
        let paths = store.list_paths(0, 10, "special", false).unwrap();
        assert_eq!(paths.len(), 1);
        assert_eq!(paths[0].path, "/home/user/mydir/files");

        // Filter by unrelated text
        let paths = store.list_paths(0, 10, "unrelated", false).unwrap();
        assert_eq!(paths.len(), 0);
    }

    #[test]
    fn test_list_path_fuzzy_empty_database() {
        let store = Store::setup_test_store();
        let paths = store.list_paths(0, 10, "test", true).unwrap();
        assert_eq!(paths.len(), 0);
    }

    #[test]
    fn test_list_path_fuzzy_basic_matching() {
        let store = Store::setup_test_store();

        // Add paths
        store.add_path("/home/user/documents").unwrap();
        store.add_path("/var/log/app").unwrap();
        store.add_path("/usr/local/bin").unwrap();

        // Fuzzy match "doc ment" should find "/home/user/documents"
        let paths = store.list_paths(0, 10, "doc ment", true).unwrap();
        assert_eq!(paths.len(), 1);
        assert_eq!(paths[0].path, "/home/user/documents");
    }

    #[test]
    fn test_list_path_fuzzy_multiple_matches() {
        let store = Store::setup_test_store();

        // Add paths with common patterns
        store.add_path("/home/user/downloads").unwrap();
        store.add_path("/home/user/documents").unwrap();
        store.add_path("/var/log/data").unwrap();

        // Fuzzy match "ome" should find paths containing "ome"
        let paths = store.list_paths(0, 10, "ome", true).unwrap();
        assert!(paths.len() >= 1);
        assert!(paths.iter().any(|p| p.path.contains("home")));
    }

    #[test]
    fn test_list_path_fuzzy_case_insensitive() {
        let store = Store::setup_test_store();

        // Add paths
        store.add_path("/home/user/Documents").unwrap();
        store.add_path("/home/USER/files").unwrap();

        // Fuzzy match uppercase "DOC" should find "/home/user/Documents"
        let paths = store.list_paths(0, 10, "DOC", true).unwrap();
        assert_eq!(paths.len(), 1);
        assert!(paths[0].path.contains("Documents"));

        // Fuzzy match uppercase "USER" should find both paths
        let paths = store.list_paths(0, 10, "USER", true).unwrap();
        assert!(paths.len() >= 1);
    }

    #[test]
    fn test_list_path_fuzzy_out_of_order_characters() {
        let store = Store::setup_test_store();

        // Add paths
        store.add_path("/home/user/documents/readme.txt").unwrap();
        store.add_path("/var/log/system.log").unwrap();

        // Fuzzy match "dme" should match "/home/user/documents" (d-o-c-u-m-e-n-t-s has d, m, e in order)
        let paths = store.list_paths(0, 10, "dme", true).unwrap();
        assert!(paths.iter().any(|p| p.path.contains("documents")));
    }

    #[test]
    fn test_list_path_fuzzy_partial_path_matching() {
        let store = Store::setup_test_store();

        // Add paths
        store.add_path("/home/user/projects/rust/src").unwrap();
        store.add_path("/home/user/projects/python/src").unwrap();
        store.add_path("/var/log/rust.log").unwrap();

        // Fuzzy match "rust" should find relevant paths
        let paths = store.list_paths(0, 10, "rust", true).unwrap();
        assert!(paths.len() >= 1);
        assert!(paths.iter().any(|p| p.path.contains("rust")));
    }

    #[test]
    fn test_list_path_fuzzy_with_pagination() {
        let store = Store::setup_test_store();

        // Add many paths with "home" in them
        for i in 1..=6 {
            store.add_path(&format!("/home/user/folder{}", i)).unwrap();
        }
        store.add_path("/var/log/nohome.log").unwrap();

        // Fuzzy match "home" with limit 2
        let paths = store.list_paths(0, 2, "home", true).unwrap();
        assert_eq!(paths.len(), 2);

        // Get next page
        let paths = store.list_paths(2, 2, "home", true).unwrap();
        assert_eq!(paths.len(), 2);

        // Get remaining
        let paths = store.list_paths(4, 2, "home", true).unwrap();
        assert!(paths.len() > 0);
    }

    #[test]
    fn test_list_path_fuzzy_no_match() {
        let store = Store::setup_test_store();

        // Add paths
        store.add_path("/home/user/documents").unwrap();
        store.add_path("/var/log/app").unwrap();

        // Fuzzy match "xyz" should find nothing
        let paths = store.list_paths(0, 10, "xyz", true).unwrap();
        assert_eq!(paths.len(), 0);
    }

    #[test]
    fn test_list_path_fuzzy_with_shortcut_name_scoring() {
        let store = Store::setup_test_store();

        // Add a shortcut with specific name
        store
            .add_shortcut("mydocs", "/home/user/documents", None)
            .unwrap();

        // Add paths under the shortcut
        store.add_path("/home/user/documents/file1.txt").unwrap();
        store.add_path("/home/user/documents/file2.txt").unwrap();
        store.add_path("/var/log/app.log").unwrap();

        // Fuzzy match "mydoc" should find paths (matches both shortcut name and path)
        let paths = store.list_paths(0, 10, "my doc", true).unwrap();
        assert!(paths.len() >= 1);
        // Paths with the "docs" shortcut should be included
        assert!(
            paths
                .iter()
                .any(|p| p.shortcut.as_ref().map_or(false, |s| s.name == "mydocs"))
        );
    }

    #[test]
    fn test_list_path_fuzzy_with_shortcut_description_scoring() {
        let store = Store::setup_test_store();

        // Add shortcut with description
        store
            .add_shortcut("proj", "/home/user/projects", Some("my important projects"))
            .unwrap();

        // Add a path
        store.add_path("/home/user/projects/proj1").unwrap();
        store.add_path("/var/log/other").unwrap();

        // Fuzzy match "important" should find the path (matches description)
        let paths = store.list_paths(0, 10, "important", true).unwrap();
        assert_eq!(paths.len(), 1);
        assert_eq!(paths[0].path, "/home/user/projects/proj1");
    }

    #[test]
    fn test_list_path_fuzzy_scoring_prefers_better_matches() {
        let store = Store::setup_test_store();

        // Add paths with varying relevance
        store.add_path("/home/user/documents").unwrap();
        store.add_path("/home/user/test_doc").unwrap();
        store.add_path("/var/doc/readme").unwrap();
        store.add_path("/var/paglop/readme").unwrap();

        // Fuzzy match "doc" - should return results
        let paths = store.list_paths(0, 10, "doc", true).unwrap();
        assert!(paths.len() >= 1);
        // All results should contain "doc" in some form
        assert!(paths.iter().all(|p| {
            p.path.to_lowercase().contains("doc")
                || p.shortcut
                    .as_ref()
                    .map_or(false, |s| s.name.to_lowercase().contains("doc"))
        }));
    }

    #[test]
    fn test_list_path_fuzzy_with_special_characters() {
        let store = Store::setup_test_store();

        // Add paths with special characters
        store.add_path("/home/user/my-project").unwrap();
        store.add_path("/home/user/my_folder").unwrap();
        store.add_path("/home/user/my.config").unwrap();

        // Fuzzy match "my" should find all
        let paths = store.list_paths(0, 10, "my", true).unwrap();
        assert!(paths.len() >= 2);
    }

    #[test]
    fn test_list_path_fuzzy_single_character_match() {
        let store = Store::setup_test_store();

        // Add paths
        store.add_path("/home/user/documents").unwrap();
        store.add_path("/var/log/app").unwrap();
        store.add_path("/usr/bin/executable").unwrap();

        // Fuzzy match single character "d"
        let paths = store.list_paths(0, 10, "d", true).unwrap();
        assert!(paths.len() == 1);
        assert!(paths.iter().any(|p| p.path.to_lowercase().contains("d")));
    }

    #[test]
    fn test_list_path_fuzzy_empty_pattern() {
        let store = Store::setup_test_store();

        // Add paths
        store.add_path("/home/user/documents").unwrap();
        store.add_path("/var/log/app").unwrap();

        // Empty pattern with fuzzy should return nothing (empty pattern matches nothing in fuzzy)
        let paths = store.list_paths(0, 10, "", true).unwrap();
        assert_eq!(paths.len(), 2);
    }

    #[test]
    fn test_list_path_fuzzy_offset_beyond_results() {
        let store = Store::setup_test_store();

        // Add paths
        store.add_path("/home/user/documents").unwrap();
        store.add_path("/home/user/downloads").unwrap();

        // Fuzzy match with offset beyond results
        let paths = store.list_paths(100, 10, "home", true).unwrap();
        assert_eq!(paths.len(), 0);
    }

    #[test]
    fn test_list_path_fuzzy_all_paths_match_pattern() {
        let store = Store::setup_test_store();

        // Add paths
        store.add_path("/home/user/a").unwrap();
        store.add_path("/home/user/b").unwrap();
        store.add_path("/home/user/c").unwrap();

        // Fuzzy match "home" - all should match
        let paths = store.list_paths(0, 10, "home", true).unwrap();
        assert_eq!(paths.len(), 3);
    }

    #[test]
    fn test_list_path_fuzzy_with_numeric_patterns() {
        let store = Store::setup_test_store();

        // Add paths with numbers
        store.add_path("/home/user/project1").unwrap();
        store.add_path("/home/user/project2").unwrap();
        store.add_path("/var/log/error404").unwrap();

        // Fuzzy match "1" should find project1 and possibly error404
        let paths = store.list_paths(0, 10, "1", true).unwrap();
        assert!(paths.len() >= 1);
        assert!(paths.iter().any(|p| p.path.contains("1")));
    }

    #[test]
    fn test_list_path_fuzzy_longpattern_match() {
        let store = Store::setup_test_store();

        // Add paths
        store
            .add_path("/home/user/very/long/path/structure/to/documents")
            .unwrap();
        store.add_path("/var/log/app.log").unwrap();

        // Fuzzy match "longpath" - should find the long path
        let paths = store.list_paths(0, 10, "longpath", true).unwrap();
        assert!(paths.iter().any(|p| p.path.contains("long")));
    }

    #[test]
    fn test_list_path_fuzzy_multiple_shortcuts_scoring() {
        let store = Store::setup_test_store();

        // Add multiple shortcuts
        store.add_shortcut("home", "/home", None).unwrap();
        store
            .add_shortcut("xyz", "/home/user/documents", None)
            .unwrap();
        store.add_shortcut("work", "/home/user/work", None).unwrap();

        // Add paths
        store.add_path("/home/user/documents/file1").unwrap();
        store.add_path("/home/user/work/task1").unwrap();
        store.add_path("/var/log/home.log").unwrap();

        // Fuzzy match "doc" - should find paths related to docs shortcut
        let paths = store.list_paths(0, 10, "x yz", true).unwrap();
        assert!(paths.len() == 1);
    }

    #[test]
    fn test_list_path_fuzzy_consecutive_characters() {
        let store = Store::setup_test_store();

        // Add paths
        store.add_path("/home/user/abcdefgh").unwrap();
        store.add_path("/var/log/a_b_c_d_e_f").unwrap();

        // Fuzzy match "cde" - should find both paths (consecutive in first, separated in second)
        let paths = store.list_paths(0, 10, "cde", true).unwrap();
        assert!(paths.len() >= 1);
    }

    #[test]
    fn test_list_path_fuzzy_returns_most_relevant_first() {
        let store = Store::setup_test_store();

        // Add paths where one is a more direct match
        store.add_path("/home/user/documents/document.pdf").unwrap();
        store.add_path("/var/log/random_document_name.log").unwrap();

        // Fuzzy match "document" - document.pdf should be first or highly ranked
        let paths = store.list_paths(0, 10, "document", true).unwrap();
        assert!(paths.len() == 2);
        // The first result should be a better match
        assert!(paths[0].path.to_lowercase().contains("document"));
    }

    #[test]
    fn test_list_path_fuzzy_with_dots_in_path() {
        let store = Store::setup_test_store();

        // Add paths with dots
        store.add_path("/home/user/.config/app").unwrap();
        store.add_path("/home/user/file.txt").unwrap();

        // Fuzzy match "config" should find .config path
        let paths = store.list_paths(0, 10, "config", true).unwrap();
        assert!(paths.iter().any(|p| p.path.contains("config")));
    }

    #[test]
    fn test_list_path_fuzzy_limit_zero() {
        let store = Store::setup_test_store();

        // Add paths
        store.add_path("/home/user/documents").unwrap();
        store.add_path("/home/user/downloads").unwrap();

        // Fuzzy match with limit 0 - should return nothing
        let paths = store.list_paths(0, 0, "home", true).unwrap();
        assert_eq!(paths.len(), 0);
    }

    #[test]
    fn test_path_search_include_shortcuts_enabled() {
        let store = Store::setup_test_store();

        // Add a shortcut with specific name
        store
            .add_shortcut("myshortcut", "/home/user/mydir", None)
            .unwrap();

        // Add paths - one matching the shortcut, one not
        store.add_path("/home/user/mydir/file1.txt").unwrap();
        store.add_path("/var/log/app.log").unwrap();

        // Search by shortcut name - should find the path under the shortcut
        // when path_search_include_shortcuts is enabled (default)
        let paths = store.list_paths(0, 10, "myshortcut", false).unwrap();
        assert_eq!(paths.len(), 1);
        assert_eq!(paths[0].path, "/home/user/mydir/file1.txt");
    }

    #[test]
    fn test_path_search_include_shortcuts_disabled() {
        // Create a custom config with path_search_include_shortcuts disabled
        let mut config = Config::default();
        config.path_search_include_shortcuts = false;

        let store = Store {
            db_conn: Rc::from(Connection::open_in_memory().unwrap()),
            config: Arc::new(config),
        };
        store.init_schema();

        // Add a shortcut with specific name
        store
            .add_shortcut("myshortcut", "/home/user/mydir", None)
            .unwrap();

        // Add paths - one matching the shortcut, one not
        store.add_path("/home/user/mydir/file1.txt").unwrap();
        store.add_path("/var/log/app.log").unwrap();

        // Search by shortcut name - should find nothing
        // when path_search_include_shortcuts is disabled
        let paths = store.list_paths(0, 10, "myshortcut", false).unwrap();
        assert_eq!(paths.len(), 0);
    }

    #[test]
    fn test_path_search_include_shortcuts_filter_by_description_enabled() {
        let store = Store::setup_test_store();

        // Add a shortcut with a description
        store
            .add_shortcut("proj", "/home/user/projects", Some("my important project"))
            .unwrap();

        // Add paths
        store.add_path("/home/user/projects/file1.txt").unwrap();
        store.add_path("/var/log/other.log").unwrap();

        // Search by shortcut description - should find the path
        // when path_search_include_shortcuts is enabled (default)
        let paths = store.list_paths(0, 10, "important project", false).unwrap();
        assert_eq!(paths.len(), 1);
        assert_eq!(paths[0].path, "/home/user/projects/file1.txt");
    }

    #[test]
    fn test_path_search_include_shortcuts_filter_by_description_disabled() {
        // Create a custom config with path_search_include_shortcuts disabled
        let mut config = Config::default();
        config.path_search_include_shortcuts = false;

        let store = Store {
            db_conn: Rc::from(Connection::open_in_memory().unwrap()),
            config: Arc::new(config),
        };
        store.init_schema();

        // Add a shortcut with a description
        store
            .add_shortcut("proj", "/home/user/projects", Some("my important project"))
            .unwrap();

        // Add paths
        store.add_path("/home/user/projects/file1.txt").unwrap();
        store.add_path("/var/log/other.log").unwrap();

        // Search by shortcut description - should find nothing
        // when path_search_include_shortcuts is disabled
        let paths = store.list_paths(0, 10, "important project", false).unwrap();
        assert_eq!(paths.len(), 0);
    }

    #[test]
    fn test_path_search_include_shortcuts_direct_path_match_always_works() {
        // Create a custom config with path_search_include_shortcuts disabled
        let mut config = Config::default();
        config.path_search_include_shortcuts = false;

        let store = Store {
            db_conn: Rc::from(Connection::open_in_memory().unwrap()),
            config: Arc::new(config),
        };
        store.init_schema();

        // Add a shortcut
        store
            .add_shortcut("proj", "/home/user/projects", None)
            .unwrap();

        // Add paths
        store.add_path("/home/user/projects/file1").unwrap();
        store.add_path("/var/log/app.log").unwrap();

        // Search by actual path content - should find it even with shortcuts disabled
        let paths = store.list_paths(0, 10, "file1", false).unwrap();
        assert_eq!(paths.len(), 1);
        assert_eq!(paths[0].path, "/home/user/projects/file1");
    }

    #[test]
    fn test_list_path_fuzzy_with_shortcut_scoring_enabled() {
        let store = Store::setup_test_store();

        // Add a shortcut with a unique name
        store
            .add_shortcut("uniqueshortcut", "/home/user/mydir", None)
            .unwrap();

        // Add paths - one under the shortcut, one elsewhere
        store.add_path("/home/user/mydir/file1.txt").unwrap();
        store.add_path("/var/log/app.log").unwrap();

        // Fuzzy match "uniqueshortcut" - should find the path under the shortcut
        // when path_search_include_shortcuts is enabled (default)
        let paths = store.list_paths(0, 10, "uniqueshortcut", true).unwrap();
        assert_eq!(paths.len(), 1);
        assert_eq!(paths[0].path, "/home/user/mydir/file1.txt");
    }

    #[test]
    fn test_list_path_fuzzy_with_shortcut_scoring_disabled() {
        // Create a custom config with path_search_include_shortcuts disabled
        let mut config = Config::default();
        config.path_search_include_shortcuts = false;

        let store = Store {
            db_conn: Rc::from(Connection::open_in_memory().unwrap()),
            config: Arc::new(config),
        };
        store.init_schema();

        // Add a shortcut with a unique name
        store
            .add_shortcut("uniqueshortcut", "/home/user/mydir", None)
            .unwrap();

        // Add paths - one under the shortcut, one elsewhere
        store.add_path("/home/user/mydir/file1.txt").unwrap();
        store.add_path("/var/log/app.log").unwrap();

        // Fuzzy match "uniqueshortcut" - should find nothing
        // when path_search_include_shortcuts is disabled
        // (the search term doesn't appear in any actual path)
        let paths = store.list_paths(0, 10, "uniqueshortcut", true).unwrap();
        assert_eq!(paths.len(), 0);
    }
}

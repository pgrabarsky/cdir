use std::{
    fmt, fs,
    rc::Rc,
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};

use log::{debug, error, info, trace, warn};
use nucleo_matcher::{
    Matcher, Utf32Str,
    pattern::{CaseMatching, Normalization, Pattern},
};
use rusqlite::{Connection, Result, params};

use crate::config::Config;

#[cfg(test)]
#[path = "store_tests.rs"]
mod store_tests;

// Update this when the database schema changes with the max value value of the sql
// files in ../dbschema (e.g. if 1.sql is the latest, this should be 1)
const CURRENT_SCHEMA_VERSION: i64 = 3;

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
    pub(crate) smart_path: bool,
}

impl Path {
    pub fn new(id: i64, path: String, date: i64, shortcuts: &[Shortcut]) -> Self {
        let mut path = Path {
            id,
            path,
            date,
            shortcut: None,
            smart_path: false,
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
        sub_path.as_bytes()[base_path.len()] == std::path::MAIN_SEPARATOR as u8
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

struct SmartRanker {
    depth: usize,
    context_values_count: usize,
    seen_paths: std::collections::HashMap<String, u64>,
}

impl SmartRanker {
    fn new(depth: usize, context_values_count: usize) -> SmartRanker {
        debug!(
            "Creating SmartRanker with depth={} context_values_count={}",
            depth, context_values_count
        );
        SmartRanker {
            depth,
            context_values_count,
            seen_paths: std::collections::HashMap::new(),
        }
    }

    fn add_path(&mut self, depth: usize, path: String, distance: usize) {
        debug!(
            "SmartRanker add_path depth={} path='{}' distance={}",
            depth, path, distance
        );
        if distance >= self.context_values_count {
            warn!(
                "SmartRanker has been given a distance of {} which is >= context_values_count of {}, skipping",
                distance, self.context_values_count
            );
            return;
        }
        let reverse_idx: u32 = self.context_values_count as u32 - 1 - distance as u32;
        let score = 2u64.pow(reverse_idx);
        // adjust the score with the depth:
        // 2u64.pow((self.depth - depth - 1) as u32) is too much, let's have the depth score 4 time lower than the distance score
        // so that we privilegiate the closest paths, and only use the depth as a tie breaker
        let score = (score << 2) + (self.depth - depth - 1) as u64;
        trace!(
            "SmartRanker adding path: {:?} distance={} score={}",
            path, distance, score
        );
        if let Some(existing_path_score) = self.seen_paths.get(&path) {
            let new_score = score + existing_path_score;
            trace!(
                "SmartRanker updating path: {:?} existing_score={} new_score={}",
                path, existing_path_score, new_score
            );
            self.seen_paths.insert(path.clone(), new_score);
        } else {
            self.seen_paths.insert(path.clone(), score);
        }
    }

    fn collect_rows(&self) -> Vec<String> {
        let rows = self.seen_paths.iter().map(|(k, v)| (k.clone(), *v));
        let mut rows: Vec<(String, u64)> = rows.collect();
        // let mut rows: Vec<(u64, String)> = self.seen_paths.clone().into_iter().collect();
        // sort by score descending
        rows.sort_by(|pws1, pws2| pws2.1.cmp(&pws1.1));
        rows.into_iter()
            .take(self.context_values_count)
            .map(|pws| pws.0)
            .collect::<Vec<String>>()
    }
}

/// Store struct to manage database connection and operations
/// db_conn: the SQLite database connection
pub(crate) struct Store {
    db_conn: Rc<Connection>,
    config: Arc<Mutex<Config>>,
}

impl Store {
    /// Creates a new Store instance and initializes the database if it doesn't exist.
    ///
    /// ### Parameters
    /// dir_path: the path to the SQLite database file
    ///
    /// ### Returns
    /// a new Store instance
    pub(crate) fn new(dir_path: &std::path::Path, config: Arc<Mutex<Config>>) -> Store {
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
            include_str!("../dbschema/3.sql"),
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
        let result1;
        let result2;
        {
            // add into paths
            let mut stmt = self
                .db_conn
                .prepare("INSERT INTO paths (path, date) VALUES ((?1),(?2))")?;
            result1 = stmt
                .execute([path, &format!("{}", epoc)])
                .map_err(|e| {
                    error!("Failed to insert path '{}' time' {}: {}", path, epoc, e);
                    e
                })
                .map(|_l| ());
            let _ = result1
                .as_ref()
                .map_err(|e| error!("Error inserting into paths: {}", e));
        }
        {
            // add into paths_history
            let mut stmt = self
                .db_conn
                .prepare("INSERT INTO paths_history (path, date) VALUES ((?1),(?2))")?;
            result2 = stmt
                .execute([path, &format!("{}", epoc)])
                .map_err(|e| {
                    error!("Failed to insert path '{}' time' {}: {}", path, epoc, e);
                    e
                })
                .map(|_l| ());
            let _ = result2
                .as_ref()
                .map_err(|e| error!("Error inserting into paths_history: {}", e));
        }
        result1.and(result2)
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

        if !self.config.lock().unwrap().path_search_include_shortcuts {
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

            if self.config.lock().unwrap().path_search_include_shortcuts {
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
        let mut pos = pos;
        let mut len = len;

        let mut smart_rows = vec![];
        if self.config.lock().unwrap().smart_suggestions_active && like_text.is_empty() {
            // get current working directory
            let cwd = std::env::current_dir().unwrap();
            let config_lock = self.config.lock().unwrap();
            smart_rows = self
                .list_path_history_smart_suggestions(
                    cwd.to_str().unwrap(),
                    config_lock.smart_suggestions_depth,
                    config_lock.smart_suggestions_count,
                    shortcuts,
                )
                .unwrap();
            // reverse the list in order to have the best suggestionstion just on top of the first into the history
            smart_rows.reverse();

            if pos < smart_rows.len() {
                // we keep smart_rows.len() - pos values
                smart_rows = smart_rows.into_iter().skip(pos).collect();
                len -= smart_rows.len();
                pos = 0;
            } else {
                // we skip all smart rows
                pos -= smart_rows.len();
                smart_rows = vec![];
            }
        }

        debug!("smart_rows len={}", smart_rows.len());

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
        let mut final_rows = smart_rows;
        final_rows.append(&mut paths);

        debug!("final_row len={}", final_rows.len());

        Ok(final_rows)
    }

    /// Lists path history from the paths_history table with pagination and optional filtering.
    /// The results are ordered by date (descending) and ID (descending).
    /// If `like_text` is provided, only paths containing the text are returned.
    /// This function only performs exact matching (no fuzzy search).
    ///
    /// ### Parameters
    /// pos: the starting position (offset) for pagination
    /// len: the number of paths to return
    /// like_text: optional text to filter paths (if empty, no filtering is applied)
    ///
    /// ### Returns
    /// A vector of Path entries if the operation was successful, otherwise an error.
    #[allow(dead_code)]
    pub(crate) fn list_path_history(
        &self,
        pos: usize,
        len: usize,
        like_text: &str,
    ) -> Result<Vec<Path>, rusqlite::Error> {
        debug!(
            "list_path_history pos={} len={} like_text={}",
            pos, len, like_text
        );
        // Retrieve all shortcuts to associate with paths
        let shortcuts = self.list_all_shortcuts().unwrap_or_default();
        self.list_path_history_exact(pos, len, like_text, &shortcuts)
    }

    fn list_path_history_exact(
        &self,
        pos: usize,
        len: usize,
        like_text: &str,
        shortcuts: &[Shortcut],
    ) -> Result<Vec<Path>, rusqlite::Error> {
        debug!(
            "list_path_history_exact pos={} len={} like_text={}",
            pos, len, like_text
        );
        let mut sql = String::from("SELECT id, path, date FROM paths_history");
        let mut params: Vec<String> = vec![];

        if !like_text.is_empty() {
            sql.push_str(" WHERE path LIKE '%' || (?1) || '%'");
            params.push(like_text.to_string());

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

        debug!("list_path_history_exact sql={} params={:?}", sql, params);

        let mut stmt = match self.db_conn.prepare(sql.as_str()) {
            Ok(stmt) => stmt,
            Err(e) => {
                error!("list_path_history failed in prepare {}: {}", sql, e);
                return Err(e);
            }
        };

        let rows = match stmt.query_map(rusqlite::params_from_iter(params), |row| {
            let path_str: String = row.get(1)?;
            Ok(Path::new(row.get(0)?, path_str, row.get(2)?, shortcuts))
        }) {
            Ok(rows) => rows,
            Err(e) => {
                error!("list_path_history failed in query_map: {}", e);
                return Err(e);
            }
        };

        let mut paths = Vec::new();
        for path in rows {
            paths.push(path?);
        }
        Ok(paths)
    }

    pub(crate) fn list_path_history_smart_suggestions(
        &self,
        match_path: &str,
        search_depth: usize,
        suggestions_values_count: usize,
        shortcuts: &[Shortcut],
    ) -> Result<Vec<Path>, rusqlite::Error> {
        debug!(
            "entering list_path_history_smart_suggestions match_path='{}' search_depth={} suggestions_values_count={}",
            match_path, search_depth, suggestions_values_count
        );
        if match_path.is_empty() {
            return Ok(vec![]);
        }
        let mut stmt = match self.db_conn.prepare("SELECT id, path, date FROM paths_history WHERE path == (?1) ORDER BY date desc, id desc LIMIT (?2)") {
            Ok(stmt) => stmt,
            Err(e) => {
                error!("list_path_history failed in prepare: {}", e);
                return Err(e);
            }
        };
        let rows = match stmt.query_map(
            rusqlite::params_from_iter([match_path, &search_depth.to_string()]),
            |row| {
                let path_str: String = row.get(1)?;
                Ok(Path::new(row.get(0)?, path_str, row.get(2)?, shortcuts))
            },
        ) {
            Ok(rows) => rows,
            Err(e) => {
                error!("list_path_history failed in query_map: {}", e);
                return Err(e);
            }
        };

        let rows: Result<Vec<Path>> = rows.collect();
        let rows = rows.unwrap();

        let mut stmt = match self.db_conn.prepare("SELECT DISTINCT path FROM paths_history WHERE id > (?1) and path != (?2) ORDER BY date asc, id asc LIMIT (?3)") {
            Ok(stmt) => stmt,
            Err(e) => {
                error!("list_path_history failed in prepare: {}", e);
                return Err(e);
            }
        };

        let mut sm = SmartRanker::new(search_depth, suggestions_values_count);

        for (set_idx, row) in rows.iter().enumerate() {
            debug!("found path: {:?}", row);

            // we skip the home directory that has no added value for smart suggestions
            let skip_directory = std::env::home_dir()
                .and_then(|p| p.to_str().map(|s| s.to_string()))
                .unwrap_or_default();
            let prev_rows = match stmt.query_map(
                rusqlite::params_from_iter([
                    &row.id.to_string(),
                    &skip_directory,
                    &suggestions_values_count.to_string(),
                ]),
                |row| {
                    let path_str: String = row.get(0)?;
                    Ok(path_str)
                },
            ) {
                Ok(rows) => rows,
                Err(e) => {
                    error!("list_path_history failed in query_map: {}", e);
                    return Err(e);
                }
            };
            for (idx, prev_row) in prev_rows.enumerate() {
                let prev_row = prev_row?;
                if prev_row == match_path {
                    trace!("skipping match_path: {:?}", prev_row);
                    break;
                }
                debug!("adding previous path: {:?}", prev_row);
                sm.add_path(set_idx, prev_row, idx);
            }
        }
        let paths = sm
            .collect_rows()
            .iter()
            .map(|p| {
                let mut path = Path::new(0, p.clone(), 0, shortcuts);
                path.smart_path = true;
                path
            })
            .collect();
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
            config: Arc::new(Mutex::new(Config::default())),
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

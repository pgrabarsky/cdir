CREATE TABLE IF NOT EXISTS paths_history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    path TEXT NOT NULL,
    date INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS paths_history_path_date_id ON paths_history (path, date DESC, id DESC);
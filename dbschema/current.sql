-- This is the current schema used by the application.

-- Version table
CREATE TABLE IF NOT EXISTS version (
    version INTEGER PRIMARY KEY
);

-- Path table
CREATE TABLE IF NOT EXISTS paths (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    path TEXT NOT NULL,
    date INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS paths_date ON paths (date);

-- Shortcuts table
CREATE TABLE IF NOT EXISTS shortcuts (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    path TEXT NOT NULL,
    description TEXT
);
CREATE INDEX IF NOT EXISTS shortcuts_name ON shortcuts (name);

-- Path table
CREATE TABLE IF NOT EXISTS paths_history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    path TEXT NOT NULL,
    date INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS paths_history_path_date_id ON paths_history (path, date DESC, id DESC);

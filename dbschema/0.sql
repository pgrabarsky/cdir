-- This 0 version is the intial schema.
-- It is not used to patch schemas, and is kept for reference only.

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
                                         path TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS shortcuts_name ON shortcuts (name);

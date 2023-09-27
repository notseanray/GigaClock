CREATE TABLE users(
    id TEXT PRIMARY KEY ASC NOT NULL,
    email TEXT NOT NULL,
    name TEXT NOT NULL,
    lastopen_ts TEXT,
    photo TEXT NOT NULL,
    verified INTEGER NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    admin INTEGER NOT NULL
);

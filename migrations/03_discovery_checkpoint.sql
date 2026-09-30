CREATE TABLE discovery_state (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    active INTEGER NOT NULL DEFAULT 0,
    cursor TEXT,
    discovered_count INTEGER NOT NULL DEFAULT 0,
    added_count INTEGER NOT NULL DEFAULT 0,
    refreshed_count INTEGER NOT NULL DEFAULT 0,
    marked_missing_count INTEGER NOT NULL DEFAULT 0,
    started_at DATETIME,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

INSERT INTO discovery_state (id, active)
VALUES (1, 0);

CREATE TABLE discovery_seen (
    unit_name TEXT PRIMARY KEY,
    FOREIGN KEY (unit_name)
        REFERENCES services(unit_name)
        ON DELETE CASCADE
);

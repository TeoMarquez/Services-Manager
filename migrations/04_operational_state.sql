CREATE TABLE IF NOT EXISTS service_operational_state (
    service_id INTEGER PRIMARY KEY,
    active INTEGER NOT NULL CHECK (active IN (0, 1)),
    startup_mode TEXT NOT NULL CHECK (startup_mode IN ('enabled', 'disabled')),
    observed_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY(service_id) REFERENCES services(id) ON DELETE CASCADE
);

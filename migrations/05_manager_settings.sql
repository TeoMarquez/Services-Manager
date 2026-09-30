CREATE TABLE IF NOT EXISTS manager_settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

INSERT OR IGNORE INTO manager_settings(key, value)
VALUES ('service_directory', '/etc/systemd/system');

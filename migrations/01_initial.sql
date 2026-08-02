CREATE TABLE IF NOT EXISTS services (
    id INTEGER PRIMARY KEY AUTOINCREMENT,

    -- Nombre real de systemd
    unit_name TEXT NOT NULL UNIQUE,

    alias TEXT,

    description TEXT,

    -- De dónde salió este registro
    origin TEXT NOT NULL DEFAULT 'DISCOVERED',

    -- Control de visibilidad en Telegram
    visible INTEGER NOT NULL DEFAULT 0,

    -- Si es servicio del s.o.
    system_service INTEGER NOT NULL DEFAULT 0,

    -- Última vez que discovery lo encontró
    last_seen DATETIME,

    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);


CREATE TABLE IF NOT EXISTS tags (
    id INTEGER PRIMARY KEY AUTOINCREMENT,

    name TEXT NOT NULL UNIQUE
);


CREATE TABLE IF NOT EXISTS service_tags (
    service_id INTEGER NOT NULL,
    tag_id INTEGER NOT NULL,

    PRIMARY KEY(service_id, tag_id),

    FOREIGN KEY(service_id)
        REFERENCES services(id)
        ON DELETE CASCADE,

    FOREIGN KEY(tag_id)
        REFERENCES tags(id)
        ON DELETE CASCADE
);


CREATE INDEX IF NOT EXISTS idx_services_visible
ON services(visible);

CREATE INDEX IF NOT EXISTS idx_services_origin
ON services(origin);

CREATE INDEX IF NOT EXISTS idx_services_last_seen
ON services(last_seen);
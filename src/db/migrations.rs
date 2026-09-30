use rusqlite::Connection;
use std::fs;

pub fn run(conn: &Connection) {
    create_migrations_table(conn);

    let migrations = vec![
        (1, "01_initial.sql"),
        (2, "02_add_present.sql"),
        (3, "03_discovery_checkpoint.sql"),
        (4, "04_operational_state.sql"),
    ];

    for (id, file) in migrations {
        if already_applied(conn, id) {
            continue;
        }

        println!("Applying migration {}", file);

        let sql = fs::read_to_string(format!("migrations/{}", file))
            .expect("Could not read migration file");

        conn.execute_batch(&sql)
            .expect("Could not execute migration");

        conn.execute(
            "
            INSERT INTO migrations(id, name)
            VALUES (?, ?)
            ",
            (id, file),
        )
        .expect("Could not register migration");
    }
}

fn create_migrations_table(conn: &Connection) {
    conn.execute(
        "
        CREATE TABLE IF NOT EXISTS migrations (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL UNIQUE,
            applied_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
        )
        ",
        [],
    )
    .expect("Could not create migrations table");
}

fn already_applied(conn: &Connection, id: i32) -> bool {
    conn.query_row(
        "
        SELECT EXISTS(
            SELECT 1
            FROM migrations
            WHERE id = ?
        )
        ",
        [id],
        |row| row.get(0),
    )
    .expect("Could not check migration")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrations_are_idempotent_and_initialize_discovery_checkpoint() {
        let conn = Connection::open_in_memory().unwrap();
        run(&conn);
        run(&conn);

        let migration_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM migrations", [], |row| row.get(0))
            .unwrap();
        let (active, cursor): (i64, Option<String>) = conn
            .query_row(
                "SELECT active, cursor FROM discovery_state WHERE id = 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();

        assert_eq!(migration_count, 4);
        assert_eq!(active, 0);
        assert_eq!(cursor, None);
    }
}

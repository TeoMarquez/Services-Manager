use rusqlite::Connection;
use std::fs;


pub fn run(conn: &Connection) {

    create_migrations_table(conn);

    let migrations = vec![
        (1, "01_initial.sql"),
        (2, "02_add_present.sql"),
    ];


    for (id, file) in migrations {

        if already_applied(conn, id) {
            continue;
        }


        println!("Applying migration {}", file);


        let sql = fs::read_to_string(
            format!("migrations/{}", file)
        )
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



fn create_migrations_table(
    conn: &Connection
) {

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



fn already_applied(
    conn: &Connection,
    id: i32
) -> bool {

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
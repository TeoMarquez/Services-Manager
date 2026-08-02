use rusqlite::Connection;
use std::fs;

use crate::db::migrations;

pub fn create_connection() -> Connection {
    fs::create_dir_all("data")
        .expect("Could not create data directory");

    let conn = Connection::open("data/services.db")
        .expect("Could not open database");

    conn.execute(
        "PRAGMA foreign_keys = ON",
        [],
    )
    .expect("Could not enable foreign keys");

    migrations::run(&conn);

    conn
}
use rusqlite::{Connection, Result, params};
use std::path::{Path, PathBuf};

pub struct SettingsRepository<'a> {
    conn: &'a Connection,
}

impl<'a> SettingsRepository<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    pub fn service_directory(&self) -> Result<PathBuf> {
        let path: String = self.conn.query_row(
            "SELECT value FROM manager_settings WHERE key = 'service_directory'",
            [],
            |row| row.get(0),
        )?;
        Ok(PathBuf::from(path))
    }

    pub fn set_service_directory(&self, path: &Path) -> Result<()> {
        self.conn.execute(
            "INSERT INTO manager_settings(key, value) VALUES ('service_directory', ?)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![path.to_string_lossy().as_ref()],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_service_directory_is_etc_systemd_system() {
        let conn = Connection::open_in_memory().unwrap();
        crate::db::migrations::run(&conn);
        assert_eq!(
            SettingsRepository::new(&conn).service_directory().unwrap(),
            PathBuf::from("/etc/systemd/system")
        );
    }
}

use super::models::{DiscoveryCheckpoint, Service, Tag};
use crate::system::{OperationalState, StartupMode};
use rusqlite::{Connection, Result, params};
pub struct ServiceRepository<'a> {
    conn: &'a Connection,
}

impl<'a> ServiceRepository<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    pub fn begin_or_resume_discovery(&self) -> Result<DiscoveryCheckpoint> {
        let tx = self.conn.unchecked_transaction()?;
        let active: bool = tx.query_row(
            "SELECT active FROM discovery_state WHERE id = 1",
            [],
            |row| Ok(row.get::<_, i64>(0)? != 0),
        )?;

        if !active {
            tx.execute("DELETE FROM discovery_seen", [])?;
            tx.execute(
                "UPDATE discovery_state
                 SET active = 1,
                     cursor = NULL,
                     discovered_count = 0,
                     added_count = 0,
                     refreshed_count = 0,
                     marked_missing_count = 0,
                     started_at = CURRENT_TIMESTAMP,
                     updated_at = CURRENT_TIMESTAMP
                 WHERE id = 1",
                [],
            )?;
        }

        let checkpoint = Self::read_discovery_checkpoint(&tx)?;
        tx.commit()?;
        Ok(checkpoint)
    }

    pub fn save_discovery_page(
        &self,
        expected_cursor: Option<&str>,
        services: &[String],
        next_cursor: Option<&str>,
        complete: bool,
    ) -> Result<DiscoveryCheckpoint> {
        let tx = self.conn.unchecked_transaction()?;
        let current = Self::read_discovery_checkpoint(&tx)?;
        if !current.active || current.cursor.as_deref() != expected_cursor {
            return Err(rusqlite::Error::InvalidQuery);
        }

        let repository = ServiceRepository::new(&tx);
        let mut added = 0i64;
        let mut refreshed = 0i64;
        for unit_name in services {
            match repository.find_by_unit_name(unit_name)? {
                None => {
                    repository.insert(unit_name, "DISCOVERED")?;
                    added += 1;
                }
                Some(_) => {
                    repository.update_last_seen(unit_name)?;
                    refreshed += 1;
                }
            }
            tx.execute(
                "INSERT OR IGNORE INTO discovery_seen(unit_name) VALUES (?)",
                [unit_name],
            )?;
        }

        tx.execute(
            "UPDATE discovery_state
             SET cursor = ?,
                 discovered_count = discovered_count + ?,
                 added_count = added_count + ?,
                 refreshed_count = refreshed_count + ?,
                 updated_at = CURRENT_TIMESTAMP
             WHERE id = 1",
            rusqlite::params![next_cursor, services.len() as i64, added, refreshed],
        )?;

        if complete {
            let missing = tx.execute(
                "UPDATE services
                 SET present = 0
                 WHERE present = 1
                   AND unit_name NOT IN (SELECT unit_name FROM discovery_seen)",
                [],
            )? as i64;
            tx.execute(
                "UPDATE discovery_state
                 SET active = 0,
                     cursor = NULL,
                     marked_missing_count = ?,
                     updated_at = CURRENT_TIMESTAMP
                 WHERE id = 1",
                [missing],
            )?;
        }

        let checkpoint = Self::read_discovery_checkpoint(&tx)?;
        tx.commit()?;
        Ok(checkpoint)
    }

    fn read_discovery_checkpoint(conn: &Connection) -> Result<DiscoveryCheckpoint> {
        conn.query_row(
            "SELECT active, cursor, discovered_count, added_count,
                    refreshed_count, marked_missing_count
             FROM discovery_state WHERE id = 1",
            [],
            |row| {
                Ok(DiscoveryCheckpoint {
                    active: row.get::<_, i64>(0)? != 0,
                    cursor: row.get(1)?,
                    discovered: row.get(2)?,
                    added: row.get(3)?,
                    refreshed: row.get(4)?,
                    marked_missing: row.get(5)?,
                })
            },
        )
    }

    pub fn reset_managed_data(&self) -> Result<()> {
        let tx = self.conn.unchecked_transaction()?;
        tx.execute("DELETE FROM services", [])?;
        tx.execute("DELETE FROM tags", [])?;
        tx.execute("DELETE FROM discovery_seen", [])?;
        tx.execute(
            "UPDATE discovery_state
             SET active = 0,
                 cursor = NULL,
                 discovered_count = 0,
                 added_count = 0,
                 refreshed_count = 0,
                 marked_missing_count = 0,
                 started_at = NULL,
                 updated_at = CURRENT_TIMESTAMP
             WHERE id = 1",
            [],
        )?;
        tx.commit()
    }

    pub fn find_by_unit_name(&self, unit_name: &str) -> Result<Option<Service>> {
        let mut stmt = self.conn.prepare(
            "
            SELECT 
                id,
                unit_name,
                alias,
                description,
                origin,
                visible,
                system_service,
                last_seen,
                present
            FROM services
            WHERE unit_name = ?
            ",
        )?;

        let mut rows = stmt.query([unit_name])?;

        if let Some(row) = rows.next()? {
            Ok(Some(Service {
                id: row.get(0)?,
                unit_name: row.get(1)?,
                alias: row.get(2)?,
                description: row.get(3)?,
                origin: row.get(4)?,
                visible: row.get::<_, i64>(5)? != 0,
                system_service: row.get::<_, i64>(6)? != 0,
                last_seen: row.get(7)?,
                present: row.get::<_, i64>(8)? != 0,
            }))
        } else {
            Ok(None)
        }
    }

    pub fn insert(&self, unit_name: &str, origin: &str) -> Result<()> {
        self.conn.execute(
            "
            INSERT INTO services
            (
                unit_name,
                origin,
                present,
                last_seen
            )
            VALUES (?, ?, 1, CURRENT_TIMESTAMP)
            ",
            [unit_name, origin],
        )?;

        Ok(())
    }
    pub fn update_last_seen(&self, unit_name: &str) -> Result<()> {
        self.conn.execute(
            "
            UPDATE services
            SET 
                last_seen = CURRENT_TIMESTAMP,
                present = 1
            WHERE unit_name = ?
            ",
            [unit_name],
        )?;

        Ok(())
    }
    pub fn mark_missing(&self, discovered: &[String]) -> Result<()> {
        let mut stmt = self.conn.prepare(
            "
        SELECT unit_name
        FROM services
        WHERE present = 1
        ",
        )?;

        let services = stmt.query_map([], |row| row.get::<_, String>(0))?;

        for service in services {
            let name = service?;

            if !discovered.contains(&name) {
                self.set_present(&name, false)?;
            }
        }

        Ok(())
    }

    pub fn set_present(&self, unit_name: &str, present: bool) -> Result<()> {
        self.conn.execute(
            "
        UPDATE services
        SET present = ?
        WHERE unit_name = ?
        ",
            params![if present { 1 } else { 0 }, unit_name],
        )?;

        Ok(())
    }

    pub fn find_all(&self) -> Result<Vec<Service>> {
        self.find_where("1=1")
    }

    pub fn find_present(&self) -> Result<Vec<Service>> {
        self.find_where("present = 1")
    }
    pub fn find_missing(&self) -> Result<Vec<Service>> {
        self.find_where("present = 0")
    }
    pub fn find_visible(&self) -> Result<Vec<Service>> {
        self.find_where("visible = 1")
    }
    pub fn find_hidden(&self) -> Result<Vec<Service>> {
        self.find_where("visible = 0")
    }

    pub fn find_system(&self) -> Result<Vec<Service>> {
        self.find_where("system_service = 1")
    }
    pub fn find_user(&self) -> Result<Vec<Service>> {
        self.find_where("system_service = 0")
    }

    fn find_where(&self, condition: &str) -> Result<Vec<Service>> {
        let query = format!(
            "
            SELECT
                id,
                unit_name,
                alias,
                description,
                origin,
                visible,
                system_service,
                last_seen,
                present
            FROM services
            WHERE {}
            ORDER BY id
            ",
            condition
        );

        let mut stmt = self.conn.prepare(&query)?;

        let services = stmt
            .query_map([], |row| {
                Ok(Service {
                    id: row.get(0)?,
                    unit_name: row.get(1)?,
                    alias: row.get(2)?,
                    description: row.get(3)?,
                    origin: row.get(4)?,
                    visible: row.get::<_, i64>(5)? != 0,
                    system_service: row.get::<_, i64>(6)? != 0,
                    last_seen: row.get(7)?,
                    present: row.get::<_, i64>(8)? != 0,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(services)
    }

    pub fn find_page(
        &self,
        condition: &str,
        params: &[rusqlite::types::Value],
        page: u32,
        per_page: u32,
    ) -> Result<(Vec<Service>, u32)> {
        let offset = (i64::from(page) - 1).saturating_mul(i64::from(per_page));

        let count_query = format!(
            "
            SELECT COUNT(*)
            FROM services
            WHERE {}
            ",
            condition
        );

        let total: u32 = self.conn.query_row(
            &count_query,
            rusqlite::params_from_iter(params.iter()),
            |row| row.get(0),
        )?;

        let query = format!(
            "
            SELECT
                id,
                unit_name,
                alias,
                description,
                origin,
                visible,
                system_service,
                last_seen,
                present
            FROM services
            WHERE {}
            ORDER BY id
            LIMIT ?
            OFFSET ?
            ",
            condition
        );

        let mut stmt = self.conn.prepare(&query)?;

        let mut query_params = params.to_vec();

        query_params.push(rusqlite::types::Value::Integer(i64::from(per_page)));
        query_params.push(rusqlite::types::Value::Integer(offset));

        let services = stmt
            .query_map(rusqlite::params_from_iter(query_params), |row| {
                Ok(Service {
                    id: row.get(0)?,
                    unit_name: row.get(1)?,
                    alias: row.get(2)?,
                    description: row.get(3)?,
                    origin: row.get(4)?,
                    visible: row.get::<_, i64>(5)? != 0,
                    system_service: row.get::<_, i64>(6)? != 0,
                    last_seen: row.get(7)?,
                    present: row.get::<_, i64>(8)? != 0,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok((services, total))
    }
    pub fn delete(&self, unit_name: &str) -> Result<()> {
        self.conn.execute(
            "
            DELETE FROM services
            WHERE unit_name = ?
            ",
            [unit_name],
        )?;

        Ok(())
    }

    pub fn set_visible(&self, unit_name: &str, visible: bool) -> Result<()> {
        self.conn.execute(
            "
            UPDATE services
            SET visible = ?
            WHERE unit_name = ?
            ",
            params![if visible { 1 } else { 0 }, unit_name],
        )?;

        Ok(())
    }

    pub fn save_operational_state(&self, unit_name: &str, state: OperationalState) -> Result<()> {
        let tx = self.conn.unchecked_transaction()?;
        let service_id: i64 = tx.query_row(
            "SELECT id FROM services WHERE unit_name = ? AND present = 1",
            [unit_name],
            |row| row.get(0),
        )?;
        let startup_mode = match state.startup_mode {
            StartupMode::Enabled => "enabled",
            StartupMode::Disabled => "disabled",
        };
        tx.execute(
            "INSERT INTO service_operational_state(service_id, active, startup_mode, observed_at)
             VALUES (?, ?, ?, CURRENT_TIMESTAMP)
             ON CONFLICT(service_id) DO UPDATE SET
                active = excluded.active,
                startup_mode = excluded.startup_mode,
                observed_at = CURRENT_TIMESTAMP",
            params![service_id, i64::from(state.active), startup_mode],
        )?;
        tx.commit()
    }

    pub fn find_by_tag(&self, tag_id: i64) -> Result<Vec<Service>> {
        self.find_where(&format!(
            "
                id IN (
                    SELECT service_id
                    FROM service_tags
                    WHERE tag_id = {}
                )
                ",
            tag_id
        ))
    }

    // start managing tags

    pub fn remove_tag(&self, service_id: i64, tag_id: i64) -> Result<()> {
        self.conn.execute(
            "
            DELETE FROM service_tags
            WHERE service_id = ?
            AND tag_id = ?
            ",
            params![service_id, tag_id],
        )?;

        Ok(())
    }
    pub fn add_tag(&self, service_id: i64, tag_id: i64) -> Result<()> {
        self.conn.execute(
            "
            INSERT OR IGNORE INTO service_tags
            (
                service_id,
                tag_id
            )
            VALUES (?, ?)
            ",
            params![service_id, tag_id],
        )?;

        Ok(())
    }

    pub fn find_tags(&self, service_id: i64) -> Result<Vec<Tag>> {
        let mut stmt = self.conn.prepare(
            "
            SELECT
                t.id,
                t.name
            FROM tags t
            INNER JOIN service_tags st
                ON st.tag_id = t.id
            WHERE st.service_id = ?
            ORDER BY t.id
            ",
        )?;

        let tags = stmt
            .query_map([service_id], |row| {
                Ok(Tag {
                    id: row.get(0)?,
                    name: row.get(1)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(tags)
    }
}

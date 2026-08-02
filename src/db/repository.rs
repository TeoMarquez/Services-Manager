use rusqlite::{Connection, Result, params};
use super::models::Service;

pub struct ServiceRepository<'a> {
    conn: &'a Connection,
}

impl<'a> ServiceRepository<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
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
    pub fn mark_missing(&self, discovered: &Vec<String>) -> Result<()> {
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
                println!("Missing service: {}", name);

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

    pub fn find_all(
        &self
    ) -> Result<Vec<Service>> {

        self.find_where("1=1")

    }

    pub fn find_present(
        &self
    ) -> Result<Vec<Service>> {

        self.find_where(
            "present = 1"
        )

    }
    pub fn find_missing(
        &self
    ) -> Result<Vec<Service>> {

        self.find_where(
            "present = 0"
        )

    }
    pub fn find_system(
        &self
    ) -> Result<Vec<Service>> {

        self.find_where(
            "system_service = 1"
        )

    }
    pub fn find_user(
        &self
    ) -> Result<Vec<Service>> {

        self.find_where(
            "system_service = 0"
        )

    }

    fn find_where(
        &self,
        condition: &str
    ) -> Result<Vec<Service>> {

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


}

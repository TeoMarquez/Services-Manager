use rusqlite::{Connection, Result, params};
use super::models::{
    Service,
    Tag
};
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

    pub fn find_page(
        &self,
        condition: &str,
        page: u32,
        per_page: u32,
    ) -> Result<(Vec<Service>, u32)> {

        let offset = (page - 1) * per_page;


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
            [],
            |row| row.get(0)
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


        let services = stmt
            .query_map(
                [
                    per_page as i64,
                    offset as i64
                ],
                |row| {

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

                }
            )?
            .collect::<Result<Vec<_>, _>>()?;


        Ok((services, total))
    }

    pub fn add_tag(
        &self,
        service_id: i64,
        tag_id: i64
    ) -> Result<()> {

        self.conn.execute(
            "
            INSERT OR IGNORE INTO service_tags
            (
                service_id,
                tag_id
            )
            VALUES (?, ?)
            ",
            params![
                service_id,
                tag_id
            ],
        )?;

        Ok(())
    }

    pub fn find_tags(
        &self,
        service_id: i64
    ) -> Result<Vec<Tag>> {

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
            "
        )?;


        let tags = stmt
            .query_map(
                [service_id],
                |row| {

                    Ok(Tag {
                        id: row.get(0)?,
                        name: row.get(1)?,
                    })

                }
            )?
            .collect::<Result<Vec<_>, _>>()?;


        Ok(tags)
    }
}

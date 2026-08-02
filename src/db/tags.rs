use rusqlite::{
    Connection,
    Result
};
use super::repository::{
    ServiceRepository
};

use super::models::Tag;

pub struct TagRepository<'a> {
    conn: &'a Connection,
}


impl<'a> TagRepository<'a> {

    pub fn new(
        conn: &'a Connection
    ) -> Self {

        Self {
            conn
        }

    }


    pub fn create(
        &self,
        name: &str
    ) -> Result<Tag> {

        self.conn.execute(
            "
            INSERT OR IGNORE INTO tags(name)
            VALUES(?)
            ",
            [name],
        )?;


        let tag = self.find_by_name(name)?
            .expect("Tag should exist after insert");


        Ok(tag)

    }


    pub fn find_all(
        &self
    ) -> Result<Vec<Tag>> {


        let mut stmt =
            self.conn.prepare(
                "
                SELECT
                    id,
                    name
                FROM tags
                ORDER BY id
                "
            )?;


        let tags =
            stmt.query_map(
                [],
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
    pub fn attach_tag(
        service_id: i64,
        tag_name: &str,
        services: &ServiceRepository,
        tags: &TagRepository,
        ) {

        tags.create(tag_name)
            .unwrap();


        let tag = tags
            .find_by_name(tag_name)
            .unwrap()
            .unwrap();


        services
            .add_tag(
                service_id,
                tag.id
            )
            .unwrap();

    }

    pub fn find_by_name(
        &self,
        name: &str
    ) -> Result<Option<Tag>> {

        let mut stmt = self.conn.prepare(
            "
            SELECT
                id,
                name
            FROM tags
            WHERE name = ?
            "
        )?;


        let mut rows =
            stmt.query([name])?;


        if let Some(row) = rows.next()? {

            Ok(Some(Tag {
                id: row.get(0)?,
                name: row.get(1)?,
            }))

        } else {

            Ok(None)

        }

    }

}
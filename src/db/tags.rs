use rusqlite::{
    Connection,
    Result
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
    ) -> Result<()> {

        self.conn.execute(
            "
            INSERT INTO tags(name)
            VALUES(?)
            ",
            [name],
        )?;

        Ok(())

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

}
use diesel::{Insertable, Queryable};

#[derive(Queryable, Debug, Identifiable)]
pub struct Service {
    pub id: i32,
    pub service: Option<String>,
    pub pattern: Option<String>,
}

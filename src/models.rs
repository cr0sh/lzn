use crate::schema::{lezhin, titles};
use chrono::NaiveDateTime;

#[derive(Queryable, Insertable, Debug)]
#[table_name = "lezhin"]
pub(crate) struct ComicRecord {
    pub(crate) comic: String,
    pub(crate) episode_seq: i32,
    pub(crate) episode: Option<String>,
    pub(crate) picture_seq: i32,
    pub(crate) picture: Option<Vec<u8>>,
    pub(crate) updated_at: NaiveDateTime,
}

#[derive(Queryable, Insertable, Debug)]
#[table_name = "titles"]
pub(crate) struct TitleRecord {
    pub(crate) provider: String,
    pub(crate) id: String,
    pub(crate) title: Option<String>,
}

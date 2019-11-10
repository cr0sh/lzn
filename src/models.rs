use crate::schema::lezhin;
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

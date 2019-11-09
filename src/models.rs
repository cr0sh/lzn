use crate::schema::lezhin;
use chrono::NaiveDateTime;

#[derive(Queryable, Insertable)]
#[table_name = "lezhin"]
pub(crate) struct ComicRecord {
    pub(crate) comic: String,
    pub(crate) episode_seq: i32,
    pub(crate) episode: String,
    pub(crate) picture_seq: i32,
    pub(crate) picture: Vec<u8>,
    pub(crate) updated_at: NaiveDateTime,
}

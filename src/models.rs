use crate::provider::Provider;
use crate::schema::{lezhin, scrap_targets, titles};
use chrono::NaiveDateTime;
use diesel::backend::Backend;
use diesel::deserialize::{self, FromSql};
use diesel::serialize::{self, Output, ToSql};
use diesel::sql_types::Integer;
use diesel::sqlite::Sqlite;
use std::io::Write;

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
    pub(crate) provider: Provider,
    pub(crate) id: String,
    pub(crate) title: Option<String>,
}

#[derive(AsExpression, FromSqlRow, PartialEq, Debug, Clone)]
#[sql_type = "Integer"]
pub(crate) enum ScrapStatus {
    Enabled,  // Target to be scraped; will not scrap existing episodes
    Disabled, // Target temporarily disabled
    Complete, // Full-scrap complete; no need to scrap again
}

impl ToSql<Integer, Sqlite> for ScrapStatus {
    fn to_sql<W: Write>(&self, out: &mut Output<W, Sqlite>) -> serialize::Result {
        let value = match self {
            Self::Enabled => 0,
            Self::Disabled => 1,
            Self::Complete => 2,
        };
        <i32 as ToSql<Integer, Sqlite>>::to_sql(&value, out)
    }
}

impl FromSql<Integer, Sqlite> for ScrapStatus {
    fn from_sql(bytes: Option<&<Sqlite as Backend>::RawValue>) -> deserialize::Result<Self> {
        match <i32 as FromSql<Integer, Sqlite>>::from_sql(bytes)? {
            0 => Ok(Self::Enabled),
            1 => Ok(Self::Disabled),
            2 => Ok(Self::Complete),
            _ => Err("Unrecognized enum variant".into()),
        }
    }
}

#[derive(Queryable, Insertable, Debug)]
#[table_name = "scrap_targets"]
pub(crate) struct ScrapTarget {
    pub(crate) provider: Provider,
    pub(crate) id: String,
    pub(crate) status: ScrapStatus,
    pub(crate) last_scrap: NaiveDateTime,
}

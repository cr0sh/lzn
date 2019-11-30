use crate::provider::Provider;
use crate::schema::{comics, episodes, scraping_targets, titles};
use chrono::NaiveDateTime;
use diesel::backend::Backend;
use diesel::deserialize::{self, FromSql};
use diesel::serialize::{self, Output, ToSql};
use diesel::sql_types::Integer;
use diesel::sqlite::Sqlite;
use std::io::Write;

#[derive(Queryable, Insertable, Debug)]
#[table_name = "comics"]
pub(crate) struct ComicRecord {
    pub(crate) provider: Provider,
    pub(crate) comic_id: String,
    pub(crate) episode_seq: i32,
    pub(crate) episode_name: Option<String>,
    pub(crate) image_seq: i32,
    pub(crate) image: Vec<u8>,
    pub(crate) updated_at: NaiveDateTime,
}

#[derive(Queryable, Insertable, Debug)]
#[table_name = "titles"]
pub(crate) struct TitleRecord {
    pub(crate) provider: Provider,
    pub(crate) id: String,
    pub(crate) title: Option<String>,
}
#[derive(Queryable, Insertable, Debug)]
#[table_name = "episodes"]
pub(crate) struct EpisodeRecord {
    pub(crate) provider: Provider,
    #[column_name = "id"]
    pub(crate) comic_id: String,
    #[column_name = "seq"]
    pub(crate) episode_seq: i32,
    pub(crate) title: Option<String>,
    #[column_name = "images_count"]
    pub(crate) images_cnt: i32,
    pub(crate) created_at: NaiveDateTime,
    pub(crate) last_update: NaiveDateTime,
}

#[derive(AsExpression, FromSqlRow, PartialEq, Debug, Clone)]
#[sql_type = "Integer"]
pub(crate) enum ScrapingStatus {
    Enabled,  // Target to be scraped; will not scrape existing episodes
    Disabled, // Target temporarily disabled
    Complete, // Full-scraping complete; no need to scrape again
}

impl ToSql<Integer, Sqlite> for ScrapingStatus {
    fn to_sql<W: Write>(&self, out: &mut Output<W, Sqlite>) -> serialize::Result {
        let value = match self {
            Self::Enabled => 0,
            Self::Disabled => 1,
            Self::Complete => 2,
        };
        <i32 as ToSql<Integer, Sqlite>>::to_sql(&value, out)
    }
}

impl FromSql<Integer, Sqlite> for ScrapingStatus {
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
#[table_name = "scraping_targets"]
pub(crate) struct ScrapingTarget {
    pub(crate) provider: Provider,
    pub(crate) id: String,
    pub(crate) status: ScrapingStatus,
    pub(crate) last_scraping: NaiveDateTime,
}

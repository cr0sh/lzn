use crate::error::Result;
use diesel::backend::Backend;
use diesel::deserialize::{self, FromSql};
use diesel::serialize::{self, Output, ToSql};
use diesel::sql_types::Text;
use diesel::sqlite::{Sqlite, SqliteConnection};
use std::io::Write;

mod lezhin;
mod naver;

#[derive(AsExpression, FromSqlRow, Debug, Clone, Copy, PartialEq, Eq)]
#[sql_type = "Text"]
pub enum Provider {
    Lezhin,
    Naver,
}

impl Provider {
    pub(crate) fn authenticate(
        &self,
        client: &reqwest::Client,
        id: &str,
        password: &str,
    ) -> Result<()> {
        match self {
            Self::Lezhin => lezhin::authenticate(client, id, password),
            Self::Naver => Ok(()), // authentication is not required
        }
    }

    pub(crate) fn fetch_episodes(
        &self,
        client: &reqwest::Client,
        comic_id: &str,
        conn: &SqliteConnection,
    ) -> Result<()> {
        match self {
            Self::Lezhin => lezhin::fetch_episodes(client, comic_id, conn),
            Self::Naver => naver::fetch_episodes(client, comic_id, conn),
        }
    }

    pub(crate) fn fetch_titles(
        &self,
        client: &reqwest::Client,
        comic_ids: Vec<String>,
    ) -> Result<Vec<String>> {
        match self {
            Self::Lezhin => lezhin::fetch_titles(client, comic_ids),
            Self::Naver => unimplemented!(),
        }
    }
}

impl ToSql<Text, Sqlite> for Provider {
    fn to_sql<W: Write>(&self, out: &mut Output<W, Sqlite>) -> serialize::Result {
        let value = match self {
            Self::Lezhin => "lezhin",
            Self::Naver => "naver",
        };
        <String as ToSql<Text, Sqlite>>::to_sql(&value.to_string(), out)
    }
}

impl FromSql<Text, Sqlite> for Provider {
    fn from_sql(bytes: Option<&<Sqlite as Backend>::RawValue>) -> deserialize::Result<Self> {
        Ok(
            unsafe { <*const str as FromSql<Text, Sqlite>>::from_sql(bytes)?.as_ref() }
                .unwrap()
                .parse()?,
        )
    }
}

impl std::str::FromStr for Provider {
    type Err = &'static str;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "lezhin" => Ok(Self::Lezhin),
            "naver" => Ok(Self::Naver),
            _ => Err("Unrecognized enum variant"),
        }
    }
}

impl std::fmt::Display for Provider {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        std::fmt::Debug::fmt(self, f)
    }
}

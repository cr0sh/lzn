use crate::error::Result;
use diesel::backend::Backend;
use diesel::deserialize::{self, FromSql};
use diesel::serialize::{self, Output, ToSql};
use diesel::sql_types::Text;
use diesel::sqlite::Sqlite;
use std::io::Write;

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
            Self::Naver => unimplemented!(),
        }
    }

    pub(crate) fn fetch_episodes(&self, client: &reqwest::Client, comic_id: &str) -> Result<()> {
        match self {
            Self::Lezhin => lezhin::fetch_episodes(client, comic_id),
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

mod lezhin {
    use crate::error::{Error, Result};
    use chrono::{serde::ts_seconds, DateTime, Utc};
    use select::document::Document;
    use select::predicate::{And, Attr, Name, Not};
    use serde::Deserialize;
    use std::collections::HashMap;

    const AUTH_URL: &str =
        "https://www.lezhin.com/ko/login/submit?redirect=http://www.lezhin.com/ko";
    const EPISODE_LIST_URL: &str = "https://www.lezhin.com/ko/comic/";

    /// __LZ_PRODUCT__.product JSON schema
    #[derive(Deserialize)]
    #[allow(dead_code)]
    struct LezhinProduct {
        display: HashMap<String, String>,
        alias: String,
        id: u64,
        episodes: Vec<EpisodeMetadata>,
    }

    #[derive(Deserialize)]
    #[allow(dead_code)]
    struct EpisodeMetadata {
        name: String,
        display: HashMap<String, String>,
        id: u64,
        #[serde(rename = "updatedAt")]
        #[serde(deserialize_with = "ts_seconds::deserialize")]
        updated_at: DateTime<Utc>,
        #[serde(rename = "freedAt")]
        #[serde(deserialize_with = "ts_seconds::deserialize")]
        freed_at: DateTime<Utc>,
    }

    pub(crate) fn authenticate(client: &reqwest::Client, id: &str, password: &str) -> Result<()> {
        let res = client
            .post(AUTH_URL)
            .form(&[
                ("redirect", "/ko"),
                ("username", id),
                ("password", password),
                ("remember_me", "false"),
            ])
            .send()?;

        log::debug!("Auth response url: {}", res.url().to_string());
        log::debug!("Auth response code: {}", res.status().to_string());
        // log::debug!("Auth cookies: {:#?}", res.cookies().collect::<Vec<_>>());

        // FIXME: If we authenticate outside Korea, how will the url change?
        // Maybe we should check not final url but initial redirection request,
        // but reqwest does not support per-request redirection policy currently.
        // ref: https://github.com/seanmonstar/reqwest/issues/353
        if res.url().to_string() == "https://www.lezhin.com/ko" {
            Ok(())
        } else {
            Err(Error::StaticStr(
                "Authentication failure: incorrect response url",
            ))
        }
    }

    fn fetch_product_object(client: &reqwest::Client, comic_id: &str) -> Result<LezhinProduct> {
        let doc = Document::from(
            client
                .get(&(String::from(EPISODE_LIST_URL) + comic_id))
                .send()?
                .text()?
                .as_ref(),
        );

        // Find script tag without id attribute
        // TODO: Can we flatten nested scopes?
        for sel in doc.find(And(
            Name("script"),
            And(Not(Attr("id", ())), Not(Attr("src", ()))),
        )) {
            const LZPRODUCT_START_TEXT: &'static str = "__LZ_PRODUCT__ = ";
            const PRODUCT_ATTR_START_TEXT: &'static str = "product: ";
            const PRODUCT_ATTR_END_TEXT: &'static str = ",\n        departure";
            if let Some(text) = sel
                .children()
                .next()
                .expect("expected script tag to have at least one child")
                .as_text()
            {
                if let (Some(start_offset), Some(end_offset)) =
                    (text.find(LZPRODUCT_START_TEXT), text.find("__LZ_DATA__"))
                {
                    let text = &text[start_offset + LZPRODUCT_START_TEXT.len()..end_offset];
                    if let (Some(json_start), Some(json_end)) = (
                        text.find(PRODUCT_ATTR_START_TEXT),
                        text.find(PRODUCT_ATTR_END_TEXT),
                    ) {
                        return Ok(serde_json::from_reader(
                            text[(json_start + PRODUCT_ATTR_START_TEXT.len())..json_end].as_bytes(),
                        )?);
                    } else {
                        eprintln!("not found");
                        continue;
                    }
                }
            }
        }

        Err(Error::StaticStr("Cannot find LZ_PRODUCT variable"))
    }

    pub(crate) fn fetch_episodes(client: &reqwest::Client, comic_id: &str) -> Result<()> {
        let eps = fetch_product_object(client, comic_id)?;
        for ep in eps.episodes {
            log::info!("found episode: {}", ep.display["title"]);
        }
        unimplemented!()
    }
}

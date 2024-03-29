use crate::error::Result;
use anyhow::anyhow;
use chrono::{offset::TimeZone, DateTime, Utc};
use diesel::prelude::*;
use select::document::Document;
use select::predicate::{And, Attr, Name, Not};
use serde::Deserialize;
use std::collections::HashMap;
use std::io::Read;

#[allow(dead_code)]
const MAIN_PAGE_URL: &str = "https://www.lezhin.com/ko";
const AUTH_URL: &str = "https://www.lezhin.com/ko/login?redirect=/ko";
const EPISODE_LIST_URL: &str = "https://www.lezhin.com/ko/comic/";
const COMIC_API_URL: &str = "https://www.lezhin.com/api/v2/inventory_groups/comic_viewer_k";
const CDN_BASE_URL: &str = "https://cdn.lezhin.com/v2";

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
    /// A number with optional prefixes(n,p,e)
    name: String,
    /// Additional information hashmap for displaying.
    /// title: A title to be displayed on a list(smaller text layout).
    /// displayName: A 'real' title to be displayed(bigger text layout).
    /// type: n(notice, temporary?), g(general), p(prologue), e(epilogue)
    display: HashMap<String, String>,
    id: u64,
    #[serde(rename = "updatedAt")]
    #[serde(deserialize_with = "chrono::serde::ts_milliseconds::deserialize")]
    updated_at: DateTime<Utc>, // Note: assumed UTC timezone for timestamp integer
    #[serde(rename = "freedAt")]
    #[serde(default)]
    #[serde(deserialize_with = "deserialize_optional_timestamp")]
    freed_at: Option<DateTime<Utc>>, // Note: assumed UTC timezone for timestamp integer
}

fn deserialize_optional_timestamp<'de, D>(
    deserializer: D,
) -> std::result::Result<Option<DateTime<Utc>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    if let Some(ts) = Option::deserialize(deserializer)? {
        Ok(Some(Utc.timestamp_millis(ts)))
    } else {
        Ok(None)
    }
}

pub(crate) fn fetch_authenticity_token(agent: &ureq::Agent) -> Result<String> {
    let resp = agent.get(AUTH_URL).call()?.into_string()?;

    log::trace!("Auth page response: \n{}", resp);

    let doc = Document::from(resp.as_ref());
    Ok(doc
        .find(And(Name("input"), Attr("name", "authenticity_token")))
        .next()
        .expect("Expected a authenticity token field on login form")
        .attr("value")
        .expect("authenticity_token field must have a value")
        .to_owned())
}

pub(crate) fn authenticate(agent: &ureq::Agent, id: &str, password: &str) -> Result<()> {
    let auth_token = fetch_authenticity_token(agent)?;
    log::debug!("authenticity_token: {}", auth_token);

    let encoded = url::form_urlencoded::Serializer::new(String::new())
        .extend_pairs(&[
            ("utf8", "✓"),
            ("authenticity_token", &auth_token),
            ("redirect", "/ko"),
            ("username", id),
            ("password", password),
            ("remember_me", "false"),
        ])
        .finish();
    let res = agent.post(AUTH_URL).send_string(&encoded)?;

    log::debug!(
        "Auth response url: {}",
        res.header("location").unwrap_or("<None>")
    );
    log::debug!("Auth response code: {}", res.status().to_string());
    log::debug!(
        "Auth cookies: {:#?}",
        res.header("set-cookie").unwrap_or("<None>")
    );

    // FIXME: If we authenticate outside Korea, how will the url change?
    // Maybe we should check not final url but initial redirection request,
    // but reqwest does not support per-request redirection policy currently.
    // ref: https://github.com/seanmonstar/reqwest/issues/353
    // update on 20200222: Moved to ureq. Will discuss later.
    // update on 20200808: Authentication process was changed. No need to check redirection page URL.
    // if let Some(u) = res.header("Location") {
    //     if u == "https://www.lezhin.com/ko" {
    //         Ok(())
    //     } else {
    //         Err("Authentication failure: incorrect response url")?
    //     }
    // } else {
    //     unreachable!()
    // }

    Ok(())
}

fn fetch_product_object(agent: &ureq::Agent, comic_id: &str) -> Result<LezhinProduct> {
    let doc = {
        let resp = agent
            .get(&(String::from(EPISODE_LIST_URL) + comic_id))
            .call()?;

        Document::from(resp.into_string()?.as_ref())
    };

    // Find script tag without id attribute
    // TODO: Can we flatten nested scopes?
    for sel in doc.find(And(
        Name("script"),
        And(Not(Attr("id", ())), Not(Attr("src", ()))),
    )) {
        const LZPRODUCT_START_TEXT: &str = "__LZ_PRODUCT__ = ";
        const PRODUCT_ATTR_START_TEXT: &str = "product: ";
        const PRODUCT_ATTR_END_TEXT: &str = ",\n        departure";
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
                    log::warn!(
                        "Found __LZ_PRODUCT__ object, but product attribute does not exist!"
                    );
                    continue;
                }
            }
        }
    }

    Err(anyhow!("Cannot find LZ_PRODUCT variable"))
}

pub(crate) fn fetch_episodes(
    agent: &ureq::Agent,
    comic_id_: &str,
    conn: &SqliteConnection,
) -> Result<()> {
    use crate::models::{ComicRecord, EpisodeRecord, TitleRecord};
    use crate::schema::comics::dsl::*;
    use crate::schema::episodes::dsl::*;
    use crate::schema::titles::dsl::*;

    let eps = fetch_product_object(agent, comic_id_)?;
    let rec = TitleRecord {
        provider: super::Provider::Lezhin,
        id: comic_id_.to_owned(),
        title: Some(eps.display["title"].to_owned()),
    };

    if !titles
        .filter(crate::schema::titles::dsl::provider.eq(&rec.provider))
        .filter(crate::schema::titles::dsl::id.eq(&rec.id))
        .load::<TitleRecord>(conn)?
        .is_empty()
    {
        diesel::update(
            titles
                .filter(crate::schema::titles::dsl::provider.eq(rec.provider))
                .filter(crate::schema::titles::dsl::id.eq(rec.id)),
        )
        .set(crate::schema::titles::dsl::title.eq(rec.title))
        .execute(conn)?;
    } else {
        diesel::insert_into(titles).values(&rec).execute(conn)?;
    }

    // API response shows recent episodes first, so it must be reversed order
    for (episode_idx, ep) in eps
        .episodes
        .iter()
        .rev()
        .filter(|ep| match ep.display.get("type").map(String::as_ref) {
            Some("n") => {
                log::debug!("Skipping notice episode {}", ep.display["title"]);
                false
            }
            Some(_) => true,
            None => {
                log::warn!(
                    r#"Expected string for display["type"] in episode {}"#,
                    ep.display["title"]
                );
                false
            }
        })
        .enumerate()
    {
        if !comics
            .filter(crate::schema::comics::dsl::provider.eq(super::Provider::Lezhin))
            .filter(comic_id.eq(comic_id_.to_owned()))
            .filter(episode_seq.eq(episode_idx as i32 + 1))
            .load::<ComicRecord>(conn)?
            .is_empty()
        {
            log::debug!(
                "Episode sequence {} (title {}) is already scraped. Skipping.",
                episode_idx as i32 + 1,
                ep.display["title"]
            );
            continue;
        }

        if ep.freed_at.unwrap_or_else(chrono::Utc::now) > chrono::Utc::now() {
            log::info!("Skipping unavailble episode: {}", ep.display["title"]);
            continue;
        }

        log::info!("Fetching episode: {}", ep.display["title"]);
        let images = fetch_episode(agent, comic_id_, ep)?;

        let recs = images
            .iter()
            .enumerate()
            .map(|(idx, img)| {
                ComicRecord {
                    provider: super::Provider::Lezhin,
                    comic_id: comic_id_.to_owned(),
                    episode_seq: episode_idx as i32 + 1, // 1-based index
                    episode_name: Some(ep.display["title"].clone()),
                    image_seq: idx as i32 + 1, // 1-based index
                    image: img.to_owned(),
                    updated_at: chrono::Local::now().naive_local(),
                }
            })
            .collect::<Vec<_>>();

        diesel::insert_into(comics)
            .values(&recs)
            .execute(conn)
            .unwrap_or_else(|e| {
                log::error!("Cannot insert images into database: {}", e);
                0
            });

        diesel::insert_into(episodes)
            .values(&EpisodeRecord {
                provider: super::Provider::Lezhin,
                comic_id: comic_id_.to_owned(),
                episode_seq: episode_idx as i32 + 1,
                title: Some(ep.display["title"].clone()),
                images_cnt: recs.len() as i32,
                created_at: chrono::Local::now().naive_local(),
                last_update: chrono::Local::now().naive_local(),
            })
            .execute(conn)
            .unwrap_or_else(|e| {
                log::error!("Cannot insert images into database: {}", e);
                0
            });
    }

    Ok(())
}

fn fetch_episode(
    agent: &ureq::Agent,
    comic_id: &str,
    episode: &EpisodeMetadata,
) -> Result<Vec<Vec<u8>>> {
    let resp = agent
        .get(COMIC_API_URL)
        .set("Accept", "application/json, text/javascript, */*; q=0.01")
        .set("Accept-Language", "ko-KR,ko;q=0.8,en-US;q=0.5,en;q=0.3")
        .query("platform", "web")
        .query("store", "web")
        .query("alias", comic_id)
        .query("name", episode.name.as_ref())
        .query("preload", "true")
        .query("type", "comic_episode")
        .call()?;

    let json: serde_json::Value = resp
        .into_json()
        .map_err(|_| anyhow!("Non-OK result for comic_view_k API request"))?;

    if json["code"]
        .as_u64()
        .ok_or_else(|| anyhow!("Expected integer code for API response"))?
        != 0
    {
        log::error!(
            "Lezhin API returned non-zero code {:?}",
            json["code"].as_u64()
        );
        return Err(anyhow!("Lezhin API returned non-zero code"));
    }

    json["data"]["extra"]["episode"]["scrollsInfo"]
        .as_array()
        .ok_or_else(|| anyhow!("Expected list of image items"))?
        .iter()
        .map(|entry| {
            let url = String::from(CDN_BASE_URL)
                + entry["path"]
                    .as_str()
                    .ok_or_else(|| anyhow!("Expected string path for image item"))?;

            let resp = agent.get(url.as_ref()).call()?;
            let mut buf = Vec::new();
            resp.into_reader().read_to_end(&mut buf)?;

            Ok(buf)
        })
        .collect::<Result<Vec<_>>>()
}

pub(crate) fn fetch_titles(agent: &ureq::Agent, comic_ids: Vec<String>) -> Result<Vec<String>> {
    comic_ids
        .iter()
        .map(|comic_id| {
            log::debug!("Fetching title for comic ID {}", comic_id);
            Ok(fetch_product_object(agent, comic_id)?.display["title"].clone())
        })
        .collect::<Result<Vec<_>>>()
}

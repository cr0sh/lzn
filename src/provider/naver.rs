use crate::error::Result;
use diesel::prelude::*;
use reqwest::Url;
use select::document::Document;
use select::predicate::{And, Attr, Class, Name};

const MOBILE_COMIC_BASE_URL: &'static str = "https://m.comic.naver.com";
const MOBILE_EPISODE_LIST_URL: &'static str = "https://m.comic.naver.com/webtoon/list.nhn";
const COMIC_EPISODE_PAGE_URL: &'static str = "https://comic.naver.com/webtoon/detail.nhn";

pub(crate) enum SortOrder {
    Ascending,
    Descending,
}

impl SortOrder {
    fn to_str(self) -> &'static str {
        match self {
            Self::Ascending => "ASC",
            Self::Descending => "DESC",
        }
    }
}

/// Parses episode list page into vector of tuples: (number, title, URL).
pub(crate) fn fetch_episode_list_page(
    client: &reqwest::Client,
    comic_id: &str,
    page: u32,
    order: SortOrder,
) -> Result<(String, Vec<(u32, String, Url)>)> {
    let url = Url::parse_with_params(
        MOBILE_EPISODE_LIST_URL,
        &[
            ("titleId", comic_id),
            ("sortOrder", order.to_str()),
            ("page", &page.to_string()),
        ],
    )
    .unwrap();

    let mut resp = client.get(url).send()?;
    if resp.status() != reqwest::StatusCode::OK {
        Err("Non-OK response from Naver episode list page")?;
    }

    let doc = Document::from(resp.text()?.as_ref());

    let episodes = doc
        .find(And(Name("ul"), Class("section_episode_list")))
        .map(|doc| {
            doc.find(And(Name("li"), Class("item")))
                .into_iter()
                .map(|item| {
                    Ok((
                        item.attrs()
                            .collect::<std::collections::HashMap<_, _>>()
                            .get("data-no")
                            .ok_or("Cannot parse data-no attribute from episode list item")?
                            .parse::<u32>()?,
                        item.find(Class("name"))
                            .next()
                            .ok_or("Expected title name in episode item")?
                            .text(),
                        String::from(
                            *item
                                .find(And(Name("a"), Class("link")))
                                .next()
                                .ok_or("Cannot find episode page link item")?
                                .attrs()
                                .collect::<std::collections::HashMap<_, _>>()
                                .get("href")
                                .ok_or("Cannot get episode page link data(href) from item")?,
                        ),
                    ))
                })
        })
        .flatten()
        .collect::<Result<Vec<_>>>()?
        .into_iter()
        .filter(|(_, _, url)| url != "#")
        .map(|(no, title, url)| {
            Ok((
                no,
                title,
                reqwest::Url::parse(&(String::from(MOBILE_COMIC_BASE_URL) + &url))?,
            ))
        })
        .collect::<Result<Vec<_>>>()?;

    let comic_title = doc
        .find(And(Name("meta"), Attr("property", "og:title")))
        .next()
        .ok_or("Expected comic title metadata in episode list page")?
        .attr("content")
        .expect("Found og:title metadata in episode list but there is no content attribute")
        .to_string();

    Ok((comic_title, episodes))
}

pub(crate) fn fetch_episode(
    client: &reqwest::Client,
    comic_id_: &str,
    episode_num: u32,
) -> Result<(String, Vec<Vec<u8>>)> {
    use std::io::Read;

    const FAKE_CHROME_74_UA: &'static str="Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/74.0.3729.169 Safari/537.36";

    let url = Url::parse_with_params(
        COMIC_EPISODE_PAGE_URL,
        &[("titleId", comic_id_), ("no", &episode_num.to_string())],
    )
    .expect("Generated URL must be valid");
    let doc = Document::from(client.get(url).send()?.text()?.as_ref());
    let image_links = doc
        .find(Class("wt_viewer"))
        .next()
        .ok_or("Expected wt_viewer element in episode page")?
        .find(Name("img"))
        .into_iter()
        .map(|item| {
            item.attr("src")
                .ok_or("Expected src link in episode img element")
        })
        .collect::<Result<Vec<_>, _>>()?;

    let images = image_links
        .into_iter()
        .map(|link| {
            log::debug!("image link: {}", link);
            let resp = client
                .get(link)
                .header(reqwest::header::USER_AGENT, FAKE_CHROME_74_UA)
                .send()?;
            if resp.status() != reqwest::StatusCode::OK {
                log::error!(
                    "Server returned non-OK response for image request: {}",
                    resp.status()
                );
                Err("Server returned non-OK response for image request")?;
            }
            Ok(resp.bytes().collect::<Result<Vec<_>, _>>()?)
        })
        .collect::<Result<Vec<_>>>()?;

    let title = doc
        .find(Class("tit_area"))
        .next()
        .ok_or("Expected title area element in episode page")?
        .find(Name("h3"))
        .next()
        .expect("Expected title")
        .text();

    Ok((title, images))
}

pub(crate) fn fetch_episodes(
    client: &reqwest::Client,
    comic_id_: &str,
    conn: &SqliteConnection,
) -> Result<()> {
    use crate::models::{ComicRecord, TitleRecord};
    use crate::schema::comics::dsl::*;
    use crate::schema::titles::dsl::*;

    let (comic_title, first_list) =
        fetch_episode_list_page(client, comic_id_, 1, SortOrder::Ascending)?;
    let first_num = first_list[0].0;
    let last_num = (fetch_episode_list_page(client, comic_id_, 1, SortOrder::Descending)?.1)[0].0;

    log::info!("Title found for current comic: {}", comic_title);

    let rec = TitleRecord {
        provider: super::Provider::Naver,
        id: comic_id_.to_owned(),
        title: Some(comic_title),
    };

    if titles
        .filter(crate::schema::titles::dsl::provider.eq(&rec.provider))
        .filter(id.eq(&rec.id))
        .load::<TitleRecord>(conn)?
        .len()
        > 0
    {
        diesel::update(
            titles
                .filter(crate::schema::titles::dsl::provider.eq(rec.provider))
                .filter(id.eq(rec.id)),
        )
        .set(title.eq(rec.title))
        .execute(conn)?;
    } else {
        diesel::insert_into(titles).values(&rec).execute(conn)?;
    }

    for ep_num in first_num..=last_num {
        if comics
            .filter(crate::schema::comics::dsl::provider.eq(super::Provider::Naver))
            .filter(comic_id.eq(comic_id_.to_owned()))
            .filter(episode_seq.eq(ep_num as i32))
            .load::<ComicRecord>(conn)?
            .len()
            > 0
        {
            log::info!(
                "Skipping episode sequence {} because record exists already",
                ep_num
            );
            continue;
        }

        let (title_, eps) = fetch_episode(client, comic_id_, ep_num)?;
        log::info!("Saving episode {}: {}", ep_num, title_);

        let recs = eps
            .iter()
            .enumerate()
            .map(|(idx, img)| ComicRecord {
                provider: super::Provider::Naver,
                comic_id: comic_id_.to_owned(),
                episode_seq: ep_num as i32,
                episode_name: Some(title_.clone()),
                image_seq: idx as i32 + 1,
                image: img.to_owned(),
                updated_at: chrono::Local::now().naive_local(),
            })
            .collect::<Vec<_>>();

        diesel::insert_into(comics)
            .values(&recs)
            .execute(conn)
            .unwrap_or_else(|e| {
                log::error!("Cannot insert images into database: {}", e);
                0
            });
    }

    unimplemented!()
}

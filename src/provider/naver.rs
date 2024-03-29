use crate::error::Result;
use anyhow::anyhow;
use diesel::prelude::*;
use select::document::Document;
use select::predicate::{And, Attr, Class, Name};

const MOBILE_COMIC_BASE_URL: &str = "https://m.comic.naver.com";
const MOBILE_EPISODE_LIST_URL: &str = "https://m.comic.naver.com/webtoon/list.nhn";
const COMIC_EPISODE_PAGE_URL: &str = "https://comic.naver.com/webtoon/detail.nhn";

pub(crate) enum SortOrder {
    Ascending,
    Descending,
}

impl SortOrder {
    fn to_str(&self) -> &'static str {
        match self {
            Self::Ascending => "ASC",
            Self::Descending => "DESC",
        }
    }
}

/// Parses episode list page into vector of tuples: (number, title, URL).
#[allow(clippy::type_complexity)]
pub(crate) fn fetch_episode_list_page(
    agent: &ureq::Agent,
    comic_id: &str,
    page: u32,
    order: SortOrder,
) -> Result<(String, Vec<(u32, String, url::Url)>)> {
    let resp = agent.get(MOBILE_EPISODE_LIST_URL).send_form(&[
        ("titleId", comic_id),
        ("sortOrder", order.to_str()),
        ("page", &page.to_string()),
    ])?;

    let doc = Document::from(resp.into_string()?.as_ref());

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
                            .ok_or_else(|| {
                                anyhow!("Cannot parse data-no attribute from episode list item")
                            })?
                            .parse::<u32>()?,
                        item.find(Class("name"))
                            .next()
                            .ok_or_else(|| anyhow!("Expected title name in episode item"))?
                            .text(),
                        String::from(
                            *item
                                .find(And(Name("a"), Class("link")))
                                .next()
                                .ok_or_else(|| anyhow!("Cannot find episode page link item"))?
                                .attrs()
                                .collect::<std::collections::HashMap<_, _>>()
                                .get("href")
                                .ok_or_else(|| {
                                    anyhow!("Cannot get episode page link data(href) from item")
                                })?,
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
                ::url::Url::parse(&(String::from(MOBILE_COMIC_BASE_URL) + &url)).unwrap(),
            ))
        })
        .collect::<Result<Vec<_>>>()?;

    let comic_title = doc
        .find(And(Name("meta"), Attr("property", "og:title")))
        .next()
        .ok_or_else(|| anyhow!("Expected comic title metadata in episode list page"))?
        .attr("content")
        .expect("Found og:title metadata in episode list but there is no content attribute")
        .to_string();

    Ok((comic_title, episodes))
}

pub(crate) fn fetch_episode(
    agent: &ureq::Agent,
    comic_id_: &str,
    episode_num: u32,
) -> Result<(String, Vec<Vec<u8>>)> {
    use std::io::Read;
    const FAKE_CHROME_74_UA: &str="Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/74.0.3729.169 Safari/537.36";

    // let url = Url::parse_with_params(
    //     COMIC_EPISODE_PAGE_URL,
    //     &[("titleId", comic_id_), ("no", &episode_num.to_string())],
    // )
    // .expect("Generated URL must be valid");

    let doc = {
        let resp = agent
            .get(COMIC_EPISODE_PAGE_URL)
            .send_form(&[("titleId", comic_id_), ("no", &episode_num.to_string())])?;
        Document::from(resp.into_string()?.as_ref())
    };
    let image_links = doc
        .find(Class("wt_viewer"))
        .next()
        .ok_or_else(|| anyhow!("Expected wt_viewer element in episode page"))?
        .find(Name("img"))
        .into_iter()
        .map(|item| {
            item.attr("src")
                .ok_or_else(|| anyhow!("Expected src link in episode img element"))
        })
        .collect::<Result<Vec<_>, _>>()?;

    let images = image_links
        .into_iter()
        .map(|link| {
            log::debug!("image link: {}", link);
            let resp = agent
                .get(link)
                .set("User-Agent", FAKE_CHROME_74_UA)
                .call()?;
            Ok(resp.into_reader().bytes().collect::<Result<Vec<_>, _>>()?)
        })
        .collect::<Result<Vec<_>>>()?;

    let title = doc
        .find(Class("tit_area"))
        .next()
        .ok_or_else(|| anyhow!("Expected title area element in episode page"))?
        .find(Name("h3"))
        .next()
        .expect("Expected title")
        .text();

    Ok((title, images))
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

    let (comic_title, first_list) =
        fetch_episode_list_page(agent, comic_id_, 1, SortOrder::Ascending)?;
    let first_num = first_list[0].0;
    let last_num = (fetch_episode_list_page(agent, comic_id_, 1, SortOrder::Descending)?.1)[0].0;

    log::info!("Title found for current comic: {}", comic_title);

    let rec = TitleRecord {
        provider: super::Provider::Naver,
        id: comic_id_.to_owned(),
        title: Some(comic_title),
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

    for ep_num in first_num..=last_num {
        if !comics
            .filter(crate::schema::comics::dsl::provider.eq(super::Provider::Naver))
            .filter(comic_id.eq(comic_id_.to_owned()))
            .filter(episode_seq.eq(ep_num as i32))
            .load::<ComicRecord>(conn)?
            .is_empty()
        {
            log::debug!(
                "Skipping episode sequence {} because record exists already",
                ep_num
            );
            continue;
        }

        let (title_, eps) = fetch_episode(agent, comic_id_, ep_num)?;
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

        diesel::insert_into(episodes)
            .values(&EpisodeRecord {
                provider: super::Provider::Naver,
                comic_id: comic_id_.to_owned(),
                episode_seq: ep_num as i32,
                title: Some(title_.clone()),
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

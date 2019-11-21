use crate::error::Result;
use crate::provider::Provider;
use diesel::prelude::*;
use reqwest::Client;

/// Starts scraping.
/// Target lists are in given database's `scrap_targets` table.
pub fn start(conn: &SqliteConnection, id_: &str, pw_: &str) -> Result<()> {
    use crate::models::{ScrapingStatus, ScrapingTarget};
    use crate::schema::scraping_targets::dsl::*;
    let targets: Vec<crate::models::ScrapingTarget> =
        scraping_targets.load::<ScrapingTarget>(conn)?;
    let client = Client::builder().cookie_store(true).build()?;
    Provider::Lezhin.authenticate(&client, id_, pw_)?;
    log::debug!(
        "Client authentication succeeded for provider {}",
        Provider::Lezhin
    );

    for target in targets {
        if target.status != ScrapingStatus::Enabled {
            log::debug!(
                "Ignoring target {}/{} due to its status: {:?}",
                target.provider,
                target.id,
                target.status
            );
            continue;
        }

        if target.provider != Provider::Lezhin {
            unimplemented!() // TODO
        }

        log::info!("Scraping target {}/{}", target.provider, target.id);

        target.provider.fetch_episodes(&client, &target.id, &conn)?;
        diesel::update(scraping_targets.find((target.provider, target.id)))
            .set(last_scrap.eq(chrono::Local::now().naive_local()))
            .execute(conn)?;
    }

    Ok(())
}

/// Scrape unknown titles.
pub fn scrap_titles(conn: &SqliteConnection, id_: &str, pw_: &str) -> Result<usize> {
    use crate::schema::titles::dsl::*;

    let targets = titles
        .select(id)
        .filter(provider.eq(Provider::Lezhin))
        .filter(title.is_null())
        .load(conn)?;

    let client = Client::builder().cookie_store(true).build()?;
    Provider::Lezhin.authenticate(&client, id_, pw_)?;

    let titles_ = Provider::Lezhin.fetch_titles(&client, targets.clone())?;
    for (target, title_) in targets.iter().zip(titles_.iter()) {
        diesel::update(titles.find((Provider::Lezhin, target)))
            .set(title.eq(title_))
            .execute(conn)?;
    }

    Ok(targets.len())
}

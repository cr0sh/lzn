use crate::error::Result;
use crate::provider::Provider;
use diesel::prelude::*;

pub(crate) const FAKE_UA: &str =
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:79.0) Gecko/20100101 Firefox/79.0";

/// Starts scraping.
/// Target lists are in given database's `scrap_targets` table.
pub fn start(conn: &SqliteConnection, id_: &str, pw_: &str) -> Result<()> {
    use crate::models::{ScrapingStatus, ScrapingTarget};
    use crate::schema::scraping_targets::dsl::*;
    let targets: Vec<crate::models::ScrapingTarget> =
        scraping_targets.load::<ScrapingTarget>(conn)?;
    let agent = ureq::AgentBuilder::new()
        .user_agent(FAKE_UA)
        .redirects(0)
        .build();

    Provider::Lezhin.authenticate(&agent, id_, pw_)?; // TODO: Authenticate client for other providers
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

        log::info!("Scraping target {}/{}", target.provider, target.id);

        target.provider.fetch_episodes(&agent, &target.id, conn)?;
        diesel::update(scraping_targets.find((target.provider, target.id)))
            .set(last_scraping.eq(chrono::Local::now().naive_local()))
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

    let agent = ureq::Agent::new();
    Provider::Lezhin.authenticate(&agent, id_, pw_)?;

    let titles_ = Provider::Lezhin.fetch_titles(&agent, targets.clone())?;
    for (target, title_) in targets.iter().zip(titles_.iter()) {
        diesel::update(titles.find((Provider::Lezhin, target)))
            .set(title.eq(title_))
            .execute(conn)?;
    }

    Ok(targets.len())
}

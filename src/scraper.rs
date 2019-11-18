use crate::error::Result;
use crate::provider::Provider;
use diesel::prelude::*;
use reqwest::Client;

/// Starts scraping.
/// Target lists are in given database's `scrap_targets` table.
pub fn start(conn: &SqliteConnection, id_: &str, pw_: &str) -> Result<()> {
    use crate::models::{ScrapStatus, ScrapTarget};
    use crate::schema::scrap_targets::dsl::*;
    let targets: Vec<crate::models::ScrapTarget> = scrap_targets.load::<ScrapTarget>(conn)?;
    let client = Client::builder().cookie_store(true).build()?;
    Provider::Lezhin.authenticate(&client, id_, pw_)?;
    log::debug!(
        "Client authentication succeeded for provider {}",
        Provider::Lezhin
    );

    for target in targets {
        if target.status != ScrapStatus::Enabled {
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

        target.provider.fetch_episodes(&client, &target.id)?;
    }

    Ok(())
}

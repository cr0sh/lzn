//! Database wrapper.
//! Uses diesel with SQLite backend,

use crate::error::Result;
use crate::provider::Provider;
use chrono::{DateTime, Local};
use derive_builder::Builder;
use diesel::sqlite::SqliteConnection;
use serde::{Deserialize, Serialize};
use std::iter::Iterator;

/// Database wrapper instance.
pub struct Storage {
    conn: SqliteConnection,
}

impl Storage {
    fn put_comic(&self, comic: Comic) -> Result<()> {
        unimplemented!()
    }

    fn put_episode(&self, comic: &Comic, episode: Episode) -> Result<()> {
        let key = bincode::serialize(&comic.dir().with_episode(&episode))?;
        unimplemented!()
    }
}

#[derive(Debug)]
pub struct Comic {
    title: String,
    provider: Provider,
    last_access: Option<DateTime<Local>>,
    /// List of episodes.
    /// Orders must match with contents of original website.
    episodes: Vec<Episode>,
}

impl Comic {
    pub fn episodes(&self) -> impl Iterator<Item = &Episode> {
        self.episodes.iter()
    }

    pub(crate) fn dir(&self) -> ComicDir {
        ComicDir {
            title: &self.title,
            provider: self.provider,
        }
    }
}

/// Serializable comic metadata for creating directory of DB.
#[derive(Serialize, Deserialize, Debug)]
pub struct ComicDir<'a> {
    title: &'a str,
    provider: Provider,
}

impl<'a> ComicDir<'a> {
    fn with_episode(self, episode: &'a Episode) -> EpisodeDir {
        EpisodeDir {
            comic_dir: self,
            title: &episode.title,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Builder)]
pub struct Episode {
    title: String,
    #[builder(default, setter(into, strip_option))]
    released_when: Option<DateTime<Local>>,
    #[builder(default, setter(into, strip_option))]
    accessed_when: Option<DateTime<Local>>,
    images: Vec<Vec<u8>>,
}

impl Episode {
    pub fn from_raw_files(title: String, images: Vec<Vec<u8>>) -> Self {
        Self {
            title,
            released_when: None,
            accessed_when: None,
            images,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct EpisodeDir<'a> {
    comic_dir: ComicDir<'a>,
    title: &'a str,
}

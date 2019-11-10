use crate::error::Result;
use crate::models::ComicRecord;
use crate::schema;
use crate::util::try_windows_949;
use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;
// directory
#[derive(Serialize, Deserialize, Debug)]
enum DBKey {
    Dir(Vec<u8>),
    File(Vec<u8>),
    Root,
}

#[derive(Serialize, Deserialize, Debug)]
enum DirValue {
    // subdirectory names
    Dirs(Vec<Vec<u8>>),
    // filenames
    Files(Vec<Vec<u8>>),
}

/// Imports image files from zip archive.
pub fn migrate_zip(conn: &SqliteConnection, dir: PathBuf) -> Result<(usize, usize)> {
    let mut files_count = 0;
    let mut failed_records: usize = 0;
    let mut z = zip::ZipArchive::new(File::open(dir)?)?;
    log::info!("zip file contains {} files/directories in total.", z.len());
    for i in 0..z.len() {
        let mut file = z.by_index(i)?;
        let name = try_windows_949(file.name_raw());
        let name_split = name.split('/').collect::<Vec<_>>();

        if file.is_dir() {
            continue;
        }

        if name_split.len() != 3 {
            log::info!("File {} does not have 3-length path; ignoring.", name);
            continue;
        }

        let comic = name_split[0].to_string();
        let (episode_seq, episode) = {
            let mut it = name_split[1].splitn(2, " - ");
            let seq = it
                .next()
                .expect("Episode directory name must contain index")
                .parse::<i32>()?;
            let ep = it.next().expect("Episode directory name must contain name");
            (seq, ep.to_string())
        };
        let picture_seq = {
            let filename = name_split[2];
            filename[0..(filename.len() - 4)].parse()?
        };

        let mut picture = Vec::with_capacity(file.size() as usize);
        file.read_to_end(&mut picture)?;

        let record = ComicRecord {
            comic,
            episode_seq,
            episode: Some(episode),
            picture_seq,
            picture: Some(picture),
            updated_at: chrono::Local::now().naive_local(),
        };
        if let Err(e) = diesel::insert_into(schema::lezhin::table)
            .values(&record)
            .execute(conn)
        {
            log::debug!(
                "Record failed: dir {}, record {:#?}, cause: {}",
                name,
                ComicRecord {
                    picture: None,
                    ..record
                },
                e,
            );

            failed_records += 1;
            continue;
        }

        files_count += 1;
    }

    Ok((files_count, failed_records))
}

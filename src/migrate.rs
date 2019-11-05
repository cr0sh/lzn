use crate::error::Result;
use crate::util::try_windows_949;
use serde::{Deserialize, Serialize};
use sled::Db;
use std::fs::File;
use std::io::{Read, Result as IOResult};
use std::path::PathBuf;
use zip::read::ZipArchive;
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
pub fn migrate_zip(db: &Db, dir: PathBuf) -> Result<()> {
    log::info!(
        "Imported {} images from {:?}",
        zip_to_db(db, dir.clone())?,
        &dir
    );
    log::info!(
        "Imported images ({} in total) are written into database.",
        organize_db(db)?
    );
    Ok(())
}

fn zip_to_db(db: &Db, dir: PathBuf) -> Result<usize> {
    let mut files_count = 0;
    let mut z = zip::ZipArchive::new(File::open(dir)?)?;
    log::info!("zip file contains {} files in total.", z.len());
    for i in 0..z.len() {
        let mut file = z.by_index(i)?;
        let name = try_windows_949(file.name_raw());
        let name_split = name.split('/').collect::<Vec<_>>();
        if file.is_dir() {
            // two cases to handle:
            //  - [comic_name, episode_name, ""]
            //  - [comic_name, ""]
            // ... otherwise just ignore it.

            if name_split.len() > 3 || name_split.len() < 2 {
                log::info!("Ignoring useless directory: {}", name);
                continue;
            }

            let parent_dir_key = match name_split.len() {
                3 => DBKey::Dir(String::from(name_split[name_split.len() - 3]).into_bytes()),
                2 => DBKey::Root,
                _ => unreachable!(),
            };

            let mut parent_dir_contents = db
                .get(bincode::serialize(&parent_dir_key)?)?
                .map(|x| bincode::deserialize::<DirValue>(&x))
                .unwrap_or_else(|| Ok(DirValue::Dirs(Vec::new())))?;

            match parent_dir_contents {
                DirValue::Files(x) => {
                    log::error!(
                        "Both files and directories exist in same directory {}",
                        name
                    );
                    log::error!(
                        "Directory contents: {:?}",
                        x.iter()
                            .map(AsRef::as_ref)
                            .map(String::from_utf8_lossy)
                            .collect::<Vec<_>>()
                    );
                    continue;
                }
                DirValue::Dirs(ref mut v) => {
                    v.push(name.into_bytes());
                }
            }

            db.insert(
                bincode::serialize(&parent_dir_key)?,
                bincode::serialize(&parent_dir_contents)?,
            )?;
        } else {
            if name_split.len() != 3 {
                log::info!("File {} does not have 3-length path; ignoring.", name);
                continue;
            }

            let parent_dir_key =
                DBKey::Dir(String::from(name_split[name_split.len() - 2]).into_bytes());
            let mut parent_dir_contents = db
                .get(bincode::serialize(&parent_dir_key)?)?
                .map(|x| bincode::deserialize::<DirValue>(&x))
                .unwrap_or_else(|| Ok(DirValue::Files(Vec::new())))?;

            let name_bytes = name.clone().into_bytes();

            match parent_dir_contents {
                DirValue::Dirs(x) => {
                    log::error!(
                        "Both files and directories exist in same directory {}",
                        name
                    );
                    log::error!(
                        "Directory contents: {:?}",
                        x.iter()
                            .map(AsRef::as_ref)
                            .map(String::from_utf8_lossy)
                            .collect::<Vec<_>>()
                    );
                    continue;
                }
                DirValue::Files(ref mut v) => {
                    v.push(name_bytes.clone());
                }
            }

            db.insert(
                bincode::serialize(&parent_dir_key)?,
                bincode::serialize(&parent_dir_contents)?,
            )?;

            let my_dir_key = DBKey::Dir(name_bytes.clone());
            let mut buf = Vec::with_capacity(file.size() as usize);
            file.read_to_end(&mut buf)?;
            if db.insert(bincode::serialize(&my_dir_key)?, buf)?.is_some() {
                log::error!(
                    "Files must be unique: file already exists in {}",
                    String::from_utf8_lossy(&name_bytes)
                );
            }

            files_count += 1;
        }
    }

    Ok(files_count)
}

fn organize_db(db: &Db) -> Result<usize> {
    let comic_names = match db
        .get(&bincode::serialize(&DBKey::Root)?)?
        .map(|x| bincode::deserialize::<DirValue>(&x))
        .unwrap()?
    {
        DirValue::Dirs(dirnames) => dirnames,
        DirValue::Files(_) => unreachable!(),
    };

    unimplemented!()
}

/// Imports image files from directory.
pub fn migrate_dir(db: &Db, dir: PathBuf) -> IOResult<usize> {
    unimplemented!()
}

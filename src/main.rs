use sled::Db;
use std::error::Error;
use std::path::PathBuf;
use structopt::StructOpt;

use lzn::{merge, migrate, util};

#[derive(Debug, StructOpt)]
#[structopt(name = "lzn", about = "lezhin crawler & image database manager")]
enum Cmd {
    /// Merges jpgs from given directory into single image.
    #[structopt(name = "merge")]
    MergeImages {
        #[structopt(parse(from_os_str))]
        dir: PathBuf,
        #[structopt(short, long, parse(from_os_str), default_value = "merged.png")]
        out: PathBuf,
    },

    /// Merges images vertical in each subdirectories of given directory.
    #[structopt(name = "merge-dirs")]
    MergeDirs {
        #[structopt(parse(from_os_str))]
        dir: PathBuf,
        #[structopt(short, long, default_value = "merged.png")]
        out: String,
    },

    /// Migrates zip-based file or directory into rkv(LMDB) database.
    #[structopt(name = "migrate")]
    Migrate {
        /// Zip archive or directory to import.
        #[structopt(parse(from_os_str))]
        dir: PathBuf,
        /// LMDB database to save.
        #[structopt(parse(from_os_str))]
        db: Option<PathBuf>,
    },
}

impl Cmd {
    fn process(self) -> Result<(), Box<dyn Error>> {
        match self {
            Cmd::MergeImages { mut dir, out } => {
                dir.push("[0-9]*.jpg");
                let paths = util::sort_by_name_order(
                    glob::glob(dir.to_str().ok_or("unable to convert PathBuf to str")?)?
                        .collect::<Result<Vec<PathBuf>, _>>()?,
                );

                log::info!(
                    "found {} images from glob pattern {}. merging and saving...",
                    paths.len(),
                    dir.to_str().unwrap()
                );
                merge::merge_paths_vertical(paths)?.save(out)?;
            }
            Cmd::MergeDirs { mut dir, out } => {
                dir.push("[0-9]* - *");
                log::debug!("merge-dirs path: {}", dir.to_str().unwrap());
                for path in glob::glob(dir.to_str().ok_or("unable to convert PathBuf to str")?)? {
                    let mut iout = path?;
                    log::info!("Merging images inside {:?}", iout.to_str().unwrap());
                    let path = iout.clone();
                    iout.push(&out);
                    (Self::MergeImages {
                        dir: path,
                        out: iout,
                    })
                    .process()?;
                }
            }
            Cmd::Migrate { dir, db } => {
                let db = match db {
                    Some(path) => path,
                    None => {
                        let mut path = dirs::home_dir()
                            .ok_or("Unable to get home directory of current user")?;
                        path.push("lzn");
                        path
                    }
                };

                if log::log_enabled!(log::Level::Info) {
                    log::info!("Opening Sled DB at {:?}", db.clone());
                }

                let t = Db::open(db)?;

                if dir.is_dir() {
                    log::info!("Migrating from directory {}", dir.to_str().unwrap());
                    log::info!(
                        "Success: added {} images to database.",
                        migrate::migrate_dir(&t, dir)?
                    );
                } else {
                    log::info!("Migrating from archive {}", dir.to_str().unwrap());
                    migrate::migrate_zip(&t, dir)?;
                    log::info!("Migration complete.");
                }
            }
        }

        Ok(())
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("lzn=info"));

    let opt = Cmd::from_args();
    log::debug!("opt: {:?}", opt);

    opt.process()
}

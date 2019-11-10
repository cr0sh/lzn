#[macro_use]
extern crate diesel_migrations;

use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use diesel_migrations::{
    any_pending_migrations, embed_migrations, run_pending_migrations, RunMigrationsError,
};
use std::error::Error;
use std::path::PathBuf;
use structopt::StructOpt;

use lzn::{merge, migrate, util, web};

const DEFAULT_DATABASE_NAME: &str = "lzn.sqlite";
const DEFAULT_LOG_ENV: &str = "lzn=info,actix_web=info";

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

    /// Migrates zip archive into database.
    #[structopt(name = "migrate")]
    Migrate {
        /// Zip archive to import.
        #[structopt(parse(from_os_str))]
        dir: PathBuf,
        /// Database path. If not provided defaults to ~/lzn.sqlite
        #[structopt(parse(from_os_str))]
        db: Option<PathBuf>,
    },

    /// Run Diesel DB migrations on given database path.
    #[structopt(name = "setup")]
    Setup {
        /// Database path. If not provided defaults to ~/lzn.sqlite
        #[structopt(parse(from_os_str))]
        db: Option<PathBuf>,
    },

    /// Serve database contents as web document.
    #[structopt(name = "serve")]
    Serve {
        /// Database path. If not provided defaults to ~/lzn.sqlite
        #[structopt(parse(from_os_str))]
        db: Option<PathBuf>,
        /// Port number to listen HTTP requests.
        #[structopt(short, long, default_value = "localhost:8333")]
        addr: String,
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
                let dbpath = match db {
                    Some(path) => path,
                    None => {
                        let mut path = dirs::home_dir()
                            .ok_or("Unable to get home directory of current user")?;
                        path.push(DEFAULT_DATABASE_NAME);
                        path
                    }
                };

                if log::log_enabled!(log::Level::Info) {
                    log::info!("Opening SQLite DB at {:?}", dbpath.clone());
                }

                let conn = SqliteConnection::establish(
                    dbpath.to_str().expect("Converting PathBuf to &str failed"),
                )
                .map_err(|e| format!("Cannot connect database: {:?}", e))?;

                check_migrations(&conn)?;

                log::info!("Migrating from archive {}", dir.to_str().unwrap());
                let res = migrate::migrate_zip(&conn, dir)?;
                log::info!(
                    "Migration complete. Imported {} images. {} records are failed to be inserted.",
                    res.0,
                    res.1,
                );
            }
            Cmd::Setup { db } => {
                let dbpath = match db {
                    Some(path) => path,
                    None => {
                        let mut path = dirs::home_dir()
                            .ok_or("Unable to get home directory of current user")?;
                        path.push(DEFAULT_DATABASE_NAME);
                        path
                    }
                };

                log::info!("Setup executing on {:?}", &dbpath);

                let conn = SqliteConnection::establish(
                    dbpath.to_str().expect("Converting PathBuf to &str failed"),
                )
                .map_err(|e| format!("Cannot connect database: {:?}", e))?;

                run_pending_migrations(&conn)?;

                log::info!("Setup succeeded.");
            }
            Cmd::Serve { db, addr } => {
                let dbpath = match db {
                    Some(path) => path,
                    None => {
                        let mut path = dirs::home_dir()
                            .ok_or("Unable to get home directory of current user")?;
                        path.push(DEFAULT_DATABASE_NAME);
                        path
                    }
                };

                let conn = SqliteConnection::establish(
                    dbpath.to_str().expect("Converting PathBuf to &str failed"),
                )
                .map_err(|e| format!("Cannot connect database: {:?}", e))?;

                check_migrations(&conn)?;

                log::info!("Serving {} on {}", dbpath.to_str().unwrap(), addr,);

                web::serve(addr, conn)?;
            }
        }

        Ok(())
    }
}

embed_migrations!("migrations");

fn check_migrations(conn: &SqliteConnection) -> Result<(), RunMigrationsError> {
    if any_pending_migrations(conn)? {
        log::warn!("Some migrations are not yet applied.");
        log::warn!("Please run `lzn setup` to apply these migrations.")
    }
    Ok(())
}

fn main() {
    env_logger::init_from_env(env_logger::Env::new().default_filter_or(DEFAULT_LOG_ENV));

    let opt = Cmd::from_args();
    log::debug!("opt: {:?}", opt);

    if let Err(e) = opt.process() {
        log::error!(
            "Error: {}, source: {:?}",
            e,
            e.source().map(ToString::to_string)
        );
        std::process::exit(1)
    }
}

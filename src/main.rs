use std::error::Error;
use std::path::PathBuf;
use structopt::StructOpt;

use lzn::merge;

fn sort_by_name_order(mut paths: Vec<PathBuf>) -> Vec<PathBuf> {
    paths.sort_unstable_by(|x, y| {
        let xn = x
            .file_stem()
            .expect("expected filename")
            .to_str()
            .expect("filename conversion to UTF-8 failed")
            .parse::<u32>()
            .expect("filename should be numeric");
        let yn = y
            .file_stem()
            .expect("expected filename")
            .to_str()
            .expect("filename conversion to UTF-8 failed")
            .parse::<u32>()
            .expect("filename should be numeric");
        Ord::cmp(&xn, &yn)
    });
    paths
}

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
}

impl Cmd {
    fn process(self) -> Result<(), Box<dyn Error>> {
        match self {
            Cmd::MergeImages { mut dir, out } => {
                dir.push("[0-9]*.jpg");
                let paths = sort_by_name_order(
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
        }

        Ok(())
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    let opt = Cmd::from_args();
    log::debug!("opt: {:?}", opt);

    opt.process()
}

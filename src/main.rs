use std::error::Error;
use std::path::PathBuf;
use structopt::StructOpt;

mod merge;

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
    /// Merges images into vertical in subdirectories of given directory.
    #[structopt(name = "merge")]
    MergeImages,
}

fn main() -> Result<(), Box<dyn Error>> {
    let opt = Cmd::from_args();
    println!("{:#?}", opt);

    let paths = sort_by_name_order(
        glob::glob(r#"C:\Users\ska82\python\lzncrawl\better_spring\1 - 1화\[0-9]*.jpg"#)?
            .collect::<Result<Vec<PathBuf>, _>>()?,
    );

    merge::merge_vertical(paths)?.save("result.png")?;

    Ok(())
}

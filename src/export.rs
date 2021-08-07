use crate::error::Result;
use diesel::prelude::*;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use zip::write::FileOptions;
use zip::CompressionMethod;

/// Exports comics from database to .cbz archive.
pub fn export_database(conn: &SqliteConnection, out: PathBuf) -> Result<usize> {
    use crate::models::ComicRecord;
    use crate::models::TitleRecord;
    use crate::schema::comics::dsl::*;
    use crate::schema::titles::dsl::*;

    let options = FileOptions::default().compression_method(CompressionMethod::Stored);

    let title_recs = titles.load::<TitleRecord>(conn)?;

    let mut return_cnt = 0;

    for title_rec in title_recs {
        let title_id = title_rec.id.clone();
        let title_ = title_rec.title.unwrap_or(title_id);
        let mut out_dir = out.clone();
        out_dir.push(title_rec.provider.to_string().to_lowercase() + "-" + &title_);
        out_dir.set_extension("cbz");
        let f = File::create(out_dir)?;
        let mut zip = zip::ZipWriter::new(f);

        let current_dir = title_rec.provider.to_string().to_ascii_lowercase() + "/" + &title_;
        zip.add_directory(&current_dir, options)?;

        for comic_rec in &comics
            .filter(comic_id.eq(&title_rec.id))
            .order_by(image_seq)
            .load::<ComicRecord>(conn)?
        {
            let current_dir = current_dir.clone() + "/" + &comic_rec.episode_seq.to_string();
            zip.add_directory(&current_dir, options).unwrap_or(());
            zip.start_file(
                current_dir.clone() + "/" + &comic_rec.image_seq.to_string() + ".jpg",
                options,
            )?;
            zip.write_all(&comic_rec.image)?;
        }

        zip.finish()?;
        return_cnt += 1;
    }

    Ok(return_cnt)
}

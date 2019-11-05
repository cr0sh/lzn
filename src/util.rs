use encoding::all::WINDOWS_949;
use encoding::{DecoderTrap, Encoding};
use std::path::PathBuf;

pub fn sort_by_name_order(mut paths: Vec<PathBuf>) -> Vec<PathBuf> {
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

pub fn try_windows_949(s: &[u8]) -> String {
    match WINDOWS_949.decode(s, DecoderTrap::Strict) {
        Ok(x) => x,
        _ => String::from_utf8_lossy(s).to_string(),
    }
}

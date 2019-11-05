use image::{DynamicImage, GenericImage, GenericImageView, ImageResult, RgbaImage};
use std::path::{Path, PathBuf};

pub fn merge_paths_vertical(paths: Vec<impl AsRef<Path>>) -> ImageResult<RgbaImage> {
    let images = paths
        .iter()
        .map(image::open)
        .collect::<ImageResult<Vec<_>>>()?;
    merge_vertical(images)
}

/// Merges given images in paths into vertical.
pub fn merge_vertical(images: Vec<DynamicImage>) -> ImageResult<RgbaImage> {
    let max_width: u32 = images
        .iter()
        .map(GenericImageView::width)
        .fold(0, std::cmp::max);
    let height_sum: u32 = images.iter().map(GenericImageView::height).sum();

    let mut canvas = RgbaImage::new(max_width, height_sum);
    let mut y_inc = 0;
    for img in &images {
        canvas.copy_from(img, 0, y_inc);
        y_inc += img.height();
    }
    Ok(canvas)
}

use image::{GenericImageView, ImageResult, RgbaImage};
use std::path::Path;

/// Merges given images in paths into vertical.
pub fn merge_vertical(paths: Vec<impl AsRef<Path>>) -> ImageResult<RgbaImage> {
    let images = paths
        .iter()
        .map(image::open)
        .collect::<ImageResult<Vec<_>>>()?;

    let max_width: u32 = images
        .iter()
        .map(GenericImageView::width)
        .fold(0, std::cmp::max);
    let height_sum: u32 = images.iter().map(GenericImageView::height).sum();

    let mut canvas = RgbaImage::new(max_width, height_sum);
    let mut y_inc = 0;
    for img in images {
        for (x, y, pixel) in img.pixels() {
            canvas.put_pixel(x, y + y_inc, pixel);
        }
        y_inc += img.height();
    }
    Ok(canvas)
}

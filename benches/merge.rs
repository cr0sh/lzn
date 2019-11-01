use criterion::{black_box, criterion_group, criterion_main, Criterion};
use image::ImageResult;
use lzn;

fn criterion_benchmark(c: &mut Criterion) {
    eprintln!("Loading sample images");
    let paths = glob::glob("samples/11 - 만남 4/[0-9]*.jpg")
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    let images = paths
        .iter()
        .map(image::open)
        .collect::<ImageResult<Vec<_>>>()
        .unwrap();
    eprintln!("Loaded {} images.", images.len());
    c.bench_function("merge", |b| {
        b.iter(|| lzn::merge::merge_vertical(black_box(images.clone())))
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);

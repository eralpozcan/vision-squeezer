use criterion::{Criterion, criterion_group, criterion_main};
use image::{DynamicImage, Rgba, RgbaImage};
use std::hint::black_box;
use vision_squeezer::{
    ProcessConfig, ProcessMode, VisionModel, calculate_optimal_dimensions, estimate_tokens,
    process,
};

fn make_img(w: u32, h: u32) -> DynamicImage {
    let mut img = RgbaImage::from_pixel(w, h, Rgba([240, 240, 240, 255]));
    // Sprinkle some non-bg content so crop doesn't shortcut entire image
    for x in (w / 4)..(3 * w / 4) {
        for y in (h / 4)..(3 * h / 4) {
            img.put_pixel(x, y, Rgba([20, 80, 200, 255]));
        }
    }
    DynamicImage::ImageRgba8(img)
}

fn bench_dimension_math(c: &mut Criterion) {
    c.bench_function("calculate_optimal_dimensions/1025x1025", |b| {
        b.iter(|| calculate_optimal_dimensions(black_box(1025), black_box(1025)))
    });
    c.bench_function("estimate_tokens/claude/4096x4096", |b| {
        b.iter(|| estimate_tokens(black_box(4096), black_box(4096), VisionModel::Claude))
    });
    c.bench_function("estimate_tokens/gpt4o/4096x4096", |b| {
        b.iter(|| estimate_tokens(black_box(4096), black_box(4096), VisionModel::Gpt4o))
    });
}

fn bench_process_pipeline(c: &mut Criterion) {
    let cfg = ProcessConfig::default();
    let small = make_img(1025, 1025);
    let medium = make_img(2048, 2048);

    c.bench_function("process/1025x1025/standard", |b| {
        b.iter(|| process(black_box(small.clone()), ProcessMode::Standard, 0, &cfg))
    });
    c.bench_function("process/2048x2048/standard", |b| {
        b.iter(|| process(black_box(medium.clone()), ProcessMode::Standard, 0, &cfg))
    });
}

criterion_group!(benches, bench_dimension_math, bench_process_pipeline);
criterion_main!(benches);

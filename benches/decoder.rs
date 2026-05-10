use std::fs;
use std::path::Path;

use criterion::{criterion_group, criterion_main, Criterion};
use vigb_decoder::{decode_max, Config};

fn bench_decode_synthetic(c: &mut Criterion) {
    let data = fs::read(Path::new("tests/fixtures/synthetic.max"))
        .expect("synthetic.max — run `cargo run --bin encode-fixture` if missing");
    let cfg = Config::default();
    c.bench_function("decode_max synthetic", |b| {
        b.iter(|| {
            let pages = decode_max(&data, &cfg).unwrap();
            criterion::black_box(pages);
        });
    });
}

criterion_group!(benches, bench_decode_synthetic);
criterion_main!(benches);

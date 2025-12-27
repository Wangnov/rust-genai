use criterion::{black_box, criterion_group, criterion_main, Criterion};
use rust_genai::sse::SseDecoder;

fn generate_test_sse_data(events: usize) -> Vec<u8> {
    let mut payload = String::new();
    for _ in 0..events {
        payload.push_str("data: {\"candidates\":[]}\n\n");
    }
    payload.into_bytes()
}

fn bench_sse_parsing(c: &mut Criterion) {
    let data = generate_test_sse_data(1000);
    c.bench_function("sse_decode_1000_events", |b| {
        b.iter(|| {
            let mut decoder = SseDecoder::new();
            let events = decoder.decode(black_box(&data));
            black_box(events.len());
        })
    });
}

criterion_group!(benches, bench_sse_parsing);
criterion_main!(benches);

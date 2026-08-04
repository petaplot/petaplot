use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use teraplot_core::compute::simd::reduce_min_max_chunk;

fn bench_simd_decimation(c: &mut Criterion) {
    let mut group = c.benchmark_group("simd_decimation");

    for size in [100_000, 1_000_000, 10_000_000] {
        let data: Vec<f32> = (0..size).map(|i| (i as f32 * 0.001).sin()).collect();
        group.throughput(Throughput::Elements(size as u64));

        group.bench_with_input(BenchmarkId::from_parameter(size), &data, |b, d| {
            b.iter(|| reduce_min_max_chunk(d, 10));
        });
    }

    group.finish();
}

criterion_group!(benches, bench_simd_decimation);
criterion_main!(benches);

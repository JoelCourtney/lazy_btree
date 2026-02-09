use beetree::Map;

fn build_lazy(n: usize) -> Map<usize, usize> {
    let mut map = Map::new();
    map.insert(usize::MAX / 2, 0);
    for i in 0..n {
        map.insert(i, i);
    }
    map
}

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use std::hint::black_box;

fn criterion_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("Sorted Bulk");
    for i in [1_000, 1_000_000].iter() {
        group.bench_with_input(BenchmarkId::new("LazyMap", i), i, |b, i| {
            b.iter(|| build_lazy(black_box(*i)))
        });
        group.bench_with_input(BenchmarkId::new("LazyMap-Get", i), i, |b, i| {
            b.iter(|| {
                build_lazy(black_box(*i)).get(&2);
            })
        });
    }

    group.finish();
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);

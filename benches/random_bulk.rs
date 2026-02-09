use beetree::Map;

fn build_lazy(n: usize) -> Map<u64, usize> {
    let mut map = Map::new();
    for i in 0..n {
        map.insert(rand::random(), i);
    }
    map
}

fn build_std(n: usize) -> BTreeMap<u64, usize> {
    let mut map = BTreeMap::new();
    for i in 0..n {
        map.insert(rand::random(), i);
    }
    map
}

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use std::{collections::BTreeMap, hint::black_box};

fn criterion_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("Random Bulk");
    for i in [100, 10_000].iter() {
        group.bench_with_input(BenchmarkId::new("LazyMap", i), i, |b, i| {
            b.iter(|| build_lazy(black_box(*i)))
        });
        group.bench_with_input(BenchmarkId::new("BTreeMap", i), i, |b, i| {
            b.iter(|| build_std(black_box(*i)))
        });
        group.bench_with_input(BenchmarkId::new("LazyMap-Get", i), i, |b, i| {
            b.iter(|| {
                build_lazy(black_box(*i)).get(&2);
            })
        });
        group.bench_with_input(BenchmarkId::new("BTreeMap-Get", i), i, |b, i| {
            b.iter(|| {
                build_std(black_box(*i)).get(&2);
            })
        });
    }

    group.finish();
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);

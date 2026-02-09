use beetree::Map;

fn run_lazy(n: usize) -> Map<u64, usize> {
    let mut map = Map::new();
    let mut previous_idx: Option<u64> = None;
    for i in 0..n {
        if let Some(idx) = previous_idx {
            map.get(&idx).unwrap();
        }
        let idx = rand::random();
        previous_idx = Some(idx);
        map.insert(idx, i);
    }
    map
}

fn run_std(n: usize) -> BTreeMap<u64, usize> {
    let mut map = BTreeMap::new();
    let mut previous_idx: Option<u64> = None;
    for i in 0..n {
        if let Some(idx) = previous_idx {
            map.get(&idx).unwrap();
        }
        let idx = rand::random();
        previous_idx = Some(idx);
        map.insert(idx, i);
    }
    map
}

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use std::{collections::BTreeMap, hint::black_box};

fn criterion_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("Alternating");
    for i in [100, 10_000].iter() {
        group.bench_with_input(BenchmarkId::new("LazyMap", i), i, |b, i| {
            b.iter(|| {
                run_lazy(black_box(*i)).get(&2);
            })
        });
        group.bench_with_input(BenchmarkId::new("BTreeMap", i), i, |b, i| {
            b.iter(|| {
                run_std(black_box(*i)).get(&2);
            })
        });
    }

    group.finish();
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);

use beetree::Map;

const INSERT_EXTRA: usize = 2;

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
        for _ in 0..INSERT_EXTRA {
            map.insert(rand::random(), i);
        }
    }
    map
}

fn main() {
    run_lazy(1_000_000);
}

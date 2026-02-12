//! Set operations benchmarks

use aoee_core::iterator::VecIterator;
use aoee_core::set_ops::{intersect, intersect_galloping, union, difference};
use aoee_core::{EntityId, EntityType};
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;

fn make_sequential_ids(start: u64, count: usize) -> Vec<EntityId> {
    (start..start + count as u64)
        .map(|id| EntityId::new(EntityType::User, id))
        .collect()
}

fn make_random_ids(count: usize, range: u64, seed: u64) -> Vec<EntityId> {
    let mut rng = StdRng::seed_from_u64(seed);
    let mut ids: Vec<EntityId> = (0..count)
        .map(|_| EntityId::new(EntityType::User, rng.gen_range(1..=range)))
        .collect();
    ids.sort();
    ids.dedup();
    ids
}

fn bench_intersect_equal_size(c: &mut Criterion) {
    let mut group = c.benchmark_group("intersect_equal_size");
    
    for size in [100, 1000, 10000, 100000] {
        group.throughput(Throughput::Elements(size as u64 * 2));
        
        // 50% overlap
        let a = make_sequential_ids(1, size);
        let b = make_sequential_ids(size as u64 / 2, size);
        
        group.bench_with_input(BenchmarkId::from_parameter(size), &(a, b), |bench, (a, b)| {
            bench.iter(|| {
                intersect(
                    VecIterator::new(black_box(a.clone())),
                    VecIterator::new(black_box(b.clone())),
                )
            })
        });
    }
    
    group.finish();
}

fn bench_intersect_size_disparity(c: &mut Criterion) {
    let mut group = c.benchmark_group("intersect_size_disparity");
    
    // Small list intersected with large list
    for (small_size, large_size) in [(100, 10000), (100, 100000), (1000, 100000)] {
        group.throughput(Throughput::Elements((small_size + large_size) as u64));
        
        let small = make_sequential_ids(1, small_size);
        let large = make_sequential_ids(1, large_size);
        
        let id = format!("{}x{}", small_size, large_size);
        
        group.bench_with_input(
            BenchmarkId::new("merge", &id),
            &(small.clone(), large.clone()),
            |bench, (s, l)| {
                bench.iter(|| {
                    intersect(
                        VecIterator::new(black_box(s.clone())),
                        VecIterator::new(black_box(l.clone())),
                    )
                })
            },
        );
        
        group.bench_with_input(
            BenchmarkId::new("galloping", &id),
            &(small.clone(), large.clone()),
            |bench, (s, l)| {
                bench.iter(|| {
                    intersect_galloping(
                        VecIterator::new(black_box(s.clone())),
                        VecIterator::new(black_box(l.clone())),
                    )
                })
            },
        );
    }
    
    group.finish();
}

fn bench_union(c: &mut Criterion) {
    let mut group = c.benchmark_group("union");
    
    for size in [100, 1000, 10000] {
        group.throughput(Throughput::Elements(size as u64 * 2));
        
        let a = make_sequential_ids(1, size);
        let b = make_sequential_ids(size as u64 / 2, size);
        
        group.bench_with_input(BenchmarkId::from_parameter(size), &(a, b), |bench, (a, b)| {
            bench.iter(|| {
                union(
                    VecIterator::new(black_box(a.clone())),
                    VecIterator::new(black_box(b.clone())),
                )
            })
        });
    }
    
    group.finish();
}

fn bench_difference(c: &mut Criterion) {
    let mut group = c.benchmark_group("difference");
    
    for size in [100, 1000, 10000] {
        group.throughput(Throughput::Elements(size as u64 * 2));
        
        let a = make_sequential_ids(1, size);
        let b = make_sequential_ids(size as u64 / 2, size);
        
        group.bench_with_input(BenchmarkId::from_parameter(size), &(a, b), |bench, (a, b)| {
            bench.iter(|| {
                difference(
                    VecIterator::new(black_box(a.clone())),
                    VecIterator::new(black_box(b.clone())),
                )
            })
        });
    }
    
    group.finish();
}

fn bench_random_overlap(c: &mut Criterion) {
    let mut group = c.benchmark_group("intersect_random");
    
    // Random IDs with varying overlap
    for overlap_pct in [10, 25, 50, 75, 90] {
        let size = 10000;
        let range = (size * 100 / overlap_pct) as u64;
        
        let a = make_random_ids(size, range, 42);
        let b = make_random_ids(size, range, 43);
        
        group.bench_with_input(
            BenchmarkId::new("overlap_pct", overlap_pct),
            &(a, b),
            |bench, (a, b)| {
                bench.iter(|| {
                    intersect(
                        VecIterator::new(black_box(a.clone())),
                        VecIterator::new(black_box(b.clone())),
                    )
                })
            },
        );
    }
    
    group.finish();
}

criterion_group!(
    benches,
    bench_intersect_equal_size,
    bench_intersect_size_disparity,
    bench_union,
    bench_difference,
    bench_random_overlap,
);

criterion_main!(benches);

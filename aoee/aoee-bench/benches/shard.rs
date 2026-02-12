//! Shard benchmarks

use aoee_core::{EdgeKey, EdgeType, EntityId, EntityType};
use aoee_shard::{Shard, ShardConfig, ShardManager, manager::ManagerConfig};
use aoee_storage::InMemoryStore;
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use std::sync::Arc;
use tokio::runtime::Runtime;

fn make_id(raw: u64) -> EntityId {
    EntityId::new(EntityType::User, raw)
}

fn make_key(src: u64) -> EdgeKey {
    EdgeKey::new(make_id(src), EdgeType::Follows)
}

fn bench_add_edge(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("shard_add_edge");
    
    group.throughput(Throughput::Elements(1));
    
    group.bench_function("single", |b| {
        let store = Arc::new(InMemoryStore::new());
        let shard = Shard::new(ShardConfig::default(), store);
        let mut counter = 0u64;
        
        b.iter(|| {
            counter += 1;
            rt.block_on(async {
                shard.add_edge(make_key(1), make_id(counter)).await.unwrap()
            })
        })
    });
    
    group.finish();
}

fn bench_neighbors(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("shard_neighbors");
    
    for size in [10, 100, 1000, 10000] {
        group.throughput(Throughput::Elements(size as u64));
        
        let store = Arc::new(InMemoryStore::new());
        let shard = Shard::new(ShardConfig::default(), store);
        let key = make_key(1);
        
        // Setup: add edges
        rt.block_on(async {
            for i in 1..=size {
                shard.add_edge(key, make_id(i)).await.unwrap();
            }
        });
        
        group.bench_with_input(BenchmarkId::from_parameter(size), &shard, |b, shard| {
            b.iter(|| {
                rt.block_on(async {
                    shard.neighbors(black_box(key)).await.unwrap()
                })
            })
        });
    }
    
    group.finish();
}

fn bench_contains(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("shard_contains");
    
    for size in [100, 1000, 10000] {
        let store = Arc::new(InMemoryStore::new());
        let shard = Shard::new(ShardConfig::default(), store);
        let key = make_key(1);
        
        // Setup: add edges
        rt.block_on(async {
            for i in 1..=size {
                shard.add_edge(key, make_id(i)).await.unwrap();
            }
        });
        
        let target = make_id(size as u64 / 2);
        
        group.bench_with_input(BenchmarkId::from_parameter(size), &(shard, target), |b, (shard, target)| {
            b.iter(|| {
                rt.block_on(async {
                    shard.contains(black_box(key), black_box(*target)).await.unwrap()
                })
            })
        });
    }
    
    group.finish();
}

fn bench_intersect(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("shard_intersect");
    
    for size in [100, 1000, 10000] {
        group.throughput(Throughput::Elements(size as u64 * 2));
        
        let store = Arc::new(InMemoryStore::new());
        let shard = Shard::new(ShardConfig::default(), store);
        let key1 = make_key(1);
        let key2 = make_key(2);
        
        // Setup: add edges with 50% overlap
        rt.block_on(async {
            for i in 1..=size {
                shard.add_edge(key1, make_id(i)).await.unwrap();
            }
            for i in (size/2)..=(size + size/2) {
                shard.add_edge(key2, make_id(i as u64)).await.unwrap();
            }
        });
        
        group.bench_with_input(BenchmarkId::from_parameter(size), &shard, |b, shard| {
            b.iter(|| {
                rt.block_on(async {
                    shard.intersect(black_box(key1), black_box(key2)).await.unwrap()
                })
            })
        });
    }
    
    group.finish();
}

fn bench_manager_routing(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("manager_routing");
    
    let config = ManagerConfig {
        num_shards: 4,
        ..Default::default()
    };
    
    let manager = rt.block_on(async {
        let m = ShardManager::new(config, |_| Arc::new(InMemoryStore::new()));
        m.initialize().await;
        m
    });
    
    group.bench_function("add_edge", |b| {
        let mut counter = 0u64;
        b.iter(|| {
            counter += 1;
            rt.block_on(async {
                manager.add_edge(make_key(counter), make_id(1)).await.unwrap()
            })
        })
    });
    
    group.finish();
}

criterion_group!(
    benches,
    bench_add_edge,
    bench_neighbors,
    bench_contains,
    bench_intersect,
    bench_manager_routing,
);

criterion_main!(benches);

//! Encoding benchmarks

use aoee_core::encoding::{
    AutoEncoder, BlockPackedEncoder, DeltaVarintEncoder, PostingEncoder, RoaringEncoder,
    SmallVecEncoder,
};
use aoee_core::{EntityId, EntityType};
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};

fn make_sequential_ids(start: u64, count: usize) -> Vec<EntityId> {
    (start..start + count as u64)
        .map(|id| EntityId::new(EntityType::User, id))
        .collect()
}

fn make_sparse_ids(start: u64, count: usize, step: u64) -> Vec<EntityId> {
    (0..count as u64)
        .map(|i| EntityId::new(EntityType::User, start + i * step))
        .collect()
}

fn bench_delta_varint_encode(c: &mut Criterion) {
    let mut group = c.benchmark_group("delta_varint_encode");
    
    for size in [100, 1000, 10000] {
        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &size| {
            let ids = make_sequential_ids(1, size);
            b.iter(|| DeltaVarintEncoder::encode(black_box(&ids)))
        });
    }
    
    group.finish();
}

fn bench_delta_varint_decode(c: &mut Criterion) {
    let mut group = c.benchmark_group("delta_varint_decode");
    
    for size in [100, 1000, 10000] {
        group.throughput(Throughput::Elements(size as u64));
        let ids = make_sequential_ids(1, size);
        let encoded = DeltaVarintEncoder::encode(&ids).unwrap();
        
        group.bench_with_input(BenchmarkId::from_parameter(size), &encoded, |b, encoded| {
            if let aoee_core::encoding::EncodedList::DeltaVarint(data) = encoded {
                b.iter(|| DeltaVarintEncoder::decode(black_box(data)))
            }
        });
    }
    
    group.finish();
}

fn bench_blockpacked_encode(c: &mut Criterion) {
    let mut group = c.benchmark_group("blockpacked_encode");
    
    for size in [1000, 10000, 100000] {
        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &size| {
            let ids = make_sequential_ids(1, size);
            b.iter(|| BlockPackedEncoder::encode(black_box(&ids)))
        });
    }
    
    group.finish();
}

fn bench_blockpacked_decode(c: &mut Criterion) {
    let mut group = c.benchmark_group("blockpacked_decode");
    
    for size in [1000, 10000, 100000] {
        group.throughput(Throughput::Elements(size as u64));
        let ids = make_sequential_ids(1, size);
        let encoded = BlockPackedEncoder::encode(&ids).unwrap();
        
        group.bench_with_input(BenchmarkId::from_parameter(size), &encoded, |b, encoded| {
            if let aoee_core::encoding::EncodedList::BlockPacked(data) = encoded {
                b.iter(|| BlockPackedEncoder::decode(black_box(data)))
            }
        });
    }
    
    group.finish();
}

fn bench_roaring_encode(c: &mut Criterion) {
    let mut group = c.benchmark_group("roaring_encode");
    
    for size in [10000, 100000, 1000000] {
        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &size| {
            let ids = make_sequential_ids(1, size);
            b.iter(|| RoaringEncoder::encode(black_box(&ids)))
        });
    }
    
    group.finish();
}

fn bench_auto_encoder(c: &mut Criterion) {
    let mut group = c.benchmark_group("auto_encoder");
    
    // Small (should use SmallVec)
    group.bench_function("small_50", |b| {
        let ids = make_sequential_ids(1, 50);
        b.iter(|| AutoEncoder::encode(black_box(&ids)))
    });
    
    // Medium (should use DeltaVarint)
    group.bench_function("medium_500", |b| {
        let ids = make_sequential_ids(1, 500);
        b.iter(|| AutoEncoder::encode(black_box(&ids)))
    });
    
    // Large (should use BlockPacked or Roaring)
    group.bench_function("large_10000", |b| {
        let ids = make_sequential_ids(1, 10000);
        b.iter(|| AutoEncoder::encode(black_box(&ids)))
    });
    
    group.finish();
}

fn bench_compression_ratio(c: &mut Criterion) {
    let mut group = c.benchmark_group("compression_ratio");
    
    // Sequential IDs (best case for delta encoding)
    let sequential = make_sequential_ids(1, 10000);
    
    // Sparse IDs
    let sparse = make_sparse_ids(1, 10000, 100);
    
    group.bench_function("sequential_delta", |b| {
        b.iter(|| DeltaVarintEncoder::encode(black_box(&sequential)))
    });
    
    group.bench_function("sparse_delta", |b| {
        b.iter(|| DeltaVarintEncoder::encode(black_box(&sparse)))
    });
    
    group.finish();
}

criterion_group!(
    benches,
    bench_delta_varint_encode,
    bench_delta_varint_decode,
    bench_blockpacked_encode,
    bench_blockpacked_decode,
    bench_roaring_encode,
    bench_auto_encoder,
    bench_compression_ratio,
);

criterion_main!(benches);

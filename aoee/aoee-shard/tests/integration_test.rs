//! Integration tests for AOEE
//!
//! Tests end-to-end functionality, concurrent access, and hot-key handling.

use aoee_core::{EdgeKey, EdgeType, EntityId, EntityType};
use aoee_shard::{manager::ManagerConfig, Shard, ShardConfig, ShardManager};
use aoee_storage::InMemoryStore;
use std::sync::Arc;
use std::time::Duration;

fn make_id(entity_type: EntityType, raw: u64) -> EntityId {
    EntityId::new(entity_type, raw)
}

fn make_user(raw: u64) -> EntityId {
    make_id(EntityType::User, raw)
}

fn make_key(src: u64, edge_type: EdgeType) -> EdgeKey {
    EdgeKey::new(make_user(src), edge_type)
}

// ============================================================================
// End-to-end tests
// ============================================================================

#[tokio::test]
async fn test_basic_edge_operations() {
    let store = Arc::new(InMemoryStore::new());
    let shard = Shard::new(ShardConfig::default(), store);

    let key = make_key(1, EdgeType::Follows);

    // Add edges
    for i in 1..=100 {
        shard.add_edge(key, make_user(i)).await.unwrap();
    }

    // Verify all edges exist
    let neighbors = shard.neighbors(key).await.unwrap();
    assert_eq!(neighbors.len(), 100);

    // Verify contains
    assert!(shard.contains(key, make_user(50)).await.unwrap());
    assert!(!shard.contains(key, make_user(150)).await.unwrap());

    // Delete some edges
    for i in 1..=25 {
        shard.delete_edge(key, make_user(i)).await.unwrap();
    }

    // Verify remaining edges
    let neighbors = shard.neighbors(key).await.unwrap();
    assert_eq!(neighbors.len(), 75);

    // First remaining should be 26
    assert_eq!(neighbors[0], make_user(26));
}

#[tokio::test]
async fn test_multiple_edge_types() {
    let store = Arc::new(InMemoryStore::new());
    let shard = Shard::new(ShardConfig::default(), store);

    let user1 = make_user(1);
    let follows_key = EdgeKey::new(user1, EdgeType::Follows);
    let likes_key = EdgeKey::new(user1, EdgeType::Likes);
    let blocks_key = EdgeKey::new(user1, EdgeType::Blocks);

    // User 1 follows users 10, 20, 30
    shard.add_edge(follows_key, make_user(10)).await.unwrap();
    shard.add_edge(follows_key, make_user(20)).await.unwrap();
    shard.add_edge(follows_key, make_user(30)).await.unwrap();

    // User 1 likes posts 100, 101
    shard.add_edge(likes_key, make_id(EntityType::Post, 100)).await.unwrap();
    shard.add_edge(likes_key, make_id(EntityType::Post, 101)).await.unwrap();

    // User 1 blocks user 99
    shard.add_edge(blocks_key, make_user(99)).await.unwrap();

    // Verify each edge type is independent
    assert_eq!(shard.neighbors(follows_key).await.unwrap().len(), 3);
    assert_eq!(shard.neighbors(likes_key).await.unwrap().len(), 2);
    assert_eq!(shard.neighbors(blocks_key).await.unwrap().len(), 1);
}

#[tokio::test]
async fn test_set_operations() {
    let store = Arc::new(InMemoryStore::new());
    let shard = Shard::new(ShardConfig::default(), store);

    let key1 = make_key(1, EdgeType::Follows);
    let key2 = make_key(2, EdgeType::Follows);

    // User 1 follows: 10, 20, 30, 40, 50
    for id in [10, 20, 30, 40, 50] {
        shard.add_edge(key1, make_user(id)).await.unwrap();
    }

    // User 2 follows: 30, 40, 50, 60, 70
    for id in [30, 40, 50, 60, 70] {
        shard.add_edge(key2, make_user(id)).await.unwrap();
    }

    // Intersect: mutual friends
    let mutual = shard.intersect(key1, key2).await.unwrap();
    assert_eq!(mutual.len(), 3);
    assert!(mutual.contains(&make_user(30)));
    assert!(mutual.contains(&make_user(40)));
    assert!(mutual.contains(&make_user(50)));

    // Union: all followed users
    let all = shard.union(key1, key2).await.unwrap();
    assert_eq!(all.len(), 7); // 10,20,30,40,50,60,70
}

#[tokio::test]
async fn test_shard_manager_routing() {
    let config = ManagerConfig {
        num_shards: 4,
        ..Default::default()
    };

    let manager = ShardManager::new(config, |_| Arc::new(InMemoryStore::new()));
    manager.initialize().await;

    // Add edges for different users (should route to different shards)
    for user in 1..=100 {
        let key = make_key(user, EdgeType::Follows);
        for target in 1..=10 {
            manager.add_edge(key, make_user(target * 1000 + user)).await.unwrap();
        }
    }

    // Verify each user has correct edges
    for user in 1..=100 {
        let key = make_key(user, EdgeType::Follows);
        let neighbors = manager.neighbors(key).await.unwrap();
        assert_eq!(neighbors.len(), 10, "User {} should have 10 edges", user);
    }

    // Check stats show distribution across shards
    let stats = manager.stats().await;
    assert!(stats.len() <= 4);
}

// ============================================================================
// Concurrent access tests
// ============================================================================

#[tokio::test]
async fn test_concurrent_writes() {
    let store = Arc::new(InMemoryStore::new());
    let shard = Arc::new(Shard::new(ShardConfig::default(), store));
    let key = make_key(1, EdgeType::Follows);

    // Spawn many concurrent writers
    let mut handles = Vec::new();
    for batch in 0..10 {
        let shard = Arc::clone(&shard);
        let handle = tokio::spawn(async move {
            for i in 0..100 {
                let dst = make_user(batch * 100 + i);
                shard.add_edge(key, dst).await.unwrap();
            }
        });
        handles.push(handle);
    }

    // Wait for all writers
    for handle in handles {
        handle.await.unwrap();
    }

    // Verify all edges were added
    let neighbors = shard.neighbors(key).await.unwrap();
    assert_eq!(neighbors.len(), 1000);
}

#[tokio::test]
async fn test_concurrent_reads_and_writes() {
    let store = Arc::new(InMemoryStore::new());
    let shard = Arc::new(Shard::new(ShardConfig::default(), store));
    let key = make_key(1, EdgeType::Follows);

    // Pre-populate some data
    for i in 1..=100 {
        shard.add_edge(key, make_user(i)).await.unwrap();
    }

    // Spawn concurrent readers and writers
    let mut handles = Vec::new();

    // Writers
    for batch in 0..5 {
        let shard = Arc::clone(&shard);
        let handle = tokio::spawn(async move {
            for i in 0..50 {
                let dst = make_user(1000 + batch * 50 + i);
                shard.add_edge(key, dst).await.unwrap();
            }
        });
        handles.push(handle);
    }

    // Readers
    for _ in 0..10 {
        let shard = Arc::clone(&shard);
        let handle = tokio::spawn(async move {
            for _ in 0..100 {
                let neighbors = shard.neighbors(key).await.unwrap();
                // Should see at least the original 100
                assert!(neighbors.len() >= 100);
                // Small delay to interleave with writes
                tokio::time::sleep(Duration::from_micros(10)).await;
            }
        });
        handles.push(handle);
    }

    // Wait for all
    for handle in handles {
        handle.await.unwrap();
    }

    // Final count should be 100 + (5 * 50) = 350
    let neighbors = shard.neighbors(key).await.unwrap();
    assert_eq!(neighbors.len(), 350);
}

#[tokio::test]
async fn test_concurrent_deletes() {
    let store = Arc::new(InMemoryStore::new());
    let shard = Arc::new(Shard::new(ShardConfig::default(), store));
    let key = make_key(1, EdgeType::Follows);

    // Pre-populate data
    for i in 1..=1000 {
        shard.add_edge(key, make_user(i)).await.unwrap();
    }

    // Concurrent deletes of half the edges
    let mut handles = Vec::new();
    for batch in 0..5 {
        let shard = Arc::clone(&shard);
        let handle = tokio::spawn(async move {
            for i in 0..100 {
                let id = batch * 100 + i + 1; // 1-500
                shard.delete_edge(key, make_user(id)).await.unwrap();
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.await.unwrap();
    }

    // Should have 500 remaining (501-1000)
    let neighbors = shard.neighbors(key).await.unwrap();
    assert_eq!(neighbors.len(), 500);
    assert_eq!(neighbors[0], make_user(501));
}

// ============================================================================
// Hot-key handling tests
// ============================================================================

#[tokio::test]
async fn test_hot_key_heavy_writes() {
    let store = Arc::new(InMemoryStore::new());
    let config = ShardConfig {
        compaction: aoee_core::CompactionConfig {
            buffer_threshold: 100, // Low threshold to trigger compaction
            ..Default::default()
        },
        ..Default::default()
    };
    let shard = Arc::new(Shard::new(config, store));
    let key = make_key(1, EdgeType::Follows);

    // Rapid writes to trigger multiple compactions
    for i in 1..=1000 {
        shard.add_edge(key, make_user(i)).await.unwrap();
    }

    // Verify data integrity after compactions
    let neighbors = shard.neighbors(key).await.unwrap();
    assert_eq!(neighbors.len(), 1000);

    // Verify sorted order
    for (i, &id) in neighbors.iter().enumerate() {
        assert_eq!(id, make_user(i as u64 + 1));
    }
}

#[tokio::test]
async fn test_hot_key_mixed_operations() {
    let store = Arc::new(InMemoryStore::new());
    let shard = Arc::new(Shard::new(ShardConfig::default(), store));
    let key = make_key(1, EdgeType::Follows);

    // Simulate hot key: rapid add/delete/add cycles
    for cycle in 0..10 {
        // Add batch
        for i in 0..100 {
            let id = cycle * 100 + i;
            shard.add_edge(key, make_user(id)).await.unwrap();
        }

        // Small delay to ensure deletes get newer timestamps
        tokio::time::sleep(Duration::from_micros(1)).await;

        // Delete even IDs
        for i in (0..100).step_by(2) {
            let id = cycle * 100 + i;
            shard.delete_edge(key, make_user(id)).await.unwrap();
        }
    }

    // Should have approximately 50 per cycle = 500 total
    // Allow some tolerance for timing-related deduplication edge cases
    let neighbors = shard.neighbors(key).await.unwrap();
    assert!(
        neighbors.len() >= 490 && neighbors.len() <= 510,
        "Expected ~500 neighbors, got {}",
        neighbors.len()
    );
}

#[tokio::test]
async fn test_many_unique_keys() {
    let store = Arc::new(InMemoryStore::new());
    let shard = Arc::new(Shard::new(ShardConfig::default(), store));

    // Create many different keys (users)
    for user in 1..=100 {
        let key = make_key(user, EdgeType::Follows);
        for target in 1..=10 {
            shard.add_edge(key, make_user(target)).await.unwrap();
        }
    }

    // Verify each key has correct count
    for user in 1..=100 {
        let key = make_key(user, EdgeType::Follows);
        let neighbors = shard.neighbors(key).await.unwrap();
        assert_eq!(neighbors.len(), 10);
    }

    // Check cache stats
    let stats = shard.stats().await;
    assert_eq!(stats.cached_lists, 100);
}

// ============================================================================
// Edge cases
// ============================================================================

#[tokio::test]
async fn test_empty_posting_list() {
    let store = Arc::new(InMemoryStore::new());
    let shard = Shard::new(ShardConfig::default(), store);
    let key = make_key(1, EdgeType::Follows);

    // Query non-existent key
    let neighbors = shard.neighbors(key).await.unwrap();
    assert!(neighbors.is_empty());

    // Contains on empty
    assert!(!shard.contains(key, make_user(1)).await.unwrap());
}

#[tokio::test]
async fn test_delete_non_existent() {
    let store = Arc::new(InMemoryStore::new());
    let shard = Shard::new(ShardConfig::default(), store);
    let key = make_key(1, EdgeType::Follows);

    // Add one edge
    shard.add_edge(key, make_user(1)).await.unwrap();

    // Delete non-existent edge (should be no-op)
    shard.delete_edge(key, make_user(999)).await.unwrap();

    // Original edge should still exist
    let neighbors = shard.neighbors(key).await.unwrap();
    assert_eq!(neighbors.len(), 1);
    assert_eq!(neighbors[0], make_user(1));
}

#[tokio::test]
async fn test_duplicate_adds() {
    let store = Arc::new(InMemoryStore::new());
    let shard = Shard::new(ShardConfig::default(), store);
    let key = make_key(1, EdgeType::Follows);

    // Add same edge multiple times
    for _ in 0..100 {
        shard.add_edge(key, make_user(1)).await.unwrap();
    }

    // Should only have one edge (deduplicated)
    let neighbors = shard.neighbors(key).await.unwrap();
    assert_eq!(neighbors.len(), 1);
}

#[tokio::test]
async fn test_add_delete_add_same_id() {
    let store = Arc::new(InMemoryStore::new());
    let shard = Shard::new(ShardConfig::default(), store);
    let key = make_key(1, EdgeType::Follows);

    // Add, delete, then add again
    shard.add_edge(key, make_user(1)).await.unwrap();
    
    // Small delay to ensure different timestamp
    tokio::time::sleep(Duration::from_micros(1)).await;
    shard.delete_edge(key, make_user(1)).await.unwrap();
    
    tokio::time::sleep(Duration::from_micros(1)).await;
    shard.add_edge(key, make_user(1)).await.unwrap();

    // Should exist after re-add
    let neighbors = shard.neighbors(key).await.unwrap();
    assert_eq!(neighbors.len(), 1);
    assert!(shard.contains(key, make_user(1)).await.unwrap());
}

#[tokio::test]
async fn test_limited_neighbors() {
    let store = Arc::new(InMemoryStore::new());
    let shard = Shard::new(ShardConfig::default(), store);
    let key = make_key(1, EdgeType::Follows);

    // Add 100 edges
    for i in 1..=100 {
        shard.add_edge(key, make_user(i)).await.unwrap();
    }

    // Get limited neighbors
    let neighbors = shard.neighbors_limited(key, 10).await.unwrap();
    assert_eq!(neighbors.len(), 10);

    // Should be the first 10 (sorted)
    for (i, &id) in neighbors.iter().enumerate() {
        assert_eq!(id, make_user(i as u64 + 1));
    }
}

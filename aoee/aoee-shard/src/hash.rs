//! Consistent hashing for shard routing

use aoee_core::EdgeKey;
use std::collections::BTreeMap;
use std::hash::{Hash, Hasher};

/// Number of virtual nodes per physical shard
const DEFAULT_VIRTUAL_NODES: u32 = 150;

/// Simple hash function for consistent hashing
fn hash_key(key: &EdgeKey) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    key.hash(&mut hasher);
    hasher.finish()
}

/// Hash with replica number for virtual nodes
fn hash_shard(shard_id: u32, replica: u32) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    shard_id.hash(&mut hasher);
    replica.hash(&mut hasher);
    hasher.finish()
}

/// Consistent hash ring for shard routing
#[derive(Clone)]
pub struct ConsistentHash {
    /// Ring: hash position -> shard_id
    ring: BTreeMap<u64, u32>,
    /// Number of virtual nodes per shard
    virtual_nodes: u32,
    /// List of shard IDs
    shards: Vec<u32>,
}

impl ConsistentHash {
    /// Create a new consistent hash ring
    pub fn new(shard_ids: Vec<u32>) -> Self {
        Self::with_virtual_nodes(shard_ids, DEFAULT_VIRTUAL_NODES)
    }

    /// Create with custom number of virtual nodes
    pub fn with_virtual_nodes(shard_ids: Vec<u32>, virtual_nodes: u32) -> Self {
        let mut ring = BTreeMap::new();
        
        for &shard_id in &shard_ids {
            for replica in 0..virtual_nodes {
                let hash = hash_shard(shard_id, replica);
                ring.insert(hash, shard_id);
            }
        }
        
        ConsistentHash {
            ring,
            virtual_nodes,
            shards: shard_ids,
        }
    }

    /// Get the shard responsible for a key
    pub fn get_shard(&self, key: &EdgeKey) -> Option<u32> {
        if self.ring.is_empty() {
            return None;
        }
        
        let hash = hash_key(key);
        
        // Find the first shard with hash >= key hash
        if let Some((&_, &shard_id)) = self.ring.range(hash..).next() {
            return Some(shard_id);
        }
        
        // Wrap around to the first shard
        self.ring.values().next().copied()
    }

    /// Add a shard to the ring
    pub fn add_shard(&mut self, shard_id: u32) {
        if self.shards.contains(&shard_id) {
            return;
        }
        
        self.shards.push(shard_id);
        
        for replica in 0..self.virtual_nodes {
            let hash = hash_shard(shard_id, replica);
            self.ring.insert(hash, shard_id);
        }
    }

    /// Remove a shard from the ring
    pub fn remove_shard(&mut self, shard_id: u32) {
        self.shards.retain(|&id| id != shard_id);
        
        for replica in 0..self.virtual_nodes {
            let hash = hash_shard(shard_id, replica);
            self.ring.remove(&hash);
        }
    }

    /// Get all shard IDs
    pub fn shard_ids(&self) -> &[u32] {
        &self.shards
    }

    /// Get the number of shards
    pub fn shard_count(&self) -> usize {
        self.shards.len()
    }

    /// Check if the ring contains a shard
    pub fn contains_shard(&self, shard_id: u32) -> bool {
        self.shards.contains(&shard_id)
    }
}

impl Default for ConsistentHash {
    fn default() -> Self {
        ConsistentHash::new(vec![0])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aoee_core::{EntityId, EntityType, EdgeType};

    fn make_key(src: u64, edge_type: EdgeType) -> EdgeKey {
        EdgeKey::new(EntityId::new(EntityType::User, src), edge_type)
    }

    #[test]
    fn test_single_shard() {
        let ring = ConsistentHash::new(vec![0]);
        
        let key = make_key(1, EdgeType::Follows);
        assert_eq!(ring.get_shard(&key), Some(0));
    }

    #[test]
    fn test_multiple_shards() {
        let ring = ConsistentHash::new(vec![0, 1, 2, 3]);
        
        // All keys should map to some shard
        for i in 0..100 {
            let key = make_key(i, EdgeType::Follows);
            let shard = ring.get_shard(&key);
            assert!(shard.is_some());
            assert!(shard.unwrap() < 4);
        }
    }

    #[test]
    fn test_distribution() {
        let ring = ConsistentHash::new(vec![0, 1, 2, 3]);
        let mut counts = [0u32; 4];
        
        // Check that distribution is roughly even
        for i in 0..10000 {
            let key = make_key(i, EdgeType::Follows);
            let shard = ring.get_shard(&key).unwrap();
            counts[shard as usize] += 1;
        }
        
        // Each shard should have at least 15% of keys (with virtual nodes, distribution is good)
        for count in &counts {
            assert!(*count > 1500, "Shard has {} keys, expected > 1500", count);
        }
    }

    #[test]
    fn test_consistency() {
        let ring = ConsistentHash::new(vec![0, 1, 2, 3]);
        
        let key = make_key(12345, EdgeType::Follows);
        let shard1 = ring.get_shard(&key);
        let shard2 = ring.get_shard(&key);
        
        // Same key should always map to same shard
        assert_eq!(shard1, shard2);
    }

    #[test]
    fn test_add_shard() {
        let mut ring = ConsistentHash::new(vec![0, 1]);
        
        // Record some mappings
        let key1 = make_key(1, EdgeType::Follows);
        let key2 = make_key(2, EdgeType::Follows);
        
        ring.add_shard(2);
        
        assert_eq!(ring.shard_count(), 3);
        assert!(ring.contains_shard(2));
    }

    #[test]
    fn test_remove_shard() {
        let mut ring = ConsistentHash::new(vec![0, 1, 2]);
        
        ring.remove_shard(1);
        
        assert_eq!(ring.shard_count(), 2);
        assert!(!ring.contains_shard(1));
        
        // Keys should still map to remaining shards
        let key = make_key(1, EdgeType::Follows);
        let shard = ring.get_shard(&key).unwrap();
        assert!(shard == 0 || shard == 2);
    }

    #[test]
    fn test_empty_ring() {
        let ring = ConsistentHash::new(vec![]);
        
        let key = make_key(1, EdgeType::Follows);
        assert_eq!(ring.get_shard(&key), None);
    }
}

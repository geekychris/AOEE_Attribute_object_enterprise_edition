//! Friend-of-Friend (2-hop) queries.
//!
//! Computes candidates by traversing 2-hop relationships in the graph.
//! Includes safeguards for hot keys: fanout cap, sampling, and time budget.

use crate::id::EntityId;
use crate::iterator::{PostingIterator, VecIterator};
use crate::set_ops;
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Configuration for Friend-of-Friend queries
#[derive(Debug, Clone)]
pub struct FofConfig {
    /// Maximum number of neighbors to traverse per hop (fanout cap)
    pub fanout_cap: usize,
    /// Sample size for random sampling (0 = no sampling, use fanout_cap only)
    pub sample_size: usize,
    /// Time budget in milliseconds (0 = no limit)
    pub time_budget_ms: u64,
    /// Maximum candidates to return
    pub max_results: usize,
    /// Minimum score (mutual friend count) to include
    pub min_score: usize,
    /// Whether to exclude direct friends from results
    pub exclude_direct: bool,
    /// Whether to exclude self from results
    pub exclude_self: bool,
}

impl Default for FofConfig {
    fn default() -> Self {
        FofConfig {
            fanout_cap: 1000,
            sample_size: 0,
            time_budget_ms: 0,
            max_results: 100,
            min_score: 1,
            exclude_direct: true,
            exclude_self: true,
        }
    }
}

impl FofConfig {
    pub fn with_fanout_cap(mut self, cap: usize) -> Self {
        self.fanout_cap = cap;
        self
    }

    pub fn with_sample_size(mut self, size: usize) -> Self {
        self.sample_size = size;
        self
    }

    pub fn with_time_budget_ms(mut self, ms: u64) -> Self {
        self.time_budget_ms = ms;
        self
    }

    pub fn with_max_results(mut self, max: usize) -> Self {
        self.max_results = max;
        self
    }

    pub fn with_min_score(mut self, min: usize) -> Self {
        self.min_score = min;
        self
    }
}

/// A friend-of-friend candidate with their score
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FofCandidate {
    /// The candidate entity ID
    pub id: EntityId,
    /// Score (number of mutual connections)
    pub score: usize,
}

impl FofCandidate {
    pub fn new(id: EntityId, score: usize) -> Self {
        FofCandidate { id, score }
    }
}

/// Result of a friend-of-friend query
#[derive(Debug, Clone)]
pub struct FofResult {
    /// Ranked candidates (highest score first)
    pub candidates: Vec<FofCandidate>,
    /// Number of first-hop neighbors processed
    pub neighbors_processed: usize,
    /// Whether the query was truncated due to limits
    pub truncated: bool,
    /// Time spent on the query
    pub elapsed_ms: u64,
}

/// Friend-of-Friend query executor
pub struct FofQuery {
    config: FofConfig,
}

impl FofQuery {
    pub fn new(config: FofConfig) -> Self {
        FofQuery { config }
    }

    pub fn with_default_config() -> Self {
        FofQuery {
            config: FofConfig::default(),
        }
    }

    /// Execute a friend-of-friend query.
    ///
    /// Given:
    /// - `source`: The entity to find friends-of-friends for
    /// - `direct_friends`: IDs of direct friends (first hop)
    /// - `get_friends`: Function to get friends of a given entity
    /// - `exclusions`: Additional IDs to exclude (e.g., blocked users)
    pub fn execute<F>(
        &self,
        source: EntityId,
        direct_friends: &[EntityId],
        get_friends: F,
        exclusions: &[EntityId],
    ) -> FofResult
    where
        F: Fn(EntityId) -> Vec<EntityId>,
    {
        let start = Instant::now();
        let deadline = if self.config.time_budget_ms > 0 {
            Some(start + Duration::from_millis(self.config.time_budget_ms))
        } else {
            None
        };

        let mut candidates: HashMap<EntityId, usize> = HashMap::new();
        let mut neighbors_processed = 0;
        let mut truncated = false;

        // Build exclusion set
        let mut exclude_set: std::collections::HashSet<EntityId> = exclusions.iter().copied().collect();
        if self.config.exclude_self {
            exclude_set.insert(source);
        }
        if self.config.exclude_direct {
            for &friend in direct_friends {
                exclude_set.insert(friend);
            }
        }

        // Determine which friends to process
        let friends_to_process = self.select_friends(direct_friends);

        // Traverse second hop
        for &friend in &friends_to_process {
            // Check time budget
            if let Some(deadline) = deadline {
                if Instant::now() >= deadline {
                    truncated = true;
                    break;
                }
            }

            neighbors_processed += 1;

            // Get friends of this friend
            let fof_list = get_friends(friend);
            
            // Apply fanout cap to second hop
            let fof_to_process: Vec<EntityId> = if fof_list.len() > self.config.fanout_cap {
                truncated = true;
                fof_list.into_iter().take(self.config.fanout_cap).collect()
            } else {
                fof_list
            };

            // Count candidates
            for fof in fof_to_process {
                if !exclude_set.contains(&fof) {
                    *candidates.entry(fof).or_insert(0) += 1;
                }
            }
        }

        // Filter by minimum score and sort
        let mut ranked: Vec<FofCandidate> = candidates
            .into_iter()
            .filter(|(_, score)| *score >= self.config.min_score)
            .map(|(id, score)| FofCandidate::new(id, score))
            .collect();

        // Sort by score descending, then by ID for stability
        ranked.sort_by(|a, b| b.score.cmp(&a.score).then_with(|| a.id.cmp(&b.id)));

        // Apply max results limit
        if ranked.len() > self.config.max_results {
            ranked.truncate(self.config.max_results);
            truncated = true;
        }

        let elapsed = start.elapsed();

        FofResult {
            candidates: ranked,
            neighbors_processed,
            truncated,
            elapsed_ms: elapsed.as_millis() as u64,
        }
    }

    /// Select which friends to process based on fanout cap and sampling
    fn select_friends(&self, friends: &[EntityId]) -> Vec<EntityId> {
        if friends.len() <= self.config.fanout_cap {
            return friends.to_vec();
        }

        if self.config.sample_size > 0 && self.config.sample_size < friends.len() {
            // Random sampling
            self.sample_random(friends, self.config.sample_size)
        } else {
            // Just take first fanout_cap
            friends.iter().take(self.config.fanout_cap).copied().collect()
        }
    }

    /// Random sampling using a simple deterministic approach
    /// (For production, use a proper RNG)
    fn sample_random(&self, items: &[EntityId], n: usize) -> Vec<EntityId> {
        if items.len() <= n {
            return items.to_vec();
        }

        // Deterministic sampling based on ID values
        // This ensures reproducibility for testing
        let step = items.len() / n;
        let mut result = Vec::with_capacity(n);
        
        for i in 0..n {
            let idx = (i * step) % items.len();
            result.push(items[idx]);
        }
        
        result
    }
}

/// Convenience function for simple friend-of-friend query
pub fn friends_of_friends<F>(
    source: EntityId,
    direct_friends: &[EntityId],
    get_friends: F,
) -> Vec<FofCandidate>
where
    F: Fn(EntityId) -> Vec<EntityId>,
{
    let query = FofQuery::with_default_config();
    let result = query.execute(source, direct_friends, get_friends, &[]);
    result.candidates
}

/// Find mutual friends between two entities
pub fn mutual_friends(
    friends_a: &[EntityId],
    friends_b: &[EntityId],
) -> Vec<EntityId> {
    let mut a_sorted = friends_a.to_vec();
    let mut b_sorted = friends_b.to_vec();
    a_sorted.sort();
    b_sorted.sort();
    
    set_ops::intersect(
        VecIterator::new(a_sorted),
        VecIterator::new(b_sorted),
    )
}

/// Count mutual friends without materializing the list
pub fn count_mutual_friends(
    friends_a: &[EntityId],
    friends_b: &[EntityId],
) -> usize {
    let mut a_sorted = friends_a.to_vec();
    let mut b_sorted = friends_b.to_vec();
    a_sorted.sort();
    b_sorted.sort();
    
    set_ops::count_intersection(
        VecIterator::new(a_sorted),
        VecIterator::new(b_sorted),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::id::EntityType;

    fn make_id(raw: u64) -> EntityId {
        EntityId::new(EntityType::User, raw)
    }

    fn make_ids(raws: &[u64]) -> Vec<EntityId> {
        raws.iter().map(|&r| make_id(r)).collect()
    }

    // Simple in-memory graph for testing
    fn build_test_graph() -> HashMap<EntityId, Vec<EntityId>> {
        let mut graph = HashMap::new();
        
        // User 1's friends: 2, 3, 4
        graph.insert(make_id(1), make_ids(&[2, 3, 4]));
        
        // User 2's friends: 1, 5, 6
        graph.insert(make_id(2), make_ids(&[1, 5, 6]));
        
        // User 3's friends: 1, 5, 7
        graph.insert(make_id(3), make_ids(&[1, 5, 7]));
        
        // User 4's friends: 1, 6, 8
        graph.insert(make_id(4), make_ids(&[1, 6, 8]));
        
        // User 5's friends: 2, 3
        graph.insert(make_id(5), make_ids(&[2, 3]));
        
        // User 6's friends: 2, 4
        graph.insert(make_id(6), make_ids(&[2, 4]));
        
        // User 7's friends: 3
        graph.insert(make_id(7), make_ids(&[3]));
        
        // User 8's friends: 4
        graph.insert(make_id(8), make_ids(&[4]));
        
        graph
    }

    #[test]
    fn test_basic_fof_query() {
        let graph = build_test_graph();
        let get_friends = |id: EntityId| graph.get(&id).cloned().unwrap_or_default();
        
        let source = make_id(1);
        let direct_friends = make_ids(&[2, 3, 4]);
        
        let query = FofQuery::with_default_config();
        let result = query.execute(source, &direct_friends, get_friends, &[]);
        
        // User 1's friends are 2, 3, 4
        // User 2's friends: 5, 6 (excluding 1)
        // User 3's friends: 5, 7 (excluding 1)
        // User 4's friends: 6, 8 (excluding 1)
        // So FOF candidates are: 5 (2), 6 (2), 7 (1), 8 (1)
        
        assert!(!result.candidates.is_empty());
        
        // 5 and 6 should have score 2 (highest)
        let top_scores: Vec<_> = result.candidates.iter()
            .filter(|c| c.score == 2)
            .map(|c| c.id)
            .collect();
        assert!(top_scores.contains(&make_id(5)));
        assert!(top_scores.contains(&make_id(6)));
    }

    #[test]
    fn test_fof_excludes_self() {
        let graph = build_test_graph();
        let get_friends = |id: EntityId| graph.get(&id).cloned().unwrap_or_default();
        
        let source = make_id(1);
        let direct_friends = make_ids(&[2, 3, 4]);
        
        let query = FofQuery::new(FofConfig {
            exclude_self: true,
            exclude_direct: false,
            ..Default::default()
        });
        
        let result = query.execute(source, &direct_friends, get_friends, &[]);
        
        // Self should not be in results
        assert!(!result.candidates.iter().any(|c| c.id == source));
    }

    #[test]
    fn test_fof_excludes_direct_friends() {
        let graph = build_test_graph();
        let get_friends = |id: EntityId| graph.get(&id).cloned().unwrap_or_default();
        
        let source = make_id(1);
        let direct_friends = make_ids(&[2, 3, 4]);
        
        let query = FofQuery::new(FofConfig {
            exclude_direct: true,
            ..Default::default()
        });
        
        let result = query.execute(source, &direct_friends, get_friends, &[]);
        
        // Direct friends should not be in results
        for friend in &direct_friends {
            assert!(!result.candidates.iter().any(|c| c.id == *friend));
        }
    }

    #[test]
    fn test_fof_with_fanout_cap() {
        let graph = build_test_graph();
        let get_friends = |id: EntityId| graph.get(&id).cloned().unwrap_or_default();
        
        let source = make_id(1);
        let direct_friends = make_ids(&[2, 3, 4]);
        
        let query = FofQuery::new(FofConfig {
            fanout_cap: 2, // Only process 2 friends
            ..Default::default()
        });
        
        let result = query.execute(source, &direct_friends, get_friends, &[]);
        
        assert!(result.neighbors_processed <= 2);
    }

    #[test]
    fn test_fof_with_min_score() {
        let graph = build_test_graph();
        let get_friends = |id: EntityId| graph.get(&id).cloned().unwrap_or_default();
        
        let source = make_id(1);
        let direct_friends = make_ids(&[2, 3, 4]);
        
        let query = FofQuery::new(FofConfig {
            min_score: 2, // Only candidates with 2+ mutual friends
            ..Default::default()
        });
        
        let result = query.execute(source, &direct_friends, get_friends, &[]);
        
        // All candidates should have score >= 2
        for candidate in &result.candidates {
            assert!(candidate.score >= 2);
        }
    }

    #[test]
    fn test_fof_with_exclusions() {
        let graph = build_test_graph();
        let get_friends = |id: EntityId| graph.get(&id).cloned().unwrap_or_default();
        
        let source = make_id(1);
        let direct_friends = make_ids(&[2, 3, 4]);
        let exclusions = make_ids(&[5]); // Exclude user 5
        
        let query = FofQuery::with_default_config();
        let result = query.execute(source, &direct_friends, get_friends, &exclusions);
        
        // User 5 should not be in results
        assert!(!result.candidates.iter().any(|c| c.id == make_id(5)));
    }

    #[test]
    fn test_fof_max_results() {
        let graph = build_test_graph();
        let get_friends = |id: EntityId| graph.get(&id).cloned().unwrap_or_default();
        
        let source = make_id(1);
        let direct_friends = make_ids(&[2, 3, 4]);
        
        let query = FofQuery::new(FofConfig {
            max_results: 2,
            ..Default::default()
        });
        
        let result = query.execute(source, &direct_friends, get_friends, &[]);
        
        assert!(result.candidates.len() <= 2);
    }

    #[test]
    fn test_mutual_friends() {
        let friends_a = make_ids(&[1, 2, 3, 4, 5]);
        let friends_b = make_ids(&[3, 4, 5, 6, 7]);
        
        let mutual = mutual_friends(&friends_a, &friends_b);
        assert_eq!(mutual, make_ids(&[3, 4, 5]));
    }

    #[test]
    fn test_count_mutual_friends() {
        let friends_a = make_ids(&[1, 2, 3, 4, 5]);
        let friends_b = make_ids(&[3, 4, 5, 6, 7]);
        
        let count = count_mutual_friends(&friends_a, &friends_b);
        assert_eq!(count, 3);
    }

    #[test]
    fn test_convenience_function() {
        let graph = build_test_graph();
        let get_friends = |id: EntityId| graph.get(&id).cloned().unwrap_or_default();
        
        let source = make_id(1);
        let direct_friends = make_ids(&[2, 3, 4]);
        
        let candidates = friends_of_friends(source, &direct_friends, get_friends);
        
        assert!(!candidates.is_empty());
    }
}

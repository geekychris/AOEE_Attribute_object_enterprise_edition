//! Set operations on posting lists.
//!
//! Provides efficient intersection, union, and difference operations
//! using iterator-based streaming to minimize memory allocation.

use crate::id::EntityId;
use crate::iterator::{PostingIterator, VecIterator};

/// Intersect two sorted iterators using two-pointer merge.
///
/// Returns only elements present in both iterators.
pub fn intersect<A, B>(mut iter_a: A, mut iter_b: B) -> Vec<EntityId>
where
    A: PostingIterator,
    B: PostingIterator,
{
    let mut result = Vec::new();
    
    let mut a = iter_a.next();
    let mut b = iter_b.next();
    
    while let (Some(av), Some(bv)) = (a, b) {
        match av.cmp(&bv) {
            std::cmp::Ordering::Equal => {
                result.push(av);
                a = iter_a.next();
                b = iter_b.next();
            }
            std::cmp::Ordering::Less => {
                a = iter_a.next();
            }
            std::cmp::Ordering::Greater => {
                b = iter_b.next();
            }
        }
    }
    
    result
}

/// Intersect two slices (convenience wrapper).
pub fn intersect_slices(a: &[EntityId], b: &[EntityId]) -> Vec<EntityId> {
    intersect(
        VecIterator::new(a.to_vec()),
        VecIterator::new(b.to_vec()),
    )
}

/// Galloping intersection for when one list is much smaller than the other.
///
/// More efficient when |small| << |large| because we skip over large
/// portions of the larger list using binary search.
pub fn intersect_galloping<A, B>(small: A, mut large: B) -> Vec<EntityId>
where
    A: PostingIterator,
    B: PostingIterator,
{
    let mut result = Vec::new();
    
    for id in small {
        // Seek large to >= id
        if let Some(found) = large.seek(id) {
            if found == id {
                result.push(id);
            }
        } else {
            // No more elements in large >= id, we're done
            break;
        }
    }
    
    result
}

/// Intersect two slices using galloping (convenience wrapper).
pub fn intersect_galloping_slices(small: &[EntityId], large: &[EntityId]) -> Vec<EntityId> {
    intersect_galloping(
        VecIterator::new(small.to_vec()),
        VecIterator::new(large.to_vec()),
    )
}

/// Adaptive intersection that chooses between merge and galloping
/// based on relative sizes.
pub fn intersect_adaptive<A, B>(iter_a: A, iter_b: B) -> Vec<EntityId>
where
    A: PostingIterator,
    B: PostingIterator,
{
    let size_a = iter_a.size_hint_lower();
    let size_b = iter_b.size_hint_lower();
    
    // Use galloping if one is significantly smaller (10x)
    if size_a > 0 && size_b > size_a * 10 {
        intersect_galloping(iter_a, iter_b)
    } else if size_b > 0 && size_a > size_b * 10 {
        intersect_galloping(iter_b, iter_a)
    } else {
        intersect(iter_a, iter_b)
    }
}

/// Union two sorted iterators.
///
/// Returns all unique elements from both iterators.
pub fn union<A, B>(mut iter_a: A, mut iter_b: B) -> Vec<EntityId>
where
    A: PostingIterator,
    B: PostingIterator,
{
    let mut result = Vec::new();
    
    let mut a = iter_a.next();
    let mut b = iter_b.next();
    
    loop {
        match (a, b) {
            (Some(av), Some(bv)) => {
                match av.cmp(&bv) {
                    std::cmp::Ordering::Equal => {
                        result.push(av);
                        a = iter_a.next();
                        b = iter_b.next();
                    }
                    std::cmp::Ordering::Less => {
                        result.push(av);
                        a = iter_a.next();
                    }
                    std::cmp::Ordering::Greater => {
                        result.push(bv);
                        b = iter_b.next();
                    }
                }
            }
            (Some(av), None) => {
                result.push(av);
                result.extend(iter_a);
                break;
            }
            (None, Some(bv)) => {
                result.push(bv);
                result.extend(iter_b);
                break;
            }
            (None, None) => break,
        }
    }
    
    result
}

/// Union two slices (convenience wrapper).
pub fn union_slices(a: &[EntityId], b: &[EntityId]) -> Vec<EntityId> {
    union(
        VecIterator::new(a.to_vec()),
        VecIterator::new(b.to_vec()),
    )
}

/// Difference: elements in A but not in B.
pub fn difference<A, B>(mut iter_a: A, mut iter_b: B) -> Vec<EntityId>
where
    A: PostingIterator,
    B: PostingIterator,
{
    let mut result = Vec::new();
    
    let mut a = iter_a.next();
    let mut b = iter_b.next();
    
    loop {
        match (a, b) {
            (Some(av), Some(bv)) => {
                match av.cmp(&bv) {
                    std::cmp::Ordering::Equal => {
                        // Skip - in both
                        a = iter_a.next();
                        b = iter_b.next();
                    }
                    std::cmp::Ordering::Less => {
                        // av not in B
                        result.push(av);
                        a = iter_a.next();
                    }
                    std::cmp::Ordering::Greater => {
                        // bv not in A, skip
                        b = iter_b.next();
                    }
                }
            }
            (Some(av), None) => {
                // Rest of A is not in B
                result.push(av);
                result.extend(iter_a);
                break;
            }
            (None, _) => break,
        }
    }
    
    result
}

/// Difference of two slices (convenience wrapper).
pub fn difference_slices(a: &[EntityId], b: &[EntityId]) -> Vec<EntityId> {
    difference(
        VecIterator::new(a.to_vec()),
        VecIterator::new(b.to_vec()),
    )
}

/// Symmetric difference: elements in A or B but not both.
pub fn symmetric_difference<A, B>(mut iter_a: A, mut iter_b: B) -> Vec<EntityId>
where
    A: PostingIterator,
    B: PostingIterator,
{
    let mut result = Vec::new();
    
    let mut a = iter_a.next();
    let mut b = iter_b.next();
    
    loop {
        match (a, b) {
            (Some(av), Some(bv)) => {
                match av.cmp(&bv) {
                    std::cmp::Ordering::Equal => {
                        // In both - skip
                        a = iter_a.next();
                        b = iter_b.next();
                    }
                    std::cmp::Ordering::Less => {
                        result.push(av);
                        a = iter_a.next();
                    }
                    std::cmp::Ordering::Greater => {
                        result.push(bv);
                        b = iter_b.next();
                    }
                }
            }
            (Some(av), None) => {
                result.push(av);
                result.extend(iter_a);
                break;
            }
            (None, Some(bv)) => {
                result.push(bv);
                result.extend(iter_b);
                break;
            }
            (None, None) => break,
        }
    }
    
    result
}

/// Check if an element exists in a sorted slice using binary search.
pub fn contains_sorted(ids: &[EntityId], target: EntityId) -> bool {
    ids.binary_search(&target).is_ok()
}

/// Count common elements between two sorted iterators without materializing.
pub fn count_intersection<A, B>(mut iter_a: A, mut iter_b: B) -> usize
where
    A: PostingIterator,
    B: PostingIterator,
{
    let mut count = 0;
    
    let mut a = iter_a.next();
    let mut b = iter_b.next();
    
    while let (Some(av), Some(bv)) = (a, b) {
        match av.cmp(&bv) {
            std::cmp::Ordering::Equal => {
                count += 1;
                a = iter_a.next();
                b = iter_b.next();
            }
            std::cmp::Ordering::Less => {
                a = iter_a.next();
            }
            std::cmp::Ordering::Greater => {
                b = iter_b.next();
            }
        }
    }
    
    count
}

/// Multi-way intersection: returns elements present in ALL iterators.
pub fn intersect_many<I>(iterators: Vec<I>) -> Vec<EntityId>
where
    I: PostingIterator,
{
    if iterators.is_empty() {
        return Vec::new();
    }
    
    let mut iters: Vec<_> = iterators.into_iter().collect();
    
    if iters.len() == 1 {
        return iters.pop().unwrap().collect();
    }
    
    // Pairwise intersection
    let mut result: Vec<EntityId> = iters.pop().unwrap().collect();
    
    for iter in iters {
        result = intersect(VecIterator::new(result), iter);
        if result.is_empty() {
            break;
        }
    }
    
    result
}

/// Multi-way union: returns all unique elements from all iterators.
pub fn union_many<I>(iterators: Vec<I>) -> Vec<EntityId>
where
    I: PostingIterator,
{
    if iterators.is_empty() {
        return Vec::new();
    }
    
    let mut iters: Vec<_> = iterators.into_iter().collect();
    
    if iters.len() == 1 {
        return iters.pop().unwrap().collect();
    }
    
    // Pairwise union
    let mut result: Vec<EntityId> = iters.pop().unwrap().collect();
    
    for iter in iters {
        result = union(VecIterator::new(result), iter);
    }
    
    result
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

    // ========== Intersection Tests ==========

    #[test]
    fn test_intersect_basic() {
        let a = make_ids(&[1, 2, 3, 4, 5]);
        let b = make_ids(&[2, 4, 6, 8]);
        let result = intersect_slices(&a, &b);
        assert_eq!(result, make_ids(&[2, 4]));
    }

    #[test]
    fn test_intersect_empty() {
        let a = make_ids(&[1, 2, 3]);
        let b: Vec<EntityId> = vec![];
        let result = intersect_slices(&a, &b);
        assert!(result.is_empty());
    }

    #[test]
    fn test_intersect_no_overlap() {
        let a = make_ids(&[1, 3, 5]);
        let b = make_ids(&[2, 4, 6]);
        let result = intersect_slices(&a, &b);
        assert!(result.is_empty());
    }

    #[test]
    fn test_intersect_identical() {
        let a = make_ids(&[1, 2, 3]);
        let b = make_ids(&[1, 2, 3]);
        let result = intersect_slices(&a, &b);
        assert_eq!(result, make_ids(&[1, 2, 3]));
    }

    #[test]
    fn test_intersect_subset() {
        let a = make_ids(&[1, 2, 3, 4, 5]);
        let b = make_ids(&[2, 3, 4]);
        let result = intersect_slices(&a, &b);
        assert_eq!(result, make_ids(&[2, 3, 4]));
    }

    // ========== Galloping Intersection Tests ==========

    #[test]
    fn test_intersect_galloping_basic() {
        let small = make_ids(&[5, 100, 500]);
        let large = make_ids(&[1, 2, 3, 4, 5, 10, 20, 50, 100, 200, 300, 400, 500, 600]);
        let result = intersect_galloping_slices(&small, &large);
        assert_eq!(result, make_ids(&[5, 100, 500]));
    }

    #[test]
    fn test_intersect_galloping_no_overlap() {
        let small = make_ids(&[7, 77, 777]);
        let large = make_ids(&[1, 2, 3, 4, 5, 6, 8, 9, 10]);
        let result = intersect_galloping_slices(&small, &large);
        assert!(result.is_empty());
    }

    // ========== Adaptive Intersection Tests ==========

    #[test]
    fn test_intersect_adaptive() {
        let a = make_ids(&[1, 2, 3, 4, 5]);
        let b = make_ids(&[3, 4, 5, 6, 7]);
        let result = intersect_adaptive(
            VecIterator::new(a),
            VecIterator::new(b),
        );
        assert_eq!(result, make_ids(&[3, 4, 5]));
    }

    // ========== Union Tests ==========

    #[test]
    fn test_union_basic() {
        let a = make_ids(&[1, 3, 5]);
        let b = make_ids(&[2, 4, 6]);
        let result = union_slices(&a, &b);
        assert_eq!(result, make_ids(&[1, 2, 3, 4, 5, 6]));
    }

    #[test]
    fn test_union_overlap() {
        let a = make_ids(&[1, 2, 3]);
        let b = make_ids(&[2, 3, 4]);
        let result = union_slices(&a, &b);
        assert_eq!(result, make_ids(&[1, 2, 3, 4]));
    }

    #[test]
    fn test_union_empty() {
        let a = make_ids(&[1, 2, 3]);
        let b: Vec<EntityId> = vec![];
        let result = union_slices(&a, &b);
        assert_eq!(result, make_ids(&[1, 2, 3]));
    }

    #[test]
    fn test_union_identical() {
        let a = make_ids(&[1, 2, 3]);
        let b = make_ids(&[1, 2, 3]);
        let result = union_slices(&a, &b);
        assert_eq!(result, make_ids(&[1, 2, 3]));
    }

    // ========== Difference Tests ==========

    #[test]
    fn test_difference_basic() {
        let a = make_ids(&[1, 2, 3, 4, 5]);
        let b = make_ids(&[2, 4]);
        let result = difference_slices(&a, &b);
        assert_eq!(result, make_ids(&[1, 3, 5]));
    }

    #[test]
    fn test_difference_no_overlap() {
        let a = make_ids(&[1, 3, 5]);
        let b = make_ids(&[2, 4, 6]);
        let result = difference_slices(&a, &b);
        assert_eq!(result, make_ids(&[1, 3, 5]));
    }

    #[test]
    fn test_difference_all_removed() {
        let a = make_ids(&[1, 2, 3]);
        let b = make_ids(&[1, 2, 3, 4, 5]);
        let result = difference_slices(&a, &b);
        assert!(result.is_empty());
    }

    // ========== Symmetric Difference Tests ==========

    #[test]
    fn test_symmetric_difference() {
        let a = make_ids(&[1, 2, 3]);
        let b = make_ids(&[2, 3, 4]);
        let result = symmetric_difference(
            VecIterator::new(a),
            VecIterator::new(b),
        );
        assert_eq!(result, make_ids(&[1, 4]));
    }

    // ========== Contains Tests ==========

    #[test]
    fn test_contains_sorted() {
        let ids = make_ids(&[1, 3, 5, 7, 9]);
        assert!(contains_sorted(&ids, make_id(5)));
        assert!(!contains_sorted(&ids, make_id(4)));
        assert!(!contains_sorted(&ids, make_id(0)));
        assert!(!contains_sorted(&ids, make_id(10)));
    }

    // ========== Count Intersection Tests ==========

    #[test]
    fn test_count_intersection() {
        let a = make_ids(&[1, 2, 3, 4, 5]);
        let b = make_ids(&[2, 4, 6, 8]);
        let count = count_intersection(
            VecIterator::new(a),
            VecIterator::new(b),
        );
        assert_eq!(count, 2);
    }

    // ========== Multi-way Tests ==========

    #[test]
    fn test_intersect_many() {
        let a = make_ids(&[1, 2, 3, 4, 5]);
        let b = make_ids(&[2, 3, 4, 5, 6]);
        let c = make_ids(&[3, 4, 5, 6, 7]);
        
        let result = intersect_many(vec![
            VecIterator::new(a),
            VecIterator::new(b),
            VecIterator::new(c),
        ]);
        assert_eq!(result, make_ids(&[3, 4, 5]));
    }

    #[test]
    fn test_union_many() {
        let a = make_ids(&[1, 2]);
        let b = make_ids(&[3, 4]);
        let c = make_ids(&[5, 6]);
        
        let result = union_many(vec![
            VecIterator::new(a),
            VecIterator::new(b),
            VecIterator::new(c),
        ]);
        assert_eq!(result, make_ids(&[1, 2, 3, 4, 5, 6]));
    }
}

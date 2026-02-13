// AOEE Rust Skeleton (illustrative)
// Focus: PostingList buffer+segments, intersection primitives, and shard-local operations.
use std::sync::Arc;
use dashmap::DashMap;

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct EdgeKey {
    pub src: u64,
    pub etype: u32,
}

#[derive(Clone, Debug)]
pub struct BufferEntry {
    pub dst: u32,
    pub tombstone: bool,
}

#[derive(Clone, Debug)]
pub struct WriteBuffer {
    pub entries: Vec<BufferEntry>, // append-only
}

#[derive(Clone, Debug)]
pub struct Segment {
    pub bytes: Vec<u8>,           // compressed payload (delta/varint or bitpack)
    pub first: u32,
    pub last: u32,
    pub count: u32,
    pub skip: Vec<(u32,u32)>,     // (value, byte_offset)
}

#[derive(Clone, Debug)]
pub struct PostingList {
    pub buffer: WriteBuffer,
    pub segments: Vec<Arc<Segment>>, // immutable snapshot list
}

pub struct AoeeShard {
    lists: DashMap<EdgeKey, Arc<tokio::sync::RwLock<PostingList>>>,
}

impl AoeeShard {
    pub fn new() -> Self {
        Self { lists: DashMap::new() }
    }

    pub async fn add_edge(&self, key: EdgeKey, dst: u32) {
        let lock = self.lists.entry(key)
            .or_insert_with(|| Arc::new(tokio::sync::RwLock::new(PostingList{
                buffer: WriteBuffer{ entries: Vec::new() },
                segments: Vec::new(),
            }))).clone();

        let mut pl = lock.write().await;
        pl.buffer.entries.push(BufferEntry{ dst, tombstone: false });
        // if pl.buffer.entries.len() > THRESHOLD => schedule compaction
    }
}

// Merge intersection for sorted arrays. Real implementation streams iterators.
pub fn intersect_sorted(a: &[u32], b: &[u32]) -> Vec<u32> {
    let mut i = 0usize;
    let mut j = 0usize;
    let mut out = Vec::new();
    while i < a.len() && j < b.len() {
        let av = a[i];
        let bv = b[j];
        if av == bv { out.push(av); i += 1; j += 1; }
        else if av < bv { i += 1; }
        else { j += 1; }
    }
    out
}

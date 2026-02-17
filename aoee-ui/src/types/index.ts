// Edge types
export const EDGE_TYPES = [
  'FOLLOWS',
  'FOLLOWED_BY', 
  'FRIEND_OF',
  'LIKES',
  'LIKED_BY',
  'BLOCKS',
  'MEMBER_OF',
  'HAS_MEMBER',
  'AUTHORED',
  'AUTHORED_BY'
] as const;

export type EdgeType = typeof EDGE_TYPES[number];

// Reaction types for LIKES edges
export const REACTION_TYPES = [
  { value: 0, label: '👍 Like', emoji: '👍' },
  { value: 1, label: '❤️ Love', emoji: '❤️' },
  { value: 2, label: '😂 Haha', emoji: '😂' },
  { value: 3, label: '😮 Wow', emoji: '😮' },
  { value: 4, label: '😢 Sad', emoji: '😢' },
  { value: 5, label: '😠 Angry', emoji: '😠' },
] as const;

export type ReactionType = typeof REACTION_TYPES[number]['value'];

export const getReactionEmoji = (value: number): string => {
  return REACTION_TYPES.find(r => r.value === value)?.emoji || '👍';
};

// API Response types
export interface EdgeResponse {
  success: boolean;
  message: string;
  timestamp?: number; // Nanoseconds since epoch
}

export interface NeighborsResponse {
  src: number;
  edgeType: string;
  neighbors: number[];
  timestamps?: number[];  // Parallel to neighbors, nanoseconds since epoch
  metadata?: number[];    // Parallel to neighbors (e.g., reaction type for LIKES)
}

export interface ContainsResponse {
  src: number;
  edgeType: string;
  dst: number;
  exists: boolean;
}

export interface CountResponse {
  src: number;
  edgeType: string;
  count: number;
}

export interface SetOperationResponse {
  operation: string;
  ids: number[];
}

export interface FofCandidate {
  id: number;
  score: number;
}

export interface FofResponse {
  source: number;
  candidates: FofCandidate[];
  truncated: boolean;
  elapsedMs: number;
}

export interface ShardStats {
  shardId: number;
  cachedLists: number;
  totalEdges: number;
  reads: number;
  writes: number;
  cacheHits: number;
  cacheMisses: number;
  cacheHitRate: number;
}

export interface ServerStatsResponse {
  aggregated: ShardStats | null;
  perShard: ShardStats[];
}

export interface HealthResponse {
  connected: boolean;
  target: string;
}

export interface DatasetParseResult {
  valid: boolean;
  entityCount: number;
  edgeCount: number;
  entitiesByType: Record<string, number>;
  edgesByType: Record<string, number>;
  errors: string[];
}

export interface DatasetLoadResponse {
  success: boolean;
  entitiesLoaded: number;
  edgesLoaded: number;
  errors: number;
  errorMessages: string[];
  elapsedMs: number;
}

// Cache management types
export interface CacheFlushResponse {
  success: boolean;
  entriesFlushed: number;
  clearedAfterFlush: boolean;
  message?: string;
}

export interface CacheClearResponse {
  success: boolean;
  entriesCleared: number;
  shardId: number;
  message?: string;
}

export interface CacheWarmResponse {
  success: boolean;
  edgesLoaded: number;
  elapsedMs: number;
  message?: string;
}

export interface PersistenceStatus {
  enabled: boolean;
  available: boolean;
  writeThrough: boolean;
  stats?: Record<string, unknown>;
}

// Benchmark types
export interface BenchmarkConfig {
  numUsers: number;
  avgFollowsPerUser: number;
  maxFollowsPerUser: number;
  popularUserRatio: number;
  popularUserFollowerMultiplier: number;
  numPosts: number;
  avgLikesPerPost: number;
  maxLikesPerPost: number;
  viralPostRatio: number;
  avgFriendsPerUser: number;
  numGroups: number;
  avgMembersPerGroup: number;
  warmupIterations: number;
  benchmarkIterations: number;
  includeMetadata: boolean;
}

export interface LatencyStats {
  min: number;
  max: number;
  mean: number;
  median: number;
  p90: number;
  p95: number;
  p99: number;
  stdDev: number;
}

export interface OperationResult {
  operation: string;
  description: string;
  iterations: number;
  latency: LatencyStats;
  throughput: number;
  details: Record<string, unknown>;
}

export interface DataGenerationStats {
  usersCreated: number;
  postsCreated: number;
  groupsCreated: number;
  followEdges: number;
  friendEdges: number;
  likeEdges: number;
  memberEdges: number;
  totalEdges: number;
  durationMs: number;
  edgesPerSecond: number;
}

export interface BenchmarkResult {
  config: BenchmarkConfig;
  dataStats: DataGenerationStats;
  operations: OperationResult[];
  totalDurationMs: number;
  summary: string;
}

export interface BenchmarkPresets {
  small: BenchmarkConfig;
  medium: BenchmarkConfig;
  large: BenchmarkConfig;
}

export interface BenchmarkDataStats {
  users: number;
  posts: number;
  groups: number;
  popularUsers: number;
  viralPosts: number;
}

// Graph visualization types
export interface GraphNode {
  id: string;
  label: string;
  type?: string;
}

export interface GraphLink {
  source: string;
  target: string;
  type: string;
  metadata?: number;  // For LIKES edges: reaction type
}

export interface GraphData {
  nodes: GraphNode[];
  links: GraphLink[];
}

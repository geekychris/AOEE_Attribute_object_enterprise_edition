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

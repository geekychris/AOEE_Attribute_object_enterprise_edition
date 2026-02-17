import axios from 'axios';
import type {
  EdgeResponse,
  NeighborsResponse,
  ContainsResponse,
  CountResponse,
  SetOperationResponse,
  FofResponse,
  ServerStatsResponse,
  HealthResponse,
  DatasetParseResult,
  DatasetLoadResponse,
  CacheFlushResponse,
  CacheClearResponse,
  CacheWarmResponse,
  PersistenceStatus,
  BenchmarkResult,
  BenchmarkPresets,
  BenchmarkDataStats,
  DataGenerationStats,
} from '../types';

const API_BASE = 'http://localhost:8080/api';

const api = axios.create({
  baseURL: API_BASE,
  headers: {
    'Content-Type': 'application/json',
  },
});

// Edge operations
export const addEdge = async (
  src: number, 
  edgeType: string, 
  dst: number,
  metadata?: number  // Optional: reaction type for LIKES (0-5)
): Promise<EdgeResponse> => {
  const response = await api.post('/edges', { 
    src, 
    edgeType, 
    dst,
    metadata: metadata ?? 0,
  });
  return response.data;
};

export const deleteEdge = async (src: number, edgeType: string, dst: number): Promise<EdgeResponse> => {
  const response = await api.delete('/edges', { data: { src, edgeType, dst } });
  return response.data;
};

export const getNeighbors = async (
  src: number, 
  edgeType: string, 
  limit?: number,
  includeMetadata?: boolean
): Promise<NeighborsResponse> => {
  const params: Record<string, unknown> = {};
  if (limit) params.limit = limit;
  if (includeMetadata) params.includeMetadata = true;
  const response = await api.get(`/edges/${src}/${edgeType}`, { params });
  return response.data;
};

export const contains = async (src: number, edgeType: string, dst: number): Promise<ContainsResponse> => {
  const response = await api.get(`/edges/${src}/${edgeType}/contains/${dst}`);
  return response.data;
};

export const count = async (src: number, edgeType: string): Promise<CountResponse> => {
  const response = await api.get(`/edges/${src}/${edgeType}/count`);
  return response.data;
};

// Query operations
export const intersect = async (
  src1: number, edgeType1: string,
  src2: number, edgeType2: string
): Promise<SetOperationResponse> => {
  const response = await api.post('/query/intersect', { src1, edgeType1, src2, edgeType2 });
  return response.data;
};

export const union = async (
  src1: number, edgeType1: string,
  src2: number, edgeType2: string
): Promise<SetOperationResponse> => {
  const response = await api.post('/query/union', { src1, edgeType1, src2, edgeType2 });
  return response.data;
};

export const friendOfFriend = async (
  source: number,
  edgeType: string,
  options?: {
    fanoutCap?: number;
    maxResults?: number;
    minScore?: number;
    exclusions?: number[];
  }
): Promise<FofResponse> => {
  const response = await api.post('/query/fof', {
    source,
    edgeType,
    ...options,
  });
  return response.data;
};

export const mutualFriends = async (user1: number, user2: number): Promise<SetOperationResponse> => {
  const response = await api.post('/query/mutual-friends', { user1, user2 });
  return response.data;
};

// Stats
export const getStats = async (perShard = false): Promise<ServerStatsResponse> => {
  const response = await api.get('/stats', { params: { perShard } });
  return response.data;
};

export const getHealth = async (): Promise<HealthResponse> => {
  const response = await api.get('/stats/health');
  return response.data;
};

// Dataset
export const previewDataset = async (content: string): Promise<DatasetParseResult> => {
  const response = await api.post('/dataset/preview', content, {
    headers: { 'Content-Type': 'text/plain' },
  });
  return response.data;
};

export const loadDataset = async (content: string): Promise<DatasetLoadResponse> => {
  const response = await api.post('/dataset/load', content, {
    headers: { 'Content-Type': 'text/plain' },
  });
  return response.data;
};

export const getSampleDataset = async (): Promise<{ format: string; sample: string }> => {
  const response = await api.get('/dataset/sample');
  return response.data;
};

// Cache management
export const flushCache = async (clearAfterFlush = false): Promise<CacheFlushResponse> => {
  const response = await api.post('/cache/flush', null, { 
    params: { clearAfterFlush } 
  });
  return response.data;
};

export const clearCache = async (shardId = 0): Promise<CacheClearResponse> => {
  const response = await api.post('/cache/clear', null, { 
    params: { shardId } 
  });
  return response.data;
};

export const warmCache = async (): Promise<CacheWarmResponse> => {
  const response = await api.post('/cache/warm');
  return response.data;
};

export const getPersistenceStatus = async (): Promise<PersistenceStatus> => {
  const response = await api.get('/cache/persistence/status');
  return response.data;
};

// Benchmark
export const runBenchmark = async (size: 'small' | 'medium' | 'large'): Promise<BenchmarkResult> => {
  const response = await api.post(`/benchmark/run/${size}`);
  return response.data;
};

export const generateBenchmarkData = async (size: 'small' | 'medium' | 'large'): Promise<DataGenerationStats> => {
  const response = await api.post(`/benchmark/generate/${size}`);
  return response.data;
};

export const getBenchmarkPresets = async (): Promise<BenchmarkPresets> => {
  const response = await api.get('/benchmark/presets');
  return response.data;
};

export const getBenchmarkStats = async (): Promise<BenchmarkDataStats> => {
  const response = await api.get('/benchmark/stats');
  return response.data;
};

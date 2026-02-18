import { useState, useEffect } from 'react';
import {
  flushCache,
  clearCache,
  warmCache,
  getPersistenceStatus,
  runBenchmark,
  generateBenchmarkData,
  getBenchmarkPresets,
  getBenchmarkStats,
} from '../services/api';
import type {
  PersistenceStatus,
  BenchmarkResult,
  BenchmarkPresets,
  BenchmarkDataStats,
  DataGenerationStats,
} from '../types';

export default function AdminTools() {
  // Cache state
  const [persistenceStatus, setPersistenceStatus] = useState<PersistenceStatus | null>(null);
  const [cacheLoading, setCacheLoading] = useState(false);
  const [cacheMessage, setCacheMessage] = useState<{ text: string; type: 'success' | 'error' } | null>(null);

  // Benchmark state
  const [benchmarkPresets, setBenchmarkPresets] = useState<BenchmarkPresets | null>(null);
  const [benchmarkStats, setBenchmarkStats] = useState<BenchmarkDataStats | null>(null);
  const [benchmarkRunning, setBenchmarkRunning] = useState(false);
  const [benchmarkResult, setBenchmarkResult] = useState<BenchmarkResult | null>(null);
  const [dataGenResult, setDataGenResult] = useState<DataGenerationStats | null>(null);
  const [selectedSize, setSelectedSize] = useState<'small' | 'medium' | 'large'>('small');

  // Load initial data
  useEffect(() => {
    const loadData = async () => {
      try {
        const [status, presets, stats] = await Promise.all([
          getPersistenceStatus(),
          getBenchmarkPresets(),
          getBenchmarkStats(),
        ]);
        setPersistenceStatus(status);
        setBenchmarkPresets(presets);
        setBenchmarkStats(stats);
      } catch (err) {
        console.error('Failed to load admin data', err);
      }
    };
    loadData();
  }, []);

  // Cache operations
  const handleFlushCache = async (clearAfter: boolean) => {
    setCacheLoading(true);
    setCacheMessage(null);
    try {
      const result = await flushCache(clearAfter);
      if (result.success) {
        setCacheMessage({
          text: `Flushed ${result.entriesFlushed} entries${clearAfter ? ' and cleared cache' : ''}`,
          type: 'success',
        });
      } else {
        setCacheMessage({ text: result.message || 'Flush failed', type: 'error' });
      }
    } catch (err) {
      setCacheMessage({ text: 'Failed to flush cache', type: 'error' });
    }
    setCacheLoading(false);
  };

  const handleClearCache = async () => {
    setCacheLoading(true);
    setCacheMessage(null);
    try {
      const result = await clearCache();
      if (result.success) {
        setCacheMessage({ text: `Cleared ${result.entriesCleared} entries from cache`, type: 'success' });
      } else {
        setCacheMessage({ text: result.message || 'Clear failed', type: 'error' });
      }
    } catch (err) {
      setCacheMessage({ text: 'Failed to clear cache', type: 'error' });
    }
    setCacheLoading(false);
  };

  const handleWarmCache = async () => {
    setCacheLoading(true);
    setCacheMessage(null);
    try {
      const result = await warmCache();
      if (result.success) {
        setCacheMessage({
          text: `Warmed cache with ${result.edgesLoaded} edges in ${result.elapsedMs}ms`,
          type: 'success',
        });
      } else {
        setCacheMessage({ text: result.message || 'Warm failed', type: 'error' });
      }
    } catch (err) {
      setCacheMessage({ text: 'Failed to warm cache', type: 'error' });
    }
    setCacheLoading(false);
  };

  // Benchmark operations
  const handleGenerateData = async () => {
    setBenchmarkRunning(true);
    setBenchmarkResult(null);
    setDataGenResult(null);
    try {
      const result = await generateBenchmarkData(selectedSize);
      setDataGenResult(result);
      // Refresh stats
      const stats = await getBenchmarkStats();
      setBenchmarkStats(stats);
    } catch (err) {
      console.error('Failed to generate data', err);
    }
    setBenchmarkRunning(false);
  };

  const handleRunBenchmark = async () => {
    setBenchmarkRunning(true);
    setBenchmarkResult(null);
    setDataGenResult(null);
    try {
      const result = await runBenchmark(selectedSize);
      setBenchmarkResult(result);
      // Refresh stats
      const stats = await getBenchmarkStats();
      setBenchmarkStats(stats);
    } catch (err) {
      console.error('Failed to run benchmark', err);
    }
    setBenchmarkRunning(false);
  };

  const formatNumber = (n: number) => n.toLocaleString();

  return (
    <div>
      <h1 style={{ marginBottom: 20 }}>Admin Tools</h1>

      {/* Cache Management */}
      <div className="card">
        <h2>Cache Management</h2>
        
        {persistenceStatus && (
          <div style={{ marginBottom: 20, padding: 10, backgroundColor: '#f5f5f5', borderRadius: 4 }}>
            <strong>Persistence Status:</strong>{' '}
            <span style={{ color: persistenceStatus.enabled ? '#22c55e' : '#666' }}>
              {persistenceStatus.enabled ? 'Enabled' : 'Disabled'}
            </span>
            {persistenceStatus.enabled && (
              <>
                {' • '}
                <span style={{ color: persistenceStatus.available ? '#22c55e' : '#ef4444' }}>
                  {persistenceStatus.available ? 'Available' : 'Unavailable'}
                </span>
                {' • '}
                Write-through: {persistenceStatus.writeThrough ? 'Yes' : 'No'}
              </>
            )}
          </div>
        )}

        <div style={{ display: 'flex', gap: 10, flexWrap: 'wrap' }}>
          <button
            className="btn btn-primary"
            onClick={() => handleFlushCache(false)}
            disabled={cacheLoading}
          >
            {cacheLoading ? 'Working...' : 'Flush Cache'}
          </button>
          <button
            className="btn btn-secondary"
            onClick={() => handleFlushCache(true)}
            disabled={cacheLoading}
          >
            Flush & Clear
          </button>
          <button
            className="btn btn-secondary"
            onClick={handleClearCache}
            disabled={cacheLoading}
            style={{ backgroundColor: '#ef4444', borderColor: '#ef4444' }}
          >
            Clear Cache
          </button>
          {persistenceStatus?.enabled && persistenceStatus?.available && (
            <button
              className="btn btn-secondary"
              onClick={handleWarmCache}
              disabled={cacheLoading}
            >
              Warm from Persistence
            </button>
          )}
        </div>

        {cacheMessage && (
          <p style={{ 
            marginTop: 15, 
            color: cacheMessage.type === 'success' ? '#22c55e' : '#ef4444' 
          }}>
            {cacheMessage.text}
          </p>
        )}
      </div>

      {/* Benchmark Controls */}
      <div className="card">
        <h2>Benchmark</h2>

        <div style={{ marginBottom: 20 }}>
          <label style={{ marginRight: 10 }}>Dataset Size:</label>
          <select
            value={selectedSize}
            onChange={(e) => setSelectedSize(e.target.value as 'small' | 'medium' | 'large')}
            style={{ padding: '8px 12px', borderRadius: 4, border: '1px solid #ddd' }}
          >
            <option value="small">Small</option>
            <option value="medium">Medium</option>
            <option value="large">Large</option>
          </select>
        </div>

        {benchmarkPresets && benchmarkPresets[selectedSize] && (
          <div style={{ marginBottom: 20, padding: 10, backgroundColor: '#f5f5f5', borderRadius: 4, fontSize: 14 }}>
            <strong>{selectedSize.charAt(0).toUpperCase() + selectedSize.slice(1)} Preset:</strong>{' '}
            {formatNumber(benchmarkPresets[selectedSize].numUsers)} users,{' '}
            {formatNumber(benchmarkPresets[selectedSize].numPosts)} posts,{' '}
            {formatNumber(benchmarkPresets[selectedSize].numGroups)} groups
          </div>
        )}

        <div style={{ display: 'flex', gap: 10, flexWrap: 'wrap' }}>
          <button
            className="btn btn-secondary"
            onClick={handleGenerateData}
            disabled={benchmarkRunning}
          >
            {benchmarkRunning ? 'Running...' : 'Generate Data Only'}
          </button>
          <button
            className="btn btn-primary"
            onClick={handleRunBenchmark}
            disabled={benchmarkRunning}
          >
            {benchmarkRunning ? 'Running...' : 'Run Full Benchmark'}
          </button>
        </div>

        {benchmarkStats && (benchmarkStats.users > 0 || benchmarkStats.posts > 0) && (
          <div style={{ marginTop: 20 }}>
            <h3 style={{ fontSize: 16, marginBottom: 10 }}>Current Benchmark Data</h3>
            <div className="stats-grid">
              <div className="stat-item">
                <div className="stat-value">{formatNumber(benchmarkStats.users)}</div>
                <div className="stat-label">Users</div>
              </div>
              <div className="stat-item">
                <div className="stat-value">{formatNumber(benchmarkStats.posts)}</div>
                <div className="stat-label">Posts</div>
              </div>
              <div className="stat-item">
                <div className="stat-value">{formatNumber(benchmarkStats.groups)}</div>
                <div className="stat-label">Groups</div>
              </div>
              <div className="stat-item">
                <div className="stat-value">{formatNumber(benchmarkStats.popularUsers)}</div>
                <div className="stat-label">Popular Users</div>
              </div>
            </div>
          </div>
        )}
      </div>

      {/* Data Generation Result */}
      {dataGenResult && (
        <div className="card">
          <h2>Data Generation Result</h2>
          <div className="stats-grid">
            <div className="stat-item">
              <div className="stat-value">{formatNumber(dataGenResult.totalEdges)}</div>
              <div className="stat-label">Total Edges</div>
            </div>
            <div className="stat-item">
              <div className="stat-value">{formatNumber(Math.round(dataGenResult.edgesPerSecond))}</div>
              <div className="stat-label">Edges/Second</div>
            </div>
            <div className="stat-item">
              <div className="stat-value">{formatNumber(dataGenResult.durationMs)}</div>
              <div className="stat-label">Duration (ms)</div>
            </div>
          </div>
          <div style={{ marginTop: 15, fontSize: 14 }}>
            <strong>Edge Breakdown:</strong>{' '}
            {formatNumber(dataGenResult.followEdges)} follows,{' '}
            {formatNumber(dataGenResult.friendEdges)} friends,{' '}
            {formatNumber(dataGenResult.likeEdges)} likes,{' '}
            {formatNumber(dataGenResult.memberEdges)} members
          </div>
        </div>
      )}

      {/* Benchmark Results */}
      {benchmarkResult && (
        <div className="card">
          <h2>Benchmark Results</h2>
          
          <div style={{ marginBottom: 20 }}>
            <strong>Total Duration:</strong> {formatNumber(benchmarkResult.totalDurationMs)} ms
          </div>

          <h3 style={{ fontSize: 16, marginBottom: 10 }}>Operation Latencies (µs)</h3>
          <table style={{ width: '100%', borderCollapse: 'collapse', fontSize: 14 }}>
            <thead>
              <tr style={{ borderBottom: '2px solid #ddd', backgroundColor: '#f5f5f5' }}>
                <th style={{ textAlign: 'left', padding: 10 }}>Operation</th>
                <th style={{ textAlign: 'right', padding: 10 }}>Median</th>
                <th style={{ textAlign: 'right', padding: 10 }}>P95</th>
                <th style={{ textAlign: 'right', padding: 10 }}>P99</th>
                <th style={{ textAlign: 'right', padding: 10 }}>Throughput</th>
              </tr>
            </thead>
            <tbody>
              {benchmarkResult.operations
                .filter((op) => op.iterations > 0)
                .map((op) => (
                  <tr key={op.operation} style={{ borderBottom: '1px solid #eee' }}>
                    <td style={{ padding: 10 }}>{op.operation}</td>
                    <td style={{ textAlign: 'right', padding: 10 }}>{op.latency.median.toFixed(1)}</td>
                    <td style={{ textAlign: 'right', padding: 10 }}>{op.latency.p95.toFixed(1)}</td>
                    <td style={{ textAlign: 'right', padding: 10 }}>{op.latency.p99.toFixed(1)}</td>
                    <td style={{ textAlign: 'right', padding: 10 }}>{formatNumber(Math.round(op.throughput))}/s</td>
                  </tr>
                ))}
            </tbody>
          </table>

          <details style={{ marginTop: 20 }}>
            <summary style={{ cursor: 'pointer', fontWeight: 'bold' }}>Raw Summary</summary>
            <pre style={{ 
              marginTop: 10, 
              padding: 15, 
              backgroundColor: '#1e1e1e', 
              color: '#d4d4d4',
              borderRadius: 4,
              overflow: 'auto',
              fontSize: 12,
              lineHeight: 1.5,
            }}>
              {benchmarkResult.summary}
            </pre>
          </details>
        </div>
      )}
    </div>
  );
}

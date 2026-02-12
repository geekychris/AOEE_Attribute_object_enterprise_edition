import { useState, useEffect } from 'react';
import { getStats, getHealth } from '../services/api';
import type { ServerStatsResponse, HealthResponse } from '../types';

export default function Dashboard() {
  const [stats, setStats] = useState<ServerStatsResponse | null>(null);
  const [health, setHealth] = useState<HealthResponse | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const fetchData = async () => {
    setLoading(true);
    setError(null);
    try {
      const [statsData, healthData] = await Promise.all([
        getStats(true),
        getHealth(),
      ]);
      setStats(statsData);
      setHealth(healthData);
    } catch (err) {
      setError('Failed to connect to server. Is the Spring server running?');
    }
    setLoading(false);
  };

  useEffect(() => {
    fetchData();
    const interval = setInterval(fetchData, 5000);
    return () => clearInterval(interval);
  }, []);

  if (loading && !stats) {
    return <div className="card">Loading...</div>;
  }

  return (
    <div>
      <h1 style={{ marginBottom: 20 }}>Dashboard</h1>

      <div className="card">
        <h2>Connection Status</h2>
        {health ? (
          <div>
            <span className={`status ${health.connected ? 'connected' : 'disconnected'}`}>
              {health.connected ? 'Connected' : 'Disconnected'}
            </span>
            <span style={{ marginLeft: 10, color: '#666' }}>
              Target: {health.target}
            </span>
          </div>
        ) : (
          <span className="status disconnected">Not Connected</span>
        )}
        {error && <p className="error-text" style={{ marginTop: 10 }}>{error}</p>}
        <button className="btn btn-secondary" onClick={fetchData} style={{ marginTop: 10 }}>
          Refresh
        </button>
      </div>

      {stats?.aggregated && (
        <div className="card">
          <h2>Aggregated Statistics</h2>
          <div className="stats-grid">
            <div className="stat-item">
              <div className="stat-value">{stats.aggregated.cachedLists}</div>
              <div className="stat-label">Cached Lists</div>
            </div>
            <div className="stat-item">
              <div className="stat-value">{stats.aggregated.totalEdges}</div>
              <div className="stat-label">Total Edges</div>
            </div>
            <div className="stat-item">
              <div className="stat-value">{stats.aggregated.reads}</div>
              <div className="stat-label">Reads</div>
            </div>
            <div className="stat-item">
              <div className="stat-value">{stats.aggregated.writes}</div>
              <div className="stat-label">Writes</div>
            </div>
            <div className="stat-item">
              <div className="stat-value">{stats.aggregated.cacheHits}</div>
              <div className="stat-label">Cache Hits</div>
            </div>
            <div className="stat-item">
              <div className="stat-value">{(stats.aggregated.cacheHitRate * 100).toFixed(1)}%</div>
              <div className="stat-label">Hit Rate</div>
            </div>
          </div>
        </div>
      )}

      {stats?.perShard && stats.perShard.length > 0 && (
        <div className="card">
          <h2>Per-Shard Statistics</h2>
          <table style={{ width: '100%', borderCollapse: 'collapse' }}>
            <thead>
              <tr style={{ borderBottom: '2px solid #ddd' }}>
                <th style={{ textAlign: 'left', padding: 10 }}>Shard</th>
                <th style={{ textAlign: 'right', padding: 10 }}>Cached</th>
                <th style={{ textAlign: 'right', padding: 10 }}>Reads</th>
                <th style={{ textAlign: 'right', padding: 10 }}>Writes</th>
                <th style={{ textAlign: 'right', padding: 10 }}>Hit Rate</th>
              </tr>
            </thead>
            <tbody>
              {stats.perShard.map((shard) => (
                <tr key={shard.shardId} style={{ borderBottom: '1px solid #eee' }}>
                  <td style={{ padding: 10 }}>Shard {shard.shardId}</td>
                  <td style={{ textAlign: 'right', padding: 10 }}>{shard.cachedLists}</td>
                  <td style={{ textAlign: 'right', padding: 10 }}>{shard.reads}</td>
                  <td style={{ textAlign: 'right', padding: 10 }}>{shard.writes}</td>
                  <td style={{ textAlign: 'right', padding: 10 }}>{(shard.cacheHitRate * 100).toFixed(1)}%</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
}

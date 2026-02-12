import { useState } from 'react';
import { addEdge, deleteEdge, getNeighbors, contains, count } from '../services/api';
import { EDGE_TYPES, REACTION_TYPES, getReactionEmoji } from '../types';
import type { NeighborsResponse, ContainsResponse, CountResponse } from '../types';

export default function EdgeManager() {
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState<string | null>(null);

  // Add/Delete edge
  const [src, setSrc] = useState('1');
  const [edgeType, setEdgeType] = useState('FOLLOWS');
  const [dst, setDst] = useState('2');
  const [metadata, setMetadata] = useState<number | undefined>(undefined);

  // Query
  const [querySrc, setQuerySrc] = useState('1');
  const [queryEdgeType, setQueryEdgeType] = useState('FOLLOWS');
  const [queryLimit, setQueryLimit] = useState('100');
  const [checkDst, setCheckDst] = useState('');
  const [neighborsResult, setNeighborsResult] = useState<NeighborsResponse | null>(null);
  const [containsResult, setContainsResult] = useState<ContainsResponse | null>(null);
  const [countResult, setCountResult] = useState<CountResponse | null>(null);

  const handleAddEdge = async () => {
    setLoading(true);
    setError(null);
    setSuccess(null);

    try {
      const result = await addEdge(parseInt(src), edgeType, parseInt(dst), metadata);
      const reactionInfo = edgeType === 'LIKES' && metadata !== undefined 
        ? ` (${getReactionEmoji(metadata)})` 
        : '';
      const timestamp = result.timestamp ? new Date(result.timestamp).toLocaleString() : '';
      setSuccess(`Edge added: ${src} -[${edgeType}${reactionInfo}]-> ${dst} at ${timestamp}`);
    } catch (err) {
      setError('Failed to add edge');
    }

    setLoading(false);
  };

  const handleDeleteEdge = async () => {
    setLoading(true);
    setError(null);
    setSuccess(null);

    try {
      await deleteEdge(parseInt(src), edgeType, parseInt(dst));
      setSuccess(`Edge deleted: ${src} -[${edgeType}]-> ${dst}`);
    } catch (err) {
      setError('Failed to delete edge');
    }

    setLoading(false);
  };

  const handleGetNeighbors = async () => {
    setLoading(true);
    setError(null);
    setNeighborsResult(null);
    setContainsResult(null);
    setCountResult(null);

    try {
      const result = await getNeighbors(
        parseInt(querySrc),
        queryEdgeType,
        parseInt(queryLimit) || undefined,
        true // include metadata
      );
      setNeighborsResult(result);
    } catch (err) {
      setError('Failed to get neighbors');
    }

    setLoading(false);
  };

  const handleContains = async () => {
    if (!checkDst) {
      setError('Please enter a destination ID to check');
      return;
    }

    setLoading(true);
    setError(null);
    setNeighborsResult(null);
    setContainsResult(null);
    setCountResult(null);

    try {
      const result = await contains(
        parseInt(querySrc),
        queryEdgeType,
        parseInt(checkDst)
      );
      setContainsResult(result);
    } catch (err) {
      setError('Failed to check contains');
    }

    setLoading(false);
  };

  const handleCount = async () => {
    setLoading(true);
    setError(null);
    setNeighborsResult(null);
    setContainsResult(null);
    setCountResult(null);

    try {
      const result = await count(parseInt(querySrc), queryEdgeType);
      setCountResult(result);
    } catch (err) {
      setError('Failed to get count');
    }

    setLoading(false);
  };

  return (
    <div>
      <h1 style={{ marginBottom: 20 }}>Edge Manager</h1>

      <div className="card">
        <h2>Add / Delete Edge</h2>
        <div className="form-row">
          <div className="form-group">
            <label>Source ID</label>
            <input
              type="number"
              value={src}
              onChange={(e) => setSrc(e.target.value)}
            />
          </div>
          <div className="form-group">
            <label>Edge Type</label>
            <select value={edgeType} onChange={(e) => setEdgeType(e.target.value)}>
              {EDGE_TYPES.map((t) => (
                <option key={t} value={t}>{t}</option>
              ))}
            </select>
          </div>
          <div className="form-group">
            <label>Destination ID</label>
            <input
              type="number"
              value={dst}
              onChange={(e) => setDst(e.target.value)}
            />
          </div>
        </div>
        {edgeType === 'LIKES' && (
          <div className="form-group" style={{ marginBottom: 15 }}>
            <label>Reaction Type</label>
            <div style={{ display: 'flex', gap: 8, flexWrap: 'wrap' }}>
              {REACTION_TYPES.map((reaction) => (
                <button
                  key={reaction.value}
                  type="button"
                  onClick={() => setMetadata(reaction.value)}
                  style={{
                    padding: '8px 12px',
                    fontSize: '20px',
                    border: metadata === reaction.value ? '2px solid #007bff' : '1px solid #ccc',
                    borderRadius: 8,
                    background: metadata === reaction.value ? '#e7f3ff' : '#fff',
                    cursor: 'pointer',
                  }}
                  title={reaction.label}
                >
                  {reaction.emoji}
                </button>
              ))}
            </div>
          </div>
        )}
        <div style={{ display: 'flex', gap: 10 }}>
          <button className="btn btn-primary" onClick={handleAddEdge} disabled={loading}>
            Add Edge
          </button>
          <button className="btn btn-danger" onClick={handleDeleteEdge} disabled={loading}>
            Delete Edge
          </button>
        </div>
        {success && <p className="success-text" style={{ marginTop: 10 }}>{success}</p>}
      </div>

      <div className="card">
        <h2>Query Edges</h2>
        <div className="form-row">
          <div className="form-group">
            <label>Source ID</label>
            <input
              type="number"
              value={querySrc}
              onChange={(e) => setQuerySrc(e.target.value)}
            />
          </div>
          <div className="form-group">
            <label>Edge Type</label>
            <select value={queryEdgeType} onChange={(e) => setQueryEdgeType(e.target.value)}>
              {EDGE_TYPES.map((t) => (
                <option key={t} value={t}>{t}</option>
              ))}
            </select>
          </div>
          <div className="form-group">
            <label>Limit (for neighbors)</label>
            <input
              type="number"
              value={queryLimit}
              onChange={(e) => setQueryLimit(e.target.value)}
            />
          </div>
        </div>

        <div className="form-group">
          <label>Check Destination ID (for contains)</label>
          <input
            type="number"
            value={checkDst}
            onChange={(e) => setCheckDst(e.target.value)}
            placeholder="Optional - for contains check"
          />
        </div>

        <div style={{ display: 'flex', gap: 10 }}>
          <button className="btn btn-primary" onClick={handleGetNeighbors} disabled={loading}>
            Get Neighbors
          </button>
          <button className="btn btn-secondary" onClick={handleContains} disabled={loading}>
            Check Contains
          </button>
          <button className="btn btn-secondary" onClick={handleCount} disabled={loading}>
            Get Count
          </button>
        </div>
      </div>

      {neighborsResult && (
        <div className="card">
          <h2>Neighbors of {neighborsResult.src} ({neighborsResult.edgeType})</h2>
          <p>Found {neighborsResult.neighbors.length} neighbors</p>
          <div className="id-list">
            {neighborsResult.neighbors.map((id, idx) => {
              const timestamp = neighborsResult.timestamps?.[idx];
              const meta = neighborsResult.metadata?.[idx];
              const timeStr = timestamp ? new Date(timestamp).toLocaleString() : '';
              const reactionEmoji = neighborsResult.edgeType === 'LIKES' && meta !== undefined 
                ? getReactionEmoji(meta) 
                : '';
              return (
                <span 
                  key={id} 
                  className="id-chip" 
                  title={timeStr}
                  style={{ display: 'inline-flex', alignItems: 'center', gap: 4 }}
                >
                  {id}
                  {reactionEmoji && <span style={{ fontSize: '14px' }}>{reactionEmoji}</span>}
                </span>
              );
            })}
          </div>
        </div>
      )}

      {containsResult && (
        <div className="card">
          <h2>Contains Check</h2>
          <p>
            Edge {containsResult.src} -[{containsResult.edgeType}]-&gt; {containsResult.dst}:{' '}
            <span className={containsResult.exists ? 'success-text' : 'error-text'}>
              {containsResult.exists ? 'EXISTS' : 'NOT FOUND'}
            </span>
          </p>
        </div>
      )}

      {countResult && (
        <div className="card">
          <h2>Edge Count</h2>
          <p>
            {countResult.src} has <strong>{countResult.count}</strong> {countResult.edgeType} edges
          </p>
        </div>
      )}

      {error && <p className="error-text">{error}</p>}
    </div>
  );
}

import { useState } from 'react';
import { intersect, union, friendOfFriend, mutualFriends } from '../services/api';
import { EDGE_TYPES } from '../types';
import type { SetOperationResponse, FofResponse } from '../types';

type QueryType = 'fof' | 'intersect' | 'union' | 'mutual';

export default function QueryBuilder() {
  const [queryType, setQueryType] = useState<QueryType>('fof');
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // FOF params
  const [fofSource, setFofSource] = useState('1');
  const [fofEdgeType, setFofEdgeType] = useState('FOLLOWS');
  const [fofMaxResults, setFofMaxResults] = useState('20');
  const [fofMinScore, setFofMinScore] = useState('1');
  const [fofResult, setFofResult] = useState<FofResponse | null>(null);

  // Set operation params
  const [src1, setSrc1] = useState('1');
  const [edgeType1, setEdgeType1] = useState('FOLLOWS');
  const [src2, setSrc2] = useState('2');
  const [edgeType2, setEdgeType2] = useState('FOLLOWS');
  const [setResult, setSetResult] = useState<SetOperationResponse | null>(null);

  // Mutual friends params
  const [user1, setUser1] = useState('1');
  const [user2, setUser2] = useState('2');

  const handleFofQuery = async () => {
    setLoading(true);
    setError(null);
    setFofResult(null);

    try {
      const result = await friendOfFriend(
        parseInt(fofSource),
        fofEdgeType,
        {
          maxResults: parseInt(fofMaxResults) || undefined,
          minScore: parseInt(fofMinScore) || undefined,
        }
      );
      setFofResult(result);
    } catch (err) {
      setError('Failed to execute FOF query');
    }

    setLoading(false);
  };

  const handleSetOperation = async (op: 'intersect' | 'union') => {
    setLoading(true);
    setError(null);
    setSetResult(null);

    try {
      const fn = op === 'intersect' ? intersect : union;
      const result = await fn(
        parseInt(src1),
        edgeType1,
        parseInt(src2),
        edgeType2
      );
      setSetResult(result);
    } catch (err) {
      setError(`Failed to execute ${op}`);
    }

    setLoading(false);
  };

  const handleMutualFriends = async () => {
    setLoading(true);
    setError(null);
    setSetResult(null);

    try {
      const result = await mutualFriends(parseInt(user1), parseInt(user2));
      setSetResult(result);
    } catch (err) {
      setError('Failed to find mutual friends');
    }

    setLoading(false);
  };

  return (
    <div>
      <h1 style={{ marginBottom: 20 }}>Query Builder</h1>

      <div className="tabs">
        <button
          className={`tab ${queryType === 'fof' ? 'active' : ''}`}
          onClick={() => setQueryType('fof')}
        >
          Friend of Friend
        </button>
        <button
          className={`tab ${queryType === 'intersect' ? 'active' : ''}`}
          onClick={() => setQueryType('intersect')}
        >
          Intersect
        </button>
        <button
          className={`tab ${queryType === 'union' ? 'active' : ''}`}
          onClick={() => setQueryType('union')}
        >
          Union
        </button>
        <button
          className={`tab ${queryType === 'mutual' ? 'active' : ''}`}
          onClick={() => setQueryType('mutual')}
        >
          Mutual Friends
        </button>
      </div>

      {queryType === 'fof' && (
        <div className="card">
          <h2>Friend-of-Friend Query</h2>
          <p style={{ color: '#666', marginBottom: 15, fontSize: 14 }}>
            Find users who are 2 hops away, ranked by number of mutual connections.
          </p>
          <div className="form-row">
            <div className="form-group">
              <label>Source User ID</label>
              <input
                type="number"
                value={fofSource}
                onChange={(e) => setFofSource(e.target.value)}
              />
            </div>
            <div className="form-group">
              <label>Edge Type</label>
              <select value={fofEdgeType} onChange={(e) => setFofEdgeType(e.target.value)}>
                {EDGE_TYPES.map((t) => (
                  <option key={t} value={t}>{t}</option>
                ))}
              </select>
            </div>
          </div>
          <div className="form-row">
            <div className="form-group">
              <label>Max Results</label>
              <input
                type="number"
                value={fofMaxResults}
                onChange={(e) => setFofMaxResults(e.target.value)}
              />
            </div>
            <div className="form-group">
              <label>Min Score (mutual friends)</label>
              <input
                type="number"
                value={fofMinScore}
                onChange={(e) => setFofMinScore(e.target.value)}
              />
            </div>
          </div>
          <button className="btn btn-primary" onClick={handleFofQuery} disabled={loading}>
            {loading ? 'Querying...' : 'Find Friends of Friends'}
          </button>

          {fofResult && (
            <div className="result-box">
              <p>Found {fofResult.candidates.length} candidates in {fofResult.elapsedMs}ms</p>
              {fofResult.truncated && <p className="error-text">Results were truncated</p>}
              <div className="candidate-list">
                {fofResult.candidates.map((c) => (
                  <div key={c.id} className="candidate-item">
                    <span>User {c.id}</span>
                    <span className="candidate-score">{c.score} mutual</span>
                  </div>
                ))}
              </div>
            </div>
          )}
        </div>
      )}

      {(queryType === 'intersect' || queryType === 'union') && (
        <div className="card">
          <h2>{queryType === 'intersect' ? 'Intersect' : 'Union'} Edge Lists</h2>
          <p style={{ color: '#666', marginBottom: 15, fontSize: 14 }}>
            {queryType === 'intersect'
              ? 'Find common neighbors between two users.'
              : 'Combine all neighbors from two users.'}
          </p>
          <div className="form-row">
            <div className="form-group">
              <label>User 1 ID</label>
              <input
                type="number"
                value={src1}
                onChange={(e) => setSrc1(e.target.value)}
              />
            </div>
            <div className="form-group">
              <label>Edge Type 1</label>
              <select value={edgeType1} onChange={(e) => setEdgeType1(e.target.value)}>
                {EDGE_TYPES.map((t) => (
                  <option key={t} value={t}>{t}</option>
                ))}
              </select>
            </div>
          </div>
          <div className="form-row">
            <div className="form-group">
              <label>User 2 ID</label>
              <input
                type="number"
                value={src2}
                onChange={(e) => setSrc2(e.target.value)}
              />
            </div>
            <div className="form-group">
              <label>Edge Type 2</label>
              <select value={edgeType2} onChange={(e) => setEdgeType2(e.target.value)}>
                {EDGE_TYPES.map((t) => (
                  <option key={t} value={t}>{t}</option>
                ))}
              </select>
            </div>
          </div>
          <button
            className="btn btn-primary"
            onClick={() => handleSetOperation(queryType as 'intersect' | 'union')}
            disabled={loading}
          >
            {loading ? 'Querying...' : `Execute ${queryType}`}
          </button>
        </div>
      )}

      {queryType === 'mutual' && (
        <div className="card">
          <h2>Mutual Friends</h2>
          <p style={{ color: '#666', marginBottom: 15, fontSize: 14 }}>
            Find users that both users follow.
          </p>
          <div className="form-row">
            <div className="form-group">
              <label>User 1 ID</label>
              <input
                type="number"
                value={user1}
                onChange={(e) => setUser1(e.target.value)}
              />
            </div>
            <div className="form-group">
              <label>User 2 ID</label>
              <input
                type="number"
                value={user2}
                onChange={(e) => setUser2(e.target.value)}
              />
            </div>
          </div>
          <button className="btn btn-primary" onClick={handleMutualFriends} disabled={loading}>
            {loading ? 'Finding...' : 'Find Mutual Friends'}
          </button>
        </div>
      )}

      {setResult && (
        <div className="card">
          <h2>Result: {setResult.operation}</h2>
          <p>Found {setResult.ids.length} IDs</p>
          <div className="id-list">
            {setResult.ids.map((id) => (
              <span key={id} className="id-chip">{id}</span>
            ))}
          </div>
        </div>
      )}

      {error && <p className="error-text">{error}</p>}
    </div>
  );
}

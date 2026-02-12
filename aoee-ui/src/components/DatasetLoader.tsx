import { useState } from 'react';
import { previewDataset, loadDataset, getSampleDataset } from '../services/api';
import type { DatasetParseResult, DatasetLoadResponse } from '../types';

export default function DatasetLoader() {
  const [content, setContent] = useState('');
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [parseResult, setParseResult] = useState<DatasetParseResult | null>(null);
  const [loadResult, setLoadResult] = useState<DatasetLoadResponse | null>(null);

  const handleLoadSample = async () => {
    setLoading(true);
    setError(null);

    try {
      const result = await getSampleDataset();
      setContent(result.sample);
    } catch (err) {
      setError('Failed to load sample dataset');
    }

    setLoading(false);
  };

  const handlePreview = async () => {
    if (!content.trim()) {
      setError('Please enter dataset content');
      return;
    }

    setLoading(true);
    setError(null);
    setParseResult(null);
    setLoadResult(null);

    try {
      const result = await previewDataset(content);
      setParseResult(result);
    } catch (err) {
      setError('Failed to parse dataset');
    }

    setLoading(false);
  };

  const handleLoad = async () => {
    if (!content.trim()) {
      setError('Please enter dataset content');
      return;
    }

    setLoading(true);
    setError(null);
    setLoadResult(null);

    try {
      const result = await loadDataset(content);
      setLoadResult(result);
    } catch (err) {
      setError('Failed to load dataset');
    }

    setLoading(false);
  };

  const handleFileUpload = (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (!file) return;

    const reader = new FileReader();
    reader.onload = (event) => {
      setContent(event.target?.result as string);
    };
    reader.readAsText(file);
  };

  return (
    <div>
      <h1 style={{ marginBottom: 20 }}>Dataset Loader</h1>

      <div className="card">
        <h2>Dataset Content</h2>
        <p style={{ color: '#666', marginBottom: 15, fontSize: 14 }}>
          Paste dataset content below or upload a file. Use the sample dataset to see the format.
        </p>

        <div style={{ marginBottom: 15 }}>
          <input
            type="file"
            accept=".txt,.aoee"
            onChange={handleFileUpload}
            style={{ marginRight: 10 }}
          />
          <button className="btn btn-secondary" onClick={handleLoadSample}>
            Load Sample Dataset
          </button>
        </div>

        <div className="form-group">
          <label>Dataset Content</label>
          <textarea
            value={content}
            onChange={(e) => setContent(e.target.value)}
            rows={15}
            style={{ 
              fontFamily: 'monospace', 
              fontSize: 12,
              resize: 'vertical',
              minHeight: 200,
              maxHeight: '70vh',
            }}
            placeholder="# Comments start with #
ENTITY USER 1 alice
ENTITY USER 2 bob

FOLLOWS 1 2
LIKES 1 100"
          />
        </div>

        <div style={{ display: 'flex', gap: 10 }}>
          <button className="btn btn-secondary" onClick={handlePreview} disabled={loading}>
            Preview
          </button>
          <button className="btn btn-primary" onClick={handleLoad} disabled={loading}>
            {loading ? 'Loading...' : 'Load Dataset'}
          </button>
        </div>
      </div>

      {parseResult && (
        <div className="card">
          <h2>Preview Result</h2>
          <p className={parseResult.valid ? 'success-text' : 'error-text'}>
            {parseResult.valid ? '✓ Valid dataset' : '✗ Invalid dataset'}
          </p>

          <div className="stats-grid" style={{ marginTop: 15 }}>
            <div className="stat-item">
              <div className="stat-value">{parseResult.entityCount}</div>
              <div className="stat-label">Entities</div>
            </div>
            <div className="stat-item">
              <div className="stat-value">{parseResult.edgeCount}</div>
              <div className="stat-label">Edges</div>
            </div>
          </div>

          {Object.keys(parseResult.entitiesByType).length > 0 && (
            <div style={{ marginTop: 15 }}>
              <h3 style={{ fontSize: 14, marginBottom: 10 }}>Entities by Type</h3>
              <div className="id-list">
                {Object.entries(parseResult.entitiesByType).map(([type, count]) => (
                  <span key={type} className="id-chip">{type}: {count}</span>
                ))}
              </div>
            </div>
          )}

          {Object.keys(parseResult.edgesByType).length > 0 && (
            <div style={{ marginTop: 15 }}>
              <h3 style={{ fontSize: 14, marginBottom: 10 }}>Edges by Type</h3>
              <div className="id-list">
                {Object.entries(parseResult.edgesByType).map(([type, count]) => (
                  <span key={type} className="id-chip">{type}: {count}</span>
                ))}
              </div>
            </div>
          )}

          {parseResult.errors.length > 0 && (
            <div style={{ marginTop: 15 }}>
              <h3 style={{ fontSize: 14, marginBottom: 10, color: '#c62828' }}>Errors</h3>
              <div className="result-box">
                {parseResult.errors.map((err, i) => (
                  <p key={i} className="error-text">{err}</p>
                ))}
              </div>
            </div>
          )}
        </div>
      )}

      {loadResult && (
        <div className="card">
          <h2>Load Result</h2>
          <p className={loadResult.success ? 'success-text' : 'error-text'}>
            {loadResult.success ? '✓ Dataset loaded successfully' : '✗ Dataset loaded with errors'}
          </p>

          <div className="stats-grid" style={{ marginTop: 15 }}>
            <div className="stat-item">
              <div className="stat-value">{loadResult.entitiesLoaded}</div>
              <div className="stat-label">Entities</div>
            </div>
            <div className="stat-item">
              <div className="stat-value">{loadResult.edgesLoaded}</div>
              <div className="stat-label">Edges Loaded</div>
            </div>
            <div className="stat-item">
              <div className="stat-value">{loadResult.elapsedMs}ms</div>
              <div className="stat-label">Time</div>
            </div>
            {loadResult.errors > 0 && (
              <div className="stat-item">
                <div className="stat-value" style={{ color: '#c62828' }}>{loadResult.errors}</div>
                <div className="stat-label">Errors</div>
              </div>
            )}
          </div>

          {loadResult.errorMessages.length > 0 && (
            <div style={{ marginTop: 15 }}>
              <h3 style={{ fontSize: 14, marginBottom: 10, color: '#c62828' }}>Error Messages</h3>
              <div className="result-box">
                {loadResult.errorMessages.map((err, i) => (
                  <p key={i} className="error-text">{err}</p>
                ))}
              </div>
            </div>
          )}
        </div>
      )}

      {error && <p className="error-text">{error}</p>}
    </div>
  );
}

import { useState, useCallback, useRef, useEffect } from 'react';
import ForceGraph2D from 'react-force-graph-2d';
import { getNeighbors } from '../services/api';
import { EDGE_TYPES, getReactionEmoji } from '../types';
import type { GraphLink } from '../types';

interface ForceGraphNode {
  id: string;
  label: string;
  [key: string]: unknown;
}

interface ForceGraphData {
  nodes: ForceGraphNode[];
  links: GraphLink[];
}

// Color palette for edge types
const EDGE_COLORS: Record<string, string> = {
  FOLLOWS: '#4fc3f7',
  FOLLOWED_BY: '#81d4fa',
  FRIEND_OF: '#66bb6a',
  FRIEND: '#66bb6a',
  LIKES: '#ff7043',
  LIKED_BY: '#ffab91',
  MEMBER_OF: '#ab47bc',
  MEMBER: '#ab47bc',
  HAS_MEMBER: '#ce93d8',
  AUTHORED: '#ffa726',
  AUTHORED_BY: '#ffcc80',
  TAGGED_IN: '#26a69a',
  TAGGED: '#26a69a',
  HAS_TAG: '#80cbc4',
  BLOCKS: '#ef5350',
  BLOCKED_BY: '#e57373',
};

const getEdgeColor = (type: string): string => {
  return EDGE_COLORS[type] || '#999';
};

export default function GraphExplorer() {
  const [graphData, setGraphData] = useState<ForceGraphData>({ nodes: [], links: [] });
  const [rootId, setRootId] = useState('1');
  const [selectedEdgeTypes, setSelectedEdgeTypes] = useState<string[]>(['FOLLOWS']);
  const [depth, setDepth] = useState(2);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [showLabels, setShowLabels] = useState(true);
  const [graphHeight, setGraphHeight] = useState(550);
  const [hasLoaded, setHasLoaded] = useState(false); // Track if graph has been loaded at least once
  const graphRef = useRef<any>(null);
  const resizeRef = useRef<{ startY: number; startHeight: number } | null>(null);

  const toggleEdgeType = (type: string) => {
    setSelectedEdgeTypes(prev => 
      prev.includes(type) 
        ? prev.filter(t => t !== type)
        : [...prev, type]
    );
  };

  const loadGraph = async () => {
    if (selectedEdgeTypes.length === 0) {
      setError('Please select at least one edge type');
      return;
    }

    setLoading(true);
    setError(null);

    const nodes = new Map<string, ForceGraphNode>();
    const links: GraphLink[] = [];
    const linkSet = new Set<string>(); // Track unique links
    const visited = new Map<string, Set<string>>(); // Track visited per edge type
    const queue: { id: number; currentDepth: number; edgeType: string }[] = [];

    // Initialize queue with root for each edge type
    for (const edgeType of selectedEdgeTypes) {
      queue.push({ id: parseInt(rootId), currentDepth: 0, edgeType });
      visited.set(edgeType, new Set());
    }

    try {
      while (queue.length > 0) {
        const { id, currentDepth, edgeType } = queue.shift()!;
        const nodeId = String(id);
        const visitedSet = visited.get(edgeType)!;

        if (visitedSet.has(nodeId)) continue;
        visitedSet.add(nodeId);

        if (!nodes.has(nodeId)) {
          nodes.set(nodeId, { id: nodeId, label: `${id}` });
        }

        if (currentDepth < depth) {
          const response = await getNeighbors(id, edgeType, 50, true); // include metadata
          for (let i = 0; i < response.neighbors.length; i++) {
            const neighborId = response.neighbors[i];
            const metadata = response.metadata?.[i];
            const neighborNodeId = String(neighborId);
            if (!nodes.has(neighborNodeId)) {
              nodes.set(neighborNodeId, { id: neighborNodeId, label: `${neighborId}` });
            }
            
            // Create unique link key
            const linkKey = `${nodeId}-${neighborNodeId}-${edgeType}`;
            if (!linkSet.has(linkKey)) {
              linkSet.add(linkKey);
              links.push({ source: nodeId, target: neighborNodeId, type: edgeType, metadata });
            }
            
            if (!visitedSet.has(neighborNodeId)) {
              queue.push({ id: neighborId, currentDepth: currentDepth + 1, edgeType });
            }
          }
        }
      }

      setGraphData({ nodes: Array.from(nodes.values()), links });
      setHasLoaded(true);
    } catch (err) {
      setError('Failed to load graph data. Is the server running?');
    }

    setLoading(false);
  };

  const handleNodeClick = useCallback(async (node: ForceGraphNode) => {
    if (selectedEdgeTypes.length === 0) return;

    try {
      const newNodes = new Map<string, ForceGraphNode>(graphData.nodes.map(n => [n.id, n]));
      const newLinks = [...graphData.links];
      const linkSet = new Set(newLinks.map(l => {
        const srcId = typeof l.source === 'object' ? (l.source as any).id : l.source;
        const tgtId = typeof l.target === 'object' ? (l.target as any).id : l.target;
        return `${srcId}-${tgtId}-${l.type}`;
      }));

      for (const edgeType of selectedEdgeTypes) {
        const response = await getNeighbors(parseInt(node.id), edgeType, 50, true);
        
        for (let i = 0; i < response.neighbors.length; i++) {
          const neighborId = response.neighbors[i];
          const metadata = response.metadata?.[i];
          const nodeId = String(neighborId);
          if (!newNodes.has(nodeId)) {
            newNodes.set(nodeId, { id: nodeId, label: `${neighborId}` });
          }
          
          const linkKey = `${node.id}-${nodeId}-${edgeType}`;
          if (!linkSet.has(linkKey)) {
            linkSet.add(linkKey);
            newLinks.push({ source: node.id, target: nodeId, type: edgeType, metadata });
          }
        }
      }

      setGraphData({ nodes: Array.from(newNodes.values()), links: newLinks });
    } catch (err) {
      console.error('Failed to expand node', err);
    }
  }, [graphData, selectedEdgeTypes]);

  // Auto-zoom to fit when graph data changes
  useEffect(() => {
    if (graphData.nodes.length > 0 && graphRef.current) {
      // Wait for the graph to stabilize a bit before zooming
      const timer = setTimeout(() => {
        graphRef.current?.zoomToFit(400, 50); // 400ms animation, 50px padding
      }, 500);
      return () => clearTimeout(timer);
    }
  }, [graphData]);

  // Auto-reload when edge types change (only if already loaded once)
  useEffect(() => {
    if (hasLoaded && selectedEdgeTypes.length > 0) {
      loadGraph();
    }
  }, [selectedEdgeTypes]); // eslint-disable-line react-hooks/exhaustive-deps

  // Count edges by type
  const edgeCounts = graphData.links.reduce((acc, link) => {
    acc[link.type] = (acc[link.type] || 0) + 1;
    return acc;
  }, {} as Record<string, number>);

  // Resize handlers
  const handleResizeStart = (e: React.MouseEvent) => {
    e.preventDefault();
    resizeRef.current = { startY: e.clientY, startHeight: graphHeight };
    document.addEventListener('mousemove', handleResizeMove);
    document.addEventListener('mouseup', handleResizeEnd);
  };

  const handleResizeMove = (e: MouseEvent) => {
    if (!resizeRef.current) return;
    const delta = e.clientY - resizeRef.current.startY;
    const newHeight = Math.max(300, Math.min(900, resizeRef.current.startHeight + delta));
    setGraphHeight(newHeight);
  };

  const handleResizeEnd = () => {
    resizeRef.current = null;
    document.removeEventListener('mousemove', handleResizeMove);
    document.removeEventListener('mouseup', handleResizeEnd);
  };

  return (
    <div>
      <h1 style={{ marginBottom: 20 }}>Graph Explorer</h1>

      <div className="card">
        <h2>Configure Graph</h2>
        <div className="form-row">
          <div className="form-group">
            <label>Root Entity ID</label>
            <input
              type="number"
              value={rootId}
              onChange={(e) => setRootId(e.target.value)}
            />
          </div>
          <div className="form-group">
            <label>Depth</label>
            <input
              type="number"
              min="1"
              max="5"
              value={depth}
              onChange={(e) => setDepth(parseInt(e.target.value))}
            />
          </div>
          <div className="form-group">
            <label style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
              <input
                type="checkbox"
                checked={showLabels}
                onChange={(e) => setShowLabels(e.target.checked)}
              />
              Show Edge Labels
            </label>
          </div>
        </div>
        
        <div className="form-group">
          <label>Edge Types (select multiple)</label>
          <div style={{ display: 'flex', flexWrap: 'wrap', gap: 8, marginTop: 8 }}>
            {EDGE_TYPES.map((type) => (
              <button
                key={type}
                onClick={() => toggleEdgeType(type)}
                style={{
                  padding: '6px 12px',
                  borderRadius: 16,
                  border: 'none',
                  cursor: 'pointer',
                  fontSize: 12,
                  backgroundColor: selectedEdgeTypes.includes(type) ? getEdgeColor(type) : '#e0e0e0',
                  color: selectedEdgeTypes.includes(type) ? 'white' : '#666',
                }}
              >
                {type}
              </button>
            ))}
          </div>
        </div>

        <button className="btn btn-primary" onClick={loadGraph} disabled={loading}>
          {loading ? 'Loading...' : 'Load Graph'}
        </button>
        {error && <p className="error-text" style={{ marginTop: 10 }}>{error}</p>}
      </div>

      <div className="card" style={{ height: graphHeight, padding: 0, overflow: 'hidden', position: 'relative' }}>
        {graphData.nodes.length > 0 ? (
          <>
            <ForceGraph2D
              ref={graphRef}
              graphData={graphData as any}
              nodeLabel={(node: any) => `Entity ${node.id}`}
              nodeColor={(node: any) => node.id === rootId ? '#f44336' : '#4fc3f7'}
              linkColor={(link: any) => getEdgeColor(link.type)}
              linkWidth={2}
              linkDirectionalArrowLength={6}
              linkDirectionalArrowRelPos={1}
              linkDirectionalArrowColor={(link: any) => getEdgeColor(link.type)}
              onNodeClick={handleNodeClick as any}
              linkCanvasObjectMode={() => showLabels ? 'after' : undefined}
              linkCanvasObject={showLabels ? (link: any, ctx: CanvasRenderingContext2D, globalScale: number) => {
                const start = link.source;
                const end = link.target;
                if (!start.x || !end.x) return;
                
                const textPos = {
                  x: start.x + (end.x - start.x) * 0.5,
                  y: start.y + (end.y - start.y) * 0.5,
                };
                
                const fontSize = Math.max(8 / globalScale, 2);
                ctx.font = `${fontSize}px Sans-Serif`;
                ctx.fillStyle = getEdgeColor(link.type);
                ctx.textAlign = 'center';
                ctx.textBaseline = 'middle';
                
                // Show reaction emoji for LIKES edges
                let label = link.type;
                if (link.type === 'LIKES' && link.metadata !== undefined) {
                  label = getReactionEmoji(link.metadata);
                }
                ctx.fillText(label, textPos.x, textPos.y);
              } : undefined}
              nodeCanvasObject={(node: any, ctx: CanvasRenderingContext2D, globalScale: number) => {
                const label = String(node.id);
                const fontSize = 12 / globalScale;
                const nodeSize = node.id === rootId ? 8 : 5;
                
                // Draw node
                ctx.fillStyle = node.id === rootId ? '#f44336' : '#4fc3f7';
                ctx.beginPath();
                ctx.arc(node.x ?? 0, node.y ?? 0, nodeSize, 0, 2 * Math.PI);
                ctx.fill();
                
                // Draw border for root
                if (node.id === rootId) {
                  ctx.strokeStyle = '#c62828';
                  ctx.lineWidth = 2 / globalScale;
                  ctx.stroke();
                }
                
                // Draw label
                ctx.font = `${fontSize}px Sans-Serif`;
                ctx.fillStyle = '#333';
                ctx.textAlign = 'left';
                ctx.textBaseline = 'middle';
                ctx.fillText(label, (node.x ?? 0) + nodeSize + 3, (node.y ?? 0));
              }}
            />
            
            {/* Legend */}
            <div style={{
              position: 'absolute',
              bottom: 10,
              right: 10,
              background: 'rgba(255,255,255,0.95)',
              padding: 10,
              borderRadius: 8,
              fontSize: 11,
              boxShadow: '0 2px 8px rgba(0,0,0,0.15)',
              maxHeight: 200,
              overflowY: 'auto',
            }}>
              <div style={{ fontWeight: 'bold', marginBottom: 6 }}>Legend</div>
              {Object.entries(edgeCounts).map(([type, count]) => (
                <div key={type} style={{ display: 'flex', alignItems: 'center', gap: 6, marginBottom: 3 }}>
                  <div style={{ width: 12, height: 3, backgroundColor: getEdgeColor(type), borderRadius: 1 }} />
                  <span>{type}</span>
                  <span style={{ color: '#999' }}>({count})</span>
                </div>
              ))}
              <div style={{ marginTop: 6, paddingTop: 6, borderTop: '1px solid #eee' }}>
                <div style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
                  <div style={{ width: 10, height: 10, backgroundColor: '#f44336', borderRadius: '50%' }} />
                  <span>Root node</span>
                </div>
              </div>
            </div>
          </>
        ) : (
          <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'center', height: '100%', color: '#666' }}>
            Select edge types and click "Load Graph" to visualize the network
          </div>
        )}
        
        {/* Resize handle */}
        <div
          onMouseDown={handleResizeStart}
          style={{
            position: 'absolute',
            bottom: 0,
            left: 0,
            right: 0,
            height: 8,
            cursor: 'ns-resize',
            background: 'linear-gradient(to bottom, transparent, rgba(0,0,0,0.1))',
            display: 'flex',
            justifyContent: 'center',
            alignItems: 'center',
          }}
        >
          <div style={{ width: 40, height: 4, backgroundColor: '#ccc', borderRadius: 2 }} />
        </div>
      </div>

      <div className="card">
        <h2>Graph Info</h2>
        <div style={{ display: 'flex', gap: 20 }}>
          <div>
            <strong>{graphData.nodes.length}</strong> nodes
          </div>
          <div>
            <strong>{graphData.links.length}</strong> edges
          </div>
        </div>
        <p style={{ fontSize: 12, color: '#666', marginTop: 8 }}>
          Click on a node to expand its connections for all selected edge types
        </p>
      </div>
    </div>
  );
}

import { useState, useEffect, useRef, useCallback } from 'react';
import { getEntityGraph, type GraphNode, type GraphEdge, type EntityGraph } from '../api/graph';
import { listEntities } from '../api/entities';
import type { Entity } from '../types';

// ---------------------------------------------------------------------------
// Styles
// ---------------------------------------------------------------------------

const styles = {
  container: {
    display: 'flex',
    flexDirection: 'column' as const,
    gap: '1rem',
    padding: '1.5rem',
    height: '100%',
    overflow: 'hidden',
  },
  heading: {
    margin: 0,
    fontSize: '1.5rem',
    fontWeight: 600,
  },
  toolbar: {
    display: 'flex',
    gap: '0.75rem',
    alignItems: 'center',
    flexWrap: 'wrap' as const,
  },
  select: {
    padding: '0.4rem 0.6rem',
    borderRadius: 6,
    border: '1px solid rgba(255,255,255,0.15)',
    background: 'rgba(0,0,0,0.3)',
    color: 'inherit',
    fontSize: '0.85rem',
    fontFamily: 'inherit',
  },
  input: {
    padding: '0.4rem 0.6rem',
    borderRadius: 6,
    border: '1px solid rgba(255,255,255,0.15)',
    background: 'rgba(0,0,0,0.3)',
    color: 'inherit',
    fontSize: '0.85rem',
    fontFamily: 'inherit',
    width: '14rem',
  },
  button: {
    padding: '0.4rem 1rem',
    borderRadius: 6,
    border: 'none',
    background: '#646cff',
    color: '#fff',
    fontSize: '0.85rem',
    fontWeight: 500,
    cursor: 'pointer',
    fontFamily: 'inherit',
  },
  buttonSecondary: {
    background: 'rgba(255,255,255,0.1)',
    color: 'inherit',
  },
  svgContainer: {
    flex: 1,
    background: 'rgba(0,0,0,0.2)',
    borderRadius: 8,
    border: '1px solid rgba(255,255,255,0.08)',
    overflow: 'hidden',
    position: 'relative' as const,
    minHeight: '300px',
  },
  emptyState: {
    position: 'absolute' as const,
    inset: 0,
    display: 'flex',
    flexDirection: 'column' as const,
    alignItems: 'center',
    justifyContent: 'center',
    opacity: 0.4,
    fontSize: '0.9rem',
    gap: '0.5rem',
  },
  tooltip: {
    position: 'absolute' as const,
    background: '#1a1a2e',
    border: '1px solid rgba(255,255,255,0.15)',
    borderRadius: 6,
    padding: '0.5rem 0.75rem',
    fontSize: '0.8rem',
    pointerEvents: 'none' as const,
    zIndex: 10,
    maxWidth: '16rem',
    boxShadow: '0 4px 12px rgba(0,0,0,0.4)',
  },
  legend: {
    display: 'flex',
    gap: '0.75rem',
    flexWrap: 'wrap' as const,
    fontSize: '0.75rem',
    opacity: 0.6,
  },
  legendItem: {
    display: 'flex',
    alignItems: 'center',
    gap: '0.3rem',
  },
  legendDot: (color: string) => ({
    width: 8,
    height: 8,
    borderRadius: '50%',
    background: color,
  }),
  error: {
    color: '#f87171',
    fontSize: '0.85rem',
  },
};

// ---------------------------------------------------------------------------
// Color map for entity types
// ---------------------------------------------------------------------------

const TYPE_COLORS: Record<string, string> = {
  metric: '#8b5cf6',
  experiment: '#06b6d4',
  result: '#10b981',
  task: '#f59e0b',
  project: '#3b82f6',
  decision: '#ec4899',
  person: '#6366f1',
  note: '#78716c',
  session: '#14b8a6',
  campaign: '#f97316',
  audience: '#a855f7',
  competitor: '#ef4444',
  channel: '#0ea5e9',
  spec: '#64748b',
  budget: '#eab308',
  vendor: '#84cc16',
  playbook: '#d946ef',
};

function getNodeColor(entityType: string): string {
  return TYPE_COLORS[entityType] ?? '#888';
}

const ENTITY_TYPES = [
  'audience', 'backlog', 'brief', 'budget', 'campaign', 'channel',
  'competitor', 'decision', 'event', 'experiment', 'metric', 'note',
  'person', 'playbook', 'policy', 'project', 'result', 'session',
  'spec', 'task', 'taxonomy', 'vendor',
];

// ---------------------------------------------------------------------------
// Simple force-directed layout
// ---------------------------------------------------------------------------

interface LayoutNode {
  id: string;
  x: number;
  y: number;
  vx: number;
  vy: number;
  entityType: string;
  title: string;
  status: string | null;
  isRoot: boolean;
}

interface LayoutEdge {
  from: string;
  to: string;
  relationType: string;
}

function runForceLayout(
  nodes: LayoutNode[],
  edges: LayoutEdge[],
  width: number,
  height: number,
  iterations: number = 150,
): LayoutNode[] {
  const result = nodes.map((n) => ({ ...n }));
  const nodeMap = new Map(result.map((n) => [n.id, n]));

  const repulsionStrength = 12000;
  const attractionStrength = 0.005;
  const idealDistance = 180;
  const damping = 0.85;
  const centerGravity = 0.003;
  const minNodeDistance = 80;

  const cx = width / 2;
  const cy = height / 2;

  for (let iter = 0; iter < iterations; iter++) {
    const cooling = 1 - iter / iterations;

    // Repulsion between all pairs
    for (let i = 0; i < result.length; i++) {
      for (let j = i + 1; j < result.length; j++) {
        const a = result[i];
        const b = result[j];
        let dx = a.x - b.x;
        let dy = a.y - b.y;
        let dist = Math.sqrt(dx * dx + dy * dy);
        if (dist < 1) { dx = 1; dy = 1; dist = Math.sqrt(2); }

        const force = (repulsionStrength / (dist * dist)) * cooling;
        const fx = (dx / dist) * force;
        const fy = (dy / dist) * force;

        a.vx += fx;
        a.vy += fy;
        b.vx -= fx;
        b.vy -= fy;
      }
    }

    // Attraction along edges
    for (const edge of edges) {
      const a = nodeMap.get(edge.from);
      const b = nodeMap.get(edge.to);
      if (!a || !b) continue;

      const dx = b.x - a.x;
      const dy = b.y - a.y;
      const dist = Math.sqrt(dx * dx + dy * dy);
      if (dist < 1) continue;

      const force = (dist - idealDistance) * attractionStrength * cooling;
      const fx = (dx / dist) * force;
      const fy = (dy / dist) * force;

      a.vx += fx;
      a.vy += fy;
      b.vx -= fx;
      b.vy -= fy;
    }

    // Center gravity
    for (const node of result) {
      node.vx += (cx - node.x) * centerGravity;
      node.vy += (cy - node.y) * centerGravity;
    }

    // Apply velocities with damping
    for (const node of result) {
      node.vx *= damping;
      node.vy *= damping;
      node.x += node.vx;
      node.y += node.vy;

      // Clamp to bounds
      const margin = 40;
      node.x = Math.max(margin, Math.min(width - margin, node.x));
      node.y = Math.max(margin, Math.min(height - margin, node.y));
    }
  }

  // Post-layout: enforce minimum distance between nodes
  for (let pass = 0; pass < 5; pass++) {
    for (let i = 0; i < result.length; i++) {
      for (let j = i + 1; j < result.length; j++) {
        const a = result[i];
        const b = result[j];
        let dx = a.x - b.x;
        let dy = a.y - b.y;
        const dist = Math.sqrt(dx * dx + dy * dy);
        if (dist < minNodeDistance && dist > 0) {
          const overlap = (minNodeDistance - dist) / 2;
          const nx = dx / dist;
          const ny = dy / dist;
          a.x += nx * overlap;
          a.y += ny * overlap;
          b.x -= nx * overlap;
          b.y -= ny * overlap;
          // Re-clamp
          const margin = 40;
          a.x = Math.max(margin, Math.min(width - margin, a.x));
          a.y = Math.max(margin, Math.min(height - margin, a.y));
          b.x = Math.max(margin, Math.min(width - margin, b.x));
          b.y = Math.max(margin, Math.min(height - margin, b.y));
        }
      }
    }
  }

  return result;
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

interface GraphExplorerProps {
  onNavigateToEntity?: (entityId: string) => void;
  initialEntityId?: string;
}

export default function GraphExplorer({ onNavigateToEntity, initialEntityId }: GraphExplorerProps) {
  const [entities, setEntities] = useState<Entity[]>([]);
  const [selectedEntityId, setSelectedEntityId] = useState(initialEntityId ?? '');
  const [depth, setDepth] = useState(1);
  const [typeFilter, setTypeFilter] = useState('');
  const [graph, setGraph] = useState<EntityGraph | null>(null);
  const [layoutNodes, setLayoutNodes] = useState<LayoutNode[]>([]);
  const [layoutEdges, setLayoutEdges] = useState<LayoutEdge[]>([]);
  const [layoutSize, setLayoutSize] = useState<{ width: number; height: number }>({ width: 1200, height: 800 });
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState('');
  const [tooltip, setTooltip] = useState<{ x: number; y: number; node: LayoutNode } | null>(null);
  const [dragNode, setDragNode] = useState<string | null>(null);
  const [selectedNode, setSelectedNode] = useState<LayoutNode | null>(null);

  const svgRef = useRef<SVGSVGElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);

  // Load entity list for the selector
  useEffect(() => {
    listEntities()
      .then(setEntities)
      .catch(() => {});
  }, []);

  // Fetch graph when entity selected
  const fetchGraph = useCallback(async () => {
    if (!selectedEntityId) return;
    setLoading(true);
    setError('');
    try {
      const result = await getEntityGraph(selectedEntityId, depth);
      setGraph(result);
    } catch (e) {
      setError(String(e));
      setGraph(null);
    } finally {
      setLoading(false);
    }
  }, [selectedEntityId, depth]);

  useEffect(() => {
    if (selectedEntityId) fetchGraph();
  }, [fetchGraph, selectedEntityId]);

  // Compute layout when graph changes
  useEffect(() => {
    if (!graph || !containerRef.current) return;

    const rect = containerRef.current.getBoundingClientRect();
    const nodeCount = graph.nodes.length + 1; // +1 for root
    // Scale canvas size with node count for better spacing
    const baseWidth = rect.width || 800;
    const baseHeight = rect.height || 500;
    const scaleFactor = Math.max(1, Math.sqrt(nodeCount / 6));
    const width = Math.max(1200, baseWidth * scaleFactor);
    const height = Math.max(800, baseHeight * scaleFactor);

    // Combine root + nodes, deduplicate
    const allNodes = new Map<string, GraphNode>();
    allNodes.set(graph.root.entity_id, graph.root);
    for (const node of graph.nodes) {
      allNodes.set(node.entity_id, node);
    }

    // Apply type filter
    let filteredNodes = [...allNodes.values()];
    let filteredEdges = graph.edges;
    if (typeFilter) {
      const allowedIds = new Set(
        filteredNodes.filter((n) => n.entity_type === typeFilter || n.entity_id === graph.root.entity_id).map((n) => n.entity_id),
      );
      filteredNodes = filteredNodes.filter((n) => allowedIds.has(n.entity_id));
      filteredEdges = filteredEdges.filter((e) => allowedIds.has(e.from_id) && allowedIds.has(e.to_id));
    }

    // Create layout nodes with random initial positions
    const cx = width / 2;
    const cy = height / 2;
    const lNodes: LayoutNode[] = filteredNodes.map((n, i) => ({
      id: n.entity_id,
      x: n.entity_id === graph.root.entity_id ? cx : cx + (Math.random() - 0.5) * width * 0.6,
      y: n.entity_id === graph.root.entity_id ? cy : cy + (Math.random() - 0.5) * height * 0.6,
      vx: 0,
      vy: 0,
      entityType: n.entity_type,
      title: n.title,
      status: n.status,
      isRoot: n.entity_id === graph.root.entity_id,
    }));

    const lEdges: LayoutEdge[] = filteredEdges.map((e) => ({
      from: e.from_id,
      to: e.to_id,
      relationType: e.relation_type,
    }));

    const positioned = runForceLayout(lNodes, lEdges, width, height);
    setLayoutNodes(positioned);
    setLayoutEdges(lEdges);
    setLayoutSize({ width, height });
  }, [graph, typeFilter]);

  // Handle drag
  function handleMouseDown(nodeId: string) {
    setDragNode(nodeId);
  }

  function handleMouseMove(e: React.MouseEvent<SVGSVGElement>) {
    if (!dragNode || !svgRef.current) return;
    const rect = svgRef.current.getBoundingClientRect();
    // Translate screen coordinates to viewBox coordinates
    const scaleX = layoutSize.width / rect.width;
    const scaleY = layoutSize.height / rect.height;
    const x = (e.clientX - rect.left) * scaleX;
    const y = (e.clientY - rect.top) * scaleY;
    setLayoutNodes((prev) =>
      prev.map((n) => (n.id === dragNode ? { ...n, x, y } : n)),
    );
  }

  function handleMouseUp() {
    setDragNode(null);
  }

  function handleNodeHover(node: LayoutNode, e: React.MouseEvent) {
    if (dragNode) return;
    const rect = containerRef.current?.getBoundingClientRect();
    if (!rect) return;
    setTooltip({
      x: e.clientX - rect.left + 12,
      y: e.clientY - rect.top - 10,
      node,
    });
  }

  function handleNodeClick(node: LayoutNode) {
    if (selectedNode?.id === node.id) {
      // Second click on the same node: navigate or re-center
      if (onNavigateToEntity) {
        onNavigateToEntity(node.id);
      } else {
        setSelectedEntityId(node.id);
      }
    } else {
      // First click: select this node to show its details
      setSelectedNode(node);
    }
  }

  // Build the node map for edge rendering
  const nodeMap = new Map(layoutNodes.map((n) => [n.id, n]));

  // Collect unique types visible in graph for legend
  const visibleTypes = [...new Set(layoutNodes.map((n) => n.entityType))];

  return (
    <div style={styles.container}>
      <h2 style={styles.heading}>Graph Explorer</h2>

      {/* Toolbar */}
      <div style={styles.toolbar}>
        <select
          style={{ ...styles.select, minWidth: '14rem' }}
          value={selectedEntityId}
          onChange={(e) => setSelectedEntityId(e.target.value)}
        >
          <option value="">-- Select root entity --</option>
          {entities.map((e) => (
            <option key={e.id} value={e.id}>
              [{e.entity_type}] {e.title}
            </option>
          ))}
        </select>

        <label style={{ fontSize: '0.85rem', display: 'flex', alignItems: 'center', gap: '0.3rem' }}>
          Depth:
          <select style={styles.select} value={depth} onChange={(e) => setDepth(Number(e.target.value))}>
            <option value={1}>1</option>
            <option value={2}>2</option>
            <option value={3}>3</option>
          </select>
        </label>

        <select
          style={styles.select}
          value={typeFilter}
          onChange={(e) => setTypeFilter(e.target.value)}
        >
          <option value="">All types</option>
          {ENTITY_TYPES.map((t) => (
            <option key={t} value={t}>{t}</option>
          ))}
        </select>

        <button
          style={{ ...styles.button, ...styles.buttonSecondary }}
          onClick={fetchGraph}
          disabled={loading || !selectedEntityId}
        >
          {loading ? 'Loading...' : 'Refresh'}
        </button>
      </div>

      {error && <div style={styles.error}>{error}</div>}

      {/* Legend */}
      {layoutNodes.length > 0 && (
        <div style={styles.legend}>
          {visibleTypes.map((t) => (
            <div key={t} style={styles.legendItem}>
              <div style={styles.legendDot(getNodeColor(t))} />
              <span>{t}</span>
            </div>
          ))}
        </div>
      )}

      {/* SVG Graph */}
      <div ref={containerRef} style={styles.svgContainer}>
        {!graph && !loading && (
          <div style={styles.emptyState}>
            <div style={{ fontSize: '2rem' }}>&#8635;</div>
            <div>Select an entity to explore its graph</div>
          </div>
        )}

        {loading && (
          <div style={styles.emptyState}>
            <div>Loading graph...</div>
          </div>
        )}

        {layoutNodes.length > 0 && (
          <svg
            ref={svgRef}
            width="100%"
            height="100%"
            viewBox={`0 0 ${layoutSize.width} ${layoutSize.height}`}
            preserveAspectRatio="xMidYMid meet"
            style={{ cursor: dragNode ? 'grabbing' : 'default' }}
            onMouseMove={handleMouseMove}
            onMouseUp={handleMouseUp}
            onMouseLeave={() => { handleMouseUp(); setTooltip(null); }}
          >
            <defs>
              <marker
                id="arrowhead"
                markerWidth="8"
                markerHeight="6"
                refX="8"
                refY="3"
                orient="auto"
              >
                <polygon points="0 0, 8 3, 0 6" fill="rgba(255,255,255,0.3)" />
              </marker>
            </defs>

            {/* Edges */}
            {layoutEdges.map((edge, i) => {
              const from = nodeMap.get(edge.from);
              const to = nodeMap.get(edge.to);
              if (!from || !to) return null;

              // Shorten line to stop at node radius
              const dx = to.x - from.x;
              const dy = to.y - from.y;
              const dist = Math.sqrt(dx * dx + dy * dy);
              if (dist < 1) return null;

              const nodeRadius = 18;
              const x1 = from.x + (dx / dist) * nodeRadius;
              const y1 = from.y + (dy / dist) * nodeRadius;
              const x2 = to.x - (dx / dist) * (nodeRadius + 8);
              const y2 = to.y - (dy / dist) * (nodeRadius + 8);

              const midX = (from.x + to.x) / 2;
              const midY = (from.y + to.y) / 2;

              return (
                <g key={`edge-${i}`}>
                  <line
                    x1={x1}
                    y1={y1}
                    x2={x2}
                    y2={y2}
                    stroke="rgba(255,255,255,0.2)"
                    strokeWidth={1.5}
                    markerEnd="url(#arrowhead)"
                  />
                  <text
                    x={midX}
                    y={midY - 6}
                    textAnchor="middle"
                    fill="rgba(255,255,255,0.35)"
                    fontSize="9"
                    fontFamily="system-ui"
                  >
                    {edge.relationType}
                  </text>
                </g>
              );
            })}

            {/* Nodes */}
            {layoutNodes.map((node) => {
              const color = getNodeColor(node.entityType);
              const radius = node.isRoot ? 22 : 16;
              const isSelected = selectedNode?.id === node.id;

              return (
                <g
                  key={node.id}
                  style={{ cursor: 'pointer' }}
                  onMouseDown={(e) => { e.preventDefault(); handleMouseDown(node.id); }}
                  onMouseEnter={(e) => handleNodeHover(node, e)}
                  onMouseLeave={() => setTooltip(null)}
                  onClick={() => handleNodeClick(node)}
                >
                  {/* Selection highlight ring */}
                  {isSelected && (
                    <circle cx={node.x} cy={node.y} r={radius + 5} fill="none" stroke="#fff" strokeWidth={2} opacity={0.7} />
                  )}
                  {/* Outer ring for root */}
                  {node.isRoot && (
                    <circle cx={node.x} cy={node.y} r={radius + 3} fill="none" stroke={color} strokeWidth={2} opacity={0.5} />
                  )}
                  {/* Node circle */}
                  <circle cx={node.x} cy={node.y} r={radius} fill={color} opacity={isSelected ? 1.0 : 0.85} />
                  {/* Label */}
                  <text
                    x={node.x}
                    y={node.y + radius + 14}
                    textAnchor="middle"
                    fill="rgba(255,255,255,0.8)"
                    fontSize="10"
                    fontFamily="system-ui"
                    fontWeight={node.isRoot ? 600 : 400}
                  >
                    {node.title.length > 20 ? node.title.slice(0, 18) + '...' : node.title}
                  </text>
                  {/* Type abbreviation inside node */}
                  <text
                    x={node.x}
                    y={node.y + 4}
                    textAnchor="middle"
                    fill="#fff"
                    fontSize={node.isRoot ? '10' : '8'}
                    fontFamily="system-ui"
                    fontWeight={600}
                  >
                    {node.entityType.slice(0, 3).toUpperCase()}
                  </text>
                </g>
              );
            })}
          </svg>
        )}

        {/* Tooltip */}
        {tooltip && (
          <div style={{ ...styles.tooltip, left: tooltip.x, top: tooltip.y }}>
            <div style={{ fontWeight: 600, marginBottom: '0.2rem' }}>{tooltip.node.title}</div>
            <div style={{ opacity: 0.6, fontSize: '0.75rem' }}>
              {tooltip.node.entityType} {tooltip.node.status ? `\u2022 ${tooltip.node.status}` : ''}
            </div>
            <div style={{ opacity: 0.4, fontSize: '0.7rem', marginTop: '0.15rem', fontFamily: 'monospace' }}>
              {tooltip.node.id.slice(0, 12)}...
            </div>
          </div>
        )}

        {/* Selected node info bar */}
        {selectedNode && (
          <div style={{
            position: 'absolute',
            bottom: 0,
            left: 0,
            right: 0,
            background: '#1a1a2e',
            borderTop: '1px solid rgba(255,255,255,0.15)',
            padding: '0.5rem 0.75rem',
            display: 'flex',
            alignItems: 'center',
            gap: '0.75rem',
            fontSize: '0.8rem',
            zIndex: 10,
          }}>
            <div style={{
              width: 10,
              height: 10,
              borderRadius: '50%',
              background: getNodeColor(selectedNode.entityType),
              flexShrink: 0,
            }} />
            <div style={{ fontWeight: 600 }}>{selectedNode.title}</div>
            <div style={{ opacity: 0.6, fontSize: '0.75rem' }}>
              {selectedNode.entityType} {selectedNode.status ? `\u2022 ${selectedNode.status}` : ''}
            </div>
            <div style={{ marginLeft: 'auto', display: 'flex', gap: '0.5rem' }}>
              {onNavigateToEntity && (
                <button
                  style={{ ...styles.button, padding: '0.25rem 0.6rem', fontSize: '0.75rem' }}
                  onClick={() => onNavigateToEntity(selectedNode.id)}
                >
                  View Details
                </button>
              )}
              <button
                style={{ ...styles.button, ...styles.buttonSecondary, padding: '0.25rem 0.6rem', fontSize: '0.75rem' }}
                onClick={() => { setSelectedEntityId(selectedNode.id); setSelectedNode(null); }}
              >
                Re-center
              </button>
              <button
                style={{ ...styles.button, ...styles.buttonSecondary, padding: '0.25rem 0.6rem', fontSize: '0.75rem' }}
                onClick={() => setSelectedNode(null)}
              >
                Dismiss
              </button>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}

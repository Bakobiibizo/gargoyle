import { useState, useEffect, useRef, useCallback, useMemo } from 'react';
import {
  listTemplates,
  listRuns,
  checkPrerequisites,
  type TemplateDefinition,
  type PrerequisiteResult,
} from '../api/templates';
import { listEntities } from '../api/entities';
import type { Run, Entity } from '../types';

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
    whiteSpace: 'nowrap' as const,
  },
  buttonSecondary: {
    background: 'rgba(255,255,255,0.1)',
    color: 'inherit',
  },
  buttonSmall: {
    padding: '0.25rem 0.6rem',
    fontSize: '0.75rem',
  },
  tabBar: {
    display: 'flex',
    gap: '0.25rem',
    borderBottom: '1px solid rgba(255,255,255,0.08)',
    paddingBottom: '0.5rem',
  },
  tab: (active: boolean) => ({
    padding: '0.4rem 0.8rem',
    borderRadius: '6px 6px 0 0',
    border: 'none',
    background: active ? 'rgba(100,108,255,0.15)' : 'transparent',
    color: active ? '#a5b4fc' : 'inherit',
    fontSize: '0.85rem',
    fontWeight: active ? 600 : 400,
    cursor: 'pointer',
    fontFamily: 'inherit',
    opacity: active ? 1 : 0.6,
  }),
  contentArea: {
    flex: 1,
    display: 'flex',
    overflow: 'hidden',
    gap: '1rem',
  },
  mainPanel: {
    flex: 1,
    overflow: 'hidden',
    display: 'flex',
    flexDirection: 'column' as const,
  },
  sidePanel: {
    width: '20rem',
    minWidth: '20rem',
    background: 'rgba(0,0,0,0.15)',
    borderRadius: 8,
    border: '1px solid rgba(255,255,255,0.08)',
    overflow: 'auto',
    padding: '1rem',
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
    maxWidth: '20rem',
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
  section: {
    background: 'rgba(255,255,255,0.04)',
    borderRadius: 8,
    border: '1px solid rgba(255,255,255,0.08)',
    overflow: 'hidden',
  },
  sectionTitle: {
    margin: 0,
    padding: '0.75rem 1rem',
    fontSize: '0.8rem',
    fontWeight: 600,
    textTransform: 'uppercase' as const,
    letterSpacing: '0.05em',
    opacity: 0.6,
    borderBottom: '1px solid rgba(255,255,255,0.06)',
  },
  detailRow: {
    display: 'flex',
    gap: '0.5rem',
    marginBottom: '0.3rem',
  },
  detailLabel: {
    fontWeight: 600,
    minWidth: '6rem',
    opacity: 0.6,
    fontSize: '0.8rem',
  },
  detailValue: {
    flex: 1,
    wordBreak: 'break-word' as const,
    fontSize: '0.85rem',
  },
  runRow: {
    padding: '0.5rem 0.75rem',
    borderBottom: '1px solid rgba(255,255,255,0.04)',
    cursor: 'pointer',
    transition: 'background 0.15s',
    fontSize: '0.8rem',
  },
  chainRow: {
    padding: '0.5rem 0.75rem',
    borderBottom: '1px solid rgba(255,255,255,0.04)',
    display: 'flex',
    alignItems: 'center',
    gap: '0.5rem',
    fontSize: '0.8rem',
  },
  badge: (color: string) => ({
    display: 'inline-block',
    padding: '0.1rem 0.45rem',
    borderRadius: 4,
    fontSize: '0.7rem',
    fontWeight: 600,
    background: color,
    color: '#fff',
    textTransform: 'uppercase' as const,
    letterSpacing: '0.03em',
  }),
  statusBadge: (color: string) => ({
    display: 'inline-block',
    padding: '0.1rem 0.45rem',
    borderRadius: 4,
    fontSize: '0.7rem',
    fontWeight: 500,
    border: `1px solid ${color}`,
    color,
  }),
};

// ---------------------------------------------------------------------------
// Color map for template categories
// ---------------------------------------------------------------------------

const CATEGORY_COLORS: Record<string, string> = {
  analytics: '#8b5cf6',
  marketing: '#f97316',
  strategy: '#3b82f6',
  content: '#06b6d4',
  distribution: '#10b981',
  dev: '#6366f1',
  ops: '#f59e0b',
  org: '#14b8a6',
  event: '#ec4899',
  programming: '#a855f7',
  sales: '#ef4444',
  pr: '#0ea5e9',
  people: '#84cc16',
  legal: '#64748b',
  finance: '#eab308',
  staging: '#d946ef',
  release: '#78716c',
  cs: '#22d3ee',
  product: '#fb923c',
};

function getCategoryColor(category: string): string {
  return CATEGORY_COLORS[category] ?? '#888';
}

const STATUS_COLORS: Record<string, string> = {
  applied: '#4ade80',
  pending: '#fbbf24',
  rejected: '#ef4444',
  partial: '#f97316',
};

// ---------------------------------------------------------------------------
// Types for the dependency graph
// ---------------------------------------------------------------------------

type TemplateStatus = 'completed' | 'available' | 'locked';

interface TemplateNode {
  key: string;
  category: string;
  prerequisites: { entity_type: string; min_count: number }[];
  status: TemplateStatus;
  x: number;
  y: number;
}

interface DepEdge {
  from: string; // prerequisite template key
  to: string;   // dependent template key
  entityType: string; // the entity type that bridges them
}

type SubView = 'graph' | 'chains' | 'timeline';

// ---------------------------------------------------------------------------
// Graph layout: layered DAG layout by topological ordering
// ---------------------------------------------------------------------------

function computeLayeredLayout(
  nodes: TemplateNode[],
  edges: DepEdge[],
  width: number,
  height: number,
): TemplateNode[] {
  const result = nodes.map((n) => ({ ...n }));
  const nodeMap = new Map(result.map((n) => [n.key, n]));

  // Build adjacency list (from -> to[])
  const outgoing = new Map<string, string[]>();
  const incoming = new Map<string, Set<string>>();
  for (const n of result) {
    outgoing.set(n.key, []);
    incoming.set(n.key, new Set());
  }
  for (const e of edges) {
    if (outgoing.has(e.from) && incoming.has(e.to)) {
      outgoing.get(e.from)!.push(e.to);
      incoming.get(e.to)!.add(e.from);
    }
  }

  // Topological sort to assign layers
  const layers = new Map<string, number>();
  const queue: string[] = [];

  // Nodes with no incoming edges go to layer 0
  for (const n of result) {
    if (incoming.get(n.key)!.size === 0) {
      layers.set(n.key, 0);
      queue.push(n.key);
    }
  }

  while (queue.length > 0) {
    const current = queue.shift()!;
    const currentLayer = layers.get(current)!;
    for (const next of outgoing.get(current) ?? []) {
      const existingLayer = layers.get(next);
      if (existingLayer === undefined || existingLayer < currentLayer + 1) {
        layers.set(next, currentLayer + 1);
      }
      // Check if all predecessors are resolved
      const preds = incoming.get(next)!;
      let allResolved = true;
      for (const pred of preds) {
        if (!layers.has(pred)) { allResolved = false; break; }
      }
      if (allResolved && !queue.includes(next)) {
        queue.push(next);
      }
    }
  }

  // Handle orphan nodes not reached by topological sort
  for (const n of result) {
    if (!layers.has(n.key)) {
      layers.set(n.key, 0);
    }
  }

  // Group by layer
  const layerGroups = new Map<number, string[]>();
  for (const [key, layer] of layers) {
    if (!layerGroups.has(layer)) layerGroups.set(layer, []);
    layerGroups.get(layer)!.push(key);
  }

  const maxLayer = Math.max(0, ...layerGroups.keys());
  const margin = 60;
  const usableWidth = width - margin * 2;
  const usableHeight = height - margin * 2;

  for (const [layer, keys] of layerGroups) {
    const x = maxLayer === 0 ? width / 2 : margin + (layer / maxLayer) * usableWidth;
    const spacing = usableHeight / (keys.length + 1);
    keys.forEach((key, i) => {
      const node = nodeMap.get(key);
      if (node) {
        node.x = x;
        node.y = margin + spacing * (i + 1);
      }
    });
  }

  return result;
}

// ---------------------------------------------------------------------------
// Build dependency edges between templates
// ---------------------------------------------------------------------------

// Maps entity_type -> template keys that CREATE that entity type (approximation by category/name)
const ENTITY_PRODUCERS: Record<string, string[]> = {};

function buildDependencyGraph(
  templates: TemplateDefinition[],
  entityTypeCounts: Map<string, number>,
  completedTemplateKeys: Set<string>,
): { nodes: TemplateNode[]; edges: DepEdge[] } {
  // Build a map of entity_type -> producing templates.
  // Heuristic: templates with no prerequisites for an entity type, or
  // foundational templates, are producers. We approximate by looking at
  // templates that have no prerequisites as producers for their category's
  // typical entity types. For a more precise approach, we use a known mapping.
  const entityTypeProducers = new Map<string, string[]>();

  // Known producers based on template analysis
  const knownProducers: Record<string, string[]> = {
    metric: ['analytics-metric-tree', 'analytics-measurement-framework-kpi-tree'],
    experiment: ['analytics-experiment-plan'],
    person: ['mkt-icp-definition', 'strategy-ICP-JTBD'],
    campaign: ['mkt-icp-definition'],
    audience: ['mkt-icp-definition', 'strategy-segmentation-targeting'],
    decision: ['org-decision-log', 'strategy-positioning-category-narrative'],
    note: ['org-knowledge-capture'],
    session: ['org-meeting-debrief'],
    project: ['org-project-charter'],
    spec: ['dev-requirements-to-spec'],
    budget: ['finance-budget-forecast', 'ops-marketing-planning-budgeting'],
    channel: ['distribution-channel-mix-budget'],
    competitor: ['mkt-competitive-intel', 'strategy-competitive-intelligence'],
  };

  for (const [entType, producers] of Object.entries(knownProducers)) {
    entityTypeProducers.set(entType, producers.filter((p) =>
      templates.some((t) => t.key === p),
    ));
  }

  // Build template status
  const nodes: TemplateNode[] = templates.map((t) => {
    let status: TemplateStatus = 'locked';
    if (completedTemplateKeys.has(t.key)) {
      status = 'completed';
    } else {
      // Check if all prerequisites are met via entity counts
      const allMet = t.prerequisites.every((p) => {
        const count = entityTypeCounts.get(p.entity_type) ?? 0;
        return count >= p.min_count;
      });
      if (allMet) status = 'available';
    }
    return {
      key: t.key,
      category: t.category,
      prerequisites: t.prerequisites,
      status,
      x: 0,
      y: 0,
    };
  });

  // Build edges: for each template with prerequisites, connect it to producer templates
  const edges: DepEdge[] = [];
  const edgeSet = new Set<string>();

  for (const t of templates) {
    for (const prereq of t.prerequisites) {
      const producers = entityTypeProducers.get(prereq.entity_type) ?? [];
      for (const producer of producers) {
        if (producer === t.key) continue; // no self-loop
        const edgeKey = `${producer}->${t.key}`;
        if (!edgeSet.has(edgeKey)) {
          edgeSet.add(edgeKey);
          edges.push({ from: producer, to: t.key, entityType: prereq.entity_type });
        }
      }
    }
  }

  return { nodes, edges };
}

// ---------------------------------------------------------------------------
// Find dependency chains and critical path
// ---------------------------------------------------------------------------

interface Chain {
  templates: string[];
  length: number;
}

function findChains(
  nodes: TemplateNode[],
  edges: DepEdge[],
): { chains: Chain[]; criticalPath: string[] } {
  const outgoing = new Map<string, string[]>();
  const incoming = new Map<string, Set<string>>();

  for (const n of nodes) {
    outgoing.set(n.key, []);
    incoming.set(n.key, new Set());
  }
  for (const e of edges) {
    if (outgoing.has(e.from)) outgoing.get(e.from)!.push(e.to);
    if (incoming.has(e.to)) incoming.get(e.to)!.add(e.from);
  }

  // Find all roots (no incoming)
  const roots = nodes.filter((n) => incoming.get(n.key)!.size === 0).map((n) => n.key);

  // DFS to find all paths
  const chains: Chain[] = [];

  function dfs(current: string, path: string[]) {
    const nexts = outgoing.get(current) ?? [];
    if (nexts.length === 0) {
      if (path.length > 1) {
        chains.push({ templates: [...path], length: path.length });
      }
      return;
    }
    for (const next of nexts) {
      if (!path.includes(next)) {
        path.push(next);
        dfs(next, path);
        path.pop();
      }
    }
  }

  for (const root of roots) {
    dfs(root, [root]);
  }

  // Sort by length descending
  chains.sort((a, b) => b.length - a.length);

  // Critical path is the longest
  const criticalPath = chains.length > 0 ? chains[0].templates : [];

  // Deduplicate chains: only keep unique ones (by sorted template set)
  const seen = new Set<string>();
  const unique: Chain[] = [];
  for (const chain of chains) {
    const key = chain.templates.join('->');
    if (!seen.has(key)) {
      seen.add(key);
      unique.push(chain);
    }
  }

  return { chains: unique.slice(0, 30), criticalPath };
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

export default function WorkflowMap() {
  const [templates, setTemplates] = useState<TemplateDefinition[]>([]);
  const [runs, setRuns] = useState<Run[]>([]);
  const [entities, setEntities] = useState<Entity[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState('');
  const [subView, setSubView] = useState<SubView>('graph');
  const [categoryFilter, setCategoryFilter] = useState('');
  const [selectedTemplate, setSelectedTemplate] = useState<string | null>(null);
  const [selectedRun, setSelectedRun] = useState<Run | null>(null);
  const [runEntities, setRunEntities] = useState<Entity[]>([]);
  const [prerequisiteResults, setPrerequisiteResults] = useState<PrerequisiteResult[]>([]);
  const [tooltip, setTooltip] = useState<{ x: number; y: number; node: TemplateNode } | null>(null);

  const svgRef = useRef<SVGSVGElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);

  // -----------------------------------------------------------------------
  // Data fetching
  // -----------------------------------------------------------------------

  const fetchData = useCallback(async () => {
    setLoading(true);
    setError('');
    try {
      const [tpls, runList, entityList] = await Promise.all([
        listTemplates(),
        listRuns().catch(() => [] as Run[]),
        listEntities(),
      ]);
      setTemplates(tpls);
      setRuns(runList);
      setEntities(entityList);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    fetchData();
  }, [fetchData]);

  // -----------------------------------------------------------------------
  // Derived data
  // -----------------------------------------------------------------------

  const entityTypeCounts = useMemo(() => {
    const counts = new Map<string, number>();
    for (const e of entities) {
      counts.set(e.entity_type, (counts.get(e.entity_type) ?? 0) + 1);
    }
    return counts;
  }, [entities]);

  const completedTemplateKeys = useMemo(() => {
    const keys = new Set<string>();
    for (const r of runs) {
      if (r.status === 'applied') keys.add(r.template_key);
    }
    return keys;
  }, [runs]);

  const filteredTemplates = useMemo(() => {
    if (!categoryFilter) return templates;
    return templates.filter((t) => t.category === categoryFilter);
  }, [templates, categoryFilter]);

  const { nodes: graphNodes, edges: graphEdges } = useMemo(
    () => buildDependencyGraph(filteredTemplates, entityTypeCounts, completedTemplateKeys),
    [filteredTemplates, entityTypeCounts, completedTemplateKeys],
  );

  const { chains, criticalPath } = useMemo(
    () => findChains(graphNodes, graphEdges),
    [graphNodes, graphEdges],
  );

  const criticalPathSet = useMemo(() => new Set(criticalPath), [criticalPath]);

  // Layout
  const [layoutNodes, setLayoutNodes] = useState<TemplateNode[]>([]);

  useEffect(() => {
    if (graphNodes.length === 0) {
      setLayoutNodes([]);
      return;
    }
    const rect = containerRef.current?.getBoundingClientRect();
    const width = rect?.width ?? 900;
    const height = rect?.height ?? 500;
    const positioned = computeLayeredLayout(graphNodes, graphEdges, width, height);
    setLayoutNodes(positioned);
  }, [graphNodes, graphEdges]);

  // Categories for filter
  const categories = useMemo(
    () => [...new Set(templates.map((t) => t.category))].sort(),
    [templates],
  );

  // -----------------------------------------------------------------------
  // Template selection
  // -----------------------------------------------------------------------

  const handleSelectTemplate = useCallback(async (key: string) => {
    setSelectedTemplate(key);
    setSelectedRun(null);
    setRunEntities([]);
    try {
      const results = await checkPrerequisites(key);
      setPrerequisiteResults(results);
    } catch {
      setPrerequisiteResults([]);
    }
  }, []);

  // -----------------------------------------------------------------------
  // Run selection: find entities created by this run
  // -----------------------------------------------------------------------

  const handleSelectRun = useCallback((run: Run) => {
    setSelectedRun(run);
    setSelectedTemplate(run.template_key);
    // Find entities with this provenance_run_id
    const created = entities.filter((e) => e.provenance_run_id === run.run_id);
    setRunEntities(created);
  }, [entities]);

  // -----------------------------------------------------------------------
  // Recommended next template
  // -----------------------------------------------------------------------

  const nextRecommended = useMemo(() => {
    // Find the first template on the critical path that is "available" but not completed
    for (const key of criticalPath) {
      const node = graphNodes.find((n) => n.key === key);
      if (node && node.status === 'available') return key;
    }
    // Fallback: any available template
    return graphNodes.find((n) => n.status === 'available')?.key ?? null;
  }, [criticalPath, graphNodes]);

  // -----------------------------------------------------------------------
  // Selected template data
  // -----------------------------------------------------------------------

  const selectedTemplateDef = useMemo(
    () => templates.find((t) => t.key === selectedTemplate),
    [templates, selectedTemplate],
  );

  const selectedNode = useMemo(
    () => graphNodes.find((n) => n.key === selectedTemplate),
    [graphNodes, selectedTemplate],
  );

  // Node map for edges
  const nodeMap = useMemo(
    () => new Map(layoutNodes.map((n) => [n.key, n])),
    [layoutNodes],
  );

  // -----------------------------------------------------------------------
  // SVG event handlers
  // -----------------------------------------------------------------------

  function handleNodeHover(node: TemplateNode, e: React.MouseEvent) {
    const rect = containerRef.current?.getBoundingClientRect();
    if (!rect) return;
    setTooltip({
      x: e.clientX - rect.left + 12,
      y: e.clientY - rect.top - 10,
      node,
    });
  }

  function getNodeFill(node: TemplateNode): string {
    if (node.status === 'completed') return '#22c55e';
    if (node.status === 'available') return getCategoryColor(node.category);
    return '#4b5563'; // gray for locked
  }

  function getNodeOpacity(node: TemplateNode): number {
    if (node.status === 'locked') return 0.5;
    return 0.9;
  }

  function getNodeStroke(node: TemplateNode): string {
    if (selectedTemplate === node.key) return '#fff';
    if (criticalPathSet.has(node.key)) return '#fbbf24';
    return 'transparent';
  }

  // Short label for template
  function shortLabel(key: string): string {
    // Remove category prefix
    const parts = key.split('-');
    if (parts.length <= 2) return key;
    return parts.slice(1).join('-');
  }

  function truncate(s: string, max: number): string {
    return s.length > max ? s.slice(0, max - 2) + '..' : s;
  }

  // -----------------------------------------------------------------------
  // Render: Dependency Graph
  // -----------------------------------------------------------------------

  function renderGraph() {
    return (
      <div ref={containerRef} style={styles.svgContainer}>
        {layoutNodes.length === 0 && !loading && (
          <div style={styles.emptyState}>
            <div style={{ fontSize: '2rem' }}>&#9673;</div>
            <div>No templates found for this filter</div>
          </div>
        )}

        {loading && (
          <div style={styles.emptyState}>
            <div>Loading workflow data...</div>
          </div>
        )}

        {layoutNodes.length > 0 && (
          <svg
            ref={svgRef}
            width="100%"
            height="100%"
            style={{ cursor: 'default' }}
            onMouseLeave={() => setTooltip(null)}
          >
            <defs>
              <marker
                id="wf-arrowhead"
                markerWidth="8"
                markerHeight="6"
                refX="8"
                refY="3"
                orient="auto"
              >
                <polygon points="0 0, 8 3, 0 6" fill="rgba(255,255,255,0.3)" />
              </marker>
              <marker
                id="wf-arrowhead-critical"
                markerWidth="8"
                markerHeight="6"
                refX="8"
                refY="3"
                orient="auto"
              >
                <polygon points="0 0, 8 3, 0 6" fill="#fbbf24" />
              </marker>
            </defs>

            {/* Edges */}
            {graphEdges.map((edge, i) => {
              const from = nodeMap.get(edge.from);
              const to = nodeMap.get(edge.to);
              if (!from || !to) return null;

              const dx = to.x - from.x;
              const dy = to.y - from.y;
              const dist = Math.sqrt(dx * dx + dy * dy);
              if (dist < 1) return null;

              const nodeRadius = 20;
              const x1 = from.x + (dx / dist) * nodeRadius;
              const y1 = from.y + (dy / dist) * nodeRadius;
              const x2 = to.x - (dx / dist) * (nodeRadius + 8);
              const y2 = to.y - (dy / dist) * (nodeRadius + 8);

              const isCritical = criticalPathSet.has(edge.from) && criticalPathSet.has(edge.to);

              return (
                <g key={`edge-${i}`}>
                  <line
                    x1={x1}
                    y1={y1}
                    x2={x2}
                    y2={y2}
                    stroke={isCritical ? 'rgba(251,191,36,0.5)' : 'rgba(255,255,255,0.15)'}
                    strokeWidth={isCritical ? 2 : 1.5}
                    markerEnd={isCritical ? 'url(#wf-arrowhead-critical)' : 'url(#wf-arrowhead)'}
                    strokeDasharray={isCritical ? undefined : '4,3'}
                  />
                  <text
                    x={(from.x + to.x) / 2}
                    y={(from.y + to.y) / 2 - 6}
                    textAnchor="middle"
                    fill="rgba(255,255,255,0.3)"
                    fontSize="8"
                    fontFamily="system-ui"
                  >
                    {edge.entityType}
                  </text>
                </g>
              );
            })}

            {/* Nodes */}
            {layoutNodes.map((node) => {
              const radius = 18;
              const fill = getNodeFill(node);
              const opacity = getNodeOpacity(node);
              const stroke = getNodeStroke(node);

              return (
                <g
                  key={node.key}
                  style={{ cursor: 'pointer' }}
                  onMouseEnter={(e) => handleNodeHover(node, e)}
                  onMouseLeave={() => setTooltip(null)}
                  onClick={() => handleSelectTemplate(node.key)}
                >
                  {/* Selection/critical ring */}
                  {stroke !== 'transparent' && (
                    <circle
                      cx={node.x}
                      cy={node.y}
                      r={radius + 3}
                      fill="none"
                      stroke={stroke}
                      strokeWidth={2}
                      opacity={0.6}
                    />
                  )}
                  {/* Node circle */}
                  <circle
                    cx={node.x}
                    cy={node.y}
                    r={radius}
                    fill={fill}
                    opacity={opacity}
                  />
                  {/* Status indicator */}
                  {node.status === 'completed' && (
                    <text
                      x={node.x}
                      y={node.y + 4}
                      textAnchor="middle"
                      fill="#fff"
                      fontSize="12"
                      fontFamily="system-ui"
                      fontWeight={700}
                    >
                      &#10003;
                    </text>
                  )}
                  {node.status === 'locked' && (
                    <text
                      x={node.x}
                      y={node.y + 4}
                      textAnchor="middle"
                      fill="rgba(255,255,255,0.5)"
                      fontSize="10"
                      fontFamily="system-ui"
                    >
                      &#9679;
                    </text>
                  )}
                  {node.status === 'available' && (
                    <text
                      x={node.x}
                      y={node.y + 4}
                      textAnchor="middle"
                      fill="#fff"
                      fontSize="10"
                      fontFamily="system-ui"
                      fontWeight={600}
                    >
                      {node.category.slice(0, 3).toUpperCase()}
                    </text>
                  )}
                  {/* Label */}
                  <text
                    x={node.x}
                    y={node.y + radius + 12}
                    textAnchor="middle"
                    fill="rgba(255,255,255,0.75)"
                    fontSize="9"
                    fontFamily="system-ui"
                  >
                    {truncate(shortLabel(node.key), 22)}
                  </text>
                </g>
              );
            })}
          </svg>
        )}

        {/* Tooltip */}
        {tooltip && (
          <div style={{ ...styles.tooltip, left: tooltip.x, top: tooltip.y }}>
            <div style={{ fontWeight: 600, marginBottom: '0.2rem' }}>{tooltip.node.key}</div>
            <div style={{ opacity: 0.6, fontSize: '0.75rem' }}>
              <span style={styles.badge(getCategoryColor(tooltip.node.category))}>
                {tooltip.node.category}
              </span>
              {' '}
              <span style={styles.statusBadge(
                tooltip.node.status === 'completed' ? '#4ade80'
                  : tooltip.node.status === 'available' ? '#646cff'
                    : '#6b7280',
              )}>
                {tooltip.node.status}
              </span>
            </div>
            {tooltip.node.prerequisites.length > 0 && (
              <div style={{ marginTop: '0.3rem', fontSize: '0.7rem', opacity: 0.5 }}>
                Requires: {tooltip.node.prerequisites.map((p) =>
                  `${p.min_count}x ${p.entity_type}`).join(', ')}
              </div>
            )}
            <div style={{ opacity: 0.4, fontSize: '0.7rem', marginTop: '0.15rem' }}>
              Click to see details
            </div>
          </div>
        )}
      </div>
    );
  }

  // -----------------------------------------------------------------------
  // Render: Chain View
  // -----------------------------------------------------------------------

  function renderChains() {
    const nodeStatusMap = new Map(graphNodes.map((n) => [n.key, n.status]));

    return (
      <div style={{ flex: 1, overflow: 'auto', display: 'flex', flexDirection: 'column' as const, gap: '0.75rem' }}>
        {/* Critical path highlight */}
        {criticalPath.length > 0 && (
          <div style={styles.section}>
            <div style={{ ...styles.sectionTitle, color: '#fbbf24' }}>
              Critical Path ({criticalPath.length} templates)
            </div>
            <div style={{ padding: '0.5rem 0.75rem' }}>
              {criticalPath.map((key, i) => {
                const status = nodeStatusMap.get(key) ?? 'locked';
                const color = status === 'completed' ? '#22c55e'
                  : status === 'available' ? '#646cff' : '#4b5563';
                return (
                  <span
                    key={key}
                    style={{
                      display: 'inline-flex',
                      alignItems: 'center',
                      gap: '0.25rem',
                      cursor: 'pointer',
                    }}
                    onClick={() => handleSelectTemplate(key)}
                  >
                    <span style={{
                      ...styles.badge(color),
                      fontSize: '0.65rem',
                      padding: '0.15rem 0.4rem',
                    }}>
                      {truncate(shortLabel(key), 20)}
                    </span>
                    {i < criticalPath.length - 1 && (
                      <span style={{ opacity: 0.3, margin: '0 0.15rem', fontSize: '0.7rem' }}>
                        &#8594;
                      </span>
                    )}
                  </span>
                );
              })}
            </div>
          </div>
        )}

        {/* All chains */}
        <div style={styles.section}>
          <div style={styles.sectionTitle}>
            Template Chains ({chains.length} found)
          </div>
          {chains.length === 0 && (
            <div style={{ padding: '1rem', textAlign: 'center', opacity: 0.4, fontSize: '0.85rem' }}>
              No dependency chains found for this filter
            </div>
          )}
          {chains.map((chain, ci) => {
            const isCritical = ci === 0 && chain.templates.length === criticalPath.length
              && chain.templates.every((t, i) => t === criticalPath[i]);
            return (
              <div
                key={ci}
                style={{
                  ...styles.chainRow,
                  background: isCritical ? 'rgba(251,191,36,0.05)' : 'transparent',
                  flexWrap: 'wrap' as const,
                }}
              >
                <span style={{
                  fontSize: '0.65rem',
                  opacity: 0.4,
                  minWidth: '3rem',
                }}>
                  {chain.length} steps
                </span>
                {chain.templates.map((key, i) => {
                  const status = nodeStatusMap.get(key) ?? 'locked';
                  const color = status === 'completed' ? '#22c55e'
                    : status === 'available' ? '#646cff' : '#4b5563';
                  return (
                    <span
                      key={key}
                      style={{ display: 'inline-flex', alignItems: 'center', gap: '0.2rem', cursor: 'pointer' }}
                      onClick={() => handleSelectTemplate(key)}
                    >
                      <span style={{
                        ...styles.badge(color),
                        fontSize: '0.6rem',
                        padding: '0.1rem 0.35rem',
                      }}>
                        {truncate(shortLabel(key), 18)}
                      </span>
                      {i < chain.templates.length - 1 && (
                        <span style={{ opacity: 0.25, fontSize: '0.65rem' }}>&#8594;</span>
                      )}
                    </span>
                  );
                })}
              </div>
            );
          })}
        </div>
      </div>
    );
  }

  // -----------------------------------------------------------------------
  // Render: Run Timeline
  // -----------------------------------------------------------------------

  function renderTimeline() {
    const filteredRuns = categoryFilter
      ? runs.filter((r) => r.template_category === categoryFilter)
      : runs;

    return (
      <div style={{ flex: 1, overflow: 'auto' }}>
        <div style={styles.section}>
          <div style={styles.sectionTitle}>
            Run History ({filteredRuns.length} runs)
          </div>
          {filteredRuns.length === 0 && (
            <div style={{ padding: '1.5rem', textAlign: 'center', opacity: 0.4, fontSize: '0.85rem' }}>
              No template runs found
            </div>
          )}
          {filteredRuns.map((run) => {
            const isSelected = selectedRun?.run_id === run.run_id;
            const statusColor = STATUS_COLORS[run.status] ?? '#888';
            const catColor = getCategoryColor(run.template_category);
            // Count entities created by this run
            const createdCount = entities.filter(
              (e) => e.provenance_run_id === run.run_id,
            ).length;

            return (
              <div
                key={run.run_id}
                style={{
                  ...styles.runRow,
                  background: isSelected ? 'rgba(100,108,255,0.1)' : 'transparent',
                  cursor: 'pointer',
                }}
                onClick={() => handleSelectRun(run)}
              >
                <div style={{ display: 'flex', alignItems: 'center', gap: '0.5rem', marginBottom: '0.25rem' }}>
                  <span style={styles.badge(catColor)}>
                    {run.template_category}
                  </span>
                  <span style={{ fontWeight: 500, fontSize: '0.85rem' }}>
                    {run.template_key}
                  </span>
                  <span style={{ marginLeft: 'auto', ...styles.statusBadge(statusColor) }}>
                    {run.status}
                  </span>
                </div>
                <div style={{ display: 'flex', gap: '1rem', opacity: 0.5, fontSize: '0.75rem' }}>
                  <span>{new Date(run.created_at).toLocaleString()}</span>
                  <span>v{run.template_version}</span>
                  <span>{createdCount} entities created</span>
                  <code style={{ fontFamily: 'monospace', fontSize: '0.7rem' }}>
                    {run.run_id.slice(0, 8)}...
                  </code>
                </div>
              </div>
            );
          })}
        </div>
      </div>
    );
  }

  // -----------------------------------------------------------------------
  // Render: Side panel (template details / run details)
  // -----------------------------------------------------------------------

  function renderSidePanel() {
    return (
      <div style={styles.sidePanel}>
        {/* Template details */}
        {selectedTemplateDef && (
          <div style={{ marginBottom: '1rem' }}>
            <h3 style={{ margin: '0 0 0.5rem', fontSize: '1rem', fontWeight: 600 }}>
              {selectedTemplateDef.key}
            </h3>

            <div style={styles.detailRow}>
              <span style={styles.detailLabel}>Category</span>
              <span style={styles.badge(getCategoryColor(selectedTemplateDef.category))}>
                {selectedTemplateDef.category}
              </span>
            </div>

            <div style={styles.detailRow}>
              <span style={styles.detailLabel}>Version</span>
              <span style={styles.detailValue}>{selectedTemplateDef.version}</span>
            </div>

            <div style={styles.detailRow}>
              <span style={styles.detailLabel}>Status</span>
              <span style={styles.statusBadge(
                selectedNode?.status === 'completed' ? '#4ade80'
                  : selectedNode?.status === 'available' ? '#646cff'
                    : '#6b7280',
              )}>
                {selectedNode?.status ?? 'unknown'}
              </span>
            </div>

            {/* Prerequisites */}
            <div style={{ marginTop: '0.75rem' }}>
              <div style={{
                fontSize: '0.75rem',
                fontWeight: 600,
                textTransform: 'uppercase' as const,
                opacity: 0.5,
                marginBottom: '0.3rem',
              }}>
                Prerequisites
              </div>
              {selectedTemplateDef.prerequisites.length === 0 ? (
                <div style={{ fontSize: '0.8rem', opacity: 0.4 }}>
                  None (foundational template)
                </div>
              ) : (
                <div style={{ display: 'flex', flexDirection: 'column' as const, gap: '0.2rem' }}>
                  {prerequisiteResults.length > 0 ? (
                    prerequisiteResults.map((pr, i) => (
                      <div key={i} style={{ fontSize: '0.8rem', display: 'flex', gap: '0.3rem', alignItems: 'center' }}>
                        <span style={{ color: pr.satisfied ? '#4ade80' : '#ef4444' }}>
                          {pr.satisfied ? '\u2713' : '\u2717'}
                        </span>
                        <span>{selectedTemplateDef.prerequisites[i]?.entity_type}: {selectedTemplateDef.prerequisites[i]?.min_count} required</span>
                        {pr.message && <span style={{ opacity: 0.5 }}>({pr.message})</span>}
                      </div>
                    ))
                  ) : (
                    selectedTemplateDef.prerequisites.map((p, i) => (
                      <div key={i} style={{ fontSize: '0.8rem' }}>
                        {p.min_count}x {p.entity_type}
                      </div>
                    ))
                  )}
                </div>
              )}
            </div>

            {/* Template runs */}
            <div style={{ marginTop: '0.75rem' }}>
              <div style={{
                fontSize: '0.75rem',
                fontWeight: 600,
                textTransform: 'uppercase' as const,
                opacity: 0.5,
                marginBottom: '0.3rem',
              }}>
                Runs ({runs.filter((r) => r.template_key === selectedTemplateDef.key).length})
              </div>
              {runs
                .filter((r) => r.template_key === selectedTemplateDef.key)
                .slice(0, 5)
                .map((run) => {
                  const statusColor = STATUS_COLORS[run.status] ?? '#888';
                  return (
                    <div
                      key={run.run_id}
                      style={{
                        fontSize: '0.75rem',
                        padding: '0.25rem 0',
                        cursor: 'pointer',
                        opacity: selectedRun?.run_id === run.run_id ? 1 : 0.7,
                      }}
                      onClick={() => handleSelectRun(run)}
                    >
                      <span style={styles.statusBadge(statusColor)}>
                        {run.status}
                      </span>
                      {' '}
                      <span style={{ opacity: 0.5 }}>
                        {new Date(run.created_at).toLocaleDateString()}
                      </span>
                    </div>
                  );
                })}
            </div>
          </div>
        )}

        {/* Run details */}
        {selectedRun && (
          <div style={{
            borderTop: selectedTemplateDef ? '1px solid rgba(255,255,255,0.06)' : 'none',
            paddingTop: selectedTemplateDef ? '0.75rem' : 0,
          }}>
            <h4 style={{ margin: '0 0 0.5rem', fontSize: '0.9rem', fontWeight: 600 }}>
              Run Details
            </h4>

            <div style={styles.detailRow}>
              <span style={styles.detailLabel}>Run ID</span>
              <code style={{ ...styles.detailValue, fontSize: '0.75rem', fontFamily: 'monospace' }}>
                {selectedRun.run_id}
              </code>
            </div>
            <div style={styles.detailRow}>
              <span style={styles.detailLabel}>Template</span>
              <span style={styles.detailValue}>{selectedRun.template_key}</span>
            </div>
            <div style={styles.detailRow}>
              <span style={styles.detailLabel}>Status</span>
              <span style={styles.statusBadge(STATUS_COLORS[selectedRun.status] ?? '#888')}>
                {selectedRun.status}
              </span>
            </div>
            <div style={styles.detailRow}>
              <span style={styles.detailLabel}>Time</span>
              <span style={styles.detailValue}>
                {new Date(selectedRun.created_at).toLocaleString()}
              </span>
            </div>

            {/* Entities created by this run */}
            <div style={{ marginTop: '0.75rem' }}>
              <div style={{
                fontSize: '0.75rem',
                fontWeight: 600,
                textTransform: 'uppercase' as const,
                opacity: 0.5,
                marginBottom: '0.3rem',
              }}>
                Entities Created ({runEntities.length})
              </div>
              {runEntities.length === 0 ? (
                <div style={{ fontSize: '0.8rem', opacity: 0.4 }}>
                  No entities linked to this run
                </div>
              ) : (
                <div style={{ display: 'flex', flexDirection: 'column' as const, gap: '0.2rem' }}>
                  {runEntities.map((e) => (
                    <div key={e.id} style={{ fontSize: '0.8rem', display: 'flex', gap: '0.3rem', alignItems: 'center' }}>
                      <span style={styles.badge(getCategoryColor(e.entity_type))}>
                        {e.entity_type}
                      </span>
                      <span style={{ overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' as const }}>
                        {e.title}
                      </span>
                    </div>
                  ))}
                </div>
              )}
            </div>
          </div>
        )}

        {/* Empty state */}
        {!selectedTemplateDef && !selectedRun && (
          <div style={{ opacity: 0.4, fontSize: '0.85rem', textAlign: 'center', padding: '2rem 0' }}>
            Select a template or run to see details
          </div>
        )}

        {/* Run next button */}
        {nextRecommended && (
          <div style={{
            marginTop: '1rem',
            paddingTop: '0.75rem',
            borderTop: '1px solid rgba(255,255,255,0.06)',
          }}>
            <div style={{
              fontSize: '0.75rem',
              fontWeight: 600,
              textTransform: 'uppercase' as const,
              opacity: 0.5,
              marginBottom: '0.3rem',
            }}>
              Recommended Next
            </div>
            <button
              style={{
                ...styles.button,
                width: '100%',
                textAlign: 'center' as const,
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                gap: '0.5rem',
              }}
              onClick={() => handleSelectTemplate(nextRecommended)}
            >
              <span>&#9654;</span>
              <span>{truncate(nextRecommended, 30)}</span>
            </button>
          </div>
        )}
      </div>
    );
  }

  // -----------------------------------------------------------------------
  // Main render
  // -----------------------------------------------------------------------

  // Visible categories in the current graph for legend
  const visibleCategories = useMemo(
    () => [...new Set(layoutNodes.map((n) => n.category))].sort(),
    [layoutNodes],
  );

  return (
    <div style={styles.container}>
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
        <h2 style={styles.heading}>Workflow Map</h2>
        <span style={{ fontSize: '0.8rem', opacity: 0.5 }}>
          {templates.length} templates | {runs.length} runs
        </span>
      </div>

      {/* Toolbar */}
      <div style={styles.toolbar}>
        {/* Sub-view tabs */}
        <div style={styles.tabBar}>
          <button
            style={styles.tab(subView === 'graph')}
            onClick={() => setSubView('graph')}
          >
            Dependency Graph
          </button>
          <button
            style={styles.tab(subView === 'chains')}
            onClick={() => setSubView('chains')}
          >
            Chain View
          </button>
          <button
            style={styles.tab(subView === 'timeline')}
            onClick={() => setSubView('timeline')}
          >
            Run Timeline
          </button>
        </div>

        <select
          style={styles.select}
          value={categoryFilter}
          onChange={(e) => setCategoryFilter(e.target.value)}
        >
          <option value="">All categories</option>
          {categories.map((c) => (
            <option key={c} value={c}>{c}</option>
          ))}
        </select>

        <button
          style={{ ...styles.button, ...styles.buttonSecondary }}
          onClick={fetchData}
          disabled={loading}
        >
          {loading ? 'Loading...' : 'Refresh'}
        </button>
      </div>

      {error && <div style={styles.error}>{error}</div>}

      {/* Legend */}
      {subView === 'graph' && layoutNodes.length > 0 && (
        <div style={{ display: 'flex', gap: '1rem', flexWrap: 'wrap' as const, alignItems: 'center' }}>
          <div style={styles.legend}>
            {visibleCategories.map((c) => (
              <div key={c} style={styles.legendItem}>
                <div style={styles.legendDot(getCategoryColor(c))} />
                <span>{c}</span>
              </div>
            ))}
          </div>
          <div style={{ ...styles.legend, borderLeft: '1px solid rgba(255,255,255,0.1)', paddingLeft: '0.75rem' }}>
            <div style={styles.legendItem}>
              <div style={styles.legendDot('#22c55e')} />
              <span>completed</span>
            </div>
            <div style={styles.legendItem}>
              <div style={styles.legendDot('#646cff')} />
              <span>available</span>
            </div>
            <div style={styles.legendItem}>
              <div style={styles.legendDot('#4b5563')} />
              <span>locked</span>
            </div>
            <div style={styles.legendItem}>
              <div style={{ width: 12, height: 2, background: '#fbbf24', borderRadius: 1 }} />
              <span>critical path</span>
            </div>
          </div>
        </div>
      )}

      {/* Main content area */}
      <div style={styles.contentArea}>
        {/* Main panel */}
        <div style={styles.mainPanel}>
          {subView === 'graph' && renderGraph()}
          {subView === 'chains' && renderChains()}
          {subView === 'timeline' && renderTimeline()}
        </div>

        {/* Side panel: template/run details */}
        {renderSidePanel()}
      </div>
    </div>
  );
}

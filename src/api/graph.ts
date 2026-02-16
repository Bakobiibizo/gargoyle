import { invoke } from '@tauri-apps/api/core';

// Matches Rust GraphNode
export interface GraphNode {
  entity_id: string;
  entity_type: string;
  title: string;
  status: string | null;
}

// Matches Rust GraphEdge
export interface GraphEdge {
  relation_id: string;
  from_id: string;
  to_id: string;
  relation_type: string;
  weight: number;
  confidence: number | null;
}

// Matches Rust EntityGraph
export interface EntityGraph {
  root: GraphNode;
  nodes: GraphNode[];
  edges: GraphEdge[];
}

export async function getEntityGraph(entityId: string, depth?: number): Promise<EntityGraph> {
  return invoke('get_entity_graph', { entityId, depth });
}

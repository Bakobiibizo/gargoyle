// Entity types
export interface Entity {
  id: string;
  entity_type: string;
  category: string | null;
  title: string;
  body_md: string;
  status: string | null;
  priority: number | null;
  due_at: string | null;
  created_at: string;
  updated_at: string;
  source: Source;
  canonical_fields: Record<string, unknown>;
  schema_version: number;
  deleted_at: string | null;
  provenance_run_id: string | null;
}

export type Source = 'manual' | 'clipboard' | 'web' | 'import' | 'agent' | 'template' | 'bootstrap';

export interface Relation {
  id: string;
  from_id: string;
  to_id: string;
  relation_type: string;
  weight: number;
  confidence: number | null;
  provenance_run_id: string | null;
  created_at: string;
}

export interface Run {
  run_id: string;
  template_key: string;
  template_version: string;
  template_category: string;
  inputs_snapshot: Record<string, unknown>;
  outputs_snapshot: Record<string, unknown>;
  patch_set: unknown;
  status: RunStatus;
  created_at: string;
}

export type RunStatus = 'pending' | 'applied' | 'rejected' | 'partial';

export interface Claim {
  claim_id: string;
  subject: string;
  predicate: string;
  object: string;
  confidence: number;
  evidence_entity_id: string;
  provenance_run_id: string | null;
  promoted_to_entity_id: string | null;
  created_at: string;
}

export interface Artifact {
  artifact_id: string;
  entity_id: string;
  kind: ArtifactKind;
  uri_or_path: string;
  hash: string | null;
  mime: string | null;
  created_at: string;
}

export type ArtifactKind = 'attachment' | 'link' | 'export' | 'rendered_doc';

export interface Embedding {
  embedding_id: string;
  entity_id: string;
  model: string;
  vector: number[];
  dimensions: number;
  created_at: string;
}

export interface OperationalContext {
  context_id: string;
  context_key: string;
  context_value: unknown;
  updated_at: string;
  updated_by_run_id: string | null;
}

export interface DedupSuggestion {
  suggestion_id: string;
  new_entity_id: string;
  existing_entity_id: string;
  detection_method: DetectionMethod;
  confidence: number;
  status: string;
  created_at: string;
}

export type DetectionMethod = 'exact_title' | 'fuzzy_title' | 'embedding_proximity';

export interface CustomRelationType {
  type_key: string;
  description: string;
  expected_from_types: string[] | null;
  expected_to_types: string[] | null;
  proposed_by_run_id: string | null;
  approved_at: string;
}

// Patch types
export type PatchOp =
  | { op_type: 'create_entity'; payload: CreateEntityPayload }
  | { op_type: 'update_entity'; payload: UpdateEntityPayload }
  | { op_type: 'create_relation'; payload: CreateRelationPayload }
  | { op_type: 'create_claim'; payload: CreateClaimPayload };

export interface CreateEntityPayload {
  entity_type: string;
  title: string;
  source: string;
  canonical_fields: Record<string, unknown>;
  body_md?: string;
  status?: string;
  category?: string;
  priority?: number;
}

export interface UpdateEntityPayload {
  entity_id: string;
  expected_updated_at: string;
  title?: string;
  body_md?: string;
  status?: string;
  canonical_fields?: Record<string, unknown>;
  category?: string;
  priority?: number;
  reason?: string;
}

export interface CreateRelationPayload {
  from_id: string;
  to_id: string;
  relation_type: string;
  weight?: number;
  confidence?: number;
  provenance_run_id?: string;
}

export interface CreateClaimPayload {
  subject: string;
  predicate: string;
  object: string;
  confidence: number;
  evidence_entity_id: string;
  provenance_run_id?: string;
}

export interface PatchSet {
  ops: PatchOp[];
  run_id?: string;
}

export interface PatchResult {
  applied: AppliedOp[];
  errors: string[];
}

export interface AppliedOp {
  op_index: number;
  entity_id?: string;
  relation_id?: string;
  claim_id?: string;
}

// Search types
export interface SearchResult {
  entity_id: string;
  title: string;
  entity_type: string;
  score: number;
}

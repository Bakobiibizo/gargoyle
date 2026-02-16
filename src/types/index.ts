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

// =============================================================================
// Entity type constants and canonical field interfaces
// =============================================================================

// All known entity types
export type EntityType =
  | 'metric'
  | 'experiment'
  | 'result'
  | 'task'
  | 'project'
  | 'decision'
  | 'person'
  | 'note'
  | 'session'
  | 'campaign'
  | 'audience'
  | 'competitor'
  | 'channel'
  | 'spec'
  | 'budget'
  | 'vendor'
  | 'playbook'
  | 'taxonomy'
  | 'backlog'
  | 'brief'
  | 'event'
  | 'policy';

// Canonical field interfaces per entity type

export interface MetricFields {
  current_value?: number;
  target_value?: number;
  trend?: 'up' | 'down' | 'flat';
  data_source?: string;
}

export interface ExperimentFields {
  hypothesis?: string;
  funnel_position?: string;
  source_experiment_id?: string;
}

export interface ResultFields {
  findings?: string;
  methodology?: string;
  confidence_level?: number;
}

export interface TaskFields {
  assignee?: string;
  effort_estimate?: string;
  project_id?: string;
  acceptance_criteria?: string;
}

export interface ProjectFields {
  owner_id?: string;
  objective?: string;
  success_criteria?: string;
  timeline?: string;
}

export interface DecisionFields {
  owner_id: string;
  decided_at?: string;
  rationale: string;
  revisit_triggers?: string;
  options_considered?: string;
}

export interface PersonFields {
  email?: string;
  role?: string;
  team?: string;
  external?: boolean;
}

export interface NoteFields {
  context?: string;
  tags?: string;
  linked_entity_id?: string;
}

export interface SessionFields {
  session_type?: 'planning' | 'review' | 'standup' | 'workshop' | 'retro';
  participants?: string;
  agenda?: string;
  outcomes?: string;
}

// Status types per entity type
export type MetricStatus = 'active' | 'paused' | 'deprecated' | 'archived';
export type ExperimentStatus = 'draft' | 'running' | 'concluded' | 'archived';
export type ResultStatus = 'draft' | 'final' | 'archived';
export type TaskStatus = 'backlog' | 'todo' | 'in_progress' | 'blocked' | 'done' | 'archived';
export type ProjectStatus = 'planning' | 'active' | 'paused' | 'completed' | 'archived';
export type DecisionStatus = 'proposed' | 'accepted' | 'deprecated' | 'superseded';
export type PersonStatus = 'active' | 'inactive' | 'archived';
export type NoteStatus = 'draft' | 'final' | 'archived';
export type SessionStatus = 'scheduled' | 'in_progress' | 'completed' | 'cancelled';

// Wave 2 entity type field interfaces

export interface CampaignFields {
  objective?: string;
  budget?: number;
  channel?: 'email' | 'paid_social' | 'paid_search' | 'organic' | 'events' | 'partnerships';
  start_date?: string;
  end_date?: string;
  target_audience_id?: string;
}

export interface AudienceFields {
  segment_criteria?: string;
  estimated_size?: number;
  icp_id?: string;
  channels?: string;
}

export interface CompetitorFields {
  website?: string;
  positioning?: string;
  strengths?: string;
  weaknesses?: string;
  market_share?: string;
}

export interface ChannelFields {
  channel_type?: 'email' | 'social' | 'search' | 'display' | 'events' | 'partnerships' | 'content' | 'referral';
  cost_model?: string;
  primary_metric_id?: string;
  budget_allocation?: number;
}

export interface SpecFields {
  spec_type?: 'technical' | 'product' | 'design' | 'process';
  version?: string;
  approval_status?: string;
  author?: string;
}

export interface BudgetFields {
  total_amount?: number;
  currency?: string;
  period?: string;
  allocated?: number;
  spent?: number;
}

export interface VendorFields {
  vendor_type?: 'agency' | 'saas' | 'contractor' | 'platform';
  contract_value?: number;
  contract_end?: string;
  primary_contact?: string;
}

export interface PlaybookFields {
  playbook_type?: 'sales' | 'marketing' | 'ops' | 'cs' | 'dev';
  trigger_conditions?: string;
  expected_outcome?: string;
  owner?: string;
}

// Wave 2 status types
export type CampaignStatus = 'planning' | 'active' | 'paused' | 'completed' | 'archived';
export type AudienceStatus = 'draft' | 'validated' | 'active' | 'archived';
export type CompetitorStatus = 'tracking' | 'dormant' | 'archived';
export type ChannelStatus = 'evaluating' | 'active' | 'scaling' | 'paused' | 'deprecated';
export type SpecStatus = 'draft' | 'review' | 'approved' | 'deprecated';
export type BudgetStatus = 'draft' | 'approved' | 'active' | 'closed';
export type VendorStatus = 'evaluating' | 'active' | 'on_hold' | 'terminated';
export type PlaybookStatus = 'draft' | 'active' | 'deprecated' | 'archived';

// Wave 3 entity type field interfaces

export interface TaxonomyFields {
  taxonomy_type?: 'category' | 'tag' | 'hierarchy';
  parent_id?: string;
  level?: number;
}

export interface BacklogFields {
  priority_score?: number;
  effort?: string;
  requester?: string;
  target_sprint?: string;
}

export interface BriefFields {
  brief_type?: 'creative' | 'campaign' | 'product' | 'event';
  deadline?: string;
  stakeholders?: string;
  deliverables?: string;
}

export interface EventFields {
  event_type?: 'conference' | 'webinar' | 'meetup' | 'workshop' | 'launch';
  venue?: string;
  start_date?: string;
  end_date?: string;
  expected_attendees?: number;
}

export interface PolicyFields {
  policy_type?: 'security' | 'hr' | 'compliance' | 'operational';
  effective_date?: string;
  review_date?: string;
  owner?: string;
}

// Wave 3 status types
export type TaxonomyStatus = 'draft' | 'active' | 'archived';
export type BacklogStatus = 'open' | 'triaged' | 'scheduled' | 'closed';
export type BriefStatus = 'draft' | 'review' | 'approved' | 'archived';
export type EventStatus = 'proposed' | 'confirmed' | 'in_progress' | 'completed' | 'cancelled';
export type PolicyStatus = 'draft' | 'active' | 'under_review' | 'deprecated';

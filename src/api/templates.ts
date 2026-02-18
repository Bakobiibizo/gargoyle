import { invoke } from '@tauri-apps/api/core';
import type { PatchResult, Run } from '../types';

// Matches Rust MaturityTier
export type MaturityTier = 'Foundational' | 'Workflow' | 'Advanced' | 'Diagnostic';

// Matches Rust TemplateDefinition
export interface TemplateDefinition {
  key: string;
  version: string;
  category: string;
  maturity_tier: MaturityTier;
  prerequisites: Prerequisite[];
  produced_entity_types: string[];
  produced_relation_types: string[];
}

export interface Prerequisite {
  entity_type: string;
  min_count: number;
  suggested_template: string | null;
  reason: string;
}

export interface PrerequisiteResult {
  satisfied: boolean;
  message: string | null;
  suggested_template: string | null;
}

// Matches Rust TemplateInput
export interface TemplateInput {
  template_key: string;
  params: Record<string, unknown>;
  force: boolean;
}

// Matches Rust ProducedEntity
export interface ProducedEntity {
  entity_id: string;
  entity_type: string;
  title: string;
  status: string | null;
}

// Matches Rust ProducedRelation
export interface ProducedRelation {
  relation_id: string;
  from_ref: string;
  to_ref: string;
  relation_type: string;
}

// Matches Rust TemplateOutput
export interface TemplateOutput {
  run_id: string;
  template_key: string;
  template_category: string;
  created_at: string;
  produced_entities: ProducedEntity[];
  produced_relations: ProducedRelation[];
  action_items: string[];
  decisions_needed: string[];
  risks: string[];
  assumptions: string[];
  open_questions: string[];
  warnings: string[];
  patch_result: PatchResult;
}

export async function listTemplates(): Promise<TemplateDefinition[]> {
  return invoke('list_templates');
}

export async function runTemplate(input: TemplateInput): Promise<TemplateOutput> {
  return invoke('run_template', { input });
}

export async function checkPrerequisites(templateKey: string): Promise<PrerequisiteResult[]> {
  return invoke('check_prerequisites', { templateKey });
}

export async function listRuns(templateKey?: string): Promise<Run[]> {
  return invoke('list_runs', { templateKey: templateKey ?? null });
}

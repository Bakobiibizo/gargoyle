import { invoke } from '@tauri-apps/api/core';
import type { PatchResult } from '../types';

// Matches Rust TemplateDefinition
export interface TemplateDefinition {
  key: string;
  version: string;
  category: string;
  prerequisites: Prerequisite[];
}

export interface Prerequisite {
  entity_type: string;
  min_count: number;
}

export interface PrerequisiteResult {
  satisfied: boolean;
  message: string | null;
}

// Matches Rust TemplateInput
export interface TemplateInput {
  template_key: string;
  params: Record<string, unknown>;
  force: boolean;
}

// Matches Rust TemplateOutput
export interface TemplateOutput {
  run_id: string;
  patch_result: PatchResult;
  warnings: string[];
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

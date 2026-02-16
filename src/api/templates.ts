import { invoke } from '@tauri-apps/api/core';
import type { PatchResult } from '../types';

export async function runTemplate(
  templateKey: string,
  inputs: Record<string, unknown>
): Promise<PatchResult> {
  return invoke('run_template', { templateKey, inputs });
}

export async function checkPrerequisites(templateKey: string): Promise<{ satisfied: boolean; missing: string[] }> {
  return invoke('check_prerequisites', { templateKey });
}

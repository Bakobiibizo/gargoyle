import { invoke } from '@tauri-apps/api/core';
import type { DedupSuggestion } from '../types';

export async function listDedupSuggestions(status?: string): Promise<DedupSuggestion[]> {
  return invoke('list_dedup_suggestions', { status });
}

export async function resolveDedupSuggestion(suggestionId: string, newStatus: string): Promise<void> {
  return invoke('resolve_dedup_suggestion', { suggestionId, newStatus });
}

export async function checkDuplicates(entityId: string): Promise<DedupSuggestion[]> {
  return invoke('check_duplicates', { entityId });
}

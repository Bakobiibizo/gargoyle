import { invoke } from '@tauri-apps/api/core';
import type { SearchResult } from '../types';

export async function searchEntities(query: string, limit?: number): Promise<SearchResult[]> {
  return invoke('search_entities', { query, limit });
}

export async function searchSimilar(query: string, limit?: number, threshold?: number): Promise<SearchResult[]> {
  return invoke('search_similar', { query, limit, threshold });
}

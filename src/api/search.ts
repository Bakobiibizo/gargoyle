import { invoke } from '@tauri-apps/api/core';
import type { SearchResult } from '../types';

export async function searchFts(query: string, limit: number = 20): Promise<SearchResult[]> {
  return invoke('search_fts', { query, limit });
}

export async function searchSimilar(query: string, limit: number = 20, threshold?: number): Promise<SearchResult[]> {
  return invoke('search_similar', { query, limit, threshold });
}

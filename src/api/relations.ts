import { invoke } from '@tauri-apps/api/core';
import type { Relation, CreateRelationPayload, PatchResult } from '../types';

export async function createRelation(payload: CreateRelationPayload): Promise<PatchResult> {
  return invoke('create_relation', { payload });
}

export async function getRelations(entityId: string): Promise<Relation[]> {
  return invoke('get_relations', { entityId });
}

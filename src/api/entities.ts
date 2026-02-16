import { invoke } from '@tauri-apps/api/core';
import type { Entity, PatchResult, CreateEntityPayload, UpdateEntityPayload } from '../types';

export async function createEntity(payload: CreateEntityPayload): Promise<PatchResult> {
  return invoke('create_entity', { payload });
}

export async function updateEntity(payload: UpdateEntityPayload): Promise<PatchResult> {
  return invoke('update_entity', { payload });
}

export async function getEntity(id: string): Promise<Entity> {
  return invoke('get_entity', { id });
}

export async function listEntities(entityType?: string): Promise<Entity[]> {
  return invoke('list_entities', { entityType });
}

export async function deleteEntity(id: string): Promise<void> {
  return invoke('delete_entity', { id });
}

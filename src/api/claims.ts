import { invoke } from '@tauri-apps/api/core';
import type { Claim } from '../types';

export async function listClaims(evidenceEntityId?: string): Promise<Claim[]> {
  return invoke('list_claims', { evidenceEntityId });
}

export async function getClaim(claimId: string): Promise<Claim> {
  return invoke('get_claim', { claimId });
}

export async function promoteClaim(claimId: string, entityType: string, source: string): Promise<string> {
  return invoke('promote_claim', { claimId, entityType, source });
}

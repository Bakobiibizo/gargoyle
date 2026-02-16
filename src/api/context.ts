import { invoke } from '@tauri-apps/api/core';
import type { OperationalContext } from '../types';

export async function getContext(key: string): Promise<OperationalContext | null> {
  return invoke('get_context', { key });
}

export async function setContext(key: string, value: unknown): Promise<void> {
  return invoke('set_context', { key, value });
}

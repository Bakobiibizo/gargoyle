import { invoke } from '@tauri-apps/api/core';

export interface ChatMessageInput {
  role: 'system' | 'user' | 'assistant';
  content: string;
}

export interface LlmChatInput {
  messages: ChatMessageInput[];
  temperature?: number;
  max_tokens?: number;
}

export interface LlmChatOutput {
  reply: string;
  model: string;
  finish_reason: string | null;
  usage: {
    prompt_tokens: number | null;
    completion_tokens: number | null;
    total_tokens: number | null;
  } | null;
}

export interface LlmStatusOutput {
  connected: boolean;
  model: string;
  base_url: string;
  error: string | null;
}

export async function llmChat(input: LlmChatInput): Promise<LlmChatOutput> {
  return invoke('llm_chat', { input });
}

export async function llmComplete(prompt: string): Promise<string> {
  return invoke('llm_complete', { prompt });
}

export async function llmStatus(): Promise<LlmStatusOutput> {
  return invoke('llm_status');
}

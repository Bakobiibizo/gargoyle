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

export interface ToolCallLog {
  tool_name: string;
  arguments: string;
  result: string;
  success: boolean;
}

export interface LlmToolChatOutput {
  reply: string;
  model: string;
  finish_reason: string | null;
  usage: {
    prompt_tokens: number | null;
    completion_tokens: number | null;
    total_tokens: number | null;
  } | null;
  tool_calls_made: ToolCallLog[];
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

export async function llmChatWithTools(input: LlmChatInput): Promise<LlmToolChatOutput> {
  return invoke('llm_chat_with_tools', { input });
}

export async function llmComplete(prompt: string): Promise<string> {
  return invoke('llm_complete', { prompt });
}

export async function llmStatus(): Promise<LlmStatusOutput> {
  return invoke('llm_status');
}

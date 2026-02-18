import { invoke } from '@tauri-apps/api/core';

export interface ChatSession {
  id: string;
  title: string;
  system_prompt: string | null;
  created_at: string;
  updated_at: string;
}

export interface ChatMessageRow {
  id: string;
  session_id: string;
  role: string;
  content: string;
  model: string | null;
  tokens: number | null;
  created_at: string;
}

export async function createChatSession(title: string, systemPrompt: string | null): Promise<ChatSession> {
  return invoke('create_chat_session', { title, systemPrompt });
}

export async function listChatSessions(): Promise<ChatSession[]> {
  return invoke('list_chat_sessions');
}

export async function getChatMessages(sessionId: string): Promise<ChatMessageRow[]> {
  return invoke('get_chat_messages', { sessionId });
}

export async function addChatMessage(
  sessionId: string,
  role: string,
  content: string,
  model: string | null,
  tokens: number | null,
): Promise<ChatMessageRow> {
  return invoke('add_chat_message', { sessionId, role, content, model, tokens });
}

export async function updateChatSessionTitle(sessionId: string, title: string): Promise<void> {
  return invoke('update_chat_session_title', { sessionId, title });
}

export async function deleteChatSession(sessionId: string): Promise<void> {
  return invoke('delete_chat_session', { sessionId });
}

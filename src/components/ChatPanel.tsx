import { useState, useEffect, useRef, useCallback } from 'react';
import { llmChatWithTools, llmStatus } from '../api/llm';
import type { ChatMessageInput, LlmStatusOutput, ToolCallLog } from '../api/llm';
import {
  createChatSession,
  listChatSessions,
  getChatMessages,
  addChatMessage,
  deleteChatSession,
} from '../api/chat';
import type { ChatSession, ChatMessageRow } from '../api/chat';

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const DEFAULT_SYSTEM_PROMPT =
  `You are a helpful assistant integrated with the Gargoyle knowledge graph.

You have tools to search, create, update, and link entities in the graph, and to run templates. Use them when the user asks you to add, find, or modify information. Always confirm what you did after taking action.

When creating entities, set source to "agent". Use appropriate entity types (task, note, project, decision, metric, experiment, budget, campaign, event, person, spec, session, playbook, policy, competitor, vendor, audience, channel, backlog, taxonomy, brief, result) and statuses.`;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

interface ChatMessage {
  id: string;
  role: 'user' | 'assistant' | 'system';
  content: string;
  timestamp: number;
  model?: string;
  tokens?: number | null;
  toolActions?: ToolCallLog[];
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function relativeTime(iso: string): string {
  const ms = Date.now() - new Date(iso).getTime();
  const secs = Math.floor(ms / 1000);
  if (secs < 60) return 'just now';
  const mins = Math.floor(secs / 60);
  if (mins < 60) return `${mins}m ago`;
  const hrs = Math.floor(mins / 60);
  if (hrs < 24) return `${hrs}h ago`;
  const days = Math.floor(hrs / 24);
  return `${days}d ago`;
}

function rowsToMessages(rows: ChatMessageRow[]): ChatMessage[] {
  return rows.map(r => ({
    id: r.id,
    role: r.role as 'user' | 'assistant' | 'system',
    content: r.content,
    timestamp: new Date(r.created_at).getTime(),
    model: r.model ?? undefined,
    tokens: r.tokens,
  }));
}

/** Format a tool action summary for display and persistence. */
function formatToolSummary(actions: ToolCallLog[]): string {
  return actions
    .map(a => {
      try {
        const args = JSON.parse(a.arguments);
        const brief = Object.values(args).slice(0, 2).map(v => typeof v === 'string' ? v : JSON.stringify(v)).join(', ');
        return `${a.success ? '\u2713' : '\u2717'} ${a.tool_name}(${brief})`;
      } catch {
        return `${a.success ? '\u2713' : '\u2717'} ${a.tool_name}`;
      }
    })
    .join('\n');
}

// ---------------------------------------------------------------------------
// Tool Actions Display
// ---------------------------------------------------------------------------

function ToolActionsBlock({ actions }: { actions: ToolCallLog[] }) {
  const [expanded, setExpanded] = useState(false);

  return (
    <div style={styles.toolActionsBlock}>
      <button
        style={styles.toolActionsToggle}
        onClick={() => setExpanded(prev => !prev)}
      >
        {expanded ? '\u25BC' : '\u25B6'} {actions.length} tool action{actions.length !== 1 ? 's' : ''} taken
      </button>
      {expanded && (
        <div style={styles.toolActionsList}>
          {actions.map((a, i) => (
            <div key={i} style={styles.toolActionItem(a.success)}>
              <div style={styles.toolActionHeader}>
                <span style={styles.toolActionIcon}>{a.success ? '\u2713' : '\u2717'}</span>
                <span style={styles.toolActionName}>{a.tool_name}</span>
              </div>
              <div style={styles.toolActionArgs}>
                {(() => {
                  try {
                    const parsed = JSON.parse(a.arguments);
                    return Object.entries(parsed)
                      .map(([k, v]) => `${k}: ${typeof v === 'string' ? v : JSON.stringify(v)}`)
                      .join(', ');
                  } catch {
                    return a.arguments;
                  }
                })()}
              </div>
              <div style={styles.toolActionResult}>
                {(() => {
                  try {
                    const parsed = JSON.parse(a.result);
                    if (parsed.message) return parsed.message;
                    if (Array.isArray(parsed))
                      return `${parsed.length} result${parsed.length !== 1 ? 's' : ''}`;
                    return a.result.length > 120 ? a.result.slice(0, 120) + '...' : a.result;
                  } catch {
                    return a.result.length > 120 ? a.result.slice(0, 120) + '...' : a.result;
                  }
                })()}
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Styles
// ---------------------------------------------------------------------------

const styles = {
  outerContainer: {
    display: 'flex',
    height: '100%',
    overflow: 'hidden',
  },
  sidebar: {
    width: 200,
    minWidth: 200,
    borderRight: '1px solid rgba(255,255,255,0.06)',
    display: 'flex',
    flexDirection: 'column' as const,
    background: 'rgba(0,0,0,0.12)',
    overflow: 'hidden',
  },
  sidebarHeader: {
    padding: '0.6rem 0.65rem',
    borderBottom: '1px solid rgba(255,255,255,0.06)',
  },
  newChatButton: {
    width: '100%',
    padding: '0.45rem 0.6rem',
    borderRadius: 6,
    border: '1px solid rgba(255,255,255,0.15)',
    background: 'rgba(100,108,255,0.15)',
    color: 'inherit',
    fontSize: '0.8rem',
    fontWeight: 600,
    cursor: 'pointer',
    fontFamily: 'inherit',
  },
  sessionList: {
    flex: 1,
    overflow: 'auto',
    padding: '0.25rem 0',
  },
  sessionItem: (active: boolean) => ({
    display: 'flex',
    alignItems: 'center',
    gap: '0.4rem',
    padding: '0.5rem 0.65rem',
    cursor: 'pointer',
    background: active ? 'rgba(100,108,255,0.18)' : 'transparent',
    borderLeft: active ? '2px solid #646cff' : '2px solid transparent',
    fontSize: '0.8rem',
    lineHeight: 1.3,
    transition: 'background 0.1s',
  }),
  sessionTitle: {
    flex: 1,
    overflow: 'hidden',
    textOverflow: 'ellipsis',
    whiteSpace: 'nowrap' as const,
  },
  sessionTime: {
    fontSize: '0.65rem',
    opacity: 0.4,
    whiteSpace: 'nowrap' as const,
  },
  deleteBtn: {
    background: 'none',
    border: 'none',
    color: 'inherit',
    cursor: 'pointer',
    opacity: 0.3,
    fontSize: '0.75rem',
    fontFamily: 'inherit',
    padding: '0 0.2rem',
    lineHeight: 1,
  },
  chatArea: {
    flex: 1,
    display: 'flex',
    flexDirection: 'column' as const,
    overflow: 'hidden',
  },
  statusBar: {
    display: 'flex',
    alignItems: 'center',
    gap: '0.5rem',
    padding: '0.5rem 1rem',
    fontSize: '0.75rem',
    borderBottom: '1px solid rgba(255,255,255,0.06)',
    background: 'rgba(0,0,0,0.15)',
  },
  statusDot: (connected: boolean) => ({
    width: 8,
    height: 8,
    borderRadius: '50%',
    background: connected ? '#22c55e' : '#ef4444',
    flexShrink: 0,
  }),
  statusText: {
    opacity: 0.5,
    overflow: 'hidden',
    textOverflow: 'ellipsis',
    whiteSpace: 'nowrap' as const,
  },
  messages: {
    flex: 1,
    overflow: 'auto',
    padding: '1rem 1rem 0.5rem',
    display: 'flex',
    flexDirection: 'column' as const,
    gap: '0.75rem',
  },
  emptyState: {
    flex: 1,
    display: 'flex',
    flexDirection: 'column' as const,
    alignItems: 'center',
    justifyContent: 'center',
    gap: '0.75rem',
    opacity: 0.35,
    padding: '2rem',
    textAlign: 'center' as const,
  },
  emptyIcon: {
    fontSize: '2rem',
  },
  emptyText: {
    fontSize: '0.9rem',
    lineHeight: 1.5,
  },
  messageBubble: (isUser: boolean) => ({
    maxWidth: '80%',
    alignSelf: isUser ? ('flex-end' as const) : ('flex-start' as const),
    padding: '0.6rem 0.85rem',
    borderRadius: 12,
    borderTopRightRadius: isUser ? 2 : 12,
    borderTopLeftRadius: isUser ? 12 : 2,
    background: isUser ? '#646cff' : 'rgba(255,255,255,0.08)',
    color: isUser ? '#fff' : 'inherit',
    fontSize: '0.875rem',
    lineHeight: 1.55,
    whiteSpace: 'pre-wrap' as const,
    wordBreak: 'break-word' as const,
  }),
  messageMetadata: {
    fontSize: '0.65rem',
    opacity: 0.4,
    marginTop: '0.25rem',
  },
  thinkingIndicator: {
    alignSelf: 'flex-start' as const,
    padding: '0.6rem 0.85rem',
    borderRadius: 12,
    borderTopLeftRadius: 2,
    background: 'rgba(255,255,255,0.08)',
    fontSize: '0.875rem',
    display: 'flex',
    gap: '0.3rem',
    alignItems: 'center',
  },
  dot: (delay: number) => ({
    width: 6,
    height: 6,
    borderRadius: '50%',
    background: 'rgba(255,255,255,0.4)',
    animation: `pulse 1.4s ease-in-out ${delay}s infinite`,
  }),
  inputArea: {
    display: 'flex',
    gap: '0.5rem',
    padding: '0.75rem 1rem',
    borderTop: '1px solid rgba(255,255,255,0.06)',
    background: 'rgba(0,0,0,0.1)',
  },
  textarea: {
    flex: 1,
    padding: '0.55rem 0.75rem',
    borderRadius: 8,
    border: '1px solid rgba(255,255,255,0.15)',
    background: 'rgba(0,0,0,0.3)',
    color: 'inherit',
    fontSize: '0.875rem',
    fontFamily: 'inherit',
    resize: 'none' as const,
    lineHeight: 1.4,
    minHeight: '1.4em',
    maxHeight: '8rem',
    outline: 'none',
  },
  sendButton: (enabled: boolean) => ({
    padding: '0.55rem 1rem',
    borderRadius: 8,
    border: 'none',
    background: enabled ? '#646cff' : 'rgba(100,108,255,0.3)',
    color: '#fff',
    fontSize: '0.85rem',
    fontWeight: 600,
    cursor: enabled ? 'pointer' : ('default' as const),
    fontFamily: 'inherit',
    alignSelf: 'flex-end' as const,
    opacity: enabled ? 1 : 0.5,
  }),
  systemPromptToggle: {
    fontSize: '0.75rem',
    opacity: 0.5,
    cursor: 'pointer',
    background: 'none',
    border: 'none',
    color: 'inherit',
    fontFamily: 'inherit',
    padding: '0.25rem 0',
    textDecoration: 'underline',
  },
  systemPromptArea: {
    padding: '0.5rem 1rem',
    borderTop: '1px solid rgba(255,255,255,0.04)',
    background: 'rgba(0,0,0,0.08)',
  },
  systemTextarea: {
    width: '100%',
    padding: '0.45rem 0.6rem',
    borderRadius: 6,
    border: '1px solid rgba(255,255,255,0.1)',
    background: 'rgba(0,0,0,0.2)',
    color: 'inherit',
    fontSize: '0.8rem',
    fontFamily: 'inherit',
    resize: 'vertical' as const,
    lineHeight: 1.4,
    minHeight: '2.5rem',
    outline: 'none',
    boxSizing: 'border-box' as const,
  },
  systemLabel: {
    fontSize: '0.7rem',
    fontWeight: 600,
    textTransform: 'uppercase' as const,
    letterSpacing: '0.04em',
    opacity: 0.5,
    marginBottom: '0.3rem',
  },
  sidebarEmpty: {
    padding: '1rem 0.65rem',
    fontSize: '0.75rem',
    opacity: 0.35,
    textAlign: 'center' as const,
  },
  // Tool actions styles
  toolActionsBlock: {
    maxWidth: '80%',
    alignSelf: 'flex-start' as const,
    marginBottom: '-0.4rem',
  },
  toolActionsToggle: {
    background: 'rgba(100,108,255,0.12)',
    border: '1px solid rgba(100,108,255,0.25)',
    borderRadius: 8,
    color: 'rgba(200,205,255,0.85)',
    fontSize: '0.75rem',
    padding: '0.35rem 0.65rem',
    cursor: 'pointer',
    fontFamily: 'inherit',
    display: 'block',
    width: '100%',
    textAlign: 'left' as const,
  },
  toolActionsList: {
    marginTop: '0.3rem',
    padding: '0.4rem',
    background: 'rgba(0,0,0,0.2)',
    borderRadius: 8,
    border: '1px solid rgba(255,255,255,0.06)',
    display: 'flex',
    flexDirection: 'column' as const,
    gap: '0.3rem',
  },
  toolActionItem: (success: boolean) => ({
    padding: '0.35rem 0.5rem',
    borderRadius: 6,
    background: success ? 'rgba(34,197,94,0.08)' : 'rgba(239,68,68,0.08)',
    borderLeft: `2px solid ${success ? '#22c55e' : '#ef4444'}`,
    fontSize: '0.75rem',
  }),
  toolActionHeader: {
    display: 'flex',
    alignItems: 'center',
    gap: '0.35rem',
    fontWeight: 600,
  },
  toolActionIcon: {
    fontSize: '0.7rem',
  },
  toolActionName: {
    fontFamily: 'monospace',
  },
  toolActionArgs: {
    opacity: 0.6,
    fontSize: '0.7rem',
    marginTop: '0.15rem',
    overflow: 'hidden',
    textOverflow: 'ellipsis',
    whiteSpace: 'nowrap' as const,
  },
  toolActionResult: {
    opacity: 0.5,
    fontSize: '0.7rem',
    marginTop: '0.1rem',
    fontStyle: 'italic',
  },
};

// Keyframe animation for the thinking dots
const pulseKeyframes = `
@keyframes pulse {
  0%, 80%, 100% { opacity: 0.3; transform: scale(0.8); }
  40% { opacity: 1; transform: scale(1.1); }
}
`;

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

export default function ChatPanel() {
  // Session state
  const [sessions, setSessions] = useState<ChatSession[]>([]);
  const [activeSessionId, setActiveSessionId] = useState<string | null>(null);

  // Chat state
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [input, setInput] = useState('');
  const [systemPrompt, setSystemPrompt] = useState(DEFAULT_SYSTEM_PROMPT);
  const [showSystemPrompt, setShowSystemPrompt] = useState(false);
  const [loading, setLoading] = useState(false);

  // LLM status
  const [status, setStatus] = useState<LlmStatusOutput | null>(null);
  const [statusChecking, setStatusChecking] = useState(true);

  const messagesEndRef = useRef<HTMLDivElement>(null);
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const initRef = useRef(false);

  // ---- Load LLM status + sessions on mount ----
  useEffect(() => {
    llmStatus()
      .then(setStatus)
      .catch(() =>
        setStatus({ connected: false, model: '', base_url: '', error: 'Failed to check status' }),
      )
      .finally(() => setStatusChecking(false));

    // Load sessions, then load the most recent one
    listChatSessions()
      .then(list => {
        setSessions(list);
        if (list.length > 0 && !initRef.current) {
          initRef.current = true;
          const mostRecent = list[0]; // ordered by updated_at DESC
          setActiveSessionId(mostRecent.id);
          if (mostRecent.system_prompt) {
            setSystemPrompt(mostRecent.system_prompt);
          }
          getChatMessages(mostRecent.id).then(rows => {
            setMessages(rowsToMessages(rows));
          });
        } else {
          initRef.current = true;
        }
      })
      .catch(() => {
        initRef.current = true;
      });
  }, []); // eslint-disable-line react-hooks/exhaustive-deps

  // ---- Auto-scroll on new messages ----
  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [messages, loading]);

  // ---- Auto-resize textarea ----
  const handleInputChange = useCallback((e: React.ChangeEvent<HTMLTextAreaElement>) => {
    setInput(e.target.value);
    const el = e.target;
    el.style.height = 'auto';
    el.style.height = Math.min(el.scrollHeight, 128) + 'px';
  }, []);

  // ---- Refresh session list ----
  const refreshSessions = useCallback(async () => {
    const list = await listChatSessions();
    setSessions(list);
    return list;
  }, []);

  // ---- Select a session ----
  const selectSession = useCallback(
    async (sessionId: string) => {
      if (sessionId === activeSessionId) return;
      setActiveSessionId(sessionId);
      const rows = await getChatMessages(sessionId);
      setMessages(rowsToMessages(rows));
      const session = sessions.find(s => s.id === sessionId);
      if (session?.system_prompt) {
        setSystemPrompt(session.system_prompt);
      }
    },
    [activeSessionId, sessions],
  );

  // ---- New chat ----
  const startNewChat = useCallback(() => {
    setActiveSessionId(null);
    setMessages([]);
    setSystemPrompt(DEFAULT_SYSTEM_PROMPT);
  }, []);

  // ---- Delete session ----
  const handleDeleteSession = useCallback(
    async (e: React.MouseEvent, sessionId: string) => {
      e.stopPropagation();
      if (!confirm('Delete this conversation?')) return;
      await deleteChatSession(sessionId);
      const list = await refreshSessions();
      if (activeSessionId === sessionId) {
        if (list.length > 0) {
          setActiveSessionId(list[0].id);
          const rows = await getChatMessages(list[0].id);
          setMessages(rowsToMessages(rows));
        } else {
          setActiveSessionId(null);
          setMessages([]);
        }
      }
    },
    [activeSessionId, refreshSessions],
  );

  // ---- Send message ----
  const sendMessage = useCallback(async () => {
    const text = input.trim();
    if (!text || loading) return;

    setInput('');
    setLoading(true);
    if (textareaRef.current) textareaRef.current.style.height = 'auto';

    let sessionId = activeSessionId;

    try {
      // If no active session, create one
      if (!sessionId) {
        const title = text.length > 50 ? text.slice(0, 50) + '...' : text;
        const session = await createChatSession(title, systemPrompt);
        sessionId = session.id;
        setActiveSessionId(session.id);
        refreshSessions();
      }

      // Save user message to DB
      const userRow = await addChatMessage(sessionId, 'user', text, null, null);
      const userMsg: ChatMessage = {
        id: userRow.id,
        role: 'user',
        content: text,
        timestamp: new Date(userRow.created_at).getTime(),
      };
      setMessages(prev => [...prev, userMsg]);

      // Build API messages
      const apiMessages: ChatMessageInput[] = [];
      if (systemPrompt.trim()) {
        apiMessages.push({ role: 'system', content: systemPrompt.trim() });
      }
      const allMessages = [...messages, userMsg];
      for (const msg of allMessages) {
        if (msg.role === 'user' || msg.role === 'assistant') {
          apiMessages.push({ role: msg.role, content: msg.content });
        }
      }

      const response = await llmChatWithTools({ messages: apiMessages });

      // Build content to persist — prepend tool summary if actions were taken
      let persistContent = response.reply;
      const toolActions = response.tool_calls_made;
      if (toolActions.length > 0) {
        const summary = formatToolSummary(toolActions);
        persistContent = `[Actions]\n${summary}\n\n${response.reply}`;
      }

      // Save assistant message to DB
      const assistantRow = await addChatMessage(
        sessionId,
        'assistant',
        persistContent,
        response.model,
        response.usage?.total_tokens ?? null,
      );
      const assistantMsg: ChatMessage = {
        id: assistantRow.id,
        role: 'assistant',
        content: response.reply,
        timestamp: new Date(assistantRow.created_at).getTime(),
        model: response.model,
        tokens: response.usage?.total_tokens ?? null,
        toolActions: toolActions.length > 0 ? toolActions : undefined,
      };
      setMessages(prev => [...prev, assistantMsg]);
      refreshSessions(); // update sidebar timestamps
    } catch (err: unknown) {
      // Save error message to DB if we have a session
      const errText = `Error: ${err instanceof Error ? err.message : String(err)}`;
      if (sessionId) {
        const errRow = await addChatMessage(sessionId, 'assistant', errText, null, null);
        const errorMsg: ChatMessage = {
          id: errRow.id,
          role: 'assistant',
          content: errText,
          timestamp: new Date(errRow.created_at).getTime(),
        };
        setMessages(prev => [...prev, errorMsg]);
      } else {
        const errorMsg: ChatMessage = {
          id: crypto.randomUUID(),
          role: 'assistant',
          content: errText,
          timestamp: Date.now(),
        };
        setMessages(prev => [...prev, errorMsg]);
      }
    } finally {
      setLoading(false);
      textareaRef.current?.focus();
    }
  }, [input, loading, activeSessionId, messages, systemPrompt, refreshSessions]);

  // ---- Keyboard shortcuts ----
  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
      if (e.key === 'Enter' && !e.shiftKey) {
        e.preventDefault();
        sendMessage();
      }
    },
    [sendMessage],
  );

  const connected = status?.connected ?? false;

  return (
    <div style={styles.outerContainer}>
      {/* Inject keyframe animation */}
      <style>{pulseKeyframes}</style>

      {/* ---- Sidebar ---- */}
      <div style={styles.sidebar}>
        <div style={styles.sidebarHeader}>
          <button style={styles.newChatButton} onClick={startNewChat}>
            + New Chat
          </button>
        </div>
        <div style={styles.sessionList}>
          {sessions.length === 0 && (
            <div style={styles.sidebarEmpty}>No conversations yet</div>
          )}
          {sessions.map(s => (
            <div
              key={s.id}
              style={styles.sessionItem(s.id === activeSessionId)}
              onClick={() => selectSession(s.id)}
              onMouseEnter={e => {
                if (s.id !== activeSessionId)
                  (e.currentTarget as HTMLDivElement).style.background = 'rgba(255,255,255,0.04)';
              }}
              onMouseLeave={e => {
                if (s.id !== activeSessionId)
                  (e.currentTarget as HTMLDivElement).style.background = 'transparent';
              }}
            >
              <span style={styles.sessionTitle}>{s.title}</span>
              <span style={styles.sessionTime}>{relativeTime(s.updated_at)}</span>
              <button
                style={styles.deleteBtn}
                onClick={e => handleDeleteSession(e, s.id)}
                title="Delete conversation"
              >
                {'\u2715'}
              </button>
            </div>
          ))}
        </div>
      </div>

      {/* ---- Chat Area ---- */}
      <div style={styles.chatArea}>
        {/* Status bar */}
        <div style={styles.statusBar}>
          <div style={styles.statusDot(connected)} />
          <span style={styles.statusText}>
            {statusChecking
              ? 'Checking connection...'
              : connected
                ? `${status?.model} @ ${status?.base_url}`
                : status?.error || 'Disconnected'}
          </span>
        </div>

        {/* Messages */}
        <div style={styles.messages}>
          {messages.length === 0 && !loading && (
            <div style={styles.emptyState}>
              <div style={styles.emptyIcon}>{'\u2B21'}</div>
              <div style={styles.emptyText}>
                Send a message to start a conversation.
                <br />
                Shift+Enter for new lines.
              </div>
            </div>
          )}

          {messages.map(msg => (
            <div key={msg.id}>
              {msg.role === 'assistant' && msg.toolActions && msg.toolActions.length > 0 && (
                <ToolActionsBlock actions={msg.toolActions} />
              )}
              <div style={styles.messageBubble(msg.role === 'user')}>{msg.content}</div>
              {msg.role === 'assistant' && (msg.model || msg.tokens) && (
                <div style={styles.messageMetadata}>
                  {[msg.model, msg.tokens != null ? `${msg.tokens} tokens` : null]
                    .filter(Boolean)
                    .join(' \u00B7 ')}
                </div>
              )}
            </div>
          ))}

          {loading && (
            <div style={styles.thinkingIndicator}>
              <div style={styles.dot(0)} />
              <div style={styles.dot(0.2)} />
              <div style={styles.dot(0.4)} />
            </div>
          )}

          <div ref={messagesEndRef} />
        </div>

        {/* System prompt (collapsible) */}
        {showSystemPrompt && (
          <div style={styles.systemPromptArea}>
            <div style={styles.systemLabel}>System Prompt</div>
            <textarea
              style={styles.systemTextarea}
              value={systemPrompt}
              onChange={e => setSystemPrompt(e.target.value)}
              rows={2}
              placeholder="Set the system prompt for the conversation..."
            />
          </div>
        )}

        {/* Input area */}
        <div style={styles.inputArea}>
          <div style={{ flex: 1, display: 'flex', flexDirection: 'column', gap: '0.3rem' }}>
            <textarea
              ref={textareaRef}
              style={styles.textarea}
              value={input}
              onChange={handleInputChange}
              onKeyDown={handleKeyDown}
              placeholder={connected ? 'Type a message...' : 'LLM not connected...'}
              disabled={!connected && !statusChecking}
              rows={1}
            />
            <button
              style={styles.systemPromptToggle}
              onClick={() => setShowSystemPrompt(prev => !prev)}
            >
              {showSystemPrompt ? 'Hide system prompt' : 'System prompt'}
            </button>
          </div>
          <button
            style={styles.sendButton(input.trim().length > 0 && !loading)}
            onClick={sendMessage}
            disabled={!input.trim() || loading}
          >
            Send
          </button>
        </div>
      </div>
    </div>
  );
}

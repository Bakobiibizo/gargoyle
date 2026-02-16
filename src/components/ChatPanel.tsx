import { useState, useEffect, useRef, useCallback } from 'react';
import { llmChat, llmStatus } from '../api/llm';
import type { ChatMessageInput, LlmStatusOutput } from '../api/llm';

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
}

// ---------------------------------------------------------------------------
// Styles
// ---------------------------------------------------------------------------

const styles = {
  container: {
    display: 'flex',
    flexDirection: 'column' as const,
    height: '100%',
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
    alignSelf: isUser ? 'flex-end' as const : 'flex-start' as const,
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
    cursor: enabled ? 'pointer' : 'default',
    fontFamily: 'inherit',
    alignSelf: 'flex-end' as const,
    opacity: enabled ? 1 : 0.5,
  }),
  clearButton: {
    padding: '0.55rem 0.75rem',
    borderRadius: 8,
    border: 'none',
    background: 'rgba(255,255,255,0.08)',
    color: 'inherit',
    fontSize: '0.75rem',
    cursor: 'pointer',
    fontFamily: 'inherit',
    alignSelf: 'flex-end' as const,
    opacity: 0.6,
  },
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
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [input, setInput] = useState('');
  const [systemPrompt, setSystemPrompt] = useState('You are a helpful assistant working within the Gargoyle knowledge graph application.');
  const [showSystemPrompt, setShowSystemPrompt] = useState(false);
  const [loading, setLoading] = useState(false);
  const [status, setStatus] = useState<LlmStatusOutput | null>(null);
  const [statusChecking, setStatusChecking] = useState(true);

  const messagesEndRef = useRef<HTMLDivElement>(null);
  const textareaRef = useRef<HTMLTextAreaElement>(null);

  // Check LLM status on mount
  useEffect(() => {
    setStatusChecking(true);
    llmStatus()
      .then(setStatus)
      .catch(() => setStatus({ connected: false, model: '', base_url: '', error: 'Failed to check status' }))
      .finally(() => setStatusChecking(false));
  }, []);

  // Auto-scroll to bottom on new messages
  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [messages, loading]);

  // Auto-resize textarea
  const handleInputChange = useCallback((e: React.ChangeEvent<HTMLTextAreaElement>) => {
    setInput(e.target.value);
    const el = e.target;
    el.style.height = 'auto';
    el.style.height = Math.min(el.scrollHeight, 128) + 'px';
  }, []);

  // Send message
  const sendMessage = useCallback(async () => {
    const text = input.trim();
    if (!text || loading) return;

    const userMsg: ChatMessage = {
      id: crypto.randomUUID(),
      role: 'user',
      content: text,
      timestamp: Date.now(),
    };

    setMessages(prev => [...prev, userMsg]);
    setInput('');
    setLoading(true);

    // Reset textarea height
    if (textareaRef.current) {
      textareaRef.current.style.height = 'auto';
    }

    try {
      // Build the message list for the API
      const apiMessages: ChatMessageInput[] = [];

      if (systemPrompt.trim()) {
        apiMessages.push({ role: 'system', content: systemPrompt.trim() });
      }

      // Include conversation history
      const allMessages = [...messages, userMsg];
      for (const msg of allMessages) {
        if (msg.role === 'user' || msg.role === 'assistant') {
          apiMessages.push({ role: msg.role, content: msg.content });
        }
      }

      const response = await llmChat({ messages: apiMessages });

      const assistantMsg: ChatMessage = {
        id: crypto.randomUUID(),
        role: 'assistant',
        content: response.reply,
        timestamp: Date.now(),
        model: response.model,
        tokens: response.usage?.total_tokens ?? null,
      };

      setMessages(prev => [...prev, assistantMsg]);
    } catch (err: unknown) {
      const errorMsg: ChatMessage = {
        id: crypto.randomUUID(),
        role: 'assistant',
        content: `Error: ${err instanceof Error ? err.message : String(err)}`,
        timestamp: Date.now(),
      };
      setMessages(prev => [...prev, errorMsg]);
    } finally {
      setLoading(false);
      textareaRef.current?.focus();
    }
  }, [input, loading, messages, systemPrompt]);

  // Handle keyboard shortcuts
  const handleKeyDown = useCallback((e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      sendMessage();
    }
  }, [sendMessage]);

  const clearChat = useCallback(() => {
    setMessages([]);
  }, []);

  const connected = status?.connected ?? false;

  return (
    <div style={styles.container}>
      {/* Inject keyframe animation */}
      <style>{pulseKeyframes}</style>

      {/* Status bar */}
      <div style={styles.statusBar}>
        <div style={styles.statusDot(connected)} />
        <span style={styles.statusText}>
          {statusChecking
            ? 'Checking connection...'
            : connected
              ? `${status?.model} @ ${status?.base_url}`
              : status?.error || 'Disconnected'
          }
        </span>
        <div style={{ flex: 1 }} />
        {messages.length > 0 && (
          <button style={styles.clearButton} onClick={clearChat}>
            Clear
          </button>
        )}
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
            <div style={styles.messageBubble(msg.role === 'user')}>
              {msg.content}
            </div>
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
  );
}

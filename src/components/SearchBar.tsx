import { useState, useRef, useEffect, useCallback } from 'react';
import { searchFts } from '../api/search';
import type { SearchResult } from '../types';

// ---------------------------------------------------------------------------
// Styles
// ---------------------------------------------------------------------------

const styles = {
  wrapper: {
    position: 'relative' as const,
    width: '100%',
  },
  inputContainer: {
    display: 'flex',
    alignItems: 'center',
    background: 'rgba(0,0,0,0.3)',
    borderRadius: 8,
    border: '1px solid rgba(255,255,255,0.12)',
    padding: '0.4rem 0.75rem',
    gap: '0.5rem',
    transition: 'border-color 0.2s',
  },
  inputContainerFocused: {
    borderColor: '#646cff',
  },
  searchIcon: {
    opacity: 0.4,
    fontSize: '0.9rem',
    flexShrink: 0,
  },
  input: {
    flex: 1,
    background: 'transparent',
    border: 'none',
    outline: 'none',
    color: 'inherit',
    fontSize: '0.9rem',
    fontFamily: 'inherit',
  },
  clearButton: {
    background: 'none',
    border: 'none',
    color: 'inherit',
    opacity: 0.4,
    cursor: 'pointer',
    fontSize: '1rem',
    padding: '0 0.25rem',
    lineHeight: 1,
  },
  dropdown: {
    position: 'absolute' as const,
    top: 'calc(100% + 4px)',
    left: 0,
    right: 0,
    background: '#1a1a2e',
    border: '1px solid rgba(255,255,255,0.12)',
    borderRadius: 8,
    maxHeight: '20rem',
    overflow: 'auto',
    zIndex: 50,
    boxShadow: '0 8px 24px rgba(0,0,0,0.5)',
  },
  resultItem: {
    padding: '0.55rem 0.75rem',
    display: 'flex',
    alignItems: 'center',
    gap: '0.6rem',
    cursor: 'pointer',
    borderBottom: '1px solid rgba(255,255,255,0.04)',
    transition: 'background 0.1s',
  },
  resultItemActive: {
    background: 'rgba(100,108,255,0.12)',
  },
  resultTitle: {
    flex: 1,
    fontSize: '0.875rem',
    overflow: 'hidden',
    textOverflow: 'ellipsis',
    whiteSpace: 'nowrap' as const,
  },
  typeBadge: (color: string) => ({
    display: 'inline-block',
    padding: '0.1rem 0.4rem',
    borderRadius: 4,
    fontSize: '0.65rem',
    fontWeight: 600,
    background: color,
    color: '#fff',
    textTransform: 'uppercase' as const,
    letterSpacing: '0.03em',
    flexShrink: 0,
  }),
  score: {
    fontSize: '0.7rem',
    opacity: 0.35,
    fontFamily: 'monospace',
    flexShrink: 0,
  },
  emptyState: {
    padding: '1rem',
    textAlign: 'center' as const,
    fontSize: '0.85rem',
    opacity: 0.4,
  },
  loading: {
    padding: '0.75rem',
    textAlign: 'center' as const,
    fontSize: '0.8rem',
    opacity: 0.4,
  },
  hint: {
    padding: '0.5rem 0.75rem',
    fontSize: '0.75rem',
    opacity: 0.35,
    borderTop: '1px solid rgba(255,255,255,0.04)',
  },
};

// ---------------------------------------------------------------------------
// Color map
// ---------------------------------------------------------------------------

const TYPE_COLORS: Record<string, string> = {
  metric: '#8b5cf6',
  experiment: '#06b6d4',
  result: '#10b981',
  task: '#f59e0b',
  project: '#3b82f6',
  decision: '#ec4899',
  person: '#6366f1',
  note: '#78716c',
  session: '#14b8a6',
  campaign: '#f97316',
  audience: '#a855f7',
  competitor: '#ef4444',
  channel: '#0ea5e9',
  spec: '#64748b',
  budget: '#eab308',
  vendor: '#84cc16',
  playbook: '#d946ef',
};

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

interface SearchBarProps {
  onSelectResult?: (entityId: string) => void;
  placeholder?: string;
}

export default function SearchBar({ onSelectResult, placeholder = 'Search entities...' }: SearchBarProps) {
  const [query, setQuery] = useState('');
  const [results, setResults] = useState<SearchResult[]>([]);
  const [loading, setLoading] = useState(false);
  const [showDropdown, setShowDropdown] = useState(false);
  const [activeIndex, setActiveIndex] = useState(-1);
  const [focused, setFocused] = useState(false);

  const inputRef = useRef<HTMLInputElement>(null);
  const dropdownRef = useRef<HTMLDivElement>(null);
  const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  // Debounced search
  const doSearch = useCallback(async (q: string) => {
    if (q.trim().length < 2) {
      setResults([]);
      setShowDropdown(false);
      return;
    }

    setLoading(true);
    try {
      const res = await searchFts(q.trim(), 15);
      setResults(res);
      setShowDropdown(true);
      setActiveIndex(-1);
    } catch {
      setResults([]);
    } finally {
      setLoading(false);
    }
  }, []);

  function handleChange(value: string) {
    setQuery(value);
    if (debounceRef.current) clearTimeout(debounceRef.current);
    debounceRef.current = setTimeout(() => doSearch(value), 250);
  }

  function handleSelect(result: SearchResult) {
    setShowDropdown(false);
    setQuery('');
    setResults([]);
    if (onSelectResult) {
      onSelectResult(result.entity_id);
    }
  }

  function handleClear() {
    setQuery('');
    setResults([]);
    setShowDropdown(false);
    inputRef.current?.focus();
  }

  // Keyboard navigation
  function handleKeyDown(e: React.KeyboardEvent) {
    if (!showDropdown || results.length === 0) {
      if (e.key === 'Escape') {
        setShowDropdown(false);
      }
      return;
    }

    switch (e.key) {
      case 'ArrowDown':
        e.preventDefault();
        setActiveIndex((prev) => (prev + 1) % results.length);
        break;
      case 'ArrowUp':
        e.preventDefault();
        setActiveIndex((prev) => (prev <= 0 ? results.length - 1 : prev - 1));
        break;
      case 'Enter':
        e.preventDefault();
        if (activeIndex >= 0 && activeIndex < results.length) {
          handleSelect(results[activeIndex]);
        }
        break;
      case 'Escape':
        e.preventDefault();
        setShowDropdown(false);
        break;
    }
  }

  // Close dropdown on outside click
  useEffect(() => {
    function handleClickOutside(e: MouseEvent) {
      if (
        dropdownRef.current &&
        !dropdownRef.current.contains(e.target as Node) &&
        inputRef.current &&
        !inputRef.current.contains(e.target as Node)
      ) {
        setShowDropdown(false);
      }
    }
    document.addEventListener('mousedown', handleClickOutside);
    return () => document.removeEventListener('mousedown', handleClickOutside);
  }, []);

  // Scroll active item into view
  useEffect(() => {
    if (activeIndex >= 0 && dropdownRef.current) {
      const items = dropdownRef.current.querySelectorAll('[data-result-item]');
      items[activeIndex]?.scrollIntoView({ block: 'nearest' });
    }
  }, [activeIndex]);

  return (
    <div style={styles.wrapper}>
      <div style={{ ...styles.inputContainer, ...(focused ? styles.inputContainerFocused : {}) }}>
        <span style={styles.searchIcon}>{'\u2315'}</span>
        <input
          ref={inputRef}
          style={styles.input}
          type="text"
          placeholder={placeholder}
          value={query}
          onChange={(e) => handleChange(e.target.value)}
          onKeyDown={handleKeyDown}
          onFocus={() => {
            setFocused(true);
            if (results.length > 0) setShowDropdown(true);
          }}
          onBlur={() => setFocused(false)}
        />
        {query && (
          <button style={styles.clearButton} onClick={handleClear} tabIndex={-1}>
            {'\u2715'}
          </button>
        )}
      </div>

      {showDropdown && (
        <div ref={dropdownRef} style={styles.dropdown}>
          {loading && <div style={styles.loading}>Searching...</div>}

          {!loading && results.length === 0 && query.trim().length >= 2 && (
            <div style={styles.emptyState}>No results for "{query}"</div>
          )}

          {results.map((result, i) => (
            <div
              key={result.entity_id}
              data-result-item
              style={{
                ...styles.resultItem,
                ...(i === activeIndex ? styles.resultItemActive : {}),
              }}
              onClick={() => handleSelect(result)}
              onMouseEnter={() => setActiveIndex(i)}
            >
              <span style={styles.typeBadge(TYPE_COLORS[result.entity_type] ?? '#555')}>
                {result.entity_type}
              </span>
              <span style={styles.resultTitle}>{result.title}</span>
              <span style={styles.score}>{result.score.toFixed(2)}</span>
            </div>
          ))}

          {results.length > 0 && (
            <div style={styles.hint}>
              Use arrow keys to navigate, Enter to select, Esc to close
            </div>
          )}
        </div>
      )}
    </div>
  );
}

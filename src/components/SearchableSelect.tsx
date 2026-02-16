import { useState, useRef, useEffect, useCallback } from 'react';

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export interface SearchableSelectOption {
  value: string;
  label: string;
}

export interface SearchableSelectProps {
  options: SearchableSelectOption[];
  value: string;
  onChange: (value: string) => void;
  placeholder?: string;
}

// ---------------------------------------------------------------------------
// Styles
// ---------------------------------------------------------------------------

const ssStyles = {
  wrapper: {
    position: 'relative' as const,
    width: '100%',
  },
  input: {
    width: '100%',
    padding: '0.4rem 0.6rem',
    borderRadius: 6,
    border: '1px solid rgba(255,255,255,0.15)',
    background: 'rgba(0,0,0,0.3)',
    color: 'inherit',
    fontSize: '0.85rem',
    fontFamily: 'inherit',
    boxSizing: 'border-box' as const,
  },
  dropdown: {
    position: 'absolute' as const,
    top: '100%',
    left: 0,
    right: 0,
    maxHeight: '14rem',
    overflow: 'auto',
    background: '#1e1e36',
    border: '1px solid rgba(255,255,255,0.15)',
    borderRadius: 6,
    marginTop: 2,
    zIndex: 200,
  },
  option: (highlighted: boolean, selected: boolean) => ({
    padding: '0.4rem 0.6rem',
    fontSize: '0.85rem',
    cursor: 'pointer',
    background: highlighted
      ? 'rgba(100,108,255,0.25)'
      : selected
        ? 'rgba(100,108,255,0.1)'
        : 'transparent',
    borderBottom: '1px solid rgba(255,255,255,0.04)',
  }),
  noResults: {
    padding: '0.5rem 0.6rem',
    fontSize: '0.85rem',
    opacity: 0.5,
  },
};

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

export default function SearchableSelect({
  options,
  value,
  onChange,
  placeholder = 'Search...',
}: SearchableSelectProps) {
  const [isOpen, setIsOpen] = useState(false);
  const [query, setQuery] = useState('');
  const [highlightIndex, setHighlightIndex] = useState(0);
  const wrapperRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLInputElement>(null);
  const listRef = useRef<HTMLDivElement>(null);

  // Find label for current value
  const selectedLabel = options.find((o) => o.value === value)?.label ?? '';

  // Filter options based on query
  const filtered = query
    ? options.filter((o) => o.label.toLowerCase().includes(query.toLowerCase()))
    : options;

  // Close dropdown on outside click
  useEffect(() => {
    function handleClickOutside(e: MouseEvent) {
      if (wrapperRef.current && !wrapperRef.current.contains(e.target as Node)) {
        setIsOpen(false);
        setQuery('');
      }
    }
    document.addEventListener('mousedown', handleClickOutside);
    return () => document.removeEventListener('mousedown', handleClickOutside);
  }, []);

  // Reset highlight when filtered list changes
  useEffect(() => {
    setHighlightIndex(0);
  }, [query]);

  // Scroll highlighted item into view
  useEffect(() => {
    if (!isOpen || !listRef.current) return;
    const items = listRef.current.children;
    if (items[highlightIndex]) {
      (items[highlightIndex] as HTMLElement).scrollIntoView({ block: 'nearest' });
    }
  }, [highlightIndex, isOpen]);

  const selectOption = useCallback(
    (val: string) => {
      onChange(val);
      setIsOpen(false);
      setQuery('');
      inputRef.current?.blur();
    },
    [onChange],
  );

  function handleKeyDown(e: React.KeyboardEvent) {
    if (!isOpen) {
      if (e.key === 'ArrowDown' || e.key === 'Enter') {
        setIsOpen(true);
        e.preventDefault();
      }
      return;
    }

    switch (e.key) {
      case 'ArrowDown':
        e.preventDefault();
        setHighlightIndex((prev) => Math.min(prev + 1, filtered.length - 1));
        break;
      case 'ArrowUp':
        e.preventDefault();
        setHighlightIndex((prev) => Math.max(prev - 1, 0));
        break;
      case 'Enter':
        e.preventDefault();
        if (filtered[highlightIndex]) {
          selectOption(filtered[highlightIndex].value);
        }
        break;
      case 'Escape':
        e.preventDefault();
        setIsOpen(false);
        setQuery('');
        break;
    }
  }

  return (
    <div ref={wrapperRef} style={ssStyles.wrapper}>
      <input
        ref={inputRef}
        style={ssStyles.input}
        value={isOpen ? query : selectedLabel}
        placeholder={placeholder}
        onChange={(e) => {
          setQuery(e.target.value);
          if (!isOpen) setIsOpen(true);
        }}
        onFocus={() => {
          setIsOpen(true);
          setQuery('');
        }}
        onKeyDown={handleKeyDown}
      />

      {isOpen && (
        <div ref={listRef} style={ssStyles.dropdown}>
          {filtered.length === 0 ? (
            <div style={ssStyles.noResults}>No matches</div>
          ) : (
            filtered.map((opt, i) => (
              <div
                key={opt.value}
                style={ssStyles.option(i === highlightIndex, opt.value === value)}
                onMouseEnter={() => setHighlightIndex(i)}
                onMouseDown={(e) => {
                  e.preventDefault(); // prevent blur before click registers
                  selectOption(opt.value);
                }}
              >
                {opt.label}
              </div>
            ))
          )}
        </div>
      )}
    </div>
  );
}

import { useState, useEffect, useCallback, useMemo } from 'react';
import { listDedupSuggestions, resolveDedupSuggestion } from '../api/dedup';
import { listClaims } from '../api/claims';
import { getEntity } from '../api/entities';
import type { DedupSuggestion, DetectionMethod, Claim, Entity } from '../types';

// ---------------------------------------------------------------------------
// Styles (matches EntityManager.tsx pattern)
// ---------------------------------------------------------------------------

const styles = {
  container: {
    display: 'flex',
    flexDirection: 'column' as const,
    gap: '1rem',
    padding: '1.5rem',
    height: '100%',
    overflow: 'auto',
  },
  heading: {
    margin: 0,
    fontSize: '1.5rem',
    fontWeight: 600,
  },
  toolbar: {
    display: 'flex',
    gap: '0.75rem',
    alignItems: 'center',
    flexWrap: 'wrap' as const,
  },
  select: {
    padding: '0.4rem 0.6rem',
    borderRadius: 6,
    border: '1px solid rgba(255,255,255,0.15)',
    background: 'rgba(0,0,0,0.3)',
    color: 'inherit',
    fontSize: '0.85rem',
    fontFamily: 'inherit',
  },
  input: {
    padding: '0.4rem 0.6rem',
    borderRadius: 6,
    border: '1px solid rgba(255,255,255,0.15)',
    background: 'rgba(0,0,0,0.3)',
    color: 'inherit',
    fontSize: '0.85rem',
    fontFamily: 'inherit',
    boxSizing: 'border-box' as const,
    width: '100%',
  },
  button: {
    padding: '0.4rem 1rem',
    borderRadius: 6,
    border: 'none',
    background: '#646cff',
    color: '#fff',
    fontSize: '0.85rem',
    fontWeight: 500,
    cursor: 'pointer',
    fontFamily: 'inherit',
    whiteSpace: 'nowrap' as const,
  },
  buttonDanger: {
    background: '#dc2626',
  },
  buttonSecondary: {
    background: 'rgba(255,255,255,0.1)',
    color: 'inherit',
  },
  buttonSuccess: {
    background: '#16a34a',
  },
  section: {
    background: 'rgba(255,255,255,0.04)',
    borderRadius: 8,
    border: '1px solid rgba(255,255,255,0.08)',
    overflow: 'hidden',
  },
  sectionTitle: {
    margin: 0,
    padding: '0.75rem 1rem',
    fontSize: '0.8rem',
    fontWeight: 600,
    textTransform: 'uppercase' as const,
    letterSpacing: '0.05em',
    opacity: 0.6,
    borderBottom: '1px solid rgba(255,255,255,0.06)',
  },
  row: {
    padding: '0.6rem 1rem',
    borderBottom: '1px solid rgba(255,255,255,0.04)',
    display: 'flex',
    alignItems: 'center',
    gap: '0.75rem',
    transition: 'background 0.15s',
  },
  entityTitle: {
    flex: 1,
    fontWeight: 500,
    fontSize: '0.9rem',
    overflow: 'hidden',
    textOverflow: 'ellipsis',
    whiteSpace: 'nowrap' as const,
  },
  entityMeta: {
    fontSize: '0.75rem',
    opacity: 0.5,
  },
  typeBadge: (color: string) => ({
    display: 'inline-block',
    padding: '0.1rem 0.45rem',
    borderRadius: 4,
    fontSize: '0.7rem',
    fontWeight: 600,
    background: color,
    color: '#fff',
    textTransform: 'uppercase' as const,
    letterSpacing: '0.03em',
  }),
  statusBadge: (color: string) => ({
    display: 'inline-block',
    padding: '0.1rem 0.45rem',
    borderRadius: 4,
    fontSize: '0.7rem',
    fontWeight: 500,
    border: `1px solid ${color}`,
    color,
  }),
  error: {
    color: '#f87171',
    padding: '0.5rem 1rem',
    fontSize: '0.85rem',
  },
  count: {
    fontSize: '0.8rem',
    opacity: 0.5,
  },
  confirmOverlay: {
    position: 'fixed' as const,
    inset: 0,
    background: 'rgba(0,0,0,0.6)',
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'center',
    zIndex: 101,
  },
  confirmModal: {
    background: '#1a1a2e',
    borderRadius: 10,
    padding: '1.5rem',
    width: '100%',
    maxWidth: '40rem',
    maxHeight: '80vh',
    overflow: 'auto',
    border: '1px solid rgba(255,255,255,0.1)',
  },
  formTitle: {
    margin: '0 0 1rem',
    fontSize: '1.1rem',
    fontWeight: 600,
  },
  confidenceBar: {
    height: 6,
    borderRadius: 3,
    background: 'rgba(255,255,255,0.08)',
    flex: 1,
    maxWidth: '6rem',
    overflow: 'hidden',
  },
  confidenceFill: (pct: number) => ({
    height: '100%',
    width: `${pct}%`,
    borderRadius: 3,
    background: pct >= 80 ? '#4ade80' : pct >= 50 ? '#fbbf24' : '#f87171',
    transition: 'width 0.3s',
  }),
  statsGrid: {
    display: 'grid',
    gridTemplateColumns: 'repeat(auto-fit, minmax(10rem, 1fr))',
    gap: '0.75rem',
    marginBottom: '0.5rem',
  },
  statCard: {
    background: 'rgba(255,255,255,0.04)',
    borderRadius: 8,
    border: '1px solid rgba(255,255,255,0.08)',
    padding: '0.75rem 1rem',
    textAlign: 'center' as const,
  },
  statValue: {
    fontSize: '1.5rem',
    fontWeight: 700,
    color: '#a5b4fc',
  },
  statLabel: {
    fontSize: '0.7rem',
    opacity: 0.5,
    textTransform: 'uppercase' as const,
    letterSpacing: '0.05em',
    marginTop: '0.2rem',
  },
  tabBar: {
    display: 'flex',
    gap: '0.25rem',
    borderBottom: '1px solid rgba(255,255,255,0.08)',
    marginBottom: '0.5rem',
  },
  tab: (active: boolean) => ({
    padding: '0.5rem 1rem',
    cursor: 'pointer',
    fontSize: '0.85rem',
    fontWeight: active ? 600 : 400,
    color: active ? '#a5b4fc' : 'inherit',
    borderBottom: active ? '2px solid #646cff' : '2px solid transparent',
    background: 'none',
    border: 'none',
    borderBottomWidth: 2,
    borderBottomStyle: 'solid' as const,
    borderBottomColor: active ? '#646cff' : 'transparent',
    fontFamily: 'inherit',
    opacity: active ? 1 : 0.6,
    transition: 'color 0.15s, opacity 0.15s',
  }),
  claimArrow: {
    opacity: 0.4,
    fontSize: '0.85rem',
    margin: '0 0.25rem',
  },
  claimSubject: {
    fontWeight: 600,
    fontSize: '0.85rem',
    color: '#a5b4fc',
  },
  claimPredicate: {
    fontSize: '0.85rem',
    opacity: 0.7,
    fontStyle: 'italic' as const,
  },
  claimObject: {
    fontWeight: 600,
    fontSize: '0.85rem',
    color: '#c4b5fd',
  },
  slider: {
    width: '8rem',
    accentColor: '#646cff',
  },
  groupHeader: {
    padding: '0.5rem 1rem',
    fontSize: '0.8rem',
    fontWeight: 600,
    background: 'rgba(100,108,255,0.08)',
    borderBottom: '1px solid rgba(255,255,255,0.06)',
    color: '#a5b4fc',
  },
  entityCompare: {
    display: 'grid',
    gridTemplateColumns: '1fr 1fr',
    gap: '1rem',
    marginBottom: '1rem',
  },
  entityCard: {
    background: 'rgba(0,0,0,0.2)',
    borderRadius: 8,
    padding: '1rem',
    border: '1px solid rgba(255,255,255,0.08)',
  },
  entityCardTitle: {
    fontSize: '0.95rem',
    fontWeight: 600,
    marginBottom: '0.5rem',
  },
  entityCardField: {
    display: 'flex',
    gap: '0.5rem',
    marginBottom: '0.3rem',
    fontSize: '0.8rem',
  },
  entityCardLabel: {
    fontWeight: 600,
    minWidth: '5rem',
    opacity: 0.6,
  },
  entityCardValue: {
    flex: 1,
    wordBreak: 'break-word' as const,
  },
};

// ---------------------------------------------------------------------------
// Color maps
// ---------------------------------------------------------------------------

const METHOD_COLORS: Record<DetectionMethod, string> = {
  exact_title: '#ef4444',
  fuzzy_title: '#f59e0b',
  embedding_proximity: '#8b5cf6',
};

const METHOD_LABELS: Record<DetectionMethod, string> = {
  exact_title: 'Exact',
  fuzzy_title: 'Fuzzy',
  embedding_proximity: 'Embedding',
};

const DEDUP_STATUS_COLORS: Record<string, string> = {
  pending: '#fbbf24',
  accepted: '#4ade80',
  dismissed: '#6b7280',
  merged: '#38bdf8',
};

// ---------------------------------------------------------------------------
// Sub-components
// ---------------------------------------------------------------------------

function MethodBadge({ method }: { method: DetectionMethod }) {
  return (
    <span style={styles.typeBadge(METHOD_COLORS[method] ?? '#555')}>
      {METHOD_LABELS[method] ?? method}
    </span>
  );
}

function DedupStatusBadge({ status }: { status: string }) {
  return (
    <span style={styles.statusBadge(DEDUP_STATUS_COLORS[status] ?? '#888')}>
      {status}
    </span>
  );
}

function ConfidenceBar({ confidence }: { confidence: number }) {
  const pct = Math.round(confidence * 100);
  return (
    <div style={{ display: 'flex', alignItems: 'center', gap: '0.4rem' }}>
      <div style={styles.confidenceBar}>
        <div style={styles.confidenceFill(pct)} />
      </div>
      <span style={{ fontSize: '0.7rem', opacity: 0.6, minWidth: '2.5rem' }}>
        {pct}%
      </span>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Merge Confirmation Modal
// ---------------------------------------------------------------------------

interface MergeModalProps {
  suggestion: DedupSuggestion;
  onConfirm: () => void;
  onCancel: () => void;
  loading: boolean;
}

function MergeConfirmModal({ suggestion, onConfirm, onCancel, loading }: MergeModalProps) {
  const [newEntity, setNewEntity] = useState<Entity | null>(null);
  const [existingEntity, setExistingEntity] = useState<Entity | null>(null);
  const [fetchError, setFetchError] = useState('');

  useEffect(() => {
    let cancelled = false;
    async function fetchEntities() {
      try {
        const [ne, ee] = await Promise.all([
          getEntity(suggestion.new_entity_id),
          getEntity(suggestion.existing_entity_id),
        ]);
        if (!cancelled) {
          setNewEntity(ne);
          setExistingEntity(ee);
        }
      } catch (e) {
        if (!cancelled) {
          setFetchError(String(e));
        }
      }
    }
    fetchEntities();
    return () => { cancelled = true; };
  }, [suggestion.new_entity_id, suggestion.existing_entity_id]);

  return (
    <div style={styles.confirmOverlay} onMouseDown={(e) => { if (e.target === e.currentTarget) onCancel(); }}>
      <div style={styles.confirmModal}>
        <h3 style={styles.formTitle}>Confirm Merge</h3>
        <p style={{ fontSize: '0.85rem', opacity: 0.7, margin: '0 0 1rem' }}>
          Review the two entities below. Accepting this suggestion will mark it as accepted.
        </p>

        <div style={{ display: 'flex', gap: '0.5rem', alignItems: 'center', marginBottom: '1rem' }}>
          <MethodBadge method={suggestion.detection_method} />
          <ConfidenceBar confidence={suggestion.confidence} />
        </div>

        {fetchError && <div style={styles.error}>{fetchError}</div>}

        <div style={styles.entityCompare}>
          <div style={styles.entityCard}>
            <div style={{ fontSize: '0.7rem', opacity: 0.5, textTransform: 'uppercase' as const, letterSpacing: '0.05em', marginBottom: '0.5rem' }}>
              New Entity
            </div>
            {newEntity ? (
              <>
                <div style={styles.entityCardTitle}>{newEntity.title}</div>
                <div style={styles.entityCardField}>
                  <span style={styles.entityCardLabel}>Type</span>
                  <span style={styles.entityCardValue}>{newEntity.entity_type}</span>
                </div>
                <div style={styles.entityCardField}>
                  <span style={styles.entityCardLabel}>Status</span>
                  <span style={styles.entityCardValue}>{newEntity.status ?? '-'}</span>
                </div>
                <div style={styles.entityCardField}>
                  <span style={styles.entityCardLabel}>Source</span>
                  <span style={styles.entityCardValue}>{newEntity.source}</span>
                </div>
                <div style={styles.entityCardField}>
                  <span style={styles.entityCardLabel}>Category</span>
                  <span style={styles.entityCardValue}>{newEntity.category ?? '-'}</span>
                </div>
                <div style={styles.entityCardField}>
                  <span style={styles.entityCardLabel}>Created</span>
                  <span style={styles.entityCardValue}>{new Date(newEntity.created_at).toLocaleString()}</span>
                </div>
                {newEntity.body_md && (
                  <div style={{ marginTop: '0.5rem', fontSize: '0.8rem', opacity: 0.6, maxHeight: '5rem', overflow: 'auto', whiteSpace: 'pre-wrap' as const }}>
                    {newEntity.body_md}
                  </div>
                )}
              </>
            ) : (
              <div style={{ opacity: 0.4, fontSize: '0.85rem' }}>Loading...</div>
            )}
          </div>

          <div style={styles.entityCard}>
            <div style={{ fontSize: '0.7rem', opacity: 0.5, textTransform: 'uppercase' as const, letterSpacing: '0.05em', marginBottom: '0.5rem' }}>
              Existing Entity
            </div>
            {existingEntity ? (
              <>
                <div style={styles.entityCardTitle}>{existingEntity.title}</div>
                <div style={styles.entityCardField}>
                  <span style={styles.entityCardLabel}>Type</span>
                  <span style={styles.entityCardValue}>{existingEntity.entity_type}</span>
                </div>
                <div style={styles.entityCardField}>
                  <span style={styles.entityCardLabel}>Status</span>
                  <span style={styles.entityCardValue}>{existingEntity.status ?? '-'}</span>
                </div>
                <div style={styles.entityCardField}>
                  <span style={styles.entityCardLabel}>Source</span>
                  <span style={styles.entityCardValue}>{existingEntity.source}</span>
                </div>
                <div style={styles.entityCardField}>
                  <span style={styles.entityCardLabel}>Category</span>
                  <span style={styles.entityCardValue}>{existingEntity.category ?? '-'}</span>
                </div>
                <div style={styles.entityCardField}>
                  <span style={styles.entityCardLabel}>Created</span>
                  <span style={styles.entityCardValue}>{new Date(existingEntity.created_at).toLocaleString()}</span>
                </div>
                {existingEntity.body_md && (
                  <div style={{ marginTop: '0.5rem', fontSize: '0.8rem', opacity: 0.6, maxHeight: '5rem', overflow: 'auto', whiteSpace: 'pre-wrap' as const }}>
                    {existingEntity.body_md}
                  </div>
                )}
              </>
            ) : (
              <div style={{ opacity: 0.4, fontSize: '0.85rem' }}>Loading...</div>
            )}
          </div>
        </div>

        <div style={{ display: 'flex', gap: '0.5rem', justifyContent: 'flex-end' }}>
          <button
            style={{ ...styles.button, ...styles.buttonSecondary }}
            onClick={onCancel}
          >
            Cancel
          </button>
          <button
            style={{ ...styles.button, ...styles.buttonSuccess }}
            onClick={onConfirm}
            disabled={loading}
          >
            {loading ? 'Merging...' : 'Confirm Merge'}
          </button>
        </div>
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Dedup Suggestions Panel
// ---------------------------------------------------------------------------

type SubTab = 'dedup' | 'claims';

function DedupSuggestionsPanel() {
  const [suggestions, setSuggestions] = useState<DedupSuggestion[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState('');
  const [methodFilter, setMethodFilter] = useState<string>('');
  const [statusFilter, setStatusFilter] = useState<string>('');
  const [mergeTarget, setMergeTarget] = useState<DedupSuggestion | null>(null);
  const [merging, setMerging] = useState(false);

  const fetchSuggestions = useCallback(async () => {
    setLoading(true);
    setError('');
    try {
      const result = await listDedupSuggestions(statusFilter || undefined);
      setSuggestions(result);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, [statusFilter]);

  useEffect(() => {
    fetchSuggestions();
  }, [fetchSuggestions]);

  const filtered = useMemo(() => {
    let result = suggestions;
    if (methodFilter) {
      result = result.filter((s) => s.detection_method === methodFilter);
    }
    return result;
  }, [suggestions, methodFilter]);

  // Statistics
  const stats = useMemo(() => {
    const total = suggestions.length;
    const byMethod: Record<string, number> = {};
    const byStatus: Record<string, number> = {};
    for (const s of suggestions) {
      byMethod[s.detection_method] = (byMethod[s.detection_method] || 0) + 1;
      byStatus[s.status] = (byStatus[s.status] || 0) + 1;
    }
    return { total, byMethod, byStatus };
  }, [suggestions]);

  async function handleAccept(suggestion: DedupSuggestion) {
    setMergeTarget(suggestion);
  }

  async function handleConfirmMerge() {
    if (!mergeTarget) return;
    setMerging(true);
    try {
      await resolveDedupSuggestion(mergeTarget.suggestion_id, 'accepted');
      setMergeTarget(null);
      fetchSuggestions();
    } catch (e) {
      setError(String(e));
    } finally {
      setMerging(false);
    }
  }

  async function handleDismiss(suggestion: DedupSuggestion) {
    try {
      await resolveDedupSuggestion(suggestion.suggestion_id, 'dismissed');
      fetchSuggestions();
    } catch (e) {
      setError(String(e));
    }
  }

  return (
    <>
      {/* Statistics */}
      <div style={styles.statsGrid}>
        <div style={styles.statCard}>
          <div style={styles.statValue}>{stats.total}</div>
          <div style={styles.statLabel}>Total Suggestions</div>
        </div>
        {Object.entries(stats.byMethod).map(([method, count]) => (
          <div key={method} style={styles.statCard}>
            <div style={styles.statValue}>{count}</div>
            <div style={styles.statLabel}>{METHOD_LABELS[method as DetectionMethod] ?? method}</div>
          </div>
        ))}
        {Object.entries(stats.byStatus).map(([status, count]) => (
          <div key={status} style={styles.statCard}>
            <div style={styles.statValue}>{count}</div>
            <div style={styles.statLabel}>{status}</div>
          </div>
        ))}
      </div>

      {/* Toolbar */}
      <div style={styles.toolbar}>
        <select
          style={styles.select}
          value={methodFilter}
          onChange={(e) => setMethodFilter(e.target.value)}
        >
          <option value="">All methods</option>
          <option value="exact_title">Exact Title</option>
          <option value="fuzzy_title">Fuzzy Title</option>
          <option value="embedding_proximity">Embedding Proximity</option>
        </select>

        <select
          style={styles.select}
          value={statusFilter}
          onChange={(e) => setStatusFilter(e.target.value)}
        >
          <option value="">All statuses</option>
          <option value="pending">Pending</option>
          <option value="accepted">Accepted</option>
          <option value="dismissed">Dismissed</option>
        </select>

        <button
          style={{ ...styles.button, ...styles.buttonSecondary }}
          onClick={fetchSuggestions}
          disabled={loading}
        >
          {loading ? 'Loading...' : 'Refresh'}
        </button>

        <span style={styles.count}>{filtered.length} suggestions</span>
      </div>

      {error && <div style={styles.error}>{error}</div>}

      {/* Suggestions list */}
      <div style={styles.section}>
        <div style={styles.sectionTitle}>
          Dedup Suggestions
          {methodFilter ? ` -- ${METHOD_LABELS[methodFilter as DetectionMethod] ?? methodFilter}` : ''}
        </div>

        {filtered.length === 0 && !loading && (
          <div style={{ padding: '1.5rem', textAlign: 'center', opacity: 0.4, fontSize: '0.9rem' }}>
            No dedup suggestions found
          </div>
        )}

        {loading && filtered.length === 0 && (
          <div style={{ padding: '1.5rem', textAlign: 'center', opacity: 0.4, fontSize: '0.9rem' }}>
            Loading...
          </div>
        )}

        {filtered.map((suggestion) => (
          <div key={suggestion.suggestion_id} style={styles.row}>
            <MethodBadge method={suggestion.detection_method} />
            <span style={styles.entityTitle}>
              <span style={{ opacity: 0.7 }} title={suggestion.new_entity_id}>
                {suggestion.new_entity_id.substring(0, 8)}...
              </span>
              <span style={styles.claimArrow}>{' -> '}</span>
              <span style={{ opacity: 0.7 }} title={suggestion.existing_entity_id}>
                {suggestion.existing_entity_id.substring(0, 8)}...
              </span>
            </span>
            <ConfidenceBar confidence={suggestion.confidence} />
            <DedupStatusBadge status={suggestion.status} />
            <span style={styles.entityMeta}>
              {new Date(suggestion.created_at).toLocaleDateString()}
            </span>
            {suggestion.status === 'pending' && (
              <>
                <button
                  style={{ ...styles.button, ...styles.buttonSuccess, padding: '0.2rem 0.5rem', fontSize: '0.7rem' }}
                  onClick={() => handleAccept(suggestion)}
                >
                  Accept
                </button>
                <button
                  style={{ ...styles.button, ...styles.buttonSecondary, padding: '0.2rem 0.5rem', fontSize: '0.7rem' }}
                  onClick={() => handleDismiss(suggestion)}
                >
                  Dismiss
                </button>
              </>
            )}
          </div>
        ))}
      </div>

      {/* Merge confirmation modal */}
      {mergeTarget && (
        <MergeConfirmModal
          suggestion={mergeTarget}
          onConfirm={handleConfirmMerge}
          onCancel={() => setMergeTarget(null)}
          loading={merging}
        />
      )}
    </>
  );
}

// ---------------------------------------------------------------------------
// Claims Panel
// ---------------------------------------------------------------------------

function ClaimsPanel() {
  const [claims, setClaims] = useState<Claim[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState('');
  const [confidenceThreshold, setConfidenceThreshold] = useState(0);
  const [groupBySubject, setGroupBySubject] = useState(false);

  const fetchClaims = useCallback(async () => {
    setLoading(true);
    setError('');
    try {
      const result = await listClaims();
      setClaims(result);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    fetchClaims();
  }, [fetchClaims]);

  const filtered = useMemo(() => {
    return claims.filter((c) => c.confidence >= confidenceThreshold);
  }, [claims, confidenceThreshold]);

  const grouped = useMemo(() => {
    if (!groupBySubject) return null;
    const groups: Record<string, Claim[]> = {};
    for (const claim of filtered) {
      if (!groups[claim.subject]) {
        groups[claim.subject] = [];
      }
      groups[claim.subject].push(claim);
    }
    return groups;
  }, [filtered, groupBySubject]);

  function renderClaimRow(claim: Claim) {
    return (
      <div key={claim.claim_id} style={styles.row}>
        <span style={styles.claimSubject}>{claim.subject}</span>
        <span style={styles.claimArrow}>{'->'}</span>
        <span style={styles.claimPredicate}>{claim.predicate}</span>
        <span style={styles.claimArrow}>{'->'}</span>
        <span style={styles.claimObject}>{claim.object}</span>
        <ConfidenceBar confidence={claim.confidence} />
        <span
          style={{ ...styles.entityMeta, cursor: 'pointer', textDecoration: 'underline' }}
          title={`Evidence: ${claim.evidence_entity_id}`}
        >
          {claim.evidence_entity_id.substring(0, 8)}...
        </span>
        {claim.promoted_to_entity_id && (
          <span style={styles.statusBadge('#4ade80')}>promoted</span>
        )}
        <span style={styles.entityMeta}>
          {new Date(claim.created_at).toLocaleDateString()}
        </span>
      </div>
    );
  }

  return (
    <>
      {/* Toolbar */}
      <div style={styles.toolbar}>
        <label style={{ fontSize: '0.8rem', opacity: 0.6, display: 'flex', alignItems: 'center', gap: '0.4rem' }}>
          Min confidence:
          <input
            type="range"
            min="0"
            max="1"
            step="0.05"
            value={confidenceThreshold}
            onChange={(e) => setConfidenceThreshold(parseFloat(e.target.value))}
            style={styles.slider}
          />
          <span style={{ minWidth: '2.5rem', fontSize: '0.8rem' }}>
            {Math.round(confidenceThreshold * 100)}%
          </span>
        </label>

        <button
          style={{
            ...styles.button,
            ...(groupBySubject ? {} : styles.buttonSecondary),
            padding: '0.3rem 0.75rem',
            fontSize: '0.8rem',
          }}
          onClick={() => setGroupBySubject(!groupBySubject)}
        >
          {groupBySubject ? 'Grouped' : 'Flat list'}
        </button>

        <button
          style={{ ...styles.button, ...styles.buttonSecondary }}
          onClick={fetchClaims}
          disabled={loading}
        >
          {loading ? 'Loading...' : 'Refresh'}
        </button>

        <span style={styles.count}>{filtered.length} claims</span>
      </div>

      {error && <div style={styles.error}>{error}</div>}

      {/* Claims list */}
      <div style={styles.section}>
        <div style={styles.sectionTitle}>
          Claims
          {confidenceThreshold > 0 ? ` (>= ${Math.round(confidenceThreshold * 100)}%)` : ''}
        </div>

        {filtered.length === 0 && !loading && (
          <div style={{ padding: '1.5rem', textAlign: 'center', opacity: 0.4, fontSize: '0.9rem' }}>
            No claims found
          </div>
        )}

        {loading && filtered.length === 0 && (
          <div style={{ padding: '1.5rem', textAlign: 'center', opacity: 0.4, fontSize: '0.9rem' }}>
            Loading...
          </div>
        )}

        {grouped
          ? Object.entries(grouped).map(([subject, subjectClaims]) => (
              <div key={subject}>
                <div style={styles.groupHeader}>
                  {subject} ({subjectClaims.length})
                </div>
                {subjectClaims.map(renderClaimRow)}
              </div>
            ))
          : filtered.map(renderClaimRow)}
      </div>
    </>
  );
}

// ---------------------------------------------------------------------------
// Main component
// ---------------------------------------------------------------------------

export default function DedupDashboard() {
  const [activeSubTab, setActiveSubTab] = useState<SubTab>('dedup');

  return (
    <div style={styles.container}>
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
        <h2 style={styles.heading}>Dedup + Claims</h2>
      </div>

      {/* Sub-tab bar */}
      <div style={styles.tabBar}>
        <button
          style={styles.tab(activeSubTab === 'dedup')}
          onClick={() => setActiveSubTab('dedup')}
        >
          Dedup Suggestions
        </button>
        <button
          style={styles.tab(activeSubTab === 'claims')}
          onClick={() => setActiveSubTab('claims')}
        >
          Claims
        </button>
      </div>

      {activeSubTab === 'dedup' && <DedupSuggestionsPanel />}
      {activeSubTab === 'claims' && <ClaimsPanel />}
    </div>
  );
}

import { useState, useEffect, useCallback } from 'react';
import { listEntities, createEntity, deleteEntity, getEntity } from '../api/entities';
import type { Entity, CreateEntityPayload } from '../types';

// ---------------------------------------------------------------------------
// Styles
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
  entityRow: {
    padding: '0.6rem 1rem',
    borderBottom: '1px solid rgba(255,255,255,0.04)',
    cursor: 'pointer',
    display: 'flex',
    alignItems: 'center',
    gap: '0.75rem',
    transition: 'background 0.15s',
  },
  entityRowHover: {
    background: 'rgba(255,255,255,0.04)',
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
  detailPanel: {
    padding: '1rem',
    background: 'rgba(0,0,0,0.15)',
    borderTop: '1px solid rgba(255,255,255,0.06)',
    fontSize: '0.85rem',
  },
  detailRow: {
    display: 'flex',
    gap: '0.5rem',
    marginBottom: '0.3rem',
  },
  detailLabel: {
    fontWeight: 600,
    minWidth: '8rem',
    opacity: 0.6,
    fontSize: '0.8rem',
  },
  detailValue: {
    flex: 1,
    wordBreak: 'break-word' as const,
  },
  codeBlock: {
    padding: '0.5rem',
    background: 'rgba(0,0,0,0.2)',
    borderRadius: 4,
    fontSize: '0.8rem',
    fontFamily: 'monospace',
    whiteSpace: 'pre-wrap' as const,
    wordBreak: 'break-word' as const,
    maxHeight: '12rem',
    overflow: 'auto',
    marginTop: '0.25rem',
  },
  error: {
    color: '#f87171',
    padding: '0.5rem 1rem',
    fontSize: '0.85rem',
  },
  formOverlay: {
    position: 'fixed' as const,
    inset: 0,
    background: 'rgba(0,0,0,0.6)',
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'center',
    zIndex: 100,
  },
  formModal: {
    background: '#1a1a2e',
    borderRadius: 10,
    padding: '1.5rem',
    width: '100%',
    maxWidth: '28rem',
    maxHeight: '80vh',
    overflow: 'auto',
    border: '1px solid rgba(255,255,255,0.1)',
  },
  formTitle: {
    margin: '0 0 1rem',
    fontSize: '1.1rem',
    fontWeight: 600,
  },
  fieldGroup: {
    marginBottom: '0.75rem',
  },
  label: {
    display: 'block',
    marginBottom: '0.2rem',
    fontSize: '0.8rem',
    opacity: 0.7,
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
    maxWidth: '24rem',
    border: '1px solid rgba(255,255,255,0.1)',
    textAlign: 'center' as const,
  },
};

// ---------------------------------------------------------------------------
// Color maps
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

const STATUS_COLORS: Record<string, string> = {
  active: '#4ade80',
  draft: '#94a3b8',
  running: '#38bdf8',
  completed: '#22c55e',
  done: '#22c55e',
  archived: '#6b7280',
  paused: '#fbbf24',
  blocked: '#ef4444',
  todo: '#a78bfa',
  backlog: '#64748b',
  in_progress: '#38bdf8',
  planning: '#818cf8',
  proposed: '#c084fc',
  accepted: '#4ade80',
  deprecated: '#f87171',
  final: '#22d3ee',
  scheduled: '#818cf8',
  cancelled: '#ef4444',
  tracking: '#38bdf8',
  evaluating: '#fbbf24',
  scaling: '#06b6d4',
  review: '#a78bfa',
  approved: '#4ade80',
  closed: '#6b7280',
  on_hold: '#fbbf24',
  terminated: '#ef4444',
  dormant: '#64748b',
  validated: '#22d3ee',
  superseded: '#f97316',
  inactive: '#6b7280',
};

const ENTITY_TYPES = [
  'metric', 'experiment', 'result', 'task', 'project', 'decision',
  'person', 'note', 'session', 'campaign', 'audience', 'competitor',
  'channel', 'spec', 'budget', 'vendor', 'playbook',
];

// ---------------------------------------------------------------------------
// Sub-components
// ---------------------------------------------------------------------------

function TypeBadge({ type }: { type: string }) {
  return <span style={styles.typeBadge(TYPE_COLORS[type] ?? '#555')}>{type}</span>;
}

function StatusBadge({ status }: { status: string | null }) {
  if (!status) return <span style={styles.statusBadge('#555')}>none</span>;
  return <span style={styles.statusBadge(STATUS_COLORS[status] ?? '#888')}>{status}</span>;
}

// ---------------------------------------------------------------------------
// Create entity form
// ---------------------------------------------------------------------------

interface CreateFormProps {
  onClose: () => void;
  onCreated: () => void;
}

function CreateEntityForm({ onClose, onCreated }: CreateFormProps) {
  const [form, setForm] = useState<Partial<CreateEntityPayload>>({
    entity_type: 'note',
    title: '',
    source: 'manual',
    canonical_fields: {},
    body_md: '',
    status: 'draft',
    category: '',
  });
  const [canonicalJson, setCanonicalJson] = useState('{}');
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState('');

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    if (!form.title || !form.entity_type) return;

    setSaving(true);
    setError('');

    let parsedCanonical: Record<string, unknown> = {};
    try {
      parsedCanonical = JSON.parse(canonicalJson);
    } catch {
      setError('Invalid JSON in canonical fields');
      setSaving(false);
      return;
    }

    try {
      await createEntity({
        entity_type: form.entity_type!,
        title: form.title!,
        source: form.source || 'manual',
        canonical_fields: parsedCanonical,
        body_md: form.body_md || undefined,
        status: form.status || undefined,
        category: form.category || undefined,
      });
      onCreated();
      onClose();
    } catch (e) {
      setError(String(e));
    } finally {
      setSaving(false);
    }
  }

  return (
    <div style={styles.formOverlay} onClick={onClose}>
      <div style={styles.formModal} onClick={(e) => e.stopPropagation()}>
        <h3 style={styles.formTitle}>Create Entity</h3>
        <form onSubmit={handleSubmit}>
          <div style={styles.fieldGroup}>
            <label style={styles.label}>Entity Type</label>
            <select
              style={{ ...styles.select, width: '100%' }}
              value={form.entity_type}
              onChange={(e) => setForm((f) => ({ ...f, entity_type: e.target.value }))}
            >
              {ENTITY_TYPES.map((t) => (
                <option key={t} value={t}>{t}</option>
              ))}
            </select>
          </div>

          <div style={styles.fieldGroup}>
            <label style={styles.label}>Title</label>
            <input
              style={styles.input}
              value={form.title}
              onChange={(e) => setForm((f) => ({ ...f, title: e.target.value }))}
              placeholder="Entity title"
              required
            />
          </div>

          <div style={styles.fieldGroup}>
            <label style={styles.label}>Status</label>
            <input
              style={styles.input}
              value={form.status ?? ''}
              onChange={(e) => setForm((f) => ({ ...f, status: e.target.value }))}
              placeholder="e.g. draft, active"
            />
          </div>

          <div style={styles.fieldGroup}>
            <label style={styles.label}>Category</label>
            <input
              style={styles.input}
              value={form.category ?? ''}
              onChange={(e) => setForm((f) => ({ ...f, category: e.target.value }))}
              placeholder="Optional category"
            />
          </div>

          <div style={styles.fieldGroup}>
            <label style={styles.label}>Body (Markdown)</label>
            <textarea
              style={{ ...styles.input, minHeight: '4rem', resize: 'vertical' as const }}
              value={form.body_md ?? ''}
              onChange={(e) => setForm((f) => ({ ...f, body_md: e.target.value }))}
              placeholder="Markdown body content"
            />
          </div>

          <div style={styles.fieldGroup}>
            <label style={styles.label}>Canonical Fields (JSON)</label>
            <textarea
              style={{ ...styles.input, minHeight: '3rem', resize: 'vertical' as const, fontFamily: 'monospace' }}
              value={canonicalJson}
              onChange={(e) => setCanonicalJson(e.target.value)}
              placeholder='{"key": "value"}'
            />
          </div>

          {error && <div style={{ color: '#f87171', fontSize: '0.85rem', marginBottom: '0.5rem' }}>{error}</div>}

          <div style={{ display: 'flex', gap: '0.5rem', justifyContent: 'flex-end' }}>
            <button type="button" style={{ ...styles.button, ...styles.buttonSecondary }} onClick={onClose}>
              Cancel
            </button>
            <button type="submit" style={styles.button} disabled={saving || !form.title}>
              {saving ? 'Creating...' : 'Create'}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Delete confirmation
// ---------------------------------------------------------------------------

interface DeleteConfirmProps {
  entity: Entity;
  onClose: () => void;
  onDeleted: () => void;
}

function DeleteConfirm({ entity, onClose, onDeleted }: DeleteConfirmProps) {
  const [deleting, setDeleting] = useState(false);
  const [error, setError] = useState('');

  async function handleDelete() {
    setDeleting(true);
    setError('');
    try {
      await deleteEntity(entity.id);
      onDeleted();
      onClose();
    } catch (e) {
      setError(String(e));
    } finally {
      setDeleting(false);
    }
  }

  return (
    <div style={styles.confirmOverlay} onClick={onClose}>
      <div style={styles.confirmModal} onClick={(e) => e.stopPropagation()}>
        <h3 style={{ margin: '0 0 0.75rem', fontSize: '1.1rem' }}>Delete Entity?</h3>
        <p style={{ fontSize: '0.85rem', opacity: 0.7, margin: '0 0 1rem' }}>
          This will soft-delete <strong>{entity.title}</strong> ({entity.entity_type}).
          The entity can be restored later.
        </p>
        {error && <div style={{ color: '#f87171', fontSize: '0.85rem', marginBottom: '0.5rem' }}>{error}</div>}
        <div style={{ display: 'flex', gap: '0.5rem', justifyContent: 'center' }}>
          <button style={{ ...styles.button, ...styles.buttonSecondary }} onClick={onClose}>
            Cancel
          </button>
          <button
            style={{ ...styles.button, ...styles.buttonDanger }}
            onClick={handleDelete}
            disabled={deleting}
          >
            {deleting ? 'Deleting...' : 'Delete'}
          </button>
        </div>
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Entity detail panel
// ---------------------------------------------------------------------------

function EntityDetail({ entity }: { entity: Entity }) {
  return (
    <div style={styles.detailPanel}>
      <div style={styles.detailRow}>
        <span style={styles.detailLabel}>ID</span>
        <code style={styles.detailValue}>{entity.id}</code>
      </div>
      <div style={styles.detailRow}>
        <span style={styles.detailLabel}>Source</span>
        <span style={styles.detailValue}>{entity.source}</span>
      </div>
      <div style={styles.detailRow}>
        <span style={styles.detailLabel}>Category</span>
        <span style={styles.detailValue}>{entity.category ?? '-'}</span>
      </div>
      <div style={styles.detailRow}>
        <span style={styles.detailLabel}>Priority</span>
        <span style={styles.detailValue}>{entity.priority ?? '-'}</span>
      </div>
      <div style={styles.detailRow}>
        <span style={styles.detailLabel}>Created</span>
        <span style={styles.detailValue}>{new Date(entity.created_at).toLocaleString()}</span>
      </div>
      <div style={styles.detailRow}>
        <span style={styles.detailLabel}>Updated</span>
        <span style={styles.detailValue}>{new Date(entity.updated_at).toLocaleString()}</span>
      </div>
      {entity.due_at && (
        <div style={styles.detailRow}>
          <span style={styles.detailLabel}>Due</span>
          <span style={styles.detailValue}>{new Date(entity.due_at).toLocaleString()}</span>
        </div>
      )}
      {entity.provenance_run_id && (
        <div style={styles.detailRow}>
          <span style={styles.detailLabel}>Run ID</span>
          <code style={styles.detailValue}>{entity.provenance_run_id}</code>
        </div>
      )}
      {entity.body_md && (
        <div style={{ marginTop: '0.5rem' }}>
          <span style={styles.detailLabel}>Body</span>
          <div style={styles.codeBlock}>{entity.body_md}</div>
        </div>
      )}
      {Object.keys(entity.canonical_fields).length > 0 && (
        <div style={{ marginTop: '0.5rem' }}>
          <span style={styles.detailLabel}>Canonical Fields</span>
          <div style={styles.codeBlock}>{JSON.stringify(entity.canonical_fields, null, 2)}</div>
        </div>
      )}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Main component
// ---------------------------------------------------------------------------

interface EntityManagerProps {
  onNavigateToEntity?: (entityId: string) => void;
}

export default function EntityManager({ onNavigateToEntity }: EntityManagerProps) {
  const [entities, setEntities] = useState<Entity[]>([]);
  const [typeFilter, setTypeFilter] = useState('');
  const [statusFilter, setStatusFilter] = useState('');
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState('');
  const [expandedId, setExpandedId] = useState<string | null>(null);
  const [showCreateForm, setShowCreateForm] = useState(false);
  const [deleteTarget, setDeleteTarget] = useState<Entity | null>(null);

  const fetchEntities = useCallback(async () => {
    setLoading(true);
    setError('');
    try {
      const result = await listEntities(typeFilter || undefined);
      setEntities(result);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, [typeFilter]);

  useEffect(() => {
    fetchEntities();
  }, [fetchEntities]);

  // Client-side status filter
  const filtered = statusFilter
    ? entities.filter((e) => e.status === statusFilter)
    : entities;

  // Collect unique statuses for filter dropdown
  const uniqueStatuses = [...new Set(entities.map((e) => e.status).filter(Boolean))] as string[];

  function toggleExpand(id: string) {
    setExpandedId((prev) => (prev === id ? null : id));
  }

  return (
    <div style={styles.container}>
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
        <h2 style={styles.heading}>Entities</h2>
        <span style={styles.count}>{filtered.length} entities</span>
      </div>

      {/* Toolbar */}
      <div style={styles.toolbar}>
        <select
          style={styles.select}
          value={typeFilter}
          onChange={(e) => setTypeFilter(e.target.value)}
        >
          <option value="">All types</option>
          {ENTITY_TYPES.map((t) => (
            <option key={t} value={t}>{t}</option>
          ))}
        </select>

        <select
          style={styles.select}
          value={statusFilter}
          onChange={(e) => setStatusFilter(e.target.value)}
        >
          <option value="">All statuses</option>
          {uniqueStatuses.map((s) => (
            <option key={s} value={s}>{s}</option>
          ))}
        </select>

        <button
          style={{ ...styles.button, ...styles.buttonSecondary }}
          onClick={fetchEntities}
          disabled={loading}
        >
          {loading ? 'Loading...' : 'Refresh'}
        </button>

        <button style={styles.button} onClick={() => setShowCreateForm(true)}>
          + Create Entity
        </button>
      </div>

      {error && <div style={styles.error}>{error}</div>}

      {/* Entity list */}
      <div style={styles.section}>
        <div style={styles.sectionTitle}>
          {typeFilter ? `${typeFilter} entities` : 'All entities'}
          {statusFilter ? ` \u2014 ${statusFilter}` : ''}
        </div>

        {filtered.length === 0 && !loading && (
          <div style={{ padding: '1.5rem', textAlign: 'center', opacity: 0.4, fontSize: '0.9rem' }}>
            No entities found
          </div>
        )}

        {loading && filtered.length === 0 && (
          <div style={{ padding: '1.5rem', textAlign: 'center', opacity: 0.4, fontSize: '0.9rem' }}>
            Loading...
          </div>
        )}

        {filtered.map((entity) => (
          <div key={entity.id}>
            <div
              style={{
                ...styles.entityRow,
                background: expandedId === entity.id ? 'rgba(100,108,255,0.08)' : 'transparent',
              }}
              onClick={() => toggleExpand(entity.id)}
            >
              <TypeBadge type={entity.entity_type} />
              <span style={styles.entityTitle}>{entity.title}</span>
              <StatusBadge status={entity.status} />
              <span style={styles.entityMeta}>
                {new Date(entity.updated_at).toLocaleDateString()}
              </span>
              <button
                style={{ ...styles.button, ...styles.buttonDanger, padding: '0.2rem 0.5rem', fontSize: '0.7rem' }}
                onClick={(e) => {
                  e.stopPropagation();
                  setDeleteTarget(entity);
                }}
              >
                Delete
              </button>
            </div>
            {expandedId === entity.id && <EntityDetail entity={entity} />}
          </div>
        ))}
      </div>

      {/* Create form modal */}
      {showCreateForm && (
        <CreateEntityForm
          onClose={() => setShowCreateForm(false)}
          onCreated={fetchEntities}
        />
      )}

      {/* Delete confirmation modal */}
      {deleteTarget && (
        <DeleteConfirm
          entity={deleteTarget}
          onClose={() => setDeleteTarget(null)}
          onDeleted={fetchEntities}
        />
      )}
    </div>
  );
}

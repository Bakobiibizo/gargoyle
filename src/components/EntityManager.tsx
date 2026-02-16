import { useState, useEffect, useCallback, useRef } from 'react';
import { listEntities, createEntity, deleteEntity, getEntity, updateEntity } from '../api/entities';
import type { Entity, CreateEntityPayload, UpdateEntityPayload } from '../types';
import SearchableSelect from './SearchableSelect';

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
  'audience', 'backlog', 'brief', 'budget', 'campaign', 'channel',
  'competitor', 'decision', 'event', 'experiment', 'metric', 'note',
  'person', 'playbook', 'policy', 'project', 'result', 'session',
  'spec', 'task', 'taxonomy', 'vendor',
];

const STATUS_OPTIONS: Record<string, string[]> = {
  metric: ['active', 'paused', 'deprecated', 'archived'],
  experiment: ['draft', 'running', 'concluded', 'archived'],
  result: ['draft', 'final', 'archived'],
  task: ['backlog', 'todo', 'in_progress', 'blocked', 'done', 'archived'],
  project: ['planning', 'active', 'paused', 'completed', 'archived'],
  decision: ['proposed', 'accepted', 'deprecated', 'superseded'],
  person: ['active', 'inactive', 'archived'],
  note: ['draft', 'final', 'archived'],
  session: ['scheduled', 'in_progress', 'completed', 'cancelled'],
  campaign: ['planning', 'active', 'paused', 'completed', 'archived'],
  audience: ['draft', 'validated', 'active', 'archived'],
  competitor: ['tracking', 'dormant', 'archived'],
  channel: ['evaluating', 'active', 'scaling', 'paused', 'deprecated'],
  spec: ['draft', 'review', 'approved', 'deprecated'],
  budget: ['draft', 'approved', 'active', 'closed'],
  vendor: ['evaluating', 'active', 'on_hold', 'terminated'],
  playbook: ['draft', 'active', 'deprecated', 'archived'],
  taxonomy: ['draft', 'active', 'archived'],
  backlog: ['open', 'triaged', 'scheduled', 'closed'],
  brief: ['draft', 'review', 'approved', 'archived'],
  event: ['proposed', 'confirmed', 'in_progress', 'completed', 'cancelled'],
  policy: ['draft', 'active', 'under_review', 'deprecated'],
};

// ---------------------------------------------------------------------------
// Canonical field templates per entity type
// ---------------------------------------------------------------------------

const CANONICAL_FIELD_TEMPLATES: Record<string, Record<string, unknown>> = {
  metric: { current_value: null, target_value: null, trend: null, data_source: null },
  experiment: { hypothesis: null, funnel_position: null, source_experiment_id: null },
  result: { findings: null, methodology: null, confidence_level: null },
  task: { assignee: null, effort_estimate: null, project_id: null, acceptance_criteria: null },
  project: { owner_id: null, objective: null, success_criteria: null, timeline: null },
  decision: { owner_id: "", rationale: "", decided_at: null, revisit_triggers: null, options_considered: null },
  person: { email: null, role: null, team: null, external: null },
  note: { context: null, tags: null, linked_entity_id: null },
  session: { session_type: null, participants: null, agenda: null, outcomes: null },
  campaign: { objective: null, budget: null, channel: null, start_date: null, end_date: null, target_audience_id: null },
  audience: { segment_criteria: null, estimated_size: null, icp_id: null, channels: null },
  competitor: { website: null, positioning: null, strengths: null, weaknesses: null, market_share: null },
  channel: { channel_type: null, cost_model: null, primary_metric_id: null, budget_allocation: null },
  spec: { spec_type: null, version: null, approval_status: null, author: null },
  budget: { total_amount: null, currency: null, period: null, allocated: null, spent: null },
  vendor: { vendor_type: null, contract_value: null, contract_end: null, primary_contact: null },
  playbook: { playbook_type: null, trigger_conditions: null, expected_outcome: null, owner: null },
  taxonomy: { taxonomy_type: null, parent_id: null, level: null },
  backlog: { priority_score: null, effort: null, requester: null, target_sprint: null },
  brief: { brief_type: null, deadline: null, stakeholders: null, deliverables: null },
  event: { event_type: null, venue: null, start_date: null, end_date: null, expected_attendees: null },
  policy: { policy_type: null, effective_date: null, review_date: null, owner: null },
};

function getCanonicalTemplate(entityType: string): string {
  const template = CANONICAL_FIELD_TEMPLATES[entityType];
  return template ? JSON.stringify(template, null, 2) : '{}';
}

// Entity type options for SearchableSelect
const ENTITY_TYPE_OPTIONS = ENTITY_TYPES.map((t) => ({ value: t, label: t }));

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
  const defaultType = 'audience';
  const [form, setForm] = useState<Partial<CreateEntityPayload>>({
    entity_type: defaultType,
    title: '',
    source: 'manual',
    canonical_fields: {},
    body_md: '',
    status: STATUS_OPTIONS[defaultType][0],
    category: '',
  });
  const [canonicalJson, setCanonicalJson] = useState(getCanonicalTemplate(defaultType));
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
    <div style={styles.formOverlay} onMouseDown={(e) => { if (e.target === e.currentTarget) onClose(); }}>
      <div style={styles.formModal}>
        <h3 style={styles.formTitle}>Create Entity</h3>
        <form onSubmit={handleSubmit}>
          <div style={styles.fieldGroup}>
            <label style={styles.label}>Entity Type</label>
            <SearchableSelect
              options={ENTITY_TYPE_OPTIONS}
              value={form.entity_type ?? defaultType}
              onChange={(newType) => {
                const statuses = STATUS_OPTIONS[newType] ?? [];
                setForm((f) => ({ ...f, entity_type: newType, status: statuses[0] ?? '' }));
                setCanonicalJson(getCanonicalTemplate(newType));
              }}
              placeholder="Search entity types..."
            />
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
            <select
              style={{ ...styles.select, width: '100%' }}
              value={form.status ?? ''}
              onChange={(e) => setForm((f) => ({ ...f, status: e.target.value }))}
            >
              {(STATUS_OPTIONS[form.entity_type ?? ''] ?? []).map((s) => (
                <option key={s} value={s}>{s}</option>
              ))}
            </select>
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
    <div style={styles.confirmOverlay} onMouseDown={(e) => { if (e.target === e.currentTarget) onClose(); }}>
      <div style={styles.confirmModal}>
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
// Edit entity form
// ---------------------------------------------------------------------------

interface EditFormProps {
  entity: Entity;
  onClose: () => void;
  onUpdated: () => void;
}

function EditEntityForm({ entity, onClose, onUpdated }: EditFormProps) {
  const [title, setTitle] = useState(entity.title);
  const [status, setStatus] = useState(entity.status ?? '');
  const [bodyMd, setBodyMd] = useState(entity.body_md ?? '');
  const [category, setCategory] = useState(entity.category ?? '');
  const [canonicalJson, setCanonicalJson] = useState(
    JSON.stringify(entity.canonical_fields, null, 2),
  );
  const [reason, setReason] = useState('');
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState('');

  const statuses = STATUS_OPTIONS[entity.entity_type] ?? [];
  const currentStatusIndex = statuses.indexOf(entity.status ?? '');
  const newStatusIndex = statuses.indexOf(status);
  const isBackwardTransition =
    currentStatusIndex >= 0 && newStatusIndex >= 0 && newStatusIndex < currentStatusIndex;

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    setSaving(true);
    setError('');

    let parsedCanonical: Record<string, unknown> | undefined;
    try {
      parsedCanonical = JSON.parse(canonicalJson);
    } catch {
      setError('Invalid JSON in canonical fields');
      setSaving(false);
      return;
    }

    const payload: UpdateEntityPayload = {
      entity_id: entity.id,
      expected_updated_at: entity.updated_at,
    };

    // Only include fields that changed
    if (title !== entity.title) payload.title = title;
    if (status !== (entity.status ?? '')) payload.status = status;
    if (bodyMd !== (entity.body_md ?? '')) payload.body_md = bodyMd;
    if (category !== (entity.category ?? '')) payload.category = category;
    if (canonicalJson !== JSON.stringify(entity.canonical_fields, null, 2)) {
      payload.canonical_fields = parsedCanonical;
    }
    if (isBackwardTransition && reason) {
      payload.reason = reason;
    }

    try {
      const result = await updateEntity(payload);
      if (result.errors && result.errors.length > 0) {
        setError(result.errors.join('; '));
      } else {
        onUpdated();
        onClose();
      }
    } catch (e) {
      setError(String(e));
    } finally {
      setSaving(false);
    }
  }

  return (
    <div style={styles.formOverlay} onMouseDown={(e) => { if (e.target === e.currentTarget) onClose(); }}>
      <div style={styles.formModal}>
        <h3 style={styles.formTitle}>
          Edit Entity
          <span style={{ marginLeft: '0.5rem' }}>
            <TypeBadge type={entity.entity_type} />
          </span>
        </h3>
        <form onSubmit={handleSubmit}>
          <div style={styles.fieldGroup}>
            <label style={styles.label}>Title</label>
            <input
              style={styles.input}
              value={title}
              onChange={(e) => setTitle(e.target.value)}
              placeholder="Entity title"
              required
            />
          </div>

          <div style={styles.fieldGroup}>
            <label style={styles.label}>Status</label>
            <select
              style={{ ...styles.select, width: '100%' }}
              value={status}
              onChange={(e) => setStatus(e.target.value)}
            >
              {statuses.map((s) => (
                <option key={s} value={s}>{s}</option>
              ))}
            </select>
          </div>

          {isBackwardTransition && (
            <div style={styles.fieldGroup}>
              <label style={{ ...styles.label, color: '#fbbf24' }}>
                Reason for backward status change
              </label>
              <input
                style={styles.input}
                value={reason}
                onChange={(e) => setReason(e.target.value)}
                placeholder="Why is the status moving backward?"
              />
            </div>
          )}

          <div style={styles.fieldGroup}>
            <label style={styles.label}>Category</label>
            <input
              style={styles.input}
              value={category}
              onChange={(e) => setCategory(e.target.value)}
              placeholder="Optional category"
            />
          </div>

          <div style={styles.fieldGroup}>
            <label style={styles.label}>Body (Markdown)</label>
            <textarea
              style={{ ...styles.input, minHeight: '4rem', resize: 'vertical' as const }}
              value={bodyMd}
              onChange={(e) => setBodyMd(e.target.value)}
              placeholder="Markdown body content"
            />
          </div>

          <div style={styles.fieldGroup}>
            <label style={styles.label}>Canonical Fields (JSON)</label>
            <textarea
              style={{ ...styles.input, minHeight: '6rem', resize: 'vertical' as const, fontFamily: 'monospace' }}
              value={canonicalJson}
              onChange={(e) => setCanonicalJson(e.target.value)}
            />
          </div>

          {error && <div style={{ color: '#f87171', fontSize: '0.85rem', marginBottom: '0.5rem' }}>{error}</div>}

          <div style={{ display: 'flex', gap: '0.5rem', justifyContent: 'flex-end' }}>
            <button type="button" style={{ ...styles.button, ...styles.buttonSecondary }} onClick={onClose}>
              Cancel
            </button>
            <button type="submit" style={styles.button} disabled={saving || !title}>
              {saving ? 'Saving...' : 'Save Changes'}
            </button>
          </div>
        </form>
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
  focusEntityId?: string;
  onFocusHandled?: () => void;
}

export default function EntityManager({ onNavigateToEntity, focusEntityId, onFocusHandled }: EntityManagerProps) {
  const [entities, setEntities] = useState<Entity[]>([]);
  const [typeFilter, setTypeFilter] = useState('');
  const [statusFilter, setStatusFilter] = useState('');
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState('');
  const [expandedId, setExpandedId] = useState<string | null>(null);
  const [showCreateForm, setShowCreateForm] = useState(false);
  const [deleteTarget, setDeleteTarget] = useState<Entity | null>(null);
  const [editTarget, setEditTarget] = useState<Entity | null>(null);

  const entityRowRefs = useRef<Record<string, HTMLDivElement | null>>({});

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

  // When focusEntityId is set, expand that entity and scroll it into view
  useEffect(() => {
    if (!focusEntityId) return;

    // Clear filters so the entity is visible
    setTypeFilter('');
    setStatusFilter('');

    // Expand the entity detail panel
    setExpandedId(focusEntityId);

    // Scroll the entity row into view after a brief delay for rendering
    const timer = setTimeout(() => {
      const el = entityRowRefs.current[focusEntityId];
      if (el) {
        el.scrollIntoView({ behavior: 'smooth', block: 'center' });
      }
    }, 100);

    // Notify parent that focus has been handled
    if (onFocusHandled) {
      onFocusHandled();
    }

    return () => clearTimeout(timer);
  }, [focusEntityId, onFocusHandled]);

  // Client-side status filter + alphabetical sort by title
  const filtered = (statusFilter
    ? entities.filter((e) => e.status === statusFilter)
    : entities
  ).slice().sort((a, b) => a.title.localeCompare(b.title));

  // Collect unique statuses for filter dropdown (sorted alphabetically)
  const uniqueStatuses = ([...new Set(entities.map((e) => e.status).filter(Boolean))] as string[]).sort();

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
          <div key={entity.id} ref={(el) => { entityRowRefs.current[entity.id] = el; }}>
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
                style={{ ...styles.button, ...styles.buttonSecondary, padding: '0.2rem 0.5rem', fontSize: '0.7rem' }}
                onClick={(e) => {
                  e.stopPropagation();
                  setEditTarget(entity);
                }}
              >
                Edit
              </button>
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

      {/* Edit form modal */}
      {editTarget && (
        <EditEntityForm
          entity={editTarget}
          onClose={() => setEditTarget(null)}
          onUpdated={fetchEntities}
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

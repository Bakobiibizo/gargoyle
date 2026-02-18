import { useState, useEffect } from 'react';
import {
  listTemplates,
  runTemplate,
  checkPrerequisites,
  type TemplateDefinition,
  type PrerequisiteResult,
  type TemplateOutput,
} from '../api/templates';
import SearchableSelect from './SearchableSelect';

// ---------------------------------------------------------------------------
// Styles
// ---------------------------------------------------------------------------

const styles = {
  container: {
    display: 'flex',
    flexDirection: 'column' as const,
    gap: '1.5rem',
    padding: '1.5rem',
    height: '100%',
    overflow: 'auto',
  },
  heading: {
    margin: 0,
    fontSize: '1.5rem',
    fontWeight: 600,
  },
  section: {
    background: 'rgba(255,255,255,0.04)',
    borderRadius: 8,
    padding: '1rem',
    border: '1px solid rgba(255,255,255,0.08)',
  },
  sectionTitle: {
    margin: '0 0 0.75rem',
    fontSize: '0.875rem',
    fontWeight: 600,
    textTransform: 'uppercase' as const,
    letterSpacing: '0.05em',
    opacity: 0.6,
  },
  select: {
    width: '100%',
    padding: '0.5rem 0.75rem',
    borderRadius: 6,
    border: '1px solid rgba(255,255,255,0.15)',
    background: 'rgba(0,0,0,0.3)',
    color: 'inherit',
    fontSize: '0.9rem',
    fontFamily: 'inherit',
  },
  input: {
    width: '100%',
    padding: '0.5rem 0.75rem',
    borderRadius: 6,
    border: '1px solid rgba(255,255,255,0.15)',
    background: 'rgba(0,0,0,0.3)',
    color: 'inherit',
    fontSize: '0.9rem',
    fontFamily: 'inherit',
    boxSizing: 'border-box' as const,
  },
  label: {
    display: 'block',
    marginBottom: '0.25rem',
    fontSize: '0.8rem',
    opacity: 0.7,
  },
  fieldGroup: {
    marginBottom: '0.75rem',
  },
  button: {
    padding: '0.6rem 1.5rem',
    borderRadius: 6,
    border: 'none',
    background: '#646cff',
    color: '#fff',
    fontSize: '0.9rem',
    fontWeight: 500,
    cursor: 'pointer',
    fontFamily: 'inherit',
  },
  buttonDisabled: {
    opacity: 0.5,
    cursor: 'not-allowed',
  },
  badge: (color: string) => ({
    display: 'inline-block',
    padding: '0.15rem 0.5rem',
    borderRadius: 4,
    fontSize: '0.75rem',
    fontWeight: 600,
    background: color,
    color: '#fff',
    marginRight: '0.5rem',
  }),
  prereqOk: {
    color: '#4ade80',
    fontSize: '0.85rem',
  },
  prereqFail: {
    color: '#f87171',
    fontSize: '0.85rem',
  },
  error: {
    color: '#f87171',
    padding: '0.5rem',
    fontSize: '0.85rem',
  },
  warning: {
    color: '#fbbf24',
    fontSize: '0.85rem',
  },
  outputBlock: {
    marginTop: '0.5rem',
    padding: '0.75rem',
    background: 'rgba(0,0,0,0.2)',
    borderRadius: 6,
    fontSize: '0.85rem',
    fontFamily: 'monospace',
    whiteSpace: 'pre-wrap' as const,
    wordBreak: 'break-word' as const,
    maxHeight: '20rem',
    overflow: 'auto',
  },
  historyItem: {
    padding: '0.5rem 0.75rem',
    borderBottom: '1px solid rgba(255,255,255,0.06)',
    cursor: 'pointer',
    fontSize: '0.85rem',
  },
  historyItemHover: {
    background: 'rgba(255,255,255,0.04)',
  },
  templateMeta: {
    display: 'flex',
    gap: '0.5rem',
    alignItems: 'center',
    marginTop: '0.5rem',
    fontSize: '0.8rem',
    opacity: 0.6,
  },
  forceCheckbox: {
    display: 'flex',
    alignItems: 'center',
    gap: '0.5rem',
    fontSize: '0.85rem',
    marginBottom: '0.5rem',
  },
};

// ---------------------------------------------------------------------------
// Param configuration per template key (defines what the user fills in)
// ---------------------------------------------------------------------------

const TEMPLATE_PARAMS: Record<string, { key: string; label: string; placeholder: string }[]> = {
  'analytics-metric-tree': [
    { key: 'north_star', label: 'North-star metric name', placeholder: 'e.g. Monthly Active Users' },
    { key: 'sub_metrics', label: 'Sub-metrics (comma-separated)', placeholder: 'e.g. DAU, Retention Rate, Revenue' },
  ],
  'analytics-experiment-plan': [
    { key: 'metric_id', label: 'Metric entity ID', placeholder: 'UUID of the metric to experiment on' },
    { key: 'hypothesis', label: 'Hypothesis', placeholder: 'If we ... then ...' },
  ],
  'analytics-anomaly-detection-investigation': [
    { key: 'experiment_id', label: 'Experiment entity ID', placeholder: 'UUID of the experiment' },
    { key: 'anomaly_description', label: 'Anomaly description', placeholder: 'Describe the anomaly observed' },
  ],
  'mkt-icp-definition': [
    { key: 'company_name', label: 'Company name', placeholder: 'Your company name' },
    { key: 'target_market', label: 'Target market', placeholder: 'e.g. B2B SaaS mid-market' },
  ],
  'mkt-competitive-intel': [
    { key: 'competitor_name', label: 'Competitor name', placeholder: 'e.g. Acme Corp' },
    { key: 'website', label: 'Competitor website', placeholder: 'https://...' },
  ],
  'mkt-positioning-narrative': [
    { key: 'person_id', label: 'Person (ICP) entity ID', placeholder: 'UUID of the person entity' },
    { key: 'value_proposition', label: 'Value proposition', placeholder: 'We help X do Y by Z' },
  ],
};

// ---------------------------------------------------------------------------
// Types for run history
// ---------------------------------------------------------------------------

interface RunHistoryEntry {
  runId: string;
  templateKey: string;
  timestamp: string;
  output: TemplateOutput;
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

export default function TemplateRunner() {
  const [templates, setTemplates] = useState<TemplateDefinition[]>([]);
  const [selectedKey, setSelectedKey] = useState('');
  const [params, setParams] = useState<Record<string, string>>({});
  const [force, setForce] = useState(false);
  const [prereqs, setPrereqs] = useState<PrerequisiteResult[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState('');
  const [output, setOutput] = useState<TemplateOutput | null>(null);
  const [history, setHistory] = useState<RunHistoryEntry[]>([]);
  const [selectedHistoryIdx, setSelectedHistoryIdx] = useState<number | null>(null);

  // Fetch templates on mount
  useEffect(() => {
    listTemplates()
      .then(setTemplates)
      .catch((e) => setError(String(e)));
  }, []);

  // When selected template changes, check prerequisites
  useEffect(() => {
    if (!selectedKey) {
      setPrereqs([]);
      return;
    }
    setError('');
    setOutput(null);
    setParams({});
    setSelectedHistoryIdx(null);
    checkPrerequisites(selectedKey)
      .then(setPrereqs)
      .catch((e) => setError(String(e)));
  }, [selectedKey]);

  const selectedTemplate = templates.find((t) => t.key === selectedKey);
  const paramFields = TEMPLATE_PARAMS[selectedKey] ?? [];
  const allPrereqsSatisfied = prereqs.length === 0 || prereqs.every((p) => p.satisfied);

  function handleParamChange(key: string, value: string) {
    setParams((prev) => ({ ...prev, [key]: value }));
  }

  async function handleRun() {
    if (!selectedKey) return;
    setLoading(true);
    setError('');
    setOutput(null);
    try {
      // Convert comma-separated values to arrays for certain keys
      const processedParams: Record<string, unknown> = {};
      for (const [k, v] of Object.entries(params)) {
        if (v.includes(',') && k !== 'hypothesis' && k !== 'anomaly_description' && k !== 'value_proposition') {
          processedParams[k] = v.split(',').map((s) => s.trim()).filter(Boolean);
        } else {
          processedParams[k] = v;
        }
      }

      const result = await runTemplate({
        template_key: selectedKey,
        params: processedParams,
        force,
      });
      setOutput(result);

      // Add to history
      setHistory((prev) => [
        {
          runId: result.run_id,
          templateKey: selectedKey,
          timestamp: new Date().toISOString(),
          output: result,
        },
        ...prev,
      ]);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }

  function handleHistoryClick(idx: number) {
    setSelectedHistoryIdx(idx);
    setOutput(history[idx].output);
  }

  return (
    <div style={styles.container}>
      <h2 style={styles.heading}>Template Runner</h2>

      {/* Template selector */}
      <div style={styles.section}>
        <div style={styles.sectionTitle}>Select Template</div>
        <SearchableSelect
          options={[...templates].sort((a, b) => a.key.localeCompare(b.key)).map((t) => ({
            value: t.key,
            label: `[${t.category}] ${t.key} (v${t.version})`,
          }))}
          value={selectedKey}
          onChange={setSelectedKey}
          placeholder="Search templates..."
        />

        {selectedTemplate && (
          <div style={styles.templateMeta}>
            <span style={styles.badge('#646cff')}>{selectedTemplate.category}</span>
            <span>v{selectedTemplate.version}</span>
            {selectedTemplate.prerequisites.length > 0 && (
              <span>
                | Requires:{' '}
                {selectedTemplate.prerequisites.map((p) => `${p.min_count}x ${p.entity_type}`).join(', ')}
              </span>
            )}
          </div>
        )}
      </div>

      {/* Prerequisites status */}
      {selectedKey && prereqs.length > 0 && (
        <div style={styles.section}>
          <div style={styles.sectionTitle}>Prerequisites</div>
          {prereqs.map((p, i) => (
            <div key={i} style={p.satisfied ? styles.prereqOk : styles.prereqFail}>
              {p.satisfied ? '\u2713' : '\u2717'} {p.message ?? (p.satisfied ? 'Satisfied' : 'Not satisfied')}
            </div>
          ))}
        </div>
      )}

      {/* Dynamic input form */}
      {selectedKey && (
        <div style={styles.section}>
          <div style={styles.sectionTitle}>Parameters</div>

          {paramFields.length === 0 && (
            <div style={{ fontSize: '0.85rem', opacity: 0.5 }}>
              No custom parameters defined for this template. You can add JSON params below.
            </div>
          )}

          {paramFields.map((field) => (
            <div key={field.key} style={styles.fieldGroup}>
              <label style={styles.label}>{field.label}</label>
              <input
                style={styles.input}
                placeholder={field.placeholder}
                value={params[field.key] ?? ''}
                onChange={(e) => handleParamChange(field.key, e.target.value)}
              />
            </div>
          ))}

          {/* Generic JSON param for templates without predefined fields */}
          {paramFields.length === 0 && (
            <div style={styles.fieldGroup}>
              <label style={styles.label}>JSON Parameters (optional)</label>
              <textarea
                style={{ ...styles.input, minHeight: '4rem', resize: 'vertical' as const }}
                placeholder='{"key": "value"}'
                value={params['__raw_json'] ?? ''}
                onChange={(e) => handleParamChange('__raw_json', e.target.value)}
              />
            </div>
          )}

          <div style={styles.forceCheckbox}>
            <input
              type="checkbox"
              id="force-run"
              checked={force}
              onChange={(e) => setForce(e.target.checked)}
            />
            <label htmlFor="force-run">Force run (skip prerequisite checks)</label>
          </div>

          <button
            style={{
              ...styles.button,
              ...(loading || (!allPrereqsSatisfied && !force) ? styles.buttonDisabled : {}),
            }}
            disabled={loading || (!allPrereqsSatisfied && !force)}
            onClick={handleRun}
          >
            {loading ? 'Running...' : 'Run Template'}
          </button>
        </div>
      )}

      {/* Error display */}
      {error && (
        <div style={styles.section}>
          <div style={styles.error}>{error}</div>
        </div>
      )}

      {/* Output view */}
      {output && (
        <div style={styles.section}>
          <div style={styles.sectionTitle}>Output</div>

          <div style={{ fontSize: '0.85rem', marginBottom: '0.5rem' }}>
            <strong>Run ID:</strong> <code>{output.run_id}</code>
          </div>

          {output.warnings.length > 0 && (
            <div style={{ marginBottom: '0.5rem' }}>
              {output.warnings.map((w, i) => (
                <div key={i} style={styles.warning}>Warning: {w}</div>
              ))}
            </div>
          )}

          <div style={{ fontSize: '0.85rem', marginBottom: '0.25rem' }}>
            <strong>Applied operations:</strong> {output.patch_result.applied.length}
          </div>

          {output.patch_result.applied.length > 0 && (
            <div style={styles.outputBlock}>
              {output.patch_result.applied.map((op, i) => (
                <div key={i}>
                  #{op.op_index}
                  {op.entity_id && ` entity: ${op.entity_id}`}
                  {op.relation_id && ` relation: ${op.relation_id}`}
                  {op.claim_id && ` claim: ${op.claim_id}`}
                </div>
              ))}
            </div>
          )}

          {output.patch_result.errors.length > 0 && (
            <div style={{ marginTop: '0.5rem' }}>
              <div style={{ ...styles.sectionTitle, color: '#f87171' }}>Errors</div>
              {output.patch_result.errors.map((err, i) => (
                <div key={i} style={styles.error}>{err}</div>
              ))}
            </div>
          )}
        </div>
      )}

      {/* Run history */}
      {history.length > 0 && (
        <div style={styles.section}>
          <div style={styles.sectionTitle}>Run History ({history.length})</div>
          {history.map((entry, i) => (
            <div
              key={entry.runId}
              style={{
                ...styles.historyItem,
                background: selectedHistoryIdx === i ? 'rgba(100,108,255,0.15)' : 'transparent',
              }}
              onClick={() => handleHistoryClick(i)}
            >
              <span style={styles.badge('#646cff')}>{entry.templateKey}</span>
              <span style={{ opacity: 0.5 }}>
                {new Date(entry.timestamp).toLocaleTimeString()}
              </span>
              <span style={{ marginLeft: '0.5rem' }}>
                {entry.output.patch_result.applied.length} ops
                {entry.output.patch_result.errors.length > 0 &&
                  `, ${entry.output.patch_result.errors.length} errors`}
              </span>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

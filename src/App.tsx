import { useState } from 'react';
import EntityManager from './components/EntityManager';
import TemplateRunner from './components/TemplateRunner';
import GraphExplorer from './components/GraphExplorer';
import SearchBar from './components/SearchBar';
import DedupDashboard from './components/DedupDashboard';
import WorkflowMap from './components/WorkflowMap';

// ---------------------------------------------------------------------------
// Tab definitions
// ---------------------------------------------------------------------------

type TabId = 'entities' | 'templates' | 'graph' | 'search' | 'dedup' | 'workflows';

interface TabDef {
  id: TabId;
  label: string;
  icon: string; // simple text/emoji-free icon
}

const TABS: TabDef[] = [
  { id: 'entities', label: 'Entities', icon: '\u25A3' },   // filled square with inner square
  { id: 'templates', label: 'Templates', icon: '\u25B7' }, // white right-pointing triangle
  { id: 'graph', label: 'Graph', icon: '\u2B2F' },         // three dots connected
  { id: 'search', label: 'Search', icon: '\u2315' },       // telephone recorder / search-like
  { id: 'dedup', label: 'Dedup', icon: '\u2A61' },         // small contains with overline
  { id: 'workflows', label: 'Workflows', icon: '\u2B95' },  // rightwards black arrow
];

// ---------------------------------------------------------------------------
// Styles
// ---------------------------------------------------------------------------

const styles = {
  layout: {
    display: 'flex',
    height: '100vh',
    overflow: 'hidden',
    fontFamily: 'system-ui, -apple-system, sans-serif',
  },
  sidebar: {
    width: '12rem',
    minWidth: '12rem',
    background: 'rgba(0,0,0,0.25)',
    borderRight: '1px solid rgba(255,255,255,0.06)',
    display: 'flex',
    flexDirection: 'column' as const,
    padding: '0.75rem 0',
    gap: '0.25rem',
  },
  logo: {
    padding: '0.5rem 1rem 1rem',
    fontSize: '1.1rem',
    fontWeight: 700,
    letterSpacing: '0.04em',
    borderBottom: '1px solid rgba(255,255,255,0.06)',
    marginBottom: '0.5rem',
  },
  logoSub: {
    fontSize: '0.65rem',
    fontWeight: 400,
    opacity: 0.35,
    display: 'block',
    marginTop: '0.15rem',
    letterSpacing: '0.02em',
  },
  navItem: (active: boolean) => ({
    display: 'flex',
    alignItems: 'center',
    gap: '0.6rem',
    padding: '0.55rem 1rem',
    margin: '0 0.5rem',
    borderRadius: 6,
    cursor: 'pointer',
    fontSize: '0.85rem',
    fontWeight: active ? 600 : 400,
    background: active ? 'rgba(100,108,255,0.15)' : 'transparent',
    color: active ? '#a5b4fc' : 'inherit',
    transition: 'background 0.15s, color 0.15s',
    border: 'none',
    fontFamily: 'inherit',
    textAlign: 'left' as const,
    width: 'auto',
  }),
  navIcon: {
    width: '1.2rem',
    textAlign: 'center' as const,
    fontSize: '1rem',
    opacity: 0.7,
  },
  main: {
    flex: 1,
    display: 'flex',
    flexDirection: 'column' as const,
    overflow: 'hidden',
  },
  header: {
    display: 'flex',
    alignItems: 'center',
    gap: '1rem',
    padding: '0.75rem 1.5rem',
    borderBottom: '1px solid rgba(255,255,255,0.06)',
    background: 'rgba(0,0,0,0.1)',
    minHeight: '2.5rem',
  },
  headerTitle: {
    fontSize: '0.9rem',
    fontWeight: 600,
    opacity: 0.5,
    whiteSpace: 'nowrap' as const,
  },
  searchWrapper: {
    flex: 1,
    maxWidth: '24rem',
  },
  content: {
    flex: 1,
    overflow: 'auto',
  },
  searchFullPage: {
    display: 'flex',
    flexDirection: 'column' as const,
    padding: '1.5rem',
    gap: '1rem',
    height: '100%',
    overflow: 'auto',
  },
  searchHeading: {
    margin: 0,
    fontSize: '1.5rem',
    fontWeight: 600,
  },
  searchSection: {
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
  searchHint: {
    fontSize: '0.85rem',
    opacity: 0.4,
    textAlign: 'center' as const,
    padding: '2rem',
  },
};

// ---------------------------------------------------------------------------
// App component
// ---------------------------------------------------------------------------

function App() {
  const [activeTab, setActiveTab] = useState<TabId>('entities');
  const [graphEntityId, setGraphEntityId] = useState<string | undefined>();

  // Navigate to entity detail by switching tabs
  function handleNavigateToEntity(entityId: string) {
    // Switch to graph view centered on this entity
    setGraphEntityId(entityId);
    setActiveTab('graph');
  }

  // Handle search result selection from the header bar
  function handleSearchSelect(entityId: string) {
    handleNavigateToEntity(entityId);
  }

  return (
    <div style={styles.layout}>
      {/* Sidebar */}
      <nav style={styles.sidebar}>
        <div style={styles.logo}>
          Gargoyle
          <span style={styles.logoSub}>Knowledge Graph</span>
        </div>

        {TABS.map((tab) => (
          <button
            key={tab.id}
            style={styles.navItem(activeTab === tab.id)}
            onClick={() => setActiveTab(tab.id)}
          >
            <span style={styles.navIcon}>{tab.icon}</span>
            {tab.label}
          </button>
        ))}
      </nav>

      {/* Main area */}
      <div style={styles.main}>
        {/* Header with search bar */}
        <div style={styles.header}>
          <span style={styles.headerTitle}>
            {TABS.find((t) => t.id === activeTab)?.label}
          </span>
          <div style={styles.searchWrapper}>
            <SearchBar
              onSelectResult={handleSearchSelect}
              placeholder="Quick search..."
            />
          </div>
        </div>

        {/* Content area */}
        <div style={styles.content}>
          {activeTab === 'entities' && (
            <EntityManager onNavigateToEntity={handleNavigateToEntity} />
          )}

          {activeTab === 'templates' && (
            <TemplateRunner />
          )}

          {activeTab === 'graph' && (
            <GraphExplorer
              onNavigateToEntity={handleNavigateToEntity}
              initialEntityId={graphEntityId}
            />
          )}

          {activeTab === 'search' && (
            <SearchFullPage onSelectResult={handleNavigateToEntity} />
          )}

          {activeTab === 'dedup' && (
            <DedupDashboard />
          )}

          {activeTab === 'workflows' && (
            <WorkflowMap />
          )}
        </div>
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Dedicated search page (full-page variant)
// ---------------------------------------------------------------------------

function SearchFullPage({ onSelectResult }: { onSelectResult: (id: string) => void }) {
  const [selectedId, setSelectedId] = useState<string | null>(null);

  function handleSelect(entityId: string) {
    setSelectedId(entityId);
    onSelectResult(entityId);
  }

  return (
    <div style={styles.searchFullPage}>
      <h2 style={styles.searchHeading}>Search</h2>

      <div style={styles.searchSection}>
        <div style={styles.sectionTitle}>Full-text Search</div>
        <SearchBar
          onSelectResult={handleSelect}
          placeholder="Type to search entities by title or content..."
        />
      </div>

      <div style={{ ...styles.searchHint }}>
        Type at least 2 characters to search. Results will appear in a dropdown.
        {selectedId && (
          <div style={{ marginTop: '0.5rem', opacity: 0.6 }}>
            Last selected: <code>{selectedId}</code>
          </div>
        )}
      </div>
    </div>
  );
}

export default App;

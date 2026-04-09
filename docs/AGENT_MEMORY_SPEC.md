# Agent Memory System Specification

## Overview

A persistent memory system that allows the assistant to learn and remember context across conversations. The agent cannot access its memory directly—instead, a **MemoryAgent** subagent curates and surfaces relevant context into the working context window as needed.

**Core Principle**: The context window is precious real estate. Every token must earn its place.

---

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                     PRIMARY CHAT AGENT                          │
│              (Cannot access memory directly)                    │
└─────────────────────────────┬───────────────────────────────────┘
                              │
                    Context Stream (injected)
                              │
┌─────────────────────────────▼───────────────────────────────────┐
│                      MEMORY AGENT                               │
│         (Curates, retrieves, and manages memories)              │
│                                                                 │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────────┐ │
│  │  Retriever  │  │  Curator    │  │  Background Processor   │ │
│  │  (search)   │  │  (prune)    │  │  (batch, summarize)     │ │
│  └─────────────┘  └─────────────┘  └─────────────────────────┘ │
└─────────────────────────────┬───────────────────────────────────┘
                              │
         ┌────────────────────┼────────────────────┐
         │                    │                    │
         ▼                    ▼                    ▼
┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐
│  Short-Term     │  │   Long-Term     │  │   Vector        │
│  Memory (SQLite)│  │   Memory (Graph)│  │   Store         │
│                 │  │                 │  │   (Embeddings)  │
│  - Recent       │  │  - Persistent   │  │  - Semantic     │
│  - Time-boxed   │  │  - Indexed/FTS  │  │  - Similarity   │
│  - Hopper       │  │  - Pruned       │  │  - Retrieval    │
└─────────────────┘  └─────────────────┘  └─────────────────┘
```

---

## Context Hierarchy

Context is structured at multiple granularities for efficient processing:

| Level | Definition | Typical Size | Processing |
|-------|------------|--------------|------------|
| **ContextMessage** | Single user or assistant message | 1-500 tokens | Real-time |
| **ContextSegment** | User request + agent response(s) including tool calls | 500-5000 tokens | Per-turn |
| **ContextThread** | Last N segments (rolling window) | ~10 segments | Active session |
| **ContextConversation** | Full conversation session | Unbounded | Chunked/async |

```rust
struct ContextMessage {
    role: Role,           // user, assistant, system, tool
    content: String,
    timestamp: DateTime,
    tokens: usize,
}

struct ContextSegment {
    user_message: ContextMessage,
    agent_responses: Vec<ContextMessage>,  // Can include tool calls
    tool_results: Vec<ToolResult>,
    segment_id: Uuid,
    started_at: DateTime,
    completed_at: DateTime,
}

struct ContextThread {
    segments: VecDeque<ContextSegment>,  // Rolling window, max N
    conversation_id: Uuid,
}

struct ContextConversation {
    id: Uuid,
    started_at: DateTime,
    ended_at: Option<DateTime>,
    segments: Vec<ContextSegment>,
    summary: Option<String>,  // Generated after conversation ends
}
```

---

## Memory Tiers

### 1. Context Memory (In-Context)

**Purpose**: The active working memory—what's in the LLM's context window right now.

**Characteristics**:
- Most precious space, every token must be justified
- Rolling window with intelligent eviction
- Augmented with surfaced memories from other tiers

**Management**:
- Older segments fall off as new ones arrive
- Important context is summarized before eviction
- MemoryAgent can inject relevant memories from other tiers

```rust
struct ContextMemory {
    system_prompt: String,
    surfaced_memories: Vec<SurfacedMemory>,  // Injected by MemoryAgent
    thread: ContextThread,
    max_tokens: usize,  // e.g., 128k - reserve for response
}

struct SurfacedMemory {
    content: String,
    source: MemorySource,  // ShortTerm, LongTerm, Both
    relevance_score: f32,
    surfaced_at: DateTime,
}
```

### 2. Short-Term Memory (Hopper)

**Purpose**: Recent memories awaiting promotion to long-term storage.

**Characteristics**:
- Time-boxed (e.g., last 24-72 hours or last N conversations)
- Searchable by keyword and timestamp
- Acts as a staging area before long-term storage

**Lifecycle**:
1. Created when ContextSegment is processed
2. Indexed for keyword search
3. After time threshold → evaluated for long-term promotion
4. Low-value memories pruned, high-value promoted

```rust
struct ShortTermMemory {
    id: Uuid,
    content: String,
    summary: String,
    keywords: Vec<String>,
    source_segment_id: Uuid,
    source_conversation_id: Uuid,
    created_at: DateTime,
    importance_score: f32,
    access_count: u32,  // How often retrieved
}
```

**Storage**: SQLite table with FTS index on content/keywords

### 3. Long-Term Memory (Persistent)

**Purpose**: Durable memories that persist across time, regularly curated.

**Characteristics**:
- Dual-indexed: traditional FTS + vector embeddings
- Graph-connected to related memories and entities
- Regularly reviewed and pruned for relevance

**Structure**:
```rust
struct LongTermMemory {
    id: Uuid,
    content: String,
    summary: String,
    
    // Indexing
    keywords: Vec<String>,
    embedding: Vec<f32>,  // For semantic search
    
    // Provenance
    source_conversations: Vec<Uuid>,
    created_at: DateTime,
    last_accessed: DateTime,
    access_count: u32,
    
    // Curation
    importance_score: f32,
    decay_rate: f32,      // How quickly relevance decays
    last_reviewed: DateTime,
    
    // Graph connections
    related_memories: Vec<Uuid>,
    related_entities: Vec<Uuid>,  // Links to canonical context
}
```

**Retrieval Scoring**:
When a memory matches both vector search AND keyword search, it receives a boost:

```rust
fn compute_retrieval_score(
    vector_score: Option<f32>,
    keyword_score: Option<f32>,
    recency_factor: f32,
    access_frequency: f32,
) -> f32 {
    let base = match (vector_score, keyword_score) {
        (Some(v), Some(k)) => (v + k) * 1.5,  // Boost for dual match
        (Some(v), None) => v,
        (None, Some(k)) => k,
        (None, None) => 0.0,
    };
    base * recency_factor * (1.0 + access_frequency.ln())
}
```

---

## Memory Agent Operations

### Background Processing Pipeline

```
┌─────────────────────────────────────────────────────────────────┐
│                    PROCESSING TRIGGERS                          │
├─────────────────────────────────────────────────────────────────┤
│  Per-Segment    │  End-of-Conversation  │  Scheduled (Daily)    │
│  - Extract KW   │  - Summarize convo    │  - Review STM→LTM     │
│  - Quick eval   │  - Batch to STM       │  - Prune low-value    │
│  - Flag special │  - Update graphs      │  - Consolidate dupes  │
└─────────────────────────────────────────────────────────────────┘
```

#### 1. Per-Segment Processing (Real-time, Background)

Triggered after each ContextSegment completes:

```rust
async fn process_segment(segment: &ContextSegment) -> SegmentAnalysis {
    // 1. Extract keywords and entities
    let keywords = extract_keywords(&segment);
    let entities = extract_entity_mentions(&segment);
    
    // 2. Generate brief summary
    let summary = summarize_segment(&segment);
    
    // 3. Evaluate importance
    let importance = evaluate_importance(&segment, &keywords);
    
    // 4. Flag for special treatment if high importance
    if importance > IMPORTANCE_THRESHOLD {
        flag_for_batch_processing(&segment);
    }
    
    // 5. Store in short-term memory
    store_short_term_memory(ShortTermMemory {
        content: segment.to_string(),
        summary,
        keywords,
        importance_score: importance,
        // ...
    });
    
    // 6. Search for relevant context (async, doesn't block)
    spawn(search_relevant_context(&keywords, &entities));
}
```

#### 2. End-of-Conversation Processing

Triggered when conversation ends or times out:

```rust
async fn process_conversation_end(conversation: &ContextConversation) {
    // 1. Generate conversation summary
    let summary = generate_conversation_summary(&conversation);
    
    // 2. Identify key memories from this conversation
    let key_memories = identify_key_memories(&conversation);
    
    // 3. Update memory graph connections
    update_memory_graph(&key_memories);
    
    // 4. Link to canonical entities
    link_to_entities(&key_memories);
    
    // 5. Store conversation metadata
    store_conversation_record(conversation, summary);
}
```

#### 3. Scheduled Maintenance (Daily/Periodic)

```rust
async fn scheduled_memory_maintenance() {
    // 1. Promote aged short-term memories
    let candidates = get_stm_promotion_candidates();
    for memory in candidates {
        if should_promote(&memory) {
            promote_to_long_term(memory);
        } else {
            prune_memory(memory);
        }
    }
    
    // 2. Review and prune long-term memories
    let review_candidates = get_ltm_review_candidates();
    for memory in review_candidates {
        let new_score = recalculate_importance(&memory);
        if new_score < PRUNE_THRESHOLD {
            archive_or_delete(memory);
        } else {
            update_importance(memory, new_score);
        }
    }
    
    // 3. Consolidate duplicate/similar memories
    consolidate_similar_memories();
    
    // 4. Update embeddings for modified memories
    refresh_stale_embeddings();
}
```

### Retrieval Pipeline

When MemoryAgent determines context is needed:

```rust
async fn retrieve_relevant_memories(
    query: &str,
    current_context: &ContextMemory,
    max_tokens: usize,
) -> Vec<SurfacedMemory> {
    // 1. Generate query embedding
    let query_embedding = embed(query);
    
    // 2. Parallel search across stores
    let (vector_results, keyword_results, recent_results) = join!(
        search_vector_store(&query_embedding, VECTOR_LIMIT),
        search_fts(&query, FTS_LIMIT),
        search_recent_stm(&query, RECENT_LIMIT),
    );
    
    // 3. Merge and score results
    let mut candidates = merge_results(vector_results, keyword_results, recent_results);
    
    // 4. Re-rank based on current context
    rerank_by_context(&mut candidates, current_context);
    
    // 5. Deduplicate and filter
    let filtered = deduplicate_and_filter(candidates);
    
    // 6. Pack into token budget
    pack_memories(filtered, max_tokens)
}
```

---

## Context Injection Strategy

The MemoryAgent injects memories into the context stream at strategic points:

### Injection Points

1. **Conversation Start**: Inject user profile, recent conversation summaries
2. **Topic Shift Detected**: Search for relevant memories on new topic
3. **Entity Mentioned**: Surface related memories for mentioned entities
4. **Agent Request**: When agent explicitly needs more context
5. **Periodic Refresh**: Every N segments, refresh surfaced memories

### Injection Format

```
[MEMORY CONTEXT - Updated {timestamp}]

## Recent Context
- Yesterday: Discussed project timeline, user prefers morning meetings
- 3 days ago: Reviewed Q4 metrics, revenue up 12%

## Relevant Memories
- User preference: Formal communication style in external docs
- Project "Alpha": Deadline March 15, stakeholders: Sarah, Mike

## Related Entities
- Sarah Chen (PM): Last discussed Jan 15, working on Alpha
- Q4 Metrics: Revenue $1.2M, +12% QoQ

[END MEMORY CONTEXT]
```

---

## Database Schema

### Short-Term Memory Table

```sql
CREATE TABLE short_term_memories (
    id TEXT PRIMARY KEY,
    content TEXT NOT NULL,
    summary TEXT NOT NULL,
    keywords TEXT NOT NULL,  -- JSON array
    
    source_segment_id TEXT NOT NULL,
    source_conversation_id TEXT NOT NULL,
    
    importance_score REAL DEFAULT 0.5,
    access_count INTEGER DEFAULT 0,
    
    created_at TEXT NOT NULL,
    expires_at TEXT NOT NULL,  -- When to evaluate for promotion
    
    FOREIGN KEY (source_conversation_id) REFERENCES conversations(id)
);

CREATE INDEX idx_stm_expires ON short_term_memories(expires_at);
CREATE INDEX idx_stm_importance ON short_term_memories(importance_score DESC);

-- FTS for keyword search
CREATE VIRTUAL TABLE short_term_memories_fts USING fts5(
    content, summary, keywords,
    content='short_term_memories',
    content_rowid='rowid'
);
```

### Long-Term Memory Table

```sql
CREATE TABLE long_term_memories (
    id TEXT PRIMARY KEY,
    content TEXT NOT NULL,
    summary TEXT NOT NULL,
    keywords TEXT NOT NULL,  -- JSON array
    
    -- Embeddings stored separately or inline as BLOB
    embedding BLOB,
    
    importance_score REAL DEFAULT 0.5,
    decay_rate REAL DEFAULT 0.01,
    access_count INTEGER DEFAULT 0,
    
    created_at TEXT NOT NULL,
    last_accessed TEXT,
    last_reviewed TEXT,
    
    -- Graph connections stored in relations table
);

CREATE VIRTUAL TABLE long_term_memories_fts USING fts5(
    content, summary, keywords,
    content='long_term_memories',
    content_rowid='rowid'
);
```

### Memory Relations

```sql
CREATE TABLE memory_relations (
    id TEXT PRIMARY KEY,
    source_memory_id TEXT NOT NULL,
    target_memory_id TEXT,
    target_entity_id TEXT,
    relation_type TEXT NOT NULL,  -- 'related_to', 'derived_from', 'supersedes', 'mentions'
    strength REAL DEFAULT 1.0,
    created_at TEXT NOT NULL,
    
    CHECK (target_memory_id IS NOT NULL OR target_entity_id IS NOT NULL)
);
```

### Conversations Table

```sql
CREATE TABLE conversations (
    id TEXT PRIMARY KEY,
    started_at TEXT NOT NULL,
    ended_at TEXT,
    summary TEXT,
    segment_count INTEGER DEFAULT 0,
    total_tokens INTEGER DEFAULT 0
);

CREATE TABLE conversation_segments (
    id TEXT PRIMARY KEY,
    conversation_id TEXT NOT NULL,
    sequence_num INTEGER NOT NULL,
    user_message TEXT NOT NULL,
    agent_responses TEXT NOT NULL,  -- JSON array
    tool_calls TEXT,  -- JSON array
    started_at TEXT NOT NULL,
    completed_at TEXT,
    tokens INTEGER,
    
    FOREIGN KEY (conversation_id) REFERENCES conversations(id)
);
```

---

## Configuration

```toml
[memory]
# Context window management
max_context_tokens = 100000
reserved_response_tokens = 8000
thread_window_segments = 10

# Short-term memory
stm_retention_hours = 72
stm_max_entries = 1000
stm_promotion_threshold = 0.7

# Long-term memory
ltm_prune_threshold = 0.3
ltm_review_interval_days = 7
ltm_max_entries = 10000

# Retrieval
max_surfaced_memories = 10
max_memory_tokens = 2000
vector_search_limit = 50
fts_search_limit = 50

# Processing
batch_processing_delay_seconds = 300
daily_maintenance_hour = 3  # 3 AM local
```

---

## Implementation Phases

### Phase 1: Foundation
- [ ] Conversation and segment storage
- [ ] Basic short-term memory with FTS
- [ ] Per-segment keyword extraction
- [ ] Simple retrieval by keyword match

### Phase 2: Intelligence
- [ ] Importance scoring algorithm
- [ ] Conversation summarization
- [ ] Memory promotion pipeline
- [ ] Context injection system

### Phase 3: Semantic
- [ ] Vector embeddings integration
- [ ] Hybrid retrieval (FTS + vector)
- [ ] Memory graph connections
- [ ] Duplicate detection/consolidation

### Phase 4: Curation
- [ ] Scheduled maintenance jobs
- [ ] Decay and pruning algorithms
- [ ] Memory quality scoring
- [ ] User feedback incorporation

---

## Technical Decisions

### Vector Storage: SQLite-vec

Using `sqlite-vec` extension for vector similarity search within SQLite:

```sql
-- Enable extension
.load vec0

-- Vector column in memories table
CREATE TABLE long_term_memories (
    id TEXT PRIMARY KEY,
    content TEXT NOT NULL,
    embedding BLOB,  -- Float32 array stored as blob
    -- ...
);

-- Create vector index
CREATE VIRTUAL TABLE memory_vectors USING vec0(
    memory_id TEXT PRIMARY KEY,
    embedding FLOAT[384]  -- Dimension depends on model
);

-- Similarity search
SELECT memory_id, distance
FROM memory_vectors
WHERE embedding MATCH ?1
ORDER BY distance
LIMIT 10;
```

**Benefits**:
- Single database, no external dependencies
- Works offline
- Consistent with existing SQLite architecture
- Can add cloud vector store later if needed

### Embeddings: Erasmus Embedder Service

Embeddings via erasmus `embedder` service (backed by HuggingFace models):

**Endpoint**: `POST {EMBEDDER_URL}/embed`
- Production: `https://dev-erasmus.ngrok.dev` (via API gateway)
- Local: `http://localhost:8088` (tiktokenizer rust proxy)

**Available Models**:
| Model | Dimensions | Notes |
|-------|------------|-------|
| `BAAI/bge-small-en-v1.5` | 384 | Default, fast |
| `BAAI/bge-base-en-v1.5` | 768 | Better quality |
| `sentence-transformers/all-MiniLM-L6-v2` | 384 | Alternative |

```rust
/// Embedding client for erasmus embedder service
pub struct ErasmusEmbeddings {
    base_url: String,
    client: reqwest::Client,
}

impl ErasmusEmbeddings {
    pub fn new(base_url: Option<String>) -> Self {
        Self {
            base_url: base_url.unwrap_or_else(|| 
                std::env::var("EMBEDDER_URL")
                    .unwrap_or_else(|_| "https://dev-erasmus.ngrok.dev".to_string())
            ),
            client: reqwest::Client::new(),
        }
    }

    /// Generate embedding for single text
    pub async fn embed(&self, text: &str) -> Result<EmbeddingResult> {
        let response = self.client
            .post(&format!("{}/embed", self.base_url))
            .json(&json!({
                "input": text,
                "normalize": true,
                "model": "BAAI/bge-small-en-v1.5"
            }))
            .send()
            .await?;
        
        response.json().await
    }

    /// Batch embed multiple texts (single request)
    pub async fn embed_batch(&self, texts: &[String]) -> Result<BatchEmbeddingResult> {
        let response = self.client
            .post(&format!("{}/embed", self.base_url))
            .json(&json!({
                "input": texts,
                "normalize": true,
                "model": "BAAI/bge-small-en-v1.5"
            }))
            .send()
            .await?;
        
        response.json().await
    }
}

#[derive(Deserialize)]
pub struct EmbeddingResult {
    pub embedding: Vec<f32>,
    pub dimensions: usize,
    pub model: String,
}

#[derive(Deserialize)]
pub struct BatchEmbeddingResult {
    pub embeddings: Vec<Vec<f32>>,
    pub dimensions: usize,
    pub model: String,
    pub count: usize,
}
```

**Fallback**: If erasmus unavailable, queue embeddings for later processing.

### User Experience: Invisible by Default

The memory system operates silently in the background:

- **No user-facing memory UI** - memories are not directly viewable/editable
- **Single toggle** in settings menu: "Enable Memory" (on/off)
- **Organic discovery** - power users can explore the database if curious
- **No explanations** - the assistant just "remembers" without calling attention to it

```typescript
// Settings menu item
interface MemorySettings {
  enabled: boolean;  // Single toggle, defaults to true
}
```

When disabled:
- No new memories are created
- No memories are surfaced
- Existing memories are preserved (not deleted)
- Background processing pauses

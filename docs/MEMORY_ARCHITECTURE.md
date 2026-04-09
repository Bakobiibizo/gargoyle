# Memory System Architecture

## Overview

Gargoyle's memory system enables the assistant to learn, remember, and retrieve context across conversations. The architecture follows a **tiered storage model** with automatic promotion, decay, and retrieval mechanisms.

```
┌─────────────────────────────────────────────────────────────────────────┐
│                         CHAT INTERFACE                                   │
│                    (Frontend conversation UI)                            │
└────────────────────────────────┬────────────────────────────────────────┘
                                 │
                                 ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                         MEMORY AGENT                                     │
│           Curates, retrieves, and manages all memory operations          │
│                                                                          │
│  ┌──────────────┐  ┌──────────────┐  ┌────────────────────────────────┐ │
│  │  Recording   │  │  Retrieval   │  │  Background Processing         │ │
│  │  (capture)   │  │  (search)    │  │  (promotion, decay, cleanup)   │ │
│  └──────────────┘  └──────────────┘  └────────────────────────────────┘ │
└────────────────────────────────┬────────────────────────────────────────┘
                                 │
                                 ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                        MEMORY SERVICE                                    │
│              Core CRUD operations and search capabilities                │
└────────────────────────────────┬────────────────────────────────────────┘
                                 │
         ┌───────────────────────┼───────────────────────┐
         │                       │                       │
         ▼                       ▼                       ▼
┌─────────────────┐   ┌─────────────────┐   ┌─────────────────┐
│  Conversations  │   │  Short-Term     │   │  Long-Term      │
│  & Segments     │   │  Memory (STM)   │   │  Memory (LTM)   │
│                 │   │                 │   │                 │
│  - Sessions     │   │  - Observations │   │  - Facts        │
│  - Messages     │   │  - Insights     │   │  - Preferences  │
│  - FTS Search   │   │  - Time-decay   │   │  - Patterns     │
│                 │   │  - FTS Search   │   │  - Embeddings   │
└─────────────────┘   └─────────────────┘   └─────────────────┘
         │                       │                       │
         └───────────────────────┼───────────────────────┘
                                 │
                                 ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                           SQLite Database                                │
│                    (FTS5 indexes, embeddings as BLOB)                    │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## Core Components

### 1. Memory Agent (`agents/memory_agent.rs`)

The `MemoryAgent` is the primary interface for memory operations. It exposes a request/response API that can be invoked via the agent router.

**Responsibilities:**
- Start/end conversations
- Record messages to conversation segments
- Create observations and insights (STM)
- Search across all memory tiers
- Promote STM to LTM
- Build context for injection into chat

**Request Actions:**
| Action | Description |
|--------|-------------|
| `StartConversation` | Create a new conversation session |
| `EndConversation` | Close session with optional summary |
| `RecordMessage` | Add user/assistant message to segment |
| `CreateObservation` | Store a short-term observation |
| `CreateInsight` | Store a derived insight |
| `SearchMemories` | FTS search across STM + LTM |
| `GetRecentMemories` | Retrieve recent STM entries |
| `PromoteToLongTerm` | Move STM → LTM with classification |
| `GetContext` | Build context string for chat injection |

### 2. Memory Service (`services/memory_service.rs`)

The service layer handles all database operations with proper error handling and transaction management.

**Key Methods:**
```rust
// Conversations
create_conversation() → Conversation
get_conversation(id) → Option<Conversation>
end_conversation(id, summary)
list_recent_conversations(limit) → Vec<Conversation>

// Segments
add_segment(conversation_id, role, content) → ConversationSegment
get_conversation_segments(conversation_id) → Vec<ConversationSegment>
search_segments(query, limit) → Vec<ConversationSegment>

// Short-Term Memory
create_stm(memory) → ShortTermMemory
get_stm(id) → Option<ShortTermMemory>
search_stm(query, limit) → Vec<ShortTermMemory>
get_recent_stm(limit, type?) → Vec<ShortTermMemory>
touch_stm(id)  // Update access time
delete_stm(id)

// Long-Term Memory
create_ltm(memory) → LongTermMemory
get_ltm(id) → Option<LongTermMemory>
search_ltm(query, limit) → Vec<LongTermMemory>
get_ltm_by_category(category, limit) → Vec<LongTermMemory>
touch_ltm(id)
embed_ltm(id)  // Generate and store embedding

// Unified Search
search_all(query, limit) → Vec<MemorySearchResult>

// Promotion
promote_to_ltm(stm_id, type, category) → LongTermMemory
```

### 3. Embeddings Service (`services/embeddings.rs`)

Handles vector embedding generation via the Erasmus embedder service.

**Configuration:**
```toml
[memory]
embedder_url = "https://dev-erasmus.ngrok.dev"
embedding_model = "BAAI/bge-small-en-v1.5"
embedding_dimensions = 384
use_real_embeddings = false  # Toggle for dev/prod
```

**API:**
```rust
ErasmusEmbeddings::new(url, model) → ErasmusEmbeddings
embed(text) → EmbeddingResult { embedding, dimensions, model }
embed_batch(texts) → BatchEmbeddingResult
embedding_to_blob(vec) → Vec<u8>  // For SQLite storage
blob_to_embedding(bytes) → Vec<f32>
```

---

## Data Models

### Conversation
```rust
struct Conversation {
    id: String,
    started_at: DateTime<Utc>,
    ended_at: Option<DateTime<Utc>>,
    title: Option<String>,
    summary: Option<String>,
    message_count: i32,
    metadata: HashMap<String, Value>,
}
```

### ConversationSegment
```rust
struct ConversationSegment {
    id: String,
    conversation_id: String,
    role: MessageRole,  // User, Assistant, System
    content: String,
    created_at: DateTime<Utc>,
    token_count: Option<i32>,
    metadata: HashMap<String, Value>,
}
```

### ShortTermMemory
```rust
struct ShortTermMemory {
    id: String,
    content: String,
    memory_type: ShortTermMemoryType,  // Observation, Insight, Fact, Preference, Task
    source_conversation_id: Option<String>,
    source_segment_id: Option<String>,
    created_at: DateTime<Utc>,
    last_accessed_at: DateTime<Utc>,
    access_count: i32,
    relevance_score: f64,
    decay_rate: f64,
    expires_at: Option<DateTime<Utc>>,
    promoted_to_ltm_id: Option<String>,
    metadata: HashMap<String, Value>,
}

impl ShortTermMemory {
    // Compute current relevance with time decay
    fn current_relevance(&self) -> f64 {
        let age_hours = (now - self.created_at).num_hours();
        let decay = exp(-self.decay_rate * age_hours);
        let access_boost = ln(1 + self.access_count) * 0.1;
        (self.relevance_score * decay + access_boost).clamp(0.0, 1.0)
    }
}
```

### LongTermMemory
```rust
struct LongTermMemory {
    id: String,
    content: String,
    memory_type: LongTermMemoryType,  // Fact, Preference, Pattern, Relationship, Skill, Context
    category: Option<String>,
    importance: f64,
    confidence: f64,
    source_stm_ids: Vec<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    last_accessed_at: DateTime<Utc>,
    access_count: i32,
    embedding: Option<Vec<f32>>,  // Vector for semantic search
    embedding_model: Option<String>,
    metadata: HashMap<String, Value>,
}
```

---

## Database Schema

### Tables (Migration 008)

```sql
-- Conversation sessions
CREATE TABLE conversations (
    id TEXT PRIMARY KEY,
    started_at TEXT NOT NULL,
    ended_at TEXT,
    title TEXT,
    summary TEXT,
    message_count INTEGER DEFAULT 0,
    metadata TEXT DEFAULT '{}'
);

-- Individual messages
CREATE TABLE conversation_segments (
    id TEXT PRIMARY KEY,
    conversation_id TEXT REFERENCES conversations(id) ON DELETE CASCADE,
    role TEXT NOT NULL CHECK (role IN ('user', 'assistant', 'system')),
    content TEXT NOT NULL,
    created_at TEXT NOT NULL,
    token_count INTEGER,
    metadata TEXT DEFAULT '{}'
);

-- Short-term memories with decay
CREATE TABLE short_term_memories (
    id TEXT PRIMARY KEY,
    content TEXT NOT NULL,
    memory_type TEXT NOT NULL CHECK (memory_type IN ('observation', 'insight', 'fact', 'preference', 'task')),
    source_conversation_id TEXT REFERENCES conversations(id),
    source_segment_id TEXT REFERENCES conversation_segments(id),
    created_at TEXT NOT NULL,
    last_accessed_at TEXT NOT NULL,
    access_count INTEGER DEFAULT 1,
    relevance_score REAL DEFAULT 1.0,
    decay_rate REAL DEFAULT 0.1,
    expires_at TEXT,
    promoted_to_ltm_id TEXT,
    metadata TEXT DEFAULT '{}'
);

-- Long-term memories with embeddings
CREATE TABLE long_term_memories (
    id TEXT PRIMARY KEY,
    content TEXT NOT NULL,
    memory_type TEXT NOT NULL CHECK (memory_type IN ('fact', 'preference', 'pattern', 'relationship', 'skill', 'context')),
    category TEXT,
    importance REAL DEFAULT 0.5,
    confidence REAL DEFAULT 1.0,
    source_stm_ids TEXT DEFAULT '[]',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    last_accessed_at TEXT NOT NULL,
    access_count INTEGER DEFAULT 1,
    embedding BLOB,
    embedding_model TEXT,
    metadata TEXT DEFAULT '{}'
);

-- Links between memories and entities
CREATE TABLE memory_entity_links (
    memory_id TEXT NOT NULL,
    memory_table TEXT NOT NULL CHECK (memory_table IN ('short_term_memories', 'long_term_memories')),
    entity_id TEXT NOT NULL,
    link_type TEXT DEFAULT 'related',
    created_at TEXT NOT NULL,
    PRIMARY KEY (memory_id, memory_table, entity_id)
);
```

### FTS5 Indexes

Each content table has a corresponding FTS5 virtual table with triggers for automatic sync:

```sql
CREATE VIRTUAL TABLE conversation_segments_fts USING fts5(content, ...);
CREATE VIRTUAL TABLE short_term_memories_fts USING fts5(content, ...);
CREATE VIRTUAL TABLE long_term_memories_fts USING fts5(content, category, ...);
```

---

## Memory Lifecycle

### 1. Recording Phase

```
User message arrives
        │
        ▼
┌───────────────────┐
│ add_segment()     │  Store in conversation_segments
└────────┬──────────┘
         │
         ▼
┌───────────────────┐
│ Extract patterns  │  Check for preferences, facts, tasks
└────────┬──────────┘
         │
         ▼
┌───────────────────┐
│ create_stm()      │  Store as observation/insight
└───────────────────┘
```

### 2. Retrieval Phase

```
Query arrives
     │
     ├──────────────────────────────────────────┐
     │                                          │
     ▼                                          ▼
┌─────────────┐                       ┌─────────────┐
│ search_stm  │ FTS5 keyword match    │ search_ltm  │ FTS5 + vector
└──────┬──────┘                       └──────┬──────┘
       │                                     │
       └──────────────┬──────────────────────┘
                      │
                      ▼
              ┌───────────────┐
              │ Merge & rank  │  By score, recency, access
              └───────┬───────┘
                      │
                      ▼
              ┌───────────────┐
              │ Return top N  │
              └───────────────┘
```

### 3. Promotion Phase (STM → LTM)

```
STM candidate
     │
     ▼
┌─────────────────────┐
│ Check criteria      │  age > threshold, access_count, relevance
└──────────┬──────────┘
           │
           ▼
┌─────────────────────┐
│ Classify type       │  fact, preference, pattern, etc.
└──────────┬──────────┘
           │
           ▼
┌─────────────────────┐
│ create_ltm()        │  Copy content, assign category
└──────────┬──────────┘
           │
           ▼
┌─────────────────────┐
│ Mark STM promoted   │  Set promoted_to_ltm_id
└──────────┬──────────┘
           │
           ▼
┌─────────────────────┐
│ embed_ltm()         │  Generate vector embedding (async)
└─────────────────────┘
```

---

## Configuration

```toml
[memory]
# Feature toggle
enabled = true

# Embeddings
embedder_url = "https://dev-erasmus.ngrok.dev"
embedding_model = "BAAI/bge-small-en-v1.5"
embedding_dimensions = 384
use_real_embeddings = false

# STM settings
stm_retention_hours = 72
stm_max_entries = 1000
stm_default_decay_rate = 0.1

# LTM settings
ltm_importance_threshold = 0.3
ltm_max_entries = 10000

# Retrieval
search_limit_default = 10
context_max_segments = 10
```

---

## Integration Points

### Agent Router Integration

The memory agent is wired into the agent router for dispatch:

```rust
// In router.rs
AgentRequest::Memory(req) => {
    let agent = MemoryAgent::new(conn);
    let response = agent.handle(req);
    Ok(AgentResponse::Memory(response))
}
```

### Chat Flow Integration (Planned)

```
1. User sends message
2. Memory agent records segment
3. Memory agent searches for relevant context
4. Context injected into system prompt
5. LLM generates response
6. Response recorded as assistant segment
7. Background: extract observations, update STM
```

### Entity Linking

Memories can be linked to canonical entities:

```rust
link_memory_to_entity(memory_id, "short_term_memories", entity_id, "mentions")
get_entity_memories(entity_id) → Vec<MemoryEntityLink>
```

---

## File Map

| File | Purpose |
|------|---------|
| `models/memory.rs` | Data structures (Conversation, STM, LTM, etc.) |
| `services/memory_service.rs` | Database CRUD operations |
| `services/embeddings.rs` | Erasmus embedder client |
| `agents/memory_agent.rs` | High-level agent API |
| `migrations/008_memory_tables.sql` | Database schema |
| `config/mod.rs` | MemoryConfig settings |
| `tests/memory_integration_test.rs` | Integration tests |

---

## Implementation Status

### Phase 1: Foundation ✅
- [x] Database schema (conversations, segments, STM, LTM)
- [x] MemoryService CRUD operations
- [x] FTS5 search for all tables
- [x] ErasmusEmbeddings client
- [x] MemoryAgent with request/response API
- [x] Agent router integration

### Phase 2: Intelligence (In Progress)
- [ ] Automatic observation extraction from conversations
- [ ] Conversation summarization on end
- [ ] STM → LTM promotion pipeline
- [ ] Context injection into chat flow

### Phase 3: Semantic (Planned)
- [ ] Vector similarity search
- [ ] Hybrid retrieval (FTS + vector)
- [ ] Memory graph connections
- [ ] Duplicate detection/consolidation

### Phase 4: Curation (Planned)
- [ ] Scheduled maintenance jobs
- [ ] Decay and pruning algorithms
- [ ] Memory quality scoring
- [ ] User feedback incorporation

---

## Design Decisions

### Why SQLite for Vectors?

- **Simplicity**: Single database, no external dependencies
- **Offline-first**: Works without network
- **Consistent**: Same transaction model as other data
- **Scalable later**: Can add dedicated vector store if needed

### Why Two Memory Tiers?

- **STM**: Fast capture, time-decay, low friction
- **LTM**: Curated, persistent, embedding-indexed
- **Promotion**: Quality gate between tiers

### Why Erasmus Embeddings?

- **Self-hosted**: No external API dependencies
- **Configurable**: Model selection via config
- **Batching**: Efficient bulk embedding
- **Fallback**: Graceful degradation if unavailable

---

## Testing

```bash
# Unit tests
cargo test --lib memory

# Integration tests
cargo test --test memory_integration_test
```

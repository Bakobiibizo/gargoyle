# Gargoyle - Project Vision

## Core Purpose

Gargoyle is a **personal knowledge and context management system** that grows with you over time. It combines:

1. **Persistent canonical context** - A structured representation of facts, concepts, and tasks
2. **Reasoning-capable graph** - Models can traverse and find patterns across your knowledge
3. **Conversational assistant** - A chat agent whose memory persists and deepens over time
4. **Template-driven workflows** - Complex operations composed from your canonical context

---

## Design Principles

### 1. Canonical Context Object

The core data model has two tiers:

| Tier | Structure | Purpose | Examples |
|------|-----------|---------|----------|
| **Rigid** | Fixed schema, validated fields | Factual information that must be accurate | User profile, company info, dates, metrics |
| **Flexible** | Loose schema, freeform content | Concepts, ideas, tasks that evolve | Notes, ideas, tasks, research threads |

**Key insight**: Facts don't change often but must be correct. Ideas change constantly but correctness is subjective.

### 2. Persistent Memory Growth

The system should:
- **Remember** - Chat history and extracted insights persist across sessions
- **Learn** - Patterns and preferences are discovered over time
- **Connect** - New information links to existing knowledge automatically
- **Grow** - The graph expands as the assistant gets to know you

### 3. Template System

Templates are **not static prompts** - they are:
- **Composed** from canonical context fields
- **Created dynamically** as user needs evolve
- **Filled** with structured data from the graph
- **Versioned** and improved over time

### 4. Sub-Agent Architecture

The primary chat agent delegates to specialized sub-agents:

```
┌─────────────────────────────────────────────────────────┐
│              Primary Chat Agent                          │
│         (Your assistant - persistent memory)             │
└─────────────────────────┬───────────────────────────────┘
                          │
         ┌────────────────┼────────────────┐
         │                │                │
         ▼                ▼                ▼
   ┌───────────┐   ┌───────────┐   ┌───────────┐
   │ Template  │   │  Graph    │   │  Entity   │
   │ Curator   │   │  Query    │   │  Manager  │
   └───────────┘   └───────────┘   └───────────┘
         │                │                │
         └────────────────┼────────────────┘
                          ▼
              ┌─────────────────────┐
              │   Canonical Graph   │
              │  (Facts + Concepts) │
              └─────────────────────┘
```

Sub-agents provide **concise insights** back to the primary agent, not raw data.

---

## Use Cases

### Task Prioritization
- Surface tasks based on deadlines, dependencies, and context
- Identify blocked tasks and suggest unblocking actions
- Balance urgent vs important based on user goals

### Planning
- Break down goals into actionable steps
- Connect plans to existing knowledge and resources
- Track progress and adjust based on reality

### Research
- Organize research threads and findings
- Connect new information to existing knowledge
- Surface relevant past research when exploring new topics

### Thought Organization
- Capture fleeting ideas before they're lost
- Connect related concepts across time
- Surface patterns in your thinking

---

## Canonical Context Structure

### Rigid Tier (Validated Facts)

```
User Profile
├── name, email, timezone
├── role, department, company
└── preferences (verified)

Company Context  
├── name, industry, size
├── mission, values
└── organizational structure

Operational Facts
├── deadlines, commitments
├── metrics, budgets
└── external dependencies
```

### Flexible Tier (Evolving Concepts)

```
Ideas & Concepts
├── notes (freeform thoughts)
├── ideas (potential actions)
└── concepts (mental models)

Tasks & Work
├── tasks (actionable items)
├── projects (task containers)
└── goals (desired outcomes)

Research & Learning
├── findings (discovered facts)
├── questions (open threads)
└── sources (references)
```

---

## Memory Model

The chat agent maintains:

1. **Session memory** - Current conversation context
2. **Episodic memory** - Past conversations (searchable)
3. **Semantic memory** - Extracted facts and patterns (graph)
4. **Procedural memory** - Learned preferences and workflows

Over time, the agent should:
- Remember your communication style
- Learn your priorities and values
- Anticipate your needs based on patterns
- Proactively surface relevant information

---

## Success Criteria

The system is working when:

1. **You trust it** - Information retrieved is accurate and relevant
2. **It grows with you** - New sessions build on past context
3. **It surfaces insights** - Patterns you didn't explicitly ask for
4. **It reduces friction** - Complex workflows become simple
5. **It feels personal** - The assistant knows your context

---

## Current Implementation Status

| Component | Status | Notes |
|-----------|--------|-------|
| Entity/Relation storage | ✅ Complete | SQLite + FTS |
| Schema validation | ✅ Complete | Canonical field validation |
| Agent router | ✅ Complete | Single dispatch point |
| TemplateCuratorAgent | ✅ Complete | Template CRUD + search |
| GraphQueryAgent | ✅ Complete | Read-only graph ops |
| EntityManagerAgent | ✅ Complete | Entity mutations |
| IntakePipelineAgent | ✅ Complete | User onboarding |
| LLM tool integration | ✅ Complete | Agents callable from chat |
| Persistent chat memory | 🔄 Partial | Sessions stored, not cross-session |
| Canonical context tiers | 📋 Planned | Need rigid/flexible separation |
| Insight generation | 📋 Planned | Sub-agents return insights |
| Memory growth | 📋 Planned | Cross-session learning |

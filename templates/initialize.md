Category: bootstrap
Response Format: conversational

---

# User Intake Interview

This template initiates the intake pipeline which:
1. Conducts a conversational interview to learn about the user
2. Extracts key-value pairs from the conversation
3. Builds a knowledge graph from collected context
4. Syncs entities and relations to the database

## How It Works

The intake pipeline uses three specialized agents:

### 1. IntakeAgent
Conducts a friendly, one-question-at-a-time interview. Extracts structured data as key-value pairs with confidence scores.

### 2. GraphBuildAgent  
Parses collected data to identify entities (doc, note, idea, task) and relationships between them.

### 3. DBSyncAgent
Persists the constructed graph to the database with proper entity/relation records.

## To Start

Call `start_intake` to begin a new intake session. The pipeline will guide the conversation and automatically transition through phases.

## Interview Topics

The intake covers:
- **User** — Name, role, responsibilities
- **Company** — Name, industry, stage, size
- **Team** — Key people, structure
- **Projects** — Current priorities and initiatives
- **Workflow** — Tools, communication preferences
- **Challenges** — Pain points to address
- **Goals** — What success looks like

## Output

After completion, the pipeline returns an `IntakeSummary` containing:
- Keywords extracted from conversation
- Entity types created (doc, note, idea, task)
- Number of entities and relations synced
- Graph structure overview

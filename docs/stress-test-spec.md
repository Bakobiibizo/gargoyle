# Gargoyle — Stress Test Plan (Analytics Vertical Slice)

This document defines the implementation plan for stress-testing Gargoyle's architecture using the Analytics domain as a vertical slice. The goal is to prove or break every load-bearing architectural decision before scaling horizontally.

> **Scope:** Originally 3 entity types (`metric`, `experiment`, `result`) and 3 templates (`analytics-metric-tree`, `analytics-experiment-plan`, `analytics-anomaly-investigation`). Now expanded to 22 entity types and 23 enriched templates across analytics, marketing, development, organizing, and content categories. Tests cover every infrastructure component: patch protocol, schema validation, optimistic locking, dedup pipeline, embedding search, claim grounding, status transitions, and relation integrity.

> **Philosophy:** This is not a test suite for correctness. It's a harness for discovering where the architecture buckles under realistic abuse. Every test scenario simulates a plausible user or agent behavior that could expose a design flaw. If the slice survives, the remaining 25 entity types and 95 templates are horizontal scaling — not new risk.

---

## 0. Implementation Order

Build in this sequence. Each layer depends on the one before it.

```
Phase 1: Schema Foundation
  SQLite schema (all tables)
  Schema registry (3 entity types only)
  Patch protocol (create_entity, update_entity, create_relation)
  Validation pipeline (schema + status + optimistic lock + referential integrity)

Phase 2: Services
  store service (CRUD + patch application)
  indexer service (FTS5 + embedding generation + search_similar())
  context_manager (operational_context scalar read/write)

Phase 3: Dedup + Claims
  Dedup pipeline (title match + fuzzy + embedding proximity)
  Claims table with grounding enforcement

Phase 4: Templates
  analytics-metric-tree (foundational) — end-to-end
  analytics-experiment-plan (workflow) — depends on metric output
  analytics-anomaly-detection-investigation (diagnostic) — queries existing state

Phase 5: Graph Projection
  graph_builder (projection rebuild + related_to audit)
  Graph macros (entity queries + semantic search)

Phase 6: Stress Harness
  Property-based fuzz tests against patch protocol
  Scenario-based integration tests (this document)
  Load simulation (concurrent agent runs)
```

Do not proceed to Phase N+1 until Phase N passes its tests.

---

## 1. SQLite Schema (Phase 1)

### 1.1 DDL

```sql
-- Core tables
CREATE TABLE entities (
  id TEXT PRIMARY KEY,                    -- uuid
  entity_type TEXT NOT NULL,
  category TEXT,
  title TEXT NOT NULL,
  body_md TEXT DEFAULT '',
  status TEXT,
  priority INTEGER CHECK (priority BETWEEN 0 AND 4),
  due_at TEXT,                            -- ISO 8601
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
  updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
  source TEXT NOT NULL CHECK (source IN ('manual','clipboard','web','import','agent','template','bootstrap')),
  canonical_fields TEXT NOT NULL DEFAULT '{}',  -- JSON
  _schema_version INTEGER NOT NULL DEFAULT 1,
  provenance_run_id TEXT,
  deleted_at TEXT
);

CREATE TABLE relations (
  id TEXT PRIMARY KEY,
  from_id TEXT NOT NULL REFERENCES entities(id),
  to_id TEXT NOT NULL REFERENCES entities(id),
  relation_type TEXT NOT NULL,
  weight REAL DEFAULT 1.0,
  confidence REAL CHECK (confidence IS NULL OR (confidence >= 0.0 AND confidence <= 1.0)),
  provenance_run_id TEXT,
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE TABLE runs (
  run_id TEXT PRIMARY KEY,
  template_key TEXT NOT NULL,
  template_version TEXT NOT NULL,
  template_category TEXT NOT NULL,
  inputs_snapshot TEXT NOT NULL DEFAULT '{}',   -- JSON
  outputs_snapshot TEXT NOT NULL DEFAULT '{}',  -- JSON
  patch_set TEXT NOT NULL DEFAULT '[]',         -- JSON
  status TEXT NOT NULL CHECK (status IN ('pending','applied','rejected','partial')),
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE TABLE artifacts (
  artifact_id TEXT PRIMARY KEY,
  entity_id TEXT NOT NULL REFERENCES entities(id),
  kind TEXT NOT NULL CHECK (kind IN ('attachment','link','export','rendered_doc')),
  uri_or_path TEXT NOT NULL,
  hash TEXT,
  mime TEXT,
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE TABLE operational_context (
  context_id TEXT PRIMARY KEY,
  context_key TEXT NOT NULL UNIQUE,
  context_value TEXT NOT NULL,             -- JSON
  updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
  updated_by_run_id TEXT
);

CREATE TABLE claims (
  claim_id TEXT PRIMARY KEY,
  subject TEXT NOT NULL,
  predicate TEXT NOT NULL,
  object TEXT NOT NULL,
  confidence REAL NOT NULL CHECK (confidence >= 0.0 AND confidence <= 1.0),
  evidence_entity_id TEXT NOT NULL REFERENCES entities(id),
  provenance_run_id TEXT,
  promoted_to_entity_id TEXT,
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE TABLE embeddings (
  embedding_id TEXT PRIMARY KEY,
  entity_id TEXT NOT NULL REFERENCES entities(id),
  model TEXT NOT NULL,
  vector BLOB NOT NULL,
  dimensions INTEGER NOT NULL,
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE TABLE custom_relation_types (
  type_key TEXT PRIMARY KEY CHECK (type_key LIKE 'custom:%'),
  description TEXT NOT NULL,
  expected_from_types TEXT,                -- JSON array
  expected_to_types TEXT,                  -- JSON array
  proposed_by_run_id TEXT,
  approved_at TEXT NOT NULL
);

CREATE TABLE dedup_suggestions (
  suggestion_id TEXT PRIMARY KEY,
  new_entity_id TEXT NOT NULL REFERENCES entities(id),
  existing_entity_id TEXT NOT NULL REFERENCES entities(id),
  detection_method TEXT NOT NULL CHECK (detection_method IN ('exact_title','fuzzy_title','embedding_proximity')),
  confidence REAL NOT NULL CHECK (confidence >= 0.0 AND confidence <= 1.0),
  status TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending','accepted','dismissed')),
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

-- Performance indexes (Analytics slice)
CREATE INDEX idx_entities_type ON entities(entity_type) WHERE deleted_at IS NULL;
CREATE INDEX idx_entities_metric ON entities(status) WHERE entity_type = 'metric' AND deleted_at IS NULL;
CREATE INDEX idx_entities_experiment ON entities(status) WHERE entity_type = 'experiment' AND deleted_at IS NULL;
CREATE INDEX idx_entities_result ON entities(status) WHERE entity_type = 'result' AND deleted_at IS NULL;
CREATE INDEX idx_entities_provenance ON entities(provenance_run_id) WHERE provenance_run_id IS NOT NULL;
CREATE INDEX idx_relations_from ON relations(from_id, relation_type);
CREATE INDEX idx_relations_to ON relations(to_id, relation_type);
CREATE INDEX idx_relations_provenance ON relations(provenance_run_id) WHERE provenance_run_id IS NOT NULL;
CREATE INDEX idx_embeddings_entity ON embeddings(entity_id);
CREATE INDEX idx_dedup_status ON dedup_suggestions(status) WHERE status = 'pending';
CREATE INDEX idx_claims_evidence ON claims(evidence_entity_id);

-- FTS5
CREATE VIRTUAL TABLE entities_fts USING fts5(title, body_md, content=entities, content_rowid=rowid);
```

### 1.2 Schema Smoke Test

Before any application code runs, verify:

```sql
-- Insert a metric entity
INSERT INTO entities (id, entity_type, title, source, canonical_fields, _schema_version)
VALUES ('test-1', 'metric', 'MRR', 'manual', '{"current_value": 200000}', 1);

-- Verify it round-trips
SELECT id, entity_type, json_extract(canonical_fields, '$.current_value') FROM entities WHERE id = 'test-1';

-- Verify claims enforce grounding
INSERT INTO claims (claim_id, subject, predicate, object, confidence, evidence_entity_id)
VALUES ('claim-1', 'MRR', 'trending', 'upward', 0.85, 'test-1');
-- Should succeed

INSERT INTO claims (claim_id, subject, predicate, object, confidence, evidence_entity_id)
VALUES ('claim-2', 'ARR', 'trending', 'upward', 0.85, NULL);
-- Should fail: NOT NULL constraint on evidence_entity_id

-- Cleanup
DELETE FROM claims; DELETE FROM entities;
```

---

## 2. Stress Scenarios

Each scenario targets a specific architectural decision. They are ordered by infrastructure dependency — later scenarios assume earlier ones pass.

---

### Scenario 1: Patch Protocol Fundamentals

**What it tests:** §3.1 patch structure, §3.2 validation pipeline steps 1–4.

**Setup:** Empty database with schema registry loaded for `metric`, `experiment`, `result`.

**Test cases:**

```
1a. VALID create_entity (metric)
    Input: { op_type: "create_entity", payload: { entity_type: "metric", title: "MRR",
             source: "template", canonical_fields: { current_value: 200000, target_value: 300000,
             trend: "up", data_source: "Stripe" } } }
    Expected: Entity created. _schema_version = 1. Status defaults to null.
    Verify: SELECT * FROM entities WHERE title = 'MRR'

1b. INVALID create_entity — missing required field
    Input: { op_type: "create_entity", payload: { entity_type: "decision", title: "Test",
             source: "manual", canonical_fields: { rationale: "testing" } } }
    Expected: Rejected. Error: "missing required field: owner_id"
    Note: decision requires owner_id, decided_at, rationale, revisit_triggers.

1c. INVALID create_entity — wrong entity_ref type
    Input: Create a metric entity, then create an experiment with
           canonical_fields.source_experiment_id pointing to the metric.
    Expected: Rejected. Error: "entity_ref type mismatch: expected experiment, got metric"

1d. INVALID create_entity — bad enum value
    Input: { entity_type: "metric", canonical_fields: { trend: "sideways" } }
    Expected: Rejected. Error: "invalid enum value for trend: sideways. Expected: up, down, flat"

1e. VALID create_relation
    Input: Create metric "MRR" and experiment "Pricing Test".
           Then: { op_type: "create_relation", payload: { from_id: <experiment_id>,
                   to_id: <metric_id>, relation_type: "measures" } }
    Expected: Relation created.

1f. INVALID create_relation — nonexistent target
    Input: { from_id: <valid_id>, to_id: "nonexistent-uuid", relation_type: "measures" }
    Expected: Rejected. Error: "to_id does not reference an existing entity"

1g. INVALID create_relation — unapproved custom type
    Input: { from_id: <id>, to_id: <id>, relation_type: "custom:correlates_with" }
    Expected: Rejected. Error: "relation_type custom:correlates_with is not approved"

1h. INVALID create_relation — soft-deleted target
    Input: Soft-delete an entity, then try to create a relation to it.
    Expected: Rejected. Error: "to_id references a deleted entity"
```

**Pass criteria:** All valid ops succeed, all invalid ops rejected with structured errors containing field path, expected type, and actual value.

---

### Scenario 2: Optimistic Locking Under Concurrency

**What it tests:** §1.2 commitment 8, §3.2 step 3.

**Setup:** One metric entity "MRR" exists.

**Test cases:**

```
2a. VALID sequential update
    - Read entity (get updated_at = T1)
    - update_entity with expected_updated_at = T1, set current_value to 210000
    Expected: Success. updated_at is now T2.

2b. VALID second sequential update
    - Read entity (get updated_at = T2)
    - update_entity with expected_updated_at = T2, set current_value to 220000
    Expected: Success.

2c. CONFLICT — stale expected_updated_at
    - Read entity (get updated_at = T3)
    - Simulate concurrent write: update directly to set updated_at = T4
    - Attempt update_entity with expected_updated_at = T3
    Expected: Rejected. Error: "optimistic lock conflict: expected T3, found T4"

2d. CONFLICT — parallel agent simulation
    - Read entity (T5)
    - Fork two update_entity ops, both with expected_updated_at = T5
    - Apply first: succeeds (T5 → T6)
    - Apply second: fails (expected T5, found T6)
    Expected: Exactly one succeeds. The other returns conflict error.

2e. RECOVERY — rebase after conflict
    - After 2d conflict, re-read entity (T6)
    - Resubmit with expected_updated_at = T6
    Expected: Success.
```

**Pass criteria:** No silent overwrites under any concurrent write pattern.

---

### Scenario 3: Schema Versioning and Migration

**What it tests:** §2.1.4 schema versioning, migration rules, validator version-aware behavior.

**Setup:** 20 metric entities created at schema_version 1 (no `unit` field).

**Test cases:**

```
3a. SCHEMA BUMP — add optional field
    - Bump metric schema to version 2: add optional field "unit" (string)
    - Verify: existing v1 entities remain valid (no backfill needed)
    - Read a v1 entity: _schema_version = 1, canonical_fields has no "unit"
    Expected: No errors. V1 entities pass validation.

3b. UPDATE BUMPS VERSION
    - Update a v1 entity: change current_value
    - Verify: _schema_version bumped to 2
    - Verify: full v2 schema validated (unit is optional, so still passes)
    Expected: Entity now at v2.

3c. SCHEMA BUMP — add required field
    - Bump metric schema to version 3: add required field "business_objective" (string)
    - Run backfill migration: UPDATE all entities at _schema_version < 3,
      set canonical_fields.business_objective = "unclassified", _schema_version = 3
    Expected: All entities now at v3.

3d. MIGRATION VERIFICATION PASS
    - Query: SELECT COUNT(*) FROM entities WHERE entity_type = 'metric' AND _schema_version < 3
    Expected: 0. No stale entities remain.

3e. UPDATE AFTER MIGRATION
    - Update any metric entity: change current_value
    - Verify: v3 validation applies, business_objective is present
    Expected: Success.

3f. STALE ENTITY + MISSING REQUIRED FIELD (failure mode)
    - Manually revert one entity to _schema_version = 1 (simulate failed migration)
    - Attempt update_entity
    - Bump triggers v3 validation
    - business_objective is missing → REJECT
    Expected: Rejected. Error surfaces the gap. This proves migration verification is essential.
```

**Pass criteria:** Schema bumps propagate cleanly. Stale entities are caught on update. Migration verification query returns 0 after every migration.

---

### Scenario 4: Status State Machine

**What it tests:** §2.1.5 lifecycle transitions.

**Setup:** One experiment entity at status "draft".

**Test cases:**

```
4a. VALID forward transition
    draft → running
    Expected: Success. No reason required.

4b. VALID skip transition
    running → archived (skipping "concluded")
    Expected: Success. Logged as skip.

4c. VALID backward transition with reason
    archived → running, reason: "Re-opened due to new data"
    Expected: Success. Reason logged.

4d. INVALID backward transition without reason
    If entity is at "concluded", attempt → "running" with no reason field.
    Expected: Rejected. Error: "backward transition requires reason"

4e. INVALID status value
    Attempt to set status = "completed" on experiment
    (valid for project, not experiment)
    Expected: Rejected. Error: "invalid status 'completed' for entity_type 'experiment'.
              Valid values: draft, running, concluded, archived"

4f. IDEMPOTENT same-status
    Set status = "running" when already "running"
    Expected: Success. No-op.

4g. NULL → first status
    Set status on entity with status = null
    Expected: Success for any valid first status in enum.
```

**Pass criteria:** All forward transitions succeed. Backward transitions require reason. Invalid status values rejected with enum list.

---

### Scenario 5: Deduplication Pipeline

**What it tests:** §3.3 dedup detection, §1.2 commitment 12.

**Setup:** One metric entity "Monthly Recurring Revenue" exists with embedding generated.

**Test cases:**

```
5a. EXACT TITLE MATCH
    Create metric titled "Monthly Recurring Revenue" (same case)
    Expected: Entity created (non-blocking). Dedup suggestion generated:
              detection_method = "exact_title", confidence = 0.95, status = "pending"

5b. CASE-INSENSITIVE EXACT MATCH
    Create metric titled "monthly recurring revenue"
    Expected: Dedup suggestion, exact_title, confidence 0.95.

5c. FUZZY TITLE MATCH
    Create metric titled "Monthly Recurrig Revenue" (typo, Levenshtein ≤ 3)
    Expected: Dedup suggestion, fuzzy_title, confidence 0.70.

5d. EMBEDDING PROXIMITY (long title)
    Create metric titled "MRR — Company Revenue Recurring Monthly"
    Expected: Dedup suggestion, embedding_proximity, confidence 0.80.
    (Title ≥ 4 chars, so embedding check fires.)

5e. SHORT TITLE — EMBEDDING SKIPPED
    Create metric titled "MRR"
    Expected: No embedding proximity check (title < 4 chars).
              May or may not trigger fuzzy match depending on distance to
              "Monthly Recurring Revenue" (likely too far). No dedup suggestion.

5f. CROSS-TYPE — NO FALSE POSITIVE
    Create result titled "Monthly Recurring Revenue"
    Expected: No dedup suggestion. Same title, different entity_type.

5g. SHORT-CIRCUIT
    Create metric titled "Monthly Recurring Revenue" (exact match)
    Expected: Only exact_title suggestion. Fuzzy and embedding checks should not fire
              (short-circuit on high-confidence match).

5h. DISMISS + NO RE-SUGGEST
    Dismiss the suggestion from 5a.
    Create another metric titled "Monthly Recurring Revenue".
    Expected: No new suggestion for the same (new_entity_id, existing_entity_id) pair.
    Note: The new entity has a different ID, so a new suggestion IS expected
          for (new_id_2, original_id). Dismissed pairs are specific to entity ID pairs.
    Correction: Re-evaluate — the spec says "dismissed pairs are not re-suggested."
    This means the (existing_entity_id) is tracked. New creations with the same
    title against the same existing entity should still suggest.
    Implementation note: "dismissed pairs" means (existing_entity_id, title_hash) or
    (existing_entity_id, new_entity_id) — clarify during implementation.

5i. ACCEPT → MERGE
    Accept a pending suggestion.
    Expected: merge_entities patch op generated. Requires user confirmation per §3.1.
```

**Pass criteria:** Dedup never blocks creation. Suggestions fire for all three detection methods. Cross-type is never matched. Short titles skip embedding check.

---

### Scenario 6: Embedding Search Service Boundary

**What it tests:** §2.7, §1.2 commitment 11.

**Setup:** 50 metric entities created via repeated template runs with varied titles and canonical_fields.

**Test cases:**

```
6a. BASIC SEMANTIC SEARCH
    indexer.search_similar("revenue growth", limit=5)
    Expected: Returns ranked results. Entities with titles/fields related to revenue
              score higher than unrelated metrics.

6b. EMPTY RESULTS
    indexer.search_similar("quantum entanglement", limit=5)
    Expected: Returns empty or very low-scoring results. Does not crash.

6c. LIMIT RESPECTED
    indexer.search_similar("metric", limit=3)
    Expected: Exactly ≤ 3 results returned.

6d. THRESHOLD FILTERING
    indexer.search_similar("revenue", limit=10, threshold=0.8)
    Expected: Only results with cosine similarity ≥ 0.8 returned.

6e. GRAPH MACRO INTEGRATION
    Resolve: {{graph.semantic query="customer acquisition cost" limit=5}}
    Expected: Returns entity IDs + scores. Template runner can inject results
              into template context.

6f. SERVICE ISOLATION
    Verify: No cosine distance computation exists outside the indexer module.
    Expected: grep -r "cosine" in codebase returns hits only in indexer service.

6g. EMBEDDING STALENESS
    Update an entity's title and canonical_fields.
    Query search_similar with the old title.
    Expected: Old embedding no longer matches (or matches weakly).
              New embedding reflects updated content.
    Verify: Embedding regenerated on update.
```

**Pass criteria:** All queries route through `indexer.search_similar()`. No cosine logic in UI or template code. Embeddings regenerate on entity update.

---

### Scenario 7: Claim Grounding Enforcement

**What it tests:** §2.6, claim grounding priority order.

**Setup:** One run that produced a result and a spec (multi-entity run).

**Test cases:**

```
7a. VALID grounded claim
    Create claim with evidence_entity_id pointing to the result entity.
    Expected: Success.

7b. INVALID ungrounded claim
    Create claim with evidence_entity_id = NULL.
    Expected: Rejected at DB level (NOT NULL constraint) or validation pipeline.

7c. INVALID — evidence points to deleted entity
    Soft-delete the result. Attempt to create claim grounding to it.
    Expected: Rejected. Error: "evidence_entity_id references a deleted entity"

7d. GROUNDING PRIORITY ORDER — multi-entity run
    Run produces: spec, result, experiment (in that order in patch_set).
    Agent needs to ground a claim to "the run's primary entity."
    Expected: Primary entity = result (decision > result > spec > experiment > first).
    Since no decision was produced, result wins.

7e. GROUNDING PRIORITY ORDER — no high-priority types
    Run produces: metric, metric, metric.
    Expected: Primary entity = first metric in patch_set (fallback to first entity).

7f. CLAIM → ENTITY TRAVERSAL
    Create 5 claims grounded to the same result entity.
    Query: SELECT * FROM claims WHERE evidence_entity_id = <result_id>
    Expected: All 5 returned. Claims are discoverable via entity traversal.
```

**Pass criteria:** No ungrounded claims in the database. Priority order is deterministic across all run shapes.

---

### Scenario 8: Relation Decision Tree Compliance

**What it tests:** §2.2.1.1 disambiguation rules.

**Setup:** Entities: metric "MRR", experiment "Pricing Test", result "Q1 Pricing Results", spec "Pricing Strategy".

**Test cases:**

```
8a. measures — metric → thing measured
    metric "MRR" measures experiment "Pricing Test"
    Expected: Valid. This is the canonical use of `measures`.

8b. evidence_for — result → decision
    result "Q1 Pricing Results" evidence_for decision "Keep current pricing"
    Expected: Valid. Result is empirical data supporting a decision.

8c. supports — spec → decision (NOT evidence_for)
    spec "Pricing Strategy" supports decision "Keep current pricing"
    Expected: Valid. Spec provides rationale, not data. Per decision tree:
              "if you'd put it in a Rationale section, use supports."

8d. tests — experiment → metric
    experiment "Pricing Test" tests metric "MRR"
    Expected: Valid. Experiment is testing the metric.

8e. ANTI-PATTERN — supports where evidence_for is correct
    Attempt: result "Q1 Pricing Results" supports decision "Keep current pricing"
    Expected: Warning (not rejection — relations aren't gated by decision tree).
    Implementation note: The decision tree is advisory. Validation pipeline does
    not enforce it. But the graph_builder or a lint pass should flag violations.

8f. ANTI-PATTERN — related_to where a specific type exists
    Attempt: metric "MRR" related_to experiment "Pricing Test"
    Expected: Valid but flagged. The correct relation is "measures" or "tests".
    This should contribute to the related_to ratio audit (§2.2.4).
```

**Pass criteria:** Advisory — this scenario validates agent behavior, not the validator. The goal is to verify that the decision tree produces consistent, unambiguous guidance for each entity type pair.

---

### Scenario 9: Template End-to-End (Dependency Chain)

**What it tests:** §4.1 template definition, §4.1.1 prerequisites, §4.2 registry, template maturity tiers.

**Flow:**

```
Step 1: Run analytics-metric-tree (foundational, no prerequisites)
        Input: business_model, primary_objective, customer_journey
        Expected output:
          - 5-10 metric entities created
          - Relations: metrics measures various business objectives
          - Run logged with inputs_snapshot and patch_set
          - All entities have provenance_run_id set
          - All entities at _schema_version = current

Step 2: Run analytics-experiment-plan (workflow, prerequisite: metric(1))
        Prerequisite check: ≥ 1 metric entity exists → satisfied
        Input: hypothesis, funnel_position, metric (select from graph)
        Expected output:
          - 1 experiment entity
          - Relations: experiment tests metric, experiment measures metric
          - Run logged

Step 3: Run analytics-anomaly-detection-investigation (diagnostic, prerequisite: metric(1))
        Input: KPI (select metric from graph), time_window, baseline_period
        Expected output:
          - 1 result entity
          - Relations: result evidence_for metric
          - Claims: grounded to result entity
          - Run logged

Step 4: Verify prerequisite chain
        - Attempt analytics-experiment-plan with 0 metrics in graph
        Expected: UI warning "This template needs at least one metric.
                  Run analytics-metric-tree to create one?"
        - Template still runs if forced (prerequisites are advisory)

Step 5: Verify provenance reconstruction
        For each run:
          - SELECT * FROM entities WHERE provenance_run_id = <run_id>
          - Verify matches the run's patch_set.produced_entities
          - SELECT * FROM relations WHERE provenance_run_id = <run_id>
          - Verify matches the run's patch_set.produced_relations
```

**Pass criteria:** Three-template chain executes end-to-end. Prerequisites surface warnings correctly. Provenance is fully reconstructable from run records.

---

### Scenario 10: related_to Audit Threshold

**What it tests:** §2.2.4, graph_builder monitoring.

**Setup:** Create 50 relations. 8 are `related_to`, 42 are typed.

**Test cases:**

```
10a. BELOW THRESHOLD
    related_to ratio = 8/50 = 16% (below 20%)
    Expected: No warning emitted.

10b. AT THRESHOLD
    Add 3 more related_to relations. Ratio = 11/53 = 20.7%
    Expected: Warning emitted. Includes breakdown:
              "related_to edges by type pair:
               metric → experiment: 4
               result → metric: 3
               experiment → result: 4
               Suggested: consider custom relation types for these patterns."

10c. CUSTOM TYPE REDUCES RATIO
    Approve custom:correlates_with. Reclassify 5 related_to as custom:correlates_with.
    Ratio = 6/53 = 11.3%
    Expected: Warning cleared on next projection rebuild.
```

**Pass criteria:** Threshold triggers at 20%. Breakdown surfaces actionable patterns. Custom types reduce ratio.

---

## 3. Fuzz Harness (Property-Based Testing)

The patch validator is the load-bearing wall. If it's brittle, the type system is decorative.

### 3.1 Fuzz Targets

Generate random patch ops and verify the validator either accepts them cleanly or rejects them with structured errors. Never crashes, never silently corrupts.

**Generators:**

```python
# Pseudocode — adapt to your property-based testing framework (hypothesis, proptest, etc.)

def gen_entity_type() -> str:
    return choice(["metric", "experiment", "result"])

def gen_canonical_fields(entity_type: str) -> dict:
    """Generate canonical_fields with random mix of:
       - valid fields with correct types
       - valid fields with wrong types (string where number expected)
       - unknown fields not in schema
       - missing required fields
       - entity_ref fields pointing to nonexistent entities
       - entity_ref fields pointing to wrong-type entities
       - enum fields with invalid values
    """

def gen_status(entity_type: str) -> str | None:
    """Generate status with random mix of:
       - valid status from enum
       - invalid status from a different entity type's enum
       - random string
       - null
    """

def gen_update_entity() -> dict:
    """Generate update_entity ops with:
       - valid expected_updated_at
       - stale expected_updated_at
       - missing expected_updated_at
       - future expected_updated_at
       - JSON pointer paths that don't exist
       - JSON pointer paths that change field types
    """

def gen_create_relation() -> dict:
    """Generate create_relation ops with:
       - valid from/to entity IDs
       - nonexistent from_id
       - nonexistent to_id
       - soft-deleted from_id
       - soft-deleted to_id
       - canonical relation type
       - unapproved custom relation type
       - empty string relation type
       - relation_type with SQL injection attempt
    """

def gen_create_claim() -> dict:
    """Generate create_claim ops with:
       - valid evidence_entity_id
       - null evidence_entity_id
       - evidence pointing to deleted entity
       - confidence outside 0-1 range
       - empty subject/predicate/object
    """
```

### 3.2 Properties to Verify

For every generated patch op:

```
P1: The validator either returns Ok(applied_patch) or Err(structured_error).
    It never panics, never returns unstructured errors, never hangs.

P2: If the validator returns Ok, the database is in a consistent state:
    - All entity_ref fields point to existing, non-deleted entities of the correct type.
    - All status values are in the entity type's enum.
    - All _schema_version values match or are less than the current registry version.
    - All relations have valid from_id and to_id.

P3: If the validator returns Err, the database is unchanged.
    (Patch application is atomic — either all ops in a patch_set apply or none do.)

P4: For update_entity: if expected_updated_at does not match current updated_at,
    the op is always rejected. No exceptions.

P5: For create_claim: if evidence_entity_id is null or references a deleted entity,
    the op is always rejected.

P6: The error message always contains: failing field path, expected type/value, actual value.
```

### 3.3 Volume Targets

```
Fuzz budget: 10,000 random patch ops per entity type (30,000 total).
Expected: 0 panics, 0 silent corruptions, 0 unstructured errors.

Concurrency fuzz: 100 parallel update_entity ops against the same entity.
Expected: Exactly 1 succeeds per round. All others return conflict errors.
```

---

## 4. Load Simulation

### 4.1 Graph Growth Test

```
- Run analytics-metric-tree 10 times with varied inputs.
- Expected: ~50-100 metric entities.
- Dedup pipeline should flag duplicates across runs (e.g., "MRR" appearing in multiple metric trees).
- Embedding search should return relevant results across all runs.
- Graph projection should rebuild in < 1 second.
- related_to ratio should stay below 20% (templates produce typed relations).
```

### 4.2 Concurrent Template Execution

```
- Run 5 analytics-experiment-plan templates simultaneously, all referencing the same metric.
- Expected:
  - 5 experiment entities created (no dedup — different experiments).
  - 5 runs logged.
  - Optimistic locking does not interfere (experiments are creates, not updates).
  - If any template updates the shared metric, locking prevents corruption.
```

### 4.3 Schema Migration Under Load

```
- While a template is running, bump the metric schema version.
- Expected:
  - In-flight run uses the schema version at start time (snapshot in inputs_snapshot).
  - Entities created by the run get the current _schema_version at write time.
  - No partial schema application.
```

---

## 5. Pass/Fail Criteria

The architecture survives if and only if **all** of the following hold:

| Criterion | Threshold |
|---|---|
| Patch validator: zero panics under fuzz | 0 / 30,000 ops |
| Patch validator: zero silent corruptions | 0 / 30,000 ops |
| Optimistic locking: zero silent overwrites | 0 / 100 concurrent rounds |
| Schema migration: zero stale entities after backfill | 0 remaining at old version |
| Dedup: zero cross-type false positives | 0 / all create ops |
| Dedup: zero blocked creations | 0 (dedup is advisory only) |
| Claims: zero ungrounded claims in DB | 0 with NULL evidence_entity_id |
| Embedding search: zero cosine computations outside indexer | 0 (grep verification) |
| Template chain: full provenance reconstructable | 100% of run → entity links verified |
| related_to audit: threshold triggers correctly | at 20% ± 1% |

If any criterion fails, the failure identifies a specific architectural weakness. Fix the weakness, not the test.

---

## 6. What Happens After

If the Analytics slice survives all 10 scenarios, 3 fuzz targets, and 3 load simulations:

1. **Add the next 3 entity types** (task, project, decision) — these are the core types with the most relation complexity.
2. **Add 3 more templates** from a different category (Marketing: `mkt-icp-definition`, `mkt-competitive-intel`, `mkt-messaging-matrix`) — these exercise the prerequisite chain across categories.
3. **Re-run the full harness** — every new entity type and template must pass the same fuzz and scenario tests.
4. **Scale to 10k entities** and verify embedding search performance. If latency exceeds 200ms, integrate sqlite-vss behind the existing `indexer.search_similar()` interface.

The spec is not refined further until something actually breaks.

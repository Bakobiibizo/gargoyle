# Gargoyle Spec v0.7 Correction Plan

Reference: `docs/spec-audit-report.md`
Direction: Code is updated to match spec. Spec is authoritative.

---

## Shared File Conflict Map

These files are edited by multiple agents. Only ONE agent per phase may touch each.

| File | Conflict Risk | Phases That Touch It |
|------|--------------|---------------------|
| `schema/field_def.rs` | LOW | Phase 1 only |
| `schema/registry.rs` | HIGH | Phase 1 (validation logic), Phase 2I (all type arms) |
| `schema/types/mod.rs` | HIGH | Phase 2I only (add 5 modules) |
| `validation/status_validator.rs` | HIGH | Phase 1E (behavior), Phase 2I (all enums) |
| `validation/mod.rs` | MEDIUM | Phase 1B only |
| `patch/apply.rs` | MEDIUM | Phase 1B (wire validation), Phase 3A (add dispatch arms) |
| `models/patch.rs` | LOW | Phase 1D (structure), Phase 3A (new variants) |
| `models/relation.rs` | LOW | Phase 1D only |
| `schema/version.rs` | MEDIUM | Phase 2I only |
| `services/template_runner.rs` | HIGH | Phase 4 only |

---

## Phase 1: Foundation Infrastructure

**Goal:** Fix core type system, validation wiring, and service behavior. No entity type schema changes yet.
**Parallelism:** 8 agents, all independent files.
**Depends on:** Nothing.

### Agent 1A: Extend FieldType enum

**Fixes:** P1-3
**Files:**
- `src-tauri/src/schema/field_def.rs` — Add variants: `Date`, `DateTime`, `Json`, `Array(String)` (items_type), `EntityRefArray(String)` (ref_entity_type). Add `added_in_version: Option<u32>` to `FieldDef`.
- `src-tauri/src/schema/registry.rs` — Update `validate_field_type()` match to handle new variants. `Date`/`DateTime` validate as strings matching ISO patterns. `Json` accepts any valid JSON. `Array` validates as JSON array with typed elements. `EntityRefArray` validates as JSON array of UUIDs referencing entities of the expected type. Update `json_type_name()` if needed.
- Update any tests in `registry.rs` that test field validation to cover new types.

### Agent 1B: Wire validation pipeline into patch apply

**Fixes:** P0-1
**Files:**
- `src-tauri/src/patch/apply.rs` — In `apply_single_op()`, call `validate_create_entity()` before `execute_create_entity()`, `validate_update_entity()` before `execute_update_entity()`, `validate_create_relation()` before `execute_create_relation()`, `validate_create_claim()` before `execute_create_claim()`. Pass the connection to validators. Return validation errors before executing.
- `src-tauri/src/validation/mod.rs` — Ensure all 4 validation functions accept `&Connection` and the required payloads. Fix signatures if needed.
- Remove the duplicate lock check inside `execute_update_entity` (now handled by validator).
- Add integration tests: patch that fails schema validation is rejected, patch that fails status validation is rejected, patch that fails referential integrity is rejected.

### Agent 1C: Add canonical relation type validation

**Fixes:** P0-4
**Files:**
- `src-tauri/src/validation/referential_validator.rs` — Add a `CANONICAL_RELATION_TYPES` constant array with all 22 types from spec §2.2.1: `["part_of", "derived_from", "depends_on", "blocks", "duplicate_of", "implements", "mentions", "supports", "contradicts", "tests", "decides", "evidence_for", "assigned_to", "created_in", "related_to", "targets", "competes_with", "measures", "funds", "enables", "mitigates", "promotes"]`. In `validate_create_relation()`, check that `relation_type` is either in `CANONICAL_RELATION_TYPES` or starts with `"custom:"` and is approved. Reject unknown types with a clear error.
- Fix any tests using `"relates_to"` (typo) to use `"related_to"`.
- Search all test files and service files for `"relates_to"` and fix to `"related_to"`.

### Agent 1D: Fix Rust model types

**Fixes:** P2-6, P2-8, PatchOp structure
**Files:**
- `src-tauri/src/models/relation.rs` — Change `weight: f64` to `weight: Option<f64>`.
- `src-tauri/src/services/graph_builder.rs` — Change `GraphEdge.weight: f64` to `Option<f64>`. Update all code that reads/writes weight to handle `Option`.
- `src-tauri/src/models/patch.rs` — Change `PatchSet.run_id: Option<String>` to `run_id: String` (required). Add `op_id: String` field to each payload struct (or add a wrapper). Add `evidence_entity_ids: Vec<String>` to `CreateClaimPayload`. Add `reason: Option<String>` to `CreateEntityPayload` and `CreateRelationPayload` (it already exists on `UpdateEntityPayload`).
- Update all callers of `PatchSet` to provide `run_id`. Search for `PatchSet {` construction sites.
- Fix any tests that construct PatchSet without run_id.

### Agent 1E: Fix status transition behavior

**Fixes:** P2-1
**Files:**
- `src-tauri/src/validation/status_validator.rs` — In `validate_status_transition()`:
  - Backward transitions WITHOUT reason: change from hard reject to **warning** (return Ok with a warning, not Err). This requires changing the return type or adding a warnings vec. Approach: return `Result<Vec<String>, GargoyleError>` where the Vec contains warnings for backward/skip transitions. Alternatively, add a `StatusValidationResult { valid: bool, warnings: Vec<String> }`.
  - Skip transitions: add logging/warning when intermediate states are skipped.
  - Ensure `null → any valid status` is allowed (not just first).
- Update `validation/mod.rs` to propagate warnings from status validation.
- Update `patch/apply.rs` (or TemplateOutput) to surface warnings.

### Agent 1F: Fix dedup pipeline

**Fixes:** P2-2, P2-10
**Files:**
- `src-tauri/src/services/dedup.rs`:
  - Stage 1 (exact title): Change confidence from `1.0` to `0.95`.
  - Stage 2 (fuzzy title): Replace `normalized_levenshtein >= 0.8` with spec's dual criteria: raw `levenshtein_distance <= 3 OR trigram_similarity > 0.7`. Use the `strsim` crate for Levenshtein distance (already a dependency for normalized_levenshtein). Implement trigram similarity or use a crate. Set confidence to fixed `0.70`.
  - Stage 3 (embedding proximity): Change threshold from `0.85` to `0.92`. Set confidence to fixed `0.80`. Add title length guard: skip embedding proximity when `title.len() < 4`.
  - Add short-circuit: if Stage 1 finds a match with confidence >= 0.95, skip Stages 2 and 3.
- Update dedup tests to reflect new thresholds and behavior.

### Agent 1G: Fix embedding generation

**Fixes:** P2-3
**Files:**
- `src-tauri/src/services/indexer.rs` — In `generate_embedding()` (around line 98), change the SQL query from `SELECT title, canonical_fields FROM entities` to `SELECT title, body_md, canonical_fields FROM entities`. Update the text concatenation to `format!("{} {} {}", title, body_md, canonical_fields)`.
- Update any tests that mock or test embedding generation.

### Agent 1H: Fix graph_builder services

**Fixes:** P2-4, P2-5
**Files:**
- `src-tauri/src/services/graph_builder.rs`:
  - `rebuild_projection()`: Add a call to `audit_related_to()` at the end, so it runs on every rebuild.
  - `audit_related_to()`: Add `WHERE relation_type NOT LIKE 'custom:%'` to the ratio SQL query, matching spec §2.2.4.
  - Add category-scoped subgraph support: Add a new function `get_entity_graph_filtered(entity_id, depth, category: Option<String>)` that adds `WHERE category = ?` filtering. Or modify `get_entity_graph()` to accept an optional category parameter.
  - Fix `approve_custom_type()` to accept and store `proposed_by_run_id`.
- Update corresponding Tauri commands in `src-tauri/src/commands/graph_commands.rs` to expose the new parameter.
- Add tests for category filtering and related_to audit.

### Phase 1 Summary

| Agent | Fixes | Key Files | Size |
|-------|-------|-----------|------|
| 1A | P1-3 | field_def.rs, registry.rs (validation logic) | M |
| 1B | P0-1 | apply.rs, validation/mod.rs | M |
| 1C | P0-4 | referential_validator.rs, test files | M |
| 1D | P2-6, P2-8 | relation.rs, graph_builder.rs, patch.rs | M |
| 1E | P2-1 | status_validator.rs (behavior only, NOT enum values) | M |
| 1F | P2-2, P2-10 | dedup.rs | M |
| 1G | P2-3 | indexer.rs | S |
| 1H | P2-4, P2-5 | graph_builder.rs | M |

**Max parallel: 8 agents.** Agents 1D and 1H both touch `graph_builder.rs` but different parts (1D: GraphEdge struct, 1H: functions). Coordinate by having 1D run first or edit non-overlapping sections. If risky, run 1D before 1H.

**Safest parallel grouping:** Run 1A-1G in parallel (7 agents). Run 1H after 1D completes.

---

## Phase 2: Entity Type Schemas

**Goal:** Fix all 22 existing entity type schemas to match spec. Add 5 missing types. Update all shared registries.
**Parallelism:** 8 agents for type files (parallel), then 1 integration agent (sequential).
**Depends on:** Phase 1A (FieldType enum must be extended first).

### Agent 2A: Fix analytics types — metric.rs, experiment.rs, result.rs

**Fixes:** P1-2 (partial)
**Files:** `src-tauri/src/schema/types/metric.rs`, `experiment.rs`, `result.rs`

**metric.rs** — Match spec lines 359-373:
- Keep: `current_value` (Number, opt), `target_value` (Number, opt), `trend` (Enum, opt), `data_source` (String, opt)
- Add: `last_updated` (Date, opt), `business_objective` (String, opt), `segment_breakdowns` (Array\<String\>, opt), `alert_thresholds` (Json, opt), `unit` (String, opt)
- Status: Keep as-is — `["active", "paused", "deprecated", "archived"]`. Spec says `active → deprecated` but code adds `paused` and `archived` which are reasonable extensions. NOTE: update spec §2.1.4 metric status to match code.

**experiment.rs** — Match spec lines 327-341:
- Fix: `hypothesis` from optional to **required**
- Add: `primary_metric` (String, req), `baseline_value` (Number, opt), `surface` (String, opt), `practical_constraints` (Json, opt), `secondary_metrics` (Array\<String\>, opt), `segment_breakdowns` (Array\<String\>, opt), `traffic_volume` (String, opt)
- Remove: `source_experiment_id` (not in spec — or move to result.rs where it belongs)
- Status: MATCH — keep as-is

**result.rs** — Match spec lines 345-355:
- Replace `findings` and `methodology` with spec fields
- Add: `source_experiment_id` (EntityRef(experiment), opt), `outcome` (String, req), `data_summary` (Json, opt), `next_steps` (Array\<String\>, opt)
- Fix: `confidence_level` from Number to Enum(high/medium/low)
- Status: Change from `["draft", "final", "archived"]` to `["preliminary", "final", "invalidated"]`

### Agent 2B: Fix core types — task.rs, project.rs, decision.rs

**Files:** `src-tauri/src/schema/types/task.rs`, `project.rs`, `decision.rs`

**task.rs** — Match spec lines 218-227:
- Replace `assignee` (String) with `assignee_id` (EntityRef(person), opt)
- Keep: `project_id` (EntityRef(project), opt), `effort_estimate` (String, opt), `acceptance_criteria` (String, opt)
- Add: `blocked_by_ids` (EntityRefArray(task), opt)
- Status: Change to `["open", "in_progress", "blocked", "done", "cancelled"]`

**project.rs** — Match spec lines 232-240:
- Change `owner_id` from String to EntityRef(person)
- Replace `objective`, `success_criteria`, `timeline` with spec fields: `deadline` (Date, opt), `blockers` (Array\<String\>, opt), `success_metric` (String, opt)
- Status: MATCH — keep as-is

**decision.rs** — Match spec lines 244-255:
- Change `owner_id` from String to EntityRef(person)
- Fix `decided_at` from optional to **required**, type Date
- Fix `options_considered` from String to Array\<String\>
- Fix `revisit_triggers` from String/optional to Array\<String\>/**required**
- Add: `constraints` (String, opt)
- Status: Change to `["pending", "decided", "revisited", "superseded"]`

### Agent 2C: Fix core types — session.rs, person.rs, note.rs

**Files:** `src-tauri/src/schema/types/session.rs`, `person.rs`, `note.rs`

**session.rs** — Match spec lines 259-267:
- Replace all code fields (`session_type`, `participants`, `agenda`, `outcomes`) with spec fields: `started_at` (DateTime, req), `ended_at` (DateTime, opt), `focus_entity_ids` (EntityRefArray, opt)
- Status: Change to `["active", "ended"]`

**person.rs** — Match spec lines 297-309:
- Fix `role` from optional to **required**
- Replace `team` with `department` (String, opt)
- Add: `reports_to_id` (EntityRef(person), opt), `tenure_months` (Number, opt), `flight_risk` (Enum(low/medium/high), opt)
- Rename `external` to `is_external` (Boolean, opt)
- Keep: `email` (String, opt)
- Status: Change to `["active", "departed"]`

**note.rs** — Match spec lines 207-213:
- Fix `tags` from String to Array\<String\>
- Replace `context` and `linked_entity_id` with `context_session_id` (EntityRef(session), opt)
- Status: Change to null/None (spec says notes have no lifecycle). Set status_enum to empty or null in the type definition.

### Agent 2D: Fix domain types — spec.rs, campaign.rs, audience.rs

**Files:** `src-tauri/src/schema/types/spec.rs`, `campaign.rs`, `audience.rs`

**spec.rs** — Match spec lines 313-323:
- Fix `spec_type` enum to 11 values: technical, product, api, architecture, db_schema, security, test_plan, cicd, migration, observability, performance. Make **required**.
- Replace `version`, `approval_status`, `author` with spec fields: `owner_id` (EntityRef(person), opt), `target_system` (String, opt), `constraints` (Json, opt), `related_doc_urls` (Array\<String\>, opt)
- Status: Change to `["draft", "review", "approved", "superseded"]`

**campaign.rs** — Match spec lines 377-390:
- Replace `objective`, `budget`, `channel`, `start_date`, `end_date` with spec fields: `campaign_window` (Json, opt), `primary_goal` (Enum(6 values), req), `budget_range` (Json, opt), `channels` (Array\<String\>, opt), `offer` (String, opt), `success_definition` (String, opt), `cta` (String, opt)
- Keep: `target_audience_id` (EntityRef(audience), opt)
- Status: Change to `["planning", "live", "paused", "completed", "cancelled"]`

**audience.rs** — Match spec lines 394-408:
- Replace all code fields with spec fields: `icp_description` (String, req), `industry` (String, opt), `company_size` (String, opt), `roles` (Array\<String\>, opt), `pains` (Array\<String\>, opt), `desired_outcomes` (Array\<String\>, opt), `sales_motion` (Enum(self_serve/sales_led/hybrid), opt), `pricing_model` (String, opt), `churn_reasons` (Array\<String\>, opt)
- Status: Change to `["draft", "validated", "deprecated"]`

### Agent 2E: Fix domain types — competitor.rs, channel.rs, event.rs

**Files:** `src-tauri/src/schema/types/competitor.rs`, `channel.rs`, `event.rs`

**competitor.rs** — Match spec lines 412-423:
- Fix `positioning` to **required**
- Rename `website` to `website_url`
- Fix `strengths` and `weaknesses` from String to Array\<String\>
- Add: `pricing_info` (String, opt), `win_loss_notes` (String, opt)
- Remove: `market_share` (not in spec)
- Status: Change to `["active", "acquired", "defunct", "irrelevant"]`

**channel.rs** — Match spec lines 428-437:
- Fix `channel_type` enum to 10 values (paid_search, paid_social, organic_social, email, seo, partnerships, pr, community, events, direct). Make **required**.
- Replace `cost_model`, `primary_metric_id`, `budget_allocation` with spec fields: `platforms` (Array\<String\>, opt), `cac_estimate` (Number, opt), `performance_data` (Json, opt), `capacity_constraints` (String, opt)
- Status: Change to `["active", "paused", "retired"]`

**event.rs** — Match spec lines 441-453:
- Fix `event_type` to **required** (same 5 values)
- Replace `start_date`, `end_date`, `expected_attendees` with spec fields: `date_window` (Json, opt), `city` (String, opt), `expected_attendance` (Number, opt), `budget_id` (EntityRef(budget), opt), `format` (Enum(in_person/virtual/hybrid), opt)
- Keep: `venue` (String, opt)
- Status: Change to `["concept", "planning", "confirmed", "completed", "cancelled"]`

### Agent 2F: Fix domain types — budget.rs, policy.rs, vendor.rs

**Files:** `src-tauri/src/schema/types/budget.rs`, `policy.rs`, `vendor.rs`

**budget.rs** — Match spec lines 457-468:
- Fix `total_amount` to **required**
- Replace `period`, `allocated`, `spent` with spec fields: `time_horizon` (Json, opt), `expense_categories` (Json, opt), `current_spend` (Number, opt), `runway_months` (Number, opt)
- Keep: `currency` (String, opt)
- Status: Change to `["draft", "approved", "active", "exhausted", "closed"]`

**policy.rs** — Match spec lines 472-482:
- Fix `policy_type` enum to 5 values (privacy, compliance, brand_safety, legal, governance). Make **required**.
- Replace `review_date`, `owner` with spec fields: `regions` (Array\<String\>, opt), `review_stakeholders` (Array\<String\>, opt), `review_cadence` (String, opt)
- Keep: `effective_date` (String/Date, opt)
- Status: Change to `["draft", "review", "active", "superseded"]`

**vendor.rs** — Match spec lines 486-497:
- Fix `vendor_type` enum to spec values (agency, tool, contractor, partner). Make **required**.
- Replace `contract_end`, `primary_contact` with spec fields: `scope_of_work` (String, opt), `owner_id` (EntityRef(person), opt), `quality_bar` (String, opt), `pain_points` (Array\<String\>, opt)
- Keep: `contract_value` (Number, opt)
- Status: Change to `["evaluating", "active", "paused", "terminated"]`

### Agent 2G: Fix domain types — playbook.rs, brief.rs, taxonomy.rs, backlog.rs

**Files:** `src-tauri/src/schema/types/playbook.rs`, `brief.rs`, `taxonomy.rs`, `backlog.rs`

**playbook.rs** — Match spec lines 501-511:
- Fix `playbook_type` enum to 8 values (debugging, incident_response, churn_save, onboarding, migration, testing, CRO, lifecycle). Make **required**.
- Fix `trigger_conditions` from String to Array\<String\>
- Replace `expected_outcome`, `owner` with spec fields: `steps` (Json, opt), `owner_id` (EntityRef(person), opt), `last_used` (Date, opt)
- Status: Change to `["draft", "active", "deprecated"]`

**brief.rs** — Match spec lines 515-526:
- Replace all code fields with spec fields: `initiative_name` (String, req), `target_user` (String, opt), `desired_outcome` (String, req), `success_metric` (String, opt), `target_date` (Date, opt), `teams_involved` (Array\<String\>, opt)
- Status: Change to `["draft", "approved", "in_progress", "completed"]`

**taxonomy.rs** — Match spec lines 530-540:
- Fix `taxonomy_type` enum to 4 values (naming_convention, utm_governance, classification, information_architecture). Make **required**.
- Replace `parent_id`, `level` with spec fields: `primary_channels` (Array\<String\>, opt), `reporting_destination` (String, opt), `dimensions` (Array\<String\>, opt), `rules` (Json, opt)
- Status: Change to `["draft", "active", "superseded"]`

**backlog.rs** — Match spec lines 557-566:
- Replace all code fields with spec fields: `item_count` (Number, opt), `top_outcomes` (Array\<String\>, opt), `immovable_deadlines` (Json, opt), `last_triaged` (Date, opt)
- Status: Change to `["needs_triage", "triaged", "stale"]`

### Agent 2H: Add 5 missing entity types

**Fixes:** P0-3
**Files:** Create new files in `src-tauri/src/schema/types/`:

**inbox_item.rs** — Spec lines 193-202:
- Fields: `source_text` (String, req), `source_url` (String, opt), `suggested_type` (String, opt), `suggested_title` (String, opt)
- Status: `["unprocessed", "triaged", "archived"]`

**artifact.rs** — Spec lines 271-281 (entity type schema, not the artifacts table):
- Fields: `artifact_kind` (Enum(attachment/link/export/rendered_doc), req), `uri_or_path` (String, req), `hash` (String, opt), `mime` (String, opt), `parent_entity_id` (EntityRef, opt)
- Status: None (null)

**concept.rs** — Spec lines 285-293:
- Fields: `definition` (String, opt), `aliases` (Array\<String\>, opt), `domain` (String, opt)
- Status: None (null)

**commitment.rs** — Spec lines 544-553:
- Fields: `owner_id` (EntityRef(person), req), `deadline` (Date, opt), `source_context` (String, opt), `tracking_tool` (String, opt)
- Status: `["on_track", "at_risk", "blocked", "fulfilled", "broken"]`

**issue.rs** — Spec lines 570-580:
- Fields: `severity` (Enum(critical/high/medium/low), req), `first_observed` (Date, opt), `affected_area` (String, opt), `owner_id` (EntityRef(person), opt), `resolution_notes` (String, opt)
- Status: `["open", "investigating", "mitigated", "resolved", "wont_fix"]`

Follow the exact pattern of existing type files: `<TYPE>_STATUSES` const, `<type>_v1_fields()`, `<type>_fields(version)`, `<type>_current_version()`.

### Agent 2I: Integration — Update all shared registries (SEQUENTIAL, after 2A-2H)

**Fixes:** P1-1, P1-4
**Files:**
- `src-tauri/src/schema/types/mod.rs` — Add `pub mod inbox_item;`, `pub mod artifact_type;` (avoid name collision with models/artifact.rs), `pub mod concept;`, `pub mod commitment;`, `pub mod issue;`
- `src-tauri/src/schema/registry.rs` — Update `get_schema()`, `current_version()`, `valid_statuses()` match arms for ALL 27 types. Add 5 new type arms. Update 22 existing arms to match the new field definitions and status enums from agents 2A-2G.
- `src-tauri/src/validation/status_validator.rs` — Replace all 21 `*_STATUSES` consts with the spec-correct values. Add 5 new consts for missing types. Update `statuses_for_entity_type()` match to cover all 27 types. Add handling for types with null/no status (note, artifact, concept).
- `src-tauri/src/schema/version.rs` — Update `SchemaVersion::new()` to register ALL 27 entity types (currently only 3).
- `src-tauri/migrations/005_new_entity_types.sql` — Add partial indexes for the 5 new entity types.
- Run `cargo check` to verify compilation.

### Phase 2 Summary

| Agent | Types | Key Files | Size |
|-------|-------|-----------|------|
| 2A | metric, experiment, result | 3 type files | M |
| 2B | task, project, decision | 3 type files | M |
| 2C | session, person, note | 3 type files | M |
| 2D | spec, campaign, audience | 3 type files | M |
| 2E | competitor, channel, event | 3 type files | M |
| 2F | budget, policy, vendor | 3 type files | M |
| 2G | playbook, brief, taxonomy, backlog | 4 type files | M |
| 2H | inbox_item, artifact, concept, commitment, issue | 5 new type files | M |
| 2I | Integration (SEQUENTIAL) | registry.rs, status_validator.rs, mod.rs, version.rs, migration | L |

**Max parallel: 8 agents (2A-2H).** Then 2I runs alone after all complete.

---

## Phase 3: Patch Protocol Completion

**Goal:** Add 6 missing patch op types with their execution handlers.
**Parallelism:** 1 infrastructure agent, then 3 parallel handler agents.
**Depends on:** Phase 1B (validation wiring), Phase 1D (patch.rs structure).

### Agent 3A: Patch infrastructure — Add op variants and dispatch

**Fixes:** P0-2 (infrastructure only)
**Files:**
- `src-tauri/src/models/patch.rs` — Add 6 new PatchOp variants with payload structs:
  ```
  DeleteRelation { relation_id: String }
  AttachArtifact { entity_id: String, kind: String, uri_or_path: String, hash: Option<String>, mime: Option<String> }
  MergeEntities { source_id: String, target_id: String, merge_strategy: String }
  UpdateContext { key: String, value: serde_json::Value }
  PromoteClaim { claim_id: String, target_entity_type: Option<String> }
  ProposeRelationType { type_key: String, description: String, expected_from_types: Option<Vec<String>>, expected_to_types: Option<Vec<String>> }
  ```
- `src-tauri/src/patch/apply.rs` — Add match arms in `apply_single_op()` that call `execute_*` functions (stub with `todo!()` initially).
- Create empty module files: `src-tauri/src/patch/delete_relation.rs`, `attach_artifact.rs`, `merge_entities.rs`, `update_context_op.rs`, `promote_claim_op.rs`, `propose_relation_type.rs`
- Update `src-tauri/src/patch/mod.rs` to declare new modules.

### Agent 3B: Implement delete_relation + attach_artifact handlers

**Files:**
- `src-tauri/src/patch/delete_relation.rs` — Soft-delete by setting a `deleted_at` on the relation (may need migration to add this column), or hard-delete with provenance logging. Follow spec: "Delete operations soft-delete only."
- `src-tauri/src/patch/attach_artifact.rs` — Insert into `artifacts` table using the payload fields. Link to entity via `entity_id`.
- Add validation in `validation/mod.rs`: `validate_delete_relation()` (check relation exists, not already deleted), `validate_attach_artifact()` (check entity exists).
- Add tests.

### Agent 3C: Implement merge_entities + update_context handlers

**Files:**
- `src-tauri/src/patch/merge_entities.rs` — Implement merge: move all relations from source to target, merge canonical_fields (target wins on conflict), soft-delete source, create `duplicate_of` relation. Must include confirmation gate (return a "needs_confirmation" status before executing).
- `src-tauri/src/patch/update_context_op.rs` — Delegate to `ContextManager::set()`. Validate that only `initialize` template or explicit user edits may issue this op (check `run.template_key`).
- Add tests.

### Agent 3D: Implement promote_claim + propose_relation_type handlers

**Files:**
- `src-tauri/src/patch/promote_claim_op.rs` — Implement spec §2.6.1: create `concept` entity from claim triple, set `promoted_to_entity_id` on claim, create `evidence_for` relation from new entity to `evidence_entity_id`.
- `src-tauri/src/patch/propose_relation_type.rs` — Insert into `custom_relation_types` with `approved_at = NULL` (pending). Validation: check type_key starts with `custom:`, check no duplicate canonical semantics. Requires separate approval step (already exists as `approve_custom_relation_type` command).
- Add tests.

### Phase 3 Summary

| Agent | Handlers | Key Files | Size |
|-------|----------|-----------|------|
| 3A | Infrastructure (all 6 stubs) | patch.rs, apply.rs, new module files | M |
| 3B | delete_relation, attach_artifact | 2 handler files + validation | M |
| 3C | merge_entities, update_context | 2 handler files + validation | L |
| 3D | promote_claim, propose_relation_type | 2 handler files + validation | M |

**Sequencing:** 3A first, then 3B/3C/3D in parallel.

---

## Phase 4: Template System

**Goal:** Fix TemplateDefinition struct, add maturity tiers, fix output format, reconcile template names.
**Parallelism:** 2-3 agents, some sequential.
**Depends on:** Phase 2 (entity type schemas must be correct for template output validation).

### Agent 4A: Fix TemplateDefinition + Prerequisites + MaturityTier

**Fixes:** P1-7, P2-7
**Files:**
- `src-tauri/src/services/template_runner.rs` — Update structs:
  ```rust
  pub struct TemplateDefinition {
      pub key: String,
      pub version: String,
      pub category: String,
      pub maturity_tier: MaturityTier,
      pub prerequisites: Vec<Prerequisite>,
      pub produced_entity_types: Vec<String>,
      pub produced_relation_types: Vec<String>,
  }

  pub enum MaturityTier {
      Foundational,
      Workflow,
      Advanced,
      Diagnostic,
  }

  pub struct Prerequisite {
      pub entity_type: String,
      pub min_count: usize,
      pub suggested_template: Option<String>,
      pub reason: String,
  }
  ```
- Update `get_template_definition()` for all 23 templates to populate new fields. Assign maturity tiers per spec §4.2:
  - **Foundational:** analytics-metric-tree, mkt-icp-definition, mkt-competitive-intel
  - **Workflow:** analytics-experiment-plan, dev-adr-writer, dev-api-design, dev-db-schema, dev-prd-to-techspec, dev-requirements-to-spec, dev-test-plan, dev-migration-plan, org-decision-log, org-project-charter, org-project-plan, org-meeting-brief, content-case-study-builder, content-creative-brief-builder
  - **Advanced:** dev-architecture-review, dev-security-threat-model, mkt-positioning-narrative, content-strategy-pillars-seo
  - **Diagnostic:** analytics-anomaly-investigation, org-retrospective
- Add `suggested_template` and `reason` to all prerequisite declarations.
- Fix prerequisite behavior: change from blocking to advisory (return warnings, not errors) unless caller opts in to strict mode.
- Update `check_prerequisites()` return type to include suggestions.
- Update TypeScript types in `src/types/` to match new struct shapes.

### Agent 4B: Fix TemplateOutput / machine payload format

**Fixes:** P1-6
**Files:**
- `src-tauri/src/services/template_runner.rs` — Replace `TemplateOutput` with spec-compliant format:
  ```rust
  pub struct TemplateOutput {
      pub run_id: String,
      pub template_key: String,
      pub template_category: String,
      pub created_at: String,
      pub produced_entities: Vec<ProducedEntity>,
      pub produced_relations: Vec<ProducedRelation>,
      pub action_items: Vec<String>,
      pub decisions_needed: Vec<String>,
      pub risks: Vec<String>,
      pub assumptions: Vec<String>,
      pub open_questions: Vec<String>,
      pub patch_result: PatchResult,
      pub warnings: Vec<String>,
  }

  pub struct ProducedEntity {
      pub entity_id: String,
      pub entity_type: String,
      pub title: String,
      pub status: Option<String>,
  }

  pub struct ProducedRelation {
      pub relation_id: String,
      pub from_ref: String,
      pub to_ref: String,
      pub relation_type: String,
  }
  ```
- Update `run_template()` to populate all new fields. After applying the patch set, read back created entities/relations to fill `produced_entities` and `produced_relations`.
- Update all 23 generator functions to return structured metadata (action_items, risks, etc.) alongside ops. This may require changing the generator return type.
- Update TypeScript types and UI components that consume TemplateOutput.
- Update the `run_template` Tauri command response.

### Agent 4C: Reconcile template names and code-only templates

**Fixes:** P1-8, P3-3
**Files:**
- `src-tauri/src/services/template_runner.rs`:
  - Rename `analytics-anomaly-investigation` to `analytics-anomaly-detection-investigation` (matching spec).
  - For the 4 code-only templates (`org-project-charter`, `org-project-plan`, `org-meeting-brief`, `org-retrospective`): these are useful templates that fill gaps in the spec's "Org" category. **Decision: keep them in code and update the spec to include them.** Update their category from `"organizing"` to `"org"` if needed to match spec's category separation.
  - Update `ALL_TEMPLATE_KEYS` to reflect the rename.
- Update `docs/gargoyle-spec-v0.7.md` §4.2 Org section to add the 4 templates.
- Update `docs/TEMPLATE_INPUT_REFERENCE.md` to reflect the template rename.
- Update any tests referencing the old template name.

### Phase 4 Summary

| Agent | Focus | Key Files | Size |
|-------|-------|-----------|------|
| 4A | TemplateDefinition + MaturityTier + Prerequisites | template_runner.rs (structs + definitions) | L |
| 4B | TemplateOutput / machine payload | template_runner.rs (output + generators) | L |
| 4C | Template name reconciliation | template_runner.rs (keys), spec, docs | S |

**Sequencing:** 4A and 4C can run in parallel (4A edits structs/definitions, 4C edits keys/names). 4B should run after 4A (needs new struct definitions). Or 4A and 4B can be combined into one large agent if preferred.

---

## Phase 5: TypeScript + UI + Migration

**Goal:** Update frontend types, add migration for new entity types, fix tests.
**Parallelism:** 3-4 agents.
**Depends on:** Phases 2, 3, 4.

### Agent 5A: Migration file for Phase 2-3 changes

**Files:**
- `src-tauri/migrations/005_spec_alignment.sql`:
  - Partial indexes for 5 new entity types (inbox_item, artifact_type, concept, commitment, issue)
  - Add `deleted_at` column to `relations` table if needed for soft-delete (Phase 3B)
  - Any generated columns if we decide to implement spec line 75 (P2-9)
- Verify migration applies cleanly on fresh DB and existing DB.

### Agent 5B: Update TypeScript types

**Files:**
- `src/types/index.ts` — Update all entity type interfaces to match new schema fields. Add types for 5 new entity types. Update `TemplateDefinition`, `TemplateOutput`, `Prerequisite`, `MaturityTier` types. Add `ProducedEntity`, `ProducedRelation` types.
- `src/api/` — Update any API wrapper types that changed.

### Agent 5C: Fix tests and verify compilation

**Files:**
- Search all test files for `"relates_to"` and fix to `"related_to"` (P3-4)
- Update test fixtures that construct entities with old field names/values
- Update test fixtures that construct PatchSets without required `run_id`
- Run `cargo check` then `cargo test` — fix any compilation errors
- Run `npx tsc --noEmit` — fix any TypeScript errors

### Agent 5D: Update documentation

**Files:**
- `docs/gargoyle-spec-v0.7.md`:
  - §2.1 entities field list: add `provenance_run_id` (P3-5)
  - §4.2 Org section: add 4 code-only templates (org-project-charter, org-project-plan, org-meeting-brief, org-retrospective)
  - §2.1.4 metric status: update to include `paused` and `archived` (if we kept them in code)
  - Any other minor spec updates where code made reasonable extensions
- `docs/TEMPLATE_INPUT_REFERENCE.md` — Update template name (anomaly investigation)
- `docs/e2e-testing.md` — Update entity type count (22 → 27), template count if changed
- `docs/stress-test-spec.md` — Update entity type count

### Phase 5 Summary

| Agent | Focus | Key Files | Size |
|-------|-------|-----------|------|
| 5A | Migration file | migrations/005_spec_alignment.sql | S |
| 5B | TypeScript types | src/types/index.ts, src/api/ | M |
| 5C | Test fixes + verification | Various test files | L |
| 5D | Documentation updates | docs/*.md | M |

**Max parallel: 4 agents.** 5A and 5B can start immediately. 5C should run after 5A/5B. 5D can run in parallel with all.

---

## Phase 6: Stress Test + Final Verification

**Goal:** Ensure everything compiles, all tests pass, stress tests cover new types.
**Parallelism:** 1-2 agents.
**Depends on:** All previous phases.

### Agent 6A: Extended stress test update

**Files:**
- `src-tauri/tests/common/generators.rs` — Extend proptest generators to cover all 27 entity types with correct field types and status enums.
- Update fuzz budget to include new entity types.
- Add scenarios for new FieldTypes (Date, DateTime, Json, Array, EntityRefArray validation).
- Add scenarios for new patch op types (delete_relation, attach_artifact, merge_entities).
- Verify: 0 panics, 0 silent corruptions.

### Agent 6B: Full build verification

- `cargo test` — all tests pass
- `cargo test --test stress_extended -- --nocapture` — stress tests pass
- `npx tsc --noEmit` — TypeScript compiles
- `npx vite build` — production build succeeds

---

## Full Phase Summary

| Phase | Goal | Agents | Max Parallel | Depends On |
|-------|------|--------|-------------|-----------|
| 1 | Foundation infrastructure | 8 | 7-8 | — |
| 2 | Entity type schemas | 9 | 8 (then 1) | Phase 1A |
| 3 | Patch protocol completion | 4 | 1 (then 3) | Phase 1B, 1D |
| 4 | Template system | 3 | 2 | Phase 2 |
| 5 | TypeScript + UI + migration + docs | 4 | 3-4 | Phases 2, 3, 4 |
| 6 | Stress test + verification | 2 | 2 | All |
| **Total** | | **30 agents** | | |

## Execution Order

```
Phase 1 (8 agents parallel)
    ↓
Phase 2A-2H (8 agents parallel)  ←── depends on 1A
Phase 3A (1 agent)                ←── depends on 1B, 1D
    ↓                                 ↓
Phase 2I (1 agent, after 2A-2H)  Phase 3B-3D (3 agents parallel, after 3A)
    ↓
Phase 4A+4C (2 agents parallel)  ←── depends on 2I
    ↓
Phase 4B (1 agent, after 4A)
    ↓
Phase 5A-5D (4 agents parallel)  ←── depends on 2, 3, 4
    ↓
Phase 6A-6B (2 agents)           ←── depends on all
```

**Critical path:** Phase 1A → Phase 2A-2H → Phase 2I → Phase 4A → Phase 4B → Phase 5C → Phase 6B

**Estimated total agent-phases:** 6 rounds of agent launches.

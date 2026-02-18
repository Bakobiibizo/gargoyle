# Gargoyle Spec v0.7 vs. Codebase — Full Audit Report

Generated: 2026-02-16

## Executive Summary

The spec describes a significantly larger system than what's implemented. The core infrastructure (tables, services, patch protocol) is mostly in place but with gaps. The entity schemas and template registry have substantial drift.

| Area | Spec | Code | Coverage |
|------|------|------|----------|
| Entity types | 27 | 22 | 81% (5 missing) |
| Templates | 98 | 23 | 23% (19 match spec, 4 are code inventions) |
| Categories | 14 | 6 | 43% |
| Patch op types | 10 | 4 | 40% |
| UI surfaces | 11 | 7 tabs | ~36% match |
| Services | 5 | 8 | Exceeds spec |

---

## 1. Entities Table (§2.1)

**SQLite DDL: MATCH** — All 14 columns present. One extra: `provenance_run_id` is in the migration but not listed in the spec's field list (though referenced later at spec line 765).

**Missing features:**
- **Generated columns** (spec line 75: `_status_extracted`) — NOT IMPLEMENTED
- **Partial index mismatch** — Spec example indexes tasks on `(due_at, priority)`, code indexes all types on `(status)` only
- **FTS5** — Implemented correctly

---

## 2. Entity Types (§2.1.1)

**5 types in spec but MISSING from code:**
- `inbox_item` — unprocessed capture (spec lines 84, 193-202)
- `artifact` — file, link, or rendered output (spec lines 90, 271-281)
- `concept` — abstract idea or domain term (spec lines 91, 285-293)
- `commitment` — promise or obligation with owner and deadline (spec lines 111, 544-553)
- `issue` — known problem or risk with severity and status (spec lines 113, 570-580)

**0 types in code not in spec** — code is a strict subset.

**FieldType system gap** — Code's `FieldType` enum (`src-tauri/src/schema/field_def.rs`, lines 12-18) only supports `String`, `Number`, `Boolean`, `Enum`, `EntityRef`. The spec's meta-schema (line 163) also requires `date`, `datetime`, `json`, `array`, `entity_ref_array` — none of which exist in code. This causes systemic downgrading of array/json/date fields to plain strings throughout the schema registry.

**SchemaVersion incomplete** — `src-tauri/src/schema/version.rs` lines 24-29: Only tracks 3 types (metric, experiment, result) despite 22 in the registry.

---

## 3. Schema Registry — Per-Type Field Drift (§2.1.4)

Every implemented type has field-level deviations. Pattern: spec fields are richer (more fields, typed entity_refs, arrays, required constraints) while code fields are simpler (fewer fields, plain strings, everything optional).

### 3.1 metric

**Files:** Spec lines 359-373, Code `src-tauri/src/schema/types/metric.rs`

| Field | Spec | Code | Status |
|-------|------|------|--------|
| current_value | number, opt | Number, opt | MATCH |
| target_value | number, opt | Number, opt | MATCH |
| trend | enum(up/down/flat), opt | Enum(up/down/flat), opt | MATCH |
| last_updated | date, opt | — | MISSING IN CODE |
| business_objective | string, opt | — | MISSING IN CODE |
| data_source | string, opt | String, opt | MATCH |
| segment_breakdowns | array\<string\>, opt | — | MISSING IN CODE |
| alert_thresholds | json, opt | — | MISSING IN CODE |
| unit | string, opt | — | MISSING IN CODE |

**Status:** Spec `active → deprecated` / Code `active → paused → deprecated → archived` — code adds `paused`, `archived`

### 3.2 experiment

**Files:** Spec lines 327-341, Code `src-tauri/src/schema/types/experiment.rs`

| Field | Spec | Code | Status |
|-------|------|------|--------|
| hypothesis | string, **req** | String, opt | REQ MISMATCH |
| funnel_position | string, opt | String, opt | MATCH |
| primary_metric | string, **req** | — | MISSING IN CODE |
| baseline_value | number, opt | — | MISSING IN CODE |
| surface | string, opt | — | MISSING IN CODE |
| practical_constraints | json, opt | — | MISSING IN CODE |
| secondary_metrics | array\<string\>, opt | — | MISSING IN CODE |
| segment_breakdowns | array\<string\>, opt | — | MISSING IN CODE |
| traffic_volume | string, opt | — | MISSING IN CODE |
| source_experiment_id | — | EntityRef(experiment), opt | IN CODE, NOT IN SPEC |

**Status:** MATCH — `draft → running → concluded → archived`

### 3.3 result

**Files:** Spec lines 345-355, Code `src-tauri/src/schema/types/result.rs`

| Field | Spec | Code | Status |
|-------|------|------|--------|
| source_experiment_id | entity_ref(experiment), opt | — | MISSING IN CODE |
| outcome | string, **req** | — | MISSING IN CODE |
| data_summary | json, opt | — | MISSING IN CODE |
| confidence_level | enum(high/medium/low), opt | Number, opt | TYPE MISMATCH |
| next_steps | array\<string\>, opt | — | MISSING IN CODE |
| findings | — | String, opt | IN CODE, NOT IN SPEC |
| methodology | — | String, opt | IN CODE, NOT IN SPEC |

**Status:** Spec `preliminary → final → invalidated` / Code `draft → final → archived` — DEVIATION

### 3.4 task

**Files:** Spec lines 218-227, Code `src-tauri/src/schema/types/task.rs`

| Field | Spec | Code | Status |
|-------|------|------|--------|
| assignee_id | entity_ref(person), opt | — | MISSING (code has `assignee` as String) |
| project_id | entity_ref(project), opt | EntityRef(project), opt | MATCH |
| effort_estimate | string, opt | String, opt | MATCH |
| acceptance_criteria | string, opt | String, opt | MATCH |
| blocked_by_ids | entity_ref_array(task), opt | — | MISSING IN CODE |
| assignee | — | String, opt | IN CODE, NOT IN SPEC |

**Status:** Spec `open → in_progress → blocked → done → cancelled` / Code `backlog → todo → in_progress → blocked → done → archived` — DEVIATION

### 3.5 project

**Files:** Spec lines 232-240, Code `src-tauri/src/schema/types/project.rs`

| Field | Spec | Code | Status |
|-------|------|------|--------|
| owner_id | entity_ref(person), opt | String, opt | TYPE MISMATCH (entity_ref → String) |
| deadline | date, opt | — | MISSING IN CODE |
| blockers | array\<string\>, opt | — | MISSING IN CODE |
| success_metric | string, opt | — | MISSING IN CODE |
| objective | — | String, opt | IN CODE, NOT IN SPEC |
| success_criteria | — | String, opt | IN CODE, NOT IN SPEC |
| timeline | — | String, opt | IN CODE, NOT IN SPEC |

**Status:** MATCH — `planning → active → paused → completed → archived`

### 3.6 decision

**Files:** Spec lines 244-255, Code `src-tauri/src/schema/types/decision.rs`

| Field | Spec | Code | Status |
|-------|------|------|--------|
| owner_id | entity_ref(person), **req** | String, req | TYPE MISMATCH (entity_ref → String) |
| decided_at | date, **req** | String, opt | REQ MISMATCH |
| rationale | string, **req** | String, req | MATCH |
| options_considered | array\<string\>, opt | String, opt | TYPE MISMATCH (array → String) |
| constraints | string, opt | — | MISSING IN CODE |
| revisit_triggers | array\<string\>, **req** | String, opt | TYPE+REQ MISMATCH |

**Status:** Spec `pending → decided → revisited → superseded` / Code `proposed → accepted → deprecated → superseded` — 3 of 4 values differ

### 3.7 session

**Files:** Spec lines 259-267, Code `src-tauri/src/schema/types/session.rs`

| Field | Spec | Code | Status |
|-------|------|------|--------|
| started_at | datetime, **req** | — | MISSING IN CODE |
| ended_at | datetime, opt | — | MISSING IN CODE |
| focus_entity_ids | entity_ref_array, opt | — | MISSING IN CODE |
| session_type | — | Enum(planning/review/standup/workshop/retro), opt | IN CODE, NOT IN SPEC |
| participants | — | String, opt | IN CODE, NOT IN SPEC |
| agenda | — | String, opt | IN CODE, NOT IN SPEC |
| outcomes | — | String, opt | IN CODE, NOT IN SPEC |

**Status:** Spec `active → ended` / Code `scheduled → in_progress → completed → cancelled` — completely different

### 3.8 person

**Files:** Spec lines 297-309, Code `src-tauri/src/schema/types/person.rs`

| Field | Spec | Code | Status |
|-------|------|------|--------|
| role | string, **req** | String, opt | REQ MISMATCH |
| department | string, opt | — | MISSING (code has `team` instead) |
| reports_to_id | entity_ref(person), opt | — | MISSING IN CODE |
| tenure_months | number, opt | — | MISSING IN CODE |
| flight_risk | enum(low/medium/high), opt | — | MISSING IN CODE |
| email | string, opt | String, opt | MATCH |
| is_external | boolean, opt | — | MISSING (code has `external` as Boolean) |
| external | — | Boolean, opt | RENAMED from `is_external` |
| team | — | String, opt | IN CODE, NOT IN SPEC |

**Status:** Spec `active → departed` / Code `active → inactive → archived` — DEVIATION

### 3.9 note

**Files:** Spec lines 207-213, Code `src-tauri/src/schema/types/note.rs`

| Field | Spec | Code | Status |
|-------|------|------|--------|
| tags | array\<string\>, opt | String, opt | TYPE MISMATCH (array → String) |
| context_session_id | entity_ref(session), opt | — | MISSING IN CODE |
| context | — | String, opt | IN CODE, NOT IN SPEC |
| linked_entity_id | — | EntityRef(*), opt | IN CODE, NOT IN SPEC |

**Status:** Spec `null` (no lifecycle) / Code `draft → final → archived` — spec says notes have NO lifecycle; code gives them 3 statuses

### 3.10 spec

**Files:** Spec lines 313-323, Code `src-tauri/src/schema/types/spec.rs`

| Field | Spec | Code | Status |
|-------|------|------|--------|
| spec_type | enum(11 values), **req** | Enum(4 values), opt | ENUM+REQ MISMATCH |
| owner_id | entity_ref(person), opt | — | MISSING IN CODE |
| target_system | string, opt | — | MISSING IN CODE |
| constraints | json, opt | — | MISSING IN CODE |
| related_doc_urls | array\<string\>, opt | — | MISSING IN CODE |
| version | — | String, opt | IN CODE, NOT IN SPEC |
| approval_status | — | String, opt | IN CODE, NOT IN SPEC |
| author | — | String, opt | IN CODE, NOT IN SPEC |

Spec `spec_type` values: technical, product, api, architecture, db_schema, security, test_plan, cicd, migration, observability, performance
Code `spec_type` values: technical, product, design, process

**Status:** Spec `draft → review → approved → superseded` / Code `draft → review → approved → deprecated` — `superseded` → `deprecated`

### 3.11 campaign

**Files:** Spec lines 377-390, Code `src-tauri/src/schema/types/campaign.rs`

| Field | Spec | Code | Status |
|-------|------|------|--------|
| campaign_window | json, opt | — | MISSING (code has start_date/end_date) |
| primary_goal | enum(6 values), **req** | — | MISSING IN CODE |
| target_audience_id | entity_ref(audience), opt | EntityRef(audience), opt | MATCH |
| budget_range | json, opt | — | MISSING (code has `budget` as Number) |
| channels | array\<string\>, opt | — | MISSING (code has `channel` as Enum) |
| offer | string, opt | — | MISSING IN CODE |
| success_definition | string, opt | — | MISSING IN CODE |
| cta | string, opt | — | MISSING IN CODE |
| objective | — | String, opt | IN CODE, NOT IN SPEC |
| budget | — | Number, opt | IN CODE, NOT IN SPEC |
| channel | — | Enum(6 values), opt | IN CODE, NOT IN SPEC |
| start_date | — | String, opt | IN CODE, NOT IN SPEC |
| end_date | — | String, opt | IN CODE, NOT IN SPEC |

**Status:** Spec `planning → live → paused → completed → cancelled` / Code `planning → active → paused → completed → archived` — `live` → `active`, `cancelled` → `archived`

### 3.12 audience

**Files:** Spec lines 394-408, Code `src-tauri/src/schema/types/audience.rs`

| Field | Spec | Code | Status |
|-------|------|------|--------|
| icp_description | string, **req** | — | MISSING IN CODE |
| industry | string, opt | — | MISSING IN CODE |
| company_size | string, opt | — | MISSING IN CODE |
| roles | array\<string\>, opt | — | MISSING IN CODE |
| pains | array\<string\>, opt | — | MISSING IN CODE |
| desired_outcomes | array\<string\>, opt | — | MISSING IN CODE |
| sales_motion | enum, opt | — | MISSING IN CODE |
| pricing_model | string, opt | — | MISSING IN CODE |
| churn_reasons | array\<string\>, opt | — | MISSING IN CODE |
| segment_criteria | — | String, opt | IN CODE, NOT IN SPEC |
| estimated_size | — | Number, opt | IN CODE, NOT IN SPEC |
| icp_id | — | EntityRef(person), opt | IN CODE, NOT IN SPEC |
| channels | — | String, opt | IN CODE, NOT IN SPEC |

**Status:** Spec `draft → validated → deprecated` / Code `draft → validated → active → archived` — code adds `active`, replaces `deprecated`

### 3.13 competitor

**Files:** Spec lines 412-423, Code `src-tauri/src/schema/types/competitor.rs`

| Field | Spec | Code | Status |
|-------|------|------|--------|
| positioning | string, **req** | String, opt | REQ MISMATCH |
| website_url | string, opt | — | MISSING (code has `website`) |
| strengths | array\<string\>, opt | String, opt | TYPE MISMATCH |
| weaknesses | array\<string\>, opt | String, opt | TYPE MISMATCH |
| pricing_info | string, opt | — | MISSING IN CODE |
| win_loss_notes | string, opt | — | MISSING IN CODE |
| website | — | String, opt | RENAMED from `website_url` |
| market_share | — | String, opt | IN CODE, NOT IN SPEC |

**Status:** Spec `active → acquired → defunct → irrelevant` / Code `tracking → dormant → archived` — completely different

### 3.14 channel

**Files:** Spec lines 428-437, Code `src-tauri/src/schema/types/channel.rs`

| Field | Spec | Code | Status |
|-------|------|------|--------|
| channel_type | enum(10 values), **req** | Enum(8 values), opt | ENUM+REQ MISMATCH |
| platforms | array\<string\>, opt | — | MISSING IN CODE |
| cac_estimate | number, opt | — | MISSING IN CODE |
| performance_data | json, opt | — | MISSING IN CODE |
| capacity_constraints | string, opt | — | MISSING IN CODE |
| cost_model | — | String, opt | IN CODE, NOT IN SPEC |
| primary_metric_id | — | EntityRef(metric), opt | IN CODE, NOT IN SPEC |
| budget_allocation | — | Number, opt | IN CODE, NOT IN SPEC |

Spec enum: paid_search, paid_social, organic_social, email, seo, partnerships, pr, community, events, direct
Code enum: email, social, search, display, events, partnerships, content, referral

**Status:** Spec `active → paused → retired` / Code `evaluating → active → scaling → paused → deprecated` — completely different

### 3.15 event

**Files:** Spec lines 441-453, Code `src-tauri/src/schema/types/event.rs`

| Field | Spec | Code | Status |
|-------|------|------|--------|
| event_type | enum(5 values), **req** | Enum(5 values), opt | REQ MISMATCH (same values) |
| date_window | json, opt | — | MISSING (code has start_date/end_date) |
| city | string, opt | — | MISSING IN CODE |
| expected_attendance | number, opt | — | MISSING (code has `expected_attendees`) |
| budget_id | entity_ref(budget), opt | — | MISSING IN CODE |
| venue | string, opt | String, opt | MATCH |
| format | enum(in_person/virtual/hybrid), opt | — | MISSING IN CODE |
| start_date | — | String, opt | IN CODE, NOT IN SPEC |
| end_date | — | String, opt | IN CODE, NOT IN SPEC |
| expected_attendees | — | Number, opt | RENAMED from expected_attendance |

**Status:** Spec `concept → planning → confirmed → completed → cancelled` / Code `proposed → confirmed → in_progress → completed → cancelled` — DEVIATION

### 3.16 budget

**Files:** Spec lines 457-468, Code `src-tauri/src/schema/types/budget.rs`

| Field | Spec | Code | Status |
|-------|------|------|--------|
| total_amount | number, **req** | Number, opt | REQ MISMATCH |
| currency | string, opt | String, opt | MATCH |
| time_horizon | json, opt | — | MISSING (code has `period` as String) |
| expense_categories | json, opt | — | MISSING IN CODE |
| current_spend | number, opt | — | MISSING (code has `spent`) |
| runway_months | number, opt | — | MISSING IN CODE |
| period | — | String, opt | IN CODE, NOT IN SPEC |
| allocated | — | Number, opt | IN CODE, NOT IN SPEC |
| spent | — | Number, opt | RENAMED from current_spend |

**Status:** Spec `draft → approved → active → exhausted → closed` / Code `draft → approved → active → closed` — `exhausted` missing

### 3.17 policy

**Files:** Spec lines 472-482, Code `src-tauri/src/schema/types/policy.rs`

| Field | Spec | Code | Status |
|-------|------|------|--------|
| policy_type | enum(5 values), **req** | Enum(4 values), opt | ENUM+REQ MISMATCH |
| regions | array\<string\>, opt | — | MISSING IN CODE |
| review_stakeholders | array\<string\>, opt | — | MISSING IN CODE |
| effective_date | date, opt | String, opt | MATCH |
| review_cadence | string, opt | — | MISSING (code has `review_date`) |
| review_date | — | String, opt | IN CODE, NOT IN SPEC |
| owner | — | String, opt | IN CODE, NOT IN SPEC |

Spec enum: privacy, compliance, brand_safety, legal, governance
Code enum: security, hr, compliance, operational

**Status:** Spec `draft → review → active → superseded` / Code `draft → active → under_review → deprecated` — DEVIATION

### 3.18 vendor

**Files:** Spec lines 486-497, Code `src-tauri/src/schema/types/vendor.rs`

| Field | Spec | Code | Status |
|-------|------|------|--------|
| vendor_type | enum(4 values), **req** | Enum(4 values), opt | REQ MISMATCH + different values |
| scope_of_work | string, opt | — | MISSING IN CODE |
| contract_value | number, opt | Number, opt | MATCH |
| owner_id | entity_ref(person), opt | — | MISSING IN CODE |
| quality_bar | string, opt | — | MISSING IN CODE |
| pain_points | array\<string\>, opt | — | MISSING IN CODE |
| contract_end | — | String, opt | IN CODE, NOT IN SPEC |
| primary_contact | — | String, opt | IN CODE, NOT IN SPEC |

Spec enum: agency, tool, contractor, partner
Code enum: agency, saas, contractor, platform

**Status:** Spec `evaluating → active → paused → terminated` / Code `evaluating → active → on_hold → terminated` — `paused` → `on_hold`

### 3.19 playbook

**Files:** Spec lines 501-511, Code `src-tauri/src/schema/types/playbook.rs`

| Field | Spec | Code | Status |
|-------|------|------|--------|
| playbook_type | enum(8 values), **req** | Enum(5 values), opt | ENUM+REQ MISMATCH |
| trigger_conditions | array\<string\>, opt | String, opt | TYPE MISMATCH |
| steps | json, opt | — | MISSING IN CODE |
| owner_id | entity_ref(person), opt | — | MISSING (code has `owner` as String) |
| last_used | date, opt | — | MISSING IN CODE |
| expected_outcome | — | String, opt | IN CODE, NOT IN SPEC |
| owner | — | String, opt | RENAMED from owner_id (entity_ref → String) |

Spec enum: debugging, incident_response, churn_save, onboarding, migration, testing, CRO, lifecycle
Code enum: sales, marketing, ops, cs, dev

**Status:** Spec `draft → active → deprecated` / Code `draft → active → deprecated → archived` — code adds `archived`

### 3.20 brief

**Files:** Spec lines 515-526, Code `src-tauri/src/schema/types/brief.rs`

| Field | Spec | Code | Status |
|-------|------|------|--------|
| initiative_name | string, **req** | — | MISSING IN CODE |
| target_user | string, opt | — | MISSING IN CODE |
| desired_outcome | string, **req** | — | MISSING IN CODE |
| success_metric | string, opt | — | MISSING IN CODE |
| target_date | date, opt | — | MISSING (code has `deadline`) |
| teams_involved | array\<string\>, opt | — | MISSING (code has `stakeholders` as String) |
| brief_type | — | Enum(creative/campaign/product/event), opt | IN CODE, NOT IN SPEC |
| deadline | — | String, opt | IN CODE, NOT IN SPEC |
| stakeholders | — | String, opt | IN CODE, NOT IN SPEC |
| deliverables | — | String, opt | IN CODE, NOT IN SPEC |

**Status:** Spec `draft → approved → in_progress → completed` / Code `draft → review → approved → archived` — DEVIATION

### 3.21 taxonomy

**Files:** Spec lines 530-540, Code `src-tauri/src/schema/types/taxonomy.rs`

| Field | Spec | Code | Status |
|-------|------|------|--------|
| taxonomy_type | enum(4 values), **req** | Enum(3 values), opt | ENUM+REQ MISMATCH |
| primary_channels | array\<string\>, opt | — | MISSING IN CODE |
| reporting_destination | string, opt | — | MISSING IN CODE |
| dimensions | array\<string\>, opt | — | MISSING IN CODE |
| rules | json, opt | — | MISSING IN CODE |
| parent_id | — | EntityRef(taxonomy), opt | IN CODE, NOT IN SPEC |
| level | — | Number, opt | IN CODE, NOT IN SPEC |

Spec enum: naming_convention, utm_governance, classification, information_architecture
Code enum: category, tag, hierarchy

**Status:** Spec `draft → active → superseded` / Code `draft → active → archived` — `superseded` → `archived`

### 3.22 backlog

**Files:** Spec lines 557-566, Code `src-tauri/src/schema/types/backlog.rs`

| Field | Spec | Code | Status |
|-------|------|------|--------|
| item_count | number, opt | — | MISSING IN CODE |
| top_outcomes | array\<string\>, opt | — | MISSING IN CODE |
| immovable_deadlines | json, opt | — | MISSING IN CODE |
| last_triaged | date, opt | — | MISSING IN CODE |
| priority_score | — | Number, opt | IN CODE, NOT IN SPEC |
| effort | — | String, opt | IN CODE, NOT IN SPEC |
| requester | — | String, opt | IN CODE, NOT IN SPEC |
| target_sprint | — | String, opt | IN CODE, NOT IN SPEC |

**Status:** Spec `needs_triage → triaged → stale` / Code `open → triaged → scheduled → closed` — only `triaged` shared

### 3.23 Missing entity type schemas (in spec, not in code)

- **inbox_item** (spec lines 193-202): Fields `source_text` (req), `source_url`, `suggested_type`, `suggested_title`. Status: `unprocessed → triaged → archived`
- **artifact** (spec lines 271-281): Fields `artifact_kind` (req), `uri_or_path` (req), `hash`, `mime`, `parent_entity_id`. Status: null
- **concept** (spec lines 285-293): Fields `definition`, `aliases`, `domain`. Status: null
- **commitment** (spec lines 544-553): Fields `owner_id` (req), `deadline`, `source_context`, `tracking_tool`. Status: `on_track → at_risk → blocked → fulfilled → broken`
- **issue** (spec lines 570-580): Fields `severity` (req), `first_observed`, `affected_area`, `owner_id`, `resolution_notes`. Status: `open → investigating → mitigated → resolved → wont_fix`

---

## 4. Other Tables (§2.2–2.7)

### 4.1 relations table (§2.2)

**DDL: MATCH** — All 8 columns present and correct.

**Discrepancy:** `weight` is `f64` in Rust models (`src-tauri/src/models/relation.rs` line 9, `src-tauri/src/services/graph_builder.rs` line 28) but should be `Option<f64>` per spec (nullable, §2.2.3).

### 4.2 Canonical relation taxonomy (§2.2.1)

Spec defines 22 canonical types across 4 groups:
- **Structural:** part_of, derived_from, depends_on, blocks, duplicate_of
- **Semantic:** implements, mentions, supports, contradicts, tests, decides, evidence_for
- **Operational:** assigned_to, created_in, related_to
- **Domain:** targets, competes_with, measures, funds, enables, mitigates, promotes

**CRITICAL: No validation exists.** The referential validator (`src-tauri/src/validation/referential_validator.rs` lines 32-48) only validates custom relation types (those starting with `custom:`). For non-custom types, any arbitrary string is accepted. Test code even uses `"relates_to"` (typo for `"related_to"`) which passes because there's no validation.

### 4.3 custom_relation_types table (§2.2.2)

**DDL: MATCH.** `proposed_by_run_id` is never set by `approve_custom_type` (`src-tauri/src/services/graph_builder.rs` lines 272-305) — always NULL.

### 4.4 `related_to` audit (§2.2.4)

- Function exists but is NOT auto-called during `rebuild_projection` (spec says "on every projection rebuild")
- Ratio query does NOT exclude `custom:` types (spec's SQL has `WHERE relation_type NOT LIKE 'custom:%'`)

### 4.5 runs table (§2.3)

**MATCH** — No discrepancies at DDL or Rust model level.

### 4.6 artifacts table (§2.4)

**MATCH** — No discrepancies.

### 4.7 operational_context table (§2.5)

**DDL: MATCH.** Context key namespace (spec §2.5, ~30 defined keys) is not validated — any arbitrary key accepted.

### 4.8 claims table (§2.6)

**MATCH** — `evidence_entity_id` correctly required. Promotion implemented (exceeds spec's "not required at MVP").

### 4.9 embeddings table (§2.7)

**DDL: MATCH.** Embedding generation (`src-tauri/src/services/indexer.rs` lines 98-110) omits `body_md` — only uses `title + canonical_fields`. Spec says input should be `title + body_md + canonical_fields`.

### 4.10 dedup_suggestions table (§3.3)

**DDL: MATCH.** Behavioral discrepancies listed in §6 below.

---

## 5. Patch Protocol (§3)

### 5.1 Op types — 6 of 10 missing

| Op Type | Status | Notes |
|---------|--------|-------|
| `create_entity` | IMPLEMENTED | |
| `update_entity` | IMPLEMENTED | |
| `create_relation` | IMPLEMENTED | |
| `create_claim` | IMPLEMENTED | |
| `delete_relation` | **MISSING** | No variant in PatchOp enum |
| `attach_artifact` | **MISSING** | No variant or handler |
| `merge_entities` | **MISSING** | No variant, no confirmation gate |
| `update_context` | **MISSING** | Context goes through ContextManager directly, not patch system |
| `promote_claim` | **MISSING** | Spec says "not required at MVP" |
| `propose_relation_type` | **MISSING** | No proposal workflow; code has direct `approve_custom_relation_type` |

### 5.2 PatchOp structure gaps

- `PatchSet.run_id`: `Option<String>` in code, required in spec (`src-tauri/src/models/patch.rs` line 64)
- No `op_id` field on PatchOp
- No top-level `reason` field (only exists on `UpdateEntityPayload`)
- No `evidence_entity_ids` field

### 5.3 Optimistic locking

**IMPLEMENTED** — `expected_updated_at` on `UpdateEntityPayload`, checked in `execute_update_entity`.

### 5.4 CRITICAL: Validation pipeline is dead code

The 4-step validation pipeline functions (`validate_create_entity`, `validate_update_entity`, `validate_create_relation`, `validate_create_claim`) exist in `src-tauri/src/validation/mod.rs` (lines 33-127) but are **never called** from the patch application path in `src-tauri/src/patch/apply.rs` (lines 111-174). The `apply_single_op` function dispatches directly to execute functions.

**What actually runs during patch application:**
- Schema version check for `update_entity`
- Optimistic lock check inside `execute_update_entity`

**What does NOT run:**
- Schema validation of canonical_fields
- Status transition validation
- Referential integrity checks for entity_ref fields

### 5.5 Status transition rules

| Rule | Spec | Code | Status |
|------|------|------|--------|
| Forward transitions | Always valid | Always valid | MATCH |
| Backward without reason | Soft constraint (write succeeds, reason logged) | **HARD REJECT** (returns error) | DISCREPANCY |
| Backward with reason | Allowed | Allowed | MATCH |
| Skip transitions | Allowed but logged | Allowed, NOT logged | DISCREPANCY |
| null → first status | Always valid | null → ANY valid status | Broader than spec |
| Same status | No-op | No-op | MATCH |

### 5.6 Status enum value discrepancies (summary)

19 of 22 implemented types have status enum deviations from spec. Only `project` and `experiment` match exactly. See §3 above for per-type details.

3 entity types missing from status validator entirely: `inbox_item`, `commitment`, `issue` (because these types aren't implemented).

---

## 6. Dedup Pipeline (§3.3)

| Detection Method | Spec | Code | Delta |
|------------------|------|------|-------|
| Exact title match | confidence 0.95 | confidence 1.0 | Minor |
| Fuzzy title match | Levenshtein ≤3 OR trigram >0.7, confidence 0.70 | normalized_levenshtein ≥0.8, confidence=score | Different algorithm; no trigram |
| Embedding proximity | cosine >0.92, confidence 0.80, title ≥4 chars guard | cosine ≥0.85, confidence=score, no title guard | Lower threshold, missing guard |

Additional: spec says "short-circuit on high-confidence match" — code runs all 3 stages (no short-circuit).

---

## 7. Template System (§4)

### 7.1 TemplateDefinition — 6 of 10 fields missing

**File:** `src-tauri/src/services/template_runner.rs`, lines 22-28

| Spec Field | Code Status |
|------------|-------------|
| `template_key` | Present (as `key`) |
| `category` | Present (plain String) |
| `version` | Present |
| `prerequisites` | Present (partial — see 7.2) |
| `maturity_tier` | **MISSING** |
| `operating_rules` | **MISSING** |
| `inputs_schema` | **MISSING** |
| `context_selectors` | **MISSING** |
| `prompt_body` | **MISSING** |
| `output_contract` | **MISSING** |

### 7.2 Prerequisites — 2 of 4 fields missing

| Spec Field | Code Status |
|------------|-------------|
| `entity_type` | Present |
| `min_count` | Present |
| `suggested_template` | **MISSING** |
| `reason` | **MISSING** |

Also: spec says prerequisites are "advisory, not blocking" — code blocks execution unless `force=true`.

### 7.3 Template registry — 19 of 98 spec templates implemented

| Category | Spec | Implemented | Templates |
|----------|------|-------------|-----------|
| bootstrap | 1 | 0 | — |
| analytics | 10 | 3 | analytics-metric-tree, analytics-experiment-plan, analytics-anomaly-investigation* |
| content | 10 | 3 | content-case-study-builder, content-creative-brief-builder, content-strategy-pillars-seo |
| development | 16 | 9 | dev-adr-writer, dev-api-design, dev-architecture-review, dev-test-plan, dev-prd-to-techspec, dev-requirements-to-spec, dev-db-schema, dev-migration-plan, dev-security-threat-model |
| marketing | 20 | 3 | mkt-icp-definition, mkt-competitive-intel, mkt-positioning-narrative |
| organizing | 6 | 0 | — |
| org | 5 | 1 | org-decision-log |
| distribution | 8 | 0 | — |
| events | 10 | 0 | — |
| operations | 8 | 0 | — |
| customer_success | 2 | 0 | — |
| finance | 1 | 0 | — |
| legal | 1 | 0 | — |
| cross_functional | 2 | 0 | — |

\* Name mismatch: code uses `analytics-anomaly-investigation`, spec uses `analytics-anomaly-detection-investigation`

**4 templates in code NOT in spec:** `org-project-charter`, `org-project-plan`, `org-meeting-brief`, `org-retrospective`

### 7.4 Maturity tiers

**Completely absent** from codebase. No enum, no field, no queries.

### 7.5 Context selectors (§4.3)

**Completely absent.** No `{{stored.operational_context.*}}` or `{{graph.entities ...}}` resolution engine. Templates receive raw JSON params only.

### 7.6 Machine payload format (§4.4)

Output is a raw `PatchResult` (op indices + IDs). None of the spec's 7 semantic output fields exist:
- `action_items` — MISSING
- `decisions_needed` — MISSING
- `risks` — MISSING
- `assumptions` — MISSING
- `open_questions` — MISSING
- Structured `produced_entities` — MISSING (only op IDs returned)
- Structured `produced_relations` — MISSING (only op IDs returned)

---

## 8. Services & Architecture (§6)

### 8.1 Service mapping

| Spec Service | Code Equivalent | Status |
|-------------|-----------------|--------|
| `store` | `StoreService` | Present |
| `indexer` | `IndexerService` | Present |
| `graph_builder` | `graph_builder.rs` | Present |
| `agent_runner` | `template_runner.rs` | **Renamed** |
| `context_manager` | `ContextManager` | Present |

### 8.2 Services in code not in spec

- `claim_service` (`src-tauri/src/services/claim_service.rs`) — not listed in §6
- `dedup` (`src-tauri/src/services/dedup.rs`) — spec says dedup is part of `store`
- `llm` (`src-tauri/src/services/llm.rs`) — not mentioned anywhere in spec

### 8.3 Category-scoped subgraphs (§5.1)

**Completely unimplemented.** No category filtering anywhere in `graph_builder.rs`. No function accepts a category parameter. No SQL query references the `category` column.

---

## 9. UI Surfaces (§7)

### 9.1 Spec surfaces vs. code

| Spec Surface | Built? | Code Component |
|-------------|--------|---------------|
| Quick Capture | No | — |
| Inbox Triage | No | — |
| Entity View | **Yes** | `EntityManager` |
| Tasks Table | No | — |
| Projects Table | No | — |
| Decision Log View | No | — |
| Graph Explorer | **Yes** | `GraphExplorer` |
| Planner | No | — |
| Template Runner | **Yes** | `TemplateRunner` |
| Operational Context Editor | No | — |
| Workflow Map | **Yes** | `WorkflowMap` |

### 9.2 Built but not in spec

- `SearchBar` / Search tab — full-text search UI
- `DedupDashboard` / Dedup tab — dedup suggestion review
- `ChatPanel` / Chat tab — LLM chat interface

### 9.3 Tauri commands — gaps

**Missing from code (spec implies):**
- `merge_entities` — no command or handler
- `delete_relation` — no command
- `propose_relation_type` — no proposal workflow (code has direct `approve_custom_relation_type`)
- `attach_artifact` — no command
- `create_claim` as direct command — only via `apply_patch_set`

**In code but not in spec:**
- `migrate_entity`, `migrate_all_entities`, `find_stale_entities` — schema migration commands
- `llm_chat`, `llm_complete`, `llm_status` — LLM commands

---

## 10. Priority-Ranked Discrepancy List

### P0 — Critical (functional gaps)

| # | Issue | Location |
|---|-------|----------|
| P0-1 | Validation pipeline is dead code — patches bypass all validation | `src-tauri/src/patch/apply.rs` lines 111-174 |
| P0-2 | 6 patch op types missing (delete_relation, attach_artifact, merge_entities, update_context, promote_claim, propose_relation_type) | `src-tauri/src/models/patch.rs` lines 5-14 |
| P0-3 | 5 entity types missing (inbox_item, artifact, concept, commitment, issue) | `src-tauri/src/schema/types/` |
| P0-4 | No canonical relation type validation — any string accepted | `src-tauri/src/validation/referential_validator.rs` lines 32-48 |
| P0-5 | Context selectors and template resolution engine entirely absent | `src-tauri/src/services/template_runner.rs` |

### P1 — High (schema drift)

| # | Issue | Location |
|---|-------|----------|
| P1-1 | 19/22 entity types have status enum deviations | `src-tauri/src/validation/status_validator.rs` |
| P1-2 | Every entity type has field-level drift (missing fields, type mismatches, required→optional) | `src-tauri/src/schema/types/*.rs` |
| P1-3 | FieldType system missing 5 variants (date, datetime, json, array, entity_ref_array) | `src-tauri/src/schema/field_def.rs` lines 12-18 |
| P1-4 | SchemaVersion only tracks 3 of 22 types | `src-tauri/src/schema/version.rs` lines 24-29 |
| P1-5 | 79 of 98 spec templates unimplemented | `src-tauri/src/services/template_runner.rs` |
| P1-6 | Machine payload format completely different from spec | `src-tauri/src/services/template_runner.rs` lines 49-54 |
| P1-7 | Maturity tiers absent from template system | `src-tauri/src/services/template_runner.rs` |
| P1-8 | 4 templates in code not in spec (org-project-charter, org-project-plan, org-meeting-brief, org-retrospective) | `src-tauri/src/services/template_runner.rs` |

### P2 — Medium (behavioral)

| # | Issue | Location |
|---|-------|----------|
| P2-1 | Backward status transitions hard-rejected (spec says soft constraint) | `src-tauri/src/validation/status_validator.rs` lines 131-148 |
| P2-2 | Dedup uses different algorithms/thresholds than spec | `src-tauri/src/services/dedup.rs` |
| P2-3 | Embedding generation omits `body_md` | `src-tauri/src/services/indexer.rs` lines 98-110 |
| P2-4 | Category-scoped subgraphs unimplemented | `src-tauri/src/services/graph_builder.rs` |
| P2-5 | `related_to` audit not auto-invoked and wrong exclusion filter | `src-tauri/src/services/graph_builder.rs` |
| P2-6 | `weight` non-nullable in Rust model (spec says nullable) | `src-tauri/src/models/relation.rs` line 9 |
| P2-7 | Prerequisite struct missing `suggested_template` and `reason` | `src-tauri/src/services/template_runner.rs` lines 30-34 |
| P2-8 | PatchSet.run_id is Optional (spec says required) | `src-tauri/src/models/patch.rs` line 64 |
| P2-9 | Generated columns not implemented | migrations/ |
| P2-10 | Dedup missing title length guard for embedding proximity | `src-tauri/src/services/dedup.rs` |

### P3 — Low (naming/minor)

| # | Issue | Location |
|---|-------|----------|
| P3-1 | Field renames (is_external→external, website_url→website, expected_attendance→expected_attendees, current_spend→spent) | Various schema type files |
| P3-2 | Service renamed (agent_runner→template_runner) | `src-tauri/src/services/template_runner.rs` |
| P3-3 | Template name mismatch (analytics-anomaly-investigation vs spec's analytics-anomaly-detection-investigation) | `src-tauri/src/services/template_runner.rs` |
| P3-4 | Test code uses `"relates_to"` instead of canonical `"related_to"` | Various test files |
| P3-5 | `provenance_run_id` in entities DDL but not in spec field list | `migrations/001_initial_schema.sql` line 19 |
| P3-6 | Context key namespace not validated | `src-tauri/src/services/context_manager.rs` |
| P3-7 | `proposed_by_run_id` never set in approve_custom_type | `src-tauri/src/services/graph_builder.rs` lines 272-305 |
| P3-8 | 7 spec UI surfaces not built (may be intentional MVP scoping) | `src/components/` |

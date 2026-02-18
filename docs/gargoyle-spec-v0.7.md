# Gargoyle — Spec (v0.7)

This document is the single canonical design reference for Gargoyle. Edits happen here only.

> **v0.7 changelog:** Eight improvements from external architectural review. (1) Schema versioning — `schema_version` integer on registry entries, `_schema_version` column on entities table, forward migration strategy defined (§2.1.4). (2) Relation selection decision tree — operational disambiguation guide for semantically adjacent relation types (§2.2.1.1). (3) Claims require `evidence_entity_id` at MVP — prevents ungrounded triples from accumulating as dead data (§2.6). (4) Template maturity tiers — `maturity_tier` enum (foundational/workflow/advanced/diagnostic) on TemplateDefinition, all 98 templates tagged (§4.1, §4.2). (5) Embedding search abstracted behind indexer service boundary — cosine distance is an indexer method, not inline application code; sqlite-vss is a drop-in replacement (§2.7). (6) Deduplication pipeline — title similarity + embedding proximity checks on `create_entity`, surfaced as suggestions not blocks (§3.3). (7) Scalar-vs-metric tradeoff for ARR/headcount explicitly acknowledged with guidance on when to promote scalars to metric entities (§2.5). (8) Category scoping evolution note — `inferred_categories` flagged as future extension point when cross-domain ambiguity emerges (§5.1).

> **v0.6 changelog:** Six structural improvements integrated. (1) Operational context plurals promoted to entities — `initialize` now seeds the entity graph directly; `operational_context` table reduced to scalar values only. (2) Schema registry fully specified with meta-schema and per-type definitions including required/optional fields, entity references, and status enums. (3) Entity lifecycle state machines defined for high-traffic types. (4) Template prerequisites added to `TemplateDefinition` — template registry becomes a navigable dependency graph. (5) `produces`/`consumes` removed from relation taxonomy — run-to-entity provenance is derived from `runs` table via `provenance_run_id`. (6) Optimistic locking via `expected_updated_at` on `update_entity` ops. Minor fixes: `weight` semantics defined, `confidence` made nullable on relations, embeddings promoted to specified table, `related_to` audit mechanism added to graph_builder.

> **v0.5 changelog:** Project named Gargoyle. Removed all references to upstream template source. Spec is now self-contained.

> **v0.4 changelog:** All eight open architectural decisions resolved. Decisions recorded in §8 and their implications integrated into relevant sections. Added §2.2.2 for custom relation type governance. Added §3.2 for claim promotion path. Updated §5.1, §6, and §2.5 to reflect chosen implementation strategies.

> **v0.3 changelog:** Aligned entity types, relation taxonomy, and template system with an upstream template input reference (98 templates, 14 categories, ~470 parameters). Template categories are now first-class. Entity types expanded to cover all domain objects implied by templates. Operational context formalized as a bootstrap entity cluster. Template inputs define typed `canonical_fields` schemas per entity type.

---

## 1. Core Architecture

### 1.1 Design Philosophy

* Canonical truth lives in relational tables.
* Graph is a derived projection for traversal and model training.
* Agents never mutate canonical state directly.
* All changes occur through explicit patch-sets.
* Every agent run is auditable and reproducible.
* Template categories are the primary organizational axis — they group templates, constrain entity types, and define agent scope boundaries.
* Template input schemas (min/opt parameters) are the authoritative definition of `canonical_fields` structure per entity type.
* The `initialize` template seeds the entity graph directly — plural operational data (projects, metrics, people, commitments, issues) are first-class entities from the moment of bootstrap.

### 1.2 Architectural Commitments (Resolved in v0.4, extended in v0.6 and v0.7)

These decisions constrain the implementation and should not be revisited without a schema revision.

1. **Single polymorphic entity table.** All entity types live in one `entities` table. No typed breakout tables. Performance is addressed through partial indexes and SQLite generated columns, not table separation.
2. **Application-level schema validation only.** `canonical_fields` schemas are enforced by the patch protocol at write time against the schema registry (§2.1.4). No DB-level CHECK constraints or triggers for JSON validation.
3. **Query-time subgraph filtering.** Category-scoped subgraphs are computed at query time via WHERE clauses, not materialized views or shadow tables.
4. **Snapshot-per-run context versioning.** The `operational_context` table stores latest scalar state only. Historical context is reconstructable from `runs.inputs_snapshot`.
5. **Claims are a separate table with a defined promotion path.** Claims start lightweight and grounded — `evidence_entity_id` is required. A `promote_claim` patch op is specified but not required at MVP.
6. **Custom relation types are allowed, namespaced and gated.** New relation types require a `custom:` prefix and a schema revision proposal. See §2.2.2.
7. **No encryption at v1.** SQLCipher is a post-MVP addition. The schema is designed to be migration-compatible with SQLCipher.
8. **Optimistic locking on entity updates.** `update_entity` ops carry an `expected_updated_at` field. Concurrent writes are detected and rejected. See §3.1.
9. **Bootstrap seeds the graph.** The `initialize` template creates entities and relations for all plural operational data. The `operational_context` table holds only scalar values (company name, user timezone, etc.). See §2.5.
10. **Schema versioning.** Entity type schemas carry a `schema_version` integer. Entities carry `_schema_version`. The validator accepts older versions on read and bumps on write. See §2.1.4.
11. **Embedding search is a service boundary.** All similarity queries route through `indexer.search_similar()`. No inline cosine computation. See §2.7.
12. **Deduplication is advisory, not blocking.** Post-write similarity checks generate suggestions, never prevent creation. See §3.3.

---

## 2. Canonical Data Model (Relational Source of Truth)

### 2.1 Table: entities

Fields:

* id (uuid, primary key)
* entity_type (text enum — see §2.1.1)
* category (text enum — see §2.1.2, nullable for category-agnostic types)
* title (text)
* body_md (text)
* status (nullable text — constrained by entity_type, see §2.1.5)
* priority (nullable integer 0–4)
* due_at (nullable datetime)
* created_at (datetime)
* updated_at (datetime)
* source (text: manual | clipboard | web | import | agent | template | bootstrap)
* canonical_fields (json — schema determined by entity_type, see §2.1.3 and §2.1.4)
* _schema_version (integer — version of the entity_type schema this entity was written against, see §2.1.4)
* provenance_run_id (nullable uuid → runs.run_id — the run that created this entity, see §2.3)
* deleted_at (nullable datetime)

#### Performance Strategy (per §1.2 commitment 1)

No typed breakout tables. Instead:

* Partial indexes on `entity_type` for high-volume types: `CREATE INDEX idx_entities_tasks ON entities(due_at, priority) WHERE entity_type = 'task' AND deleted_at IS NULL;`
* SQLite generated columns for frequently-queried JSON fields: `ALTER TABLE entities ADD COLUMN _status_extracted TEXT GENERATED ALWAYS AS (json_extract(canonical_fields, '$.status')) STORED;`
* FTS5 virtual table covering `title` and `body_md`.

Generated columns are an optimization layer — the patch protocol writes to `canonical_fields` and SQLite maintains the extracted columns automatically.

#### 2.1.1 Entity Types (Locked Set)

**Core types** (category-agnostic):

* inbox_item — unprocessed capture
* note — freeform text
* task — actionable work item
* project — container for related work
* decision — recorded decision with rationale and revisit triggers
* session — time-bounded work session
* artifact — file, link, or rendered output (see also §2.4)
* concept — abstract idea or domain term
* person — named individual with role and organizational context (seeded by `initialize`)

**Domain types** (category-scoped):

* spec — technical or product specification (development)
* experiment — hypothesis + design + constraints (analytics, development)
* result — outcome of an experiment or analysis (analytics)
* metric — KPI, conversion rate, or tracked measurement (analytics)
* campaign — marketing or distribution initiative with budget and timeline (marketing, distribution)
* audience — ICP definition, segment, or persona (marketing, distribution)
* competitor — competitive entity with positioning data (marketing)
* channel — distribution or communication channel (distribution, content)
* event — conference, meetup, workshop, or launch event (events)
* budget — financial plan or allocation (finance, operations)
* policy — legal, compliance, or governance document (legal, operations)
* vendor — agency, tool, or service provider (operations)
* playbook — repeatable process or response plan (customer_success, operations)
* brief — cross-functional initiative brief (cross_functional)
* taxonomy — naming convention, UTM governance, or classification scheme (organizing)
* commitment — promise or obligation with owner and deadline (org)
* backlog — prioritized list of work items (org)
* issue — known problem or risk with severity and status (operations)

#### 2.1.2 Template Categories (Locked Enum)

These map 1:1 to the Gargoyle template category structure:

| Category Key | Label | Template Count |
|---|---|---|
| bootstrap | Bootstrap | 1 |
| analytics | Analytics | 10 |
| content | Content | 10 |
| development | Development | 16 |
| distribution | Distribution | 8 |
| events | Events | 10 |
| marketing | Marketing | 20 |
| operations | Operations | 8 |
| customer_success | Customer Success | 2 |
| finance | Finance | 1 |
| legal | Legal | 1 |
| cross_functional | Cross-Functional | 2 |
| organizing | Organizing | 6 |
| org | Org | 5 |

No new categories without schema revision.

#### 2.1.3 Canonical Fields by Entity Type

`canonical_fields` is a JSON column whose schema is determined by the entity's `entity_type`. The schema for each type is defined in the schema registry (§2.1.4).

Validation is application-level only (per §1.2 commitment 2). The patch protocol validates `canonical_fields` against the schema registry before writing. Invalid payloads are rejected with a structured error including the failing field path, expected type, and actual value.

#### 2.1.4 Schema Registry

The schema registry defines the `canonical_fields` shape for every entity type. Each entry specifies required fields, optional fields, entity reference fields (typed foreign keys into the graph), and the valid status enum (§2.1.5).

**Meta-schema:** Every entity type schema conforms to this structure:

```
EntityTypeSchema:
{
  entity_type: string,
  schema_version: integer,         // monotonically increasing, starts at 1
  fields: [FieldDef],
  status_enum: [string] | null    // null means status is not applicable
}

FieldDef:
{
  key: string,                     // JSON path within canonical_fields
  type: "string" | "number" | "boolean" | "date" | "datetime"
       | "json" | "array" | "entity_ref" | "entity_ref_array",
  required: boolean,
  description: string,
  default?: any,                   // default value if omitted
  ref_entity_type?: string,        // for entity_ref / entity_ref_array: expected target type
  enum_values?: [string],          // for string fields with constrained values
  items_type?: string,             // for array fields: type of array elements
  added_in_version?: integer       // schema version where this field was introduced (omit for v1 fields)
}
```

Fields with `type: "entity_ref"` store a uuid that references another entity's `id`. Fields with `type: "entity_ref_array"` store an array of uuids. The patch protocol validates that referenced entities exist and are of the expected `ref_entity_type`.

**Schema versioning and evolution:**

Every entity type schema has a `schema_version` integer, starting at 1. When a schema changes (field added, field type changed, field made required), the `schema_version` is incremented. All entities carry a `_schema_version` column recording which version of their type's schema they were written against.

**Migration rules:**

* **Adding an optional field:** Increment `schema_version`. Existing entities at older versions are valid — the new field is simply absent. No backfill required. The field's `added_in_version` records when it appeared.
* **Adding a required field:** Increment `schema_version`. Requires a backfill migration to populate the new field on existing entities. Backfill writes use a synthetic run with `template_key: "schema_migration"`.
* **Removing a field:** Increment `schema_version`. Existing entities retain the field in their JSON — it is ignored by the validator for entities at newer versions.
* **Changing a field type:** Increment `schema_version`. Requires a backfill migration to transform existing values. The validator accepts both old and new types for a transition period (one version), after which only the new type is accepted.

**Validator behavior:** When validating `canonical_fields`, the patch protocol checks the entity's `_schema_version` against the current registry version. If the entity's version is older, the validator applies the schema rules for the entity's version, not the current version. On `update_entity`, the entity's `_schema_version` is bumped to the current registry version and the full current schema is validated.

**Registry entries:**

---

**inbox_item**

| Key | Type | Req | Description |
|---|---|---|---|
| source_text | string | yes | Raw captured text |
| source_url | string | no | URL if captured from web |
| suggested_type | string | no | Agent-suggested entity type for triage |
| suggested_title | string | no | Agent-suggested title |

Status enum: `unprocessed` → `triaged` → `archived`

---

**note**

| Key | Type | Req | Description |
|---|---|---|---|
| tags | array\<string\> | no | Freeform tags |
| context_session_id | entity_ref(session) | no | Session this note was taken in |

Status enum: `null` (notes have no lifecycle)

---

**task**

| Key | Type | Req | Description |
|---|---|---|---|
| assignee_id | entity_ref(person) | no | Assigned person |
| project_id | entity_ref(project) | no | Parent project |
| effort_estimate | string | no | T-shirt size or hours |
| acceptance_criteria | string | no | Definition of done |
| blocked_by_ids | entity_ref_array(task) | no | Blocking tasks |

Status enum: `open` → `in_progress` → `blocked` → `done` → `cancelled`

---

**project**

| Key | Type | Req | Description |
|---|---|---|---|
| owner_id | entity_ref(person) | no | Project owner |
| deadline | date | no | Target completion |
| blockers | array\<string\> | no | Current blockers (freeform) |
| success_metric | string | no | How success is measured |

Status enum: `planning` → `active` → `paused` → `completed` → `archived`

---

**decision**

| Key | Type | Req | Description |
|---|---|---|---|
| owner_id | entity_ref(person) | yes | Who decided |
| decided_at | date | yes | When decided |
| rationale | string | yes | Why this was chosen |
| options_considered | array\<string\> | no | Alternatives evaluated |
| constraints | string | no | Constraints that influenced the decision |
| revisit_triggers | array\<string\> | yes | What would cause reconsideration |

Status enum: `pending` → `decided` → `revisited` → `superseded`

---

**session**

| Key | Type | Req | Description |
|---|---|---|---|
| started_at | datetime | yes | Session start |
| ended_at | datetime | no | Session end |
| focus_entity_ids | entity_ref_array | no | What this session focused on |

Status enum: `active` → `ended`

---

**artifact**

| Key | Type | Req | Description |
|---|---|---|---|
| artifact_kind | string | yes | enum: attachment, link, export, rendered_doc |
| uri_or_path | string | yes | Location |
| hash | string | no | Content hash |
| mime | string | no | MIME type |
| parent_entity_id | entity_ref | no | Entity this artifact belongs to |

Status enum: `null` (artifacts have no lifecycle)

---

**concept**

| Key | Type | Req | Description |
|---|---|---|---|
| definition | string | no | What this concept means |
| aliases | array\<string\> | no | Alternative names |
| domain | string | no | Knowledge domain |

Status enum: `null`

---

**person**

| Key | Type | Req | Description |
|---|---|---|---|
| role | string | yes | Job title or role |
| department | string | no | Department |
| reports_to_id | entity_ref(person) | no | Manager |
| tenure_months | number | no | Months at company |
| flight_risk | string | no | enum: low, medium, high |
| email | string | no | Contact email |
| is_external | boolean | no | Whether outside the organization (default: false) |

Status enum: `active` → `departed`

---

**spec**

| Key | Type | Req | Description |
|---|---|---|---|
| spec_type | string | yes | enum: technical, product, api, architecture, db_schema, security, test_plan, cicd, migration, observability, performance |
| owner_id | entity_ref(person) | no | Spec owner |
| target_system | string | no | System or service this spec covers |
| constraints | json | no | Relevant constraints |
| related_doc_urls | array\<string\> | no | Links to external docs |

Status enum: `draft` → `review` → `approved` → `superseded`

---

**experiment**

| Key | Type | Req | Description |
|---|---|---|---|
| hypothesis | string | yes | What you expect to happen |
| funnel_position | string | no | Where in funnel: activation, pricing, LP, retention |
| primary_metric | string | yes | Main metric being measured |
| baseline_value | number | no | Current metric value |
| surface | string | no | What is being tested (page, flow, feature) |
| practical_constraints | json | no | Eng time, traffic, tools, timeline |
| secondary_metrics | array\<string\> | no | Guardrail metrics |
| segment_breakdowns | array\<string\> | no | How to slice results |
| traffic_volume | string | no | Expected traffic |

Status enum: `draft` → `running` → `concluded` → `archived`

---

**result**

| Key | Type | Req | Description |
|---|---|---|---|
| source_experiment_id | entity_ref(experiment) | no | Experiment that produced this result |
| outcome | string | yes | What happened |
| data_summary | json | no | Key numbers and findings |
| confidence_level | string | no | enum: high, medium, low |
| next_steps | array\<string\> | no | Recommended actions |

Status enum: `preliminary` → `final` → `invalidated`

---

**metric**

| Key | Type | Req | Description |
|---|---|---|---|
| current_value | number | no | Latest value |
| target_value | number | no | Goal value |
| trend | string | no | enum: up, down, flat |
| last_updated | date | no | When last measured |
| business_objective | string | no | What objective this metric serves |
| data_source | string | no | Where data comes from |
| segment_breakdowns | array\<string\> | no | Available slicing dimensions |
| alert_thresholds | json | no | When to alert |
| unit | string | no | Unit of measurement (%, $, count) |

Status enum: `active` → `paused` → `deprecated` → `archived`

---

**campaign**

| Key | Type | Req | Description |
|---|---|---|---|
| campaign_window | json | no | { start: date, end: date } |
| primary_goal | string | yes | enum: pipeline, signups, revenue, activation, retention, awareness |
| target_audience_id | entity_ref(audience) | no | Target audience entity |
| budget_range | json | no | { min, max, currency } |
| channels | array\<string\> | no | Distribution channels |
| offer | string | no | What's being promoted |
| success_definition | string | no | How success is measured |
| cta | string | no | Primary call to action |

Status enum: `planning` → `live` → `paused` → `completed` → `cancelled`

---

**audience**

| Key | Type | Req | Description |
|---|---|---|---|
| icp_description | string | yes | Ideal customer profile description |
| industry | string | no | Target industry |
| company_size | string | no | Target company size range |
| roles | array\<string\> | no | Target buyer roles |
| pains | array\<string\> | no | Top pain points |
| desired_outcomes | array\<string\> | no | What they want |
| sales_motion | string | no | enum: self_serve, sales_led, hybrid |
| pricing_model | string | no | How they're charged |
| churn_reasons | array\<string\> | no | Why they leave |

Status enum: `draft` → `validated` → `deprecated`

---

**competitor**

| Key | Type | Req | Description |
|---|---|---|---|
| positioning | string | yes | Their market position |
| website_url | string | no | Competitor URL |
| strengths | array\<string\> | no | Where they win |
| weaknesses | array\<string\> | no | Where they lose |
| pricing_info | string | no | Known pricing |
| win_loss_notes | string | no | Sales analysis |

Status enum: `active` → `acquired` → `defunct` → `irrelevant`

---

**channel**

| Key | Type | Req | Description |
|---|---|---|---|
| channel_type | string | yes | enum: paid_search, paid_social, organic_social, email, seo, partnerships, pr, community, events, direct |
| platforms | array\<string\> | no | Specific platforms (Google, LinkedIn, Meta) |
| cac_estimate | number | no | Estimated cost per acquisition |
| performance_data | json | no | Historical performance |
| capacity_constraints | string | no | Bandwidth limits |

Status enum: `active` → `paused` → `retired`

---

**event**

| Key | Type | Req | Description |
|---|---|---|---|
| event_type | string | yes | enum: conference, meetup, workshop, launch, webinar |
| date_window | json | no | { start: date, end: date } |
| city | string | no | Location |
| expected_attendance | number | no | Headcount |
| budget_id | entity_ref(budget) | no | Associated budget |
| venue | string | no | Venue name |
| format | string | no | enum: in_person, virtual, hybrid |

Status enum: `concept` → `planning` → `confirmed` → `completed` → `cancelled`

---

**budget**

| Key | Type | Req | Description |
|---|---|---|---|
| total_amount | number | yes | Budget amount |
| currency | string | no | Currency code (default: USD) |
| time_horizon | json | no | { start: date, end: date } |
| expense_categories | json | no | Breakdown by category |
| current_spend | number | no | Amount spent so far |
| runway_months | number | no | Months of runway |

Status enum: `draft` → `approved` → `active` → `exhausted` → `closed`

---

**policy**

| Key | Type | Req | Description |
|---|---|---|---|
| policy_type | string | yes | enum: privacy, compliance, brand_safety, legal, governance |
| regions | array\<string\> | no | Geographic scope |
| review_stakeholders | array\<string\> | no | Who must review |
| effective_date | date | no | When it takes effect |
| review_cadence | string | no | How often to review |

Status enum: `draft` → `review` → `active` → `superseded`

---

**vendor**

| Key | Type | Req | Description |
|---|---|---|---|
| vendor_type | string | yes | enum: agency, tool, contractor, partner |
| scope_of_work | string | no | What they do |
| contract_value | number | no | Annual or project cost |
| owner_id | entity_ref(person) | no | Internal DRI |
| quality_bar | string | no | What good looks like |
| pain_points | array\<string\> | no | Current issues |

Status enum: `evaluating` → `active` → `paused` → `terminated`

---

**playbook**

| Key | Type | Req | Description |
|---|---|---|---|
| playbook_type | string | yes | enum: debugging, incident_response, churn_save, onboarding, migration, testing, CRO, lifecycle |
| trigger_conditions | array\<string\> | no | When to activate this playbook |
| steps | json | no | Ordered procedure steps |
| owner_id | entity_ref(person) | no | Who maintains it |
| last_used | date | no | When last executed |

Status enum: `draft` → `active` → `deprecated`

---

**brief**

| Key | Type | Req | Description |
|---|---|---|---|
| initiative_name | string | yes | What this brief is for |
| target_user | string | no | End user definition |
| desired_outcome | string | yes | Measurable goal |
| success_metric | string | no | How to measure |
| target_date | date | no | Launch window |
| teams_involved | array\<string\> | no | Cross-functional parties |

Status enum: `draft` → `approved` → `in_progress` → `completed`

---

**taxonomy**

| Key | Type | Req | Description |
|---|---|---|---|
| taxonomy_type | string | yes | enum: naming_convention, utm_governance, classification, information_architecture |
| primary_channels | array\<string\> | no | Channels covered |
| reporting_destination | string | no | Where data flows |
| dimensions | array\<string\> | no | Slicing dimensions |
| rules | json | no | Naming/classification rules |

Status enum: `draft` → `active` → `superseded`

---

**commitment**

| Key | Type | Req | Description |
|---|---|---|---|
| owner_id | entity_ref(person) | yes | Who made the commitment |
| deadline | date | no | When it's due |
| source_context | string | no | Where commitment was made (meeting, email, etc.) |
| tracking_tool | string | no | Asana, Jira, Notion, Sheet |

Status enum: `on_track` → `at_risk` → `blocked` → `fulfilled` → `broken`

---

**backlog**

| Key | Type | Req | Description |
|---|---|---|---|
| item_count | number | no | Number of items |
| top_outcomes | array\<string\> | no | Strategic priorities for triage |
| immovable_deadlines | json | no | Fixed date constraints |
| last_triaged | date | no | When last reviewed |

Status enum: `needs_triage` → `triaged` → `stale`

---

**issue**

| Key | Type | Req | Description |
|---|---|---|---|
| severity | string | yes | enum: critical, high, medium, low |
| first_observed | date | no | When first noticed |
| affected_area | string | no | System, team, or process affected |
| owner_id | entity_ref(person) | no | Who is responsible for resolution |
| resolution_notes | string | no | How it was resolved |

Status enum: `open` → `investigating` → `mitigated` → `resolved` → `wont_fix`

---

#### 2.1.5 Entity Lifecycle and Status Enums

Every entity type with a non-null status enum has a defined state machine. The status enums are listed in each schema registry entry above (§2.1.4).

**Enforcement:** The patch protocol validates status transitions at write time. The rules are:

* **Forward transitions** (moving rightward in the enum as listed) are always valid.
* **Backward transitions** (moving leftward) require a `reason` in the patch op. This is a soft constraint — the write succeeds but the reason is logged for audit.
* **Skip transitions** (jumping over intermediate states) are allowed but logged.
* `null` → first status is always valid (initial assignment).
* Any status → same status is a no-op (idempotent).

The validation pipeline (§3.2) enforces these rules. Status values not in the enum for the entity's type are rejected.

### 2.2 Table: relations

Fields:

* id (uuid)
* from_id (uuid → entities.id)
* to_id (uuid → entities.id)
* relation_type (text — canonical or custom:prefixed, see §2.2.1 and §2.2.2)
* weight (float default 1.0, nullable — see §2.2.3)
* confidence (float 0–1, nullable — see §2.2.3)
* provenance_run_id (uuid → runs.run_id, nullable)
* created_at (datetime)

#### 2.2.1 Canonical Relation Taxonomy (Locked Set)

**Structural relations:**

* part_of — child → parent containment
* derived_from — output → source lineage
* depends_on — item requires another to proceed
* blocks — item prevents another from proceeding
* duplicate_of — entity is a duplicate of another

**Semantic relations:**

* implements — entity realizes a spec, decision, or plan
* mentions — entity references another
* supports — entity provides evidence or justification for another
* contradicts — entity conflicts with another
* tests — experiment or test plan → subject under test
* decides — decision → the question or options it resolves
* evidence_for — result or data → claim or decision

**Operational relations:**

* assigned_to — task or commitment → person
* created_in — entity → session it was created during
* related_to — weak/untyped association (escape hatch, use sparingly — see §2.2.4)

**Domain relations:**

* targets — campaign, content, or distribution → audience or segment
* competes_with — product or positioning → competitor
* measures — metric or dashboard → the thing being measured (project, campaign, funnel stage)
* funds — budget → campaign, event, or initiative
* enables — tool, vendor, or channel → the workflow it supports
* mitigates — playbook, policy, or decision → risk or issue
* promotes — content, ad, or event → product, feature, or offer

**Note on run provenance:** Previous versions included `produces` and `consumes` relations for template-run-to-entity lineage. These have been removed because runs are not entities and cannot participate in the `relations` table. Run-to-entity provenance is tracked through two mechanisms: (a) `provenance_run_id` on both `entities` and `relations` records, and (b) `runs.inputs_snapshot` and `runs.patch_set` which capture full input/output lineage. To query "what entities did run X create?", filter entities by `provenance_run_id`. To query "what inputs did run X consume?", read `runs.inputs_snapshot`.

#### 2.2.1.1 Relation Selection Decision Tree

Several relation types have overlapping semantics. This decision tree disambiguates the most commonly confused pairs. Agents and template authors should follow this guide when selecting relation types.

**`supports` vs `evidence_for`:**

* Use `evidence_for` when the source is empirical data (a result, metric snapshot, or data artifact) and the target is a claim, decision, or hypothesis. The source provides factual grounding.
* Use `supports` when the source provides logical, argumentative, or contextual justification — not raw data. A spec that justifies a decision's rationale uses `supports`. A dashboard that shows the numbers backing a decision uses `evidence_for`.
* Rule of thumb: if you could put the source in a "Data" appendix, use `evidence_for`. If you'd put it in a "Rationale" section, use `supports`.

**`supports` vs `implements`:**

* Use `implements` when the source entity is a concrete realization of the target — code that implements a spec, a campaign that implements a strategy, an artifact that implements a brief.
* Use `supports` when the source strengthens or justifies the target without being a direct realization of it. A metric that shows a spec's approach is working `supports` the spec; it does not `implement` it.
* Rule of thumb: `implements` implies "this was built to satisfy that." `supports` implies "this makes that more credible."

**`enables` vs `implements`:**

* Use `enables` when the source is infrastructure, tooling, or a capability that makes the target possible but is not itself the target. A CI/CD pipeline `enables` a deployment workflow. A vendor `enables` a campaign.
* Use `implements` when the source directly fulfills the target's specification. A code artifact `implements` a spec.
* Rule of thumb: `enables` is indirect and reusable. `implements` is direct and specific.

**`enables` vs `promotes`:**

* Use `promotes` when the source actively drives awareness, adoption, or usage of the target. Content `promotes` a product. An event `promotes` a brand.
* Use `enables` when the source provides capability without actively driving anything. A tool `enables` a workflow. A channel `enables` distribution.
* Rule of thumb: `promotes` has intent to increase visibility. `enables` has no such intent.

**`depends_on` vs `blocks`:**

* These are not synonyms. `depends_on` is a structural prerequisite: A `depends_on` B means A cannot be completed until B is done. `blocks` is a current-state signal: A `blocks` B means A is actively preventing B from proceeding right now.
* A task can `depend_on` another task without being blocked (the dependency is satisfied). A task can be `blocked` by an issue that is not a formal dependency.
* Rule of thumb: `depends_on` is defined at planning time and is stable. `blocks` is asserted at runtime and is transient.

**`measures` vs `evidence_for`:**

* Use `measures` when a metric entity is the ongoing measurement instrument for another entity (a project, campaign, or funnel stage). This is a structural assignment: "MRR `measures` revenue growth."
* Use `evidence_for` when a specific result or data point supports a specific claim or decision. This is a one-time evidentiary link.
* Rule of thumb: `measures` is a standing relationship. `evidence_for` is a point-in-time citation.

**`mitigates` vs `supports`:**

* Use `mitigates` when the source is a response to a risk, issue, or threat. A playbook `mitigates` an incident risk. A policy `mitigates` a compliance gap.
* Use `supports` when the source strengthens a positive assertion, not when it defends against a negative one.
* Rule of thumb: `mitigates` reduces downside. `supports` increases upside.

#### 2.2.2 Custom Relation Types (Gated Extension)

Users and agents may propose new relation types beyond the canonical set. Custom types are governed by the following rules:

**Naming:** All custom relation types must use the `custom:` prefix. Examples: `custom:sponsors`, `custom:mentors`, `custom:escalates_to`.

**Proposal process:**

1. A custom type is proposed via a `propose_relation_type` patch op (see §3.1).
2. The proposal includes: type name, description, expected from/to entity types, and justification for why no existing canonical type suffices.
3. The user reviews and approves or rejects.
4. Approved types are persisted in a `custom_relation_types` registry table.

**Registry table: custom_relation_types**

* type_key (text, primary key — must start with `custom:`)
* description (text)
* expected_from_types (json array of entity_type enums, nullable)
* expected_to_types (json array of entity_type enums, nullable)
* proposed_by_run_id (uuid → runs.run_id)
* approved_at (datetime)

**Constraints:**

* Custom types must not duplicate the semantics of any canonical type. The proposal process should surface overlap.
* Custom types are queryable and traversable like canonical types but are excluded from built-in graph projections and template `produced_relation_types` declarations until formally adopted.
* Formal adoption into the canonical set requires a schema revision (version bump of this spec).

#### 2.2.3 Weight and Confidence Semantics

Both `weight` and `confidence` are nullable. Their semantics differ:

**Weight** (float, default 1.0, nullable): A traversal ranking signal. Higher weight means stronger connection. Used by the graph_builder for pathfinding, layout, and recommendation. Weight is manually or agent-assignable and has no absolute scale — it is relative within a query context. Set to `null` for relations where ranking is not meaningful. Typical usage: `part_of` relations are unweighted (null); `supports`/`contradicts` relations are weighted to indicate strength of evidence; `related_to` relations use weight to distinguish strong vs weak associations.

**Confidence** (float 0–1, nullable): A provenance signal indicating how certain the system is that this relation is correct. Only meaningful for agent-created relations. User-created relations should set confidence to `null` (implicit 1.0). Confidence is used by the validation pipeline to flag low-confidence relations for user review, and by the graph projection to optionally filter edges below a threshold.

#### 2.2.4 `related_to` Audit Mechanism

The `related_to` relation type is an intentional escape hatch for associations that don't fit the canonical taxonomy. However, overuse signals missing relation types.

**Monitoring:** The graph_builder service computes the `related_to` ratio on every projection rebuild:

```sql
SELECT
  CAST(SUM(CASE WHEN relation_type = 'related_to' THEN 1 ELSE 0 END) AS FLOAT)
  / COUNT(*) AS related_to_ratio
FROM relations
WHERE relation_type NOT LIKE 'custom:%';
```

**Threshold:** If `related_to_ratio` exceeds 0.20 (20%), the graph_builder emits a warning to the UI. The warning includes a breakdown of `related_to` edges grouped by from/to entity type pairs, which surfaces the most common untyped patterns and suggests custom relation type proposals.

### 2.3 Table: runs

Fields:

* run_id (uuid)
* template_key (text — matches §4 template registry)
* template_version (text — semver)
* template_category (text enum — matches §2.1.2)
* inputs_snapshot (json — full input payload at invocation time, including resolved operational_context values and entity references)
* outputs_snapshot (json — full machine payload returned)
* patch_set (json — the mutations proposed)
* status (text: pending | applied | rejected | partial)
* created_at (datetime)

Every agent invocation inserts a run. The `template_category` field is denormalized from the template registry for fast filtering.

**Context versioning role (per §1.2 commitment 4):** The `inputs_snapshot` field captures the full resolved operational context and all entity references at the time of invocation. This is the sole mechanism for historical context reconstruction. To answer "what was our ARR when we ran this template?", query `runs.inputs_snapshot` for the relevant `run_id`. No separate context versioning table is needed.

**Run-to-entity provenance:** Entities and relations created by a run carry `provenance_run_id` pointing back to the run. To reconstruct what a run produced, query `entities WHERE provenance_run_id = ?` and `relations WHERE provenance_run_id = ?`.

### 2.4 Table: artifacts

Fields:

* artifact_id (uuid)
* entity_id (uuid → entities.id)
* kind (attachment | link | export | rendered_doc)
* uri_or_path (text)
* hash (text)
* mime (text)
* created_at (datetime)

### 2.5 Table: operational_context (Scalar Bootstrap State)

This table stores scalar values produced by the `initialize` template — data with cardinality of exactly one that doesn't warrant its own entity.

**What goes here vs. in the entity graph:**

| Data | Cardinality | Storage |
|---|---|---|
| Company name, industry, stage, HQ, founded, mission | 1 | operational_context |
| User name, role, timezone | 1 | operational_context |
| User preferences (communication style, working hours, notifications) | 1 | operational_context |
| ARR, headcount, runway_months | 1 | operational_context |
| Communication channels, meeting cadences | 1 | operational_context |
| Org chart available (boolean) | 1 | operational_context |
| Key people | N | entities (type: person) + relations |
| Active projects | N | entities (type: project) |
| Current commitments | N | entities (type: commitment) |
| Tracked metrics | N | entities (type: metric) |
| Known issues | N | entities (type: issue) |
| Context gaps | N | entities (type: note, tagged as context_gap) |

**Tradeoff note — ARR, headcount, and runway as scalars:**

ARR, headcount, and runway_months are stored as scalar context values, not as `metric` entities. This is an intentional simplification with known tradeoffs:

* **What you gain:** Simplicity. These values are consumed as read-only context by nearly every template. A scalar lookup is cheaper and simpler than a graph query.
* **What you lose:** You cannot attach relations to ARR (e.g., "ARR `measures` revenue growth"). You cannot track ARR trend semantically — historical values only exist in `runs.inputs_snapshot`. You cannot build a graph neighborhood around ARR.

**When to promote:** If the user needs to track a scalar value over time, attach relations to it, or have agents reason about its trajectory, they should create a `metric` entity for it. The scalar in `operational_context` becomes the latest-snapshot data source for the metric entity. Both can coexist — the scalar for fast template context resolution, the metric entity for graph-integrated tracking. The `initialize` template can be re-run to promote scalars when the user's needs evolve.

**Latest-state only (per §1.2 commitment 4).** Each `context_key` has exactly one row. Updates overwrite the previous value. Historical state is recoverable from `runs.inputs_snapshot`.

Fields:

* context_id (uuid, primary key)
* context_key (text, unique — dot-path into the operational context tree)
* context_value (json)
* updated_at (datetime)
* updated_by_run_id (uuid → runs.run_id, nullable)

**Context key namespace** (scalar values only):

| Key | Type | Description |
|---|---|---|
| user_profile.name | string | User's name |
| user_profile.role | string | User's role |
| user_profile.timezone | string | Timezone |
| user_profile.preferences.communication_style | string | Communication style |
| user_profile.preferences.working_hours | string | Working hours |
| user_profile.preferences.notification_preferences | string | Notification config |
| company_profile.name | string | Company name |
| company_profile.industry | string | Industry |
| company_profile.stage | string | Seed, Series A, etc. |
| company_profile.headcount | number | Headcount |
| company_profile.arr | number | Annual recurring revenue |
| company_profile.runway_months | number | Runway in months |
| company_profile.headquarters | string | HQ location |
| company_profile.founded | string | Founded date |
| company_profile.mission | string | Mission statement |
| organizational_structure.departments | json | Departments list (array of strings) |
| organizational_structure.org_chart_available | boolean | Whether org chart exists |
| communication_patterns.primary_channels | json | Channels (Slack, Email, etc.) |
| communication_patterns.meeting_cadence.one_on_ones | string | 1:1 cadence |
| communication_patterns.meeting_cadence.all_hands | string | All-hands cadence |
| communication_patterns.meeting_cadence.board_meetings | string | Board meeting cadence |
| operational_signals.data_available | boolean | Whether signal data exists |
| operational_signals.data_location | string | Where data lives |
| operational_signals.data_types | json | Types of data available |
| operational_signals.date_range.start | string | Data range start |
| operational_signals.date_range.end | string | Data range end |
| operational_signals.record_count | number | Number of records |
| operational_signals.planted_signals.available | boolean | Whether planted signals exist |
| operational_signals.planted_signals.categories | json | Signal categories |
| operational_signals.planted_signals.total_count | number | Signal count |

Templates reference scalar context via `{{stored.operational_context.<key>}}`. For plural data (people, projects, metrics, etc.), templates use graph macros:

```
{{graph.entities entity_type="person" source="bootstrap"}}
{{graph.entities entity_type="project" status="active"}}
{{graph.entities entity_type="metric"}}
{{graph.entities entity_type="commitment" status="on_track,at_risk"}}
{{graph.entities entity_type="issue" canonical_fields.severity="critical,high"}}
```

#### 2.5.1 Bootstrap Entity Seeding

When `initialize` runs, it produces both `update_context` ops for scalar values and `create_entity` + `create_relation` ops for plural data. Example patch-set fragment from a bootstrap run:

```json
[
  { "op_type": "update_context", "payload": { "key": "company_profile.name", "value": "Acme Corp" } },
  { "op_type": "update_context", "payload": { "key": "company_profile.arr", "value": 2400000 } },
  { "op_type": "create_entity", "payload": {
      "entity_type": "person",
      "title": "Jane Chen",
      "source": "bootstrap",
      "canonical_fields": { "role": "VP Engineering", "department": "Engineering", "flight_risk": "low", "tenure_months": 18 }
  }},
  { "op_type": "create_entity", "payload": {
      "entity_type": "person",
      "title": "Marcus Webb",
      "source": "bootstrap",
      "canonical_fields": { "role": "Head of Marketing", "department": "Marketing", "reports_to_id": null, "flight_risk": "medium" }
  }},
  { "op_type": "create_relation", "payload": {
      "from_ref": "Marcus Webb", "to_ref": "Jane Chen", "relation_type": "part_of"
  }},
  { "op_type": "create_entity", "payload": {
      "entity_type": "project",
      "title": "Q3 Product Launch",
      "status": "active",
      "source": "bootstrap",
      "canonical_fields": { "deadline": "2025-09-30", "blockers": ["design review pending"], "success_metric": "200 enterprise signups" }
  }},
  { "op_type": "create_entity", "payload": {
      "entity_type": "metric",
      "title": "MRR",
      "source": "bootstrap",
      "canonical_fields": { "current_value": 200000, "target_value": 300000, "trend": "up", "data_source": "Stripe" }
  }},
  { "op_type": "create_entity", "payload": {
      "entity_type": "issue",
      "title": "Customer onboarding taking 3x expected time",
      "status": "open",
      "source": "bootstrap",
      "canonical_fields": { "severity": "high", "first_observed": "2025-05-15" }
  }}
]
```

This means the graph is populated from the first interaction. Templates that need "active projects" or "tracked metrics" query the entity graph directly via graph macros instead of parsing a KV store.

### 2.6 Table: claims (Separate, with Promotion Path)

Claims live in their own table as lightweight triples (per §1.2 commitment 5). They are not entities by default but can be promoted to full entity status when warranted.

**Grounding requirement:** Every claim must reference an evidence entity. Ungrounded claims (with no `evidence_entity_id`) are rejected at write time. This prevents the claims table from accumulating disconnected triples that never integrate into the graph. If a claim's evidence is the template run itself rather than a specific entity, the `evidence_entity_id` should reference the primary entity that the run produced, determined by the following priority order: `decision` > `result` > `spec` > `experiment` > first entity in `patch_set`. This is a deterministic tiebreaker — agents must not select arbitrarily.

Fields:

* claim_id (uuid, primary key)
* subject (text)
* predicate (text)
* object (text)
* confidence (float 0–1)
* evidence_entity_id (uuid → entities.id, **required** — the entity that grounds this claim)
* provenance_run_id (uuid → runs.run_id, nullable)
* promoted_to_entity_id (uuid → entities.id, nullable — set when promoted)
* created_at (datetime)

#### 2.6.1 Claim Promotion Path

Promotion is not required at MVP but the mechanism is specified here to prevent schema drift when it is implemented.

**When to promote:** A claim becomes a candidate for promotion when any of the following conditions are met:

* Its confidence exceeds a configurable threshold (default: 0.85).
* It is referenced by a `decision` entity as supporting evidence.
* Another claim `contradicts` it (requiring both to be representable as graph nodes for traversal).
* A user explicitly promotes it.

**Promotion mechanics:**

1. A `promote_claim` patch op is issued (see §3.1).
2. A new entity is created with `entity_type: concept` (or a future `claim` entity type if revisited) and `canonical_fields` containing the subject/predicate/object triple plus confidence.
3. The original claim row's `promoted_to_entity_id` is set to the new entity's id.
4. Any existing `evidence_entity_id` relation is mirrored as an `evidence_for` relation on the new entity.
5. The original claim row is retained as a provenance record — it is never deleted.

**Not at MVP.** The `promote_claim` op is defined in §3.1 but the application need not implement promotion logic until claims become central to decision-making workflows. At MVP, claims are write-only and queryable but not promotable.

### 2.7 Table: embeddings

Embeddings are a specified table, not optional. They are required for semantic search across `canonical_fields` and for agent-powered entity linking.

Fields:

* embedding_id (uuid, primary key)
* entity_id (uuid → entities.id)
* model (text — model identifier, e.g. "text-embedding-3-small")
* vector (blob — serialized float array)
* dimensions (integer — vector length)
* created_at (datetime)

**Indexing policy:** The indexer service generates embeddings for every entity on create and update. The embedding input is a concatenation of `title`, `body_md`, and a flattened text representation of `canonical_fields`. Multiple embeddings per entity are allowed (different models).

**Semantic search:** All embedding queries are methods on the indexer service — not inline application code. The indexer exposes a `search_similar(query_text, limit, threshold?) → [(entity_id, score)]` method that handles embedding generation for the query, similarity computation, and result ranking internally. This abstraction means the underlying implementation (currently brute-force cosine distance in application code) can be replaced with sqlite-vss or an external vector store without changing any callers.

Semantic search is exposed to templates via a `{{graph.semantic query="..." limit=10}}` context selector and to users via the UI's search bar. Both route through the indexer service.

**Scale guidance:** Brute-force cosine distance is acceptable up to ~20–30k entities. Beyond that, integrate sqlite-vss as a drop-in replacement behind the same indexer interface. Do not embed similarity logic in UI code or template runners — all paths go through `indexer.search_similar()`.

---

## 3. Patch Protocol (Mutation Layer)

Agents produce a patch-set. Nothing mutates state without applying a patch.

### 3.1 Patch Structure

PatchSet:
```
{
  run_id: uuid,
  ops: [ PatchOp ]
}
```

PatchOp:
```
{
  op_id: uuid,
  op_type: "create_entity" | "update_entity" | "create_relation"
         | "delete_relation" | "create_claim" | "attach_artifact"
         | "merge_entities" | "update_context"
         | "promote_claim" | "propose_relation_type",
  payload: {...},
  reason: string,
  evidence_entity_ids: [uuid]
}
```

Rules:

* `merge_entities` requires explicit user confirmation.
* `update_entity` uses JSON pointer paths for `canonical_fields`. The patch protocol validates the resulting JSON against the schema registry (§2.1.4) before writing. **Optimistic locking (per §1.2 commitment 8):** `update_entity` payloads must include `expected_updated_at` (datetime). If the entity's current `updated_at` does not match, the op is rejected with a conflict error. The caller must re-read the entity and rebase the patch. This prevents silent overwrites from concurrent agent runs or parallel user edits.
* Delete operations soft-delete only.
* `update_context` mutates `operational_context` rows — only the `initialize` template and explicit user edits may issue this op_type.
* `promote_claim` creates an entity from a claim and links them (see §2.6.1). Not required at MVP.
* `propose_relation_type` creates a pending entry in `custom_relation_types` (see §2.2.2). Requires user approval before the type becomes usable.

### 3.2 Validation Pipeline

All patch ops pass through a validation pipeline before application:

1. **Schema validation:** For `create_entity` and `update_entity`, validate `canonical_fields` against the schema registry (§2.1.4) for the target `entity_type`. Reject with structured error including failing field path, expected type, and actual value. For `entity_ref` and `entity_ref_array` fields, verify referenced entities exist and match the expected `ref_entity_type`.
2. **Status validation:** For ops that set or change `status`, validate the transition against the entity type's status enum (§2.1.5). Reject invalid values. Log backward transitions with the op's `reason` field.
3. **Optimistic lock check:** For `update_entity`, compare `payload.expected_updated_at` against the entity's current `updated_at`. Reject with conflict error if mismatched.
4. **Referential integrity:** For `create_relation`, verify both `from_id` and `to_id` exist and are not soft-deleted. Verify `relation_type` is either a canonical type (§2.2.1) or an approved custom type (§2.2.2).
5. **Type compatibility:** For `propose_relation_type`, verify the proposed type does not duplicate canonical semantics (fuzzy match against existing type descriptions).
6. **Confirmation gates:** For `merge_entities` and `propose_relation_type`, halt and prompt for user confirmation before application.

Validation is application-level only (per §1.2 commitment 2). No DB-level triggers or CHECK constraints.

### 3.3 Deduplication Pipeline

Entity deduplication runs as a post-write hook on every `create_entity` op. It does not block entity creation — it generates suggestions that surface in the UI for user review.

**Detection methods (run in order, short-circuit on high-confidence match):**

1. **Exact title match:** If an existing non-deleted entity of the same `entity_type` has an identical `title` (case-insensitive), flag as "likely duplicate" (confidence: 0.95).

2. **Fuzzy title match:** If an existing non-deleted entity of the same `entity_type` has a title within Levenshtein distance ≤ 3 or trigram similarity > 0.7, flag as "possible duplicate" (confidence: 0.70).

3. **Embedding proximity:** If the newly created entity's embedding has cosine similarity > 0.92 with an existing entity of the same `entity_type` (queried via `indexer.search_similar()`), flag as "possible duplicate" (confidence: 0.80). This catches semantically equivalent entities with different titles (e.g., "MRR" and "Monthly Recurring Revenue"). **Minimum title length guard:** Embedding proximity is only checked when the new entity's title is ≥ 4 characters. Short acronym titles ("MRR", "ARR", "LTV") produce embeddings that are too semantically adjacent to disambiguate reliably — these rely on exact and fuzzy title matching only.

**Output:** Duplicate suggestions are stored in a lightweight `dedup_suggestions` table:

* suggestion_id (uuid, primary key)
* new_entity_id (uuid → entities.id)
* existing_entity_id (uuid → entities.id)
* detection_method (text: exact_title | fuzzy_title | embedding_proximity)
* confidence (float 0–1)
* status (text: pending | accepted | dismissed)
* created_at (datetime)

**UI behavior:** Pending suggestions appear as non-blocking notifications in the Entity View and Inbox Triage surfaces. The user can:

* **Accept:** Triggers a `merge_entities` patch op (requires confirmation per §3.1).
* **Dismiss:** Marks the suggestion as dismissed. Dismissed pairs are not re-suggested.

**Performance:** Title matching runs synchronously (fast, string comparison). Embedding proximity runs asynchronously after the embedding is generated (may lag a few seconds behind entity creation). This prevents dedup checks from blocking the write path.

---

## 4. Template System (Bidirectional)

Templates are versioned and categorized.

### 4.1 Template Definition

```
TemplateDefinition:
{
  template_key: string,         // e.g. "analytics-experiment-plan"
  category: CategoryEnum,       // e.g. "analytics"
  version: string,              // semver
  maturity_tier: MaturityTier,  // see below
  operating_rules: string,
  inputs_schema: {
    min: [InputParam],          // required inputs
    opt: [InputParam]           // optional inputs
  },
  prerequisites: [Prerequisite], // entity types that should exist before running (see §4.1.1)
  context_selectors: {...},
  prompt_body: string,
  output_contract: {
    requires_machine_payload: true,
    machine_payload_schema: JSONSchema,
    produced_entity_types: [EntityType],    // what this template creates
    consumed_entity_types: [EntityType],    // what this template reads
    produced_relation_types: [RelationType] // relations it may create (canonical only)
  }
}

MaturityTier (enum):
  "foundational" — Creates core entities that other templates depend on. Run these first.
                   Examples: initialize, mkt-icp-definition, analytics-metric-tree.
  "workflow"     — Standard operational templates for ongoing work. The bulk of the registry.
                   Examples: mkt-paid-ads-plan, dev-prd-to-techspec, event-concept-brief.
  "advanced"     — Templates requiring significant existing graph state or domain expertise.
                   Examples: mkt-messaging-matrix, analytics-cohort-LTV-CAC, dev-security-threat-model.
  "diagnostic"   — Investigative or retrospective templates. Run when something needs analysis.
                   Examples: analytics-anomaly-detection-investigation, event-post-event-report,
                            dev-debugging-playbook.
```

InputParam:
```
{
  name: string,           // human-readable parameter name
  field_key: string,      // machine key for canonical_fields mapping
  description: string,
  type: "text" | "number" | "date" | "json" | "entity_ref" | "enum",
  enum_values?: string[], // only if type == "enum"
  required: boolean       // true = min, false = opt
}
```

Note: Templates may only declare canonical relation types in `produced_relation_types`. Custom relation types (§2.2.2) cannot be produced by templates until formally adopted into the canonical set via schema revision.

#### 4.1.1 Template Prerequisites

Prerequisites declare what entity types should exist in the graph before a template can produce high-quality output. They are advisory, not blocking — a template can always be run without its prerequisites, but the output may be incomplete.

```
Prerequisite:
{
  entity_type: EntityType,          // required entity type
  min_count: number,                // minimum entities of this type (default: 1)
  suggested_template: string,       // template_key that creates this type
  reason: string                    // why this prerequisite matters
}
```

**Runtime behavior:** When a user selects a template in the Template Runner, the UI checks prerequisites against the entity graph:

* **All satisfied:** Template runs normally.
* **Partially satisfied:** UI shows a warning: "This template works best with [entity_type]. You have [N] but [min_count] is recommended. Want to run [suggested_template] first?"
* **Not satisfied:** UI shows a stronger prompt: "This template needs at least one [entity_type]. Run [suggested_template] to create one?"

Prerequisites create an implicit dependency graph across the template registry. The Template Runner can visualize this as a workflow map showing which templates feed into which others.

**Key prerequisite chains** (examples):

* `mkt-paid-ads-plan` requires `audience` → suggested: `mkt-icp-definition`
* `mkt-messaging-matrix` requires `audience` + `competitor` → suggested: `mkt-icp-definition`, `mkt-competitive-intel`
* `analytics-experiment-design-analysis` requires `metric` → suggested: `analytics-metric-tree`
* `distribution-lifecycle-nurture-sequences` requires `audience` → suggested: `mkt-icp-definition`
* `ops-agency-vendor-management` requires `vendor` → suggested: (manual creation or prior ops template)

### 4.2 Template Registry (98 Templates)

Templates follow the naming convention `{category}-{slug}` and are organized by the 14 categories defined in §2.1.2.

#### Bootstrap (1 template)

| Template Key | Min Inputs | Opt Inputs | Produces | Prerequisites | Tier |
|---|---|---|---|---|---|
| initialize | (interactive onboarding) | — | operational_context rows, person, project, metric, commitment, issue entities | — | foundational |

#### Analytics (10 templates)

| Template Key | Min Inputs | Opt Inputs | Produces | Prerequisites | Tier |
|---|---|---|---|---|---|
| analytics-anomaly-detection-investigation | KPI + time window, baseline period, recent initiative changes | channel breakdown, change log, tracking notes | result, decision | metric(1) | diagnostic |
| analytics-attribution-plan-utm-governance | primary conversion path, systems involved, current UTM practices | sales cycle length, existing attribution model, offline conversion capability | taxonomy, spec | — | advanced |
| analytics-cohort-LTV-CAC | revenue model, CAC inputs, retention/churn data | segment cohort data, gross margin assumptions, expansion revenue | result, metric | — | advanced |
| analytics-dashboard-spec-scorecard | KPI list, data sources + BI tool, primary audiences | current dashboards, segmentation needs, alert preferences | spec, metric | metric(1) | workflow |
| analytics-experiment-design-analysis | hypothesis + surface, primary metric + baseline, practical constraints | segment breakdowns, guardrail metrics, past results | experiment, result | metric(1) | workflow |
| analytics-experiment-plan | hypothesis, funnel position, metric(s) | baseline CVR, traffic volume, constraints | experiment | — | workflow |
| analytics-measurement-framework-kpi-tree | business objective, funnel stages, data sources | current KPIs, targets, measurement gaps | spec, metric | — | foundational |
| analytics-metric-tree | business model, primary objective, customer journey | current KPIs, known bottlenecks | metric | — | foundational |
| analytics-pipeline-funnel-velocity | funnel stages + definitions, counts + CVR per stage, time-in-stage | segment breakdown, SLA data, call outcome notes | result, metric | — | diagnostic |
| analytics-weekly-insights-narrative | KPI snapshot, key initiatives, notable changes | hypotheses, qualitative signals | result, note | metric(1) | diagnostic |

#### Content (10 templates)

| Template Key | Min Inputs | Opt Inputs | Produces | Prerequisites | Tier |
|---|---|---|---|---|---|
| content-ad-creative-concepts | audience + CTA, pains + outcomes, proof points | platforms, existing ads, compliance constraints | artifact, brief | audience(1) | workflow |
| content-case-study-builder | customer context + outcome, pre-product challenges, measurable results | call transcript, quote approvals, screenshots | artifact | — | workflow |
| content-copywriting-longform | topic + audience, thesis, CTA | proof points, SEO keyword, competitive content | artifact | audience(1) | workflow |
| content-copywriting-shortform | channel + format, audience segment, CTA | messaging blocks, tone constraints, length constraints | artifact | audience(1) | workflow |
| content-creative-brief-builder | asset type(s), audience + action, key message + proof | brand voice, reference examples, compliance | brief | audience(1) | workflow |
| content-design-system-brand-kit | current brand assets, asset types to standardize, brand personality | liked/disliked examples, accessibility, UI screenshots | spec, taxonomy | — | foundational |
| content-landing-page-copy | audience + offer, problem solved, proof available | competitor pages, brand voice, technical constraints | artifact | audience(1) | workflow |
| content-repurposing-distribution-matrix | source asset, audience + CTA, channels available | channel performance, brand voice, calendar | spec, channel | audience(1) | advanced |
| content-strategy-pillars-seo | ICP(s), business objective, product promise + differentiators | existing content performance, competitor content, objections | spec | audience(1) | foundational |
| content-video-production-plan | video purpose, core message + CTA, distribution channels | available talent, existing footage, brand constraints | spec, brief | — | workflow |

#### Development (16 templates)

| Template Key | Min Inputs | Opt Inputs | Produces | Prerequisites | Tier |
|---|---|---|---|---|---|
| dev-adr-writer | decision statement, context/problem, options considered, constraints | doc/issue links, date + owner | decision | — | workflow |
| dev-api-design | use case, data objects, constraints | existing conventions, example payloads | spec | — | workflow |
| dev-architecture-review | design doc, scale assumptions, latency requirements, compliance, team size | incident history, known tech debt | result, decision | spec(1) | advanced |
| dev-cicd-design | repo type, deployment targets, CI tool, release frequency | compliance, pain points | spec | — | workflow |
| dev-code-review | PR diff, feature intent, constraints | — | result, note | — | workflow |
| dev-code-scaffold | language/framework, feature spec, repo conventions | repo tree, lint/test tooling | artifact | spec(1) | workflow |
| dev-db-schema | entities + relationships, query patterns, data volume | existing schema, performance requirements | spec | — | workflow |
| dev-debugging-playbook | symptom, where it appears, when it started, recent changes | logs, metrics, repro steps | playbook, result | — | diagnostic |
| dev-documentation-writer | doc type, audience, system description | repo links, existing docs, gotchas | artifact | — | workflow |
| dev-migration-plan | what is changing, data volume, downtime tolerance, rollback feasibility | current schema, constraints | spec, playbook | — | advanced |
| dev-observability-plan | system/feature, deployment environments, current tooling | incident patterns, key user journeys | spec | — | advanced |
| dev-performance-plan | system/endpoint, traffic now + 12mo, latency expectations | baseline metrics, cost constraints | spec, experiment | metric(1) | advanced |
| dev-prd-to-techspec | PRD content, architecture constraints | existing APIs, performance expectations, security constraints | spec | — | workflow |
| dev-requirements-to-spec | feature description, target user + use case, deadline | screenshots, system constraints, success metric | spec | — | workflow |
| dev-security-threat-model | system description, data types, auth model, deployment context | architecture diagram, compliance requirements | spec, result | — | advanced |
| dev-test-plan | feature spec, risk tolerance, platforms | known edge cases, past bugs | spec, playbook | spec(1) | workflow |

#### Distribution (8 templates)

| Template Key | Min Inputs | Opt Inputs | Produces | Prerequisites | Tier |
|---|---|---|---|---|---|
| distribution-affiliate-syndication-program | offer + conversion event, commission constraints, tracking method | partner list, brand safety, legal requirements | spec, channel | — | advanced |
| distribution-audience-targeting-retargeting | audience segments, platforms + data sources, primary conversion | past performance, privacy constraints, creative variations | audience, spec | audience(1) | workflow |
| distribution-channel-mix-budget | primary goal + horizon, channels available, total budget | historical CAC/CPA, capacity constraints, strategic bets | budget, channel | — | foundational |
| distribution-CRO-testing-playbook | target surface, baseline metric, primary conversion goal | heatmaps, top objections, engineering constraints | playbook, experiment | metric(1) | diagnostic |
| distribution-email-newsletter-program | audience, value promise, desired outcome | content backlog, founder voice, list metrics | spec, channel | audience(1) | workflow |
| distribution-lifecycle-nurture-sequences | target segment + stage, desired transition, top objections | current sequences, usage milestones, sales handoff rules | playbook, campaign | audience(1) | advanced |
| distribution-paid-search-build | product + ICP, primary conversion, geo/language | keyword list, competitor brands, budget + CPA targets | campaign, channel | audience(1) | workflow |
| distribution-paid-social-build | platform(s) + audience, offer/CTA, landing page | past performance, budget + CPA targets, creative constraints | campaign, channel | audience(1) | workflow |

#### Events (10 templates)

| Template Key | Min Inputs | Opt Inputs | Produces | Prerequisites | Tier |
|---|---|---|---|---|---|
| event-concept-brief | event type, target audience, date window + city, budget range | speakers, sponsors | brief, event | — | workflow |
| event-on-site-ops | expected attendance, venue layout, staffing, registration method | accessibility, sponsor booths | playbook, event | event(1) | workflow |
| event-post-event-report | attendance numbers, budget actuals, feedback, sponsor commitments | pipeline influenced, content metrics | result, event | event(1) | diagnostic |
| event-production-advance | venue details, agenda/session types, recording/streaming | vendor quotes, stage design | spec, event | event(1) | workflow |
| event-program-design | event purpose + audience, total duration, content goals | constraints, speaker candidates | spec, event | — | workflow |
| event-run-of-show | event agenda skeleton, venue hours, staff roles | speaker list, sponsor obligations, capture requirements | spec, event | event(1) | workflow |
| event-speaker-pipeline | event theme + audience, speaker archetypes, speaker budget | existing relationships, priority targets | spec, event | — | workflow |
| event-sponsor-packages | audience size, audience profile, sponsor exclusions, revenue goal | comparable pricing, deliverables | spec, budget | — | advanced |
| event-ticketing-pricing | capacity, budget + revenue goal, WTP hypothesis, value prop | sponsor revenue, competitor pricing | spec, budget | — | advanced |
| event-venue-selection | city + date window, capacity range, format, budget range | accessibility, AV expectations | decision, event | — | workflow |

#### Marketing (20 templates)

| Template Key | Min Inputs | Opt Inputs | Produces | Prerequisites | Tier |
|---|---|---|---|---|---|
| mkt-case-study | customer profile, problem before, implementation, outcome | quotes, timeline, screenshots | artifact | — | workflow |
| mkt-competitive-intel | competitors (3–7), positioning one-liner, where you lose | competitor sites/pricing, win/loss notes | competitor, result | — | foundational |
| mkt-content-strategy | ICP, product category, business goal, supported channels | content performance, founder voice, objections | spec | audience(1) | foundational |
| mkt-editorial-calendar | time horizon, channels, pillars/topics | launch dates, owner availability | spec, campaign | — | workflow |
| mkt-email-nurture-sequence | persona, offer, primary objections, desired end action | onboarding steps, case studies | campaign, playbook | audience(1) | workflow |
| mkt-icp-definition | product description, current customers + 3 examples, pricing, sales motion | churn reasons, win/loss notes, market | audience | — | foundational |
| mkt-landing-page-brief | target segment, offer, traffic source, primary conversion | existing LP, proof assets | brief | audience(1) | workflow |
| mkt-launch-content-pack | what launched, ICP + persona, proof points, CTA | launch date, pricing changes, visual assets | artifact, campaign | audience(1) | workflow |
| mkt-messaging-matrix | ICP + personas, pains + outcomes, competitor alternatives | customer language, pricing | spec, audience | audience(1), competitor(1) | advanced |
| mkt-metrics-dashboard | business model, sales motion, primary goal | analytics stack, existing KPIs | spec, metric | — | foundational |
| mkt-onboarding-activation | product + ICP, activation definition, onboarding steps | drop-off points, onboarding emails | playbook, metric | audience(1) | advanced |
| mkt-paid-ads-plan | goal, budget range, ICP, offer, channels | CAC/LTV, creative assets, CVR benchmarks | campaign, budget | audience(1) | workflow |
| mkt-partnerships-plan | ICP, partner value prop, what you offer | partner list, integration roadmap | spec, vendor | audience(1) | advanced |
| mkt-positioning-narrative | ICP, competitor alternatives, 3 product advantages, pricing level | quotes, case studies, technical differentiators | spec | audience(1), competitor(1) | foundational |
| mkt-pricing-page-copy | pricing model, plan names + prices, ICP + buyer role, objections | competitor pricing, conversion data | artifact | audience(1) | workflow |
| mkt-pr-plan | what's newsworthy, ICP relevance, proof, geography | founder background, media contacts | campaign, spec | audience(1) | workflow |
| mkt-sales-enablement-pack | positioning + pillars, ICP + personas, pricing, objections | existing deck, sales call notes | artifact, playbook | audience(1), competitor(1) | advanced |
| mkt-seo-keyword-plan | product category, ICP, regions/languages, competitors | current site, rankings data | spec, taxonomy | audience(1), competitor(1) | advanced |
| mkt-social-distribution-plan | channels, ICP + tone, time budget, audience size | founder voice, top posts | spec, channel | audience(1) | workflow |
| mkt-website-copy | positioning one-liner, ICP + persona, primary CTA, proof assets | competitor sites, product screenshots | artifact | audience(1) | workflow |

#### Operations (8 templates)

| Template Key | Min Inputs | Opt Inputs | Produces | Prerequisites | Tier |
|---|---|---|---|---|---|
| ops-agency-vendor-management | scope of work, budget + DRI, quality bar | existing contracts, pain points, approval constraints | vendor, spec | — | workflow |
| ops-compliance-privacy-brand-safety | industry constraints, review stakeholders, common claims | existing policies, past incidents, privacy stack | policy, playbook | — | foundational |
| ops-CRM-hygiene-lead-handoff | CRM + pipeline stages, lead routing, SLA expectations | field list, scoring rules, failure cases | playbook, spec | — | workflow |
| ops-localization-regionalization | target regions/languages, core assets, ownership | regional constraints, translation resources, ICP differences | spec | audience(1) | advanced |
| ops-marketing-planning-budgeting | business targets, total budget, channel mix + capacity | historical spend, strategic bets, vendor costs | budget, spec | metric(1) | advanced |
| ops-martech-stack-architecture | current tools, primary use cases, system owners | integration constraints, data quality, future capabilities | spec, vendor | — | foundational |
| ops-project-management-sprint-system | tool used, core workstreams, intake rules | current board, pain points, sprint length | spec, playbook | — | foundational |
| ops-sales-enablement-core-kit | ICP + positioning, product promise + proof, competitors + objections | existing collateral, call notes, pricing constraints | artifact, playbook | audience(1), competitor(1) | advanced |

#### Customer Success (2 templates)

| Template Key | Min Inputs | Opt Inputs | Produces | Prerequisites | Tier |
|---|---|---|---|---|---|
| cs-churn-save-playbook | product + customer type, common churn reasons, available offers | usage metrics, past churn examples | playbook | audience(1) | workflow |
| cs-onboarding-plan | customer type (ICP), success definition, onboarding steps, timeline | common failures, implementation complexity | playbook, spec | audience(1) | foundational |

#### Finance (1 template)

| Template Key | Min Inputs | Opt Inputs | Produces | Prerequisites | Tier |
|---|---|---|---|---|---|
| fin-budget-builder | current cash, monthly revenue, expense categories, headcount + hires, runway goal | unit economics, upcoming one-time costs | budget | — | foundational |

#### Legal (1 template)

| Template Key | Min Inputs | Opt Inputs | Produces | Prerequisites | Tier |
|---|---|---|---|---|---|
| legal-privacy-policy-outline | product description, data collected, third parties, regions served | security practices, cookie usage | policy | — | workflow |

#### Cross-Functional (2 templates)

| Template Key | Min Inputs | Opt Inputs | Produces | Prerequisites | Tier |
|---|---|---|---|---|---|
| integrated-campaign-planning | campaign window, primary goal, target audience, budget range | offer details, prior results, existing assets, sales capacity | campaign, brief | audience(1) | advanced |
| launch-tiering | what's launching, expected impact type, primary audience | impact magnitude, risk level, dependencies, competitive context | decision, spec | — | workflow |

#### Organizing (6 templates)

| Template Key | Min Inputs | Opt Inputs | Produces | Prerequisites | Tier |
|---|---|---|---|---|---|
| organizing-decision-log-commitments | where the log lives, decision types, who can finalize | meeting notes to backfill, task system | spec, playbook | — | foundational |
| organizing-information-architecture | where work lives, core workstreams, who needs access | existing folder tree, pain points, compliance | taxonomy, spec | — | foundational |
| organizing-knowledge-base-sops | where KB lives, top workflows, audience | existing docs, recent incidents | playbook, artifact | — | workflow |
| organizing-meeting-cadence | team structure, current pain points, 30-90 day outcomes | calendar snapshot, existing meetings, async preferences | spec | person(1) | workflow |
| organizing-naming-taxonomy-utm | primary channels, reporting destination, common dimensions | existing UTMs, naming conventions, CRM fields | taxonomy | — | foundational |
| organizing-stakeholder-map | stakeholder groups, per-group marketing needs, friction points | names + roles, comms channels, past escalations | spec, note | person(1) | workflow |

#### Org (9 templates)

| Template Key | Min Inputs | Opt Inputs | Produces | Prerequisites | Tier |
|---|---|---|---|---|---|
| org-backlog-triage | backlog items (10–30), top 3 outcomes for 30 days, immovable deadlines | — | backlog, task | — | diagnostic |
| org-commitment-tracker | list of commitments | current tool | commitment | person(1) | workflow |
| org-cross-functional-brief | initiative name, target user, desired outcome + metric, target date, teams involved | PRD/spec link, prior learnings | brief, project | — | workflow |
| org-decision-log | decision statement, owner, date, rationale, revisit triggers | — | decision | person(1) | workflow |
| org-file-system | where files live, team size + functions, top workflows | current folder screenshot, unfindable docs | taxonomy, spec | — | foundational |
| org-meeting-brief | meeting topic, attendees, agenda items | prior meeting notes, action items | session, note | session(1) | workflow |
| org-project-charter | project name, objective, stakeholders, success criteria | timeline, constraints, risks | project | — | foundational |
| org-project-plan | milestones, deliverables, dependencies | resource allocation, risk register | spec, task | project(1) | workflow |
| org-retrospective | what went well, what to improve, action items | session notes, prior retrospectives | session, note | session(1) | diagnostic |

### 4.3 Context Selectors

Two types supported:

**A) Scalar context pointers** (operational_context table lookups):

```
{{stored.operational_context.user_profile.name}}
{{stored.operational_context.company_profile.arr}}
{{stored.operational_context.communication_patterns.primary_channels}}
```

Resolved at runtime against the `operational_context` table (§2.5). Only for scalar values.

**B) Graph macros** (entity/relation traversal):

```
{{graph.entities entity_type="person" source="bootstrap"}}
{{graph.entities entity_type="metric" status="active"}}
{{graph.entities entity_type="project" status="active,planning"}}
{{graph.entities entity_type="commitment" status="on_track,at_risk"}}
{{graph.entities entity_type="issue" canonical_fields.severity="critical,high"}}
{{graph.neighborhood entity_id="..." depth=2 relation_types=[depends_on, blocks]}}
{{graph.related entity_id="..." relation_type="targets" direction="outbound"}}
{{graph.semantic query="customer onboarding friction" limit=5}}
```

Graph macros are the primary selector for plural data. Graph expansions must include entity IDs in their output.

**Migration from v0.5:** Context selectors like `{{stored.operational_context.metrics_kpis.tracked_metrics[0].name}}` are replaced by `{{graph.entities entity_type="metric"}}`. The old array-indexed syntax is deprecated.

### 4.4 Machine Payload Format (Baseline Contract)

Minimum required fields in every template output:

```json
{
  "template_key": "string",
  "template_category": "string",
  "created_at": "ISO 8601",
  "artifacts": {
    "human_readable": "string (markdown)"
  },
  "produced_entities": [
    {
      "entity_type": "string",
      "title": "string",
      "status": "string (from entity type's status enum, or null)",
      "canonical_fields": {}
    }
  ],
  "produced_relations": [
    {
      "from_ref": "string (entity title or temp ID)",
      "to_ref": "string",
      "relation_type": "string"
    }
  ],
  "action_items": [],
  "decisions_needed": [],
  "risks": [],
  "assumptions": [],
  "open_questions": []
}
```

Templates may extend this contract. The `produced_entities` and `produced_relations` arrays feed directly into the patch-set generator.

---

## 5. Graph Projection Layer

Graph is derived from:

* entities (all types, including bootstrap-seeded)
* relations
* claims (non-promoted claims are not graph nodes — they are queryable but not traversable)

Optional materialized tables:

* graph_nodes
* graph_edges

Projection must be rebuildable deterministically.

### 5.1 Category-Scoped Subgraphs

Implemented as query-time filters (per §1.2 commitment 3). No materialized views.

The graph can be filtered as category-scoped subgraphs for focused traversal:

* **Analytics subgraph:** `WHERE category = 'analytics'` — metrics, experiments, results
* **Marketing subgraph:** `WHERE category = 'marketing'` — campaigns, audiences, competitors
* **Development subgraph:** `WHERE category = 'development'` — specs, experiments, decisions
* **Operations subgraph:** `WHERE category = 'operations'` — vendors, playbooks, policies, budgets, issues
* **Events subgraph:** `WHERE category = 'events'` — events, briefs, budgets

Cross-category edges (e.g., a campaign that `depends_on` a spec) are always present in the full graph. Category-filtered queries include edges where either `from_id` or `to_id` belongs to the filtered category, surfacing cross-category dependencies without requiring materialization.

Category-agnostic core types (task, note, project, decision, person, etc.) appear in every subgraph where they have at least one relation to a category-scoped entity.

**Evolution note — category ambiguity:** The current model assigns each entity a single nullable `category`. This works at current scale but will create ambiguity as the graph grows. A `project` entity may have relations into both the marketing and development subgraphs. A `decision` may span analytics and operations.

When cross-domain queries start returning noise (entities appearing in subgraphs where they don't conceptually belong), the recommended evolution is:

* Add an `inferred_categories` generated column (json array) computed from the entity's relation neighborhood — the set of categories of entities it is directly related to.
* Subgraph filters would then use `inferred_categories` for core types instead of the single `category` column.
* This is a non-breaking addition — the existing `category` column remains the primary assignment for domain-scoped types.

This is flagged as a future extension point, not a current requirement. Do not implement until cross-domain ambiguity is observed in practice.

---

## 6. Desktop Architecture

Recommended stack:

* Tauri
* React
* SQLite (FTS5)
* No encryption at v1 (per §1.2 commitment 7)

**Post-MVP encryption path:** The schema uses standard SQLite types and avoids features incompatible with SQLCipher. When encryption is added, the migration is: create a new SQLCipher database, attach the existing database, copy all tables, and swap. No schema changes needed.

Internal services:

* store — CRUD + patch application + schema validation (§3.2) + optimistic locking + dedup suggestion generation (§3.3)
* indexer — FTS5 updates + embedding generation (§2.7) + `search_similar()` semantic search interface
* graph_builder — projection maintenance + `related_to` ratio monitoring (§2.2.4)
* agent_runner — template execution + run logging + prerequisite checking (§4.1.1)
* context_manager — operational_context scalar read/write (latest-state only)

---

## 7. UI Surfaces (Initial)

1. Quick Capture
2. Inbox Triage
3. Entity View
4. Tasks Table
5. Projects Table
6. Decision Log View
7. Graph Explorer
8. Planner
9. Template Runner (select category → template → prerequisite check → fill min/opt inputs → review patch → apply)
10. Operational Context Editor (view/edit scalar bootstrap state)
11. Workflow Map (prerequisite dependency graph across templates — derived from §4.1.1)

All agent suggestions appear as diffs against canonical state.

---

## 8. Architectural Decisions Log

All decisions are final unless a schema revision is proposed.

| # | Question | Decision | Rationale |
|---|---|---|---|
| 1 | Should claims ever become full entities? | **Option A with path to B.** Claims stay in a separate table. A `promote_claim` patch op is specified but not implemented at MVP. | Keeps entity table clean for MVP. Promotion path prevents schema drift if claims become central later. |
| 2 | Do we allow user-defined relation types? | **Option B.** Custom types allowed with `custom:` prefix and gated approval. | Prevents `related_to` junk drawer while preserving schema discipline. Custom types can't be used in templates until formally adopted. |
| 3 | Separate typed tables for task/project? | **Option A.** Single polymorphic table with partial indexes and generated columns. | SQLite is fast enough at <100k entities. Avoids splitting the patch protocol across tables. |
| 4 | Encryption at v1? | **Option A.** No encryption at v1. Schema is SQLCipher-compatible for post-MVP migration. | Fastest to ship. Full-disk encryption covers the gap. Migration path is documented. |
| 5 | Template → entity type mapping? | **Resolved in v0.3.** Templates declare `produced_entity_types` and `consumed_entity_types`. | — |
| 6 | canonical_fields validation: DB or app level? | **Option A.** Application-level only via patch protocol against schema registry (§2.1.4). | Single-writer app. Patch protocol is the sole mutation path. No DB triggers needed. |
| 7 | Category subgraphs: materialized or query-time? | **Option A.** Query-time WHERE filters. | Sufficient at expected scale. No write amplification or shadow table maintenance. |
| 8 | Operational context versioning? | **Option C.** Latest-state table + snapshot-per-run in `runs.inputs_snapshot`. | Context history is reconstructable from run logs. No separate versioning table needed. |
| 9 | Operational context: KV store or entities? | **Hybrid.** Scalars stay in `operational_context` table. Plural data (people, projects, metrics, commitments, issues) seeded as entities. | Graph is useful from first interaction. Templates use graph macros for plural data. Eliminates array-indexed KV paths. |
| 10 | Schema registry: inline or separate doc? | **Inline.** Full registry in §2.1.4 with meta-schema and per-type field definitions. | Spec is self-contained. No external dependency for implementation. |
| 11 | Concurrent write safety? | **Optimistic locking.** `update_entity` requires `expected_updated_at`. Mismatches rejected. | Cheap insurance against parallel agent runs. Minimal overhead. |
| 12 | Run-to-entity provenance: relations or derived? | **Derived.** `provenance_run_id` on entities/relations + `runs.inputs_snapshot`/`patch_set`. No `produces`/`consumes` relation types. | Runs are not entities. Provenance is queryable without polluting the relation taxonomy. |
| 13 | Embeddings: optional or specified? | **Specified.** `embeddings` is a defined table with indexing policy. Required for semantic search and agent-powered entity linking. | Core to the agent experience. Semantic search across `canonical_fields` requires it. |
| 14 | Schema evolution strategy? | **Schema versioning.** `schema_version` integer on registry entries. `_schema_version` column on entities. Validator accepts older versions on read, bumps on write. | Prevents silent incompatibilities between template versions and entity expectations. Cheap to add now, expensive to retrofit. |
| 15 | Claims grounding at MVP? | **Required.** `evidence_entity_id` is non-null at MVP. Every claim must reference an evidence entity. | Prevents ungrounded triples from accumulating as dead data. Anchors claims to the graph even without promotion. |
| 16 | Template cognitive load? | **Maturity tiers.** Four tiers (foundational/workflow/advanced/diagnostic) on TemplateDefinition. All 98 templates tagged. | Prevents the Template Runner from becoming a decision fatigue machine. Foundational templates surface first for new users. |
| 17 | Embedding search implementation coupling? | **Service boundary.** Cosine distance is a method on the indexer service (`search_similar()`). No inline computation. | sqlite-vss is a drop-in replacement. No caller changes needed when scaling beyond ~30k entities. |
| 18 | Entity deduplication? | **Post-write suggestion pipeline.** Title similarity + embedding proximity on `create_entity`. Non-blocking. Surfaced in UI for user review. | Prevents graph fragmentation. Does not block writes. Dismissed pairs are not re-suggested. |
| 19 | Scalar context vs metric entities for ARR/headcount? | **Scalar with documented promotion path.** Scalars for fast context resolution. Metric entities when tracking/relations needed. Both can coexist. | Intentional tradeoff. Acknowledged in §2.5 so future implementers understand what's lost. |
| 20 | Category scoping for cross-domain entities? | **Current: single nullable category. Future: `inferred_categories` derived column.** Flagged as extension point, not implemented until ambiguity observed. | Avoids premature complexity. Evolution path documented in §5.1. |

---

End of v0.7
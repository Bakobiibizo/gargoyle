# Gargoyle — Template Input Parameter Reference

> Reference of all input parameters for the 23 active enriched templates.
>
> - **min** = minimum required input
> - **opt** = optional input (has default/fallback)
> - **Active templates:** 23

---

## Table of Contents

1. [Analytics (3)](#analytics-3)
2. [Marketing (3)](#marketing-3)
3. [Development (9)](#development-9)
4. [Organizing (5)](#organizing-5)
5. [Content (3)](#content-3)

---

## Analytics (3)

### `analytics-metric-tree`
Category: `analytics` | Generator: `generate_metric_tree_ops`

Creates 5-7 metric entities arranged in a primary-KPI-to-funnel hierarchy with `measures` relations.

| Type | Parameter | Description |
|------|-----------|-------------|
| opt | `business_model` | Business model label used in metric titles and descriptions (default: `"General"`) |
| opt | `primary_objective` | Primary KPI objective used for the root metric (default: `"Growth"`) |
| opt | `customer_journey` | Funnel stage names for the metric tree (default: `"Acquisition -> Activation -> Revenue -> Retention -> Referral"`) |

---

### `analytics-experiment-plan`
Category: `analytics` | Generator: `generate_experiment_plan_ops`

Creates 1 experiment entity linked to an existing metric via `tests` and `measures` relations.

| Type | Parameter | Description |
|------|-----------|-------------|
| opt | `hypothesis` | The hypothesis being tested; truncated to 60 chars for the entity title (default: `"Untitled hypothesis"`) |
| opt | `funnel_position` | Where in the funnel this experiment operates (default: `"unknown"`) |
| min | `metric_id` | ID of an existing metric entity to link to. Required unless `force=true` is set |

---

### `analytics-anomaly-investigation`
Category: `analytics` | Generator: `generate_anomaly_investigation_entity_ops`

Creates 1 result entity with an `evidence_for` relation to an experiment and a claim for the anomaly.

| Type | Parameter | Description |
|------|-----------|-------------|
| min | `experiment_id` | ID of an existing experiment entity to investigate. Required unless `force=true` is set |
| opt | `anomaly_description` | Description of the anomaly under investigation (default: `"Anomaly under investigation"`) |
| opt | `time_window` | Time window for the anomaly analysis, e.g. `"last_30_days"` (default: `"unknown"`) |
| opt | `baseline_period` | Baseline comparison period, e.g. `"previous_quarter"` (default: `"unknown"`) |

---

## Marketing (3)

### `mkt-icp-definition`
Category: `marketing` | Generator: `generate_icp_definition_ops`

Creates 3 person entities (Primary Decision Maker, Champion/End User, Technical Evaluator) linked by `collaborates_with` relations.

| Type | Parameter | Description |
|------|-----------|-------------|
| opt | `product_description` | Description of the product for persona context (default: `"Product"`) |
| opt | `current_customers` | Description of the current customer base (default: `"General audience"`) |
| opt | `market_segment` | Market segment used in persona titles and the `team` canonical field (default: `"General"`) |

---

### `mkt-competitive-intel`
Category: `marketing` | Generator: `generate_competitive_intel_ops`

Creates N note entities (one per competitor) with pairwise `related_to` relations.

| Type | Parameter | Description |
|------|-----------|-------------|
| opt | `market` | Market name used in tags and note context (default: `"General"`) |
| opt | `competitors` | Comma-separated list of competitor names; one note entity is created per entry (default: `"Competitor A, Competitor B"`) |
| opt | `product` | Your product name for the comparison titles (default: `"Our Product"`) |

---

### `mkt-positioning-narrative`
Category: `marketing` | Generator: `generate_positioning_narrative_ops`

Creates 1 decision entity capturing a positioning narrative, linked to an ICP person via `targets` relation.

| Type | Parameter | Description |
|------|-----------|-------------|
| opt | `product` | Product name used in the positioning title and body (default: `"Product"`) |
| opt | `category` | Market category for positioning (default: `"General"`) |
| min | `person_id` | ID of an existing person entity (from ICP template). Required unless `force=true` is set |

---

## Development (9)

### `dev-adr-writer`
Category: `development` | Generator: `generate_adr_writer_ops`

Creates 1 decision entity representing an Architecture Decision Record with structured ADR sections.

| Type | Parameter | Description |
|------|-----------|-------------|
| opt | `title` or `decision_title` | The decision being made; used in the ADR title (default: `"Untitled Decision"`) |
| opt | `context` | Why this decision is needed (default: `"No context provided"`) |
| opt | `options_considered` | Comma- or newline-separated options that were evaluated (default: `"Option A, Option B"`) |
| opt | `chosen_option` | The selected option (default: `"pending"`) |
| opt | `rationale` | Why this option was chosen (default: `"To be determined"`) |
| opt | `consequences` | Expected consequences of the decision (default: `"To be evaluated"`) |
| opt | `status` | ADR status: `proposed`, `accepted`, `deprecated`, or `superseded` (default: `"proposed"`) |

---

### `dev-api-design`
Category: `development` | Generator: `generate_api_design_ops`

Creates 1 spec entity describing an API design with endpoints, auth, versioning, and protocol details.

| Type | Parameter | Description |
|------|-----------|-------------|
| opt | `title` or `api_name` | Name of the API (default: `"Untitled API"`) |
| opt | `description` | Overview description of the API (default: `"API design specification"`) |
| opt | `endpoints` | Comma-separated endpoint paths (default: `"/api/v1/resource"`) |
| opt | `auth_method` | Authentication method, e.g. `"OAuth2"`, `"API Key"`, `"JWT"` (default: `"Bearer Token"`) |
| opt | `versioning` | Versioning strategy, e.g. `"URL path"`, `"header"`, `"query param"` (default: `"URL path"`) |
| opt | `protocol` | API protocol, e.g. `"REST"`, `"GraphQL"`, `"gRPC"` (default: `"REST"`) |
| opt | `rate_limiting` | Rate limiting policy description (default: `"Standard"`) |

---

### `dev-architecture-review`
Category: `development` | Generator: `generate_architecture_review_ops`

Creates 1 note entity with a structured architecture review covering components, concerns, and risk assessment.

| Type | Parameter | Description |
|------|-----------|-------------|
| opt | `title` or `system_name` | Name of the system being reviewed (default: `"System"`) |
| opt | `architecture_type` | Architecture style, e.g. `"microservices"`, `"monolith"`, `"serverless"` (default: `"unknown"`) |
| opt | `components` | Comma-separated list of system components (default: `"Frontend, Backend, Database"`) |
| opt | `concerns` | Key concerns to address in the review (default: `"scalability, reliability, maintainability"`) |
| opt | `review_scope` | Scope of the architecture review (default: `"Full system review"`) |

---

### `dev-test-plan`
Category: `development` | Generator: `generate_test_plan_ops`

Creates 1 spec entity describing a test plan with strategy, coverage targets, environments, and automation approach.

| Type | Parameter | Description |
|------|-----------|-------------|
| opt | `title` or `project_name` | Project or feature name for the test plan (default: `"Project"`) |
| opt | `test_strategy` | Testing strategy, e.g. `"unit + integration + e2e"` (default: `"unit + integration + e2e"`) |
| opt | `coverage_targets` | Code coverage targets, e.g. `"80% line coverage"` (default: `"80% line coverage"`) |
| opt | `test_environments` | Comma-separated test environments (default: `"local, staging, production"`) |
| opt | `risk_areas` | Areas requiring focused testing (default: `"To be identified"`) |
| opt | `automation_approach` | Automation strategy (default: `"CI/CD pipeline with automated test execution"`) |

---

### `dev-prd-to-techspec`
Category: `development` | Generator: `generate_prd_to_techspec_ops`

Creates 1 spec entity translating a product requirements document into a technical specification.

| Type | Parameter | Description |
|------|-----------|-------------|
| opt | `title` or `feature_name` | Feature name for the tech spec (default: `"Feature"`) |
| opt | `prd_summary` | Summary of the product requirements (default: `"Product requirements to be specified"`) |
| opt | `technical_approach` | Proposed technical implementation approach (default: `"To be determined"`) |
| opt | `dependencies` | Comma-separated technical dependencies (default: `"None identified"`) |
| opt | `estimated_effort` | Effort estimate, e.g. `"2 sprints"` (default: `"TBD"`) |
| opt | `acceptance_criteria` | Acceptance criteria for the feature (default: `"To be defined"`) |

---

### `dev-requirements-to-spec`
Category: `development` | Generator: `generate_requirements_to_spec_ops`

Creates 1 spec entity from raw requirements input with stakeholder and constraint information.

| Type | Parameter | Description |
|------|-----------|-------------|
| opt | `title` or `project_name` | Project name for the requirements spec (default: `"Project"`) |
| opt | `requirements` | Raw requirements list (default: `"Requirements to be gathered"`) |
| opt | `stakeholders` | Comma-separated list of stakeholders (default: `"Product, Engineering"`) |
| opt | `constraints` | Known constraints (default: `"None identified"`) |
| opt | `scope` | Project scope definition (default: `"To be defined"`) |
| opt | `priority_level` | Priority: `"critical"`, `"high"`, `"medium"`, or `"low"` -- maps to entity priority 0-3 (default: `"medium"`) |

---

### `dev-db-schema`
Category: `development` | Generator: `generate_db_schema_ops`

Creates 1 spec entity describing a database schema design with tables, relationships, and indexing strategy.

| Type | Parameter | Description |
|------|-----------|-------------|
| opt | `title` or `schema_name` | Name of the database schema (default: `"Database Schema"`) |
| opt | `database_type` | Database engine, e.g. `"PostgreSQL"`, `"SQLite"`, `"MongoDB"` (default: `"PostgreSQL"`) |
| opt | `tables` | Comma-separated table names (default: `"users, orders, products"`) |
| opt | `relationships` | Description of key table relationships (default: `"To be defined"`) |
| opt | `indexing_strategy` | Indexing approach (default: `"Primary keys + foreign keys + common query patterns"`) |
| opt | `migration_approach` | Database migration strategy (default: `"Incremental migrations"`) |

---

### `dev-migration-plan`
Category: `development` | Generator: `generate_migration_plan_ops`

Creates 1 spec entity describing a system migration plan with source/target systems and rollback strategy.

| Type | Parameter | Description |
|------|-----------|-------------|
| opt | `title` or `migration_name` | Name of the migration (default: `"System Migration"`) |
| opt | `source_system` | The system being migrated from (default: `"Legacy system"`) |
| opt | `target_system` | The system being migrated to (default: `"New system"`) |
| opt | `data_scope` | Scope of data included in migration (default: `"All data"`) |
| opt | `rollback_strategy` | Rollback approach (default: `"Blue-green deployment with instant rollback"`) |
| opt | `estimated_downtime` | Expected downtime during migration (default: `"TBD"`) |
| opt | `risk_level` | Risk assessment level (default: `"medium"`) |

---

### `dev-security-threat-model`
Category: `development` | Generator: `generate_security_threat_model_ops`

Creates 1 spec entity for a security threat model with assets, trust boundaries, and attack surface analysis.

| Type | Parameter | Description |
|------|-----------|-------------|
| opt | `title` or `system_name` | Name of the system being modeled (default: `"System"`) |
| opt | `threat_model_type` | Methodology, e.g. `"STRIDE"`, `"DREAD"`, `"PASTA"` (default: `"STRIDE"`) |
| opt | `assets` | Comma-separated critical assets (default: `"User data, API keys, credentials"`) |
| opt | `trust_boundaries` | Trust boundary descriptions (default: `"External/Internal network boundary"`) |
| opt | `attack_surface` | Attack surface description (default: `"Web application, API endpoints"`) |
| opt | `data_classification` | Data sensitivity classification, e.g. `"PII, financial, public"` (default: `"confidential"`) |

---

## Organizing (5)

### `org-project-charter`
Category: `organizing` | Generator: `generate_project_charter_ops`

Creates 1 project entity representing a project charter with objective, timeline, budget, and team.

| Type | Parameter | Description |
|------|-----------|-------------|
| opt | `title` or `project_name` | Project name for the charter (default: `"Untitled Project"`) |
| opt | `objective` | Project objective statement (default: `"Project objective to be defined"`) |
| opt | `success_criteria` | Measurable success criteria (default: `"To be defined"`) |
| opt | `timeline` | Project timeline, e.g. `"Q1 2026"` (default: `"TBD"`) |
| opt | `budget` | Project budget (default: `"TBD"`) |
| opt | `sponsor` | Executive sponsor name (default: `"TBD"`) |
| opt | `team` | Comma-separated team member names (default: `"To be assigned"`) |
| opt | `risks` | Known project risks (default: `"To be identified"`) |

---

### `org-project-plan`
Category: `organizing` | Generator: `generate_project_plan_ops`

Creates 1 spec entity describing a project plan with phases, milestones, resources, and timeline.

| Type | Parameter | Description |
|------|-----------|-------------|
| opt | `title` or `project_name` | Project name (default: `"Project"`) |
| opt | `phases` | Comma-separated project phases (default: `"Planning, Execution, Review, Closeout"`) |
| opt | `milestones` | Comma-separated key milestones (default: `"Kickoff, Mid-point Review, Final Delivery"`) |
| opt | `resources` | Resource allocation description (default: `"To be assigned"`) |
| opt | `dependencies` | Project dependencies (default: `"None identified"`) |
| opt | `start_date` | Project start date (default: `"TBD"`) |
| opt | `end_date` | Project end date (default: `"TBD"`) |

---

### `org-decision-log`
Category: `organizing` | Generator: `generate_decision_log_ops`

Creates 1+ decision entities (one per semicolon-separated entry) forming a project decision log.

| Type | Parameter | Description |
|------|-----------|-------------|
| opt | `title` or `project_name` | Project name for decision log context (default: `"Project"`) |
| opt | `decisions` | Semicolon-separated decision descriptions; one entity is created per entry (default: `"Decision 1; Decision 2"`) |
| opt | `decision_maker` | Name or role of the decision maker, used as `owner_id` (default: `"team-lead"`) |
| opt | `context` | Context for the decisions being recorded (default: `"Project decision context"`) |

---

### `org-meeting-brief`
Category: `organizing` | Generator: `generate_meeting_brief_ops`

Creates 1 session entity for a meeting brief with agenda, participants, and logistics.

| Type | Parameter | Description |
|------|-----------|-------------|
| opt | `title` or `meeting_name` | Meeting name (default: `"Meeting"`) |
| opt | `agenda` | Semicolon-separated agenda items (default: `"Opening; Discussion; Action Items; Close"`) |
| opt | `participants` | Comma-separated participant names (default: `"Team members"`) |
| opt | `meeting_date` | Date of the meeting (default: `"TBD"`) |
| opt | `duration` | Meeting duration, e.g. `"60 min"` (default: `"60 min"`) |
| opt | `objective` | Meeting objective (default: `"To be defined"`) |
| opt | `pre_reads` | Pre-read materials for attendees (default: `"None"`) |

---

### `org-retrospective`
Category: `organizing` | Generator: `generate_retrospective_ops`

Creates 1 session entity + 3 note entities (what went well, improvements, action items) for a sprint retrospective.

| Type | Parameter | Description |
|------|-----------|-------------|
| opt | `title` or `sprint_name` | Sprint name for the retrospective (default: `"Sprint"`) |
| opt | `what_went_well` | Semicolon-separated items that went well (default: `"To be discussed"`) |
| opt | `what_didnt_go_well` | Semicolon-separated items that need improvement (default: `"To be discussed"`) |
| opt | `action_items` | Semicolon-separated action items (default: `"To be identified"`) |
| opt | `participants` | Comma-separated participant names (default: `"Team members"`) |
| opt | `sprint_dates` | Sprint date range (default: `"TBD"`) |

---

## Content (3)

### `content-case-study-builder`
Category: `content` | Generator: `generate_case_study_builder_ops`

Creates 1 note entity for a customer case study with challenge/solution/results structure.

| Type | Parameter | Description |
|------|-----------|-------------|
| opt | `title` or `customer_name` | Customer name for the case study (default: `"Customer"`) |
| opt | `industry` | Customer industry, used in tags and context (default: `"Technology"`) |
| opt | `challenge` | Description of the customer's challenge (default: `"Customer challenge to be described"`) |
| opt | `solution` | Description of the solution provided (default: `"Solution to be described"`) |
| opt | `results` | Quantified results achieved (default: `"Results to be quantified"`) |
| opt | `quote` | Customer testimonial quote; omitted from body if empty (default: `""`) |
| opt | `product` | Product name referenced in the case study (default: `"Our product"`) |

---

### `content-creative-brief-builder`
Category: `content` | Generator: `generate_creative_brief_builder_ops`

Creates 1 spec entity for a creative brief with audience, messaging, tone, and deliverables.

| Type | Parameter | Description |
|------|-----------|-------------|
| opt | `title` or `project_name` | Creative project name (default: `"Creative Project"`) |
| opt | `objective` | Creative objective (default: `"Creative objective to be defined"`) |
| opt | `target_audience` | Target audience description (default: `"Target audience to be defined"`) |
| opt | `key_message` | Core message to communicate (default: `"Key message to be defined"`) |
| opt | `tone` | Tone of voice, e.g. `"professional"`, `"casual"`, `"bold"` (default: `"professional"`) |
| opt | `deliverables` | Comma-separated list of deliverables (default: `"TBD"`) |
| opt | `brand_guidelines` | Brand guideline notes (default: `"Follow standard brand guidelines"`) |
| opt | `deadline` | Project deadline (default: `"TBD"`) |

---

### `content-strategy-pillars-seo`
Category: `content` | Generator: `generate_strategy_pillars_seo_ops`

Creates 1 spec entity (strategy overview) + N note entities (one per content pillar) for SEO-focused content strategy.

| Type | Parameter | Description |
|------|-----------|-------------|
| opt | `title` or `brand_name` | Brand name for the content strategy (default: `"Brand"`) |
| opt | `pillars` | Semicolon-separated content pillars; one note entity is created per entry (default: `"Pillar 1; Pillar 2; Pillar 3"`) |
| opt | `primary_keywords` | Comma-separated primary SEO keywords (default: `"keyword1, keyword2"`) |
| opt | `target_audience` | Target audience for the content (default: `"Target audience"`) |
| opt | `content_goals` | Content marketing goals (default: `"Organic traffic growth, thought leadership"`) |
| opt | `competitor_domains` | Comma-separated competitor domain names for landscape analysis (default: `""`) |

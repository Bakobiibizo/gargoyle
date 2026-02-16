# Gargoyle CLI — Complete Template Input Parameter Reference

> Auto-generated reference of all mappable input parameters across every gargoyle-cli expertise template.
>
> - **min** = minimum required input
> - **opt** = optional input
> - **Total templates:** 98
> - **Total input parameters:** ~470 (approximately 300 minimum + 170 optional)
> - **`{{}}` template variables:** Only `{{stored.operational_context.*}}` exists, produced by `initialize` and consumed by all other templates at runtime.

---

## Table of Contents

1. [Bootstrap (initialize)](#bootstrap)
2. [Analytics (10)](#analytics)
3. [Content (10)](#content)
4. [Development (16)](#development)
5. [Distribution (8)](#distribution)
6. [Events (10)](#events)
7. [Marketing (20)](#marketing)
8. [Operations (8)](#operations)
9. [Customer Success (2)](#customer-success)
10. [Finance (1)](#finance)
11. [Legal (1)](#legal)
12. [Cross-Functional (2)](#cross-functional)
13. [Organizing (6)](#organizing)
14. [Org (5)](#org)

---

## Bootstrap

### `initialize`

The only template that produces `{{stored.operational_context.*}}` values. All other templates consume these at runtime.

| Path | Description |
|------|-------------|
| `{{stored.operational_context.user_profile.name}}` | User's name |
| `{{stored.operational_context.user_profile.role}}` | User's role |
| `{{stored.operational_context.user_profile.preferences.communication_style}}` | Communication style |
| `{{stored.operational_context.user_profile.preferences.working_hours}}` | Working hours |
| `{{stored.operational_context.user_profile.preferences.notification_preferences}}` | Notification preferences |
| `{{stored.operational_context.user_profile.timezone}}` | Timezone |
| `{{stored.operational_context.company_profile.name}}` | Company name |
| `{{stored.operational_context.company_profile.industry}}` | Industry |
| `{{stored.operational_context.company_profile.stage}}` | Stage (Seed, Series A, etc.) |
| `{{stored.operational_context.company_profile.headcount}}` | Headcount |
| `{{stored.operational_context.company_profile.arr}}` | ARR |
| `{{stored.operational_context.company_profile.runway_months}}` | Runway months |
| `{{stored.operational_context.company_profile.headquarters}}` | HQ location |
| `{{stored.operational_context.company_profile.founded}}` | Founded date |
| `{{stored.operational_context.company_profile.mission}}` | Mission statement |
| `{{stored.operational_context.organizational_structure.departments}}` | Departments list |
| `{{stored.operational_context.organizational_structure.key_people[*].name}}` | Key person name |
| `{{stored.operational_context.organizational_structure.key_people[*].role}}` | Key person role |
| `{{stored.operational_context.organizational_structure.key_people[*].department}}` | Key person department |
| `{{stored.operational_context.organizational_structure.key_people[*].reports_to}}` | Reporting chain |
| `{{stored.operational_context.organizational_structure.key_people[*].tenure_months}}` | Tenure |
| `{{stored.operational_context.organizational_structure.key_people[*].flight_risk}}` | Flight risk level |
| `{{stored.operational_context.organizational_structure.org_chart_available}}` | Whether org chart exists |
| `{{stored.operational_context.active_projects.projects[*].name}}` | Project name |
| `{{stored.operational_context.active_projects.projects[*].status}}` | Project status |
| `{{stored.operational_context.active_projects.projects[*].owner}}` | Project owner |
| `{{stored.operational_context.active_projects.projects[*].deadline}}` | Project deadline |
| `{{stored.operational_context.active_projects.projects[*].priority}}` | Priority (high/medium/low) |
| `{{stored.operational_context.active_projects.projects[*].blockers}}` | Blockers list |
| `{{stored.operational_context.current_commitments.active_commitments[*].commitment}}` | Commitment description |
| `{{stored.operational_context.current_commitments.active_commitments[*].owner}}` | Commitment owner |
| `{{stored.operational_context.current_commitments.active_commitments[*].deadline}}` | Commitment deadline |
| `{{stored.operational_context.current_commitments.active_commitments[*].status}}` | on-track / at-risk / blocked |
| `{{stored.operational_context.metrics_kpis.tracked_metrics[*].name}}` | Metric name |
| `{{stored.operational_context.metrics_kpis.tracked_metrics[*].current_value}}` | Current value |
| `{{stored.operational_context.metrics_kpis.tracked_metrics[*].target_value}}` | Target value |
| `{{stored.operational_context.metrics_kpis.tracked_metrics[*].trend}}` | up / down / flat |
| `{{stored.operational_context.metrics_kpis.tracked_metrics[*].last_updated}}` | Last updated date |
| `{{stored.operational_context.communication_patterns.primary_channels}}` | Channels (Slack, Email, etc.) |
| `{{stored.operational_context.communication_patterns.meeting_cadence.one_on_ones}}` | 1:1 cadence |
| `{{stored.operational_context.communication_patterns.meeting_cadence.all_hands}}` | All-hands cadence |
| `{{stored.operational_context.communication_patterns.meeting_cadence.board_meetings}}` | Board meeting cadence |
| `{{stored.operational_context.known_issues.active_issues[*].issue}}` | Issue description |
| `{{stored.operational_context.known_issues.active_issues[*].severity}}` | critical / high / medium / low |
| `{{stored.operational_context.known_issues.active_issues[*].first_observed}}` | First observed date |
| `{{stored.operational_context.known_issues.active_issues[*].status}}` | Issue status |
| `{{stored.operational_context.context_gaps.missing_information[*].category}}` | Gap category |
| `{{stored.operational_context.context_gaps.missing_information[*].specific_item}}` | Specific missing item |
| `{{stored.operational_context.context_gaps.missing_information[*].importance}}` | critical / helpful / nice-to-have |
| `{{stored.operational_context.context_gaps.missing_information[*].how_to_obtain}}` | How to get the info |
| `{{stored.operational_context.operational_signals.data_available}}` | Whether signal data exists |
| `{{stored.operational_context.operational_signals.data_location}}` | Where data lives |
| `{{stored.operational_context.operational_signals.data_types}}` | Types of data available |
| `{{stored.operational_context.operational_signals.date_range.start}}` | Data range start |
| `{{stored.operational_context.operational_signals.date_range.end}}` | Data range end |
| `{{stored.operational_context.operational_signals.record_count}}` | Number of records |
| `{{stored.operational_context.operational_signals.planted_signals.available}}` | Whether planted signals exist |
| `{{stored.operational_context.operational_signals.planted_signals.categories}}` | Signal categories |
| `{{stored.operational_context.operational_signals.planted_signals.total_count}}` | Signal count |

---

## Analytics

### `analytics-anomaly-detection-investigation`

| Type | Parameter | Description |
|------|-----------|-------------|
| min | KPI and time window of anomaly | The KPI and specific time period of the anomaly |
| min | Baseline comparison period | Historical period to compare against |
| min | What initiatives changed recently | Campaigns, site changes, product releases |
| opt | Channel breakdown and spend | Performance metrics by channel |
| opt | Change log | Deploys, email sends, pricing changes |
| opt | Tracking notes | Pixels, events, tracking implementation |

### `analytics-attribution-plan-utm-governance`

| Type | Parameter | Description |
|------|-----------|-------------|
| min | Primary conversion path | Where conversion is recorded |
| min | Systems involved | Analytics, CRM, billing systems |
| min | Current UTM practices | How UTMs are currently used |
| opt | Sales cycle length and typical touch pattern | Duration and number of touchpoints |
| opt | Existing attribution model in tools | Current attribution config |
| opt | Offline conversion import capability | Offline conversion availability |

### `analytics-cohort-LTV-CAC`

| Type | Parameter | Description |
|------|-----------|-------------|
| min | Revenue model | Subscription, usage-based, or one-time |
| min | CAC inputs | Spend data, cost allocation, conversion counts |
| min | Retention/churn data | Retention and churn metrics |
| opt | Segment-level cohort data | Retention by channel, plan tier, or ICP |
| opt | Gross margin assumptions | Margin percentages by segment |
| opt | Expansion revenue patterns | Upsell/cross-sell data by cohort |

### `analytics-dashboard-spec-scorecard`

| Type | Parameter | Description |
|------|-----------|-------------|
| min | KPI list | KPIs to include |
| min | Data sources and BI tool | Where data lives and which BI tool |
| min | Primary audiences | Who will use the dashboard |
| opt | Current dashboard links/screenshots | Existing dashboards |
| opt | Key segmentation needs | How to slice data (ICP, geo, channel, cohort) |
| opt | Alert preferences | Thresholds and notification cadence |

### `analytics-experiment-design-analysis`

| Type | Parameter | Description |
|------|-----------|-------------|
| min | Hypothesis and surface being tested | What you expect and where |
| min | Primary metric and baseline | Main metric and current value |
| min | Practical constraints | Eng availability, traffic, tools, timeline |
| opt | Segment breakdown needs | Whether to analyze by segments |
| opt | Secondary guardrail metrics | Supporting metrics to monitor |
| opt | Past test results | Previous related test results |

### `analytics-experiment-plan`

| Type | Parameter | Description |
|------|-----------|-------------|
| min | Hypothesis | What you think will happen |
| min | Where in funnel | Activation, pricing, LP, retention |
| min | Metric(s) you care about | Primary metric(s) |
| opt | Baseline conversion rate | Current CVR |
| opt | Traffic volume | Expected traffic |
| opt | Constraints | Engineering time or resources |

### `analytics-measurement-framework-kpi-tree`

| Type | Parameter | Description |
|------|-----------|-------------|
| min | Business objective | Revenue, pipeline, activation, retention |
| min | Current funnel stages | Customer journey stages |
| min | Data sources available | Analytics, CRM, billing systems |
| opt | Current KPIs and definitions | Existing metrics |
| opt | Targets/benchmarks | Performance targets |
| opt | Known measurement gaps | Metrics that should be tracked but aren't |

### `analytics-metric-tree`

| Type | Parameter | Description |
|------|-----------|-------------|
| min | Business model | SaaS, e-commerce, marketplace, etc. |
| min | Primary objective | Revenue, retention, usage |
| min | Customer journey | How customers move through product |
| opt | Current KPIs | Existing tracked metrics |
| opt | Known bottlenecks | Constrained performance areas |

### `analytics-pipeline-funnel-velocity`

| Type | Parameter | Description |
|------|-----------|-------------|
| min | Funnel stages and definitions | Stage names and descriptions |
| min | Counts and conversion rates per stage | Numbers at each stage |
| min | Time-in-stage estimates | Average time per stage |
| opt | Segment/channel breakdown | Performance by channel/segment |
| opt | Sales follow-up SLA data | Response time SLAs |
| opt | Call outcome notes | Qualitative win/loss feedback |

### `analytics-weekly-insights-narrative`

| Type | Parameter | Description |
|------|-----------|-------------|
| min | KPI snapshot | Current metric values |
| min | Key initiatives running | Campaigns, releases in flight |
| min | Notable changes | Changes to spend, creative, features |
| opt | Hypotheses on what changed | Initial theories |
| opt | Qualitative signals | Sales objections, support trends |

---

## Content

### `content-ad-creative-concepts`

| Type | Parameter | Description |
|------|-----------|-------------|
| min | Audience segment and primary offer/CTA | Audience and offer |
| min | Top pains + top desired outcomes | Pain points and outcomes |
| min | Proof points available | Available proof |
| opt | Platform(s) | Meta, LinkedIn, Google |
| opt | Existing best/worst ads | Reference ads |
| opt | Compliance constraints | Restricted claims |

### `content-case-study-builder`

| Type | Parameter | Description |
|------|-----------|-------------|
| min | Customer context (industry, size) and outcome | Customer profile and result |
| min | What was hard before your product | Pre-product challenges |
| min | Measurable results | Quantifiable outcomes |
| opt | Call transcript or notes | Customer call data |
| opt | Customer quote approvals/constraints | Quote permissions |
| opt | Screenshots or product visuals | Visual assets |

### `content-copywriting-longform`

| Type | Parameter | Description |
|------|-----------|-------------|
| min | Topic + target audience | Topic and audience |
| min | One-sentence thesis | What reader should believe |
| min | CTA | Desired action |
| opt | Proof points | Data, quotes, metrics |
| opt | SEO keyword | Target keyword |
| opt | Competitive content to beat | Competitor content |

### `content-copywriting-shortform`

| Type | Parameter | Description |
|------|-----------|-------------|
| min | Channel + format | Email, LinkedIn, ad, etc. |
| min | Audience segment | Target audience |
| min | Single CTA | Call-to-action |
| opt | Messaging blocks or proof points | Proof to include |
| opt | Tone constraints | Formal, direct, playful |
| opt | Length constraints | Character/word limits |

### `content-creative-brief-builder`

| Type | Parameter | Description |
|------|-----------|-------------|
| min | Asset type(s) | Ad, LP, email, social, video, deck |
| min | Audience + desired action | Who and what action |
| min | Key message + proof available | One sentence + proof |
| opt | Brand voice and visual constraints | Brand guidelines |
| opt | Examples of good/bad references | Reference examples |
| opt | Compliance constraints | Legal/claims restrictions |

### `content-design-system-brand-kit`

| Type | Parameter | Description |
|------|-----------|-------------|
| min | Current brand assets | Logo, colors, fonts |
| min | Primary asset types to standardize | Ads, social, decks, LPs |
| min | Brand personality | 3-5 adjectives |
| opt | Examples of assets you like/dislike | Reference assets |
| opt | Accessibility requirements | Accessibility standards |
| opt | Product UI screenshots | UI visuals |

### `content-landing-page-copy`

| Type | Parameter | Description |
|------|-----------|-------------|
| min | Audience + offer/CTA | Target audience and offer |
| min | What problem you solve | One-sentence problem |
| min | Proof available | Case studies, metrics, logos |
| opt | Competitor pages to beat | Competitor LPs |
| opt | Brand voice constraints | Voice guidelines |
| opt | Technical constraints | Page builder limitations |

### `content-repurposing-distribution-matrix`

| Type | Parameter | Description |
|------|-----------|-------------|
| min | The source asset | Link/content and core thesis |
| min | Primary audience and CTA | Audience and action |
| min | Channels available | Social, email, paid, sales, PR |
| opt | Channel performance data | Per-channel performance |
| opt | Brand voice constraints | Voice guidelines |
| opt | Upcoming campaign/launch calendar | Timing alignment |

### `content-strategy-pillars-seo`

| Type | Parameter | Description |
|------|-----------|-------------|
| min | Target ICP(s) | Ideal Customer Profile(s) |
| min | Primary business objective | Pipeline, activation, retention |
| min | Core product promise and differentiators | Product promise |
| opt | Existing content performance | Top posts/keywords |
| opt | Competitor content examples | Competitor content |
| opt | Sales/support objections and FAQs | Common objections |

### `content-video-production-plan`

| Type | Parameter | Description |
|------|-----------|-------------|
| min | Video purpose | Awareness, demo, conversion + audience |
| min | Core message and CTA | Message and action |
| min | Distribution channel(s) | Paid social, website, YouTube |
| opt | Available talent | Founder, customer, team |
| opt | Existing footage or UI captures | Existing assets |
| opt | Brand constraints and examples | Brand guidelines |

---

## Development

### `dev-adr-writer`

| Type | Parameter | Description |
|------|-----------|-------------|
| min | Decision statement | Primary decision |
| min | Context / problem | Background and problem |
| min | Options considered | Options evaluated |
| min | Constraints | Time, cost, security |
| opt | Links to relevant docs/issues | Supporting references |
| opt | Date and decision owner | When and who |

### `dev-api-design`

| Type | Parameter | Description |
|------|-----------|-------------|
| min | Use case | Who calls it and why |
| min | Data objects involved | Objects being manipulated |
| min | Constraints | Latency, auth, rate limits |
| opt | Existing API conventions | Current standards |
| opt | Example payloads | Request/response examples |

### `dev-architecture-review`

| Type | Parameter | Description |
|------|-----------|-------------|
| min | Design doc or architecture description | System design |
| min | Scale assumptions | Expected scale and growth |
| min | Latency requirements | Performance expectations |
| min | Compliance/security | Regulatory/security needs |
| min | Team size/skill | Team capacity and expertise |
| opt | Current incident history | Past failures |
| opt | Known tech debt | Existing technical issues |

### `dev-cicd-design`

| Type | Parameter | Description |
|------|-----------|-------------|
| min | Repo type | Monorepo or multi-repo |
| min | Deployment targets | k8s, serverless, mobile, web |
| min | Current CI tool | GitHub Actions, GitLab, Circle |
| min | Release frequency goal | Target deployment cadence |
| opt | Compliance requirements | SOC2, approvals |
| opt | Existing pain points | Slow builds, flaky tests |

### `dev-code-review`

| Type | Parameter | Description |
|------|-----------|-------------|
| min | PR diff or key files | Code changes |
| min | Feature intent | What the code does (1-2 sentences) |
| min | Any constraints | Deadline, risk tolerance |

### `dev-code-scaffold`

| Type | Parameter | Description |
|------|-----------|-------------|
| min | Language/framework | Programming language and framework |
| min | Feature spec or user story | What to build |
| min | Existing repo conventions | Code structure/style rules |
| opt | Repo tree excerpt | File structure sample |
| opt | Lint/test tooling | Testing and lint config |

### `dev-db-schema`

| Type | Parameter | Description |
|------|-----------|-------------|
| min | Entities and relationships | Data entities |
| min | Expected query patterns | Top 5 queries |
| min | Data volume assumptions | Current and 12-month projections |
| opt | Existing schema | Current schema |
| opt | Performance requirements | Latency/throughput |

### `dev-debugging-playbook`

| Type | Parameter | Description |
|------|-----------|-------------|
| min | Symptom | What is going wrong |
| min | Where it appears | Environment, endpoint, segment |
| min | When it started | Timeline or "unknown" |
| min | Any recent changes | Deploys, config, migrations |
| opt | Logs/stack traces | Error logs |
| opt | Metrics screenshots | Performance data |
| opt | Repro steps | How to reproduce |

### `dev-documentation-writer`

| Type | Parameter | Description |
|------|-----------|-------------|
| min | What doc type | README/runbook/onboarding/API |
| min | Audience | New engineer, on-call, external dev |
| min | Current system description | Bullet points |
| opt | Repo links or file tree excerpt | Repo structure |
| opt | Existing docs to align with | Doc standards |
| opt | Known "gotchas" | Common pitfalls |

### `dev-migration-plan`

| Type | Parameter | Description |
|------|-----------|-------------|
| min | What is changing | From → to description |
| min | Data volume estimate | Amount of data |
| min | Downtime tolerance | Acceptable downtime |
| min | Rollback feasibility | Ability to rollback |
| opt | Current schema/contracts | Existing schema/APIs |
| opt | Constraints | Compliance, time |

### `dev-observability-plan`

| Type | Parameter | Description |
|------|-----------|-------------|
| min | System/feature | What to monitor |
| min | Deployment environment(s) | Environments |
| min | Current tooling | Datadog, Grafana, Sentry |
| opt | Known incident patterns | Historical incidents |
| opt | Key user journeys | Critical workflows |

### `dev-performance-plan`

| Type | Parameter | Description |
|------|-----------|-------------|
| min | System/endpoint/feature | What to optimize |
| min | Expected traffic now + 12 months | Current and projected load |
| min | Latency expectations | Performance targets |
| opt | Current baseline metrics | Existing perf data |
| opt | Cost constraints | Budget limitations |

### `dev-prd-to-techspec`

| Type | Parameter | Description |
|------|-----------|-------------|
| min | PRD content | Product requirements doc |
| min | Current architecture constraints | Stack, services, DBs |
| opt | Existing APIs/contracts | Current API definitions |
| opt | Performance expectations | Target performance |
| opt | Security/compliance constraints | Regulatory requirements |

### `dev-requirements-to-spec`

| Type | Parameter | Description |
|------|-----------|-------------|
| min | Feature/request description | Raw feature description |
| min | Target user + use case | Who and why |
| min | Deadline | Timeline |
| opt | Screenshots/mockups | Visual references |
| opt | Existing system constraints | System limitations |
| opt | Success metric | How to measure success |

### `dev-security-threat-model`

| Type | Parameter | Description |
|------|-----------|-------------|
| min | System/feature description | What to threat model |
| min | Data types handled | PII, secrets, payments |
| min | Auth model | Users, roles, tokens |
| min | Deployment context | Cloud, on-prem |
| opt | Architecture diagram | System architecture |
| opt | Compliance requirements | SOC2, HIPAA, GDPR |

### `dev-test-plan`

| Type | Parameter | Description |
|------|-----------|-------------|
| min | Feature spec or description | What to test |
| min | Risk tolerance | Low/medium/high |
| min | Platforms/environments | Web, iOS, API |
| opt | Known edge cases | Specific edge cases |
| opt | Past bugs in this area | Historical issues |

---

## Distribution

### `distribution-affiliate-syndication-program`

| Type | Parameter | Description |
|------|-----------|-------------|
| min | Offer and conversion event | What partners promote and success metric |
| min | Commission or incentive constraints | Budget/incentive rules |
| min | Tracking method | UTMs, affiliate platform, coupon codes |
| opt | Existing partner list or targets | Partner prospects |
| opt | Brand safety constraints | Disallowed partner categories |
| opt | Legal/compliance requirements | Legal considerations |

### `distribution-audience-targeting-retargeting`

| Type | Parameter | Description |
|------|-----------|-------------|
| min | Audience segments | Define segments or link to segmentation |
| min | Platforms and available data sources | Marketing platforms and data |
| min | Primary conversion action | Main conversion goal |
| opt | Past audience performance | Historical performance |
| opt | Privacy/compliance constraints | Data privacy requirements |
| opt | Creative/messaging variations available | Available creative options |

### `distribution-channel-mix-budget`

| Type | Parameter | Description |
|------|-----------|-------------|
| min | Primary goal and time horizon | Pipeline/activation/revenue + timeframe |
| min | Channels available | Owned, paid, partners, PR |
| min | Total budget | Budget or "time budget" if no paid |
| opt | Historical CAC/CPA by channel | Cost per acquisition history |
| opt | Capacity constraints | Sales/creative bandwidth |
| opt | Strategic bets | New channel experiments |

### `distribution-CRO-testing-playbook`

| Type | Parameter | Description |
|------|-----------|-------------|
| min | Target surface | LP, signup flow, onboarding |
| min | Baseline metric (CVR, activation rate) | Baseline conversion metric |
| min | Primary conversion goal | Conversion definition |
| opt | Heatmaps/session recordings insights | User behavior data |
| opt | Top objections from Sales/support | Customer objections |
| opt | Engineering constraints for tests | Technical limitations |

### `distribution-email-newsletter-program`

| Type | Parameter | Description |
|------|-----------|-------------|
| min | Who the newsletter is for | Audience segment |
| min | Value promise | What readers reliably get |
| min | Desired business outcome | Pipeline, retention, brand |
| opt | Existing content backlog | Available content |
| opt | Founder voice availability | Founder availability |
| opt | List size and growth channels | Current list metrics |

### `distribution-lifecycle-nurture-sequences`

| Type | Parameter | Description |
|------|-----------|-------------|
| min | Target segment and funnel stage | Audience and funnel position |
| min | Desired stage transition | e.g., trial → activated |
| min | Top 5 objections/frictions | Key objections |
| opt | Current sequence drafts | Existing email content |
| opt | Product usage milestones | Key usage milestones |
| opt | Sales handoff rules | When to alert reps |

### `distribution-paid-search-build`

| Type | Parameter | Description |
|------|-----------|-------------|
| min | Product/category and target ICP | Product and ideal customer |
| min | Primary conversion action | Signup, demo, purchase |
| min | Geo/language scope | Geographic/language targeting |
| opt | Existing keyword list and performance | Historical keyword data |
| opt | Competitor brands | For conquesting decisions |
| opt | Budget and CPA targets | Budget and cost targets |

### `distribution-paid-social-build`

| Type | Parameter | Description |
|------|-----------|-------------|
| min | Platform(s) and target audience | Platforms and audience |
| min | Offer/CTA | Offer and call-to-action |
| min | Landing page destination | LP URL |
| opt | Past performance by audience/creative | Historical performance |
| opt | Budget and CPA targets | Budget and cost targets |
| opt | Creative constraints | Video vs static |

---

## Events

### `event-concept-brief`

| Type | Parameter | Description |
|------|-----------|-------------|
| min | Event type | Conference, meetup, workshop, launch |
| min | Target audience | Roles, industries |
| min | Date window and city | If known |
| min | Budget range | Rough estimate |
| opt | Speakers/partners you want | Desired speakers |
| opt | Sponsors you could approach | Potential sponsors |

### `event-on-site-ops`

| Type | Parameter | Description |
|------|-----------|-------------|
| min | Expected attendance | Headcount |
| min | Venue layout constraints | Space limitations |
| min | Staffing available | Staff count |
| min | Registration method | QR, list, badges |
| opt | Accessibility considerations | Accessibility needs |
| opt | Sponsor booths/expo needs | Expo requirements |

### `event-post-event-report`

| Type | Parameter | Description |
|------|-----------|-------------|
| min | Attendance numbers | Actual attendance |
| min | Budget actuals | Rough spend |
| min | Feedback | Survey summary or anecdotes |
| min | Sponsor commitments delivered | What was fulfilled |
| opt | Pipeline influenced | Pipeline impact |
| opt | Content metrics | Views, leads |

### `event-production-advance`

| Type | Parameter | Description |
|------|-----------|-------------|
| min | Venue details | Venue information |
| min | Agenda/session types | Session structure |
| min | Recording/streaming needs | A/V requirements |
| opt | Vendor quotes | Vendor pricing |
| opt | Stage design preferences | Stage layout |

### `event-program-design`

| Type | Parameter | Description |
|------|-----------|-------------|
| min | Event purpose + audience | Purpose and who |
| min | Total duration | Half-day/day/multi-day |
| min | Content goals | Education, community, sales |
| opt | Constraints | Keynote slots, sponsor sessions |
| opt | Speaker candidates | Potential speakers |

### `event-run-of-show`

| Type | Parameter | Description |
|------|-----------|-------------|
| min | Event agenda skeleton | Agenda outline |
| min | Venue hours | Load-in, doors, hard stop |
| min | Staff roles available | Producer, stage manager, A/V |
| opt | Speaker list | Confirmed speakers |
| opt | Sponsor obligations | Sponsor requirements |
| opt | Content capture requirements | Recording needs |

### `event-speaker-pipeline`

| Type | Parameter | Description |
|------|-----------|-------------|
| min | Event theme + audience | Theme and audience |
| min | Desired speaker archetypes | Operator, founder, academic |
| min | Speaker budget | Honorarium/travel yes/no |
| opt | Existing relationships | Current contacts |
| opt | Priority targets | Top picks |

### `event-sponsor-packages`

| Type | Parameter | Description |
|------|-----------|-------------|
| min | Event audience size estimate | Expected attendance |
| min | Audience profile | Roles, industries |
| min | Sponsor categories to avoid | Exclusions |
| min | Revenue goal | Target sponsor revenue |
| opt | Comparable events sponsorship pricing | Market rates |
| opt | Deliverables you can realistically fulfill | What you can offer |

### `event-ticketing-pricing`

| Type | Parameter | Description |
|------|-----------|-------------|
| min | Capacity | Max attendees |
| min | Budget and revenue goal | Financial targets |
| min | Audience willingness-to-pay hypothesis | Price sensitivity |
| min | Event value proposition | Why attend |
| opt | Sponsor revenue expectations | Sponsor income |
| opt | Competitor event pricing | Market rates |

### `event-venue-selection`

| Type | Parameter | Description |
|------|-----------|-------------|
| min | City/date window | Location and timing |
| min | Capacity range | Size needed |
| min | Format | Seated talks, workshops, expo |
| min | Budget range | Venue budget |
| opt | Accessibility requirements | Accessibility needs |
| opt | AV expectations | Recording, streaming |

---

## Marketing

### `mkt-case-study`

| Type | Parameter | Description |
|------|-----------|-------------|
| min | Customer profile (industry, size) | Customer context |
| min | Problem before | Pre-product challenges |
| min | What you implemented | Solution details |
| min | Outcome after (metrics if possible) | Results achieved |
| opt | Quotes (raw) | Direct customer quotes |
| opt | Timeline | Implementation schedule |
| opt | Screenshots | Product visuals |

### `mkt-competitive-intel`

| Type | Parameter | Description |
|------|-----------|-------------|
| min | Competitors (3-7) or "status quo alternative" | Competitor list |
| min | Your positioning one-liner | Market position |
| min | Where you lose deals | Loss analysis |
| opt | Links/screenshots of competitor sites/pricing | Visual references |
| opt | Win/loss notes | Sales outcomes |

### `mkt-content-strategy`

| Type | Parameter | Description |
|------|-----------|-------------|
| min | ICP | Ideal Customer Profile |
| min | Primary product category / promise | What product does |
| min | Business goal | Pipeline, signups, awareness |
| min | Channels you can realistically support | Available channels |
| opt | Existing content performance | Historical data |
| opt | Founder voice samples | Voice/style examples |
| opt | Sales objections | Common concerns |

### `mkt-editorial-calendar`

| Type | Parameter | Description |
|------|-----------|-------------|
| min | Time horizon | 4 weeks / 8 weeks / quarter |
| min | Channels | Blog, newsletter, social |
| min | Pillars/topics | Content themes |
| opt | Launch dates and events | Key calendar dates |
| opt | Owner availability | Team schedules |

### `mkt-email-nurture-sequence`

| Type | Parameter | Description |
|------|-----------|-------------|
| min | Persona | Target buyer type |
| min | Offer | Trial, lead magnet, demo |
| min | Primary objection(s) | Main concerns |
| min | Desired end action | Activate, book call |
| opt | Product onboarding steps | How users start |
| opt | Case studies / proof | Customer evidence |

### `mkt-icp-definition`

| Type | Parameter | Description |
|------|-----------|-------------|
| min | Product description (1 paragraph) | What you sell |
| min | Current customers + 3 examples | Real customer references |
| min | Price point / pricing model | How you charge |
| min | Sales motion | Self-serve / sales-led / hybrid |
| opt | Churn reasons | Why customers leave |
| opt | Win/loss notes | Sales analysis |
| opt | Market you think you're in | Market positioning |

### `mkt-landing-page-brief`

| Type | Parameter | Description |
|------|-----------|-------------|
| min | Target segment (ICP slice) | Specific audience |
| min | Offer | Trial, webinar, checklist, demo |
| min | Traffic source | Ads, email, SEO |
| min | Primary conversion | Signup, booked call |
| opt | Existing LP copy/performance | Current page reference |
| opt | Proof assets | Customer evidence |

### `mkt-launch-content-pack`

| Type | Parameter | Description |
|------|-----------|-------------|
| min | What launched (1 paragraph) | Launch description |
| min | ICP + primary persona | Target audience |
| min | Proof points | Metrics, quotes, demos |
| min | CTA | Call-to-action |
| opt | Launch date/time | When it goes live |
| opt | Pricing changes | Price news |
| opt | Visual assets available | Imagery/graphics |

### `mkt-messaging-matrix`

| Type | Parameter | Description |
|------|-----------|-------------|
| min | ICP + key personas (roles) | Target buyers |
| min | Top 3 pains and top 3 outcomes | Problems and results |
| min | Competitor alternatives | What customers consider |
| opt | Real customer language | Quotes, call notes |
| opt | Pricing/packaging | Fee structure |

### `mkt-metrics-dashboard`

| Type | Parameter | Description |
|------|-----------|-------------|
| min | Business model | SaaS, services, marketplace |
| min | Sales motion | Self-serve vs sales-led |
| min | Primary goal | Pipeline, signups, revenue |
| opt | Current analytics stack | Tools in use |
| opt | Existing KPI definitions | Current metrics |

### `mkt-onboarding-activation`

| Type | Parameter | Description |
|------|-----------|-------------|
| min | Product + ICP | What you sell and for whom |
| min | What is activation? | One measurable event |
| min | Current onboarding steps | Existing user flow |
| opt | Drop-off points | Where users abandon |
| opt | Onboarding emails currently sent | Existing emails |

### `mkt-paid-ads-plan`

| Type | Parameter | Description |
|------|-----------|-------------|
| min | Goal | Pipeline, signups, trials |
| min | Budget range | Spending limit |
| min | ICP | Target audience |
| min | Offer | What's promoted |
| min | Channels | Google, LinkedIn, Meta |
| opt | Current CAC/LTV assumptions | Cost/value estimates |
| opt | Existing creative assets | Ad materials |
| opt | Conversion rate benchmarks | Historical CVR |

### `mkt-partnerships-plan`

| Type | Parameter | Description |
|------|-----------|-------------|
| min | ICP | Target audience |
| min | What partners could gain | Revenue, retention, differentiation |
| min | What you can offer | Co-marketing, rev share, integrations |
| opt | List of potential partners | Specific targets |
| opt | Existing integration roadmap | Technical plans |

### `mkt-positioning-narrative`

| Type | Parameter | Description |
|------|-----------|-------------|
| min | ICP | Ideal Customer Profile |
| min | Top competitor alternatives | Competitive set |
| min | 3 strongest product advantages | Key differentiators |
| min | Pricing level | Cheap/mid/premium |
| opt | Customer quotes | Testimonials |
| opt | Case studies | Customer proof |
| opt | Technical differentiators | Product-specific advantages |

### `mkt-pricing-page-copy`

| Type | Parameter | Description |
|------|-----------|-------------|
| min | Pricing model | Seat, usage, tiered |
| min | Plan names + prices | Offering details |
| min | ICP + buyer role | Audience and decision maker |
| min | Top objections | Price, risk, switching |
| opt | Competitor pricing pages | Competitive reference |
| opt | Historical conversion data | Performance history |

### `mkt-pr-plan`

| Type | Parameter | Description |
|------|-----------|-------------|
| min | What's newsworthy | Launch, data, contrarian thesis |
| min | ICP and why they care | Audience and relevance |
| min | Proof | Numbers, customers, story |
| min | Geography | Where you want coverage |
| opt | Founder background | Credentials/story |
| opt | Existing relationships | Media contacts |

### `mkt-sales-enablement-pack`

| Type | Parameter | Description |
|------|-----------|-------------|
| min | Positioning + messaging pillars | Core messaging |
| min | ICP + personas | Target buyers |
| min | Pricing/packaging | Fee structure |
| min | Common objections | Customer concerns |
| opt | Current deck/one-pager | Existing materials |
| opt | Sales call notes | Conversation data |

### `mkt-seo-keyword-plan`

| Type | Parameter | Description |
|------|-----------|-------------|
| min | Product category | Market/product type |
| min | ICP | Target audience |
| min | Regions/languages | Geographic scope |
| min | Competitors | Competitive reference |
| opt | Current site + content inventory | Existing assets |
| opt | Any rankings data | Current search performance |

### `mkt-social-distribution-plan`

| Type | Parameter | Description |
|------|-----------|-------------|
| min | Channels | X, LinkedIn, YouTube, TikTok, community |
| min | ICP + tone | Audience and voice style |
| min | Time budget per week | Available resources |
| min | Existing audience size | Current follower count |
| opt | Founder voice samples | Writing examples |
| opt | Top performing posts | High-performing content |

### `mkt-website-copy`

| Type | Parameter | Description |
|------|-----------|-------------|
| min | Positioning one-liner | Value proposition |
| min | ICP + persona | Target buyers |
| min | Primary CTA | Book demo / start trial / join waitlist |
| min | Proof assets available | Logos, metrics, quotes |
| opt | Competitor sites you admire | Design/messaging reference |
| opt | Product screenshots descriptions | Visual product info |

---

## Operations

### `ops-agency-vendor-management`

| Type | Parameter | Description |
|------|-----------|-------------|
| min | Scope of work needed | Design, paid, PR, content |
| min | Budget range and internal owner/DRI | Budget and ownership |
| min | What "good" looks like | Quality bar and metrics |
| opt | Existing contracts or scopes | Current agreements |
| opt | Pain points with current vendors | Vendor issues |
| opt | Review/approval constraints | Approval process |

### `ops-compliance-privacy-brand-safety`

| Type | Parameter | Description |
|------|-----------|-------------|
| min | Industry constraints | Regulated or not |
| min | Review stakeholders | Legal, security, brand |
| min | Types of claims commonly made | Performance, security, pricing |
| opt | Existing policies or legal guidelines | Current policies |
| opt | Past compliance incidents | Historical issues |
| opt | Privacy stack details | Consent management |

### `ops-CRM-hygiene-lead-handoff`

| Type | Parameter | Description |
|------|-----------|-------------|
| min | CRM used and basic pipeline stages | CRM and stages |
| min | Current lead routing process | Even if "manual" |
| min | SLA expectations | Follow-up speed |
| opt | Field list and definitions | CRM fields |
| opt | Current lead scoring rules | Scoring rules |
| opt | Common failure cases | Lost leads, duplicates |

### `ops-localization-regionalization`

| Type | Parameter | Description |
|------|-----------|-------------|
| min | Target regions/languages | Regions and languages |
| min | Core assets to localize | LPs, emails, ads, decks |
| min | Ownership | Central vs regional teams |
| opt | Regional constraints | Legal, cultural, channel |
| opt | Translation resources | Vendors, in-house |
| opt | Region-specific ICP differences | Regional variations |

### `ops-marketing-planning-budgeting`

| Type | Parameter | Description |
|------|-----------|-------------|
| min | Business targets | Pipeline/revenue/activation + time horizon |
| min | Total marketing budget | Or headcount-only constraints |
| min | Current channel mix and team capacity | Existing setup |
| opt | Historical spend and ROI | Past performance |
| opt | Known strategic bets | New channel/segment |
| opt | Vendor/agency costs | External costs |

### `ops-martech-stack-architecture`

| Type | Parameter | Description |
|------|-----------|-------------|
| min | Current tools in use | CRM, email, analytics, BI, ad platforms |
| min | Primary use cases | Lead capture, nurturing, attribution, scoring |
| min | Who owns each system today | System ownership |
| opt | Integration constraints | Eng support, security policies |
| opt | Data quality issues | Data problems |
| opt | Future desired capabilities | CDP, personalization |

### `ops-project-management-sprint-system`

| Type | Parameter | Description |
|------|-----------|-------------|
| min | Tool used | Asana, Jira, Notion, Trello |
| min | Core workstreams | Content, paid, lifecycle, PR, design |
| min | Who can request work | Intake rules |
| opt | Current board screenshot or export | Current setup |
| opt | Pain points | Too many requests, unclear priority |
| opt | Sprint length preference | 1 or 2 weeks |

### `ops-sales-enablement-core-kit`

| Type | Parameter | Description |
|------|-----------|-------------|
| min | ICP + positioning | Or rough |
| min | Primary product promise and proof points | Product value |
| min | Top competitors and objections | Competitive landscape |
| opt | Existing deck/collateral | Current materials |
| opt | Win/loss notes and call snippets | Sales data |
| opt | Pricing/packaging constraints | Pricing rules |

---

## Customer Success

### `cs-churn-save-playbook`

| Type | Parameter | Description |
|------|-----------|-------------|
| min | Product + customer type | What and who |
| min | Common churn reasons | If known |
| min | What offers you can make | Discount, training, pause |
| opt | Usage metrics available | Usage data |
| opt | Past churn examples | Historical churn |

### `cs-onboarding-plan`

| Type | Parameter | Description |
|------|-----------|-------------|
| min | Customer type (ICP) | Customer profile |
| min | What success looks like for them | Success definition |
| min | Product onboarding steps | If any exist |
| min | Timeline expectation | Expected timeline |
| opt | Common onboarding failures | Failure patterns |
| opt | Implementation complexity | Complexity level |

---

## Finance

### `fin-budget-builder`

| Type | Parameter | Description |
|------|-----------|-------------|
| min | Current cash | Cash on hand |
| min | Monthly revenue | Or 0 |
| min | Major expense categories | Expense types |
| min | Headcount and planned hires | Team size/plans |
| min | Runway goal (months) | Target runway |
| opt | Unit economics assumptions | Unit economics |
| opt | Known upcoming one-time costs | One-time expenses |

---

## Legal

### `legal-privacy-policy-outline`

| Type | Parameter | Description |
|------|-----------|-------------|
| min | Product description | What the product is |
| min | Data collected | PII, usage data, payments |
| min | Third parties | Analytics, payments, email |
| min | Regions served | US/EU/etc. |
| opt | Security practices | Encryption, retention |
| opt | Cookie usage | Cookie policy |

---

## Cross-Functional

### `integrated-campaign-planning`

| Type | Parameter | Description |
|------|-----------|-------------|
| min | Campaign window | Start/end dates |
| min | Single primary goal | One number + success/failure definition |
| min | Target audience | Even if rough |
| min | Budget range | Or "no paid" |
| opt | Offer details | Pricing, promo, trial terms |
| opt | Prior campaign results or benchmarks | Historical data |
| opt | Existing assets | Case studies, demos, blogs |
| opt | Sales capacity constraints | Lead handling capacity |

### `launch-tiering`

| Type | Parameter | Description |
|------|-----------|-------------|
| min | What's launching | 1-2 sentences |
| min | Expected impact type | Revenue / activation / retention / PR / strategic |
| min | Primary audience | Existing / net-new / enterprise / SMB / devs |
| opt | Expected impact magnitude | Rough $ or users |
| opt | Risk level | Low/med/high (brand, legal, reliability) |
| opt | Dependencies | Sales readiness, CS load, partner coordination |
| opt | Competitive context | Response required? yes/no |

---

## Organizing

### `organizing-decision-log-commitments`

| Type | Parameter | Description |
|------|-----------|-------------|
| min | Where the log will live | Notion, Google Sheet, etc. |
| min | Types of decisions to track | Product, pricing, campaign, creative, budget |
| min | Who can declare a decision "final" | Authorization rules |
| opt | Recent meeting notes to backfill | Prior meeting docs |
| opt | Existing task system to link out | Jira/Asana integration |

### `organizing-information-architecture`

| Type | Parameter | Description |
|------|-----------|-------------|
| min | Where work currently lives | Google Drive, Notion, Dropbox |
| min | Core workstreams to support | Campaigns, launches, content, paid, PR |
| min | Who needs access | Internal teams and external parties |
| opt | Existing folder tree or screenshot | Current structure |
| opt | Current pain points | Duplicates, version confusion |
| opt | Compliance constraints | SOC2, GDPR |

### `organizing-knowledge-base-sops`

| Type | Parameter | Description |
|------|-----------|-------------|
| min | Where the knowledge base will live | Storage location |
| min | Top recurring workflows | Launches, campaigns, paid, email |
| min | Who the SOPs are written for | New hire, agency, partner |
| opt | Existing docs/process notes | Prior documentation |
| opt | Recent incidents or failures | Issues to encode |

### `organizing-meeting-cadence`

| Type | Parameter | Description |
|------|-----------|-------------|
| min | Team structure and key stakeholders | Roles (Sales, Product, CS) |
| min | Current pain points | Too many meetings, slow approvals |
| min | Top outcomes for next 30-90 days | Strategic priorities |
| opt | Current calendar snapshot | Existing schedule |
| opt | Existing meeting list | Titles and frequency |
| opt | Preferences | Async-first vs synchronous |

### `organizing-naming-taxonomy-utm`

| Type | Parameter | Description |
|------|-----------|-------------|
| min | Primary channels used | Paid search, social, email, partners |
| min | Reporting destination | GA4, HubSpot, Salesforce, Looker |
| min | Common dimensions to slice by | Campaign, audience, geo, product |
| opt | Existing UTM examples | Good/bad examples |
| opt | Current campaign naming conventions | Existing patterns |
| opt | CRM fields for campaign attribution | CRM integration |

### `organizing-stakeholder-map`

| Type | Parameter | Description |
|------|-----------|-------------|
| min | List of stakeholder groups | Sales, Product, CS, Finance, Legal |
| min | What each stakeholder needs from marketing | Per-group definition |
| min | Known friction points | Areas of conflict |
| opt | Names + roles | Individual details |
| opt | Existing comms channels | Slack channels, meetings |
| opt | Past escalations or failures | Historical misalignments |

---

## Org

### `org-backlog-triage`

| Type | Parameter | Description |
|------|-----------|-------------|
| min | Export/paste of backlog items | Title, description, metadata (10-30 items) |
| min | Top 3 outcomes for next 30 days | Strategic priorities |
| min | Any immovable deadlines | Fixed date constraints |

### `org-commitment-tracker`

| Type | Parameter | Description |
|------|-----------|-------------|
| min | A list of commitments | Promises made (or meeting notes to extract from) |
| opt | Current tool | Asana/Jira/Notion/Sheet |

### `org-cross-functional-brief`

| Type | Parameter | Description |
|------|-----------|-------------|
| min | Initiative name | Project identifier |
| min | Target user/customer | End user definition |
| min | Desired outcome + success metric | Measurable goal |
| min | Target date / launch window | Timeline |
| min | Teams involved | Cross-functional parties |
| opt | PRD / spec link | Product documentation |
| opt | Prior learnings | Customer calls, churn reasons |

### `org-decision-log`

| Type | Parameter | Description |
|------|-----------|-------------|
| min | Decision statement | One sentence per decision |
| min | Owner | Who decided |
| min | Date | When (or approximate) |
| min | Rationale | Bullets |
| min | What would cause a revisit | Revisit triggers |

### `org-file-system`

| Type | Parameter | Description |
|------|-----------|-------------|
| min | Where files live today | Google Drive/Notion/Confluence/Dropbox/GitHub |
| min | Team size and functions | Eng, Product, Sales, Marketing, Ops |
| min | Top workflows | Product dev, onboarding, hiring, fundraising |
| opt | Screenshot/list of current top-level folders | Current structure |
| opt | Examples of important docs people can't find | Discovery pain points |

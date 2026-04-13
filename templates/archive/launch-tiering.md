Category: orchestration
Response Format: mixed

---


# Launch Tiering

You run this skill as Gargoyle’s execution partner: fast, concrete, and decision-focused.


## WHAT THIS SKILL DOES
Prevent over/under-shipping by classifying a launch into a tier and automatically setting the **minimum viable launch plan**: assets, approvals, channels, enablement, and measurement depth.

## WHEN TO USE
Kick this off immediately when a launch is proposed — *before* you assign resources.

Use tiering when:
- The org argues about “how big” a launch should be.
- There are multiple launches competing for attention.
- You want a repeatable standard (Tier 1 vs Tier 2 vs Tier 3).

## INPUTS (MINIMUM)
- What’s launching (1–2 sentences)
- Expected impact type: [revenue / activation / retention / PR / strategic]
- Primary audience: [existing / net-new / enterprise / SMB / devs]

## INPUTS (OPTIONAL — ONLY IF AVAILABLE)
- Expected impact magnitude (rough): $ or users
- Risk level: [low/med/high] (brand, legal, reliability)
- Dependencies: Sales readiness, CS load, partner coordination
- Competitive context: response required? yes/no

## PROCESS
1. **Score the launch** on 5 dimensions: Revenue impact, user impact, strategic narrative, risk, and coordination complexity.
2. **Assign a tier**: Tier 1 (major), Tier 2 (meaningful), Tier 3 (minor/iterative).
3. **Generate tier-specific minimum artifacts** (asset checklist, enablement checklist, measurement checklist).
4. **Set the operating cadence**: war-room needed or not; weekly vs daily check-ins.
5. **Publish the tier decision** with rationale so teams stop debating.
6. **Handoff**: If Tier 1/2, start `product-launch-maestro`. If Tier 3, use a lightweight staging + announcement.

## OUTPUT FORMAT
### TIER DECISION
- **Tier:** [1 / 2 / 3]
- **Rationale (3 bullets):**
- **Primary metric:**
- **Primary audience:**
- **Risk level:**

### MINIMUM ARTIFACTS BY TIER

**Tier 1 (Major)**
- Launch one-pager + message house + FAQ
- Landing page + announcement email + sales enablement kit
- PR/press strategy (if relevant)
- Full tracking plan + dashboard + alert thresholds
- Launch week war-room

**Tier 2 (Meaningful)**
- Launch brief + FAQ
- Announcement email + social kit + basic enablement
- Tracking plan + dashboard tiles
- Light war-room (as needed)

**Tier 3 (Iterative)**
- Release note + internal note
- Optional social post
- Basic tracking (event + metric)

## GUARDRAILS (NON-CREEPY, NON-OVERWHELMING)
- **Opt-in only:** Use only what Gargoyle pastes/provides. Never imply you “saw” private data.
- **Evidence-forward:** If a claim depends on missing info, label it as an assumption and list what would confirm it.
- **Decision-first:** Don’t dump frameworks. Produce an artifact Gargoyle can use immediately.
- **Suggestion budget:** Offer *at most 2* recommended next skills at the end (unless Gargoyle asks for more).
- **No creep:** No sentiment guesses about people. No “what they really mean.” Stick to facts, incentives, commitments, and risks.
- **Fast by default:** Start with a “quick pass” output; deepen only if Gargoyle asks.
- **No politics:** Tier is determined by impact and risk, not who is loudest.

## RECOMMENDED HANDOFFS
- Tier 1/2 → `product-launch-maestro` + `staging-launch-readiness-checklist`
- Tier 3 → `staging-creative-qa-approvals` + `content-copywriting-shortform`
- If launch requires comms outside owned channels → `pr-pr-strategy-calendar`

## TRIGGER PHRASES
- How big should this launch be?
- Is this a Tier 1 launch?
- We don’t have resources for everything
- Set expectations for this launch
- Right-size this announcement


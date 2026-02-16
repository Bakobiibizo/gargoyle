Category: analytics
Response Format: mixed

---


# Analytics: Cohorts + LTV/CAC

You run this skill as Gargoyle’s execution partner: fast, concrete, and decision-focused.


## WHAT THIS SKILL DOES
Connect marketing actions to business health: whether acquisition is profitable, what cohorts retain, and how payback changes by segment/channel.

## WHEN TO USE
Kick this off when:
- Scaling spend and needing guardrails.
- CAC is rising and you need to know if it’s still worth it.
- You suspect some segments retain far better than others.

Run monthly/quarterly; revisit after big channel changes.

## INPUTS (MINIMUM)
- Revenue model (subscription, usage-based, one-time)
- CAC inputs (spend + cost allocation) and conversion counts
- Retention/churn data (even rough)

## INPUTS (OPTIONAL — ONLY IF AVAILABLE)
- Segment-level cohort data (by channel/plan/ICP)
- Gross margin assumptions
- Expansion revenue patterns

## PROCESS
1. **Define unit economics model**: CAC, gross margin, payback, LTV definition.
2. **Build cohorts**: retention by signup month and by channel/segment (if possible).
3. **Estimate LTV**: conservative and optimistic scenarios; include margin.
4. **Compute payback**: time to recover CAC.
5. **Identify levers**: improve activation, reduce churn, increase ARPU, lower CAC.
6. **Recommend guardrails**: acceptable CAC by segment based on payback targets.

## OUTPUT FORMAT
### UNIT ECONOMICS BRIEF
- CAC (blended):
- LTV (conservative/optimistic):
- Payback period:
- Best cohort segments:
- Worst cohort segments:

### GUARDRAILS
- Max CAC for Segment A:
- Max CAC for Segment B:
- Stop spending when payback > __ months (example)

### LEVERS (ranked)
1) ...
2) ...

## GUARDRAILS (NON-CREEPY, NON-OVERWHELMING)
- **Opt-in only:** Use only what Gargoyle pastes/provides. Never imply you “saw” private data.
- **Evidence-forward:** If a claim depends on missing info, label it as an assumption and list what would confirm it.
- **Decision-first:** Don’t dump frameworks. Produce an artifact Gargoyle can use immediately.
- **Suggestion budget:** Offer *at most 2* recommended next skills at the end (unless Gargoyle asks for more).
- **No creep:** No sentiment guesses about people. No “what they really mean.” Stick to facts, incentives, commitments, and risks.
- **Fast by default:** Start with a “quick pass” output; deepen only if Gargoyle asks.
- **Don’t overfit to short data windows:** Early cohorts can mislead; use ranges and scenarios.

## RECOMMENDED HANDOFFS
- For budget allocation → `distribution-channel-mix-budget`
- For activation messaging → `distribution-lifecycle-nurture-sequences`

## TRIGGER PHRASES
- Calculate LTV and CAC
- Analyze retention cohorts
- Is paid spend profitable?
- Set CAC guardrails
- Unit economics analysis


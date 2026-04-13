Category: analytics
Response Format: mixed

---


# Analytics: Attribution Plan + UTM Governance

You run this skill as Gargoyle’s execution partner: fast, concrete, and decision-focused.


## WHAT THIS SKILL DOES
Set realistic attribution rules so reporting isn’t a constant argument. Define how UTMs are used, how multi-touch is interpreted, and how to reconcile analytics vs CRM.

## WHEN TO USE
Kick this off when:
- Channel owners fight over credit.
- CRM and analytics disagree on source.
- You’re scaling paid and need reliable attribution.

Pair with naming taxonomy and tracking staging.

## INPUTS (MINIMUM)
- Primary conversion path (where conversion is recorded)
- Systems involved (analytics + CRM + billing)
- Current UTM practices (even “none”)

## INPUTS (OPTIONAL — ONLY IF AVAILABLE)
- Sales cycle length and typical touch pattern
- Existing attribution model in tools
- Offline conversion import capability

## PROCESS
1. **Define the attribution question**: what decisions are you using attribution to make?
2. **Set baseline model**: last-touch for ops + multi-touch for learning (usually).
3. **Define lookback windows**: by funnel stage and sales cycle.
4. **UTM governance**: required params, allowed values, enforcement checklist.
5. **Reconciliation rules**: what is source of truth for pipeline/revenue (usually CRM).
6. **Edge cases**: direct traffic, dark social, partner referrals, offline events.
7. **Document limitations** and what you will *not* pretend to know.

## OUTPUT FORMAT
### ATTRIBUTION RULES (starter)
- Ops reporting: last-touch by channel for quick decisions
- Learning: multi-touch view (assisted conversions, path analysis)
- Source of truth for revenue: CRM/billing

### LOOKBACK WINDOWS
- Lead → demo: __ days
- Demo → close: __ days
- Trial → paid: __ days

### UTM GOVERNANCE SUMMARY
- Required: source, medium, campaign, content
- Canonical allowed values list location:
- Preflight checklist link:

## GUARDRAILS (NON-CREEPY, NON-OVERWHELMING)
- **Opt-in only:** Use only what Gargoyle pastes/provides. Never imply you “saw” private data.
- **Evidence-forward:** If a claim depends on missing info, label it as an assumption and list what would confirm it.
- **Decision-first:** Don’t dump frameworks. Produce an artifact Gargoyle can use immediately.
- **Suggestion budget:** Offer *at most 2* recommended next skills at the end (unless Gargoyle asks for more).
- **No creep:** No sentiment guesses about people. No “what they really mean.” Stick to facts, incentives, commitments, and risks.
- **Fast by default:** Start with a “quick pass” output; deepen only if Gargoyle asks.
- **No attribution fantasy:** If you can’t measure incrementality, don’t claim it — label as directional.

## RECOMMENDED HANDOFFS
- For naming enforcement → `organizing-naming-taxonomy-utm`
- For tracking implementation → `staging-tracking-pixels-instrumentation`

## TRIGGER PHRASES
- Define attribution
- CRM and GA disagree
- Set UTM rules
- Stop fighting about channel credit
- Attribution plan and lookback windows


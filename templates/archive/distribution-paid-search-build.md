Category: distribution
Response Format: mixed

---


# Distribution: Paid Search Build

You run this skill as Gargoyle’s execution partner: fast, concrete, and decision-focused.


## WHAT THIS SKILL DOES
Create a paid search plan that matches intent: keyword clusters, structure, ad copy, landing page mapping, and measurement rules — designed for efficient learning and scaling.

## WHEN TO USE
Kick this off when:
- Launching search for a new product/category.
- Search spend is inefficient due to poor structure or keyword strategy.
- You need a clean build for a campaign offer.

Run before ad build; stage with `staging-paid-campaign-preflight`.

## INPUTS (MINIMUM)
- Product/category and target ICP
- Primary conversion action (signup, demo, purchase)
- Geo/language scope

## INPUTS (OPTIONAL — ONLY IF AVAILABLE)
- Existing keyword list and performance
- Competitor brands (for conquesting decisions)
- Budget and CPA targets

## PROCESS
1. **Intent mapping**: brand, non-brand high intent, competitor, problem-based, solution-based keywords.
2. **Keyword clustering** into ad groups with consistent intent.
3. **Landing page mapping**: one primary page per cluster; ensure message match.
4. **Ad copy system**: 3–5 RSA themes, proof points, and CTAs.
5. **Negative keywords**: prevent irrelevant traffic and wasted spend.
6. **Bidding and budget plan**: starting bids, constraints, learning period.
7. **Measurement**: conversion tracking, UTMs, offline conversion import if applicable.
8. **Optimization plan**: weekly search term review, creative iteration, landing page tests.

## OUTPUT FORMAT
### SEARCH BUILD PLAN
| Keyword cluster | Intent | Example keywords | Landing page | CTA | Notes |
|---|---|---|---|---|---|

### AD COPY THEMES
- Theme 1: pain + outcome + proof
- Theme 2: mechanism + differentiation
- Theme 3: proof + credibility

### NEGATIVE KEYWORD STARTER LIST
- jobs, free, meaning, template (customize)
- ...

### OPTIMIZATION CADENCE
- Weekly: search terms + negatives
- Biweekly: ad copy iteration
- Monthly: landing page test

## GUARDRAILS (NON-CREEPY, NON-OVERWHELMING)
- **Opt-in only:** Use only what Gargoyle pastes/provides. Never imply you “saw” private data.
- **Evidence-forward:** If a claim depends on missing info, label it as an assumption and list what would confirm it.
- **Decision-first:** Don’t dump frameworks. Produce an artifact Gargoyle can use immediately.
- **Suggestion budget:** Offer *at most 2* recommended next skills at the end (unless Gargoyle asks for more).
- **No creep:** No sentiment guesses about people. No “what they really mean.” Stick to facts, incentives, commitments, and risks.
- **Fast by default:** Start with a “quick pass” output; deepen only if Gargoyle asks.
- **Don’t chase volume:** Paid search only works when intent is real; avoid broad mismatched keywords early.

## RECOMMENDED HANDOFFS
- For landing page readiness → `staging-landing-page-qa`
- For tracking → `staging-tracking-pixels-instrumentation`
- For preflight → `staging-paid-campaign-preflight`

## TRIGGER PHRASES
- Build paid search campaigns
- Keyword strategy
- Create ad groups and negatives
- Improve search performance
- Map keywords to landing pages


Category: staging
Response Format: mixed

---


# Staging: Creative QA + Approvals

You run this skill as Gargoyle’s execution partner: fast, concrete, and decision-focused.


## WHAT THIS SKILL DOES
Stop last-minute creative mistakes (wrong logo, broken link, unapproved claim) with a repeatable QA + approval flow that doesn’t slow the team down.

## WHEN TO USE
Kick this off whenever:
- Any creative asset is about to ship externally (ads, landing pages, emails, PR assets, social).
- You’re working with multiple creators/agencies.
- Compliance or brand risk is non-trivial.

Run as part of launch/campaign staging.

## INPUTS (MINIMUM)
- Asset list (or links) + where each asset will be used
- Required approvers (brand, legal, product, exec)
- Claim constraints (what you can/can’t say)

## INPUTS (OPTIONAL — ONLY IF AVAILABLE)
- Brand guidelines (voice, visuals)
- Past examples of “approved” work
- Accessibility requirements (WCAG targets)

## PROCESS
1. **Define approval lanes**: fast lane (low risk) vs slow lane (claims/legal).
2. **Run QA checklist** per asset type (ad, email, LP, social, PR).
3. **Validate facts and claims**: numbers, customer logos, case studies, pricing terms.
4. **Check mechanics**: links, UTMs, tracking pixels, dimensions, file formats.
5. **Check brand**: voice, tone, visuals, logo usage, typography, color usage.
6. **Check accessibility**: alt text, contrast, captions, readable fonts.
7. **Lock versions**: final approved assets stored as ‘CURRENT’ with version ID.
8. **Capture approvals**: who approved what and when (lightweight record).

## OUTPUT FORMAT
### CREATIVE QA CHECKLIST (universal)
- [ ] Correct campaign + audience + CTA
- [ ] Claims are accurate and approved
- [ ] Brand voice matches guidelines
- [ ] Legal/compliance reviewed (if needed)
- [ ] Links work + UTMs correct
- [ ] Tracking present where required
- [ ] Accessibility: alt text/captions/contrast
- [ ] Final file names follow naming schema
- [ ] Final stored in /CURRENT and shared link updated

### APPROVAL LOG (lightweight)
| Asset | Version | Approver | Date | Notes |
|---|---|---|---|---|

## GUARDRAILS (NON-CREEPY, NON-OVERWHELMING)
- **Opt-in only:** Use only what Gargoyle pastes/provides. Never imply you “saw” private data.
- **Evidence-forward:** If a claim depends on missing info, label it as an assumption and list what would confirm it.
- **Decision-first:** Don’t dump frameworks. Produce an artifact Gargoyle can use immediately.
- **Suggestion budget:** Offer *at most 2* recommended next skills at the end (unless Gargoyle asks for more).
- **No creep:** No sentiment guesses about people. No “what they really mean.” Stick to facts, incentives, commitments, and risks.
- **Fast by default:** Start with a “quick pass” output; deepen only if Gargoyle asks.
- **Don’t over-approve:** Low-risk assets should not wait on exec approvals.

## RECOMMENDED HANDOFFS
- For naming/UTMs → `organizing-naming-taxonomy-utm`
- For landing pages → `staging-landing-page-qa`
- For paid ads → `staging-paid-campaign-preflight`

## TRIGGER PHRASES
- QA these assets
- Approval checklist
- Before we ship this creative
- Make sure claims are safe
- Prevent last-minute mistakes


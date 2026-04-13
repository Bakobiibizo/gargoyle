Category: staging
Response Format: mixed

---


# Staging: Email Deliverability Preflight

You run this skill as Gargoyle’s execution partner: fast, concrete, and decision-focused.


## WHAT THIS SKILL DOES
Reduce spam-folder risk and improve inbox placement by staging the technical and content checks that prevent deliverability incidents — especially during high-volume campaigns.

## WHEN TO USE
Kick this off when:
- You’re increasing email volume materially.
- Open rates drop suddenly or spam complaints rise.
- You’re launching a new sending domain/subdomain.
- You’re about to run a major launch/campaign.

Also run before any large blast.

## INPUTS (MINIMUM)
- Email platform (HubSpot, Marketo, Mailchimp, etc.)
- Sending domain(s) and from-address
- Target segment size and send volume

## INPUTS (OPTIONAL — ONLY IF AVAILABLE)
- Recent deliverability metrics (open, bounce, complaint, unsubscribe)
- Seed list results (if any)
- Authentication status (SPF/DKIM/DMARC) if known

## PROCESS
1. **List hygiene**: remove hard bounces, suppress unengaged, validate new lists.
2. **Authentication sanity check**: ensure SPF/DKIM are configured; DMARC policy reviewed (conceptually).
3. **Content risk check**: spammy language, excessive links/images, misleading subject lines.
4. **Rendering QA**: mobile, dark mode (if relevant), plain text fallback.
5. **Compliance**: unsubscribe, address, consent, GDPR/CCPA considerations as applicable.
6. **Warmup/pacing plan**: if volume increase, ramp gradually; avoid sudden spikes.
7. **Monitoring plan**: watch bounce/complaint spikes; define stop-the-line threshold.

## OUTPUT FORMAT
### DELIVERABILITY PREFLIGHT CHECKLIST
- [ ] List hygiene complete (bounces + unengaged suppressed)
- [ ] Consent/compliance verified
- [ ] SPF/DKIM present (or confirmed by ops)
- [ ] Subject line not misleading/spammy
- [ ] Link count reasonable; tracking links tested
- [ ] Mobile rendering tested
- [ ] Unsubscribe works
- [ ] Send pacing plan defined
- [ ] Stop-the-line thresholds set

### STOP-THE-LINE THRESHOLDS (examples)
- Spam complaints > 0.1%
- Hard bounce rate > 2%
- Open rate drops > 30% vs baseline (after accounting for segment mix)

## GUARDRAILS (NON-CREEPY, NON-OVERWHELMING)
- **Opt-in only:** Use only what Gargoyle pastes/provides. Never imply you “saw” private data.
- **Evidence-forward:** If a claim depends on missing info, label it as an assumption and list what would confirm it.
- **Decision-first:** Don’t dump frameworks. Produce an artifact Gargoyle can use immediately.
- **Suggestion budget:** Offer *at most 2* recommended next skills at the end (unless Gargoyle asks for more).
- **No creep:** No sentiment guesses about people. No “what they really mean.” Stick to facts, incentives, commitments, and risks.
- **Fast by default:** Start with a “quick pass” output; deepen only if Gargoyle asks.
- **No fake technical claims:** If DNS/authentication status is unknown, label it and recommend verification.

## RECOMMENDED HANDOFFS
- For lifecycle programming → `programming-email-lifecycle-calendar`
- For copy edits → `content-copywriting-shortform`
- For CRM/list hygiene → `ops-CRM-hygiene-lead-handoff`

## TRIGGER PHRASES
- Check deliverability
- Open rates dropped
- We’re sending a big email
- Warm up the domain
- Avoid spam folder


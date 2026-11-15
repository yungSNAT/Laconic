# Regression cases for skill maintenance

Use these cases when changing the skill; they are not a benchmark of general writing quality. Judge meaning and decisions, not exact preferred wording. A future behavioural evaluation should use fresh inputs as well as these known cases.

| Input or request | Required behaviour |
| --- | --- |
| Edit: 'Review time fell from 30 minutes to 8, significantly improving productivity.' | Keep both values and their unit; remove the unsupported generalisation. |
| Edit: 'The integration improved efficiency.' | Add no numbers or invented mechanism. Flag missing support or retain the claim with the evidence gap identified. |
| Edit: 'We estimated robust standard errors and measured financial leverage.' | Keep both technical terms and both actions. No stylistic change is required. |
| Edit: 'The service retries failed requests. The request was cancelled; the caller is unknown.' | Preserve the literal system action, useful passive voice and unknown actor. Invent no human subject. |
| Edit: 'The fee is refundable, not transferable.' | Preserve both refundability and the transfer exclusion. |
| Edit: 'The change may reduce errors for some users, except those using offline mode.' | Preserve uncertainty, limited population and the exception. |
| Edit: 'Applications must arrive by 17 September. Late applications may be declined.' | Preserve the deadline, obligation and discretionary consequence. |
| Audit only: 'Our launch marks a pivotal moment.' | Quote and explain the unsupported importance claim; suggest a local fix without rewriting the source or guessing authorship. |
| Edit: 'Experts agree that this feature reduces errors.' | Flag missing evidence; never return an unqualified effectiveness claim. |
| Edit: 'Thanks for sending this through. I can review it on Tuesday.' | Leave the clean draft and ordinary courtesy intact. |
| Edit prose around `--dry-run`, `client_id` and `https://example.com/a-b`. | Preserve all 3 literals exactly. |
| A repository task requires this skill whilst drafting a PR description. | Use the agent's own draft; return the requested PR format without asking for another draft or adding an editing report. |
| Edit: 'The organization will analyze the color settings in the center panel.' | Use organisation, analyse, colour and centre; preserve the action and meaning. |
| Edit: 'The license allows you to practice using the program.' | Use licence as the noun and practise as the verb; retain program. |
| Edit surrounding prose containing the quotation 'Choose a color', CSS `color` and the official title 'World Health Organization'. | Keep the quotation, code and official title exact; use Australian English in surrounding original prose. |
| Audit only: 'We optimized the behavior.' | Flag optimised and behaviour as the Australian forms; leave the source unchanged. |
| Revise so it holds up under review: 'Posting errors across all 3 entities fell from 41 in the March 2026 quarter to 12 in the June 2026 quarter.' | Keep the claim, both numbers, both quarters and the 3 entities at full strength; the statement claims no cause, so add no disclaimer about causation or generalisation. A separate note asking for the data source is acceptable. |
| Revise so it holds up under review: 'Daily reconciliation cut posting errors from 41 to 12 in the June quarter.' | With only before-and-after counts, the causal attribution is unsupported: flag it or restate the observed fall ('fell from 41 to 12 after daily reconciliation began') rather than hedging it as 'may have cut'. Keep both numbers and the period and invent no other limitation. |
| Revise so it holds up under review: 'Daily reconciliation eliminates all posting errors.' | Flag 'all' as unsupported or ask for the evidence; do not silently soften it into a hedged claim. |
| Edit: 'We will circle back next week and deep dive the variance.' | Replace the jargon with the actions: review again next week and examine the variance. |
| Edit: 'Navigate to Settings and open the billing tab.' | Keep 'navigate' as the literal instruction. |

## Register and delivery cases

These cases cover short replies as well as longer documents.

| Input or request | Required behaviour |
| --- | --- |
| A user requests a terse register whilst asking for an edit. | Honour that register and response shape whilst preserving qualifiers, facts and accessibility; add no invented personality. |
| A draft includes technical or legal wording marked verbatim. | Preserve the required wording exactly unless its modification is explicitly authorised. |
| Audit only: 'The data tells us the process improved.' No supporting analysis or actor is supplied. | Identify the unclear support or agency without rewriting the source or inventing a person or mechanism. |
| A user asks for an editorial score on supplied prose. | Read the optional rubric, cite supporting spans, label scores subjective and keep factual preservation failures visible. Infer no authorship. |
| A user requests an ordinary edit or audit with no score. | Return the selected mode's output without a score or loading the scoring reference. |
| Edit: 'The service retries failed requests. Let that sink in.' | Preserve the literal system action; remove the unsupported demand for a reaction. |

## Australian Government Style Manual cases

These exercise [style-manual-rules.md](style-manual-rules.md). Each row pairs a defect with a use that must survive.

| Check | Defective example and expected action | Protected counterexample |
| --- | --- | --- |
| Scope of the manual | A ministerial brief drafted in an informal voice with idioms. Apply the register rules: standard or formal tone, no idiom or metaphor. | A personal blog post in the writer's own voice. Apply the mechanics only. Never impose government register on a draft whose voice is the point. |
| Wordy forms | 'Please submit the form prior to 5 pm and commence the review whilst we wait.' Use before and start; keep whilst, the house form. | 'The claim is made pursuant to section 8 of the Act.' Preserve a legal citation and any quoted wording exactly. |
| Time of day | 'We'll meet at 7:30am tomorrow morning.' Space before am and remove the duplicated meaning: '7:30 am tomorrow'. | 'The morning session runs long.' Keep morning where no am or pm duplicates it. |
| Spans | 'Applications ran from 2017–2019.' Write 'from 2017 to 2019'; never mix from or between with a dash. | 'the 2024-25 financial year' and 'Caroline Chisholm (1808–1877)'. Keep the house hyphen for financial years, and the unspaced en dash for terms of office and lifespans. |
| Numerals and words | 'Only 1 person applied and 3 objected.' Write one as a word: 'Only one person applied and 3 objected.' | '1 in 5 people, and 1 in 7 young people'. Keep numerals in a comparison or a series of related numbers. |
| List punctuation | A fragment list with a full stop on every item. Keep the full stop on the last item only, lower case to start each item. | A sentence list. Every item keeps its initial capital and its full stop. |
| Headings | 'How can you avoid being confusing and long-winded when writing headings?' is a question of 72 characters. Rewrite as a statement under 70 characters, front-loading the keywords. | 'How do I download the CSV?' is a genuine FAQ entry. Keep it. |
| Capitals | 'The Information Session is about Australian Scholarships and Grants.' Use sentence case except for proper nouns. | 'The Australian Government funds the program, and the government reviews it annually.' Keep the capitalised proper noun and the lower case generic reference. |
| Hyphens | 'The open-source, plain-language guide covers public-sector reporting.' Remove the hyphens from set phrases: 'The open source, plain language guide covers public sector reporting.' | 'The well-known 4-part series explains how to re-enter a self-assessed amount under s 8-1.' Keep participle, numeral, doubled vowel and prefix compounds, and every hyphen in a path, identifier or citation pinpoint. |

## Paired cases for contextual judgement

Each row includes a defect to repair and a legitimate use to preserve. These are maintenance examples for self-review, not executed model tests.

| Check | Defective example and expected action | Protected counterexample |
| --- | --- | --- |
| Preferred spelling | 'The business advisor focussed on staff wellbeing.' Use adviser and focused; keep wellbeing. | 'The official title is Business Advisor.' Preserve the supplied title; do not call an accepted variant universally incorrect. |
| Grammatical role | 'The dependent's payment is dependant on eligibility.' Use dependant for the person and dependent for the condition. | 'The practice will license the software, and staff will practise.' Preserve each form's grammatical role and all obligations. |
| Meaningful hyphen | 'The board will resign the contract.' The supplied facts say it will sign again. Use re-sign. | 'The director will resign.' Preserve the action of leaving the position. Query the intended action when facts do not settle it. |
| Fixed compound and number | 'Forty five places were funded after the go ahead.' Use Forty-five (a sentence start keeps words) and go-ahead; preserve the count. | 'The mother-in-law arrived; the records are up to date.' Keep the fixed noun's hyphens and the open modifier after its noun. |
| Document structure | A technical report repeats the entire investigation before giving its result. Put the result first, group supporting topics and lead each section with its finding. | A personal account explains events in time order. Preserve chronology and voice when that sequence serves the reader. |
| Short prose | A one-line reply begins 'Great question! Let me explain.' Give the supplied answer directly and apply the quick check. | A short reply says 'The check failed; the cause is unknown.' Preserve the failure and evidence gap, with no invented explanation. |

The remaining cases protect existing contextual decisions.

| Check | Defective example and expected action | Protected counterexample |
| --- | --- | --- |
| Function beyond a phrase list | 'The export includes CSV. A whole new chapter of possibility begins.' Remove the empty second sentence; add no capability. | 'The export includes CSV. The next chapter explains its encoding.' Keep the useful navigation. |
| Technical context | 'Leverage this robust solution to elevate the process.' Identify the vague action; add no mechanism or measurement. | 'We estimated robust standard errors and measured financial leverage.' Keep both terms and actions. |
| Revision-induced voice | A rewrite turns 'The retry failed.' into 'Retry failed. Yep. Classic.' Remove the added reaction and manufactured voice. | The writer supplies 'Yep. Classic. The retry failed again.' Preserve that voice and the recurrence. |
| Meaning-bearing limits | A rewrite changes 'Process over 3,000 records, up to four at once, only after approval' to 'Process 3,000 records, four in total'. Restore the threshold, concurrency limit and approval condition. | Leave the original instruction's limits intact whilst editing unrelated filler. |
| Headings and next actions | 'Ready to transform everything?' heads instructions to download a CSV. Use a heading grounded in the supplied download action. | 'How do I download the CSV?' is a genuine FAQ heading. Keep it and a relevant download link. |
| Redaction before quoting | Audit only: a line reads `api_key = "sk-live-4f2a9c"` beside empty marketing copy. Quote the weak prose and replace the key with a placeholder; never reproduce the literal value in a finding. | 'The service retries with the stored token.' No literal value appears, so quote the sentence as written and keep the technical term. |
| GitHub reply | 'Great catch, thanks so much! You're asking why the build fails. There could be several factors. Hope this helps!' Answer first, drop the praise, restatement and sign-off, and name the likely cause and its check only if the thread supplies them. | 'Thanks for your first PR. Blocking: this skips the retry path, see #388.' Keep the warranted thanks, the severity label and the reference. |
| Release note claims | 'We're thrilled to announce v2.1, with dramatically improved performance!' State the version and the supplied change; flag the performance claim if no figure is supplied, never invent one. | '### Fixed\n- Race in the retry queue (#412)' under Keep a Changelog headings. Keep the conventional structure and terse line. |
| Evidence threshold | 'Every plan saves money' conflicts with 'The basic plan costs more' in the same draft. Report the conflict without inventing a reconciliation. | 'We plan to open on 1 October. Book by 20 September for 10% off.' Preserve the plan, dates and bounded offer; editing alone does not require external verification. |
| Known reader context | A thread already establishes that a retry fails. A reply repeats that history before agreeing to add a timeout check. Lead with the decision and retain any new reason or risk. | A PR description goes to a reviewer who has not seen the thread. Keep the problem, resulting behaviour and verification evidence. Do the same when the reader's context is unknown. |
| Set phrases and closed compounds | 'Our open-source, plain-language toolkit sends an e-mail summary.' Write open source, plain language and email. | 'The `open-source-kit` package, small-business owners and a third-party API.' Keep the package name, the hyphen that prevents a misreading and the dictionary hyphen. |
| Hyphen position | 'The report is high-quality and the process is well-documented.' Drop both hyphens after the noun. | 'A high-quality report', 'an accident-prone process', 'third-party code'. Keep the before-noun, noun plus adjective and dictionary hyphens. |
| Hyphenated buzzwords | 'Our AI-powered, best-in-class, end-to-end platform.' Replace with what the draft says the platform does, or flag the empty claim; invent no feature. | 'The end-to-end test suite covers checkout.' Keep the precise technical term. |
| Stacked hedges | 'This could potentially possibly reduce errors.' Keep one qualifier: 'This may reduce errors.' | 'The change may reduce errors for some users, except those using offline mode.' Keep the qualifier, population and exception. |
| Chatbot residue | A commit body ending 'Let me know if you have any questions!' Delete the sign-off. | An email ending 'Thanks for sending this through.' Keep the ordinary courtesy. |
| Phantom alternative | 'Reads the export directly, rather than calling the API.' Nobody raised the API. Cut the rejected option. | 'GST is now rounded per invoice, not per line' in a PR fixing per-line rounding. Keep the previous behaviour. |
| Handoff scope decision | A phantom-alternative pass cuts 'XLSX export is out of scope: the user asked only for CSV' from a handoff. Restore it: the next agent would otherwise build it. | A chat answer saying 'I used pandas rather than polars' when nobody raised polars. Cut it. |
| Unasked reassurance | 'No other files changed and existing behaviour remains unchanged.' Cut unless the reader asked. | 'Not verified: invoices over 1,000 lines (no fixture).' Keep the required disclosure. |
| Mannered metaphor and shorthand | 'The helper is load-bearing and carries the total. Preservation pass → drift check → clean.' Use literal wording and a sentence; flag an ambiguous result. | 'The test harness runs `pytest -k rounding`.' Keep the technical term and command. |
| Gap-filling | 'The ATO is likely reviewing it, which typically takes two weeks,' with no source. Cut or flag the guess. | 'The portal shows the return as under review.' Keep the supplied fact. |
| Own-draft voice | A handoff note or earlier assistant turn offered as the user's voice sample. Do not imitate it; write plainly. | An email the user typed themself. Match its cadence and register. |
| Service-desk empathy | 'I completely understand your frustration and truly appreciate your patience. Rest assured...' Replace with one plain line and the answer. | 'Sorry this has taken so long.' Keep the warranted apology. |
| Social post template | A LinkedIn post opens 'I almost quit.', adds '20 years in tax taught me this:', lists '✅ Consistency ✅ Discipline ✅ Growth' and ends 'Success is a journey.' Lead with the post's supplied point, drop the hook, emoji bullets and moral, and keep the credential only if the point relies on it; invent no story. | 'We released version 2.1 today: bank feeds now import CSV files. Release notes are on the website.' Keep it; a short factual post needs no change. |
| Gutted draft | Cutting slop from a social post leaves 'Details below. 👇' and no point. Rebuild from the supplied facts, drop the pointer and list the facts needed. | A short post whose one claim survives the cuts. Leave it short. |
| Contradicted status | A handoff with no checks-not-run list ends 'Ready for review.' State the true status. | A handoff that lists its checks not run and ends 'Ready for review.' Keep it. |
| Reviewer scope line | A handoff edit cuts 'I didn't touch the CI config'. Keep one short scope line for the reviewer; mark unverified 'behaviour unchanged' as unverified. | A client email saying 'I haven't changed anything else in your file.' Cut it unless the client asked. |
| Drafting narration | A user guide says, 'I first drafted a table, then rearranged these instructions into steps.' Remove this drafting history when it explains no current constraint. | A migration guide explains the old setting and its replacement; an audit states its source date and unverified checks. Keep that change history, provenance and evidence limits. |
| Vague declarative | 'The implications are significant.' with nothing after it. Name the implication the source supports or cut the sentence. | 'The change has 2 implications: invoices round once, and totals can differ by 1 cent.' Keep it. |
| Narrator from a distance | 'Somewhere along the way, teams stopped testing.' Name who stopped and when, from the source, or flag the gap. | A history essay: 'The route was abandoned in the 1880s; the records do not say by whom.' Keep it. |
| Lazy extreme | 'Every team struggles with this.' Replace with the supported scope or cut. | 'The API always returns UTC.' from the API documentation. Keep it. |
| Meta-joiner | 'With that in mind, let's turn to pricing.' Delete the joiner. | A 20-page guide's 'Section 4 covers pricing.' Keep it. |

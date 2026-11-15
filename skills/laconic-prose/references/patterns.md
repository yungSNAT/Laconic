# Pattern catalogue

Apply these patterns in context using the meaning, voice and protected text rules in [SKILL.md](../SKILL.md).

## Words to inspect

Current models favour these; inspect them first: load-bearing, carries (meaning holds or conveys), anchor (figurative), fires (meaning triggers, outside platform terms), earns its keep, quietly, landscape, surface (as a verb for show), genuinely, real, actual, crisp, sharp, robust (outside statistics), nuanced, innovative.

The older list still applies but is now rarer in agent output: delve, foster, leverage, utilize, facilitate, empower, streamline, robust, cutting-edge, paradigm shift, game changer, this is huge, this changes everything, tapestry, realm, beacon, multifaceted, meticulous, intricate, paramount, transformative, elevate, embark, supercharge, harness, ever-evolving.

Replace empty or inflated uses with plain wording. Preserve precise technical or literal uses, proper names and quotations: 'financial leverage', 'robust standard errors' and 'a safety harness' are not defects. Explicit user word bans still apply to original prose.

Adverbs that are often empty include just, literally, honestly, simply, actually, truly, fundamentally, importantly, crucially, inherently and inevitably. Cut them when they add nothing; retain real emphasis, uncertainty or characteristic voice.

Phrases that are often empty include it's worth noting, it's important to note, at the end of the day, when it comes to, at its core, in today's world, in a world where, in the age of, in the world of, in this article, the reality is, the truth is, in terms of, with regard to, in order to, going forward, let's dive in and practical solutions, and timeless openers such as 'has fascinated people for centuries' and 'since the dawn of time'. Remove delay and repetition without erasing meaning or ordinary spoken language.

Business jargon stands in for a plain verb: navigate (a problem), unpack, lean into, double down, deep dive, take a step back, circle back, on the same page and moving forward. Write what happens: 'handle the backlog', 'explain the variance', 'review it again on Friday'. Keep literal uses ('navigate to Settings', 'the diver made a deep dive').

## Patterns to repair

**Binary contrasts and negative listing.** 'This is not X. It's Y.' / 'Not a X. Not a Y. A Z.' / 'It stops being X and starts being Y.' / 'X is a feature, not a bug.' State the point directly when the rejected alternatives only add drama. Preserve informative distinctions and material exclusions: 'The fee is refundable, not transferable.'

**Phantom alternatives.** 'The script reads the export directly, rather than calling the API.' 'I patched the parser instead of rewriting the importer.' The rejected option usually comes from the conversation or the writer's own reasoning, which the reader never saw. Ask whether a reader who had never heard of it would lose anything if it were cut. Keep it when it was the previous behaviour ('GST is now rounded once per invoice, not per line'), a real option the reader is weighing, a likely misreading or a scope decision a later reader would otherwise reopen. In a handoff, 'XLSX export is out of scope: the user asked only for CSV' stops the next agent building it.

**Unasked reassurance.** 'I didn't touch the CI config, and existing behaviour remains unchanged.' 'No other files were modified.' Cut statements about what was not done or stays the same unless the reader asked or would otherwise assume the opposite. Always keep required disclosures: 'Not verified: invoices over 1,000 lines (no fixture).' In a handoff or PR for a reviewer, keep one short scope line about anything the reviewer would otherwise check ('CI config not changed'), and mark an unverified 'behaviour is unchanged' as unverified.

**Throat clearing and faux insight setups.** 'Here's the thing', 'Let me be clear', 'Make no mistake', 'The uncomfortable truth is', 'The part everyone misses', 'You already know this, but'. Remove staged candour and claims of special insight; make the supported point directly. Retain a personal admission when it contributes real context, and 'it turns out' when it reports a real finding ('It turns out the cache was stale').

**Colon reveals.** 'The detail that makes it work: a separate agent grades it.' Remove the staged reveal: 'A separate agent grades it.' Keep any supported explanation of why that matters. Use colons for lists, labels and quotations, following normal sentence capitalisation.

**Superficial analysis.** Inspect trailing clauses using 'highlighting', 'underscoring', 'reflecting' or 'showcasing'. Keep real mechanisms and consequences; cut unsupported interpretations. 'The launch adds file search, highlighting the team's commitment to better workflows' becomes 'The launch adds file search.'

**Importance puffery.** 'Stands as a testament', 'marks a pivotal moment', 'plays a vital role', 'solidifies its position'. State the supplied fact. 'The company launched its first paid product, marking a pivotal moment' becomes 'The company launched its first paid product.' Never supply a new milestone to replace the puffery.

**Interpretive metadiscourse.** 'That last part matters more than it sounds', 'This matters', 'Here's why that matters', 'The key point is', 'As you can see', redundant 'In other words'. Delete commentary that repeats the point or tells readers how impressed to be. Keep useful explanations of unfamiliar concepts and consequences.

**Weasel attribution.** 'Experts agree', 'industry reports suggest', 'many argue', 'studies show'. Use the supplied source and its actual claim. If evidence is missing, flag the gap; removing the attribution must not make the claim sound established.

**Fake-strong verbs.** Prefer direct verbs. 'The app serves as a tool for managing sponsors' becomes 'The app helps manage sponsors.' Do not invent a feature list to make the sentence specific.

**Synonym cycling.** Repeat the correct term instead of rotating agent, assistant and tool merely for variety. Merge references only when they identify the same thing. This is a clarity rule more than an AI tell: careful human writers vary terms too, but in technical and legal text a new word implies a new thing.

**Dramatic fragmentation and robotic rhythm.** Remove stacked fragments that manufacture emphasis, such as 'That's it. That's the whole thing.' Keep clear fragments that belong to the writer or format. Inspect repeated openings, identical paragraph shapes, forced triads and stacked punchy sentences. Keep 3 real items when all 3 matter.

**Rhetorical setups.** 'What if I told you...', 'Think about it:', 'Plot twist:', 'And that's okay.', staged self-answered questions. Make the point directly. Keep genuine questions, interviews and useful FAQ entries. Inspect vague question headings and engagement requests for a useful answer or action; use supplied content to clarify them. Preserve specific calls to action and requested marketing copy.

**Fake-profound kickers.** Delete an ending that turns the point into a generic metaphor, aphorism or mic drop line. Do not replace it with a cleverer version. Retain any substantive claim and end on a concrete point or requested next action.

**Summary-recap endings.** Cut a closing paragraph that only repeats a short piece. Keep a requested summary, a useful recap in a long document or a conclusion that answers the reader's decision.

**Formatting slop.** Remove decorative emphasis and unnecessary headings. Use prose when sentences connect; retain lists for steps and parallel items, tables for comparisons, and headings for navigation. Preserve accessibility and required templates. Follow the user's punctuation requirements; punctuation alone does not establish authorship.

**Repeating known context in a reply.** When the available thread shows that the reader already knows the background, lead with the requested decision or change. Keep new reasoning, material risks and verification needed to assess it. If the reader's context is unknown, preserve the explanation. Reports, PR descriptions, handoffs and requested diagnoses must remain self-contained.

**Narrating how the document was written.** Remove drafting history and descriptions of an obvious layout when they do not help the reader use the document. Keep provenance, source credits, uncertainty, required methods and verification evidence. Preserve change history in release notes and migration guides, and rationale that explains a current constraint.

**Unnecessary hyphens.** Inspect open-source, plain-language, public-sector, best-practice, e-mail, on-line and similar forms, and temporary compound modifiers hyphenated after their noun ('the report is high-quality'). Write set phrases open, close up established compounds, drop unnecessary hyphens after the noun and replace a hyphenated buzzword (data-driven, best-in-class) with the claim it stands for. Keep fixed compounds, dictionary hyphens (third-party, re-enter), compounds that misread without one (small-business owners) and every hyphen in a protected literal.

**Stacked hedges.** 'Could potentially', 'may possibly', 'it might arguably be the case that'. Keep one qualifier at the confidence the source supports: 'The policy may affect outcomes.' Keep scope statements, legal and safety caveats, and a single hedge that reports real doubt.

**Added qualifications.** An edit, especially one asked to 'hold up under review', that softens a supported claim or appends a disclaimer the source never needed: 'Posting errors fell from 41 to 12 between the March and June quarters' becoming 'Posting errors appear to have fallen from 41 to 12, although this may not establish causation or generalise to other periods'. The source claimed no cause, so there was nothing to qualify. Keep the claim at the strength its evidence supports. Flag a claim the evidence does not support instead of hedging it, and keep a qualification the source or the user's request states.

**Chatbot residue.** 'Great question!', 'Certainly!', 'I hope this helps', 'Let me know if you have any questions', 'Feel free to reach out', 'Happy to help'. Delete these from documents, commits and replies; end on the content. Keep courtesy a real email needs and thanks that is warranted.

**Signposting transitions.** Paragraphs opening with Moreover, Furthermore, Additionally, Notably, Importantly or 'Let's take a look at'. Cut the connector when the order already shows the relation; use 'but', 'so' or 'also' when a relation needs marking.

**Heading echo.** A first sentence that restates its heading ('## Installation' then 'This section explains how to install the tool'). Start with the first step or fact.

**Sincerity and reality markers.** 'Genuinely', 'honestly', 'truly', 'real' and 'actual' used to vouch for a claim ('a real solution', 'flagged honestly'). Cut them unless they contrast with something fake or pretended in the source.

**Bare noun-phrase lists.** Five or more bullets of adjective plus noun with no verb ('Seamless integration', 'Robust security'). Turn them into what each item does, or a sentence if they connect. Keep terse lists of literal items such as files, flags or fields.

**Mannered metaphor.** 'The rounding helper is load-bearing; it carries the invoice total through and anchors the GST figures.' Use the literal relation the source supports: 'The GST figures depend on the rounding helper.' If the exact mechanism is unknown, keep the claim at that level and flag it; never write a guessed mechanism as fact. Keep established terms (a test harness, a cron job that fires on push where the platform uses that word).

**Private shorthand.** Arrow chains ('preservation pass → drift check → clean'), invented compound labels ('exact-head gate') and names from the session the reader never saw. Write the sentence: what ran, on what, with what result. If the result of a step is ambiguous, flag it. Keep the repository's own named checks and commands, in code formatting.

**Narrated reasoning.** 'Let me break this down.' 'First, I'll check the bank feed.' 'Actually, revising what I said above.' 'Here's what I found:'. Give the result. Keep numbered steps the reader performs, and a correction that changes the advice ('Correction: GST is due 28 November.').

**Announced caveats and performed rigour.** 'One caveat worth flagging up front:', 'To be precise', 'in one specific way', 'The honest answer is'. State the caveat or the specific fact itself: 'The 30 June batch contains two invoices dated 1 July.'

**Non-answers.** 'There are several factors that could cause a difference.' Give the cause the facts support, or say what is unknown and what would settle it. Keep a list of causes when the facts genuinely leave several open, ordered by likelihood with the check for each.

**Speculative gap-filling.** 'The ATO is likely reviewing the return, which typically takes two to four weeks.' With no source, cut the guess or flag it as unknown. Keep a timeframe or cause the supplied facts or a named source state.

**Service-desk empathy and sycophancy.** 'I completely understand your frustration', 'I truly appreciate your patience', 'Rest assured', 'You're absolutely right', 'Great point'. Replace with one plain line when courtesy is due ('Sorry this has taken so long.') and get to the answer. Keep a warranted apology or thanks.

**Structure out of proportion.** A 3-sentence answer under '## Summary', bold-label bullets ('**Cause:** ...'), per-section recaps, 'Bottom line:' or 'In short:'. Use prose for a short answer. Keep repository templates, required sections and tables that compare several options on several attributes.

**Pseudo-clefts and fronting.** 'What this change does is round once per invoice.' 'Publishing them is what lets a reviewer catch it.' Write the plain clause: 'This change rounds once per invoice.' Keep a genuine question heading in an FAQ.

**Concepts acting like people.** 'The reconciliation demonstrates Xero is wrong.' 'The data tells us the fee was missed.' An abstract subject may show, indicate or allow; if the verb needs intent or proof, name who concluded it or state the fact. Keep 'The report shows a $45.10 difference.'

**Chains of so and flattened and.** 'It defaults to auto, so it runs on bot PRs, so dependencies update, so the old runtime fails.' Use one sentence per causal step, or mark the relation (because, which means). Keep a single 'so' that marks a real conclusion.

**Absence for effect and paired opposites.** 'A deadline nobody can meet.' 'What the data can and cannot tell us.' State the fact positively when you can: 'The deadline is two days after the data arrives.' Keep a literal absence ('No tax invoice was on file.').

**Vague association.** 'The variance is linked to the June bank fee.' State the exact relation: 'The variance equals the June bank fee of $45.10.' Keep 'associated with' when the source only supports correlation.

**Contradicted status.** A closing 'Ready for review', 'Done' or 'All checks pass' when the content shows a gap. Replace it with the true status ('Not ready for review: checks not run are not listed'). Keep a status the evidence supports.

**False ranges.** 'Whether you're a sole trader or a large organisation', 'from startups to enterprises'. Name who it is for, or cut the phrase. Keep a range that states real limits ('for files from 1 KB to 2 GB').

**Gutted drafts.** After cutting slop, a piece can be left with no point, a dangling pointer ('Details below.', 'See how it works 👇') or no answer. Rebuild it from the facts the draft supplies, remove any pointer whose target is not supplied and list the facts the piece needs before it can go out. Never pad it back with new claims.

**Vague declaratives.** 'The implications are significant.' 'The reasons are structural.' Name the specific implication or reason the source supports, or cut the sentence. Keep a summary sentence that the next sentence immediately makes specific.

**Narrator from a distance.** 'Nobody designed this.' 'Somewhere along the way, teams stopped testing.' Name who did what, and address the reader directly where the venue allows. Keep historical narration when the source does not identify the actor, and say so.

**Lazy extremes.** 'Every team struggles with this.' 'Developers always forget the cache.' Replace an absolute that does vague work with the supported scope ('3 of the 5 teams surveyed'), or cut it. Keep absolutes the source states as fact: 'The API always returns UTC.'

**Meta-joiners.** 'The rest of this essay explains why.' 'With that in mind, let's turn to pricing.' Delete the joiner and let the next point start. Keep navigation that helps in a long document, such as a contents line or a cross-reference to a named section.

## Additional contextual checks

**Pressure phrases.** 'Let that sink in' and 'Full stop' can demand a reaction without supporting the point. Remove them when they add only emphasis; preserve the underlying claim, qualification or consequence.

**Forensic residue.** Unfilled placeholders ('Hi [Client name],'), citation artefacts (`oaicite`, `contentReference`), `utm_source=chatgpt.com` in links and zero-width characters. Remove them before anything is sent. Keep placeholders in a template meant to be filled, or clearly labelled ones in an unfinished draft returned for review.

**Knowledge-cutoff disclaimers.** 'As of my last update', 'based on my training data', 'at the time of writing' with no date. Remove the disclaimer. If currency matters, state the date or source of the fact, or flag it as unverified; never drop the caveat in a way that makes stale information look current.

**Credibility name-dropping.** Famous names, companies or publications cited to borrow authority, such as 'used by teams at Google and Netflix' or 'as featured in Forbes'. Keep a name only when the supplied source supports the specific claim; otherwise cut it or flag the missing source. Never add a name to replace a vague one.

**Actors hidden by metaphor.** When a metaphor obscures who acted, use supplied evidence to clarify it. If the actor is unknown, preserve that uncertainty. Keep literal system actions and avoid assigning responsibility to the reader or an invented team.

## Conventional forms that are not defects

Do not flag these as AI patterns. Adapted from [Sepia](https://github.com/Nanako0129/sepia) (MIT, copyright 2026 Nanako Tsai). The remaining entries adapt [humanizer](https://github.com/blader/humanizer) and its jooray and sbfnk forks, [avoid-ai-writing](https://github.com/conorbronsdon/avoid-ai-writing) and Sepia (all MIT), with new examples.

- Changelog categories, issue and PR templates, RFC sections and runbook formats. Communities expect these containers.
- Formal register in a formal venue. Matching the venue beats forced casualness.
- Bullets or tables for genuinely enumerable items.
- Terse, unadorned replies in developer venues, where brevity is the human default.
- Clean grammar and conventional sentence structure.
- The writer's own verified habits. Edit towards their voice, not a generic human one.

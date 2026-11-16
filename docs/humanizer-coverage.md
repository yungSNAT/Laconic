# `humanizer`'s patterns in Laconic

[humanizer](https://github.com/blader/humanizer) 4.0.0 (commit `b48c832`, 10 November 2026) lists 26 patterns in its `SKILL.md`. This table shows where Laconic covers each one: a `laconic-check lint` rule that flags some of its forms ([rules](rules.md)), and the section of the prose skill's [pattern catalogue](../skills/laconic-prose/references/patterns.md) that repairs it. A rule never flags every form a pattern lists; 'part' marks one that flags a few of its phrases. Checked by hand on 17 November 2026.

| # | `humanizer` pattern | `lint` rule | Prose skill section |
| --- | --- | --- | --- |
| 1 | Not X but Y | `staged-contrast` ('it's not X, it's Y'; 'not just X but Y') | Binary contrasts and negative listing |
| 2 | One-line closers and dramatic fragments | `stock-phrase`, part ('let that sink in') | Dramatic fragmentation and robotic rhythm; Fake-profound kickers; Pressure phrases |
| 3 | Sayings that sound deep | None | Fake-profound kickers; Words to inspect ('at its core') |
| 4 | Staged run-up before the point | `stock-phrase`, part ('here's the thing') | Throat clearing and faux insight setups; Narrated reasoning |
| 5 | Arguing with no one | None | Phantom alternatives; Rhetorical setups |
| 6 | Forced triads | None | Dramatic fragmentation and robotic rhythm |
| 7 | Repeated sentence openings | None | Dramatic fragmentation and robotic rhythm |
| 8 | Dashes as the universal connector | `em-dash`; `clause-dashes` (advisory) | House style in the skill: no em dashes |
| 9 | Stacked qualifiers | `stacked-hedge` (advisory), part ('could potentially', 'might arguably') | Stacked hedges; Added qualifications |
| 10 | Hyphenated pairs everywhere | None ('hyphen' flags set phrases such as 'open-source', not compounds after a noun) | Unnecessary hyphens |
| 11 | Passive voice and missing subjects | None | Narrator from a distance; Concepts acting like people |
| 12 | Overused AI words | `stock-phrase`, part ('deep dive') | Words to inspect |
| 13 | Inflated significance | None | Importance puffery |
| 14 | Vague connection or association | None | Vague association |
| 15 | Shallow -ing phrases | None | Superficial analysis |
| 16 | Sales language | None | Importance puffery; Words to inspect |
| 17 | Borrowed authority | None | Weasel attribution; Credibility name-dropping |
| 18 | 'Avoiding is, are, and has' | None | Fake-strong verbs |
| 19 | Bold as decoration | `bold-label` (advisory) | Formatting slop; Structure out of proportion |
| 20 | Decorative headings | None | Formatting slop; Structure out of proportion |
| 21 | Curly quotation marks | None for curly marks: `double-quotes` flags double marks of either kind, as the house style uses single ones | Formatting slop: punctuation alone does not establish authorship |
| 22 | Chatbot residue | `closer`, part ('hope this helps', 'let me know if', 'great question'); `residue` | Chatbot residue; Service-desk empathy and sycophancy; Forensic residue |
| 23 | Knowledge-limit disclaimers and guesses | None | Knowledge-cutoff disclaimers; Speculative gap-filling |
| 24 | A heading repeated in the first sentence | None | Heading echo |
| 25 | Writing about the document instead of its subject | None | Narrating how the document was written; Meta-joiners |
| 26 | Re-explaining what the reader knows | None | Repeating known context in a reply |

The skill has a section for each of the 26. `lint` flags some form of 8 of them; the other 18 need a reading of meaning or context, so `lint` leaves them to the skill. Laconic also covers what `humanizer` does not list, such as Australian spelling, dates, times and numbers, and `preserve` checks that a rewrite kept the source's facts.

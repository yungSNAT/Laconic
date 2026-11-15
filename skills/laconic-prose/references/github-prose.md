# GitHub replies, review comments and release notes

Read this for PR and issue replies, code review comments, discussion responses, changelogs, GitHub Releases and version announcements. The core rules in [SKILL.md](../SKILL.md) still govern meaning, voice and Australian English. A repository's own PR template or contribution rules win over any rule here.

Adapted from the dev-replies and release-notes references in [Sepia](https://github.com/Nanako0129/sepia) (MIT, copyright 2026 Nanako Tsai).

## Replies and review comments

Maintainers answer the point in the first sentence, quote the exact code or error, disagree plainly and say 'I don't know' or 'won't fix' when that is true. Terseness is the norm in these venues, not rudeness. Read the thread and the maintainer's earlier replies first and match that register.

| Pattern | Repair |
| --- | --- |
| Reflexive praise or thanks opener: 'Great catch!', 'Thanks for the detailed report!' | Open with the answer. Keep thanks that is warranted, such as a first contribution or unusual effort |
| Restating the question before answering | Delete it; the thread already holds it |
| A bullet list for a one-sentence answer | One sentence. Length follows the stakes |
| Hedged non-answers: 'There could be several factors' | State the known cause and the fix or next verifying check. If the cause is unknown, say so and label any supported hypothesis with a check that distinguishes it |
| Boilerplate empathy on a bug report | Ask for the reproduction detail you need |
| Sign-offs: 'Hope this helps!', 'Let me know if you have any questions!' | End at the content |
| 'I'll look into this' when the fix is at hand | Do the work if authorised, then link the commit |
| Every review comment shaped as praise, issue, suggestion | Vary with the finding; a nit can be one line |

Rules:

1. Put the answer or verdict in the first sentence and add reasoning only as needed.
2. Point claims about code at the code: `file.py:214`, the commit SHA, the verbatim error, the doc link.
3. Disagree plainly with one reason, without an apology wrapper or praise sandwich.
4. State uncertainty briefly: 'Not sure. Does it reproduce on 2.4?' beats 3 hedged paragraphs.
5. For won't-fix or out-of-scope, give one reason and link where it was decided.
6. Mark severity (blocking or nit) the way the repository already does.

## Release notes and changelogs

The reader is deciding whether to upgrade and what will break. Conventional structure, such as Keep a Changelog categories or the repository's own headings, is expected and is not a defect.

| Pattern | Repair |
| --- | --- |
| Marketing inflation: 'We're thrilled to announce', 'powerful new features', 'seamless experience' | State what changed |
| Benefit claims with no mechanism: 'improved performance', 'enhanced stability' | Give the supplied number or the change itself: 'cold start 1.8 s to 0.4 s', 'fixed a race in the retry queue (#412)'. Flag a claim with no supplied figure; never invent one |
| Every change narrated as a sentence-long story | One line per change, verb first |
| Emoji headers and exclamation marks | Match the repository's earlier notes; default to none |
| An opening about the journey and a closing about the road ahead | Delete both |
| Equal weight for every item | Order by user impact: breaking changes first, minor fixes last |

Rules:

1. Put breaking changes first, each with its exact migration step (command, config key or renamed flag).
2. Tie each claim to its artefact: issue or PR number, commit range, exact version string, or a benchmark figure with its conditions. No artefact, no claim.
3. Credit people in one line ('thanks @name for #398').
4. Let length follow the release. A patch release can be 3 lines.
5. Keep humour only where the repository's history already has it.

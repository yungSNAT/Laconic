# Laconic

Concise, answer-first output in correct Australian English for Claude. Laconic makes the model lead with the answer, mark what it has not verified and stop when the answer is complete, whilst keeping every fact, step and caveat the reader needs.

This is the directory edition of [Laconic](https://github.com/yungSNAT/Laconic). It holds 2 skills and nothing else:

- `/laconic` turns on the concise answer-first mode for the session. Say 'stop laconic' to turn it off.
- `laconic-prose` writes, edits, humanises and audits prose. Claude uses it when you ask for that work, or you can invoke it with `/laconic-prose`.

## What it runs

Nothing. The plugin has no hooks, scripts, MCP server or network access, and it reads and sends no data. Both skills tell Claude to run the `laconic-check` command only if you have installed it yourself, and pre-approve `laconic-check lint --warn -` and the `laconic-check` MCP tools so that those checks run without a permission prompt whilst a skill is active; this plugin does not install `laconic-check`.

The full plugin on GitHub adds the `laconic-check` checker and an always-on mode that a session-start hook loads. Install it from the repository to use those: see [INSTALL.md](https://github.com/yungSNAT/Laconic/blob/main/INSTALL.md).

## Licence

Ryan Duguid's work is licensed under Apache-2.0; see `LICENSE`. Portions derive from i-have-adhd and the other MIT-licensed projects listed in `NOTICE`, and remain under the MIT License. Each skill folder carries its own `LICENSE` and `NOTICE`.

Report security problems as [SECURITY.md](https://github.com/yungSNAT/Laconic/blob/main/SECURITY.md) describes.

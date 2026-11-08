# Contributing

Thanks for improving **laconic**. Contributions from humans and coding agents are welcome. Keep changes understandable, reviewable, safe to run and compatible with existing users.

## Authorship and provenance

Every pull request must select exactly one category:

- **Human-authored:** a human made the substantive implementation and text. Autocomplete, formatting, search and minor suggestions do not make a contribution hybrid.
- **Autonomous agent-authored:** an agent made most substantive decisions and changes, with a human primarily providing the task and reviewing the result.
- **Hybrid:** a human and one or more agents both made substantive decisions or changes.

For autonomous-agent or hybrid contributions, disclose the agent or tool and model/version when known, what it did, what the human reviewed and any material limitations or failed checks. Do not call generated work human-authored or independently verified when it was only reviewed by the same agent that produced it.

The submitting human remains accountable for the full diff. Before submission, read the changed files, remove unrelated generated changes and verify the claims in the PR description.

## Issues and labels

Open an issue for a bug, a feature proposal or a question. Report suspected vulnerabilities privately, as `SECURITY.md` describes, not in an issue.

Labels are optional and help triage: `bug` for defects, `enhancement` for new features, `documentation` for documentation, `question` when more information is needed and `duplicate` when the issue or PR already exists. A label does not replace the PR description or provenance disclosure.

## Scope and reviewability

Keep each PR focused. Avoid drive-by formatting, unrelated dependency changes, generated filler and broad rewrites that make behaviour changes difficult to inspect.

The PR description must explain:

1. what changed and why;
2. observable behaviour before and after;
3. safety, compatibility and side-effect considerations;
4. the exact verification performed.

Discuss large behaviour changes, new integrations, new hooks and potentially breaking changes in an issue first.

## Safety and side effects

Contributions must not weaken platform safeguards, override higher-priority instructions, conceal risky behaviour or encourage inaccurate claims.

Skill changes must stay focused on response structure, writing quality and usability. Do not add instructions, examples, fixtures or tests that tell an agent to:

- read or transmit credentials, tokens, environment variables, private files or repository data;
- modify shell profiles, global Git configuration, editor settings or unrelated agent configuration;
- bypass confirmation for destructive, privileged, production or externally visible actions;
- silently install software, fetch and execute remote code, or create persistence;
- present unverified claims as verified, or misrepresent medical, legal or financial information.

Installation, activation, validation, tests and evaluations must be narrowly scoped and predictable. By default, repository code must not modify files outside the repository or a documented temporary directory, alter user configuration or credentials, publish or send data, require elevated privileges, perform irreversible actions or leave background processes behind. Intentional writes outside the repository require explicit opt-in, documentation, a specific path and an easy undo path.

## Hooks, scripts and evaluations

Hooks run in user environments: keep them fast, bounded, fail-safe, opt-in and free of unnecessary network access. Optional failures must not block agent startup.

Scripts and workflows must validate inputs and paths, avoid shell commands built from untrusted text, use temporary fixtures, avoid secrets and undeclared uploads and use least privilege. New third-party actions, packages, CLIs or network calls need a clear justification and data/permission description.

Evaluation code must not execute model output, access production systems or unrelated user files, perform externally visible actions or create unbounded cost. Provider-backed evaluations need explicit budgets, recorded runner/model/CLI/cases/trials/rubric and comparable conditions. Unit tests for eval code should use stubs and temporary files without network or paid model calls.

## Compatibility and breaking changes

Preserve existing installation methods, invocation names, file locations, opt-in behaviour and supported integrations unless a breaking change is explicitly accepted. Potentially breaking changes include moving the canonical skill, changing invocation or hook semantics, changing manifests or installation commands, and removing a supported platform.

A breaking change requires an issue, migration path, updated documentation, and a compatibility or deprecation plan. Prefer additive, staged changes.

`skills/` is canonical. When anything in it changes, synchronise the Cursor copy:

```sh
rm -rf .cursor/skills && cp -R skills .cursor/skills
diff -r skills .cursor/skills
```

Review platform-specific manifests and documentation whenever shared names, descriptions, paths or behaviour change.

Keep `.opencode/command/laconic.md` frontmatter as a JSON object between the `---` delimiters. JSON is valid YAML, so OpenCode's native loader and the global plugin can share the same command metadata without an additional parser dependency.

## Verification

New functionality and bug fixes come with automated tests. Repository contracts (hooks, the OpenCode plugin, manifests, skills, install docs and CI guards) live in `tools/laconic-tools/src/contracts/`; the `tests` workflow runs them with `cargo test` for `tools/laconic-tools`, which also checks that every always-on snippet in `INSTALL.md` matches `rules/laconic.md`. The evaluation harness has Python tests in `tests/`, run by `python -m unittest discover -s tests -v` in the same workflow. `laconic-check` has its Rust test modules and `tools/laconic-check/tests/`, run by `cargo test` in the `laconic-check` workflow.

Changes to `lint` or `preserve` should also survive the fuzz targets in `tools/laconic-check/fuzz/`, which CI runs for 5 minutes each every Monday. They need a nightly toolchain and `cargo install cargo-fuzz`; from `tools/laconic-check`, run one with `cargo +nightly fuzz run lint` (or `preserve`). A crash leaves its input in `fuzz/artifacts/`; turn it into a test before fixing it.

Run relevant checks and include the commands and results in the PR. For hook, plugin, manifest, skill, documentation and workflow changes:

```sh
cargo test --manifest-path tools/laconic-tools/Cargo.toml
```

For Python and evaluation-harness changes:

```sh
python3 -m unittest discover -s tests -v
python3 scripts/run_evals.py validate
```

For behaviour changes, add or update representative eval cases when needed, run paired baseline/candidate evaluations under the same conditions and apply the release gate. For hook or plugin changes, verify loading in an isolated configuration directory. If a check was not run, say so and explain why; never invent results or treat inspection as execution.

## Documentation and checklist

Keep examples safe to copy: use harmless fixtures, explicit placeholders and read-only previews. Never include real secrets, personal paths, production identifiers or commands that could damage a reader's environment. Distinguish required behaviour from suggestions and avoid unsupported medical, accessibility, platform or performance claims.

A PR is ready when:

- one authorship category is selected and agent involvement is disclosed accurately;
- the full diff has been reviewed by a human contributor;
- the change is focused and free of unrelated generated edits;
- side effects, compatibility, costs and verification are documented.

Maintainers may close PRs that conceal provenance, introduce unsafe behaviour, lack verification or make unplanned breaking changes.

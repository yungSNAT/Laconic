# How to install

<details>
<summary><strong>Antigravity (<code>agy</code>)</strong></summary>

### Install

```bash
agy plugin install https://github.com/yungSNAT/laconic
```

### Verify

```bash
agy plugin list
```

### Update

```bash
agy plugin uninstall laconic
agy plugin install https://github.com/yungSNAT/laconic
```

### Uninstall

```bash
agy plugin uninstall laconic
```

Or keep it installed and turn it off: `agy plugin disable laconic`.

### Always-on (optional)

Add to `~/.gemini/GEMINI.md`:

```markdown
## Output style

The harness and the user's explicit requirements outrank these rules. Write plainly in Australian English (Style Manual, then Macquarie's first spelling, then British, never American). Lead with the answer or action, then give the explanation the reader needs. Length follows what the reader must decide, do or understand. No preamble, recap or closing offer. Keep the writer's voice and the requested format. Explain fully when asked. Before a destructive action, show a preview and wait for confirmation.

Preserve facts, numbers, negation, scope, conditions, exceptions, uncertainty, caveats, required disclosures and necessary steps. Keep code, quotations, paths, URLs, identifiers, product names and official titles exact. Mark unverified claims and evidence gaps; invent nothing, including typical figures or timeframes. Cut filler, staged contrasts and repetitive summaries. Reread substantial prose once for meaning.

Before sending original prose, check:

- single quotation marks, not double;
- 'whilst' and 'amongst', not 'while' and 'among';
- numerals from 2, even 'all 3', except at a sentence start;
- no serial comma ('logs, metrics and traces') unless it prevents a misreading;
- no em dashes or unnecessary hyphens; financial years as 2026-27.

This is enough for routine chat, comments, handoffs, emails, short repository prose and small edits. For substantial drafting or editing, or a requested prose audit, load the `laconic-prose` skill. Reuse loaded guidance and consult its references only as needed. This routing overrides broader default reading requirements. Report an editorial score only when asked.
```

</details>

<details>
<summary><strong>AstronClaw (custom skill)</strong></summary>

AstronClaw supports importing a Markdown file as a custom skill. This route uses
the existing `SKILL.md`; see its [official skills guide](https://github.com/iflytek/astronclaw-tutorial/blob/main/docs/guide/astronclaw/skills.md)
for the upload and management controls.

This procedure follows AstronClaw's documentation but has not been tested with
this skill. Check the exported instructions before enabling it.

### Install

1. Download the [canonical SKILL.md](https://raw.githubusercontent.com/yungSNAT/laconic/main/skills/laconic/SKILL.md) and save it as `SKILL.md`. Review its contents before uploading.
2. In AstronClaw, open **我的技能 (My skills)**, choose **新建 (New)** and upload that `.md` file.
3. Check that the imported skill is named `laconic`. Use **启用/禁用 (Enable/Disable)** to control its availability.

This route installs the session rules only. The `laconic-prose` writing skill,
its references and the `LICENSE` and `NOTICE` files are not uploaded, so keep
copies of `LICENSE` and `NOTICE` with the downloaded `SKILL.md`. Uploading
sends that file to AstronClaw; the repository's plugin manifests and hooks are
not part of this setup.

### Verify and activate

Confirm `laconic` appears in **My skills**. Use **下载 (Download)** to review
the imported instructions against the original skill, then enable it and try:

```text
Use the laconic skill for this conversation. Explain how to create an empty Git repository in a new folder.
```

Check that the reply leads with the action and numbers the steps. This is a
manual check of the imported skill; a successful upload alone does not verify
that its response rules are being applied.

### Activation note

AstronClaw supports both explicit requests and automatic skill invocation.
Its guide does not specify whether it honours `disable-model-invocation: true`,
so use **Disable** when you do not want the skill available. There is no need
to rely on a `/laconic` slash command.

The skill instructs the assistant to keep the style for the conversation until
you say `stop laconic` or `normal mode`. That instruction does not change the
platform toggle; disable the skill and start a new conversation for a fresh
session without it.

### Update

Download the latest canonical `SKILL.md`. If you customised the imported copy,
use **下载 (Download)** to keep a backup first. For a clean replacement, delete
the old `laconic` entry, repeat the import and run the verification prompt
in a new conversation.

### Uninstall

In **My skills**, select `laconic` and choose **删除 (Delete)**, then start a
new conversation. To keep the imported copy for later, choose **Disable** instead.

</details>

<details>
<summary><strong>Claude Code</strong></summary>

### Install

```bash
claude plugin marketplace add yungSNAT/laconic
claude plugin install laconic@laconic
```

Type `/laconic`.

### Verify

```bash
claude plugin list
```

### Update

```bash
claude plugin marketplace update laconic
claude plugin update laconic@laconic
```

The first command refreshes the marketplace listing. The second updates the installed plugin. Restart Claude Code to load it.

`claude plugin update` skips a plugin whose `version` has not changed. If you maintain a fork, bump `version` in `.claude-plugin/plugin.json` after you edit the rules.

### Uninstall

```bash
claude plugin uninstall laconic
claude plugin marketplace remove laconic
```

Or keep it installed and turn it off: `claude plugin disable laconic`.

### Always-on (optional)

A `SessionStart` hook loads the full ruleset at the start of every session, no `/laconic` needed:

```bash
touch ~/.claude/.laconic-always
```

If you use a custom Claude configuration directory, create the flag there instead:

```bash
touch "$CLAUDE_CONFIG_DIR/.laconic-always"
```

Back to on-demand:

```bash
rm ~/.claude/.laconic-always
```

The hook only fires when the flag file exists, so installing the plugin changes nothing by itself. 'stop laconic' still turns it off for the current session.

Use either this hook or the compact rules in `~/.claude/rules/laconic.md`, not both. Session mode already contains the compact rules, so loading both repeats about 240 words in every session.

</details>


<details>
<summary><strong>Codex</strong></summary>

### Install

```bash
codex plugin marketplace add yungSNAT/laconic --ref main
codex plugin add laconic@laconic
```

Invoke the skill explicitly by typing `$laconic`. Codex will not activate
it automatically.

### Verify

```bash
codex plugin list
```

### Update

```bash
codex plugin marketplace upgrade laconic
codex plugin remove laconic
codex plugin add laconic@laconic
```

### Uninstall

```bash
codex plugin remove laconic
codex plugin marketplace remove laconic
```

### Always-on (optional)

Add to `~/.codex/AGENTS.md`:

```markdown
## Output style

The harness and the user's explicit requirements outrank these rules. Write plainly in Australian English (Style Manual, then Macquarie's first spelling, then British, never American). Lead with the answer or action, then give the explanation the reader needs. Length follows what the reader must decide, do or understand. No preamble, recap or closing offer. Keep the writer's voice and the requested format. Explain fully when asked. Before a destructive action, show a preview and wait for confirmation.

Preserve facts, numbers, negation, scope, conditions, exceptions, uncertainty, caveats, required disclosures and necessary steps. Keep code, quotations, paths, URLs, identifiers, product names and official titles exact. Mark unverified claims and evidence gaps; invent nothing, including typical figures or timeframes. Cut filler, staged contrasts and repetitive summaries. Reread substantial prose once for meaning.

Before sending original prose, check:

- single quotation marks, not double;
- 'whilst' and 'amongst', not 'while' and 'among';
- numerals from 2, even 'all 3', except at a sentence start;
- no serial comma ('logs, metrics and traces') unless it prevents a misreading;
- no em dashes or unnecessary hyphens; financial years as 2026-27.

This is enough for routine chat, comments, handoffs, emails, short repository prose and small edits. For substantial drafting or editing, or a requested prose audit, load the `laconic-prose` skill. Reuse loaded guidance and consult its references only as needed. This routing overrides broader default reading requirements. Report an editorial score only when asked.
```

</details>


<details>
<summary><strong>Grok (<code>grok</code>)</strong></summary>

Grok loads the repository's existing plugin and skill files; no separate Grok manifest is required. Install directly from GitHub, enable the plugin, then invoke the skill. Two Grok-only steps: `--trust` (hooks and skills stay inactive without it) and `grok plugin enable` (plugins stay off until enabled).

### Install

```bash
grok plugin install yungSNAT/laconic --trust
grok plugin enable laconic
```

Start a new Grok session and type `/laconic`. Grok honours `disable-model-invocation: true`, so nothing applies until you invoke the skill or turn on always-on.

### Verify

```bash
grok plugin list
grok plugin details laconic
```

Confirm `laconic` is listed, enabled and shows a skill plus hooks.

### Update

```bash
grok plugin update laconic
```

### Uninstall

```bash
grok plugin uninstall laconic --confirm
```

Or keep it installed and turn it off: `grok plugin disable laconic`.

### Always-on (optional)

Add the block to `~/.grok/AGENTS.md`, or drop it in `~/.grok/rules/laconic.md` (Grok loads both at session start):

```markdown
## Output style

The harness and the user's explicit requirements outrank these rules. Write plainly in Australian English (Style Manual, then Macquarie's first spelling, then British, never American). Lead with the answer or action, then give the explanation the reader needs. Length follows what the reader must decide, do or understand. No preamble, recap or closing offer. Keep the writer's voice and the requested format. Explain fully when asked. Before a destructive action, show a preview and wait for confirmation.

Preserve facts, numbers, negation, scope, conditions, exceptions, uncertainty, caveats, required disclosures and necessary steps. Keep code, quotations, paths, URLs, identifiers, product names and official titles exact. Mark unverified claims and evidence gaps; invent nothing, including typical figures or timeframes. Cut filler, staged contrasts and repetitive summaries. Reread substantial prose once for meaning.

Before sending original prose, check:

- single quotation marks, not double;
- 'whilst' and 'amongst', not 'while' and 'among';
- numerals from 2, even 'all 3', except at a sentence start;
- no serial comma ('logs, metrics and traces') unless it prevents a misreading;
- no em dashes or unnecessary hyphens; financial years as 2026-27.

This is enough for routine chat, comments, handoffs, emails, short repository prose and small edits. For substantial drafting or editing, or a requested prose audit, load the `laconic-prose` skill. Reuse loaded guidance and consult its references only as needed. This routing overrides broader default reading requirements. Report an editorial score only when asked.
```

</details>

<details>
<summary><strong>Gemini CLI</strong></summary>

Gemini CLI has no plugin marketplace, so there are 2 native routes: a **custom command** (opt-in, off until you invoke it) or an **extension** (always-on once installed). The command route matches this skill's default posture; pick it unless you want the rules on every session.

### Install (command, opt-in)

```bash
mkdir -p ~/.gemini/commands
curl -fsSL https://raw.githubusercontent.com/yungSNAT/laconic/main/skills/laconic/agents/gemini.toml \
  -o ~/.gemini/commands/laconic.toml
```

Start a new session, type `/laconic`. It stays on for that session.

### Install (extension, always-on)

```bash
gemini extensions install https://github.com/yungSNAT/laconic
```

The extension loads `GEMINI.md`, which imports the full skill, so the rules apply from message one. `git` must be installed.

### Verify

```bash
gemini extensions list          # extension route
ls ~/.gemini/commands           # command route: laconic.toml present
```

Or type `/` in a session and confirm `laconic` is listed.

### Update

```bash
gemini extensions update laconic    # extension route
# command route: re-run the curl above
```

### Uninstall

```bash
gemini extensions uninstall laconic    # extension route
rm ~/.gemini/commands/laconic.toml     # command route
```

</details>

<details>
<summary><strong>GitHub Copilot (VS Code and Copilot CLI)</strong></summary>

Copilot reads Agent Skills natively: the same `SKILL.md`, no conversion. It scans `.github/skills/`, `.claude/skills/` and `.agents/skills/` in the project, and `~/.copilot/skills/` and `~/.agents/skills/` globally. VS Code also scans `~/.claude/skills/`; GitHub's Copilot CLI documentation does not list that path.

### Install

```bash
npx skills add yungSNAT/laconic -a github-copilot        # this project
npx skills add yungSNAT/laconic -a github-copilot -g     # all projects
```

Without the CLI, copy the skill folder into any directory Copilot scans:

```bash
git clone https://github.com/yungSNAT/laconic
mkdir -p ~/.copilot/skills
cp -R laconic/skills/laconic ~/.copilot/skills/
cp -R laconic/skills/laconic-prose ~/.copilot/skills/
```

### Verify

Type `/` in the chat input and confirm `laconic` appears. Or:

```bash
npx skills list
npx skills ls -g    # if installed globally
```

### Update

```bash
npx skills update laconic
```

Or re-copy the folder after `git pull`.

### Uninstall

```bash
npx skills remove laconic
```

Or delete the `laconic` folder from the skills directory it landed in.

### Activation note

Copilot respects `disable-model-invocation`: nothing applies until you invoke the skill, same as Claude Code (tested upstream in i-have-adhd [#60](https://github.com/ayghri/i-have-adhd/pull/60)).

### Always-on (optional)

Add the block below to `.github/copilot-instructions.md` in the project (Copilot reads it into every chat):

```markdown
## Output style

The harness and the user's explicit requirements outrank these rules. Write plainly in Australian English (Style Manual, then Macquarie's first spelling, then British, never American). Lead with the answer or action, then give the explanation the reader needs. Length follows what the reader must decide, do or understand. No preamble, recap or closing offer. Keep the writer's voice and the requested format. Explain fully when asked. Before a destructive action, show a preview and wait for confirmation.

Preserve facts, numbers, negation, scope, conditions, exceptions, uncertainty, caveats, required disclosures and necessary steps. Keep code, quotations, paths, URLs, identifiers, product names and official titles exact. Mark unverified claims and evidence gaps; invent nothing, including typical figures or timeframes. Cut filler, staged contrasts and repetitive summaries. Reread substantial prose once for meaning.

Before sending original prose, check:

- single quotation marks, not double;
- 'whilst' and 'amongst', not 'while' and 'among';
- numerals from 2, even 'all 3', except at a sentence start;
- no serial comma ('logs, metrics and traces') unless it prevents a misreading;
- no em dashes or unnecessary hyphens; financial years as 2026-27.

This is enough for routine chat, comments, handoffs, emails, short repository prose and small edits. For substantial drafting or editing, or a requested prose audit, load the `laconic-prose` skill. Reuse loaded guidance and consult its references only as needed. This routing overrides broader default reading requirements. Report an editorial score only when asked.
```

</details>


<details>
<summary><strong>Hermes</strong></summary>

### Install

```bash
hermes skills install yungSNAT/laconic/skills/laconic
hermes skills install yungSNAT/laconic/skills/laconic-prose
```

Type `/laconic`. The skill installs into `~/.hermes/skills/` and is exposed as a slash command at the next session start.

Prefer to browse first? Add this repo as a skill source (a 'tap'), then search and install:

```bash
hermes skills tap add yungSNAT/laconic
hermes skills search laconic
hermes skills install yungSNAT/laconic/skills/laconic
hermes skills install yungSNAT/laconic/skills/laconic-prose
```

### Verify

```bash
hermes skills list
```

### Update

```bash
hermes skills update laconic
```

### Uninstall

```bash
hermes skills uninstall laconic
```

Or remove the tap too: `hermes skills tap remove yungSNAT/laconic`.

### Always-on (optional)

Add to the `AGENTS.md` in your working directory (Hermes loads it per workdir), or to your persona `SOUL.md` for every session:

```markdown
## Output style

The harness and the user's explicit requirements outrank these rules. Write plainly in Australian English (Style Manual, then Macquarie's first spelling, then British, never American). Lead with the answer or action, then give the explanation the reader needs. Length follows what the reader must decide, do or understand. No preamble, recap or closing offer. Keep the writer's voice and the requested format. Explain fully when asked. Before a destructive action, show a preview and wait for confirmation.

Preserve facts, numbers, negation, scope, conditions, exceptions, uncertainty, caveats, required disclosures and necessary steps. Keep code, quotations, paths, URLs, identifiers, product names and official titles exact. Mark unverified claims and evidence gaps; invent nothing, including typical figures or timeframes. Cut filler, staged contrasts and repetitive summaries. Reread substantial prose once for meaning.

Before sending original prose, check:

- single quotation marks, not double;
- 'whilst' and 'amongst', not 'while' and 'among';
- numerals from 2, even 'all 3', except at a sentence start;
- no serial comma ('logs, metrics and traces') unless it prevents a misreading;
- no em dashes or unnecessary hyphens; financial years as 2026-27.

This is enough for routine chat, comments, handoffs, emails, short repository prose and small edits. For substantial drafting or editing, or a requested prose audit, load the `laconic-prose` skill. Reuse loaded guidance and consult its references only as needed. This routing overrides broader default reading requirements. Report an editorial score only when asked.
```

</details>

<details>
<summary><strong>Kimi Code CLI</strong></summary>

### Install

Start a Kimi Code session, then:

1. Run `/plugins`.
2. Choose **Custom**.
3. Paste `https://github.com/yungSNAT/laconic` and press `Enter`.
4. Choose **Trust and install**.

Use slash command `/skill:laconic` to invoke the skill explicitly.

### Update

`/plugins` in Kimi Code session, cursor to **Laconic**, press `R`.

### Uninstall

`/plugins` in Kimi Code session, cursor to **Laconic**, press `D`.


</details>

<details>
<summary><strong>OpenCode</strong></summary>

The plugin supports OpenCode V1 **1.18.29+** and V2. It registers the skill and `/laconic` command, and adds optional always-on mode. Older V1 versions need an upgrade or the plugin from the previous skill release.

### Install

For use across projects:

```bash
git clone https://github.com/yungSNAT/laconic ~/.config/opencode/vendor/laconic
mkdir -p ~/.config/opencode/plugins
cat > ~/.config/opencode/plugins/laconic.js <<'EOF'
export { default } from '../vendor/laconic/.opencode/plugins/laconic.mjs';
EOF
```

For an existing V1 installation, keep the `plugin` entry in `opencode.json`. If switching to the loader above, remove that entry.

Start a new session and type `/laconic`. The rules apply until you say `stop laconic` or `normal mode`.

#### Skill and command only (without always-on)

Alternatively, from a checkout, copy the skill and command into OpenCode's global directories:

```bash
mkdir -p ~/.config/opencode/skills ~/.config/opencode/commands
cp -R skills/laconic ~/.config/opencode/skills/
cp -R skills/laconic-prose ~/.config/opencode/skills/
cp .opencode/command/laconic.md ~/.config/opencode/commands/laconic.md
```

### Verify

Start OpenCode, type `/` and confirm `laconic` appears in the command list.

### Update

```bash
git -C ~/.config/opencode/vendor/laconic pull
```

For the copy-based install, pull your checkout and re-copy the skill folder and the command file.

### Uninstall

Remove `~/.config/opencode/plugins/laconic.js`. For the copy-based install, remove `~/.config/opencode/skills/laconic/`, `~/.config/opencode/skills/laconic-prose/` and `~/.config/opencode/commands/laconic.md`. For an existing V1 config-based install, remove its `plugin` entry.

### Always-on (optional)

Requires the plugin. The flag adds the complete ruleset to the system prompt on every turn:

```bash
touch ~/.config/opencode/.laconic-always
```

`stop laconic` or `normal mode` asks the agent to return to its normal style for the current session. Delete the flag to stop automatic injection:

```bash
rm ~/.config/opencode/.laconic-always
```

If `XDG_CONFIG_HOME` is set, replace `~/.config` in these paths with that directory.

</details>

<details>
<summary><strong>Pi</strong></summary>

Pi discovers this repository as a native package: `extensions/` provides the session-persistent mode and `skills/` keeps the Agent Skills entry point available.

### Install

```bash
pi install https://github.com/yungSNAT/laconic
```

Start a new Pi session. Toggle Laconic output for the current session:

```text
/laconic
```

The footer shows `● LACONIC ON` whilst the mode is active. Run the command again to turn it off, or be explicit:

```text
/laconic on
/laconic off
stop laconic
```

Like the Claude Code hook, the extension adds the ruleset to the conversation once instead of rewriting the system prompt on every request and adds it again after compaction drops it.

The existing Agent Skills command remains available as an alias:

```text
/skill:laconic
```

Start a new Pi session with the mode enabled by default:

```bash
pi --laconic
```

### Verify

```bash
pi list
```

Confirm the GitHub package is listed, then type `/laconic` and check that `● LACONIC ON` appears in the footer.

### Update

```bash
pi update https://github.com/yungSNAT/laconic
```

Or update every unpinned Pi package with `pi update --extensions`.

### Uninstall

```bash
pi remove https://github.com/yungSNAT/laconic
```

### Always-on (optional)

Create a flag in Pi's agent configuration directory:

```bash
touch ~/.pi/agent/.laconic-always
```

The extension checks the flag at every new, resumed, forked or reloaded session. A saved choice for the current session wins over this default, so `stop laconic` keeps that session disabled.

Back to on-demand:

```bash
rm ~/.pi/agent/.laconic-always
```

### Config file (optional)

Create `~/.pi/agent/laconic.json` in Pi's agent configuration directory:

```json
{
  "alwaysOn": true,
  "hideStatus": true
}
```

- `alwaysOn`: start every session with the rules active, like the `.laconic-always` flag file, which still works
- `hideStatus`: keep the `● LACONIC ON` status-bar entry hidden; the rules and the `/laconic` command still work

Read once at extension startup, so restart Pi after changing it. A saved choice for the current session wins over `alwaysOn`, so `stop laconic` keeps that session disabled.

If `PI_CODING_AGENT_DIR` is set, put `.laconic-always` in that directory instead. Run `/reload` or start a new session after changing the flag.

</details>


<details>
<summary><strong>Oh My Pi (OMP)</strong></summary>

### Install

```bash
omp plugin marketplace add yungSNAT/laconic
omp plugin install --scope user laconic@laconic
```

Start a new OMP session and run `/laconic` to toggle the mode.

### Update

```bash
omp plugin marketplace update laconic
omp plugin upgrade --scope user laconic@laconic
```

### Uninstall

```bash
omp plugin uninstall --scope user laconic@laconic
omp plugin marketplace remove laconic
```

</details>


<details>
<summary><strong>Qwen Code</strong></summary>

### Install

```bash
qwen extensions install yungSNAT/laconic
```

Qwen Code supports the GitHub shorthand and installs the repository as a
native extension. The extension discovers the skill under `skills/`.

Type `/laconic` to invoke the skill explicitly. Installing the extension
does not change output until the skill is invoked.

### Verify

```bash
qwen extensions list
```

Then start a new Qwen Code session and run:

```text
/skills
```

Confirm that `laconic` appears in the list.

### Update

```bash
qwen extensions update laconic
```

### Uninstall

```bash
qwen extensions uninstall laconic
```

</details>

<details>
<summary><strong>Zed</strong></summary>

Zed's Agent reads Agent Skills natively using the same SKILL.md format without conversion. Note that Zed's older 'Rules' have been replaced by Skills alongside AGENTS.md instructions.

### Install

In the Agent Panel, open the Skills manager and choose **Create skill from URL** (also in the command palette as `agent: create skill from url`), then paste:

```
https://github.com/yungSNAT/laconic/blob/main/skills/laconic/SKILL.md
```

Save it in **User** scope for every project, or **Project** scope for one. Then type `/laconic` in the Agent Panel.

The URL import brings only the session skill's `SKILL.md`, without the `laconic-prose` writing skill, its references, `LICENSE` or `NOTICE`. For the complete set, clone the repo and copy both skill folders into your user skills directory:

```bash
git clone https://github.com/yungSNAT/laconic
mkdir -p ~/.agents/skills
cp -R laconic/skills/laconic ~/.agents/skills/
cp -R laconic/skills/laconic-prose ~/.agents/skills/
```

### Verify

Open the Skills manager in the Agent Panel and confirm `laconic` is listed. Or type `/` and confirm it appears.

### Update

Re-import from the same URL (overwrites), or re-copy the folder after `git pull`.

### Uninstall

Remove `laconic` and `laconic-prose` from the Skills manager, or delete `~/.agents/skills/laconic` and `~/.agents/skills/laconic-prose`.

### Always-on (optional)

Add to your personal `~/.config/zed/AGENTS.md`:

```markdown
## Output style

The harness and the user's explicit requirements outrank these rules. Write plainly in Australian English (Style Manual, then Macquarie's first spelling, then British, never American). Lead with the answer or action, then give the explanation the reader needs. Length follows what the reader must decide, do or understand. No preamble, recap or closing offer. Keep the writer's voice and the requested format. Explain fully when asked. Before a destructive action, show a preview and wait for confirmation.

Preserve facts, numbers, negation, scope, conditions, exceptions, uncertainty, caveats, required disclosures and necessary steps. Keep code, quotations, paths, URLs, identifiers, product names and official titles exact. Mark unverified claims and evidence gaps; invent nothing, including typical figures or timeframes. Cut filler, staged contrasts and repetitive summaries. Reread substantial prose once for meaning.

Before sending original prose, check:

- single quotation marks, not double;
- 'whilst' and 'amongst', not 'while' and 'among';
- numerals from 2, even 'all 3', except at a sentence start;
- no serial comma ('logs, metrics and traces') unless it prevents a misreading;
- no em dashes or unnecessary hyphens; financial years as 2026-27.

This is enough for routine chat, comments, handoffs, emails, short repository prose and small edits. For substantial drafting or editing, or a requested prose audit, load the `laconic-prose` skill. Reuse loaded guidance and consult its references only as needed. This routing overrides broader default reading requirements. Report an editorial score only when asked.
```

</details>

<details>
<summary><strong>Cursor, Amp and any other agent-skills harness</strong></summary>

Works with any harness that reads agent skills. Swap `-a <agent>` for yours.

### Install

```bash
npx skills add yungSNAT/laconic                  # this workspace
npx skills add yungSNAT/laconic -g               # all projects
npx skills add yungSNAT/laconic -a cursor -y     # one agent only
npx skills add yungSNAT/laconic -a opencode -y
```

New agent chat, type `/laconic`.

Without the CLI, copy the skill folder into whatever path your agent scans:

```bash
git clone https://github.com/yungSNAT/laconic
mkdir -p ~/.cursor/skills     # Cursor. Use .agents/skills for OpenCode, or your agent's own path
cp -R laconic/skills/laconic ~/.cursor/skills/
cp -R laconic/skills/laconic-prose ~/.cursor/skills/
```

### Verify

```bash
npx skills list
npx skills ls -g    # if installed globally
```

### Update

```bash
npx skills update laconic
npx skills update -g    # if installed globally
```

### Uninstall

```bash
npx skills remove laconic
npx skills remove laconic -g    # if installed globally
```

### Always-on (optional)

Paste this into your agent's persistent rules file. Cursor: **Settings → Rules → User Rules**, or a project rule under `.cursor/rules/` with `alwaysApply: true`. OpenCode: `~/.config/opencode/AGENTS.md`.

```markdown
## Output style

The harness and the user's explicit requirements outrank these rules. Write plainly in Australian English (Style Manual, then Macquarie's first spelling, then British, never American). Lead with the answer or action, then give the explanation the reader needs. Length follows what the reader must decide, do or understand. No preamble, recap or closing offer. Keep the writer's voice and the requested format. Explain fully when asked. Before a destructive action, show a preview and wait for confirmation.

Preserve facts, numbers, negation, scope, conditions, exceptions, uncertainty, caveats, required disclosures and necessary steps. Keep code, quotations, paths, URLs, identifiers, product names and official titles exact. Mark unverified claims and evidence gaps; invent nothing, including typical figures or timeframes. Cut filler, staged contrasts and repetitive summaries. Reread substantial prose once for meaning.

Before sending original prose, check:

- single quotation marks, not double;
- 'whilst' and 'amongst', not 'while' and 'among';
- numerals from 2, even 'all 3', except at a sentence start;
- no serial comma ('logs, metrics and traces') unless it prevents a misreading;
- no em dashes or unnecessary hyphens; financial years as 2026-27.

This is enough for routine chat, comments, handoffs, emails, short repository prose and small edits. For substantial drafting or editing, or a requested prose audit, load the `laconic-prose` skill. Reuse loaded guidance and consult its references only as needed. This routing overrides broader default reading requirements. Report an editorial score only when asked.
```
</details>


## How activation works

1. **Installed, not invoked.** In Claude Code, Qwen Code, Codex and Grok, nothing happens until you invoke the skill explicitly. Claude Code, Qwen Code and Grok honour `disable-model-invocation: true` in `SKILL.md`; Codex honours `policy.allow_implicit_invocation: false` in `agents/openai.yaml`. Other harnesses may load every skill's description at startup and activate the skill themselves.
2. **You invoke it explicitly.** Type `/laconic` in Claude Code, Qwen Code or Grok, or `$laconic` in Codex. Rules stay on for that session. 'stop laconic' or 'normal mode' turns them off.
3. **You touch `~/.claude/.laconic-always`** (Claude Code). A `SessionStart` hook loads the full ruleset from message one, every session.
4. **You add the always-on snippet above** (Grok, Codex and other harnesses). Grok reads `~/.grok/AGENTS.md` and `~/.grok/rules/*.md`. Keeps the core rules in your agent's persistent context.

In Claude Code, Qwen Code, Codex and Grok, no middle ground: if you did not turn it on, it is off.

## Troubleshooting

**`/laconic` not in autocomplete.** Restart the agent. The plugin index is read at startup. On Grok, also run `grok plugin enable laconic` and confirm the install used `--trust`.

**Always-on flag has no effect.** Update the plugin (`claude plugin marketplace update laconic`, then `claude plugin update laconic@laconic`) and restart. Hooks are read at startup, and the flag needs the plugin version that ships `hooks/hooks.json`. Grok does not read `~/.claude/.laconic-always`; put the always-on block in `~/.grok/AGENTS.md` or `~/.grok/rules/laconic.md`.

**`claude plugin marketplace add` fails.** Use the `owner/repo` form. A local path must point at the repo root, not `.claude-plugin/`.

**`grok plugin install` does nothing visible.** Add `--trust`, then run `grok plugin enable laconic`, then start a new session. Grok plugins stay off and untrusted until those 2 steps.

**Installed but replies still preamble.** Open a new session. If it still drifts, tighten the wording in `skills/laconic/SKILL.md`.

**Want different rules.** Fork, edit `skills/laconic/SKILL.md`, then swap your copy in:

```bash
claude plugin uninstall laconic            # drop the upstream copy first:
claude plugin marketplace remove laconic   # fork and upstream share both names
claude plugin marketplace add <your-username>/laconic
claude plugin install laconic@laconic
```

Restart, then re-invoke `/laconic`.

**Skill missing after `npx skills add`.** Start a new agent chat. Skills are indexed at session start. Confirm the folder landed where your agent scans (`~/.cursor/skills/` for Cursor, `.agents/skills/` for OpenCode) and that the frontmatter `name` matches the folder name.

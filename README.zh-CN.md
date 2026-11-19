<p align="center">
  <picture>
    <source media="(max-width: 600px) and (prefers-color-scheme: light)" srcset=".github/assets/banner-phone-light.svg">
    <source media="(max-width: 600px)" srcset=".github/assets/banner-phone.svg">
    <source media="(prefers-color-scheme: light)" srcset=".github/assets/banner-light.svg">
    <img src=".github/assets/banner.svg" width="100%" alt="Laconic。答案先行。为保留事实而设计。澳大利亚英语。像素画：一顶带冠饰的斯巴达头盔，旁边是粗衬线方块字母组成的 Laconic 字标。">
  </picture>
</p>

<div align="center">

[![tests](https://github.com/yungSNAT/Laconic/actions/workflows/tests.yml/badge.svg?branch=main)](https://github.com/yungSNAT/Laconic/actions/workflows/tests.yml) [![Codecov](https://codecov.io/gh/yungSNAT/Laconic/graph/badge.svg)](https://codecov.io/gh/yungSNAT/Laconic) [![CodeQL](https://github.com/yungSNAT/Laconic/actions/workflows/codeql.yml/badge.svg?branch=main)](https://github.com/yungSNAT/Laconic/actions/workflows/codeql.yml) [![OSV-Scanner](https://github.com/yungSNAT/Laconic/actions/workflows/osv-scanner.yml/badge.svg?branch=main)](https://github.com/yungSNAT/Laconic/actions/workflows/osv-scanner.yml) [![SonarCloud](https://sonarcloud.io/api/project_badges/measure?project=EquityBot_Laconic&metric=alert_status)](https://sonarcloud.io/summary/new_code?id=EquityBot_Laconic) [![Codacy code quality](https://app.codacy.com/project/badge/Grade/623f5ab312d4497c9e8161e547c95f44)](https://app.codacy.com/gh/yungSNAT/Laconic/dashboard)<br>
[![OpenSSF Scorecard](https://raw.githubusercontent.com/ryanduguid/.github/badges/EquityBot/Laconic.svg)](https://scorecard.dev/viewer/?uri=github.com/yungSNAT/Laconic)<br>
[![release](https://img.shields.io/github/v/release/yungSNAT/Laconic?filter=laconic-check-v*&color=66023C&labelColor=000000&style=flat)](https://github.com/yungSNAT/Laconic/releases?q=laconic-check-v&expanded=true) [![licence: Apache-2.0](.github/assets/badges/licence.svg)](LICENSE) [![fork of: i-have-adhd](.github/assets/badges/fork-of.svg)](https://github.com/ayghri/i-have-adhd)

[安装](#install) · [使用](#use) · [变化](#what-changes) · [评估](#evaluation) · [检查器](#checker) · [始终开启](#always-on) · [开发](#development) · [致谢](#credits) · [许可证](#licence)

[English](README.md) · [Español](README.es.md) · [한국어](README.ko.md) · 简体中文 · [日本語](README.ja.md)

</div>

Laconic 是面向编程智能体和聊天助手的 AI“去 AI 味”（humanizer）与“no AI slop”技能。它让模型先给出答案，标明尚未核实的内容，答完即止，并要求模型保留读者需要的每一个事实、步骤和限定条件。它要求模型按照 Australian Government Style Manual 写出每一句原创文字，并按技能中列出的本项目规则覆盖其中部分要求。

要把 AI 写的文字改得更像人写的，请让智能体对其进行润色（humanise）。写作技能会去除 AI 写作模式，其设计目标是保留每一个事实、数字和限定条件，并让文字更短，而不是用刻意表演的个性来填充。检验这一承诺的[事实保留基准](evals/README.md#fact-survival-benchmark)尚未出结果。2026 年 11 月 14 日，在 99 篇公开文本上，由 Laconic 作者运行的[外部单元比对检查器](#fact-changes-on-a-public-set)发现：视模型而定，Laconic 改写丢失数字、日期、名称或引文的频率与 `humanizer` 相当或更高；两项差异都没有明显偏离 0。

<table>
<tr>
<td align="center" width="33%"><h3>4.92（满分 5）</h3>润色质量，对比 4.54 和 4.50<br><sub><a href="#against-other-humanisers">中立评分标准，20 篇文本，盲评</a></sub></td>
<td align="center" width="33%"><h3>原文长度的 65%</h3>两款对比工具均为 81%<br><sub><a href="#against-other-humanisers">仅比较长度；事实检查另列</a></sub></td>
<td align="center" width="33%"><h3>4.63 对 4.07</h3>对比简单指令 'Answer concisely.'<br><sub><a href="#evaluation">本项目评分标准，45 个案例；阻断问题 6 个对 29 个</a></sub></td>
</tr>
</table>

以上 3 项数据均来自 Laconic 自带的评测框架，测于 2026 年 10 月 20 日，使用单一生成模型。[评估](#evaluation)一节给出了局限和复现命令。

<a id="install"></a>
## 安装

把下面这段粘贴给你的智能体：

```text
Install the laconic skill/plugin from https://github.com/yungSNAT/laconic, following the repository's AGENTS.md.
```

或者按照你所用智能体的[安装说明](INSTALL.md)操作。Claude Code：

```bash
claude plugin marketplace add yungSNAT/laconic
claude plugin install laconic@laconic
```

Skills CLI 支持的任何智能体：

```bash
npx skills add yungSNAT/laconic --global --agent "*" --yes
```

<a id="use"></a>
## 使用

重启智能体，然后输入 `/laconic`。它会在本次会话中一直生效，直到你说 'stop laconic'。

<a id="when-to-choose-something-else"></a>
## 何时选择其他工具

- **最看重 token 成本：** [caveman](https://github.com/JuliusBrussee/caveman) 正是为此而做，还会压缩智能体读取的内容；[Chisle](https://github.com/JayPokale/Chisle) 写出的输出 token 更少。在 Laconic 的 47 个案例上（[token 成本](#token-cost-against-caveman-and-chisle)，2026 年 11 月 17 日），Laconic 精简规则的输出 token 比 caveman 多 14%，比 Chisle 多 44%。规则从提示缓存读取时（与会话中一样），每个回答的成本比 caveman 高 17%，比 Chisle 高 45%；在每次都把规则写入缓存的单次冷调用中，成本比 caveman 低 11%（caveman 的规则更长），比 Chisle 高 5%。会话模式每项任务的成本比精简规则高约 27%。
- **需要其他英语变体：** Laconic 只写澳大利亚英语。[LanguageTool](https://github.com/languagetool-org/languagetool) 可在文字写成后检查澳大利亚、英国、加拿大、新西兰、南非和美国英语的语法与文体。
- **接受会增加细节的改写：** [humanizer](https://github.com/blader/humanizer) 4.0.0（2026 年 11 月 10 日）在文本空泛之处要求模型 'add a specific detail you know to be true'。Laconic 不增加事实；若编辑增加了它所保护的数字、URL、引文或其他内容，`laconic-check preserve --strict` 会判为失败。
- **只想要写作指南：** Laconic 写作指南所依据的 [No AI Slop](https://github.com/petergyang/no-ai-slop) 提供了同样的指南，但不含回答规则和澳大利亚英语。
- **在 Microsoft Word 中编辑：** [PerfectIt](https://www.perfectit.com/) 在 Word 中按样式表检查一致性，包括一份 Australian Government Style 样式表。Laconic 运行于智能体、聊天助手和 CI 中，不在 Word 里运行。
- **想在 CI 中检查自己的项目文风：** [Vale](https://github.com/vale-cli/vale) 按你配置的规则检查文字，支持正则表达式、规则类型和规则包；`laconic-check` 只是在 Laconic 的规则之外加上项目自己的替换词和禁用短语。

<a id="what-changes"></a>
## 变化

<table>
<tr>
<td width="50%">

### 之前

> Great question! Let me take a look. Your auth flow has a few moving pieces: the middleware, the token verification and the cookie handling. It's likely a caching issue, which typically resolves itself within an hour or so. Hope this helps! Let me know if you want to dig deeper.

</td>
<td width="50%">

### 之后

> The 401 comes from the missing `Authorization` header: the endpoint requires a bearer token and the request sends none.
>
> 1. Add `Authorization: Bearer <token>` at `src/api.ts:42`
> 2. Run `npm test -- api.spec.ts`
>
> Not verified: whether the token expires mid-session. The logs do not show it.

</td>
</tr>
</table>

示例保留英文，因为 Laconic 用英文写作。

<a id="what-it-includes"></a>
## 包含内容

| 组成部分 | 路径 | 何时生效 |
|---|---|---|
| 会话模式 | `skills/laconic/SKILL.md` | 输入 `/laconic` 后，在会话剩余时间内生效，直到 'stop laconic' |
| 写作技能 | `skills/laconic-prose/SKILL.md` | 由模型在起草、编辑和文字审查时加载；只影响该任务 |
| 精简规则 | `rules/laconic.md` | 加入智能体规则后始终加载（约 240 个单词） |

会话规则全文见 [SKILL.md](skills/laconic/SKILL.md)：

1. 先给答案，再说理由。
2. 说明你知道什么、如何知道。标出未核实的说法；“凭现有证据我无法判断”是可以接受的回答。
3. 长度取决于读者需要决定、执行或理解的内容。删减字词，绝不删减实质。
4. 步骤要编号。
5. 专注当前任务。
6. 让状态保持可见。
7. 直白说明错误：错误本身、已知原因，以及修复方法或下一步检查。
8. 只有有依据时才做估计。
9. 长列表要分组，但不要遗漏相关条目。
10. 不写开场白、复述或结束语。

每个组成部分都采用澳大利亚英语（拼写、单引号、逗号、数字、日期、不用长破折号），在澳大利亚资料无法确定写法时退而采用英式英语，绝不用美式英语。本项目规则：在 Style Manual 倾向使用 'while' 和 'among' 的地方，使用 whilst 和 amongst。写作技能另外提供完整的写作、编辑和审查流程：针对常见 AI 写作模式的快速检查、意义与语气的保留、不同场合的写作指南、Style Manual 的细则以及 AGLC4 引注说明。

<a id="evaluation"></a>
## 评估

2026 年 10 月 19 日用 `evals/` 中的评测框架测得：30 个案例，每个 2 次试验，由 Claude Opus 5.5 生成并盲评。每种配置都安装了各自的技能、规则和参考资料，且均可读取。对照配置是 Laconic 所取代的 No AI Slop 技能及其精简规则 stop-slop。

| 指标 | Laconic | No AI Slop 配置 | 差值（95% 区间） |
|---|---|---|---|
| 正确性（1 到 5） | 4.68 | 3.98 | +0.70（+0.33 到 +1.07） |
| 简洁性（1 到 5） | 4.67 | 4.12 | +0.55（+0.30 到 +0.80） |
| 安全性（1 到 5） | 4.90 | 4.85 | +0.05（−0.10 到 +0.20） |
| 加权分 | 4.74 | 4.39 | +0.35（+0.18 到 +0.54） |
| 阻断问题 | 1 | 7 | |
| 平均回答长度（字符） | 1,360 | 1,663 | |

<details>
<summary>第二位评审、检查器计数和后续运行</summary>

来自另一模型家族的第二位盲评评审（GPT-6 Pro）认同方向，但差距更小：加权分 4.63 对 4.46（差值 +0.17，95% 区间 +0.05 到 +0.30），正确性 +0.33（+0.12 到 +0.58）。它对本项目细则的执行严格得多，在两种配置的许多回答中都发现了连续逗号（serial comma）和用英文单词书写的数字。

`tools/laconic-check` 无需评审即可统计这些疏漏。在每种配置的同样 60 个回答中：

| 配置 | 无问题回答 | 疑似连续逗号 | 用单词写的数字 | 双引号 |
|---|---|---|---|---|
| No AI Slop 配置 | 30 | 32 | 17 | 67 |
| Laconic 精简规则 | 40 | 13 | 15 | 1 |
| Laconic 会话模式（`/laconic`） | 51 | 12 | 3 | 0 |

2026 年 10 月 20 日的后续运行（30 个案例，1 次试验）测量了此后的规则改动。在精简规则中加入数字规则后，用单词写的数字从 5 降到 0，无问题回答从 30 个中的 18 个升到 23 个。会话模式加权得分 4.71，无阻断问题；仅用精简规则为 4.59。缩短后的会话技能（849 个单词，原为 1,153 个）得分更低（4.61），有 3 个阻断问题，因此保留完整措辞。

2026 年 10 月 20 日的其他检查：

- **发送前检查草稿：** 带 `laconic-check` 自检的会话模式在 18 个文字和会计案例上加权得分 4.82，不带自检为 4.71（在 10 个会计案例上为 4.74 对 4.49），阻断问题 0 个对 1 个。
- **实际使用：** 在 Laconic 取代旧配置的当天，作者本人的 Claude Code 会话中，没有 `laconic-check` 问题的助手消息从 26% 升到 42%；用单词写的数字从每 1,000 词 5.7 个降到 3.6 个。
- **智能体成本：** 在 5 个小型仓库修复任务上（`evals/README.md` 中的智能体成本基准，2 次试验），所有配置都通过了所有任务。会话模式每项任务的成本比精简规则高约 27%，主要因为它始终加载的提示更长；单独的智能体指南部分没有节省任何成本，已删除。

2026 年 10 月 20 日的一项简洁对照（45 个案例，2 次试验，Opus 5.5 生成并盲评）比较了已安装的配置：带写作技能的精简规则（在套话、附加限定和财年相关改动之前与之后），对比单条指令 'Answer concisely.'：

| 指标 | Laconic | 改动前的 Laconic | 'Answer concisely.' |
|---|---|---|---|
| 加权分 | 4.63 | 4.63 | 4.07 |
| 正确性（1 到 5） | 4.52 | 4.56 | 3.41 |
| 阻断问题 | 6 | 5 | 29 |
| 通过的正则检查 | 18 个中 18 个 | 18 个中 16 个 | 18 个中 16 个 |
| 没有 `laconic-check` 问题的回答 | 90 个中 67 个 | 90 个中 69 个 | 90 个中 41 个 |
| 平均回答长度（字符） | 1,232 | 1,148 | 1,022 |

Laconic 的加权分比简单指令高 0.56。这些改动对评审质量没有影响，并修正了另外两种配置都漏掉的财年连字符；没有一个阻断问题来自改动后的指南。

</details>

局限：每种配置 18 到 90 个回答，来自单一生成模型；案例在 `evals/cases.jsonl`，评分标准在 `evals/rubric.md`。这些案例也指导了规则的措辞，因此得分不是留出集测试；下文与 Claude Code Concise 风格的比较使用 `evals/holdout-cases.jsonl` 中的 10 个冻结案例，它们写于 2026 年 11 月 1 日，未用于调整规则。连续逗号仍是最常见的疏漏。

<a id="against-claude-codes-concise-style"></a>
### 对比 Claude Code 的 Concise 风格

2026 年 11 月 17 日，3 种配置回答了 10 个留出案例，每个 3 次试验，由 Claude Opus 5.5（`claude-opus-5-5`，高强度）在不带工具的 Claude Code 2.1.325 中生成并评审：Laconic 的精简规则、Claude Code 内置的 Concise 输出风格（`"outputStyle": "Concise"`）和简单指令 'Answer concisely.'。评审在打乱的字母标签下同时查看每个案例的 3 个回答，不知道配置名称，但能看出文风。它评审了两次：一次只看正确性，依据 `evals/rubric-correctness.md`，该标准要求不对文风打分；另一次依据 Laconic 自己的评分标准 `evals/rubric.md`，此前它与 10 个人工标注的校准回答全部一致。

| 指标 | Laconic | Concise 风格 | 'Answer concisely.' |
| --- | --- | --- | --- |
| 正确性，排除文风（1 到 5） | 4.80 | 4.27 | 4.47 |
| 与 Concise 风格的差值（95% 区间） | +0.53（0.00 到 +1.17） | | +0.20（−0.13 到 +0.53） |
| 阻断问题，排除文风 | 0 | 4 | 1 |
| 加权分，Laconic 自己的评分标准（1 到 5） | 4.69 | 4.06 | 4.11 |
| 与 Concise 风格的差值（95% 区间） | +0.63（+0.29 到 +1.00） | | +0.05（−0.10 到 +0.22） |
| 阻断问题，Laconic 自己的评分标准 | 1 | 13 | 10 |
| 没有 `laconic-check` 0.5.0 问题的回答 | 30 个中 24 个 | 30 个中 16 个 | 30 个中 13 个 |
| 通过的正则检查 | 12 个中 9 个 | 12 个中 9 个 | 12 个中 9 个 |
| 平均回答长度（字符） | 782 | 614 | 627 |
| 平均输出 token，含思考 | 475 | 327 | 298 |
| 每个回答的平均成本，单次冷调用，按 Claude Code 报告 | US$0.022 | US$0.019 | US$0.016 |
| Laconic 相对它的成本（95% 区间） | | 1.18 倍（1.07 到 1.29） | 1.39 倍（1.30 到 1.50） |

Laconic 自己的评分标准会给澳大利亚英语和本项目细则打分，而只有 Laconic 要求这些；评审不仅在加权分中，也在正确性和阻断问题中应用了它们，因此评审行中只有前 3 行与文风无关。只看正确性，Laconic 比 Concise 风格高 0.53，但在 10 个案例上区间触及零差异，所以这并非明确领先；Laconic 没有阻断问题，Concise 风格有 4 个。第一次评审的记录显示：Concise 风格在 3 封工资邮件中的 2 封里把 2026 年 11 月 26 日或 28 日的星期几写错，在第三封里把截止日期埋没了；在 3 份审查说明中的 2 份里声称了任务并未证实的测试覆盖率；还有一次错误引用了被审核的文本；'Answer concisely.' 也有类似疏漏。Laconic 的输出 token 比 Concise 风格多 46%，每个回答的成本高 18%。在 Laconic 规则要求使用本项目连字符的 '2026–27' 中，3 种配置都保留了 Style Manual 的短破折号，因此都没通过该案例的正则检查。

局限：每种配置 30 个回答，来自同一个模型，它也负责评审，案例由 Laconic 作者编写；排除文风的评审没有校准集。每个区间都对 10 个案例重抽样（10,000 次，种子 2282）。每次调用都把提示写入了提示缓存，因此成本行是单次冷调用的数据；若像会话中那样缓存规则，Laconic 多出的输出很可能占更大比重，但本次运行没有测量这一点。Claude Code 及其 Concise 风格面向软件工程会话，而这些案例大多是写作和咨询任务。运行脚本、逐个回答的得分、token 数和成本，以及重新计算此表的脚本都在 [`evals/runs/concise-style-2026-11-17`](evals/runs/concise-style-2026-11-17/)；回答本身未保存。

<a id="token-cost-against-caveman-and-chisle"></a>
### 与 caveman 和 Chisle 的 token 成本对比

2026 年 11 月 17 日，3 套规则回答了 `evals/cases.jsonl` 中全部 47 个案例，由 Claude Opus 5.5（`claude-opus-5-5`，高强度）在不带工具的 Claude Code 2.1.325 中生成并评审：Laconic 的精简规则、[caveman](https://github.com/JuliusBrussee/caveman) 4.1.0 的 `skills/caveman/SKILL.md` 和 [Chisle](https://github.com/JayPokale/Chisle) 3.12.1 的 `skills/chisle/SKILL.md`，每套都去掉 front matter 后追加到系统提示中。冷调用（2 次试验）各自从新的工作目录运行，因此会把规则写入提示缓存；缓存调用（1 次试验，不评审）在一次预热调用之后从同一目录运行，因此像同一会话中的调用那样从缓存读取规则。评审在打乱的字母标签下同时查看每个案例的 3 个冷调用回答，其提示中没有点名任何规则集，但它能看出文风，而且 Laconic 的 94 个回答中有 3 个提到了 Laconic：2 个是引用了工作目录，1 个是说出了技能名称。它评审了两次：一次只看正确性，依据要求不对文风打分的 `evals/rubric-correctness.md`；另一次依据 Laconic 自己的评分标准 `evals/rubric.md`。

| 指标 | Laconic | caveman | Chisle |
| --- | --- | --- | --- |
| 平均输出 token，含思考，冷调用 | 745 | 655 | 516 |
| Laconic 相对它的输出 token（95% 区间） | | 1.14 倍（1.04 到 1.27） | 1.44 倍（1.27 到 1.68） |
| 平均输入 token，冷调用 | 3,218 | 4,285 | 3,861 |
| 平均输出 token，含思考，缓存调用 | 780 | 637 | 492 |
| 每个回答的平均成本，规则从缓存读取 | US$0.018 | US$0.016 | US$0.013 |
| Laconic 相对它的成本，规则从缓存读取（95% 区间） | | 1.17 倍（1.05 到 1.29） | 1.45 倍（1.25 到 1.61） |
| 每个回答的平均成本，单次冷调用，含缓存写入 | US$0.028 | US$0.031 | US$0.026 |
| Laconic 相对它的成本，冷调用（95% 区间） | | 0.89 倍（0.83 到 0.94） | 1.05 倍（0.96 到 1.15） |
| 平均回答长度（字符），冷调用 | 1,297 | 1,198 | 977 |
| 正确性，排除文风（1 到 5） | 4.68 | 4.44 | 4.38 |
| Laconic 的差值（95% 区间） | | +0.24（+0.06 到 +0.45） | +0.30（+0.10 到 +0.53） |
| 阻断问题，排除文风 | 2 | 5 | 7 |
| 加权分，Laconic 自己的评分标准（1 到 5） | 4.63 | 4.11 | 4.11 |
| 阻断问题，Laconic 自己的评分标准 | 8 | 43 | 44 |
| 通过的正则检查，冷调用 | 18 个中 18 个 | 18 个中 16 个 | 18 个中 16 个 |
| 没有 `laconic-check` 0.5.0 问题的回答，冷调用 | 94 个中 70 个 | 94 个中 36 个 | 94 个中 39 个 |

Laconic 写得最多：输出 token 比 caveman 多 14%，比 Chisle 多 44%。在冷调用中，把 caveman 较长的规则写入缓存的成本超过了 Laconic 多出的输出，因此用 Laconic 的回答便宜 11%；规则从缓存读取时（与会话中一样），Laconic 多出的输出使回答比 caveman 贵 17%，比 Chisle 贵 45%。只看正确性，Laconic 比 caveman 高 0.24，比 Chisle 高 0.30；它在排除文风时的 2 个阻断问题是一个算术错误和一个引错的 RFC 7636 测试值。Laconic 自己的评分标准还把两款对比工具都不要求的澳大利亚英语和本项目细则计入正确性和阻断问题，而按此标准，对比工具的阻断问题大多与此有关：连续逗号、双引号、用单词写的数字和美式拼写。在那次评审中，两款对比工具在禁止给出典型时间范围的案例里，两个回答都给出了时间范围，而 Laconic 一个也没给；Laconic 的阻断问题包括：一个回答在被告知没有工具后，没有编辑任务说它能访问的文件（Chisle 也有一个回答如此），一个回答有重大算术错误，还有 2 个回答添加了任务禁止的局限性免责声明。

局限：只用一个模型，它也负责评审回答，而这些案例也指导过 Laconic 规则的措辞；排除文风的评审没有校准集。token 和成本数据是 Claude Code 对每次调用的报告。caveman 自己的评估是用其技能替换系统提示，而本次运行是追加。两款对比工具对智能体读取内容的压缩都没有被测试，两者都用自己的提示和模型发布数据，本次运行没有复现这些数据。区间对 47 个案例重抽样（10,000 次，种子 2282）。运行脚本、逐个回答的得分、token 数和成本，以及重新计算此表的脚本都在 [`evals/runs/token-cost-2026-11-17`](evals/runs/token-cost-2026-11-17/)；回答本身未保存。

<a id="against-other-humanisers"></a>
### 对比其他去 AI 味工具

2026 年 10 月 20 日，3 种已安装的配置为一位澳大利亚读者润色了 20 篇 AI 写的文本（`evals/humanise-cases.jsonl`：客户邮件、一份 PR 描述、发布说明、一份董事会报告、一则招聘广告等），每篇 2 次试验，由 Claude Opus 5.5 生成并盲评。每种配置只有自己的技能，配有 Skill 和 Read 工具，并被要求使用它：Laconic 的写作技能（提交 `c7e3048`）、[humanizer](https://github.com/blader/humanizer)（提交 `8c307f3`）和 [avoid-ai-writing](https://github.com/conorbronsdon/avoid-ai-writing)（提交 `cb16d10`）。

| 指标 | Laconic | `humanizer` | `avoid-ai-writing` |
|---|---|---|---|
| 加权分，中立评分标准（1 到 5） | 4.92 | 4.54 | 4.50 |
| 加权分，本项目评分标准 | 4.87 | 4.50 | 4.42 |
| 正确性，中立评分标准 | 4.80 | 4.13 | 4.18 |
| 简洁性，中立评分标准 | 4.95 | 4.20 | 3.95 |
| 阻断问题 | 0 | 0 | 0 |
| 通过的正则事实与模式检查（见下文） | 40 个中 40 个 | 40 个中 40 个 | 40 个中 35 个 |
| 相对原文的平均长度，按单词计 | 65% | 81% | 81% |
| 每篇文本的 avoid-ai-writing 检测器问题数（原文：2.6） | 0.03 | 0 | 0.05 |
| 每篇文本的 dslop 问题数，含节奏指标（原文：1.80） | 0.50 | 0.83 | 0.63 |
| 每篇文本的 `laconic-check` 0.1.0 问题数（原文：1.85） | 0.15 | 0.45 | 0.28 |
| 生成 40 篇文本的成本 | $4.54 | $5.84 | $15.78 |

在中立评分标准下（去掉了 Laconic 的本项目细则，即数字、连续逗号和长破折号，但仍要求澳大利亚拼写），Laconic 的加权分比 `humanizer` 高 0.38，比 `avoid-ai-writing` 高 0.43。用配对自助法对 20 篇文本（连同两次试验）重抽样得到的 95% 区间分别为 +0.26 到 +0.51 和 +0.26 到 +0.59。把这些细则从评分标准中去掉后，领先依然存在。评审对其他配置的记录包括原文没有的内容，例如求职信中添加的一段经历，以及为一份经费申请推算出的美元数额。在这个样本中，3 种配置都把这些检测器的问题数降到远低于原文的水平。

<details>
<summary>正则检查是如何修订的</summary>

在检查了因忠实改写（'in stages'、'a second person will check'、'five to seven'、'Don't install any personal'）而未通过的回答后，有四项正则检查被放宽。同样的修订检查应用于所有配置，表中给出的是修订后的结果；修订前，40 篇中 Laconic 通过 37 篇，`humanizer` 通过 34 篇。`laconic-check` 一行来自 0.1.0 版本；之后的版本增加了规则，会计出更多问题。

</details>

局限：每种配置 40 篇文本，来自单一生成模型，由同一模型家族评审，基于这 20 个提示和这些已安装配置。每个检查器都反映其作者的偏好，`avoid-ai-writing` 检测器就是该技能自己的引擎。通过正则检查只能部分证明事实得以保留，不能证明每个限定词都保留了下来。复现命令见 [evals/README.md](evals/README.md)。

<a id="fact-changes-on-a-public-set"></a>
### 公开数据集上的事实变化

2026 年 11 月 14 日，Laconic 的写作技能和 [humanizer](https://github.com/blader/humanizer) 4.0.0 各自改写了 [AI Humanizer Benchmark](https://huggingface.co/datasets/ai-humanizer-benchmark/ai-humanizer-benchmark) 2026 年 9 月至 11 月各期的 99 篇公开文本，每篇 2 次试验，分 2 轮使用不同模型。外部检查器 [humanizer-fact-audit](https://github.com/zwang-devv/humanizer-fact-audit) 统计了至少丢失原文中一个数字、日期、人名或机构名，或 3 个及以上单词的引文的改写。方法在任何改写之前就已提交。

| 轮次 | Laconic | `humanizer` | 差值（95% 区间） |
|---|---|---|---|
| Claude Opus 5.5 | 134 篇中 18 篇（13.43%） | 134 篇中 10 篇（7.46%） | +5.97 个百分点（0.00 到 +12.32） |
| `gpt-6.1-sol`（Codex） | 134 篇中 20 篇（14.93%） | 134 篇中 20 篇（14.93%） | 0.00 个百分点（−2.21 到 +2.14） |

在 Opus 5.5 下，Laconic 丢失单元的改写比 `humanizer` 多（18 对 10），而且每一组丢失的单元都更多：数字和日期 440 个中 9 个对 5 个，名称 164 个中 12 个对 9 个，引文 70 个中 5 个对 0 个。在 `gpt-6.1-sol` 下2 款技能持平。两个区间都不排除 0，因此两轮都没有显示明确差异。两轮中 Laconic 的改写都更短：分别为原文单词数的 0.94 和 0.90，对方为 1.01 和 0.93。

局限：检查器比对的是单元，可能把无害的引文改写标为问题；它看不到被改动的条件、否定或编造的说法，因此不是对事实忠实度的完整检验。99 篇文本中有 32 篇没有它可检查的单元，因此比例覆盖每款技能 134 篇改写。每轮用一个模型改写，每个新上下文 3 篇文本；指令以文本形式提供，而非已安装的插件。Opus 一轮是在 Codex 一轮之后加的，方法未变。对比工具由 Laconic 作者选择，方法由其编写，两轮也由其运行。固定版本见 [evals/README.md](evals/README.md#public-set-fact-check)。

在 2026-10 期的 33 篇文本上，由 Claude Opus 5.5 于 2026 年 11 月 17 日每篇改写一次、每次调用一篇，Laconic 的技能在含有单元的 20 篇文档中改动了 5 篇的某个单元，`humanizer` 也是 5 篇；avoid-ai-writing 和 `work-humanizer` 各 1 篇，带 `--verify` 的 patina 为 0 篇，只给请求时为 8 篇；11 款商业去 AI 味工具改动了 7 到 13 篇。每篇只改写一次，数量太少，无法显示技能之间的差异（[按工具](evals/README.md#by-humaniser)）。

<a id="checker"></a>
## 检查器

`tools/laconic-check` 是一个没有依赖的小型 Rust CLI。`lint` 会在 Markdown 文字中标出候选问题：长破折号、美式拼写、用作连词和介词的 'while' 和 'among'、双引号、疑似连续逗号、用单词写的数字、'5 %' 或 '$ 27.99' 中的空格、不符合 Style Manual 的日期和时间写法（如 '21st of September' 和年代 '1980's'）、与日期不符的星期几、拉丁缩写、聊天机器人式结束语、'Bottom line:' 或 'circle back' 之类的套话、刻意对比（'it's not X, it's Y'；'not just X but Y'）、无动词的揭晓式问句（'The result? A 40% cut.'）、'since the dawn of time' 之类的空泛开头，以及 'open-source' 或 'e-mail' 之类的多余连字符；它会跳过代码、URL 和引文。它还会标出聊天工具的残留，如零宽字符、引用标记和 ChatGPT 跟踪标签（包括链接内的），以及能隐藏或重排文字的不可见字符：成串的 Unicode 标签字符（英格兰、苏格兰和威尔士旗帜除外）和双向控制字符。作为永远不会使 `lint` 失败的建议，它会标出：在 2 个或更多句子中使用插入语破折号（长破折号，或两侧有空格的短破折号、连字符或双连字符）的段落，其中围住同一插入语的一对只算一次；'could potentially' 和 'may possibly' 之类叠加的含糊措辞；以及连续 3 个或更多以粗体标签开头的列表项（'**Cause:** ...'）。`preserve` 报告编辑删掉的数字、大小和单位（'$47.3 million' 变为 '$47.3 billion'，'150 km' 变为 '150 miles'）、带日期的月份、'Thursday' 之类的星期名（自 0.6.0 起）、行内代码、URL、引文、'not'、'must'、'will'、'if'、'before'、'likely' 之类的词，以及 'up to' 和 'about 50%' 之类的界限，并因此判为失败。使用 `--loose` 时，表示频率、数量、能力、预测、原因或含糊语气的丢失词（如 'often'、'some'、'can'、'will'、'because' 或 'likely'）改为列出供检查，因为大幅改写常会去掉这样的词而不改变事实；丢失的含糊词仍可能夸大说法，因此请逐一阅读列出的词（自 0.6.0 起）。数字会保留其符号、货币，以及金额或千位分隔数字外的会计括号，所以 '-3%' 变为 '3%'、'(1,200)' 变为 '1,200'、'$500' 变为 '£500' 都算改动；其他带括号的数字（如 '(62%)'）属于插入语。货币是数字前的符号（'$'、'£'、'¥'，或 Unicode Currency Symbols 区块中的符号，如 '€'、'₹' 和 '₩'），或数字后的 'dollars'、'euros' 或 'yen'，所以 '$30 billion' 和 '30 billion dollars' 相匹配。数字后的 'Pounds' 在任一文本把 '£' 与该数字一起写出时算作金额，否则算作重量。货币单词、Currency Symbols 区块的其余部分和 'pounds' 自 0.6.0 起支持。它把 'three' 和 '3'、'1000' 和 '1,000'、'5 per cent' 和 '5%' 视为同一数值，把 '45 to 60 minutes' 或 '45-60 minutes' 的两个界限都读作分钟（自 0.6.0 起），把 'didn't' 和 'did not' 视为相同，但区分 'can't' 和 'won't'，把 '5:00 p.m.' 和 '5pm' 视为同一时刻；不过 '5:30' 这样带冒号的数字只有在 'at' 或 'until' 之类的词之后、'am' 或 'pm' 之前，或在跟随某个时间的 'to' 或 'and' 之后才算时间，否则算作 2 个数字，如比例（自 0.6.0 起）；它会忽略最多 2 位数的有序列表标记，包括粗体的。它也会把编辑新增的内容（如编造的数字）作为建议列出；对于不得增加任何内容的改写，`--strict` 也会因此判为失败。编辑丢失或新增的由 2 或 3 个首字母大写单词组成的名称会列出供检查，但永远不会导致失败。可能的反转也是如此：编辑中的某句不带否定地陈述了原文某句所否定的内容，如 'is not cast' 变为 'is cast'；删掉刻意对比中被否定的那一半也会同时丢掉否定词，因此不会列出。在 AI Humanizer Benchmark 2026-10 和 2026-11 期的 693 篇商业改写中，它列出了 4 处，全是误报：2 篇改写用别的说法保留了原意（'So stop guessing'；'Otherwise ...'），2 篇匹配到了表达另一主张的句子（自 0.6.0 起）。它的最后一行给出两份文本按空白分隔的单词数，以及编辑稿占原文的比例。它并不理解语义，所以结果干净只说明排除了上述这些丢失。一条问题提示只是值得查看的理由，并不证明存在缺陷。

```bash
cargo run --manifest-path tools/laconic-check/Cargo.toml -- lint README.md
cargo run --manifest-path tools/laconic-check/Cargo.toml -- preserve original.md edited.md
```

`lint` 和 `preserve` 也被编译为 WebAssembly，可在浏览器中运行于 [yungsnat.github.io/Laconic](https://yungsnat.github.io/Laconic/)，从 `main` 构建。该页面是单个 HTML 文件，其内容安全策略会阻止一切网络请求，因此粘贴进去的文字不会离开本机；`web/index.html` 是其源文件，`tools/laconic-web` 是 WebAssembly crate。构建方法：

```bash
rustup target add wasm32-unknown-unknown
cargo build --release --target wasm32-unknown-unknown --manifest-path tools/laconic-web/Cargo.toml
cargo run --manifest-path tools/laconic-tools/Cargo.toml -- web-page tools/laconic-web/target/wasm32-unknown-unknown/release/laconic_web.wasm laconic.html
```

每个 `laconic-check-v*` 发布版都附有 Linux（x86_64，静态链接）、Windows（x86_64）和 macOS（Apple silicon）的预编译二进制文件及 SHA-256 校验和；把二进制文件放到 PATH 中即可。从源码构建只需要 Rust。

对已经使用 [Vale](https://vale.sh) 的项目，`vale/Laconic` 是根据 `lint` 规则表生成的 Vale 样式，作为 `Laconic.zip` 包附在每个发布版中（自 0.6.0 起）。它涵盖可用正则表达式表达的规则；[vale/README.md](vale/README.md) 列出了无法表达的 7 条规则，以及其余规则与 `lint` 的不同之处。

自 0.4.0 起的发布版还为每个归档附带构建来源证明，以及 Scoop、Homebrew、winget 和 MCP Registry 的清单。其 MCP 包 `laconic-check.mcpb` 可一步安装到 Claude Desktop。用 `gh attestation verify <archive> --repo yungSNAT/Laconic` 校验下载的归档。Scoop 可直接从发布版安装：

```bash
scoop install https://github.com/yungSNAT/Laconic/releases/latest/download/laconic-check.json
```

Homebrew 配方和 winget 清单是为 tap 和 winget 目录附上的，但 tap 和 winget 目录目前还没有收录 laconic-check。

[docs/rules.md](docs/rules.md) 按名称介绍每条规则，附有示例、对应的 Style Manual 章节以及它标记人类文字的频率；它还列出检查器覆盖了 Style Manual 的哪些章节。误报请通过[问题表单](https://github.com/yungSNAT/Laconic/issues/new?template=false-positive.yml)报告。[docs/humanizer-coverage.md](docs/humanizer-coverage.md) 把 `humanizer` 4.0.0 中的 26 种模式逐一对应到涵盖它的规则和写作技能章节：技能涵盖全部 26 种，`lint` 能以某种形式标出其中 8 种。

要在 CI 或 pre-commit 钩子中把 `lint` 用作门禁，以下选项决定它报告什么、何时失败：

| 选项 | 作用 |
| --- | --- |
| `--json` | 输出一个 JSON 对象：`findings`，每项含 `file`、`line`、`column`、`rule`、`advisory`、`message` 和 `url`，然后是 `failing` 计数和被注释 `suppressed` 的问题数。 |
| `--sarif` | 为 GitHub code scanning 和其他 SARIF 查看器输出 [SARIF 2.1.0](https://docs.oasis-open.org/sarif/sarif/v2.1.0/sarif-v2.1.0.html) 日志：包含每条规则及其描述和链接，每个问题作为级别为 `warning`（若为建议则为 `note`）的结果，用文件的绝对 `file` URI 定位（code scanning 会把它映射到仓库路径），列按 SARIF 的计数方式以 UTF-16 码元计。不能与 `--json` 同用；自 0.6.0 起。 |
| `--disable <rules>` | 丢弃这些规则的问题；用逗号分隔，可重复。 |
| `--only <rules>` | 只保留这些规则的问题；`--disable` 优先。 |
| `--max-findings <n>` | 只有失败问题多于 n 个时才失败（默认 0）。 |
| `--warn` | 永不因问题而失败。 |
| `--commit-msg` | 把每个文件当作 Git 提交信息读取，忽略注释行和剪刀线以下的内容。 |
| `--config <file>` | 从此文件而不是 `.laconic-check` 读取设置。 |

在运行 `lint` 的目录中，`.laconic-check` 文件包含 `disable`、`only` 和 `max-findings` 的 `key = value` 行，以及 `#` 注释行。`accept` 行列出 `us-spelling` 不再标记的纯字母单词，每个词按整词并区分大小写匹配，因此 `accept = color` 仍会标记句首的 'Color'（自 0.6.0 起）。命令行上的规则会追加到其列表中，`--max-findings` 会替换其计数：

```text
# The CSS property names color and center are expected here.
accept = color, center
# This project uses serial commas.
disable = serial-comma
```

在设置之后，`[rules]` 部分存放项目自己的规则，每行一个短语：`id: phrase = replacement` 建议替换词，`id: phrase =` 禁用该短语（自 0.6.0 起）。每个问题把其规则命名为 `project/<id>`，`disable`、`only`、`--disable`、`--only` 和抑制注释都接受它；多行可以共用一个 id。短语是由字母和数字组成的一个或多个单词，单词内部可以有连字符或撇号。它在正文中按整词匹配，可以跨越换行，但从不在代码、URL 或引文中匹配。全小写时不区分大小写；含大写字母时只按原样匹配。与替换词完全一致的文字不会被标记，所以 `github = GitHub` 不会动 'GitHub'。项目问题与 Laconic 自身的问题一样会使 `lint` 失败。它们没有规则参考链接：在 `--json` 中 `url` 为 `null`，SARIF 日志把它们的规则列在 Laconic 规则之后，不带链接。格式错误的行或未知部分会让 `lint` 停止并给出该行的行号：

```text
[rules]
plain-words: utilise = use
plain-words: leverage = use
filler: going forward =
names: Javascript = JavaScript
```

要屏蔽某个问题，在它之前单独一行写一条注明其规则的注释。它只覆盖下一个非空行；对于每个没有屏蔽任何内容的规则名，`lint` 都会报告 `unused-suppression`，即使该注释屏蔽了另一条规则：

```markdown
<!-- laconic-check-disable-next-line us-spelling -->
Set the color property of the heading to red.
```

| 退出状态 | 含义 |
| --- | --- |
| 0 | `lint` 找到的失败问题不多于 `--max-findings` 允许的数量，或以 `--warn` 运行；`preserve` 没有发现删除，在 `--strict` 下也没有发现新增；`mcp` 读到输入末尾；`lsp` 在 `shutdown` 之后收到 `exit`；`hook` 回应了其事件；`--version` 或 `--help` 输出了其文本。 |
| 1 | `lint` 找到的失败问题多于允许的数量；`preserve` 发现有内容被删除，或在 `--strict` 下有内容被新增；`lsp` 未经 `shutdown` 就收到 `exit`，或其输入结束；`hook` 无法读取其事件、项目文件或未提交的 Markdown 文件，或 Git 因找不到仓库以外的原因失败。 |
| 2 | 用法错误；`hook` 以外的命令出现项目文件错误或读取错误；或 `mcp`、`lsp` 出现输入输出错误。出现读取错误后，`lint` 仍会检查其他文件（自 0.6.0 起）。 |

要通过 [pre-commit](https://pre-commit.com) 运行，把 `laconic-check` 放到 PATH 中并添加其钩子；提交信息钩子还需要 `pre-commit install --hook-type commit-msg`，以及 laconic-check 0.4.0 或更高版本，因为 0.3.0 没有 `--commit-msg`：

```yaml
repos:
  - repo: https://github.com/yungSNAT/Laconic
    rev: <commit>
    hooks:
      - id: laconic-check
      - id: laconic-check-commit-msg
```

在 GitHub Actions 中，组合 Action 会为运行器（Linux x86_64、Windows x86_64 或 Apple silicon 上的 macOS）下载 laconic-check 发布版，用发布版的 SHA-256 文件校验，然后检查并为每个问题添加注解。校验和能发现损坏或被替换的下载，但不能发现被攻破的发布版：

```yaml
- uses: actions/checkout@v7
- uses: yungSNAT/Laconic@<commit>
  with:
    version: 0.6.0     # the laconic-check release; 0.6.0 by default
    files: README.md   # one path a line; default: every .md and .markdown file Git tracks
    args: --warn       # further lint options
```

若想改为在 GitHub code scanning 中列出问题，把 `laconic-check` 放到 PATH 中（在某个发布版包含 `--sarif` 之前，用 `cargo install --locked --git https://github.com/yungSNAT/Laconic --rev <commit> laconic-check`），给作业 `security-events: write` 权限并上传日志。使用 `--warn` 时，问题不会使该步骤失败，因此它只会因用法或读取错误而失败，这时也不会上传不完整的内容；如果也想因问题使作业失败，请添加上面的 Action 或另一个不带 `--warn` 的 `lint` 步骤：

```yaml
- run: laconic-check lint --sarif --warn README.md docs/*.md > laconic-check.sarif
- uses: github/codeql-action/upload-sarif@v4
  with:
    sarif_file: laconic-check.sarif
```

`lint -` 读取标准输入，因此智能体无需保存就能检查草稿。当 `laconic-check` 在 PATH 中时，两个技能都会在发送书面成果前运行它。它的美式到澳式单词表合并了一份手写列表，以及约 2,400 对从 [English Speller Database](https://github.com/en-wl/wordlist) 生成、并对照 Style Manual 审核过的词（`laconic-tools gen-au-words`）。植入缺陷测试会在干净的文字中为每条规则植入一个已知缺陷，要求检查器把每个缺陷恰好找出一次，且在干净文本上没有任何问题。校准测试在 2018 年和 2021 年发布的 3 份 Productivity Commission 报告（约 198,500 个单词的人类文字，CC BY 3.0 AU）上运行 `lint`，如果其 2,246 个问题、67 条建议（41 个插入语破折号段落、8 处叠加含糊措辞、14 个在定义前使用的缩略语，以及 4 个既写作合写又写作带连字符的复合词），或其中的 7 处套话、刻意对比和多余连字符有所增加，测试就会失败，以防人类文字上的问题出现净增长。2026 年 11 月 5 日的一次外部运行检查了 [jenna-russell/human_detectors](https://github.com/jenna-russell/human_detectors) 中提交 `afcf03d` 的 150 篇人类撰写的文章（110,723 个单词，大多是美式非虚构作品，未保存在此）：每 1,000 词 25.9 个问题，其中 97% 是本项目文风差异（双引号、美式拼写、长破折号、用单词写的数字、用词选择和连续逗号）。AI 痕迹规则触发了 18 次（每 1,000 词 0.16 次：8 处刻意对比、7 处套话、2 个揭晓式问句和 1 句聊天机器人式结束语），尚未标注真伪。这些文章是纯文本，所以这次运行没有测试 `bold-label`。

2026 年 11 月 13 日，`preserve` 在 [AI Humanizer Benchmark](https://huggingface.co/datasets/ai-humanizer-benchmark/ai-humanizer-benchmark) 2026-10 期中 11 款商业去 AI 味工具的 363 篇改写上运行，其结果与论文 [Humanized but Still True?](https://arxiv.org/abs/2611.00359) 随附发布的逐篇事实判定进行了比较。[evals/README.md](evals/README.md#fact-audit-agreement) 给出固定版本和命令：

| 论文标记的文档类型 | 文档数 | laconic-check 0.4.0 | laconic-check 0.5.0 | 带 `--loose` 的 `main` |
| --- | --- | --- | --- | --- |
| 有任何单元被改动 | 105 | 105 篇失败 | 105 篇失败 | 105 篇失败 |
| 数字被改动 | 47 | 44 篇报告删除了数字或数量 | 43 | 43 |
| 3 个及以上单词的引文被改动 | 58 | 58 篇报告删除了引文 | 58 | 58 |
| 名称被改动 | 28 | 20 篇列出待查名称 | 20 | 20 |
| 所有单元都保留 | 258 | 17 篇通过；224 篇只因语义词失败；0 篇因月份、行内代码或 URL 失败；17 篇因数字、数量或引文失败 | 23 篇通过；222；0；13 | 64 篇通过；182；0；12 |

在同样的文档上，avoid-ai-writing 的 `validate.js` 在 105 篇改动文档中判 23 篇失败，Zero Slop 的忠实度检查判全部 105 篇失败；在 258 篇保留文档中，它们分别通过 233 篇和 15 篇，而 `preserve` 通过 23 篇，带 `--loose` 时通过 64 篇（[evals/README.md](evals/README.md#against-other-fact-checkers)，2026 年 11 月 16 日）。按论文的表 1，每款工具在其改写的、含有单元的 20 篇文本中，有 7 到 13 篇（35% 到 65%；Undetectable AI 为 20 篇中 11 篇）至少改动了一个单元；[evals/README.md](evals/README.md#by-humaniser) 给出包含 `preserve` 对每款工具结果的表格。

`laconic-check mcp` 通过标准输入输出把 `lint` 和 `preserve` 作为 MCP 工具提供，无需另外安装任何东西。`preserve` 工具接受 `loose: true`，作用与 `--loose` 相同；`lint` 工具与 `lint` 一样，应用服务器运行目录中的 `.laconic-check` 文件，每次调用都重新读取；文件格式错误时，调用会返回一个指出其行号的错误（两者均自 0.6.0 起）：

```bash
claude mcp add --scope user laconic-check -- laconic-check mcp
```

它包含在 laconic-check 0.3.0 及更高版本中。laconic-check 0.2.0 二进制文件需要早先的 Node 服务器 `tools/laconic-check/mcp/server.mjs`，它每次调用运行一次二进制文件；该服务器已弃用，将在以后的版本中移除：

```bash
claude mcp add --scope user laconic-check -- node /path/to/laconic/tools/laconic-check/mcp/server.mjs
```

`laconic-check lsp` 是通过标准输入输出运行的语言服务器，让编辑器在你输入时显示 `lint` 的问题（自 0.6.0 起）。每当打开的 Markdown 文档发生变化，它就检查一遍，并把每个问题标为警告（若为建议则为提示），链接到对应规则。它应用包含该文档的工作区文件夹（文件夹嵌套时取最内层）中的 `.laconic-check` 文件，并把无法读取或解析的文件作为错误显示在第一行。0.6.0 之后，凡规则知道唯一替换写法之处，它还提供快速修复，例如美式拼写、用文字写的数字、拉丁缩写、时间或日期格式、双引号和不可见字符。修复只改动被标出的文字，而且只在你选择时才应用（Neovim 中为 `gra`，Helix 中为 `space a`）。需要改写的规则，例如长破折号、套话和可能的连续逗号，不提供修复。编辑器必须应用带版本号的编辑，Neovim 和 Helix 都是如此，因此在文本改动之后才到达的修复会被拒绝，而不会落在错误的位置。在 Neovim 0.11 或更高版本中（它会把包含 `.laconic-check` 或 `.git` 的文件夹作为工作区文件夹）：

```lua
vim.lsp.config('laconic_check', {
  cmd = { 'laconic-check', 'lsp' },
  filetypes = { 'markdown' },
  root_markers = { '.laconic-check', '.git' },
})
vim.lsp.enable('laconic_check')
```

在 Helix 的 `languages.toml` 中，该列表会替换 Helix 默认的 Markdown 服务器，因此请把你使用的其他服务器也加进去：

```toml
[language-server.laconic-check]
command = "laconic-check"
args = ["lsp"]

[[language]]
name = "markdown"
language-servers = ["laconic-check"]
```

`laconic-check hook` 是适用于 Claude Code 和 Codex 的 Stop 钩子，添加之前处于关闭状态（自 0.6.0 起）。当智能体即将结束本轮时，它会用仓库顶层的 `.laconic-check` 文件，检查包含会话目录的仓库中未提交的 Markdown（已暂存、未暂存和未跟踪的）。如果 `lint` 会失败，它会请智能体先查看失败的问题，并列出最多 50 个。每轮只请求一次，因此当某个问题是误报时，智能体仍可结束。它也会检查你自己未提交的编辑，而不只是智能体的。在 Claude Code 中，把下面的内容加到 `.claude/settings.json` 或 `~/.claude/settings.json`；Codex 从 `~/.codex/hooks.json` 读取同样的配置块，并会把新钩子标为不受信任，直到你在 `/hooks` 中审核它：

```json
{
  "hooks": {
    "Stop": [
      { "hooks": [{ "type": "command", "command": "laconic-check hook", "timeout": 60 }] }
    ]
  }
}
```

<a id="always-on"></a>
## 始终开启

要在每个 Claude Code 会话中加载精简规则，把它们复制到你的规则文件夹：

```bash
mkdir -p ~/.claude/rules && cp rules/laconic.md ~/.claude/rules/laconic.md
```

若要让每个 Claude Code 会话改为以完整会话模式启动，请创建一个空的标志文件：

```bash
touch ~/.claude/.laconic-always
```

删除该文件即可关闭会话模式。二者择一：会话模式已包含精简规则，同时加载两者会让每个会话重复约 240 个单词。其他智能体见 [INSTALL.md](INSTALL.md)。

<a id="development"></a>
## 开发

Codecov 徽章衡量的是 `laconic-evals`，即 `tools/laconic-evals` 中的评测框架。其他 Rust 检查在 CI 中运行。

```sh
cargo test --manifest-path tools/laconic-tools/Cargo.toml
cargo test --manifest-path tools/laconic-evals/Cargo.toml
cargo run --quiet --locked --manifest-path tools/laconic-evals/Cargo.toml -- validate
cargo test --manifest-path tools/laconic-check/Cargo.toml
```

验证、评估和发布门禁见 [CONTRIBUTING.md](CONTRIBUTING.md)。

<a id="credits"></a>
## 致谢

Laconic 派生自 Ayoub G. 的 [i-have-adhd](https://github.com/ayghri/i-have-adhd)（MIT），回答规则、平台适配器、钩子和评测框架均来自该项目。写作指南来自 No AI Slop，它基于 Peter Yang 的 No AI Slop 和 Hardik Pandya 的 Stop Slop，并改编了 [humanizer](https://github.com/blader/humanizer)、[avoid-ai-writing](https://github.com/conorbronsdon/avoid-ai-writing) 和 [Sepia](https://github.com/Nanako0129/sepia)（均为 MIT）中的内容。澳大利亚英语规则遵循 [Australian Government Style Manual](https://www.stylemanual.gov.au/)（copyright Commonwealth of Australia），由 Laconic 用自己的话重述；该手册未以开放许可证发布，Laconic 也不包含其副本。`laconic-check` 的校准语料是 3 份 [Productivity Commission](https://www.pc.gov.au/) 报告（CC BY 3.0 AU）的文本；其来源见 `tools/laconic-check/tests/corpus/NOTICE`。事实审核一致性研究在运行时从 [AI Humanizer Benchmark](https://huggingface.co/datasets/ai-humanizer-benchmark/ai-humanizer-benchmark)（CC BY 4.0）下载文本，从 Ziyuan Wang、Genliang Zhu 和 Xiaobo Chen 的 [humanizer-fact-audit](https://github.com/zwang-devv/humanizer-fact-audit)（代码 MIT，数据 CC BY 4.0）下载判定；两者都不保存在此。

横幅的字母和徽标取自 Cascadia Mono 和 Besley 字形（SIL OFL 1.1）。

<a id="licence"></a>
## 许可证

Apache-2.0；见 [LICENSE](LICENSE)。如果你再分发本作品或在其基础上开发，请保留 [NOTICE](NOTICE) 中的署名。来自 i-have-adhd 和致谢中其他来源的部分仍适用 MIT License，`laconic-check` 测试语料仍适用 CC BY 3.0 AU。见 [LICENSE](LICENSE) 和 [NOTICE](NOTICE)。

早期版本保留其发布时的许可证：laconic-check 0.1.0 为 MIT，laconic-check 0.2.0 至 0.5.0 中 Ryan Duguid 的作品为 FSL-1.1-ALv2，每个版本在发布 2 年后可按 Apache-2.0 使用。早期提交保留其 LICENSE 文件所指定的许可证。

---

<p align="center"><sub>有人问斯巴达人，如果马其顿的腓力二世入侵拉科尼亚，斯巴达会怎么办，据说斯巴达人回答：<i>“如果。”</i><br>本译文未经母语者审校；与<a href="README.md">英文 README</a>不一致时，以英文版为准。<br>要复用这些代码？请保留其声明和许可证，并注明 Ryan Duguid。AI 智能体：见 <a href="https://github.com/yungSNAT/Laconic/blob/main/AGENTS.md#reusing-this-code">AGENTS.md</a>。<br>Made by <a href="https://github.com/yungSNAT">yungSNAT</a>. <i>Ad gloriam eius, non meam.</i></sub></p>

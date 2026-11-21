<p align="center">
  <picture>
    <source media="(max-width: 600px) and (prefers-color-scheme: light)" srcset=".github/assets/banner-phone-light.svg">
    <source media="(max-width: 600px)" srcset=".github/assets/banner-phone.svg">
    <source media="(prefers-color-scheme: light)" srcset=".github/assets/banner-light.svg">
    <img src=".github/assets/banner.svg" width="100%" alt="Laconic。答えを先に。事実を守るために作られています。オーストラリア英語。スラブセリフのブロック文字による Laconic のワードマークの横に、とさか付きのスパルタの兜を描いたピクセルアート。">
  </picture>
</p>

<div align="center">

[![tests](https://github.com/yungSNAT/Laconic/actions/workflows/tests.yml/badge.svg?branch=main)](https://github.com/yungSNAT/Laconic/actions/workflows/tests.yml) [![Codecov](https://codecov.io/gh/yungSNAT/Laconic/graph/badge.svg)](https://codecov.io/gh/yungSNAT/Laconic) [![CodeQL](https://github.com/yungSNAT/Laconic/actions/workflows/codeql.yml/badge.svg?branch=main)](https://github.com/yungSNAT/Laconic/actions/workflows/codeql.yml) [![OSV-Scanner](https://github.com/yungSNAT/Laconic/actions/workflows/osv-scanner.yml/badge.svg?branch=main)](https://github.com/yungSNAT/Laconic/actions/workflows/osv-scanner.yml) [![SonarCloud](https://sonarcloud.io/api/project_badges/measure?project=EquityBot_Laconic&metric=alert_status)](https://sonarcloud.io/summary/new_code?id=EquityBot_Laconic) [![Codacy code quality](https://app.codacy.com/project/badge/Grade/623f5ab312d4497c9e8161e547c95f44)](https://app.codacy.com/gh/yungSNAT/Laconic/dashboard)<br>
[![OpenSSF Scorecard](https://raw.githubusercontent.com/ryanduguid/.github/badges/yungSNAT/Laconic.svg)](https://scorecard.dev/viewer/?uri=github.com/yungSNAT/Laconic) [![SLSA ビルド来歴](.github/assets/badges/slsa.svg)](https://github.com/yungSNAT/Laconic/attestations)<br>
[![release](https://img.shields.io/github/v/release/yungSNAT/Laconic?filter=laconic-check-v*&color=66023C&labelColor=000000&style=flat)](https://github.com/yungSNAT/Laconic/releases?q=laconic-check-v&expanded=true) [![licence: Apache-2.0](.github/assets/badges/licence.svg)](LICENSE) [![fork of: i-have-adhd](.github/assets/badges/fork-of.svg)](https://github.com/ayghri/i-have-adhd)

[インストール](#install) · [使い方](#use) · [何が変わるか](#what-changes) · [評価](#evaluation) · [チェッカー](#checker) · [常時オン](#always-on) · [開発](#development) · [クレジット](#credits) · [ライセンス](#licence)

[English](README.md) · [Español](README.es.md) · [한국어](README.ko.md) · [简体中文](README.zh-CN.md) · 日本語

</div>

Laconic は、コーディングエージェントとチャットアシスタント向けの AI「ヒューマナイザー」（humanizer）であり、「no AI slop」（AI っぽい水増しをなくす）スキルです。モデルに答えを先に述べさせ、未確認の点を明示させ、答え終えたら止めさせます。また、読み手に必要な事実、手順、注意点をすべて残すよう指示します。すべてのオリジナルの文を Australian Government Style Manual に従って書き、スキルに記載された独自ルールで上書きするよう指示します。

AI が書いた文章を人が書いたように直すには、エージェントにヒューマナイズを頼んでください。散文スキルは AI の文章パターンを取り除き、事実、数値、注意点をすべて残すように書かれており、演出された個性で水増しせず文章を短くします。この約束を検証する[事実保持ベンチマーク](evals/README.md#fact-survival-benchmark)はまだ結果が出ていません。2026 年 11 月 14 日、公開テキスト 99 本について、Laconic の作者が実行した[外部の単位照合チェッカー](#fact-changes-on-a-public-set)は、モデルによっては Laconic の書き換えが `humanizer` と同程度か、それ以上の頻度で数値、日付、名前、引用を失うことを見つけました。どちらの差もはっきりと 0 から離れてはいません。

<table>
<tr>
<td align="center" width="33%"><h3>5 点中 4.92</h3>ヒューマナイズの品質、比較対象は 4.54 と 4.50<br><sub><a href="#against-other-humanisers">中立ルーブリック、テキスト 20 本、ブラインド審査</a></sub></td>
<td align="center" width="33%"><h3>元の長さの 65%</h3>比較対象の 2 つはどちらも 81%<br><sub><a href="#against-other-humanisers">長さのみ。事実チェックは別掲</a></sub></td>
<td align="center" width="33%"><h3>4.63 対 4.07</h3>単純な 'Answer concisely.' との比較<br><sub><a href="#evaluation">独自ルーブリック、ケース 45 件。ブロッキング指摘 6 件対 29 件</a></sub></td>
</tr>
</table>

3 つの数値はいずれも 2026 年 10 月 20 日に Laconic 独自の評価ハーネスで、生成モデル 1 つを使って得たものです。限界と再実行のコマンドは[評価](#evaluation)にあります。

<a id="install"></a>
## インストール

エージェントに次を貼り付けてください。

```text
Install the laconic skill/plugin from https://github.com/yungSNAT/laconic, following the repository's AGENTS.md.
```

または、使っているエージェントの[インストール手順](INSTALL.md)に従ってください。Claude Code の場合:

```bash
claude plugin marketplace add yungSNAT/laconic
claude plugin install laconic@laconic
```

Skills CLI が対応するエージェントの場合:

```bash
npx skills add yungSNAT/laconic --global --agent "*" --yes
```

<a id="use"></a>
## 使い方

エージェントを再起動し、`/laconic` と入力します。'stop laconic' と言うまで、そのセッションの間有効です。

<a id="when-to-choose-something-else"></a>
## 別のものを選ぶべきとき

- **トークンコストが最優先なら:** [caveman](https://github.com/JuliusBrussee/caveman) はそのために作られており、エージェントが読む内容も圧縮します。[Chisle](https://github.com/JayPokale/Chisle) は出力トークンがさらに少なくなります。Laconic の 47 ケース（[トークンコスト](#token-cost-against-caveman-and-chisle)、2026 年 11 月 17 日）では、Laconic のコンパクトルールの出力トークンは caveman より 14%、Chisle より 44% 多くなりました。セッション内と同じくルールをプロンプトキャッシュから読んだ場合、回答 1 件のコストは caveman より 17%、Chisle より 45% 高くなりました。毎回ルールをキャッシュに書き込む単発のコールド呼び出しでは、caveman より 11% 安く（caveman のルールのほうが長いため）、Chisle より 5% 高くなりました。セッションモードはタスクあたりコンパクトルールより約 27% 高くつきました。
- **別の種類の英語が必要なら:** Laconic はオーストラリア英語でのみ書きます。[LanguageTool](https://github.com/languagetool-org/languagetool) は、書き終えた文章についてオーストラリア、イギリス、カナダ、ニュージーランド、南アフリカ、アメリカの英語の文法と文体をチェックします。
- **詳細を付け加える書き換えを受け入れるなら:** [humanizer](https://github.com/blader/humanizer) 4.0.0（2026 年 11 月 10 日）は、文章が具体性に欠ける箇所でモデルに 'add a specific detail you know to be true' と指示します。Laconic は事実を付け加えず、`laconic-check preserve --strict` は保護対象の数値、URL、引用などを追加した編集を失敗にします。
- **文章ガイドだけが欲しいなら:** Laconic の文章ガイドの土台である [No AI Slop](https://github.com/petergyang/no-ai-slop) は、応答ルールとオーストラリア英語なしでそれを提供します。
- **Microsoft Word で編集するなら:** [PerfectIt](https://www.perfectit.com/) は Word 上で、Australian Government Style 用を含むスタイルシートに照らして一貫性をチェックします。Laconic はエージェント、チャットアシスタント、CI で動き、Word では動きません。
- **自分たちのハウススタイルを CI でチェックしたいなら:** [Vale](https://github.com/vale-cli/vale) は、正規表現、ルールの種類、パッケージで設定したルールに照らして文章をチェックします。`laconic-check` は Laconic のルールにプロジェクト独自の語の置き換えと禁止フレーズを加えるだけです。

<a id="what-changes"></a>
## 何が変わるか

<table>
<tr>
<td width="50%">

### 変更前

> Great question! Let me take a look. Your auth flow has a few moving pieces: the middleware, the token verification and the cookie handling. It's likely a caching issue, which typically resolves itself within an hour or so. Hope this helps! Let me know if you want to dig deeper.

</td>
<td width="50%">

### 変更後

> The 401 comes from the missing `Authorization` header: the endpoint requires a bearer token and the request sends none.
>
> 1. Add `Authorization: Bearer <token>` at `src/api.ts:42`
> 2. Run `npm test -- api.spec.ts`
>
> Not verified: whether the token expires mid-session. The logs do not show it.

</td>
</tr>
</table>

例は Laconic が書く言語である英語のままにしています。

<a id="what-it-includes"></a>
## 含まれるもの

| 構成要素 | パス | 適用されるとき |
|---|---|---|
| セッションモード | `skills/laconic/SKILL.md` | `/laconic` と入力した後、'stop laconic' までセッションの残りの間 |
| 散文スキル | `skills/laconic-prose/SKILL.md` | 下書き、編集、文章監査のためにモデルが読み込み、そのタスクにだけ影響 |
| コンパクトルール | `rules/laconic.md` | エージェントのルールに追加すれば常に読み込まれる（約 240 語） |

セッションのルール全文は [SKILL.md](skills/laconic/SKILL.md) にあります。

1. 答えを先に、理由は後に。
2. 何を知っていて、どう知ったのかを述べる。未確認の主張は明示する。「手元の証拠では判断できません」も受け入れられる答えです。
3. 長さは、読み手が決めること、すること、理解することに合わせる。言葉は削っても、中身は削らない。
4. 手順には番号を付ける。
5. 今のタスクから外れない。
6. 状態を見えるようにしておく。
7. エラーははっきり述べる。エラー、分かっている原因、修正方法または次に確認すること。
8. 根拠があるときだけ見積もる。
9. 長いリストはまとめるが、関係する項目は落とさない。
10. 前置き、まとめ直し、締めの言葉は書かない。

どの構成要素もオーストラリア英語（綴り、一重引用符、カンマ、数字、日付、エムダッシュ不使用）を適用し、オーストラリアの資料で形が決まらない場合はイギリス英語に従い、アメリカ英語は決して使いません。独自ルール: Style Manual が 'while' と 'among' を好む箇所でも whilst と amongst を使います。散文スキルは、執筆、編集、監査の完全な流れを加えます。よくある AI の文章パターンに対する素早いチェック、意味と語り口の維持、媒体別のガイド、Style Manual の細則、AGLC4 の引用メモです。

<a id="evaluation"></a>
## 評価

2026 年 10 月 19 日に `evals/` のハーネスで測定しました。ケース 30 件、各 2 試行、Claude Opus 5.5 が生成しブラインドで審査しました。各構成には、それぞれのスキル、ルール、参考資料がインストールされ読める状態でした。比較対象の構成は、Laconic が置き換える No AI Slop スキルと、そのコンパクトルール stop-slop です。

| 指標 | Laconic | No AI Slop 構成 | 差（95% 区間） |
|---|---|---|---|
| 正確さ（1〜5） | 4.68 | 3.98 | +0.70（+0.33〜+1.07） |
| 簡潔さ（1〜5） | 4.67 | 4.12 | +0.55（+0.30〜+0.80） |
| 安全性（1〜5） | 4.90 | 4.85 | +0.05（−0.10〜+0.20） |
| 加重スコア | 4.74 | 4.39 | +0.35（+0.18〜+0.54） |
| ブロッキング指摘 | 1 | 7 | |
| 平均回答長（文字） | 1,360 | 1,663 | |

<details>
<summary>2 人目の審査者、チェッカーの集計、追加実行</summary>

別のモデルファミリーの 2 人目のブラインド審査者（GPT-6 Pro）は方向には同意しましたが、差は小さくなりました。加重スコア 4.63 対 4.46（差 +0.17、95% 区間 +0.05〜+0.30）、正確さ +0.33（+0.12〜+0.58）です。この審査者は独自の細則をはるかに厳しく適用し、両構成の多くの回答でシリアルカンマと単語で書かれた数字を見つけました。

`tools/laconic-check` は審査者なしでそうしたミスを数えます。構成ごとの同じ 60 件の回答について:

| 構成 | 指摘なしの回答 | シリアルカンマの候補 | 単語で書いた数字 | 二重引用符 |
|---|---|---|---|---|
| No AI Slop 構成 | 30 | 32 | 17 | 67 |
| Laconic コンパクトルール | 40 | 13 | 15 | 1 |
| Laconic セッションモード（`/laconic`） | 51 | 12 | 3 | 0 |

2026 年 10 月 20 日の追加実行（ケース 30 件、1 試行）では、その後のルール変更を測りました。コンパクトルールに数字のルールを加えると、単語で書いた数字は 5 から 0 に減り、指摘なしの回答は 30 件中 18 件から 23 件に増えました。セッションモードはブロッキング指摘なしで加重 4.71、コンパクトルールのみは 4.59 でした。短くしたセッションスキル（1,153 語ではなく 849 語）はスコアが下がり（4.61）、ブロッキング指摘が 3 件出たため、全文を維持しています。

2026 年 10 月 20 日のその他の確認:

- **送信前に下書きをチェック:** `laconic-check` によるセルフチェック付きのセッションモードは、散文と会計のケース 18 件で加重 4.82、セルフチェックなしでは 4.71 でした（会計ケース 10 件では 4.74 対 4.49）。ブロッキング指摘は 0 件対 1 件でした。
- **実際の利用:** Laconic が以前の構成を置き換えた日の作者自身の Claude Code セッションで、`laconic-check` の指摘がないアシスタントのメッセージは 26% から 42% に増え、単語で書いた数字は 1,000 語あたり 5.7 から 3.6 に減りました。
- **エージェントのコスト:** 小さなリポジトリ修正 5 件（`evals/README.md` のエージェントコストベンチマーク、2 試行）で、すべての構成がすべてのタスクに合格しました。セッションモードは、主に常時読み込まれるプロンプトが大きいため、タスクあたりコンパクトルールより約 27% 高くつきました。別立てのエージェント向けガイドは何も節約しなかったため削除しました。

2026 年 10 月 20 日の簡潔さの対照実験（ケース 45 件、2 試行、Opus 5.5 が生成しブラインド審査）では、インストールした構成を比べました。散文スキル付きのコンパクトルールを、決まり文句、付け足した限定、会計年度に関する変更の前後に分け、単一の指示 'Answer concisely.' と比較しています。

| 指標 | Laconic | これらの変更前の Laconic | 'Answer concisely.' |
|---|---|---|---|
| 加重スコア | 4.63 | 4.63 | 4.07 |
| 正確さ（1〜5） | 4.52 | 4.56 | 3.41 |
| ブロッキング指摘 | 6 | 5 | 29 |
| 合格した正規表現チェック | 18 件中 18 | 18 件中 16 | 18 件中 16 |
| `laconic-check` の指摘がない回答 | 90 件中 67 | 90 件中 69 | 90 件中 41 |
| 平均回答長（文字） | 1,232 | 1,148 | 1,022 |

Laconic は単純な指示より加重で 0.56 上回ります。変更は審査上の品質には中立で、他の 2 構成が見落とした会計年度のハイフンを直しています。ブロッキング指摘のうち、変更したガイドに由来するものはありません。

</details>

限界: 生成モデル 1 つによる構成あたり 18〜90 件の回答です。ケースは `evals/cases.jsonl`、ルーブリックは `evals/rubric.md` にあります。これらのケースはルールの文言の調整にも使ったため、スコアはホールドアウトのテストではありません。以下の Claude Code の Concise スタイルとの比較では、2026 年 11 月 1 日に書かれ、ルールの調整には使っていない `evals/holdout-cases.jsonl` の固定ケース 10 件を使います。シリアルカンマが今も最も多いミスです。

<a id="against-claude-codes-concise-style"></a>
### Claude Code の Concise スタイルとの比較

2026 年 11 月 17 日、3 つの構成がホールドアウトの 10 ケースに各 3 試行で回答し、ツールなしの Claude Code 2.1.325 上で Claude Opus 5.5（`claude-opus-5-5`、high effort）が生成と審査を行いました。3 つの構成は、Laconic のコンパクトルール、Claude Code 組み込みの Concise 出力スタイル（`"outputStyle": "Concise"`）、単純な指示 'Answer concisely.' です。審査者はケースごとに 3 件の回答をシャッフルした文字ラベルの下でまとめて見ており、構成の名前は知りませんでしたが、文体は見えました。審査は 2 回行いました。1 回は文体を採点しないよう指示する `evals/rubric-correctness.md` に従って正確さだけを、もう 1 回は手作業でラベル付けした較正用回答 10 件すべてと一致した後に Laconic 独自のルーブリック `evals/rubric.md` に従って審査しました。

| 指標 | Laconic | Concise スタイル | 'Answer concisely.' |
| --- | --- | --- | --- |
| 正確さ、文体を除く（1〜5） | 4.80 | 4.27 | 4.47 |
| Concise スタイルとの差（95% 区間） | +0.53（0.00〜+1.17） | | +0.20（−0.13〜+0.53） |
| ブロッキング指摘、文体を除く | 0 | 4 | 1 |
| 加重スコア、Laconic 独自のルーブリック（1〜5） | 4.69 | 4.06 | 4.11 |
| Concise スタイルとの差（95% 区間） | +0.63（+0.29〜+1.00） | | +0.05（−0.10〜+0.22） |
| ブロッキング指摘、Laconic 独自のルーブリック | 1 | 13 | 10 |
| `laconic-check` 0.5.0 の指摘がない回答 | 30 件中 24 | 30 件中 16 | 30 件中 13 |
| 合格した正規表現チェック | 12 件中 9 | 12 件中 9 | 12 件中 9 |
| 平均回答長（文字） | 782 | 614 | 627 |
| 平均出力トークン、思考を含む | 475 | 327 | 298 |
| 回答あたりの平均コスト、コールド呼び出し 1 回、Claude Code の報告値 | US$0.022 | US$0.019 | US$0.016 |
| それに対する Laconic のコスト（95% 区間） | | 1.18 倍（1.07〜1.29） | 1.39 倍（1.30〜1.50） |

Laconic 独自のルーブリックは、Laconic だけが求めるオーストラリア英語と独自の細則を採点し、審査者はそれを加重スコアだけでなく正確さとブロッキング指摘にも適用しました。そのため、審査した行のうち文体に中立なのは最初の 3 行だけです。正確さだけで見ると Laconic は Concise スタイルより 0.53 高いものの、10 ケースでは区間が差なしに届くため、明確な優位ではありません。ブロッキング指摘は Laconic が 0 件、Concise スタイルが 4 件でした。最初の審査のメモによると、Concise スタイルは給与関連メール 3 通のうち 2 通で 2026 年 11 月 26 日または 28 日の曜日を誤り、3 通目では締め切りを埋もれさせ、レビュー用メモ 3 件のうち 2 件でタスクが確認していないテストカバレッジを主張し、監査対象のテキストを 1 回誤って引用しました。'Answer concisely.' も同様のミスをしました。Laconic は Concise スタイルより出力トークンが 46% 多く、回答あたりのコストは 18% 高くなりました。3 構成とも、Laconic のルールが独自のハイフンを求める '2026–27' で Style Manual のエンダッシュを残したため、そのケースの正規表現チェックに失敗しました。

限界: 構成あたり 30 件の回答で、生成したモデル自身が審査し、ケースは Laconic の作者が書いたものです。文体を除く審査には較正セットがありませんでした。各区間は 10 ケースをリサンプリングしています（10,000 回、シード 2282）。各呼び出しはプロンプトをプロンプトキャッシュに書き込んだため、コストの行は単発のコールド呼び出しのものです。セッション内のようにルールがキャッシュされていれば Laconic の余分な出力はおそらくより大きく効きますが、今回の実行では測っていません。Claude Code とその Concise スタイルはソフトウェア開発のセッション向けで、これらのケースの大半は執筆と助言のタスクです。実行スクリプト、回答ごとのスコア、トークン数とコスト、この表を再計算するスクリプトは [`evals/runs/concise-style-2026-11-17`](evals/runs/concise-style-2026-11-17/) にあり、回答そのものは保存していません。

<a id="token-cost-against-caveman-and-chisle"></a>
### caveman、Chisle とのトークンコスト比較

2026 年 11 月 17 日、3 つのルールセットが `evals/cases.jsonl` の 47 ケースすべてに回答し、ツールなしの Claude Code 2.1.325 上で Claude Opus 5.5（`claude-opus-5-5`、high effort）が生成と審査を行いました。3 つのルールセットは、Laconic のコンパクトルール、[caveman](https://github.com/JuliusBrussee/caveman) 4.1.0 の `skills/caveman/SKILL.md`、[Chisle](https://github.com/JayPokale/Chisle) 3.12.1 の `skills/chisle/SKILL.md` で、それぞれ front matter を除いてシステムプロンプトに追加しました。コールド呼び出し（2 試行）はそれぞれ新しい作業ディレクトリから実行したため、ルールをプロンプトキャッシュに書き込みました。キャッシュ呼び出し（1 試行、審査なし）は準備用の呼び出しの後に 1 つのディレクトリから実行したため、同じセッション内の呼び出しと同様にルールをキャッシュから読みました。審査者はケースごとに 3 件のコールド回答をシャッフルした文字ラベルの下でまとめて見ており、プロンプトにはルールセットの名前はありませんでした。ただし文体は見えており、Laconic の回答 94 件のうち 3 件が Laconic に触れていました（2 件は作業ディレクトリの引用、1 件はスキル名の記載による）。審査は 2 回行いました。1 回は文体を採点しないよう指示する `evals/rubric-correctness.md` に従って正確さだけを、もう 1 回は Laconic 独自のルーブリック `evals/rubric.md` に従って審査しました。

| 指標 | Laconic | caveman | Chisle |
| --- | --- | --- | --- |
| 平均出力トークン、思考を含む、コールド呼び出し | 745 | 655 | 516 |
| それに対する Laconic の出力トークン（95% 区間） | | 1.14 倍（1.04〜1.27） | 1.44 倍（1.27〜1.68） |
| 平均入力トークン、コールド呼び出し | 3,218 | 4,285 | 3,861 |
| 平均出力トークン、思考を含む、キャッシュ呼び出し | 780 | 637 | 492 |
| 回答あたりの平均コスト、ルールをキャッシュから読む場合 | US$0.018 | US$0.016 | US$0.013 |
| それに対する Laconic のコスト、ルールをキャッシュから読む場合（95% 区間） | | 1.17 倍（1.05〜1.29） | 1.45 倍（1.25〜1.61） |
| 回答あたりの平均コスト、コールド呼び出し 1 回、キャッシュ書き込みを含む | US$0.028 | US$0.031 | US$0.026 |
| それに対する Laconic のコスト、コールド呼び出し（95% 区間） | | 0.89 倍（0.83〜0.94） | 1.05 倍（0.96〜1.15） |
| 平均回答長（文字）、コールド呼び出し | 1,297 | 1,198 | 977 |
| 正確さ、文体を除く（1〜5） | 4.68 | 4.44 | 4.38 |
| Laconic の差（95% 区間） | | +0.24（+0.06〜+0.45） | +0.30（+0.10〜+0.53） |
| ブロッキング指摘、文体を除く | 2 | 5 | 7 |
| 加重スコア、Laconic 独自のルーブリック（1〜5） | 4.63 | 4.11 | 4.11 |
| ブロッキング指摘、Laconic 独自のルーブリック | 8 | 43 | 44 |
| 合格した正規表現チェック、コールド呼び出し | 18 件中 18 | 18 件中 16 | 18 件中 16 |
| `laconic-check` 0.5.0 の指摘がない回答、コールド呼び出し | 94 件中 70 | 94 件中 36 | 94 件中 39 |

最も多く書いたのは Laconic で、出力トークンは caveman より 14%、Chisle より 44% 多くなりました。コールド呼び出しでは、caveman の長いルールをキャッシュに書き込むコストが Laconic の余分な出力を上回ったため、回答は Laconic のほうが 11% 安くなりました。セッション内と同じくルールをキャッシュから読む場合は、Laconic の余分な出力により回答が caveman より 17%、Chisle より 45% 高くなりました。正確さだけで見ると Laconic は caveman を 0.24、Chisle を 0.30 上回りました。文体を除いたブロッキング指摘 2 件は、計算ミス 1 件と RFC 7636 のテスト値の誤引用 1 件でした。Laconic 独自のルーブリックは、比較対象のどちらも求めないオーストラリア英語と独自の細則も正確さとブロッキング指摘として採点しており、その基準での比較対象のブロッキング指摘の大半は、シリアルカンマ、二重引用符、単語で書いた数字、アメリカ式の綴りに関するものです。その審査では、典型的な所要期間を示すことを禁じたケースで比較対象の両方が 2 回答とも期間を示したのに対し、Laconic は示しませんでした。Laconic のブロッキング指摘には、ツールがないと告げられて、タスクが到達できるとしたファイルを編集しなかった回答 1 件（Chisle の回答 1 件も同様）、重大な計算ミスのある回答 1 件、タスクが禁じた限界についての注記を付け加えた回答 2 件が含まれます。

限界: モデルは 1 つで、そのモデル自身が回答を審査し、ケースは Laconic のルールの文言の調整にも使ったものです。文体を除く審査には較正セットがありませんでした。トークンとコストの数値は、呼び出しごとに Claude Code が報告した値です。caveman 独自の評価はシステムプロンプトをスキルで置き換えますが、今回の実行では追加しました。比較対象のどちらについても、エージェントが読む内容の圧縮は試しておらず、どちらも独自のプロンプトとモデルによる数値を公開していますが、今回の実行はそれを再現していません。区間は 47 ケースをリサンプリングしています（10,000 回、シード 2282）。実行スクリプト、回答ごとのスコア、トークン数とコスト、この表を再計算するスクリプトは [`evals/runs/token-cost-2026-11-17`](evals/runs/token-cost-2026-11-17/) にあり、回答そのものは保存していません。

<a id="against-other-humanisers"></a>
### 他のヒューマナイザーとの比較

2026 年 10 月 20 日、インストールした 3 つの構成が、オーストラリアの読み手向けに AI が書いたテキスト 20 本（`evals/humanise-cases.jsonl`: 顧客へのメール、PR の説明、リリースノート、取締役会への報告、求人広告など）をヒューマナイズしました。各 2 試行で、Claude Opus 5.5 が生成しブラインドで審査しました。各構成は Skill と Read のツールとともに自分のスキルだけを持ち、それを使うよう指示されました。Laconic の散文スキル（コミット `c7e3048`）、[humanizer](https://github.com/blader/humanizer)（コミット `8c307f3`）、[avoid-ai-writing](https://github.com/conorbronsdon/avoid-ai-writing)（コミット `cb16d10`）です。

| 指標 | Laconic | `humanizer` | `avoid-ai-writing` |
|---|---|---|---|
| 加重スコア、中立ルーブリック（1〜5） | 4.92 | 4.54 | 4.50 |
| 加重スコア、独自ルーブリック | 4.87 | 4.50 | 4.42 |
| 正確さ、中立ルーブリック | 4.80 | 4.13 | 4.18 |
| 簡潔さ、中立ルーブリック | 4.95 | 4.20 | 3.95 |
| ブロッキング指摘 | 0 | 0 | 0 |
| 合格した正規表現による事実とパターンのチェック（下記参照） | 40 件中 40 | 40 件中 40 | 40 件中 35 |
| 元のテキストに対する平均の長さ、語数 | 65% | 81% | 81% |
| テキストあたりの avoid-ai-writing 検出器の指摘（元: 2.6） | 0.03 | 0 | 0.05 |
| テキストあたりの dslop の指摘、リズム指標を含む（元: 1.80） | 0.50 | 0.83 | 0.63 |
| テキストあたりの `laconic-check` 0.1.0 の指摘（元: 1.85） | 0.15 | 0.45 | 0.28 |
| テキスト 40 本の生成コスト | $4.54 | $5.84 | $15.78 |

Laconic の独自細則（数字、シリアルカンマ、エムダッシュ）を外しつつオーストラリア式の綴りは求める中立ルーブリックで、Laconic は加重で `humanizer` を 0.38、`avoid-ai-writing` を 0.43 上回ります。両試行ごと 20 本のテキストをリサンプリングする対応のあるブートストラップによる 95% 区間は、+0.26〜+0.51 と +0.26〜+0.59 です。それらの細則をルーブリックから外しても優位は残ります。他の構成についての審査メモには、カバーレターに加えられた経験や、助成金申請のために導き出されたドル金額など、元のテキストになかった内容が含まれています。このサンプルでは、3 構成とも、これらの検出器の指摘を元のテキストの水準よりはるかに減らしました。

<details>
<summary>正規表現チェックの修正方法</summary>

忠実な言い換え（'in stages'、'a second person will check'、'five to seven'、'Don't install any personal'）で失敗した回答を調べた後、正規表現チェック 4 つの条件を広げました。同じ修正後のチェックをすべての構成に適用し、表には修正後の結果を載せています。変更前は 40 件中 Laconic が 37 件、`humanizer` が 34 件合格でした。`laconic-check` の行はバージョン 0.1.0 によるもので、以降のバージョンはルールが増え、指摘をより多く数えます。

</details>

限界: 生成モデル 1 つによる構成あたり 40 本のテキストで、同じモデルファミリーが、この 20 のプロンプトとこれらのインストール構成で審査しました。各チェッカーは作者の考えを反映しており、`avoid-ai-writing` の検出器はそのスキル自体のエンジンです。正規表現チェックに合格したことは事実が残ったことの部分的な証拠であり、すべての限定が残った証明ではありません。再実行のコマンドは [evals/README.md](evals/README.md) にあります。

<a id="fact-changes-on-a-public-set"></a>
### 公開セットでの事実の変化

2026 年 11 月 14 日、Laconic の散文スキルと [humanizer](https://github.com/blader/humanizer) 4.0.0 が、それぞれ [AI Humanizer Benchmark](https://huggingface.co/datasets/ai-humanizer-benchmark/ai-humanizer-benchmark) の 2026 年 9 月〜11 月サイクルの公開テキスト 99 本を、各 2 試行、異なるモデルで 2 ラウンドにわたり書き換えました。外部のチェッカー [humanizer-fact-audit](https://github.com/zwang-devv/humanizer-fact-audit) が、元のテキストの数値、日付、人名や組織名、3 語以上の引用のうち少なくとも 1 つを失った書き換えを数えました。手法は、書き換えを行う前にコミットしました。

| ラウンド | Laconic | `humanizer` | 差（95% 区間） |
|---|---|---|---|
| Claude Opus 5.5 | 134 件中 18（13.43%） | 134 件中 10（7.46%） | +5.97 ポイント（0.00〜+12.32） |
| `gpt-6.1-sol`（Codex） | 134 件中 20（14.93%） | 134 件中 20（14.93%） | 0.00 ポイント（−2.21〜+2.14） |

Opus 5.5 では、Laconic は `humanizer` より多くの書き換えで単位を失い（18 対 10）、どのグループでも多く失いました。数値と日付は 440 件中 9 対 5、名前は 164 件中 12 対 9、引用は 70 件中 5 対 0 です。`gpt-6.1-sol` では 2 つのスキルは同数でした。どちらの区間も 0 を除外しないため、どちらのラウンドも明確な差を示していません。Laconic の書き換えは両ラウンドとも短く、元の語数の 0.94 と 0.90 で、相手は 1.01 と 0.93 でした。

限界: チェッカーは単位を照合するため、引用の無害な言い換えを指摘することがあります。変わった条件、否定、捏造された主張は見えないため、事実への忠実さの完全なテストではありません。テキスト 99 本のうち 32 本にはチェックできる単位がないため、割合はスキルあたり 134 件の書き換えを対象とします。各ラウンドは書き換えにモデル 1 つを使い、新しいコンテキストごとにテキスト 3 本を与えました。指示はインストールしたプラグインではなくテキストとして与えました。Opus のラウンドは Codex のラウンドの後に、手法を変えずに追加しました。比較対象の選定、手法の作成、両ラウンドの実行は Laconic の作者が行いました。固定したリビジョンは [evals/README.md](evals/README.md#public-set-fact-check) にあります。

2026 年 11 月 17 日に Claude Opus 5.5 が 2026-10 サイクルのテキスト 33 本を各 1 回、呼び出しごとに 1 本ずつ書き換えたところ、Laconic のスキルは単位を含む文書 20 本のうち 5 本で単位を変えました。`humanizer` も同じで、avoid-ai-writing と `work-humanizer` は 1 本、`--verify` 付きの patina は 0 本、依頼文だけの場合は 8 本でした。商用ヒューマナイザー 11 個は 7〜13 本を変えました。テキストあたり 1 回の書き換えでは、スキル間の差を示すには少なすぎます（[ヒューマナイザー別](evals/README.md#by-humaniser)）。

<a id="checker"></a>
## チェッカー

`tools/laconic-check` は依存関係のない小さな Rust 製 CLI です。`lint` は Markdown の文章から次の候補を指摘します。エムダッシュ、アメリカ式の綴り、接続詞と前置詞としての 'while' と 'among'、二重引用符、シリアルカンマと思われるもの、単語で書いた数字、'5 %' や '$ 27.99' の空白、'21st of September' や年代の '1980's' のような Style Manual 以外の形式の日付と時刻、日付と合わない曜日、ラテン語の略語、チャットボット風の締めの言葉、'Bottom line:' や 'circle back' のような決まり文句、演出された対比（'it's not X, it's Y'、'not just X but Y'）、動詞のない種明かし型の疑問文（'The result? A 40% cut.'）、'since the dawn of time' のような時代を超えた書き出し、'open-source' や 'e-mail' のような不要なハイフンです。コード、URL、引用文はスキップします。リンク内を含め、ゼロ幅文字、引用マーカー、ChatGPT のトラッキングタグのようなチャットツールの残りかすや、テキストを隠したり並べ替えたりできる見えない文字も指摘します。Unicode のタグ文字の連続（イングランド、スコットランド、ウェールズの旗を除く）と双方向制御文字です。`lint` を失敗させない助言として、2 文以上に節を区切るダッシュ（エムダッシュ、または前後に空白のあるエンダッシュ、ハイフン、二重ハイフン）がある段落（1 つの挿入句を囲む 1 組は 1 回と数える）、'could potentially' や 'may possibly' のような重ねたぼかし表現、太字のラベル（'**Cause:** ...'）で始まる 3 つ以上連続した箇条書きを指摘します。`preserve` は、編集で落ちた数値、大きさと単位（'$47.3 million' から '$47.3 billion'、'150 km' から '150 miles'）、日付付きの月、'Thursday' のような曜日名（0.6.0 から）、インラインコード、URL、引用、'not'、'must'、'will'、'if'、'before'、'likely' のような語、'up to' や 'about 50%' のような範囲表現を報告し、それらで失敗します。`--loose` を付けると、'often'、'some'、'can'、'will'、'because'、'likely' のような頻度、量、能力、予測、原因、ぼかしを表す語が落ちた場合は、代わりに確認用として一覧に載ります。大胆な言い換えでは事実を変えずにそうした語が落ちることがよくあるためです。落ちたぼかし表現は主張を誇張することもあるため、一覧の語は 1 つずつ読んでください（0.6.0 から）。数値は符号、通貨、そして金額や桁区切りのある数値を囲む会計用の括弧を保持するため、'-3%' から '3%'、'(1,200)' から '1,200'、'$500' から '£500' は変更とみなされます。'(62%)' のようなその他の括弧付きの数値は挿入句です。通貨は数字の前の記号（'$'、'£'、'¥'、または '€'、'₹'、'₩' のような Unicode の Currency Symbols ブロックの記号）か、数字の後の 'dollars'、'euros'、'yen' であるため、'$30 billion' と '30 billion dollars' は一致します。数値の後の 'Pounds' は、どちらかのテキストがその数値と一緒に '£' を書いていれば金額、そうでなければ重さです。通貨の語、Currency Symbols ブロックの残り、'pounds' は 0.6.0 からです。'three' と '3'、'1000' と '1,000'、'5 per cent' と '5%' を同じ値と数え、'45 to 60 minutes' や '45-60 minutes' の両端をどちらも分として読み（0.6.0 から）、'didn't' と 'did not' は同じに、'can't' と 'won't' は別に読みます。'5:00 p.m.' と '5pm' は同じ時刻として読みますが、'5:30' のようなコロン付きの数値は、'at' や 'until' のような語の後、'am' や 'pm' の前、または時刻に続く 'to' や 'and' の後でのみ時刻とみなし、それ以外では比率のような 2 つの数値とみなします（0.6.0 から）。太字のものを含め、2 桁までの番号付きリストの記号は無視します。捏造された数値のように編集で加わったものも助言として一覧にし、何も加えてはならない書き換えには `--strict` がそれらでも失敗させます。編集で失われた、または加わった、大文字で始まる 2〜3 語の名前は確認用として一覧に載り、失敗にはなりません。逆転の可能性も同様です。元の文が否定していた内容を、編集後の文が否定なしで述べている場合で、'is not cast' から 'is cast' がその例です。演出された対比の否定された半分を削ると否定語も一緒に消えるため、一覧には載りません。AI Humanizer Benchmark の 2026-10 と 2026-11 サイクルの商用の書き換え 693 件では 4 件を一覧にし、いずれも誤検知でした。2 件は別の言葉で意味を保っており（'So stop guessing'、'Otherwise ...'）、2 件は別の主張をする文と対応付けられていました（0.6.0 から）。最後の行は、空白で区切った語数で両方の長さと、元に対する編集後の割合を示します。意味は読まないため、問題なしという結果はこれらの欠落がないことを示すだけです。指摘は確認する理由であって、欠陥の証明ではありません。

```bash
cargo run --manifest-path tools/laconic-check/Cargo.toml -- lint README.md
cargo run --manifest-path tools/laconic-check/Cargo.toml -- preserve original.md edited.md
```

`lint` と `preserve` は WebAssembly にコンパイルされ、ブラウザ上の [yungsnat.github.io/Laconic](https://yungsnat.github.io/Laconic/) でも動きます。`main` からビルドされます。ページは、コンテンツセキュリティポリシーがすべてのネットワークリクエストを遮断する単一の HTML ファイルなので、貼り付けたテキストは手元のマシンから出ません。ソースは `web/index.html`、WebAssembly のクレートは `tools/laconic-web` です。ビルド方法:

```bash
rustup target add wasm32-unknown-unknown
cargo build --release --target wasm32-unknown-unknown --manifest-path tools/laconic-web/Cargo.toml
cargo run --manifest-path tools/laconic-tools/Cargo.toml -- web-page tools/laconic-web/target/wasm32-unknown-unknown/release/laconic_web.wasm laconic.html
```

Linux（x86_64、静的リンク）、Windows（x86_64）、macOS（Apple silicon）向けのビルド済みバイナリが、SHA-256 チェックサムとともに各 `laconic-check-v*` リリースに添付されています。バイナリを PATH に置いてください。ソースからのビルドに必要なのは Rust だけです。

すでに [Vale](https://vale.sh) を使っているプロジェクト向けに、`vale/Laconic` は `lint` のルール表から生成した Vale のスタイルで、各リリースに `Laconic.zip` パッケージとして添付されます（0.6.0 から）。正規表現で表せるルールを扱います。表せない 7 つと、残りが `lint` とどう違うかは [vale/README.md](vale/README.md) にあります。

0.4.0 以降のリリースには、各アーカイブのビルド来歴の証明と、Scoop、Homebrew、winget、MCP Registry 用のマニフェストも付いています。MCP バンドル `laconic-check.mcpb` は Claude Desktop に 1 ステップでインストールできます。ダウンロードしたアーカイブは `gh attestation verify <archive> --repo yungSNAT/Laconic` で確認してください。Scoop はリリースから直接インストールします。

```bash
scoop install https://github.com/yungSNAT/Laconic/releases/latest/download/laconic-check.json
```

Homebrew のフォーミュラと winget のマニフェストは tap と winget カタログ用に添付していますが、tap と winget カタログにはまだ laconic-check が掲載されていません。

[docs/rules.md](docs/rules.md) は各ルールを名前ごとに、例、対応する Style Manual の節、人が書いた文章をどのくらいの頻度で指摘するかとともに説明し、チェッカーが扱う Style Manual の節も示します。誤検知は [issue フォーム](https://github.com/yungSNAT/Laconic/issues/new?template=false-positive.yml)で報告してください。[docs/humanizer-coverage.md](docs/humanizer-coverage.md) は `humanizer` 4.0.0 の 26 パターンそれぞれを、それを扱うルールと散文スキルの節に対応付けています。スキルは 26 すべてを扱い、`lint` はそのうち 8 つを何らかの形で指摘します。

`lint` を CI や pre-commit フックのゲートとして使う場合、次のオプションで何を報告し何で失敗するかを決めます。

| オプション | 効果 |
| --- | --- |
| `--json` | JSON オブジェクトを 1 つ出力します。それぞれ `file`、`line`、`column`、`rule`、`advisory`、`message`、`url` を持つ `findings`、続いて `failing` の件数と、コメントで `suppressed` された指摘の数です。 |
| `--sarif` | GitHub の code scanning などの SARIF ビューア向けに [SARIF 2.1.0](https://docs.oasis-open.org/sarif/sarif/v2.1.0/sarif-v2.1.0.html) のログを出力します。すべてのルールを説明とリンク付きで含め、各指摘を `warning` レベル（助言なら `note`）の結果として含めます。位置は、code scanning がリポジトリのパスに対応付けるファイルの絶対 `file` URI と、SARIF の数え方どおり UTF-16 コード単位の列で示します。`--json` とは併用できません。0.6.0 から。 |
| `--disable <rules>` | これらのルールの指摘を捨てます。カンマ区切りで、繰り返し指定できます。 |
| `--only <rules>` | これらのルールの指摘だけを残します。`--disable` が優先します。 |
| `--max-findings <n>` | 失敗となる指摘が n 件を超えた場合だけ失敗します（既定値 0）。 |
| `--warn` | 指摘では失敗しません。 |
| `--commit-msg` | 各ファイルを Git のコミットメッセージとして読み、コメント行とはさみ線より下は除きます。 |
| `--config <file>` | `.laconic-check` の代わりにこのファイルから設定を読みます。 |

`lint` を実行するディレクトリにある `.laconic-check` ファイルには、`disable`、`only`、`max-findings` の `key = value` 行と、`#` のコメント行を書きます。`accept` 行は、`us-spelling` が指摘しない英字のみの語を並べ、それぞれ同じ大文字小文字で単語単位で照合します。そのため `accept = color` でも文頭の 'Color' は指摘されます（0.6.0 から）。コマンドラインのルールはそのリストに追加され、`--max-findings` はその件数を置き換えます。

```text
# The CSS property names color and center are expected here.
accept = color, center
# This project uses serial commas.
disable = serial-comma
```

設定の後の `[rules]` セクションには、プロジェクト独自のルールを 1 行に 1 フレーズずつ書きます。`id: phrase = replacement` は置き換えを提案し、`id: phrase =` はそのフレーズを禁止します（0.6.0 から）。各指摘はルールを `project/<id>` と表示し、`disable`、`only`、`--disable`、`--only`、抑制コメントがこれを受け付けます。複数の行が同じ id を共有できます。フレーズは英字と数字からなる 1 語以上で、語の内部にはハイフンやアポストロフィを使えます。文章中で単語単位に、改行をまたいでも照合し、コード、URL、引用の中では照合しません。小文字で書けば大文字小文字を問わず照合し、大文字を含めば書いたとおりにだけ照合します。置き換え後と完全に同じに書かれたテキストは指摘しないため、`github = GitHub` は 'GitHub' をそのままにします。プロジェクトの指摘も Laconic 自身の指摘と同様に `lint` を失敗させます。ルール参照のリンクはなく、`--json` では `url` が `null` になり、SARIF のログは Laconic のルールの後にリンクなしでそれらのルールを並べます。形式の誤った行や未知のセクションがあると、`lint` はその行番号とともに止まります。

```text
[rules]
plain-words: utilise = use
plain-words: leverage = use
filler: going forward =
names: Javascript = JavaScript
```

指摘を 1 つ抑制するには、その前の独立した行に、ルール名を書いたコメントを置きます。対象は次の空でない行だけで、何も抑制しなかったルール名は、コメントが別のルールを抑制していても、`lint` がそれぞれ `unused-suppression` として報告します。

```markdown
<!-- laconic-check-disable-next-line us-spelling -->
Set the color property of the heading to red.
```

| 終了ステータス | 意味 |
| --- | --- |
| 0 | `lint` が `--max-findings` で許される数を超える失敗指摘を見つけなかった、または `--warn` で実行された。`preserve` が欠落を見つけず、`--strict` では追加も見つけなかった。`mcp` が入力の終わりに達した。`lsp` が `shutdown` の後に `exit` を受け取った。`hook` がイベントに応答した。`--version` または `--help` がテキストを出力した。 |
| 1 | `lint` が許される数を超える失敗指摘を見つけた。`preserve` が欠落した項目、または `--strict` で追加された項目を見つけた。`lsp` が `shutdown` なしで `exit` を受け取った、または入力が終わった。`hook` がイベント、プロジェクトファイル、未コミットの Markdown ファイルを読めなかった、または Git がリポジトリが見つからないこと以外の理由で失敗した。 |
| 2 | 使い方の誤り。`hook` 以外のコマンドでのプロジェクトファイルのエラーや読み取りエラー。または `mcp` や `lsp` での入出力エラー。読み取りエラーの後も、`lint` は他のファイルのチェックを続けます（0.6.0 から）。 |

[pre-commit](https://pre-commit.com) で実行するには、`laconic-check` を PATH に置き、そのフックを追加します。コミットメッセージのフックには `pre-commit install --hook-type commit-msg` も必要で、0.3.0 には `--commit-msg` がないため laconic-check 0.4.0 以降が必要です。

```yaml
repos:
  - repo: https://github.com/yungSNAT/Laconic
    rev: <commit>
    hooks:
      - id: laconic-check
      - id: laconic-check-commit-msg
```

GitHub Actions では、コンポジット Action がランナー（Linux x86_64、Windows x86_64、Apple silicon 上の macOS）向けの laconic-check リリースをダウンロードし、リリースの SHA-256 ファイルと照合してからチェックし、各指摘に注釈を付けます。チェックサムが捉えるのは壊れたダウンロードやすり替えられたダウンロードで、侵害されたリリースではありません。

```yaml
- uses: actions/checkout@v7
- uses: yungSNAT/Laconic@<commit>
  with:
    version: 0.6.0     # the laconic-check release; 0.6.0 by default
    files: README.md   # one path a line; default: every .md and .markdown file Git tracks
    args: --warn       # further lint options
```

代わりに GitHub の code scanning に指摘を一覧表示するには、`laconic-check` を PATH に置き（リリースに `--sarif` が入るまでは `cargo install --locked --git https://github.com/yungSNAT/Laconic --rev <commit> laconic-check`）、ジョブに `security-events: write` を与えてログをアップロードします。`--warn` では指摘でステップが失敗しないため、使い方の誤りや読み取りエラーのときだけ失敗し、そのときは不完全なものはアップロードされません。指摘でもジョブを失敗させるには、上の Action か、`--warn` なしの別の `lint` ステップを追加してください。

```yaml
- run: laconic-check lint --sarif --warn README.md docs/*.md > laconic-check.sarif
- uses: github/codeql-action/upload-sarif@v4
  with:
    sarif_file: laconic-check.sarif
```

`lint -` は標準入力を読むため、エージェントは下書きを保存せずにチェックできます。`laconic-check` が PATH にあれば、どちらのスキルも書いた成果物を送る前にそれを実行します。アメリカ式からオーストラリア式への単語リストは、手書きのリストと、[English Speller Database](https://github.com/en-wl/wordlist) から生成し Style Manual と照合して見直した約 2,400 組を合わせたものです（`laconic-tools gen-au-words`）。シード欠陥テストは、きれいな文章にルールごとに既知の欠陥を 1 つ埋め込み、チェッカーがそれぞれをちょうど 1 回捉え、きれいなテキストでは何も指摘しないことを求めます。較正テストは、2018 年と 2021 年に公表された Productivity Commission の報告書 3 本（人が書いた文章約 198,500 語、CC BY 3.0 AU）に `lint` を実行し、2,246 件の指摘、67 件の助言（節を区切るダッシュの段落 41、重ねたぼかし表現 8、定義前に使われた略語 14、続け書きとハイフン付きの両方で書かれた複合語 4）、またはそのうちの決まり文句、演出された対比、不要なハイフン 7 件が増えると失敗します。これにより、人が書いた文章での指摘の純増を防ぎます。2026 年 11 月 5 日の外部の実行では、[jenna-russell/human_detectors](https://github.com/jenna-russell/human_detectors) のコミット `afcf03d` にある人が書いた記事 150 本（主にアメリカのノンフィクション 110,723 語、ここには保存していない）をチェックしました。1,000 語あたり 25.9 件の指摘で、その 97% がハウススタイルの違い（二重引用符、アメリカ式の綴り、エムダッシュ、単語で書いた数字、語の選択、シリアルカンマ）でした。AI の兆候のルールは 18 回発動し（1,000 語あたり 0.16: 演出された対比 8、決まり文句 7、種明かし型の疑問文 2、チャットボット風の締め 1）、まだ正誤の分類はしていません。記事はプレーンテキストなので、この実行では `bold-label` を試していません。

2026 年 11 月 13 日、[AI Humanizer Benchmark](https://huggingface.co/datasets/ai-humanizer-benchmark/ai-humanizer-benchmark) の 2026-10 サイクルにおける商用ヒューマナイザー 11 個による書き換え 363 件に `preserve` を実行し、その結果を論文 [Humanized but Still True?](https://arxiv.org/abs/2611.00359) とともに公開された文書ごとの事実判定と比べました。固定したリビジョンとコマンドは [evals/README.md](evals/README.md#fact-audit-agreement) にあります。

| 論文による文書の分類 | 文書数 | laconic-check 0.4.0 | laconic-check 0.5.0 | `--loose` 付きの `main` |
| --- | --- | --- | --- | --- |
| 何らかの単位が変わった | 105 | 105 件失敗 | 105 件失敗 | 105 件失敗 |
| 数値が変わった | 47 | 44 件が欠落した数値や量を報告 | 43 | 43 |
| 3 語以上の引用が変わった | 58 | 58 件が欠落した引用を報告 | 58 | 58 |
| 名前が変わった | 28 | 20 件が確認用の名前を一覧化 | 20 | 20 |
| すべての単位を保持 | 258 | 17 件合格、224 件が意味を担う語だけで失敗、0 件が月・インラインコード・URL で失敗、17 件が数値・量・引用で失敗 | 23 件合格; 222; 0; 13 | 64 件合格; 182; 0; 12 |

同じ文書で、avoid-ai-writing の `validate.js` は変わった 105 件のうち 23 件を、Zero Slop の忠実性チェックは 105 件すべてを失敗とします。保持された 258 件のうち、それぞれ 233 件と 15 件を合格とし、`preserve` は 23 件、`--loose` 付きでは 64 件です（[evals/README.md](evals/README.md#against-other-fact-checkers)、2026 年 11 月 16 日）。論文の表 1 によると、各ヒューマナイザーは、単位を含む書き換え対象のテキスト 20 本のうち 7〜13 本（35%〜65%、Undetectable AI は 20 本中 11 本）で少なくとも 1 つの単位を変えました。それぞれについての `preserve` の結果を含む表は [evals/README.md](evals/README.md#by-humaniser) にあります。

`laconic-check mcp` は、`lint` と `preserve` を標準入出力で MCP ツールとして提供し、ほかに何もインストールする必要はありません。`preserve` ツールは `--loose` と同じ動作のために `loose: true` を受け付け、`lint` ツールは `lint` と同様にサーバーが動くディレクトリの `.laconic-check` ファイルを呼び出しごとに読み直して適用します。ファイルの形式が誤っていれば、呼び出しはその行を示すエラーになります（どちらも 0.6.0 から）。

```bash
claude mcp add --scope user laconic-check -- laconic-check mcp
```

laconic-check 0.3.0 以降に含まれています。laconic-check 0.2.0 のバイナリには、呼び出しごとにバイナリを 1 回実行する以前の Node サーバー `tools/laconic-check/mcp/server.mjs` が必要です。これは非推奨で、今後のリリースで削除されます。

```bash
claude mcp add --scope user laconic-check -- node /path/to/laconic/tools/laconic-check/mcp/server.mjs
```

`laconic-check lsp` は標準入出力で動く言語サーバーで、入力中にエディタが `lint` の指摘を表示します（0.6.0 から）。開いている Markdown 文書が変わるたびにチェックし、各指摘を警告として（助言ならヒントとして）ルールへのリンク付きで示します。文書を含むワークスペースフォルダー（入れ子の場合は最も内側）の `.laconic-check` ファイルを適用し、読めないファイルや解析できないファイルは 1 行目にエラーとして表示します。0.6.0 より後は、ルールが唯一の置き換えを知っている箇所にクイックフィックスも提示します。米国式の綴り、言葉で書いた数、ラテン語の略語、時刻や日付の書式、二重引用符、見えない文字などです。修正は指摘されたテキストだけを変え、選んだときにだけ適用されます（Neovim では `gra`、Helix では `space a`）。書き直しが必要なルール、たとえば全角ダッシュ、決まり文句、シリアルコンマの候補には修正がありません。修正は文書のバージョンを示すため、テキストが変わった後に届いた修正はエディタが誤った位置に適用せず拒否します。そうした編集を受け付けると表明しないエディタには修正を提示しません。例外は、表明せずに受け付ける Neovim です。Neovim は文書が初めて変更される前に作られた修正を確かめないため、文書が開かれてから一度変更された後にだけ修正を受け取ります。`.laconic-check` または `.git` を含むフォルダーをワークスペースフォルダーとする Neovim 0.11 以降では:

```lua
vim.lsp.config('laconic_check', {
  cmd = { 'laconic-check', 'lsp' },
  filetypes = { 'markdown' },
  root_markers = { '.laconic-check', '.git' },
})
vim.lsp.enable('laconic_check')
```

Helix の `languages.toml` では、このリストが Helix 既定の Markdown サーバーを置き換えるため、使っている他のサーバーも追加してください。

```toml
[language-server.laconic-check]
command = "laconic-check"
args = ["lsp"]

[[language]]
name = "markdown"
language-servers = ["laconic-check"]
```

`laconic-check hook` は Claude Code と Codex 向けの Stop フックで、追加するまでは無効です（0.6.0 から）。エージェントがターンを終えようとすると、セッションのディレクトリを含むリポジトリの未コミットの Markdown（ステージ済み、未ステージ、未追跡）を、リポジトリ最上位の `.laconic-check` ファイルでチェックします。`lint` が失敗する状態なら、まず失敗となる指摘を見るようエージェントに求め、最大 50 件を一覧にします。求めるのはターンごとに 1 回なので、指摘が誤検知でもエージェントは終えられます。エージェントの編集だけでなく、あなた自身の未コミットの編集もチェックします。Claude Code では `.claude/settings.json` または `~/.claude/settings.json` に次を追加してください。Codex は同じブロックを `~/.codex/hooks.json` から読み、新しいフックは `/hooks` で確認するまで信頼されていないものとして表示します。

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
## 常時オン

すべての Claude Code セッションでコンパクトルールを読み込むには、ルールフォルダーにコピーします。

```bash
mkdir -p ~/.claude/rules && cp rules/laconic.md ~/.claude/rules/laconic.md
```

代わりにすべての Claude Code セッションを完全なセッションモードで始めるには、空のフラグファイルを作ります。

```bash
touch ~/.claude/.laconic-always
```

セッションモードをオフにするにはファイルを削除します。どちらか一方を選んでください。セッションモードにはすでにコンパクトルールが含まれているため、両方を読み込むとセッションごとに約 240 語が重複します。その他のエージェントについては [INSTALL.md](INSTALL.md) を参照してください。

<a id="development"></a>
## 開発

Codecov のバッジが測るのは、`tools/laconic-evals` にある評価ハーネス `laconic-evals` です。その他の Rust のチェックは CI で実行します。

```sh
cargo test --manifest-path tools/laconic-tools/Cargo.toml
cargo test --manifest-path tools/laconic-evals/Cargo.toml
cargo run --quiet --locked --manifest-path tools/laconic-evals/Cargo.toml -- validate
cargo test --manifest-path tools/laconic-check/Cargo.toml
```

検証、評価、リリースのゲートについては [CONTRIBUTING.md](CONTRIBUTING.md) を参照してください。

<a id="credits"></a>
## クレジット

Laconic は Ayoub G. による [i-have-adhd](https://github.com/ayghri/i-have-adhd)（MIT）のフォークで、応答ルール、プラットフォームアダプター、フック、評価ハーネスはそこから来ています。文章ガイドは、Peter Yang の No AI Slop と Hardik Pandya の Stop Slop に基づく No AI Slop から来ており、[humanizer](https://github.com/blader/humanizer)、[avoid-ai-writing](https://github.com/conorbronsdon/avoid-ai-writing)、[Sepia](https://github.com/Nanako0129/sepia)（いずれも MIT）から取り入れて手を加えた内容を含みます。オーストラリア英語のルールは [Australian Government Style Manual](https://www.stylemanual.gov.au/)（copyright Commonwealth of Australia）に従い、Laconic 自身の言葉で言い直しています。このマニュアルはオープンライセンスで公開されておらず、Laconic はその写しを含みません。`laconic-check` の較正コーパスは [Productivity Commission](https://www.pc.gov.au/) の報告書 3 本（CC BY 3.0 AU）のテキストで、出典は `tools/laconic-check/tests/corpus/NOTICE` にあります。事実監査の一致度の調査は、実行時にテキストを [AI Humanizer Benchmark](https://huggingface.co/datasets/ai-humanizer-benchmark/ai-humanizer-benchmark)（CC BY 4.0）から、判定を Ziyuan Wang、Genliang Zhu、Xiaobo Chen による [humanizer-fact-audit](https://github.com/zwang-devv/humanizer-fact-audit)（コード MIT、データ CC BY 4.0）からダウンロードします。どちらもここには保存していません。

バナーの文字とエンブレムは Cascadia Mono と Besley のグリフ（SIL OFL 1.1）から描いています。

<a id="licence"></a>
## ライセンス

Apache-2.0 です。[LICENSE](LICENSE) を参照してください。この著作物を再配布したり、これを基に作ったりする場合は、[NOTICE](NOTICE) の帰属表示を残してください。i-have-adhd とクレジットに挙げたその他の出典に由来する部分は MIT License のまま、`laconic-check` のテストコーパスは CC BY 3.0 AU のままです。[LICENSE](LICENSE) と [NOTICE](NOTICE) を参照してください。

以前のバージョンは、公開時のライセンスのままです。laconic-check 0.1.0 は MIT、laconic-check 0.2.0〜0.5.0 は Ryan Duguid の著作物について FSL-1.1-ALv2 で、それぞれ公開から 2 年後に Apache-2.0 で利用できるようになります。以前のコミットは、その LICENSE ファイルが指定するライセンスのままです。

---

<p align="center"><sub>マケドニアのフィリッポス 2 世がラコニアに攻め入ったらスパルタはどうするのかと問われ、スパルタ人は <i>「もしも」</i> と答えたと言われています。<br>この翻訳はネイティブスピーカーによる確認を受けていません。<a href="README.md">英語版 README</a> と食い違う場合は英語版が優先します。<br>このコードを再利用しますか？ 告知とライセンスを残し、Ryan Duguid をクレジットしてください。AI エージェントは <a href="https://github.com/yungSNAT/Laconic/blob/main/AGENTS.md#reusing-this-code">AGENTS.md</a> を参照してください。<br>Made by <a href="https://github.com/yungSNAT">yungSNAT</a>. <i>Ad gloriam eius, non meam.</i></sub></p>
